//! Phase 10 — Agents (MK-II)
//!
//! Purpose
//! - Introduce explicit, auditable, mortal agents above the MK-I world stack.
//! - Preserve lower-layer sovereignty by consuming world observations rather than mutating
//!   lower physical or biosphere layers directly.
//! - Keep agent stepping deterministic and replay-stable.

pub mod affect;
pub mod behavior;
pub mod cognition;
pub mod decision;
pub mod endocrinology;
pub mod memory;
pub mod mortality;
pub mod nervous_system;
pub mod physiology;
pub mod will;

use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

pub use affect::{AffectSnapshot, EmotionalValence};
pub use behavior::{ActionKind, AgentAction, BehaviorSnapshot, NeedsVector};
pub use cognition::{CognitiveSnapshot, IntentFocus};
pub use decision::{DecisionAudit, DecisionEngine};
pub use endocrinology::{EndocrineSnapshot, HormoneAxis};
pub use memory::MemorySystem;
pub use mortality::{MortalityReason, MortalitySnapshot};
pub use nervous_system::{ArousalBand, NervousSystemSnapshot};
pub use physiology::{PhysiologySnapshot, VitalStatus};
pub use will::{WillSnapshot, WillStatus};

/// Grid position for agents on the world grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPosition {
    pub row: i32,
    pub col: i32,
}

impl GridPosition {
    pub fn new(row: i32, col: i32) -> Self {
        Self { row, col }
    }
}

/// Resource/item types agents can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Food,
    Water,
    Wood,
    Stone,
    MetalOre,
    Fiber,
    Herbal,
    Mineral,
    Tool,
    Weapon,
    Structure,
    Clay,
    Sand,
    Planks,
    Masonry,
    Rope,
    Fuel,
    IronIngot,
    CopperIngot,
    IronOre,
    CopperOre,
    Coal,
}

/// A stack of a single item kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub kind: ItemKind,
    pub quantity: u32,
}

impl ItemStack {
    pub fn new(kind: ItemKind, quantity: u32) -> Self {
        Self { kind, quantity }
    }
}

/// A physical container carried by a being or vehicle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarriedContainer {
    pub name: String,
    pub capacity: u32,
    pub contents: Vec<ItemStack>,
}

/// Physical carrying state. There is no hidden inventory: only two hands, a
/// mouth, and explicitly attached/carried containers can hold items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarryingState {
    pub hand_slots: [Option<ItemStack>; 2],
    pub mouth: Option<ItemStack>,
    pub containers: Vec<CarriedContainer>,
    pub held_tools: [Option<crate::resource_economy::ToolInstance>; 2],
}

impl Default for CarryingState {
    fn default() -> Self {
        Self::new()
    }
}

impl CarryingState {
    pub fn new() -> Self {
        Self {
            hand_slots: [None, None],
            mouth: None,
            containers: Vec::new(),
            held_tools: [None, None],
        }
    }

    pub fn add_container(&mut self, name: impl Into<String>, capacity: u32) {
        self.containers.push(CarriedContainer {
            name: name.into(),
            capacity,
            contents: Vec::new(),
        });
    }

    pub fn has_tool(&self, required: crate::resource_economy::ToolKind) -> bool {
        required == crate::resource_economy::ToolKind::Hands
            || self
                .held_tools
                .iter()
                .flatten()
                .any(|tool| tool.kind == required && tool.durability > 0)
    }

