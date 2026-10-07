//! Phase 4 Task 1: the island world from one constructor, and its state hash
//! identical across separate processes.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).expect("the island bootstraps")
}

fn hex(h: [u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn one_constructor_builds_every_system() {
    let w = world();
    let l = &w.life;
    assert_eq!((w.tick(), w.sim_time_seconds()), (0, 0.0));
    assert!(l.domain.rows(mk_island::DomainLevel::Medium) > 0);
    assert!(l.physical.geophysics.land_area_m2 > 0.0, "island");
    assert!(l.ecology.total_biomass_kgc(&l.domain) > 0.0, "ecology");
    assert_eq!(l.placed.property.properties.len(), 2, "property");
    assert_eq!(l.placed.layout.buildings.len(), 5, "layout");
    assert!(
        !l.vegetation.trees.is_empty() || l.vegetation.total_carbon_kgc() > 0.0,
        "patch vegetation"
    );
    assert_eq!(l.humans.registry.iter().count(), 2, "founders");
    assert_eq!(l.positions.0.len(), 2);
}

/// Child mode: print the hash and exit.
#[test]
fn child_prints_the_state_hash() {
    if std::env::var("MK_WORLD_HASH_CHILD").is_ok() {
        println!("HASH {}", hex(world().state_hash().unwrap()));
    }
}

#[test]
fn the_state_hash_is_identical_across_separate_processes() {
    let exe = std::env::current_exe().unwrap();
    let run = || {
        let out = Command::new(&exe)
            .args([
                "child_prints_the_state_hash",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("MK_WORLD_HASH_CHILD", "1")
            .output()
            .expect("spawn the test binary");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|l| l.split("HASH ").nth(1).map(|h| h.trim().to_string()))
            .expect("the child printed a hash")
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    assert_eq!(
        a,
        hex(world().state_hash().unwrap()),
        "in-process hash differs from the children's"
    );
}
