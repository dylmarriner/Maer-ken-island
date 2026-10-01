//! Bounded Phase 7 release artifact emitter.
//!
//! Writes `verification_*.json`, `canon_digest.txt`, and `determinism_proof.txt` under a chosen
//! directory using the same
//! engine types as interactive verification (no hand-authored simulation dumps).

use std::fs;
use std::path::Path;
use tracing::info;

use mk_core::canon::CanonLocked;
use serde::Serialize;

use crate::long_horizon_verification::{
    generate_final_report, run_release_verifications_for_horizons, VerificationHorizon,
    VerificationResults,
};
use crate::mki_declaration::MKIDeclaration;

/// JSON sidecar describing how artifacts were produced (for release consumers / parent tickets).
#[derive(Debug, Serialize)]
pub struct Phase7ArtifactEmitterMeta {
    pub schema: &'static str,
    pub engine_crate: &'static str,
    pub engine_version: &'static str,
    pub canon_digest_standard: &'static str,
    pub binary: &'static str,
    pub notes: &'static str,
}

const META_SCHEMA: &str = "mk.phase7.emitter_meta.v1";

/// Same as [`emit_phase7_artifacts`] but only runs the requested horizons (e.g. CI smoke tests).
pub fn emit_phase7_artifacts_for_horizons(
    out_dir: impl AsRef<Path>,
    horizons: &[VerificationHorizon],
) -> std::io::Result<()> {
    emit_phase7_artifacts_inner(out_dir, horizons)
}

/// Run the standard three horizons with the bounded release profile (see
/// [`crate::long_horizon_verification::run_all_verifications_for_release_artifacts`]) and write files.
///
/// Filenames: `verification_100kyr.json`, `verification_1myr.json`, `verification_10myr.json`,
/// `canon_digest.txt`, `determinism_proof.txt`, `emitter_meta.json`.
pub fn emit_phase7_artifacts(out_dir: impl AsRef<Path>) -> std::io::Result<()> {
    let horizons = [
        VerificationHorizon::Kyr100,
        VerificationHorizon::Myr1,
        VerificationHorizon::Myr10,
    ];
    emit_phase7_artifacts_inner(out_dir, &horizons[..])
}

fn emit_phase7_artifacts_inner(
    out_dir: impl AsRef<Path>,
    horizons: &[VerificationHorizon],
) -> std::io::Result<()> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;

    let results = run_release_verifications_for_horizons(horizons);

    for result in &results {
        let path = out_dir.join(format!("verification_{}.json", result.horizon.name()));
        let body = serde_json::to_string_pretty(result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, body)?;

        // Also save full WorldState snapshot for observatory visualization
        let snapshot_path = out_dir.join(format!("verification_{}.bincode", result.horizon.name()));
        if let Some(world_state) = &result.final_world_state {
            crate::io::save_snapshot(world_state, &snapshot_path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            info!(path = ?snapshot_path, "Saved WorldState snapshot");
        }
    }

    let mki = MKIDeclaration::new(CanonLocked::default());
    let digest = mki.generate_canon_digest();
    fs::write(out_dir.join("canon_digest.txt"), format!("{}\n", digest))?;

    let proof = generate_final_report(&results);
    fs::write(out_dir.join("determinism_proof.txt"), proof)?;

    let meta = Phase7ArtifactEmitterMeta {
        schema: META_SCHEMA,
        engine_crate: env!("CARGO_PKG_NAME"),
        engine_version: env!("CARGO_PKG_VERSION"),
        canon_digest_standard: "mk_core::canon::CanonDerived JSON-serialized CanonLocked + BLAKE3 (see docs/DETERMINISM_PROOF.md)",
        binary: "mk_phase7_artifacts",
        notes: "World integration uses WorldStepMode::CoarseSingleStepPerSample for bounded work; see each verification_*.json config.world_step_mode. Full reproducibility reruns (enable_determinism_tests) are off for wall-clock bounds—run mk_engine::run_all_verifications locally when you need the expensive double-run. Added .bincode snapshots for observatory visualization.",
    };
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(out_dir.join("emitter_meta.json"), meta_json)?;

    Ok(())
}

/// Serialize a single horizon result (for tests and custom tooling).
pub fn verification_results_to_json(results: &VerificationResults) -> serde_json::Result<String> {
    serde_json::to_string_pretty(results)
}
