//! Phase 3 Task 2: the island's mineral resource sites on the owner's
//! island (`regional::resources`).

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_engine::regional::deposits::DepositKind;
use mk_engine::regional::geology::Lithology;
use mk_engine::regional::hydrology::Receiver;
use mk_engine::regional::labour::{LabourTable, LabourTool};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::resources::{IslandResources, ResourceError, SecondaryModels, SiteKind};
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

struct Base {
    canon: Arc<CanonLocked>,
    seed: [u8; 32],
    domain: IslandDomain,
    physical: RegionalPhysicalState,
    models: SecondaryModels,
    resources: IslandResources,
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
        let models = SecondaryModels::load_default().unwrap();
        let resources = IslandResources::survey(&physical, &domain, &seed, &models);
        Base {
            canon,
            seed,
            domain,
            physical,
            models,
            resources,
        }
    })
}

fn cols() -> usize {
    base().domain.cols(DomainLevel::Medium)
}

/// The cells water from `from` passes through, `from` included.
fn downstream_of(from: (usize, usize)) -> Vec<(usize, usize)> {
    let b = base();
    let network = b.physical.flow_network();
    let mut path = vec![from];
    let mut here = from;
    while let Receiver::Cell(next) = network.receiver_of(here.0, here.1) {
        here = (next as usize / cols(), next as usize % cols());
        path.push(here);
    }
    path
}

#[test]
fn the_secondary_models_are_the_sources_numbers() {
    let m = base().models;
    assert_eq!(m.placer_median_t, 1.1e6);
    assert_eq!(m.placer_median_g_t, 0.2);
    // Orris & Bliss: 0.022 to 50 Mt and 0.084 to 0.48 g/t between the 90th
    // and 10th percentiles.
    assert!((m.placer_tonnage_log10_sd - 1.310).abs() < 1e-3);
    assert!((m.placer_grade_log10_sd - 0.295).abs() < 1e-3);
    assert_eq!(m.placer_gravel_t_m3, 2.0);
    // 767.5 Mt of magnetics in 24 onshore deposits (GNS 2024, Table 7.2).
    assert!((m.ironsand_mean_t - 767.5e6 / 24.0).abs() < 1.0);
    assert_eq!(m.ironsand_concentrate_fe_pct, 55.0);
}

#[test]
fn every_primary_deposit_is_one_site_with_its_own_tonnage() {
    let b = base();
    let deposits = &b.physical.geophysics.deposits;
    let primaries: Vec<_> = b
        .resources
        .sites()
        .iter()
        .filter(|s| matches!(s.kind, SiteKind::Primary(_)))
        .collect();
    assert_eq!(primaries.len(), deposits.len());
    for d in deposits {
        let matching = primaries
            .iter()
            .filter(|s| {
                s.cell == d.cell
                    && s.kind == SiteKind::Primary(d.kind)
                    && s.tonnage_t == d.tonnage_t
                    && s.grade == d.grade
                    && s.depth_m == d.depth_m
            })
            .count();
        assert_eq!(matching, 1, "deposit {} has {matching} sites", d.id);
    }
    let count = |kind| {
        primaries
            .iter()
            .filter(|s| s.kind == SiteKind::Primary(kind))
            .count()
    };
    for kind in DepositKind::ALL {
        println!("{kind:?}: {}", count(kind));
    }
}

#[test]
fn placer_gold_lies_downstream_of_a_gold_source_and_only_there() {
    let b = base();
    let deposits = &b.physical.geophysics.deposits;
    let placers: Vec<_> = b
        .resources
        .sites()
        .iter()
        .filter(|s| s.kind == SiteKind::PlacerGold)
        .collect();
    let sources = deposits
        .iter()
        .filter(|d| {
            d.kind
                .yields()
                .contains(&mk_engine::materials::catalogue::CatalogueItem::GoldOre)
        })
        .count();
    println!(
        "{} placers from {sources} gold-bearing deposits",
        placers.len()
    );
    assert!(sources > 0, "the owner's island has gold-bearing deposits");
    assert!(!placers.is_empty());
    assert!(placers.len() <= sources);
    let elevation = &b.physical.geophysics.elevation_m;
    for p in placers {
        let source = &deposits[p.source_deposit.expect("a placer has a source") as usize];
        assert!(source
            .kind
            .yields()
            .contains(&mk_engine::materials::catalogue::CatalogueItem::GoldOre));
        let path = downstream_of(source.cell);
        assert!(
            path[1..].contains(&p.cell),
            "placer {} at {:?} is not below its source at {:?}",
            p.id,
            p.cell,
            source.cell
        );
        assert!(
            *elevation.get(p.cell.0, p.cell.1) > 0.0,
            "placers lie on land"
        );
        assert_eq!(p.depth_m, 0.0);
        assert!(p.grade > 0.0 && p.tonnage_t > 0.0);
    }
}

