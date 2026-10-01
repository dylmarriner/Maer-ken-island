use crate::biosphere::evolution::Species;
use crate::biosphere::genetics::Genome;
/// Phase 4 Deep-Time Evolution
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

// Phase 4 locked evolution inputs
pub const BASELINE_MUTATION_PRESSURE: f64 = 1.0e-6;
pub const STRESS_MUTATION_AMPLIFICATION_CEILING: f64 = 3.4;
pub const SELECTION_RESPONSE_GAIN: f64 = 0.18;
pub const TRAIT_DRIFT_DAMPING: f64 = 0.07;
pub const ISOLATION_THRESHOLD: f64 = 0.78;
pub const DIVERGENCE_THRESHOLD: f64 = 0.34;
pub const MINIMUM_SPECIATION_TIMESCALE_YEARS: f64 = 18000.0;
pub const MAX_ADAPTIVE_RADIATION_AMPLIFICATION: f64 = 2.8;
pub const RECOVERY_DELAY_BASELINE_YEARS: f64 = 340.0;
pub const DEEP_COLLAPSE_RECOVERY_TIMELINE_YEARS: f64 = 4200.0;

/// Background per-year hazard rate for a stochastic extinction-trigger event.
/// This crate has no population-growth mechanism (population only shrinks via
/// selection/mortality or drifts by a small +/-1%; the only source of new
/// individuals is speciation, itself gated by `MINIMUM_SPECIATION_TIMESCALE_YEARS`).
/// A per-step probability that scales linearly with `dt_years` (`rate * dt_years`)
/// blows past 1.0 for large coarse-sampling steps and, worse, fires so often at
/// realistic step sizes that the biosphere is driven to total, irreversible
/// extinction well before a 100kyr verification horizon completes. Using the
/// proper Poisson survival form `1 - exp(-rate * dt_years)` keeps the
/// per-step probability bounded in (0, 1) for any `dt_years`, and this rate is
/// tuned so a 100kyr run sees a small number of background extinction events
/// (enough to satisfy the "extinction_count > 0" Phase 6 gate) without wiping
/// out the biosphere long before the horizon ends.
pub const EXTINCTION_HAZARD_RATE_PER_YEAR: f64 = 3.0e-5;

/// Sub-sampling granularity for the extinction hazard, in years.
///
/// The Phase 6 bounded-artifacts verification profile samples ~200 coarse
/// steps regardless of horizon length, so `dt_years` per call scales with
/// the horizon (500y at 100kyr, up to 50,000y at 10myr). Evaluating the
/// Poisson trigger probability once against the *whole* `dt_years` chunk
/// still saturates near-certain at large step sizes (e.g. ~78% at
/// dt_years=50,000), which reproduces the same total-biosphere-collapse
/// failure this rate was tuned to avoid, just via a larger step. Splitting
/// each call into fixed-size sub-steps and rolling an independent trial per
/// sub-step keeps the per-trial probability at the same (tested-safe)
/// magnitude regardless of how coarsely the caller samples, while the
/// expected event count still scales correctly with total elapsed time.
pub const EXTINCTION_SUBSTEP_YEARS: f64 = 500.0;

/// Intrinsic per-year population growth rate toward a species' carrying
/// capacity (logistic growth). This model previously had no growth term at
/// all — population could only shrink (selection pressure, extinction
/// mortality) or drift by a trivial +/-1%, so any sustained positive
/// extinction hazard eventually drove every species to zero given a long
/// enough horizon, no matter how low the hazard rate was tuned. Chosen so a
/// species recovers substantially within a few thousand years after a
/// mortality event, matching the order of magnitude of
/// `DEEP_COLLAPSE_RECOVERY_TIMELINE_YEARS` already used elsewhere in this
/// module for post-extinction recovery pacing.
pub const POPULATION_INTRINSIC_GROWTH_RATE_PER_YEAR: f64 = 0.005;

