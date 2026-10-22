//! Phase 3 Task 1b: upstream's species at island scale on the owner's
//! island (`regional::species`).

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_engine::biosphere::{SpeciesCategory, TerrainAffinity};
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::species::{Abundance, IslandBiota, NodeStock, AFFINITIES};
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const M: DomainLevel = DomainLevel::Medium;

struct Base {
    canon: Arc<CanonLocked>,
    seed: [u8; 32],
    domain: IslandDomain,
    physical: RegionalPhysicalState,
    ecology: RegionalEcologyState,
    biota: IslandBiota,
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
        let ecology = RegionalEcologyState::bootstrap(&domain, &physical).unwrap();
        let biota = IslandBiota::survey(canon.clone(), &seed, &domain, &ecology, &physical);
        Base {
            canon,
            seed,
            domain,
            physical,
            ecology,
            biota,
        }
    })
}

fn is_marine(affinity: &TerrainAffinity) -> bool {
    matches!(
        affinity,
        TerrainAffinity::OpenOcean | TerrainAffinity::ReefSea
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn land_species_live_on_land_and_sea_species_in_the_sea() {
    let b = base();
    let elevation = b.physical.geophysics.elevation_m.data();
    for affinity in &AFFINITIES {
        let mut cells = 0usize;
        for cell in b.biota.habitat_cells(affinity) {
            cells += 1;
            let land = elevation[cell] > 0.0;
            assert_eq!(
                land,
                !is_marine(affinity),
                "{affinity:?} has habitat at cell {cell}, elevation {}",
                elevation[cell]
            );
        }
        println!("{affinity:?}: {cells} cells");
    }
}

#[test]
fn the_island_has_its_trees_its_open_ground_plants_and_its_sea_life() {
    let b = base();
    for (affinity, what) in [
        (TerrainAffinity::ForestCanopy, "forest"),
        (TerrainAffinity::Hill, "open ground"),
        (TerrainAffinity::OpenOcean, "open sea"),
        (TerrainAffinity::Cliff, "coast"),
        (TerrainAffinity::River, "river"),
    ] {
        assert!(
            b.biota.habitat_area_m2(&affinity) > 0.0,
            "the island has no {what}"
        );
    }
    let carbon = b.biota.carbon_by_species(&b.ecology);
    let living = |category: SpeciesCategory| {
        b.biota
            .species
            .iter()
            .zip(&carbon)
            .filter(|(s, c)| s.category == category && c.is_some_and(|c| c > 0.0))
            .count()
    };
    assert!(
        living(SpeciesCategory::Tree) > 0,
        "no tree species holds carbon"
    );
    assert!(
        living(SpeciesCategory::Flower) > 0,
        "no flower species holds carbon"
    );
    assert!(
        living(SpeciesCategory::Herbivore) > 0,
        "no grazer holds carbon"
    );
}

#[test]
fn the_plants_carbon_is_the_producer_field_before_and_after_a_step() {
    let b = base();
    let field = b.ecology.total_biomass_kgc(&b.domain);
    let species = b.biota.producer_carbon_kgc(&b.ecology);
    assert!(
        close(species, field),
        "plant species hold {species:e} kg C, the field {field:e}"
    );

    // A month of growth and grazing later, the species still are the field:
    // they are a view of it, not a second account.
    let mut stepped = b.ecology.clone();
    stepped.step_with_grazers(30.0 * 86_400.0, b.domain.cell_area_m2(M));
    let field = stepped.total_biomass_kgc(&b.domain);
    let species = b.biota.producer_carbon_kgc(&stepped);
    assert!(close(species, field), "{species:e} against {field:e}");

    // And one at a time agrees with all at once.
    let all = b.biota.carbon_by_species(&stepped);
    let one_by_one: f64 = (0..b.biota.species.len())
        .filter(|&i| b.biota.species[i].abundance == Abundance::ProducerCarbon)
        .filter_map(|i| b.biota.carbon_kgc(i, &stepped))
        .sum();
    let summed: f64 = b
        .biota
        .species
        .iter()
        .zip(&all)
        .filter(|(s, _)| s.abundance == Abundance::ProducerCarbon)
        .filter_map(|(_, c)| *c)
        .sum();
    assert!(close(one_by_one, summed));
}

#[test]
fn the_grazers_carbon_is_the_grazer_pool_on_their_plains_and_no_more() {
    let b = base();
    let cell_area = b.domain.cell_area_m2(M);
    let pool = b.ecology.herbivore_kgc_m2.data();
    let on_plains: f64 = b
        .biota
        .habitat_cells(&TerrainAffinity::Plains)
        .map(|cell| pool[cell] * cell_area)
        .sum();
    let total: f64 = pool.iter().sum::<f64>() * cell_area;
    let species = b.biota.grazer_carbon_kgc(&b.ecology);
    assert!(
        close(species, on_plains),
        "{species:e} against {on_plains:e}"
    );
    assert!(species <= total);
    println!(
        "grazer species hold {species:.3e} kg C of the pool's {total:.3e}; {:.1}% is on ground no grazer species lives on",
        100.0 * (total - species) / total
    );
}

#[test]
fn everything_else_is_upstreams_density_over_the_islands_habitat() {
    let b = base();
    for (i, s) in b.biota.species.iter().enumerate() {
        match s.abundance {
            Abundance::UpstreamDensity => {
                let n = b.biota.population(i).unwrap();
                let expected = s.upstream_density_per_m2 * b.biota.habitat_area_m2(&s.affinity);
                assert!(close(n, expected), "{}", s.name);
                assert!(b.biota.carbon_kgc(i, &b.ecology).is_none());
            }
            Abundance::ProducerCarbon | Abundance::GrazerCarbon => {
                assert!(b.biota.population(i).is_none(), "{}", s.name);
            }
        }
    }
    // A species with no ground on the island has nobody on it.
    for (i, s) in b.biota.species.iter().enumerate() {
        if b.biota.habitat_area_m2(&s.affinity) == 0.0 {
            assert!(b.biota.population(i).is_none_or(|n| n == 0.0));
        }
    }
}

#[test]
fn the_animal_nodes_add_up_to_each_species_on_the_island() {
    let b = base();
    let cells = b.ecology.herbivore_kgc_m2.data().len();
    let mut head = vec![0.0; b.biota.species.len()];
    let mut carbon = vec![0.0; b.biota.species.len()];
    for cell in 0..cells {
        for node in b.biota.animal_nodes(cell, &b.ecology) {
            assert_eq!(node.cell, cell);
            let s = &b.biota.species[node.species];
            assert!(b.biota.is_habitat(cell, &s.affinity), "{}", s.name);
            match node.stock {
                NodeStock::Head(n) => head[node.species] += n,
                NodeStock::CarbonKgc(c) => carbon[node.species] += c,
            }
        }
    }
    for (i, s) in b.biota.species.iter().enumerate() {
        match s.abundance {
            Abundance::ProducerCarbon => assert_eq!((head[i], carbon[i]), (0.0, 0.0)),
            Abundance::GrazerCarbon => {
                let whole = b.biota.carbon_kgc(i, &b.ecology).unwrap();
                assert!(
                    close(carbon[i], whole),
                    "{}: {} against {whole}",
                    s.name,
                    carbon[i]
                );
            }
            Abundance::UpstreamDensity => {
                let whole = b.biota.population(i).unwrap();
                assert!(
                    close(head[i], whole),
                    "{}: {} against {whole}",
                    s.name,
                    head[i]
                );
            }
        }
    }
    // The sea's fish are nodes in the sea.
    let fish = b
        .biota
        .species
        .iter()
        .position(|s| s.category == SpeciesCategory::Aquatic && is_marine(&s.affinity));
    if let Some(fish) = fish {
        let affinity = &b.biota.species[fish].affinity;
        let cell = b.biota.habitat_cells(affinity).next().expect("sea habitat");
        assert!(b
            .biota
            .animal_nodes(cell, &b.ecology)
            .iter()
            .any(|n| n.species == fish));
    }
}

#[test]
fn the_plant_nodes_add_up_to_each_plant_species_on_the_island() {
    let b = base();
    let cells = b.ecology.biomass_kgc_m2.data().len();
    let mut carbon = vec![0.0; b.biota.species.len()];
    for cell in 0..cells {
        for node in b.biota.plant_nodes(cell, &b.ecology) {
            match node.stock {
                NodeStock::CarbonKgc(c) => carbon[node.species] += c,
                NodeStock::Head(_) => panic!("plants are carbon, not head"),
            }
        }
    }
    for (i, s) in b.biota.species.iter().enumerate() {
        if s.abundance == Abundance::ProducerCarbon {
            let whole = b.biota.carbon_kgc(i, &b.ecology).unwrap();
            assert!(
                close(carbon[i], whole),
                "{}: {} against {whole}",
                s.name,
                carbon[i]
            );
        } else {
            assert_eq!(carbon[i], 0.0, "{}", s.name);
        }
    }
}

#[test]
fn the_survey_is_the_same_every_time_for_the_same_seed() {
    let b = base();
    let again = IslandBiota::survey(b.canon.clone(), &b.seed, &b.domain, &b.ecology, &b.physical);
    assert_eq!(again.species.len(), b.biota.species.len());
    for (x, y) in again.species.iter().zip(&b.biota.species) {
        assert_eq!((x.species_id, &x.name), (y.species_id, &y.name));
        assert_eq!(x.upstream_population, y.upstream_population);
    }
    assert_eq!(
        again.producer_carbon_kgc(&b.ecology).to_bits(),
        b.biota.producer_carbon_kgc(&b.ecology).to_bits()
    );
    // And a cell's species list is the same species upstream generated.
    let forest = b
        .biota
        .habitat_cells(&TerrainAffinity::ForestCanopy)
        .next()
        .expect("the island has forest");
    assert!(b
        .biota
        .species_in(forest)
        .any(|s| s.category == SpeciesCategory::Tree));
    assert!(!b
        .biota
        .species_in(forest)
        .any(|s| s.affinity == TerrainAffinity::OpenOcean));
}
