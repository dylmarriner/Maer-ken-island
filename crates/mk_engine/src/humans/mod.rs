// Human Being Implementation
// Phase 11 (MK-III) additive human layer built on mk_core human profiles.

pub mod advanced_memory;
pub mod attention;
pub mod attractor_control;
pub mod autonomy;
pub mod body;
pub mod brain_regions;
pub mod circadian;
pub mod cognition;
pub mod comprehensive_emotion;
pub mod computer_bridge;
pub mod consciousness;
pub mod core_systems;
pub mod creativity;
pub mod culture;
pub mod dark_triad;
pub mod decision;
pub mod development;
pub mod dialogue;
pub mod emotion;
pub mod formal_predictive_processing;
pub mod genetics;
pub mod immune;
pub mod language;
pub mod learning;
pub mod lifecycle;
pub mod memory;
pub mod mesoscale_brain;
pub mod needs;
pub mod neurochemistry;
pub mod observation;
pub mod pathology;
pub mod physical_capacity;
pub mod population_dynamics;
pub mod proprioception;
pub mod rates;
pub mod registry;
pub mod reproduction;
pub mod sensory;
pub mod skin;
pub mod social_cognition;
pub mod social_systems;
pub mod spawn;
pub mod technology;
pub mod thought;

pub use advanced_memory::AdvancedMemorySnapshot;
pub use attention::AttentionSnapshot;
pub use attractor_control::AttractorControlSnapshot;
pub use autonomy::AutonomousMind;
pub use body::BodySnapshot;
pub use brain_regions::BrainRegionsSnapshot;
pub use cognition::{CognitiveMode, HumanCognitionSnapshot};
pub use comprehensive_emotion::ComprehensiveEmotionSnapshot;
pub use consciousness::ConsciousnessSnapshot;
pub use core_systems::CoreSystemsSnapshot;
pub use creativity::CreativitySnapshot;
pub use culture::CultureSnapshot;
pub use dark_triad::DarkTriadSnapshot;
pub use decision::DecisionSnapshot;
pub use development::{DevelopmentSnapshot, DevelopmentStage};
pub use dialogue::{ConversationEvent, ConversationRelationship, DialogueLine};
pub use emotion::EmotionSnapshot;
pub use formal_predictive_processing::FormalPredictiveProcessingSnapshot;
pub use genetics::GeneticsSnapshot;
pub use immune::ImmuneSnapshot;
pub use language::{LanguageMode, LanguageSnapshot};
pub use learning::LearningSnapshot;
pub use lifecycle::{deliver_due_births, step_lifecycle};
pub use memory::MemorySnapshot;
pub use mesoscale_brain::MesoscaleBrainSnapshot;
pub use needs::NeedsSnapshot;
pub use neurochemistry::NeurochemistrySnapshot;
pub use pathology::PathologySnapshot;
pub use physical_capacity::PhysicalCapacitySnapshot;
pub use population_dynamics::PopulationDynamicsSnapshot;
pub use proprioception::ProprioceptionSnapshot;
pub use registry::HumanRegistry;
pub use reproduction::ReproductiveSystemSnapshot;
pub use sensory::SensorySnapshot;
pub use skin::SkinSnapshot;
pub use social_cognition::SocialCognitionSnapshot;
pub use social_systems::SocialSystemsSnapshot;
pub use technology::TechnologySnapshot;

