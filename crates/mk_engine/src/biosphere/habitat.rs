//! Habitat suitability: where on the live planet a catalogue species lives.
//!
//! Every [`BiodiversityEntry`] names a `temperature_band` and a
//! `moisture_band`. This module matches those bands against the world's real
//! per-cell state — surface temperature, the land/ocean split (elevation
//! below sea level is ocean, as in `mk_view::PlanetView`), soil moisture and
//! volcanic heat — and picks a cell for the species deterministically from
//! its id, so the same species settles in the same place in every process.
//!
//! When no cell satisfies both bands the constraint relaxes in a fixed
//! order — moisture only (keeping marine species in the sea and terrestrial
//! ones on land), then temperature only, then anywhere — so a species is
//! always placed somewhere plausible rather than dropped.

use super::biodiversity_catalogue::BiodiversityEntry;
use crate::world_integration::WorldState;

/// 0 °C in kelvin.
const FREEZING_K: f64 = 273.15;

/// Eruption probability above which a cell counts as a geothermal habitat.
const GEOTHERMAL_ERUPTION_PROB: f64 = 0.05;

/// Whether a cell at `temperature_k` (with or without geothermal heat)
/// suits `band`. Unknown bands are unconstrained.
pub fn temperature_band_matches(band: &str, temperature_k: f64, geothermal: bool) -> bool {
    let celsius = temperature_k - FREEZING_K;
    match band {
        "cold" => celsius < 0.0,
        "cool" => (0.0..10.0).contains(&celsius),
        "temperate" => (10.0..20.0).contains(&celsius),
        "warm" => (20.0..28.0).contains(&celsius),
        "hot" => celsius >= 28.0,
        "thermal-gradient" => geothermal,
        _ => true,
    }
}

/// Whether a cell suits `band`, given whether it is ocean and its soil
/// moisture fraction (`0..=1`, land only). Unknown bands are unconstrained.
pub fn moisture_band_matches(band: &str, is_ocean: bool, soil_moisture: f64) -> bool {
    match band {
        "submerged" => is_ocean,
        "saturated" => !is_ocean && soil_moisture >= 0.7,
        "wet-seasonal" => !is_ocean && (0.45..0.85).contains(&soil_moisture),
        "mesic" => !is_ocean && (0.3..0.65).contains(&soil_moisture),
        "dry-seasonal" => !is_ocean && (0.15..0.4).contains(&soil_moisture),
        "xeric" => !is_ocean && soil_moisture < 0.2,
        _ => true,
    }
}

/// Every cell matching both bands, relaxing as described in the module docs
/// when none do. Never empty for a non-empty grid.
pub fn habitat_cells(
    world: &WorldState,
    temperature_band: &str,
    moisture_band: &str,
) -> Vec<(usize, usize)> {
    let (nlat, nlon) = (world.grid_spec.nlat(), world.grid_spec.nlon());
    let cells: Vec<(usize, usize, bool, bool)> = (0..nlat)
        .flat_map(|row| (0..nlon).map(move |col| (row, col)))
        .map(|(row, col)| {
            let temperature_k = *world.climate_state.surface_temperature.get(row, col);
            let geothermal = world.volcanism_state.volcanism.get(row, col).eruption_prob
                > GEOTHERMAL_ERUPTION_PROB;
            let is_ocean = *world.elevation_grid.get(row, col) < 0.0;
            let soil_moisture = world
                .hydrology_state
                .soil_water
                .get(row, col)
                .moisture_fraction;
            (
                row,
                col,
                temperature_band_matches(temperature_band, temperature_k, geothermal),
                moisture_band_matches(moisture_band, is_ocean, soil_moisture),
            )
        })
        .collect();

    let select = |keep: &dyn Fn(bool, bool) -> bool| -> Vec<(usize, usize)> {
        cells
            .iter()
            .filter(|(_, _, temperature, moisture)| keep(*temperature, *moisture))
            .map(|(row, col, _, _)| (*row, *col))
            .collect()
    };
    for keep in [
        &(|t: bool, m: bool| t && m) as &dyn Fn(bool, bool) -> bool,
        &|_, m| m,
        &|t, _| t,
    ] {
        let found = select(keep);
        if !found.is_empty() {
            return found;
        }
    }
    select(&|_, _| true)
}

