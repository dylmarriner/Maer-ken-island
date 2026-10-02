//! The dashboard's HTTP API and pages. Reads never change anything; the one
//! write, `POST /api/humans`, is gated by [`ControlAuth`].

use super::auth::ControlAuth;
use super::pages;
use island_humans::{CreateHumanError, CreateHumanRequest, IslandHumanPopulation};
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;
use warp::Filter;

pub type SharedPopulation = Arc<Mutex<IslandHumanPopulation>>;

/// Option lists offered by the Creator page, from upstream's foundry panel
/// (`apps/mk_studio/src/ui/foundry_panel.rs`). The server accepts any
/// non-empty value, as upstream validation does.
pub const SEXES: [&str; 2] = ["female", "male"];
pub const BUILDS: [&str; 4] = ["Slim", "Average", "Athletic", "Heavy"];
pub const HAIR_COLORS: [&str; 6] = ["Black", "Brown", "Blonde", "Red", "Gray", "White"];
pub const EYE_COLORS: [&str; 6] = ["Brown", "Blue", "Green", "Hazel", "Gray", "Amber"];
pub const SKIN_TONES: [&str; 5] = ["Fair", "Light", "Medium", "Olive", "Dark"];

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

pub fn status(population: &IslandHumanPopulation, auth: &ControlAuth) -> serde_json::Value {
    serde_json::json!({
        "population": population.len(),
        "data_dir": population.data_dir().map(|d| d.display().to_string()),
        "seed_hex": hex::encode(population.seed()),
        "time_running": false,
        "writes": auth.describe(),
        "notes": ["Humans are created and stored but time does not pass yet; they are not stepped until the island world exists (Phase 4)."],
    })
}

pub fn detail(population: &IslandHumanPopulation, agent_id: &str) -> Option<serde_json::Value> {
    let human = population.get(agent_id)?;
    Some(serde_json::json!({
        "summary": population.summary(agent_id),
        "folder": population.folder_of(agent_id).map(|f| f.display().to_string()),
        "human": serde_json::to_value(human).unwrap_or(serde_json::Value::Null),
    }))
}

pub fn options() -> serde_json::Value {
    serde_json::json!({
        "biological_sex": SEXES,
        "build": BUILDS,
        "hair_color": HAIR_COLORS,
        "eye_color": EYE_COLORS,
        "skin_tone": SKIN_TONES,
        "max_age_years": mk_interventions::MAX_SPAWN_AGE_YEARS,
        "height_cm": [mk_interventions::SPAWN_HEIGHT_RANGE_CM.start(), mk_interventions::SPAWN_HEIGHT_RANGE_CM.end()],
    })
}

pub fn create(
    population: &mut IslandHumanPopulation,
    auth: &ControlAuth,
    authorization: Option<&str>,
    request: CreateHumanRequest,
) -> (StatusCode, serde_json::Value) {
    if !auth.permits(authorization) {
        return (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({ "errors": ["unauthorised"], "writes": auth.describe() }),
        );
    }
    match population.create_human(request, "dashboard") {
        Ok(created) => (
            StatusCode::CREATED,
            serde_json::to_value(created).unwrap_or_default(),
        ),
        Err(CreateHumanError::Invalid(errors)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            serde_json::json!({ "errors": errors }),
        ),
    }
}

pub fn routes(
    population: SharedPopulation,
    auth: ControlAuth,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let get_status = warp::path!("api" / "status")
        .and(warp::get())
        .and(with(population.clone()))
        .and(with(auth.clone()))
        .map(|population: SharedPopulation, auth: ControlAuth| {
            let population = population.lock().expect("population lock poisoned");
            json(StatusCode::OK, &status(&population, &auth))
        });

    let list = warp::path!("api" / "humans")
        .and(warp::get())
        .and(with(population.clone()))
        .map(|population: SharedPopulation| {
            let population = population.lock().expect("population lock poisoned");
            json(
                StatusCode::OK,
                &serde_json::to_value(population.summaries()).unwrap_or_default(),
            )
        });

    let one = warp::path!("api" / "humans" / String)
        .and(warp::get())
        .and(with(population.clone()))
        .map(|agent_id: String, population: SharedPopulation| {
            let population = population.lock().expect("population lock poisoned");
            match detail(&population, &agent_id) {
                Some(body) => json(StatusCode::OK, &body),
                None => json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": ["no such human"] }),
                ),
            }
        });

    let get_options = warp::path!("api" / "creator" / "options")
        .and(warp::get())
        .map(|| json(StatusCode::OK, &options()));

    let post = warp::path!("api" / "humans")
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(16 * 1024))
        .and(warp::body::json::<CreateHumanRequest>())
        .and(with(population))
        .and(with(auth))
        .map(
            |authorization: Option<String>,
             request: CreateHumanRequest,
             population: SharedPopulation,
             auth: ControlAuth| {
                let mut population = population.lock().expect("population lock poisoned");
                let (code, body) =
                    create(&mut population, &auth, authorization.as_deref(), request);
                json(code, &body)
            },
        );

    get_status
        .or(get_options)
        .or(one)
        .or(list)
        .or(post)
        .or(pages::routes())
}