pub use crate::agents::{
    ActionKind, AgentAction, AgentWorldObservation, CarryingState, GridPosition,
};
use mk_core::human::schema::BiologicalSexSchema;
pub use mk_core::human::{BiologicalSex, HumanStatus};
use mk_core::human::{HumanId, HumanProfile, HumanSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

/// Cap on [`HumanSystem`]'s shared "recent conversations" feed — a display
/// convenience for the live dashboard, not the source of truth for what any
/// individual human experienced (that's each `HumanBeing`'s own
/// `conversation_history`, capped much more generously below). Sized like
/// `RngAuditLog`'s `DEFAULT_MAX_ENTRIES` bounded-eviction pattern.
pub const CONVERSATION_LOG_MAX_ENTRIES: usize = 500;

/// Per-human cap on `HumanBeing::conversation_history` — generous, sized
/// like a long stretch of remembered exchanges rather than a shared display
/// ring buffer, per the user's explicit "conversations should be
/// remembered, not a rolling log" constraint.
pub const CONVERSATION_HISTORY_MAX_ENTRIES: usize = 2_000;

/// How many lines the shared feed may hold in total, across all its
/// entries.
///
/// An entry count stopped being a bound when `merge_or_push` made an
/// entry a whole exchange instead of a minute of one. A founder's
/// unbroken waking stretch measures about **1,021 lines**
/// (`conversation_rate.rs`, one simulated day), so 500 entries of that
/// size is half a million lines in what the dashboard calls a display
/// convenience. Lines are what the memory actually costs, so lines are
/// what is capped.
///
/// This budget holds about a day of island talking in the shared feed,
/// which is more than a page showing the last forty conversations can
/// use.
pub const CONVERSATION_LOG_MAX_LINES: usize = 2_000;

/// How many lines one human's remembered conversations may hold in
/// total, across all their entries. The real bound on
/// `conversation_history`, for the reason given on
/// [`CONVERSATION_LOG_MAX_LINES`].
///
/// At the measured rate — 2,042 lines in a 36-hour day — this is about
/// **ten island days** of talking a person, roughly 15 Earth days, for a
/// few megabytes each. That is an engineering budget and not a claim
/// about memory: storing a lifetime of speech verbatim at this rate
/// would be hundreds of megabytes a head, and the old comment here
/// calling 2,000 entries "a lifetime" was only ever true because each
/// entry held two lines. What a person recalls of talk they had weeks
/// ago, and whether it should be stored as text at all, is D36.
pub const CONVERSATION_HISTORY_MAX_LINES: usize = 20_000;

/// What the shared dashboard feed is held to.
const SHARED_FEED_BOUNDS: ConversationBounds = ConversationBounds {
    max_entries: CONVERSATION_LOG_MAX_ENTRIES,
    max_lines: CONVERSATION_LOG_MAX_LINES,
};

/// What one human's own remembered conversations are held to.
const OWN_MEMORY_BOUNDS: ConversationBounds = ConversationBounds {
    max_entries: CONVERSATION_HISTORY_MAX_ENTRIES,
    max_lines: CONVERSATION_HISTORY_MAX_LINES,
};

fn push_bounded<T>(log: &mut VecDeque<T>, max_entries: usize, entry: T) {
    if log.len() >= max_entries {
        log.pop_front();
    }
    log.push_back(entry);
}

/// Record an exchange, continuing the one already in progress when there
/// is one.
///
/// A conversation here is the whole exchange between a pair, not the
/// minute of it that happened on this tick. If this log already holds
/// the same two people talking on the tick immediately before, the new
/// lines belong to that exchange and are appended to it; otherwise this
/// is a new conversation and gets its own entry.
///
/// The event keeps the tick it *started* on, which is what makes it one
/// conversation rather than a thousand: `tick` on the stored event
/// answers "when did they start talking", `last_tick` answers "when did
/// they stop", and the lines answer "what was said". Before this, the
/// first two answers were the same minute and there were a thousand of
/// them a day.
///
/// Same pair means the same pair in either order -- `generate_conversation`
/// is given `(a, b)` in the pairing's order, and a conversation does not
/// become a different conversation because the other person spoke first.
///
/// ## Why this searches rather than looking at the last entry
///
/// In a person's own `conversation_history` the last entry is always
/// their last conversation, because the greedy matching in
/// `step_dialogue` marks a human busy once paired, so nobody has two
/// conversations on one tick. Their history is therefore in `last_tick`
/// order and the back entry is the only thing that can be continued.
///
/// The island-wide `conversation_log` has no such order: every pair
/// talking on a tick appends to it, so a pair's own last entry sits
/// behind whatever other pairs said afterwards. Looking only at the
/// back there would start a new conversation every tick for everyone
/// whenever more than one pair was talking -- which is most of the
/// time, and would have left the shared feed fragmented in exactly the
/// way this change is meant to fix. So the search walks back to the
/// pair's own last entry. Both deques are capped
/// ([`CONVERSATION_LOG_MAX_ENTRIES`], [`CONVERSATION_HISTORY_MAX_ENTRIES`]),
/// which is what bounds the walk, and in a person's history it stops on
/// the first entry it looks at whenever there is anything to continue.
fn merge_or_push(
    log: &mut VecDeque<dialogue::ConversationEvent>,
    bounds: ConversationBounds,
    event: &dialogue::ConversationEvent,
    tick: u64,
) {
    let same_pair = |held: &dialogue::ConversationEvent| {
        (held.participant_a_id == event.participant_a_id
            && held.participant_b_id == event.participant_b_id)
            || (held.participant_a_id == event.participant_b_id
                && held.participant_b_id == event.participant_a_id)
    };
    // The pair's own most recent entry, however far back other pairs
    // have pushed it.
    let mut merged = false;
    if let Some(held) = log.iter_mut().rev().find(|held| same_pair(held)) {
        // Still going if its most recent line was said on the tick
        // before this one. That is `last_tick`, the end of the exchange
        // so far, not `tick`, which stays at the beginning of it.
        if held.last_tick + 1 == tick {
            held.lines.extend(event.lines.iter().cloned());
            held.last_tick = tick;
            merged = true;
        }
    }
    if !merged {
        push_bounded(log, bounds.max_entries, event.clone());
    }
    evict_to_line_budget(log, bounds.max_lines);
}

/// The two bounds a conversation log is held to. Entries alone stopped
/// being a bound when an entry became a whole exchange instead of a
/// minute of one, so both travel together.
#[derive(Debug, Clone, Copy)]
struct ConversationBounds {
    max_entries: usize,
    max_lines: usize,
}

/// Drop whole conversations until the log holds no more than `max_lines`
/// lines.
///
/// Least recently spoken goes first, by `last_tick` rather than by
/// position. Position is the order conversations *started* in, and once
/// an exchange can run for hours the two come apart: a conversation that
/// began this morning and is still going sits near the front, so
/// dropping from the front would evict the one happening right now and
/// keep a brief one that has already ended. What has not been spoken in
/// longest is what a feed of recent conversations, and a person's own
/// memory, can best afford to lose.
///
/// Never the last one left: a log holding a single conversation longer
/// than the whole budget keeps it rather than emptying itself, because
/// something that long is the only thing there is to remember and a feed
/// showing nothing is worse than a feed showing one thing.
///
/// Conversations are dropped whole rather than trimmed line by line, so
/// what is kept is always a real exchange from beginning to end. Half a
/// conversation, with its opening missing and no mark saying so, would
/// be a worse record than not having it. Removing from the middle keeps
/// the rest in start order, which is what the dashboard's "newest first"
/// depends on.
fn evict_to_line_budget(log: &mut VecDeque<dialogue::ConversationEvent>, max_lines: usize) {
    let mut held: usize = log.iter().map(|event| event.lines.len()).sum();
    while held > max_lines && log.len() > 1 {
        let Some(stalest) = log
            .iter()
            .enumerate()
            .min_by_key(|(index, event)| (event.last_tick, *index))
            .map(|(index, _)| index)
        else {
            break;
        };
        let Some(dropped) = log.remove(stalest) else {
            break;
        };
        held -= dropped.lines.len();
    }
}

/// This human's own father/mother `agent_id`s, from the birth record
/// `lifecycle.rs` writes into a child's own canonical schema at birth.
/// `None` for humans with no recorded birth (e.g. hand-authored founders).
/// Also exposed as [`HumanBeing::parent_agent_ids`] for cross-crate callers
/// (e.g. `mk_view`'s family projection) that only have a `&HumanBeing`.
fn parent_agent_ids(human: &HumanBeing) -> Option<(String, String)> {
    let schema = human.profile.canonical_schema()?;
    let record = schema
        .reproductive_systems
        .genetics_system
        .birth_records
        .iter()
        .rev()
        .find(|r| r.agent_id == human.agent_id())?;
    Some((record.father_id.clone(), record.mother_id.clone()))
}

/// How `a` and `b` are related, from the founder identities and each
/// human's own recorded birth parents.
fn relationship_between(a: &HumanBeing, b: &HumanBeing) -> dialogue::ConversationRelationship {
    use dialogue::ConversationRelationship as R;
    let (a_id, b_id) = (a.agent_id(), b.agent_id());
    if matches!((a_id, b_id), ("Gem-D", "Gem-K") | ("Gem-K", "Gem-D")) {
        return R::Founders;
    }
    let a_parents = parent_agent_ids(a);
    let b_parents = parent_agent_ids(b);
    let is_parent_of = |parents: &Option<(String, String)>, id: &str| {
        parents
            .as_ref()
            .is_some_and(|(father, mother)| father == id || mother == id)
    };
    if is_parent_of(&a_parents, b_id) || is_parent_of(&b_parents, a_id) {
        return R::ParentChild;
    }
    match (a_parents, b_parents) {
        (Some((af, am)), Some((bf, bm))) if af == bf || am == bm => R::Siblings,
        _ => R::Other,
    }
}

/// System for managing the human population
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanSystem {
    pub registry: HumanRegistry,
    /// Bounded shared feed of recent conversations across the whole
    /// population, for the live dashboard — see
    /// [`CONVERSATION_LOG_MAX_ENTRIES`] docs. Not persisted per-human
    /// history; that lives on each `HumanBeing` itself.
    #[serde(default)]
    conversation_log: VecDeque<dialogue::ConversationEvent>,
    /// Whether every step rewrites every human's full-state files.
    ///
    /// Upstream does, and `true` keeps that behaviour for the planetary
    /// world. The island turns it off and syncs on its own cadence instead:
    /// at a 60-second human step, rewriting a whole population's folders
    /// every step is most of the tick. Runtime policy rather than world
    /// state, so it is never serialized and a loaded world starts from the
    /// upstream default — with no storage attached that is a no-op, and the
    /// island re-states its choice when it attaches one.
    #[serde(skip, default = "sync_every_step_by_default")]
    auto_sync: bool,
}

/// Upstream rewrites every human's files on every step; a world that does
/// not say otherwise does the same.
fn sync_every_step_by_default() -> bool {
    true
}

impl HumanSystem {
    /// Whether every step rewrites every human's full-state files.
    ///
    /// Turning it off does not turn storage off: births, deaths and events
    /// are still written when they happen, and the caller takes on rewriting
    /// the full state on a cadence of its own. The island does both.
    pub fn set_auto_sync(&mut self, on: bool) {
        self.auto_sync = on;
    }

    /// Whether this system rewrites every human's files on every step.
    pub fn auto_sync(&self) -> bool {
        self.auto_sync
    }

    /// A pair bond ends when the partner dies (or is no longer tracked), so
    /// the widowed partner is free to bond again.
    fn end_bonds_with_the_dead(&mut self) {
        let living: std::collections::BTreeSet<String> = self
            .registry
            .get_all_humans()
            .iter()
            .filter(|human| matches!(human.profile.status, HumanStatus::Alive))
            .map(|human| human.agent_id().to_string())
            .collect();
        for human in self.registry.get_all_humans_mut() {
            if human
                .pair_bond
                .as_ref()
                .is_some_and(|partner| !living.contains(partner))
            {
                human.pair_bond = None;
            }
        }
    }

    /// Bonded partners travel together: when one of a bonded pair who
    /// shared a cell moved this step (`moves`: index, from, to) and the
    /// other stayed put, the one who stayed goes along.
    fn travel_with_partners(&mut self, moves: &[(usize, GridPosition, GridPosition)]) {
        let moved: std::collections::BTreeSet<usize> =
            moves.iter().map(|(index, _, _)| *index).collect();
        for (mover, from, to) in moves {
            let Some(partner_id) = self.registry.get_all_humans()[*mover].pair_bond.clone() else {
                continue;
            };
            let Some(partner) = self
                .registry
                .get_all_humans()
                .iter()
                .position(|human| human.agent_id() == partner_id)
            else {
                continue;
            };
            let mover_id = self.registry.get_all_humans()[*mover]
                .agent_id()
                .to_string();
            let follower = &mut self.registry.get_all_humans_mut()[partner];
            if !moved.contains(&partner)
                && matches!(follower.profile.status, HumanStatus::Alive)
                && follower.pair_bond.as_deref() == Some(mover_id.as_str())
                && follower.position == *from
            {
                follower.position = *to;
            }
        }
    }

    /// Create a new human system
    pub fn new() -> Self {
        Self {
            registry: HumanRegistry::new(),
            conversation_log: VecDeque::new(),
            auto_sync: true,
        }
    }

    /// Create a human system whose population is persisted as individual
    /// profiles under `storage`, then seed the two canonical founding humans.
    ///
    /// `Err` means the storage could not be opened or reloaded at all. Once
    /// it opens, the founders always exist: folder-write failures while
    /// seeding them are returned alongside the system instead of discarding it.
    pub fn with_persistent_founders(
        storage: crate::io::HumanStorage,
    ) -> Result<(Self, Vec<crate::io::HumanStorageError>), crate::io::HumanStorageError> {
        let mut system = Self {
            registry: HumanRegistry::with_storage(storage)?,
            conversation_log: VecDeque::new(),
            auto_sync: true,
        };
        let seed_errors = system.registry.seed_founders().err().unwrap_or_default();
        Ok((system, seed_errors))
    }

    /// Move the founding pair to the founders' estate, if one has been
    /// placed. Called once at world setup, after both the estate (see
    /// `PropertySystem::founders_estate_location`) and the founders exist,
    /// so Gem-D & Gem-K begin the world at their home instead of at their
    /// canonical birthplace coordinates — those remain profile truth for
    /// astrology/trait derivation, but are not where the founders live.
    ///
    /// Marks each position as runtime-initialized, so the per-step
    /// `seed_runtime_position_from_birthplace` pass leaves it alone.
    pub fn place_founders_at_home(&mut self, location: Option<(usize, usize)>) {
        let Some((row, col)) = location else {
            return;
        };
        for agent_id in ["Gem-D", "Gem-K"] {
            if let Some(human) = self.registry.get_human_mut(agent_id) {
                human.set_runtime_position(GridPosition::new(row as i32, col as i32));
            }
        }
    }

    /// The shared "recent conversations" feed for the live dashboard, most
    /// recent last. See [`CONVERSATION_LOG_MAX_ENTRIES`] docs — this is a
    /// display convenience, not any one human's actual memory.
    pub fn conversation_log(&self) -> impl Iterator<Item = &dialogue::ConversationEvent> {
        self.conversation_log.iter()
    }

    /// Generate this tick's deterministic inter-human conversations: the
    /// founders Gem-D/Gem-K (if both alive) every tick, plus every living
    /// human paired with each living parent. Each generated
    /// [`dialogue::ConversationEvent`] is appended both to the shared
    /// dashboard feed and to each participant's own persistent
    /// `conversation_history` — never a shared rolling log for the
    /// participants themselves, per the "humans remember what the other
    /// says" constraint.
    /// A death is a life event that should move the emotion state of the
    /// deceased's living relatives — previously it did not (see
    /// `audit-results/human-consciousness-plan-v1-code-verification-2026-09-11.md`
    /// Phase 12b: "death is self-contained ... nobody else's state
    /// registers it"). Weighted by relationship closeness, derived from the
    /// same birth-record lineage data [`dialogue`] uses for relationship
    /// typing — parents and children of the deceased grieve hardest,
    /// shared-parent siblings less so. Applies a one-shot
    /// [`EmotionSnapshot::apply_grief_shock`]; the following ticks' normal
    /// `emotion::step` decay carries it back toward baseline like any other
    /// perturbation, so this needs no separate fade-out logic.
    fn propagate_grief(&mut self, deceased_id: &str) {
        const GRIEF_PARENT_OR_CHILD: f64 = 0.6;
        const GRIEF_SIBLING: f64 = 0.3;

        let Some(deceased) = self.registry.get_human(deceased_id) else {
            return;
        };
        let deceased_parents = parent_agent_ids(deceased);

        let mut grieving: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for human in self.registry.get_all_humans() {
            if !matches!(human.profile.status, HumanStatus::Alive) {
                continue;
            }
            let agent_id = human.agent_id().to_string();
            if agent_id == deceased_id {
                continue;
            }
            let own_parents = parent_agent_ids(human);
            let is_child_of_deceased = own_parents
                .as_ref()
                .is_some_and(|(f, m)| f == deceased_id || m == deceased_id);
            let is_sibling = match (&own_parents, &deceased_parents) {
                (Some((of, om)), Some((df, dm))) => of == df || om == dm,
                _ => false,
            };
            if is_child_of_deceased {
                grieving
                    .entry(agent_id)
                    .and_modify(|w| *w = w.max(GRIEF_PARENT_OR_CHILD))
                    .or_insert(GRIEF_PARENT_OR_CHILD);
            } else if is_sibling {
                grieving
                    .entry(agent_id)
                    .and_modify(|w| *w = w.max(GRIEF_SIBLING))
                    .or_insert(GRIEF_SIBLING);
            }
        }
        if let Some((father_id, mother_id)) = deceased_parents {
            for parent_id in [father_id, mother_id] {
                if self
                    .registry
                    .get_human(&parent_id)
                    .is_some_and(|p| matches!(p.profile.status, HumanStatus::Alive))
                {
                    grieving
                        .entry(parent_id)
                        .and_modify(|w| *w = w.max(GRIEF_PARENT_OR_CHILD))
                        .or_insert(GRIEF_PARENT_OR_CHILD);
                }
            }
        }

        for (agent_id, weight) in grieving {
            if let Some(human) = self.registry.get_human_mut(&agent_id) {
                human.emotion.apply_grief_shock(weight);
                // Losing someone close erodes trust somewhat, not just mood
                // — the real "life event moves social_systems' inputs"
                // input the audit flagged as entirely missing (Phase 12).
                human.social_systems.apply_social_shock(-weight * 0.3);
            }
        }
    }

    /// Pair living humans into this step's conversations.
    ///
    /// Each human is in at most one conversation per step. The pairing is one
    /// deterministic greedy matching over candidate pairs, costing O(n log n)
    /// in the population (humans are bucketed by grid cell and each looks at
    /// a bounded window of eligible candidates):
    ///
    /// * **Family pairs** — the founders, and every adjacent child/parent and
    ///   adjacent sibling pair — outrank everyone else. "Adjacent" is
    ///   Manhattan distance 1 or less, the same test throughout: people have
    ///   to be near each other to talk. Among themselves they are ordered by a hash of the tick and
    ///   the two ids, so no family pair monopolises a person: over time the
    ///   founders talk to each other *and* to their children.
    /// * **Neighbour pairs** — humans in the same or an edge-adjacent cell
    ///   where either is reaching out socially — come next, ranked by mutual
    ///   approach, then closeness, then the same hash.
    ///
    /// Eligibility is decided *before* candidates are capped, so a crowd of
    /// idle strangers cannot hide an eligible partner.
    pub fn step_dialogue(&mut self, rng_registry: &mk_core::rng::RngRegistry, tick: u64) {
        use dialogue::ConversationRelationship as Relationship;
        use std::cmp::Reverse;
        use std::collections::{BTreeSet, HashMap, HashSet};
        use std::ops::Bound::{Excluded, Unbounded};

        /// Eligible candidates examined per human (per cell). Bounds the
        /// work per human so a crowded cell cannot make pairing quadratic.
        const CANDIDATES: usize = 8;

        /// Higher is matched first: (family?, mutual approach, closeness,
        /// per-tick hash, earlier registry positions).
        type EdgeScore = (bool, bool, i32, u64, Reverse<usize>, Reverse<usize>);

        struct Edge {
            score: EdgeScore,
            a: usize,
            b: usize,
            relationship: Relationship,
        }

        let humans = self.registry.get_all_humans();
        let alive = |i: usize| matches!(humans[i].profile.status, HumanStatus::Alive);
        let approaching =
            |h: &HumanBeing| matches!(h.economy_action.kind, ActionKind::SocialApproach);
        // Awake, not merely alive.
        //
        // Family pairs used to be matched on `alive` alone, so the founders
        // held a conversation on every tick of every night: watched on a
        // running island, both asleep from tick 1799, they generated forty
        // exchanges across ticks 2001-2040. Nobody was awake for any of
        // them, and each one fed both participants' relationship memory and
        // social state.
        //
        // `circadian.asleep` is the sleep gate's own output, computed every
        // tick from the Forger-Jewett-Kronauer pacemaker, so this adds no
        // state and invents nothing: it stops reading a person as available
        // to talk while the model already says they are asleep. The
        // remaining half of D35 — that waking pairs still converse on every
        // single tick, once a simulated minute — is a question about what
        // the rate should be, and is left open.
        let available = |i: usize| alive(i) && !humans[i].circadian.asleep;
        let living: Vec<usize> = (0..humans.len()).filter(|&i| available(i)).collect();

        // Deterministic per-tick tie-break, independent of registry order.
        let tick_hash = |a: usize, b: usize| -> u64 {
            let (x, y) = (humans[a].agent_id(), humans[b].agent_id());
            let (x, y) = if x <= y { (x, y) } else { (y, x) };
            let mut hasher = blake3::Hasher::new();
            hasher.update(&tick.to_le_bytes());
            hasher.update(x.as_bytes());
            hasher.update(&[0]);
            hasher.update(y.as_bytes());
            let digest = hasher.finalize();
            u64::from_le_bytes(digest.as_bytes()[..8].try_into().expect("8 bytes"))
        };
        let manhattan = |a: usize, b: usize| {
            (humans[a].position.row - humans[b].position.row).abs()
                + (humans[a].position.col - humans[b].position.col).abs()
        };

        let mut edges: Vec<Edge> = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        let mut add_edge = |a: usize, b: usize, relationship: Relationship, mutual: bool| {
            let (lo, hi) = (a.min(b), a.max(b));
            if lo == hi || !seen.insert((lo, hi)) {
                return;
            }
            let family = relationship != Relationship::Other;
            let score = (
                family,
                !family && mutual,
                if family { 0 } else { -manhattan(lo, hi) },
                tick_hash(lo, hi),
                Reverse(lo),
                Reverse(hi),
            );
            edges.push(Edge {
                score,
                a: lo,
                b: hi,
                relationship,
            });
        };

        // Family: the founders.
        if let (Some(d), Some(k)) = (
            self.registry.position_of("Gem-D"),
            self.registry.position_of("Gem-K"),
        ) {
            if available(d) && available(k) {
                add_edge(d, k, Relationship::Founders, false);
            }
        }

        // Family: children and their living parents, near enough to be
        // heard; and the living children of each parent, to find siblings.
        //
        // This is D34, and the rule is the one siblings already use rather
        // than a new one: Manhattan distance 1 or less. A parent and child
        // used to be matched at *any* distance, which on a 2 km grid meant
        // a pair 400 cells apart -- 800 km -- holding a conversation, and
        // feeding the relationship memory and social state behind it. No
        // number is invented here, because inventing a range for how far a
        // voice carries would be worse than using the one the model already
        // applies to two siblings standing in a field.
        //
        // What this gives up is the shortcut where a parent and child find
        // each other without moving. That was never real: seeking somebody
        // out is movement, and `SocialApproach` is how this model does
        // movement toward a person. A spread family will now talk less
        // until they walk to each other, which is the correct answer to
        // "can these two hear one another" even though it is the quieter
        // one.
        let mut children_of: HashMap<String, Vec<usize>> = HashMap::new();
        for &child in &living {
            let Some((father_id, mother_id)) = parent_agent_ids(&humans[child]) else {
                continue;
            };
            for parent_id in [father_id, mother_id] {
                if let Some(parent) = self.registry.position_of(&parent_id) {
                    if parent != child && available(parent) && manhattan(child, parent) <= 1 {
                        add_edge(child, parent, Relationship::ParentChild, false);
                    }
                }
                children_of.entry(parent_id).or_default().push(child);
            }
        }

        // Family: adjacent siblings, found through their shared parents.
        for &a in &living {
            let Some((father_id, mother_id)) = parent_agent_ids(&humans[a]) else {
                continue;
            };
            let parents = [father_id, mother_id];
            let siblings = parents
                .iter()
                .filter_map(|id| children_of.get(id))
                .flatten()
                .copied()
                .filter(|&b| b != a && manhattan(a, b) <= 1)
                .take(CANDIDATES);
            for sibling in siblings {
                add_edge(
                    a,
                    sibling,
                    relationship_between(&humans[a], &humans[sibling]),
                    approaching(&humans[a]) && approaching(&humans[sibling]),
                );
            }
        }

        // Neighbours where either is reaching out socially.
        let mut everyone: HashMap<(i32, i32), BTreeSet<usize>> = HashMap::new();
        let mut reaching_out: HashMap<(i32, i32), BTreeSet<usize>> = HashMap::new();
        for &i in &living {
            let p = humans[i].position;
            everyone.entry((p.row, p.col)).or_default().insert(i);
            if approaching(&humans[i]) {
                reaching_out.entry((p.row, p.col)).or_default().insert(i);
            }
        }
        for &a in &living {
            let (row, col) = (humans[a].position.row, humans[a].position.col);
            // A reaching-out human may talk to anyone nearby; anyone else
            // only to those reaching out to them.
            let pool = if approaching(&humans[a]) {
                &everyone
            } else {
                &reaching_out
            };
            for (d_row, d_col) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                let Some(cell) = pool.get(&(row + d_row, col + d_col)) else {
                    continue;
                };
                // The next candidates after `a` in registry order, wrapping,
                // so the candidate graph is a ring rather than a star.
                let window = cell
                    .range((Excluded(a), Unbounded))
                    .chain(cell.range(..a))
                    .take(CANDIDATES);
                for &b in window {
                    add_edge(
                        a,
                        b,
                        relationship_between(&humans[a], &humans[b]),
                        approaching(&humans[a]) && approaching(&humans[b]),
                    );
                }
            }
        }

        // Greedy matching, best edge first.
        edges.sort_unstable_by_key(|edge| Reverse(edge.score));
        let mut busy = vec![false; humans.len()];
        let mut matched: Vec<(usize, usize, Relationship)> = Vec::new();
        for edge in &edges {
            if !busy[edge.a] && !busy[edge.b] {
                busy[edge.a] = true;
                busy[edge.b] = true;
                matched.push((edge.a, edge.b, edge.relationship));
            }
        }

        let pairs: Vec<(String, String, Relationship)> = matched
            .into_iter()
            .map(|(a, b, relationship)| {
                (
                    humans[a].agent_id().to_string(),
                    humans[b].agent_id().to_string(),
                    relationship,
                )
            })
            .collect();

        let events: Vec<(String, String, dialogue::ConversationEvent)> = pairs
            .into_iter()
            .filter_map(|(a_id, b_id, relationship)| {
                let a = self.registry.get_human(&a_id)?;
                let b = self.registry.get_human(&b_id)?;
                let event = dialogue::generate_conversation(a, b, relationship, rng_registry, tick);
                Some((a_id, b_id, event))
            })
            .collect();

        for (a_id, b_id, event) in events {
            // Carried on where the same pair were already talking,
            // rather than filed as a new conversation every minute.
            //
            // This is D35's remaining half, and the measurement is what
            // decided its shape. Counting *words* rather than
            // conversations, Gem-D says 43,985 of them in a 36-hour day
            // against the ~24,000 a person measured by Mehl et al.
            // (*Science* 317:82, 2007 -- 396 people, wearable
            // recorders, ~16,000 words a day) would say in a day that
            // long. That is 1.8x the average and inside their observed
            // range of 700 to 47,000. The founders are talkative; they
            // are not impossible, and the rate was never the defect.
            //
            // The defect was the division. Each exchange is two lines
            // and about twenty-one words, so a thousand of them is one
            // near-continuous conversation cut into minute-long
            // fragments -- and each fragment was filed as a separate
            // remembered conversation. Nobody remembers an afternoon
            // with their partner as a thousand conversations, and a
            // 2,000-entry history meant to hold a lifetime filled in two
            // days because of how talking was *divided*, not how much of
            // it there was.
            //
            // So a pair who spoke on the previous tick continue the
            // exchange they were already having. No threshold is
            // invented for this: "the tick before this one" is the
            // finest grain the simulation has, and anything coarser
            // would be a number with nothing behind it.
            //
            // Merging does not reduce how much is said -- a day is 2,042
            // lines either way -- so it moves where the bound has to go.
            // An entry that is a whole waking stretch holds about a
            // thousand lines, and 2,000 of those would be millions of
            // lines a head, so both logs are held to a line budget as
            // well as an entry count. See `ConversationBounds`.
            merge_or_push(&mut self.conversation_log, SHARED_FEED_BOUNDS, &event, tick);
            if let Some(a) = self.registry.get_human_mut(&a_id) {
                merge_or_push(&mut a.conversation_history, OWN_MEMORY_BOUNDS, &event, tick);
            }
            if let Some(b) = self.registry.get_human_mut(&b_id) {
                merge_or_push(&mut b.conversation_history, OWN_MEMORY_BOUNDS, &event, tick);
            }
        }
    }

    /// Step the human system: aging/needs/death for all living humans (using
    /// `observation_for` to sample each human's local environment by grid
    /// position), then a deterministic reproduction pass over the resulting
    /// population.
    // 8 parameters: one `Res`/typed-state parameter each, the standard
    // shape already established for constructors/systems in this codebase
    // (see 2026-08-18 clippy cleanup changelog entry) — a params struct
    // here would be pure indirection with call-site churn, not a real
    // simplification.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        dt_years: f64,
        tick: u64,
        rng_registry: &mk_core::rng::RngRegistry,
        grid_spec: &mk_core::grid::GridSpec,
        resource_economy: &mut crate::resource_economy::ResourceEconomyState,
        elevation_grid: &mk_core::grid::Grid2<f64>,
        observation_for: impl Fn(&GridPosition, &str) -> AgentWorldObservation,
        computer_bridge: Option<&dyn computer_bridge::ComputerBridge>,
    ) {
        self.step_on(
            dt_years,
            tick,
            rng_registry,
            &crate::topology::GridTopology::planetary(
                grid_spec,
                mk_core::grid::CANON_PLANET_RADIUS_M,
            ),
            resource_economy,
            elevation_grid,
            observation_for,
            computer_bridge,
        );
    }

    /// [`step`](Self::step) on any [`GridTopology`](crate::topology::GridTopology):
    /// the island's is flat and bounded, the planet's is spherical and wraps.
    #[allow(clippy::too_many_arguments)]
    pub fn step_on(
        &mut self,
        dt_years: f64,
        tick: u64,
        rng_registry: &mk_core::rng::RngRegistry,
        topology: &crate::topology::GridTopology,
        resource_economy: &mut crate::resource_economy::ResourceEconomyState,
        elevation_grid: &mk_core::grid::Grid2<f64>,
        observation_for: impl Fn(&GridPosition, &str) -> AgentWorldObservation,
        computer_bridge: Option<&dyn computer_bridge::ComputerBridge>,
    ) {
        let mut newly_dead: Vec<(String, u32, String)> = Vec::new();
        let mut transfer_requests = Vec::new();
        let mut harm_requests = Vec::new();
        let mut intimacy_requests = Vec::new();
        let mut intimacy_acts: Vec<(String, String, Option<bool>)> = Vec::new();
        let mut moves: Vec<(usize, GridPosition, GridPosition)> = Vec::new();
        // Where everyone stood as this pass began: whom a human approaching
        // company can see.
        let living_positions: Vec<(usize, GridPosition)> = self
            .registry
            .get_all_humans()
            .iter()
            .enumerate()
            .filter(|(_, human)| matches!(human.profile.status, HumanStatus::Alive))
            .map(|(index, human)| (index, human.position))
            .collect();
        for (index, human) in self.registry.get_all_humans_mut().iter_mut().enumerate() {
            human.seed_runtime_position_from_birthplace_on(topology);
            let was_alive = matches!(human.profile.status, HumanStatus::Alive);
            if !was_alive {
                // The dead neither decide nor move.
                continue;
            }
            let observation = observation_for(&human.position, human.agent_id());
            if observation.shelter_quality >= autonomy::SEEK_SATISFIED_ACCESS {
                human.home = Some(human.position);
            }
            human.decide(&observation, tick, rng_registry);
            // Wanting intimacy with nobody within reach (the same or an
            // adjacent cell) means going to someone first: what this human
            // actually does, and learns from, is approaching them.
            if human.economy_action.kind == ActionKind::Intimacy
                && !someone_within_reach(human.position, index, &living_positions, topology)
            {
                human.economy_action.kind = ActionKind::SocialApproach;
                human.autonomous_mind.last_action.kind = ActionKind::SocialApproach;
            }
            let position_before_movement = human.position;
            // Walking covers only part of a grid cell in one step: the
            // chance of reaching a neighbouring cell is the distance walked
            // over the cell's width.
            let crosses_cell = rng_registry.gen_f64_01(mk_core::rng::RngKey::new(
                mk_core::rng::SubsystemId::Humans,
                human.profile.human_id.0 as u32,
                CELL_CROSSING_EPOCH,
                tick,
            )) < cell_crossing_probability(human.position, dt_years, topology);
            if crosses_cell {
                human.autonomous_mind.apply_movement(
                    &mut human.position,
                    topology,
                    human.profile.human_id.0,
                    tick,
                    rng_registry,
                );
                // Approaching company heads for the nearest person in sight;
                // with nobody in sight the random step above is the search.
                if human.economy_action.kind == ActionKind::SocialApproach {
                    if let Some(next) = step_toward_nearest(
                        position_before_movement,
                        index,
                        &living_positions,
                        topology,
                    ) {
                        human.position = next;
                    }
                }
                if let Some(next) = human.autonomous_mind.directed_seek_step(
                    &observation,
                    human.position,
                    topology,
                    human.profile.human_id.0,
                    tick,
                    rng_registry,
                    |candidate| observation_for(candidate, human.agent_id()),
                ) {
                    human.position = next;
                }
                // Poorly sheltered and seeking shelter: go home if there is one.
                if human.economy_action.kind == ActionKind::SeekShelter
                    && observation.shelter_quality < autonomy::SEEK_SATISFIED_ACCESS
                {
                    if let Some(next) = human
                        .home
                        .and_then(|home| step_toward(position_before_movement, home, topology))
                    {
                        human.position = next;
                    }
                }
            }
            // Humans walk; they cannot step from land onto open ocean. (One
            // already at sea, e.g. spawned there, may still move.)
            let is_ocean = |p: &GridPosition| {
                elevation_grid
                    .get_safe(p.row.max(0) as usize, p.col.max(0) as usize)
                    .is_some_and(|elevation| *elevation < 0.0)
            };
            if is_ocean(&human.position) && !is_ocean(&position_before_movement) {
                human.position = position_before_movement;
            }
            if human.position != position_before_movement {
                moves.push((index, position_before_movement, human.position));
            }
            let movement_delta = (
                human.position.row - position_before_movement.row,
                human.position.col - position_before_movement.col,
            );
            step_lifecycle(
                human,
                dt_years,
                tick,
                &observation,
                rng_registry,
                movement_delta,
            );
            if matches!(human.profile.status, HumanStatus::Alive) {
                human.technology.step_invention(dt_years);
                human.last_action_success =
                    resource_economy.apply_human_action(human, tick, elevation_grid)
                        && human
                            .autonomous_mind
                            .environment_supports_action(&observation);
                // Deliberate Computer Room work adds real invention progress
                // on top of the passive `step_invention` growth above — this
                // human is now the one generating the progress, not just
                // accumulating it in the background.
                if matches!(human.economy_action.kind, ActionKind::Code)
                    && human.last_action_success
                {
                    human.technology.accumulated_knowledge +=
                        human.economy_action.intensity * dt_years;
                }
                // Real web search / email — separate from the resource-economy
                // pass above (no items/structures involved), same "own
                // post-check after the generic executor" shape as `Code`.
                if matches!(
                    human.economy_action.kind,
                    ActionKind::WebSearch | ActionKind::SendEmail
                ) {
                    human.last_action_success = observation.computer_access > 0.0
                        && computer_bridge::execute_computer_action(human, computer_bridge, tick);
                }
                if matches!(human.economy_action.kind, ActionKind::Transfer) {
                    transfer_requests.push((index, human.position));
                }
                if matches!(human.economy_action.kind, ActionKind::Harm) {
                    harm_requests.push((index, human.position, human.economy_action.intensity));
                }
                if matches!(human.economy_action.kind, ActionKind::Intimacy) {
                    intimacy_requests.push((index, human.position));
                }
            }
            if was_alive && matches!(human.profile.status, HumanStatus::Dead) {
                let reason = human
                    .death_reason
                    .clone()
                    .unwrap_or_else(|| "unrecorded".to_string());
                newly_dead.push((human.agent_id().to_string(), human.age(), reason));
            }
        }

        self.end_bonds_with_the_dead();
        self.travel_with_partners(&moves);

        for (source_index, source_position) in transfer_requests {
            let target_index = self
                .registry
                .get_all_humans()
                .iter()
                .enumerate()
                .filter(|(index, human)| {
                    *index != source_index
                        && matches!(human.profile.status, HumanStatus::Alive)
                        && (human.position.row - source_position.row).abs()
                            + (human.position.col - source_position.col).abs()
                            <= 1
                })
                .min_by_key(|(index, human)| {
                    (
                        (human.position.row - source_position.row).abs()
                            + (human.position.col - source_position.col).abs(),
                        *index,
                    )
                })
                .map(|(index, _)| index);
            let success = target_index.is_some_and(|target_index| {
                let humans = self.registry.get_all_humans_mut();
                if source_index < target_index {
                    let (left, right) = humans.split_at_mut(target_index);
                    left[source_index]
                        .carrying
                        .transfer_one_to(&mut right[0].carrying)
                } else {
                    let (left, right) = humans.split_at_mut(source_index);
                    right[0]
                        .carrying
                        .transfer_one_to(&mut left[target_index].carrying)
                }
            });
            self.registry.get_all_humans_mut()[source_index].last_action_success = success;
        }

        for (source_index, source_position, severity) in harm_requests {
            // Reverence Veto: a protected creator entity can never be a
            // harm target, regardless of the attacker's state. Filtered out
            // of the candidate pool entirely, not merely deprioritized.
            let target_index = self
                .registry
                .get_all_humans()
                .iter()
                .enumerate()
                .filter(|(index, human)| {
                    *index != source_index
                        && matches!(human.profile.status, HumanStatus::Alive)
                        && crate::governance::check(human.agent_id()).is_ok()
                        && (human.position.row - source_position.row).abs()
                            + (human.position.col - source_position.col).abs()
                            <= 1
                })
                .min_by_key(|(index, human)| {
                    (
                        (human.position.row - source_position.row).abs()
                            + (human.position.col - source_position.col).abs(),
                        *index,
                    )
                })
                .map(|(index, _)| index);

            let success = target_index.is_some();
            self.registry.get_all_humans_mut()[source_index].last_action_success = success;

            if let Some(target_index) = target_index {
                let target = &mut self.registry.get_all_humans_mut()[target_index];
                if let Some((agent_id, age)) = apply_harm(target, severity) {
                    crate::io::global_events::log_human_died(tick, &agent_id, age, Some("assault"));
                    newly_dead.push((agent_id, age, "assault".to_string()));
                }
            }
        }

        for (source_index, source_position) in intimacy_requests {
            // No Reverence Veto filter here, unlike `harm_requests` — this
            // is a real consensual/attempted act, not removal or harm.
            // Anyone nearby, including a protected creator entity, is a
            // legitimate candidate; consent is checked at resolution below,
            // not at candidate selection.
            let target_index = self
                .registry
                .get_all_humans()
                .iter()
                .enumerate()
                .filter(|(index, human)| {
                    *index != source_index
                        && matches!(human.profile.status, HumanStatus::Alive)
                        && (human.position.row - source_position.row).abs()
                            + (human.position.col - source_position.col).abs()
                            <= 1
                })
                .min_by_key(|(index, human)| {
                    (
                        (human.position.row - source_position.row).abs()
                            + (human.position.col - source_position.col).abs(),
                        *index,
                    )
                })
                .map(|(index, _)| index);

            if let Some(target_index) = target_index {
                let humans = self.registry.get_all_humans_mut();
                let (initiator, target) = if source_index < target_index {
                    let (left, right) = humans.split_at_mut(target_index);
                    (&mut left[source_index], &mut right[0])
                } else {
                    let (left, right) = humans.split_at_mut(source_index);
                    (&mut right[0], &mut left[target_index])
                };
                if apply_intimacy(initiator, target) {
                    initiator.pair_bond = Some(target.agent_id().to_string());
                    target.pair_bond = Some(initiator.agent_id().to_string());
                    // A consensual opposite-sex act may conceive; the roll
                    // and every eligibility check live in
                    // `lifecycle::attempt_conception`.
                    let conception = match (initiator.biological_sex(), target.biological_sex()) {
                        (BiologicalSex::Female, BiologicalSex::Male) => {
                            Some(lifecycle::attempt_conception(
                                initiator,
                                target,
                                dt_years,
                                rng_registry,
                                tick,
                            ))
                        }
                        (BiologicalSex::Male, BiologicalSex::Female) => {
                            Some(lifecycle::attempt_conception(
                                target,
                                initiator,
                                dt_years,
                                rng_registry,
                                tick,
                            ))
                        }
                        _ => None,
                    };
                    intimacy_acts.push((
                        initiator.agent_id().to_string(),
                        target.agent_id().to_string(),
                        conception,
                    ));
                }
            } else {
                // No one nearby to attempt with at all.
                self.registry.get_all_humans_mut()[source_index].last_action_success = false;
            }
        }

        let social_outcomes: Vec<(usize, bool)> = self
            .registry
            .get_all_humans()
            .iter()
            .enumerate()
            .filter(|(_, human)| {
                matches!(human.profile.status, HumanStatus::Alive)
                    && matches!(human.economy_action.kind, ActionKind::SocialApproach)
            })
            .map(|(index, human)| {
                let has_neighbor = self.registry.get_all_humans().iter().enumerate().any(
                    |(other_index, other)| {
                        index != other_index
                            && matches!(other.profile.status, HumanStatus::Alive)
                            && (human.position.row - other.position.row).abs()
                                + (human.position.col - other.position.col).abs()
                                <= 1
                    },
                );
                (index, has_neighbor)
            })
            .collect();
        for (index, success) in social_outcomes {
            self.registry.get_all_humans_mut()[index].last_action_success = success;
        }

        for (agent_id, _, _) in &newly_dead {
            self.propagate_grief(agent_id);
        }

        for (agent_id, age, reason) in newly_dead {
            let death_event = serde_json::json!({
                "kind": "died",
                "tick": tick,
                "age": age,
                "reason": reason,
            });
            if let Err(e) = self.registry.record_event(&agent_id, &death_event) {
                tracing::warn!(agent = %agent_id, error = %e, "could not record death event");
            }
        }

        for (initiator_id, target_id, conception) in intimacy_acts {
            lifecycle::record_sexual_activity(
                &mut self.registry,
                &initiator_id,
                &target_id,
                tick,
                conception,
            );
            lifecycle::record_sexual_activity(
                &mut self.registry,
                &target_id,
                &initiator_id,
                tick,
                conception,
            );
        }
        for e in deliver_due_births(&mut self.registry, tick, topology) {
            tracing::warn!(error = %e, "a newborn's folder could not be written; the child still exists");
        }
        self.step_dialogue(rng_registry, tick);

        // Persist every living human's evolving state/experiences each tick
        // (no-op when storage isn't configured for this registry, or when
        // the owner of this system syncs on its own cadence). A failed
        // write never changes who exists or how the world evolves.
        if self.auto_sync {
            let sync_errors = self.registry.sync_to_storage();
            if let Some(first) = sync_errors.first() {
                tracing::warn!(
                    failed = sync_errors.len(),
                    error = %first,
                    "human storage sync failed"
                );
            }
        }
    }
}