    pub fn add_tool(&mut self, kind: crate::resource_economy::ToolKind) -> bool {
        if let Some(slot) = self.held_tools.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(crate::resource_economy::ToolInstance {
                kind,
                durability: 100,
            });
            true
        } else {
            false
        }
    }

    pub fn use_tool(&mut self, kind: crate::resource_economy::ToolKind, quantity: u32) {
        if kind == crate::resource_economy::ToolKind::Hands {
            return;
        }
        if let Some(tool) = self
            .held_tools
            .iter_mut()
            .flatten()
            .find(|tool| tool.kind == kind && tool.durability > 0)
        {
            tool.durability = tool.durability.saturating_sub(quantity);
        }
    }

    pub fn add(&mut self, kind: ItemKind, quantity: u32) -> u32 {
        let mut remaining = quantity;
        for stack in self.hand_slots.iter_mut().flatten() {
            if stack.kind == kind {
                let accepted = remaining.min(1u32.saturating_sub(stack.quantity));
                stack.quantity += accepted;
                remaining -= accepted;
                if remaining == 0 {
                    return quantity;
                }
            }
        }
        if let Some(stack) = self.mouth.as_mut().filter(|stack| stack.kind == kind) {
            let accepted = remaining.min(1u32.saturating_sub(stack.quantity));
            stack.quantity += accepted;
            remaining -= accepted;
            if remaining == 0 {
                return quantity;
            }
        }
        for container in &mut self.containers {
            let used: u32 = container.contents.iter().map(|stack| stack.quantity).sum();
            let free = container.capacity.saturating_sub(used);
            if free == 0 {
                continue;
            }
            if let Some(stack) = container
                .contents
                .iter_mut()
                .find(|stack| stack.kind == kind)
            {
                let accepted = remaining.min(free);
                stack.quantity = stack.quantity.saturating_add(accepted);
                remaining -= accepted;
                if remaining == 0 {
                    return quantity;
                }
            }
        }
        for slot in &mut self.hand_slots {
            if slot.is_none() {
                let accepted = remaining.min(1);
                *slot = Some(ItemStack::new(kind, accepted));
                remaining -= accepted;
                if remaining == 0 {
                    return quantity;
                }
            }
        }
        if self.mouth.is_none() {
            let accepted = remaining.min(1);
            self.mouth = Some(ItemStack::new(kind, accepted));
            remaining -= accepted;
            if remaining == 0 {
                return quantity;
            }
        }
        for container in &mut self.containers {
            let used: u32 = container.contents.iter().map(|stack| stack.quantity).sum();
            let accepted = remaining.min(container.capacity.saturating_sub(used));
            if accepted > 0 {
                if let Some(stack) = container
                    .contents
                    .iter_mut()
                    .find(|stack| stack.kind == kind)
                {
                    stack.quantity += accepted;
                } else {
                    container.contents.push(ItemStack::new(kind, accepted));
                }
                remaining -= accepted;
                if remaining == 0 {
                    return quantity;
                }
            }
        }
        quantity - remaining
    }

    pub fn remove(&mut self, kind: ItemKind, quantity: u32) -> u32 {
        let mut remaining = quantity;
        for container in &mut self.containers {
            remove_from_stacks(&mut container.contents, kind, &mut remaining);
        }
        for slot in &mut self.hand_slots {
            if slot.as_ref().is_some_and(|stack| stack.kind == kind) {
                if let Some(stack) = slot.as_mut() {
                    let taken = take_stack(stack, kind, remaining);
                    remaining -= taken;
                    if stack.quantity == 0 {
                        *slot = None;
                    }
                }
            }
        }
        if let Some(stack) = self.mouth.as_mut() {
            let taken = take_stack(stack, kind, remaining);
            remaining -= taken;
            if stack.quantity == 0 {
                self.mouth = None;
            }
        }
        quantity - remaining
    }

    pub fn count(&self, kind: ItemKind) -> u32 {
        self.hand_slots
            .iter()
            .flatten()
            .chain(self.mouth.iter())
            .map(|stack| {
                if stack.kind == kind {
                    stack.quantity
                } else {
                    0
                }
            })
            .sum::<u32>()
            + self
                .containers
                .iter()
                .flat_map(|container| container.contents.iter())
                .map(|stack| {
                    if stack.kind == kind {
                        stack.quantity
                    } else {
                        0
                    }
                })
                .sum::<u32>()
    }

    /// Transfer one physically carried item into another being's carrying
    /// state using stable slot/container order.
    pub fn transfer_one_to(&mut self, recipient: &mut Self) -> bool {
        let Some(kind) = self
            .hand_slots
            .iter()
            .flatten()
            .chain(self.mouth.iter())
            .chain(
                self.containers
                    .iter()
                    .flat_map(|container| container.contents.iter()),
            )
            .map(|stack| stack.kind)
            .next()
        else {
            return false;
        };
        if recipient.add(kind, 1) != 1 {
            return false;
        }
        self.remove(kind, 1) == 1
    }

    pub fn occupied_slots(&self) -> usize {
        self.hand_slots.iter().flatten().count()
            + usize::from(self.mouth.is_some())
            + self
                .containers
                .iter()
                .map(|container| container.contents.len())
                .sum::<usize>()
    }
}

