//! Phase 1 Task 4: one island of the right size and shape, inside its
//! ocean buffer, from regional geophysics.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::geophysics::{
    bootstrap_regional_geophysics, primary_land_component, RegionalGeophysics,
    RegionalGeophysicsError,
};
use mk_engine::regional::volcanism::RegionalVolcanismGeometry;
use mk_engine::tectonics::BoundaryType;
use mk_engine::volcanism::VolcanismGeometry;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

/// Provisional passing seeds, found by a deterministic search over
/// `[0u8; 32]`, `[1u8; 32]`, …; Task 7 replaces the full-island seed with
/// the owner's choice.
const FULL_SEED: [u8; 32] = [0u8; 32];
const SMALL_SEED: [u8; 32] = [4u8; 32];
/// A small-island seed whose outline fails the shape requirements.
const SMALL_FAILING_SEED: [u8; 32] = [1u8; 32];

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

fn build(
    profile: IslandProfile,
    seed: [u8; 32],
) -> (
    IslandDomain,
    Result<RegionalGeophysics, RegionalGeophysicsError>,
) {
    let c = canon();
    let d = IslandDomain::from_profile(profile).unwrap();
    let b = sample_regional_boundaries(seed, &c, &d, 0.0);
    let g = bootstrap_regional_geophysics(&c, &d, &b, seed);
    (d, g)
}

fn check_island(d: &IslandDomain, g: &RegionalGeophysics) {
    let p = d.profile();
    let (lo, hi) = p.land_area_bounds_m2();
    assert!(
        g.land_area_m2 >= lo && g.land_area_m2 <= hi,
        "{} km²",
        g.land_area_m2 / 1e6
    );

    // One connected landmass: the land mask is its own primary component.
    assert_eq!(
        primary_land_component(&g.elevation_m).data(),
        g.land_mask.data()
    );

    let medium = DomainLevel::Medium;
    let (rows, cols) = (d.rows(medium), d.cols(medium));
    for r in 0..rows {
        for c in 0..cols {
            let e = *g.elevation_m.get(r, c);
            let land = *g.land_mask.get(r, c);
            assert_eq!(land, e > 0.0);
            if d.is_edge_buffer_cell(medium, r, c) {
                assert!(!land, "land in the ocean buffer at ({r}, {c})");
                assert!(*g.bathymetry_m.get(r, c) < 0.0);
            }
            if !land {
                assert!(*g.bathymetry_m.get(r, c) <= 0.0);
            }
        }
    }
    // Real relief: mountains above 1.5 km, deep ocean beyond the shelf.
    let max = g
        .elevation_m
        .data()
        .iter()
        .cloned()
        .fold(f64::MIN, f64::max);
    let min = g
        .elevation_m
        .data()
        .iter()
        .cloned()
        .fold(f64::MAX, f64::min);
    assert!(max > 1_500.0, "highest point {max} m");
    assert!(min < -3_000.0, "deepest point {min} m");
    assert!(p.shape.unmet(&g.shape).is_empty());
}

#[test]
fn the_full_island_meets_area_buffer_connectivity_and_shape() {
    let (d, g) = build(IslandProfile::default_nz_scale(), FULL_SEED);
    let g = g.expect("provisional full seed makes an island");
    check_island(&d, &g);
    // Only regional grids exist: coarse tectonics, medium terrain.
    assert_eq!(
        (g.tectonics.plates.nlat(), g.tectonics.plates.nlon()),
        (160, 200)
    );
    assert_eq!((g.elevation_m.nlat(), g.elevation_m.nlon()), (960, 1_200));
}

#[test]
fn the_small_island_meets_area_buffer_connectivity_and_shape() {
    let (d, g) = build(IslandProfile::test_small(), SMALL_SEED);
    check_island(&d, &g.expect("provisional small seed makes an island"));
}

#[test]
fn generation_is_deterministic_to_the_byte() {
    let bytes = |g: &RegionalGeophysics| {
        g.elevation_m
            .data()
            .iter()
            .flat_map(|e| e.to_le_bytes())
            .collect::<Vec<u8>>()
    };
    let (_, a) = build(IslandProfile::test_small(), SMALL_SEED);
    let (_, b) = build(IslandProfile::test_small(), SMALL_SEED);
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(bytes(&a), bytes(&b));
    assert_eq!(
        a.sea_level_offset_m.to_bits(),
        b.sea_level_offset_m.to_bits()
    );
}

#[test]
fn a_failing_seed_reports_its_shape_metrics() {
    let (_, g) = build(IslandProfile::test_small(), SMALL_FAILING_SEED);
    match g {
        Err(RegionalGeophysicsError::ShapeRequirementsUnmet { metrics, reasons }) => {
            assert!(!reasons.is_empty());
            assert!(metrics.area_m2 > 0.0);
        }
        other => panic!(
            "expected ShapeRequirementsUnmet, got {:?}",
            other.map(|g| g.shape)
        ),
    }
}

#[test]
fn volcanism_sits_on_convergent_boundaries_and_never_wraps() {
    let (d, g) = build(IslandProfile::test_small(), SMALL_SEED);
    let g = g.unwrap();
    let (rows, cols) = (d.rows(DomainLevel::Coarse), d.cols(DomainLevel::Coarse));
    let mut active_on_margin = 0;
    for r in 0..rows {
        for c in 0..cols {
            let v = g.volcanism.volcanism.get(r, c);
            if v.dominates_cell() {
                let b = g.tectonics.plates.get(r, c).boundary;
                assert!(
                    b.is_some(),
                    "volcanic ground off any boundary at ({r}, {c})"
                );
                if b == Some(BoundaryType::Subduction) {
                    active_on_margin += 1;
                }
            }
        }
    }
    assert!(active_on_margin > 0);
    let geometry = RegionalVolcanismGeometry::coarse(&d);
    for r in 0..rows {
        assert!(geometry.neighbours(r, 0).iter().all(|&(_, c, _, _)| c <= 1));
        assert!(geometry
            .neighbours(r, cols - 1)
            .iter()
            .all(|&(_, c, _, _)| c >= cols - 2));
    }
}
