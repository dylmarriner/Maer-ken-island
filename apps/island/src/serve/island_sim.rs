//! The live island: one thread owns the `IslandWorldState`, paces it, and
//! publishes an immutable projection after every batch of steps. Readers
//! (HTTP handlers) only ever see the projection; writers send commands that
//! the sim thread validates against live state and queues for the next tick.

use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use mk_engine::io::island_snapshot::save_island_snapshot;
use mk_engine::regional::commands::{CommandError, IslandCommand, SimSpeed};
use mk_engine::regional::pacing::Pacer;
use mk_engine::regional::world::IslandWorldState;
use serde_json::{json, Value};

/// How long the sim thread waits for a request before it checks the pacer.
const IDLE_WAIT: Duration = Duration::from_millis(40);
/// How long an HTTP handler waits for the sim thread to answer.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(120);

/// An immutable picture of the island at one tick.
#[derive(Debug, Clone)]
pub struct IslandProjection {
    pub tick: u64,
    pub sim_time_s: f64,
    pub paused: bool,
    pub speed: SimSpeed,
    pub achieved_speed: f64,
    pub behind: bool,
    pub roster: Vec<Value>,
    pub store: Option<Value>,
    pub shortfalls: (u64, u64),
    pub snapshots_saved: u64,
    pub last_error: Option<String>,
    pub vegetation: Value,
    pub economy: Value,
    /// The last commands applied, newest last.
    pub timeline: Vec<Value>,
}

/// The estate layout: fixed for a run, so built once (the location picker).
pub fn estate_view(world: &IslandWorldState) -> Value {
    let layout = &world.life.placed.layout;
    let rect = |r: &mk_engine::regional::estate_layout::Rect| json!([r.x0, r.y0, r.x1, r.y1]);
    json!({
        "patch": {
            "origin_m": [layout.patch.origin_x_m, layout.patch.origin_y_m],
            "size_m": [layout.patch.width_m, layout.patch.height_m],
            "cell_size_m": layout.patch.cell_size_m,
        },
        "location_medium_cell": [world.life.placed.location.0, world.life.placed.location.1],
        "yard": rect(&layout.yard),
        "buildings": layout.buildings.iter().map(|b| json!({
            "building_id": b.building_id, "kind": format!("{:?}", b.kind), "rect_m": rect(&b.rect_m),
        })).collect::<Vec<_>>(),
        "spaces": layout.spaces.iter().map(|s| json!({
            "id": s.id.0, "label": s.label, "kind": format!("{:?}", s.kind), "rect_m": rect(&s.rect_m),
        })).collect::<Vec<_>>(),
    })
}

/// The starter property: fixed for a run, so built once.
pub fn properties_view(world: &IslandWorldState) -> Value {
    let placed = &world.life.placed;
    let layout = &placed.layout;
    json!([{
        "property_id": layout.property_id,
        "medium_cell": [placed.location.0, placed.location.1],
        "buildings": layout.buildings.len(),
        "spaces": layout.spaces.len(),
        "doors": layout.doors.len(),
        "items": layout.items.len(),
        "patch_size_m": [layout.patch.width_m, layout.patch.height_m],
    }])
}

impl IslandProjection {
    fn capture(
        world: &IslandWorldState,
        pacer: &Pacer,
        saved: u64,
        error: &Option<String>,
    ) -> Self {
        let life = &world.life;
        let roster = life
            .humans
            .registry
            .iter()
            .map(|h| {
                let id = h.agent_id();
                let at = life.positions.0.get(id);
                json!({
                    "agent_id": id,
                    "space": at.map(|p| format!("{:?}", p.space)),
                    "position_m": at.map(|p| [p.position_m.0, p.position_m.1]),
                })
            })
            .collect();
        Self {
            tick: world.tick(),
            sim_time_s: world.sim_time_seconds(),
            paused: life.commands.paused,
            speed: life.commands.speed,
            achieved_speed: pacer.achieved_speed(),
            behind: pacer.is_behind(),
            roster,
            store: world.human_store_status().map(|s| {
                json!({ "run_id": s.run_id, "syncs": s.syncs, "errors": s.errors, "last_error": s.last_error })
            }),
            shortfalls: (life.shortfalls.food, life.shortfalls.water),
            snapshots_saved: saved,
            last_error: error.clone(),
            vegetation: json!({
                "trees": life.vegetation.trees.len(),
                "tree_biomass_kgc": life.vegetation.trees.iter().map(|t| t.biomass_kgc).sum::<f64>(),
            }),
            economy: json!({
                "food_shortfalls": life.shortfalls.food,
                "water_shortfalls": life.shortfalls.water,
                "fuel_stores": life.energy.fuel_stores.len(),
                "generators": life.energy.generators.len(),
                "solar_arrays": life.energy.solar.len(),
                "batteries": life.energy.batteries.len(),
                "vehicles": life.energy.vehicles.len(),
            }),
            timeline: life
                .commands
                .log
                .iter()
                .rev()
                .take(50)
                .rev()
                .map(|e| json!({ "tick": e.tick, "sequence": e.sequence, "command": format!("{:?}", e.command).chars().take(120).collect::<String>() }))
                .collect(),
        }
    }