/// Deep-time evolution system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepTimeEvolution {
    pub current_time_years: f64,
    pub last_speciation_times: Vec<(u64, f64)>,
    pub extinction_events: Vec<ExtinctionEvent>,
    pub recovery_state: RecoveryState,
    pub intelligence_regression_active: bool,
    pub adaptive_radiation_potential: f64,
    /// Per-species carrying capacity for logistic population growth, keyed
    /// by `species_id`. Recorded the first time a species is observed (its
    /// starting/natural population), so growth recovers a species toward its
    /// own baseline after a mortality event rather than toward an unrelated
    /// external quantity. `geographic_range` was considered as the capacity
    /// basis but is on a wildly different numeric scale than
    /// `population_size` in this codebase (~0.2-0.8 vs. hundreds of
    /// thousands), so using it collapsed every population toward
    /// near-zero instead of supporting growth.
    ///
    /// Ordered map so the serialized state (and therefore the world hash)
    /// is independent of hash-map iteration order.
    pub carrying_capacities: std::collections::BTreeMap<u64, f64>,
    /// Number of evolution steps taken. Keys every RNG draw, so each step —
    /// however small its `dt_years` — draws fresh values instead of reusing
    /// the draws of every other step within the same simulated year.
    #[serde(default)]
    pub step_index: u64,
}

/// Extinction event for Phase 4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtinctionEvent {
    pub event_id: u64,
    pub start_time_years: f64,
    pub duration_years: f64,
    pub severity: ExtinctionSeverity,
    pub affected_species: Vec<u64>,
}

/// Extinction severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExtinctionSeverity {
    Background,
    Minor,
    Moderate,
    Major,
    Mass,
}

/// Recovery state after extinction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryState {
    pub time_since_extinction_years: f64,
    pub recovery_rate_multiplier: f64,
    pub available_niches: u32,
    pub adaptive_radiation_potential: f64,
}

impl DeepTimeEvolution {
    fn rng_key(&self, salt: u32) -> RngKey {
        RngKey::new(SubsystemId::Biosphere, salt, 0, self.step_index)
    }

    /// Like `rng_key`, but with an explicit epoch component so a single
    /// coarse `dt_years` step can be sub-sampled into several independent,
    /// deterministic trials (see `EXTINCTION_SUBSTEP_YEARS`).
    fn rng_key_epoch(&self, salt: u32, epoch: u32) -> RngKey {
        RngKey::new(SubsystemId::Biosphere, salt, epoch, self.step_index)
    }

    pub fn new() -> Self {
        Self {
            current_time_years: 0.0,
            last_speciation_times: Vec::new(),
            extinction_events: Vec::new(),
            recovery_state: RecoveryState {
                time_since_extinction_years: 0.0,
                recovery_rate_multiplier: 1.0,
                available_niches: 1000,
                adaptive_radiation_potential: 1.0,
            },
            intelligence_regression_active: false,
            adaptive_radiation_potential: 1.0,
            carrying_capacities: std::collections::BTreeMap::new(),
            step_index: 0,
        }
    }

    pub fn step_deep_time_evolution(
        &mut self,
        species: &mut Vec<Species>,
        dt_years: f64,
        environmental_stress: f64,
        rng: &mut RngRegistry,
    ) -> Result<Vec<String>, String> {
        let mut events =
            self.step_evolution_processes(species, dt_years, environmental_stress, rng)?;
        events.extend(self.step_extinction_processes(species, dt_years, rng));
        Ok(events)
    }

    /// Everything except extinction: population growth, mutation,
    /// selection, drift, speciation, intelligence regression and recovery
    /// dynamics, advancing the model's clock by `dt_years`.
    pub fn step_evolution_processes(
        &mut self,
        species: &mut Vec<Species>,
        dt_years: f64,
        environmental_stress: f64,
        rng: &mut RngRegistry,
    ) -> Result<Vec<String>, String> {
        let mut events = Vec::new();

        self.step_index += 1;
        self.current_time_years += dt_years;

        // Apply population growth toward each species' carrying capacity.
        // This is the only counterbalance to mortality (selection pressure,
        // extinction events) in this model; without it, any sustained
        // positive extinction hazard drives every species to zero given a
        // long enough horizon, regardless of how low the hazard rate is
        // tuned (see EXTINCTION_HAZARD_RATE_PER_YEAR).
        self.apply_population_growth(species, dt_years);

        // Apply mutation pressure
        self.apply_mutations(species, dt_years, environmental_stress, rng)?;

        // Apply selection pressure
        self.apply_selection(species, dt_years, environmental_stress, rng);

        // Apply genetic drift
        self.apply_drift(species, dt_years, rng);

        // Check for speciation
        self.process_speciation(species, dt_years, rng, &mut events);

        // Apply intelligence regression
        self.apply_intelligence_regression(species, dt_years, &mut events);

        // Update recovery dynamics
        self.update_recovery_dynamics(dt_years, &mut events);

        Ok(events)
    }

