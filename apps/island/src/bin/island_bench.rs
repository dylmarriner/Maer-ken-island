//! `island_bench`: repeatable performance baseline of the real regional paths.
//!
//! ```text
//! island_bench [--scenario <path>] [--canon <path>] [--ticks 1000] [--dt 60]
//!              [--people <extra settlers>] [--tmp <dir>] [--compare-threads] [--json]
//! ```
//!
//! Everything is measured, nothing estimated: bootstrap, stepping, the
//! per-subsystem split, a snapshot save and load (verified to give the same
//! state hash), a full human-folder sync, and peak resident memory
//! (`VmHWM`, Linux only). `--tmp` must be on disk, not a RAM-backed `/tmp`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::commands::{IslandCommand, IslandCreateHumanRequest};
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;
use serde_json::{json, Value};

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn number(args: &[String], name: &str, default: u64) -> Result<u64, String> {
    flag(args, name).map_or(Ok(default), |v| {
        v.parse()
            .map_err(|_| format!("{name} must be a whole number, got {v:?}"))
    })
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn peak_rss_bytes() -> Value {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("VmHWM:")?
                    .trim()
                    .strip_suffix("kB")?
                    .trim()
                    .parse::<u64>()
                    .ok()
            })
        })
        .map_or(Value::Null, |kb| json!(kb * 1024))
}

fn open(args: &[String]) -> Result<(Arc<CanonLocked>, IslandScenario), String> {
    let scenario =
        flag(args, "--scenario").unwrap_or_else(|| "fixtures/island/default_scenario.json".into());
    let canon = flag(args, "--canon").unwrap_or_else(|| "fixtures/island/canon.json".into());
    Ok((
        Arc::new(
            CanonLocked::load(Path::new(&canon)).map_err(|e| format!("canon {canon}: {e:?}"))?,
        ),
        IslandScenario::load(Path::new(&scenario))
            .map_err(|e| format!("scenario {scenario}: {e}"))?,
    ))
}

fn add_settlers(world: &mut IslandWorldState, people: u64) -> Result<(), String> {
    for i in 0..people {
        world
            .queue(IslandCommand::CreateHuman(IslandCreateHumanRequest {
                agent_id: format!("Settler-{i:05}"),
                sex: if i % 2 == 0 {
                    BiologicalSex::Female
                } else {
                    BiologicalSex::Male
                },
                age_years: 18.0 + (i % 40) as f64,
                appearance: None,
                location: None,
            }))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn run_steps(world: &mut IslandWorldState, ticks: u64, dt: u64) -> Result<Duration, String> {
    let started = Instant::now();
    for _ in 0..ticks {
        world.step(dt).map_err(|e| e.to_string())?;
    }
    Ok(started.elapsed())
}

/// Steps per second for `ticks` on a fresh world, inside a pool of `threads`.
fn rate_in_pool(
    args: &[String],
    threads: usize,
    ticks: u64,
    dt: u64,
    people: u64,
) -> Result<f64, String> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| e.to_string())?;
    pool.install(|| {
        let (canon, scenario) = open(args)?;
        let mut world = IslandWorldState::new(canon, scenario).map_err(|e| e.to_string())?;
        add_settlers(&mut world, people)?;
        let elapsed = run_steps(&mut world, ticks, dt)?;
        Ok(ticks as f64 / elapsed.as_secs_f64())
    })
}

fn bench(args: &[String]) -> Result<Value, String> {
    let ticks = number(args, "--ticks", 1000)?;
    let dt = number(args, "--dt", 60)?;
    let people = number(args, "--people", 0)?;
    let tmp = PathBuf::from(flag(args, "--tmp").unwrap_or_else(|| "target/bench".into()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;

    let (canon, scenario) = open(args)?;
    let started = Instant::now();
    let mut world = IslandWorldState::new(canon, scenario).map_err(|e| e.to_string())?;
    let bootstrap = started.elapsed();
    add_settlers(&mut world, people)?;
    world
        .enable_human_store(&tmp.join("store"))
        .map_err(|e| e.to_string())?;

    let stepping = run_steps(&mut world, ticks, dt)?;
    let timings = world.life.timings;
    let simulated_days = ticks as f64 * dt as f64 / 86_400.0;
    let population = world.life.humans.registry.iter().count();

    let t = Instant::now();
    world.sync_human_store();
    let store_sync = t.elapsed();

    let snapshot = tmp.join("bench.snap");
    let t = Instant::now();
    save_island_snapshot(&world, &snapshot).map_err(|e| e.to_string())?;
    let save = t.elapsed();
    let snapshot_bytes = std::fs::metadata(&snapshot)
        .map_err(|e| e.to_string())?
        .len();
    let t = Instant::now();
    let restored = load_island_snapshot(&snapshot).map_err(|e| e.to_string())?;
    let load = t.elapsed();
    let same = restored.state_hash().map_err(|e| e.to_string())?
        == world.state_hash().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&snapshot);
    drop(restored);

    let mut out = json!({
        "scenario": flag(args, "--scenario").unwrap_or_else(|| "fixtures/island/default_scenario.json".into()),
        "ticks": ticks, "dt_seconds": dt, "population": population,
        "bootstrap_ms": ms(bootstrap),
        "ticks_per_second": ticks as f64 / stepping.as_secs_f64(),
        "simulated_days_per_real_minute": simulated_days / (stepping.as_secs_f64() / 60.0),
        "step_ms_per_tick": ms(stepping) / ticks as f64,
        "subsystem_ms": {
            "humans": ms(timings.humans), "households": ms(timings.households),
            "physics_hourly": ms(timings.physics_hourly),
            "ecology_six_hourly": ms(timings.ecology_six_hourly),
            "human_store_cadence": ms(timings.human_store),
        },
        "serialized_state_bytes": snapshot_bytes, "snapshot_bytes": snapshot_bytes,
        "save_ms": ms(save), "load_ms": ms(load), "snapshot_round_trip_hash_equal": same,
        "human_store_sync_ms": ms(store_sync),
        "peak_rss_bytes": peak_rss_bytes(),
        "threads": rayon::current_num_threads(),
        "spin_up_1000y_seconds": Value::Null,
        "notes": ["spin_up_1000y_seconds is null: no deep-time spin-up exists (plan Task 2b is blocked on missing soil, erosion and tectonics models)."],
    });
    if args.iter().any(|a| a == "--compare-threads") {
        let cmp_ticks = ticks.min(200);
        let one = rate_in_pool(args, 1, cmp_ticks, dt, people)?;
        let all = rate_in_pool(args, rayon::current_num_threads(), cmp_ticks, dt, people)?;
        out["thread_comparison"] = json!({
            "ticks": cmp_ticks, "one_thread_ticks_per_second": one,
            "all_threads_ticks_per_second": all, "speedup": all / one,
        });
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match bench(&args) {
        Ok(result) => {
            let text = serde_json::to_string_pretty(&result).expect("JSON");
            println!("{text}");
        }
        Err(message) => {
            eprintln!("island_bench: {message}");
            std::process::exit(1);
        }
    }
}
