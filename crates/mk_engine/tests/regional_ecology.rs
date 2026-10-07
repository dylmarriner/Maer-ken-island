//! Phase 3 Task 1: regional biomes, primary production and producer
//! biomass on the owner's island.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_engine::organisms::vegetation::PlantKind;
use mk_engine::regional::ecology::{residence_years, RegionalEcologyState};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const M: DomainLevel = DomainLevel::Medium;

struct Base {
    domain: IslandDomain,
    physical: RegionalPhysicalState,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().unwrap();
        let domain = IslandDomain::from_profile(profile).unwrap();
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, seed).unwrap();
        Base { domain, physical }
    })
}

fn ecology() -> RegionalEcologyState {
    let b = base();
    RegionalEcologyState::bootstrap(&b.domain, &b.physical).unwrap()
}

#[test]
fn land_and_sea_biomes_agree_with_the_terrain_and_plants_stand_on_land() {
    let b = base();
    let e = ecology();
    let elevation = &b.physical.geophysics.elevation_m;
    let net = b.physical.flow_network();
    let (rows, cols) = (b.domain.rows(M), b.domain.cols(M));
    let (mut land_cells, mut sea_cells) = (0usize, 0usize);
    for r in 0..rows {
        for c in 0..cols {
            let biome = *e.biome_grid.get(r, c);
            if *elevation.get(r, c) > 0.0 {
                land_cells += 1;
                // Land is a land biome, a river, or (for lakes) the
                // aquatic biome that keeps land plants out.
                if net.lake_depth_m(r, c) == 0.0 {
                    assert!(biome.is_terrestrial(), "({r},{c}) land is {biome:?}");
                }
            } else {
                sea_cells += 1;
                assert!(biome.is_aquatic(), "({r},{c}) sea is {biome:?}");
                // No biomass or production over the ocean.
                assert_eq!(*e.biomass_kgc_m2.get(r, c), 0.0);
                assert_eq!(*e.npp_kgc_m2_yr.get(r, c), 0.0);
            }
        }
    }
    assert!(land_cells > 0 && sea_cells > land_cells);

    // The sparse plant view is on land, in land biomes, and has trees,
    // shrubs or grass.
    assert!(!e.vegetation.plants.is_empty());
    let mut kinds = std::collections::HashSet::new();
    for p in &e.vegetation.plants {
        let (r, c) = (p.position.row as usize, p.position.col as usize);
        assert!(*elevation.get(r, c) > 0.0, "plant {} in the sea", p.id);
        assert!(!e.biome_grid.get(r, c).is_aquatic());
        kinds.insert(format!("{:?}", p.kind));
    }
    assert!(
        kinds.contains(&format!("{:?}", PlantKind::Tree))
            || kinds.contains(&format!("{:?}", PlantKind::Shrub))
            || kinds.contains(&format!("{:?}", PlantKind::Grass)),
        "{kinds:?}"
    );
}

#[test]
fn production_is_higher_in_the_warm_wet_lowland_than_on_the_cold_heights() {
    let b = base();
    let e = ecology();
    let elevation = &b.physical.geophysics.elevation_m;
    let mean = |pred: &dyn Fn(f64) -> bool| {
        let (mut sum, mut n) = (0.0, 0.0);
        for (i, &h) in elevation.data().iter().enumerate() {
            let npp = e.npp_kgc_m2_yr.data()[i];
            if h > 0.0 && pred(h) && e.biome_grid.data()[i] != BiomeType::River {
                sum += npp;
                n += 1.0;
            }
        }
        (n > 0.0).then(|| sum / n)
    };
    let lowland = mean(&|h| h < 300.0).expect("lowland");
    let heights = mean(&|h| h > 1_200.0).expect("heights");
    println!("mean NPP: lowland {lowland:.3}, heights {heights:.3} kgC/m2/yr");
    assert!(lowland > heights, "lowland {lowland} vs heights {heights}");
    // Plausible magnitudes: 0 to ~1.5 kgC/m2/yr everywhere.
    assert!(e
        .npp_kgc_m2_yr
        .data()
        .iter()
        .all(|&n| (0.0..=1.5).contains(&n)));
}

#[test]
fn biomass_starts_at_equilibrium_stays_there_and_renormalises_to_a_total() {
    let b = base();
    let mut e = ecology();
    // Initial biomass is NPP x residence time of the biome.
    for i in 0..e.biomass_kgc_m2.data().len() {
        let want = e.npp_kgc_m2_yr.data()[i] * residence_years(e.biome_grid.data()[i]);
        assert!((e.biomass_kgc_m2.data()[i] - want).abs() < 1e-12);
    }
    let start = e.total_biomass_kgc(&b.domain);
    assert!(start > 0.0);

    // At equilibrium, a long step changes nothing (to rounding).
    e.step(5.0 * 365.25 * 86_400.0, None, None);
    assert!((e.total_biomass_kgc(&b.domain) - start).abs() <= 1e-9 * start);

    // Harvest and disturbance reduce it; it never goes negative and
    // regrows toward equilibrium when they stop.
    let spec = b.domain.storage_spec(M);
    let harvest = mk_core::grid::Grid2::new(&spec, 0.5);
    let disturbance = mk_core::grid::Grid2::new(&spec, 0.2);
    e.step(365.25 * 86_400.0, Some(&harvest), Some(&disturbance));
    let cut = e.total_biomass_kgc(&b.domain);
    assert!(cut < start && e.biomass_kgc_m2.data().iter().all(|&v| v >= 0.0));
    // 400 years is 16 residence times of the slowest (25-year) forest.
    e.step(400.0 * 365.25 * 86_400.0, None, None);
    assert!((e.total_biomass_kgc(&b.domain) - start).abs() <= 1e-3 * start);

    // The producer species' carbon is authoritative: the field is
    // rescaled so the land total equals it.
    e.renormalise_to_total(&b.domain, 0.5 * start);
    assert!((e.total_biomass_kgc(&b.domain) - 0.5 * start).abs() <= 1e-9 * start);
    // A zero step changes nothing.
    let before = e.total_biomass_kgc(&b.domain);
    e.step(0.0, None, None);
    assert_eq!(e.total_biomass_kgc(&b.domain), before);
}

#[test]
fn bootstrap_is_deterministic() {
    let (a, b) = (ecology(), ecology());
    assert_eq!(a.biome_grid.data(), b.biome_grid.data());
    assert_eq!(a.npp_kgc_m2_yr.data(), b.npp_kgc_m2_yr.data());
    assert_eq!(a.biomass_kgc_m2.data(), b.biomass_kgc_m2.data());
    assert_eq!(a.vegetation.plants.len(), b.vegetation.plants.len());
}
