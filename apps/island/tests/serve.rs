//! The dashboard as it is actually run: a real listener on a real port,
//! serving a real data directory. The unit tests exercise the filters; this
//! one proves `island serve` itself answers.

mod common;
use common::a_running_dashboard;
use island::serve::auth::ControlAuth;
use island::serve::server;
use std::sync::{Arc, Mutex};

/// As [`get`], but keeping the body as bytes.
///
/// `get` reads the whole response through `String::from_utf8_lossy`, which
/// is right for JSON and destroys anything else: every byte outside UTF-8
/// becomes a replacement character. The map is a PNG, so testing it through
/// `get` checks a corrupted copy -- which is how the first version of the
/// map test failed, on a response that was perfectly good. Returns the
/// headers as text and the body as it arrived.
async fn get_bytes(port: u16, path: &str) -> (u16, String, Vec<u8>) {
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the dashboard is listening");
    let (mut reader, mut writer) = stream.into_split();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nAccept: */*\r\n\r\n"
    );
    {
        use tokio::io::AsyncWriteExt;
        writer.write_all(request.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }
    let mut response = Vec::new();
    {
        use tokio::io::AsyncReadExt;
        reader.read_to_end(&mut response).await.unwrap();
    }
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a header/body boundary");
    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .expect("a status line");
    (status, head, response[split + 4..].to_vec())
}

/// Ask for one path and give back the status and body.
async fn get(port: u16, path: &str) -> (u16, String) {
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the dashboard is listening");
    let (mut reader, mut writer) = stream.into_split();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nAccept: */*\r\n\r\n"
    );
    {
        use tokio::io::AsyncWriteExt;
        writer.write_all(request.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }
    let mut response = Vec::new();
    {
        use tokio::io::AsyncReadExt;
        reader.read_to_end(&mut response).await.unwrap();
    }
    let response = String::from_utf8_lossy(&response).into_owned();
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, response)
}

#[tokio::test]
async fn the_server_serves_its_pages_and_its_api_on_a_real_port() {
    let data_dir = tempfile::tempdir().unwrap();
    let (population, warnings) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [7u8; 32]).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let population = Arc::new(Mutex::new(population));

    // Port 0 lets the OS pick a free one, which keeps parallel test runs apart.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes(population, ControlAuth::LoopbackOnly);
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });

    let (status, body) = get(port, "/healthz").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"status\":\"ok\""), "{body}");

    let (status, body) = get(port, "/").await;
    assert_eq!(status, 200);
    assert!(body.contains("content-security-policy"), "{body}");
    assert!(
        body.contains("The island, before the clock starts"),
        "{body}"
    );

    let (status, body) = get(port, "/people").await;
    assert_eq!(status, 200);
    assert!(body.contains("Everyone on the island"), "{body}");

    let (status, body) = get(port, "/creator").await;
    assert_eq!(status, 200);
    assert!(body.contains("Add someone to the island"), "{body}");

    let (status, body) = get(port, "/api/humans").await;
    assert_eq!(status, 200);
    assert!(body.contains("Gem-D") && body.contains("Gem-K"), "{body}");

    let (status, body) = get(port, "/api/humans/Gem-K").await;
    assert_eq!(status, 200);
    assert!(body.contains("\"id\":\"identity\""), "{body}");

    let (status, _) = get(port, "/nowhere-at-all").await;
    assert_eq!(status, 404);

    server.abort();
}

/// A dashboard with no world says so, plainly, rather than erroring.
///
/// `island serve` without `--scenario` is an ordinary way to run this: the
/// human-only bootstrap is what the dashboard did before there was a world
/// to run, and asking for the world then is not a mistake worth a 404.
#[tokio::test]
async fn a_dashboard_without_a_world_says_there_is_no_world() {
    let data_dir = tempfile::tempdir().unwrap();
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [3u8; 32]).unwrap();
    let population = Arc::new(Mutex::new(population));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes(population, ControlAuth::LoopbackOnly);
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });

    let (status, body) = get(port, "/api/world").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"running\":false"), "{body}");
    assert!(
        body.contains("--scenario"),
        "it does not say how to get one: {body}"
    );

    // And the page keeps its no-world copy, which is the honest one here.
    let (_, page) = get(port, "/").await;
    assert!(
        page.contains("The island, before the clock starts"),
        "a dashboard with no world should not claim a clock"
    );

    server.abort();
}

/// A dashboard with a world serves it, and the island is really stepping.
#[tokio::test]
async fn a_dashboard_with_a_world_serves_the_island_as_it_steps() {
    use island::serve::sim::{spawn, SimSpeed};
    use std::path::PathBuf;

    let repo = |path: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    let mut scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    // The stems are not what this is testing, and 200,000 of them make the
    // bootstrap slow enough to dominate the test.
    scenario.estate_patch.tree_cap = 50;
    // A small island: 240 x 192 medium cells against the default's
    // 1,200 x 960, 25x fewer. The grids are what load this suite, not the
    // trees -- `tree_cap` shrinks the vegetation and leaves two
    // 1,152,000-cell grids to build on every bootstrap and hash on every
    // digest, a dozen times over, on four cores.
    //
    // The seed changes with the profile because it has to. `IslandLife`
    // validates the coastline it generates against the profile's shape
    // rules, and the default scenario's seed makes a small island with
    // "0 major headlands, need 3". Seed 16 is simply the first that does
    // not: found by trying 0, 1, 2 ... through
    // `RegionalPhysicalState::bootstrap`, which runs that check before the
    // expensive spin-up, so bad seeds cost nothing. It took 17 tries and
    // 1.3 seconds.
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut small_seed = [0u8; 32];
    small_seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = small_seed;
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();
    // Paced, not flat out. This container has four cores and this file
    // holds eighteen islands, each on its own sim thread: at
    // `AsFastAsPossible` every one of them pegs a core and the tokio
    // runtime serving the HTTP requests is starved, so
    // `a_new_person_can_be_read_the_moment_they_exist` misses its
    // sixty-second deadline. Measured, that was about a coin flip --
    // 19 passed twice, then failed twice, on the same commit.
    //
    // The dominant cost is not the stepping, it is `DIGEST_EVERY`: every
    // 60 ticks the loop hashes the whole state, and that walks two
    // 1,152,000-cell grids whatever `tree_cap` is, so a test island pays
    // nearly what the real one does. Slowing the ticks is what thins the
    // digests out. `Times(600)` runs a 60-second step every 100 ms, so a
    // digest lands every six seconds rather than every half one, and a
    // test needing sixty ticks still gets them well inside its deadline.
    let world = spawn(life, SimSpeed::Times(600));

    let data_dir = tempfile::tempdir().unwrap();
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [5u8; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes_with_world(
        Arc::new(Mutex::new(population)),
        ControlAuth::LoopbackOnly,
        Some(world.clone()),
    );
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });

    let (status, body) = get(port, "/api/world").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"running\":true"), "{body}");
    assert!(body.contains("Gem-D"), "the founders are missing: {body}");
    // The space is named the way the layout names it, not as a debug id.
    assert!(
        body.contains("Bedroom"),
        "nobody is anywhere nameable: {body}"
    );
    assert!(
        !body.contains("SpaceId("),
        "a space id reached the wire: {body}"
    );
    assert!(
        body.contains("\"day_length_hours\":36.0"),
        "the island's day is not 36 hours: {body}"
    );

    // The clock moves. Polled with a deadline rather than waited on for a
    // fixed moment: the island hashes itself once at startup, which costs
    // about 930 ms against a 1.4 ms step, so a short fixed wait can expire
    // before the first step has run at all. That is what the first version
    // of this did, and it failed for that reason rather than for a world
    // that was not stepping.
    let first = tick_of(&body);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let moved = loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let (_, later) = get(port, "/api/world").await;
        let now = tick_of(&later);
        if now > first {
            break now;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the island never stepped past tick {first}"
        );
    };
    assert!(moved > first);

    world.stop();
    server.abort();
}