    /// Stochastic extinction-trigger events over the `dt_years` just
    /// advanced by [`Self::step_evolution_processes`].
    pub fn step_extinction_processes(
        &mut self,
        species: &mut Vec<Species>,
        dt_years: f64,
        rng: &mut RngRegistry,
    ) -> Vec<String> {
        let mut events = Vec::new();
        self.process_extinction_events(species, dt_years, rng, &mut events);
        events
    }

    fn apply_mutations(
        &self,
        species: &mut [Species],
        dt_years: f64,
        environmental_stress: f64,
        rng: &mut RngRegistry,
    ) -> Result<(), String> {
        let mutation_rate = BASELINE_MUTATION_PRESSURE
            * (1.0 + environmental_stress * (STRESS_MUTATION_AMPLIFICATION_CEILING - 1.0))
                .min(STRESS_MUTATION_AMPLIFICATION_CEILING);

        // Mutations arrive as a Poisson process at `mutation_rate` per year,
        // so the chance of at least one over the step is `1 − e^(−rate·dt)`.
        // The linear `rate·dt` overstates it for long steps and passes 1.
        let mutation_probability = -(-mutation_rate * dt_years.max(0.0)).exp_m1();
        for sp in species.iter_mut() {
            if rng.gen_f64_01(self.rng_key(sp.species_id as u32)) < mutation_probability {
                self.mutate_genome(&mut sp.representative_genome, sp.species_id as u32, rng)?;

                if sp.representative_genome.intelligence_index()
                    > crate::biosphere::genetics::PRE_SAPIENT_CEILING
                {
                    return Err(format!(
                        "Species {} exceeded intelligence ceiling",
                        sp.species_id
                    ));
                }
            }
        }

        Ok(())
    }

    fn mutate_genome(
        &self,
        genome: &mut Genome,
        lineage: u32,
        rng: &mut RngRegistry,
    ) -> Result<(), String> {
        let key = |salt: u32| self.rng_key_epoch(salt, lineage);
        match rng.gen_u32(key(10_001)) % 4 {
            0 => {
                let delta = rng.gen_i32_range(key(10_002), -1, 2) as i8;
                genome.structural.size_modifier =
                    (genome.structural.size_modifier as i8 + delta).clamp(0, 15) as u8;
            }
            1 => {
                let delta = rng.gen_i32_range(key(10_003), -1, 2) as i8;
                genome.metabolic.basal_rate =
                    (genome.metabolic.basal_rate as i8 + delta).clamp(0, 15) as u8;
            }
            2 => {
                let delta = rng.gen_i32_range(key(10_004), -1, 2) as i8;
                genome.sensory.visual_bandwidth =
                    (genome.sensory.visual_bandwidth as i8 + delta).clamp(0, 15) as u8;
            }
            3 if rng.gen_f64_01(key(10_005)) < 0.1 => {
                genome.neural.processing_capacity =
                    (genome.neural.processing_capacity as i8 - 1).max(0) as u8;
            }
            _ => {}
        }

        Ok(())
    }

