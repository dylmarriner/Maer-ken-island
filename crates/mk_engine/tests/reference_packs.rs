//! Phase 0c Task 3: the real-world reference packs load, cite a source and
//! licence for every value, cover what the realism standard requires, and
//! agree with the formulas they quote.

use mk_engine::validation::{
    default_reference_dir, Confidence, ReferenceDomain, ReferenceLibrary, ReferencePack,
};

fn library() -> ReferenceLibrary {
    ReferenceLibrary::load(&default_reference_dir()).expect("reference packs load")
}

/// Items each domain must provide (`docs/superpowers/plans/2026-10-02-island-phase0c-canon-and-realism.md`, Task 3).
const REQUIRED: [(ReferenceDomain, &[&str]); 6] = [
    (
        ReferenceDomain::Climate,
        &[
            "environmental_lapse_rate",
            "diurnal_temperature_range_by_surface",
            "orographic_precipitation_southern_alps",
            "windward_to_leeward_precipitation_ratio",
            "sst_seasonal_range_40_45",
            "synoptic_period",
            "cyclone_lifetime",
        ],
    ),
    (
        ReferenceDomain::Hydrology,
        &[
            "budyko_curve",
            "mean_discharge_area_exponent",
            "flood_peak_area_exponent",
            "q100_to_mean_annual_flood",
        ],
    ),
    (
        ReferenceDomain::Ecology,
        &[
            "npp_by_biome",
            "chave_agb",
            "chave_height_diameter",
            "stem_density_by_forest",
            "tree_mortality_rate",
            "tree_diameter_increment",
            "soil_carbon_by_biome",
            "kleiber_law",
        ],
    ),
    (
        ReferenceDomain::Geology,
        &[
            "gutenberg_richter_b",
            "omori_utsu",
            "active_margin_uplift_rate",
            "active_range_erosion_rate",
            "vei_global_frequency",
            "grade_tonnage_medians",
            "kimberlite_diamond_grade",
        ],
    ),
    (
        ReferenceDomain::Humans,
        &[
            "bmr_mifflin_st_jeor",
            "met_by_activity",
            "total_water_adequate_intake",
            "survival_without_water",
            "survival_without_food",
            "sleep_need_by_age",
            "intrinsic_circadian_period",
            "circadian_entrainment_range",
            "comfortable_walking_speed",
            "sustainable_load_fraction",
            "gestation_length",
            "natural_marital_fertility",
            "annual_mortality_by_age",
            "median_height_for_age",
        ],
    ),
    (
        ReferenceDomain::Labour,
        &[
            "felling_time_30cm_tree",
            "hand_excavation_rate",
            "artisanal_hard_rock_mining",
            "gold_panning_throughput",
            "placer_gold_recovery",
            "charcoal_yield",
            "bloomery_iron",
            "timber_frame_construction_labour",
            "light_4wd_fuel_consumption",
            "vehicle_speed_by_surface",
        ],
    ),
];

#[test]
fn every_pack_loads_with_sources_and_licences() {
    let lib = library();
    for domain in ReferenceDomain::ALL {
        assert!(lib.in_domain(domain).count() > 0, "no pack for {domain:?}");
    }
    for pack in lib.packs() {
        assert!(!pack.licence.is_empty(), "{}", pack.id);
        for item in &pack.items {
            let source = pack.source(&item.source).expect("validated source");
            assert!(!source.citation.is_empty() && !source.licence.is_empty());
        }
    }
}

#[test]
fn every_domain_covers_its_required_items() {
    let lib = library();
    for (domain, keys) in REQUIRED {
        for key in keys {
            assert!(lib.item(domain, key).is_some(), "{domain:?} lacks {key}");
        }
    }
}

#[test]
fn loading_is_deterministic() {
    let a: Vec<ReferencePack> = library().packs().cloned().collect();
    let b: Vec<ReferencePack> = library().packs().cloned().collect();
    assert_eq!(a, b);
}

#[test]
fn malformed_packs_are_rejected() {
    let lib = library();
    let good = lib.pack("catchments").unwrap().clone();

    let mut unknown_source = good.clone();
    unknown_source.items[0].source = "nobody".into();
    assert!(unknown_source.validate().is_err());

    let mut no_licence = good.clone();
    no_licence.sources[0].licence = " ".into();
    assert!(no_licence.validate().is_err());

    let mut bad_date = good.clone();
    bad_date.retrieved = "07/10/2026".into();
    assert!(bad_date.validate().is_err());

    let mut duplicate = good.clone();
    duplicate.items.push(good.items[0].clone());
    assert!(duplicate.validate().is_err());

    let mut inverted = good;
    inverted.items[1].value = mk_engine::validation::ReferenceValue::Range {
        min: 2.0,
        max: 1.0,
        typical: None,
    };
    assert!(inverted.validate().is_err());
}

#[test]
fn budyko_table_matches_its_formula() {
    let item = library()
        .item(ReferenceDomain::Hydrology, "budyko_curve")
        .cloned()
        .unwrap();
    let table = item.table().unwrap();
    for row in &table.rows {
        let phi = row.values[0];
        let e = (phi * (1.0 / phi).tanh() * (1.0 - (-phi).exp())).sqrt();
        assert!(
            (row.values[1] - e).abs() < 5e-4,
            "phi {phi}: {} vs {e}",
            row.values[1]
        );
        assert!((row.values[1] + row.values[2] - 1.0).abs() < 1e-9);
    }
}

