//! Phase 1 Task 3: regional tectonics on the island domain.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::tectonics::{
    regional_neighbours, regional_plates, step_regional_tectonics,
};
use mk_engine::tectonics::BoundaryType;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

fn run(
    seed: [u8; 32],
    profile: IslandProfile,
) -> (IslandDomain, mk_engine::tectonics::TectonicsState) {
    let c = canon();
    let d = IslandDomain::from_profile(profile).unwrap();
    let b = sample_regional_boundaries(seed, &c, &d, 0.0);
    let t = step_regional_tectonics(&c, 0, &d, &b.tectonic);
    (d, t)
}

#[test]
fn plate_assignment_is_deterministic() {
    let (_, a) = run([3; 32], IslandProfile::test_small());
    let (_, b) = run([3; 32], IslandProfile::test_small());
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
    let (_, other) = run([4; 32], IslandProfile::test_small());
    assert_ne!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&other).unwrap()
    );
}

#[test]
fn every_seed_has_several_plates_a_convergent_and_a_divergent_boundary() {
    for seed in 0u8..16 {
        let (d, t) = run([seed; 32], IslandProfile::default_nz_scale());
        let (rows, cols) = (d.rows(DomainLevel::Coarse), d.cols(DomainLevel::Coarse));
        let mut ids = BTreeSet::new();
        let mut kinds = BTreeSet::new();
        for r in 0..rows {
            for c in 0..cols {
                let cell = t.plates.get(r, c);
                ids.insert(cell.plate_id);
                if let Some(b) = cell.boundary {
                    kinds.insert(format!("{b:?}"));
                }
            }
        }
        assert!(ids.len() >= 3, "seed {seed}: {} plates", ids.len());
        let convergent = kinds.contains("Subduction") || kinds.contains("Collision");
        assert!(convergent, "seed {seed}: {kinds:?}");
        assert!(kinds.contains("Ridge"), "seed {seed}: {kinds:?}");
    }
}

#[test]
fn the_core_margin_runs_through_the_middle_of_the_domain() {
    let (d, t) = run([5; 32], IslandProfile::default_nz_scale());
    let (rows, cols) = (d.rows(DomainLevel::Coarse), d.cols(DomainLevel::Coarse));
    let near_centre = (rows / 4..3 * rows / 4)
        .flat_map(|r| (cols / 4..3 * cols / 4).map(move |c| (r, c)))
        .filter(|&(r, c)| {
            matches!(
                t.plates.get(r, c).boundary,
                Some(BoundaryType::Subduction | BoundaryType::Collision)
            )
        })
        .count();
    assert!(
        near_centre > 10,
        "{near_centre} convergent cells near the centre"
    );
}

#[test]
fn heat_flow_is_finite_and_realistic() {
    let (_, t) = run([7; 32], IslandProfile::default_nz_scale());
    let data = t.heat_flow.data();
    assert!(data
        .iter()
        .all(|q| q.is_finite() && (20.0..=200.0).contains(q)));
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    // Global means: continents 65, oceans 101 mW/m² (Pollack et al. 1993).
    assert!((40.0..=130.0).contains(&mean), "{mean}");
}

#[test]
fn plate_speeds_are_earth_like() {
    let c = canon();
    let d = IslandDomain::from_profile(IslandProfile::default_nz_scale()).unwrap();
    let b = sample_regional_boundaries([8; 32], &c, &d, 0.0);
    for p in regional_plates(&d, &b.tectonic) {
        let speed = p.velocity_east_m_yr.hypot(p.velocity_north_m_yr);
        assert!(speed <= 0.1, "plate {} moves {speed} m/yr", p.id);
    }
}

#[test]
fn neighbours_never_wrap_across_the_domain() {
    let (rows, cols) = (32, 40);
    let west: Vec<_> = regional_neighbours(10, 0, rows, cols).collect();
    assert!(west.iter().all(|&(_, c)| c <= 1), "{west:?}");
    assert_eq!(west.len(), 5);
    let east: Vec<_> = regional_neighbours(10, cols - 1, rows, cols).collect();
    assert!(east.iter().all(|&(_, c)| c >= cols - 2));
    let corner: Vec<_> = regional_neighbours(0, 0, rows, cols).collect();
    assert_eq!(corner.len(), 3);
    assert_eq!(regional_neighbours(5, 5, rows, cols).count(), 8);
}
