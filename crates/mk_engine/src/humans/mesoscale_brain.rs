//! Mesoscale Brain Engine Snapshot - a pure coordination/aggregation layer
//! over the already-implemented brain Layers 1-3.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `MesoscaleBrainEngineSchema`. Deliberately **not** new invented biology:
//! this module computes no new neural mechanism of its own. It aggregates
//! [`super::brain_regions::BrainRegionsSnapshot`] (Layer 1),
//! [`super::population_dynamics::PopulationDynamicsSnapshot`] (Layer 2),
//! and [`super::neurochemistry::NeurochemistrySnapshot`] (Layer 3) into the
//! canon's `global_dynamics`-shaped summary (total activity, overall
//! metabolic cost, global stability) — the mesoscale layer's actual job in
//! the schema is cross-region integration, not per-region simulation.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Share of the composite metabolic cost driven by (budget-weighted) regional
/// activity; the rest is driven by network instability. This split is an
/// engine choice: canon's mesoscale schema gives per-region budgets and an
/// activity ceiling (both read below) but no ratio between activity and
/// instability cost.
const ACTIVITY_COST_SHARE: f64 = 0.6;
const INSTABILITY_COST_SHARE: f64 = 1.0 - ACTIVITY_COST_SHARE;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MesoscaleBrainSnapshot {
    /// Mean activation across all Layer 1 functional regions.
    pub total_activity: f64,
    /// Mean fatigue across all Layer 1 functional regions.
    pub total_fatigue: f64,
    /// Layer 2's network-level stability, carried through unchanged — this
    /// is already the mesoscale-appropriate summary for population
    /// dynamics.
    pub network_stability: f64,
    /// Layer 2's global excitation/inhibition balance (positive = net
    /// excitatory).
    pub excitation_inhibition_balance: f64,
    /// Layer 3's current derived brain state, carried through — see
    /// [`super::neurochemistry::BrainState`].
    pub dominant_brain_state: super::neurochemistry::BrainState,
    /// Composite metabolic cost proxy: high activity and low stability both
    /// raise apparent "cost" (a working, hard-working, poorly-regulated
    /// brain costs more than a calm, stable one).
    pub metabolic_cost: f64,
}

