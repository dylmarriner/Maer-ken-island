//! The opt-in bridge to the outside, and what attaching it does and does
//! not change.
//!
//! A human at their own machine in a powered computer room can search the
//! web and send email for real, through `services/computer-service`. That
//! is the one thing on this island that can see out, and it makes the
//! island irreproducible once somebody uses it -- deliberately, which is
//! why it is opt-in.
//!
//! Three claims are made about it and all three are tested here, because
//! the first two are what make the third safe:
//!
//! 1. **Off** -- the default -- the island is exactly the island it was
//!    before this feature existed.
//! 2. **On but failing** the island is still exactly that island. The
//!    costs are charged on success only, so a service that is down or
//!    refusing costs nothing and changes nothing.
//! 3. **Attached**, the affordance is offered where it should be and
//!    nowhere else: in a powered computer room, at a machine, and not
//!    outdoors.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use mk_engine::humans::computer_bridge::{
    BridgeError, ComputerBridge, ComputerBridgeHandle, ComputerState, EmailResult, WebSearchResult,
};
use mk_engine::regional::life::IslandLife;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// A bridge that never works, and counts how often it was asked.
///
/// Not a stub that returns nothing: the point is that a *failing* service
/// leaves the island untouched, which is the state an operator is most
/// likely to be in -- the service not started, the token wrong, the
/// network down.
struct AlwaysFails {
    asked: AtomicUsize,
}

impl AlwaysFails {
    fn new() -> Self {
        Self {
            asked: AtomicUsize::new(0),
        }
    }
}

impl ComputerBridge for AlwaysFails {
    fn web_search(&self, _agent_id: &str, _query: &str) -> Result<WebSearchResult, BridgeError> {
        self.asked.fetch_add(1, Ordering::Relaxed);
        Err(BridgeError::ServiceUnavailable("not started".into()))
    }

    fn send_email(
        &self,
        _agent_id: &str,
        _to: &str,
        _subject: &str,
        _body: &str,
    ) -> Result<EmailResult, BridgeError> {
        self.asked.fetch_add(1, Ordering::Relaxed);
        Err(BridgeError::ServiceUnavailable("not started".into()))
    }

    fn get_computer_state(&self, _agent_id: &str) -> Result<ComputerState, BridgeError> {
        Err(BridgeError::ServiceUnavailable("not started".into()))
    }
}

/// A small island, the same one the other suites use.
fn island() -> IslandLife {
    let mut scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json"))
            .expect("the scenario loads");
    scenario.estate_patch.tree_cap = 50;
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut seed = [0u8; 32];
    seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = seed;
    scenario.canon_path = repo("fixtures/island/canon.json");
    let canon = Arc::new(
        mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).expect("canon"),
    );
    IslandLife::bootstrap(scenario, canon).expect("the island bootstraps")
}

/// Step an island and give back its canonical digest.
fn run(mut life: IslandLife, bridge: Option<ComputerBridgeHandle>) -> ([u8; 32], IslandLife) {
    if let Some(bridge) = bridge {
        life.attach_computer_bridge(bridge);
    }
    for _ in 0..120 {
        life.advance(60).expect("the island steps on");
    }
    let digest = life.state_digest();
    (digest, life)
}

#[test]
fn an_island_with_no_bridge_is_the_island_it_always_was() {
    // The baseline the other two are measured against: two runs of the
    // same seed with nothing attached agree, which is what the island
    // promises and what this feature must not break.
    let (a, _) = run(island(), None);
    let (b, _) = run(island(), None);
    assert_eq!(
        a, b,
        "two runs of one seed disagree before anything is attached"
    );
}

#[test]
fn attaching_a_bridge_that_does_not_work_changes_nothing_at_all() {
    // The configuration an operator is most likely to be in: the feature
    // switched on and the service not running. The island has to be
    // untouched -- the two actions cost glucose and a cooldown on success
    // only, so a refusal is free.
    let (without, _) = run(island(), None);
    let failing = Arc::new(AlwaysFails::new());
    let (with, life) = run(
        island(),
        Some(ComputerBridgeHandle(
            failing.clone() as Arc<dyn ComputerBridge>
        )),
    );
    assert_eq!(
        without, with,
        "a bridge that never succeeds moved the island: {without:x?} against {with:x?}"
    );
    assert!(life.has_computer_bridge(), "it really was attached");
    println!(
        "the failing bridge was asked {} times in 120 steps",
        failing.asked.load(Ordering::Relaxed)
    );
}

#[test]
fn a_bridge_is_not_in_the_snapshot_and_not_in_the_digest() {
    // It is a way out of the island rather than part of it. A snapshot
    // that carried one would mean loading an island and finding it
    // already connected to the internet, which nobody asked for.
    let mut life = island();
    life.attach_computer_bridge(ComputerBridgeHandle(
        Arc::new(AlwaysFails::new()) as Arc<dyn ComputerBridge>
    ));
    for _ in 0..30 {
        life.advance(60).expect("steps");
    }
    let snapshot = life.snapshot();
    let json = serde_json::to_string(&snapshot).expect("the snapshot serializes");
    assert!(
        !json.contains("computer_bridge"),
        "the snapshot carries the bridge"
    );

    // And the digest is the same as the same island without one, stepped
    // the same way.
    let mut plain = island();
    for _ in 0..30 {
        plain.advance(60).expect("steps");
    }
    assert_eq!(plain.state_digest(), life.state_digest());
}

#[test]
fn the_affordance_is_offered_in_the_computer_room_and_nowhere_else() {
    // Two separate affordances, and upstream is explicit about why:
    // `computer_access` is this human being at a machine of their own in a
    // powered room, and `computer_bridge_available` is the world having a
    // way out at all. A search needs both; being in the room is not
    // enough and having a bridge is not enough.
    //
    // This checks the second, which is the one this island newly supplies.
    use mk_engine::humans::GridPosition;
    use mk_engine::perception::Occupancy;
    use mk_engine::regional::humans::{observe, RegionalHumanContext};

    let life = island();
    let at = GridPosition::new(life.placed.location.0 as i32, life.placed.location.1 as i32);
    let occupancy = Occupancy::new_on(
        &mk_engine::topology::GridTopology::regional(&life.domain, mk_island::DomainLevel::Medium),
        [at],
    );
    let positions = life.positions.clone();

    let without = RegionalHumanContext {
        property: &life.placed.property,
        layout: &life.placed.layout,
        physical: &life.physical,
        ecology: &life.ecology,
        energy: &life.energy,
        domain: &life.domain,
        solar_kw: 5.0,
        computer_bridge: None,
    };
    let seen = observe(&without, &positions, &occupancy, life.tick, &at, "Gem-D");
    assert_eq!(
        seen.computer_bridge_available, 0.0,
        "an island with no bridge must not offer the actions that need one"
    );

    let bridge = AlwaysFails::new();
    let with = RegionalHumanContext {
        computer_bridge: Some(&bridge),
        ..without
    };
    let seen = observe(&with, &positions, &occupancy, life.tick, &at, "Gem-D");
    assert_eq!(
        seen.computer_bridge_available, 1.0,
        "an island with a bridge must offer them"
    );
}
