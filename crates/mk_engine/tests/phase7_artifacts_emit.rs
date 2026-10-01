//! Smoke test for bounded Phase 7 artifact emission.

use mk_core::canon::{CanonDerived, CanonLocked};
use mk_engine::{emit_phase7_artifacts_for_horizons, MKIDeclaration, VerificationHorizon};

#[test]
fn emit_phase7_artifacts_writes_files_and_canon_digest_matches_core() {
    let dir = tempfile::tempdir().expect("tempdir");
    emit_phase7_artifacts_for_horizons(dir.path(), &[VerificationHorizon::Kyr100]).expect("emit");

    let name = "100kyr";
    let p = dir.path().join(format!("verification_{}.json", name));
    assert!(p.is_file(), "missing {}", p.display());
    let raw = std::fs::read_to_string(&p).unwrap();
    assert!(
        raw.contains("world_step_mode"),
        "{} should include config / world_step_mode",
        p.display()
    );

    let digest_path = dir.path().join("canon_digest.txt");
    let line = std::fs::read_to_string(&digest_path)
        .expect("canon_digest.txt")
        .trim()
        .to_string();
    assert!(line.starts_with("blake3:"));

    let mki = MKIDeclaration::new(CanonLocked::default());
    assert_eq!(line, mki.generate_canon_digest());

    let derived = CanonDerived::from(&CanonLocked::default());
    let expect = format!(
        "blake3:{}",
        blake3::Hash::from(derived.canon_digest).to_hex()
    );
    assert_eq!(line, expect);

    assert!(dir.path().join("determinism_proof.txt").is_file());
    assert!(dir.path().join("emitter_meta.json").is_file());
}
