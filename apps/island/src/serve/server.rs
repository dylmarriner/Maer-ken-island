//! The dashboard's HTTP API and pages. Reads never change anything; the one
//! write, `POST /api/humans`, is gated by [`ControlAuth`].

use super::auth::ControlAuth;
use super::pages;
use super::read;
use super::sim::{IslandCommand, SimHandle, SimSpeed};
use super::view;
use island_humans::{CreateHumanError, CreateHumanRequest, IslandHumanPopulation};
use mk_engine::regional::commands::ControlCommand;
use mk_engine::regional::create_human::{CreateLocation, IslandCreateHuman};
use mk_engine::regional::estate_layout::SpaceId;
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

pub fn create(
    population: &mut IslandHumanPopulation,
    auth: &ControlAuth,
    authorization: Option<&str>,
    request: CreateHumanRequest,
) -> (StatusCode, serde_json::Value) {
    if !auth.permits(authorization) {
        return (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({
                "errors": [match auth.mode() {
                    "token" => "That control token was not accepted.",
                    _ => "This dashboard is not allowed to create people.",
                }],
                "writes": auth.describe(),
                "writes_mode": auth.mode(),
            }),
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

/// A request to create somebody *in the world*.
///
/// The dashboard's stored-person fields, plus where on the island they
/// start. `space` names a room of the estate by its layout id; `row`/`col`
/// put them on a cell. Exactly one is required, because "somewhere" is not
/// a place and guessing one would put a person where nobody asked.
#[derive(Debug, serde::Deserialize)]
struct WorldCreateRequest {
    name: String,
    biological_sex: String,
    birth_timestamp: String,
    age_years: f64,
    height_cm: f64,
    build: String,
    hair_color: String,
    eye_color: String,
    skin_tone: String,
    space: Option<u32>,
    row: Option<usize>,
    col: Option<usize>,
    /// Default true: somebody created on the island was, as a rule, born
    /// there. A creator who means otherwise says so and gives coordinates.
    #[serde(default = "yes")]
    birthplace_here: bool,
    #[serde(default)]
    birth_latitude: f64,
    #[serde(default)]
    birth_longitude: f64,
}

fn yes() -> bool {
    true
}

impl WorldCreateRequest {
    /// Turn the wire shape into a command, or say what is wrong with it.
    ///
    /// Only the parts the engine cannot check are checked here: the sex
    /// word and the choice of location. Everything else — the age, the
    /// height, whether that room exists, whether that cell is in the sea —
    /// belongs to the island and is checked on its own thread against the
    /// live world, which is the only place those answers are true.
    fn into_command(self) -> Result<IslandCreateHuman, Vec<String>> {
        let mut problems = Vec::new();
        let sex = match self.biological_sex.trim().to_ascii_lowercase().as_str() {
            "male" => Some(mk_core::human::BiologicalSex::Male),
            "female" => Some(mk_core::human::BiologicalSex::Female),
            other => {
                problems.push(format!(
                    "biological_sex: {other:?} is not one the spawn templates support; use male or female."
                ));
                None
            }
        };
        let location = match (self.space, self.row, self.col) {
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
                    "location: give either `space` or both `row` and `col`, not a mixture."
                        .to_string(),
                );
                None
            }
        };
        match (sex, location) {
            (Some(biological_sex), Some(location)) if problems.is_empty() => {
                Ok(IslandCreateHuman {
                    name: self.name,
                    biological_sex,
                    birth_timestamp: self.birth_timestamp,
                    age_years: self.age_years,
                    height_cm: self.height_cm,
                    build: self.build,
                    hair_color: self.hair_color,
                    eye_color: self.eye_color,
                    skin_tone: self.skin_tone,
                    location,
                    birthplace_here: self.birthplace_here,
                    birth_latitude: self.birth_latitude,
                    birth_longitude: self.birth_longitude,
                })
            }
            _ => Err(problems),
        }
    }
}

/// Running the island: pause it, resume it, step it, or change its speed.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum ControlRequest {
    Pause,
    Resume,
    /// Advance exactly this many steps and then hold. `ControlCommand::Step`
    /// has been in the island's vocabulary since Task 4 with nothing behind
    /// it; this is the loop's step budget, which is also what an upstream
    /// `InterventionAction::Step` now drives.
    Step {
        ticks: u64,
    },
    /// Write the whole island to the directory the server was started with.
    ///
    /// Slow enough that it is worth saying so: about half a minute on the
    /// full island, during which the simulation thread is writing rather
    /// than stepping. It is queued like any other command, so the reply is
    /// a command id and the outcome carries what it cost.
    Snapshot,
    /// `real`, `max`, or a multiplier like `60`.
    SetSpeed {
        speed: String,
    },
}