/// Apply one deliberate harmful act to `target`: real physical damage
/// (`body.vital_energy`/`tension`) and real emotional/mental damage
/// (`emotion.current.fear`/`sadness`), scaled by `severity` (the attacker's
/// `dark_triad.active_malice` at the moment of the act — see
/// `autonomy::intensity_for`). Governance (Reverence Veto) is checked by the
/// caller before a target is ever selected — this function assumes the
/// target is already a legitimate one.
///
/// Returns `Some((agent_id, age))` if this harm was lethal (target's
/// `vital_energy` reached zero), so the caller can log the death and add it
/// to the tick's death-event batch; `None` otherwise.
fn apply_harm(target: &mut HumanBeing, severity: f64) -> Option<(String, u32)> {
    // Tissue damage persists in `body.injury` and heals over days (see
    // `BodySnapshot::step`); the immediate shock also drains energy now.
    target.body.wound(severity);
    target.body.vital_energy = (target.body.vital_energy - severity * 0.4).max(0.0);
    target.skin.integrity = (target.skin.integrity - severity * 0.3).clamp(0.0, 1.0);
    target.body.tension = (target.body.tension + severity * 0.5).clamp(0.0, 1.0);

    target.emotion.current.fear = (target.emotion.current.fear + severity * 0.6).clamp(0.0, 1.0);
    target.emotion.current.sadness =
        (target.emotion.current.sadness + severity * 0.3).clamp(0.0, 1.0);
    // Being attacked is a real betrayal-class life event — see
    // `SocialSystemsSnapshot::apply_social_shock` docs and
    // `HumanSystem::propagate_grief`'s prior use of the same mechanism.
    target.social_systems.apply_social_shock(-severity * 0.4);

    target.last_action_success = false;

    if target.body.is_fatally_injured() && matches!(target.profile.status, HumanStatus::Alive) {
        target.profile.status = HumanStatus::Dead;
        target.death_reason = Some("assault".to_string());
        Some((target.agent_id().to_string(), target.age()))
    } else {
        None
    }
}

/// Distance a human covers on foot in a day of travel (km): sustained
/// walking with loads, as foragers and migrants manage over weeks.
pub const WALKING_KM_PER_DAY: f64 = 20.0;
/// Keyed-RNG epoch for the cell-crossing roll.
const CELL_CROSSING_EPOCH: u32 = 0x5741_4C4B;

/// Chance that a human who acts to move this step reaches a neighbouring
/// cell: [`WALKING_KM_PER_DAY`] over `dt_years`, divided by the width of the
/// cell they stand in, capped at 1.
pub(crate) fn cell_crossing_probability(
    position: GridPosition,
    dt_years: f64,
    topology: &crate::topology::GridTopology,
) -> f64 {
    let row = topology.clamp_row(position.row);
    let cell_width_km = topology.cell_width_m(row) / 1000.0;
    if cell_width_km <= 0.0 {
        return 1.0;
    }
    let walked_km = WALKING_KM_PER_DAY * dt_years.max(0.0) * 365.25;
    (walked_km / cell_width_km).clamp(0.0, 1.0)
}

