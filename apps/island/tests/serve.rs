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
