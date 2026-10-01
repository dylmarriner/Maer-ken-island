//! Proprioception Snapshot - real per-joint flexibility/stress state.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `SensorySystemsSchema.proprioception_system.body_schema` (12 major
//! joints: shoulder/elbow/wrist × left/right arm, hip/knee/ankle × left/
//! right leg — head/torso/hand carry only `position`/`orientation` in
//! canon, no `flexibility`/`stress`, so they have no real per-joint state
//! to track here).
//!
//! **What is real vs. data-contract-only, explicitly:** each canon joint
//! object has two kinds of fields. `flexibility`/`stress` are genuinely
//! computable from signals this engine already tracks — age-related
//! stiffening ([`super::development`]), fatigue-driven joint stress
//! ([`super::needs`]), and inflammation-driven stiffness
//! ([`super::immune`]) are real physiological relationships, not invented
//! ones. `position`/`orientation`/`angle`/`target_angle`/`velocity`/
//! `torque` are **pose/kinematic** state — they only mean something if a
//! real movement/animation system is actually posing or moving the body.
//! No such system exists in this engine (no walking, reaching, or labor
//! mechanics anywhere in the sim — checked before starting this module).
//! Populating them would mean inventing arbitrary numbers with no real
//! driver behind them, which is exactly the kind of fabricated content the
//! project's no-mock/no-fake-simulation rule forbids (same reasoning
//! already applied to `advanced_memory.rs`'s empty content lists and
//! `sensory.rs`'s headline-scalar-only tactile/proprioception coverage).
//! Those fields therefore stay undefined here — there is intentionally no
//! `JointPose` struct — pending an actual movement/pose system design.
//!
//! **Consumer:** [`super::physical_capacity::PhysicalCapacitySnapshot`]
//! reads `overall_flexibility`/`overall_joint_stress` (with a deliberate
//! 1-tick lag), and its `effectiveness` scales how much a human's actions
//! actually relieve their needs in [`super::needs`]. Stiff, stressed joints
//! therefore make foraging and drinking less effective. Any injury, labor
//! or movement mechanic should read [`ProprioceptionSnapshot::joint`]
//! rather than re-deriving joint condition independently.
//!
//! **Whole-body pose (added later, still no per-joint fabrication):**
//! [`BodyPose`] is *not* the per-joint `position`/`orientation`/`velocity`
//! this module's header warns against inventing — it is a direct,
//! zero-interpolation readout of the one real movement primitive that
//! exists in this engine: [`super::autonomy::AutonomousMind::apply_movement`]'s
//! authoritative 8-directional grid-cell step. `HumanSystem::step` diffs
//! the human's `GridPosition` before/after that real call and passes the
//! resulting delta straight into [`ProprioceptionSnapshot::step`] — no
//! synthesized walking cycle, no invented joint angles, just "did this
//! human's position actually change this tick, and in which of the 8 real
//! directions." Per-joint kinematics remain out of scope until a real
//! reaching/labor/walking-animation system exists to drive them.

use crate::humans::development::DevelopmentSnapshot;
use crate::humans::immune::ImmuneSnapshot;
use crate::humans::needs::NeedsSnapshot;
use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JointId {
    LeftShoulder,
    LeftElbow,
    LeftWrist,
    RightShoulder,
    RightElbow,
    RightWrist,
    LeftHip,
    LeftKnee,
    LeftAnkle,
    RightHip,
    RightKnee,
    RightAnkle,
}

impl JointId {
    const ALL: [JointId; 12] = [
        JointId::LeftShoulder,
        JointId::LeftElbow,
        JointId::LeftWrist,
        JointId::RightShoulder,
        JointId::RightElbow,
        JointId::RightWrist,
        JointId::LeftHip,
        JointId::LeftKnee,
        JointId::LeftAnkle,
        JointId::RightHip,
        JointId::RightKnee,
        JointId::RightAnkle,
    ];