    /// Logistic population growth toward carrying capacity, using the
    /// closed-form solution (not forward-Euler) so it stays numerically
    /// stable for any `dt_years` — including the 50,000-year coarse steps
    /// used by the 10myr bounded-artifacts verification profile, where a
    /// naive `N += r*N*(1-N/K)*dt` update would overshoot wildly since
    /// `r*dt` exceeds the stability bound for such large steps.
    fn apply_population_growth(&mut self, species: &mut [Species], dt_years: f64) {
        for sp in species.iter_mut() {
            let n0 = sp.population_size as f64;
            if n0 <= 0.0 {
                continue;
            }
            let capacity = *self
                .carrying_capacities
                .entry(sp.species_id)
                .or_insert_with(|| n0.max(1.0));
            let exp_rt = (POPULATION_INTRINSIC_GROWTH_RATE_PER_YEAR * dt_years).exp();
            let n_t = capacity * n0 * exp_rt / (capacity + n0 * (exp_rt - 1.0));
            sp.population_size = n_t.max(1.0).round() as u64;
        }
    }

    fn apply_selection(
        &self,
        species: &mut [Species],
        dt_years: f64,
        environmental_stress: f64,
        _rng: &mut RngRegistry,
    ) {
        for sp in species.iter_mut() {
            let fitness = self.calculate_fitness(sp, environmental_stress);
            let selection_pressure =
                SELECTION_RESPONSE_GAIN * environmental_stress * (1.0 - fitness);
            // Selection removes the unfit at `selection_pressure · 10⁻⁴` per
            // year, compounded over the step (the closed form, so a long step
            // cannot overshoot), and rounded rather than truncated so short
            // steps carry no downward bias of one individual per step.
            let removal_rate = selection_pressure * 0.0001;
            let surviving = (-removal_rate * dt_years.max(0.0)).exp();
            sp.population_size = (sp.population_size as f64 * surviving).round().max(1.0) as u64;
        }
    }

    fn calculate_fitness(&self, species: &Species, environmental_stress: f64) -> f64 {
        let size_fitness = match species.representative_genome.structural.size_modifier {
            0..=5 => {
                if environmental_stress < 0.5 {
                    0.8
                } else {
                    0.4
                }
            }
            6..=10 => {
                if environmental_stress < 0.7 {
                    0.9
                } else {
                    0.3
                }
            }
            _ => {
                if environmental_stress < 0.3 {
                    0.7
                } else {
                    0.2
                }
            }
        };

        let intelligence_penalty =
            species.representative_genome.intelligence_index() * environmental_stress * 0.5;
        (size_fitness - intelligence_penalty).max(0.1f64)
    }

    /// Random drift of population and range. Drift is zero-mean noise, so
    /// its spread over an interval grows with the square root of the
    /// interval: each step's kick scales with `√dt`, which keeps a year of
    /// short steps as noisy as one yearly step.
    fn apply_drift(&self, species: &mut [Species], dt_years: f64, rng: &mut RngRegistry) {
        let noise_scale = dt_years.max(0.0).sqrt();
        for sp in species.iter_mut() {
            let drift_factor =
                rng.gen_f64_range(self.rng_key(sp.species_id as u32 ^ 20_001), -0.01, 0.01)
                    * TRAIT_DRIFT_DAMPING
                    * noise_scale
                    * 0.0001;
            sp.population_size = (sp.population_size as f64 * (1.0 + drift_factor))
                .round()
                .max(1.0) as u64;

            let range_drift =
                rng.gen_f64_range(self.rng_key(sp.species_id as u32 ^ 20_002), -0.005, 0.005)
                    * TRAIT_DRIFT_DAMPING
                    * noise_scale
                    * 0.0001;
            sp.geographic_range = (sp.geographic_range * (1.0 + range_drift)).max(1.0);
        }
    }

