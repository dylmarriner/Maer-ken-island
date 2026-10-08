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