#[test]
fn chave_example_matches_its_formula() {
    let item = library()
        .item(ReferenceDomain::Ecology, "chave_agb")
        .cloned()
        .unwrap();
    let t = item.table().unwrap();
    let v = |k| t.get(k, "value").unwrap();
    let agb = v("a")
        * (v("example_rho_g_cm3") * v("example_d_cm").powi(2) * v("example_h_m")).powf(v("b"));
    assert!((agb - v("example_agb_kg")).abs() / agb < 0.002, "{agb}");
}

#[test]
fn mifflin_examples_match_the_formula_and_agree_with_kleiber() {
    let lib = library();
    let t = lib
        .item(ReferenceDomain::Humans, "bmr_mifflin_st_jeor")
        .unwrap()
        .table()
        .unwrap()
        .clone();
    let v = |k| t.get(k, "value").unwrap();
    let male = v("per_kg") * 70.0 + v("per_cm") * 175.0 + v("per_year") * 30.0 + v("male_constant");
    let female =
        v("per_kg") * 60.0 + v("per_cm") * 165.0 + v("per_year") * 30.0 + v("female_constant");
    assert_eq!(male, v("example_male_70kg_175cm_30y"));
    assert_eq!(female, v("example_female_60kg_165cm_30y"));

    // Kleiber's interspecific law should land within ~5% of the human
    // equation for a 70 kg adult (both describe basal metabolism).
    let k = lib
        .item(ReferenceDomain::Ecology, "kleiber_law")
        .unwrap()
        .table()
        .unwrap();
    let kleiber = k.get("coefficient_kcal_per_day", "value").unwrap()
        * 70f64.powf(k.get("exponent", "value").unwrap());
    assert!(
        (kleiber - male).abs() / male < 0.05,
        "kleiber {kleiber} vs mifflin {male}"
    );
}

#[test]
fn hack_coefficient_is_the_converted_imperial_fit() {
    let item = library()
        .item(ReferenceDomain::Hydrology, "hack_law")
        .cloned()
        .unwrap();
    let t = item.table().unwrap();
    let mile_km: f64 = 1.609_344;
    let converted = 1.4 * mile_km / (mile_km * mile_km).powf(t.get("exponent_h", "value").unwrap());
    assert!((converted - t.get("coefficient_c_km", "value").unwrap()).abs() < 0.005);
}

#[test]
fn human_life_history_tables_are_ordered_like_reality() {
    let lib = library();
    // Children grow every year in the table.
    let h = lib
        .item(ReferenceDomain::Humans, "median_height_for_age")
        .unwrap()
        .table()
        .unwrap();
    for col in ["boys", "girls"] {
        let heights: Vec<f64> = h.column(col).unwrap().into_iter().map(|(_, v)| v).collect();
        assert!(
            heights.windows(2).all(|w| w[0] < w[1]),
            "{col}: {heights:?}"
        );
    }
    // Natural fertility declines after the early twenties.
    let f = lib
        .item(ReferenceDomain::Humans, "natural_marital_fertility")
        .unwrap()
        .table()
        .unwrap();
    let rates: Vec<f64> = f
        .column("births_per_woman_year")
        .unwrap()
        .into_iter()
        .map(|(_, v)| v)
        .collect();
    assert!(rates.windows(2).all(|w| w[0] > w[1]));
    // Adult mortality rises with age, doubling every 7-9 years on average
    // from 40 to 90.
    let q = lib
        .item(ReferenceDomain::Humans, "annual_mortality_by_age")
        .unwrap()
        .table()
        .unwrap();
    let q40 = q.get("40", "qx").unwrap();
    let q90 = q.get("90", "qx").unwrap();
    let doubling = 50.0 * std::f64::consts::LN_2 / (q90 / q40).ln();
    let mrdt = lib
        .item(ReferenceDomain::Humans, "mortality_rate_doubling_time")
        .unwrap()
        .range()
        .unwrap();
    assert!(
        doubling >= mrdt.0 && doubling <= mrdt.1,
        "doubling {doubling}"
    );
}

#[test]
fn the_body_clock_cannot_entrain_to_a_36_hour_day() {
    // The fact Phase 0c Task 4 is built on: Marr'Kena's day lies far outside
    // the human entrainment range.
    let lib = library();
    let (lo, hi) = lib
        .item(ReferenceDomain::Humans, "circadian_entrainment_range")
        .unwrap()
        .range()
        .unwrap();
    let tau = lib
        .item(ReferenceDomain::Humans, "intrinsic_circadian_period")
        .unwrap()
        .central()
        .unwrap();
    assert!(lo <= tau && tau <= hi);
    assert!(!(lo..=hi).contains(&36.0));
}

#[test]
fn low_confidence_values_are_marked_with_a_note() {
    // A low-confidence value must say why, so nobody gates a tight test on it.
    for pack in library().packs() {
        for item in &pack.items {
            if item.confidence == Confidence::Low {
                assert!(
                    item.note.is_some(),
                    "{}/{} is low confidence without a note",
                    pack.id,
                    item.key
                );
            }
        }
    }
}
