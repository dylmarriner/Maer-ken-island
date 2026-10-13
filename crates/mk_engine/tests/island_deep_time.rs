//! Phase 4 Task 2b: the deep-time spin-up. The regional ecology is
//! bootstrapped at the plants' own fixed point (biomass = NPP x residence
//! time), which nothing moves on its own; what the spin-up settles is the
//! grazers' share of production and the soil they and the litter feed
//! (`regional/deep_time.rs`).

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

#[test]
fn island_biomass_starts_at_its_equilibrium_and_does_not_drift_over_centuries() {
    // The bootstrap's own guess, before deep time: without grazers it is
    // the plants' fixed point.
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.spin_up_years = 0;
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = IslandLife::bootstrap(scenario, canon).unwrap();
    let start = life.ecology.biomass_kgc_m2.data().to_vec();
    let total = |v: &[f64]| v.iter().sum::<f64>();
    assert!(total(&start) > 0.0, "the island has producers");

    // One century in one step, and in a hundred yearly steps: both stay put.
    let mut one = life.ecology.clone();
    one.step(100.0 * SECONDS_PER_YEAR, None, None);
    let mut many = life.ecology.clone();
    for _ in 0..100 {
        many.step(SECONDS_PER_YEAR, None, None);
    }
    for (name, state) in [("one 100-year step", &one), ("100 yearly steps", &many)] {
        let drift = (total(state.biomass_kgc_m2.data()) - total(&start)).abs() / total(&start);
        assert!(drift < 1e-9, "{name}: total biomass drifted by {drift:e}");
        let worst = start
            .iter()
            .zip(state.biomass_kgc_m2.data())
            .filter(|(a, _)| **a > 0.0)
            .map(|(a, b)| ((a - b) / a).abs())
            .fold(0.0, f64::max);
        assert!(worst < 1e-9, "{name}: a cell drifted by {worst:e}");
    }
}

// ---------------------------------------------------------------------------
// Phase 4 Task 2b: the spin-up itself, on the small test island (seed 16).
// ---------------------------------------------------------------------------

use mk_engine::regional::deep_time::{self, SpinUpSettings};
use mk_engine::regional::world::{IslandWorldError, IslandWorldState};

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}

/// The small island, founders and all, spinning up `years` of deep time.
fn small(years: u32) -> IslandScenario {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut seed = [0u8; 32];
    seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = seed;
    scenario.canon_path = repo("fixtures/island/canon.json");
    scenario.spin_up_years = years;
    scenario
}

fn soil_total(life: &IslandLife) -> f64 {
    let soil = life.ecology.soil.as_ref().unwrap();
    (0..soil.pools.len()).map(|i| soil.total(i)).sum()
}

/// Spinning up moves the island off the bootstrap's guess and onto its own
/// equilibrium, and by the end it has stopped moving: soil carbon, plants
/// and grazers each change by less than 1% over the last century.
#[test]
fn a_thousand_years_bring_the_living_world_to_equilibrium() {
    let life = IslandLife::bootstrap(small(1_000), canon()).unwrap();
    let report = life.spin_up.clone().expect("the scenario spins up");
    let e = report.equilibrium;
    assert_eq!(report.settings.years, 1_000);
    assert!(!report.from_cache);
    for (what, trend) in [
        ("soil carbon", e.soil_carbon_trend_per_century),
        ("plant biomass", e.biomass_trend_per_century),
        ("grazers", e.herbivore_trend_per_century),
    ] {
        assert!(
            trend.abs() < 0.01,
            "{what} still moving by {:.3}% per century",
            100.0 * trend
        );
    }
    // The spin-up did something: the grazers eat part of every year's
    // production, so the plants stand lower than NPP x residence.
    assert!(
        e.biomass_kgc < e.start_biomass_kgc,
        "biomass {} did not settle below the bootstrap's {}",
        e.biomass_kgc,
        e.start_biomass_kgc
    );
    let moved = (e.soil_carbon_kgc - e.start_soil_carbon_kgc).abs() / e.start_soil_carbon_kgc;
    assert!(moved > 1e-3, "soil carbon moved only {moved:e}");
    // And the report describes the island that came out.
    let area = life.domain.cell_area_m2(mk_island::DomainLevel::Medium);
    let soil = soil_total(&life) * area;
    assert!((soil - e.soil_carbon_kgc).abs() <= 1e-9 * soil);
}

/// The same inputs give the same island, at any thread count.
#[test]
fn the_spin_up_is_deterministic_at_any_thread_count() {
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                IslandLife::bootstrap(small(200), canon())
                    .unwrap()
                    .state_digest()
            })
    };
    let one = run(1);
    assert_eq!(one, run(4));
    assert_eq!(one, run(4));
}

/// Deep time changes the living world only: the physical state (terrain,
/// rivers, climatology) is exactly what the physical bootstrap made, so a
/// full-detail year afterwards has the climate the spin-up was driven by.
#[test]
fn the_spin_up_leaves_the_physical_world_and_its_climatology_alone() {
    let before = IslandLife::bootstrap(small(0), canon()).unwrap();
    let after = IslandLife::bootstrap(small(300), canon()).unwrap();
    assert!(before.spin_up.is_none());
    assert_eq!(
        serde_json::to_value(&before.physical).unwrap(),
        serde_json::to_value(&after.physical).unwrap()
    );
    assert_ne!(before.state_digest(), after.state_digest());
}

