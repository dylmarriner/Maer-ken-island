//! Phase 3 Task 6 Step 1/4: threading `GridTopology` through the human and
//! organism runtimes must not change the planetary world. The hash chain
//! after ten steps of the default `WorldState` is pinned.

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::world_integration::WorldState;

/// Hash chain head after ten 3,600 s steps of `WorldState::new(default
/// canon, [11u8; 32])`. First recorded before the topology refactor, which
/// left it unchanged; re-pinned (2026-10-12) when the starter property's
/// shed gained a felling axe, a crosscut saw and a gold pan and every human
/// a `labour_met` field. Both are state the planetary world now carries,
/// not a change to how it steps. Re-pinned again (2026-10-13) when the shed
/// gained two 20 L water jerrycans (R1b, UPSTREAM.md), likewise state only.
const EXPECTED: &str = "e4ece246a21e726f63f1f5561eb3c2a039f8d38c1b503f1647395f6864ba73e2";

#[test]
fn ten_planetary_steps_hash_as_before_the_topology_refactor() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [11u8; 32]);
    for _ in 0..10 {
        world.step_world(3_600).unwrap();
    }
    let hex: String = world
        .hash_chain
        .current
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    println!("hash {hex}");
    assert_eq!(hex, EXPECTED);
}