/// The `tick` out of a world response.
fn tick_of(body: &str) -> u64 {
    let json = body
        .split("\r\n\r\n")
        .nth(1)
        .expect("a body after the headers");
    let value: serde_json::Value = serde_json::from_str(json.trim()).expect("the world is JSON");
    value["clock"]["tick"].as_u64().expect("a tick")
}

/// Ask with a body, and give back the status and response.
async fn post(port: u16, path: &str, body: &str) -> (u16, String) {
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the dashboard is listening");
    let (mut reader, mut writer) = stream.into_split();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    {
        use tokio::io::AsyncWriteExt;
        writer.write_all(request.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }
    let mut response = Vec::new();
    {
        use tokio::io::AsyncReadExt;
        reader.read_to_end(&mut response).await.unwrap();
    }
    let response = String::from_utf8_lossy(&response).into_owned();
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (status, response)
}

/// Somebody can be created in a room of the estate, from the API, and ends
/// up in the world rather than only on disk.
#[tokio::test]
async fn a_person_created_through_the_api_turns_up_in_the_world() {
    let (world, port, server) = a_running_dashboard().await;

    // The estate's own spaces, as the creator page offers them.
    let (_, world_body) = get(port, "/api/world").await;
    assert!(world_body.contains("Bedroom"), "{world_body}");

    let body = r#"{"name":"Rangi","biological_sex":"female","birth_timestamp":"2001-01-02T03:04:05Z",
        "age_years":30,"height_cm":170,"build":"average","hair_color":"black","eye_color":"brown",
        "skin_tone":"olive","space":1}"#;
    let (status, response) = post(port, "/api/world/humans", body).await;
    assert_eq!(status, 202, "{response}");
    let queued: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let id = queued["command"].as_u64().expect("a command id");

    // The island applies it before its next step; poll for the outcome.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let outcome = loop {
        let (status, response) = get(port, &format!("/api/world/commands/{id}")).await;
        if status == 200 {
            let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
            if value["state"] != "queued" {
                break value;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };

    assert_eq!(outcome["state"], "created", "{outcome}");
    let agent_id = outcome["agent_id"].as_str().expect("an agent id");
    assert!(
        outcome["space"].is_string(),
        "they were put nowhere: {outcome}"
    );

    // And they are in the world the dashboard serves, not just in a reply.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, body) = get(port, "/api/world").await;
        if body.contains(agent_id) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{agent_id} was created but never appeared in the world"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    world.stop();
    server.abort();
}

/// A request the island refuses comes back as a refusal with reasons, not as
/// a person who quietly went somewhere else.
#[tokio::test]
async fn the_island_refuses_a_creation_it_cannot_honour_and_says_why() {
    let (world, port, server) = a_running_dashboard().await;

    // No location at all: refused before it is even queued.
    let no_place = r#"{"name":"Nowhere","biological_sex":"female","birth_timestamp":"2001-01-02T03:04:05Z",
        "age_years":30,"height_cm":170,"build":"average","hair_color":"black","eye_color":"brown",
        "skin_tone":"olive"}"#;
    let (status, response) = post(port, "/api/world/humans", no_place).await;
    assert_eq!(status, 422, "{response}");
    assert!(response.contains("location"), "{response}");

    // A room that does not exist: queued, then refused by the island with
    // the spaces that do exist.
    let nonsense = r#"{"name":"Ghost","biological_sex":"male","birth_timestamp":"2001-01-02T03:04:05Z",
        "age_years":300,"height_cm":170,"build":"average","hair_color":"black","eye_color":"brown",
        "skin_tone":"olive","space":9999}"#;
    let (status, response) = post(port, "/api/world/humans", nonsense).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let outcome = loop {
        let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
        let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if value["state"] != "queued" {
            break value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    assert_eq!(outcome["state"], "refused", "{outcome}");
    let problems = outcome["problems"].to_string();
    assert!(problems.contains("no space 9999"), "{problems}");
    // Every problem at once, not one trip per mistake.
    assert!(problems.contains("age_years"), "{problems}");

    world.stop();
    server.abort();
}

/// A dashboard with no island says so rather than pretending to queue.
#[tokio::test]
async fn creating_in_a_world_that_is_not_running_is_refused() {
    let data_dir = tempfile::tempdir().unwrap();
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [9u8; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes(Arc::new(Mutex::new(population)), ControlAuth::LoopbackOnly);
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });

    let body = r#"{"name":"Nobody","biological_sex":"female","birth_timestamp":"2001-01-02T03:04:05Z",
        "age_years":30,"height_cm":170,"build":"average","hair_color":"black","eye_color":"brown",
        "skin_tone":"olive","space":1}"#;
    let (status, response) = post(port, "/api/world/humans", body).await;
    assert_eq!(status, 409, "{response}");
    assert!(response.contains("--scenario"), "{response}");

    server.abort();
}

/// Where the founders' estate stands, in degrees, read from the island
/// rather than written down here.
///
/// These tests used to spell out `-41.03, 173.56`, which is land on the
/// default island and sea on any other. Asking the island means the
/// coordinates follow the profile and the seed instead of pinning the
/// tests to one geography.
async fn estate_degrees(port: u16) -> (f64, f64) {
    let (status, response) = get(port, "/api/world").await;
    assert_eq!(status, 200, "{response}");
    let world: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let estate = &world["estate"];
    (
        estate["latitude"].as_f64().expect("an estate latitude"),
        estate["longitude"].as_f64().expect("an estate longitude"),
    )
}

/// The body of an HTTP response, after the headers.
fn body_of(response: &str) -> &str {
    response
        .split("\r\n\r\n")
        .nth(1)
        .expect("a body after the headers")
        .trim()
}

/// The roster page's data, from the world rather than from the disk.
#[tokio::test]
async fn the_world_has_its_own_roster_and_its_own_person_pages() {
    let (world, port, server) = a_running_dashboard().await;

    let (status, response) = get(port, "/api/world/humans").await;
    assert_eq!(status, 200, "{response}");
    let roster: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let people = roster["people"].as_array().expect("a roster");
    assert_eq!(people.len(), 2, "the founders are not both there: {roster}");

    let id = people[0]["agent_id"].as_str().unwrap().to_string();
    let (status, response) = get(port, &format!("/api/world/humans/{id}")).await;
    assert_eq!(status, 200, "{response}");
    let person: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();

    // The same shape the stored roster serves, so the page draws both the
    // same way — and the world's own answer to where they are, which the
    // stored roster has no way to know.
    assert_eq!(person["summary"]["agent_id"], id.as_str());
    let sections = person["sections"].as_array().expect("sections");
    assert_eq!(
        sections.len(),
        10,
        "the ten per-person sections are not there"
    );
    assert!(person["where"]["space"].is_string(), "{person}");
    assert!(person["where"]["body_carbon_kg"].is_number(), "{person}");
    // How old the record is, so a page never implies it is live.
    assert!(person["records_at_tick"].is_number());
    assert!(person["tick"].is_number());

    let (status, response) = get(port, "/api/world/humans/nobody-at-all").await;
    assert_eq!(status, 404, "{response}");
    assert!(response.contains("nobody-at-all"), "{response}");

    world.stop();
    server.abort();
}

