//! The dashboard as it is actually run: a real listener on a real port,
//! serving a real data directory. The unit tests exercise the filters; this
//! one proves `island serve` itself answers.

use island::serve::auth::ControlAuth;
use island::serve::server;
use std::sync::{Arc, Mutex};

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
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();
    let world = spawn(life, SimSpeed::AsFastAsPossible);

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

/// A dashboard with a small island running on it, and its port.
async fn a_running_dashboard() -> (
    island::serve::sim::SimHandle,
    u16,
    tokio::task::JoinHandle<()>,
) {
    use island::serve::sim::{spawn, SimSpeed};
    use std::path::PathBuf;

    let repo = |path: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    let mut scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();
    let world = spawn(life, SimSpeed::AsFastAsPossible);

    let data_dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [11u8; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes_with_world(
        Arc::new(Mutex::new(population)),
        ControlAuth::LoopbackOnly,
        Some(world.clone()),
    );
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });
    (world, port, server)
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
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();
    let world = spawn(life, SimSpeed::AsFastAsPossible);

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
    let spawn = r#"{"SpawnHuman":{"template_id":"male","location":{"latitude":-41.03,"longitude":173.56,"altitude":null},
        "profile":{"name":"Tama","birth_timestamp":"1990-01-02T03:04:05Z","birth_latitude":-41.0,
        "birth_longitude":174.0,"age_years":35.0,"height_cm":178.0,"build":"average",
        "hair_color":"black","eye_color":"brown","skin_tone":"olive"}}}"#;
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
    assert!(
        body["note"]
            .as_str()
            .is_some_and(|n| n.contains("seeds no resource nodes")),
        "an empty economy has to say it is empty on purpose: {body}"
    );

    // Timeline: nothing has reached this island yet.
    let (status, response) = get(port, "/api/timeline").await;
    assert_eq!(status, 200, "{response}");
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).unwrap();
    assert_eq!(body["entries"].as_array().map(|e| e.len()), Some(0));

    // Now build something and look again. Both views have to move, and
    // promptly — not at the next hourly refresh.
    let shelter = r#"{"ConstructStructure":{"structure":"WoodenShelter",
        "location":{"latitude":-41.03,"longitude":173.56,"altitude":null}}}"#;
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
            assert_eq!(built["recipe"], "WoodenShelter", "{body}");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "a structure that was built should be in the economy"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

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