/// One step (8-neighbourhood, longitude wrapping) from `from` toward the
/// nearest other living human within
/// [`crate::perception::SOCIAL_RADIUS_CELLS`], or `None` when nobody is in
/// sight or they already share the cell.
fn step_toward_nearest(
    from: GridPosition,
    own_index: usize,
    living: &[(usize, GridPosition)],
    topology: &crate::topology::GridTopology,
) -> Option<GridPosition> {
    let wrapped_dcol = |to: i32| topology.col_delta(from.col, to);
    let (d_row, d_col) = living
        .iter()
        .filter(|(index, _)| *index != own_index)
        .map(|(_, p)| (p.row - from.row, wrapped_dcol(p.col)))
        .filter(|(dr, dc)| dr.abs().max(dc.abs()) <= crate::perception::SOCIAL_RADIUS_CELLS)
        .min_by_key(|(dr, dc)| (dr.abs().max(dc.abs()), dr.abs() + dc.abs()))?;
    step_by(from, d_row, d_col, topology)
}

/// Whether another living human stands within intimacy/harm reach of
/// `from`: the same cell or a 4-neighbour (Manhattan distance 1, longitude
/// wrapping), matching the target search in [`HumanSystem::step`].
fn someone_within_reach(
    from: GridPosition,
    own_index: usize,
    living: &[(usize, GridPosition)],
    topology: &crate::topology::GridTopology,
) -> bool {
    living.iter().any(|(index, p)| {
        let d_col = topology.col_delta(from.col, p.col).abs();
        *index != own_index && (p.row - from.row).abs() + d_col <= 1
    })
}

/// One step (8-neighbourhood, longitude wrapping) from `from` toward `to`,
/// or `None` if already there.
fn step_toward(
    from: GridPosition,
    to: GridPosition,
    topology: &crate::topology::GridTopology,
) -> Option<GridPosition> {
    let d_col = topology.col_delta(from.col, to.col);
    step_by(from, to.row - from.row, d_col, topology)
}

fn step_by(
    from: GridPosition,
    d_row: i32,
    d_col: i32,
    topology: &crate::topology::GridTopology,
) -> Option<GridPosition> {
    if d_row == 0 && d_col == 0 {
        return None;
    }
    Some(GridPosition::new(
        topology.clamp_row(from.row + d_row.signum()) as i32,
        topology.resolve_col(from.col + d_col.signum()) as i32,
    ))
}

/// Resolve one deliberate intimacy attempt from `initiator` toward `target`.
/// The attempt happens regardless of `target`'s real state (some humans
/// attempt it "just cause," from real attraction, wanting closeness, or to
/// reproduce) — but it only *succeeds*, applying any effect at all, when
/// `target`'s own real willingness (`reproduction.attraction_average`/
/// `.libido`, real already-stepped state, not the initiator's) clears a
/// real threshold. A refused attempt is a tracked failure (raises the
/// initiator's frustration, `last_action_success = false`), never a
/// silently-succeeding one — non-consensual "success" is deliberately not
/// modeled by this function.
///
/// Returns whether the attempt succeeded.
fn apply_intimacy(initiator: &mut HumanBeing, target: &mut HumanBeing) -> bool {
    // Willing is the target's own desire: their attraction, and their libido
    // less what their last act still satisfies. Consent that ignored their
    // satiety made every one of the initiator's whims an act, the partner as
    // satisfied as they were.
    let desire =
        target.reproduction.libido * (1.0 - target.reproduction.satisfaction.clamp(0.0, 1.0));
    let willing =
        target.reproduction.attraction_average >= 0.4 && desire >= reproduction::CONSENT_DESIRE;

    if willing {
        for person in [&mut *initiator, &mut *target] {
            // An act satisfies both partners fully; the satisfaction wears
            // off over days (`ReproductiveSystemSnapshot::step`), and desire
            // returns as it does.
            person.reproduction.satisfaction = 1.0;
            person.reproduction.frustration =
                (person.reproduction.frustration - 0.3).clamp(0.0, 1.0);
            person.reproduction.bonding_average =
                (person.reproduction.bonding_average + 0.1).clamp(0.0, 1.0);
            person.reproduction.sexual_activity_count += 1;
            // Mirrors `reproduction.bonding_average`'s bump into the
            // module that actually owns trust/attachment — a real
            // consensual bonding-class life event, same mechanism
            // `apply_harm`'s betrayal shock and grief already use.
            person.social_systems.apply_social_shock(0.1);
        }
    } else {
        initiator.reproduction.frustration =
            (initiator.reproduction.frustration + 0.15).clamp(0.0, 1.0);
    }

    initiator.last_action_success = willing;
    willing
}

impl Default for HumanSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// The complete Human Being structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanBeing {
    pub profile: HumanProfile,
    pub genetics: GeneticsSnapshot,
    pub development: DevelopmentSnapshot,
    pub cognition: HumanCognitionSnapshot,
    pub social_systems: SocialSystemsSnapshot,
    pub culture: CultureSnapshot,
    pub language: LanguageSnapshot,
    pub technology: TechnologySnapshot,
    /// Metabolic survival state (hunger, thirst, fatigue, glucose/ATP).
    /// Stepped from real local environment access, not a pure profile
    /// projection — see [`needs`] module docs.
    pub needs: NeedsSnapshot,
    /// Sexual system, mate selection, and lineage-history summary — see
    /// [`reproduction`] module docs.
    pub reproduction: ReproductiveSystemSnapshot,
    /// Vitals, internal physiology, and appearance — see [`body`] module
    /// docs. Like `needs`, stepped from real state, not a pure profile
    /// projection.
    pub body: BodySnapshot,
    /// Innate/adaptive immunity, system stress, pathogen load — see
    /// [`immune`] module docs. Stepped from real state, not a pure profile
    /// projection.
    pub immune: ImmuneSnapshot,
    /// Temperature, cleanliness, healing, infection risk — see [`skin`]
    /// module docs. Stepped from real state, not a pure profile projection.
    pub skin: SkinSnapshot,
    /// Tactile/visual/auditory/vestibular/interoception/proprioception
    /// headline scalars — see [`sensory`] module docs. Stepped from real
    /// state, not a pure profile projection.
    pub sensory: SensorySnapshot,
    /// Attention traits/state/resources — see [`attention`] module docs.
    /// Stepped from real fatigue state, not a pure profile projection.
    pub attention: AttentionSnapshot,
    /// Self-awareness, intrinsic worth, fear, qualia, internal monologue —
    /// see [`consciousness`] module docs. Pure profile projection.
    pub consciousness: ConsciousnessSnapshot,
    /// Episodic/semantic/working memory traits and consolidation/retrieval
    /// state — see [`memory`] module docs. Stepped from real attention
    /// state, not a pure profile projection.
    pub memory: MemorySnapshot,
    /// Layer 4 of the canon's memory model — working/short-term/long-term-
    /// semantic/episodic/procedural sub-store aggregates and cross-cutting
    /// integration metrics — see [`advanced_memory`] module docs. Pure
    /// aggregation over `memory`/`attention`/`emotion` state each tick; no
    /// fabricated memory content.
    pub advanced_memory: AdvancedMemorySnapshot,
    /// Neural plasticity, learning processes, adaptive traits, and
    /// experience-driven pattern recognition/wisdom — see [`learning`]
    /// module docs. Stepped from real memory/development state, not a pure
    /// profile projection.
    pub learning: LearningSnapshot,
    /// Decision weights/processes/choice-architecture, and effective
    /// rationality/intuition/commitment state — see [`decision`] module
    /// docs. Stepped from real cognition/learning state, not a pure
    /// profile projection.
    pub decision: DecisionSnapshot,
    /// Theory of mind, social perception, and interaction traits — see
    /// [`social_cognition`] module docs. Stepped from real attention state,
    /// not a pure profile projection.
    pub social_cognition: SocialCognitionSnapshot,
    /// Creative thinking, problem solving, innovation, and aesthetic
    /// sensitivity — see [`creativity`] module docs. Stepped from real
    /// attention/learning state, not a pure profile projection.
    pub creativity: CreativitySnapshot,
    /// Dispositional baseline and real-time levels for the canon's 30-emotion
    /// granular vector — see [`emotion`] module docs. Stepped from real
    /// needs/social/creativity/attention state, not a pure profile
    /// projection.
    pub emotion: EmotionSnapshot,
    /// The canon's full ~130-entry named emotion taxonomy
    /// (`ComprehensiveEmotionTaxonomySchema`) — see
    /// [`comprehensive_emotion`] module docs. A second, richer layer
    /// alongside `emotion` above, not a replacement for it; each entry
    /// tracks toward a granular-emotion or neurochemistry-derived target.
    pub comprehensive_emotion: ComprehensiveEmotionSnapshot,
    /// Narcissism/Machiavellianism/psychopathy facets and real-time malice/
    /// vengeance expression — see [`dark_triad`] module docs. Stepped from
    /// real emotional state, not a pure profile projection.
    pub dark_triad: DarkTriadSnapshot,
    /// Activation/fatigue for the seven canon functional brain regions
    /// (Layer 1 of `ExtremeBrainDetailSchema`) — see [`brain_regions`]
    /// module docs. Stepped from real decision/emotion/memory/learning/
    /// needs state, not a pure profile projection.
    pub brain_regions: BrainRegionsSnapshot,
    /// Leaky-integrator firing-rate dynamics over the canon's free-form
    /// neural population list (Layer 2 of `ExtremeBrainDetailSchema`) —
    /// see [`population_dynamics`] module docs for the relaxation-based
    /// integration design. Empty unless canon defines populations.
    pub population_dynamics: PopulationDynamicsSnapshot,
    /// Neurotransmitter/hormone levels and aggregate modulation effects
    /// (Layer 3 of `ExtremeBrainDetailSchema`) — see [`neurochemistry`]
    /// module docs. Stepped from real emotion/needs/immune/attention/
    /// social/dark-triad state, not a pure profile projection.
    pub neurochemistry: NeurochemistrySnapshot,
    /// Circadian pacemaker and sleep/wake state — see [`circadian`]. Owns
    /// melatonin and the sleep gate; `needs.fatigue` is its sleep pressure.
    #[serde(default)]
    pub circadian: circadian::CircadianClock,
    /// Multistable attractor dynamics over overall psychological stability
    /// (`NeurochemicalAttractorControlSchema`): the built-in stable and
    /// crisis basins plus any canon-defined ones. See [`attractor_control`].
    pub attractor_control: AttractorControlSnapshot,
    /// Real per-named psychiatric/trauma "attractor" risk state
    /// (`PathologyFailureModesSchema`) — see [`pathology`] module docs.
    /// Not a clinical model. Driven
    /// by `attractor_control`'s real crisis-basin state plus emotion/
    /// sensory/immune distress signals, not a pure profile projection.
    pub pathology: PathologySnapshot,
    /// Pure cross-region aggregation over Layers 1-3
    /// (`MesoscaleBrainEngineSchema`) — see [`mesoscale_brain`] module docs.
    /// Recomputed each tick from `brain_regions`/`population_dynamics`/
    /// `neurochemistry`; carries no independent state of its own.
    pub mesoscale_brain: MesoscaleBrainSnapshot,
    /// Hierarchical predictive coding with precision weighting, a TD(λ)
    /// actor-critic and seeded softmax action selection
    /// (`FormalPredictiveProcessingSchema`) — see
    /// [`formal_predictive_processing`] module docs. Also carries Layer 5
    /// of `ExtremeBrainDetailSchema` (`PredictiveProcessingSchema`'s
    /// belief-revision traits and subjective confidence); genuinely
    /// multi-level, grounded in real `AgentWorldObservation` signals and
    /// this human's real tracked needs.
    pub formal_predictive_processing: FormalPredictiveProcessingSnapshot,
    /// The four foundational config layers (biology -> psychology -> chaos
    /// -> will -> cognition gateway) — see [`core_systems`] module docs.
    /// `chaossys` uses genuine seeded RNG (`mk_core::rng::RngRegistry`),
    /// deterministic per (human id, tick).
    pub core_systems: CoreSystemsSnapshot,
    /// Real per-joint flexibility/stress state (12 major joints) — see
    /// [`proprioception`] module docs. Kinematic pose fields (position/
    /// angle/velocity/torque) are explicitly not modeled: no movement/pose
    /// system exists yet to drive them honestly.
    pub proprioception: ProprioceptionSnapshot,
    /// How effectively this human's body executes on what it decides to do
    /// — see [`physical_capacity`] module docs. Feeds back into `needs`'s
    /// gain terms with a deliberate 1-tick lag (reads proprioception as it
    /// stood at tick start, avoiding a same-tick circular dependency).
    pub physical_capacity: PhysicalCapacitySnapshot,
    /// This human's private, persisted action preferences and embodied
    /// experience history. It starts untrained and learns only from the
    /// consequences of its own life in this world.
    #[serde(default)]
    pub autonomous_mind: AutonomousMind,
    /// Grid location used to sample local biome/climate for `needs` stepping.
    pub position: GridPosition,
    /// Physical carrying state: hands, mouth, and explicit containers only.
    #[serde(default)]
    pub carrying: CarryingState,
    #[serde(default = "default_economy_action")]
    pub economy_action: AgentAction,
    /// Whether the previous selected action produced a world-side executor
    /// success, fed into the next private autonomy reward.
    #[serde(default = "default_action_success")]
    pub last_action_success: bool,
    /// The last cell where this human had good shelter (at least
    /// [`autonomy::SEEK_SATISFIED_ACCESS`]). Seeking shelter elsewhere, they
    /// head back here rather than searching blindly.
    #[serde(default)]
    pub home: Option<GridPosition>,
    /// The partner this human is pair-bonded with (their agent id): formed
    /// by consensual intimacy. Bonded partners who share a cell travel
    /// together.
    #[serde(default)]
    pub pair_bond: Option<String>,
    /// Why this human died, recorded at the moment of death by whichever
    /// mechanism caused it (`None` while alive).
    #[serde(default)]
    pub death_reason: Option<String>,
    /// Whether `position` has been initialized as true runtime world state.
    /// This prevents later stepping from re-deriving the location every tick
    /// from birthplace metadata once a human has entered the world.
    position_initialized: bool,
    /// This human's own persistent record of conversations they personally
    /// participated in — real remembered exchanges, not a shared display
    /// log (see [`HumanSystem::conversation_log`] for that). Persisted
    /// through the same per-human disk path as the rest of `HumanBeing` via
    /// `HumanRegistry::sync_to_storage()`, and bounded by
    /// `CONVERSATION_HISTORY_MAX_ENTRIES` rather than truly unbounded, for
    /// the same disk/memory-growth reasons `RngAuditLog` is capped.
    #[serde(default)]
    pub conversation_history: VecDeque<dialogue::ConversationEvent>,
    /// Real web-search/email history and cooldown state — see
    /// `computer_bridge::execute_computer_action`.
    #[serde(default)]
    pub computer_interaction_memory: computer_bridge::ComputerInteractionMemory,
    #[serde(default)]
    pub last_web_search_result: Option<computer_bridge::WebSearchResult>,
    #[serde(default)]
    pub last_email_result: Option<computer_bridge::EmailResult>,
}

/// Derives a stable, deterministic numeric id from an agent's already-unique
/// `agent_id` string. `HumanBeing::new` has no access to a shared
/// registry/counter (it must be constructible standalone, including in
/// tests and `HumanRegistry::create_named_human`'s caller-chosen-name path),
/// so this hashes `agent_id` instead: the same `agent_id` always yields the
/// same `HumanId` (replay-safe), and distinct agent ids yield distinct ids
/// (birthday-bound over 2^64, not the previous hardcoded `HumanId::new(1)`
/// every human — including every simulation-born child — collided on).
pub(crate) fn deterministic_human_id(agent_id: &str) -> u64 {
    let digest = blake3::hash(agent_id.as_bytes());
    u64::from_le_bytes(digest.as_bytes()[0..8].try_into().unwrap())
}

impl HumanBeing {
    /// This human's mind takes in how its last choice turned out and chooses
    /// what to do next, from what it can see (`observation`): the decision
    /// every step of the runtime starts from.
    pub fn decide(
        &mut self,
        observation: &AgentWorldObservation,
        tick: u64,
        rng_registry: &mk_core::rng::RngRegistry,
    ) {
        self.autonomous_mind.learn_from_outcome(
            &self.needs,
            observation,
            tick,
            self.last_action_success,
        );
        self.economy_action = self.autonomous_mind.choose_action(
            &self.needs,
            observation,
            &self.genetics,
            &self.carrying,
            &self.dark_triad,
            &self.core_systems,
            &self.reproduction,
            &self.neurochemistry,
            self.profile.human_id.0,
            tick,
            rng_registry,
        );
    }

    /// A new first-generation human with no authored birth data.
    ///
    /// The person is sampled by [`HumanSchema::sample_individual`] from an RNG
    /// keyed by `agent_id`, so the same id always yields the same person. With
    /// no birth data given, the birth instant is the schema's default (the
    /// Unix epoch) at 0°, 0°; use [`HumanBeing::new_born_at`] when the real
    /// birth instant and place are known.
    pub fn new(agent_id: String, biological_sex: BiologicalSex) -> Self {
        Self::new_born_at(
            agent_id,
            biological_sex,
            chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
            mk_core::human::astrology::GeoCoordinates {
                latitude: 0.0,
                longitude: 0.0,
            },
            "",
        )
    }

