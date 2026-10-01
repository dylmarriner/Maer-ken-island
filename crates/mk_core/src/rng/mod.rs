/**
 * Purpose
 * - Deterministic random number generation for Maer'Ken simulation.
 * - Provides namespaced, replay-stable RNG streams.
 *
 * Invariants
 * - Uses ChaCha20 (not other RNG).
 * - Derives stream via BLAKE3(seed || subsystem || chunk || epoch || tick).
 * - Deterministic (same key → same stream).
 * - NEVER uses global RNG (thread_rng, rand::random).
 *
 * Failure Modes
 * - Non-deterministic RNG → replay failure.
 * - Global RNG usage → simulation divergence.
 * - Incorrect stream derivation → cross-subsystem correlation.
 *
 * Debug Notes
 * - Same RngKey must produce identical streams across runs.
 * - Check for global RNG usage with grep searches.
 * - All RNG streams must be replay-stable.
 */
pub mod audit_log;

pub use audit_log::{RngAuditEntry, RngAuditLog, RngAuditMismatch};

use crate::time::Tick;
use blake3::Hasher;
use rand_chacha::ChaCha20Rng;
use rand_core::RngCore;
use rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Deterministic RNG factory for replay-stable random streams
///
/// Creates independent RNG streams for different subsystems.
/// MUST use ChaCha20 (not other RNG).
/// MUST derive stream via BLAKE3(seed || subsystem || chunk || epoch || tick).
/// MUST be deterministic (same key → same stream).
/// MUST NEVER use global RNG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngRegistry {
    seed: [u8; 32],
}

impl RngRegistry {
    /// Create new RNG registry with seed
    ///
    /// # Arguments
    ///
    /// * `seed` - 32-byte seed for all streams
    ///
    /// # Returns
    ///
    /// New RngRegistry
    pub fn new(seed: [u8; 32]) -> Self {
        Self { seed }
    }

    /// Get RNG stream for specific key
    ///
    /// Derives deterministic stream via BLAKE3.
    /// Same key always produces same stream.
    ///
    /// # Arguments
    ///
    /// * `key` - Stream identifier
    ///
    /// # Returns
    ///
    /// ChaCha20Rng instance for this stream
    pub fn stream(&self, key: RngKey) -> ChaCha20Rng {
        let stream_seed = self.derive_stream_seed(&key);
        ChaCha20Rng::from_seed(stream_seed)
    }

    /// Derive stream seed from registry seed and key
    ///
    /// Uses BLAKE3(seed || subsystem || chunk || epoch || tick).
    /// This ensures deterministic, independent streams.
    ///
    /// # Arguments
    ///
    /// * `key` - Stream identifier
    ///
    /// # Returns
    ///
    /// 32-byte seed for ChaCha20
    fn derive_stream_seed(&self, key: &RngKey) -> [u8; 32] {
        let mut hasher = Hasher::new();

        // Hash the base seed
        hasher.update(&self.seed);

        // Hash subsystem ID
        let subsystem_bytes = (key.subsystem as u32).to_le_bytes();
        hasher.update(&subsystem_bytes);

        // Hash chunk
        hasher.update(&key.chunk.to_le_bytes());

        // Hash epoch
        hasher.update(&key.epoch.to_le_bytes());

        // Hash tick
        hasher.update(&key.tick.to_le_bytes());

        let hash = hasher.finalize();
        *hash.as_bytes()
    }

    /// Get the base seed
    ///
    /// # Returns
    ///
    /// The 32-byte base seed
    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    /// Generate random f64 in [0, 1) for a key
    pub fn gen_f64_01(&self, key: RngKey) -> f64 {
        let mut rng = self.stream(key);
        use rand_core::RngCore;
        let bits = rng.next_u64();
        let fraction = (bits >> 12) as f64;
        fraction / (1u64 << 52) as f64
    }

    /// Generate random f64 in range [min, max] for a key
    pub fn gen_f64_range(&self, key: RngKey, min: f64, max: f64) -> f64 {
        let rand = self.gen_f64_01(key);
        min + rand * (max - min)
    }

    /// Generate random i32 in range [min, max] for a key
    ///
    /// # Panics
    ///
    /// If `min > max`.
    pub fn gen_i32_range(&self, key: RngKey, min: i32, max: i32) -> i32 {
        assert!(min <= max, "min must be <= max");
        let mut rng = self.stream(key);
        use rand_core::RngCore;
        // Widen before subtracting: `max - min + 1` overflows i32 for wide
        // ranges such as i32::MIN..=i32::MAX.
        let range = (max as i64 - min as i64 + 1) as u128;
        let scaled = (rng.next_u64() as u128 * range) >> 64;
        (min as i64 + scaled as i64) as i32
    }