impl ControlRequest {
    /// Apply it, and give back the command to record.
    fn apply(&self, world: &SimHandle) -> Result<ControlCommand, String> {
        match self {
            Self::Pause => {
                world.set_paused(true);
                Ok(ControlCommand::Pause)
            }
            Self::Resume => {
                world.set_paused(false);
                Ok(ControlCommand::Resume)
            }
            Self::Step { ticks } => {
                world.step_for(*ticks);
                Ok(ControlCommand::Step(*ticks))
            }
            // Unlike the others this one is not applied here: it has to
            // happen on the thread that owns the island, between steps,
            // like anything else that touches the world or its files.
            Self::Snapshot => Ok(ControlCommand::Snapshot),
            Self::SetSpeed { speed } => {
                let parsed: SimSpeed = speed.parse()?;
                world.set_speed(parsed);
                Ok(ControlCommand::SetSpeed(speed.clone()))
            }
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
                if !auth.permits(authorization.as_deref()) {
                    // The same rule as storing a person: a world write is
                    // no less a write for happening a step later.
                    return json(
                        StatusCode::UNAUTHORIZED,
                        &serde_json::json!({
                            "errors": [match auth.mode() {
                                "token" => "That control token was not accepted.",
                                _ => "This dashboard is not allowed to create people.",
                            }],
                            "writes": auth.describe(),
                            "writes_mode": auth.mode(),
                        }),
                    );
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
                match request.into_command() {
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
                    return json(
                        StatusCode::UNAUTHORIZED,
                        &serde_json::json!({
                            "errors": [match auth.mode() {
                                "token" => "That control token was not accepted.",
                                _ => "This dashboard is not allowed to intervene in the world.",
                            }],
                            "writes": auth.describe(),
                            "writes_mode": auth.mode(),
                        }),
                    );
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
                    return json(
                        StatusCode::UNAUTHORIZED,
                        &serde_json::json!({
                            "errors": ["This dashboard is not allowed to control the island."],
                            "writes": auth.describe(),
                            "writes_mode": auth.mode(),
                        }),
                    );
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
                match request.apply(&world) {
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
                &serde_json::json!({ "errors": ["No such endpoint. The dashboard serves /api/status, /api/world, /api/world/<estate|vegetation|materials|clock>, /api/world/humans, /api/world/humans/<agent-id>, /api/world/interventions, /api/world/commands/<id>, /api/properties, /api/economy, /api/timeline, /api/conversations, /api/control, /api/health, /api/humans, /api/humans/<agent-id>, /api/activity and /api/creator/options."] }),
            )
        });

    let api = get_status
        .or(get_properties)
        .or(get_economy)
        .or(get_timeline)
        .or(get_conversations)
        .or(get_world)
        .or(post_world_human)
        .or(post_world_intervention)
        .or(get_world_human)
        .or(get_world_humans)
        .or(get_command)
        .or(post_control)
        .or(world_part)
        .or(get_health)
        .or(get_options)
        .or(get_activity)
        .or(one)
        .or(list)
        .or(post)
        .or(unknown_api);

    api.or(pages::routes())
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
    let listener = tokio::net::TcpListener::bind(bind).await?;
    warp::serve(routes_with_world(population, auth, world))
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