#[test]
fn ironsand_lies_only_at_the_mouths_of_rivers_that_drain_andesite() {
    let b = base();
    let network = b.physical.flow_network();
    let rocks = b.physical.geophysics.lithology.data();
    let ironsand: Vec<_> = b
        .resources
        .sites()
        .iter()
        .filter(|s| s.kind == SiteKind::Ironsand)
        .collect();
    println!("{} ironsand sites", ironsand.len());
    let andesite = rocks.contains(&Lithology::ArcAndesite);
    assert_eq!(!ironsand.is_empty(), andesite);
    for s in &ironsand {
        assert_eq!(network.receiver_of(s.cell.0, s.cell.1), Receiver::Sea);
        assert_eq!(s.tonnage_t, b.models.ironsand_mean_t);
        assert_eq!(s.grade, b.models.ironsand_concentrate_fe_pct);
    }
    // Every ironsand mouth drains some andesite: walking down from every
    // andesite cell reaches each of them from at least one.
    let cols = cols();
    let mut reached = std::collections::BTreeSet::new();
    for (i, rock) in rocks.iter().enumerate() {
        if *rock == Lithology::ArcAndesite
            && *b.physical.geophysics.elevation_m.data().get(i).unwrap() > 0.0
        {
            if let Some(mouth) = downstream_of((i / cols, i % cols)).last() {
                reached.insert(*mouth);
            }
        }
    }
    for s in &ironsand {
        assert!(
            reached.contains(&s.cell),
            "ironsand at {:?} drains no andesite",
            s.cell
        );
    }
}

#[test]
fn the_survey_is_a_pure_function_of_the_island() {
    let b = base();
    let again = IslandResources::survey(&b.physical, &b.domain, &b.seed, &b.models);
    assert_eq!(again, b.resources);
    for (i, s) in b.resources.sites().iter().enumerate() {
        assert_eq!(s.id as usize, i);
    }
}

#[test]
fn the_sites_do_not_change_with_the_weather() {
    // Rivers come from the island's year of flows, not from whatever the
    // last step's discharge was, so a survey days later finds the same
    // sites.
    let b = base();
    let mut later = b.physical.clone();
    let day = 86_400;
    for d in 1..=3u64 {
        later
            .step(&b.canon, &b.domain, b.seed, (d * day) as f64, d, day)
            .unwrap();
    }
    let again = IslandResources::survey(&later, &b.domain, &b.seed, &b.models);
    assert_eq!(again, b.resources);
}

#[test]
fn a_site_runs_out_at_its_tonnage_and_never_refills() {
    let b = base();
    let mut resources = b.resources.clone();
    let site = resources.sites()[0].clone();
    let first = resources.remove(site.id, site.tonnage_t * 0.75).unwrap();
    assert_eq!(first, site.tonnage_t * 0.75);
    let second = resources.remove(site.id, site.tonnage_t).unwrap();
    assert!((first + second - site.tonnage_t).abs() <= 1e-9 * site.tonnage_t);
    assert!(resources.site(site.id).unwrap().is_exhausted());
    assert_eq!(
        resources.remove(site.id, 1.0),
        Err(ResourceError::Exhausted(site.id))
    );
}

#[test]
fn gold_is_panned_with_a_pan_and_not_without_one() {
    let b = base();
    let labour = LabourTable::load_default().unwrap();
    let mut resources = b.resources.clone();
    let placer = resources
        .sites()
        .iter()
        .find(|s| s.kind == SiteKind::PlacerGold)
        .unwrap()
        .clone();
    assert!(matches!(
        resources.work_day(
            placer.id,
            LabourTool::PickAndShovel,
            true,
            &labour,
            &b.models
        ),
        Err(ResourceError::WrongTool { .. })
    ));
    let day = resources
        .work_day(placer.id, LabourTool::GoldPan, true, &labour, &b.models)
        .unwrap();
    let washed_m3 = labour
        .gravel_washed_m3_per_day(LabourTool::GoldPan)
        .unwrap();
    assert!((day.moved_t - washed_m3 * 2.0).abs() < 1e-9);
    let expected = labour
        .pan_gravel(LabourTool::GoldPan, placer.grade * 2.0, true)
        .unwrap()
        .output;
    assert!((day.task.output - expected).abs() < 1e-12);
    assert!((day.product_kg - expected / 1_000.0).abs() < 1e-15);
    assert!(
        (resources.site(placer.id).unwrap().remaining_t - (placer.tonnage_t - day.moved_t)).abs()
            < 1e-6
    );
    println!(
        "a day's panning at {:.3} g/t: {:.2} t of gravel, {:.3} g of gold",
        placer.grade, day.moved_t, day.task.output
    );
}

#[test]
fn deposits_underground_and_oil_are_beyond_hand_tools() {
    let b = base();
    let labour = LabourTable::load_default().unwrap();
    let mut resources = b.resources.clone();
    let ids: Vec<_> = resources
        .sites()
        .iter()
        .filter(|s| matches!(s.kind, SiteKind::Primary(_)))
        .map(|s| (s.id, s.kind))
        .collect();
    for (id, kind) in ids {
        let result = resources.work_day(id, LabourTool::PickAndShovel, true, &labour, &b.models);
        match kind {
            SiteKind::Primary(DepositKind::Petroleum) => {
                assert_eq!(result, Err(ResourceError::NeedsDrillRig(id)))
            }
            _ => assert!(matches!(result, Err(ResourceError::Buried { site, .. }) if site == id)),
        }
    }
}
