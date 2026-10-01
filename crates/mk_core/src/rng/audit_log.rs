/**
 * Purpose
 * - Queryable, per-draw RNG audit trail for the live `mk_core::rng`
 *   registry used throughout `mk_engine`.
 * - Closes the gap flagged in `audit-results/maerken-vs-gemini-markenz-gap-audit.md`
 *   (finding 3 / "RNG-audit false-confidence gap"): the predecessor
 *   (`markenz/crates/rng/audit_log.rs`) recorded every draw as
 *   `{tick, subsystem, stream_id, callsite, counter, value}` and could be
 *   queried by subsystem/tick/stream; Maer-Ken had no equivalent, and its
 *   closest analog (`DeterminismAuditor::validate_rng`) ignored its
 *   `RngRegistry` parameter and hardcoded `success: true`.
 *
 * Design note
 * - `RngRegistry::stream(key)` is stateless-by-derivation (`&self`, not
 *   `&mut self`): the same key always derives the same fresh stream, so
 *   unlike markenz's stateful RNG, Maer-Ken's registry never *needs* a log
 *   to reproduce a draw — this log exists to make that guarantee checkable
 *   after the fact (e.g. in CI or a targeted debug session), not to make
 *   replay itself work. Recording is opt-in via the `*_audited` methods on
 *   [`super::RngRegistry`] — existing call sites are unaffected.
 *
 * Invariants
 * - Recording never mutates the registry or the derived stream's output.
 * - `verify_against` re-derives every recorded stream from scratch; a
 *   mismatch means either the registry's seed changed since recording, or
 *   the original value did not actually come from a deterministic
 *   `RngRegistry` stream (e.g. a call to the non-seeded global RNG was
 *   recorded in its place).
 */
use super::{RngKey, RngRegistry, SubsystemId};
use crate::time::Tick;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Default cap on recorded entries before the oldest are evicted.
///
/// `verify_against` is O(entries * counter), and the log is append-only —
/// with no bound, a subsystem wired into this log for a long-running or
/// replay session accumulates unbounded memory and turns `verify_against`
/// into an unbounded CPU cost (found in hostile audit 2026-09-03). Nothing
/// currently wires this log into a live subsystem by default, but the type
/// itself should not be a footgun the day something does.
pub const DEFAULT_MAX_ENTRIES: usize = 100_000;

/// One recorded RNG draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngAuditEntry {
    pub tick: Tick,
    pub subsystem: SubsystemId,
    pub chunk: u32,
    pub epoch: u32,
    /// Position of this draw within its key's stream (0 = first `next_u64`
    /// pulled from `RngRegistry::stream(key)`).
    pub counter: u32,
    /// Raw u64 output at `counter` draws into the stream for this key.
    pub value: u64,
}

/// Append-only, queryable log of RNG draws.
///
/// Not wired into any subsystem by default — attach it explicitly (e.g. as
/// an optional field on a caller's state) and record draws via
/// [`RngRegistry::gen_u64_audited`] / [`RngRegistry::gen_f64_01_audited`]
/// wherever per-draw provenance is worth the bookkeeping cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngAuditLog {
    entries: VecDeque<RngAuditEntry>,
    max_entries: usize,
}

impl Default for RngAuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl RngAuditLog {
    pub fn new() -> Self {
        Self::with_max_entries(DEFAULT_MAX_ENTRIES)
    }

    /// Create a log that evicts its oldest entry once `max_entries` is
    /// exceeded. Pass `usize::MAX` for the old unbounded behavior.
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    /// Record a draw of `value` as the `counter`-th `next_u64()` pulled
    /// from the stream identified by `key`. If the log is at capacity, the
    /// oldest recorded entry is evicted first.
    pub fn record(&mut self, key: RngKey, counter: u32, value: u64) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(RngAuditEntry {
            tick: key.tick(),
            subsystem: key.subsystem(),
            chunk: key.chunk(),
            epoch: key.epoch(),
            counter,
            value,
        });
    }

    pub fn entries(&self) -> impl Iterator<Item = &RngAuditEntry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// All entries recorded under `subsystem`.
    pub fn query_by_subsystem(
        &self,
        subsystem: SubsystemId,
    ) -> impl Iterator<Item = &RngAuditEntry> {
        self.entries
            .iter()
            .filter(move |e| e.subsystem == subsystem)
    }

    /// All entries recorded at `tick`.
    pub fn query_by_tick(&self, tick: Tick) -> impl Iterator<Item = &RngAuditEntry> {
        self.entries.iter().filter(move |e| e.tick == tick)
    }

    /// All entries recorded under one specific `(subsystem, chunk, epoch)`
    /// stream, across ticks.
    pub fn query_by_stream(
        &self,
        subsystem: SubsystemId,
        chunk: u32,
        epoch: u32,
    ) -> impl Iterator<Item = &RngAuditEntry> {
        self.entries
            .iter()
            .filter(move |e| e.subsystem == subsystem && e.chunk == chunk && e.epoch == epoch)
    }

    /// Re-derive every recorded stream from `registry` and confirm the
    /// value at each recorded `counter` position still matches what was
    /// logged. Returns the first mismatch found, if any.
    pub fn verify_against(&self, registry: &RngRegistry) -> Result<(), RngAuditMismatch> {
        use rand_core::RngCore;

        for entry in &self.entries {
            let key = RngKey::new(entry.subsystem, entry.chunk, entry.epoch, entry.tick);
            let mut stream = registry.stream(key);
            let mut actual = 0u64;
            for _ in 0..=entry.counter {
                actual = stream.next_u64();
            }
            if actual != entry.value {
                return Err(RngAuditMismatch {
                    key,
                    counter: entry.counter,
                    expected: entry.value,
                    actual,
                });
            }
        }
        Ok(())
    }
}