fn remove_from_stacks(stacks: &mut Vec<ItemStack>, kind: ItemKind, remaining: &mut u32) {
    for stack in stacks.iter_mut() {
        let taken = take_stack(stack, kind, *remaining);
        *remaining -= taken;
    }
    stacks.retain(|stack| stack.quantity > 0);
}

fn take_stack(stack: &mut ItemStack, kind: ItemKind, quantity: u32) -> u32 {
    if stack.kind != kind {
        return 0;
    }
    let taken = stack.quantity.min(quantity);
    stack.quantity -= taken;
    taken
}

/// Immutable world-facing observation packet consumed by agents.
///
/// This is intentionally read-only and additive. Lower layers remain sovereign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorldObservation {
    pub tick: Tick,
    pub ambient_temperature_c: f64,
    pub hydration_access: f64,
    pub caloric_access: f64,
    pub shelter_quality: f64,
    pub social_density: f64,
    pub hazard_index: f64,
    pub daylight_fraction: f64,
    pub biome_type: Option<mk_core::biomes::BiomeType>,
    pub resource_abundance: f64,
    /// 1.0 if a property with a `ComputerRoom` building stands at this
    /// human's position *and* this human holds a real `NetworkAccount` on
    /// that property, 0.0 otherwise. Per-agent, not per-position — two
    /// humans standing in the same `ComputerRoom` can observe different
    /// values if only one of them has an account there. This is what makes
    /// `NetworkAccount` (`organisms::property::NetworkAccount`) a real gate
    /// instead of unused bookkeeping.
    #[serde(default)]
    pub computer_access: f64,
    /// 1.0 if `WorldState.computer_bridge` is attached (real web
    /// search/email available this tick), 0.0 otherwise. Unlike
    /// `computer_access` this is not a per-position building check — the
    /// bridge is a per-world singleton, so every human observes the same
    /// value. `#[serde(skip)]` bridge state itself never enters this
    /// projection; only this boolean-as-f64 flag does. `ActionKind::Code`
    /// requires only `computer_access` (local hardware); `WebSearch`/
    /// `SendEmail` require both this *and* `computer_access` — real
    /// internet access still needs a real machine to sit at.
    #[serde(default)]
    pub computer_bridge_available: f64,
}

impl Default for AgentWorldObservation {
    fn default() -> Self {
        Self {
            tick: 0,
            ambient_temperature_c: 18.0,
            hydration_access: 0.75,
            caloric_access: 0.75,
            shelter_quality: 0.5,
            social_density: 0.5,
            hazard_index: 0.05,
            daylight_fraction: 0.5,
            biome_type: None,
            resource_abundance: 0.5,
            computer_access: 0.0,
            computer_bridge_available: 0.0,
        }
    }
}

/// Stable agent identifier for phase-10 runtime entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// High-level auditable agent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub age_ticks: u64,
    pub position: GridPosition,
    pub carrying: CarryingState,
    pub memory: MemorySystem,
    pub will: WillSnapshot,
    pub decision: DecisionEngine,
    pub physiology: PhysiologySnapshot,
    pub endocrinology: EndocrineSnapshot,
    pub nervous_system: NervousSystemSnapshot,
    pub cognition: CognitiveSnapshot,
    pub affect: AffectSnapshot,
    pub behavior: BehaviorSnapshot,
    pub mortality: MortalitySnapshot,
    pub last_action: AgentAction,
}

