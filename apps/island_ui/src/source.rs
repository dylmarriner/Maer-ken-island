//! Where the island comes from.
//!
//! Two ways to run this application, and deliberately **one** code path
//! for reading an island:
//!
//! - `--server https://island.example` reads a backend somewhere else.
//! - `--scenario fixtures/.../default_scenario.json` runs one here.
//!
//! The second does not get its own way in. It starts the same backend
//! `island serve` starts, on a loopback port nobody else can reach, and
//! then reads it through the same [`IslandClient`] the first uses. That
//! costs a loopback round trip -- microseconds against a 4.3 ms step --
//! and buys the thing worth having: the local case is proven by the
//! remote one, and there is no second reader to drift.
//!
//! The island is read on a thread of its own. A renderer must never block
//! on a network: a frame that waits for a reply is a frame that takes as
//! long as the slowest hop, and on a bad link that is seconds. The thread
//! polls, and the renderer reads whatever was last published.

use mk_island_api::{
    Capabilities, Conversation, Economy, Property, ServerVersion, Terrain, TimelineEntry, World,
};
use mk_island_client::{ClientError, IslandClient};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// How often the island is asked for its state.
///
/// Four times a second. The island's own clock runs at the speed an
/// operator set, so this is not about keeping up with the simulation: it
/// is about how stale the numbers on screen are allowed to be, and a
/// quarter of a second is below what anybody notices. Faster buys nothing
/// and costs a request on somebody else's machine.
pub const POLL: Duration = Duration::from_millis(250);

/// And how often the slower views are asked for.
///
/// The properties, the economy, the timeline and the conversations change
/// rarely or on their own cadence; the island itself refreshes them
/// hourly in its own time. Two seconds is already far faster than they
/// move.
pub const SLOW_POLL: Duration = Duration::from_secs(2);

/// The island as this application last saw it.
///
/// Cloned out of the lock for the renderer to read. Everything in it came
/// from one backend at one moment, so the clock and the people on screen
/// always belong to the same island.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub world: World,
    pub properties: Arc<Vec<Property>>,
    pub economy: Arc<Economy>,
    pub timeline: Arc<Vec<TimelineEntry>>,
    pub conversations: Arc<Vec<Conversation>>,
    /// What went wrong last time, if anything. Carried rather than
    /// logged: a desktop application that silently showed an island five
    /// minutes out of date would be worse than one that said it had lost
    /// the connection.
    pub trouble: Option<String>,
    /// Whether anything has been read at all yet.
    pub ever_read: bool,
}

/// A live connection to an island, polled on its own thread.
pub struct Island {
    client: IslandClient,
    version: ServerVersion,
    /// The ground, fetched once. Terrain cannot change: the island
    /// refuses every terrain edit, and the backend serves it immutable.
    terrain: Option<Terrain>,
    latest: Arc<RwLock<Snapshot>>,
    stop: Arc<AtomicBool>,
    /// Kept so the embedded backend lives exactly as long as this does.
    _local: Option<LocalBackend>,
    poller: Option<std::thread::JoinHandle<()>>,
}

impl Island {
    /// Connect to a backend somewhere else, and start reading it.
    pub fn remote(address: &str, token: Option<String>) -> Result<Self, ClientError> {
        let (client, version) = IslandClient::connect(address, token)?;
        Self::start(client, version, None)
    }

    /// Run an island here, behind the same backend `island serve` runs,
    /// and read it the same way.
    pub fn embedded(local: LocalBackend) -> Result<Self, ClientError> {
        let (client, version) = IslandClient::connect(&local.address, None)?;
        Self::start(client, version, Some(local))
    }