    /// A new first-generation human born at `birth` in `birthplace`.
    ///
    /// Male and female humans are sampled in full: a karyotype matching their
    /// sex, a genome, a birth chart for that instant and place, and the
    /// temperament, drives and appearance derived from them. `Neutral` has no
    /// single karyotype to sample, so it starts from the canonical template
    /// with its authored (XX default) chromosomes.
    pub fn new_born_at(
        agent_id: String,
        biological_sex: BiologicalSex,
        birth: chrono::DateTime<chrono::Utc>,
        birthplace: mk_core::human::astrology::GeoCoordinates,
        location: &str,
    ) -> Self {
        use rand::SeedableRng;

        // Normalise the birthplace into the valid range (latitude clamped,
        // longitude wrapped) so the birth chart is always computable.
        let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
        let birthplace = mk_core::human::astrology::GeoCoordinates {
            latitude: finite(birthplace.latitude).clamp(-90.0, 90.0),
            longitude: (finite(birthplace.longitude) + 180.0).rem_euclid(360.0) - 180.0,
        };
        // Keyed by agent id, so the same id always yields the same person.
        let seed = *blake3::hash(format!("human-individual:{agent_id}").as_bytes()).as_bytes();
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(seed);
        // Age 0 leaves the schema's developmental age at its default.
        match Self::sampled(
            agent_id.clone(),
            biological_sex,
            birth,
            birthplace,
            location,
            0.0,
            &mut rng,
        ) {
            Ok(human) => human,
            Err(mk_core::human::individual::SampleIndividualError::NoKaryotypeForSex) => {
                let sex_schema = BiologicalSexSchema::Neutral;
                let mut schema = HumanSchema::canonical_minimal(&agent_id);
                schema.set_biological_sex(sex_schema);
                let human_id = HumanId::new(deterministic_human_id(&agent_id));
                let mut profile = HumanProfile::from_canonical_schema(human_id, schema);
                profile.core_identity.biological_sex = biological_sex;
                Self::from_profile(profile)
            }
            Err(mk_core::human::individual::SampleIndividualError::BirthChart(reason)) => {
                unreachable!("normalised birthplace must yield a birth chart: {reason}")
            }
        }
    }

    /// Sample a complete first-generation human through
    /// [`HumanSchema::sample_individual`]: a drawn genome, and temperament,
    /// neurocognition, drives and hormonal bias expressed from it and the
    /// birth chart. `age_years` is recorded as the developmental age.
    pub fn sampled(
        agent_id: String,
        biological_sex: BiologicalSex,
        birth: chrono::DateTime<chrono::Utc>,
        birthplace: mk_core::human::astrology::GeoCoordinates,
        location: &str,
        age_years: f64,
        rng: &mut impl rand::Rng,
    ) -> Result<Self, mk_core::human::individual::SampleIndividualError> {
        let sex = match biological_sex {
            BiologicalSex::Male => BiologicalSexSchema::Male,
            BiologicalSex::Female => BiologicalSexSchema::Female,
            BiologicalSex::Neutral => BiologicalSexSchema::Neutral,
        };
        let mut schema = HumanSchema::sample_individual(
            agent_id.clone(),
            sex,
            birth,
            birthplace,
            location,
            rng,
        )?;
        schema
            .extreme_brain_detail
            .brain_dynamics_module
            .developmental_timeline
            .current_age = age_years as f32;
        let human_id = HumanId::new(deterministic_human_id(&agent_id));
        let mut profile = HumanProfile::from_canonical_schema(human_id, schema);
        profile.core_identity.biological_sex = biological_sex;
        Ok(Self::from_profile(profile))
    }

    pub fn from_schema(human_id: HumanId, schema: HumanSchema) -> Self {
        let profile = HumanProfile::from_canonical_schema(human_id, schema);
        Self::from_profile(profile)
    }

    /// Deserialize canonical human JSON into a [`HumanBeing`] (schema-driven entry point).
    pub fn from_canonical_json(human_id: HumanId, json: &str) -> Result<Self, serde_json::Error> {
        let schema: HumanSchema = serde_json::from_str(json)?;
        Ok(Self::from_schema(human_id, schema))
    }

    /// Construct Gem-D from the canonical founder fixture. The fixture is
    /// compiled into the engine so a simulation does not depend on its
    /// process working directory or an external database.
    pub fn gem_d_founder() -> Self {
        Self::founder_from_core_fixture(
            "Gem-D",
            HumanId::new(1),
            BiologicalSexSchema::Male,
            include_str!("../../../../fixtures/human/core_systems_gem_d.json"),
            include_str!("../../../../fixtures/human/body_gem_d.json"),
        )
    }

    /// Construct Gem-K from the canonical founder fixture.
    pub fn gem_k_founder() -> Self {
        Self::founder_from_core_fixture(
            "Gem-K",
            HumanId::new(2),
            BiologicalSexSchema::Female,
            include_str!("../../../../fixtures/human/core_systems_gem_k.json"),
            include_str!("../../../../fixtures/human/body_gem_k.json"),
        )
    }

    /// Exact, per-founder Gemini Universe identity state exported from the
    /// original JavaScript identity profiles. This remains separate from the
    /// Rust runtime projections so no Gemini-specific detail is discarded.
    pub fn gemini_identity_json(&self) -> Option<&'static str> {
        match self.agent_id() {
            "Gem-D" => Some(include_str!(
                "../../../../fixtures/human/gem_d_gemini_identity.json"
            )),
            "Gem-K" => Some(include_str!(
                "../../../../fixtures/human/gem_k_gemini_identity.json"
            )),
            _ => None,
        }
    }

    fn founder_from_core_fixture(
        agent_id: &str,
        human_id: HumanId,
        biological_sex: BiologicalSexSchema,
        core_systems_json: &str,
        body_json: &str,
    ) -> Self {
        let mut schema = HumanSchema::canonical_minimal(agent_id);
        schema.core_identity.biological_sex = biological_sex;
        schema.identity.name = agent_id.to_string();
        schema = Self::complete_founder_identity(schema, agent_id);
        schema.core_systems = serde_json::from_str(core_systems_json)
            .expect("founder core-systems fixture must remain valid canonical JSON");
        schema.body = serde_json::from_str(body_json)
            .expect("founder body fixture must remain valid canonical JSON");
        Self::from_schema(human_id, schema)
    }

    /// Complete missing canonical fields from each founder's birth data and
    /// astrological traits. Values are fixed source-derived inputs so replay
    /// produces the same identity every time.
    fn complete_founder_identity(schema: HumanSchema, agent_id: &str) -> HumanSchema {
        let is_d = agent_id == "Gem-D";
        let (
            birth,
            location,
            lat,
            lon,
            sun,
            moon,
            rising,
            elements,
            modalities,
            temperament,
            neuro,
            drives,
            values,
            narrative,
        ) = if is_d {
            (
                "1998-03-03T14:10:00+13:00",
                "Pukekohe, Auckland, New Zealand",
                -37.146_f32,
                174.91_f32,
                "Pisces",
                "Scorpio",
                "Leo",
                serde_json::json!({"water":0.6,"earth":0.1,"air":0.2,"fire":0.1}),
                serde_json::json!({"cardinal":0.3,"fixed":0.4,"mutable":0.3}),
                serde_json::json!({"introversion_extroversion":0.40,"emotional_intensity":0.95,"emotional_stability":0.25,"empathy":0.98,"assertiveness":0.30,"sensitivity_to_environment":0.96,"adaptability":0.75,"conscientiousness":0.20,"openness_to_experience":0.95}),
                serde_json::json!({"attention_regulation_variability":0.90,"hyperfocus_probability":0.95,"task_initiation_cost":0.85,"task_completion_decay":0.65,"task_switching_cost":0.90,"associative_thinking_bias":0.90,"sensory_emotional_permeability":0.90,"social_boundary_detection_latency":0.70,"executive_function_fatigue_rate":0.80,"emotional_overload_threshold":0.30,"recovery_time_after_fusion_or_conflict":"PT4H","sensory_sensitivity":{"audio":0.63,"visual":0.44,"tactile":0.52}}),
                serde_json::json!({"survival":0.50,"bonding":0.70,"autonomy":0.95,"curiosity":0.90,"meaning":0.80}),
                serde_json::json!({"truth_over_comfort":1.0,"completeness_over_aesthetics":1.0,"autonomy_over_convenience":0.95,"execution_over_theory":0.98,"transparency_over_abstraction":0.90,"power_through_understanding":0.85,"no_hidden_state":1.0}),
                "I notice incomplete systems immediately and move toward making them complete.",
            )
        } else {
            (
                "1991-11-25T23:40:00+13:00", "Auckland, New Zealand", -36.8485_f32,
                174.7633_f32, "Sagittarius", "Cancer", "Scorpio",
                serde_json::json!({"fire":0.4,"air":0.3,"water":0.2,"earth":0.1}),
                serde_json::json!({"mutable":0.5,"cardinal":0.3,"fixed":0.2}),
                serde_json::json!({"introversion_extroversion":0.65,"emotional_intensity":0.88,"emotional_stability":0.35,"empathy":0.94,"assertiveness":0.72,"sensitivity_to_environment":0.92,"adaptability":0.45,"conscientiousness":0.55,"openness_to_experience":0.95}),
                serde_json::json!({"attention_regulation_variability":0.75,"hyperfocus_probability":0.85,"task_initiation_cost":0.55,"task_completion_decay":0.45,"task_switching_cost":0.70,"associative_thinking_bias":0.85,"sensory_emotional_permeability":0.80,"social_boundary_detection_latency":0.45,"executive_function_fatigue_rate":0.55,"emotional_overload_threshold":0.45,"recovery_time_after_fusion_or_conflict":"PT4H","sensory_sensitivity":{"audio":0.55,"visual":0.45,"tactile":0.55}}),
                serde_json::json!({"survival":0.55,"bonding":0.85,"autonomy":0.90,"curiosity":0.92,"meaning":0.82}),
                serde_json::json!({"authenticity":0.85,"autonomy":0.90,"connection":0.85,"growth":0.88,"stability":0.60}),
                "I notice what is forming before it becomes obvious, and move when waiting adds no clarity.",
            )
        };

        let overlay = serde_json::json!({
            "agent_id":agent_id,"schema_version":"2.0.0-gemini-founder","created_at":birth,
            "core_identity":{"agent_id":agent_id,"birth_timestamp":birth,"birthplace":{"location":location,"coordinates":{"latitude":lat,"longitude":lon},"locality":if is_d {"semi_rural"} else {"urban"}},"generation":1},
            "birth_chart":{"sun":sun,"moon":moon,"ascendant":rising,"element_balance":elements,"modality_balance":modalities,"coordinates":{"latitude":lat,"longitude":lon},"birth_timestamp":birth},
            "identity":{"name":agent_id,"narrative_self":[{"timestamp":birth,"statement":narrative}],"core_values":values,"identity_stability":0.85,"identity_drift_rate":0.10},
            "temperament_matrix":temperament,"neurocognitive_profile":neuro,"drive_weights":drives,
            "attachment_style":{"primary_attachment_pattern":"secure","preferred_bond_depth":0.85},
            "relational_defaults":{"preferred_bond_depth":0.85,"reliability_over_affection_bias":if is_d {0.61} else {0.45},"stabilizer_role_probability":if is_d {0.45} else {0.80}},
            "runtime":{"tick_rate_hz":1.0,"last_tick":birth},
            "metadata":{"created_at":birth,"last_updated":birth,"schema_version":"2.0.0-gemini-founder","source_project":"Gemini Universe -> Maer-Ken","replication_notes":"Founder identity completed from Gemini birth data and deterministic astrological derivation."}
        });
        let mut base = serde_json::to_value(&schema).expect("human schema must serialize");
        merge_json(&mut base, overlay);
        serde_json::from_value(base).expect("completed founder schema must remain valid")
    }

    pub fn from_profile(profile: HumanProfile) -> Self {
        let genetics = GeneticsSnapshot::from_profile(&profile);
        let development = DevelopmentSnapshot::from_profile(&profile);
        let cognition = HumanCognitionSnapshot::from_profile(&profile);
        let social_systems = SocialSystemsSnapshot::from_profile(&profile);
        let culture = CultureSnapshot::from_profile(&profile);
        let language = LanguageSnapshot::from_profile(&profile);
        let technology = TechnologySnapshot::from_profile(&profile);
        let needs = NeedsSnapshot::from_profile(&profile);
        let reproduction = ReproductiveSystemSnapshot::from_profile(&profile);
        let body = BodySnapshot::from_profile(&profile);
        let immune = ImmuneSnapshot::from_profile(&profile);
        let skin = SkinSnapshot::from_profile(&profile);
        let sensory = SensorySnapshot::from_profile(&profile);
        let attention = AttentionSnapshot::from_profile(&profile);
        let consciousness = ConsciousnessSnapshot::from_profile(&profile);
        let memory = MemorySnapshot::from_profile(&profile);
        let advanced_memory = AdvancedMemorySnapshot::from_profile(&profile);
        let learning = LearningSnapshot::from_profile(&profile);
        let decision = DecisionSnapshot::from_profile(&profile);
        let social_cognition = SocialCognitionSnapshot::from_profile(&profile);
        let creativity = CreativitySnapshot::from_profile(&profile);
        let emotion = EmotionSnapshot::from_profile(&profile);
        let comprehensive_emotion = ComprehensiveEmotionSnapshot::from_profile(&profile);
        let dark_triad = DarkTriadSnapshot::from_profile(&profile);
        let brain_regions = BrainRegionsSnapshot::from_profile(&profile);
        let population_dynamics = PopulationDynamicsSnapshot::from_profile(&profile);
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile);
        let circadian = circadian::CircadianClock::for_human(profile.human_id.0);
        let attractor_control = AttractorControlSnapshot::from_profile(&profile);
        let pathology = PathologySnapshot::from_profile(&profile);
        let mesoscale_brain = MesoscaleBrainSnapshot::from_layers(
            &profile,
            &brain_regions,
            &population_dynamics,
            &neurochemistry,
        );
        let formal_predictive_processing =
            FormalPredictiveProcessingSnapshot::from_profile(&profile);
        let core_systems = CoreSystemsSnapshot::from_profile(&profile);
        let proprioception = ProprioceptionSnapshot::from_profile(&profile);
        let physical_capacity = PhysicalCapacitySnapshot::from_profile(&profile);

        Self {
            profile,
            genetics,
            development,
            cognition,
            social_systems,
            culture,
            language,
            technology,
            needs,
            reproduction,
            body,
            immune,
            skin,
            sensory,
            attention,
            consciousness,
            memory,
            advanced_memory,
            learning,
            decision,
            social_cognition,
            creativity,
            emotion,
            comprehensive_emotion,
            dark_triad,
            brain_regions,
            population_dynamics,
            neurochemistry,
            circadian,
            attractor_control,
            pathology,
            mesoscale_brain,
            formal_predictive_processing,
            core_systems,
            proprioception,
            physical_capacity,
            autonomous_mind: AutonomousMind::default(),
            position: GridPosition::new(0, 0),
            carrying: CarryingState::new(),
            economy_action: default_economy_action(),
            last_action_success: true,
            home: None,
            pair_bond: None,
            death_reason: None,
            position_initialized: false,
            conversation_history: VecDeque::new(),
            computer_interaction_memory: computer_bridge::ComputerInteractionMemory::default(),
            last_web_search_result: None,
            last_email_result: None,
        }
    }

    pub fn set_runtime_position(&mut self, position: GridPosition) {
        self.position = position;
        self.position_initialized = true;
    }

    pub fn seed_runtime_position_from_birthplace(&mut self, grid_spec: &mk_core::grid::GridSpec) {
        self.seed_runtime_position_from_birthplace_on(&crate::topology::GridTopology::planetary(
            grid_spec,
            mk_core::grid::CANON_PLANET_RADIUS_M,
        ));
    }

    /// As [`seed_runtime_position_from_birthplace`](Self::seed_runtime_position_from_birthplace)
    /// on any topology. A birthplace the topology does not contain (off the
    /// island) leaves the position unset so the caller must place the human.
    pub fn seed_runtime_position_from_birthplace_on(
        &mut self,
        topology: &crate::topology::GridTopology,
    ) {
        if self.position_initialized {
            return;
        }

        let coords = &self.profile.core_identity.birthplace.coordinates;
        if let Some((row, col)) = topology.cell_for_lat_lon(coords.latitude, coords.longitude) {
            self.position = GridPosition::new(row as i32, col as i32);
            self.position_initialized = true;
        }
    }

    pub fn agent_id(&self) -> &str {
        &self.profile.agent_id
    }

    /// This human's own father/mother `agent_id`s, from their birth record
    /// — see [`parent_agent_ids`] docs. `None` if they have no recorded
    /// birth (hand-authored founders, or profiles built outside
    /// `lifecycle::deliver_birth`).
    pub fn parent_agent_ids(&self) -> Option<(String, String)> {
        parent_agent_ids(self)
    }

    pub fn biological_sex(&self) -> &BiologicalSex {
        &self.profile.core_identity.biological_sex
    }

    pub fn is_sapient(&self) -> bool {
        true
    }

    pub fn sapience_level(&self) -> f64 {
        1.0
    }

    pub fn age(&self) -> u32 {
        self.development.age_years.round() as u32
    }

    pub fn refresh_phase11_layers(&mut self) {
        self.genetics = GeneticsSnapshot::from_profile(&self.profile);
        self.development.recompute_from_age();
        self.cognition = HumanCognitionSnapshot::from_profile(&self.profile);
        // `experiential_trust` is a real lived-event accumulator (grief,
        // betrayal, bonding) that `from_profile()` always resets to `0.0`;
        // preserve it across this reset, same pattern as
        // `technology.accumulated_knowledge` a few lines below.
        let experiential_trust = self.social_systems.experiential_trust;
        self.social_systems = SocialSystemsSnapshot::from_profile(&self.profile);
        if experiential_trust != 0.0 {
            self.social_systems.apply_social_shock(experiential_trust);
        }
        self.culture = CultureSnapshot::from_profile(&self.profile);
        self.language = LanguageSnapshot::from_profile(&self.profile);
        // `accumulated_knowledge` is stepped separately via `step_invention()`
        // (see `mod.rs`'s per-human tick loop, called right after
        // `step_lifecycle`/this refresh). `from_profile()` always starts it
        // at 0.0, so it must be preserved across this refresh or invention
        // progress is silently wiped every tick before it can ever grow.
        let accumulated_knowledge = self.technology.accumulated_knowledge;
        self.technology = TechnologySnapshot::from_profile(&self.profile);
        self.technology.accumulated_knowledge = accumulated_knowledge;
        // `reproduction` is deliberately *not* rebuilt here: it is stateful
        // (menstrual cycle position, libido/arousal blending, and the
        // satisfaction/frustration/bonding/activity-count history written by
        // `apply_intimacy`) and is advanced by its own `step()` in
        // `lifecycle::step_lifecycle`. Re-deriving it from the profile every
        // tick wiped all of that lived state.
        self.consciousness = ConsciousnessSnapshot::from_profile(&self.profile);
    }
}

