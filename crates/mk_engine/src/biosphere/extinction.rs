use serde::{Deserialize, Serialize};

/// Extinction event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtinctionEvent {
    pub event_id: u64,
    pub severity: String,
}

/// Extinction recovery system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtinctionRecoverySystem {
    pub active_events: Vec<ExtinctionEvent>,
}

impl ExtinctionRecoverySystem {
    pub fn new() -> Self {
        Self {
            active_events: Vec::new(),
        }
    }

    /// Record an extinction event produced by the world's evolution model
    /// ([`crate::biosphere::deep_time_evolution::DeepTimeEvolution`]).
    pub fn record(&mut self, event_id: u64, severity: &str) {
        self.active_events.push(ExtinctionEvent {
            event_id,
            severity: severity.to_string(),
        });
    }

    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl Default for ExtinctionRecoverySystem {
    fn default() -> Self {
        Self::new()
    }
}
