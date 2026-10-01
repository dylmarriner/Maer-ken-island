/**
 * Purpose
 * - Cryptographic state verification for Maer'Ken simulation.
 * - Provides append-only hash chain for integrity checking.
 *
 * Invariants
 * - Uses deterministic serialization (bincode).
 * - Uses hash[t] = BLAKE3(hash[t-1] || len || state[t] || len || ledger[t]).
 * - Each hash commits to the whole history before it; `reset` starts a new
 *   chain rather than editing an old one.
 *
 * Failure Modes
 * - Non-deterministic serialization → hash divergence.
 * - Hash chain mutation → integrity violation.
 * - Incorrect hash computation → verification failure.
 *
 * Debug Notes
 * - HashChain::tick_hash() appends one step.
 * - Same state + ledger must produce same hash.
 * - `current` is public so a caller that computes its own step hash (the
 *   engine's `step_hash_commit`) can commit it.
 */
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Cryptographic state verification chain
///
/// Hash chain for integrity verification: each step's hash commits to the
/// previous hash, the step's state bytes and its ledger bytes (see
/// [`HashChain::tick_hash`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashChain {
    pub current: [u8; 32],
}

/// One link of the chain: `BLAKE3(prev || len(state) || state || len(ledger) || ledger)`,
/// with lengths as little-endian `u64`. The length prefixes make the split
/// between state and ledger bytes unambiguous, so moving bytes from one to
/// the other changes the hash.
fn chain_step(prev: &[u8; 32], state_bytes: &[u8], ledger_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(prev);
    hasher.update(&(state_bytes.len() as u64).to_le_bytes());
    hasher.update(state_bytes);
    hasher.update(&(ledger_bytes.len() as u64).to_le_bytes());
    hasher.update(ledger_bytes);
    *hasher.finalize().as_bytes()
}

impl HashChain {
    /// Create new hash chain with seed
    ///
    /// # Arguments
    ///
    /// * `seed` - Initial seed hash
    ///
    /// # Returns
    ///
    /// New HashChain starting with given seed
    pub fn new(seed: [u8; 32]) -> Self {
        Self { current: seed }
    }

    /// Compute hash for current tick
    ///
    /// Updates hash chain with new state and ledger.
    /// Uses deterministic formula: hash[t] = BLAKE3(hash[t-1] || state[t] || ledger[t]).
    ///
    /// # Arguments
    ///
    /// * `state_bytes` - Serialized state for current tick
    /// * `ledger_bytes` - Serialized ledger for current tick
    pub fn tick_hash(&mut self, state_bytes: &[u8], ledger_bytes: &[u8]) {
        self.current = chain_step(&self.current, state_bytes, ledger_bytes);
    }

    /// Get current hash
    ///
    /// # Returns
    ///
    /// Current hash value
    pub fn current(&self) -> [u8; 32] {
        self.current
    }

    /// Start a new chain from `seed`, discarding the current position (for
    /// example when branching a world). It does not rewrite earlier hashes;
    /// anything that recorded them still verifies against its own history.
    ///
    /// # Arguments
    ///
    /// * `seed` - New seed hash
    pub fn reset(&mut self, seed: [u8; 32]) {
        self.current = seed;
    }

    /// Compute expected hash for given state and ledger
    ///
    /// # Arguments
    ///
    /// * `state_bytes` - Serialized state
    /// * `ledger_bytes` - Serialized ledger
    ///
    /// # Returns
    ///
    /// Expected hash without updating current
    pub fn compute_expected_hash(&self, state_bytes: &[u8], ledger_bytes: &[u8]) -> [u8; 32] {
        chain_step(&self.current, state_bytes, ledger_bytes)
    }

    /// Verify hash chain integrity
    ///
    /// Re-computes hash chain from given data and verifies against current.
    ///
    /// # Arguments
    ///
    /// * `seed` - Initial seed
    /// * `states` - Sequence of state bytes
    /// * `ledgers` - Sequence of ledger bytes
    ///
    /// # Returns
    ///
    /// True if hash chain is valid, false otherwise
    pub fn verify(&self, seed: [u8; 32], states: &[Vec<u8>], ledgers: &[Vec<u8>]) -> bool {
        if states.len() != ledgers.len() {
            return false;
        }

        let mut test_chain = HashChain::new(seed);

        for (state_bytes, ledger_bytes) in states.iter().zip(ledgers.iter()) {
            test_chain.tick_hash(state_bytes, ledger_bytes);
        }

        test_chain.current == self.current
    }
}

impl Default for HashChain {
    fn default() -> Self {
        Self::new([0u8; 32])
    }
}