/// Somebody created is readable at once, not at the next hourly refresh.
#[tokio::test]
async fn a_new_person_can_be_read_the_moment_they_exist() {
    let (world, port, server) = a_running_dashboard().await;

    let body = r#"{"name":"Ata","biological_sex":"female","birth_timestamp":"1999-09-09T09:09:09Z",
        "age_years":22,"height_cm":165,"build":"average","hair_color":"black","eye_color":"brown",
        "skin_tone":"olive","space":1}"#;
    let (status, response) = post(port, "/api/world/humans", body).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let agent_id = loop {
        let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
        let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if value["state"] == "created" {
            break value["agent_id"].as_str().unwrap().to_string();
        }
        assert_ne!(value["state"], "refused", "{value}");
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };

    // Without the refresh on creation this would 404 until the next hourly
    // one — creating somebody and then being told they do not exist.
    let (status, response) = get(port, &format!("/api/world/humans/{agent_id}")).await;
    assert_eq!(
        status, 200,
        "a person who was just created cannot be read: {response}"
    );
    let person: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(person["summary"]["agent_id"], agent_id.as_str());

    world.stop();
    server.abort();
}

/// Pausing and speed change how the island is run, not what it is.
#[tokio::test]
async fn the_island_can_be_paused_and_sped_up_without_changing_it() {
    let (world, port, server) = a_running_dashboard().await;

    // Let it get going, then stop it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, body) = get(port, "/api/world").await;
        if tick_of(&body) > 2 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the island never started"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let (status, response) = post(port, "/api/control", r#"{"command":"pause"}"#).await;
    assert_eq!(status, 200, "{response}");
    assert!(response.contains("\"paused\":true"), "{response}");

    // Paused means paused: the digest and the tick stay put.
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    let (_, first) = get(port, "/api/world").await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    let (_, second) = get(port, "/api/world").await;
    assert_eq!(
        tick_of(&first),
        tick_of(&second),
        "a paused island kept stepping"
    );

    // Resuming leaves exactly the state it was paused in, and carries on.
    let (status, response) = post(port, "/api/control", r#"{"command":"resume"}"#).await;
    assert_eq!(status, 200, "{response}");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, body) = get(port, "/api/world").await;
        if tick_of(&body) > tick_of(&second) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the island did not resume"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // A speed can be set, and a nonsense one is refused by name.
    let (status, response) = post(
        port,
        "/api/control",
        r#"{"command":"set_speed","speed":"60"}"#,
    )
    .await;
    assert_eq!(status, 200, "{response}");
    assert!(response.contains("60x real time"), "{response}");

    let (status, response) = post(
        port,
        "/api/control",
        r#"{"command":"set_speed","speed":"briskly"}"#,
    )
    .await;
    assert_eq!(status, 422, "{response}");
    assert!(response.contains("briskly"), "{response}");

    // And the control commands are in the island's replay log, which is
    // what a record of the run is for.
    world.stop();
    server.abort();
}

/// Each part of the world on its own path, and an unknown one named.
#[tokio::test]
async fn the_world_serves_its_parts_separately() {
    let (world, port, server) = a_running_dashboard().await;

    for (part, expect) in [
        ("estate", "battery_capacity_kwh"),
        ("vegetation", "patch_carbon_kgc"),
        ("materials", "audits_closed"),
        ("clock", "day_length_hours"),
    ] {
        let (status, response) = get(port, &format!("/api/world/{part}")).await;
        assert_eq!(status, 200, "{part}: {response}");
        assert!(
            response.contains(expect),
            "{part} is missing {expect}: {response}"
        );
    }

    let (status, response) = get(port, "/api/world/weather").await;
    assert_eq!(status, 404, "{response}");
    assert!(
        response.contains("weather"),
        "the refusal does not name it: {response}"
    );

    world.stop();
    server.abort();
}

/// People stored before there was a world can be carried into one.
///
/// Phase 0b's dashboard stored complete people with nowhere to be. They are
/// carried in as ordinary creations — recorded in the replay log, reproduced
/// by a replay — rather than by a second path that would drift from the
/// one everybody else goes through.
#[tokio::test]
async fn a_phase_0b_population_can_be_carried_into_the_world() {
    use island::run::commands_from_phase_0b;
    use island::serve::sim::{spawn, SimSpeed};
    use mk_engine::regional::estate_layout::SpaceId;
    use std::path::PathBuf;

    // A stored population, made the way the Phase-0b dashboard makes one.
    let data_dir = tempfile::tempdir().unwrap();
    let (mut stored, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [13u8; 32]).unwrap();
    for name in ["Rawiri", "Ngaire"] {
        stored
            .create_human(
                island_humans::CreateHumanRequest {
                    name: name.to_string(),
                    biological_sex: "female".to_string(),
                    birth_timestamp: "1990-01-01T00:00:00Z".to_string(),
                    birth_latitude: -41.3,
                    birth_longitude: 174.8,
                    age_years: 34.0,
                    height_cm: 168.0,
                    build: "average".to_string(),
                    hair_color: "black".to_string(),
                    eye_color: "brown".to_string(),
                    skin_tone: "olive".to_string(),
                },
                "test",
            )
            .expect("the stored person is created");
    }

    let repo = |path: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    let mut scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    // A small island: 240 x 192 medium cells against the default's
    // 1,200 x 960, 25x fewer. The grids are what load this suite, not the
    // trees -- `tree_cap` shrinks the vegetation and leaves two
    // 1,152,000-cell grids to build on every bootstrap and hash on every
    // digest, a dozen times over, on four cores.
    //
    // The seed changes with the profile because it has to. `IslandLife`
    // validates the coastline it generates against the profile's shape
    // rules, and the default scenario's seed makes a small island with
    // "0 major headlands, need 3". Seed 16 is simply the first that does
    // not: found by trying 0, 1, 2 ... through
    // `RegionalPhysicalState::bootstrap`, which runs that check before the
    // expensive spin-up, so bad seeds cost nothing. It took 17 tries and
    // 1.3 seconds.
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut small_seed = [0u8; 32];
    small_seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = small_seed;
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();
    // Paced, not flat out. This container has four cores and this file
    // holds eighteen islands, each on its own sim thread: at
    // `AsFastAsPossible` every one of them pegs a core and the tokio
    // runtime serving the HTTP requests is starved, so
    // `a_new_person_can_be_read_the_moment_they_exist` misses its
    // sixty-second deadline. Measured, that was about a coin flip --
    // 19 passed twice, then failed twice, on the same commit.
    //
    // The dominant cost is not the stepping, it is `DIGEST_EVERY`: every
    // 60 ticks the loop hashes the whole state, and that walks two
    // 1,152,000-cell grids whatever `tree_cap` is, so a test island pays
    // nearly what the real one does. Slowing the ticks is what thins the
    // digests out. `Times(600)` runs a 60-second step every 100 ms, so a
    // digest lands every six seconds rather than every half one, and a
    // test needing sixty ticks still gets them well inside its deadline.
    let world = spawn(life, SimSpeed::Times(600));

    let commands = commands_from_phase_0b(data_dir.path(), SpaceId(0)).expect("the log reads");
    assert_eq!(commands.len(), 2, "both stored people should be carried");
    for command in commands {
        world.send(command);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [13u8; 32]).unwrap();
    let routes = server::routes_with_world(
        Arc::new(Mutex::new(population)),
        ControlAuth::LoopbackOnly,
        Some(world.clone()),
    );
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });

    // Both turn up in the world, keeping the birthplace they were made
    // with rather than being given the estate's.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, body) = get(port, "/api/world/humans").await;
        let roster: serde_json::Value = serde_json::from_str(body_of(&body)).unwrap();
        let people = roster["people"].as_array().unwrap();
        if people.len() == 4 {
            let ids: Vec<&str> = people
                .iter()
                .map(|p| p["agent_id"].as_str().unwrap())
                .collect();
            assert!(ids.contains(&"rawiri"), "{ids:?}");
            assert!(ids.contains(&"ngaire"), "{ids:?}");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "only {} people reached the world",
            people.len()
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    world.stop();
    server.abort();
}

