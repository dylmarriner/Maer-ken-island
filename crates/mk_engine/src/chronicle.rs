//! World chronicle: the persistent record of notable things that happened.
//!
//! Most subsystem state is a snapshot of *now* — a retired disturbance, a
//! dead human or a species that went extinct leaves no trace in it. The
//! chronicle is the world's own history: each tick,
//! [`WorldChronicle::observe`] compares the live world against what it has
//! already recorded and appends an [`ChronicleEvent`] for every new
//! disturbance onset, construction, arrival/birth, death and technology era.
//! Subsystems with a natural hook (species creation/extinction, operator
//! interventions) record directly via [`WorldChronicle::record`].
//!
//! Events are bounded to [`MAX_CHRONICLE_EVENTS`], dropping the oldest, so a
//! long-running daemon's snapshot size stays bounded; eras are few and kept
//! in full. The chronicle is history *about* hashed subsystem state, so it is
//! not itself part of the hash chain.

use std::collections::{BTreeSet, VecDeque};

use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

use crate::resource_economy::{recipe_unlock_threshold, EconomyEventKind, RecipeId};
use crate::world_integration::WorldState;

/// Most events retained; older ones are dropped first.
pub const MAX_CHRONICLE_EVENTS: usize = 4096;

/// What kind of happening an event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChronicleKind {
    Disturbance,
    Construction,
    HumanArrival,
    HumanDeath,
    SpeciesEmerged,
    SpeciesExtinct,
    Intervention,
}

/// One notable happening.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronicleEvent {
    pub tick: Tick,
    pub kind: ChronicleKind,
    pub description: String,
    /// Relative significance, `0.0..=1.0`.
    pub severity: f32,
}

/// The start of a technological era: the tick the first human's accumulated
/// knowledge crossed the unlock threshold of the era's defining recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronicleEra {
    pub name: String,
    pub start_tick: Tick,
}

/// Technology eras, each defined by the recipe whose unlock opens it.
const TECHNOLOGY_ERAS: [(RecipeId, &str); 3] = [
    (RecipeId::IronAxe, "Iron Age"),
    (RecipeId::Cart, "Age of the Wheel"),
    (RecipeId::StoneHouse, "Age of Masonry"),
];

/// Name of the era every world starts in.
pub const FOUNDING_ERA: &str = "Founding";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldChronicle {
    pub events: VecDeque<ChronicleEvent>,
    pub eras: Vec<ChronicleEra>,
    /// Last tick [`Self::observe`] scanned; events at or before it are
    /// already recorded.
    observed_through: Option<Tick>,
    /// Humans alive at the last scan, to detect arrivals and deaths.
    known_humans: BTreeSet<String>,
}

impl WorldChronicle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an event, dropping the oldest past [`MAX_CHRONICLE_EVENTS`].
    pub fn record(
        &mut self,
        tick: Tick,
        kind: ChronicleKind,
        description: impl Into<String>,
        severity: f32,
    ) {
        self.events.push_back(ChronicleEvent {
            tick,
            kind,
            description: description.into(),
            severity: severity.clamp(0.0, 1.0),
        });
        while self.events.len() > MAX_CHRONICLE_EVENTS {
            self.events.pop_front();
        }
    }

    /// Record everything new in `world` since the previous scan.
    ///
    /// Takes the pieces of world state it reads rather than `&WorldState`
    /// so the caller can hold `&mut self.chronicle` at the same time.
    pub(crate) fn observe(&mut self, world: &ObservedWorld<'_>) {
        let tick = world.tick;
        let observed_through = self.observed_through;
        let is_new = |event_tick: Tick| observed_through.is_none_or(|seen| event_tick > seen);

        if self.eras.is_empty() {
            self.eras.push(ChronicleEra {
                name: FOUNDING_ERA.to_string(),
                start_tick: tick,
            });
        }

        let mut fresh = Vec::new();
        for disturbance in world.disturbances {
            if is_new(disturbance.onset_tick) {
                fresh.push(ChronicleEvent {
                    tick: disturbance.onset_tick,
                    kind: ChronicleKind::Disturbance,
                    description: format!(
                        "{} at cell ({}, {})",
                        disturbance.kind.as_str(),
                        disturbance.row,
                        disturbance.col
                    ),
                    severity: disturbance.intensity as f32,
                });
            }
        }
        for event in world.economy_events {
            if event.kind == EconomyEventKind::Constructed && is_new(event.tick) {
                fresh.push(ChronicleEvent {
                    tick: event.tick,
                    kind: ChronicleKind::Construction,
                    description: format!("{} built {}", event.agent_id, event.subject),
                    severity: 0.3,
                });
            }
        }

        let alive: BTreeSet<String> = world.human_ids.iter().map(|id| id.to_string()).collect();
        // The first scan establishes who exists; they are the founding
        // population, not arrivals.
        if self.observed_through.is_some() {
            for id in alive.difference(&self.known_humans) {
                fresh.push(ChronicleEvent {
                    tick,
                    kind: ChronicleKind::HumanArrival,
                    description: format!("{id} joined the world"),
                    severity: 0.4,
                });
            }
            for id in self.known_humans.difference(&alive) {
                fresh.push(ChronicleEvent {
                    tick,
                    kind: ChronicleKind::HumanDeath,
                    description: format!("{id} died"),
                    severity: 0.6,
                });
            }
        }
        self.known_humans = alive;

        fresh.sort_by_key(|event| event.tick);
        for event in fresh {
            self.record(event.tick, event.kind, event.description, event.severity);
        }

        for (recipe, name) in TECHNOLOGY_ERAS {
            let reached = world.max_knowledge >= recipe_unlock_threshold(recipe);
            if reached && !self.eras.iter().any(|era| era.name == name) {
                self.eras.push(ChronicleEra {
                    name: name.to_string(),
                    start_tick: tick,
                });
            }
        }

        self.observed_through = Some(tick);
    }
}

