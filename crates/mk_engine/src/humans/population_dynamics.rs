//! Neural Population Dynamics Snapshot - leaky-integrator firing-rate
//! simulation over the canon's free-form population list.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `ExtremeBrainDetailSchema.layer_2_population_dynamics`
//! (`NeuralPopulationDynamicsSchema`), ported in `mk_core::human::schema`
//! but previously unread by the engine.
//!
//! **Design decision (documented per project rule to state the plan before
//! editing):** real neural population dynamics operate on millisecond
//! timescales, while a simulation tick here (`dt_years`) can span anywhere
//! from ~500 years to ~50,000 years (see `deep_time_evolution.rs`'s
//! substep note). Integrating literally at that timescale is meaningless.
//! Instead, population activity is treated as equilibrating effectively
//! instantly relative to a sim tick: `step` relaxes the leaky-integrator
//! network toward its fixed point using a **fixed internal iteration
//! count** (`RELAXATION_STEPS`, independent of `dt_years`), driven by the
//! seven [`super::brain_regions::BrainRegionsSnapshot`] region activations
//! as external input to whichever population's `region` field names that
//! region. This keeps the model deterministic (no RNG/wall-clock,
//! `noise_level` from canon is intentionally unused) and bounded-cost
//! regardless of how large a tick's `dt_years` is.
//!
//! If a human's canonical schema has no populations defined (the common
//! case — canon doesn't seed any by default), this snapshot is simply
//! empty; per project rule, no populations are fabricated.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const RELAXATION_STEPS: u32 = 8;
const RELAXATION_DT: f64 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PopulationKind {
    Excitatory,
    Inhibitory,
    Modulatory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationState {
    pub id: String,
    pub region: String,
    pub kind: PopulationKind,
    pub firing_rate: f64,
    pub adaptation: f64,
    decay_constant: f64,
    baseline_excitation: f64,
    baseline_inhibition: f64,
    connection_weights: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkState {
    pub global_excitation: f64,
    pub global_inhibition: f64,
    pub synchrony_level: f64,
    pub stability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PopulationDynamicsSnapshot {
    pub populations: Vec<PopulationState>,
    pub network_state: NetworkState,
}

impl PopulationDynamicsSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::default();
        };
        let dynamics = &schema.extreme_brain_detail.layer_2_population_dynamics;

        let populations = dynamics
            .populations
            .iter()
            .map(|p| PopulationState {
                id: p.id.clone(),
                region: p.region.clone(),
                kind: match p.population_type {
                    Some(mk_core::human::schema::PopulationTypeSchema::Inhibitory) => {
                        PopulationKind::Inhibitory
                    }
                    Some(mk_core::human::schema::PopulationTypeSchema::Modulatory) => {
                        PopulationKind::Modulatory
                    }
                    _ => PopulationKind::Excitatory,
                },
                firing_rate: p.firing_rate as f64,
                adaptation: p.adaptation_level as f64,
                decay_constant: if p.decay_constant != 0.0 {
                    p.decay_constant as f64
                } else {
                    0.3
                },
                baseline_excitation: p.excitation as f64,
                baseline_inhibition: p.inhibition as f64,
                connection_weights: p
                    .connection_weights
                    .iter()
                    .map(|(k, v)| (k.clone(), *v as f64))
                    .collect(),
            })
            .collect::<Vec<_>>();

        let network_state = Self::compute_network_state(&populations);

        Self {
            populations,
            network_state,
        }
    }

    /// Relax the leaky-integrator network toward its fixed point given the
    /// current brain-region activations as external drive. See module docs
    /// for why this uses a fixed internal iteration count rather than
    /// scaling with `dt_years`.
    pub fn step(&self, brain_regions: &super::brain_regions::BrainRegionsSnapshot) -> Self {
        if self.populations.is_empty() {
            return self.clone();
        }

        let region_drive = |region: &str| -> f64 {
            match region {
                "prefrontal_cortex" => brain_regions.prefrontal_cortex.activation,
                "limbic_system" => brain_regions.limbic_system.activation,
                "amygdala" => brain_regions.amygdala.activation,
                "hippocampus" => brain_regions.hippocampus.activation,
                "basal_ganglia" => brain_regions.basal_ganglia.activation,
                "hypothalamus" => brain_regions.hypothalamus.activation,
                "brainstem" => brain_regions.brainstem.activation,
                _ => 0.0,
            }
        };

        let mut populations = self.populations.clone();
        let rates_by_id: BTreeMap<String, f64> = populations
            .iter()
            .map(|p| (p.id.clone(), p.firing_rate))
            .collect();
        let mut rates = rates_by_id;

        for _ in 0..RELAXATION_STEPS {
            let mut next_rates = rates.clone();
            for p in &populations {
                let recurrent: f64 = p
                    .connection_weights
                    .iter()
                    .map(|(peer, weight)| weight * rates.get(peer).copied().unwrap_or(0.0))
                    .sum();

                let sign = match p.kind {
                    PopulationKind::Inhibitory => -1.0,
                    _ => 1.0,
                };

                let drive = p.baseline_excitation - p.baseline_inhibition
                    + region_drive(&p.region)
                    + sign * recurrent
                    - p.adaptation;

                let current = rates.get(&p.id).copied().unwrap_or(p.firing_rate);
                let updated = current
                    + RELAXATION_DT * (-p.decay_constant * current + drive.clamp(-1.0, 1.0));
                next_rates.insert(p.id.clone(), updated.clamp(0.0, 1.0));
            }
            rates = next_rates;
        }

        for p in &mut populations {
            let new_rate = rates.get(&p.id).copied().unwrap_or(p.firing_rate);
            // Spike-frequency adaptation: builds up with sustained firing,
            // decays otherwise.
            p.adaptation = (p.adaptation
                + if new_rate > 0.5 {
                    0.05 * new_rate
                } else {
                    -0.1
                })
            .clamp(0.0, 1.0);
            p.firing_rate = new_rate;
        }

        let network_state = Self::compute_network_state(&populations);

        Self {
            populations,
            network_state,
        }
    }

    fn compute_network_state(populations: &[PopulationState]) -> NetworkState {
        if populations.is_empty() {
            return NetworkState::default();
        }

        let excitatory: Vec<f64> = populations
            .iter()
            .filter(|p| matches!(p.kind, PopulationKind::Excitatory))
            .map(|p| p.firing_rate)
            .collect();
        let inhibitory: Vec<f64> = populations
            .iter()
            .filter(|p| matches!(p.kind, PopulationKind::Inhibitory))
            .map(|p| p.firing_rate)
            .collect();

        let mean = |v: &[f64]| -> f64 {
            if v.is_empty() {
                0.0
            } else {
                v.iter().sum::<f64>() / v.len() as f64
            }
        };

        let global_excitation = mean(&excitatory);
        let global_inhibition = mean(&inhibitory);

        let all_rates: Vec<f64> = populations.iter().map(|p| p.firing_rate).collect();
        let overall_mean = mean(&all_rates);
        let variance = mean(
            &all_rates
                .iter()
                .map(|r| (r - overall_mean).powi(2))
                .collect::<Vec<_>>(),
        );
        // High synchrony = low spread across the network.
        let synchrony_level = (1.0 - variance.sqrt()).clamp(0.0, 1.0);
        // Stability = balance between excitation and inhibition (an
        // unchecked excitation/inhibition imbalance is the textbook
        // instability signature).
        let stability = (1.0 - (global_excitation - global_inhibition).abs()).clamp(0.0, 1.0);

        NetworkState {
            global_excitation,
            global_inhibition,
            synchrony_level,
            stability,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::brain_regions::BrainRegionsSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile_with_populations() -> HumanProfile {
        let mut schema = HumanSchema::canonical_minimal("population_test");
        let dynamics = &mut schema.extreme_brain_detail.layer_2_population_dynamics;
        dynamics
            .populations
            .push(mk_core::human::schema::NeuralPopulationSchema {
                id: "pfc_exc".to_string(),
                region: "prefrontal_cortex".to_string(),
                population_type: Some(mk_core::human::schema::PopulationTypeSchema::Excitatory),
                firing_rate: 0.2,
                excitation: 0.1,
                inhibition: 0.05,
                decay_constant: 0.3,
                connection_weights: BTreeMap::new(),
                noise_level: 0.0,
                adaptation_level: 0.0,
            });
        dynamics
            .populations
            .push(mk_core::human::schema::NeuralPopulationSchema {
                id: "pfc_inh".to_string(),
                region: "prefrontal_cortex".to_string(),
                population_type: Some(mk_core::human::schema::PopulationTypeSchema::Inhibitory),
                firing_rate: 0.1,
                excitation: 0.05,
                inhibition: 0.05,
                decay_constant: 0.3,
                connection_weights: {
                    let mut m = BTreeMap::new();
                    m.insert("pfc_exc".to_string(), 0.4);
                    m
                },
                noise_level: 0.0,
                adaptation_level: 0.0,
            });
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn empty_by_default() {
        let schema = HumanSchema::canonical_minimal("empty_test");
        let profile = HumanProfile::from_canonical_schema(HumanId::new(1), schema);
        let snapshot = PopulationDynamicsSnapshot::from_profile(&profile);
        assert!(snapshot.populations.is_empty());
    }

    #[test]
    fn region_drive_raises_excitatory_population_firing_rate() {
        let profile = profile_with_populations();
        let snapshot = PopulationDynamicsSnapshot::from_profile(&profile);

        let mut active_regions = BrainRegionsSnapshot::from_profile(&profile);
        active_regions.prefrontal_cortex.activation = 1.0;
        let mut quiet_regions = BrainRegionsSnapshot::from_profile(&profile);
        quiet_regions.prefrontal_cortex.activation = 0.0;

        let active_stepped = snapshot.step(&active_regions);
        let quiet_stepped = snapshot.step(&quiet_regions);

        let active_rate = active_stepped
            .populations
            .iter()
            .find(|p| p.id == "pfc_exc")
            .unwrap()
            .firing_rate;
        let quiet_rate = quiet_stepped
            .populations
            .iter()
            .find(|p| p.id == "pfc_exc")
            .unwrap()
            .firing_rate;

        assert!(active_rate >= quiet_rate);
    }

    #[test]
    fn deterministic_repeat_steps_converge() {
        let profile = profile_with_populations();
        let snapshot = PopulationDynamicsSnapshot::from_profile(&profile);
        let regions = BrainRegionsSnapshot::from_profile(&profile);

        let step_a = snapshot.step(&regions);
        let step_b = snapshot.step(&regions);

        for (a, b) in step_a.populations.iter().zip(step_b.populations.iter()) {
            assert!((a.firing_rate - b.firing_rate).abs() < 1e-12);
        }
    }
}
