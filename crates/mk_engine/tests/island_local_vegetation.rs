//! Phase 3 Task 7: individual trees and stand cover around the estate.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::local_vegetation::{
    carbon_of_diameter, diameter_of_carbon, seed_local_vegetation, MIN_STEM_DIAMETER_M,
};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::property::{place_regional_estate, PlacedEstate};
use mk_island::{DomainLevel, EstatePatchConfig, IslandDomain, IslandScenario};

const M: DomainLevel = DomainLevel::Medium;

struct Base {
    domain: IslandDomain,
    ecology: RegionalEcologyState,
    config: EstatePatchConfig,
    seed: [u8; 32],
    placed: PlacedEstate,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let scenario =
            IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let domain = IslandDomain::from_profile(scenario.profile.clone()).unwrap();
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, scenario.seed).unwrap();
        let ecology = RegionalEcologyState::bootstrap(&domain, &physical).unwrap();
        let placed = place_regional_estate(
            &canon,
            &physical,
            &ecology,
            &domain,
            &scenario.estate_patch,
            scenario.seed,
        )
        .unwrap();
        Base {
            domain,
            ecology,
            config: scenario.estate_patch,
            seed: scenario.seed,
            placed,
        }
    })
}

/// The ecology with the four estate cells set to `biome` and `biomass`.
fn variant(b: &Base, biome: BiomeType, biomass: f64) -> RegionalEcologyState {
    let mut e = b.ecology.clone();
    let (row, col) = b.placed.location;
    for (r, c) in [
        (row, col),
        (row, col + 1),
        (row + 1, col),
        (row + 1, col + 1),
    ] {
        e.biome_grid.set(r, c, biome);
        e.biomass_kgc_m2.set(r, c, biomass);
    }
    e
}

fn block_carbon(b: &Base, e: &RegionalEcologyState) -> f64 {
    let (row, col) = b.placed.location;
    let area = b.domain.cell_area_m2(M);
    [
        (row, col),
        (row, col + 1),
        (row + 1, col),
        (row + 1, col + 1),
    ]
    .iter()
    .map(|&(r, c)| e.biomass_kgc_m2.get(r, c) * area)
    .sum()
}

fn seed(
    b: &Base,
    e: &RegionalEcologyState,
    cap: usize,
) -> mk_engine::regional::local_vegetation::LocalVegetationPatch {
    let config = EstatePatchConfig {
        tree_cap: cap,
        ..b.config
    };
    seed_local_vegetation(e, &b.placed.layout, &b.domain, &config, b.seed).unwrap()
}

#[test]
fn stem_density_follows_biome_and_biomass_and_water_has_none() {
    let b = base();
    let forest = seed(b, &variant(b, BiomeType::TemperateForest, 9.0), 1_000_000);
    let grass = seed(b, &variant(b, BiomeType::Grassland, 1.0), 1_000_000);
    let water = seed(b, &variant(b, BiomeType::ShallowOcean, 0.0), 1_000_000);
    println!(
        "forest trees {}, grassland {}, water {}",
        forest.trees.len(),
        grass.trees.len(),
        water.trees.len()
    );
    assert!(forest.trees.len() > 1_000, "forest {}", forest.trees.len());
    assert_eq!(grass.trees.len(), 0, "grassland has no individual stems");
    assert_eq!(water.trees.len(), 0);
    assert_eq!(water.total_carbon_kgc(), 0.0);
    // Grass is stand cover, not nothing.
    assert!(grass.total_carbon_kgc() > 0.0);
    // Every individual stem is at least 10 cm and carries its allometric carbon.
    for t in &forest.trees {
        assert!(
            t.stem_diameter_m >= MIN_STEM_DIAMETER_M - 1e-6,
            "{}",
            t.stem_diameter_m
        );
        assert!(
            (carbon_of_diameter(t.stem_diameter_m) - t.biomass_kgc).abs() <= 1e-6 * t.biomass_kgc
        );
        assert!(t.height_m > 1.3 && t.alive);
    }
    // A richer forest has more stems than a sparser one.
    let sparse = seed(b, &variant(b, BiomeType::TemperateForest, 3.0), 1_000_000);
    assert!(forest.trees.len() > sparse.trees.len());
    // The allometry inverts.
    let d = diameter_of_carbon(carbon_of_diameter(0.4));
    assert!((d - 0.4).abs() < 1e-6);
}

#[test]
fn trees_and_stands_carry_the_estate_blocks_biomass_with_the_cap_off_and_on() {
    let b = base();
    let e = variant(b, BiomeType::TemperateForest, 9.0);
    let total = block_carbon(b, &e);
    let yard = b.placed.layout.yard;
    for cap in [10_000_000usize, 1_000] {
        let p = seed(b, &e, cap);
        let have = p.total_carbon_kgc();
        assert!(
            (have - total).abs() <= 0.02 * total,
            "cap {cap}: patch {have:.4e} kgC vs block {total:.4e}"
        );
        assert!(
            p.trees.len() <= cap,
            "{} trees over cap {cap}",
            p.trees.len()
        );
        // No tree stands in the estate's yard (footprints included).
        assert!(p
            .trees
            .iter()
            .all(|t| !yard.contains(t.position_m.0, t.position_m.1)));
        // Nor does vegetation cover the yard ground.
        let patch = &b.placed.patch;
        for r in 0..patch.rows {
            for c in 0..patch.cols {
                let (x, y) = patch.cell_center_m(r, c);
                if yard.contains(x, y) {
                    assert_eq!(p.stands.get(r, c).biomass_kgc, 0.0);
                }
            }
        }
        if cap == 1_000 {
            // The radius shrank to fit, and the displaced carbon stayed in stands.
            assert!(p.individual_radius_m < b.config.individual_tree_radius_m);
            assert!(p.trees.len() > 100, "{}", p.trees.len());
        }
    }
}