impl Agent {
    pub fn new(id: AgentId, position: GridPosition) -> Self {
        Self {
            id,
            age_ticks: 0,
            position,
            carrying: CarryingState::new(),
            memory: MemorySystem::new(),
            will: WillSnapshot::new(),
            decision: DecisionEngine::new(),
            physiology: PhysiologySnapshot::default(),
            endocrinology: EndocrineSnapshot::default(),
            nervous_system: NervousSystemSnapshot::default(),
            cognition: CognitiveSnapshot::default(),
            affect: AffectSnapshot::default(),
            behavior: BehaviorSnapshot::default(),
            mortality: MortalitySnapshot::default(),
            last_action: AgentAction::idle(),
        }
    }

    pub fn spawn_at(id: AgentId, row: i32, col: i32) -> Self {
        Self::new(id, GridPosition::new(row, col))
    }

    pub fn is_alive(&self) -> bool {
        self.mortality.is_alive()
    }

    pub fn step(
        &mut self,
        observation: &AgentWorldObservation,
        rng: &RngRegistry,
    ) -> AgentStepAudit {
        let tick = observation.tick;

        self.age_ticks = self.age_ticks.saturating_add(1);

        if !self.is_alive() {
            return AgentStepAudit::from_agent(self, tick, "agent already dead".to_string());
        }

        let action_intensity = self.last_action.intensity;

        // Layer 1: Physiology
        self.physiology = self.physiology.step(observation, action_intensity);

        // Layer 2: Endocrinology
        self.endocrinology =
            self.endocrinology
                .step(&self.physiology, observation, action_intensity);

        // Layer 3: Nervous system
        self.nervous_system =
            self.nervous_system
                .step(&self.physiology, &self.endocrinology, observation);

        // Layer 4: Cognition (uses memory)
        self.cognition =
            self.cognition
                .step(&self.nervous_system, &self.endocrinology, observation);

        // Layer 5: Affect
        self.affect = self
            .affect
            .step(&self.cognition, &self.nervous_system, &self.endocrinology);

        // Compute needs from homeostasis
        let (hunger_urgency, thirst_urgency, rest_urgency) =
            self.physiology.homeostasis.compute_needs(&self.physiology);
        let needs = NeedsVector {
            hunger: hunger_urgency.min(1.0),
            thirst: thirst_urgency.min(1.0),
            rest: rest_urgency.min(1.0),
            ..Default::default()
        };

        // Layer 6: Will (depletes/recharges based on activity)
        let is_resting = matches!(self.last_action.kind, ActionKind::Rest | ActionKind::Idle);
        self.will = self.will.step(
            action_intensity,
            self.endocrinology.stress_load(),
            is_resting,
            self.cognition.coherence,
        );

        // Layer 7: Decision engine (deterministic action selection)
        let (chosen_action, _audit) = self.decision.evaluate(
            &self.cognition,
            &self.affect,
            &self.physiology,
            &self.endocrinology,
            &self.will,
            &needs,
            &self.memory,
            observation,
        );

        // Layer 8: Behavior (stores the selected action)
        let key = RngKey::for_tick(SubsystemId::Humans, tick);
        self.behavior = self.behavior.step(
            &self.cognition,
            &self.affect,
            &self.physiology,
            &needs,
            observation,
            rng,
            key,
        );
        // Override action from decision engine
        self.behavior.action = chosen_action;
        self.last_action = self.behavior.action.clone();

        // Update memory: record this event
        self.memory.remember_event(memory::EpisodicMemoryEntry {
            tick,
            event_type: format!("{:?}", self.last_action.kind),
            context: format!("biome={:?}", observation.biome_type),
            salience: action_intensity * 0.5 + self.endocrinology.reward_response() * 0.5,
            emotional_valence: if matches!(self.affect.valence, EmotionalValence::Distressed) {
                -self.affect.urgency
            } else {
                self.affect.affiliation
            },
        });
        // Practice the skill we just used
        self.memory.practice_skill(match self.last_action.kind {
            ActionKind::Gather | ActionKind::SeekFood | ActionKind::SeekWater => "foraging",
            ActionKind::Mine => "mining",
            ActionKind::Build => "building",
            ActionKind::Craft => "crafting",
            ActionKind::Rest => "resting",
            _ => "walking",
        });
        // Learn what this biome actually affords, from the observation's own
        // real access values. This is what makes the semantic store live:
        // `learn_fact`/`recall_fact` were fully implemented but had no call
        // sites anywhere in the workspace (finding 27 of the 2026-09-18
        // audit), so nothing was ever learned or consulted.
        if let Some(biome) = observation.biome_type {
            let biome_name = biome.name();
            self.memory
                .learn_affordance(biome_name, "food", observation.caloric_access, tick);
            self.memory
                .learn_affordance(biome_name, "water", observation.hydration_access, tick);
            self.memory
                .learn_affordance(biome_name, "shelter", observation.shelter_quality, tick);
        }
        // Decay old memories
        self.memory.decay(tick);

        // Layer 9: Mortality
        self.mortality = self.mortality.step(
            &self.physiology,
            &self.endocrinology,
            observation,
            self.age_ticks,
        );

        AgentStepAudit::from_agent(self, tick, "ok".to_string())
    }
}