/// Serializable state wrapper for hash chain
///
/// Any state that needs to be hashed should implement this trait.
/// The serialization MUST be deterministic.
pub trait HashableState {
    /// Serialize state to bytes deterministically
    ///
    /// # Returns
    ///
    /// Byte representation of state
    fn to_bytes(&self) -> Vec<u8>;
}

/// Serializable ledger wrapper for hash chain
///
/// Any ledger that needs to be hashed should implement this trait.
/// The serialization MUST be deterministic.
pub trait HashableLedger {
    /// Serialize ledger to bytes deterministically
    ///
    /// # Returns
    ///
    /// Byte representation of ledger
    fn to_bytes(&self) -> Vec<u8>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// Test state for hash chain verification
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct TestState {
        pub tick: u64,
        pub value: i64,
    }

    impl TestState {
        /// Create new test state
        ///
        /// # Arguments
        ///
        /// * `tick` - Current tick
        /// * `value` - Test value
        ///
        /// # Returns
        ///
        /// New TestState
        pub fn new(tick: u64, value: i64) -> Self {
            Self { tick, value }
        }
    }

    impl HashableState for TestState {
        fn to_bytes(&self) -> Vec<u8> {
            bincode::serialize(self).expect("Serialization should not fail")
        }
    }

    /// Test ledger for hash chain verification
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct TestLedger {
        pub entries: Vec<TestLedgerEntry>,
    }

    impl TestLedger {
        /// Create new test ledger
        ///
        /// # Returns
        ///
        /// New TestLedger
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        /// Add entry to ledger
        ///
        /// # Arguments
        ///
        /// * `entry` - Ledger entry to add
        pub fn add_entry(&mut self, entry: TestLedgerEntry) {
            self.entries.push(entry);
        }
    }

    impl HashableLedger for TestLedger {
        fn to_bytes(&self) -> Vec<u8> {
            bincode::serialize(self).expect("Serialization should not fail")
        }
    }

    impl Default for TestLedger {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Test ledger entry
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct TestLedgerEntry {
        pub amount: i64,
        pub source: String,
        pub sink: String,
    }

    impl TestLedgerEntry {
        /// Create new test ledger entry
        ///
        /// # Arguments
        ///
        /// * `amount` - Transfer amount
        /// * `source` - Source identifier
        /// * `sink` - Sink identifier
        ///
        /// # Returns
        ///
        /// New TestLedgerEntry
        pub fn new(amount: i64, source: &str, sink: &str) -> Self {
            Self {
                amount,
                source: source.to_string(),
                sink: sink.to_string(),
            }
        }
    }

    #[test]
    fn moving_bytes_between_state_and_ledger_changes_the_hash() {
        let mut a = HashChain::new([7u8; 32]);
        let mut b = HashChain::new([7u8; 32]);
        a.tick_hash(b"abc", b"def");
        b.tick_hash(b"abcd", b"ef");
        assert_ne!(a.current(), b.current());
    }

    #[test]
    fn hash_chain_deterministic() {
        let seed = [42u8; 32];
        let mut chain1 = HashChain::new(seed);
        let mut chain2 = HashChain::new(seed);

        let state = TestState::new(100, 1000);
        let ledger = TestLedger::new();

        let state_bytes = state.to_bytes();
        let ledger_bytes = ledger.to_bytes();

        chain1.tick_hash(&state_bytes, &ledger_bytes);
        chain2.tick_hash(&state_bytes, &ledger_bytes);

        assert_eq!(chain1.current(), chain2.current());
    }

    #[test]
    fn hash_chain_append_only() {
        let seed = [42u8; 32];
        let mut chain = HashChain::new(seed);

        let initial_hash = chain.current();

        let state = TestState::new(100, 1000);
        let ledger = TestLedger::new();

        let state_bytes = state.to_bytes();
        let ledger_bytes = ledger.to_bytes();

        chain.tick_hash(&state_bytes, &ledger_bytes);

        let after_hash = chain.current();
        assert_ne!(initial_hash, after_hash);

        // Hash should be different after each tick
        chain.tick_hash(&state_bytes, &ledger_bytes);
        let after_second_hash = chain.current();
        assert_ne!(after_hash, after_second_hash);
    }

    #[test]
    fn hash_chain_different_inputs_different_hashes() {
        let seed = [42u8; 32];
        let mut chain1 = HashChain::new(seed);
        let mut chain2 = HashChain::new(seed);

        let state1 = TestState::new(100, 1000);
        let state2 = TestState::new(200, 2000);
        let ledger = TestLedger::new();

        let state1_bytes = state1.to_bytes();
        let state2_bytes = state2.to_bytes();
        let ledger_bytes = ledger.to_bytes();

        chain1.tick_hash(&state1_bytes, &ledger_bytes);
        chain2.tick_hash(&state2_bytes, &ledger_bytes);

        assert_ne!(chain1.current(), chain2.current());
    }

