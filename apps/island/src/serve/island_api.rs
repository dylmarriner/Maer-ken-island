//! HTTP API over the live island. Reads come from the immutable projection
//! (or a detail query to the sim thread); the writes queue commands for the
//! next tick and answer `202` with the command's sequence number.

use super::auth::ControlAuth;
use super::island_sim::{SimError, SimHandle};
use mk_engine::regional::commands::{
    CommandError, ControlCommand, IslandCommand, IslandCreateHumanRequest,
};
use warp::http::StatusCode;
use warp::Filter;

const PAGE: &str = include_str!("island_page.html");

fn json(
    status: StatusCode,
    body: &serde_json::Value,
) -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(warp::reply::json(body), status)
}

fn with<T: Clone + Send>(
    value: T,
) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || value.clone())
}

fn refusal(error: SimError) -> (StatusCode, serde_json::Value) {
    match error {
        SimError::Command(CommandError::DuplicateHuman(id)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({ "errors": [format!("{id} already exists or is already queued")] }),
        ),
        SimError::Command(CommandError::InvalidRequest(why)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({ "errors": [why] }),
        ),
        SimError::Command(CommandError::NotSupportedOnIsland(what)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({ "errors": [format!("not supported on the island: {what}")] }),
        ),
        SimError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "errors": ["the simulation is not answering"] }),
        ),
    }
}

/// Run a blocking call to the sim thread off the async executor.
async fn ask<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, SimError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| SimError::Unavailable)
}

async fn write(
    sim: SimHandle,
    auth: ControlAuth,
    authorization: Option<String>,
    command: IslandCommand,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, std::convert::Infallible> {
    if !auth.permits(authorization.as_deref()) {
        return Ok(json(
            StatusCode::UNAUTHORIZED,
            &serde_json::json!({ "errors": ["unauthorised"], "writes": auth.describe() }),
        ));
    }
    let outcome = match ask(move || sim.command(command)).await {
        Ok(r) => r,
        Err(e) => Err(e),
    };
    Ok(match outcome {
        Ok(id) => json(
            StatusCode::ACCEPTED,
            &serde_json::json!({ "command_id": id, "note": "queued for the next tick" }),
        ),
        Err(e) => {
            let (code, body) = refusal(e);
            json(code, &body)
        }
    })
}

pub fn routes(
    sim: SimHandle,
    auth: ControlAuth,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let status = warp::path!("api" / "status")
        .and(warp::get())
        .and(with(sim.clone()))
        .and(with(auth.clone()))
        .map(|sim: SimHandle, auth: ControlAuth| {
            let mut body = sim.projection().status();
            body["writes"] = serde_json::json!(auth.describe());
            json(StatusCode::OK, &body)
        });

    let roster = warp::path!("api" / "humans")
        .and(warp::get())
        .and(with(sim.clone()))
        .map(|sim: SimHandle| json(StatusCode::OK, &serde_json::json!(sim.projection().roster)));

    let one = warp::path!("api" / "humans" / String)
        .and(warp::get())
        .and(with(sim.clone()))
        .and_then(|id: String, sim: SimHandle| async move {
            let found = ask(move || sim.detail(&id)).await.and_then(|r| r);
            Ok::<_, std::convert::Infallible>(match found {
                Ok(Some(body)) => json(StatusCode::OK, &body),
                Ok(None) => json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": ["no such human"] }),
                ),
                Err(e) => {
                    let (code, body) = refusal(e);
                    json(code, &body)
                }
            })
        });

    let views = {
        let view =
            |name: &'static str, sim: SimHandle, pick: fn(&SimHandle) -> serde_json::Value| {
                warp::path!("api" / String)
                    .and(warp::get())
                    .and(warp::any().map(move || sim.clone()))
                    .and_then(move |path: String, sim: SimHandle| async move {
                        if path != name {
                            return Err(warp::reject::not_found());
                        }
                        Ok(json(StatusCode::OK, &pick(&sim)))
                    })
            };
        view("estate", sim.clone(), |s| (*s.estate()).clone())
            .or(view("vegetation", sim.clone(), |s| {
                s.projection().vegetation.clone()
            }))
            .or(view("economy", sim.clone(), |s| {
                s.projection().economy.clone()
            }))
            .or(view("timeline", sim.clone(), |s| {
                serde_json::json!(s.projection().timeline)
            }))
    };

    let create = warp::path!("api" / "humans")
        .and(warp::post())
        .and(with(sim.clone()))
        .and(with(auth.clone()))
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::body::json::<IslandCreateHumanRequest>())
        .and_then(
            |sim: SimHandle,
             auth: ControlAuth,
             authorization: Option<String>,
             r: IslandCreateHumanRequest| {
                write(sim, auth, authorization, IslandCommand::CreateHuman(r))
            },
        );

    let control = warp::path!("api" / "control")
        .and(warp::post())
        .and(with(sim))
        .and(with(auth))
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(4 * 1024))
        .and(warp::body::json::<ControlCommand>())
        .and_then(
            |sim: SimHandle,
             auth: ControlAuth,
             authorization: Option<String>,
             c: ControlCommand| {
                write(sim, auth, authorization, IslandCommand::Control(c))
            },
        );

    let page = warp::path::end()
        .and(warp::get())
        .map(|| warp::reply::with_header(PAGE, "content-type", "text/html; charset=utf-8"));

    status
        .or(one)
        .or(roster)
        .or(views)
        .or(create)
        .or(control)
        .or(page)
}

