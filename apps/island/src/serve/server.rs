//! The dashboard's HTTP API and pages. Reads never change anything; the one
//! write, `POST /api/humans`, is gated by [`ControlAuth`].

use super::auth::{ControlAuth, ReadAuth};
use super::pages;
use super::read;
use super::sim::{IslandCommand, SimHandle, SimSpeed};
use super::view;
use island_humans::{CreateHumanError, CreateHumanRequest, IslandHumanPopulation};
use mk_engine::regional::commands::ControlCommand;
use mk_engine::regional::create_human::{CreateLocation, IslandCreateHuman};
use mk_engine::regional::estate_layout::SpaceId;
// The write shapes, from the schema every frontend compiles against. The
// island's own validation stays here: whether that room exists and whether
// that cell is in the sea are questions only the live world can answer, and
// a request type on somebody else's laptop must not pretend to know them.
use mk_island_api::{ControlRequest, CreateHumanRequest as WorldCreateRequest};
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;
use warp::Filter;

pub type SharedPopulation = Arc<Mutex<IslandHumanPopulation>>;

/// The build this binary came from, shown in the footer and `/api/status` so
/// a bug report can say which one it was.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How many creations the activity feed returns when the caller does not say.
const ACTIVITY_DEFAULT: usize = 20;
const ACTIVITY_MAX: usize = 200;

/// The largest create request worth reading. One human is a few hundred bytes
/// of JSON; anything near this is a mistake or an attack.
const MAX_CREATE_BODY: u64 = 16 * 1024;

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

/// Take the population lock, surviving a poisoned mutex. A panic in one
/// request must not turn every later request into a 500: the population is
/// only ever read or appended to, so the state behind a poisoned lock is
/// still the state the panicking request found.
fn locked(population: &SharedPopulation) -> std::sync::MutexGuard<'_, IslandHumanPopulation> {
    population
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn with<T: Clone + Send>(
    value: T,
) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || value.clone())
}

/// What the dashboard says about itself: how many people there are, where
/// they are kept, and who is allowed to add more.
pub fn status(population: &IslandHumanPopulation, auth: &ControlAuth) -> serde_json::Value {
    let summaries = population.summaries();
    let count = |status: &str| {
        summaries
            .iter()
            .filter(|human| human.status == status)
            .count()
    };
    let by_sex = |sex: &str| summaries.iter().filter(|h| h.biological_sex == sex).count();
    let mut ages: Vec<f64> = summaries
        .iter()
        .map(|human| human.age_years)
        .filter(|age| age.is_finite())
        .collect();
    ages.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = match ages.len() {
        0 => None,
        n if n % 2 == 1 => Some(ages[n / 2]),
        n => Some((ages[n / 2 - 1] + ages[n / 2]) / 2.0),
    };

    serde_json::json!({
        "version": VERSION,
        "population": population.len(),
        "alive": count("alive"),
        "dead": count("dead"),
        "dormant": count("dormant"),
        "female": by_sex("female"),
        "male": by_sex("male"),
        "youngest_years": ages.first().copied(),
        "oldest_years": ages.last().copied(),
        "median_age_years": median,
        "data_dir": population.data_dir().map(|d| d.display().to_string()),
        "seed_hex": hex::encode(population.seed()),
        "time_running": false,
        "writes": auth.describe(),
        "writes_mode": auth.mode(),
        "notes": ["Humans are created and stored but time does not pass yet; they are not stepped until the island world exists (Phase 4)."],
    })
}

/// A liveness check for whatever is watching this process. It touches the
/// population lock, so it fails if the dashboard has wedged rather than only
/// reporting that the port is open.
pub fn health(population: &IslandHumanPopulation) -> serde_json::Value {
    serde_json::json!({
        "status": "ok",
        "version": VERSION,
        "population": population.len(),
        "storage": if population.data_dir().is_some() { "on disk" } else { "in memory" },
    })
}

pub fn detail(population: &IslandHumanPopulation, agent_id: &str) -> Option<serde_json::Value> {
    let human = population.get(agent_id)?;
    let summary = population.summary(agent_id)?;
    let folder = population
        .folder_of(agent_id)
        .map(|f| f.display().to_string());
    Some(serde_json::json!({
        "summary": summary,
        "folder": folder,
        "sections": view::sections(human, &summary, folder.as_deref()),
        "human": serde_json::to_value(human).unwrap_or(serde_json::Value::Null),
    }))
}

/// One islander, in the same shape the stored roster uses.
///
/// `view::sections` wants a `HumanSummary`, which the stored population
/// builds from its own side-table of typed-in names. A person in the world
/// has no such table — an `agent_id` is all they are called — so the
/// summary is built from the record itself and the name *is* the id. That
/// is the honest answer rather than a prettier invented one.
pub fn world_detail(
    human: &mk_engine::humans::HumanBeing,
    current: &super::projection::IslandProjection,
) -> serde_json::Value {
    let summary = island_humans::HumanSummary {
        agent_id: human.agent_id().to_string(),
        name: human.agent_id().to_string(),
        human_id: human.profile.human_id.to_string(),
        biological_sex: format!("{:?}", human.biological_sex()),
        status: format!("{:?}", human.profile.status),
        age_years: human.development.age_years,
    };
    let here = current
        .people
        .iter()
        .find(|p| p.agent_id == summary.agent_id);
    serde_json::json!({
        "summary": summary,
        "folder": serde_json::Value::Null,
        "sections": view::sections(human, &summary, None),
        "human": serde_json::to_value(human).unwrap_or(serde_json::Value::Null),
        // Where they are now, which the stored roster has no answer for.
        "where": here.map(|p| serde_json::json!({
            "space": p.space,
            "asleep": p.asleep,
            "body_carbon_kg": p.body_carbon_kg,
        })),
        // Records are refreshed on the hour, so the page can say how old
        // this reading is rather than implying it is live.
        "records_at_tick": current.records_at_tick,
        "tick": current.clock.tick,
    })
}

