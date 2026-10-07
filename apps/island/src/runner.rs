//! The headless island runner: `island run`, `island replay`, `island inspect`.
//!
//! No UI dependency. Every command prints a final `state hash`, so two runs
//! of the same scenario can be compared byte for byte.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::commands::IslandReplayLog;
use mk_engine::regional::replay::replay_island;
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;
use serde::{Deserialize, Serialize};

pub const RUNNER_USAGE: &str = "usage:
  island run --scenario <path> [--canon <path>] [--steps <n>] [--dt <seconds>] [--load <snapshot>] [--save <snapshot>] [--log <path>] [--save-root <dir>]
  island replay --scenario <path> [--canon <path>] --log <path> [--save <snapshot>]
  island inspect --snapshot <path>
  island serve --scenario <path> | --snapshot <path> [--canon <path>] [--bind 127.0.0.1:8080] [--save-root <dir>] [--snapshot-out <path>]
  island serve [--data-dir ./island-data] [--seed <64 hex chars>] [--bind 127.0.0.1:8080]";

const DEFAULT_CANON: &str = "fixtures/island/canon.json";

/// A run's replay log with the tick the run reached.
#[derive(Serialize, Deserialize)]
pub struct RunLogFile {
    pub until_tick: u64,
    pub log: IslandReplayLog,
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn number(args: &[String], name: &str, default: u64) -> Result<u64, String> {
    match flag(args, name) {
        None => Ok(default),
        Some(v) => v
            .parse::<u64>()
            .map_err(|_| format!("{name} must be a whole number, got {v:?}")),
    }
}

fn canon(args: &[String]) -> Result<Arc<CanonLocked>, String> {
    let path = PathBuf::from(flag(args, "--canon").unwrap_or_else(|| DEFAULT_CANON.into()));
    CanonLocked::load(&path)
        .map(Arc::new)
        .map_err(|e| format!("cannot load canon {}: {e:?}", path.display()))
}

fn scenario(args: &[String]) -> Result<IslandScenario, String> {
    let path = flag(args, "--scenario").ok_or("--scenario <path> is required")?;
    IslandScenario::load(Path::new(&path)).map_err(|e| format!("cannot load scenario {path}: {e}"))
}

fn hex(hash: [u8; 32]) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

/// A concise, deterministic summary of the world.
pub fn summary(world: &IslandWorldState) -> Result<String, String> {
    let hash = world.state_hash().map_err(|e| e.to_string())?;
    let life = &world.life;
    let days = world.sim_time_seconds() / 86_400.0;
    let store = world.human_store_status().map_or(String::new(), |s| {
        format!(
            "\nhuman store run {}: {} syncs, {} write errors{}",
            s.run_id,
            s.syncs,
            s.errors,
            s.last_error
                .as_ref()
                .map_or(String::new(), |e| format!(" (last: {e})"))
        )
    });
    Ok(format!(
        "tick {} ({days:.3} simulated days)\nhumans {}{store}\nstate hash {}",
        world.tick(),
        life.humans.registry.iter().count(),
        hex(hash),
    ))
}

fn save(world: &mut IslandWorldState, args: &[String]) -> Result<(), String> {
    if let Some(path) = flag(args, "--save") {
        // The folders must match the snapshot being written.
        world.sync_human_store();
        save_island_snapshot(world, Path::new(&path)).map_err(|e| e.to_string())?;
        println!("saved {path}");
    }
    Ok(())
}

/// Build the world a command works on: from a snapshot if `--load` is given,
/// otherwise from `--scenario` (and `--canon`).
pub fn open_world(args: &[String], load_flag: &str) -> Result<IslandWorldState, String> {
    match flag(args, load_flag) {
        Some(path) => load_island_snapshot(Path::new(&path)).map_err(|e| e.to_string()),
        None => IslandWorldState::new(canon(args)?, scenario(args)?).map_err(|e| e.to_string()),
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let steps = number(args, "--steps", 0)?;
    let dt = number(args, "--dt", 60)?;
    if dt == 0 {
        return Err("--dt must be at least 1".into());
    }
    let mut world = open_world(args, "--load")?;
    if let Some(root) = flag(args, "--save-root") {
        let run_id = world
            .enable_human_store(Path::new(&root))
            .map_err(|e| e.to_string())?;
        println!("run {run_id}: human folders in {root}/{run_id}/humans");
    }
    for _ in 0..steps {
        world.step(dt).map_err(|e| e.to_string())?;
    }
    save(&mut world, args)?;
    if let Some(path) = flag(args, "--log") {
        let file = RunLogFile {
            until_tick: world.tick(),
            log: world.replay_log(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("cannot write {path}: {e}"))?;
        println!("wrote {path}");
    }
    summary(&world)
}

fn replay(args: &[String]) -> Result<String, String> {
    let path = flag(args, "--log").ok_or("--log <path> is required")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let file: RunLogFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("{path} is not a run log: {e}"))?;
    let mut world = replay_island(canon(args)?, scenario(args)?, &file.log, file.until_tick)
        .map_err(|e| e.to_string())?;
    save(&mut world, args)?;
    summary(&world)
}

fn inspect(args: &[String]) -> Result<String, String> {
    let path = flag(args, "--snapshot").ok_or("--snapshot <path> is required")?;
    let world = load_island_snapshot(Path::new(&path)).map_err(|e| e.to_string())?;
    summary(&world)
}

/// Run `run`, `replay` or `inspect`; `None` if `args[0]` is none of them.
/// With no command at all, the default scenario is bootstrapped and summarised.
pub fn run_cli(args: &[String]) -> Option<Result<String, String>> {
    if args.is_empty() {
        let default = ["--scenario", "fixtures/island/default_scenario.json"];
        return Some(run(&default.map(String::from)));
    }
    let rest = args.get(1..)?;
    match args.first()?.as_str() {
        "run" => Some(run(rest)),
        "replay" => Some(replay(rest)),
        "inspect" => Some(inspect(rest)),
        _ => None,
    }
}