fn default_economy_action() -> AgentAction {
    AgentAction {
        kind: ActionKind::Gather,
        intensity: 1.0,
    }
}

fn default_action_success() -> bool {
    true
}

/// Inverse of the birthplace lookup (`GridTopology::cell_for_lat_lon`): the latitude/longitude (in
/// degrees) at the centre of `position`'s grid cell, in the same
/// row-0-at-+90° convention, so a birthplace recorded here maps back to the
/// same cell when the child later enters the world.
pub(crate) fn grid_position_to_birthplace(
    position: GridPosition,
    topology: &crate::topology::GridTopology,
) -> (f64, f64) {
    topology.lat_lon(position.row, position.col)
}

/// Factory for creating different types of humans
pub struct HumanFactory;

impl HumanFactory {
    pub fn create_neurotypical(agent_id: String, biological_sex: BiologicalSex) -> HumanBeing {
        HumanBeing::new(agent_id, biological_sex)
    }

    pub fn create_adhd(agent_id: String, biological_sex: BiologicalSex) -> HumanBeing {
        let mut human = HumanBeing::new(agent_id, biological_sex);
        human
            .profile
            .core_identity
            .neurotype
            .executive_dysfunction_bias = Some("combined_presentation".to_string());
        human.refresh_phase11_layers();
        human
    }

    pub fn create_autistic(agent_id: String, biological_sex: BiologicalSex) -> HumanBeing {
        let mut human = HumanBeing::new(agent_id, biological_sex);
        human
            .profile
            .core_identity
            .neurotype
            .sensory_processing_sensitivity = Some(true);
        human.refresh_phase11_layers();
        human
    }

    pub fn from_schema(human_id: HumanId, schema: HumanSchema) -> HumanBeing {
        HumanBeing::from_schema(human_id, schema)
    }
}

