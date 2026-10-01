use mk_core::time::Tick;

pub mod long_horizon_runner;

pub use long_horizon_runner::LongHorizonRunner;

#[derive(Debug, Clone)]
pub struct VerificationConfig {
    pub duration_ticks: Tick,
    pub checkpoint_interval: Tick,
    pub output_dir: String,
}

impl VerificationConfig {
    pub fn new(duration_ticks: Tick, checkpoint_interval: Tick, output_dir: &str) -> Self {
        Self {
            duration_ticks,
            checkpoint_interval,
            output_dir: output_dir.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub final_hash: [u8; 32],
    pub tick_count: Tick,
    pub biomass_variance: f64,
    pub intelligence_variance: f64,
    pub extinction_count: u64,
    pub apex_crash_count: u64,
    pub conservation_drift: ConservationDrift,
    pub average_intelligence: f64,
    pub maximum_intelligence: f64,
}

#[derive(Debug, Clone)]
pub struct ConservationDrift {
    pub energy_drift: f64,
    pub water_drift: f64,
    pub carbon_drift: f64,
    pub oxygen_drift: f64,
    pub nitrogen_drift: f64,
    pub phosphorus_drift: f64,
}

impl ConservationDrift {
    pub fn new() -> Self {
        Self {
            energy_drift: 0.0,
            water_drift: 0.0,
            carbon_drift: 0.0,
            oxygen_drift: 0.0,
            nitrogen_drift: 0.0,
            phosphorus_drift: 0.0,
        }
    }
}

impl Default for ConservationDrift {
    fn default() -> Self {
        Self::new()
    }
}
