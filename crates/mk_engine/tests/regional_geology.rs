//! Phase 1 Task 5: rock types and primary mineral deposits.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::deposits::{allowed_hosts, DepositKind};
use mk_engine::regional::geology::Lithology;
use mk_engine::regional::geophysics::{generate_regional_geophysics, RegionalGeophysics};
use mk_engine::regional::terrain::landform_scale;
use mk_engine::tectonics::BoundaryType;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

fn generate(profile: IslandProfile, seed: [u8; 32]) -> Option<(IslandDomain, RegionalGeophysics)> {
    let c = canon();
    let d = IslandDomain::from_profile(profile).unwrap();
    let b = sample_regional_boundaries(seed, &c, &d, 0.0);
    generate_regional_geophysics(&c, &d, &b, seed)
        .ok()
        .map(|g| (d, g))
}

/// The small-island seeds 0..32 that make an island of the right area.
fn islands() -> Vec<(IslandDomain, RegionalGeophysics)> {
    (0u8..32)
        .filter_map(|s| generate(IslandProfile::test_small(), [s; 32]))
        .collect()
}

#[test]
fn lithology_and_deposits_are_deterministic() {
    let (_, a) = generate(IslandProfile::test_small(), [4; 32]).unwrap();
    let (_, b) = generate(IslandProfile::test_small(), [4; 32]).unwrap();
    assert_eq!(a.lithology.data(), b.lithology.data());
    assert_eq!(
        serde_json::to_vec(&a.deposits).unwrap(),
        serde_json::to_vec(&b.deposits).unwrap()
    );
}

#[test]
fn rules_hold_and_every_deposit_kind_forms_somewhere() {
    let islands = islands();
    assert!(
        islands.len() >= 16,
        "only {} seeds made an island",
        islands.len()
    );
    let mut kinds = BTreeSet::new();
    let (mut gold_t, mut diamond_t, mut stone_t) = (0.0, 0.0, 0.0);
    for (d, g) in &islands {
        let medium = DomainLevel::Medium;
        let cell_m = d.cell_size_m(medium);
        // Arc volcanics lie along convergent margins.
        let coarse = DomainLevel::Coarse;
        let margin: Vec<(f64, f64)> = (0..d.rows(coarse))
            .flat_map(|r| (0..d.cols(coarse)).map(move |c| (r, c)))
            .filter(|&(r, c)| {
                matches!(
                    g.tectonics.plates.get(r, c).boundary,
                    Some(BoundaryType::Subduction | BoundaryType::Collision)
                )
            })
            .map(|(r, c)| d.cell_center_m(coarse, r, c))
            .collect();
        let reach_m = 250_000.0 * landform_scale(d) + 25_000.0;
        let (mut arc, mut near) = (0usize, 0usize);
        for r in (0..d.rows(medium)).step_by(3) {
            for c in (0..d.cols(medium)).step_by(3) {
                if *g.lithology.get(r, c) == Lithology::ArcAndesite {
                    arc += 1;
                    let (x, y) = d.cell_center_m(medium, r, c);
                    if margin
                        .iter()
                        .any(|&(mx, my)| (x - mx).hypot(y - my) <= reach_m)
                    {
                        near += 1;
                    }
                }
            }
        }
        if arc > 0 {
            assert!(
                near as f64 >= 0.9 * arc as f64,
                "{near} of {arc} arc cells near a margin"
            );
        }
        // Kimberlite only inside the craton.
        for r in 0..d.rows(medium) {
            for c in 0..d.cols(medium) {
                if *g.lithology.get(r, c) == Lithology::Kimberlite {
                    let b = g.basement.expect("kimberlite needs a craton");
                    let (x, y) = d.cell_center_m(medium, r, c);
                    assert!((x - b.centre_m.0).hypot(y - b.centre_m.1) < 1.5 * b.radius_m);
                }
            }
        }
        for dep in &g.deposits {
            kinds.insert(dep.kind);
            assert!(
                allowed_hosts(dep.kind).contains(&dep.host),
                "{:?} in {:?}",
                dep.kind,
                dep.host
            );
            assert_eq!(*g.lithology.get(dep.cell.0, dep.cell.1), dep.host);
            assert!(dep.tonnage_t > 0.0 && dep.grade > 0.0 && dep.depth_m >= 0.0);
            match dep.kind {
                // Contained metal or gem mass (t).
                DepositKind::OrogenicGold | DepositKind::EpithermalGoldSilver => {
                    gold_t += dep.tonnage_t * dep.grade * 1e-6
                }
                DepositKind::Kimberlite => diamond_t += dep.tonnage_t * dep.grade / 100.0 * 2e-7,
                DepositKind::BuildingStone => stone_t += dep.tonnage_t,
                _ => {}
            }
        }
        let _ = cell_m;
    }
    let missing: Vec<_> = DepositKind::ALL
        .iter()
        .filter(|k| !kinds.contains(k))
        .collect();
    assert!(missing.is_empty(), "never formed: {missing:?}");
    assert!(gold_t > 0.0 && diamond_t > 0.0);
    assert!(
        stone_t >= 100.0 * gold_t,
        "stone {stone_t:e} t vs gold {gold_t:e} t"
    );
    assert!(
        stone_t >= 100.0 * diamond_t,
        "stone {stone_t:e} t vs diamond {diamond_t:e} t"
    );
}

#[test]
fn a_young_arc_without_a_craton_has_no_diamonds() {
    let mut profile = IslandProfile::test_small();
    profile.geology.ancient_basement = false;
    let (_, g) = generate(profile, [4; 32]).unwrap();
    assert!(g.basement.is_none());
    assert!(!g
        .lithology
        .data()
        .iter()
        .any(|l| matches!(l, Lithology::Kimberlite | Lithology::CratonicGneiss)));
    assert!(!g.deposits.iter().any(|d| matches!(
        d.kind,
        DepositKind::Kimberlite | DepositKind::BandedIronFormation
    )));
}

#[test]
fn the_full_island_has_varied_geology() {
    let (_, g) = generate(IslandProfile::default_nz_scale(), [0; 32]).unwrap();
    let rocks: BTreeSet<_> = g.lithology.data().iter().collect();
    assert!(rocks.len() >= 12, "{rocks:?}");
    assert!(g
        .deposits
        .iter()
        .any(|d| d.kind == DepositKind::OrogenicGold));
}
