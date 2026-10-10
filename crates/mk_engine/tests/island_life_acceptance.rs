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
///
/// Last changed by giving a spoken sentence a capital letter: most of
/// D36's phrasings are written as clauses, because that is how they read
/// mid-sentence, and whichever is drawn first has to open one. The
/// People page was showing `... "I want to be close to someone right
/// now." come here a minute`, which no test saw and no reader could
/// miss. Capitalising changes the text, so it changes the hash.
///
/// Before that, by D36 itself: speech was given its own phrasings instead of
/// being the speaker's internal monologue verbatim, so the words in a
/// week of conversation differ and the relationship memory built from
/// them differs with it. Audited the same way as every other change to
/// this constant, by deferring the assertion and running the rest:
/// audits closed 196 against a required 196, food and water shortfalls
/// both zero, Gem-D at 13.0 kg of body carbon against an expected 13.0
/// and Gem-K at 10.4 against 10.4, temperate-forest NPP at 707 g
/// C/m2/yr inside the pack's 400-900. Nothing physical moved; only the
/// hash did.
///
/// Before that, by the other half of D35: a pair who speak again on the
/// next tick now carry the same exchange on rather than opening a new
/// one, so `ConversationEvent` gained a `last_tick`, and the logs that
/// hold those exchanges are bounded by lines rather than by an entry
/// count that stopped meaning anything once an entry became a whole
/// conversation. A week of island life therefore holds different
/// conversation records and different relationship memory. (The half
/// before it was the sleep gate on `humans::step_dialogue`, which
/// stopped family pairs being matched on `alive` alone and so talking
/// through every night.)
///
/// It was changed only after checking that everything this test measures
/// about the week still holds, because a pinned digest is the easiest
/// thing in the repository to update for the wrong reason. Deferring this
/// assertion to the end of the test and running the rest gave: audits
/// closed 196 against a required 196, food and water shortfalls both zero,
/// Gem-D at 13.0 kg of body carbon against an expected 13.0 and Gem-K at
/// 10.4 against 10.4 — inside the 0.98-1.02 band that a fixed portion
/// (1.05x) and an action-gated harvest (0.90x) both failed — and the
/// temperate-forest NPP at 707 g C/m2/yr inside the ecology pack's
/// 400-900. Nothing physical moved; only the hash did.
///
/// D34's fix -- a parent and child now need the same adjacency as
/// everyone else -- did *not* move this digest, which is worth recording
/// because it is evidence rather than an argument: on the reference
/// island no parent/child pair is matched at a distance in a week, so
/// giving them a distance rule costs the reference island nothing and
/// only silences the conversations that were never audible.
///
/// Re-pinned for D32, D30 and D23 together: biomes by Köppen-Geiger on
/// monthly bins, residence times from Whittaker & Likens, and a house
/// with lamps, windows and curtains. The deferred audit gave audits 196 of
/// 196, food and water shortfalls zero, Gem-D 13.0 kg of body carbon
/// against 13.0 and Gem-K 10.4 against 10.4. Temperate-forest NPP fell
/// from 707 to 350 g C/m2/yr -- inside this test's 0.5x tolerance but
/// under the pack's 400-900 -- because Köppen now puts hemiboreal forest
/// on cold continental cells (annual mean ~2 C) that the old rule called
/// tundra; that is the too-continental year of D37 showing through, not
/// a production error.
///
/// Re-pinned again for D37, maritime air carried over the land: the same
/// audit gave 196 of 196, shortfalls zero, 13.0 and 10.4 kg against 13.0
/// and 10.4, and temperate-forest NPP back inside the pack at 595 g
/// C/m2/yr, the forests now standing in a maritime year.
///
/// And for D38: the sea's humidity carried over the land, rain from a
/// vapour budget, and a river taking its channel and esplanade reserve
/// rather than its whole 2 km cell (without which a watered island had no
/// buildable estate site). Audited: 196 of 196, no shortfalls, 13.0 and
/// 10.4 kg against 13.0 and 10.4, temperate-forest NPP 567 g C/m2/yr.
///
/// And for the founders' behaviour: desire with satiety (they had chosen
/// intimacy in most of every minute), seeking food where it is plentiful
/// costed as eating rather than walking (it had been a death spiral), and
/// a founder who walks out let back into the house. The founders now walk
/// up to 4 km from home by day and sleep at home most nights, so the test
/// checks that rather than where they stand at the week's last instant.
/// Audited: 196 of 196, no shortfalls, 13.0 and 10.4 kg against 13.0 and
/// 10.4, Gem-D asleep 56 of 168 hours (48 at home), Gem-K 54 (46),
/// temperate-forest NPP 567.
const WEEK_DIGEST: &str = "372a04e8301edb401e038e8e2636ec18caed2ad080cca1b6b01e62cc486ea032";

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

    // Both founders are alive and living at the estate.
    for id in ["Gem-D", "Gem-K"] {
        let h = w.humans.registry.get_human(id).unwrap();
        assert!(matches!(h.profile.status, HumanStatus::Alive), "{id} died");
        // Out on a walk is not leaving: within a day's foraging radius of
        // home -- about 10 km on foot (Kelly 1995, *The Foraging Spectrum*),
        // five of the island's 2 km cells. The week test watches where they
        // sleep.
        let estate = w.placed.location;
        let away = (h.position.row - estate.0 as i32)
            .abs()
            .max((h.position.col - estate.1 as i32).abs());
        assert!(away <= 5, "{id} is {away} cells from home after a day");
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
    // A minute at a time, which steps the island exactly as one week-long
    // advance does (`island_scheduler.rs`), so where the founders are can
    // be watched each hour -- (asleep samples, asleep in the estate,
    // farthest cells) -- and what they eat each minute.
    let estate = w.placed.location;
    let mut watched: std::collections::BTreeMap<&str, (u32, u32, i32)> =
        std::collections::BTreeMap::new();
    // (minutes spent eating, lowest glycogen seen): food reaches them only
    // when they choose to eat, so both are theirs.
    let mut fed: std::collections::BTreeMap<&str, (u32, f64)> = std::collections::BTreeMap::new();
    for minute in 1..=(7 * 24 * 60) {
        w.advance(60).unwrap();
        for id in ["Gem-D", "Gem-K"] {
            let h = w.humans.registry.get_human(id).unwrap();
            let meals = fed.entry(id).or_insert((0, 1.0));
            meals.0 += u32::from(mk_engine::humans::lifecycle::is_eating(
                h.economy_action.kind,
            ));
            meals.1 = meals.1.min(h.needs.glucose);
            if minute % 60 != 0 {
                continue;
            }
            let away = (h.position.row - estate.0 as i32)
                .abs()
                .max((h.position.col - estate.1 as i32).abs());
            let entry = watched.entry(id).or_insert((0, 0, 0));
            if h.circadian.asleep {
                entry.0 += 1;
                entry.1 += u32::from(w.in_estate(id));
            }
            entry.2 = entry.2.max(away);
        }
    }
    println!("digest {}", hex(w.state_digest()));
    let deferred_digest = hex(w.state_digest());
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
        // They live there: asleep, they are mostly at home, and awake they
        // never go beyond a day's foraging radius of it -- about 10 km for
        // people on foot (Kelly 1995, *The Foraging Spectrum*), five of the
        // island's 2 km cells. Being out on a walk at the moment the week
        // ends is not leaving.
        let (asleep, at_home, farthest) = watched[id];
        println!("{id}: asleep at home {at_home} of {asleep} hours, farthest {farthest} cells");
        assert!(
            asleep > 0 && at_home * 2 >= asleep,
            "{id} slept at home {at_home} of {asleep} sleeping hours"
        );
        assert!(farthest <= 5, "{id} went {farthest} cells from home");
        // They feed themselves: with food at hand, choosing when to eat
        // never lets glycogen fall to the level only fasting reaches, and
        // takes about as long a day as people spend eating -- within a
        // factor of two of the 64.5 minutes the American Time Use Survey
        // measures. A sated mind that kept eating (a learned taste outbidding
        // a full stomach) spent fourteen hours a day at it.
        let (eating, lowest) = fed[id];
        let per_day = f64::from(eating) / 7.0;
        println!("{id}: eating {per_day:.0} minutes a day, lowest glycogen {lowest:.2}");
        assert!(
            lowest > mk_engine::humans::needs::FASTING_GLUCOSE_FLOOR,
            "{id} went hungry to {lowest:.2} with food at hand"
        );
        let atus = mk_engine::humans::needs::EATING_MINUTES_PER_DAY;
        assert!(
            (0.5 * atus..=2.0 * atus).contains(&per_day),
            "{id} ate {per_day:.0} minutes a day"
        );
        // Alive is the needs model's answer; this is the ledger's. A meal is
        // harvested only when its eater chooses to eat, so a week in which
        // nobody ever did would leave their body carbon drained. It must not
        // have.
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
        // and 1.10x, and gating the harvest on chosen actions while the
        // needs model still fed them regardless at 0.90x, and both would
        // pass anything looser.
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

    // Founders' water use against the IOM's adequate intake. They drink when
    // they choose to, and each drink puts back what they lost at 1 mL per
    // kcal they burned, so this is their own expenditure and their own
    // choices meeting the survey: drinks are the pack's share of total water
    // not supplied by food, held to the half-to-one-and-a-half band the
    // other pack comparisons here use.
    let intake = lib
        .item(ReferenceDomain::Humans, "total_water_adequate_intake")
        .unwrap()
        .table()
        .unwrap();
    let food_share = lib
        .item(ReferenceDomain::Humans, "food_water_share")
        .unwrap()
        .central()
        .unwrap();
    for id in ["Gem-D", "Gem-K"] {
        let h = w.humans.registry.get_human(id).unwrap();
        let sex = match h.biological_sex() {
            mk_core::human::BiologicalSex::Male => "male",
            _ => "female",
        };
        let drinks = intake.get(sex, "value").unwrap() * (1.0 - food_share);
        let drunk = w.water_drunk_kg.get(id).copied().unwrap_or(0.0) / 7.0;
        println!(
            "{id}: drank {drunk:.2} L a day against the {sex} adequate {drinks:.2} L from drinks"
        );
        assert!(
            (0.5 * drinks..=1.5 * drinks).contains(&drunk),
            "{id} drank {drunk:.2} L a day"
        );
    }
    assert_eq!(w.shortfalls.water, 0, "the founders found water every time");
    assert_eq!(deferred_digest, WEEK_DIGEST);
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

/// Four weeks of the founders' own choices: a willing couple is intimate
/// about as often as couples measured by the General Social Survey -- 55
/// times a year for married and cohabiting adults in 2014, 80 for adults
/// in their twenties (Twenge, Sherman & Wells 2017) -- not most of every
/// waking minute, which is what a drive with no satiety gave. Two people
/// over four weeks make a small sample, so the band is wide: 30-130 a
/// year, which a satiety-free drive (thousands) and no desire at all (zero)
/// both fail. Slow tier.
#[test]
#[ignore = "slow: four simulated weeks"]
fn slow_humans_choose_intimacy_about_as_often_as_couples_do() {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 200;
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let mut w = IslandLife::bootstrap(scenario, canon).expect("the island bootstraps");
    let count = |w: &IslandLife, id: &str| {
        w.humans
            .registry
            .get_human(id)
            .unwrap()
            .reproduction
            .sexual_activity_count
    };
    let before = count(&w, "Gem-D");
    let days = 28.0;
    w.advance((days * DAY as f64) as u64).unwrap();
    let acts = (count(&w, "Gem-D") - before) as f64;
    let per_year = acts * 365.25 / days;
    println!("{acts} acts in {days} days: {per_year:.0} a year");
    assert!(
        (30.0..=130.0).contains(&per_year),
        "{per_year:.0} acts a year against a measured 55-80"
    );
}