    /// Generate random u32 for a key
    pub fn gen_u32(&self, key: RngKey) -> u32 {
        let mut rng = self.stream(key);
        use rand_core::RngCore;
        rng.next_u32()
    }

    /// Generate random u64 for a key
    pub fn gen_u64(&self, key: RngKey) -> u64 {
        let mut rng = self.stream(key);
        use rand_core::RngCore;
        rng.next_u64()
    }

    /// Like [`Self::gen_u64`], but also records the draw into `log` for
    /// later provenance/replay verification (see [`audit_log`]).
    pub fn gen_u64_audited(&self, key: RngKey, log: &mut audit_log::RngAuditLog) -> u64 {
        let value = self.gen_u64(key);
        log.record(key, 0, value);
        value
    }

    /// Like [`Self::gen_f64_01`], but also records the underlying u64 draw
    /// into `log` for later provenance/replay verification.
    pub fn gen_f64_01_audited(&self, key: RngKey, log: &mut audit_log::RngAuditLog) -> f64 {
        let mut rng = self.stream(key);
        use rand_core::RngCore;
        let bits = rng.next_u64();
        log.record(key, 0, bits);
        let fraction = (bits >> 12) as f64;
        fraction / (1u64 << 52) as f64
    }
}

/// Stream identifier for deterministic RNG
///
/// Combines subsystem, chunk, epoch, and tick for unique streams.
/// Each combination produces a unique but deterministic stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RngKey {
    pub subsystem: SubsystemId,
    pub chunk: u32,
    pub epoch: u32,
    pub tick: Tick,
}

impl RngKey {
    /// Create new RNG key
    ///
    /// # Arguments
    ///
    /// * `subsystem` - Subsystem identifier
    /// * `chunk` - Chunk identifier (for spatial subdivision)
    /// * `epoch` - Epoch identifier (for temporal subdivision)
    /// * `tick` - Current tick
    ///
    /// # Returns
    ///
    /// New RngKey
    pub fn new(subsystem: SubsystemId, chunk: u32, epoch: u32, tick: Tick) -> Self {
        Self {
            subsystem,
            chunk,
            epoch,
            tick,
        }
    }

    /// Create key for current tick (chunk=0, epoch=0)
    ///
    /// # Arguments
    ///
    /// * `subsystem` - Subsystem identifier
    /// * `tick` - Current tick
    ///
    /// # Returns
    ///
    /// New RngKey with chunk=0, epoch=0
    pub fn for_tick(subsystem: SubsystemId, tick: Tick) -> Self {
        Self::new(subsystem, 0, 0, tick)
    }

    /// Get subsystem
    pub fn subsystem(&self) -> SubsystemId {
        self.subsystem
    }

    /// Get chunk
    pub fn chunk(&self) -> u32 {
        self.chunk
    }

    /// Get epoch
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Get tick
    pub fn tick(&self) -> Tick {
        self.tick
    }
}

/// Subsystem identifiers for RNG namespacing
///
/// MUST be exhaustive (all subsystems that use RNG).
/// MUST NEVER add subsystems not in this list without plan amendment.
/// Each subsystem gets independent RNG streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    Orbit,
    Insolation,
    Weather,
    Tectonics,
    Volcanism,
    Biosphere,
    Disturbance,
    Humans,
    /// Genetic recombination (`human::genetics::create_gamete` and
    /// `conceive`), which draw from a caller-supplied stream. Callers should
    /// key it per conception (parents and tick), so two couples conceiving
    /// in the same tick don't share gamete draws, and should not reuse
    /// another subsystem's stream, which would correlate unrelated draws.
    Genetics,
    /// Reserved for `maerken::governance` if/when it starts drawing RNG, so
    /// it never has to share a stream with an unrelated subsystem.
    Governance,
    /// `humans::dialogue`'s deterministic conversation-line template
    /// selection (per-pair-per-tick variety among several valid template
    /// phrasings for the same underlying state) — a genuine RNG consumer,
    /// not reserved-for-later like `Genetics`/`Governance` above.
    Dialogue,
    /// World agents' (`mk_engine::agents`) movement between cells.
    Agents,
}

impl SubsystemId {
    /// Get all subsystem IDs
    ///
    /// # Returns
    ///
    /// Array of all subsystem identifiers
    pub fn all() -> &'static [SubsystemId] {
        use SubsystemId::*;
        &[
            Orbit,
            Insolation,
            Weather,
            Tectonics,
            Volcanism,
            Biosphere,
            Disturbance,
            Humans,
            Genetics,
            Governance,
            Dialogue,
            Agents,
        ][..]
    }

    /// Get subsystem as string
    ///
    /// # Returns
    ///
    /// String representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SubsystemId::Orbit => "Orbit",
            SubsystemId::Insolation => "Insolation",
            SubsystemId::Weather => "Weather",
            SubsystemId::Tectonics => "Tectonics",
            SubsystemId::Volcanism => "Volcanism",
            SubsystemId::Biosphere => "Biosphere",
            SubsystemId::Disturbance => "Disturbance",
            SubsystemId::Humans => "Humans",
            SubsystemId::Genetics => "Genetics",
            SubsystemId::Governance => "Governance",
            SubsystemId::Dialogue => "Dialogue",
            SubsystemId::Agents => "Agents",
        }
    }
}

