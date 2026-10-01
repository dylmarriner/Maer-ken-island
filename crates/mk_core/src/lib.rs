/**
 * Purpose
 * - Core library for Maer'Ken world simulation.
 * - Contains canon definitions, identifier types, and validation.
 * - Phase 1: Determinism infrastructure (time, RNG, flux, hash, audit, math, grid).
 *
 * Invariants
 * - CanonLocked values are immutable after initialization.
 * - Physical and cognitive quantities are f64/f32 (per the 2026-09-11
 *   realignment in docs/decisions.md); Q32.32 helpers in `math` remain for
 *   subsystems that still need fixed point.
 * - World identifiers (Maer'Ken) are separate from planet identifiers (Marr'Kena).
 * - Keyed RNG streams are reproducible for the same seed and key.
 *
 * Failure Modes
 * - Canon validation failure → `validate_canon` error.
 * - Naming violations → audit failure.
 * - Non-deterministic RNG → replay failure.
 * - Ledger imbalance → conservation violation.
 *
 * Debug Notes
 * - Check validate_canon() output for specific field mismatches.
 * - Use cargo test to verify all invariants hold.
 * - Canon digest must be deterministic across runs.
 * - RNG streams must be identical for same keys.
 * - Hash chain must be append-only and deterministic.
 */
pub mod canon;
pub mod human;
pub mod ids;

// Phase 1: Determinism Infrastructure
pub mod flux;
pub mod grid;
pub mod hash;
pub mod math;
pub mod rng;
pub mod time;

// Biomes: Terrain classification and resources
pub mod biomes;