    pub fn status(&self) -> Value {
        json!({
            "tick": self.tick,
            "simulated_days": self.sim_time_s / 86_400.0,
            "paused": self.paused,
            "speed": format!("{:?}", self.speed),
            "achieved_speed": self.achieved_speed,
            "behind": self.behind,
            "population": self.roster.len(),
            "human_store": self.store,
            "food_shortfalls": self.shortfalls.0,
            "water_shortfalls": self.shortfalls.1,
            "snapshots_saved": self.snapshots_saved,
            "last_error": self.last_error,
        })
    }
}

enum Request {
    Command(IslandCommand, Sender<Result<u64, CommandError>>),
    Detail(String, Sender<Option<Value>>),
    Stop,
}

/// Handle to the sim thread. Cloning shares the thread.
#[derive(Clone)]
pub struct SimHandle {
    tx: Sender<Request>,
    projection: Arc<RwLock<Arc<IslandProjection>>>,
    estate: Arc<Value>,
    properties: Arc<Value>,
}

#[derive(Debug)]
pub enum SimError {
    Command(CommandError),
    /// The sim thread is gone or did not answer in time.
    Unavailable,
}

impl SimHandle {
    pub fn projection(&self) -> Arc<IslandProjection> {
        self.projection.read().expect("projection lock").clone()
    }

    /// Queue a command; returns its sequence number once validated.
    pub fn command(&self, command: IslandCommand) -> Result<u64, SimError> {
        let (reply, rx) = channel();
        self.tx
            .send(Request::Command(command, reply))
            .map_err(|_| SimError::Unavailable)?;
        match rx.recv_timeout(REPLY_TIMEOUT) {
            Ok(Ok(sequence)) => Ok(sequence),
            Ok(Err(e)) => Err(SimError::Command(e)),
            Err(_) => Err(SimError::Unavailable),
        }
    }

    /// One human's full state as JSON; `None` if there is no such human.
    pub fn detail(&self, agent_id: &str) -> Result<Option<Value>, SimError> {
        let (reply, rx) = channel();
        self.tx
            .send(Request::Detail(agent_id.to_string(), reply))
            .map_err(|_| SimError::Unavailable)?;
        rx.recv_timeout(REPLY_TIMEOUT)
            .map_err(|_| SimError::Unavailable)
    }

    pub fn estate(&self) -> Arc<Value> {
        self.estate.clone()
    }

    pub fn properties(&self) -> Arc<Value> {
        self.properties.clone()
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Request::Stop);
    }
}

/// Start the sim thread. `snapshot_path`, if set, is written whenever a
/// `Snapshot` control command is applied.
pub fn spawn(mut world: IslandWorldState, snapshot_path: Option<PathBuf>) -> SimHandle {
    let step_s = world.life.scenario.cadences.human_seconds;
    let mut pacer = Pacer::new(step_s);
    let projection = Arc::new(RwLock::new(Arc::new(IslandProjection::capture(
        &world, &pacer, 0, &None,
    ))));
    let (tx, rx) = channel::<Request>();
    let estate = Arc::new(estate_view(&world));
    let properties = Arc::new(properties_view(&world));
    let shared = projection.clone();
    std::thread::Builder::new()
        .name("island-sim".into())
        .spawn(move || {
            let (mut saved, mut handled, mut error) = (0u64, 0u64, None::<String>);
            let mut last = Instant::now();
            loop {
                match rx.recv_timeout(IDLE_WAIT) {
                    Ok(Request::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                    Ok(Request::Command(c, reply)) => {
                        let _ = reply.send(world.queue(c).map(|id| id.0));
                    }
                    Ok(Request::Detail(id, reply)) => {
                        let _ = reply.send(
                            world
                                .life
                                .humans
                                .registry
                                .get_human(&id)
                                .and_then(|h| serde_json::to_value(h).ok()),
                        );
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
                let now = Instant::now();
                let elapsed = now - last;
                last = now;
                let c = &world.life.commands;
                let due = if c.paused {
                    0
                } else {
                    pacer.steps_due(elapsed, c.speed)
                };
                let work = due > 0 || !c.pending.is_empty() || c.paused && c.step_credit > 0;
                if work {
                    let started = Instant::now();
                    let mut ran = 0;
                    let mut failed = None;
                    if due == 0 {
                        failed = world.step(0).err(); // apply waiting commands now
                    }
                    for _ in 0..due {
                        if let Err(e) = world.step(step_s) {
                            failed = Some(e);
                            break;
                        }
                        ran += 1;
                    }
                    pacer.record(ran, started.elapsed());
                    if let Some(e) = failed {
                        error = Some(e.to_string());
                    }
                    if let (Some(path), true) = (
                        &snapshot_path,
                        world.life.commands.snapshots_requested > handled,
                    ) {
                        handled = world.life.commands.snapshots_requested;
                        world.sync_human_store();
                        match save_island_snapshot(&world, path) {
                            Ok(()) => saved += 1,
                            Err(e) => error = Some(e.to_string()),
                        }
                    }
                    let p = Arc::new(IslandProjection::capture(&world, &pacer, saved, &error));
                    *shared.write().expect("projection lock") = p;
                }
            }
        })
        .expect("spawn the island sim thread");
    SimHandle {
        tx,
        projection,
        estate,
        properties,
    }
}
