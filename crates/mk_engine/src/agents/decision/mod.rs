use serde::{Deserialize, Serialize};

use crate::agents::{
    affect::AffectSnapshot,
    behavior::{ActionKind, AgentAction, NeedsVector},
    cognition::{CognitiveSnapshot, IntentFocus},
    endocrinology::EndocrineSnapshot,
    memory::MemorySystem,
    physiology::PhysiologySnapshot,
    will::WillSnapshot,
    AgentWorldObservation,
};

/// A single candidate action evaluated by the decision engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateAction {
    pub kind: ActionKind,
    pub expected_value: f64,
    pub urgency: f64,
    pub selected: bool,
}

/// Full audit record of a decision cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionAudit {
    pub tick: u64,
    pub intent: IntentFocus,
    pub candidates: Vec<CandidateAction>,
    pub chosen: ActionKind,
    pub intensity: f64,
    pub reasoning: String,
    pub will_ok: bool,
}

impl DecisionAudit {
    pub fn new(
        tick: u64,
        intent: IntentFocus,
        chosen: ActionKind,
        intensity: f64,
        reasoning: String,
        will_ok: bool,
        candidates: Vec<CandidateAction>,
    ) -> Self {
        Self {
            tick,
            intent,
            candidates,
            chosen,
            intensity,
            reasoning,
            will_ok,
        }
    }
}