/// Deterministic audit output for one agent step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStepAudit {
    pub tick: Tick,
    pub agent_id: AgentId,
    pub position: GridPosition,
    pub alive: bool,
    pub vitality_score: f64,
    pub stress_score: f64,
    pub arousal_score: f64,
    pub dominant_intent: IntentFocus,
    pub emotional_valence: EmotionalValence,
    pub action: ActionKind,
    pub carrying_slot_count: usize,
    pub will_status: WillStatus,
    pub memory_count: usize,
    pub note: String,
}

impl AgentStepAudit {
    fn from_agent(agent: &Agent, tick: Tick, note: String) -> Self {
        Self {
            tick,
            agent_id: agent.id.clone(),
            position: agent.position,
            alive: agent.is_alive(),
            vitality_score: agent.physiology.vitality_score(),
            stress_score: agent.endocrinology.stress_load(),
            arousal_score: agent.nervous_system.arousal_score,
            dominant_intent: agent.cognition.intent,
            emotional_valence: agent.affect.valence,
            action: agent.behavior.action.kind,
            carrying_slot_count: agent.carrying.occupied_slots(),
            will_status: agent.will.status,
            memory_count: agent.memory.episodic.len(),
            note,
        }
    }
}

/// Multi-agent phase-10 runtime container.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSystem {
    pub agents: Vec<Agent>,
    pub last_audits: Vec<AgentStepAudit>,
}

impl AgentSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    pub fn population(&self) -> usize {
        self.agents.len()
    }

    pub fn living_population(&self) -> usize {
        self.agents.iter().filter(|agent| agent.is_alive()).count()
    }

    pub fn step_all(
        &mut self,
        observation: &AgentWorldObservation,
        rng: &RngRegistry,
    ) -> &[AgentStepAudit] {
        self.last_audits.clear();
        for agent in &mut self.agents {
            let audit = agent.step(observation, rng);
            self.last_audits.push(audit);
        }
        &self.last_audits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_step_produces_auditable_state() {
        let mut agent = Agent::spawn_at(AgentId::new("agent-1"), 10, 15);
        let rng = RngRegistry::new([7; 32]);
        let observation = AgentWorldObservation {
            tick: 12,
            ..Default::default()
        };

        let audit = agent.step(&observation, &rng);

        assert_eq!(audit.tick, 12);
        assert_eq!(audit.agent_id.0, "agent-1");
        assert_eq!(audit.position.row, 10);
        assert_eq!(audit.position.col, 15);
        assert!(audit.vitality_score >= 0.0);
    }

    #[test]
    fn agent_system_tracks_population() {
        let mut system = AgentSystem::new();
        system.register(Agent::spawn_at(AgentId::new("a"), 0, 0));
        system.register(Agent::spawn_at(AgentId::new("b"), 1, 1));

        assert_eq!(system.population(), 2);
        assert_eq!(system.living_population(), 2);
    }

    #[test]
    fn carrying_transfer_is_physical_and_capacity_aware() {
        let mut source = CarryingState::new();
        let mut recipient = CarryingState::new();
        assert_eq!(source.add(ItemKind::Food, 1), 1);
        assert!(source.transfer_one_to(&mut recipient));
        assert_eq!(source.count(ItemKind::Food), 0);
        assert_eq!(recipient.count(ItemKind::Food), 1);
    }
}
