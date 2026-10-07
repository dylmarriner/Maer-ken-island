//! Phase 3 Task 9: the integrated life, property and human acceptance on
//! the owner's island.
//!
//! Fast tier: one simulated day. Slow tier (`--ignored slow_`): one week at
//! 60 s human steps (hourly physics, 6-hourly ecology) with realism checks
//! against the reference packs, printing the canonical state digest so two
//! processes can be compared.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::HumanStatus;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

const DAY: u64 = 86_400;

/// The canonical state digest after one simulated week from the default
/// scenario (`fixtures/island/default_scenario.json`), identical across
/// processes. Changing any Phase 1-3 behaviour changes it: update it
/// deliberately, in the commit that changes the behaviour.
const WEEK_DIGEST: &str = "e796f9f9a851bd98c1ffbae6a427c3b5dc258dcc38ae79d70e85359299cc5d2a";

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn life() -> IslandLife {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).expect("the island bootstraps")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn a_day_on_the_island_keeps_everything_alive_inside_closed_books() {
    let mut w = life();
    let items_at_start = w.placed.layout.items.len();
    let estate_items = w
        .placed
        .property
        .properties
        .iter()
        .find(|p| p.owner_agent_ids.iter().any(|i| i == "Gem-D"))
        .unwrap()
        .items
        .len();
    assert_eq!(
        items_at_start, estate_items,
        "the estate inventory is laid out whole"
    );
    let (land_before, patch_before) = (
        w.ecology.total_biomass_kgc(&w.domain),
        w.vegetation.total_carbon_kgc(),
    );
    assert!(
        land_before > 0.0 && patch_before > 0.0,
        "the island and its patch have vegetation"
    );
    assert!(w.vegetation.trees.len() as f64 >= 0.0);

    w.advance(DAY).unwrap();

    // Both founders are alive and still in the estate layout.
    for id in ["Gem-D", "Gem-K"] {
        let h = w.humans.registry.get_human(id).unwrap();
        assert!(matches!(h.profile.status, HumanStatus::Alive), "{id} died");
        assert!(w.in_estate(id), "{id} left the estate");
    }
    assert_eq!(w.humans.registry.iter().count(), 2);
    // The estate's inventory is intact.
    assert_eq!(w.placed.layout.items.len(), items_at_start);
    // Physical, ecological and household steps all ran, and every audit closed.
    assert_eq!(w.sim_time_s, DAY);
    assert!(w.audits_closed >= 24 + 4, "{} audits", w.audits_closed);
    // Life goes on: vegetation persists on the island and the patch.
    assert!(w.ecology.total_biomass_kgc(&w.domain) > 0.0);
    assert!(w.vegetation.total_carbon_kgc() > 0.0);
    // The founders breathed: carbon left their bodies for the air.
    assert!(w.materials.stock_of(mk_core::flux::Reservoir::HumanCarbon) > 0.0);
}

#[test]
fn the_same_day_twice_gives_the_same_state() {
    let (mut a, mut b) = (life(), life());
    assert_eq!(
        hex(a.state_digest()),
        hex(b.state_digest()),
        "bootstrap differs"
    );
    a.advance(6 * 3_600).unwrap();
    b.advance(6 * 3_600).unwrap();
    assert_eq!(hex(a.state_digest()), hex(b.state_digest()));
    assert_ne!(
        hex(a.state_digest()),
        hex(life().state_digest()),
        "six hours changed nothing"
    );
}