/// Any change to canon, scenario, seed, settings or the ecology the spin-up
/// starts from is a different key.
#[test]
fn the_cache_key_follows_every_input() {
    let canon = canon();
    let base = small(1_000);
    // The ecology a spin-up would start from: the island bootstrapped
    // without one.
    let start = IslandLife::bootstrap(small(0), canon.clone())
        .unwrap()
        .ecology;
    let key = |c: &CanonLocked, s: &IslandScenario, years: u32| {
        deep_time::cache_key(c, s, SpinUpSettings::new(c, years), &start)
    };
    let k = key(&canon, &base, 1_000);
    assert_eq!(k, key(&canon, &base, 1_000));
    assert_ne!(k, key(&canon, &base, 999));
    let mut seed = base.clone();
    seed.seed[0] ^= 1;
    assert_ne!(k, key(&canon, &seed, 1_000));
    let mut profile = base.clone();
    profile.profile.target_land_area_m2 *= 1.01;
    assert_ne!(k, key(&canon, &profile, 1_000));
    let mut other = (*canon).clone();
    other.orbital_period_s += 1.0;
    assert_ne!(k, key(&other, &base, 1_000));

    // A change anywhere upstream of the spin-up -- climate, hydrology, a
    // reference pack -- reaches it as a different starting ecology, and
    // must not load a world spun up from the old one.
    let mut moved = start.clone();
    moved.npp_kgc_m2_yr.data_mut()[0] += 1e-9;
    assert_ne!(
        k,
        deep_time::cache_key(&canon, &base, SpinUpSettings::new(&canon, 1_000), &moved)
    );
}

/// A run that loads the cached spin-up is the island that spun it up, and
/// stays the same island as both run on.
#[test]
fn loading_the_cache_then_running_equals_spinning_up_then_running() {
    let root = tempfile::tempdir().unwrap();
    let mut spun = IslandLife::bootstrap_cached(small(200), canon(), Some(root.path())).unwrap();
    assert!(!spun.spin_up.as_ref().unwrap().from_cache);
    let files: Vec<_> = std::fs::read_dir(root.path().join("spinup"))
        .unwrap()
        .collect();
    assert_eq!(files.len(), 1, "one cache file");

    let mut loaded = IslandLife::bootstrap_cached(small(200), canon(), Some(root.path())).unwrap();
    let report = loaded.spin_up.clone().unwrap();
    assert!(report.from_cache);
    assert_eq!(
        report.equilibrium,
        spun.spin_up.as_ref().unwrap().equilibrium
    );
    assert_eq!(spun.state_digest(), loaded.state_digest());
    for _ in 0..3 {
        spun.advance(3_600).unwrap();
        loaded.advance(3_600).unwrap();
    }
    assert_eq!(spun.state_digest(), loaded.state_digest());
}

/// A damaged cache is not trusted: the island spins up again, the same as
/// with no cache, and the file is replaced with a good one.
#[test]
fn a_damaged_cache_is_spun_up_again_and_replaced() {
    let root = tempfile::tempdir().unwrap();
    let clean = IslandLife::bootstrap(small(50), canon())
        .unwrap()
        .state_digest();
    let first = IslandLife::bootstrap_cached(small(50), canon(), Some(root.path())).unwrap();
    assert_eq!(first.state_digest(), clean);
    let path = std::fs::read_dir(root.path().join("spinup"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut bytes = std::fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&path, &bytes).unwrap();

    let again = IslandLife::bootstrap_cached(small(50), canon(), Some(root.path())).unwrap();
    assert!(!again.spin_up.as_ref().unwrap().from_cache);
    assert_eq!(again.state_digest(), clean);
    let third = IslandLife::bootstrap_cached(small(50), canon(), Some(root.path())).unwrap();
    assert!(
        third.spin_up.as_ref().unwrap().from_cache,
        "the file was replaced"
    );
}

/// Humans are never fast-forwarded: once anybody lives on the island, the
/// spin-up refuses and changes nothing.
#[test]
fn the_spin_up_refuses_once_humans_exist() {
    let mut world = IslandWorldState::new(canon(), small(0)).unwrap();
    let before = world.state_hash().unwrap();
    match world.spin_up(100) {
        Err(IslandWorldError::DeepTime(deep_time::DeepTimeError::HumansExist(n))) => {
            assert!(n >= 2, "the founders are there");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(world.state_hash().unwrap(), before);
}

/// The full default island's own spin-up, measured (Phase 4 Task 2b Step
/// 3). Slow tier: `cargo test --release -p mk_engine --test island_deep_time
/// -- --ignored slow_ --nocapture`.
#[test]
#[ignore = "slow: the full island's 1,000-year spin-up, about 11 s in release"]
fn slow_the_default_island_spins_up_to_equilibrium() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    assert!(
        scenario.spin_up_years >= 1_000,
        "the default scenario spins up"
    );
    let life = IslandLife::bootstrap(scenario, canon()).unwrap();
    let report = life.spin_up.clone().unwrap();
    let e = report.equilibrium;
    println!(
        "spin-up: {} years in {:.1} s on {} threads",
        report.settings.years,
        report.wall_seconds,
        rayon::current_num_threads()
    );
    println!(
        "soil {:.4e} -> {:.4e} kgC ({:+.4}%/century); biomass {:.4e} -> {:.4e} kgC ({:+.4}%/century); grazers {:.4e} -> {:.4e} kgC ({:+.4}%/century)",
        e.start_soil_carbon_kgc,
        e.soil_carbon_kgc,
        100.0 * e.soil_carbon_trend_per_century,
        e.start_biomass_kgc,
        e.biomass_kgc,
        100.0 * e.biomass_trend_per_century,
        e.start_herbivore_kgc,
        e.herbivore_kgc,
        100.0 * e.herbivore_trend_per_century,
    );
    for trend in [
        e.soil_carbon_trend_per_century,
        e.biomass_trend_per_century,
        e.herbivore_trend_per_century,
    ] {
        assert!(trend.abs() < 0.01);
    }
}