    fn process_extinction_events(
        &mut self,
        species: &mut Vec<Species>,
        dt_years: f64,
        rng: &mut RngRegistry,
        events: &mut Vec<String>,
    ) {
        // Sub-sample `dt_years` into fixed-size chunks so the per-trial
        // trigger probability stays at the tested-safe magnitude regardless
        // of how coarsely the caller samples (see EXTINCTION_SUBSTEP_YEARS).
        let n_substeps = (dt_years / EXTINCTION_SUBSTEP_YEARS).ceil().max(1.0) as u32;
        let substep_years = dt_years / n_substeps as f64;
        let trigger_probability = 1.0 - (-EXTINCTION_HAZARD_RATE_PER_YEAR * substep_years).exp();

        for substep in 0..n_substeps {
            if rng.gen_f64_01(self.rng_key_epoch(30_001, substep)) < trigger_probability {
                let severity = if rng.gen_f64_01(self.rng_key_epoch(30_002, substep)) < 0.1 {
                    ExtinctionSeverity::Mass
                } else if rng.gen_f64_01(self.rng_key_epoch(30_003, substep)) < 0.3 {
                    ExtinctionSeverity::Major
                } else {
                    ExtinctionSeverity::Moderate
                };

                let extinction_event = ExtinctionEvent {
                    event_id: rng.gen_u64(self.rng_key_epoch(30_004, substep)),
                    start_time_years: self.current_time_years + substep as f64 * substep_years,
                    duration_years: 1000.0
                        + rng.gen_f64_01(self.rng_key_epoch(30_005, substep)) * 9000.0,
                    severity: severity.clone(),
                    affected_species: species.iter().map(|s| s.species_id).collect(),
                };

                self.extinction_events.push(extinction_event.clone());
                events.push(format!("EXTINCTION EVENT: {:?}", severity));

                self.apply_extinction_effects(species, &severity, events);

                if species.is_empty() {
                    break;
                }
            }
        }
    }

    fn apply_extinction_effects(
        &mut self,
        species: &mut Vec<Species>,
        severity: &ExtinctionSeverity,
        events: &mut Vec<String>,
    ) {
        let mortality_rate = match severity {
            ExtinctionSeverity::Background => 0.01,
            ExtinctionSeverity::Minor => 0.1,
            ExtinctionSeverity::Moderate => 0.3,
            ExtinctionSeverity::Major => 0.6,
            ExtinctionSeverity::Mass => 0.9,
        };

        for sp in species.iter_mut() {
            sp.population_size =
                (sp.population_size as f64 * (1.0 - mortality_rate)).max(0.0) as u64;

            if sp.population_size == 0 {
                events.push(format!("Species {} extinct", sp.species_id));
            }
        }

        species.retain(|s| s.population_size > 0);

        if matches!(
            severity,
            ExtinctionSeverity::Major | ExtinctionSeverity::Mass
        ) {
            self.recovery_state.time_since_extinction_years = 0.0;
            self.recovery_state.available_niches = 1000;
            events.push("Recovery phase initiated".to_string());
        }
    }

    fn process_speciation(
        &mut self,
        species: &mut Vec<Species>,
        dt_years: f64,
        rng: &mut RngRegistry,
        events: &mut Vec<String>,
    ) {
        let mut new_species = Vec::new();

        for parent in species.iter() {
            let last_speciation = self
                .last_speciation_times
                .iter()
                .find(|(id, _)| *id == parent.species_id)
                .map(|(_, time)| *time)
                .unwrap_or(0.0);

            if self.current_time_years - last_speciation < MINIMUM_SPECIATION_TIMESCALE_YEARS {
                continue;
            }

            let population_factor = (parent.population_size as f64 / 10000.0).min(1.0);
            let geographic_factor = (parent.geographic_range / 10000.0).min(1.0);
            let recovery_factor = self.recovery_state.adaptive_radiation_potential;

            let speciation_probability =
                0.0001 * population_factor * geographic_factor * recovery_factor * dt_years;

            if rng.gen_f64_01(self.rng_key(parent.species_id as u32 ^ 40_001))
                < speciation_probability
            {
                let mut new_genome = parent.representative_genome.clone();
                // The child lineage draws its own mutation, distinct from
                // any mutation its parent drew this step.
                let child_lineage = (parent.species_id as u32) ^ 0x5EED_0001;
                if let Err(reason) = self.mutate_genome(&mut new_genome, child_lineage, rng) {
                    // `mutate_genome` currently always returns `Ok(())` (no
                    // branch produces an `Err`), so this is defensive: if a
                    // future mutation branch ever does fail, the new
                    // species must not silently clone the parent's genome
                    // unmutated with no signal — record it in the same real
                    // event log this function already writes speciation
                    // successes into.
                    events.push(format!(
                        "Speciation: parent {} genome mutation failed ({reason}), new species created as unmutated clone",
                        parent.species_id
                    ));
                }

                let new_species_id = species.iter().map(|s| s.species_id).max().unwrap_or(0) + 1;
                let new_spec = Species::new(
                    new_species_id,
                    new_genome,
                    (parent.population_size as f64 * 0.1).max(10.0) as u64,
                    parent.geographic_range * 0.5,
                );

                new_species.push(new_spec);
                self.last_speciation_times
                    .push((parent.species_id, self.current_time_years));
                events.push(format!(
                    "Speciation: parent {} -> child {}",
                    parent.species_id, new_species_id
                ));
            }
        }

        species.extend(new_species);
    }

