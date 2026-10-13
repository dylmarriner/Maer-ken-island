//! The island's soil (realism programme R0, D43): litterfall, faeces and
//! carcasses feed soil carbon that decomposes by RothC, instead of leaving
//! the books.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_engine::regional::life::IslandLife;
use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

#[test]
fn the_island_has_soil_that_takes_what_falls_and_breathes_it_out() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let mut life = IslandLife::bootstrap(scenario, canon).unwrap();
    let soil = life.ecology.soil.clone().expect("the island has soil");
    let biomes = life.ecology.biome_grid.data().to_vec();

    // Land has soil and water has none.
    let water = |b: BiomeType| {
        matches!(
            b,
            BiomeType::River
                | BiomeType::DeepOcean
                | BiomeType::ShallowOcean
                | BiomeType::CoastalWaters
                | BiomeType::ReefSea
        )
    };
    for (i, b) in biomes.iter().enumerate() {
        if water(*b) {
            assert_eq!(soil.total(i), 0.0, "soil under {b:?}");
        }
    }

    // Against Jobbágy & Jackson (2000), soil organic carbon in the top
    // metre. RothC describes the topsoil, so the island's soil is expected
    // to be lower, but of the same order.
    let library = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    let observed = |row: &str| {
        library
            .item(ReferenceDomain::Ecology, "soil_carbon_by_biome")
            .and_then(|i| i.table())
            .and_then(|t| t.get(row, "top_1m"))
            .unwrap()
    };
    for (biome, row) in [
        (BiomeType::TemperateForest, "temperate_evergreen_forest"),
        (BiomeType::Grassland, "temperate_grassland"),
    ] {
        let cells: Vec<f64> = biomes
            .iter()
            .enumerate()
            .filter(|(_, b)| **b == biome)
            .map(|(i, _)| soil.total(i))
            .collect();
        if cells.is_empty() {
            continue;
        }
        let mean = cells.iter().sum::<f64>() / cells.len() as f64;
        let reference = observed(row);
        println!(
            "{biome:?}: {} cells, mean soil {mean:.2} kgC/m² (top metre observed {reference})",
            cells.len()
        );
        assert!(
            (0.1 * reference..=1.5 * reference).contains(&mean),
            "{biome:?} soil {mean} kgC/m² against {reference} observed"
        );
    }

    // A day: the soil breathes, and starting at its steady state it barely
    // moves.
    let start: f64 = (0..biomes.len()).map(|i| soil.total(i)).sum();
    for _ in 0..(86_400 / 60) {
        life.advance(60).unwrap();
    }
    let after = life.ecology.soil.as_ref().unwrap();
    let end: f64 = (0..biomes.len()).map(|i| after.total(i)).sum();
    assert!(after.respired_kgc_m2 > 0.0, "the soil did not respire");
    assert!(
        (end - start).abs() < 0.01 * start,
        "soil carbon moved {:.3}% in a day",
        100.0 * (end - start) / start
    );
    println!(
        "a day: soil {:.6e} -> {:.6e} kgC/m² summed, respired {:.6e}",
        start, end, after.respired_kgc_m2
    );
}