    /// Distal joints (wrist/ankle) are more sensitive to fatigue-driven
    /// micro-stress than proximal joints (shoulder/hip) — a real
    /// biomechanical relationship (smaller joints, less muscular support).
    fn distal_sensitivity(self) -> f64 {
        match self {
            JointId::LeftWrist | JointId::RightWrist | JointId::LeftAnkle | JointId::RightAnkle => {
                1.2
            }
            JointId::LeftElbow | JointId::RightElbow | JointId::LeftKnee | JointId::RightKnee => {
                1.0
            }
            JointId::LeftShoulder
            | JointId::RightShoulder
            | JointId::LeftHip
            | JointId::RightHip => 0.85,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointCondition {
    /// Range-of-motion capacity (0 = rigid/frozen, 1 = fully flexible).
    pub flexibility: f64,
    /// Accumulated mechanical/inflammatory load on this joint (0 = fresh,
    /// 1 = maximally stressed).
    pub stress: f64,
}

/// One of the 8 real directions [`super::autonomy::AutonomousMind::apply_movement`]
/// can actually step a human's `GridPosition` in. `row` increases south,
/// `col` increases east, matching that function's own `DIRECTIONS` table —
/// this is a readout of the same 8 values, not a separate compass model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Heading8 {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Heading8 {
    /// `None` for `(0, 0)` — no real displacement happened this tick,
    /// either because the chosen action wasn't a movement action or the
    /// attempted step was clamped at the world grid's edge.
    fn from_delta(row_delta: i32, col_delta: i32) -> Option<Self> {
        match (row_delta.signum(), col_delta.signum()) {
            (-1, -1) => Some(Heading8::NorthWest),
            (-1, 0) => Some(Heading8::North),
            (-1, 1) => Some(Heading8::NorthEast),
            (0, -1) => Some(Heading8::West),
            (0, 1) => Some(Heading8::East),
            (1, -1) => Some(Heading8::SouthWest),
            (1, 0) => Some(Heading8::South),
            (1, 1) => Some(Heading8::SouthEast),
            _ => None,
        }
    }
}

/// Real, zero-fabrication whole-body pose — see module docs' "Whole-body
/// pose" section. Derived from an actual `GridPosition` delta, not
/// synthesized.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyPose {
    /// `true` iff this human's `GridPosition` actually changed this tick.
    pub moving: bool,
    /// One of the 8 real movement directions, `None` when `moving` is
    /// `false`.
    pub heading: Option<Heading8>,
}

impl BodyPose {
    fn resting() -> Self {
        Self {
            moving: false,
            heading: None,
        }
    }