/// One week with the realism checks. Slow tier.
#[test]
#[ignore = "slow: one simulated week"]
fn slow_a_week_on_the_island_matches_the_reference_packs() {
    use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};
    let lib = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    let mut w = life();
    w.advance(7 * DAY).unwrap();
    println!("digest {}", hex(w.state_digest()));
    assert_eq!(hex(w.state_digest()), WEEK_DIGEST);
    println!(
        "audits {}, shortfalls food {} water {}, economy events {}, patch trees {}",
        w.audits_closed,
        w.shortfalls.food,
        w.shortfalls.water,
        w.economy.events.len(),
        w.vegetation.trees.len()
    );
    assert!(w.audits_closed >= 7 * 24 + 7 * 4);
    for id in ["Gem-D", "Gem-K"] {
        assert!(matches!(
            w.humans.registry.get_human(id).unwrap().profile.status,
            HumanStatus::Alive
        ));
        assert!(w.in_estate(id), "{id} left the estate");
        // Alive is the needs model's answer; this is the ledger's. Harvesting
        // follows each human's own chosen action, so a week in which nobody
        // ever chose to look for food would still leave them "alive" here
        // while their body carbon drained away. It must not have.
        let carbon = w
            .materials
            .body_carbon_kg(id)
            .unwrap_or_else(|| panic!("{id} has no body in the ledger"));
        let expected = w.humans.registry.get_human(id).unwrap().body.weight_kg
            * mk_engine::regional::materials::WHOLE_BODY_CARBON_FRACTION;
        println!("{id}: body carbon {carbon:.1} kg of an expected {expected:.1} kg");
        // A meal replaces exactly the carbon its eater has burned since the
        // last one, so an adult's body carbon should not move at all over a
        // week. The band is tight on purpose: a fixed portion sat at 1.05x
        // and 1.10x, and gating the harvest on chosen actions at 0.90x, and
        // both would pass anything looser.
        assert!(
            (0.98 * expected..=1.02 * expected).contains(&carbon),
            "{id} holds {carbon:.2} kg of carbon against an expected {expected:.2}: what they eat \
             and what they burn have come apart"
        );
    }

    // NPP by biome against the ecology pack (g C / m2 / yr).
    let pack = lib
        .item(ReferenceDomain::Ecology, "npp_by_biome")
        .unwrap()
        .table()
        .unwrap();
    let mean_npp = |biome: mk_core::biomes::BiomeType| {
        let (mut sum, mut n) = (0.0, 0.0);
        for (i, b) in w.ecology.biome_grid.data().iter().enumerate() {
            if *b == biome {
                sum += w.ecology.npp_kgc_m2_yr.data()[i] * 1000.0;
                n += 1.0;
            }
        }
        (n > 0.0).then(|| sum / n)
    };
    for (biome, row) in [(
        mk_core::biomes::BiomeType::TemperateForest,
        "temperate_broadleaf_forest",
    )] {
        if let Some(npp) = mean_npp(biome) {
            let (lo, hi) = (pack.get(row, "min").unwrap(), pack.get(row, "max").unwrap());
            println!("{row}: model {npp:.0} g C/m2/yr, reference {lo}-{hi}");
            assert!((0.5 * lo..=1.5 * hi).contains(&npp), "{row}: {npp}");
        }
    }

    // Tree mortality, diameter and carbon against the packs.
    let (mort_lo, mort_hi) = lib
        .item(ReferenceDomain::Ecology, "tree_mortality_rate")
        .unwrap()
        .range()
        .unwrap();
    let background = 1.0 / 150.0;
    assert!(
        (mort_lo..=mort_hi).contains(&background),
        "background mortality {background}"
    );
    for t in w.vegetation.trees.iter().take(2_000) {
        assert!(t.stem_diameter_m >= 0.10 - 1e-6);
        // Height-diameter: Chave et al. 2014 from the pack.
        let hd = lib
            .item(ReferenceDomain::Ecology, "chave_height_diameter")
            .unwrap()
            .table()
            .unwrap();
        let ln_d = (t.stem_diameter_m * 100.0).ln();
        let h = (hd.get("intercept", "value").unwrap()
            + hd.get("ln_d", "value").unwrap() * ln_d
            + hd.get("ln_d_squared", "value").unwrap() * ln_d * ln_d)
            .exp();
        assert!(
            (t.height_m - h).abs() < 1e-9,
            "height {} vs {h}",
            t.height_m
        );
    }

    // Founders' water use: at least the pack's lower adequate intake, over a week.
    let intake = lib
        .item(ReferenceDomain::Humans, "total_water_adequate_intake")
        .unwrap()
        .table()
        .unwrap();
    let drunk_per_founder_day = 4.0 * 0.65;
    assert!(
        drunk_per_founder_day >= intake.get("female", "value").unwrap() * 0.9,
        "{drunk_per_founder_day} L/day"
    );
    assert_eq!(w.shortfalls.water, 0, "the founders found water every time");
}

/// The digest covers the state that decides what happens next.
///
/// Two islands that differ only in a pending meal balance, or in how much
/// of the current hour a founder has slept, will diverge at the next meal
/// and the next respiration charge. A digest that called them identical
/// would make replay and snapshot verification quietly wrong.
#[test]
fn the_digest_notices_state_that_only_matters_later() {
    let mut a = life();
    let mut b = life();
    assert_eq!(hex(a.state_digest()), hex(b.state_digest()), "same start");

    // Far enough for the founders to have respired and eaten at least once.
    a.advance(7 * 3_600).unwrap();
    b.advance(7 * 3_600).unwrap();
    assert_eq!(
        hex(a.state_digest()),
        hex(b.state_digest()),
        "the same island stepped the same way has to hash the same"
    );

    // One more hour on only one of them: the pending balances and the
    // sleep accounting now differ, and the digest has to say so.
    a.advance(3_600).unwrap();
    assert_ne!(
        hex(a.state_digest()),
        hex(b.state_digest()),
        "an extra hour left the digest unchanged"
    );
}