    #[test]
    fn hash_chain_verify_valid() {
        let seed = [42u8; 32];
        let mut chain = HashChain::new(seed);

        let mut states = Vec::new();
        let mut ledgers = Vec::new();

        // Generate test data
        for i in 0..5 {
            let state = TestState::new(i, (i * 100) as i64);
            let ledger = TestLedger::new();

            let state_bytes = state.to_bytes();
            let ledger_bytes = ledger.to_bytes();

            chain.tick_hash(&state_bytes, &ledger_bytes);

            states.push(state_bytes);
            ledgers.push(ledger_bytes);
        }

        // Verification should pass
        assert!(chain.verify(seed, &states, &ledgers));
    }

    #[test]
    fn hash_chain_verify_invalid_state() {
        let seed = [42u8; 32];
        let mut chain = HashChain::new(seed);

        let mut states = Vec::new();
        let mut ledgers = Vec::new();

        // Generate test data
        for i in 0..5 {
            let state = TestState::new(i, (i * 100) as i64);
            let ledger = TestLedger::new();

            let state_bytes = state.to_bytes();
            let ledger_bytes = ledger.to_bytes();

            chain.tick_hash(&state_bytes, &ledger_bytes);

            states.push(state_bytes);
            ledgers.push(ledger_bytes);
        }

        // Modify one state
        states[2] = TestState::new(999, 999).to_bytes();

        // Verification should fail
        assert!(!chain.verify(seed, &states, &ledgers));
    }

    #[test]
    fn hash_chain_verify_invalid_ledger() {
        let seed = [42u8; 32];
        let mut chain = HashChain::new(seed);

        let mut states = Vec::new();
        let mut ledgers = Vec::new();

        // Generate test data
        for i in 0..5 {
            let state = TestState::new(i, (i * 100) as i64);
            let mut ledger = TestLedger::new();

            // Add entry to one ledger
            if i == 2 {
                ledger.add_entry(TestLedgerEntry::new(100, "source", "sink"));
            }

            let state_bytes = state.to_bytes();
            let ledger_bytes = ledger.to_bytes();

            chain.tick_hash(&state_bytes, &ledger_bytes);

            states.push(state_bytes);
            ledgers.push(ledger_bytes);
        }

        // Remove entry from verification data
        ledgers[2] = TestLedger::new().to_bytes();

        // Verification should fail
        assert!(!chain.verify(seed, &states, &ledgers));
    }

    #[test]
    fn hash_chain_reset() {
        let seed1 = [42u8; 32];
        let seed2 = [123u8; 32];
        let mut chain = HashChain::new(seed1);

        let state = TestState::new(100, 1000);
        let ledger = TestLedger::new();

        let state_bytes = state.to_bytes();
        let ledger_bytes = ledger.to_bytes();

        chain.tick_hash(&state_bytes, &ledger_bytes);

        let after_hash = chain.current();
        assert_ne!(seed1, after_hash);

        // Reset to different seed
        chain.reset(seed2);
        assert_eq!(chain.current(), seed2);
    }

    #[test]
    fn test_state_serialization() {
        let state = TestState::new(100, 1000);
        let bytes = state.to_bytes();

        // Deserialization should work
        let deserialized: TestState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_ledger_serialization() {
        let mut ledger = TestLedger::new();
        ledger.add_entry(TestLedgerEntry::new(100, "source1", "sink1"));
        ledger.add_entry(TestLedgerEntry::new(200, "source2", "sink2"));

        let bytes = ledger.to_bytes();

        // Deserialization should work
        let deserialized: TestLedger = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ledger, deserialized);
    }

    #[test]
    fn hash_chain_different_seeds_different_hashes() {
        let seed1 = [42u8; 32];
        let seed2 = [123u8; 32];
        let mut chain1 = HashChain::new(seed1);
        let mut chain2 = HashChain::new(seed2);

        let state = TestState::new(100, 1000);
        let ledger = TestLedger::new();

        let state_bytes = state.to_bytes();
        let ledger_bytes = ledger.to_bytes();

        chain1.tick_hash(&state_bytes, &ledger_bytes);
        chain2.tick_hash(&state_bytes, &ledger_bytes);

        assert_ne!(chain1.current(), chain2.current());
    }

    #[test]
    fn hash_chain_empty_verify() {
        let seed = [42u8; 32];
        let chain = HashChain::new(seed);

        // Empty verification should pass
        assert!(chain.verify(seed, &[], &[]));
    }

    #[test]
    fn hash_chain_verify_length_mismatch() {
        let seed = [42u8; 32];
        let chain = HashChain::new(seed);

        let states = vec![TestState::new(1, 100).to_bytes()];
        let ledgers = vec![]; // Empty

        // Length mismatch should fail
        assert!(!chain.verify(seed, &states, &ledgers));
    }
}