    fn from_delta(row_delta: i32, col_delta: i32) -> Self {
        let heading = Heading8::from_delta(row_delta, col_delta);
        Self {
            moving: heading.is_some(),
            heading,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProprioceptionSnapshot {
    joints: [JointCondition; 12],
    /// Mean flexibility across all 12 joints — a real summary quantity a
    /// future mobility/labor-capacity check could read without iterating
    /// every joint individually.
    pub overall_flexibility: f64,
    /// Mean stress across all 12 joints.
    pub overall_joint_stress: f64,
    /// This tick's real whole-body pose (see module docs). `#[serde(default)]`
    /// so pre-existing saves deserialize as resting rather than failing.
    #[serde(default = "BodyPose::resting")]
    pub pose: BodyPose,
}

impl ProprioceptionSnapshot {
    /// Per-joint flexibility/stress from canon
    /// `sensory_systems.proprioception_system.body_schema`. An unfilled
    /// (zero) flexibility takes the engine default; zero stress is a
    /// legitimate unstressed joint.
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let mut snapshot = Self::defaults();
        let Some(schema) = profile.canonical_schema() else {
            return snapshot;
        };
        let body = &schema.sensory_systems.proprioception_system.body_schema;
        let canon_joints = [
            &body.left_arm.shoulder,
            &body.left_arm.elbow,
            &body.left_arm.wrist,
            &body.right_arm.shoulder,
            &body.right_arm.elbow,
            &body.right_arm.wrist,
            &body.left_leg.hip,
            &body.left_leg.knee,
            &body.left_leg.ankle,
            &body.right_leg.hip,
            &body.right_leg.knee,
            &body.right_leg.ankle,
        ];
        for (condition, canon) in snapshot.joints.iter_mut().zip(canon_joints) {
            if canon.flexibility > 0.0 {
                condition.flexibility = (canon.flexibility as f64).clamp(0.0, 1.0);
            }
            condition.stress = (canon.stress as f64).clamp(0.0, 1.0);
        }
        let count = snapshot.joints.len() as f64;
        snapshot.overall_flexibility =
            snapshot.joints.iter().map(|j| j.flexibility).sum::<f64>() / count;
        snapshot.overall_joint_stress =
            snapshot.joints.iter().map(|j| j.stress).sum::<f64>() / count;
        snapshot
    }

    /// Age-related stiffening, fatigue-driven stress accumulation
    /// (weighted by each joint's real distal/proximal sensitivity), and
    /// inflammation-driven flexibility loss — all genuine physiological
    /// couplings to signals this engine already steps each tick.
    /// `movement_delta` is the real `(row, col)` change in this human's
    /// `GridPosition` this tick, diffed by the caller around the real
    /// [`super::autonomy::AutonomousMind::apply_movement`] call — see
    /// module docs' "Whole-body pose" section.
    pub fn step(
        &self,
        needs: &NeedsSnapshot,
        immune: &ImmuneSnapshot,
        development: &DevelopmentSnapshot,
        dt_years: f64,
        movement_delta: (i32, i32),
    ) -> Self {
        let dt = dt_years.max(0.0);

        // Flexibility declines slowly with age (real: connective tissue
        // stiffens over decades) and drops faster under active
        // inflammation (real: swelling restricts range of motion).
        let age_stiffening = (development.age_years / 100.0).clamp(0.0, 0.4);
        let inflammation_stiffening = (immune.inflammation * 0.3).clamp(0.0, 0.3);

        let joints = std::array::from_fn(|i| {
            let joint = JointId::ALL[i];
            let prior = self.joints[i];
            let sensitivity = joint.distal_sensitivity();

            let flexibility_target =
                (1.0 - age_stiffening - inflammation_stiffening).clamp(0.1, 1.0);
            let flexibility = lerp(
                prior.flexibility,
                flexibility_target,
                // Joints loosen or stiffen toward their target at 0.1 per
                // hour (a time constant of about ten hours).
                super::rates::relaxation_fraction(0.1, dt, super::rates::HOUR_YEARS),
            );

            // Fatigue accumulates as joint stress (real: tired muscles
            // transfer more load to joints/tendons); rest recovers it.
            let stress_target = (needs.fatigue * sensitivity).clamp(0.0, 1.0);
            // Joint stress follows fatigue at 0.15 per hour.
            let stress_blend =
                super::rates::relaxation_fraction(0.15, dt, super::rates::HOUR_YEARS);
            let stress = lerp(prior.stress, stress_target, stress_blend);

            JointCondition {
                flexibility,
                stress,
            }
        });

        let overall_flexibility =
            joints.iter().map(|j| j.flexibility).sum::<f64>() / joints.len() as f64;
        let overall_joint_stress =
            joints.iter().map(|j| j.stress).sum::<f64>() / joints.len() as f64;

        let (row_delta, col_delta) = movement_delta;

        Self {
            joints,
            overall_flexibility,
            overall_joint_stress,
            pose: BodyPose::from_delta(row_delta, col_delta),
        }
    }

    pub fn joint(&self, id: JointId) -> JointCondition {
        self.joints[JointId::ALL.iter().position(|j| *j == id).unwrap()]
    }

    fn defaults() -> Self {
        let joints = [JointCondition {
            flexibility: 0.9,
            stress: 0.1,
        }; 12];
        Self {
            joints,
            overall_flexibility: 0.9,
            overall_joint_stress: 0.1,
            pose: BodyPose::resting(),
        }
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (a + (b - a) * t.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("proprioception_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = ProprioceptionSnapshot::from_profile(&profile());
        assert!(snapshot.overall_flexibility > 0.0);
        assert!(snapshot.overall_joint_stress < 1.0);
        assert!(!snapshot.pose.moving);
        assert_eq!(snapshot.pose.heading, None);
    }

    #[test]
    fn step_derives_real_pose_from_the_actual_position_delta() {
        let snapshot = ProprioceptionSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        let development = DevelopmentSnapshot::from_profile(&profile());

        let moved_north = snapshot.step(&needs, &immune, &development, 1.0, (-1, 0));
        assert!(moved_north.pose.moving);
        assert_eq!(moved_north.pose.heading, Some(Heading8::North));

        let moved_southeast = snapshot.step(&needs, &immune, &development, 1.0, (1, 1));
        assert!(moved_southeast.pose.moving);
        assert_eq!(moved_southeast.pose.heading, Some(Heading8::SouthEast));

        // No real displacement (didn't move, or clamped at a grid edge) ->
        // honestly reports resting rather than a fabricated last-known
        // heading.
        let stayed_put = snapshot.step(&needs, &immune, &development, 1.0, (0, 0));
        assert!(!stayed_put.pose.moving);
        assert_eq!(stayed_put.pose.heading, None);
    }

    #[test]
    fn sustained_fatigue_raises_joint_stress() {
        let snapshot = ProprioceptionSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.fatigue = 1.0;
        let immune = ImmuneSnapshot::from_profile(&profile());
        let development = DevelopmentSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(&needs, &immune, &development, 1.0, (0, 0));
        }

        assert!(stepped.overall_joint_stress > snapshot.overall_joint_stress);
    }

    #[test]
    fn distal_joints_accumulate_more_stress_than_proximal_under_fatigue() {
        let snapshot = ProprioceptionSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.fatigue = 1.0;
        let immune = ImmuneSnapshot::from_profile(&profile());
        let development = DevelopmentSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(&needs, &immune, &development, 1.0, (0, 0));
        }

        assert!(
            stepped.joint(JointId::LeftWrist).stress > stepped.joint(JointId::LeftShoulder).stress
        );
    }

    #[test]
    fn inflammation_reduces_flexibility() {
        let snapshot = ProprioceptionSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let mut inflamed_immune = ImmuneSnapshot::from_profile(&profile());
        inflamed_immune.inflammation = 1.0;
        let healthy_immune = ImmuneSnapshot::from_profile(&profile());
        let development = DevelopmentSnapshot::from_profile(&profile());

        let mut inflamed = snapshot.clone();
        let mut healthy = snapshot.clone();
        for _ in 0..10 {
            inflamed = inflamed.step(&needs, &inflamed_immune, &development, 1.0, (0, 0));
            healthy = healthy.step(&needs, &healthy_immune, &development, 1.0, (0, 0));
        }

        assert!(inflamed.overall_flexibility < healthy.overall_flexibility);
    }

    #[test]
    fn older_age_reduces_flexibility() {
        let snapshot = ProprioceptionSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        let mut old = DevelopmentSnapshot::from_profile(&profile());
        old.age_years = 90.0;
        let young = DevelopmentSnapshot::from_profile(&profile());

        let mut old_stepped = snapshot.clone();
        let mut young_stepped = snapshot.clone();
        for _ in 0..10 {
            old_stepped = old_stepped.step(&needs, &immune, &old, 1.0, (0, 0));
            young_stepped = young_stepped.step(&needs, &immune, &young, 1.0, (0, 0));
        }

        assert!(old_stepped.overall_flexibility < young_stepped.overall_flexibility);
    }

    #[test]
    fn canon_joint_condition_is_read() {
        let mut schema = mk_core::human::HumanSchema::canonical_minimal("stiff_knee");
        let knee = &mut schema
            .sensory_systems
            .proprioception_system
            .body_schema
            .left_leg
            .knee;
        knee.flexibility = 0.3;
        knee.stress = 0.6;
        let profile = HumanProfile::from_canonical_schema(mk_core::human::HumanId::new(9), schema);

        let snapshot = ProprioceptionSnapshot::from_profile(&profile);
        let knee = snapshot.joint(JointId::LeftKnee);
        assert!((knee.flexibility - 0.3).abs() < 1e-6);
        assert!((knee.stress - 0.6).abs() < 1e-6);
        assert!(snapshot.overall_flexibility < 0.9);
    }
}