/// Somebody can be put on a cell of the island, and not in the sea.
#[tokio::test]
async fn a_person_can_be_created_on_a_cell_of_the_island() {
    let (world, port, server) = a_running_dashboard().await;

    // The page is told the grid's shape, so it can offer a cell at all.
    let (_, body) = get(port, "/api/world/vegetation").await;
    let land: serde_json::Value = serde_json::from_str(body_of(&body)).unwrap();
    let rows = land["rows"].as_u64().expect("the grid's rows");
    assert!(rows > 100, "the island is {rows} rows tall");
    assert!(land["land_cells"].as_u64().unwrap() > 0, "no land at all");

    // The estate's own cell is land, so somebody can start there.
    let (_, body) = get(port, "/api/world/estate").await;
    let estate: serde_json::Value = serde_json::from_str(body_of(&body)).unwrap();
    let cell = estate["cell"].as_array().unwrap();
    let (row, col) = (cell[0].as_u64().unwrap(), cell[1].as_u64().unwrap());

    let body = format!(
        r#"{{"name":"Hemi","biological_sex":"male","birth_timestamp":"1995-05-05T05:05:05Z",
        "age_years":31,"height_cm":178,"build":"average","hair_color":"black","eye_color":"brown",
        "skin_tone":"olive","row":{row},"col":{col}}}"#
    );
    let (status, response) = post(port, "/api/world/humans", &body).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let outcome = loop {
        let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
        let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if value["state"] != "queued" {
            break value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    assert_eq!(outcome["state"], "created", "{outcome}");
    assert_eq!(outcome["cell"][0], row, "{outcome}");
    // On a cell rather than in a room, so no estate space.
    assert!(outcome["space"].is_null(), "{outcome}");

    world.stop();
    server.abort();
}

/// An intervention reaches the island in upstream's own vocabulary, and one
/// with no island meaning comes back refused by name rather than quietly
/// doing nothing.
#[tokio::test]
async fn interventions_reach_the_island_and_the_refusals_say_why() {
    let (world, port, server) = a_running_dashboard().await;

    let settled = |port: u16, id: u64| async move {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        loop {
            let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
            let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
            if value["state"] != "queued" {
                break value;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the command never resolved"
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    };

    // Terrain is canon, and the island says so rather than shrugging.
    let sculpt = r#"{"SculptTerrain":{"elevation_delta_m":100.0,
        "region":{"center":{"latitude":-41.0,"longitude":174.0,"altitude":null},"radius_km":5.0}}}"#;
    let (status, response) = post(port, "/api/world/interventions", sculpt).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();
    let outcome = settled(port, id).await;
    assert_eq!(outcome["state"], "refused", "{outcome}");
    let problems = outcome["problems"].to_string();
    assert!(problems.contains("SculptTerrain"), "{problems}");
    assert!(problems.contains("canon"), "{problems}");

    // And one the island can honour is honoured, with a summary of what it
    // did rather than a bare acknowledgement.
    let water = r#"{"InjectResource":{"resource_type":"Water","amount":120.0,
        "location":{"latitude":-41.0,"longitude":174.0,"altitude":null}}}"#;
    let (status, response) = post(port, "/api/world/interventions", water).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();
    let outcome = settled(port, id).await;
    assert_eq!(outcome["state"], "intervened", "{outcome}");
    let summary = outcome["summary"].as_str().expect("a summary");
    assert!(summary.contains("120"), "{summary}");

    // A body that is not an intervention at all is a bad request, not a
    // queued command nobody can account for.
    let (status, response) = post(port, "/api/world/interventions", r#"{"Nonsense":{}}"#).await;
    assert_eq!(status, 400, "{response}");

    world.stop();
    server.abort();
}

