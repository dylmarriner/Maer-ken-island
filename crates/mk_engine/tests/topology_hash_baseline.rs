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
/// Re-pinned (2026-10-31) after the island worktree reconciliation intentionally
/// changed world state: household gas was replaced by the wood burner, and
/// physiques and human wills became part of the hashed simulation state.
/// The previous pre-reconciliation hash was
/// e4ece246a21e726f63f1f5561eb3c2a039f8d38c1b503f1647395f6864ba73e2.
/// Re-pinned (2026-11-06) after the founding-species generator began capping
/// geographic range at one planetary surface. Species 43 previously started
/// at 1.0175; correcting that invalid physical state intentionally changes the
/// default world's deterministic hash chain. Exact-head CI measured the new
/// ten-step hash below after all other fast tests passed.
/// Future changes must still explain any divergence from the new baseline.
const EXPECTED: &str = "66d0b56023d72ff7079eb74499820dbfd31a0f44ecbddbd69e493fa320e0bdfe";

#[test]
fn ten_planetary_steps_hash_after_island_reconciliation() {
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
