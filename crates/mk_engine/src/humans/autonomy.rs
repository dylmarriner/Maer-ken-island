//! Persistent per-human action selection.
//!
//! This is a small embodied learning controller, not a pretrained model.
//! Every human owns its own value estimates and experience history. The
//! world supplies observations and consequences; the controller chooses an
//! action from the engine's currently executable action space.

use crate::agents::{ActionKind, AgentAction, AgentWorldObservation, CarryingState, ItemKind};
use crate::humans::core_systems::CoreSystemsSnapshot;
use crate::humans::dark_triad::DarkTriadSnapshot;
use crate::humans::genetics::GeneticsSnapshot;
use crate::humans::needs::NeedsSnapshot;
use crate::humans::neurochemistry::{BrainState, NeurochemistrySnapshot};
use crate::humans::reproduction::ReproductiveSystemSnapshot;
use mk_core::rng::{RngExt, RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const ACTIONS: [ActionKind; 18] = [
    ActionKind::Idle,
    ActionKind::Rest,
    ActionKind::SeekWater,
    ActionKind::SeekFood,
    ActionKind::SeekShelter,
    ActionKind::SocialApproach,
    ActionKind::Explore,
    ActionKind::Gather,
    ActionKind::Mine,
    ActionKind::Move,
    ActionKind::Build,
    ActionKind::Craft,
    ActionKind::Transfer,
    ActionKind::Harm,
    ActionKind::Code,
    ActionKind::Intimacy,
    ActionKind::WebSearch,
    ActionKind::SendEmail,
];
const MAX_EXPERIENCES: usize = 2_000;
const AUTONOMY_EPOCH: u32 = 0x4155_544F;

/// The 8 grid neighbours a human can step to in one tick.
const NEIGHBOUR_OFFSETS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

/// Local access (0..1) to a sought resource at which a human uses what is
/// here instead of moving on to look for more. Matches the level at which
/// `needs::NeedsSnapshot::step` is calibrated to keep reserves topped up
/// indefinitely ("abundant" access, >~0.8): below it, staying put means
/// slowly running down, so the human keeps looking.
pub const SEEK_SATISFIED_ACCESS: f64 = 0.8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionPreference {
    pub action: ActionKind,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomousExperience {
    pub tick: u64,
    pub action: ActionKind,
    pub reward: f64,
    pub fitness_after: f64,
    #[serde(default = "default_action_success")]
    pub action_success: bool,
    #[serde(default)]
    pub caloric_access: f64,
    #[serde(default)]
    pub hydration_access: f64,
    #[serde(default)]
    pub shelter_quality: f64,
    #[serde(default)]
    pub hazard_index: f64,
    #[serde(default)]
    pub resource_abundance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousMind {
    pub preferences: Vec<ActionPreference>,
    pub last_action: AgentAction,
    pub last_fitness: f64,
    pub has_last_fitness: bool,
    pub total_reward: f64,
    pub experiences: VecDeque<AutonomousExperience>,
    pub learning_rate: f64,
    pub exploration_rate: f64,
}

impl Default for AutonomousMind {
    fn default() -> Self {
        Self {
            preferences: ACTIONS
                .into_iter()
                .map(|action| ActionPreference { action, value: 0.0 })
                .collect(),
            last_action: AgentAction::idle(),
            last_fitness: 0.0,
            has_last_fitness: false,
            total_reward: 0.0,
            experiences: VecDeque::new(),
            learning_rate: 0.2,
            exploration_rate: 0.15,
        }
    }
}

impl AutonomousMind {
    #[allow(clippy::too_many_arguments)]
    pub fn choose_action(
        &mut self,
        needs: &NeedsSnapshot,
        observation: &AgentWorldObservation,
        genetics: &GeneticsSnapshot,
        carrying: &CarryingState,
        dark_triad: &DarkTriadSnapshot,
        core_systems: &CoreSystemsSnapshot,
        reproduction: &ReproductiveSystemSnapshot,
        neurochemistry: &NeurochemistrySnapshot,
        human_id: u64,
        tick: u64,
        rng_registry: &RngRegistry,
    ) -> AgentAction {
        // Discrete sleep/wake gate: once the real melatonin-driven brain
        // state (`neurochemistry.rs`) crosses into `Fatigued`, sleep wins
        // over deliberation outright rather than just nudging `Rest`'s
        // score up alongside every other drive — a genuinely asleep human
        // does not weigh options. `neurochemistry` lags the current tick by
        // one step (see call site), same lag already tolerated elsewhere
        // in this pipeline.
        if neurochemistry.modulation_effects.current_brain_state == BrainState::Fatigued {
            let action = AgentAction {
                kind: ActionKind::Rest,
                intensity: intensity_for(ActionKind::Rest, needs, dark_triad, reproduction),
            };
            self.last_action = action.clone();
            return action;
        }

        let mut rng = rng_registry.stream(RngKey::new(
            SubsystemId::Humans,
            human_id as u32,
            AUTONOMY_EPOCH,
            tick,
        ));
        let mut selected = ActionKind::Idle;
        let mut best_score = f64::NEG_INFINITY;
        for action in ACTIONS {
            // Hard gate, not just a zeroed drive score: without it, a large
            // enough learned preference or exploration-noise draw could
            // still pick `Code`/`WebSearch`/`SendEmail` for a human who
            // isn't physically at an in-world computer with their own
            // network account (`AgentWorldObservation::computer_access`).
            if matches!(
                action,
                ActionKind::Code | ActionKind::WebSearch | ActionKind::SendEmail
            ) && observation.computer_access <= 0.0
            {
                continue;
            }
            // What was learned about an action that serves a need is worth
            // only as much as the need is felt now (alliesthesia; see
            // [`incentive`]): without it a learned taste for eating outbids
            // everything else even on a full stomach.
            let learned = (self.preference(action) + self.context_preference(action, observation))
                * incentive(action, needs);
            let score = learned
                + drive_for(
                    action,
                    needs,
                    observation,
                    genetics,
                    carrying,
                    dark_triad,
                    core_systems,
                    reproduction,
                )
                + (rng.gen_f64_01() - 0.5) * self.exploration_rate;
            if score > best_score {
                best_score = score;
                selected = action;
            }
        }
        let action = AgentAction {
            kind: selected,
            intensity: intensity_for(selected, needs, dark_triad, reproduction),
        };
        self.last_action = action.clone();
        action
    }

    pub fn learn_from_outcome(
        &mut self,
        needs: &NeedsSnapshot,
        observation: &AgentWorldObservation,
        tick: u64,
        action_success: bool,
    ) {
        let fitness_after = fitness(needs);
        if !self.has_last_fitness {
            self.last_fitness = fitness_after;
            self.has_last_fitness = true;
            return;
        }
        let reward = fitness_after - self.last_fitness + if action_success { 0.0 } else { -0.1 };
        self.total_reward += reward;
        if let Some(preference) = self
            .preferences
            .iter_mut()
            .find(|preference| preference.action == self.last_action.kind)
        {
            preference.value = (preference.value + self.learning_rate * reward).clamp(-1.0, 1.0);
        }
        self.experiences.push_back(AutonomousExperience {
            tick,
            action: self.last_action.kind,
            reward,
            fitness_after,
            action_success,
            caloric_access: observation.caloric_access,
            hydration_access: observation.hydration_access,
            shelter_quality: observation.shelter_quality,
            hazard_index: observation.hazard_index,
            resource_abundance: observation.resource_abundance,
        });
        if self.experiences.len() > MAX_EXPERIENCES {
            self.experiences.pop_front();
        }
        self.last_fitness = fitness_after;
    }

    /// Apply the physical consequence of an exploratory action. Movement is
    /// bounded by the authoritative world grid and uses a separate keyed
    /// stream, so replay does not depend on call order.
    pub fn apply_movement(
        &self,
        position: &mut crate::agents::GridPosition,
        topology: &crate::topology::GridTopology,
        human_id: u64,
        tick: u64,
        rng_registry: &RngRegistry,
    ) {
        if !matches!(
            self.last_action.kind,
            ActionKind::Move | ActionKind::Explore | ActionKind::SocialApproach
        ) {
            return;
        }
        let mut rng = rng_registry.stream(RngKey::new(
            SubsystemId::Humans,
            human_id as u32,
            AUTONOMY_EPOCH.wrapping_add(1),
            tick,
        ));
        let direction = rng.gen_i64_range(0, 7);
        let (row_delta, col_delta) = NEIGHBOUR_OFFSETS[direction as usize];
        // Longitude wraps around the planet and stops at an island's edge;
        // latitude stops at the poles and at the edge.
        position.row = topology.clamp_row(position.row + row_delta) as i32;
        position.col = topology.resolve_col(position.col + col_delta) as i32;
    }

    /// Directed search for the resource a `SeekFood`/`SeekWater`/
    /// `SeekShelter` action is after. When the current cell already offers
    /// at least [`SEEK_SATISFIED_ACCESS`] of it the human stays put and uses
    /// it; otherwise they step to whichever of the 8 neighbouring cells
    /// perceptibly offers the most of it. On a plateau where no neighbour is
    /// better (a featureless ice sheet or desert), they keep searching with a
    /// keyed random step rather than waiting in place. Returns the new
    /// position, or `None` when the action is not a seek or no step is taken.
    #[allow(clippy::too_many_arguments)]
    pub fn directed_seek_step(
        &self,
        here: &AgentWorldObservation,
        position: crate::agents::GridPosition,
        topology: &crate::topology::GridTopology,
        human_id: u64,
        tick: u64,
        rng_registry: &RngRegistry,
        observe: impl Fn(&crate::agents::GridPosition) -> AgentWorldObservation,
    ) -> Option<crate::agents::GridPosition> {
        let access = |observation: &AgentWorldObservation| match self.last_action.kind {
            ActionKind::SeekFood => Some(observation.caloric_access),
            ActionKind::SeekWater => Some(observation.hydration_access),
            ActionKind::SeekShelter => Some(observation.shelter_quality),
            _ => None,
        };
        let here_access = access(here)?;
        if here_access >= SEEK_SATISFIED_ACCESS {
            return None;
        }
        let neighbours: Vec<crate::agents::GridPosition> = NEIGHBOUR_OFFSETS
            .iter()
            .map(|(row_delta, col_delta)| crate::agents::GridPosition {
                row: topology.clamp_row(position.row + row_delta) as i32,
                col: topology.resolve_col(position.col + col_delta) as i32,
            })
            .filter(|candidate| *candidate != position)
            .collect();
        let best = neighbours
            .iter()
            .filter_map(|candidate| access(&observe(candidate)).map(|value| (*candidate, value)))
            .filter(|(_, value)| *value > here_access)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(candidate, _)| candidate);
        if best.is_some() || neighbours.is_empty() {
            return best;
        }
        let pick = rng_registry.gen_f64_01(RngKey::new(
            SubsystemId::Humans,
            human_id as u32,
            AUTONOMY_EPOCH.wrapping_add(2),
            tick,
        ));
        let index = ((pick * neighbours.len() as f64) as usize).min(neighbours.len() - 1);
        Some(neighbours[index])
    }

    /// Whether the current action has enough local environmental support to
    /// count as successful. Material actions also require the economy
    /// executor's stronger success result.
    pub fn environment_supports_action(&self, observation: &AgentWorldObservation) -> bool {
        match self.last_action.kind {
            ActionKind::SeekFood => observation.caloric_access > 0.0,
            ActionKind::SeekWater => observation.hydration_access > 0.0,
            ActionKind::SeekShelter => observation.shelter_quality > 0.0,
            ActionKind::SocialApproach => observation.social_density > 0.0,
            ActionKind::Explore => observation.hazard_index < 1.0,
            ActionKind::Harm => observation.social_density > 0.0,
            ActionKind::Code => observation.computer_access > 0.0,
            ActionKind::WebSearch | ActionKind::SendEmail => {
                observation.computer_access > 0.0 && observation.computer_bridge_available > 0.0
            }
            // Someone must be nearby to attempt with at all — whether the
            // attempt actually *succeeds* is a separate, stronger check
            // (the target's own real willingness) resolved in
            // `HumanSystem::step`'s intimacy-resolution pass, the same
            // split `Harm` uses between "a target exists" and "governance
            // allows it."
            ActionKind::Intimacy => observation.social_density > 0.0,
            _ => true,
        }
    }

    fn preference(&self, action: ActionKind) -> f64 {
        self.preferences
            .iter()
            .find(|preference| preference.action == action)
            .map_or(0.0, |preference| preference.value)
    }

    // Kernel-weighted recall: every remembered experience of this action
    // votes with weight 1 / (1 + distance) between its context and now. The
    // experience memory is bounded at 2,000 entries, so a linear scan is
    // O(2,000) per decision and needs no spatial index.
    fn context_preference(&self, action: ActionKind, observation: &AgentWorldObservation) -> f64 {
        let mut weighted_reward = 0.0;
        let mut weight_total = 0.0;
        for experience in self.experiences.iter().filter(|e| e.action == action) {
            let distance = (experience.caloric_access - observation.caloric_access).abs()
                + (experience.hydration_access - observation.hydration_access).abs()
                + (experience.shelter_quality - observation.shelter_quality).abs()
                + (experience.hazard_index - observation.hazard_index).abs()
                + (experience.resource_abundance - observation.resource_abundance).abs();
            let weight = 1.0 / (1.0 + distance);
            weighted_reward += experience.reward * weight;
            weight_total += weight;
        }
        if weight_total == 0.0 {
            0.0
        } else {
            (weighted_reward / weight_total).clamp(-1.0, 1.0)
        }
    }
}

fn default_action_success() -> bool {
    true
}

/// How much of what has been learned about `action` the body's present
/// state lets count. Eating and drinking are rewarding in proportion to the
/// hunger or thirst they relieve: the pleasantness of food and drink
/// follows internal state, so the same stimulus a hungry person finds good
/// is indifferent or unpleasant to a sated one (alliesthesia; Cabanac 1971,
/// *Science* 173:1103-1107). Every other action keeps its learned value
/// whole.
fn incentive(action: ActionKind, needs: &NeedsSnapshot) -> f64 {
    match action {
        ActionKind::SeekFood => needs.hunger,
        ActionKind::SeekWater => needs.thirst,
        _ => 1.0,
    }
    .clamp(0.0, 1.0)
}

fn fitness(needs: &NeedsSnapshot) -> f64 {
    needs.glucose * 0.45 + needs.hydration * 0.35 + (1.0 - needs.fatigue) * 0.20
}

#[allow(clippy::too_many_arguments)]
fn drive_for(
    action: ActionKind,
    needs: &NeedsSnapshot,
    observation: &AgentWorldObservation,
    genetics: &GeneticsSnapshot,
    carrying: &CarryingState,
    dark_triad: &DarkTriadSnapshot,
    core_systems: &CoreSystemsSnapshot,
    reproduction: &ReproductiveSystemSnapshot,
) -> f64 {
    // The `willsys` gate: an urge only pushes action selection once it has
    // genuinely reached cognition this tick (`core_systems.rs`'s real
    // chaos-perturbed, willpower-gated urge pipeline) — not just background
    // pressure. Previously computed every tick and read by nothing; this is
    // its first real consumer.
    let willed_urge = |kind_score: f64| -> f64 {
        if core_systems.willpower_available {
            kind_score
        } else {
            0.0
        }
    };

    match action {
        ActionKind::SeekFood => needs.hunger * (0.6 + observation.caloric_access * 0.4),
        ActionKind::SeekWater => needs.thirst * (0.6 + observation.hydration_access * 0.4),
        ActionKind::SeekShelter => {
            needs.fatigue * 0.35 + (1.0 - observation.shelter_quality) * 0.65
        }
        ActionKind::Rest => needs.fatigue * 0.9,
        ActionKind::SocialApproach => {
            observation.social_density * 0.6
                + genetics.extraversion * 0.35
                + willed_urge(core_systems.chaotic_urges.social) * 0.3
        }
        ActionKind::Explore => {
            observation.hazard_index.mul_add(-0.3, 0.4)
                + genetics.novelty_seek * 0.35
                + willed_urge(core_systems.chaotic_urges.exploration) * 0.3
        }
        ActionKind::Gather => {
            needs.hunger * 0.25 + needs.thirst * 0.25 + observation.resource_abundance * 0.25
        }
        ActionKind::Mine => observation.resource_abundance * 0.4,
        ActionKind::Craft => observation.shelter_quality * 0.2 + material_readiness(carrying) * 0.4,
        ActionKind::Build => observation.shelter_quality * 0.2 + building_readiness(carrying) * 0.6,
        ActionKind::Move => observation.social_density * 0.2,
        ActionKind::Transfer => observation.social_density * 0.2,
        ActionKind::Idle => 0.05,
        // Driven by this human's own real-time malice/vengeance (dark_triad.rs
        // couples both to hate/anger/resentment/humiliation, gated by
        // remorse deficit), and requires someone nearby to act on. Currently
        // rare in practice because most of the emotions that feed
        // active_malice/vengeance_drive have no real driver of their own
        // yet — see `docs/plans` human-consciousness audit's "23 undriven
        // emotions" finding. This is intentional: Harm should stay rare
        // until that gap closes, not be artificially boosted here to
        // compensate.
        ActionKind::Harm => {
            observation.social_density
                * (dark_triad.active_malice * 0.6 + dark_triad.vengeance_drive * 0.4)
        }
        // Requires a real Computer Room at this human's position
        // (`observation.computer_access`), driven by the same willed
        // achievement urge that would also apply to a future "invent"/
        // "build" ambition — genuinely gated by `willsys`'s gateway, not
        // always-on busywork.
        ActionKind::Code => {
            observation.computer_access
                * (0.3 + willed_urge(core_systems.chaotic_urges.achievement))
        }
        // Driven by this human's own real libido/attraction — some humans
        // attempt this "just cause," some from real attraction, some
        // wanting closeness, some for reproduction (the willed
        // reproduction urge term covers that last case) — the attempt
        // itself does not require the *target's* consent to be scored;
        // only the resolution in `HumanSystem::step` checks the target's
        // real willingness before anything succeeds. Requires someone
        // nearby to attempt with — enforced by the hard feasibility gate
        // (`social_density > 0`), not by scaling desire with crowd size: a
        // couple alone together wants closeness no less than one in a crowd.
        // Libido is the trait-level desire, arousal the momentary state
        // (stepped in `reproduction`, suppressed by exhaustion); both drive it.
        // Satiety holds it down after an act: desire is the drive less what
        // the last act still satisfies, which wears off over days. Without
        // it a willing couple chose intimacy more than half of every waking
        // minute, where real couples manage about once a week.
        ActionKind::Intimacy => {
            (reproduction.libido * 0.5
                + reproduction.arousal * 0.3
                + reproduction.attraction_average * 0.3
                + willed_urge(core_systems.chaotic_urges.reproduction) * 0.2)
                * (1.0 - reproduction.satisfaction.clamp(0.0, 1.0))
        }
        // Curiosity proxy: same `exploration` willed urge and
        // `novelty_seek` trait that drive physical `Explore`, gated by
        // both a real machine (`computer_access` — this human's own
        // `NetworkAccount` on a `ComputerRoom` property) and the bridge
        // being configured (`computer_bridge_available`, a per-world
        // singleton) — see `ActionKind::WebSearch` doc.
        ActionKind::WebSearch => {
            observation.computer_access
                * observation.computer_bridge_available
                * (genetics.novelty_seek * 0.4
                    + willed_urge(core_systems.chaotic_urges.exploration) * 0.6)
        }
        // Bonding proxy: same `social` willed urge that drives
        // `SocialApproach`, applied to a real remote contact instead of a
        // nearby human. Gated the same way as `WebSearch`.
        ActionKind::SendEmail => {
            observation.computer_access
                * observation.computer_bridge_available
                * (willed_urge(core_systems.chaotic_urges.social) * 0.7
                    + genetics.extraversion * 0.3)
        }
    }
}

fn material_readiness(carrying: &CarryingState) -> f64 {
    [ItemKind::Wood, ItemKind::Stone, ItemKind::Fiber]
        .into_iter()
        .map(|kind| (carrying.count(kind) as f64 / 3.0).min(1.0))
        .fold(0.0, f64::max)
}

fn building_readiness(carrying: &CarryingState) -> f64 {
    let planks = carrying.count(ItemKind::Planks) as f64 / 6.0;
    let masonry = carrying.count(ItemKind::Masonry) as f64 / 16.0;
    planks.max(masonry).min(1.0)
}

fn intensity_for(
    action: ActionKind,
    needs: &NeedsSnapshot,
    dark_triad: &DarkTriadSnapshot,
    reproduction: &ReproductiveSystemSnapshot,
) -> f64 {
    let urgency = match action {
        ActionKind::SeekFood | ActionKind::Gather => needs.hunger,
        ActionKind::SeekWater => needs.thirst,
        ActionKind::Rest | ActionKind::SeekShelter => needs.fatigue,
        ActionKind::Harm => dark_triad.active_malice,
        ActionKind::Intimacy => reproduction.libido,
        _ => 0.5,
    };
    urgency.clamp(0.05, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanId, HumanProfile, HumanSchema};

    #[test]
    fn every_action_kind_variant_is_in_the_selectable_actions_list() {
        // Regression test: `Code` was fully wired into `drive_for`,
        // `intensity_for`, and `environment_supports_action` but never
        // added to the `ACTIONS` array `choose_action` actually iterates —
        // so it could never be selected despite compiling and testing
        // clean everywhere else. This exhaustive match forces a compile
        // error (not just a silent gap) the next time a variant is added
        // here without also being added to `ACTIONS`.
        fn assert_in_actions(kind: ActionKind) {
            assert!(
                ACTIONS.contains(&kind),
                "{kind:?} is a real ActionKind variant but missing from ACTIONS — it will \
                 compile and pass its own unit tests but can never actually be selected"
            );
        }
        let sample = ActionKind::Idle;
        match sample {
            ActionKind::Idle => {}
            ActionKind::Rest => {}
            ActionKind::SeekWater => {}
            ActionKind::SeekFood => {}
            ActionKind::SeekShelter => {}
            ActionKind::SocialApproach => {}
            ActionKind::Explore => {}
            ActionKind::Gather => {}
            ActionKind::Mine => {}
            ActionKind::Move => {}
            ActionKind::Build => {}
            ActionKind::Craft => {}
            ActionKind::Transfer => {}
            ActionKind::Harm => {}
            ActionKind::Code => {}
            ActionKind::Intimacy => {}
            ActionKind::WebSearch => {}
            ActionKind::SendEmail => {}
        }
        for kind in [
            ActionKind::Idle,
            ActionKind::Rest,
            ActionKind::SeekWater,
            ActionKind::SeekFood,
            ActionKind::SeekShelter,
            ActionKind::SocialApproach,
            ActionKind::Explore,
            ActionKind::Gather,
            ActionKind::Mine,
            ActionKind::Move,
            ActionKind::Build,
            ActionKind::Craft,
            ActionKind::Transfer,
            ActionKind::Harm,
            ActionKind::Code,
            ActionKind::Intimacy,
            ActionKind::WebSearch,
            ActionKind::SendEmail,
        ] {
            assert_in_actions(kind);
        }
    }

    #[test]
    fn a_learned_taste_for_eating_does_not_outbid_a_full_stomach() {
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(1),
            HumanSchema::canonical_minimal("autonomy_test"),
        );
        let mut sated = NeedsSnapshot::from_profile(&profile);
        sated.glucose = 1.0;
        sated.hunger = 0.0;
        let mut hungry = sated.clone();
        hungry.glucose = 0.3;
        hungry.hunger = 0.7;
        // Eating has been nothing but rewarding: the strongest preference
        // a mind can hold.
        let mut mind = AutonomousMind::default();
        for preference in mind.preferences.iter_mut() {
            if preference.action == ActionKind::SeekFood {
                preference.value = 1.0;
            }
        }
        assert_eq!(incentive(ActionKind::SeekFood, &sated), 0.0);
        assert_eq!(incentive(ActionKind::SeekFood, &hungry), 0.7);
        assert_eq!(incentive(ActionKind::Explore, &sated), 1.0);
        let registry = RngRegistry::new([7; 32]);
        let observation = AgentWorldObservation {
            caloric_access: 1.0,
            ..AgentWorldObservation::default()
        };
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let carrying = CarryingState::new();
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let core_systems = CoreSystemsSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile);
        let (mut sated_meals, mut hungry_meals) = (0, 0);
        for tick in 0..200 {
            for (needs, meals) in [(&sated, &mut sated_meals), (&hungry, &mut hungry_meals)] {
                let action = mind.clone().choose_action(
                    needs,
                    &observation,
                    &genetics,
                    &carrying,
                    &dark_triad,
                    &core_systems,
                    &reproduction,
                    &neurochemistry,
                    1,
                    tick,
                    &registry,
                );
                *meals += usize::from(action.kind == ActionKind::SeekFood);
            }
        }
        assert_eq!(sated_meals, 0, "ate {sated_meals} times on a full stomach");
        assert!(
            hungry_meals > 100,
            "hungry, ate only {hungry_meals} of 200 times"
        );
    }

    #[test]
    fn each_mind_learns_in_its_own_state() {
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(1),
            HumanSchema::canonical_minimal("autonomy_test"),
        );
        let needs = NeedsSnapshot::from_profile(&profile);
        let registry = RngRegistry::new([7; 32]);
        let mut first = AutonomousMind::default();
        let second = AutonomousMind::default();
        let observation = AgentWorldObservation::default();
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let carrying = CarryingState::new();
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let core_systems = CoreSystemsSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile);
        first.choose_action(
            &needs,
            &observation,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
            &neurochemistry,
            1,
            1,
            &registry,
        );
        first.learn_from_outcome(&needs, &observation, 2, true);
        let mut improved_needs = needs.clone();
        improved_needs.glucose = 0.9;
        improved_needs.hydration = 0.9;
        improved_needs.fatigue = 0.0;
        first.learn_from_outcome(&improved_needs, &observation, 3, true);
        assert_eq!(first.experiences.len(), 1);
        assert!(second.experiences.is_empty());
    }

    #[test]
    fn willpower_gate_controls_whether_chaotic_urges_boost_explore_score() {
        // Regression test: `CoreSystemsSnapshot` used to be computed live
        // every tick and read by nothing that affects behavior. This proves
        // its `chaotic_urges`/`willpower_available` output now genuinely
        // changes `drive_for`'s score for `Explore`, gated correctly on the
        // willpower gate rather than always applying.
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(1),
            HumanSchema::canonical_minimal("willpower_gate_test"),
        );
        let needs = NeedsSnapshot::from_profile(&profile);
        let observation = AgentWorldObservation::default();
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let carrying = CarryingState::new();
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);

        let mut core_systems = CoreSystemsSnapshot::from_profile(&profile);
        core_systems.chaotic_urges.exploration = 1.0;

        core_systems.willpower_available = false;
        let score_without_gate = drive_for(
            ActionKind::Explore,
            &needs,
            &observation,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
        );

        core_systems.willpower_available = true;
        let score_with_gate = drive_for(
            ActionKind::Explore,
            &needs,
            &observation,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
        );

        assert!(
            score_with_gate > score_without_gate,
            "a gated high exploration urge should raise Explore's score: \
             without gate = {score_without_gate}, with gate = {score_with_gate}"
        );
    }

    #[test]
    fn fatigued_brain_state_forces_rest_regardless_of_drive_scores() {
        // Regression test: a genuinely asleep human (real melatonin-driven
        // `BrainState::Fatigued`, see `neurochemistry.rs`) must not keep
        // deliberating between drives — sleep is a discrete gate, not just
        // one more score term.
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(1),
            HumanSchema::canonical_minimal("sleep_gate_test"),
        );
        let needs = NeedsSnapshot::from_profile(&profile);
        // Starve hunger sky-high so, absent the sleep gate, SeekFood/Gather
        // would dominate every other action's score.
        let mut needs = needs;
        needs.hunger = 1.0;
        let observation = AgentWorldObservation {
            caloric_access: 1.0,
            ..AgentWorldObservation::default()
        };
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let carrying = CarryingState::new();
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let core_systems = CoreSystemsSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);
        let mut neurochemistry = NeurochemistrySnapshot::from_profile(&profile);
        neurochemistry.modulation_effects.current_brain_state = BrainState::Fatigued;
        let registry = RngRegistry::new([1; 32]);

        let mut mind = AutonomousMind::default();
        let action = mind.choose_action(
            &needs,
            &observation,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
            &neurochemistry,
            1,
            1,
            &registry,
        );

        assert_eq!(action.kind, ActionKind::Rest);
    }

    #[test]
    fn code_action_requires_computer_access() {
        let mind = AutonomousMind {
            last_action: AgentAction {
                kind: ActionKind::Code,
                intensity: 1.0,
            },
            ..AutonomousMind::default()
        };

        let no_computer_room = AgentWorldObservation {
            computer_access: 0.0,
            ..AgentWorldObservation::default()
        };
        assert!(!mind.environment_supports_action(&no_computer_room));

        let has_computer_room = AgentWorldObservation {
            computer_access: 1.0,
            ..AgentWorldObservation::default()
        };
        assert!(mind.environment_supports_action(&has_computer_room));
    }

    #[test]
    fn code_drive_is_zero_without_computer_access_regardless_of_achievement_urge() {
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(1),
            HumanSchema::canonical_minimal("code_drive_test"),
        );
        let needs = NeedsSnapshot::from_profile(&profile);
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let carrying = CarryingState::new();
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);
        let mut core_systems = CoreSystemsSnapshot::from_profile(&profile);
        core_systems.chaotic_urges.achievement = 1.0;
        core_systems.willpower_available = true;

        let no_computer_room = AgentWorldObservation {
            computer_access: 0.0,
            ..AgentWorldObservation::default()
        };
        let score = drive_for(
            ActionKind::Code,
            &needs,
            &no_computer_room,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
        );
        assert_eq!(
            score, 0.0,
            "Code should score zero with no Computer Room present, however high the urge"
        );

        let has_computer_room = AgentWorldObservation {
            computer_access: 1.0,
            ..AgentWorldObservation::default()
        };
        let score_with_access = drive_for(
            ActionKind::Code,
            &needs,
            &has_computer_room,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
        );
        assert!(score_with_access > 0.0);
    }

    #[test]
    fn web_search_requires_both_computer_access_and_bridge() {
        let mind = AutonomousMind {
            last_action: AgentAction {
                kind: ActionKind::WebSearch,
                intensity: 1.0,
            },
            ..AutonomousMind::default()
        };

        let neither = AgentWorldObservation {
            computer_access: 0.0,
            computer_bridge_available: 0.0,
            ..AgentWorldObservation::default()
        };
        assert!(!mind.environment_supports_action(&neither));

        let bridge_only = AgentWorldObservation {
            computer_access: 0.0,
            computer_bridge_available: 1.0,
            ..AgentWorldObservation::default()
        };
        assert!(
            !mind.environment_supports_action(&bridge_only),
            "bridge configured but no in-world NetworkAccount/ComputerRoom should not suffice"
        );

        let machine_only = AgentWorldObservation {
            computer_access: 1.0,
            computer_bridge_available: 0.0,
            ..AgentWorldObservation::default()
        };
        assert!(
            !mind.environment_supports_action(&machine_only),
            "in-world machine access without a configured bridge should not suffice"
        );

        let both = AgentWorldObservation {
            computer_access: 1.0,
            computer_bridge_available: 1.0,
            ..AgentWorldObservation::default()
        };
        assert!(mind.environment_supports_action(&both));
    }

    #[test]
    fn send_email_requires_both_computer_access_and_bridge() {
        let mind = AutonomousMind {
            last_action: AgentAction {
                kind: ActionKind::SendEmail,
                intensity: 1.0,
            },
            ..AutonomousMind::default()
        };

        let bridge_only = AgentWorldObservation {
            computer_access: 0.0,
            computer_bridge_available: 1.0,
            ..AgentWorldObservation::default()
        };
        assert!(!mind.environment_supports_action(&bridge_only));

        let both = AgentWorldObservation {
            computer_access: 1.0,
            computer_bridge_available: 1.0,
            ..AgentWorldObservation::default()
        };
        assert!(mind.environment_supports_action(&both));
    }

    #[test]
    fn web_search_drive_is_zero_without_computer_access_even_with_bridge_available() {
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(1),
            HumanSchema::canonical_minimal("web_search_drive_test"),
        );
        let needs = NeedsSnapshot::from_profile(&profile);
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let carrying = CarryingState::new();
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);
        let core_systems = CoreSystemsSnapshot::from_profile(&profile);

        let bridge_only = AgentWorldObservation {
            computer_access: 0.0,
            computer_bridge_available: 1.0,
            ..AgentWorldObservation::default()
        };
        let score = drive_for(
            ActionKind::WebSearch,
            &needs,
            &bridge_only,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
        );
        assert_eq!(
            score, 0.0,
            "WebSearch should score zero without a real in-world NetworkAccount, even if the bridge is configured"
        );

        let both = AgentWorldObservation {
            computer_access: 1.0,
            computer_bridge_available: 1.0,
            ..AgentWorldObservation::default()
        };
        let score_with_both = drive_for(
            ActionKind::WebSearch,
            &needs,
            &both,
            &genetics,
            &carrying,
            &dark_triad,
            &core_systems,
            &reproduction,
        );
        assert!(score_with_both >= 0.0);
    }

    #[test]
    fn learned_mind_state_round_trips_through_persistence_format() {
        let mut mind = AutonomousMind {
            total_reward: 0.75,
            ..AutonomousMind::default()
        };
        mind.experiences.push_back(AutonomousExperience {
            tick: 9,
            action: ActionKind::Gather,
            reward: 0.75,
            fitness_after: 0.8,
            action_success: true,
            caloric_access: 0.75,
            hydration_access: 0.75,
            shelter_quality: 0.5,
            hazard_index: 0.05,
            resource_abundance: 0.5,
        });
        let encoded = serde_json::to_string(&mind).expect("mind should serialize");
        let restored: AutonomousMind = serde_json::from_str(&encoded).expect("mind should load");
        assert_eq!(restored.total_reward, mind.total_reward);
        assert_eq!(restored.experiences, mind.experiences);
    }

    #[test]
    fn movement_is_deterministic_and_grid_bounded() {
        let mind = AutonomousMind {
            last_action: AgentAction {
                kind: ActionKind::Explore,
                intensity: 1.0,
            },
            ..AutonomousMind::default()
        };
        let registry = RngRegistry::new([3; 32]);
        let grid = crate::topology::GridTopology::planetary(
            &mk_core::grid::GridSpec::new(2, 2),
            mk_core::grid::CANON_PLANET_RADIUS_M,
        );
        let mut first = crate::agents::GridPosition::new(0, 0);
        let mut second = first;
        mind.apply_movement(&mut first, &grid, 42, 7, &registry);
        mind.apply_movement(&mut second, &grid, 42, 7, &registry);
        assert_eq!(first, second);
        assert!(first.row < 2 && first.col < 2);
    }

    #[test]
    fn failed_world_action_is_a_negative_learning_outcome() {
        let profile = HumanProfile::from_canonical_schema(
            HumanId::new(2),
            HumanSchema::canonical_minimal("failed_action_test"),
        );
        let needs = NeedsSnapshot::from_profile(&profile);
        let observation = AgentWorldObservation::default();
        let mut mind = AutonomousMind::default();
        mind.learn_from_outcome(&needs, &observation, 1, true);
        mind.learn_from_outcome(&needs, &observation, 2, false);
        assert_eq!(mind.experiences.len(), 1);
        assert!(mind.experiences[0].reward < 0.0);
        assert!(!mind.experiences[0].action_success);
    }

    #[test]
    fn resource_seeking_without_access_is_not_successful() {
        let mind = AutonomousMind {
            last_action: AgentAction {
                kind: ActionKind::SeekFood,
                intensity: 1.0,
            },
            ..AutonomousMind::default()
        };
        assert!(!mind.environment_supports_action(&AgentWorldObservation {
            caloric_access: 0.0,
            ..AgentWorldObservation::default()
        }));
    }
}