/// The creation log, newest first: who has been added to the island and by
/// which door (the dashboard, the CLI, a test).
pub fn activity(population: &IslandHumanPopulation, limit: usize) -> serde_json::Value {
    let limit = limit.clamp(1, ACTIVITY_MAX);
    let Some(dir) = population.data_dir() else {
        return serde_json::json!({
            "creations": [],
            "total": 0,
            "stored": false,
            "note": "This population is held in memory, so there is no creation log to read.",
        });
    };
    match island_humans::read_creations(dir) {
        Ok(records) => {
            let total = records.len();
            let creations: Vec<serde_json::Value> = records
                .iter()
                .rev()
                .take(limit)
                .map(|record| {
                    serde_json::json!({
                        "counter": record.counter,
                        "agent_id": record.agent_id,
                        "name": record.request.name.trim(),
                        "by": record.by,
                        "biological_sex": record.request.biological_sex,
                        "age_years": record.request.age_years,
                    })
                })
                .collect();
            serde_json::json!({ "creations": creations, "total": total, "stored": true })
        }
        Err(err) => serde_json::json!({
            "creations": [],
            "total": 0,
            "stored": true,
            "error": format!("The creation log could not be read: {err}"),
        }),
    }
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

/// The answer to a write this caller may not make.
///
/// One place, because three routes were each writing their own copy of it
/// and a frontend has to be able to tell "your token is wrong" from "this
/// server takes no writes from off-host at all" -- which is what
/// `writes_mode` is for, and why it has to be the same word everywhere.
///
/// `what` completes "This dashboard is not allowed to ...".
fn may_not_write(auth: &ControlAuth, what: &str) -> warp::reply::WithStatus<warp::reply::Json> {
    json(
        StatusCode::UNAUTHORIZED,
        &serde_json::json!({
            "errors": [match auth.mode() {
                "token" => "That control token was not accepted.".to_string(),
                _ => format!("This dashboard is not allowed to {what}."),
            }],
            "writes": auth.describe(),
            "writes_mode": auth.mode(),
        }),
    )
}

pub fn create(
    population: &mut IslandHumanPopulation,
    auth: &ControlAuth,
    authorization: Option<&str>,
    request: CreateHumanRequest,
) -> (StatusCode, serde_json::Value) {
    debug_assert!(
        auth.permits(authorization),
        "the route checks this before parsing; see `may_not_write`"
    );
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

/// Turn the wire shape into a command, or say what is wrong with it.
///
/// Only the parts the engine cannot check are checked here: the sex
/// word and the choice of location. Everything else — the age, the
/// height, whether that room exists, whether that cell is in the sea —
/// belongs to the island and is checked on its own thread against the
/// live world, which is the only place those answers are true.
fn into_command(request: WorldCreateRequest) -> Result<IslandCreateHuman, Vec<String>> {
    let mut problems = Vec::new();
    let sex = match request.biological_sex.trim().to_ascii_lowercase().as_str() {
        "male" => Some(mk_core::human::BiologicalSex::Male),
        "female" => Some(mk_core::human::BiologicalSex::Female),
        other => {
            problems.push(format!(
                "biological_sex: {other:?} is not one the spawn templates support; use male or female."
            ));
            None
        }
    };
    let location = match (request.space, request.row, request.col) {
        (Some(id), None, None) => Some(CreateLocation::EstateSpace(SpaceId(id))),
        (None, Some(row), Some(col)) => Some(CreateLocation::IslandCell { row, col }),
        (None, None, None) => {
            problems.push(
                "location: say where they start — `space` for a room of the estate, or `row` and `col` for a cell of the island."
                    .to_string(),
            );
            None
        }
        _ => {
            problems.push(
                "location: give either `space` or both `row` and `col`, not a mixture.".to_string(),
            );
            None
        }
    };
    match (sex, location) {
        (Some(biological_sex), Some(location)) if problems.is_empty() => Ok(IslandCreateHuman {
            name: request.name,
            biological_sex,
            birth_timestamp: request.birth_timestamp,
            age_years: request.age_years,
            height_cm: request.height_cm,
            build: request.build,
            hair_color: request.hair_color,
            eye_color: request.eye_color,
            skin_tone: request.skin_tone,
            location,
            birthplace_here: request.birthplace_here,
            birth_latitude: request.birth_latitude,
            birth_longitude: request.birth_longitude,
        }),
        _ => Err(problems),
    }
}

/// Apply a control request, and give back the command to record.
fn apply_control(request: &ControlRequest, world: &SimHandle) -> Result<ControlCommand, String> {
    match request {
        ControlRequest::Pause => {
            world.set_paused(true);
            Ok(ControlCommand::Pause)
        }
        ControlRequest::Resume => {
            world.set_paused(false);
            Ok(ControlCommand::Resume)
        }
        ControlRequest::Step { ticks } => {
            world.step_for(*ticks);
            Ok(ControlCommand::Step(*ticks))
        }
        // Unlike the others this one is not applied here: it has to
        // happen on the thread that owns the island, between steps,
        // like anything else that touches the world or its files.
        ControlRequest::Snapshot => Ok(ControlCommand::Snapshot),
        ControlRequest::SetSpeed { speed } => {
            let parsed: SimSpeed = speed.parse()?;
            world.set_speed(parsed);
            Ok(ControlCommand::SetSpeed(speed.clone()))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct ActivityQuery {
    limit: Option<usize>,
}

/// Said in `/api/economy` rather than left for a reader to infer from three
/// empty lists.
///
/// Plain prose with no markup: the dashboard puts it on the page with
/// `textContent`, as it does everything from the server, so a backtick
/// would be a backtick on screen rather than code formatting.
const NO_RESOURCE_NODES: &str =
    "The island seeds no resource nodes from its biomes, the way a planetary world does, so the \
     count of them is always zero. Structures and events are real: they are what has actually \
     been built here and what the economy recorded doing it, which is nothing until somebody \
     builds something.";

/// Said in `/api/timeline` for the same reason.
const TIMELINE_IS_EXTERNAL: &str =
    "Everything that has reached this island from outside, in the order it applied — the same \
     record the replay runner reads, so this cannot disagree with it. It is not a history of \
     what the islanders did: nothing here is a person going to bed or felling a tree.";

/// A PNG, or a 404 when there is no island to draw.
///
/// `no-cache` rather than the elevation map's silence: these layers are
/// redrawn as the island's biomass grows, and a browser that held the
/// first one would show the forest it bootstrapped with for ever.
fn png_reply(bytes: Option<std::sync::Arc<Vec<u8>>>) -> impl warp::Reply {
    match bytes {
        None => warp::reply::with_status(
            warp::reply::with_header(
                warp::reply::with_header(Vec::new(), "content-type", "text/plain; charset=utf-8"),
                "cache-control",
                "no-cache",
            ),
            StatusCode::NOT_FOUND,
        ),
        Some(bytes) => warp::reply::with_status(
            warp::reply::with_header(
                warp::reply::with_header(bytes.to_vec(), "content-type", "image/png"),
                "cache-control",
                "no-cache",
            ),
            StatusCode::OK,
        ),
    }
}

/// The terrain bytes, or a 404 when there is no island to describe.
///
/// `immutable` for a year, and that is earned rather than optimistic: the
/// island refuses `SculptTerrain` and `SmoothTerrain`, so these bytes
/// cannot change while the process lives. A client across a network
/// fetches 5.8 MB once and never again.
fn elevation_reply(bytes: Option<std::sync::Arc<Vec<u8>>>, gzip: bool) -> impl warp::Reply {
    // The refusal is JSON and says `--scenario`, like every other endpoint
    // that has no island to answer about. This used to be an empty body
    // with a 404, which is the one answer a client cannot do anything
    // with: it cannot tell "this backend is not simulating an island"
    // from "something between us ate the response".
    const NO_ISLAND: &str = "{\"errors\":[\"No island is running, so it has no terrain. Start \
                             the server with --scenario.\"]}";
    let (status, body) = match bytes {
        None => (StatusCode::NOT_FOUND, NO_ISLAND.as_bytes().to_vec()),
        Some(bytes) => (StatusCode::OK, bytes.to_vec()),
    };
    let kind = if status == StatusCode::OK {
        "application/octet-stream"
    } else {
        "application/json"
    };
    let encoding = if gzip && status == StatusCode::OK {
        "gzip"
    } else {
        "identity"
    };
    warp::reply::with_status(
        warp::reply::with_header(
            warp::reply::with_header(
                warp::reply::with_header(body, "content-type", kind),
                "content-encoding",
                encoding,
            ),
            "cache-control",
            // Immutable only when there is terrain: the island refuses
            // every terrain edit, so those bytes cannot change while the
            // process lives. A refusal is not immutable -- the island it
            // is refusing about may well be started next.
            if status == StatusCode::OK {
                "public, max-age=31536000, immutable"
            } else {
                "no-store"
            },
        ),
        status,
    )
}

/// Whether a real computer bridge is attached to this process.
///
/// Read from the environment rather than from the world, because that is
/// where it is decided: `HttpComputerBridge::from_env` attaches only when
/// `COMPUTER_ACTIONS_ENABLED=1`, and nothing downstream of that decision
/// is allowed to change the island's deterministic state.
fn computer_service_attached() -> bool {
    std::env::var("COMPUTER_ACTIONS_ENABLED").as_deref() == Ok("1")
}

/// Said in `/api/trees`, because a viewer that drew an empty island and
/// said nothing would be read as a treeless one.
const TREES_ARE_LOCAL: &str =
    "Individual stems exist only inside `individual_radius_m` of `individual_centre_m` — the \
     ground around the founders' estate. Out there the engine holds a `TreeInstance` per stem \
     above a minimum diameter, with its own height, diameter and biomass. Everywhere else, \
     including the rest of the 4 km patch and the whole island beyond it, vegetation is modelled \
     as stand cover and biomass per cell, not as trees, so there is nothing individual to draw \
     and an empty box here does not mean bare ground. `kind` is Tree, Shrub or Grass and is not \
     a species: the island's vegetation carries no species, and island-scale species is open \
     work. `in_box` is how many really stood in the box before thinning, `shown` how many came \
     back; zoom in until they agree and you are looking at every stem there is.";

/// How many stems `/api/trees` will return however large a `cap` is asked
/// for. Four thousand draws in a few milliseconds on a canvas and costs
/// about 400 kB of JSON; a caller asking for the whole 200,000 would get
/// a 20 MB body and a picture indistinguishable from a thinned one.
const TREE_CAP_CEILING: usize = 4_000;

/// The box `/api/trees` is asked about, in domain metres, with a cap on
/// how many stems come back.
///
/// Defaulted rather than required so `/api/trees` with no query at all
/// answers something sensible — a box large enough to hold any island the
/// engine builds, which the thinning then samples.
#[derive(serde::Deserialize)]
struct TreeBox {
    #[serde(default = "lowest")]
    x0: f64,
    #[serde(default = "lowest")]
    y0: f64,
    #[serde(default = "highest")]
    x1: f64,
    #[serde(default = "highest")]
    y1: f64,
    #[serde(default = "default_tree_cap")]
    cap: usize,
}

/// Not zero: a default of zero would quietly clip to the north-east
/// quadrant of the domain, and nothing says domain metres are positive.
fn lowest() -> f64 {
    f64::MIN
}

fn highest() -> f64 {
    f64::MAX
}

fn default_tree_cap() -> usize {
    TREE_CAP_CEILING
}

/// Said in `/api/conversations`, because a reader needs to know what these
/// lines are and, just as much, what they are not.
const CONVERSATIONS_ARE_COMPOSED: &str =
    "What the islanders have said to each other, newest first. Every line is composed by the \
     engine from state it had already computed: the speaker's own emotion and internal \
     monologue, and what the listener said to them last time these two spoke. No language \
     model is involved, and the same two people at the same tick always say the same thing. \
     This is the whole island's recent feed, not one person's memory: each islander keeps their \
     own, far longer, history. Expect repetition: a family pair converses on every tick, which \
     is once a simulated minute, awake or asleep, and the state the lines are composed from \
     barely moves in that time. That is a known limit of the model rather than of this page \
     (D35 in the deviation register), and it is shown as it is rather than tidied up.";

/// Headers every response carries. The pages load nothing from anywhere but
/// this server, so the policy can say exactly that: no third-party script,
/// style, image or connection, no framing, and no form posting its own way
/// out if a script fails to load.
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
     img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; \
     frame-ancestors 'none'";

/// One line per request on stdout, the way a server log reads. Set
/// `ISLAND_ACCESS_LOG=off` to keep the terminal quiet.
fn access_log() -> warp::log::Log<impl Fn(warp::log::Info<'_>) + Copy> {
    warp::log::custom(|info| {
        if std::env::var("ISLAND_ACCESS_LOG").is_ok_and(|value| value.eq_ignore_ascii_case("off")) {
            return;
        }
        println!(
            "{} {} {} {:.1}ms",
            info.method(),
            info.path(),
            info.status().as_u16(),
            info.elapsed().as_secs_f64() * 1000.0
        );
    })
}

/// How this backend is exposed.
///
/// The dashboard used to be one program on one machine, where none of
/// these questions came up: reads were open because the port was, and a
/// browser only ever talked to the server that sent it the page. A backend
/// serving a web app hosted somewhere else and a desktop application on
/// somebody's laptop has to answer all three deliberately.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Who may change the island.
    pub control: ControlAuth,
    /// Who may read it. [`ReadAuth::Open`] is right on a loopback bind and
    /// is a decision anywhere else.
    pub reads: ReadAuth,
    /// Browser origins allowed to call this server's API, exactly as they
    /// appear in an `Origin` header — scheme, host and port, no path, no
    /// trailing slash (`https://island.example:8443`).
    ///
    /// Empty means no cross-origin header is sent at all, which is not the
    /// same as refusing: a browser's own same-origin rule then applies,
    /// which is what it does for a server that has never heard of CORS.
    /// Nothing but a browser is affected either way — a desktop client and
    /// `curl` do not send an `Origin` and are not bound by one.
    pub allowed_origins: Vec<String>,
}

impl ServeConfig {
    /// The way the dashboard has always run: writes by the control rule,
    /// reads open, same-origin only.
    pub fn local(control: ControlAuth) -> Self {
        Self {
            control,
            reads: ReadAuth::Open,
            allowed_origins: Vec::new(),
        }
    }
}

/// A read refused for want of a token.
///
/// A rejection rather than an answer, because the gate sits in front of
/// every route and has no idea which one was being asked for.
#[derive(Debug)]
struct NeedsReadToken;

impl warp::reject::Reject for NeedsReadToken {}

/// Turn the gate's refusal, and a browser's blocked origin, into JSON.
///
/// Total, because the filter it recovers has to come out `Infallible` for
/// `warp::serve`. In routing terms the last arm is unreachable: the page
/// routes end in a catch-all that matches everything, so the only
/// rejections that reach here are the two this names. It answers anyway,
/// and says plainly that it does not know what happened, rather than
/// letting warp return a bare 500 with no body.
async fn refusals(err: warp::Rejection) -> Result<Box<dyn warp::Reply>, std::convert::Infallible> {
    if err.find::<NeedsReadToken>().is_some() {
        return Ok(Box::new(json(
            StatusCode::UNAUTHORIZED,
            &serde_json::json!({
                "errors": ["This island is not readable without a token. Send it as \
                            `Authorization: Bearer <token>`."],
                "reads": "token",
            }),
        )));
    }
    if let Some(forbidden) = err.find::<warp::filters::cors::CorsForbidden>() {
        return Ok(Box::new(json(
            StatusCode::FORBIDDEN,
            &serde_json::json!({
                "errors": [format!(
                    "This browser origin is not one the island accepts: {forbidden}. Start the \
                     server with --allow-origin for the address the page is served from."
                )],
            }),
        )));
    }
    Ok(Box::new(json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &serde_json::json!({
            "errors": [format!(
                "The island could not route that request and does not know why: {err:?}. This \
                 is a bug in the server rather than anything wrong with the request."
            )],
        }),
    )))
}

/// Refuse every `/api` request that does not carry the read token.
///
/// In front of the routes rather than inside each of them, because a
/// request that may not be read must not be answered differently
/// depending on which endpoint it asked for -- a 404 for an unknown path
/// and a 401 for a known one would tell an unauthenticated caller which
/// paths exist.
///
/// The pages are deliberately outside it. They are markup with no island
/// in them; every number on them arrives by `fetch`, which is gated. A
/// browser pointed at a locked server therefore loads the page and is
/// asked for the token, instead of being handed a bare 401 with no way to
/// enter one.
fn read_gate(
    reads: ReadAuth,
    control: ControlAuth,
) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::path::peek()
        .and(warp::header::optional::<String>("authorization"))
        .and_then(move |peek: warp::path::Peek, header: Option<String>| {
            let (reads, control) = (reads.clone(), control.clone());
            async move {
                if !peek.as_str().starts_with("api") || reads.permits(header.as_deref(), &control) {
                    Ok(())
                } else {
                    Err(warp::reject::custom(NeedsReadToken))
                }
            }
        })
        .untuple_one()
}

pub fn routes(
    population: SharedPopulation,
    auth: ControlAuth,
) -> impl Filter<Extract = (impl warp::Reply,), Error = std::convert::Infallible> + Clone {
    routes_with_world(population, auth, None)
}

/// The routes, with a running island behind `/api/world` when there is one.
///
/// `island serve` without `--scenario` serves the stored population alone,
/// exactly as it did before there was a world to serve: the endpoint then
/// says so rather than inventing a world or disappearing.
pub fn routes_with_world(
    population: SharedPopulation,
    auth: ControlAuth,
    world: Option<SimHandle>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = std::convert::Infallible> + Clone {
    routes_with_config(population, world, ServeConfig::local(auth))
}

/// The routes as an operator configured them: who may read, who may write,
/// and which browser origins may ask at all.
pub fn routes_with_config(
    population: SharedPopulation,
    world: Option<SimHandle>,
    config: ServeConfig,
) -> impl Filter<Extract = (impl warp::Reply,), Error = std::convert::Infallible> + Clone {
    let auth = config.control.clone();
    let get_world = warp::path!("api" / "world")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            Some(handle) => json(
                StatusCode::OK,
                &serde_json::to_value(handle.projection()).unwrap_or_default(),
            ),
            // Not an error: this dashboard is simply not running one.
            None => json(
                StatusCode::OK,
                &serde_json::json!({
                    "running": false,
                    "reason": "No island is running. Start the server with --scenario to simulate one."
                }),
            ),
        });

    let get_status = warp::path!("api" / "status")
        .and(read())
        .and(with(population.clone()))
        .and(with(auth.clone()))
        .map(|population: SharedPopulation, auth: ControlAuth| {
            let population = locked(&population);
            json(StatusCode::OK, &status(&population, &auth))
        });

    let get_health = warp::path("healthz")
        .or(warp::path!("api" / "health"))
        .unify()
        .and(warp::path::end())
        .and(read())
        .and(with(population.clone()))
        .map(|population: SharedPopulation| {
            let population = locked(&population);
            json(StatusCode::OK, &health(&population))
        });

    let list = warp::path!("api" / "humans")
        .and(read())
        .and(with(population.clone()))
        .map(|population: SharedPopulation| {
            let population = locked(&population);
            json(
                StatusCode::OK,
                &serde_json::to_value(population.summaries()).unwrap_or_default(),
            )
        });

    let one = warp::path!("api" / "humans" / String)
        .and(read())
        .and(with(population.clone()))
        .map(|agent_id: String, population: SharedPopulation| {
            let population = locked(&population);
            match detail(&population, &agent_id) {
                Some(body) => json(StatusCode::OK, &body),
                None => json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": ["Nobody on the island has that id."] }),
                ),
            }
        });

    let get_activity = warp::path!("api" / "activity")
        .and(read())
        .and(warp::query::<ActivityQuery>())
        .and(with(population.clone()))
        .map(|query: ActivityQuery, population: SharedPopulation| {
            let population = locked(&population);
            json(
                StatusCode::OK,
                &activity(&population, query.limit.unwrap_or(ACTIVITY_DEFAULT)),
            )
        });

    let get_options = warp::path!("api" / "creator" / "options")
        .and(read())
        .map(|| json(StatusCode::OK, &options()));

    // The body is read as bytes and parsed here rather than by
    // `warp::body::json`, so a malformed request gets a 400 that says what was
    // wrong instead of a bare rejection falling through to the 404 below.
    let post = warp::path!("api" / "humans")
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(MAX_CREATE_BODY))
        .and(warp::body::bytes())
        .and(with(population))
        .and(with(auth.clone()))
        .map(
            |authorization: Option<String>,
             body: bytes::Bytes,
             population: SharedPopulation,
             auth: ControlAuth| {
                // Who before what. Parsing first told an unauthenticated
                // caller the difference between a malformed body and a
                // refused one, and made the server do the parsing for
                // them; neither matters on a loopback bind and both do on
                // a backend reachable from somewhere else.
                if !auth.permits(authorization.as_deref()) {
                    return may_not_write(&auth, "create people");
                }
                let request: CreateHumanRequest = match serde_json::from_slice(&body) {
                    Ok(request) => request,
                    Err(err) => {
                        return json(
                            StatusCode::BAD_REQUEST,
                            &serde_json::json!({ "errors": [format!(
                                "That request body is not the JSON this endpoint expects: {err}."
                            )] }),
                        )
                    }
                };
                let mut population = locked(&population);
                let (code, body) =
                    create(&mut population, &auth, authorization.as_deref(), request);
                json(code, &body)
            },
        );

    // Creating somebody *in the world*, as against storing them. The reply
    // is 202 and a command id: the island applies it on its own thread
    // before its next step, and the page polls for what happened. A
    // request cannot be allowed to reach into a running world and change
    // it mid-step, so there is no way to answer this synchronously.
    let post_world_human = warp::path!("api" / "world" / "humans")
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(MAX_CREATE_BODY))
        .and(warp::body::bytes())
        .and(with(world.clone()))
        .and(with(auth.clone()))
        .map(
            |authorization: Option<String>,
             body: bytes::Bytes,
             world: Option<SimHandle>,
             auth: ControlAuth| {
                let Some(world) = world else {
                    return json(
                        StatusCode::CONFLICT,
                        &serde_json::json!({ "errors": ["No island is running, so there is nowhere to put anybody. Start the server with --scenario."] }),
                    );
                };
                // The same rule as storing a person: a world write is no
                // less a write for happening a step later.
                if !auth.permits(authorization.as_deref()) {
                    return may_not_write(&auth, "create people");
                }
                let request: WorldCreateRequest = match serde_json::from_slice(&body) {
                    Ok(request) => request,
                    Err(err) => {
                        return json(
                            StatusCode::BAD_REQUEST,
                            &serde_json::json!({ "errors": [format!(
                                "That request body is not the JSON this endpoint expects: {err}."
                            )] }),
                        )
                    }
                };
                match into_command(request) {
                    Err(problems) => json(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        &serde_json::json!({ "errors": problems }),
                    ),
                    Ok(command) => match world.send(IslandCommand::CreateHuman(Box::new(command))) {
                        Some(id) => json(
                            StatusCode::ACCEPTED,
                            &serde_json::json!({
                                "command": id,
                                "poll": format!("/api/world/commands/{id}"),
                            }),
                        ),
                        None => json(
                            StatusCode::CONFLICT,
                            &serde_json::json!({ "errors": ["The island has stopped, so nothing more will be applied."] }),
                        ),
                    },
                }
            },
        );

    // The three endpoints Task 6's interface list names, and which were
    // recorded as "not served" for longer than they should have been.
    //
    // I had written that serving them would mean inventing the data. That
    // was wrong, and measuring the island is what showed it: the estate
    // holds two properties with six buildings and eighty-odd items each,
    // the economy holds whatever has been built in it, and the replay log
    // is a timeline of everything that has reached the island from outside.
    // All three are re-slicing, not invention.
    //
    // What *is* honest to say is that the economy is empty until somebody
    // builds something, and that the island seeds no resource nodes the way
    // a planetary world does. So it says so, in the reply, rather than
    // being left out and read as a missing feature.
    let get_properties = warp::path!("api" / "properties")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running, so it has no properties. Start the server with --scenario."] }),
            ),
            Some(world) => {
                let current = world.projection();
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "properties": *current.properties,
                        "estate_cell": current.estate.cell,
                        "spaces": current.estate.spaces,
                    }),
                )
            }
        });

    let get_economy = warp::path!("api" / "economy")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running, so it has no economy. Start the server with --scenario."] }),
            ),
            Some(world) => {
                let current = world.projection();
                let economy = &*current.economy;
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "resource_nodes": economy.resource_nodes,
                        "structures": economy.structures,
                        "events": economy.events,
                        "at_tick": current.economy_at_tick,
                        "tick": current.clock.tick,
                        "note": NO_RESOURCE_NODES,
                    }),
                )
            }
        });

    // What is at one cell, for a click on the map.
    //
    // The map is one pixel per medium cell, so the page can work out which
    // cell was clicked and ask about that one. Answering per click rather
    // than shipping the whole grid keeps this to a few reads: the grid is
    // 1,152,000 cells, which is megabytes nobody asked for.
    let get_cell = warp::path!("api" / "cell" / usize / usize)
        .and(read())
        .and(with(world.clone()))
        .map(|row: usize, col: usize, world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running. Start the server with --scenario."] }),
            ),
            Some(world) => match world.cell(row, col) {
                None => json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({
                        "errors": [format!("No cell at row {row}, column {col}: the island's medium grid is smaller than that.")]
                    }),
                ),
                Some(cell) => json(StatusCode::OK, &serde_json::json!({ "cell": cell })),
            },
        });

    // The island, drawn. Everything else this dashboard serves describes
    // the world in words and tables; this is the one endpoint that shows
    // it. The picture is rendered once at startup and handed out as an
    // `Arc`, so serving it costs a clone and never touches the sim thread.
    let get_map = warp::path!("api" / "map.png")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            None => warp::reply::with_status(
                warp::reply::with_header(Vec::new(), "content-type", "text/plain; charset=utf-8"),
                StatusCode::NOT_FOUND,
            ),
            Some(world) => warp::reply::with_status(
                warp::reply::with_header(world.map_png().to_vec(), "content-type", "image/png"),
                StatusCode::OK,
            ),
        });

    // The stems themselves, inside a box of domain metres, so a viewer can
    // zoom from the whole island down to a stand of trees and have the
    // server send only what is in frame.
    //
    // A box rather than the lot: the patch holds about 200,000 stems, and
    // the honest answer to "show me all the trees" is a thinned sample
    // plus the count that was thinned, which `Trees` carries. Zoom in far
    // enough and the thinning stops and every stem in view is real.
    let get_trees = warp::path!("api" / "trees")
        .and(read())
        .and(warp::query::<TreeBox>())
        .and(with(world.clone()))
        .map(|query: TreeBox, world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running, so nothing is growing. Start the server with --scenario."] }),
            ),
            Some(world) => {
                let found = world.trees_in(
                    query.x0,
                    query.y0,
                    query.x1,
                    query.y1,
                    query.cap.min(TREE_CAP_CEILING),
                );
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "trees": found,
                        "note": TREES_ARE_LOCAL,
                    }),
                )
            }
        });

    // The two vegetation layers. Unlike the elevation these are redrawn
    // as the island grows, so a browser must not keep the first one for
    // ever: they are served `no-cache`, which is a revalidation rather
    // than a refetch and costs a 304 when nothing has changed.
    let get_vegetation = warp::path!("api" / "vegetation.png")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| png_reply(world.map(|w| w.vegetation_png())));

    let get_patch = warp::path!("api" / "patch.png")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| png_reply(world.map(|w| w.patch_png())));

    let get_conversations = warp::path!("api" / "conversations")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running, so nobody is talking. Start the server with --scenario."] }),
            ),
            Some(world) => {
                let current = world.projection();
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "conversations": *current.conversations,
                        "tick": current.clock.tick,
                        "shown": crate::serve::projection::CONVERSATIONS_SHOWN,
                        "note": CONVERSATIONS_ARE_COMPOSED,
                    }),
                )
            }
        });

    let get_timeline = warp::path!("api" / "timeline")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running, so nothing has happened to it. Start the server with --scenario."] }),
            ),
            Some(world) => {
                let current = world.projection();
                // Said here rather than only on stderr. This list is read
                // from the copy in memory, so if writing the log has
                // started failing the page would otherwise show a record
                // that is not on disk and give no sign of it.
                let log_error = world.replay_log_error();
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "entries": *current.timeline,
                        "tick": current.clock.tick,
                        "note": TIMELINE_IS_EXTERNAL,
                        "log_error": log_error,
                    }),
                )
            }
        });

    // An operator intervention, in upstream's own vocabulary. The body is
    // a serialized `InterventionAction`, so what reaches the island is
    // what upstream's executor would have been given — a dashboard is not
    // a second vocabulary for the same thing.
    //
    // Answered 202 and a command id, like a creation and for the same
    // reason: it applies on the island's own thread between steps. The
    // refusals the island makes for actions with no island meaning come
    // back through that poll, naming the action and why, rather than being
    // pre-screened here against a list that would then have to be kept in
    // step with the engine's.
    let post_world_intervention = warp::path!("api" / "world" / "interventions")
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(MAX_CREATE_BODY))
        .and(warp::body::bytes())
        .and(with(world.clone()))
        .and(with(auth.clone()))
        .map(
            |authorization: Option<String>,
             body: bytes::Bytes,
             world: Option<SimHandle>,
             auth: ControlAuth| {
                let Some(world) = world else {
                    return json(
                        StatusCode::CONFLICT,
                        &serde_json::json!({ "errors": ["No island is running, so there is nothing to intervene in. Start the server with --scenario."] }),
                    );
                };
                if !auth.permits(authorization.as_deref()) {
                    return may_not_write(&auth, "intervene in the world");
                }
                let action: mk_interventions::InterventionAction =
                    match serde_json::from_slice(&body) {
                        Ok(action) => action,
                        Err(err) => {
                            return json(
                                StatusCode::BAD_REQUEST,
                                &serde_json::json!({ "errors": [format!(
                                    "That request body is not an intervention: {err}."
                                )] }),
                            )
                        }
                    };
                match world.send(IslandCommand::Intervention(Box::new(action))) {
                    Some(id) => json(
                        StatusCode::ACCEPTED,
                        &serde_json::json!({
                            "command": id,
                            "poll": format!("/api/world/commands/{id}"),
                        }),
                    ),
                    None => json(
                        StatusCode::CONFLICT,
                        &serde_json::json!({ "errors": ["The island has stopped, so nothing more will be applied."] }),
                    ),
                }
            },
        );

    // The world's roster, and one islander in full. The same per-person
    // sections the stored roster uses, built from the world's own people —
    // so a dashboard with an island running has one set of people rather
    // than two different lists.
    let get_world_humans = warp::path!("api" / "world" / "humans")
        .and(read())
        .and(with(world.clone()))
        .map(|world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running."] }),
            ),
            Some(world) => {
                let current = world.projection();
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "people": current.people,
                        "records_at_tick": current.records_at_tick,
                        "tick": current.clock.tick,
                    }),
                )
            }
        });

    let get_world_human = warp::path!("api" / "world" / "humans" / String)
        .and(read())
        .and(with(world.clone()))
        .map(|agent_id: String, world: Option<SimHandle>| {
            let Some(world) = world else {
                return json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": ["No island is running."] }),
                );
            };
            let current = world.projection();
            match current.records.get(&agent_id) {
                Some(human) => json(StatusCode::OK, &world_detail(human, &current)),
                None => json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": [format!(
                        "Nobody on the island has the id {agent_id:?}."
                    )] }),
                ),
            }
        });

    // Running the island, as against changing it. Pausing and speed move
    // nothing in the world — they decide only how often a fixed step
    // happens — so these apply at once rather than being queued, and are
    // recorded in the replay log for the account of what the operator did.
    let post_control = warp::path!("api" / "control")
        .and(warp::post())
        .and(warp::header::optional::<String>("authorization"))
        .and(warp::body::content_length_limit(MAX_CREATE_BODY))
        .and(warp::body::bytes())
        .and(with(world.clone()))
        .and(with(auth.clone()))
        .map(
            |authorization: Option<String>,
             body: bytes::Bytes,
             world: Option<SimHandle>,
             auth: ControlAuth| {
                let Some(world) = world else {
                    return json(
                        StatusCode::CONFLICT,
                        &serde_json::json!({ "errors": ["No island is running. Start the server with --scenario."] }),
                    );
                };
                if !auth.permits(authorization.as_deref()) {
                    // This used to say "not allowed to control the island"
                    // whatever the mode was, so a wrong token and a server
                    // that takes no writes at all read identically. They
                    // need different answers: one is fixed by typing the
                    // right token, the other by starting the server
                    // differently.
                    return may_not_write(&auth, "control the island");
                }
                let request: ControlRequest = match serde_json::from_slice(&body) {
                    Ok(request) => request,
                    Err(err) => {
                        return json(
                            StatusCode::BAD_REQUEST,
                            &serde_json::json!({ "errors": [format!(
                                "That request body is not the JSON this endpoint expects: {err}."
                            )] }),
                        )
                    }
                };
                match apply_control(&request, &world) {
                    Err(problem) => json(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        &serde_json::json!({ "errors": [problem] }),
                    ),
                    Ok(control) => {
                        // Pause, resume, step and speed have already taken
                        // effect and are queued only so the log says they
                        // did. `Snapshot` is the exception: nothing has
                        // happened yet, and it is the queue that will do
                        // it. So the command id comes back for all of them
                        // — harmless for the four, and the only way to
                        // learn how a snapshot went for the fifth.
                        let queued = world.send(IslandCommand::Control(control));
                        let pacing = world.pacing();
                        json(
                            StatusCode::OK,
                            &serde_json::json!({
                                "speed": pacing.speed.describe(),
                                "paused": pacing.paused,
                                "command": queued,
                                "poll": queued.map(|id| format!("/api/world/commands/{id}")),
                            }),
                        )
                    }
                }
            },
        );

    // The parts of the world, each on its own path. The same data the
    // projection carries, split the way somebody asking for one of them
    // would expect to find it.
    let world_part = warp::path!("api" / "world" / String)
        .and(read())
        .and(with(world.clone()))
        .map(|part: String, world: Option<SimHandle>| {
            let Some(world) = world else {
                return json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": ["No island is running."] }),
                );
            };
            let current = world.projection();
            let body = match part.as_str() {
                "estate" => serde_json::to_value(&current.estate),
                "vegetation" => serde_json::to_value(&current.land),
                "materials" => serde_json::to_value(&current.stocks),
                "clock" => serde_json::to_value(&current.clock),
                other => {
                    return json(
                        StatusCode::NOT_FOUND,
                        &serde_json::json!({ "errors": [format!(
                            "The island has no part called {other:?}. It serves estate, vegetation, materials and clock."
                        )] }),
                    )
                }
            };
            json(StatusCode::OK, &body.unwrap_or_default())
        });

    let get_command = warp::path!("api" / "world" / "commands" / u64)
        .and(read())
        .and(with(world.clone()))
        .map(|id: u64, world: Option<SimHandle>| match world {
            None => json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No island is running."] }),
            ),
            Some(world) => match world.outcome(id) {
                Some(outcome) => json(
                    StatusCode::OK,
                    &serde_json::to_value(outcome).unwrap_or_default(),
                ),
                None => json(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "errors": [format!(
                        "Command {id} is not one this island remembers. Outcomes are kept only briefly."
                    )] }),
                ),
            },
        });

    // Anything under /api that matched no route is a client error worth
    // returning as JSON, so a script never has to parse an HTML page to find
    // out it asked for the wrong thing. Everything else gets the page.
    let unknown_api = warp::path("api")
        .and(warp::path::tail())
        .and(warp::method())
        .map(|tail: warp::path::Tail, method: warp::http::Method| {
            // A create request only reaches here when its body was refused
            // before anything read it, which means it was too big.
            if method == warp::http::Method::POST && tail.as_str() == "humans" {
                return json(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    &serde_json::json!({ "errors": [format!(
                        "That request body is larger than {} KB, which is far more than one person takes.",
                        MAX_CREATE_BODY / 1024
                    )] }),
                );
            }
            json(
                StatusCode::NOT_FOUND,
                &serde_json::json!({ "errors": ["No such endpoint. The dashboard serves /api/status, /api/world, /api/world/<estate|vegetation|materials|clock>, /api/world/humans, /api/world/humans/<agent-id>, /api/world/interventions, /api/world/commands/<id>, /api/properties, /api/economy, /api/timeline, /api/conversations, /api/map.png, /api/vegetation.png, /api/patch.png, /api/cell/<row>/<col>, /api/trees, /api/control, /api/health, /api/humans, /api/humans/<agent-id>, /api/activity and /api/creator/options."] }),
            )
        });

    // Asked before anything else, and answered without a token: a client
    // cannot negotiate a schema it is not allowed to ask about, and a
    // version endpoint behind the read gate would mean a frontend showing
    // "unauthorized" where it should show "this server is older than you".
    //
    // It says nothing a reader could not learn by watching which endpoints
    // answer: no token, no bind address, no data. What it does say is what
    // this server can do, so a frontend offers what will work instead of
    // offering everything and letting half of it fail.
    let get_version = {
        let reads_need_token = config.reads.needs_token();
        let origins = config.allowed_origins.clone();
        let mode = auth.mode().to_string();
        warp::path!("api" / "version")
            .and(read())
            .and(with(world.clone()))
            .map(move |world: Option<SimHandle>| {
                let version = mk_island_api::ServerVersion {
                    api_version: mk_island_api::API_VERSION,
                    // One version so far, so the floor is the ceiling.
                    // When the schema breaks, this is what an old client
                    // is measured against.
                    api_version_minimum: 1,
                    server_version: VERSION.to_string(),
                    capabilities: mk_island_api::Capabilities {
                        world: world.is_some(),
                        writes: mode.clone(),
                        reads_need_token,
                        allowed_origins: origins.clone(),
                        elevation: world.is_some(),
                        computer_service: computer_service_attached(),
                        snapshots: world.as_ref().is_some_and(|w| w.can_snapshot()),
                    },
                };
                json(
                    StatusCode::OK,
                    &serde_json::to_value(version).unwrap_or_default(),
                )
            })
    };

    // The terrain as numbers rather than as a picture, for a client that
    // builds a mesh out of it. `/api/map.png` cannot serve that: a colour
    // ramp is not a height, and a renderer that tried to read metres back
    // out of one would be inventing them.
    let get_elevation = warp::path!("api" / "elevation.bin")
        .and(read())
        .and(warp::header::optional::<String>("accept-encoding"))
        .and(with(world.clone()))
        .map(|encodings: Option<String>, world: Option<SimHandle>| {
            let Some(world) = world else {
                return elevation_reply(None, false);
            };
            // Gzip only when it was offered. Both copies are built at
            // startup, so answering a caller that asked for `identity`
            // costs nothing but the bytes it asked for.
            let gzip = encodings
                .as_deref()
                .is_some_and(|value| value.to_ascii_lowercase().contains("gzip"));
            elevation_reply(Some(world.elevation_bin(gzip)), gzip)
        });

    // Two endpoints answer without a read token, and both for the same
    // reason: they are what a caller asks *before* it can have one. A
    // monitor needs to know whether this process is alive, and a client
    // needs to know whether this server still speaks its schema. Neither
    // says anything about the island.
    let open = get_health.or(get_version).unify();

    let api = get_status
        .or(get_properties)
        .or(get_economy)
        .or(get_timeline)
        .or(get_conversations)
        .or(get_map)
        .or(get_cell)
        .or(get_trees)
        .or(get_vegetation)
        .or(get_patch)
        .or(get_elevation)
        .or(get_world)
        .or(post_world_human)
        .or(post_world_intervention)
        .or(get_world_human)
        .or(get_world_humans)
        .or(get_command)
        .or(post_control)
        .or(world_part)
        .or(get_options)
        .or(get_activity)
        .or(one)
        .or(list)
        .or(post)
        .or(unknown_api);

    // Everything else goes through the read gate, which passes freely when
    // no read token is configured -- the default, and what every existing
    // caller sees.
    let gated = read_gate(config.reads.clone(), auth.clone()).and(api.or(pages::routes()));

    let served = open
        .map(|reply| Box::new(reply) as Box<dyn warp::Reply>)
        .or(gated.map(|reply| Box::new(reply) as Box<dyn warp::Reply>))
        .unify();

    // Cross-origin headers only when an operator named an origin. Sending
    // none is not a refusal: it is what a server that has never heard of
    // CORS does, and leaves a browser's own same-origin rule in charge.
    // Configuring an empty allow-list instead would refuse the same-origin
    // POSTs the Creator page makes, because a browser sends `Origin` on
    // those too.
    //
    // Outermost, so a preflight is answered before the read gate sees it.
    // A browser never puts an `Authorization` header on a preflight, so a
    // gate in front of this would refuse every cross-origin write with a
    // 401 the page could do nothing about.
    let served: warp::filters::BoxedFilter<(Box<dyn warp::Reply>,)> =
        if config.allowed_origins.is_empty() {
            served.boxed()
        } else {
            let cors = warp::cors()
                .allow_origins(config.allowed_origins.iter().map(String::as_str))
                .allow_methods(vec!["GET", "HEAD", "POST", "OPTIONS"])
                .allow_headers(vec!["authorization", "content-type", "accept"])
                .allow_credentials(false)
                .max_age(600);
            served
                .with(cors)
                .map(|reply| Box::new(reply) as Box<dyn warp::Reply>)
                .boxed()
        };

    served
        .recover(refusals)
        .unify()
        .with(warp::reply::with::header(
            "content-security-policy",
            CONTENT_SECURITY_POLICY,
        ))
        .with(warp::reply::with::header(
            "x-content-type-options",
            "nosniff",
        ))
        .with(warp::reply::with::header("x-frame-options", "DENY"))
        .with(warp::reply::with::header("referrer-policy", "no-referrer"))
        .with(warp::reply::with::header(
            "permissions-policy",
            "geolocation=(), camera=(), microphone=()",
        ))
        .with(access_log())
}

