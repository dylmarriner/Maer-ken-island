//! Phase 4 Task 3: a whole island through a file and back.
//!
//! Fast tier: a small island — enough to exercise every way a snapshot can
//! be written, refused or restored. Slow tier (`--ignored slow_`): the real
//! island, a quarter of a million individual stems and a day of history,
//! because the one thing a small patch cannot tell us is whether the size
//! the owner actually runs fits through a file.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::{load_island_snapshot, save_island_snapshot, IslandSnapshotError};
use mk_engine::regional::life::{IslandLife, IslandLifeError};
use mk_island::IslandScenario;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}

fn scenario() -> IslandScenario {
    IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap()
}

/// The default island with a small tree cap.
///
/// The patch is what makes a full island slow to serialize, and nothing in
/// the file's framing cares how many stems there are. The full-size round
/// trip is the slow tier's job, at the bottom of this file.
fn small_island() -> IslandLife {
    let mut scenario = scenario();
    scenario.estate_patch.tree_cap = 200;
    IslandLife::bootstrap(scenario, canon()).expect("the island bootstraps")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn a_saved_island_comes_back_as_the_same_island() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("island.mks");

    let mut life = small_island();
    life.advance(3_600).unwrap();
    let before = hex(life.state_digest());

    save_island_snapshot(&mut life, &path).unwrap();
    let mut back = load_island_snapshot(canon(), &path).unwrap();

    assert_eq!(hex(back.state_digest()), before, "the island changed");
    assert_eq!(back.sim_time_s, life.sim_time_s);
    assert_eq!(back.audits_closed, life.audits_closed);
    assert_eq!(back.vegetation.trees, life.vegetation.trees);

    // And it carries on from where it stopped rather than merely looking
    // alike: the same hour simulated on both sides lands on the same state.
    // This is what catches state left out of the snapshot — anything the
    // next hour reads and the file did not carry shows up here.
    back.advance(3_600).unwrap();
    life.advance(3_600).unwrap();
    assert_eq!(
        hex(back.state_digest()),
        hex(life.state_digest()),
        "the restored island diverged after an hour"
    );
}

#[test]
fn saving_twice_leaves_one_whole_file_and_no_temporaries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("island.mks");
    let mut life = small_island();

    save_island_snapshot(&mut life, &path).unwrap();
    save_island_snapshot(&mut life, &path).unwrap();

    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left.len(), 1, "temporary files were left behind: {left:?}");
    assert!(
        load_island_snapshot(canon(), &path).is_ok(),
        "the second save is not readable"
    );
}

#[test]
fn a_flipped_byte_anywhere_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("island.mks");
    save_island_snapshot(&mut small_island(), &path).unwrap();
    let good = std::fs::read(&path).unwrap();

    // The canon digest, the start of the state and its last byte: each is
    // inside the digested region and each must be caught.
    for at in [40, 72, good.len() - 1] {
        let mut bytes = good.clone();
        bytes[at] ^= 0xFF;
        std::fs::write(&path, &bytes).unwrap();
        match load_island_snapshot(canon(), &path) {
            Err(IslandSnapshotError::Tampered) => {}
            Err(other) => panic!("a flipped byte at {at} gave {other}"),
            Ok(_) => panic!("a flipped byte at {at} was accepted"),
        }
    }
}

#[test]
fn a_truncated_or_foreign_file_is_refused_before_it_is_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("island.mks");

    std::fs::write(&path, b"MKISLND\0short").unwrap();
    assert!(matches!(
        load_island_snapshot(canon(), &path).map(|_| ()),
        Err(IslandSnapshotError::Truncated { .. })
    ));

    std::fs::write(&path, b"").unwrap();
    assert!(matches!(
        load_island_snapshot(canon(), &path).map(|_| ()),
        Err(IslandSnapshotError::Truncated { .. })
    ));

    // A JSON file, or any other file, that happens to be handed to us.
    std::fs::write(&path, b"{\"version\":1,\"scenario\":{}}").unwrap();
    assert!(matches!(
        load_island_snapshot(canon(), &path).map(|_| ()),
        Err(IslandSnapshotError::NotASnapshot)
    ));

    // A real snapshot cut short: whole header, half a payload.
    save_island_snapshot(&mut small_island(), &path).unwrap();
    let good = std::fs::read(&path).unwrap();
    std::fs::write(&path, &good[..good.len() / 2]).unwrap();
    assert!(
        load_island_snapshot(canon(), &path).is_err(),
        "half a snapshot loaded"
    );

    // And one whose magic was replaced but whose payload survived.
    let mut bytes = good.clone();
    bytes[0] = b'X';
    std::fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        load_island_snapshot(canon(), &path).map(|_| ()),
        Err(IslandSnapshotError::NotASnapshot)
    ));

    // A file that does not exist at all is a file error, not a bad snapshot.
    assert!(matches!(
        load_island_snapshot(canon(), &dir.path().join("nothing.mks")).map(|_| ()),
        Err(IslandSnapshotError::Io(_))
    ));
}