/// Serve until Ctrl-C.
pub async fn run(
    sim: SimHandle,
    auth: ControlAuth,
    bind: std::net::SocketAddr,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    warp::serve(routes(sim.clone(), auth))
        .incoming(listener)
        .graceful(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .run()
        .await;
    sim.stop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::island_sim::spawn;
    use mk_core::canon::CanonLocked;
    use mk_engine::regional::world::IslandWorldState;
    use mk_island::IslandScenario;
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    fn sim() -> SimHandle {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let scenario =
            IslandScenario::load(&root.join("fixtures/island/default_scenario.json")).unwrap();
        let canon = Arc::new(CanonLocked::load(&root.join("fixtures/island/canon.json")).unwrap());
        spawn(IslandWorldState::new(canon, scenario).unwrap(), None)
    }

    fn person(name: &str) -> serde_json::Value {
        serde_json::json!({ "agent_id": name, "sex": "Female", "age_years": 31.0 })
    }

    async fn get(
        filter: &(impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone + 'static),
        path: &str,
    ) -> (u16, serde_json::Value) {
        let r = warp::test::request().path(path).reply(filter).await;
        (
            r.status().as_u16(),
            serde_json::from_slice(r.body()).unwrap_or_default(),
        )
    }

    #[tokio::test]
    async fn writes_need_the_token_and_a_created_person_appears_once_time_runs() {
        let handle = sim();
        let filter = routes(
            handle.clone(),
            ControlAuth::resolve(Some("s3cret".into()), LOOPBACK),
        );

        let (code, status) = get(&filter, "/api/status").await;
        assert_eq!(code, 200);
        assert_eq!(status["population"], 2);

        let refused = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .json(&person("Visitor-1"))
            .reply(&filter)
            .await;
        assert_eq!(refused.status(), 401);

        let fast = warp::test::request()
            .method("POST")
            .path("/api/control")
            .header("authorization", "Bearer s3cret")
            .json(&"AsFastAsPossible".to_string())
            .reply(&filter)
            .await;
        // A bare string is not a ControlCommand: the speed command is an object.
        assert_eq!(fast.status(), 400);
        let fast = warp::test::request()
            .method("POST")
            .path("/api/control")
            .header("authorization", "Bearer s3cret")
            .json(&serde_json::json!({ "SetSpeed": "AsFastAsPossible" }))
            .reply(&filter)
            .await;
        assert_eq!(fast.status(), 202, "{:?}", fast.body());

        let created = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("authorization", "Bearer s3cret")
            .json(&person("Visitor-1"))
            .reply(&filter)
            .await;
        assert_eq!(created.status(), 202, "queued, not created yet");

        let started = Instant::now();
        loop {
            let (_, roster) = get(&filter, "/api/humans").await;
            if roster
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["agent_id"] == "Visitor-1")
            {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(90),
                "the visitor never appeared"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let (code, detail) = get(&filter, "/api/humans/Visitor-1").await;
        assert_eq!(code, 200);
        assert!(detail.is_object());
        assert_eq!(get(&filter, "/api/humans/nobody").await.0, 404);

        let duplicate = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("authorization", "Bearer s3cret")
            .json(&person("Visitor-1"))
            .reply(&filter)
            .await;
        assert_eq!(duplicate.status(), 422);
        let bad_speed = warp::test::request()
            .method("POST")
            .path("/api/control")
            .header("authorization", "Bearer s3cret")
            .json(&serde_json::json!({ "SetSpeed": { "Times": 7 } }))
            .reply(&filter)
            .await;
        assert_eq!(bad_speed.status(), 422);

        // The estate view feeds the location picker; read views need no token.
        let (code, estate) = get(&filter, "/api/estate").await;
        assert_eq!(code, 200);
        let spaces = estate["spaces"].as_array().unwrap();
        assert!(spaces.iter().any(|s| s["label"] == "Kitchen"), "{estate}");
        for view in ["/api/vegetation", "/api/economy", "/api/timeline"] {
            assert_eq!(get(&filter, view).await.0, 200, "{view}");
        }
        assert_eq!(get(&filter, "/api/nonsense").await.0, 404);

        // An authored person placed in the kitchen; a bad height is a 422.
        let authored = |name: &str, height: f64| {
            serde_json::json!({
                "agent_id": name, "sex": "Male", "age_years": 44.0,
                "appearance": { "height_cm": height, "build": "Heavy", "hair_color": "Gray",
                                "eye_color": "Green", "skin_tone": "Dark" },
                "location": { "EstateSpace": { "label": "Kitchen" } }
            })
        };
        let bad = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("authorization", "Bearer s3cret")
            .json(&authored("Cook", 5.0))
            .reply(&filter)
            .await;
        assert_eq!(bad.status(), 422);
        assert!(String::from_utf8_lossy(bad.body()).contains("height"));
        let good = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("authorization", "Bearer s3cret")
            .json(&authored("Cook", 182.0))
            .reply(&filter)
            .await;
        assert_eq!(good.status(), 202, "{:?}", good.body());
        let started = Instant::now();
        loop {
            let (_, roster) = get(&filter, "/api/humans").await;
            if let Some(cook) = roster
                .as_array()
                .unwrap()
                .iter()
                .find(|h| h["agent_id"] == "Cook")
            {
                assert!(
                    cook["space"].as_str().unwrap().starts_with("Inside"),
                    "{cook}"
                );
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "Cook never appeared"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let (_, timeline) = get(&filter, "/api/timeline").await;
        assert!(timeline
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["command"].as_str().unwrap().contains("Cook")));
        handle.stop();
    }

    #[tokio::test]
    async fn pause_stops_the_clock_and_lan_without_a_token_may_not_write() {
        let handle = sim();
        let local = routes(handle.clone(), ControlAuth::resolve(None, LOOPBACK));
        let post = |path: &'static str, body: serde_json::Value| {
            let f = local.clone();
            async move {
                warp::test::request()
                    .method("POST")
                    .path(path)
                    .json(&body)
                    .reply(&f)
                    .await
            }
        };
        assert_eq!(
            post(
                "/api/control",
                serde_json::json!({ "SetSpeed": "AsFastAsPossible" })
            )
            .await
            .status(),
            202
        );
        let started = Instant::now();
        while handle.projection().tick < 120 {
            assert!(started.elapsed() < Duration::from_secs(90));
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(
            post("/api/control", serde_json::json!("Pause"))
                .await
                .status(),
            202
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
        let paused_at = handle.projection().tick;
        assert!(handle.projection().paused);
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_eq!(
            handle.projection().tick,
            paused_at,
            "time passed while paused"
        );
        assert_eq!(
            post("/api/control", serde_json::json!({ "Step": 3 }))
                .await
                .status(),
            202
        );
        let started = Instant::now();
        while handle.projection().tick < paused_at + 3 {
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "Step(3) did not run"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(
            handle.projection().tick,
            paused_at + 3,
            "exactly three steps"
        );

        let lan = routes(
            handle.clone(),
            ControlAuth::resolve(None, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))),
        );
        let refused = warp::test::request()
            .method("POST")
            .path("/api/control")
            .json(&serde_json::json!("Resume"))
            .reply(&lan)
            .await;
        assert_eq!(refused.status(), 401);
        handle.stop();
    }
}
