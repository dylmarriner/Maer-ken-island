//! Integration test for biodiversity catalogue with BiosphereSystem
//!
//! Tests that the biodiversity catalogue can be loaded into the biosphere system
//! and that the system can boot with the catalogue data available.

use mk_core::canon::CanonLocked;
use mk_engine::biosphere::BiosphereSystem;
use std::path::PathBuf;
use std::sync::Arc;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn full_biosphere_boot_with_catalogue() {
    let catalogue_path = repo_root().join(
        "assets/biodiversity_catalogue_10000_bundle/maerken_biodiversity_catalogue_10000.jsonl",
    );

    if !catalogue_path.exists() {
        println!(
            "Skipping test: catalogue file not found at {:?}",
            catalogue_path
        );
        return;
    }

    // Create biosphere system
    let canon = Arc::new(CanonLocked::default());
    let mut biosphere = BiosphereSystem::new(canon, 42);

    // Load biodiversity catalogue
    biosphere
        .load_biodiversity_catalogue(&catalogue_path)
        .expect("Failed to load biodiversity catalogue");

    // Verify catalogue is loaded
    assert!(
        biosphere.biodiversity_catalogue.is_some(),
        "Biodiversity catalogue not loaded"
    );

    let catalogue = biosphere.biodiversity_catalogue.as_ref().unwrap();
    assert_eq!(
        catalogue.count(),
        20000,
        "Catalogue should have 20000 entries"
    );

    // Initialize biosphere with procedural species (current behavior)
    biosphere
        .initialize()
        .expect("Failed to initialize biosphere");

    // Verify species were generated
    assert!(
        !biosphere.species.is_empty(),
        "Biosphere should have species after initialization"
    );

    // Verify statistics reflect biosphere state
    assert_eq!(
        biosphere.statistics.total_species,
        biosphere.species.len(),
        "Statistics should match species count"
    );
}
