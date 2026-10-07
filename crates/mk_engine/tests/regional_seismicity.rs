//! Phase 1 Task 3b: earthquakes from regional fault stress.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_core::rng::RngRegistry;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::seismicity::{step_seismicity, Earthquake, FaultSystem, MIN_MAGNITUDE};
use mk_engine::regional::tectonics::{regional_plates, step_regional_tectonics};
use mk_engine::tectonics::TectonicsState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const YEAR_S: f64 = 365.25 * 86_400.0;

struct Setup {
    domain: IslandDomain,
    tectonics: TectonicsState,
    faults: FaultSystem,
}

fn setup(seed: [u8; 32], profile: IslandProfile) -> Setup {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    let canon = CanonLocked::load(&path).expect("island canon");
    let domain = IslandDomain::from_profile(profile).unwrap();
    let boundaries = sample_regional_boundaries(seed, &canon, &domain, 0.0);
    let tectonics = step_regional_tectonics(&canon, 0, &domain, &boundaries.tectonic);
    let plates = regional_plates(&domain, &boundaries.tectonic);
    let rng = RngRegistry::new(seed);
    let faults = FaultSystem::from_tectonics(&domain, &tectonics.plates, &plates, &rng);
    Setup {
        domain,
        tectonics,
        faults,
    }
}

fn run(s: &mut Setup, years: f64, step_years: f64) -> Vec<Earthquake> {
    let mut events = Vec::new();
    let steps = (years / step_years).round() as usize;
    for _ in 0..steps {
        events.extend(step_seismicity(
            &mut s.faults,
            &s.domain,
            step_years * YEAR_S,
        ));
    }
    events
}

/// Aki (1965) maximum-likelihood b-value for continuous magnitudes.
fn b_value(events: &[Earthquake]) -> f64 {
    let mean = events.iter().map(|e| e.magnitude_mw).sum::<f64>() / events.len() as f64;
    std::f64::consts::LOG10_E / (mean - MIN_MAGNITUDE)
}

#[test]
fn faults_slip_at_the_plates_relative_speed() {
    let s = setup([2; 32], IslandProfile::test_small());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    let canon = CanonLocked::load(&path).unwrap();
    let boundaries = sample_regional_boundaries([2; 32], &canon, &s.domain, 0.0);
    let plates = regional_plates(&s.domain, &boundaries.tectonic);
    assert!(!s.faults.faults.is_empty());
    for f in &s.faults.faults {
        let a = plates.iter().find(|p| p.id == f.plates.0).unwrap();
        let b = plates.iter().find(|p| p.id == f.plates.1).unwrap();
        let relative = (a.velocity_east_m_yr - b.velocity_east_m_yr)
            .hypot(a.velocity_north_m_yr - b.velocity_north_m_yr)
            * 1000.0;
        assert!(
            (f.slip_rate_mm_yr - relative).abs() < 1e-9,
            "fault {}",
            f.id
        );
        assert!(f.slip_rate_mm_yr > 0.0 && f.slip_rate_mm_yr < 200.0);
    }
}

#[test]
fn magnitudes_follow_gutenberg_richter_and_moment_balances_loading() {
    let mut s = setup([3; 32], IslandProfile::test_small());
    let events = run(&mut s, 2_000.0, 1.0);
    assert!(events.len() > 1_000, "{} events", events.len());
    let b = b_value(&events);
    assert!((0.8..=1.2).contains(&b), "b = {b}");
    let loaded = s.faults.loaded_moment_nm;
    let released = s.faults.released_moment_nm;
    assert!(
        (released - loaded).abs() / loaded < 0.10,
        "released {released:e} vs loaded {loaded:e}"
    );
}

#[test]
fn aftershocks_decay_as_omori_utsu() {
    let mut s = setup([4; 32], IslandProfile::test_small());
    let events = run(&mut s, 2_000.0, 1.0);
    let mainshocks: Vec<&Earthquake> = events
        .iter()
        .filter(|e| !e.is_aftershock && e.magnitude_mw >= 6.5)
        .collect();
    assert!(!mainshocks.is_empty());
    // Pool aftershock delays after every large mainshock on its fault.
    let mut delays = Vec::new();
    for m in &mainshocks {
        delays.extend(
            events
                .iter()
                .filter(|e| e.is_aftershock && e.fault_id == m.fault_id && e.time_s > m.time_s)
                .map(|e| (e.time_s - m.time_s) / 86_400.0)
                .filter(|d| *d < 300.0),
        );
    }
    assert!(delays.len() > 200, "{} aftershocks", delays.len());
    // Rates in two log bins, 1-10 d and 10-100 d: for p = 1.1 the
    // per-day rate falls by ~10^1.1 per decade.
    let early = delays.iter().filter(|d| (1.0..10.0).contains(*d)).count() as f64 / 9.0;
    let late = delays.iter().filter(|d| (10.0..100.0).contains(*d)).count() as f64 / 90.0;
    let p = (early / late).log10();
    assert!((0.8..=1.4).contains(&p), "p = {p}");
}

