//! Phase 3 Task 6 Step 1/4: threading `GridTopology` through the human and
//! organism runtimes must not change the planetary world. The hash chain
//! after ten steps of the default `WorldState` is pinned.

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::world_integration::WorldState;

/// Hash chain head after ten 3,600 s steps of `WorldState::new(default
/// canon, [11u8; 32])`, recorded before the topology refactor.
const EXPECTED: &str = "c771907ad278eb72c0d6a4c091cc6ca78c5e1cd6cafbd6c718a1046c391e6e73";

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
