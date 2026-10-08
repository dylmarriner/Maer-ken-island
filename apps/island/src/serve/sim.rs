//! Phase 4 Task 6: the island running behind the dashboard.
//!
//! The simulation owns its state on one thread and nobody else touches it.
//! After each step it publishes an [`IslandProjection`] behind an
//! `RwLock`; the HTTP handlers only ever read that. So a slow request can
//! never hold up a step, a burst of requests cannot slow the world down,
//! and no page can catch the island half-stepped.
//!
//! Pacing is the other half. The simulated clock is not the real clock:
//! speed decides only how *often* a fixed step runs, never how large it is,
//! so an island run fast and an island run slowly pass through exactly the
//! same states. Wall-clock time never reaches simulation state — if it did,
//! a busy machine would quietly produce a different world, and the digest
//! the whole project is built on would stop meaning anything.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use mk_engine::regional::commands::{Applied, CommandError, ControlCommand};
use mk_engine::regional::create_human::{CreateHumanError, CreatedIslander};
use mk_engine::regional::interventions::{IslandApplied, IslandDirective};
use mk_engine::regional::life::IslandLife;

use super::projection::{Digest, IslandProjection, Views};

/// Something the world is asked to do.
///
/// The engine's own command vocabulary, not a second one: a command the
/// dashboard applies is recorded in the island's replay log exactly as one
/// applied anywhere else, so a dashboard session can be replayed.
///
/// Writes never touch the island directly. They are queued here and applied
/// by the thread that owns it, between steps, so the world is only ever
/// changed from one place and a request can never land in the middle of a
/// step.
pub use mk_engine::regional::commands::IslandCommand;

/// What became of a command.
///
/// Defined in `mk_island_api` and re-exported, so the desktop client polling
/// `/api/world/commands/<id>` across a network matches on exactly the
/// variants this loop writes.
pub use mk_island_api::Outcome;

/// A queued command and the id it will be answered under.
struct Queued {
    id: u64,
    command: IslandCommand,
}

/// How fast simulated time should pass against real time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimSpeed {
    /// One simulated second per real second. A 36-hour local day takes 36
    /// real hours, which is the honest setting for watching people live.
    RealTime,
    /// `k` simulated seconds per real second.
    Times(u32),
    /// Flat out, for a machine that is catching up or a test that is not
    /// watching.
    AsFastAsPossible,
}

impl SimSpeed {
    /// How long a step of `step_seconds` should take in real time, or
    /// `None` when it should not be waited for at all.
    fn budget(self, step_seconds: u64) -> Option<Duration> {
        match self {
            Self::AsFastAsPossible => None,
            Self::RealTime => Some(Duration::from_secs(step_seconds)),
            Self::Times(0) => Some(Duration::from_secs(step_seconds)),
            Self::Times(k) => Some(Duration::from_secs_f64(step_seconds as f64 / f64::from(k))),
        }
    }

    /// How it reads on the page.
    pub fn describe(self) -> String {
        match self {
            Self::RealTime => "real time".into(),
            Self::Times(k) => format!("{k}x real time"),
            Self::AsFastAsPossible => "as fast as possible".into(),
        }
    }
}

impl std::str::FromStr for SimSpeed {
    type Err = String;

    /// `real`, `max`, or a multiplier like `60` or `1440x`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        match text {
            "real" | "realtime" | "real-time" | "1" | "1x" => Ok(Self::RealTime),
            "max" | "fast" | "unpaced" => Ok(Self::AsFastAsPossible),
            other => other
                .trim_end_matches('x')
                .parse::<u32>()
                .map(Self::Times)
                .map_err(|_| {
                    format!("{other:?} is not a speed: use `real`, `max`, or a multiplier like 60")
                }),
        }
    }
}

/// How the island is being run, as against what it is.
///
/// Nothing here reaches simulation state: pausing leaves exactly the state
/// it was paused in, and a speed decides only how often a fixed step runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pacing {
    pub speed: SimSpeed,
    pub paused: bool,
    /// Steps a `Step` request still owes. While it is non-zero the island
    /// advances even though it is paused, and holds again when it reaches
    /// zero — which is what "advance exactly n and stop" means on a loop
    /// that is otherwise standing still.
    pub steps_owed: u64,
}

/// How many command outcomes are remembered.
///
/// Long enough that a page polling every couple of seconds will always find
/// its own, short enough that a dashboard left running for a week does not
/// grow without bound.
const OUTCOMES_KEPT: usize = 256;

