//! Phase 4 Task 5: running the island without a browser.
//!
//! `island run` advances the island a fixed number of steps and prints the
//! canonical state digest. Two runs of the same scenario and seed print the
//! same digest, which is what Gate 4 asks for, and the digest is the handle
//! on everything else: a run that stops and resumes through `--save` reaches
//! the same digest as one that never stopped.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::life::IslandLife;
use mk_engine::regional::replay::{replay_island, IslandReplayLog};
use mk_island::IslandScenario;

/// Hex for a digest, the form every other tool in the project prints.
fn hex(digest: [u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Find the canon a scenario names.
///
/// The path in the scenario is usually repo-relative (`fixtures/island/…`),
/// which works when the runner is started from the repository root and not
/// otherwise. Rather than demand a working directory, this also looks beside
/// the scenario file, so a scenario and its canon can be kept together
/// anywhere.
fn canon_for(scenario_path: &Path, scenario: &IslandScenario) -> Result<CanonLocked, String> {
    let named = &scenario.canon_path;
    let beside = scenario_path
        .parent()
        .map(|dir| dir.join(named))
        .unwrap_or_else(|| named.clone());
    for candidate in [named.clone(), beside] {
        if candidate.is_file() {
            return CanonLocked::load(&candidate)
                .map_err(|e| format!("cannot read the canon at {}: {e}", candidate.display()));
        }
    }
    Err(format!(
        "the scenario names a canon at {}, which is not there — looked from the working \
         directory and beside {}",
        named.display(),
        scenario_path.display()
    ))
}

/// Everything `island run` was asked for.
pub struct RunArgs {
    pub scenario: Option<PathBuf>,
    pub snapshot: Option<PathBuf>,
    pub steps: u64,
    pub dt_seconds: u64,
    pub save: Option<PathBuf>,
    pub save_root: Option<PathBuf>,
    /// Where to write the commands this run applied, so it can be run
    /// again.
    pub log: Option<PathBuf>,
}

/// Advance an island and print what it came to.
///
/// Nothing here is timed against the real clock: `--steps` and `--dt` decide
/// how much simulated time passes, and the run goes as fast as the machine
/// allows. Pacing against wall-clock time belongs to the server, where
/// somebody is watching.
pub fn run(args: RunArgs) -> Result<(), String> {
    let mut life = open_world(args.scenario.as_deref(), args.snapshot.as_deref())?;

    if let Some(root) = &args.save_root {
        let id = life
            .enable_human_store(root)
            .map_err(|e| format!("cannot keep human folders under {}: {e}", root.display()))?;
        println!("run {id}");
        println!("  folders   {}", root.join(id.as_str()).display());
    }

    println!(
        "  start     {} ({})",
        hex(life.state_digest()),
        elapsed(&life)
    );
    let seconds = args
        .steps
        .checked_mul(args.dt_seconds)
        .ok_or("--steps times --dt is more time than the island can count")?;
    life.advance(seconds)
        .map_err(|e| format!("the island stopped: {e}"))?;

    println!("  digest    {}", hex(life.state_digest()));
    println!("  elapsed   {}", elapsed(&life));
    println!(
        "  audits    {} closed, shortfalls: food {}, water {}",
        life.audits_closed, life.shortfalls.food, life.shortfalls.water
    );
    println!(
        "  alive     {}",
        life.humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .count()
    );
    if let Some(store) = life.human_store() {
        match store.failures() {
            (0, _) => println!("  folders   written, no failures"),
            (n, why) => println!(
                "  folders   {n} write(s) failed, most recently: {}",
                why.unwrap_or("unknown")
            ),
        }
    }

    if let Some(path) = &args.save {
        save_island_snapshot(&mut life, path)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        println!("  saved     {} ({} MB)", path.display(), size / 1_048_576);
    }
    if let Some(path) = &args.log {
        life.replay_log()
            .save(path)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        println!(
            "  log       {} ({} command(s))",
            path.display(),
            life.replay_log().entries.len()
        );
    }
    Ok(())
}

/// Run a scenario again from its log, and print where it got to.
///
/// The point of the digest it prints is that it should equal the one the
/// original run printed. A replay that reached somewhere else would mean
/// the island depends on something the log does not record, and the digest
/// would stop being a statement about the world.
pub fn replay(scenario_path: &Path, log_path: &Path, until: Option<u64>) -> Result<(), String> {
    let scenario = IslandScenario::load(scenario_path).map_err(|e| e.to_string())?;
    let canon = canon_for(scenario_path, &scenario)?;
    let log = IslandReplayLog::load(log_path).map_err(|e| e.to_string())?;

    let until = until.unwrap_or_else(|| log.entries.iter().map(|e| e.tick).max().unwrap_or(0));
    println!("{}", log_path.display());
    println!("  commands  {}", log.entries.len());
    println!("  to tick   {until}");

    let life = replay_island(Arc::new(canon), scenario, &log, until)
        .map_err(|e| format!("the replay stopped: {e}"))?;

    println!("  digest    {}", hex(life.state_digest()));
    println!("  elapsed   {}", elapsed(&life));
    println!("  people    {}", life.humans.registry.iter().count());
    Ok(())
}

/// Describe a saved island without running it.
pub fn inspect(path: &Path, scenario_path: Option<&Path>) -> Result<(), String> {
    // A snapshot is only readable against the canon it was written under, so
    // one has to be found before anything can be said about the file.
    let canon = match scenario_path {
        Some(path) => {
            let scenario = IslandScenario::load(path).map_err(|e| e.to_string())?;
            canon_for(path, &scenario)?
        }
        None => CanonLocked::load(Path::new("fixtures/island/canon.json")).map_err(|e| {
            format!("cannot read fixtures/island/canon.json: {e} — pass --scenario to say which canon this snapshot was written under")
        })?,
    };
    let life = load_island_snapshot(Arc::new(canon), path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    println!("{}", path.display());
    println!("  digest    {}", hex(life.state_digest()));
    println!("  elapsed   {}", elapsed(&life));
    println!("  tick      {}", life.tick);
    println!(
        "  audits    {} closed, shortfalls: food {}, water {}",
        life.audits_closed, life.shortfalls.food, life.shortfalls.water
    );
    println!("  people    {}", life.humans.registry.iter().count());
    println!("  trees     {}", life.vegetation.trees.len());
    Ok(())
}

/// Simulated time, in the units a person reading a run report thinks in.
fn elapsed(life: &IslandLife) -> String {
    let days = life.sim_time_s as f64 / 86_400.0;
    if days < 1.0 {
        format!("{:.1} h", life.sim_time_s as f64 / 3_600.0)
    } else {
        format!("{days:.2} days")
    }
}

/// Start from a scenario or carry on from a snapshot, but not both.
///
/// Shared with `island serve --scenario`, so the dashboard's island and the
/// headless one are opened by exactly the same rules.
pub fn open_world(scenario: Option<&Path>, snapshot: Option<&Path>) -> Result<IslandLife, String> {
    match (scenario, snapshot) {
        (Some(_), Some(_)) => {
            Err("--scenario starts a new island and --snapshot carries one on; pick one".into())
        }
        (None, None) => Err("--scenario or --snapshot is needed".into()),
        (Some(path), None) => {
            let scenario = IslandScenario::load(path).map_err(|e| e.to_string())?;
            let canon = canon_for(path, &scenario)?;
            IslandLife::bootstrap(scenario, Arc::new(canon))
                .map_err(|e| format!("the island will not start: {e}"))
        }
        (None, Some(path)) => {
            // The canon is not in the file, so it comes from where a fresh
            // run would have found it; the load refuses if it disagrees.
            let canon = CanonLocked::load(Path::new("fixtures/island/canon.json"))
                .map_err(|e| format!("cannot read fixtures/island/canon.json: {e}"))?;
            load_island_snapshot(Arc::new(canon), path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))
        }
    }
}