#[test]
fn felling_removes_the_stem_adds_wood_and_moves_exactly_its_carbon() {
    let b = base();
    let mut e = variant(b, BiomeType::TemperateForest, 9.0);
    let mut p = seed(b, &e, 1_000_000);
    let tree = p.trees[p.trees.len() / 2].clone();
    let before_patch = p.total_carbon_kgc();
    let (row, col) = (
        (tree.position_m.1 / b.domain.cell_size_m(M)).floor() as usize,
        (tree.position_m.0 / b.domain.cell_size_m(M)).floor() as usize,
    );
    let before_cell = *e.biomass_kgc_m2.get(row, col);
    let mut ledger = Ledger::new();
    let wood = p.fell_tree(tree.id, &mut e, &mut ledger).unwrap();

    assert!(p.trees.iter().all(|t| t.id != tree.id), "the stem is gone");
    assert_eq!(wood.carbon_kgc, tree.biomass_kgc);
    assert!((wood.wood_kg - tree.biomass_kgc / 0.475 * 0.7).abs() < 1e-9);
    assert!((before_patch - p.total_carbon_kgc() - tree.biomass_kgc).abs() < 1e-6);
    let cell_loss = (before_cell - *e.biomass_kgc_m2.get(row, col)) * b.domain.cell_area_m2(M);
    assert!((cell_loss - tree.biomass_kgc).abs() < 1e-6 * tree.biomass_kgc.max(1.0));
    let entry = ledger
        .entries()
        .iter()
        .find(|x| x.kind == FluxKind::Carbon)
        .unwrap();
    assert_eq!(
        (entry.source, entry.sink),
        (Reservoir::BiomassCarbon, Reservoir::DetritusCarbon)
    );
    assert_eq!(entry.amount, tree.biomass_kgc);
    // Felling it again, or a tree that never existed, is an error.
    assert!(p.fell_tree(tree.id, &mut e, &mut ledger).is_err());
    assert!(p.fell_tree(u64::MAX, &mut e, &mut ledger).is_err());
}

#[test]
fn the_patch_follows_the_field_and_is_deterministic() {
    let b = base();
    let mut e = variant(b, BiomeType::TemperateForest, 9.0);
    let (a, c) = (seed(b, &e, 1_000_000), seed(b, &e, 1_000_000));
    assert_eq!(a.trees, c.trees);
    assert_eq!(a.stands.data(), c.stands.data());

    // The field grows 10%: the patch's carbon follows, stems thicken.
    let mut p = a;
    let (row, col) = b.placed.location;
    for (r, k) in [
        (row, col),
        (row, col + 1),
        (row + 1, col),
        (row + 1, col + 1),
    ] {
        let v = *e.biomass_kgc_m2.get(r, k);
        e.biomass_kgc_m2.set(r, k, v * 1.1);
    }
    let before = p.total_carbon_kgc();
    let mean_d: f64 = p.trees.iter().map(|t| t.stem_diameter_m).sum::<f64>() / p.trees.len() as f64;
    p.step(&e, 86_400);
    let after = p.total_carbon_kgc();
    assert!(
        (after / before - 1.1).abs() < 0.01,
        "carbon ratio {}",
        after / before
    );
    let mean_after: f64 =
        p.trees.iter().map(|t| t.stem_diameter_m).sum::<f64>() / p.trees.len() as f64;
    assert!(mean_after > mean_d);

    // A decade of mortality removes some trees but not their carbon.
    let n = p.trees.len();
    let carbon = p.total_carbon_kgc();
    p.step(&e, 10 * 365 * 86_400);
    assert!(p.trees.len() < n, "no tree died in a decade");
    assert!((p.total_carbon_kgc() / carbon - 1.0).abs() < 0.01);
    // A zero step changes nothing.
    let snapshot = p.trees.clone();
    p.step(&e, 0);
    assert_eq!(p.trees, snapshot);
}

/// The patch and every tree in it survive a snapshot unchanged.
///
/// Phase 4 Task 3's largest state by far: the owner's estate carries about
/// 200,000 individual stems, each with its own position, species, diameter
/// and age. A round trip that lost or rounded any of them would change the
/// island's carbon and its state digest.
#[test]
fn the_vegetation_patch_round_trips_through_a_snapshot() {
    let b = base();
    let patch = seed(b, &variant(b, BiomeType::TemperateForest, 9.0), 20_000);
    let before = patch.trees.len();
    assert!(before > 1_000, "only {before} trees to round trip");

    let text = serde_json::to_string(&patch).expect("the patch serializes");
    let back: mk_engine::regional::local_vegetation::LocalVegetationPatch =
        serde_json::from_str(&text).expect("and comes back");

    assert_eq!(
        back.trees.len(),
        before,
        "trees were lost in the round trip"
    );
    assert_eq!(back.trees, patch.trees, "a tree changed in the round trip");
    assert_eq!(
        back.total_carbon_kgc(),
        patch.total_carbon_kgc(),
        "the patch's carbon changed in the round trip"
    );
    assert_eq!(
        back.stands.data(),
        patch.stands.data(),
        "stand cover changed in the round trip"
    );
}
