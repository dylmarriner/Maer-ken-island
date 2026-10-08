//! What Phase 4 costs the island.
//!
//! `benchmarks/island_week.md` measures the Phase 1-3 island and says in so
//! many words: "Not in this number: Phases 4, 4b and 5 (persistence,
//! replay, the app…)". Phase 4 is built now, so that sentence can stop
//! being a promise. This measures what each of its parts adds to a step
//! that is otherwise identical — a folder per human syncing, a replay log
//! being written, and a whole island going to disk and coming back.
//!
//! Slow tier, and release or the numbers mean nothing:
//!
//!     cargo test --release -p mk_engine --test island_phase4_cost \
//!         -- --ignored slow_ --nocapture
//!
//! Every pair is run on its own freshly bootstrapped island from the same
//! scenario and seed, so the only difference between them is the feature
//! being measured. The digests are compared as well as the times, because
//! a persistence feature that changed the world would be a far worse
//! finding than a slow one.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mk_core::canon::CanonLocked;
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::commands::IslandCommand;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

/// Long enough to average over the hourly and six-hourly cadences rather
/// than whichever one a short run happened to land on: 6 hours of island
/// time at the 60-second human step.
const STEPS: u64 = 360;
const STEP_SECONDS: u64 = 60;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn life() -> IslandLife {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).expect("the island bootstraps")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn run(life: &mut IslandLife, steps: u64) -> Duration {
    let started = Instant::now();
    for _ in 0..steps {
        life.advance(STEP_SECONDS).expect("the island steps on");
    }
    started.elapsed()
}

fn per_step_us(total: Duration, steps: u64) -> f64 {
    total.as_secs_f64() * 1e6 / steps as f64
}

#[test]
#[ignore = "slow: bootstraps the full island several times over"]
fn slow_what_phase_4_costs() {
    println!("\n== Phase 4's cost, {STEPS} steps of {STEP_SECONDS} s each ==\n");

    // 1. The island as Phases 1-3 left it.
    let mut plain = life();
    let bootstrap = Instant::now();
    let _ = life();
    println!("bootstrap                 : {:?}", bootstrap.elapsed());
    let bare = run(&mut plain, STEPS);
    let bare_digest = hex(plain.state_digest());
    println!(
        "steps, nothing attached   : {:?} total, {:.0} us/step",
        bare,
        per_step_us(bare, STEPS)
    );

    // 2. The same island with a folder per human syncing (Task 3b).
    let store_root = tempfile::tempdir().unwrap();
    let mut stored = life();
    let run_id = stored
        .enable_human_store(store_root.path())
        .expect("the run opens");
    let with_store = run(&mut stored, STEPS);
    let stored_digest = hex(stored.state_digest());
    println!(
        "steps, a folder per human : {:?} total, {:.0} us/step  (+{:.0} us, run {run_id:?})",
        with_store,
        per_step_us(with_store, STEPS),
        per_step_us(with_store, STEPS) - per_step_us(bare, STEPS)
    );

    // 3. The same island with every command recorded (Task 4). Commands are
    //    what a log costs; an island nobody touches writes nothing, so this
    //    applies one per hour, which is far more than a real session.
    let mut logged = life();
    let log_started = Instant::now();
    for step in 0..STEPS {
        if step.is_multiple_of(60) {
            logged
                .apply_command(IslandCommand::Control(
                    mk_engine::regional::commands::ControlCommand::Pause,
                ))
                .expect("a control command is honoured");
        }
        logged.advance(STEP_SECONDS).expect("the island steps on");
    }
    let with_log = log_started.elapsed();
    let logged_digest = hex(logged.state_digest());
    println!(
        "steps, a command an hour  : {:?} total, {:.0} us/step  ({} recorded)",
        with_log,
        per_step_us(with_log, STEPS),
        logged.replay_log().entries.len()
    );

    // 4. A whole island to disk and back (Task 3).
    let snapshot_dir = tempfile::tempdir().unwrap();
    let path = snapshot_dir.path().join("island.mks");
    let saving = Instant::now();
    save_island_snapshot(&mut plain, &path).expect("the island saves");
    let save = saving.elapsed();
    let bytes = std::fs::metadata(&path).unwrap().len();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let loading = Instant::now();
    let restored = load_island_snapshot(canon, &path).expect("the island loads");
    let load = loading.elapsed();
    println!(
        "snapshot save             : {save:?} for {:.1} MB on disk",
        bytes as f64 / 1_048_576.0
    );
    println!("snapshot load             : {load:?}");

    // The point of all of it: none of these changed the world.
    println!("\ndigest, nothing attached  : {bare_digest}");
    println!("digest, folders syncing   : {stored_digest}");
    println!("digest, commands recorded : {logged_digest}");
    println!(
        "digest, through a file    : {}",
        hex(restored.state_digest())
    );

    assert_eq!(
        bare_digest, stored_digest,
        "a folder per human must not change the island"
    );
    assert_eq!(
        bare_digest, logged_digest,
        "recording control commands must not change the island"
    );
    assert_eq!(
        bare_digest,
        hex(restored.state_digest()),
        "an island that went to disk and came back must be the same island"
    );
}