/// Serve until Ctrl-C, following upstream `mk serve`'s pattern.
pub async fn run(
    population: SharedPopulation,
    auth: ControlAuth,
    bind: std::net::SocketAddr,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    warp::serve(routes(population, auth))
        .incoming(listener)
        .graceful(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .run()
        .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    fn stored() -> (tempfile::TempDir, SharedPopulation) {
        let tmp = tempfile::tempdir().unwrap();
        let (population, _) = IslandHumanPopulation::open(tmp.path(), [3u8; 32]).unwrap();
        (tmp, Arc::new(Mutex::new(population)))
    }

    fn body() -> serde_json::Value {
        serde_json::json!({
            "name": "Hine Moana", "biological_sex": "female",
            "birth_timestamp": "1992-11-03T10:15:00+13:00",
            "birth_latitude": -41.3, "birth_longitude": 174.8, "age_years": 33.9,
            "height_cm": 166.0, "build": "Athletic", "hair_color": "Black",
            "eye_color": "Brown", "skin_tone": "Medium"
        })
    }

    #[tokio::test]
    async fn reads_work_without_a_token() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(Some("t".into()), LOOPBACK));
        for path in [
            "/api/status",
            "/api/humans",
            "/api/humans/Gem-D",
            "/api/creator/options",
        ] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 200, "{path}");
        }
        let missing = warp::test::request()
            .path("/api/humans/nobody")
            .reply(&filter)
            .await;
        assert_eq!(missing.status(), 404);
    }

    #[tokio::test]
    async fn creating_requires_the_token_when_one_is_set() {
        let (tmp, population) = stored();
        let filter = routes(
            population,
            ControlAuth::resolve(Some("s3cret".into()), LOOPBACK),
        );
        let refused = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .json(&body())
            .reply(&filter)
            .await;
        assert_eq!(refused.status(), 401);
        let created = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("authorization", "Bearer s3cret")
            .json(&body())
            .reply(&filter)
            .await;
        assert_eq!(created.status(), 201);
        let created: serde_json::Value = serde_json::from_slice(created.body()).unwrap();
        assert_eq!(created["summary"]["agent_id"], "hine-moana");
        assert!(tmp.path().join("humans/hine-moana/profile").is_dir());

        let roster = warp::test::request()
            .path("/api/humans")
            .reply(&filter)
            .await;
        let roster: serde_json::Value = serde_json::from_slice(roster.body()).unwrap();
        assert!(roster
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["agent_id"] == "hine-moana"));
    }

    #[tokio::test]
    async fn loopback_without_a_token_may_create_and_lan_without_a_token_may_not() {
        let (_tmp, population) = stored();
        let local = routes(population.clone(), ControlAuth::resolve(None, LOOPBACK));
        let ok = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .json(&body())
            .reply(&local)
            .await;
        assert_eq!(ok.status(), 201);
        let lan = routes(
            population,
            ControlAuth::resolve(None, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))),
        );
        let refused = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .json(&body())
            .reply(&lan)
            .await;
        assert_eq!(refused.status(), 401);
    }

    #[tokio::test]
    async fn an_invalid_request_returns_upstream_messages() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        let mut bad = body();
        bad["height_cm"] = serde_json::json!(5.0);
        let response = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .json(&bad)
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 422);
        let errors: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        assert!(
            errors["errors"][0].as_str().unwrap().contains("height"),
            "{errors}"
        );
    }

    #[tokio::test]
    async fn a_path_cannot_reach_outside_the_population() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        for path in [
            "/api/humans/..%2F..%2Fetc%2Fpasswd",
            "/api/humans/../../etc/passwd",
        ] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_ne!(response.status(), 200, "{path}");
        }
    }
}