/// The decision engine that selects actions based on all agent subsystems
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionEngine {
    pub last_audit: Option<DecisionAudit>,
    pub override_intent: Option<IntentFocus>,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluate all possible actions and select the best one
    // Same tradeoff as `agents::behavior::BehaviorSnapshot::step` — one
    // typed subsystem snapshot per parameter, mirroring the agent
    // pipeline's shape rather than adding an indirection layer.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        &self,
        cognition: &CognitiveSnapshot,
        affect: &AffectSnapshot,
        physiology: &PhysiologySnapshot,
        endocrinology: &EndocrineSnapshot,
        will: &WillSnapshot,
        needs: &NeedsVector,
        memory: &MemorySystem,
        observation: &AgentWorldObservation,
    ) -> (AgentAction, DecisionAudit) {
        let intent = self.override_intent.unwrap_or(cognition.intent);
        let tick = observation.tick;
        // Learned biome affordances are keyed by biome name.
        let biome_name = observation.biome_type.map(|biome| biome.name());

        let mut candidates: Vec<CandidateAction> = Vec::new();

        // Build candidate actions based on intent and state
        let primary_action = Self::action_for_intent(intent, needs, observation);
        candidates.push(CandidateAction {
            kind: primary_action,
            expected_value: Self::expected_value(
                primary_action,
                needs,
                endocrinology,
                memory,
                biome_name,
            ),
            urgency: affect.urgency,
            selected: false,
        });

        // Add fallback actions
        for &fallback in &[ActionKind::Rest, ActionKind::Explore, ActionKind::Idle] {
            if fallback != primary_action {
                candidates.push(CandidateAction {
                    kind: fallback,
                    expected_value: Self::expected_value(
                        fallback,
                        needs,
                        endocrinology,
                        memory,
                        biome_name,
                    ),
                    urgency: affect.urgency * 0.5,
                    selected: false,
                });
            }
        }

        // Apply will gate: if exhausted, force rest
        let will_ok = will.can_act();
        let chosen = if !will_ok {
            ActionKind::Rest
        } else {
            // Select best candidate by expected value
            candidates
                .iter()
                .max_by(|a, b| {
                    a.expected_value
                        .partial_cmp(&b.expected_value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|c| c.kind)
                .unwrap_or(ActionKind::Explore)
        };

        // Mark selected candidate
        for candidate in &mut candidates {
            if candidate.kind == chosen {
                candidate.selected = true;
            }
        }

        // Calculate intensity
        let intensity = (affect.urgency * 0.4
            + (1.0 - physiology.vitality_score()) * 0.2
            + will.effectiveness_multiplier() * 0.3
            + endocrinology.drive_urgency() * 0.1)
            .clamp(0.0, 1.0);

        let reasoning = format!(
            "intent={:?} will={:?} best={:?} urgency={:.2}",
            intent, will.status, chosen, affect.urgency
        );

        let audit = DecisionAudit::new(
            tick, intent, chosen, intensity, reasoning, will_ok, candidates,
        );

        (
            AgentAction {
                kind: chosen,
                intensity,
            },
            audit,
        )
    }

    fn action_for_intent(
        intent: IntentFocus,
        needs: &NeedsVector,
        _observation: &AgentWorldObservation,
    ) -> ActionKind {
        match intent {
            IntentFocus::Recover => ActionKind::Rest,
            IntentFocus::Feed => {
                if needs.hunger > 0.7 {
                    ActionKind::SeekFood
                } else {
                    ActionKind::Gather
                }
            }
            IntentFocus::Hydrate => {
                if needs.thirst > 0.7 {
                    ActionKind::SeekWater
                } else {
                    ActionKind::Gather
                }
            }
            IntentFocus::Shelter => ActionKind::SeekShelter,
            IntentFocus::Socialize => ActionKind::SocialApproach,
            IntentFocus::Explore => {
                if needs.curiosity > 0.7 {
                    ActionKind::Mine
                } else if needs.hunger > 0.5 || needs.thirst > 0.5 {
                    ActionKind::Gather
                } else {
                    ActionKind::Explore
                }
            }
        }
    }

    fn expected_value(
        action: ActionKind,
        needs: &NeedsVector,
        endocrinology: &EndocrineSnapshot,
        memory: &MemorySystem,
        // Biome the agent is currently in, for consulting learned
        // affordances. `None` when the observation has no biome.
        biome_name: Option<&str>,
    ) -> f64 {
        let base = match action {
            ActionKind::Rest => needs.rest * 0.3 + (1.0 - endocrinology.drive_urgency()) * 0.2,
            ActionKind::SeekFood => needs.hunger * 0.8,
            ActionKind::SeekWater => needs.thirst * 0.8,
            ActionKind::SeekShelter => needs.shelter * 0.7,
            ActionKind::Gather => (needs.hunger + needs.thirst) * 0.3 + needs.curiosity * 0.2,
            ActionKind::Mine => needs.curiosity * 0.6,
            ActionKind::SocialApproach => needs.social * 0.6,
            ActionKind::Explore => needs.curiosity * 0.4,
            ActionKind::Move => needs.curiosity * 0.3,
            ActionKind::Build => needs.shelter * 0.2 + needs.social * 0.2,
            ActionKind::Craft => needs.curiosity * 0.3,
            ActionKind::Transfer => needs.social * 0.2,
            ActionKind::Idle => 0.1,
            // Harm and Code are human-specific mechanics (dark-triad-driven
            // conflict, and Computer Room invention work — see
            // `crate::humans::autonomy::drive_for`). Generic non-human
            // agents never have a reason to select either.
            ActionKind::Harm => 0.0,
            ActionKind::Code => 0.0,
            ActionKind::Intimacy => 0.0,
            // WebSearch/SendEmail are human-specific (real
            // `humans::computer_bridge::ComputerBridge` access) — see
            // `crate::humans::autonomy::drive_for`. Generic non-human
            // agents never have a reason to select either.
            ActionKind::WebSearch => 0.0,
            ActionKind::SendEmail => 0.0,
        };

        // Boost by relevant procedural skill proficiency
        let skill_boost = memory.skill_proficiency(match action {
            ActionKind::Gather | ActionKind::SeekFood | ActionKind::SeekWater => "foraging",
            ActionKind::Mine => "mining",
            ActionKind::Build => "building",
            ActionKind::Craft => "crafting",
            _ => "walking",
        }) * 0.2;

        // Boost by what the agent has *learned* about the biome it is in.
        // Weighted below the skill boost: knowing a place has water is worth
        // less than being good at finding it.
        let knowledge_boost = match (biome_name, action) {
            (Some(biome), ActionKind::SeekFood) => memory.affordance_confidence(biome, "food"),
            (Some(biome), ActionKind::SeekWater) => memory.affordance_confidence(biome, "water"),
            (Some(biome), ActionKind::SeekShelter) => {
                memory.affordance_confidence(biome, "shelter")
            }
            _ => 0.0,
        } * 0.1;

        (base + skill_boost + knowledge_boost).clamp(0.0, 1.0)
    }
}
