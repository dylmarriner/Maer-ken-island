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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use mk_engine::regional::life::IslandLife;

use super::projection::{Digest, IslandProjection};

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

/// The dashboard's end of the simulation.
#[derive(Clone)]
pub struct SimHandle {
    projection: Arc<RwLock<IslandProjection>>,
    stop: Arc<AtomicBool>,
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
/// Measured by `slow_what_a_step_and_a_digest_cost`: a step is 1.4 ms and a
/// digest is **932 ms** — 650 times more — because hashing walks both
/// 1,152,000-cell grids and every human's canonical JSON. (It is not the
/// stems: that measurement is on a 50-tree patch.) Hashing every step made
/// the island run hundreds of times slower than it needed to, for a number
/// nobody reads that often.
///
/// Sixty steps is one simulated hour at the default cadence. It still
/// dominates the loop — about 15 ms amortised per step against 1.4 ms of
/// actual simulation — which caps the island near 3,500x real time. That is
/// far above the `RealTime` default and enough for a dashboard; if a faster
/// headless-style speed is ever wanted here, this is the knob.
const DIGEST_EVERY: u64 = 60;

/// Start an island running on its own thread.
///
/// Returns as soon as the thread is spawned, with a projection already
/// published for the island's starting state, so a page loaded immediately
/// shows the world at tick 0 rather than an empty one.
pub fn spawn(life: IslandLife, speed: SimSpeed) -> SimHandle {
    let step_seconds = life.scenario.cadences.human_seconds;
    // The starting island is hashed once — and only once: hashing costs
    // about 930 ms against a 1.4 ms step, so the thread is handed this one
    // rather than computing its own.
    let first = IslandProjection::digest_now(&life);
    let projection = Arc::new(RwLock::new(IslandProjection::of(
        &life,
        &speed.describe(),
        None,
        first.clone(),
    )));
    let handle = SimHandle {
        projection: Arc::clone(&projection),
        stop: Arc::new(AtomicBool::new(false)),
    };
    let stop = Arc::clone(&handle.stop);

    std::thread::Builder::new()
        .name("island-sim".into())
        .spawn(move || run(life, speed, step_seconds, projection, stop, first))
        .expect("the simulation thread starts");

    handle
}

/// The loop itself: step, publish, wait if there is time to spare.
fn run(
    mut life: IslandLife,
    speed: SimSpeed,
    step_seconds: u64,
    projection: Arc<RwLock<IslandProjection>>,
    stop: Arc<AtomicBool>,
    mut digest: Digest,
) {
    // (when, simulated time then), oldest first. Speed is the simulated
    // time between the ends of the window over the real time between them.
    let mut recent: std::collections::VecDeque<(Instant, u64)> = std::collections::VecDeque::new();

    while !stop.load(Ordering::Relaxed) {
        // Timed around the whole iteration, not just the step. Publishing
        // is real work the island has to do before it can step again, and
        // a readout that left it out would have claimed the island was
        // keeping up while it ran two hundred times slower.
        let started = Instant::now();
        if let Err(e) = life.advance(step_seconds) {
            // The island cannot continue — an audit that did not close, a
            // step that failed. Publishing a stale projection forever would
            // be a lie, so the thread stops, says why on the console the
            // operator is already watching, and marks the projection as no
            // longer running so the page says so too.
            eprintln!("the island stopped at t={} s: {e}", life.sim_time_s);
            if let Ok(mut slot) = projection.write() {
                slot.running = false;
            }
            break;
        }
        if life.tick.is_multiple_of(DIGEST_EVERY) {
            digest = IslandProjection::digest_now(&life);
        }

        let now = Instant::now();
        recent.push_back((now, life.sim_time_s));
        while recent
            .front()
            .is_some_and(|(at, _)| now.duration_since(*at) > SPEED_WINDOW)
            && recent.len() > 2
        {
            recent.pop_front();
        }
        publish(
            &projection,
            &life,
            &speed,
            achieved_speed(&recent),
            digest.clone(),
        );

        let took = started.elapsed();

        // Only ever sleep off time that is left over. A step that overran
        // its budget is not slept on at all, so the island catches up where
        // it can rather than falling further behind.
        if let Some(budget) = speed.budget(step_seconds) {
            if let Some(spare) = budget.checked_sub(took) {
                // Woken often enough that a stop is acted on promptly even
                // at real time, where a budget is a whole minute.
                let deadline = Instant::now() + spare;
                while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50).min(deadline - Instant::now()));
                }
            }
        }
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

fn publish(
    projection: &RwLock<IslandProjection>,
    life: &IslandLife,
    speed: &SimSpeed,
    achieved: Option<f64>,
    digest: Digest,
) {
    let next = IslandProjection::of(life, &speed.describe(), achieved, digest);
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
        let t = Instant::now();
        let _ = IslandProjection::of(&life, "x", None, Default::default());
        println!("one projection: {:?}", t.elapsed());
        let t = Instant::now();
        for _ in 0..20 {
            life.advance(step).unwrap();
        }
        println!("twenty steps  : {:?}", t.elapsed());
    }
}
