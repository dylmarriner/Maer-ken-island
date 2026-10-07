//! Scale evidence for 8-16 GB machines: the integrated island with many
//! humans. Slow tier; prints time and peak memory (`ISLAND_SCALE_HUMANS`,
//! default 200; `ISLAND_SCALE_STEPS`, default 360 minute-steps).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::regional::commands::{IslandCommand, IslandCreateHumanRequest};
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("VmHWM:")?
                    .trim()
                    .strip_suffix("kB")?
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
        })
        .map_or(f64::NAN, |kb| kb / 1024.0)
}

fn env_n(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
#[ignore = "slow: scale measurement"]
fn a_populated_island_runs_within_memory_and_time_budgets() {
    let people = env_n("ISLAND_SCALE_HUMANS", 200);
    let steps = env_n("ISLAND_SCALE_STEPS", 360);
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let started = Instant::now();
    let mut w = IslandWorldState::new(canon, scenario).unwrap();
    let boot = started.elapsed().as_secs_f64();
    for i in 0..people {
        w.queue(IslandCommand::CreateHuman(IslandCreateHumanRequest {
            agent_id: format!("Settler-{i:05}"),
            sex: if i % 2 == 0 {
                BiologicalSex::Female
            } else {
                BiologicalSex::Male
            },
            age_years: 18.0 + (i % 40) as f64,
        }))
        .unwrap();
    }
    let t = Instant::now();
    for _ in 0..steps {
        w.step(60).unwrap();
    }
    let run = t.elapsed().as_secs_f64();
    let n = w.life.humans.registry.iter().count();
    println!(
        "SCALE humans={n} steps={steps} bootstrap={boot:.1}s run={run:.1}s ({:.1} ms/step) peak_rss={:.0} MB",
        1000.0 * run / steps as f64,
        peak_rss_mb()
    );
    assert_eq!(n, people + 2);
    assert!(
        peak_rss_mb() < 4096.0,
        "over the 4 GB budget for a 16 GB machine's headroom"
    );
}