/// Serve until Ctrl-C, following upstream `mk serve`'s pattern.
pub async fn run(
    population: SharedPopulation,
    auth: ControlAuth,
    bind: std::net::SocketAddr,
) -> std::io::Result<()> {
    run_with_world(population, auth, bind, None).await
}

/// Serve, with a running island behind `/api/world` when there is one.
pub async fn run_with_world(
    population: SharedPopulation,
    auth: ControlAuth,
    bind: std::net::SocketAddr,
    world: Option<SimHandle>,
) -> std::io::Result<()> {
    run_with_config(population, bind, world, ServeConfig::local(auth)).await
}

/// Serve as an operator configured it: the backend a frontend on another
/// machine talks to.
pub async fn run_with_config(
    population: SharedPopulation,
    bind: std::net::SocketAddr,
    world: Option<SimHandle>,
    config: ServeConfig,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    warp::serve(routes_with_config(population, world, config))
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

    async fn get(
        filter: &(impl Filter<Extract = (impl warp::Reply + 'static,), Error = std::convert::Infallible>
              + Clone
              + 'static),
        path: &str,
    ) -> (StatusCode, serde_json::Value) {
        let response = warp::test::request().path(path).reply(filter).await;
        let status = response.status();
        let body = serde_json::from_slice(response.body()).unwrap_or(serde_json::Value::Null);
        (status, body)
    }

    #[tokio::test]
    async fn reads_work_without_a_token() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(Some("t".into()), LOOPBACK));
        for path in [
            "/api/status",
            "/api/health",
            "/healthz",
            "/api/humans",
            "/api/humans/Gem-D",
            "/api/activity",
            "/api/creator/options",
        ] {
            let (status, _) = get(&filter, path).await;
            assert_eq!(status, 200, "{path}");
        }
        let (missing, body) = get(&filter, "/api/humans/nobody").await;
        assert_eq!(missing, 404);
        assert!(body["errors"][0].as_str().unwrap().ends_with('.'), "{body}");
    }

    #[tokio::test]
    async fn status_counts_the_population_and_names_the_build() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        let (_, status) = get(&filter, "/api/status").await;
        assert_eq!(status["population"], 2);
        assert_eq!(status["alive"], 2);
        assert_eq!(status["dead"], 0);
        assert_eq!(status["version"], VERSION);
        assert_eq!(status["writes_mode"], "loopback");
        assert_eq!(status["time_running"], false);
        assert!(status["median_age_years"].as_f64().unwrap() > 0.0);
        assert!(status["seed_hex"].as_str().unwrap().len() == 64);
    }

    #[tokio::test]
    async fn a_person_comes_back_in_readable_sections() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        let (_, person) = get(&filter, "/api/humans/Gem-D").await;
        let sections = person["sections"].as_array().unwrap();
        assert!(sections.len() >= 9, "{} sections", sections.len());
        assert_eq!(sections[0]["id"], "identity");
        assert!(
            !person["human"].is_null(),
            "the whole record is still there"
        );
        let labels: Vec<&str> = sections[0]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field["label"].as_str().unwrap())
            .collect();
        assert!(labels.contains(&"Name"), "{labels:?}");
    }

    #[tokio::test]
    async fn the_activity_feed_lists_creations_newest_first() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        for name in ["Hine Moana", "Tane Rapu"] {
            let mut request = body();
            request["name"] = serde_json::json!(name);
            let created = warp::test::request()
                .method("POST")
                .path("/api/humans")
                .json(&request)
                .reply(&filter)
                .await;
            assert_eq!(created.status(), 201);
        }
        let (_, feed) = get(&filter, "/api/activity").await;
        assert_eq!(feed["total"], 2);
        assert_eq!(feed["stored"], true);
        assert_eq!(feed["creations"][0]["name"], "Tane Rapu");
        assert_eq!(feed["creations"][0]["by"], "dashboard");
        assert_eq!(feed["creations"][1]["name"], "Hine Moana");

        let (_, one) = get(&filter, "/api/activity?limit=1").await;
        assert_eq!(one["creations"].as_array().unwrap().len(), 1);
        assert_eq!(one["total"], 2, "the total still counts everyone");
    }

    #[tokio::test]
    async fn an_in_memory_population_says_why_it_has_no_log() {
        let population = Arc::new(Mutex::new(IslandHumanPopulation::with_founders()));
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        let (status, feed) = get(&filter, "/api/activity").await;
        assert_eq!(status, 200);
        assert_eq!(feed["stored"], false);
        assert!(feed["note"].as_str().unwrap().contains("in memory"));
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

        let (_, roster) = get(&filter, "/api/humans").await;
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
        let refused: serde_json::Value = serde_json::from_slice(refused.body()).unwrap();
        assert_eq!(refused["writes_mode"], "disabled");
        assert!(refused["errors"][0].as_str().unwrap().ends_with('.'));
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
    async fn a_body_that_is_not_the_expected_json_gets_a_reason() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        for (body, expected) in [
            ("not json at all", StatusCode::BAD_REQUEST),
            ("{\"name\": \"Half A Person\"}", StatusCode::BAD_REQUEST),
        ] {
            let response = warp::test::request()
                .method("POST")
                .path("/api/humans")
                .header("content-type", "application/json")
                .body(body)
                .reply(&filter)
                .await;
            assert_eq!(response.status(), expected, "{body}");
            let answer: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
            assert!(
                answer["errors"][0].as_str().unwrap().contains("JSON"),
                "{answer}"
            );
        }
    }

    #[tokio::test]
    async fn an_oversized_body_is_refused_by_size_not_by_confusion() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        let huge = "x".repeat((MAX_CREATE_BODY as usize) + 1);
        let response = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("content-type", "application/json")
            .body(huge)
            .reply(&filter)
            .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let answer: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        assert!(answer["errors"][0]
            .as_str()
            .unwrap()
            .contains("larger than"));
    }

    #[tokio::test]
    async fn a_monitor_may_ask_with_head() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        for path in [
            "/healthz",
            "/api/status",
            "/",
            "/people",
            "/static/style.css",
        ] {
            let response = warp::test::request()
                .method("HEAD")
                .path(path)
                .reply(&filter)
                .await;
            assert_eq!(response.status(), 200, "{path}");
        }
    }

    #[tokio::test]
    async fn an_unknown_api_path_answers_in_json() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        let response = warp::test::request()
            .path("/api/nothing-here")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 404);
        assert!(response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json"));
        let body: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        assert!(body["errors"][0].as_str().unwrap().contains("/api/status"));
    }

    #[tokio::test]
    async fn every_response_carries_the_hardening_headers() {
        let (_tmp, population) = stored();
        let filter = routes(population, ControlAuth::resolve(None, LOOPBACK));
        for path in [
            "/",
            "/people",
            "/creator",
            "/api/status",
            "/static/style.css",
        ] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 200, "{path}");
            let headers = response.headers();
            assert_eq!(headers["x-content-type-options"], "nosniff", "{path}");
            assert_eq!(headers["x-frame-options"], "DENY", "{path}");
            assert_eq!(headers["referrer-policy"], "no-referrer", "{path}");
            let csp = headers["content-security-policy"].to_str().unwrap();
            assert!(csp.contains("default-src 'self'"), "{path}: {csp}");
            assert!(csp.contains("frame-ancestors 'none'"), "{path}: {csp}");
        }
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