impl fmt::Display for SubsystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Extension trait for convenient RNG operations
pub trait RngExt {
    /// Generate random f64 in [0, 1)
    ///
    /// Uses 53 bits of precision (double precision).
    ///
    /// # Returns
    ///
    /// Random f64 in [0, 1)
    fn gen_f64_01(&mut self) -> f64;

    /// Generate random i64 in range [min, max]
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum value (inclusive)
    /// * `max` - Maximum value (inclusive)
    ///
    /// # Returns
    ///
    /// Random i64 in specified range
    fn gen_i64_range(&mut self, min: i64, max: i64) -> i64;

    /// Generate random u64 in range [min, max]
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum value (inclusive)
    /// * `max` - Maximum value (inclusive)
    ///
    /// # Returns
    ///
    /// Random u64 in specified range
    fn gen_u64_range(&mut self, min: u64, max: u64) -> u64;
}

impl<R: RngCore> RngExt for R {
    fn gen_f64_01(&mut self) -> f64 {
        // The top 52 bits as a fraction of 2^52: uniform on a 2^-52 lattice
        // in [0.0, 1.0). Changing the bit count would change every stream.
        let bits = self.next_u64();
        let fraction = (bits >> 12) as f64;
        fraction / (1u64 << 52) as f64
    }

    fn gen_i64_range(&mut self, min: i64, max: i64) -> i64 {
        assert!(min <= max, "min must be <= max");
        // Width in i128, so the full i64 range (2^64 values) doesn't
        // overflow; multiplying a u64 draw by it and taking the top 64 bits
        // then covers every value, including the full range.
        let range = (max as i128 - min as i128 + 1) as u128;
        let scaled = (self.next_u64() as u128 * range) >> 64;
        (min as i128 + scaled as i128) as i64
    }

    fn gen_u64_range(&mut self, min: u64, max: u64) -> u64 {
        assert!(min <= max, "min must be <= max");
        let range = (max - min) as u128 + 1;
        let scaled = (self.next_u64() as u128 * range) >> 64;
        min + scaled as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_helpers_cover_full_and_edge_ranges() {
        let registry = RngRegistry::new([3u8; 32]);
        for tick in 0..64 {
            let key = RngKey::for_tick(SubsystemId::Weather, tick);
            let v = registry.gen_i32_range(key, i32::MIN, i32::MAX);
            let _ = v; // any i32 is valid; the point is that it doesn't overflow
            let w = registry.gen_i32_range(key, -3, 3);
            assert!((-3..=3).contains(&w));
            assert_eq!(registry.gen_i32_range(key, 7, 7), 7);

            let mut stream = registry.stream(key);
            let _ = stream.gen_i64_range(i64::MIN, i64::MAX);
            let _ = stream.gen_u64_range(0, u64::MAX);
            assert!((10..=20).contains(&stream.gen_u64_range(10, 20)));
            assert!((-5..=5).contains(&stream.gen_i64_range(-5, 5)));
        }
    }

    #[test]
    #[should_panic(expected = "min must be <= max")]
    fn i32_range_rejects_inverted_bounds() {
        let registry = RngRegistry::new([3u8; 32]);
        registry.gen_i32_range(RngKey::for_tick(SubsystemId::Weather, 0), 5, 1);
    }

    #[test]
    fn rng_registry_deterministic() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);

        let key = RngKey::for_tick(SubsystemId::Weather, 100);
        let mut rng1 = registry.stream(key);
        let mut rng2 = registry.stream(key);

