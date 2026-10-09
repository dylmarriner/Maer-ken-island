//! Talk to a Maer-Ken Island backend over HTTP.
//!
//! The island runs in one process and is looked at from others — a web
//! page on one machine, a desktop application on another. This is the
//! second half of that split: [`mk_island_api`] says what the shapes are
//! and this knows how to ask for them.
//!
//! Blocking on purpose. Every caller so far reads the island from a thread
//! of its own — the desktop app polls on a background thread and hands the
//! result to its renderer — and an async client would make each of them
//! bring a runtime along to do it.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mk_island_client::IslandClient;
//!
//! // `connect` asks `/api/version` first and refuses a server this
//! // client cannot read, rather than failing later on a missing field.
//! let (island, server) = IslandClient::connect("http://island.local:8080", None)?;
//! if server.capabilities.world {
//!     let world = island.world()?;
//!     println!("tick {} with {} people", world.clock.tick, world.people.len());
//! }
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

pub mod error;

pub use error::ClientError;
pub use mk_island_api as api;

use mk_island_api::{
    Accepted, Capabilities, Cell, ControlRequest, Conversation, CreateHumanRequest, Economy,
    InterventionRequest, Outcome, Property, Refusal, ServerVersion, Terrain, TimelineEntry, Trees,
    World,
};
use std::time::Duration;

/// How long to wait for the backend before calling it unreachable.
///
/// Generous for a read, because the one request that is genuinely large is
/// the terrain: 5.8 MB on the real island, which is a second or two on a
/// slow link and should not be mistaken for a dead server.
const TIMEOUT: Duration = Duration::from_secs(30);

/// A backend, and how to reach it.
#[derive(Clone)]
pub struct IslandClient {
    base: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl std::fmt::Debug for IslandClient {
    /// Written by hand so a token never reaches a log through a `{:?}`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IslandClient")
            .field("base", &self.base)
            .field("token", &self.token.as_ref().map(|_| "<set>"))
            .finish()
    }
}

