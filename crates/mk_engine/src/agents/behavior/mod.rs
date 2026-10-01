use mk_core::rng::{RngExt, RngKey, RngRegistry};
use serde::{Deserialize, Serialize};

use crate::agents::{
    affect::AffectSnapshot,
    cognition::{CognitiveSnapshot, IntentFocus},
    physiology::PhysiologySnapshot,
    AgentWorldObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Idle,
    Rest,
    SeekWater,
    SeekFood,
    SeekShelter,
    SocialApproach,
    Explore,
    Gather,
    Mine,
    Move,
    Build,
    Craft,
    Transfer,
    /// A deliberate harmful act against the nearest living adjacent human —
    /// physical (body vitals), emotional (fear/sadness), and potentially
    /// lethal. Gated by `crate::governance::check` against the target
    /// before any effect is applied (Reverence Veto refuses harm against
    /// protected creator entities regardless of the attacker's state).
    Harm,
    /// Work at a `PropertyBuildingKind::ComputerRoom` building — feeds
    /// `humans::technology::TechnologySnapshot::accumulated_knowledge`
    /// directly, on top of its normal passive per-tick growth. Gated by
    /// `AgentWorldObservation::computer_access` (real building presence at
    /// this human's position), same affordance pattern as
    /// `SeekFood`/`SeekWater`/`SeekShelter`.
    Code,
    /// A deliberate attempt at intimacy with the nearest living adjacent
    /// human. The attempt can be made regardless of the target's real
    /// state (some humans ask/attempt anyway) — but it only *succeeds*,
    /// and only produces any satisfaction/bonding/reproduction-eligible
    /// effect, when the target's own real willingness (attraction/libido,
    /// `reproduction.rs`) clears a real threshold. A refused attempt is a
    /// real, tracked failure (raises the initiator's frustration), never a
    /// silently-succeeding one — see `crate::humans::mod::apply_intimacy`.
    Intimacy,
    /// Real internet search via `humans::computer_bridge::ComputerBridge`.
    /// Gated by `AgentWorldObservation::computer_bridge_available` (the
    /// bridge is a per-world singleton attached to `WorldState`, not a
    /// physical building like `Code`'s `computer_access`) — same
    /// affordance-gate pattern otherwise. The search query and result are
    /// not carried on this action; `computer_bridge::execute_computer_action`
    /// derives the query and stores the result on `HumanBeing` directly, the
    /// same way `Move`/`Explore` don't carry an explicit destination.
    WebSearch,
    /// Real email to a pre-configured contact (Dylan/Kirsty) via
    /// `humans::computer_bridge::ComputerBridge`. Same gating and
    /// no-payload-on-the-action pattern as `WebSearch`.
    SendEmail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub kind: ActionKind,
    pub intensity: f64,
}

impl AgentAction {
    pub fn idle() -> Self {
        Self {
            kind: ActionKind::Idle,
            intensity: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NeedsVector {
    pub hunger: f64,
    pub thirst: f64,
    pub rest: f64,
    pub shelter: f64,
    pub social: f64,
    pub curiosity: f64,
}

impl Default for NeedsVector {
    fn default() -> Self {
        Self {
            hunger: 0.2,
            thirst: 0.2,
            rest: 0.1,
            shelter: 0.3,
            social: 0.3,
            curiosity: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSnapshot {
    pub action: AgentAction,
    pub compliance: f64,
    pub needs: NeedsVector,
}

impl Default for BehaviorSnapshot {
    fn default() -> Self {
        Self {
            action: AgentAction::idle(),
            compliance: 1.0,
            needs: NeedsVector::default(),
        }
    }
}

impl BehaviorSnapshot {
    // One typed subsystem snapshot per parameter (cognition/affect/
    // physiology/needs/observation/rng) — a params struct would just move
    // the same fields one layer out with no behavior change, and this
    // mirrors the shape of every other subsystem `step()` in the agent
    // pipeline (see `agents::decision::DecisionEngine::evaluate`).
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        cognition: &CognitiveSnapshot,
        affect: &AffectSnapshot,
        physiology: &PhysiologySnapshot,
        needs: &NeedsVector,
        observation: &AgentWorldObservation,
        rng_registry: &RngRegistry,
        key: RngKey,
    ) -> Self {
        let mut rng = rng_registry.stream(key);
        let noise = rng.gen_f64_01() * 0.05;
        let intensity = (affect.urgency * 0.6 + (1.0 - physiology.vitality_score()) * 0.25 + noise)
            .clamp(0.0, 1.0);

        let kind = match cognition.intent {
            IntentFocus::Recover => ActionKind::Rest,
            IntentFocus::Feed => ActionKind::SeekFood,
            IntentFocus::Hydrate => ActionKind::SeekWater,
            IntentFocus::Shelter => ActionKind::SeekShelter,
            IntentFocus::Socialize => ActionKind::SocialApproach,
            IntentFocus::Explore => {
                if needs.hunger > 0.6 || needs.thirst > 0.6 {
                    ActionKind::Gather
                } else if needs.curiosity > 0.7 {
                    ActionKind::Mine
                } else if observation.hazard_index > 0.5 {
                    ActionKind::SeekShelter
                } else {
                    ActionKind::Explore
                }
            }
        };

        Self {
            action: AgentAction { kind, intensity },
            compliance: cognition.coherence.clamp(0.0, 1.0),
            needs: *needs,
        }
    }
}