/// A recorded draw that no longer reproduces from its `RngRegistry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RngAuditMismatch {
    pub key: RngKey,
    pub counter: u32,
    pub expected: u64,
    pub actual: u64,
}

impl fmt::Display for RngAuditMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RNG audit mismatch for {:?} at counter {}: expected {}, got {}",
            self.key, self.counter, self.expected, self.actual
        )
    }
}

impl std::error::Error for RngAuditMismatch {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_query() {
        let mut log = RngAuditLog::new();
        let key_a = RngKey::for_tick(SubsystemId::Weather, 10);
        let key_b = RngKey::for_tick(SubsystemId::Orbit, 10);
        log.record(key_a, 0, 111);
        log.record(key_b, 0, 222);
        log.record(key_a, 1, 333);

        assert_eq!(log.len(), 3);
        assert_eq!(log.query_by_subsystem(SubsystemId::Weather).count(), 2);
        assert_eq!(log.query_by_subsystem(SubsystemId::Orbit).count(), 1);
        assert_eq!(log.query_by_tick(10).count(), 3);
        assert_eq!(log.query_by_stream(SubsystemId::Weather, 0, 0).count(), 2);
    }

    #[test]
    fn verify_against_matching_registry_succeeds() {
        let registry = RngRegistry::new([7u8; 32]);
        let key = RngKey::for_tick(SubsystemId::Biosphere, 42);

        let mut log = RngAuditLog::new();
        let value = registry.gen_u64_audited(key, &mut log);

        assert_eq!(log.len(), 1);
        assert!(log.verify_against(&registry).is_ok());
        // Sanity: the recorded value really is what was returned.
        assert_eq!(log.entries().next().unwrap().value, value);
    }

    #[test]
    fn verify_against_tampered_entry_fails() {
        let registry = RngRegistry::new([7u8; 32]);
        let key = RngKey::for_tick(SubsystemId::Biosphere, 42);

        let mut log = RngAuditLog::new();
        registry.gen_u64_audited(key, &mut log);

        // Simulate a non-deterministic source having produced this draw
        // instead of the registry (the exact scenario this log exists to
        // catch).
        log.entries[0].value ^= 1;

        assert!(log.verify_against(&registry).is_err());
    }

    #[test]
    fn record_evicts_oldest_entry_once_at_capacity() {
        let registry = RngRegistry::new([3u8; 32]);
        let mut log = RngAuditLog::with_max_entries(2);

        let key0 = RngKey::for_tick(SubsystemId::Orbit, 0);
        let key1 = RngKey::for_tick(SubsystemId::Orbit, 1);
        let key2 = RngKey::for_tick(SubsystemId::Orbit, 2);
        registry.gen_u64_audited(key0, &mut log);
        registry.gen_u64_audited(key1, &mut log);
        registry.gen_u64_audited(key2, &mut log);

        assert_eq!(log.len(), 2, "log must not grow past max_entries");
        assert_eq!(
            log.query_by_tick(0).count(),
            0,
            "oldest entry (tick 0) should have been evicted"
        );
        assert_eq!(log.query_by_tick(1).count(), 1);
        assert_eq!(log.query_by_tick(2).count(), 1);
    }

    #[test]
    fn clear_empties_log() {
        let registry = RngRegistry::new([1u8; 32]);
        let key = RngKey::for_tick(SubsystemId::Orbit, 0);
        let mut log = RngAuditLog::new();
        registry.gen_u64_audited(key, &mut log);
        assert!(!log.is_empty());
        log.clear();
        assert!(log.is_empty());
    }
}
