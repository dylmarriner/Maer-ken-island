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