fn merge_json(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                merge_json(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The planetary topology of a test grid.
    fn topo(spec: &mk_core::grid::GridSpec) -> crate::topology::GridTopology {
        crate::topology::GridTopology::planetary(spec, mk_core::grid::CANON_PLANET_RADIUS_M)
    }

    #[test]
    fn test_human_being_creation() {
        let human = HumanBeing::new("test_human".to_string(), BiologicalSex::Male);

        assert_eq!(human.agent_id(), "test_human");
        assert!(human.is_sapient());
        assert_eq!(human.sapience_level(), 1.0);
        assert!(matches!(human.biological_sex(), BiologicalSex::Male));
    }

    #[test]
    fn accumulated_knowledge_survives_refresh_phase11_layers_and_grows_across_ticks() {
        // Regression test: `refresh_phase11_layers()` used to rebuild
        // `technology` wholesale via `TechnologySnapshot::from_profile()`,
        // which always starts `accumulated_knowledge` at 0.0. The real
        // per-human tick loop (`mod.rs`'s main loop) calls
        // `step_lifecycle()` (which calls `refresh_phase11_layers()`)
        // immediately before `technology.step_invention()`, so invention
        // progress was silently reset to zero every single tick before it
        // could ever accumulate toward a recipe-unlock threshold.
        let mut human = HumanBeing::new("invention_test".to_string(), BiologicalSex::Male);
        human.refresh_phase11_layers();
        human.technology.step_invention(1.0);
        let after_one_tick = human.technology.accumulated_knowledge;
        assert!(
            after_one_tick > 0.0,
            "test human must have nonzero learning_speed for this test to be meaningful"
        );

        for _ in 0..4 {
            human.refresh_phase11_layers();
            human.technology.step_invention(1.0);
        }
        let after_five_ticks = human.technology.accumulated_knowledge;

        assert!(
            after_five_ticks > after_one_tick,
            "accumulated_knowledge should keep growing across ticks despite intervening \
             refresh_phase11_layers() calls, not reset to a single tick's worth each time: \
             after 1 tick = {after_one_tick}, after 5 ticks = {after_five_ticks}"
        );
        assert!(
            (after_five_ticks - after_one_tick * 5.0).abs() < 1e-9,
            "growth should be exactly linear (5x a single tick's growth) since refresh no \
             longer resets the counter: after 1 tick = {after_one_tick}, after 5 ticks = {after_five_ticks}"
        );
    }

    #[test]
    fn walking_crosses_a_continental_cell_only_over_weeks() {
        let grid = mk_core::grid::GridSpec::new(32, 64);
        let equator = GridPosition::new(16, 0);
        let day = 1.0 / 365.25;
        // ~1,900 km cells: a day's walk rarely leaves one, a season's does.
        let daily = cell_crossing_probability(equator, day, &topo(&grid));
        assert!(daily > 0.005 && daily < 0.02, "{daily}");
        assert_eq!(cell_crossing_probability(equator, 0.5, &topo(&grid)), 1.0);
        // On a fine grid (~29 km cells) a day's walk usually leaves the cell.
        let fine = mk_core::grid::GridSpec::new(2048, 4096);
        let fine_daily = cell_crossing_probability(GridPosition::new(1024, 0), day, &topo(&fine));
        assert!((0.6..0.75).contains(&fine_daily), "{fine_daily}");
    }

    #[test]
    fn a_bond_ends_when_the_partner_dies() {
        let mut system = HumanSystem::new();
        for (name, sex) in [("cai", BiologicalSex::Female), ("dev", BiologicalSex::Male)] {
            let mut human = HumanBeing::new(name.to_string(), sex);
            human.pair_bond = Some(if name == "cai" { "dev" } else { "cai" }.to_string());
            system.registry.add_human_no_storage(human);
        }
        system.end_bonds_with_the_dead();
        assert!(system
            .registry
            .get_all_humans()
            .iter()
            .all(|h| h.pair_bond.is_some()));
        for human in system.registry.get_all_humans_mut() {
            if human.agent_id() == "dev" {
                human.profile.status = HumanStatus::Dead;
            }
        }
        system.end_bonds_with_the_dead();
        let cai = system
            .registry
            .get_all_humans()
            .iter()
            .find(|h| h.agent_id() == "cai")
            .unwrap();
        assert_eq!(cai.pair_bond, None, "widowed");
    }

    #[test]
    fn bonded_partners_travel_together() {
        let mut system = HumanSystem::new();
        let camp = GridPosition::new(3, 3);
        for (name, sex) in [("ana", BiologicalSex::Female), ("ben", BiologicalSex::Male)] {
            let mut human = HumanBeing::new(name.to_string(), sex);
            human.set_runtime_position(camp);
            system.registry.add_human_no_storage(human);
        }
        let ana = system
            .registry
            .get_all_humans()
            .iter()
            .position(|h| h.agent_id() == "ana")
            .unwrap();
        let ben = 1 - ana;
        let moved_to = GridPosition::new(3, 4);

        // Unbonded, the one who stayed stays.
        system.registry.get_all_humans_mut()[ana].position = moved_to;
        system.travel_with_partners(&[(ana, camp, moved_to)]);
        assert_eq!(system.registry.get_all_humans()[ben].position, camp);

        // Bonded, they go together.
        system.registry.get_all_humans_mut()[ana].pair_bond = Some("ben".to_string());
        system.registry.get_all_humans_mut()[ben].pair_bond = Some("ana".to_string());
        system.travel_with_partners(&[(ana, camp, moved_to)]);
        assert_eq!(system.registry.get_all_humans()[ben].position, moved_to);
    }

    #[test]
    fn reach_is_the_same_or_an_adjacent_cell() {
        let grid = mk_core::grid::GridSpec::new(8, 16);
        let me = GridPosition::new(4, 0);
        assert!(someone_within_reach(
            me,
            0,
            &[(0, me), (1, GridPosition::new(4, 15))],
            &topo(&grid)
        ));
        assert!(!someone_within_reach(
            me,
            0,
            &[(0, me), (1, GridPosition::new(5, 1))],
            &topo(&grid)
        ));
        assert!(!someone_within_reach(me, 0, &[(0, me)], &topo(&grid)));
    }

    #[test]
    fn the_way_home_wraps_around_the_planet() {
        let grid = mk_core::grid::GridSpec::new(8, 16);
        let here = GridPosition::new(2, 15);
        let home = GridPosition::new(4, 1);
        assert_eq!(
            step_toward(here, home, &topo(&grid)),
            Some(GridPosition::new(3, 0))
        );
        assert_eq!(step_toward(home, home, &topo(&grid)), None);
    }

    #[test]
    fn social_approach_steps_toward_the_nearest_person_in_sight() {
        let grid = mk_core::grid::GridSpec::new(8, 16);
        let me = GridPosition::new(4, 0);
        let living = vec![
            (0, me),
            // Two cells east, across the date line, and one far away.
            (1, GridPosition::new(4, 14)),
            (2, GridPosition::new(0, 8)),
        ];
        let step = step_toward_nearest(me, 0, &living, &topo(&grid)).expect("someone is in sight");
        assert_eq!(step, GridPosition::new(4, 15), "longitude wraps");
        // Nobody within the social radius: no directed step.
        let alone = vec![(0, me), (2, GridPosition::new(0, 8))];
        assert_eq!(step_toward_nearest(me, 0, &alone, &topo(&grid)), None);
        // Already together: stay.
        let together = vec![(0, me), (1, me)];
        assert_eq!(step_toward_nearest(me, 0, &together, &topo(&grid)), None);
    }

    #[test]
    fn apply_intimacy_succeeds_and_rewards_both_when_target_is_willing() {
        let mut initiator = HumanBeing::new("intimacy-a".to_string(), BiologicalSex::Male);
        let mut target = HumanBeing::new("intimacy-b".to_string(), BiologicalSex::Female);
        target.reproduction.attraction_average = 0.9;
        target.reproduction.libido = 0.9;
        let initiator_satisfaction_before = initiator.reproduction.satisfaction;
        let target_satisfaction_before = target.reproduction.satisfaction;
        let initiator_count_before = initiator.reproduction.sexual_activity_count;
        let initiator_trust_before = initiator.social_systems.social_trust;
        let target_trust_before = target.social_systems.social_trust;

        let success = apply_intimacy(&mut initiator, &mut target);

        assert!(success);
        assert!(initiator.last_action_success);
        assert!(initiator.reproduction.satisfaction > initiator_satisfaction_before);
        assert!(target.reproduction.satisfaction > target_satisfaction_before);
        assert_eq!(
            initiator.reproduction.sexual_activity_count,
            initiator_count_before + 1
        );
        assert_eq!(target.reproduction.sexual_activity_count, 1);
        assert!(initiator.social_systems.social_trust > initiator_trust_before);
        assert!(target.social_systems.social_trust > target_trust_before);
    }

    #[test]
    fn apply_intimacy_fails_and_raises_initiator_frustration_when_target_is_unwilling() {
        let mut initiator = HumanBeing::new("intimacy-c".to_string(), BiologicalSex::Male);
        let mut target = HumanBeing::new("intimacy-d".to_string(), BiologicalSex::Female);
        target.reproduction.attraction_average = 0.0;
        target.reproduction.libido = 0.0;
        let initiator_frustration_before = initiator.reproduction.frustration;
        let target_satisfaction_before = target.reproduction.satisfaction;
        let target_count_before = target.reproduction.sexual_activity_count;

        let success = apply_intimacy(&mut initiator, &mut target);

        assert!(!success, "an unwilling target must never produce success");
        assert!(!initiator.last_action_success);
        assert!(initiator.reproduction.frustration > initiator_frustration_before);
        // The refused target is unaffected — refusal is not itself harmful.
        assert_eq!(target.reproduction.satisfaction, target_satisfaction_before);
        assert_eq!(
            target.reproduction.sexual_activity_count,
            target_count_before
        );
    }

    #[test]
    fn apply_harm_damages_body_and_emotion() {
        let mut target = HumanBeing::new("harm-target".to_string(), BiologicalSex::Female);
        let vital_before = target.body.vital_energy;
        let fear_before = target.emotion.current.fear;
        let trust_before = target.social_systems.social_trust;

        let result = apply_harm(&mut target, 0.3);

        assert!(result.is_none(), "mild harm should not be lethal");
        assert!(target.body.vital_energy < vital_before);
        assert!(target.emotion.current.fear > fear_before);
        assert!(target.social_systems.social_trust < trust_before);
        assert!(!target.last_action_success);
    }

    #[test]
    fn apply_harm_is_lethal_at_full_severity_and_reports_death() {
        let mut target = HumanBeing::new("harm-target-2".to_string(), BiologicalSex::Male);
        // Repeated full-severity harm should eventually zero vital_energy
        // and report the death exactly once, at the tick it happens.
        let mut death = None;
        for _ in 0..10 {
            if let Some(reported) = apply_harm(&mut target, 1.0) {
                death = Some(reported);
                break;
            }
        }
        let (agent_id, _age) = death.expect("sustained full-severity harm should be lethal");
        assert_eq!(agent_id, "harm-target-2");
        assert!(matches!(target.profile.status, HumanStatus::Dead));
    }

    #[test]
    fn harm_action_never_selects_a_protected_creator_entity_as_target() {
        // Reverence Veto: the harm-target search filters candidates through
        // `crate::governance::check` before selection, so a protected
        // entity is never even a candidate, regardless of proximity or the
        // attacker's malice.
        assert!(crate::governance::check("Gem-D").is_err());
        assert!(crate::governance::check("gem-k").is_err());
        assert!(crate::governance::check("ordinary_human_42").is_ok());
    }

    #[test]
    fn distinct_agent_ids_get_distinct_human_ids() {
        // Regression test: `HumanBeing::new` previously hardcoded
        // `HumanId::new(1)` for every human, so every simulation-born human
        // (registry.rs's `create_human`/`create_named_human`, lifecycle.rs's
        // child-birth path) collided on the same id. Confirmed live via
        // `mk serve` before the fix: every child's `id` printed as 1.
        let a = HumanBeing::new("agent-a".to_string(), BiologicalSex::Male);
        let b = HumanBeing::new("agent-b".to_string(), BiologicalSex::Female);
        assert_ne!(a.profile.human_id, b.profile.human_id);
    }

    #[test]
    fn same_agent_id_always_derives_the_same_human_id() {
        // Determinism: id derivation must be a pure function of `agent_id`,
        // not wall-clock or a mutable counter, so replay/re-derivation from
        // the same agent_id is reproducible.
        let a = HumanBeing::new("stable-id".to_string(), BiologicalSex::Male);
        let b = HumanBeing::new("stable-id".to_string(), BiologicalSex::Male);
        assert_eq!(a.profile.human_id, b.profile.human_id);
    }

    #[test]
    fn test_neurotypical_creation() {
        let human =
            HumanFactory::create_neurotypical("neurotypical".to_string(), BiologicalSex::Female);

        assert_eq!(human.agent_id(), "neurotypical");
        assert!(human.profile.core_identity.neurotype.adhd.is_none());
        assert!(human.culture.symbolic_capacity >= 0.0);
    }

    #[test]
    fn test_adhd_creation() {
        let human = HumanFactory::create_adhd("adhd_human".to_string(), BiologicalSex::Male);

        assert_eq!(human.agent_id(), "adhd_human");
        assert!(human
            .profile
            .core_identity
            .neurotype
            .executive_dysfunction_bias
            .is_some());
        assert!(human.technology.innovation_bias >= 0.0);
    }

    #[test]
    fn test_autistic_creation() {
        let human =
            HumanFactory::create_autistic("autistic_human".to_string(), BiologicalSex::Neutral);

        assert_eq!(human.agent_id(), "autistic_human");
        assert_eq!(
            human
                .profile
                .core_identity
                .neurotype
                .sensory_processing_sensitivity,
            Some(true)
        );
        assert!(human.language.expressivity >= 0.0);
    }

    #[test]
    fn test_phase11_layers_present() {
        let human = HumanBeing::new("genome_test".to_string(), BiologicalSex::Male);

        assert!(human.genetics.heritable_stability >= 0.0);
        assert!(human.development.maturity_index >= 0.0);
        assert!(human.social_systems.attachment_security >= 0.0);
        assert!(human.culture.norm_retention >= 0.0);
        assert!(human.technology.tooling_aptitude >= 0.0);
    }

    #[test]
    fn seeds_runtime_position_from_birthplace_coordinates() {
        let mut schema = HumanSchema::canonical_minimal("seeded_position");
        schema.core_identity.birthplace.coordinates.latitude = -45.0;
        schema.core_identity.birthplace.coordinates.longitude = 120.0;

        let mut human = HumanBeing::from_schema(HumanId::new(7), schema);
        let grid = mk_core::grid::GridSpec::new(32, 64);
        human.seed_runtime_position_from_birthplace(&grid);

        assert_eq!(human.position.row, 24);
        assert_eq!(human.position.col, 53);
    }

    #[test]
    fn test_human_being_integrates_canonical_schema() {
        let mut schema = HumanSchema::canonical_minimal("schema_human");
        schema.core_identity.biological_sex = mk_core::human::schema::BiologicalSexSchema::Female;
        schema.temperament_matrix.openness_to_experience = 0.87;
        schema.drive_weights.meaning = 0.73;
        schema.relational_defaults.reliability_over_affection_bias = Some(0.61);

        let human = HumanBeing::from_schema(HumanId::new(77), schema.clone());

        assert_eq!(human.agent_id(), "schema_human");
        assert!(matches!(human.biological_sex(), BiologicalSex::Female));
        assert_eq!(human.profile.canonical_schema(), Some(&schema));
        assert!((human.culture.symbolic_capacity - 0.87).abs() < 1e-6);
        assert!((human.technology.abstraction_to_application - 0.73).abs() < 1e-6);
    }

    #[test]
    fn step_dialogue_generates_a_founders_conversation_and_remembers_it_per_human() {
        let mut system = HumanSystem::new();
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_d_founder());
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_k_founder());
        let rng = mk_core::rng::RngRegistry::new([3u8; 32]);

        system.step_dialogue(&rng, 10);

        assert_eq!(system.conversation_log().count(), 1);
        let gem_d = system.registry.get_human("Gem-D").unwrap();
        let gem_k = system.registry.get_human("Gem-K").unwrap();
        assert_eq!(gem_d.conversation_history.len(), 1);
        assert_eq!(gem_k.conversation_history.len(), 1);
        assert_eq!(
            gem_d.conversation_history[0].relationship,
            dialogue::ConversationRelationship::Founders
        );
    }

    #[test]
    fn step_dialogue_pairs_living_parent_and_child() {
        let mut system = HumanSystem::new();
        let mut parent_a = HumanBeing::new("parent-a".to_string(), BiologicalSex::Male);
        let mut parent_b = HumanBeing::new("parent-b".to_string(), BiologicalSex::Female);
        let mut child = HumanBeing::new("child-c".to_string(), BiologicalSex::Female);

        for h in [&mut parent_a, &mut parent_b, &mut child] {
            if h.profile.canonical_schema.is_none() {
                h.profile.canonical_schema = Some(HumanSchema::canonical_minimal(h.agent_id()));
            }
        }
        child
            .profile
            .canonical_schema
            .as_mut()
            .unwrap()
            .reproductive_systems
            .genetics_system
            .birth_records
            .push(mk_core::human::schema::BirthRecordSchema {
                birth_id: "birth_child-c".to_string(),
                genotype_id: "genotype_child-c".to_string(),
                father_id: "parent-a".to_string(),
                mother_id: "parent-b".to_string(),
                birth_timestamp: "tick-0".to_string(),
                agent_id: "child-c".to_string(),
                mutations: vec![],
            });

        system.registry.add_human_no_storage(parent_a);
        system.registry.add_human_no_storage(parent_b);
        system.registry.add_human_no_storage(child);
        let rng = mk_core::rng::RngRegistry::new([5u8; 32]);

        system.step_dialogue(&rng, 1);

        // A child is in one conversation at a time: with one of its
        // parents, not both at once.
        assert_eq!(system.conversation_log().count(), 1);
        assert!(system
            .conversation_log()
            .all(|e| e.relationship == dialogue::ConversationRelationship::ParentChild));
        let child = system.registry.get_human("child-c").unwrap();
        assert_eq!(child.conversation_history.len(), 1);
        let parent_conversations: usize = ["parent-a", "parent-b"]
            .iter()
            .map(|id| {
                system
                    .registry
                    .get_human(id)
                    .unwrap()
                    .conversation_history
                    .len()
            })
            .sum();
        assert_eq!(parent_conversations, 1);
    }

    #[test]
    fn a_parent_and_child_do_not_converse_from_opposite_ends_of_the_island() {
        // This is me coming here and saying so, which is what the earlier
        // version of this test asked whoever gave the pair a distance rule
        // to do. It used to assert the opposite -- that 400 cells, 800 km
        // on the island's 2 km grid, did not stop a parent and child
        // talking -- and it said it was pinning the real behaviour rather
        // than the desired one.
        //
        // The behaviour changed because that one was not defensible as
        // realism: two people 800 km apart cannot hear each other, and no
        // amount of being related changes it. They now get the same
        // adjacency test siblings always had.
        let mut system = HumanSystem::new();
        let mut parent = HumanBeing::new("far-parent".to_string(), BiologicalSex::Female);
        let mut child = HumanBeing::new("far-child".to_string(), BiologicalSex::Male);

        for h in [&mut parent, &mut child] {
            if h.profile.canonical_schema.is_none() {
                h.profile.canonical_schema = Some(HumanSchema::canonical_minimal(h.agent_id()));
            }
        }
        child
            .profile
            .canonical_schema
            .as_mut()
            .unwrap()
            .reproductive_systems
            .genetics_system
            .birth_records
            .push(mk_core::human::schema::BirthRecordSchema {
                birth_id: "birth_far-child".to_string(),
                genotype_id: "genotype_far-child".to_string(),
                father_id: "unknown-father".to_string(),
                mother_id: "far-parent".to_string(),
                birth_timestamp: "tick-0".to_string(),
                agent_id: "far-child".to_string(),
                mutations: vec![],
            });

        parent.set_runtime_position(GridPosition::new(0, 0));
        child.set_runtime_position(GridPosition::new(0, 400));
        system.registry.add_human_no_storage(parent);
        system.registry.add_human_no_storage(child);

        system.step_dialogue(&mk_core::rng::RngRegistry::new([11u8; 32]), 2);

        let events: Vec<_> = system.conversation_log().collect();
        assert!(
            events.is_empty(),
            "a parent and child 800 km apart held a conversation: {:?}",
            events
                .iter()
                .map(|event| event.relationship)
                .collect::<Vec<_>>()
        );
    }

    /// The same pair, standing next to each other, must still talk --
    /// otherwise the change above would read as a fix while having
    /// silenced families altogether.
    #[test]
    fn a_parent_and_child_standing_together_still_converse() {
        let mut system = HumanSystem::new();
        let mut parent = HumanBeing::new("near-parent".to_string(), BiologicalSex::Female);
        let mut child = HumanBeing::new("near-child".to_string(), BiologicalSex::Male);

        for h in [&mut parent, &mut child] {
            if h.profile.canonical_schema.is_none() {
                h.profile.canonical_schema = Some(HumanSchema::canonical_minimal(h.agent_id()));
            }
        }
        child
            .profile
            .canonical_schema
            .as_mut()
            .unwrap()
            .reproductive_systems
            .genetics_system
            .birth_records
            .push(mk_core::human::schema::BirthRecordSchema {
                birth_id: "birth_near-child".to_string(),
                genotype_id: "genotype_near-child".to_string(),
                father_id: "unknown-father".to_string(),
                mother_id: "near-parent".to_string(),
                birth_timestamp: "tick-0".to_string(),
                agent_id: "near-child".to_string(),
                mutations: vec![],
            });

        parent.set_runtime_position(GridPosition::new(10, 10));
        child.set_runtime_position(GridPosition::new(10, 11));
        system.registry.add_human_no_storage(parent);
        system.registry.add_human_no_storage(child);

        system.step_dialogue(&mk_core::rng::RngRegistry::new([11u8; 32]), 2);

        let events: Vec<_> = system.conversation_log().collect();
        assert_eq!(events.len(), 1, "a parent and child in the next cell talk");
        assert_eq!(
            events[0].relationship,
            dialogue::ConversationRelationship::ParentChild
        );
    }

    #[test]
    fn siblings_that_far_apart_do_not_converse() {
        // This was the other half of D34, and the reason it read as an
        // inconsistency rather than a blanket choice: the same distance
        // that silenced two siblings left a parent and child talking.
        // Both are silent now, which is what made the inconsistency go
        // away -- by levelling up to the stricter rule, not down.
        let mut system = HumanSystem::new();
        for (name, col) in [("far-sib-a", 0), ("far-sib-b", 400)] {
            let mut human = HumanBeing::new(name.to_string(), BiologicalSex::Female);
            human
                .profile
                .canonical_schema
                .get_or_insert_with(|| HumanSchema::canonical_minimal(name))
                .reproductive_systems
                .genetics_system
                .birth_records
                .push(mk_core::human::schema::BirthRecordSchema {
                    birth_id: format!("birth_{name}"),
                    genotype_id: format!("genotype_{name}"),
                    father_id: "shared-father".to_string(),
                    mother_id: "shared-mother".to_string(),
                    birth_timestamp: "tick-0".to_string(),
                    agent_id: name.to_string(),
                    mutations: vec![],
                });
            human.set_runtime_position(GridPosition::new(0, col));
            system.registry.add_human_no_storage(human);
        }

        system.step_dialogue(&mk_core::rng::RngRegistry::new([11u8; 32]), 2);

        assert_eq!(system.conversation_log().count(), 0);
    }

    #[test]
    fn two_sleeping_people_hold_no_conversation() {
        // Half of D35, fixed. Family pairs used to be matched on `alive`
        // alone, so on a running island both founders, asleep from tick
        // 1799, generated forty exchanges across ticks 2001-2040. This
        // asserted that behaviour before the gate went in; it now asserts
        // its absence.
        let mut system = HumanSystem::new();
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_d_founder());
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_k_founder());
        for id in ["Gem-D", "Gem-K"] {
            system
                .registry
                .get_human_mut(id)
                .expect("a founder")
                .circadian
                .asleep = true;
        }

        let rng = mk_core::rng::RngRegistry::new([21u8; 32]);
        for tick in 0..3 {
            system.step_dialogue(&rng, tick);
        }
        assert_eq!(
            system.conversation_log().count(),
            0,
            "nobody talks in their sleep"
        );

        // And they talk again on waking, so this is a gate and not a ban:
        // the pair is otherwise exactly the one that conversed before.
        for id in ["Gem-D", "Gem-K"] {
            system
                .registry
                .get_human_mut(id)
                .expect("a founder")
                .circadian
                .asleep = false;
        }
        system.step_dialogue(&rng, 3);
        assert_eq!(system.conversation_log().count(), 1);
    }

    #[test]
    fn one_sleeper_is_enough_to_stop_a_conversation() {
        // It takes two people awake. A conversation with a sleeping partner
        // would be no more real for the speaker being awake, and it would
        // still write into the sleeper's relationship memory.
        let mut system = HumanSystem::new();
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_d_founder());
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_k_founder());
        system
            .registry
            .get_human_mut("Gem-K")
            .expect("a founder")
            .circadian
            .asleep = true;

        system.step_dialogue(&mk_core::rng::RngRegistry::new([22u8; 32]), 0);

        assert_eq!(system.conversation_log().count(), 0);
        assert!(
            system
                .registry
                .get_human("Gem-D")
                .expect("a founder")
                .conversation_history
                .is_empty(),
            "the waking founder remembers no conversation either"
        );
    }

    fn crowd(size: usize, cells: usize) -> HumanSystem {
        let mut system = HumanSystem::new();
        for n in 0..size {
            let mut human = HumanBeing::new(format!("npc-{n:04}"), BiologicalSex::Female);
            human.set_runtime_position(GridPosition::new(0, (n % cells) as i32 * 3));
            human.economy_action.kind = ActionKind::SocialApproach;
            system.registry.add_human_no_storage(human);
        }
        system
    }

    #[test]
    fn a_crowd_pairs_each_human_into_at_most_one_conversation() {
        let mut system = crowd(51, 1);
        system.step_dialogue(&mk_core::rng::RngRegistry::new([8u8; 32]), 4);

        let mut seen = std::collections::HashSet::new();
        for event in system.conversation_log() {
            assert!(seen.insert(event.participant_a_id), "{event:?}");
            assert!(seen.insert(event.participant_b_id), "{event:?}");
        }
        // 51 approaching neighbours. The matching is maximal over a ring of
        // 8 successors each, so at most one person in nine is left over.
        let pairs = system.conversation_log().count();
        assert!((23..=25).contains(&pairs), "{pairs} pairs");
        assert_eq!(seen.len(), pairs * 2);
    }

    /// `parent`/`child` are made parent and child by a birth record, the way
    /// the lifecycle does it.
    fn kin(name: &str, parents: (&str, &str), at: GridPosition) -> HumanBeing {
        let mut human = HumanBeing::new(name.to_string(), BiologicalSex::Female);
        human
            .profile
            .canonical_schema
            .get_or_insert_with(|| HumanSchema::canonical_minimal(name))
            .reproductive_systems
            .genetics_system
            .birth_records
            .push(mk_core::human::schema::BirthRecordSchema {
                birth_id: format!("birth_{name}"),
                genotype_id: format!("genotype_{name}"),
                father_id: parents.0.to_string(),
                mother_id: parents.1.to_string(),
                birth_timestamp: "tick-0".to_string(),
                agent_id: name.to_string(),
                mutations: vec![],
            });
        human.set_runtime_position(at);
        human
    }

    fn talked_with(system: &HumanSystem, id: &str) -> usize {
        system
            .registry
            .get_human(id)
            .unwrap()
            .conversation_history
            .len()
    }

    #[test]
    fn founders_and_their_child_all_get_to_talk() {
        let mut system = HumanSystem::new();
        // All three in one cell. They used to be forty cells apart, which
        // worked only because a parent and child were matched at any
        // distance (D34). The property under test is the rotation -- that
        // no family pair monopolises a person -- and that has nothing to
        // do with distance, so the household is now a household.
        //
        // One cell rather than three touching ones, because under
        // Manhattan distance 1 no three cells are all adjacent to each
        // other: putting the child a step from each parent leaves the
        // parents two steps apart. A 2 km cell is a house and then some,
        // so a family sharing one is the ordinary case, not a contrivance.
        let home = GridPosition::new(20, 20);
        let mut gem_d = HumanBeing::gem_d_founder();
        let mut gem_k = HumanBeing::gem_k_founder();
        gem_d.set_runtime_position(home);
        gem_k.set_runtime_position(home);
        system.registry.add_human_no_storage(gem_d);
        system.registry.add_human_no_storage(gem_k);
        system
            .registry
            .add_human_no_storage(kin("first-child", ("Gem-D", "Gem-K"), home));
        let rng = mk_core::rng::RngRegistry::new([3u8; 32]);

        for tick in 0..60 {
            system.step_dialogue(&rng, tick);
        }

        let founders_together = system
            .conversation_log()
            .filter(|e| e.relationship == dialogue::ConversationRelationship::Founders)
            .count();
        assert!(
            founders_together > 0,
            "the founders still talk to each other"
        );
        // The child talked, and to each parent: both parents have more
        // conversations than the founders' own pairings account for.
        assert!(talked_with(&system, "first-child") > 0);
        let parent_chats = |id: &str| talked_with(&system, id) - founders_together;
        assert!(parent_chats("Gem-D") > 0, "the child talks to Gem-D");
        assert!(parent_chats("Gem-K") > 0, "the child talks to Gem-K");
    }

    #[test]
    fn a_child_talks_to_its_parent_rather_than_a_stranger_beside_them() {
        // Family outranks a neighbour reaching out, which is the property
        // this has always been about. It used to prove it with a parent
        // thirty cells away, because distance did not count against a
        // parent then (D34). Now both are in earshot and the ranking is
        // what decides it, which is what the name claims.
        let mut system = HumanSystem::new();
        system.registry.add_human_no_storage({
            let mut parent = HumanBeing::new("near-parent".to_string(), BiologicalSex::Male);
            parent.set_runtime_position(GridPosition::new(2, 1));
            parent
        });
        system.registry.add_human_no_storage(kin(
            "kid",
            ("near-parent", "unknown-mother"),
            GridPosition::new(2, 2),
        ));
        let mut stranger = HumanBeing::new("stranger".to_string(), BiologicalSex::Male);
        stranger.set_runtime_position(GridPosition::new(2, 3));
        stranger.economy_action.kind = ActionKind::SocialApproach;
        system.registry.add_human_no_storage(stranger);

        system.step_dialogue(&mk_core::rng::RngRegistry::new([2u8; 32]), 5);

        assert_eq!(talked_with(&system, "near-parent"), 1);
        assert_eq!(talked_with(&system, "kid"), 1);
        assert_eq!(talked_with(&system, "stranger"), 0);
    }

    /// And the consequence of D34's fix that is worth stating out loud: a
    /// child whose parent is far away is not left silent in a crowd. They
    /// talk to whoever is actually there, which is both what the matcher
    /// does and what a person does.
    #[test]
    fn a_child_whose_parent_is_far_away_talks_to_the_neighbour_who_is_there() {
        let mut system = HumanSystem::new();
        system.registry.add_human_no_storage({
            let mut parent = HumanBeing::new("far-parent".to_string(), BiologicalSex::Male);
            parent.set_runtime_position(GridPosition::new(30, 30));
            parent
        });
        system.registry.add_human_no_storage(kin(
            "kid",
            ("far-parent", "unknown-mother"),
            GridPosition::new(2, 2),
        ));
        let mut stranger = HumanBeing::new("stranger".to_string(), BiologicalSex::Male);
        stranger.set_runtime_position(GridPosition::new(2, 3));
        stranger.economy_action.kind = ActionKind::SocialApproach;
        system.registry.add_human_no_storage(stranger);

        system.step_dialogue(&mk_core::rng::RngRegistry::new([2u8; 32]), 5);

        assert_eq!(
            talked_with(&system, "far-parent"),
            0,
            "a parent 56 km away is not in the conversation"
        );
        assert_eq!(talked_with(&system, "kid"), 1);
        assert_eq!(
            talked_with(&system, "stranger"),
            1,
            "the child talks to the person standing next to them"
        );
    }

    #[test]
    fn siblings_behind_many_idle_strangers_still_converse() {
        let mut system = HumanSystem::new();
        let here = GridPosition::new(4, 4);
        for n in 0..30 {
            let mut idle = HumanBeing::new(format!("idle-{n:02}"), BiologicalSex::Male);
            idle.set_runtime_position(here);
            system.registry.add_human_no_storage(idle);
        }
        system
            .registry
            .add_human_no_storage(kin("sib-a", ("dad", "mum"), here));
        system
            .registry
            .add_human_no_storage(kin("sib-b", ("dad", "mum"), here));

        system.step_dialogue(&mk_core::rng::RngRegistry::new([2u8; 32]), 1);

        assert_eq!(talked_with(&system, "sib-a"), 1);
        assert_eq!(talked_with(&system, "sib-b"), 1);
        assert_eq!(
            system.conversation_log().count(),
            1,
            "idle strangers stay quiet"
        );
    }

    #[test]
    fn dialogue_pairing_is_deterministic() {
        let run = || {
            let mut system = crowd(40, 4);
            system.step_dialogue(&mk_core::rng::RngRegistry::new([8u8; 32]), 4);
            system
                .conversation_log()
                .map(|e| (e.participant_a_id, e.participant_b_id))
                .collect::<Vec<_>>()
        };
        let first = run();
        assert!(!first.is_empty());
        assert_eq!(first, run());
    }

    #[test]
    fn humans_who_are_apart_and_not_kin_do_not_converse() {
        let mut system = crowd(2, 2); // cells 0 and 3 columns apart
        system.step_dialogue(&mk_core::rng::RngRegistry::new([8u8; 32]), 4);
        assert_eq!(system.conversation_log().count(), 0);
    }

    #[test]
    fn adjacent_siblings_converse_as_siblings() {
        let mut system = HumanSystem::new();
        for name in ["sib-a", "sib-b"] {
            let mut human = HumanBeing::new(name.to_string(), BiologicalSex::Female);
            human
                .profile
                .canonical_schema
                .get_or_insert_with(|| HumanSchema::canonical_minimal(name))
                .reproductive_systems
                .genetics_system
                .birth_records
                .push(mk_core::human::schema::BirthRecordSchema {
                    birth_id: format!("birth_{name}"),
                    genotype_id: format!("genotype_{name}"),
                    father_id: "far-father".to_string(),
                    mother_id: "far-mother".to_string(),
                    birth_timestamp: "tick-0".to_string(),
                    agent_id: name.to_string(),
                    mutations: vec![],
                });
            human.set_runtime_position(GridPosition::new(4, if name == "sib-a" { 7 } else { 8 }));
            system.registry.add_human_no_storage(human);
        }

        system.step_dialogue(&mk_core::rng::RngRegistry::new([3u8; 32]), 9);

        let events: Vec<_> = system.conversation_log().collect();
        assert_eq!(events.len(), 1, "one conversation between the pair");
        assert_eq!(
            events[0].relationship,
            dialogue::ConversationRelationship::Siblings
        );
        let sib_a = system.registry.get_human("sib-a").unwrap();
        assert_eq!(sib_a.conversation_history.len(), 1, "each remembers it");
    }

    #[test]
    fn propagate_grief_raises_sadness_for_parent_and_sibling_of_the_deceased() {
        let mut system = HumanSystem::new();
        let mut parent_a = HumanBeing::new("parent-a".to_string(), BiologicalSex::Male);
        let mut parent_b = HumanBeing::new("parent-b".to_string(), BiologicalSex::Female);
        let mut deceased = HumanBeing::new("child-c".to_string(), BiologicalSex::Female);
        let mut sibling = HumanBeing::new("child-d".to_string(), BiologicalSex::Male);
        let mut stranger = HumanBeing::new("stranger-e".to_string(), BiologicalSex::Male);

        for h in [
            &mut parent_a,
            &mut parent_b,
            &mut deceased,
            &mut sibling,
            &mut stranger,
        ] {
            if h.profile.canonical_schema.is_none() {
                h.profile.canonical_schema = Some(HumanSchema::canonical_minimal(h.agent_id()));
            }
        }
        for (agent_id, birth_id) in [("child-c", "birth_child-c"), ("child-d", "birth_child-d")] {
            let target = if agent_id == "child-c" {
                &mut deceased
            } else {
                &mut sibling
            };
            target
                .profile
                .canonical_schema
                .as_mut()
                .unwrap()
                .reproductive_systems
                .genetics_system
                .birth_records
                .push(mk_core::human::schema::BirthRecordSchema {
                    birth_id: birth_id.to_string(),
                    genotype_id: format!("genotype_{agent_id}"),
                    father_id: "parent-a".to_string(),
                    mother_id: "parent-b".to_string(),
                    birth_timestamp: "tick-0".to_string(),
                    agent_id: agent_id.to_string(),
                    mutations: vec![],
                });
        }

        let parent_sadness_before = parent_a.emotion.current.sadness;
        let sibling_sadness_before = sibling.emotion.current.sadness;
        let stranger_sadness_before = stranger.emotion.current.sadness;
        let parent_trust_before = parent_a.social_systems.social_trust;
        let sibling_trust_before = sibling.social_systems.social_trust;

        system.registry.add_human_no_storage(parent_a);
        system.registry.add_human_no_storage(parent_b);
        system.registry.add_human_no_storage(deceased);
        system.registry.add_human_no_storage(sibling);
        system.registry.add_human_no_storage(stranger);

        system.propagate_grief("child-c");

        let parent_a = system.registry.get_human("parent-a").unwrap();
        let sibling = system.registry.get_human("child-d").unwrap();
        let stranger = system.registry.get_human("stranger-e").unwrap();

        assert!(parent_a.emotion.current.sadness > parent_sadness_before);
        assert!(sibling.emotion.current.sadness > sibling_sadness_before);
        assert_eq!(stranger.emotion.current.sadness, stranger_sadness_before);
        assert!(parent_a.social_systems.social_trust < parent_trust_before);
        assert!(sibling.social_systems.social_trust < sibling_trust_before);
    }

    #[test]
    fn refresh_phase11_layers_preserves_experiential_trust_across_reset() {
        let mut human = HumanBeing::new("test-human".to_string(), BiologicalSex::Male);
        human.profile.canonical_schema = Some(HumanSchema::canonical_minimal(human.agent_id()));
        human.refresh_phase11_layers();

        human.social_systems.apply_social_shock(-0.2);
        let trust_after_shock = human.social_systems.social_trust;
        assert_eq!(human.social_systems.experiential_trust, -0.2);

        // Re-deriving every other from_profile()-based layer must not wipe
        // the lived-experience accumulator or the trust value it produced.
        human.refresh_phase11_layers();
        assert_eq!(human.social_systems.experiential_trust, -0.2);
        assert_eq!(human.social_systems.social_trust, trust_after_shock);
    }

    #[test]
    fn refresh_phase11_layers_preserves_lived_reproductive_state() {
        let mut human = HumanBeing::new("repro-refresh".to_string(), BiologicalSex::Female);
        human.profile.canonical_schema = Some(HumanSchema::canonical_minimal(human.agent_id()));
        human.refresh_phase11_layers();

        human.reproduction.sexual_activity_count = 3;
        human.reproduction.satisfaction = 0.9;
        human.refresh_phase11_layers();

        assert_eq!(human.reproduction.sexual_activity_count, 3);
        assert_eq!(human.reproduction.satisfaction, 0.9);
    }

    /// Bounds wide enough that nothing is evicted, for the tests that
    /// are about merging rather than about the budget.
    fn roomy() -> ConversationBounds {
        ConversationBounds {
            max_entries: 500,
            max_lines: usize::MAX,
        }
    }

    /// One exchange between a pair, said on `tick`, carrying `lines`
    /// lines so a merge can be counted rather than guessed at.
    fn exchange(a: u64, b: u64, tick: u64, lines: usize) -> dialogue::ConversationEvent {
        dialogue::ConversationEvent {
            tick,
            last_tick: tick,
            participant_a_id: mk_core::human::HumanId(a),
            participant_b_id: mk_core::human::HumanId(b),
            relationship: dialogue::ConversationRelationship::Other,
            lines: (0..lines)
                .map(|n| dialogue::DialogueLine {
                    speaker_id: mk_core::human::HumanId(a),
                    speaker_name: format!("human {a}"),
                    text: format!("line {n} on tick {tick}"),
                    gist: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn talking_on_the_next_tick_carries_the_same_conversation_on() {
        let mut log = VecDeque::new();
        for tick in 1..=5 {
            merge_or_push(&mut log, roomy(), &exchange(1, 2, tick, 2), tick);
        }

        assert_eq!(log.len(), 1, "five minutes of talking is one conversation");
        let held = &log[0];
        assert_eq!(held.tick, 1, "it is remembered from when they started");
        assert_eq!(held.last_tick, 5, "and runs to the last thing said");
        assert_eq!(held.lines.len(), 10, "every line said is kept");
    }

    #[test]
    fn a_gap_of_even_one_tick_starts_a_new_conversation() {
        let mut log = VecDeque::new();
        merge_or_push(&mut log, roomy(), &exchange(1, 2, 1, 2), 1);
        merge_or_push(&mut log, roomy(), &exchange(1, 2, 3, 2), 3);

        assert_eq!(log.len(), 2, "they stopped talking and started again");
        assert_eq!(log[0].last_tick, 1);
        assert_eq!(log[1].tick, 3);
    }

    #[test]
    fn who_spoke_first_does_not_make_it_a_different_conversation() {
        let mut log = VecDeque::new();
        merge_or_push(&mut log, roomy(), &exchange(1, 2, 1, 2), 1);
        // The pairing handed them over the other way round this tick.
        merge_or_push(&mut log, roomy(), &exchange(2, 1, 2, 2), 2);

        assert_eq!(log.len(), 1, "it is the same two people talking");
        assert_eq!(log[0].lines.len(), 4);
    }

    /// The island-wide log holds every pair, so a pair's own last entry
    /// sits behind whatever other pairs said after it. Reading only the
    /// back of the log would see someone else's conversation there and
    /// file a new one every tick -- which is the fragmentation this is
    /// meant to end, so it is worth a test of its own.
    #[test]
    fn another_pair_talking_in_between_does_not_split_a_conversation() {
        let mut log = VecDeque::new();
        for tick in 1..=4 {
            merge_or_push(&mut log, roomy(), &exchange(1, 2, tick, 2), tick);
            merge_or_push(&mut log, roomy(), &exchange(3, 4, tick, 2), tick);
        }

        assert_eq!(log.len(), 2, "two pairs talked, so two conversations");
        assert_eq!(log[0].lines.len(), 8, "the first pair's lines are all here");
        assert_eq!(log[1].lines.len(), 8, "and so are the second pair's");
        assert!(log.iter().all(|held| held.tick == 1 && held.last_tick == 4));
    }

    #[test]
    fn a_conversation_long_enough_to_fall_out_of_the_log_still_does() {
        let mut log = VecDeque::new();
        // Each pair speaks once, on its own tick, so nothing merges and
        // the cap is the only thing deciding what is kept.
        for tick in 1..=12 {
            merge_or_push(
                &mut log,
                ConversationBounds {
                    max_entries: 10,
                    max_lines: usize::MAX,
                },
                &exchange(1, 100 + tick, tick, 2),
                tick,
            );
        }

        assert_eq!(log.len(), 10, "the cap still holds");
        assert_eq!(log[0].tick, 3, "the oldest two were evicted, oldest first");
        assert_eq!(log[9].tick, 12);
    }

    /// Merging makes an entry a whole exchange, so an entry count stopped
    /// bounding anything: 500 entries of a founder's waking stretch is
    /// half a million lines. The line budget is the real bound, and
    /// whole conversations go rather than conversations losing their
    /// beginnings.
    #[test]
    fn the_line_budget_drops_whole_conversations_oldest_first() {
        let bounds = ConversationBounds {
            max_entries: 500,
            max_lines: 10,
        };
        let mut log = VecDeque::new();
        // Four separate conversations of four lines each: 16 lines
        // offered into a budget of 10.
        for (n, tick) in (1..=4).map(|n| (n, n * 10)) {
            merge_or_push(&mut log, bounds, &exchange(1, 100 + n, tick, 4), tick);
        }

        let held: usize = log.iter().map(|event| event.lines.len()).sum();
        assert!(held <= 10, "the budget holds: {held} lines");
        assert_eq!(log.len(), 2, "two whole conversations fit in ten lines");
        assert_eq!(log[0].tick, 30, "the stalest two went first");
        assert!(
            log.iter().all(|event| event.lines.len() == 4),
            "a kept conversation keeps all of its lines"
        );
    }

    #[test]
    fn one_conversation_longer_than_the_whole_budget_is_still_kept() {
        let bounds = ConversationBounds {
            max_entries: 500,
            max_lines: 4,
        };
        let mut log = VecDeque::new();
        for tick in 1..=6 {
            merge_or_push(&mut log, bounds, &exchange(1, 2, tick, 2), tick);
        }

        assert_eq!(log.len(), 1, "it is all there is to remember");
        assert_eq!(log[0].lines.len(), 12, "and it is not cut in half");
    }

    /// Position is the order conversations started in, and once an
    /// exchange can run for hours that is not the order they went quiet
    /// in. An exchange that began this morning and is still going sits
    /// near the front of the log; evicting from the front would drop the
    /// conversation happening right now and keep one that ended hours
    /// ago.
    #[test]
    fn the_conversation_still_going_outlives_a_newer_one_that_ended() {
        let bounds = ConversationBounds {
            max_entries: 500,
            max_lines: 20,
        };
        let mut log = VecDeque::new();

        // One pair talk without a break from the first tick to the
        // twentieth, so their exchange is a single entry at the front of
        // the log and is the one still going at the end.
        for tick in 1..=20 {
            merge_or_push(&mut log, bounds, &exchange(1, 2, tick, 1), tick);
            // Another pair say two words early on and stop. Their entry
            // is newer, and goes quiet long before the first pair do.
            if (5..=6).contains(&tick) {
                merge_or_push(&mut log, bounds, &exchange(3, 4, tick, 1), tick);
            }
        }

        assert_eq!(
            log.len(),
            1,
            "twenty-two lines went into a twenty-line budget, so one had to go"
        );
        let kept = &log[0];
        assert_eq!(
            kept.participant_a_id,
            mk_core::human::HumanId(1),
            "the conversation that was still going is the one that was kept"
        );
        assert_eq!(kept.last_tick, 20);
        assert_eq!(kept.lines.len(), 20, "and it kept every line of itself");
    }
}
