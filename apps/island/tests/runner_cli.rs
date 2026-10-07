//! The headless runner, driven as a process: fixed seed twice gives the same
//! hash; a saved run inspects to the hash it printed; replay reproduces it.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn island(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_island"))
        .current_dir(root())
        .args(args)
        .output()
        .expect("island runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn hash(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("state hash "))
        .expect("a state hash line")
        .to_string()
}

const SCENARIO: &str = "fixtures/island/default_scenario.json";

#[test]
fn the_runner_is_deterministic_saves_inspects_and_replays() {
    let dir = tempfile::tempdir().unwrap();
    let snap = dir.path().join("a.snap");
    let log = dir.path().join("a.log.json");
    let (snap_s, log_s) = (snap.to_str().unwrap(), log.to_str().unwrap());
    let run = ["run", "--scenario", SCENARIO, "--steps", "30", "--dt", "60"];

    let (ok1, out1, err1) = island(&[&run[..], &["--save", snap_s, "--log", log_s]].concat());
    assert!(ok1, "{err1}");
    let (ok2, out2, _) = island(&run);
    assert!(ok2);
    assert_eq!(hash(&out1), hash(&out2), "same scenario, different hash");
    assert!(out1.contains("tick 30 "), "{out1}");

    let (ok, inspected, err) = island(&["inspect", "--snapshot", snap_s]);
    assert!(ok, "{err}");
    assert_eq!(hash(&inspected), hash(&out1));

    let (ok, replayed, err) = island(&["replay", "--scenario", SCENARIO, "--log", log_s]);
    assert!(ok, "{err}");
    assert_eq!(hash(&replayed), hash(&out1));

    // Continuing a loaded run equals one longer run.
    let (ok, resumed, err) = island(&["run", "--load", snap_s, "--steps", "10", "--dt", "60"]);
    assert!(ok, "{err}");
    let (_, straight, _) = island(&["run", "--scenario", SCENARIO, "--steps", "40", "--dt", "60"]);
    assert_eq!(hash(&resumed), hash(&straight), "{err}");
}

#[test]
fn bad_input_fails_with_a_message_and_no_command_prints_a_summary() {
    let (ok, _, err) = island(&["run", "--scenario", "no/such.json"]);
    assert!(!ok);
    assert!(err.contains("cannot load scenario"), "{err}");
    let (ok, _, err) = island(&["run", "--scenario", SCENARIO, "--steps", "x"]);
    assert!(!ok && err.contains("--steps"), "{err}");
    let (ok, out, err) = island(&[]);
    assert!(ok, "{err}");
    assert!(out.contains("state hash "), "{out}");
}