    fn apply_intelligence_regression(
        &mut self,
        species: &mut [Species],
        _dt_years: f64,
        events: &mut Vec<String>,
    ) {
        for sp in species.iter_mut() {
            let current_intelligence = sp.representative_genome.intelligence_index();

            if current_intelligence > crate::biosphere::genetics::PRE_SAPIENT_CEILING * 0.7
                && sp.representative_genome.neural.processing_capacity > 0
            {
                sp.representative_genome.neural.processing_capacity -= 1;
                events.push(format!(
                    "Intelligence regression in species {}",
                    sp.species_id
                ));
            }
        }
    }

    fn update_recovery_dynamics(&mut self, dt_years: f64, _events: &mut Vec<String>) {
        self.recovery_state.time_since_extinction_years += dt_years;

        let recovery_phase =
            if self.recovery_state.time_since_extinction_years < RECOVERY_DELAY_BASELINE_YEARS {
                "Immediate Aftermath"
            } else if self.recovery_state.time_since_extinction_years
                < DEEP_COLLAPSE_RECOVERY_TIMELINE_YEARS
            {
                "Recovery"
            } else {
                "Full Recovery"
            };

        if recovery_phase == "Recovery" {
            self.recovery_state.adaptive_radiation_potential =
                (self.recovery_state.time_since_extinction_years
                    / DEEP_COLLAPSE_RECOVERY_TIMELINE_YEARS
                    * MAX_ADAPTIVE_RADIATION_AMPLIFICATION)
                    .min(MAX_ADAPTIVE_RADIATION_AMPLIFICATION);
        } else if recovery_phase == "Full Recovery" {
            self.recovery_state.adaptive_radiation_potential = 1.0;
        }

        if self.recovery_state.available_niches > 0 {
            self.recovery_state.available_niches = self
                .recovery_state
                .available_niches
                .saturating_sub((dt_years * 0.0001) as u32);
        }
    }

    pub fn validate(&self, species: &[Species]) -> Result<(), String> {
        // Invariant: no species should have two recorded speciation events
        // closer together than MINIMUM_SPECIATION_TIMESCALE_YEARS. (A species
        // that speciated within the last cooldown window is normal and
        // expected — that is not itself a violation.)
        let mut times_by_species: std::collections::HashMap<u64, Vec<f64>> =
            std::collections::HashMap::new();
        for &(species_id, speciation_time) in &self.last_speciation_times {
            times_by_species
                .entry(species_id)
                .or_default()
                .push(speciation_time);
        }
        for (species_id, mut times) in times_by_species {
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in times.windows(2) {
                if pair[1] - pair[0] < MINIMUM_SPECIATION_TIMESCALE_YEARS {
                    return Err(format!("Species {} speciated too recently", species_id));
                }
            }
        }

        for sp in species {
            if sp.representative_genome.intelligence_index()
                > crate::biosphere::genetics::PRE_SAPIENT_CEILING
            {
                return Err(format!(
                    "Species {} exceeds intelligence ceiling",
                    sp.species_id
                ));
            }
        }

        if self.recovery_state.adaptive_radiation_potential > MAX_ADAPTIVE_RADIATION_AMPLIFICATION {
            return Err("Adaptive radiation potential exceeds maximum".to_string());
        }

        Ok(())
    }
}

impl Default for DeepTimeEvolution {
    fn default() -> Self {
        Self::new()
    }
}