/// The dashboard's end of the simulation.
#[derive(Clone)]
pub struct SimHandle {
    projection: Arc<RwLock<IslandProjection>>,
    stop: Arc<AtomicBool>,
    /// How fast to run, and whether to run at all. Read by the loop each
    /// iteration rather than captured at startup, so an operator can change
    /// speed or pause without restarting the island — and neither changes
    /// the world, only how often a fixed step happens.
    pacing: Arc<Mutex<Pacing>>,
    commands: Sender<Queued>,
    outcomes: Arc<Mutex<BTreeMap<u64, Outcome>>>,
    next_id: Arc<AtomicU64>,
    /// The island's terrain, copied once at startup.
    ///
    /// Terrain cannot change: the island refuses `SculptTerrain` and
    /// `SmoothTerrain`. So the grids are copied here once and a click on
    /// the map is answered from them, without the sim thread ever being
    /// asked.
    terrain: Arc<crate::serve::projection::Terrain>,
    /// The island's elevation map, rendered once as a PNG.
    ///
    /// Terrain does not move: the island refuses `SculptTerrain` and
    /// `SmoothTerrain`, so the picture taken at startup stays true for the
    /// life of the process. Rendering it once and handing out an `Arc`
    /// keeps it off the sim thread's path entirely.
    map_png: Arc<Vec<u8>>,
    /// The island's standing vegetation, and the estate patch's, as PNGs.
    ///
    /// Unlike the elevation render these go stale: biomass grows. They are
    /// redrawn on the stems' cadence, behind an `RwLock` the HTTP handlers
    /// only ever read, for the same reason everything else here is --
    /// nobody but the sim thread may touch the island.
    vegetation_png: Arc<RwLock<Arc<Vec<u8>>>>,
    patch_png: Arc<RwLock<Arc<Vec<u8>>>>,
    /// Why the replay log last failed to write, if it did.
    ///
    /// The log is what makes a run reproducible, and the timeline the
    /// dashboard serves is read from the copy in memory — so when writing
    /// stops working the page goes on showing commands that are not on
    /// disk, and says nothing. This used to go to stderr alone, which in a
    /// server process is nowhere. A full disk is not hypothetical: it
    /// happened twice while this branch was being written.
    log_error: Arc<Mutex<Option<String>>>,
}