/// Pausing through the intervention endpoint actually pauses the loop.
///
/// Upstream hands `Pause` back as a directive for the host to carry out,
/// and this is the host. A directive that were reported and not acted on
/// would be a dashboard that accepted a pause and kept running, which is
/// the kind of defect only running the thing catches.
#[tokio::test]
async fn pausing_through_an_intervention_actually_stops_the_clock() {
    let (world, port, server) = a_running_dashboard().await;

    let (status, response) = post(port, "/api/world/interventions", r#""Pause""#).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
        let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if value["state"] != "queued" {
            assert_eq!(value["state"], "intervened", "{value}");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Give the loop a moment to come to rest on the pause, then check the
    // tick is still where it was a real second later. (The loop drains its
    // inbox *before* the pause gate for exactly this reason: it used to
    // gate first, which left a paused island deaf to the `Resume` queued
    // behind the pause.)
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let tick_of = |body: &str| -> u64 {
        serde_json::from_str::<serde_json::Value>(body).unwrap()["clock"]["tick"]
            .as_u64()
            .expect("a tick")
    };
    let (_, before) = get(port, "/api/world").await;
    let before = tick_of(body_of(&before));
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    let (_, after) = get(port, "/api/world").await;
    assert_eq!(
        tick_of(body_of(&after)),
        before,
        "the island kept running through a pause"
    );

    world.stop();
    server.abort();
}

/// Resuming and stepping through the queue, which a paused island used to
/// be deaf to.
///
/// The loop gated on the pause before draining its inbox, so a `Resume`
/// queued as an intervention sat behind the pause it was meant to lift and
/// the island never came back. Running the real island is what showed it;
/// this is the test that would have.
#[tokio::test]
async fn a_paused_island_still_hears_the_queue() {
    let (world, port, server) = a_running_dashboard().await;

    let settled = |port: u16, id: u64| async move {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        loop {
            let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
            let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
            if value["state"] != "queued" {
                break value;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the command never resolved — a paused island that cannot be resumed is the \
                 defect this test exists for"
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    };
    let send = |port: u16, body: &'static str| async move {
        let (status, response) = post(port, "/api/world/interventions", body).await;
        assert_eq!(status, 202, "{response}");
        serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
            .as_u64()
            .unwrap()
    };
    let tick = |port: u16| async move {
        let (_, body) = get(port, "/api/world").await;
        serde_json::from_str::<serde_json::Value>(body_of(&body)).unwrap()["clock"]["tick"]
            .as_u64()
            .expect("a tick")
    };

    let id = send(port, r#""Pause""#).await;
    assert_eq!(settled(port, id).await["state"], "intervened");

    // A creation queued while paused lands at the paused tick, rather than
    // waiting in an inbox nobody is reading.
    let (lat, lon) = estate_degrees(port).await;
    let spawn = format!(
        r#"{{"SpawnHuman":{{"template_id":"male","location":{{"latitude":{lat},"longitude":{lon},"altitude":null}},
        "profile":{{"name":"Tama","birth_timestamp":"1990-01-02T03:04:05Z","birth_latitude":{lat},
        "birth_longitude":{lon},"age_years":35.0,"height_cm":178.0,"build":"average",
        "hair_color":"black","eye_color":"brown","skin_tone":"olive"}}}}}}"#
    );
    let spawn: &'static str = Box::leak(spawn.into_boxed_str());
    let id = send(port, spawn).await;
    let outcome = settled(port, id).await;
    assert_eq!(outcome["state"], "intervened", "{outcome}");

    // And resuming through the queue works, which is the whole point.
    let id = send(port, r#""Resume""#).await;
    assert_eq!(settled(port, id).await["state"], "intervened");
    let before = tick(port).await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while tick(port).await == before {
        assert!(
            std::time::Instant::now() < deadline,
            "the island did not resume"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // `Step` advances exactly that many and then holds.
    let id = send(port, r#"{"Step":{"ticks":3}}"#).await;
    let outcome = settled(port, id).await;
    assert_eq!(outcome["state"], "intervened", "{outcome}");
    let at = outcome["tick"].as_u64().expect("a tick");
    // Let the budget be paid, then check it stays put.
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let settled_at = tick(port).await;
    assert_eq!(
        settled_at,
        at + 3,
        "a 3-step from {at} should settle at {}",
        at + 3
    );
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    assert_eq!(
        tick(port).await,
        settled_at,
        "it should hold after stepping"
    );

    world.stop();
    server.abort();
}

/// The three endpoints Task 6 names, which were recorded as "not served"
/// on the grounds that serving them would mean inventing the data.
///
/// That was wrong, and this is the test that holds the corrected claim:
/// each one serves something the island actually holds, and says plainly
/// what it does not.
#[tokio::test]
async fn a_replay_log_that_cannot_be_written_is_said_on_the_page() {
    // The timeline is served from the island's own memory, so it reads the
    // same whether or not the log reached disk. Until this, a failed write
    // went to stderr and nowhere else: the page went on showing commands
    // that were not recorded anywhere, and said nothing. A full disk is not
    // hypothetical — it happened twice while this branch was written.
    use island::serve::sim::{spawn_with, SimSpeed};
    use std::path::PathBuf;

    let repo = |path: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    let mut scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    // A small island: 240 x 192 medium cells against the default's
    // 1,200 x 960, 25x fewer. The grids are what load this suite, not the
    // trees -- `tree_cap` shrinks the vegetation and leaves two
    // 1,152,000-cell grids to build on every bootstrap and hash on every
    // digest, a dozen times over, on four cores.
    //
    // The seed changes with the profile because it has to. `IslandLife`
    // validates the coastline it generates against the profile's shape
    // rules, and the default scenario's seed makes a small island with
    // "0 major headlands, need 3". Seed 16 is simply the first that does
    // not: found by trying 0, 1, 2 ... through
    // `RegionalPhysicalState::bootstrap`, which runs that check before the
    // expensive spin-up, so bad seeds cost nothing. It took 17 tries and
    // 1.3 seconds.
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut small_seed = [0u8; 32];
    small_seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = small_seed;
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();

    // A directory that does not exist, so every save fails for a reason
    // the operating system supplies rather than one this test invents.
    let unwritable = PathBuf::from("/nonexistent-by-design/replay.json");
    // Paced rather than flat out, unlike its neighbours. This test needs
    // one command applied, not accumulated ticks, and every test in this
    // file holds an island on its own sim thread: one more spinning a core
    // at `AsFastAsPossible` is what pushed `a_new_person_can_be_read_the_
    // moment_they_exist` past its deadline when the whole workspace runs
    // at once. At `Times(600)` the loop sleeps between 60-second steps,
    // waking ten times a second -- far inside the deadline below, and
    // costing almost nothing while it waits.
    let world = spawn_with(life, SimSpeed::Times(600), Some(unwritable), None);

    // Nothing has been applied yet, so nothing has been written: no error.
    assert_eq!(world.replay_log_error(), None);

    // A command makes the loop try to save, and fail.
    world.send(mk_engine::regional::commands::IslandCommand::Control(
        mk_engine::regional::commands::ControlCommand::Pause,
    ));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let said = loop {
        if let Some(said) = world.replay_log_error() {
            break said;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "a failing replay log was never reported"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    assert!(
        said.contains("replay.json"),
        "the message names the path it could not write: {said}"
    );
    // Shown as text, so it must read as prose rather than markdown.
    assert!(!said.contains('`'), "shown on the page as text: {said}");

    world.stop();
}

#[tokio::test]
async fn a_cell_of_the_island_can_be_asked_about_by_row_and_column() {
    let (world, port, server) = a_running_dashboard().await;

    // The estate's cell, read two independent ways. `/api/world` derives
    // the estate's coordinates from its position in metres; `/api/cell`
    // derives them from the grid. If they disagree, one of them is doing
    // the radians-to-degrees conversion differently -- which is exactly
    // the bug that put a birth 0.716 degrees from the equator.
    let (status, response) = get(port, "/api/world").await;
    assert_eq!(status, 200, "{response}");
    let world_json: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let estate = &world_json["estate"];
    let (row, col) = (
        estate["cell"][0].as_u64().expect("an estate row"),
        estate["cell"][1].as_u64().expect("an estate column"),
    );

    let (status, response) = get(port, &format!("/api/cell/{row}/{col}")).await;
    assert_eq!(status, 200, "{response}");
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let cell = &body["cell"];

    for (name, from_world, from_cell) in [
        ("latitude", &estate["latitude"], &cell["latitude"]),
        ("longitude", &estate["longitude"], &cell["longitude"]),
    ] {
        let (a, b) = (
            from_world.as_f64().expect("a number"),
            from_cell.as_f64().expect("a number"),
        );
        // Within half a cell: the estate sits somewhere inside its cell and
        // the cell answers for its centre, so they need not be identical --
        // but they must be the same place, not the same number of radians.
        assert!(
            (a - b).abs() < 0.05,
            "{name} disagrees between /api/world ({a}) and /api/cell ({b})"
        );
    }

    // The estate is on land and buildable; that is where the founders live.
    assert_eq!(cell["land"], true, "{cell}");
    assert_eq!(cell["buildable"], true, "{cell}");

    // Off the grid is a 404 with a reason, not a panic or a zeroed cell.
    let (status, response) = get(port, "/api/cell/99999/99999").await;
    assert_eq!(status, 404, "{response}");
    assert!(
        body_of(&response).contains("medium grid"),
        "says why: {}",
        body_of(&response)
    );

    world.stop();
    server.abort();
}

#[tokio::test]
async fn the_islands_individual_trees_can_be_asked_for_by_the_box() {
    let (world, port, server) = a_running_dashboard().await;

    // Everything there is. The default box is the whole domain, so this
    // is the island's entire stock of individual stems.
    let (status, response) = get(port, "/api/trees").await;
    assert_eq!(status, 200, "{response}");
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let all = &body["trees"];
    let total = all["total"].as_u64().expect("a total");
    assert!(total > 0, "this island has individual stems: {all}");
    assert_eq!(
        all["in_box"].as_u64(),
        Some(total),
        "the default box is the whole domain, so everything is in it: {all}"
    );
    assert_eq!(
        all["shown"].as_u64(),
        Some(total),
        "fewer stems than the cap, so nothing is thinned: {all}"
    );

    // The note is the endpoint's own account of what it is not: a reader
    // who takes an empty box for bare ground has been misled by this page.
    let note = body["note"].as_str().expect("a note");
    assert!(
        note.contains("individual_radius_m") && note.contains("stand cover"),
        "says where individual stems exist and what is everywhere else: {note}"
    );

    // Each stem is a real measurement, not a placeholder. `kind` is the
    // engine's `PlantKind` -- Tree, Shrub or Grass -- and deliberately not
    // a species: the island's vegetation carries none, and a page claiming
    // one would be fabricating exactly what `HUMAN_SCOPE.md` forbids.
    let stems = all["trees"].as_array().expect("stems");
    for stem in stems {
        let kind = stem["kind"].as_str().expect("a kind");
        assert!(
            matches!(kind, "Tree" | "Shrub" | "Grass"),
            "the engine's whole plant vocabulary, and not a species: {kind}"
        );
        assert!(
            stem["height_m"].as_f64().expect("a height") > 0.0,
            "a stem has a height: {stem}"
        );
        assert!(
            stem["stem_diameter_m"].as_f64().expect("a diameter") > 0.0,
            "a stem has a trunk: {stem}"
        );
    }

    // Nothing stands in the estate's yard. The viewer draws that hole and
    // says what it is, so if the engine ever started planting there the
    // page would be explaining a clearing that was not one.
    let yard = all["yard"].as_array().expect("a yard");
    let at = |i: usize| yard[i].as_f64().expect("a yard edge");
    let (w, s, e, n) = (at(0), at(1), at(2), at(3));
    assert!(
        e > w && n > s,
        "the yard is a rectangle with area: {yard:?}"
    );
    for stem in stems {
        let (x, y) = (
            stem["x_m"].as_f64().expect("an x"),
            stem["y_m"].as_f64().expect("a y"),
        );
        assert!(
            x < w || x > e || y < s || y > n,
            "a stem stands in the estate's cleared yard: {stem}"
        );
    }

    // Asked for fewer than there are, the answer is thinned -- and says
    // so. `in_box` is the count before thinning, which is the number that
    // stops a sampled wood being read as a thin one.
    let want = (total / 2).max(1);
    let (status, response) = get(port, &format!("/api/trees?cap={want}")).await;
    assert_eq!(status, 200, "{response}");
    let thinned: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let thinned = &thinned["trees"];
    assert_eq!(
        thinned["in_box"].as_u64(),
        Some(total),
        "thinning changes what is sent, never what is reported to be there: {thinned}"
    );
    let shown = thinned["shown"].as_u64().expect("a count");
    assert!(
        shown <= want && shown > 0,
        "asked for at most {want} and got {shown}: {thinned}"
    );

    // The same box answers the same way twice. The thinning is a hash of
    // each stem's id and not a sample of the list, so it has to be a pure
    // function of the request -- two people looking at one wood see one
    // wood.
    let (_, again) = get(port, &format!("/api/trees?cap={want}")).await;
    let again: serde_json::Value = serde_json::from_str(body_of(&again)).unwrap();
    assert_eq!(
        again["trees"]["trees"], thinned["trees"],
        "the same box thinned differently on a second request"
    );

    // A box with nothing in it is an empty list and an honest zero, not a
    // 404 and not the whole island.
    let (status, response) = get(port, "/api/trees?x0=-9000&y0=-9000&x1=-8000&y1=-8000").await;
    assert_eq!(status, 200, "{response}");
    let empty: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(empty["trees"]["in_box"].as_u64(), Some(0), "{empty}");
    assert_eq!(empty["trees"]["shown"].as_u64(), Some(0), "{empty}");
    assert_eq!(
        empty["trees"]["total"].as_u64(),
        Some(total),
        "an empty box does not mean an empty island: {empty}"
    );

    world.stop();
    server.abort();
}

#[tokio::test]
async fn the_islands_vegetation_is_served_at_two_resolutions() {
    let (world, port, server) = a_running_dashboard().await;

    // The island's standing vegetation, one pixel per medium cell, the
    // same shape as the elevation render it sits beside. The dashboard
    // draws one over the other, so a different shape would misplace every
    // cell on the map.
    let (status, head, island) = get_bytes(port, "/api/vegetation.png").await;
    assert_eq!(status, 200, "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains("content-type: image/png"),
        "served as a PNG: {}",
        head.lines().take(8).collect::<Vec<_>>().join(" | ")
    );
    // Not cached. This layer is redrawn as biomass grows, and a browser
    // holding the first one would show the forest the island bootstrapped
    // with for ever -- which is the one way a growing world can look
    // static and nobody notices.
    assert!(
        head.to_ascii_lowercase()
            .contains("cache-control: no-cache"),
        "the vegetation changes, so it must be revalidated: {}",
        head.lines().take(8).collect::<Vec<_>>().join(" | ")
    );

    let (_, _, elevation) = get_bytes(port, "/api/map.png").await;
    assert_eq!(
        png_size(&island),
        png_size(&elevation),
        "the vegetation layer is drawn over the elevation one, so they are the same grid"
    );

    // The estate's patch, at its own far finer resolution. This is the
    // middle rung of the zoom: 2 km cells across the island, this over
    // the estate, individual stems in the wood.
    let (status, head, patch) = get_bytes(port, "/api/patch.png").await;
    assert_eq!(status, 200, "{head}");
    let (pw, ph) = png_size(&patch);
    let (iw, _) = png_size(&island);
    assert!(
        pw > iw,
        "the patch covers 4 km in {pw} pixels against the island's {iw} for its whole width, so it \
         must be the finer picture or it is not worth serving"
    );
    assert_eq!(pw, ph, "the patch is square: {pw} by {ph}");

    world.stop();
    server.abort();
}

/// Width and height out of a PNG's IHDR, which is always the first chunk.
fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert!(
        bytes.len() > 24 && bytes.starts_with(&[0x89, b'P', b'N', b'G']),
        "not a PNG: {} bytes",
        bytes.len()
    );
    let be = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    (be(16), be(20))
}

#[tokio::test]
async fn the_island_is_served_as_a_picture_of_itself() {
    let (world, port, server) = a_running_dashboard().await;

    let (status, head, body) = get_bytes(port, "/api/map.png").await;
    assert_eq!(status, 200, "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains("content-type: image/png"),
        "served as a PNG, since a browser decides what to do by the type: {}",
        head.lines().take(8).collect::<Vec<_>>().join(" | ")
    );

    // A real PNG of this island, not an empty body with a hopeful header.
    let body = body.as_slice();
    assert!(
        body.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
        "the body begins with the PNG signature"
    );
    // The default test island is 240 x 192 medium cells, and the renderer
    // paints a pixel per cell, so a few hundred bytes would mean a blank.
    assert!(
        body.len() > 2_000,
        "a drawn island, not a blank one: {} bytes",
        body.len()
    );

    world.stop();
    server.abort();
}

#[tokio::test]
async fn properties_economy_and_timeline_serve_what_the_island_really_holds() {
    let (world, port, server) = a_running_dashboard().await;

    // Properties: the founders' estate, with its buildings and its things.
    let (status, response) = get(port, "/api/properties").await;
    assert_eq!(status, 200, "{response}");
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let properties = body["properties"].as_array().expect("a property list");
    assert!(!properties.is_empty(), "{body}");
    let estate = properties
        .iter()
        .find(|p| p["on_this_island"] == true)
        .expect("the founders' estate stands on this island");
    assert!(
        estate["owners"]
            .as_array()
            .is_some_and(|o| o.iter().any(|x| x == "Gem-D")),
        "{estate}"
    );
    assert!(
        estate["buildings"]
            .as_array()
            .is_some_and(|b| !b.is_empty()),
        "an estate with no buildings is not an estate: {estate}"
    );
    assert!(
        estate["items"].as_array().is_some_and(|i| !i.is_empty()),
        "{estate}"
    );

    // Economy: empty, and saying why rather than looking broken.
    let (status, response) = get(port, "/api/economy").await;
    assert_eq!(status, 200, "{response}");
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(body["resource_nodes"], 0);
    assert_eq!(body["structures"].as_array().map(|s| s.len()), Some(0));
    let note = body["note"].as_str().expect("a note");
    assert!(
        note.contains("seeds no resource nodes"),
        "an empty economy has to say it is empty on purpose: {body}"
    );
    // The dashboard puts these on the page with `textContent`, like
    // everything from the server, so a backtick would be a backtick on
    // screen rather than code formatting.
    assert!(
        !note.contains('`'),
        "a note read by a person carries no markup: {note}"
    );

    // Timeline: nothing has reached this island yet.
    let (status, response) = get(port, "/api/timeline").await;
    assert_eq!(status, 200, "{response}");
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(body["entries"].as_array().map(|e| e.len()), Some(0));

    // Now build something and look again. Both views have to move, and
    // promptly — not at the next hourly refresh.
    let (lat, lon) = estate_degrees(port).await;
    let shelter = format!(
        r#"{{"ConstructStructure":{{"structure":"WoodenShelter",
        "location":{{"latitude":{lat},"longitude":{lon},"altitude":null}}}}}}"#
    );
    let shelter = shelter.as_str();
    let (status, response) = post(port, "/api/world/interventions", shelter).await;
    assert_eq!(status, 202, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
        let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if value["state"] != "queued" {
            assert_eq!(value["state"], "intervened", "{value}");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let (_, response) = get(port, "/api/economy").await;
        let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if body["structures"].as_array().is_some_and(|s| s.len() == 1) {
            let built = &body["structures"][0];
            // The display name, not the enum's `{:?}`: this is read by a
            // person, and `StructureKind::display_name` exists for it.
            assert_eq!(built["recipe"], "Wooden Shelter", "{body}");
            // And the economy's own record of it names it the same way, so
            // the two tables on the page do not disagree about one thing.
            let recorded = body["events"]
                .as_array()
                .and_then(|e| e.first())
                .expect("the economy recorded the construction");
            assert_eq!(recorded["subject"], "Wooden Shelter", "{body}");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "a structure that was built should be in the economy"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Conversations, on this same island rather than another.
    //
    // This lives here, and not in a test of its own, deliberately. Every
    // test in this file bootstraps a full island and spins a sim thread at
    // AsFastAsPossible, and they run in parallel: a nineteenth was enough
    // to starve the others and push `a_new_person_can_be_read_the_moment_
    // they_exist` past its sixty-second deadline. Measured — the suite is
    // 18 passed / 0 failed without that extra island and 18/1 with it.
    // Folding these assertions into a dashboard that already exists keeps
    // the island count where it was.
    // Polled, not asserted on the first read: this view refreshes on the
    // island's cadence rather than when a command lands, so a test that
    // reads it once is racing the loop. The economy assertions above wait
    // the same way and for the same reason.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let body = loop {
        let (status, response) = get(port, "/api/conversations").await;
        assert_eq!(status, 200, "{response}");
        let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if !body["conversations"]
            .as_array()
            .expect("a conversation list")
            .is_empty()
        {
            break body;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the islanders never said anything: {body}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };

    // The note is put on the page with `textContent`, so markdown in it
    // renders as punctuation. This branch shipped that bug once already.
    let note = body["note"].as_str().expect("a note");
    assert!(!note.contains('`'), "the note is shown as text: {note}");
    assert!(
        note.contains("No language model"),
        "a reader must be told what composed these lines: {note}"
    );

    let conversations = body["conversations"]
        .as_array()
        .expect("a conversation list");
    assert!(
        conversations.len() <= body["shown"].as_u64().unwrap() as usize,
        "no more than the cap it declares: {body}"
    );

    let first = &conversations[0];
    for line in first["lines"].as_array().expect("lines") {
        assert!(
            !line["text"].as_str().expect("text").trim().is_empty(),
            "every line says something: {line}"
        );
        assert!(
            !line["speaker_name"]
                .as_str()
                .expect("a speaker")
                .trim()
                .is_empty(),
            "every line names who said it: {line}"
        );
    }
    // Words, not an enum name: the page prints this.
    let relationship = first["relationship"].as_str().expect("a relationship");
    assert!(
        ["founders", "parent and child", "siblings", "neighbours"].contains(&relationship),
        "unexpected relationship wording: {relationship}"
    );
    // Newest first, so a reader sees what was just said.
    let ticks: Vec<u64> = conversations
        .iter()
        .map(|c| c["tick"].as_u64().expect("a tick"))
        .collect();
    assert!(
        ticks.windows(2).all(|w| w[0] >= w[1]),
        "newest first: {ticks:?}"
    );

    let (_, response) = get(port, "/api/timeline").await;
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    let entries = body["entries"].as_array().expect("a timeline");
    assert_eq!(entries.len(), 1, "{body}");
    assert_eq!(entries[0]["what"], "ConstructStructure", "{body}");

    world.stop();
    server.abort();
}

/// Without an island there is nothing to serve, and these say so rather
/// than returning an empty shape that reads as "there is nothing here".
#[tokio::test]
async fn the_three_views_need_an_island() {
    let data_dir = tempfile::tempdir().unwrap();
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [7u8; 32]).unwrap();
    let population = Arc::new(Mutex::new(population));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes(population, ControlAuth::LoopbackOnly);
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });

    for path in ["/api/properties", "/api/economy", "/api/timeline"] {
        let (status, response) = get(port, path).await;
        assert_eq!(status, 404, "{path}: {response}");
        assert!(
            response.contains("No island is running"),
            "{path}: {response}"
        );
    }
    server.abort();
}

/// `ControlCommand::Snapshot` was in the vocabulary, the timeline rendered
/// it as "wrote a snapshot", and nothing anywhere wrote one — the page
/// would have said a thing that had not happened. These hold both halves:
/// it is refused when there is nowhere to write, and when there is
/// somewhere, a file actually appears.
#[tokio::test]
async fn asking_for_a_snapshot_with_nowhere_to_put_it_is_refused() {
    let (world, port, server) = a_running_dashboard().await;

    let (status, response) = post(port, "/api/control", r#"{"command":"snapshot"}"#).await;
    assert_eq!(status, 200, "{response}");
    let id = serde_json::from_str::<serde_json::Value>(body_of(&response)).unwrap()["command"]
        .as_u64()
        .expect("a command id, so the outcome can be read");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let outcome = loop {
        let (_, response) = get(port, &format!("/api/world/commands/{id}")).await;
        let value: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
        if value["state"] != "queued" {
            break value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the command never resolved"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    assert_eq!(outcome["state"], "refused", "{outcome}");
    assert!(
        outcome["problems"].to_string().contains("--snapshot-dir"),
        "the refusal should say what is missing: {outcome}"
    );

    // And nothing claimed otherwise in the record of what was done.
    let (_, response) = get(port, "/api/timeline").await;
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    assert!(
        !body["entries"].to_string().contains("wrote a snapshot"),
        "a refused snapshot must not be recorded as written: {body}"
    );

    world.stop();
    server.abort();
}

/// As [`get_bytes`], with one extra request header.
async fn get_bytes_with(port: u16, path: &str, header: &str) -> (u16, String, Vec<u8>) {
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the dashboard is listening");
    let (mut reader, mut writer) = stream.into_split();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nAccept: */*\r\n\
         {header}\r\n\r\n"
    );
    {
        use tokio::io::AsyncWriteExt;
        writer.write_all(request.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }
    let mut response = Vec::new();
    {
        use tokio::io::AsyncReadExt;
        reader.read_to_end(&mut response).await.unwrap();
    }
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a header/body boundary");
    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .expect("a status line");
    (status, head, response[split + 4..].to_vec())
}

#[tokio::test]
async fn the_terrain_is_served_as_numbers_a_renderer_can_use() {
    // A desktop client builds a mesh out of the island. It cannot do that
    // from `/api/map.png`: a colour ramp is not a height. This is the same
    // grid as numbers, and this test reads it back the way that client
    // will -- including against the island's own `/api/world`, so a
    // mismatch between the two is caught here rather than as terrain that
    // is subtly the wrong size.
    let (world, port, server) = a_running_dashboard().await;

    let (status, head, body) = get_bytes(port, "/api/elevation.bin").await;
    assert_eq!(status, 200, "{head}");
    assert!(
        head.to_ascii_lowercase()
            .contains("content-type: application/octet-stream"),
        "{head}"
    );
    assert!(
        head.to_ascii_lowercase().contains("immutable"),
        "terrain cannot change while the process lives, and should say so: {head}"
    );

    assert_eq!(&body[0..4], b"MKIE", "the magic a client checks");
    let format = u16::from_le_bytes(body[4..6].try_into().unwrap());
    assert_eq!(format, island::serve::projection::ELEVATION_FORMAT);
    let rows = u32::from_le_bytes(body[6..10].try_into().unwrap()) as usize;
    let cols = u32::from_le_bytes(body[10..14].try_into().unwrap()) as usize;
    let cell_size_m = f32::from_le_bytes(body[14..18].try_into().unwrap());

    // The shape has to agree with what `/api/world` tells the same client
    // about the same island, or it will draw the right numbers at the
    // wrong scale.
    let (_, world_body) = get(port, "/api/world").await;
    let published: serde_json::Value = serde_json::from_str(body_of(&world_body)).unwrap();
    assert_eq!(rows, published["land"]["rows"].as_u64().unwrap() as usize);
    assert_eq!(cols, published["land"]["cols"].as_u64().unwrap() as usize);
    assert_eq!(
        f64::from(cell_size_m),
        published["land"]["cell_size_m"].as_f64().unwrap()
    );

    let header = island::serve::projection::ELEVATION_HEADER_BYTES;
    assert_eq!(
        body.len(),
        header + rows * cols * 5,
        "{rows}x{cols}: four bytes of elevation and one of mask per cell"
    );

    // The numbers themselves: every land cell's elevation, and the count
    // of land cells, have to match what the island says they are.
    let mut land = 0usize;
    let mut highest = f32::MIN;
    for i in 0..rows * cols {
        let at = header + i * 4;
        let elevation = f32::from_le_bytes(body[at..at + 4].try_into().unwrap());
        assert!(elevation.is_finite(), "cell {i} is {elevation}");
        let is_land = body[header + rows * cols * 4 + i] == 1;
        if is_land {
            land += 1;
            highest = highest.max(elevation);
        }
    }
    assert_eq!(
        land,
        published["land"]["land_cells"].as_u64().unwrap() as usize,
        "the mask and the island disagree about how much of it is land"
    );
    assert!(highest > 0.0, "no land cell is above sea level");

    // And one cell read both ways. `/api/cell` answers from the same
    // grids; if these disagree, one of the two is lying about the island.
    let (_, cell_body) = get(port, &format!("/api/cell/{}/{}", rows / 2, cols / 2)).await;
    // `/api/cell` wraps its answer in a `cell` object.
    let cell =
        serde_json::from_str::<serde_json::Value>(body_of(&cell_body)).unwrap()["cell"].clone();
    let i = (rows / 2) * cols + cols / 2;
    let at = header + i * 4;
    let elevation = f32::from_le_bytes(body[at..at + 4].try_into().unwrap());
    assert!(
        (f64::from(elevation) - cell["elevation_m"].as_f64().unwrap()).abs() < 0.5,
        "the binary grid says {elevation} and /api/cell says {}",
        cell["elevation_m"]
    );
    assert_eq!(
        body[header + rows * cols * 4 + i] == 1,
        cell["land"].as_bool().unwrap()
    );

    // Gzip when it is offered, and the same bytes underneath.
    let (status, head, gz) =
        get_bytes_with(port, "/api/elevation.bin", "Accept-Encoding: gzip").await;
    assert_eq!(status, 200, "{head}");
    assert!(
        head.to_ascii_lowercase().contains("content-encoding: gzip"),
        "{head}"
    );
    assert!(
        gz.len() < body.len(),
        "gzip made it bigger: {} against {}",
        gz.len(),
        body.len()
    );
    let mut inflated = Vec::new();
    {
        use std::io::Read;
        flate2::read::GzDecoder::new(&gz[..])
            .read_to_end(&mut inflated)
            .expect("the gzip stream is well formed");
    }
    assert_eq!(inflated, body, "the two encodings are not the same island");

    world.stop();
    server.abort();
}

#[tokio::test]
async fn a_client_is_told_what_this_backend_can_do_before_it_asks_for_anything() {
    let (world, port, server) = a_running_dashboard().await;
    let (status, response) = get(port, "/api/version").await;
    assert_eq!(status, 200);
    let version: mk_island_api::ServerVersion =
        serde_json::from_str(body_of(&response)).expect("the shared schema reads it");
    assert!(version.compatibility(mk_island_api::API_VERSION).is_ok());
    assert!(version.capabilities.world, "an island is running");
    assert!(version.capabilities.elevation);
    assert!(!version.capabilities.reads_need_token);
    assert!(
        !version.capabilities.snapshots,
        "this fixture was started with no snapshot directory, and saying it \
         could write one would send an operator to a button that refuses"
    );
    world.stop();
    server.abort();
}
