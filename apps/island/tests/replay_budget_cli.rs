//! The CLI must pass a selected replay budget to the bounded engine.
use mk_engine::regional::replay::IslandReplayLog;
use std::process::Command;

#[test]
fn replay_cli_enforces_explicit_tick_budget_before_bootstrap() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replay.json");
    IslandReplayLog::new("b".repeat(64)).save(&path).unwrap();
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/island/default_scenario.json");
    let output = Command::new(env!("CARGO_BIN_EXE_island"))
        .arg("replay")
        .arg("--scenario")
        .arg(scenario)
        .arg("--log")
        .arg(&path)
        .arg("--until")
        .arg("2")
        .arg("--tick-budget")
        .arg("1")
        .output()
        .expect("run island binary");
    assert!(
        !output.status.success(),
        "replay must enforce its explicit budget"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("per-call limit of 1 ticks"), "{stderr}");
}
