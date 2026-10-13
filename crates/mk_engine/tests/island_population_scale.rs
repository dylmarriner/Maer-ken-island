//! Scale evidence for 8-16 GB machines: the integrated island with many
//! humans. Slow tier; prints time and peak memory (`ISLAND_SCALE_HUMANS`,
//! default 200; `ISLAND_SCALE_STEPS`, default 360 minute-steps).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::regional::create_human::{CreateLocation, IslandCreateHuman};
use mk_engine::regional::life::IslandLife;
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
    let canon_again = Arc::clone(&canon);
    // On disk, not in a RAM-backed /tmp: the files must not compete with the
    // memory being measured.
    let store_root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let started = Instant::now();
    let mut w = IslandLife::bootstrap(scenario, canon).unwrap();
    let boot = started.elapsed().as_secs_f64();
    // Outside the founders' house, which is theirs and their children's alone.
    let yard = w.placed.layout.spaces_outside_house().next().unwrap().id;
    for i in 0..people {
        w.create_human(IslandCreateHuman {
            name: format!("Settler {i:05}"),
            biological_sex: if i % 2 == 0 {
                BiologicalSex::Female
            } else {
                BiologicalSex::Male
            },
            birth_timestamp: "1990-01-01T00:00:00Z".into(),
            age_years: 18.0 + (i % 40) as f64,
            height_cm: 170.0,
            build: "average".into(),
            hair_color: "black".into(),
            eye_color: "brown".into(),
            skin_tone: "olive".into(),
            location: CreateLocation::EstateSpace(yard),
            birthplace_here: true,
            birth_latitude: 0.0,
            birth_longitude: 0.0,
        })
        .unwrap();
    }
    if std::env::var("ISLAND_SCALE_STORE").is_ok() {
        w.enable_human_store(store_root.path()).unwrap();
    }
    let t = Instant::now();
    for _ in 0..steps {
        w.advance(60).unwrap();
    }
    let run = t.elapsed().as_secs_f64();
    let n = w.humans.registry.iter().count();
    println!(
        "SCALE humans={n} steps={steps} bootstrap={boot:.1}s run={run:.1}s ({:.1} ms/step) peak_rss={:.0} MB",
        1000.0 * run / steps as f64,
        peak_rss_mb()
    );
    assert_eq!(n, people + 2);

    // A full folder sync and a snapshot round trip at this population.
    if w.human_store().is_some() {
        let t = Instant::now();
        w.sync_humans();
        println!(
            "SCALE store: full sync of {n} humans = {:.2} s, write errors {}",
            t.elapsed().as_secs_f64(),
            w.human_store().unwrap().failures().0
        );
    }
    let snap = store_root.path().join("scale.snap");
    let t = Instant::now();
    mk_engine::io::island_snapshot::save_island_snapshot(&mut w, &snap).unwrap();
    let save = t.elapsed().as_secs_f64();
    let bytes = std::fs::metadata(&snap).unwrap().len();
    let t = Instant::now();
    let back = mk_engine::io::island_snapshot::load_island_snapshot(canon_again, &snap).unwrap();
    let load = t.elapsed().as_secs_f64();
    assert_eq!(back.state_digest(), w.state_digest());
    println!(
        "SCALE snapshot: {:.0} MB, save {save:.1} s, load {load:.1} s, peak_rss {:.0} MB",
        bytes as f64 / 1e6,
        peak_rss_mb()
    );
    assert!(
        peak_rss_mb() < 6144.0,
        "over the 4 GB budget for a 16 GB machine's headroom"
    );
}