/// The slices of world state [`WorldChronicle::observe`] reads.
pub(crate) struct ObservedWorld<'a> {
    pub tick: Tick,
    pub disturbances: &'a [crate::disturbance::ActiveDisturbance],
    pub economy_events: &'a [crate::resource_economy::EconomyEvent],
    pub human_ids: Vec<&'a str>,
    /// Highest accumulated invention knowledge of any living human.
    pub max_knowledge: f64,
}

impl WorldState {
    /// Scan this tick's state into the chronicle.
    pub(crate) fn observe_chronicle(&mut self) {
        let humans = self.humans_state.registry.get_all_humans();
        let observed = ObservedWorld {
            tick: self.tick,
            disturbances: &self.disturbance_state.active,
            economy_events: &self.resource_economy_state.events,
            human_ids: humans.iter().map(|h| h.agent_id()).collect(),
            max_knowledge: humans
                .iter()
                .map(|h| h.technology.accumulated_knowledge)
                .fold(0.0, f64::max),
        };
        self.chronicle.observe(&observed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::canon::CanonLocked;
    use std::sync::Arc;

    fn world() -> WorldState {
        WorldState::new(Arc::new(CanonLocked::default()), [3u8; 32])
    }

    #[test]
    fn first_scan_opens_the_founding_era_without_inventing_arrivals() {
        let mut w = world();
        w.observe_chronicle();
        assert_eq!(w.chronicle.eras.len(), 1);
        assert_eq!(w.chronicle.eras[0].name, FOUNDING_ERA);
        assert!(w
            .chronicle
            .events
            .iter()
            .all(|e| e.kind != ChronicleKind::HumanArrival));
    }

    #[test]
    fn a_new_disturbance_is_recorded_exactly_once() {
        let mut w = world();
        w.observe_chronicle();
        let tick = w.tick + 1;
        w.tick = tick;
        w.disturbance_state
            .push(crate::disturbance::ActiveDisturbance::new(
                crate::disturbance::DisturbanceKind::Fire,
                1,
                2,
                0.7,
                tick,
            ));
        w.observe_chronicle();
        w.observe_chronicle();
        let fires: Vec<_> = w
            .chronicle
            .events
            .iter()
            .filter(|e| e.kind == ChronicleKind::Disturbance)
            .collect();
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].tick, tick);
        assert!((fires[0].severity - 0.7).abs() < 1e-6);
    }

    #[test]
    fn arrivals_and_deaths_are_recorded() {
        let mut w = world();
        w.observe_chronicle();
        w.humans_state
            .registry
            .create_human(mk_core::human::BiologicalSex::Female)
            .expect("in-memory registry");
        let id = w
            .humans_state
            .registry
            .iter()
            .last()
            .expect("just created")
            .agent_id()
            .to_string();
        w.observe_chronicle();
        assert!(w
            .chronicle
            .events
            .iter()
            .any(|e| e.kind == ChronicleKind::HumanArrival && e.description.contains(&id)));

        w.humans_state
            .registry
            .remove_human(&id)
            .expect("not a protected entity");
        w.observe_chronicle();
        assert!(w
            .chronicle
            .events
            .iter()
            .any(|e| e.kind == ChronicleKind::HumanDeath && e.description.contains(&id)));
    }

    #[test]
    fn technology_eras_open_when_knowledge_crosses_the_unlock_threshold() {
        let mut w = world();
        w.observe_chronicle();
        w.humans_state
            .registry
            .create_human(mk_core::human::BiologicalSex::Male)
            .expect("in-memory registry");
        let id = w
            .humans_state
            .registry
            .iter()
            .last()
            .expect("just created")
            .agent_id()
            .to_string();
        let human = w
            .humans_state
            .registry
            .get_human_mut(&id)
            .expect("just created");
        human.technology.accumulated_knowledge = recipe_unlock_threshold(RecipeId::Cart);
        w.observe_chronicle();
        let names: Vec<_> = w.chronicle.eras.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Iron Age"));
        assert!(names.contains(&"Age of the Wheel"));
        assert!(!names.contains(&"Age of Masonry"));
    }

    #[test]
    fn events_are_bounded() {
        let mut chronicle = WorldChronicle::new();
        for tick in 0..(MAX_CHRONICLE_EVENTS as u64 + 10) {
            chronicle.record(tick, ChronicleKind::Intervention, "edit", 0.5);
        }
        assert_eq!(chronicle.events.len(), MAX_CHRONICLE_EVENTS);
        assert_eq!(chronicle.events.front().map(|e| e.tick), Some(10));
    }
}