impl IslandClient {
    /// A client for a backend at `base`, without asking it anything.
    ///
    /// `base` is scheme, host and port: `http://island.local:8080`. A
    /// trailing slash is harmless and is trimmed.
    pub fn new(base: &str, token: Option<String>) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.filter(|t| !t.trim().is_empty()),
            agent: ureq::AgentBuilder::new()
                .timeout(TIMEOUT)
                .user_agent(concat!("mk_island_client/", env!("CARGO_PKG_VERSION")))
                .build(),
        }
    }

    /// A client, and the version negotiation that says it can be used.
    ///
    /// Asks `/api/version` and refuses a server outside this client's
    /// schema range. Doing it here rather than letting the first real
    /// request fail is the difference between "update the desktop app"
    /// and "the island sent a person with no name".
    pub fn connect(
        base: &str,
        token: Option<String>,
    ) -> Result<(Self, ServerVersion), ClientError> {
        let client = Self::new(base, token);
        let version = client.version()?;
        version
            .compatibility(mk_island_api::API_VERSION)
            .map_err(ClientError::Incompatible)?;
        Ok((client, version))
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// The same backend, with a different token. Used when somebody types
    /// one into a frontend that started without one.
    pub fn with_token(&self, token: Option<String>) -> Self {
        Self {
            base: self.base.clone(),
            token: token.filter(|t| !t.trim().is_empty()),
            agent: self.agent.clone(),
        }
    }

    // ---- reads ----

    /// What this server speaks and what it can do. Answered without a
    /// token, so it works before anybody has one.
    pub fn version(&self) -> Result<ServerVersion, ClientError> {
        self.json("/api/version")
    }

    /// What this server can do, for a caller that already negotiated.
    pub fn capabilities(&self) -> Result<Capabilities, ClientError> {
        Ok(self.version()?.capabilities)
    }

    /// The island at this moment.
    pub fn world(&self) -> Result<World, ClientError> {
        let value: serde_json::Value = self.json("/api/world")?;
        // A backend with no island answers 200 and `{"running": false,
        // "reason": ...}` rather than a world, which is kinder to a page
        // than a 404 and has to be turned back into a refusal here.
        if value.get("reason").is_some() && value.get("clock").is_none() {
            return Err(ClientError::NoIsland {
                what: "world to describe".to_string(),
            });
        }
        serde_json::from_value(value).map_err(|err| ClientError::Malformed {
            url: self.url("/api/world"),
            reason: err.to_string(),
        })
    }

    /// Everyone on the island, as the roster shows them.
    ///
    /// The same [`mk_island_api::Person`] summaries `world()` carries, on
    /// their own, for a frontend that wants the roster without the rest.
    pub fn roster(&self) -> Result<Vec<mk_island_api::Person>, ClientError> {
        self.field("/api/world/humans", "people")
    }

    /// One islander's full canonical record, as JSON.
    ///
    /// Deliberately untyped. This is the whole of upstream's `HumanBeing`
    /// -- identity, genetics, neurochemistry, memory, every system the
    /// engine steps -- and re-declaring that in the wire schema would be
    /// copying the engine into a crate that is not allowed to depend on
    /// it, and would be wrong the first time upstream changed a field. A
    /// frontend reads the parts it shows.
    pub fn human(&self, agent_id: &str) -> Result<serde_json::Value, ClientError> {
        self.json(&format!("/api/world/humans/{agent_id}"))
    }

    /// The estate's properties: buildings, items, who owns them.
    pub fn properties(&self) -> Result<Vec<Property>, ClientError> {
        self.field("/api/properties", "properties")
    }

    /// What has been built, and what the economy recorded doing it.
    ///
    /// `/api/economy` puts the economy's own fields at the top level
    /// beside a note and a tick, so this reads the body rather than a
    /// field of it; serde ignores the rest.
    pub fn economy(&self) -> Result<Economy, ClientError> {
        self.json("/api/economy")
    }

    /// Everything that has reached the island from outside, in order.
    pub fn timeline(&self) -> Result<Vec<TimelineEntry>, ClientError> {
        self.field("/api/timeline", "entries")
    }

    /// The island's recent conversations, newest first.
    pub fn conversations(&self) -> Result<Vec<Conversation>, ClientError> {
        self.field("/api/conversations", "conversations")
    }

    /// Individual stems inside a box of domain metres, thinned to `cap`.
    ///
    /// `Trees::in_box` says how many really stood there, so a thinned view
    /// can never be read as a thin wood.
    pub fn trees_in(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        cap: usize,
    ) -> Result<Trees, ClientError> {
        self.field(
            &format!("/api/trees?x0={x0}&y0={y0}&x1={x1}&y1={y1}&cap={cap}"),
            "trees",
        )
    }

    /// What is at one medium-grid cell.
    pub fn cell(&self, row: usize, col: usize) -> Result<Cell, ClientError> {
        self.field(&format!("/api/cell/{row}/{col}"), "cell")
    }

    /// The island's ground, as numbers rather than as a picture.
    ///
    /// The one large request this client makes. Fetched once: the island
    /// refuses every terrain edit, so the backend serves it immutable and
    /// it cannot change while that process lives.
    pub fn terrain(&self) -> Result<Terrain, ClientError> {
        let bytes = self.bytes("/api/elevation.bin")?;
        Terrain::parse(&bytes).map_err(|err| ClientError::Malformed {
            url: self.url("/api/elevation.bin"),
            reason: err.to_string(),
        })
    }

    /// The island drawn: elevation, standing vegetation, or the estate's
    /// patch. PNG bytes, for a client that would rather show the picture
    /// than build a mesh.
    pub fn map_png(&self) -> Result<Vec<u8>, ClientError> {
        self.bytes("/api/map.png")
    }

    pub fn vegetation_png(&self) -> Result<Vec<u8>, ClientError> {
        self.bytes("/api/vegetation.png")
    }

    pub fn patch_png(&self) -> Result<Vec<u8>, ClientError> {
        self.bytes("/api/patch.png")
    }

    // ---- writes ----
    //
    // Every one of these answers with a command id rather than a result.
    // The simulation owns its state on one thread and applies a command
    // between steps, so there is no way to answer synchronously; poll
    // `outcome` for what happened.

    /// Create somebody in the world.
    pub fn create_human(&self, request: &CreateHumanRequest) -> Result<Accepted, ClientError> {
        self.post(
            "/api/world/humans",
            &serde_json::to_value(request).unwrap_or_default(),
        )
    }

    /// Pause, resume, step, snapshot or change the speed.
    pub fn control(&self, request: &ControlRequest) -> Result<Accepted, ClientError> {
        self.post(
            "/api/control",
            &serde_json::to_value(request).unwrap_or_default(),
        )
    }

    /// Intervene in the island. Many interventions are refused by name and
    /// reason, which arrives as an [`Outcome::Refused`] rather than as an
    /// error: the island said no, and said why.
    pub fn intervene(&self, request: &InterventionRequest) -> Result<Accepted, ClientError> {
        self.post("/api/world/interventions", &request.0)
    }

    /// What became of a command. Outcomes are kept only briefly.
    pub fn outcome(&self, command: u64) -> Result<Outcome, ClientError> {
        self.json(&format!("/api/world/commands/{command}"))
    }

    // ---- the plumbing ----

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn request(&self, method: &str, path: &str) -> ureq::Request {
        let request = self.agent.request(method, &self.url(path));
        match &self.token {
            Some(token) => request.set("authorization", &format!("Bearer {token}")),
            None => request,
        }
    }

    /// A JSON body, or the island's own refusal.
    fn json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let text = self.text(path)?;
        serde_json::from_str(&text).map_err(|err| ClientError::Malformed {
            url: self.url(path),
            reason: err.to_string(),
        })
    }

    /// One named field out of a JSON object.
    ///
    /// Several endpoints wrap their answer — `{"cell": ...}`,
    /// `{"properties": [...], "note": "..."}` — because the note beside it
    /// is part of what they are saying, and a reader deserves it. A client
    /// wanting the payload should not have to know which ones.
    fn field<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        field: &str,
    ) -> Result<T, ClientError> {
        let value: serde_json::Value = self.json(path)?;
        let inner = value
            .get(field)
            .cloned()
            .ok_or_else(|| ClientError::Malformed {
                url: self.url(path),
                reason: format!("there is no `{field}` in the answer"),
            })?;
        serde_json::from_value(inner).map_err(|err| ClientError::Malformed {
            url: self.url(path),
            reason: err.to_string(),
        })
    }

    fn text(&self, path: &str) -> Result<String, ClientError> {
        match self.request("GET", path).call() {
            Ok(response) => response
                .into_string()
                .map_err(|err| ClientError::Malformed {
                    url: self.url(path),
                    reason: err.to_string(),
                }),
            Err(err) => Err(self.failure(path, err)),
        }
    }

    fn bytes(&self, path: &str) -> Result<Vec<u8>, ClientError> {
        match self.request("GET", path).call() {
            Ok(response) => {
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut response.into_reader(), &mut buffer).map_err(
                    |err| ClientError::Malformed {
                        url: self.url(path),
                        reason: err.to_string(),
                    },
                )?;
                Ok(buffer)
            }
            Err(err) => Err(self.failure(path, err)),
        }
    }

    fn post(&self, path: &str, body: &serde_json::Value) -> Result<Accepted, ClientError> {
        match self.request("POST", path).send_json(body.clone()) {
            Ok(response) => {
                let url = self.url(path);
                let text = response
                    .into_string()
                    .map_err(|err| ClientError::Malformed {
                        url: url.clone(),
                        reason: err.to_string(),
                    })?;
                serde_json::from_str(&text).map_err(|err| ClientError::Malformed {
                    url,
                    reason: err.to_string(),
                })
            }
            Err(err) => Err(self.failure(path, err)),
        }
    }

    /// Turn ureq's failure into one of the four things a frontend can act
    /// on.
    fn failure(&self, path: &str, err: ureq::Error) -> ClientError {
        match err {
            ureq::Error::Status(status, response) => {
                let body = response.into_string().unwrap_or_default();
                let refusal: Refusal = serde_json::from_str(&body).unwrap_or_else(|_| Refusal {
                    // Not JSON at all: something between here and the
                    // island answered instead of it. Say so with what it
                    // actually sent, trimmed, rather than an empty
                    // refusal that reads like the island being silent.
                    errors: vec![format!(
                        "{} answered {status} with something that is not the island's JSON: {}",
                        self.url(path),
                        body.chars().take(200).collect::<String>()
                    )],
                    ..Refusal::default()
                });
                // A 404 or 409 from a world endpoint on a backend with no
                // island is not an error to show in red: it is a backend
                // doing exactly what it was started to do.
                let no_island = matches!(status, 404 | 409)
                    && path.starts_with("/api/")
                    && refusal
                        .errors
                        .iter()
                        .any(|e| e.contains("No island is running") || e.contains("--scenario"));
                if no_island {
                    ClientError::NoIsland {
                        what: path.trim_start_matches("/api/").to_string(),
                    }
                } else {
                    ClientError::Refused { status, refusal }
                }
            }
            ureq::Error::Transport(transport) => ClientError::Unreachable {
                url: self.url(path),
                reason: transport.to_string(),
            },
        }
    }
}
