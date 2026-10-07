//! Phase 4 Task 3: complete island snapshots.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::island_snapshot::{
    load_island_snapshot, save_island_snapshot, IslandSnapshotError,
};
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
    IslandWorldState::new(canon, scenario).unwrap()
}

fn hex(w: &IslandWorldState) -> String {
    w.state_hash()
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn saved(w: &IslandWorldState, dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("island.snap");
    save_island_snapshot(w, &path).unwrap();
    path
}

#[test]
fn a_restored_island_has_the_same_hash_and_keeps_evolving_identically() {
    let dir = tempfile::tempdir().unwrap();
    let mut w = world();
    w.step(1_800).unwrap();
    let started = std::time::Instant::now();
    let path = saved(&w, &dir);
    println!(
        "snapshot {} MB, saved in {:.1} s",
        std::fs::metadata(&path).unwrap().len() / 1_000_000,
        started.elapsed().as_secs_f64()
    );
    // Atomic write: no temp file is left behind.
    assert!(!path.with_extension("tmp").exists());

    let started = std::time::Instant::now();
    let mut restored = load_island_snapshot(&path).expect("loads");
    println!("loaded in {:.1} s", started.elapsed().as_secs_f64());
    assert_eq!(hex(&restored), hex(&w), "round trip changed the state");
    assert_eq!(
        (restored.tick(), restored.sim_time_seconds()),
        (30, 1_800.0)
    );
    assert_eq!(
        restored.life.positions, w.life.positions,
        "estate positions"
    );
    assert_eq!(
        restored.life.placed.layout.items.len(),
        w.life.placed.layout.items.len()
    );
    assert_eq!(
        restored.life.vegetation.trees.len(),
        w.life.vegetation.trees.len()
    );
    assert_eq!(
        restored.life.physical.zonal_background.sim_time_seconds,
        w.life.physical.zonal_background.sim_time_seconds
    );

    // A snapshot is complete only if both worlds then run on identically
    // across an hour boundary (physics fires) and a scheduler remainder.
    w.step(1_900).unwrap();
    restored.step(1_900).unwrap();
    assert_eq!(hex(&restored), hex(&w), "the restored island diverged");
}

#[test]
fn tampering_truncation_and_wrong_formats_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let w = world();
    let path = saved(&w, &dir);
    let good = std::fs::read(&path).unwrap();

    let write = |bytes: &[u8]| {
        let p = dir.path().join("bad.snap");
        std::fs::write(&p, bytes).unwrap();
        p
    };
    // A flipped payload byte.
    let mut tampered = good.clone();
    let mid = tampered.len() / 2;
    tampered[mid] ^= 0xff;
    assert!(matches!(
        load_island_snapshot(&write(&tampered)),
        Err(IslandSnapshotError::DigestMismatch)
    ));
    // Cut in half: the digest no longer matches.
    assert!(matches!(
        load_island_snapshot(&write(&good[..good.len() / 2])),
        Err(IslandSnapshotError::DigestMismatch)
    ));
    // Cut inside the header.
    assert!(matches!(
        load_island_snapshot(&write(&good[..20])),
        Err(IslandSnapshotError::Truncated)
    ));
    // Not a snapshot.
    let mut wrong = good.clone();
    wrong[0] = b'X';
    assert!(matches!(
        load_island_snapshot(&write(&wrong)),
        Err(IslandSnapshotError::BadMagic)
    ));
    // A format version this code does not read.
    let mut future = good.clone();
    future[8] = 99;
    assert!(matches!(
        load_island_snapshot(&write(&future)),
        Err(IslandSnapshotError::UnsupportedFormat(99))
    ));
    // A missing file is an I/O error.
    assert!(matches!(
        load_island_snapshot(&dir.path().join("nope.snap")),
        Err(IslandSnapshotError::Io(..))
    ));
}

#[test]
fn an_incompatible_scenario_or_an_invalid_canon_is_rejected_before_any_state_is_returned() {
    let dir = tempfile::tempdir().unwrap();
    let mut w = world();
    w.life.scenario.version = 2;
    assert!(matches!(
        load_island_snapshot(&saved(&w, &dir)),
        Err(IslandSnapshotError::IncompatibleScenario(_))
    ));

    // A canon with a non-positive planet radius is invalid.
    let mut w = world();
    let mut canon = (*w.life.canon).clone();
    canon.planet_radius_m = -1.0;
    w.life.canon = Arc::new(canon);
    assert!(matches!(
        load_island_snapshot(&saved(&w, &dir)),
        Err(IslandSnapshotError::InvalidCanon(_))
    ));
    // A non-finite canon value never even parses back.
    let mut w = world();
    let mut canon = (*w.life.canon).clone();
    canon.planet_radius_m = f64::NAN;
    w.life.canon = Arc::new(canon);
    assert!(load_island_snapshot(&saved(&w, &dir)).is_err());

    // A profile version this code does not read.
    let mut w = world();
    w.life.scenario.profile.version = 7;
    assert!(matches!(
        load_island_snapshot(&saved(&w, &dir)),
        Err(IslandSnapshotError::IncompatibleScenario(_))
    ));
}
