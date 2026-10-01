use crate::verification::{ConservationDrift, VerificationConfig, VerificationResult};
use crate::world_integration::WorldState;

pub struct LongHorizonRunner {
    config: VerificationConfig,
}

impl LongHorizonRunner {
    pub fn new(config: VerificationConfig) -> Self {
        Self { config }
    }

    pub fn run(&self, initial_state: WorldState) -> Result<VerificationResult, String> {
        let mut state = initial_state;

        for tick in 0..self.config.duration_ticks {
            if let Err(e) = state.step_world(1) {
                return Err(format!("Step failed at tick {}: {:?}", tick, e));
            }
        }

        let apex_crash_count = self.calculate_apex_crash_count(&state);
        let result = VerificationResult {
            final_hash: state.hash_chain.current,
            tick_count: self.config.duration_ticks,
            biomass_variance: self.calculate_biomass_variance(&state),
            intelligence_variance: self.calculate_intelligence_variance(&state),
            extinction_count: state.biosphere_state.extinction_system.active_events.len() as u64,
            apex_crash_count,
            conservation_drift: self.calculate_conservation_drift(&state),
            average_intelligence: state.biosphere_state.statistics.average_intelligence,
            maximum_intelligence: state.biosphere_state.statistics.maximum_intelligence,
        };

        Ok(result)
    }

    /// Count Major/Mass-severity extinction events — the same real signal
    /// `long_horizon_verification.rs::capture_time_point` derives
    /// `TimeSeriesPoint::apex_crashes` from, applied here to this runner's
    /// own `WorldState` extinction trail.
    fn calculate_apex_crash_count(&self, state: &WorldState) -> u64 {
        state
            .biosphere_state
            .extinction_system
            .active_events
            .iter()
            .filter(|e| e.severity == "Major" || e.severity == "Mass")
            .count() as u64
    }

    fn calculate_biomass_variance(&self, state: &WorldState) -> f64 {
        let species = &state.biosphere_state.species;
        if species.is_empty() {
            return 0.0;
        }

        let biomass_values: Vec<f64> = species
            .iter()
            .map(|s| s.population_size as f64 * s.representative_genome.body_mass_kg())
            .collect();

        if biomass_values.is_empty() {
            return 0.0;
        }

        let mean: f64 = biomass_values.iter().sum::<f64>() / biomass_values.len() as f64;
        let variance: f64 = biomass_values
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>()
            / biomass_values.len() as f64;

        variance
    }

    fn calculate_intelligence_variance(&self, state: &WorldState) -> f64 {
        let species = &state.biosphere_state.species;
        if species.is_empty() {
            return 0.0;
        }

        let intelligence_values: Vec<f64> = species
            .iter()
            .map(|s| s.representative_genome.intelligence_index())
            .collect();

        if intelligence_values.is_empty() {
            return 0.0;
        }

        let mean: f64 = intelligence_values.iter().sum::<f64>() / intelligence_values.len() as f64;
        let variance: f64 = intelligence_values
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>()
            / intelligence_values.len() as f64;

        variance
    }

    /// Real conservation drift, read from the world's own audit trail
    /// (`Ledger::net_imbalance` per flux kind via `calculate_closure_metrics`)
    /// — the same authoritative source `WorldState::validate()` checks and
    /// `long_horizon_verification.rs::capture_time_point` records, rather
    /// than a hardcoded always-zero placeholder.
    fn calculate_conservation_drift(&self, state: &WorldState) -> ConservationDrift {
        ConservationDrift {
            energy_drift: state.audit_trail.energy_closure,
            water_drift: state.audit_trail.water_closure,
            carbon_drift: state.audit_trail.carbon_closure,
            oxygen_drift: state.audit_trail.oxygen_closure,
            nitrogen_drift: state.audit_trail.nitrogen_closure,
            phosphorus_drift: state.audit_trail.phosphorus_closure,
        }
    }
}