impl SimHandle {
    /// The most recently published island.
    ///
    /// A poisoned lock is recovered from rather than propagated: the only
    /// writer is the sim thread, which publishes a freshly built projection
    /// and holds the lock across nothing that can fail, so the worst a
    /// panic elsewhere leaves behind is a projection one step old.
    pub fn projection(&self) -> IslandProjection {
        match self.projection.read() {
            Ok(current) => current.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Ask the thread to finish its current step and stop.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// What is at one medium-grid cell, or `None` off the island.
    ///
    /// Answered from the terrain snapshot taken at startup, never from the
    /// island: the sim thread owns that and nothing else may touch it. A
    /// first attempt at this reached for a `life` field on the handle,
    /// which does not exist and should not -- that separation is the whole
    /// point of the projection.
    pub fn cell(&self, row: usize, col: usize) -> Option<crate::serve::projection::Cell> {
        crate::serve::projection::cell_of(&self.terrain, row, col)
    }

    /// The island's elevation map as a PNG, rendered once at startup.
    pub fn map_png(&self) -> Arc<Vec<u8>> {
        Arc::clone(&self.map_png)
    }

    /// The island's standing vegetation as a PNG, at one pixel per medium
    /// cell, as of the last redraw.
    pub fn vegetation_png(&self) -> Arc<Vec<u8>> {
        match self.vegetation_png.read() {
            Ok(it) => Arc::clone(&it),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// The estate's 4 km patch as a PNG, at one pixel per 5 m stand cell.
    pub fn patch_png(&self) -> Arc<Vec<u8>> {
        match self.patch_png.read() {
            Ok(it) => Arc::clone(&it),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// The individual stems standing inside a box of domain metres.
    ///
    /// Read off the published projection and the startup terrain, never
    /// off the island: the same rule as `cell`. The stems are behind an
    /// `Arc`, so this clones a pointer and then walks it, and the sim
    /// thread is free to publish a newer set underneath in the meantime --
    /// the caller just answers from the one it took.
    pub fn trees_in(
        &self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        cap: usize,
    ) -> crate::serve::projection::Trees {
        let stems = match self.projection.read() {
            Ok(current) => Arc::clone(&current.trees),
            Err(poisoned) => Arc::clone(&poisoned.into_inner().trees),
        };
        crate::serve::projection::trees_in(&self.terrain, &stems, x0, y0, x1, y1, cap)
    }

    /// Why the replay log last failed to write, if it did. `None` means the
    /// last attempt succeeded, or that no log was asked for.
    pub fn replay_log_error(&self) -> Option<String> {
        match self.log_error.lock() {
            Ok(it) => it.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// How the island is being run right now.
    pub fn pacing(&self) -> Pacing {
        *self.locked_pacing()
    }

    /// Run at a different speed from the next step on.
    pub fn set_speed(&self, speed: SimSpeed) {
        self.locked_pacing().speed = speed;
    }

    /// Stop stepping, or start again. The world does not move while paused
    /// and is exactly as it was when it resumes.
    pub fn set_paused(&self, paused: bool) {
        let mut pacing = self.locked_pacing();
        pacing.paused = paused;
        // Resuming cancels a half-finished `Step`: somebody who asks the
        // island to run has asked for more than the three steps they had
        // left owing, and pausing cancels it because a pause is a stop.
        pacing.steps_owed = 0;
    }

    /// Advance exactly `ticks` more steps, then hold.
    pub fn step_for(&self, ticks: u64) {
        let mut pacing = self.locked_pacing();
        pacing.paused = true;
        pacing.steps_owed = ticks;
    }

    fn locked_pacing(&self) -> std::sync::MutexGuard<'_, Pacing> {
        match self.pacing.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Queue a command and give back the id it will be answered under.
    ///
    /// Returns as soon as it is queued — the island applies it before its
    /// next step. `None` means the simulation has stopped and nothing more
    /// will be applied, which the caller should say rather than leave
    /// somebody polling an id that will never resolve.
    pub fn send(&self, command: IslandCommand) -> Option<u64> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.remember(id, Outcome::Queued);
        self.commands.send(Queued { id, command }).ok()?;
        Some(id)
    }

    /// What became of a command, if it is still remembered.
    pub fn outcome(&self, id: u64) -> Option<Outcome> {
        self.locked_outcomes().get(&id).cloned()
    }

    fn remember(&self, id: u64, outcome: Outcome) {
        let mut outcomes = self.locked_outcomes();
        outcomes.insert(id, outcome);
        while outcomes.len() > OUTCOMES_KEPT {
            let oldest = *outcomes.keys().next().expect("not empty");
            outcomes.remove(&oldest);
        }
    }

    /// The only writers are this handle and the sim thread, and neither
    /// holds the lock across anything that can panic, so a poisoned lock is
    /// recovered from rather than propagated.
    fn locked_outcomes(&self) -> std::sync::MutexGuard<'_, BTreeMap<u64, Outcome>> {
        match self.outcomes.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// How far back the achieved-speed readout looks.
///
/// Measured over wall-clock time rather than a count of steps, and that is
/// not a detail. The steps are not all alike: one in every `DIGEST_EVERY`
/// also hashes the island, which costs two orders of magnitude more than
/// the others. A window of the last N *steps* usually falls between two of
/// those and reports a speed the island is not actually sustaining — the
/// first version of this reported 8,930x while the island was advancing at
/// 2,160x. A window of the last few seconds cannot miss them.
const SPEED_WINDOW: Duration = Duration::from_secs(5);

/// How often the canonical digest is recomputed, in steps.
///
/// A digest is **800 ms**, because hashing walks both 1,152,000-cell grids
/// and every human's canonical JSON. It is not the stems, which is why
/// `slow_what_a_step_and_a_digest_cost` can measure it on a 50-tree patch.
///
/// **The step it is compared against must not be measured there.** That
/// test's step figure is 1.2 ms on its fifty trees; the real island, with
/// 199,997 of them, steps at **4.3 ms** (`benchmarks/phase4_cost.md`), and
/// `benchmarks/island_week.md` implied it all along at 49 s for 10,080
/// steps. An earlier version of this comment said "650 times more" from
/// the toy step. On the real island a digest is about **186x** a step —
/// still enough that hashing every one made the island run far slower than
/// it needed to, for a number nobody reads that often.
///
/// Sixty steps is one simulated hour at the default cadence. The digest
/// still dominates the loop — 13.3 ms amortised per step against 4.3 ms of
/// actual simulation — which caps the island near 3,400x real time. That is
/// far above the `RealTime` default and enough for a dashboard.
///
/// Raising it would buy simulation speed and lose digest freshness, and
/// nothing else — the digest is a read of existing state, so how often it
/// is taken cannot change the island. It is a constant rather than a knob:
/// a settable version was written while chasing a flaky test, did not fix
/// the flake, and ended up with no caller at all, so it went out again
/// rather than sit here as an unreachable method promising a tuning nobody
/// could reach. Changing the cadence still means changing this line.
const DIGEST_EVERY: u64 = 60;

/// Steps between refreshes of the published views -- the people, the
/// economy and the conversation feed.
///
/// One simulated hour, the same as the digest's default, but for a quite
/// different reason: these are cheap, and the number is about how stale a
/// dashboard may be rather than about what the loop can afford. They used
/// to be refreshed inside the digest's cadence check, which coupled "how
/// fresh is the page" to "how expensive is a hash" and meant that turning
/// the digest down turned the page stale.
const VIEWS_EVERY: u64 = 60;

/// Steps between refreshes of the published stems.
///
/// Its own cadence, and a far slower one, because this copy is nothing
/// like the others: the patch holds about 200,000 `TreeInstance`s at 56
/// bytes apiece, so republishing them is an eleven-megabyte memcpy, where
/// the whole conversation feed is forty short strings. `tests/tree_cost.rs`
/// measures what it actually costs against a step.
///
/// A simulated day is the right staleness for what it shows. Trees are the
/// slowest thing on the island -- a stem puts on millimetres in a year --
/// so a map of them an hour or a day old is the same map. Nothing a person
/// does through the dashboard plants or fells one: the commands that exist
/// place structures and people, and those ride the fast views.
const TREES_EVERY: u64 = 1_440;

/// Start an island running on its own thread.
///
/// Returns as soon as the thread is spawned, with a projection already
/// published for the island's starting state, so a page loaded immediately
/// shows the world at tick 0 rather than an empty one.
pub fn spawn(life: IslandLife, speed: SimSpeed) -> SimHandle {
    spawn_logging(life, speed, None)
}

/// Start an island running, writing its replay log to `log` as commands
/// land.
///
/// Written on each command rather than at the end: a log that only existed
/// in memory would be lost by the thing a log is for, which is a run that
/// stopped unexpectedly.
pub fn spawn_logging(life: IslandLife, speed: SimSpeed, log: Option<PathBuf>) -> SimHandle {
    spawn_with(life, speed, log, None)
}

/// As [`spawn_logging`], and `snapshots` is the directory
/// `ControlCommand::Snapshot` writes into.
pub fn spawn_with(
    life: IslandLife,
    speed: SimSpeed,
    log: Option<PathBuf>,
    snapshots: Option<PathBuf>,
) -> SimHandle {
    let step_seconds = life.scenario.cadences.human_seconds;
    // The starting island is hashed once — and only once: hashing costs
    // about 800 ms against a 4.3 ms step on the real island
    // (`benchmarks/phase4_cost.md`), so the thread is handed this one
    // rather than computing its own. This comment said "930 ms against a
    // 1.4 ms step" until a reviewer caught it: both figures were ones I had
    // already corrected elsewhere in this branch and missed here.
    // Drawn once, here, while this thread still owns the island and before
    // the sim thread takes it. `island_preview` already paints this exact
    // picture for the headless galleries.
    let map_png =
        Arc::new(island_preview::elevation_image(&life.domain, &life.physical.geophysics).png());
    let terrain = Arc::new(crate::serve::projection::Terrain::of(&life));
    // The vegetation layers, likewise drawn here and then redrawn by the
    // thread on the stems' cadence. Unlike the elevation they move:
    // biomass grows, and a page that drew the island's vegetation once at
    // bootstrap would be showing last year's forest for ever.
    let vegetation_png = Arc::new(RwLock::new(Arc::new(
        island_preview::biomass_image(&life.domain, &life.ecology, &life.physical.geophysics).png(),
    )));
    let patch_png = Arc::new(RwLock::new(Arc::new(
        island_preview::stand_image(&life.vegetation).png(),
    )));
    let first = IslandProjection::digest_now(&life);
    let views = Views::of(&life);
    let projection = Arc::new(RwLock::new(IslandProjection::of(
        &life,
        &speed.describe(),
        None,
        first.clone(),
        views.clone(),
    )));
    let (commands, inbox) = channel();
    let handle = SimHandle {
        projection: Arc::clone(&projection),
        stop: Arc::new(AtomicBool::new(false)),
        pacing: Arc::new(Mutex::new(Pacing {
            speed,
            paused: false,
            steps_owed: 0,
        })),
        commands,
        outcomes: Arc::new(Mutex::new(BTreeMap::new())),
        next_id: Arc::new(AtomicU64::new(1)),
        log_error: Arc::new(Mutex::new(None)),
        map_png,
        terrain,
        vegetation_png,
        patch_png,
    };
    let stop = Arc::clone(&handle.stop);
    let outcomes = Arc::clone(&handle.outcomes);
    let pacing = Arc::clone(&handle.pacing);
    let log_error = Arc::clone(&handle.log_error);
    let vegetation_png = Arc::clone(&handle.vegetation_png);
    let patch_png = Arc::clone(&handle.patch_png);

    std::thread::Builder::new()
        .name("island-sim".into())
        .spawn(move || {
            run(Loop {
                life,
                pacing,
                step_seconds,
                projection,
                stop,
                digest: first,
                views,
                inbox,
                outcomes,
                log,
                log_error,
                snapshots,
                vegetation_png,
                patch_png,
            })
        })
        .expect("the simulation thread starts");

    handle
}

/// Everything the thread owns.
struct Loop {
    life: IslandLife,
    pacing: Arc<Mutex<Pacing>>,
    step_seconds: u64,
    projection: Arc<RwLock<IslandProjection>>,
    stop: Arc<AtomicBool>,
    digest: Digest,
    views: Views,
    inbox: Receiver<Queued>,
    outcomes: Arc<Mutex<BTreeMap<u64, Outcome>>>,
    log: Option<PathBuf>,
    /// Shared with the handle, so a failed write reaches the dashboard
    /// rather than only stderr.
    log_error: Arc<Mutex<Option<String>>>,
    /// Shared with the handle, and redrawn on the stems' cadence.
    vegetation_png: Arc<RwLock<Arc<Vec<u8>>>>,
    patch_png: Arc<RwLock<Arc<Vec<u8>>>>,
    /// Where `ControlCommand::Snapshot` writes. Without one the command is
    /// *refused*, because the alternative is what this used to do: record
    /// "wrote a snapshot" in the timeline and write nothing.
    snapshots: Option<PathBuf>,
}

/// The loop itself: apply what was asked, step, publish, wait if there is
/// time to spare.
fn run(mut it: Loop) {
    // (when, simulated time then), oldest first. Speed is the simulated
    // time between the ends of the window over the real time between them.
    let mut recent: std::collections::VecDeque<(Instant, u64)> = std::collections::VecDeque::new();

    while !it.stop.load(Ordering::Relaxed) {
        // Timed around the whole iteration, not just the step. Publishing
        // and applying commands are real work the island has to do before
        // it can step again, and a readout that left them out would have
        // claimed the island was keeping up while it ran much slower.
        let started = Instant::now();

        // Commands first, and *before* the pause gate. A creation applies
        // at the tick it was queued for, before the step that then moves
        // that person along with everybody else — and applying a command
        // does not advance the clock, so a paused island can still be
        // asked things.
        //
        // The gate used to come first. That meant a paused island drained
        // nothing: a `Resume` queued as an intervention sat in the inbox
        // behind the pause it was meant to lift, and so did any creation
        // made while paused. Running it is what showed it — the island
        // paused on request and then would not come back.
        let mut created_somebody = false;
        let mut applied_anything = false;
        while let Ok(Queued { id, command }) = it.inbox.try_recv() {
            let outcome = apply(&mut it.life, &it.pacing, it.snapshots.as_ref(), command);
            created_somebody |= matches!(outcome, Outcome::Created { .. });
            applied_anything |= !matches!(outcome, Outcome::Refused { .. });
            record(&it.outcomes, id, outcome);
        }
        // Read after the drain, not before: an operator may have changed
        // the speed or paused since the last iteration, and one of the
        // commands just applied may itself have been a pause, a resume or
        // a step. This iteration should honour that rather than a reading
        // taken before it landed.
        let pacing = *match it.pacing.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let stepping = !pacing.paused || pacing.steps_owed > 0;
        if applied_anything {
            if let Some(path) = &it.log {
                // A failure here loses the record, not the world: carry on,
                // as the folders do — but say so somewhere a person will
                // look. The island keeps running and the timeline keeps
                // serving from memory, so without this the page shows
                // commands that are not on disk and gives no sign.
                let outcome = match it.life.replay_log().save(path) {
                    Ok(()) => None,
                    Err(e) => {
                        let said = format!(
                            "the replay log could not be written to {}: {e}",
                            path.display()
                        );
                        eprintln!("{said}");
                        Some(said)
                    }
                };
                match it.log_error.lock() {
                    Ok(mut slot) => *slot = outcome,
                    Err(poisoned) => *poisoned.into_inner() = outcome,
                }
            }
        }
        if created_somebody {
            // Somebody who has just been created should be readable now,
            // not at the next hourly refresh. Waiting would mean creating a
            // person and then being told they do not exist.
            it.views.records = IslandProjection::records_now(&it.life);
            it.views.records_at_tick = it.life.tick;
        }
        if applied_anything {
            // The same argument for the other two: a structure somebody
            // just built, and the record of what they just did, are the
            // things they are about to go and look at.
            it.views.economy = IslandProjection::economy_now(&it.life);
            it.views.economy_at_tick = it.life.tick;
            it.views.timeline = IslandProjection::timeline_now(&it.life);
        }

        if !stepping {
            // Nothing moves, and the clock is left exactly where it was,
            // which is the truth: the island is where it was paused. The
            // speed window is cleared so that resuming does not report a
            // speed measured across the pause.
            recent.clear();
            if applied_anything {
                // Something did change — somebody was created, or an
                // intervention landed — so the page has to be told, even
                // though the clock did not move. A paused island that
                // quietly held a stale projection would show an operator
                // a world without the person they just made.
                publish(
                    &it.projection,
                    &it.life,
                    &pacing.speed,
                    None,
                    it.digest.clone(),
                    it.views.clone(),
                );
            }
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        if let Err(e) = it.life.advance(it.step_seconds) {
            // The island cannot continue — an audit that did not close, a
            // step that failed. Publishing a stale projection forever would
            // be a lie, so the thread stops, says why on the console the
            // operator is already watching, and marks the projection as no
            // longer running so the page says so too.
            eprintln!("the island stopped at t={} s: {e}", it.life.sim_time_s);
            if let Ok(mut slot) = it.projection.write() {
                slot.running = false;
            }
            break;
        }

        if pacing.steps_owed > 0 {
            // One of the steps this iteration owed has been taken. When
            // the last is paid the island is simply paused, which is what
            // `Step` asks for: advance exactly n, then hold.
            let mut owed = match it.pacing.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            owed.steps_owed = owed.steps_owed.saturating_sub(1);
        }

        // The views and the digest are on separate cadences, and that is
        // not tidiness: they were one block until turning the digest off
        // in the tests silently turned the conversation feed off with it,
        // because the feed had been refreshed inside the digest's `if`.
        // They share no cost and no purpose. Reading a handful of totals
        // and copying at most forty short conversations is cheap; hashing
        // two 1,152,000-cell grids is 186 steps' worth of work.
        if it.life.tick.is_multiple_of(VIEWS_EVERY) {
            it.views.records = IslandProjection::records_now(&it.life);
            it.views.records_at_tick = it.life.tick;
            it.views.economy = IslandProjection::economy_now(&it.life);
            it.views.economy_at_tick = it.life.tick;
            // Conversations ride the same cadence as the records of the
            // people having them, which is a coherent rule and, more to
            // the point, the only one that works.
            //
            // Refreshing them every step was tried first, on the argument
            // that they are the liveliest thing the page shows and no
            // command causes them. That argument was right and the change
            // was still wrong. A step is not a fixed amount of work: the
            // island runs at `AsFastAsPossible` here and in every test,
            // where a step on a small island is microseconds, while this
            // copy costs about the same 17 us whatever the island holds,
            // because it is sized by the conversations and not by the
            // world. Per step, across the test suite's concurrent sim
            // threads, that was enough allocator churn to starve the HTTP
            // runtime: `a_new_person_can_be_read_the_moment_they_exist`
            // missed its sixty-second deadline, reproducibly, and passed
            // again the moment this came off the per-step path.
            //
            // The 0.98% of a step measured by `tests/conversation_cost.rs`
            // is real and was the wrong thing to reason from: it is a
            // share of a *full* island's 1.7 ms step, not of the fast ones
            // that actually set the loop's pace.
            it.views.conversations = IslandProjection::conversations_now(&it.life);
        }

        if it.life.tick.is_multiple_of(TREES_EVERY) {
            it.views.trees = IslandProjection::trees_now(&it.life);
            // The vegetation layers, on the same cadence and for the same
            // reason: they are the only things on the map that change and
            // are expensive to draw. The lock is held for the swap alone,
            // never across the render, so a request can never wait on a
            // million-cell repaint.
            let island = Arc::new(
                island_preview::biomass_image(
                    &it.life.domain,
                    &it.life.ecology,
                    &it.life.physical.geophysics,
                )
                .png(),
            );
            let patch = Arc::new(island_preview::stand_image(&it.life.vegetation).png());
            match it.vegetation_png.write() {
                Ok(mut slot) => *slot = island,
                Err(poisoned) => *poisoned.into_inner() = island,
            }
            match it.patch_png.write() {
                Ok(mut slot) => *slot = patch,
                Err(poisoned) => *poisoned.into_inner() = patch,
            }
        }

        if it.life.tick.is_multiple_of(DIGEST_EVERY) {
            it.digest = IslandProjection::digest_now(&it.life);
        }

        let now = Instant::now();
        recent.push_back((now, it.life.sim_time_s));
        while recent
            .front()
            .is_some_and(|(at, _)| now.duration_since(*at) > SPEED_WINDOW)
            && recent.len() > 2
        {
            recent.pop_front();
        }
        publish(
            &it.projection,
            &it.life,
            &pacing.speed,
            achieved_speed(&recent),
            it.digest.clone(),
            it.views.clone(),
        );

        let took = started.elapsed();

        // Only ever sleep off time that is left over. A step that overran
        // its budget is not slept on at all, so the island catches up where
        // it can rather than falling further behind.
        if let Some(budget) = pacing.speed.budget(it.step_seconds) {
            if let Some(spare) = budget.checked_sub(took) {
                // Woken often enough that a stop is acted on promptly even
                // at real time, where a budget is a whole minute.
                let deadline = Instant::now() + spare;
                while Instant::now() < deadline && !it.stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50).min(deadline - Instant::now()));
                }
            }
        }
    }
}

/// Do what was asked of the world, on the thread that owns it.
///
/// Through `apply_command`, so the island records it: a person created from
/// the dashboard is in the replay log beside one created from the command
/// line, and a dashboard session replays like any other run.
/// `pacing` is here because an intervention can be a run-loop control.
/// Upstream's executor hands `Pause` back as a directive for the host to
/// carry out, and this is the host: a directive that were only reported
/// and not acted on would be a dashboard that accepted a pause and kept
/// running.
fn apply(
    life: &mut IslandLife,
    pacing: &Mutex<Pacing>,
    snapshots: Option<&PathBuf>,
    command: IslandCommand,
) -> Outcome {
    // `Snapshot` is the one control that does touch the world's surroundings,
    // so it is carried out here rather than left to `apply_command`, which
    // records every control as having moved nothing. Before this it *was*
    // left there: the timeline said "wrote a snapshot" and no snapshot was
    // written, which is worse than the command not existing.
    if let IslandCommand::Control(ControlCommand::Snapshot) = &command {
        let Some(dir) = snapshots else {
            return Outcome::Refused {
                problems: vec![
                    "This dashboard has nowhere to write a snapshot. Start the server with \
                     --snapshot-dir <dir>."
                        .to_string(),
                ],
            };
        };
        let path = dir.join(format!("island-{:08}.mks", life.tick));
        if let Err(e) = std::fs::create_dir_all(dir) {
            return Outcome::Refused {
                problems: vec![format!("{} could not be made: {e}", dir.display())],
            };
        }
        // Timed, because it is slow enough to matter: measured at 27 s for
        // the full island (`benchmarks/phase4_cost.md`), during which this
        // thread is not stepping. The duration goes back to whoever asked,
        // so a stopped clock has an explanation.
        let started = Instant::now();
        let saved = mk_engine::io::island_snapshot::save_island_snapshot(life, &path);
        let took = started.elapsed();
        return match saved {
            Err(e) => Outcome::Refused {
                problems: vec![format!("the snapshot was not written: {e}")],
            },
            Ok(()) => {
                let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                // Recorded only now that it happened. The result is not
                // checked because a `Control` command cannot fail —
                // `apply_command`'s arm for it records and returns
                // `Ok(Applied::Noted)` with no fallible step in between —
                // and `.expect()` here would put a panic on the thread that
                // owns the island for something that cannot occur. If that
                // ever stops being true, this match stops compiling.
                match life.apply_command(command) {
                    Ok(_) => {}
                    Err(e) => eprintln!("the snapshot was written but not recorded: {e}"),
                }
                Outcome::Saved {
                    path: path.display().to_string(),
                    bytes,
                    took_ms: took.as_millis() as u64,
                    tick: life.tick,
                }
            }
        };
    }
    match life.apply_command(command) {
        Ok(Applied::Created(CreatedIslander {
            agent_id,
            space,
            cell,
            storage_error,
            ..
        })) => Outcome::Created {
            agent_id,
            space,
            cell,
            tick: life.tick,
            storage_error,
        },
        Ok(Applied::Intervened(IslandApplied::Mutated { summary, touched })) => {
            Outcome::Intervened {
                summary,
                touched,
                tick: life.tick,
            }
        }
        Ok(Applied::Intervened(IslandApplied::Control { directive })) => {
            let summary = match directive {
                IslandDirective::Pause => {
                    let mut pacing = pacing.lock().unwrap_or_else(|e| e.into_inner());
                    pacing.paused = true;
                    pacing.steps_owed = 0;
                    "paused".to_string()
                }
                IslandDirective::Resume => {
                    let mut pacing = pacing.lock().unwrap_or_else(|e| e.into_inner());
                    pacing.paused = false;
                    pacing.steps_owed = 0;
                    "resumed".to_string()
                }
                // Upstream's `Step` advances exactly `ticks` and then
                // pauses, and so does this: the loop pays the budget down
                // one step per iteration and holds when it reaches zero.
                IslandDirective::Step { ticks } => {
                    let mut pacing = pacing.lock().unwrap_or_else(|e| e.into_inner());
                    pacing.paused = true;
                    pacing.steps_owed = ticks;
                    format!("stepping {ticks} and then holding")
                }
            };
            Outcome::Intervened {
                summary,
                touched: 0,
                tick: life.tick,
            }
        }
        Ok(Applied::Noted) => Outcome::Noted,
        Err(CommandError::Create(CreateHumanError::Invalid(problems))) => {
            Outcome::Refused { problems }
        }
        Err(e) => Outcome::Refused {
            problems: vec![e.to_string()],
        },
    }
}

fn record(outcomes: &Mutex<BTreeMap<u64, Outcome>>, id: u64, outcome: Outcome) {
    let mut outcomes = match outcomes.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    outcomes.insert(id, outcome);
    while outcomes.len() > OUTCOMES_KEPT {
        let oldest = *outcomes.keys().next().expect("not empty");
        outcomes.remove(&oldest);
    }
}

/// Simulated seconds per real second across the window.
///
/// This is what somebody watching experiences: how fast the island's clock
/// moves against theirs. It counts everything the loop does — stepping,
/// publishing, hashing, and any waiting the pacing imposed — because all of
/// it is time in which the island did not advance further.
fn achieved_speed(recent: &std::collections::VecDeque<(Instant, u64)>) -> Option<f64> {
    let (first_at, first_sim) = *recent.front()?;
    let (last_at, last_sim) = *recent.back()?;
    let real = last_at.duration_since(first_at).as_secs_f64();
    if real <= 0.0 {
        return None;
    }
    Some(last_sim.saturating_sub(first_sim) as f64 / real)
}

#[allow(clippy::too_many_arguments)]
fn publish(
    projection: &RwLock<IslandProjection>,
    life: &IslandLife,
    speed: &SimSpeed,
    achieved: Option<f64>,
    digest: Digest,
    views: Views,
) {
    let next = IslandProjection::of(life, &speed.describe(), achieved, digest, views);
    match projection.write() {
        Ok(mut slot) => *slot = next,
        Err(poisoned) => *poisoned.into_inner() = next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }

    /// The default island with a small patch: nothing here depends on the
    /// stems, and 200,000 of them make every bootstrap slow.
    pub(super) fn island() -> IslandLife {
        let mut scenario =
            mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json"))
                .unwrap();
        scenario.estate_patch.tree_cap = 50;
        let canon = std::sync::Arc::new(
            mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap(),
        );
        IslandLife::bootstrap(scenario, canon).expect("the island bootstraps")
    }

    /// Wait for the thread to publish a projection at or past `tick`.
    ///
    /// Bounded, because a test that hangs tells nobody anything.
    fn wait_for_tick(handle: &SimHandle, tick: u64) -> IslandProjection {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let current = handle.projection();
            if current.clock.tick >= tick || !current.running {
                return current;
            }
            assert!(
                Instant::now() < deadline,
                "the island never reached tick {tick} (stopped at {})",
                current.clock.tick
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// The digest a plain `advance` reaches after `ticks` steps.
    fn direct_digest(ticks: u64) -> String {
        let mut life = island();
        let step = life.scenario.cadences.human_seconds;
        life.advance(ticks * step).unwrap();
        life.state_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn how_fast_it_runs_does_not_change_what_happens() {
        // The whole promise of pacing: speed decides how often a fixed step
        // runs, never how big it is. If a thread's island ever differs from
        // the one a plain run reaches, wall-clock time has got into
        // simulation state and no digest means anything.
        //
        // Each thread is checked against a direct run *at its own tick*,
        // both read from one projection so the tick and the digest cannot
        // disagree. Comparing two threads to each other does not work: the
        // unpaced one races past whatever tick the paced one is on, and the
        // comparison is then between two different moments. That is what
        // the first version of this test did, and it failed for that reason
        // rather than for anything wrong in the loop.
        // Past the first digest refresh, so what is checked is a digest the
        // loop recomputed rather than the one it started with.
        for speed in [SimSpeed::AsFastAsPossible, SimSpeed::Times(100_000)] {
            let handle = spawn(island(), speed);
            let seen = wait_for_tick(&handle, DIGEST_EVERY + 1);
            handle.stop();

            assert!(seen.running, "the island stopped early at {speed:?}");
            assert!(seen.clock.tick > DIGEST_EVERY);
            assert_eq!(
                seen.digest.at_tick, DIGEST_EVERY,
                "the digest was not refreshed on its cadence at {speed:?}"
            );
            assert_eq!(
                seen.digest.value,
                direct_digest(seen.digest.at_tick),
                "at {speed:?}, tick {} is not the island a plain run reaches",
                seen.digest.at_tick
            );
            assert!(
                !seen.digest.current,
                "a digest from tick {} was shown as current at tick {}",
                seen.digest.at_tick, seen.clock.tick
            );
            assert_eq!(
                seen.clock.sim_time_s,
                seen.clock.tick * 60,
                "the clock and the tick count disagree"
            );
        }
    }

    #[test]
    fn a_projection_is_published_before_the_first_step() {
        // A page loaded in the first moments should show the island at tick
        // 0, not an empty one that fills in later.
        let handle = spawn(island(), SimSpeed::RealTime);
        let at_once = handle.projection();
        handle.stop();

        assert!(at_once.running, "nothing was published at startup");
        assert_eq!(at_once.clock.tick, 0);
        assert_eq!(at_once.digest.at_tick, 0);
        assert!(
            at_once.digest.current,
            "the starting digest describes the island on show, so it is current"
        );
        assert_eq!(at_once.digest.value.len(), 64, "the digest is not a hash");
        assert_eq!(at_once.people.len(), 2, "the founders are missing");
        assert!(at_once.people.iter().all(|p| p.alive));
        assert!(at_once.land.trees > 0, "the estate has no trees");
        assert!(at_once.estate.battery_capacity_kwh > 0.0);
        assert_eq!(at_once.clock.requested_speed, "real time");
        assert!(
            (at_once.clock.day_length_hours - 36.0).abs() < 0.001,
            "the local day is {} hours, not 36",
            at_once.clock.day_length_hours
        );
    }

    #[test]
    fn speeds_read_and_write_the_way_a_person_would_type_them() {
        use std::str::FromStr;
        assert_eq!(SimSpeed::from_str("real").unwrap(), SimSpeed::RealTime);
        assert_eq!(SimSpeed::from_str("1x").unwrap(), SimSpeed::RealTime);
        assert_eq!(
            SimSpeed::from_str("max").unwrap(),
            SimSpeed::AsFastAsPossible
        );
        assert_eq!(SimSpeed::from_str("60").unwrap(), SimSpeed::Times(60));
        assert_eq!(SimSpeed::from_str("1440x").unwrap(), SimSpeed::Times(1_440));
        assert!(SimSpeed::from_str("quickly").is_err());
        assert_eq!(SimSpeed::Times(60).describe(), "60x real time");

        // A step of 60 simulated seconds: a real minute at real time, a
        // second at 60x, and no wait at all unpaced.
        assert_eq!(SimSpeed::RealTime.budget(60), Some(Duration::from_secs(60)));
        assert_eq!(SimSpeed::Times(60).budget(60), Some(Duration::from_secs(1)));
        assert_eq!(SimSpeed::AsFastAsPossible.budget(60), None);
        // A nonsense multiplier falls back to real time rather than
        // dividing by zero.
        assert_eq!(SimSpeed::Times(0).budget(60), Some(Duration::from_secs(60)));
    }
}

#[cfg(test)]
mod timing {
    use super::*;

    /// Not an assertion about speed — a measurement, printed, so the cost of
    /// a step and of a digest are visible rather than guessed at.
    #[test]
    #[ignore = "slow: prints timings rather than asserting"]
    fn slow_what_a_step_and_a_digest_cost() {
        let mut life = super::tests::island();
        let step = life.scenario.cadences.human_seconds;
        let t = Instant::now();
        life.advance(step).unwrap();
        println!("one step      : {:?}", t.elapsed());
        let t = Instant::now();
        let _ = IslandProjection::digest_now(&life);
        println!("one digest    : {:?}", t.elapsed());
        // Views built once, outside the timer: this measures publishing a
        // projection, not rebuilding every view, which the loop does on its
        // own cadence rather than per step.
        let views = Views::of(&life);
        let t = Instant::now();
        let _ = IslandProjection::of(&life, "x", None, Default::default(), views);
        println!("one projection: {:?}", t.elapsed());
        let t = Instant::now();
        let _ = IslandProjection::records_now(&life);
        println!("every record  : {:?}", t.elapsed());
        let t = Instant::now();
        for _ in 0..20 {
            life.advance(step).unwrap();
        }
        println!("twenty steps  : {:?}", t.elapsed());
    }
}