        // Same key should produce identical streams
        for _ in 0..1000 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn rng_different_keys_different_streams() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);

        let key1 = RngKey::for_tick(SubsystemId::Weather, 100);
        let key2 = RngKey::for_tick(SubsystemId::Orbit, 100);

        let mut rng1 = registry.stream(key1);
        let mut rng2 = registry.stream(key2);

        // Different keys should produce different streams
        let mut different = false;
        for _ in 0..100 {
            if rng1.next_u64() != rng2.next_u64() {
                different = true;
                break;
            }
        }
        assert!(different, "Different keys should produce different streams");
    }

    #[test]
    fn rng_same_tick_different_subsystems() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);
        let tick = 1000;

        let mut streams = Vec::new();
        for &subsystem in SubsystemId::all() {
            let key = RngKey::for_tick(subsystem, tick);
            streams.push(registry.stream(key));
        }

        // All streams should be different
        for i in 0..streams.len() {
            for j in (i + 1)..streams.len() {
                let val_i = streams[i].next_u64();
                let val_j = streams[j].next_u64();
                assert_ne!(
                    val_i, val_j,
                    "Different subsystems should produce different values"
                );
            }
        }
    }

    #[test]
    fn rng_different_ticks_same_subsystem() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);
        let subsystem = SubsystemId::Weather;

        let key1 = RngKey::for_tick(subsystem, 100);
        let key2 = RngKey::for_tick(subsystem, 101);

        let mut rng1 = registry.stream(key1);
        let mut rng2 = registry.stream(key2);

        // Different ticks should produce different streams
        let mut different = false;
        for _ in 0..100 {
            if rng1.next_u64() != rng2.next_u64() {
                different = true;
                break;
            }
        }
        assert!(
            different,
            "Different ticks should produce different streams"
        );
    }

    #[test]
    fn rng_chunk_and_epoch_affect_stream() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);

        let key1 = RngKey::new(SubsystemId::Weather, 0, 0, 100);
        let key2 = RngKey::new(SubsystemId::Weather, 1, 0, 100);
        let key3 = RngKey::new(SubsystemId::Weather, 0, 1, 100);

        let mut rng1 = registry.stream(key1);
        let mut rng2 = registry.stream(key2);
        let mut rng3 = registry.stream(key3);

        let val1 = rng1.next_u64();
        let val2 = rng2.next_u64();
        let val3 = rng3.next_u64();

        assert_ne!(
            val1, val2,
            "Different chunks should produce different streams"
        );
        assert_ne!(
            val1, val3,
            "Different epochs should produce different streams"
        );
        assert_ne!(
            val2, val3,
            "Different chunks/epochs should produce different streams"
        );
    }

    #[test]
    fn rng_ext_gen_f64_01() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);
        let key = RngKey::for_tick(SubsystemId::Weather, 100);
        let mut rng = registry.stream(key);

        // Generate many values and check range
        for _ in 0..1000 {
            let val = rng.gen_f64_01();
            assert!(val >= 0.0, "Value should be >= 0.0");
            assert!(val < 1.0, "Value should be < 1.0");
        }
    }

    #[test]
    fn rng_ext_gen_i64_range() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);
        let key = RngKey::for_tick(SubsystemId::Weather, 100);
        let mut rng = registry.stream(key);

        // Test small range
        for _ in 0..1000 {
            let val = rng.gen_i64_range(-10, 10);
            assert!(val >= -10, "Value should be >= -10");
            assert!(val <= 10, "Value should be <= 10");
        }

        // Test single value
        for _ in 0..10 {
            let val = rng.gen_i64_range(5, 5);
            assert_eq!(val, 5, "Single value range should always return that value");
        }
    }

    #[test]
    fn rng_ext_gen_u64_range() {
        let seed = [42u8; 32];
        let registry = RngRegistry::new(seed);
        let key = RngKey::for_tick(SubsystemId::Weather, 100);
        let mut rng = registry.stream(key);

        // Test small range
        for _ in 0..1000 {
            let val = rng.gen_u64_range(0, 100);
            assert!(val <= 100, "Value should be <= 100");
        }

        // Test single value
        for _ in 0..10 {
            let val = rng.gen_u64_range(42, 42);
            assert_eq!(
                val, 42,
                "Single value range should always return that value"
            );
        }
    }

    #[test]
    fn subsystem_id_all() {
        let all = SubsystemId::all();
        assert_eq!(all.len(), 12);

        // Check all subsystems are present
        let expected = [
            SubsystemId::Orbit,
            SubsystemId::Insolation,
            SubsystemId::Weather,
            SubsystemId::Tectonics,
            SubsystemId::Volcanism,
            SubsystemId::Biosphere,
            SubsystemId::Disturbance,
            SubsystemId::Humans,
            SubsystemId::Genetics,
            SubsystemId::Governance,
            SubsystemId::Dialogue,
            SubsystemId::Agents,
        ];

        for &expected_subsystem in &expected {
            assert!(
                all.contains(&expected_subsystem),
                "Missing subsystem: {:?}",
                expected_subsystem
            );
        }
    }

    #[test]
    fn rng_key_accessors() {
        let key = RngKey::new(SubsystemId::Weather, 42, 7, 1000);

        assert_eq!(key.subsystem(), SubsystemId::Weather);
        assert_eq!(key.chunk(), 42);
        assert_eq!(key.epoch(), 7);
        assert_eq!(key.tick(), 1000);
    }

    #[test]
    fn rng_registry_seed_access() {
        let seed = [123u8; 32];
        let registry = RngRegistry::new(seed);

        assert_eq!(registry.seed(), seed);
    }
}
