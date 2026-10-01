//! Physical Capacity Snapshot - a real scalar answering "how effectively
//! can this human's body execute on what it decides to do right now."
//!
//! No canon schema surfaces this directly by name; it exists to close a
//! gap between [`super::autonomy`] and [`super::proprioception`]: the
//! autonomous mind decides *what* a human does (`SeekFood`/`SeekWater`/
//! `SeekShelter`/`Rest`/…), and `proprioception` tracks real joint condition,
//! but nothing connected "wants to seek food" to "how well their body can
//! actually go do it" — a human with stiff, stressed joints, exhausted,
//! sick, or old does not gather food/water as effectively as one who is
//! fit, rested, and healthy, even given identical environmental access.
//! This module is that missing link: a genuine multiplier on how much of
//! an environmental affordance a human's actions actually convert into
//! real outcome, coupling [`super::needs`]'s glucose/hydration gain to real
//! [`super::proprioception`], [`super::body`], and [`super::immune`] state.
//!
//! **Dependency ordering (1-tick lag, deliberate)**: `proprioception.step`
//! itself depends on `needs.fatigue`, so feeding `physical_capacity` into
//! the *same* tick's `needs.step` would be circular. This snapshot instead
//! reads `human.proprioception` as it stood at the *start* of this tick
//! (before this tick's `proprioception.step` runs), the same 1-tick-lag
//! pattern [`super::mesoscale_brain`] already uses when aggregating
//! Layer 1-3 state that itself updates within the same tick. Over any
//! sustained trend this lag is invisible; it only matters for the exact
//! tick condition changes, which is an acceptable, already-precedented
//! trade-off in this engine.

use crate::humans::body::BodySnapshot;
use crate::humans::immune::ImmuneSnapshot;
use crate::humans::proprioception::ProprioceptionSnapshot;
use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhysicalCapacitySnapshot {
    /// How effectively this human's body converts environmental affordance
    /// into real outcome this tick (0 = incapacitated, 1 = full capacity).
    /// Multiplies (never adds to) `needs.rs`'s gain terms — a human with
    /// zero capacity still benefits from abundant access, just far less.
    pub effectiveness: f64,
}

impl PhysicalCapacitySnapshot {
    /// Initial capacity from the same joint, body and immune state the
    /// profile starts this human with.
    pub fn from_profile(profile: &HumanProfile) -> Self {
        Self::step(
            &ProprioceptionSnapshot::from_profile(profile),
            &BodySnapshot::from_profile(profile),
            &ImmuneSnapshot::from_profile(profile),
        )
    }

    /// Real joint stress/flexibility (from the *prior* tick's
    /// proprioception — see module docs), body vital energy, and immune
    /// system stress each degrade effectiveness; none of these are
    /// invented weights, they're a direct readout of "how worn down is
    /// this body right now" from state already tracked elsewhere.
    pub fn step(
        proprioception: &ProprioceptionSnapshot,
        body: &BodySnapshot,
        immune: &ImmuneSnapshot,
    ) -> Self {
        let joint_capacity = (proprioception.overall_flexibility
            * (1.0 - proprioception.overall_joint_stress * 0.5))
            .clamp(0.0, 1.0);
        let vital_capacity = body.vital_energy.clamp(0.0, 1.0);
        let sickness_penalty = (immune.system_stress * 0.4).clamp(0.0, 0.4);

        let effectiveness =
            (joint_capacity * 0.4 + vital_capacity * 0.4 + (1.0 - sickness_penalty) * 0.2)
                .clamp(0.05, 1.0);

        Self { effectiveness }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::body::BodySnapshot;
    use crate::humans::immune::ImmuneSnapshot;
    use crate::humans::proprioception::ProprioceptionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("physical_capacity_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_derives_capacity_from_the_starting_body() {
        let profile = profile();
        let snapshot = PhysicalCapacitySnapshot::from_profile(&profile);
        let derived = PhysicalCapacitySnapshot::step(
            &ProprioceptionSnapshot::from_profile(&profile),
            &BodySnapshot::from_profile(&profile),
            &ImmuneSnapshot::from_profile(&profile),
        );
        assert_eq!(snapshot.effectiveness, derived.effectiveness);
        assert!(snapshot.effectiveness > 0.5 && snapshot.effectiveness <= 1.0);
    }

    #[test]
    fn healthy_fit_body_has_high_capacity() {
        let proprioception = ProprioceptionSnapshot::from_profile(&profile());
        let body = BodySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let snapshot = PhysicalCapacitySnapshot::step(&proprioception, &body, &immune);

        assert!(snapshot.effectiveness > 0.5);
    }

    #[test]
    fn worn_down_body_has_reduced_capacity() {
        let needs = crate::humans::needs::NeedsSnapshot::from_profile(&profile());
        let development = crate::humans::development::DevelopmentSnapshot::from_profile(&profile());
        let mut fatigued_needs = needs.clone();
        fatigued_needs.fatigue = 1.0;
        let mut inflamed_immune = ImmuneSnapshot::from_profile(&profile());
        inflamed_immune.inflammation = 1.0;
        inflamed_immune.system_stress = 1.0;

        let mut proprioception = ProprioceptionSnapshot::from_profile(&profile());
        for _ in 0..10 {
            proprioception =
                proprioception.step(&fatigued_needs, &inflamed_immune, &development, 1.0, (0, 0));
        }
        let mut body = BodySnapshot::from_profile(&profile());
        body.vital_energy = 0.1;

        let healthy_proprioception = ProprioceptionSnapshot::from_profile(&profile());
        let healthy_body = BodySnapshot::from_profile(&profile());
        let healthy_immune = ImmuneSnapshot::from_profile(&profile());

        let worn = PhysicalCapacitySnapshot::step(&proprioception, &body, &inflamed_immune);
        let fit =
            PhysicalCapacitySnapshot::step(&healthy_proprioception, &healthy_body, &healthy_immune);

        assert!(worn.effectiveness < fit.effectiveness);
    }

    #[test]
    fn effectiveness_never_reaches_absolute_zero() {
        let mut proprioception = ProprioceptionSnapshot::from_profile(&profile());
        let needs = {
            let mut n = crate::humans::needs::NeedsSnapshot::from_profile(&profile());
            n.fatigue = 1.0;
            n
        };
        let development = crate::humans::development::DevelopmentSnapshot::from_profile(&profile());
        let mut immune = ImmuneSnapshot::from_profile(&profile());
        immune.inflammation = 1.0;
        immune.system_stress = 1.0;
        for _ in 0..50 {
            proprioception = proprioception.step(&needs, &immune, &development, 1.0, (0, 0));
        }
        let mut body = BodySnapshot::from_profile(&profile());
        body.vital_energy = 0.0;

        let snapshot = PhysicalCapacitySnapshot::step(&proprioception, &body, &immune);

        assert!(snapshot.effectiveness >= 0.05);
    }
}
