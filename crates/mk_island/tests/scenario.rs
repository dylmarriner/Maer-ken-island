//! Phase 3 Task 4: the island scenario's defaults, round trip and
//! validation.

use std::path::PathBuf;

use mk_island::{
    EstateEnergyConfig, EstatePatchConfig, IslandCadenceProfile, IslandProfile, IslandScenario,
    IslandScenarioError, SCENARIO_VERSION,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/island")
        .join(name)
}

#[test]
fn defaults_are_the_planned_ones() {
    let c = IslandCadenceProfile::default();
    assert_eq!(
        (
            c.human_seconds,
            c.weather_ocean_seconds,
            c.hydrology_ecology_resource_seconds,
            c.geophysics_seconds,
            c.human_store_seconds
        ),
        (60, 3_600, 21_600, 86_400, 3_600)
    );
    let p = EstatePatchConfig::default();
    assert_eq!(
        (
            p.extent_m,
            p.cell_size_m,
            p.individual_tree_radius_m,
            p.tree_cap
        ),
        (4_000.0, 5.0, 1_000.0, 200_000)
    );
    let e = EstateEnergyConfig::default();
    assert_eq!(
        (
            e.generator_rated_kw,
            e.diesel_litres,
            e.petrol_litres,
            e.solar_rated_kw,
            e.battery_kwh
        ),
        (10.0, 2_000.0, 400.0, 5.0, 10.0)
    );
    assert!(c.validate().is_ok() && p.validate().is_ok() && e.validate().is_ok());
}

#[test]
fn the_default_fixture_loads_and_matches_the_default_profile() {
    let scenario = IslandScenario::load(&fixture("default_scenario.json")).unwrap();
    let profile = IslandProfile::load(&fixture("default_profile.json")).unwrap();
    assert_eq!(scenario.profile, profile);
    assert_eq!(Some(scenario.seed), profile.seed_bytes());
    assert_eq!(scenario.version, SCENARIO_VERSION);
    assert!(scenario.founders_enabled && scenario.estate_enabled);
    assert_eq!(
        scenario.canon_path,
        PathBuf::from("fixtures/island/canon.json")
    );
    assert_eq!(scenario, IslandScenario::new(profile, scenario.seed));
}

#[test]
fn json_round_trips() {
    let scenario = IslandScenario::load(&fixture("default_scenario.json")).unwrap();
    let text = serde_json::to_string(&scenario).unwrap();
    let back: IslandScenario = serde_json::from_str(&text).unwrap();
    assert_eq!(back, scenario);
    // The seed is written as 64 hex characters.
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["seed"].as_str().unwrap().len(), 64);
}

#[test]
fn bad_scenarios_are_rejected() {
    let good = IslandScenario::load(&fixture("default_scenario.json")).unwrap();
    let mut wrong_version = good.clone();
    wrong_version.version = 2;
    assert!(matches!(
        wrong_version.validate(),
        Err(IslandScenarioError::UnsupportedVersion(2))
    ));
    for mutate in [
        (|c: &mut IslandCadenceProfile| c.human_seconds = 0) as fn(&mut IslandCadenceProfile),
        |c| c.weather_ocean_seconds = 0,
        |c| c.geophysics_seconds = 0,
        // Not a multiple of the human step.
        |c| c.weather_ocean_seconds = 3_601,
        |c| c.human_seconds = 7,
    ] {
        let mut s = good.clone();
        mutate(&mut s.cadences);
        assert!(matches!(
            s.validate(),
            Err(IslandScenarioError::InvalidCadence(_))
        ));
    }
    let mut patch = good.clone();
    patch.estate_patch.cell_size_m = 7.0; // does not divide 4,000 m
    assert!(matches!(
        patch.validate(),
        Err(IslandScenarioError::InvalidEstatePatch(_))
    ));
    let mut trees = good.clone();
    trees.estate_patch.tree_cap = 0;
    assert!(trees.validate().is_err());
    let mut energy = good.clone();
    energy.estate_energy.battery_kwh = f64::NAN;
    assert!(matches!(
        energy.validate(),
        Err(IslandScenarioError::InvalidEnergy(_))
    ));
    // A seed that is not 64 hex characters fails to load.
    let mut json: serde_json::Value = serde_json::to_value(&good).unwrap();
    json["seed"] = "zz".into();
    assert!(serde_json::from_value::<IslandScenario>(json).is_err());
    // A missing file is an I/O error.
    assert!(matches!(
        IslandScenario::load(&fixture("missing.json")),
        Err(IslandScenarioError::Io(..))
    ));
}