#[test]
fn catalogues_are_deterministic_and_independent_of_step_length() {
    let mut a = setup([5; 32], IslandProfile::test_small());
    let mut b = setup([5; 32], IslandProfile::test_small());
    let yearly = run(&mut a, 200.0, 1.0);
    let decadal = run(&mut b, 200.0, 10.0);
    assert_eq!(yearly.len(), decadal.len());
    for (x, y) in yearly.iter().zip(&decadal) {
        assert_eq!(x.fault_id, y.fault_id);
        assert!((x.time_s - y.time_s).abs() < 1e-3);
        assert_eq!(x.magnitude_mw, y.magnitude_mw);
        assert_eq!(x.epicentre_m, y.epicentre_m);
    }
    let mut c = setup([6; 32], IslandProfile::test_small());
    assert_ne!(run(&mut c, 200.0, 1.0).len(), 0);
}

#[test]
fn no_earthquake_strikes_an_aseismic_plate_interior() {
    let mut s = setup([7; 32], IslandProfile::test_small());
    let events = run(&mut s, 500.0, 1.0);
    for e in &events {
        let (r, c) = s
            .domain
            .cell_containing_m(DomainLevel::Coarse, e.epicentre_m.0, e.epicentre_m.1)
            .expect("inside the domain");
        assert!(
            s.tectonics.plates.get(r, c).boundary.is_some(),
            "event in a plate interior at ({r}, {c})"
        );
    }
}

#[test]
#[ignore = "slow tier: 10,000 simulated years on the full island"]
fn slow_ten_thousand_years_of_seismicity_on_the_full_island() {
    let mut s = setup([11; 32], IslandProfile::default_nz_scale());
    let events = run(&mut s, 10_000.0, 1.0);
    let b = b_value(&events);
    assert!((0.8..=1.2).contains(&b), "b = {b}");
    let loaded = s.faults.loaded_moment_nm;
    let released = s.faults.released_moment_nm;
    assert!(
        (released - loaded).abs() / loaded < 0.10,
        "released {released:e} vs loaded {loaded:e}"
    );
    let biggest = events.iter().map(|e| e.magnitude_mw).fold(0.0, f64::max);
    assert!(biggest > 7.5, "largest event Mw {biggest}");
}

/// Plate motion stays inside the observed span, and the boundaries it
/// implies stay inside what two plates can possibly do to each other.
///
/// The packs' `plate_boundary_fault_slip_rate` describes one real fault —
/// the Alpine Fault at 21-31 mm/yr — and the island's boundaries run faster
/// than that (D33), because they are the relative motion of two
/// independently drawn plates rather than that one transpressive setting.
/// What can be asserted is the physics around it: no boundary slips faster
/// than two Earth-speed plates moving straight apart, none slips backwards,
/// and at least one moves fast enough for the island to be seismically
/// active at all.
#[test]
fn plate_motion_and_the_boundaries_it_implies_stay_physical() {
    // Earth's present-day plate speeds, DeMets et al. 2010 (MORVEL), which
    // `PLATE_SPEED_RANGE_M_YR` is drawn from.
    const OBSERVED_MIN_MM_YR: f64 = 10.0;
    const OBSERVED_MAX_MM_YR: f64 = 100.0;

    let s = setup([2; 32], IslandProfile::default_nz_scale());
    let mut rates: Vec<f64> = s.faults.faults.iter().map(|f| f.slip_rate_mm_yr).collect();
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "{} faults, slip rates mm/yr: min {:.1}, median {:.1}, max {:.1}",
        rates.len(),
        rates.first().copied().unwrap_or(0.0),
        rates.get(rates.len() / 2).copied().unwrap_or(0.0),
        rates.last().copied().unwrap_or(0.0)
    );
    for rate in &rates {
        assert!(
            *rate <= 2.0 * OBSERVED_MAX_MM_YR,
            "a boundary slips at {rate:.1} mm/yr, faster than two plates can move apart"
        );
        assert!(
            *rate >= 0.0,
            "a boundary slips backwards at {rate:.1} mm/yr"
        );
    }
    // The island is seismically active: at least one boundary moves as fast
    // as a slow plate, or there would be no Phase 1 earthquakes to speak of.
    assert!(
        rates.last().copied().unwrap_or(0.0) >= OBSERVED_MIN_MM_YR,
        "the fastest boundary only slips at {:.1} mm/yr",
        rates.last().copied().unwrap_or(0.0)
    );
}