impl MesoscaleBrainSnapshot {
    /// Pure aggregation — no schema readout of its own, since every field
    /// here is derived from the Layer 1-3 snapshots already stepped this
    /// tick.
    ///
    /// Each region's share of the metabolic cost is its canon
    /// `mesoscale_brain_engine.region_dynamics[<region>].metabolic_cost`
    /// (the region's relative energy budget). Regions the canon does not
    /// budget, or a profile with no canonical schema, weigh equally.
    pub fn from_layers(
        profile: &HumanProfile,
        regions: &super::brain_regions::BrainRegionsSnapshot,
        population: &super::population_dynamics::PopulationDynamicsSnapshot,
        neurochemistry: &super::neurochemistry::NeurochemistrySnapshot,
    ) -> Self {
        let region_list = [
            ("prefrontal_cortex", &regions.prefrontal_cortex),
            ("limbic_system", &regions.limbic_system),
            ("amygdala", &regions.amygdala),
            ("hippocampus", &regions.hippocampus),
            ("basal_ganglia", &regions.basal_ganglia),
            ("hypothalamus", &regions.hypothalamus),
            ("brainstem", &regions.brainstem),
        ];
        let n = region_list.len() as f64;
        let total_activity = region_list.iter().map(|(_, r)| r.activation).sum::<f64>() / n;
        let total_fatigue = region_list.iter().map(|(_, r)| r.fatigue).sum::<f64>() / n;

        let canon_budget = |name: &str| -> f64 {
            profile
                .canonical_schema()
                .and_then(|s| {
                    s.extreme_brain_detail
                        .mesoscale_brain_engine
                        .region_dynamics
                        .get(name)
                })
                .map(|d| f64::from(d.metabolic_cost))
                .filter(|c| c.is_finite() && *c > 0.0)
                .unwrap_or(1.0)
        };
        let (weighted_activity, budget_total) =
            region_list
                .iter()
                .fold((0.0, 0.0), |(act, total), (name, r)| {
                    let w = canon_budget(name);
                    (act + w * r.activation, total + w)
                });
        // Canon `stability_constraints.max_activity_rate` is the ceiling on
        // sustainable network activity; unset (zero) means no ceiling.
        let activity_ceiling = profile
            .canonical_schema()
            .map(|s| {
                f64::from(
                    s.extreme_brain_detail
                        .mesoscale_brain_engine
                        .equation_parameters
                        .stability_constraints
                        .max_activity_rate,
                )
            })
            .filter(|c| c.is_finite() && *c > 0.0)
            .unwrap_or(f64::INFINITY);
        let budget_weighted_activity = (weighted_activity / budget_total).min(activity_ceiling);

        let network_stability = population.network_state.stability;
        let excitation_inhibition_balance =
            population.network_state.global_excitation - population.network_state.global_inhibition;

        let metabolic_cost = (budget_weighted_activity * ACTIVITY_COST_SHARE
            + (1.0 - network_stability) * INSTABILITY_COST_SHARE)
            .clamp(0.0, 1.0);

        Self {
            total_activity,
            total_fatigue,
            network_stability,
            excitation_inhibition_balance,
            dominant_brain_state: neurochemistry.modulation_effects.current_brain_state,
            metabolic_cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::brain_regions::BrainRegionsSnapshot;
    use crate::humans::neurochemistry::NeurochemistrySnapshot;
    use crate::humans::population_dynamics::PopulationDynamicsSnapshot;
    use mk_core::human::{HumanId, HumanProfile, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("mesoscale_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn aggregates_layer_1_through_3_without_panicking() {
        let regions = BrainRegionsSnapshot::from_profile(&profile());
        let population = PopulationDynamicsSnapshot::from_profile(&profile());
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile());

        let meso =
            MesoscaleBrainSnapshot::from_layers(&profile(), &regions, &population, &neurochemistry);

        assert!(meso.total_activity >= 0.0);
        assert!(meso.metabolic_cost >= 0.0 && meso.metabolic_cost <= 1.0);
    }

    #[test]
    fn high_region_activation_raises_metabolic_cost() {
        let mut high_regions = BrainRegionsSnapshot::from_profile(&profile());
        for r in [
            &mut high_regions.prefrontal_cortex,
            &mut high_regions.limbic_system,
            &mut high_regions.amygdala,
            &mut high_regions.hippocampus,
            &mut high_regions.basal_ganglia,
            &mut high_regions.hypothalamus,
            &mut high_regions.brainstem,
        ] {
            r.activation = 1.0;
        }
        let low_regions = BrainRegionsSnapshot::from_profile(&profile());
        let population = PopulationDynamicsSnapshot::from_profile(&profile());
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile());

        let high = MesoscaleBrainSnapshot::from_layers(
            &profile(),
            &high_regions,
            &population,
            &neurochemistry,
        );
        let low = MesoscaleBrainSnapshot::from_layers(
            &profile(),
            &low_regions,
            &population,
            &neurochemistry,
        );

        assert!(high.metabolic_cost >= low.metabolic_cost);
    }

    #[test]
    fn canon_region_budget_weights_the_metabolic_cost() {
        use mk_core::human::schema::RegionDynamicsEntrySchema;

        let mut regions = BrainRegionsSnapshot::from_profile(&profile());
        regions.brainstem.activation = 1.0;
        regions.prefrontal_cortex.activation = 0.0;
        let population = PopulationDynamicsSnapshot::from_profile(&profile());
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile());

        let budgeted = |brainstem: f32, prefrontal: f32| {
            let mut schema = HumanSchema::canonical_minimal("mesoscale_budget");
            let dynamics = &mut schema
                .extreme_brain_detail
                .mesoscale_brain_engine
                .region_dynamics;
            for (name, cost) in [("brainstem", brainstem), ("prefrontal_cortex", prefrontal)] {
                dynamics.insert(
                    name.to_string(),
                    RegionDynamicsEntrySchema {
                        metabolic_cost: cost,
                        ..Default::default()
                    },
                );
            }
            let p = HumanProfile::from_canonical_schema(HumanId::new(2), schema);
            MesoscaleBrainSnapshot::from_layers(&p, &regions, &population, &neurochemistry)
        };

        let brainstem_heavy = budgeted(10.0, 1.0);
        let cortex_heavy = budgeted(1.0, 10.0);
        assert!(brainstem_heavy.metabolic_cost > cortex_heavy.metabolic_cost);
    }

    #[test]
    fn canon_activity_ceiling_caps_the_metabolic_cost() {
        let mut regions = BrainRegionsSnapshot::from_profile(&profile());
        for r in [
            &mut regions.prefrontal_cortex,
            &mut regions.limbic_system,
            &mut regions.amygdala,
            &mut regions.hippocampus,
            &mut regions.basal_ganglia,
            &mut regions.hypothalamus,
            &mut regions.brainstem,
        ] {
            r.activation = 1.0;
        }
        let population = PopulationDynamicsSnapshot::from_profile(&profile());
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile());

        let with_ceiling = |ceiling: f32| {
            let mut schema = HumanSchema::canonical_minimal("mesoscale_ceiling");
            schema
                .extreme_brain_detail
                .mesoscale_brain_engine
                .equation_parameters
                .stability_constraints
                .max_activity_rate = ceiling;
            let p = HumanProfile::from_canonical_schema(HumanId::new(3), schema);
            MesoscaleBrainSnapshot::from_layers(&p, &regions, &population, &neurochemistry)
        };

        assert!(with_ceiling(0.25).metabolic_cost < with_ceiling(0.0).metabolic_cost);
    }
}
