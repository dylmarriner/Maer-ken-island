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

fn bootstrap_world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).expect("the island bootstraps")
}

/// Every test here starts from the same island, and bootstrapping spins its
/// climate up for two orbits -- about half a minute in a debug build. So it
/// is bootstrapped once and each test gets its own copy restored from the
/// snapshot, which brings an island back exactly (`island_snapshot.rs`).
fn world() -> IslandWorldState {
    static BOOTSTRAPPED: std::sync::OnceLock<mk_engine::regional::life::IslandLifeSnapshot> =
        std::sync::OnceLock::new();
    let snapshot = BOOTSTRAPPED
        .get_or_init(|| bootstrap_world().life.snapshot())
        .clone();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState {
        life: mk_engine::regional::life::IslandLife::restore(canon, snapshot)
            .expect("the island restores"),
    }
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
    assert_eq!(
        l.humans.registry.iter().count(),
        2 + l.townsfolk.len(),
        "founders and townsfolk"
    );
    assert_eq!(l.positions.0.len(), 2);
}

/// The town (Task 8): wherever the island's river has a town-sized hydro
/// plant, its first staff and their households are living people on the
/// town's cell, and every worker is on a town business's books at no less
/// than the minimum wage. An island with no town plant has no town.
#[test]
fn the_town_s_people_live_on_the_town_cell_and_its_staff_are_employed() {
    let w = world();
    let l = &w.life;
    let Some(cell) = l.town_cell else {
        assert!(l.townsfolk.is_empty());
        return;
    };
    assert!(!l.townsfolk.is_empty());
    for id in &l.townsfolk {
        let h = l
            .humans
            .registry
            .get_human(id)
            .expect("a townsperson lives");
        assert_eq!(
            (h.position.row as usize, h.position.col as usize),
            cell,
            "{id} stands in the town"
        );
        assert!(
            !l.positions.0.contains_key(id),
            "{id} lives outside the estate"
        );
    }
    let money = l.money.as_ref().expect("the books are open");
    let staff: Vec<&str> = money
        .enterprises
        .iter()
        .filter(|e| e.name == "Gem Health" || e.name == "Gem School")
        .flat_map(|e| e.staff.iter().map(|s| s.person.as_str()))
        .collect();
    assert!(
        staff.len() >= 2,
        "a GP and a nurse at least, and the school's teachers"
    );
    for person in staff {
        assert!(l.townsfolk.contains(person), "{person} lives in the town");
    }
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