#[test]
fn an_island_saved_under_other_physics_will_not_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("island.mks");
    save_island_snapshot(&mut small_island(), &path).unwrap();

    let mut other = CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap();
    other.rotation_period_s *= 2.0;
    match load_island_snapshot(Arc::new(other), &path) {
        Err(IslandSnapshotError::CanonMismatch { stored, canon }) => {
            assert_ne!(stored, canon, "the refusal named the same canon twice");
        }
        Err(other) => panic!("wrong refusal for other physics: {other}"),
        Ok(_) => panic!("a day twice as long was accepted"),
    }
}

#[test]
fn a_snapshot_from_a_future_build_is_refused_rather_than_guessed_at() {
    let life = small_island();
    let mut snapshot = life.snapshot();
    snapshot.version = u32::MAX;

    match IslandLife::restore(canon(), snapshot) {
        Err(IslandLifeError::SnapshotVersion(v)) => {
            assert_eq!(v, u32::MAX, "the refusal named the wrong format");
        }
        Err(other) => panic!("wrong refusal for a future format: {other}"),
        Ok(_) => panic!("a future format was accepted"),
    }
}

#[test]
fn a_snapshot_carrying_a_scenario_this_build_cannot_run_is_refused() {
    let life = small_island();

    // A scenario format this build does not know.
    let mut snapshot = life.snapshot();
    snapshot.scenario.version = 99;
    assert!(
        matches!(
            IslandLife::restore(canon(), snapshot).map(|_| ()),
            Err(IslandLifeError::Scenario(_))
        ),
        "an unknown scenario version was accepted"
    );

    // A profile that no longer describes a domain. The profile is checked
    // where the domain is derived from it, not taken on trust from the file.
    let mut snapshot = life.snapshot();
    snapshot.scenario.profile.medium_cell_m = 0.0;
    assert!(
        matches!(
            IslandLife::restore(canon(), snapshot).map(|_| ()),
            Err(IslandLifeError::Scenario(_)) | Err(IslandLifeError::Domain(_))
        ),
        "a profile with no cell size was accepted"
    );
}

/// The real island through a file: a quarter of a million individual stems,
/// a dozen grids of 1,152,000 cells, two people with a day behind them, and
/// the keyed RNG streams that decide what happens next.
///
/// The fast tests above use a small patch because the framing does not care
/// how many trees there are. This one does care: it is the only check that
/// the size the owner actually runs fits through a file, at a size worth
/// keeping, and comes back as the same island.
#[test]
#[ignore = "slow: a full-size island through a file"]
fn slow_a_full_island_survives_a_trip_through_a_file() {
    let mut w = IslandLife::bootstrap(scenario(), canon()).expect("the island bootstraps");
    w.advance(86_400).unwrap();
    let before = hex(w.state_digest());
    let trees = w.vegetation.trees.len();
    assert!(trees > 100_000, "only {trees} trees to save");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("island.mks");
    save_island_snapshot(&mut w, &path).unwrap();
    let bytes = std::fs::metadata(&path).unwrap().len();
    println!("{trees} trees, {} MB on disk", bytes / 1_048_576);
    assert!(
        bytes < 128 * 1_048_576,
        "a full island takes {} MB; it was 44 MB when this was written, from 338 MB uncompressed",
        bytes / 1_048_576
    );

    let mut back = load_island_snapshot(canon(), &path).unwrap();
    assert_eq!(hex(back.state_digest()), before, "the island changed");
    assert_eq!(back.vegetation.trees.len(), trees, "trees were lost");
    assert_eq!(back.vegetation.trees, w.vegetation.trees);

    // And it keeps running as the same island, not just reading as one.
    back.advance(6 * 3_600).unwrap();
    w.advance(6 * 3_600).unwrap();
    assert_eq!(
        hex(back.state_digest()),
        hex(w.state_digest()),
        "the restored island diverged over six hours"
    );
}