#[cfg(test)]
mod remote_tests {
    //! The backend as something a frontend on another machine talks to.
    //!
    //! Everything here was open or absent while the dashboard and the
    //! island were one process on one machine. None of it is a change to
    //! what the island does; all of it is a change to who may ask.

    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
    const LAN: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
    const PAGE: &str = "https://island.example";

    fn stored() -> (tempfile::TempDir, SharedPopulation) {
        let tmp = tempfile::tempdir().unwrap();
        let (population, _) = IslandHumanPopulation::open(tmp.path(), [3u8; 32]).unwrap();
        (tmp, Arc::new(Mutex::new(population)))
    }

    fn served(
        config: ServeConfig,
    ) -> (
        tempfile::TempDir,
        impl Filter<Extract = (impl warp::Reply + 'static,), Error = std::convert::Infallible>
            + Clone
            + 'static,
    ) {
        let (tmp, population) = stored();
        (tmp, routes_with_config(population, None, config))
    }

    fn remote(reads: ReadAuth, origins: &[&str]) -> ServeConfig {
        ServeConfig {
            control: ControlAuth::resolve(Some("control".into()), LAN),
            reads,
            allowed_origins: origins.iter().map(|o| o.to_string()).collect(),
        }
    }

    /// Every path a frontend reads, so a gate that missed one would show
    /// up here rather than in production.
    const READS: [&str; 6] = [
        "/api/status",
        "/api/world",
        "/api/humans",
        "/api/activity",
        "/api/creator/options",
        "/api/nonsense",
    ];

    #[tokio::test]
    async fn a_client_can_ask_what_this_server_speaks_before_it_has_a_token() {
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        let response = warp::test::request()
            .path("/api/version")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 200, "version must not need a token");
        let version: mk_island_api::ServerVersion =
            serde_json::from_slice(response.body()).expect("the shared schema reads it");
        assert_eq!(version.api_version, mk_island_api::API_VERSION);
        assert!(version.compatibility(mk_island_api::API_VERSION).is_ok());
        // Without `--scenario` there is no island, and a frontend should
        // be told so rather than drawing an empty sea.
        assert!(!version.capabilities.world);
        assert!(!version.capabilities.elevation);
        assert_eq!(version.capabilities.writes, "token");
        assert!(version.capabilities.reads_need_token);
    }