    fn start(
        client: IslandClient,
        version: ServerVersion,
        local: Option<LocalBackend>,
    ) -> Result<Self, ClientError> {
        // The one blocking fetch, before anything is drawn: without the
        // ground there is no scene to put anything in, and a renderer
        // that started without it would show an empty sky and then
        // rebuild everything a second later.
        let terrain = if version.capabilities.elevation {
            Some(client.terrain()?)
        } else {
            None
        };

        let latest = Arc::new(RwLock::new(Snapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let poller = spawn_poller(client.clone(), Arc::clone(&latest), Arc::clone(&stop));

        Ok(Self {
            client,
            version,
            terrain,
            latest,
            stop,
            _local: local,
            poller: Some(poller),
        })
    }

    /// The island as of the last poll.
    pub fn snapshot(&self) -> Snapshot {
        match self.latest.read() {
            Ok(current) => current.clone(),
            // The only writer is the poller, which holds the lock across
            // nothing that can fail, so a poisoned lock still holds a
            // whole snapshot.
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn terrain(&self) -> Option<&Terrain> {
        self.terrain.as_ref()
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.version.capabilities
    }

    pub fn version(&self) -> &ServerVersion {
        &self.version
    }

    /// The client, for the writes the application makes on demand --
    /// pausing, creating somebody, intervening. Reads go through the
    /// poller instead, so a frame never waits on one.
    pub fn client(&self) -> &IslandClient {
        &self.client
    }

    /// Where this island is, for a window title and a status line.
    pub fn address(&self) -> &str {
        self.client.base()
    }

    pub fn is_local(&self) -> bool {
        self._local.is_some()
    }
}

impl Drop for Island {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(poller) = self.poller.take() {
            // Joined rather than detached: the embedded backend is about
            // to be dropped too, and a thread still polling a server that
            // is going away would log a failure nobody can act on.
            let _ = poller.join();
        }
    }
}

fn spawn_poller(
    client: IslandClient,
    latest: Arc<RwLock<Snapshot>>,
    stop: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("island-poll".into())
        .spawn(move || {
            let mut slow_at = Instant::now() - SLOW_POLL;
            while !stop.load(Ordering::Relaxed) {
                let started = Instant::now();
                let mut next = {
                    let held = match latest.read() {
                        Ok(current) => current.clone(),
                        Err(poisoned) => poisoned.into_inner().clone(),
                    };
                    held
                };

                match client.world() {
                    Ok(world) => {
                        next.world = world;
                        next.trouble = None;
                        next.ever_read = true;
                    }
                    Err(err) => {
                        // Kept, not thrown away, and the previous world
                        // is kept with it: a connection that drops for a
                        // moment should dim the screen, not empty it.
                        next.trouble = Some(err.to_string());
                    }
                }

                if next.trouble.is_none() && slow_at.elapsed() >= SLOW_POLL {
                    slow_at = Instant::now();
                    if let Ok(properties) = client.properties() {
                        next.properties = Arc::new(properties);
                    }
                    if let Ok(economy) = client.economy() {
                        next.economy = Arc::new(economy);
                    }
                    if let Ok(timeline) = client.timeline() {
                        next.timeline = Arc::new(timeline);
                    }
                    if let Ok(conversations) = client.conversations() {
                        next.conversations = Arc::new(conversations);
                    }
                }

                match latest.write() {
                    Ok(mut current) => *current = next,
                    Err(poisoned) => *poisoned.into_inner() = next,
                }

                // Sleep what is left of the interval rather than a fixed
                // amount: on a slow link a request can take longer than
                // the interval, and sleeping anyway would compound it.
                let spent = started.elapsed();
                if spent < POLL {
                    // Broken into short naps so stopping is prompt. A
                    // quarter-second sleep would make closing the window
                    // wait for it.
                    let mut left = POLL - spent;
                    while left > Duration::ZERO && !stop.load(Ordering::Relaxed) {
                        let nap = left.min(Duration::from_millis(25));
                        std::thread::sleep(nap);
                        left -= nap;
                    }
                }
            }
        })
        .expect("a thread to poll the island")
}

/// An island running inside this process, behind a loopback backend.
///
/// The same `island serve` the command line runs: the same routes, the
/// same projection, the same simulation thread. Bound to port 0 on
/// loopback, so the operating system picks a free port and nothing off
/// this machine can reach it.
pub struct LocalBackend {
    pub address: String,
    stop: Arc<Mutex<Option<tokio::runtime::Runtime>>>,
    world: island::serve::sim::SimHandle,
}

impl LocalBackend {
    /// Start an island from a scenario or a snapshot, and serve it.
    pub fn start(
        scenario: Option<&std::path::Path>,
        snapshot: Option<&std::path::Path>,
        data_dir: &std::path::Path,
        speed: island::serve::sim::SimSpeed,
    ) -> Result<Self, String> {
        use island::serve::auth::ControlAuth;
        use island::serve::server::ServeConfig;
        use std::sync::Mutex as StdMutex;

        let mut life = island::run::open_world(scenario, snapshot)?;
        // Opt-in and off unless an operator set COMPUTER_ACTIONS_ENABLED,
        // exactly as `island serve` does it -- this is the same backend.
        if let Some(bridge) = mk_engine::humans::computer_bridge::HttpComputerBridge::from_env() {
            life.attach_computer_bridge(
                mk_engine::humans::computer_bridge::ComputerBridgeHandle::new(bridge),
            );
            eprintln!(
                "The computer service is attached: this island's founders may reach the real \
                 internet, and it is no longer reproducible."
            );
        }
        let world = island::serve::sim::spawn(life, speed);

        let (population, warnings) =
            island_humans::IslandHumanPopulation::open(data_dir, island_humans::DEFAULT_SEED)
                .map_err(|err| format!("could not open {}: {err}", data_dir.display()))?;
        for warning in warnings {
            eprintln!("warning: {warning}");
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|err| format!("could not start the local backend: {err}"))?;

        // Bound before the server is spawned, so the port is known by the
        // time this returns and the client has something to connect to.
        // Asking the runtime for it afterwards would be a race.
        let listener = runtime
            .block_on(async { tokio::net::TcpListener::bind("127.0.0.1:0").await })
            .map_err(|err| format!("could not bind a local port: {err}"))?;
        let port = listener
            .local_addr()
            .map_err(|err| format!("the local port has no address: {err}"))?
            .port();

        runtime.spawn(island::serve::server::serve_on(
            listener,
            Arc::new(StdMutex::new(population)),
            Some(world.clone()),
            ServeConfig {
                // Loopback, so this is the same rule `island serve`
                // applies on a loopback bind: this machine may write, and
                // nothing else can reach the port to try.
                control: ControlAuth::LoopbackOnly,
                // A line per request is right for a server somebody is
                // watching and noise in a desktop application polling it
                // four times a second.
                access_log: Some(false),
                ..ServeConfig::local(ControlAuth::LoopbackOnly)
            },
        ));

        Ok(Self {
            address: format!("http://127.0.0.1:{port}"),
            stop: Arc::new(Mutex::new(Some(runtime))),
            world,
        })
    }
}

impl Drop for LocalBackend {
    fn drop(&mut self) {
        self.world.stop();
        // Dropping the runtime stops the server with it. `shutdown_background`
        // rather than a plain drop, because a plain drop blocks until every
        // task finishes and one of them is an HTTP server that never does.
        if let Ok(mut held) = self.stop.lock() {
            if let Some(runtime) = held.take() {
                runtime.shutdown_background();
            }
        }
    }
}
