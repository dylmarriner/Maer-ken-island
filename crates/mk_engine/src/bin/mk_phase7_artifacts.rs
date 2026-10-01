//! CLI entry point for Phase 7 bounded release artifacts.
//!
//! Usage: `mk_phase7_artifacts [OUTPUT_DIR]` (default: `artifacts` relative to cwd).

use tracing::{error, info};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "artifacts".to_string());
    match mk_engine::emit_phase7_artifacts(&out) {
        Ok(()) => info!(dir = %out, "Wrote Phase 7 artifacts"),
        Err(e) => {
            error!(error = %e, "emit_phase7_artifacts failed");
            std::process::exit(1);
        }
    }
}