    #[tokio::test]
    async fn a_monitor_can_still_ask_whether_the_process_is_alive() {
        // A load balancer has no token and should not need one to find out
        // that the thing behind it is up.
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        for path in ["/healthz", "/api/health"] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 200, "{path}");
        }
    }

    #[tokio::test]
    async fn with_a_read_token_every_other_endpoint_is_refused_without_it() {
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        for path in READS {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 401, "{path} answered without a token");
            let body: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
            assert_eq!(body["reads"], "token", "{path}");
        }
    }

    #[tokio::test]
    async fn an_unknown_path_is_refused_the_same_way_a_known_one_is() {
        // Otherwise the gate is an oracle: a 404 for a path that does not
        // exist and a 401 for one that does would map the API for somebody
        // with no token at all.
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        let known = warp::test::request()
            .path("/api/status")
            .reply(&filter)
            .await;
        let unknown = warp::test::request()
            .path("/api/nonsense")
            .reply(&filter)
            .await;
        assert_eq!(known.status(), unknown.status());
        assert_eq!(known.body(), unknown.body());
    }

    #[tokio::test]
    async fn the_read_token_opens_them_again() {
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        for path in ["/api/status", "/api/humans", "/api/activity"] {
            let response = warp::test::request()
                .path(path)
                .header("authorization", "Bearer read")
                .reply(&filter)
                .await;
            assert_eq!(response.status(), 200, "{path}");
        }
    }

    #[tokio::test]
    async fn the_pages_load_even_when_the_island_is_locked() {
        // A browser pointed at a locked server should get the page and be
        // asked for the token, not a bare 401 with nowhere to type one.
        // The page carries no island state; every number on it arrives by
        // a fetch that is gated.
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        for path in ["/", "/people", "/island", "/creator", "/static/app.js"] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 200, "{path}");
        }
    }

    #[tokio::test]
    async fn without_a_read_token_nothing_changes_for_anybody() {
        let (_tmp, filter) = served(remote(ReadAuth::Open, &[]));
        for path in ["/api/status", "/api/humans", "/", "/api/version"] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 200, "{path}");
        }
    }

    #[tokio::test]
    async fn a_named_origin_gets_the_header_that_lets_a_browser_read_the_answer() {
        let (_tmp, filter) = served(remote(ReadAuth::Open, &[PAGE]));
        let response = warp::test::request()
            .path("/api/status")
            .header("origin", PAGE)
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            PAGE,
            "a browser will discard the body without this"
        );
    }

    #[tokio::test]
    async fn a_preflight_is_answered_without_a_token_and_names_the_write_headers() {
        // A browser never puts `Authorization` on a preflight, so this has
        // to be answered in front of the read gate. It was not, once.
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[PAGE]));
        let response = warp::test::request()
            .method("OPTIONS")
            .path("/api/world/humans")
            .header("origin", PAGE)
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "authorization,content-type",
            )
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 200, "the preflight was refused");
        let allowed = response.headers()["access-control-allow-headers"]
            .to_str()
            .unwrap()
            .to_ascii_lowercase();
        assert!(allowed.contains("authorization"), "{allowed}");
        assert!(allowed.contains("content-type"), "{allowed}");
        assert_eq!(response.headers()["access-control-allow-origin"], PAGE);
    }

    #[tokio::test]
    async fn an_origin_nobody_named_is_refused_in_words() {
        let (_tmp, filter) = served(remote(ReadAuth::Open, &[PAGE]));
        let response = warp::test::request()
            .path("/api/status")
            .header("origin", "https://somewhere.else")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 403);
        let body: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        let message = body["errors"][0].as_str().unwrap();
        assert!(message.contains("--allow-origin"), "{message}");
    }

    #[tokio::test]
    async fn with_no_origins_configured_a_same_origin_post_is_not_refused() {
        // Configuring an empty allow-list would have refused this: a
        // browser sends `Origin` on a same-origin POST too, and warp's
        // CORS filter judges every request that carries one.
        let (_tmp, population) = stored();
        let filter = routes_with_config(
            population,
            None,
            ServeConfig::local(ControlAuth::resolve(None, LOOPBACK)),
        );
        let response = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("origin", PAGE)
            .json(&serde_json::json!({
                "name": "Hine Moana", "biological_sex": "female",
                "birth_timestamp": "1992-11-03T10:15:00+13:00",
                "birth_latitude": -41.3, "birth_longitude": 174.8, "age_years": 33.9,
                "height_cm": 166.0, "build": "Athletic", "hair_color": "Black",
                "eye_color": "Brown", "skin_tone": "Medium"
            }))
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 201, "a same-origin create was refused");
        assert!(
            !response
                .headers()
                .contains_key("access-control-allow-origin"),
            "a server told about no origins should send no cross-origin header"
        );
    }

    #[tokio::test]
    async fn writes_are_still_refused_without_the_control_token() {
        // The read gate is a second lock, not a replacement for the first.
        let (_tmp, filter) = served(remote(ReadAuth::resolve(Some("read".into())), &[]));
        let response = warp::test::request()
            .method("POST")
            .path("/api/humans")
            .header("authorization", "Bearer read")
            .json(&serde_json::json!({"name": "x"}))
            .reply(&filter)
            .await;
        assert_eq!(
            response.status(),
            401,
            "the read token must not be a licence to write"
        );
    }

    #[tokio::test]
    async fn asking_for_terrain_without_an_island_says_so_rather_than_sending_nothing() {
        let (_tmp, filter) = served(remote(ReadAuth::Open, &[]));
        let response = warp::test::request()
            .path("/api/elevation.bin")
            .reply(&filter)
            .await;
        assert_eq!(response.status(), 404);
        let body: serde_json::Value = serde_json::from_slice(response.body())
            .expect("a refusal is JSON, like every other endpoint's");
        assert!(
            body["errors"][0].as_str().unwrap().contains("--scenario"),
            "{body}"
        );
    }
}