/// FNV-1a over a species id, so placement is stable across processes.
fn stable_hash(id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The cell `entry`'s species occupies: one of its [`habitat_cells`],
/// chosen deterministically from the entry id.
pub fn habitat_cell(world: &WorldState, entry: &BiodiversityEntry) -> (usize, usize) {
    let cells = habitat_cells(world, &entry.temperature_band, &entry.moisture_band);
    choose_habitat_cell(&cells, &entry.id)
}

/// The cell a species with `id` settles in among its precomputed
/// `habitat_cells` — the same choice [`habitat_cell`] makes, for callers
/// that cache cells per band pair. `cells` must be non-empty.
pub fn choose_habitat_cell(cells: &[(usize, usize)], id: &str) -> (usize, usize) {
    cells[(stable_hash(id) % cells.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::canon::CanonLocked;
    use std::sync::Arc;

    fn world() -> WorldState {
        WorldState::new(Arc::new(CanonLocked::default()), [4u8; 32])
    }

    #[test]
    fn bands_match_the_physical_state_they_name() {
        assert!(temperature_band_matches("cold", 260.0, false));
        assert!(!temperature_band_matches("cold", 290.0, false));
        assert!(temperature_band_matches("hot", 305.0, false));
        assert!(temperature_band_matches("thermal-gradient", 200.0, true));
        assert!(!temperature_band_matches("thermal-gradient", 300.0, false));
        assert!(temperature_band_matches("unlisted", 0.0, false));

        assert!(moisture_band_matches("submerged", true, 0.0));
        assert!(!moisture_band_matches("submerged", false, 1.0));
        assert!(moisture_band_matches("xeric", false, 0.05));
        assert!(!moisture_band_matches("xeric", true, 0.05));
    }

    #[test]
    fn marine_species_are_placed_in_the_ocean() {
        let w = world();
        for (row, col) in habitat_cells(&w, "any", "submerged") {
            assert!(*w.elevation_grid.get(row, col) < 0.0);
        }
    }

    #[test]
    fn placement_is_deterministic_and_never_empty() {
        let w = world();
        assert!(!habitat_cells(&w, "hot", "saturated").is_empty());
        let e = BiodiversityEntry {
            id: "MK-BIO-AN-00042".to_string(),
            temperature_band: "cool".to_string(),
            moisture_band: "mesic".to_string(),
            ..sample_entry()
        };
        assert_eq!(habitat_cell(&w, &e), habitat_cell(&w, &e));
    }

    fn sample_entry() -> BiodiversityEntry {
        serde_json::from_str(
            r#"{"id":"x","major_group":"animal","category":"c","common_name":"n",
            "canonical_taxon":"t","lineage_stem":"l","clade":"c","trophic_role":"r",
            "diet_or_resource_use":"d","edible_or_resource_detail":"e",
            "body_or_growth_form":"b","size_class":"small","length_or_height_range":"1",
            "mass_or_biomass_range":"1","habitat":"h","biome":"b","moisture_band":"mesic",
            "temperature_band":"cool","salinity_band":"s","movement_or_growth_strategy":"m",
            "reproduction_strategy":"r","reproduction_rate":"r","maturation_time":"m",
            "lifespan_or_turnover":"l","population_pattern":"p","defense_or_resilience":"d",
            "attack_or_competition":"a","sensory_or_response_mode":"s","symbiosis":"s",
            "ecological_function":"e","nutrient_cycle_role":"n","seasonality":"s",
            "climate_sensitivity":"c","disease_sensitivity":"d","extinction_sensitivity":"e",
            "intelligence_index_ceiling":0.1,"mk_phase_scope":"m","ledger_links":"l",
            "ui_render_hint":"u","implementation_tags":"i","description":"d",
            "forbidden_state_flags":"f"}"#,
        )
        .expect("sample catalogue entry parses")
    }
}
