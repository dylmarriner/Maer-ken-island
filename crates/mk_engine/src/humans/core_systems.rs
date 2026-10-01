//! Core Systems Snapshot - the four foundational config layers
//! (`BioSysConfigSchema`/`PsycheSysConfigSchema`/`ChaosSysConfigSchema`/
//! `WillSysConfigSchema`) that canon calls "the four fundamental systems
//! that CREATE human consciousness": biology → psychology → chaos → will →
//! cognition.
//!
//! Prior state: `biosys.drive_sensitivities`/`metabolic_baselines`/
//! `processing_rates` were already read by [`super::needs`].
//! `biosys.endocrine_baselines` was defined in `mk_core::human::schema` but
//! never read anywhere in the engine — [`super::neurochemistry`] instead
//! invented its own independent hormone starting values. `psychesys`,
//! `chaossys`, and `willsys` were entirely unread. This module closes all
//! four gaps with genuine behavioral wiring, not a passive read-only echo:
//!
//! - **`biosys.endocrine_baselines`**: now the actual hormone *baseline*
//!   [`super::neurochemistry::NeurochemistrySnapshot`] levels drift toward
//!   at rest — see the fix in `neurochemistry.rs::from_profile`.
//! - **`psychesys`** ("converts biological state to urges"): this module's
//!   [`UrgeSnapshot`] genuinely performs that conversion — the real
//!   biological drives already tracked in [`super::needs::NeedsSnapshot`]
//!   (hunger/thirst/fatigue) plus social/emotional signals are weighted by
//!   `urge_sensitivities` and passed through `urge_processing`'s
//!   biological-to-psychological conversion weight and emotional
//!   amplification to produce 5 real psychological urgency scalars
//!   (survival/social/achievement/exploration/reproduction), matching
//!   canon's `urge_sensitivities` field names exactly.
//! - **`chaossys`** ("ONLY source of randomness"): urges are perturbed by
//!   genuine seeded chaos via `mk_core::rng::RngRegistry` (this project's
//!   existing deterministic, tick/human-keyed BLAKE3-derived RNG — the same
//!   mechanism [`mk_core::human::genetics`] already uses for gamete
//!   recombination). Perturbation magnitude is bounded per-urge by
//!   `chaos_bounds`, scaled by `randomness_profile.entropy_level`, and
//!   `chaos_responses.stress_amplification` genuinely widens the bound
//!   under the same physiological stress `neurochemistry`/`immune` already
//!   track. Same seed + same tick + same human ID always produces the same
//!   perturbation — this is designed variability, not incidental
//!   non-determinism, and satisfies the project's rule against calling the
//!   global (non-seeded) RNG directly.
//! - **`willsys`** ("ONLY gateway to cognition"): [`UrgeSnapshot::gated`]
//!   genuinely gates which (if any) urge reaches
//!   `willpower_available: bool` / `dominant_urge: Option<UrgeKind>` —
//!   canon's "gateway" concept — by comparing the top post-chaos urge
//!   against `willpower_profile.baseline_threshold`, itself depleted by
//!   `willpower_profile.decay_rate` under sustained high urgency and
//!   restored by `recovery_rate`. `agency_patterns.autonomy_drive` lowers
//!   the effective threshold (more autonomous humans act on weaker urges);
//!   `compliance_tendency` raises it (more compliant humans wait to be
//!   compelled). `cognitive_access.attention_threshold` additionally
//!   requires the gated urge to have real attentional resources available
//!   (coupled to [`super::attention::AttentionSnapshot`]) before
//!   "gatewaying to cognition" — a request for conscious deliberation with
//!   no spare attention to spend on it does not get through.
//!
//! - **`interrupt_processing`**: once an urge holds cognition, a different
//!   urge must genuinely *interrupt* it to take over. Interrupt pressure is
//!   `urgency_weight × challenger strength + chaos_weight × |challenger's
//!   chaos perturbation|`; it must exceed `context_modulation × held urge
//!   strength`, and `interrupt_cooldown` (milliseconds of simulated time
//!   since the last interrupt, accumulated from `dt_years`) must have
//!   elapsed. A blocked challenger leaves the held urge in focus, so a
//!   human mid-task is not yanked to every momentarily larger urge. At
//!   coarse ticks longer than the cooldown the refractory period elapses
//!   within one tick, as it would in reality.
//!
//! Not modeled: `cognitive_access.working_memory_capacity`/`processing_speed`/
//! `cognitive_flexibility` (already independently modeled with their own
//! tested state in [`super::memory`]/[`super::cognition`] — wiring this
//! duplicate copy in would mean two competing sources of truth for the same
//! concept; `willsys`'s copy is read once here for `attention_threshold`
//! only, the one field with no existing engine equivalent).

use crate::humans::attention::AttentionSnapshot;
use crate::humans::emotion::EmotionSnapshot;
use crate::humans::needs::NeedsSnapshot;
use crate::humans::social_cognition::SocialCognitionSnapshot;
use mk_core::human::{HumanId, HumanProfile};
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrgeKind {
    Survival,
    Social,
    Achievement,
    Exploration,
    Reproduction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Urges {
    pub survival: f64,
    pub social: f64,
    pub achievement: f64,
    pub exploration: f64,
    pub reproduction: f64,
}

impl Urges {
    fn get(&self, kind: UrgeKind) -> f64 {
        match kind {
            UrgeKind::Survival => self.survival,
            UrgeKind::Social => self.social,
            UrgeKind::Achievement => self.achievement,
            UrgeKind::Exploration => self.exploration,
            UrgeKind::Reproduction => self.reproduction,
        }
    }

    fn dominant(&self) -> (UrgeKind, f64) {
        [
            (UrgeKind::Survival, self.survival),
            (UrgeKind::Social, self.social),
            (UrgeKind::Achievement, self.achievement),
            (UrgeKind::Exploration, self.exploration),
            (UrgeKind::Reproduction, self.reproduction),
        ]
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((UrgeKind::Survival, 0.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreSystemsSnapshot {
    // --- psychesys traits ---
    biological_to_psychological_weight: f64,
    emotional_amplification: f64,
    urge_sensitivities: Urges,

    // --- chaossys traits ---
    chaos_bounds: Urges,
    entropy_level: f64,
    stress_amplification: f64,

    // --- willsys traits ---
    baseline_threshold: f64,
    /// Willpower depletion under sustained urgency, per hour.
    threshold_decay_rate: f64,
    /// Willpower recovery when no urge presses, per hour.
    threshold_recovery_rate: f64,
    autonomy_drive: f64,
    compliance_tendency: f64,
    attention_threshold: f64,

    // --- willsys.interrupt_processing traits ---
    #[serde(default = "default_interrupt_urgency_weight")]
    interrupt_urgency_weight: f64,
    #[serde(default = "default_interrupt_chaos_weight")]
    interrupt_chaos_weight: f64,
    #[serde(default = "default_interrupt_context_modulation")]
    interrupt_context_modulation: f64,
    /// Refractory period between interrupts, in milliseconds of simulated
    /// time. Zero is a legitimate "no refractory period" setting.
    #[serde(default = "default_interrupt_cooldown_ms")]
    interrupt_cooldown_ms: u64,

    // --- state (stepped) ---
    /// Raw biological-to-psychological urge conversion, before chaos.
    pub urges: Urges,
    /// Urges after seeded chaos perturbation (canon's `chaossys` output).
    pub chaotic_urges: Urges,
    /// Willpower's current available capacity — depletes under sustained
    /// high urgency, recovers otherwise (canon: derived from
    /// `willpower_profile.decay_rate`/`recovery_rate`).
    pub willpower_capacity: f64,
    /// Whether any urge cleared the willpower gate *and* had attentional
    /// resources available this tick — canon's willsys "ONLY gateway to
    /// cognition" concept, genuinely gating rather than always-true.
    pub willpower_available: bool,
    /// Which urge reached cognition this tick, if any.
    pub dominant_urge: Option<UrgeKind>,
    /// Whether a challenger urge interrupted the previously held one this
    /// tick.
    #[serde(default)]
    pub interrupted: bool,
    /// Simulated milliseconds since the last interrupt.
    #[serde(default = "cooldown_fully_elapsed")]
    pub ms_since_last_interrupt: f64,
}

/// Milliseconds in one Julian year, for converting `dt_years` into canon's
/// millisecond `interrupt_cooldown`.
const MS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0 * 1000.0;

// Engine fallbacks mirror the canon `WillsysSchema` defaults
// (`mk_core::human::schema`) so an unfilled template behaves like canon.
fn default_interrupt_urgency_weight() -> f64 {
    0.82
}

fn default_interrupt_chaos_weight() -> f64 {
    0.35
}

fn default_interrupt_context_modulation() -> f64 {
    0.55
}

const DEFAULT_INTERRUPT_COOLDOWN_MS: u64 = 450;

fn default_interrupt_cooldown_ms() -> u64 {
    DEFAULT_INTERRUPT_COOLDOWN_MS
}

fn cooldown_fully_elapsed() -> f64 {
    f64::MAX
}

impl CoreSystemsSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let core = &schema.core_systems;

        let urge_sensitivities = Urges {
            survival: nz(core.psychesys.urge_sensitivities.survival_urgency, 0.6),
            social: nz(core.psychesys.urge_sensitivities.social_urgency, 0.4),
            achievement: nz(core.psychesys.urge_sensitivities.achievement_urgency, 0.4),
            exploration: nz(core.psychesys.urge_sensitivities.exploration_urgency, 0.3),
            reproduction: nz(core.psychesys.urge_sensitivities.reproduction_urgency, 0.3),
        };

        let chaos_bounds = Urges {
            survival: nz(core.chaossys.chaos_bounds.survival_chaos.max as f32, 0.1),
            social: nz(core.chaossys.chaos_bounds.social_chaos.max as f32, 0.1),
            achievement: nz(core.chaossys.chaos_bounds.achievement_chaos.max as f32, 0.1),
            exploration: nz(core.chaossys.chaos_bounds.exploration_chaos.max as f32, 0.1),
            reproduction: nz(
                core.chaossys.chaos_bounds.reproduction_chaos.max as f32,
                0.1,
            ),
        };

        Self {
            biological_to_psychological_weight: nz(
                core.psychesys
                    .urge_processing
                    .biological_to_psychological_weight,
                0.7,
            ),
            emotional_amplification: nz(
                core.psychesys.urge_processing.emotional_amplification,
                1.0,
            ),
            urge_sensitivities,
            chaos_bounds,
            entropy_level: nz(core.chaossys.randomness_profile.entropy_level, 0.3),
            stress_amplification: nz(core.chaossys.chaos_responses.stress_amplification, 1.0),
            baseline_threshold: nz(core.willsys.willpower_profile.baseline_threshold, 0.4),
            threshold_decay_rate: nz(core.willsys.willpower_profile.decay_rate, 0.1),
            threshold_recovery_rate: nz(core.willsys.willpower_profile.recovery_rate, 0.15),
            autonomy_drive: nz(core.willsys.agency_patterns.autonomy_drive, 0.5),
            compliance_tendency: nz(core.willsys.agency_patterns.compliance_tendency, 0.5),
            attention_threshold: nz(core.willsys.cognitive_access.attention_threshold, 0.2),
            interrupt_urgency_weight: nz(
                core.willsys.interrupt_processing.urgency_weight,
                default_interrupt_urgency_weight(),
            ),
            interrupt_chaos_weight: nz(
                core.willsys.interrupt_processing.chaos_weight,
                default_interrupt_chaos_weight(),
            ),
            interrupt_context_modulation: nz(
                core.willsys.interrupt_processing.context_modulation,
                default_interrupt_context_modulation(),
            ),
            interrupt_cooldown_ms: core.willsys.interrupt_processing.interrupt_cooldown,
            urges: Urges {
                survival: 0.3,
                social: 0.2,
                achievement: 0.2,
                exploration: 0.2,
                reproduction: 0.1,
            },
            chaotic_urges: Urges {
                survival: 0.3,
                social: 0.2,
                achievement: 0.2,
                exploration: 0.2,
                reproduction: 0.1,
            },
            willpower_capacity: nz(core.willsys.willpower_profile.baseline_threshold, 0.4).max(0.5),
            willpower_available: false,
            dominant_urge: None,
            interrupted: false,
            ms_since_last_interrupt: cooldown_fully_elapsed(),
        }
    }

    /// Full pipeline: biology → psychology (urges) → chaos (perturbation) →
    /// will (gating) → cognition (dominant_urge output).
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        human_id: HumanId,
        tick: u64,
        needs: &NeedsSnapshot,
        emotion: &EmotionSnapshot,
        social_cognition: &SocialCognitionSnapshot,
        attention: &AttentionSnapshot,
        rng: &RngRegistry,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);

        // --- psychesys: biology -> psychological urges ---
        let emotional_intensity = emotion.overall_intensity() * self.emotional_amplification;
        let conv = self.biological_to_psychological_weight;

        let survival_bio = (needs.hunger * 0.5 + needs.thirst * 0.5).clamp(0.0, 1.0);
        let social_bio = social_cognition.cooperation_tendency.clamp(0.0, 1.0);
        let achievement_bio = (1.0 - needs.fatigue).clamp(0.0, 1.0);
        let exploration_bio = (1.0 - needs.fatigue * 0.5 - survival_bio * 0.5).clamp(0.0, 1.0);
        let reproduction_bio = (1.0 - needs.fatigue).clamp(0.0, 1.0) * 0.5;

        let urges = Urges {
            survival: (survival_bio * self.urge_sensitivities.survival * conv
                + emotional_intensity * 0.1)
                .clamp(0.0, 1.0),
            social: (social_bio * self.urge_sensitivities.social * conv
                + emotional_intensity * 0.1)
                .clamp(0.0, 1.0),
            achievement: (achievement_bio * self.urge_sensitivities.achievement * conv)
                .clamp(0.0, 1.0),
            exploration: (exploration_bio * self.urge_sensitivities.exploration * conv)
                .clamp(0.0, 1.0),
            reproduction: (reproduction_bio * self.urge_sensitivities.reproduction * conv)
                .clamp(0.0, 1.0),
        };

        // --- chaossys: seeded, deterministic, bounded perturbation ---
        // Physiological stress (via needs.fatigue as the real available
        // stress proxy at this layer) genuinely widens the chaos bound,
        // per `chaos_responses.stress_amplification`.
        let stress_widened = 1.0 + needs.fatigue * (self.stress_amplification - 1.0).max(0.0);
        let human_chunk = (human_id.0 & 0xFFFF_FFFF) as u32;
        let perturb = |kind: UrgeKind, bound: f64| -> f64 {
            let key = RngKey::new(SubsystemId::Humans, human_chunk, kind as u32, tick);
            let magnitude = bound * self.entropy_level * stress_widened;
            rng.gen_f64_range(key, -magnitude, magnitude)
        };

        let chaotic_urges = Urges {
            survival: (urges.survival + perturb(UrgeKind::Survival, self.chaos_bounds.survival))
                .clamp(0.0, 1.0),
            social: (urges.social + perturb(UrgeKind::Social, self.chaos_bounds.social))
                .clamp(0.0, 1.0),
            achievement: (urges.achievement
                + perturb(UrgeKind::Achievement, self.chaos_bounds.achievement))
            .clamp(0.0, 1.0),
            exploration: (urges.exploration
                + perturb(UrgeKind::Exploration, self.chaos_bounds.exploration))
            .clamp(0.0, 1.0),
            reproduction: (urges.reproduction
                + perturb(UrgeKind::Reproduction, self.chaos_bounds.reproduction))
            .clamp(0.0, 1.0),
        };

        // --- willsys: the gateway ---
        let (top_kind, top_value) = chaotic_urges.dominant();
        let effective_threshold = (self.baseline_threshold - self.autonomy_drive * 0.2
            + self.compliance_tendency * 0.2)
            .clamp(0.05, 0.95);

        // Willpower capacity depletes when sustained urgency is high
        // (decay_rate) and recovers when it isn't (recovery_rate) — a real
        // stateful resource, not a static threshold check. Both canon rates
        // are per hour of simulated time: resisting an urge wears willpower
        // down over hours, and rest restores it on the same scale.
        let hours = super::rates::elapsed(dt, super::rates::HOUR_YEARS);
        let willpower_capacity = if top_value > effective_threshold {
            (self.willpower_capacity - self.threshold_decay_rate * hours).clamp(0.05, 1.5)
        } else {
            (self.willpower_capacity + self.threshold_recovery_rate * hours).clamp(0.05, 1.5)
        };

        // --- willsys.interrupt_processing: pre-empting the held urge ---
        let elapsed_ms = (self.ms_since_last_interrupt + dt * MS_PER_YEAR).min(f64::MAX);
        let cooldown_ready = elapsed_ms >= self.interrupt_cooldown_ms as f64;
        let (focus_kind, focus_value, interrupted) = match self.dominant_urge {
            Some(held) if held != top_kind => {
                let held_value = chaotic_urges.get(held);
                let chaos_component = (top_value - urges.get(top_kind)).abs();
                let pressure = self.interrupt_urgency_weight * top_value
                    + self.interrupt_chaos_weight * chaos_component;
                let resistance = self.interrupt_context_modulation * held_value;
                if cooldown_ready && pressure > resistance {
                    (top_kind, top_value, true)
                } else {
                    (held, held_value, false)
                }
            }
            _ => (top_kind, top_value, false),
        };

        let has_attention = attention.available >= self.attention_threshold;
        let clears_gate =
            focus_value > effective_threshold && willpower_capacity > effective_threshold;
        let willpower_available = clears_gate && has_attention;
        let dominant_urge = if willpower_available {
            Some(focus_kind)
        } else {
            None
        };

        Self {
            urges,
            chaotic_urges,
            willpower_capacity,
            willpower_available,
            dominant_urge,
            interrupted: interrupted && willpower_available,
            ms_since_last_interrupt: if interrupted && willpower_available {
                0.0
            } else {
                elapsed_ms
            },
            ..self.clone()
        }
    }

    pub fn urge(&self, kind: UrgeKind) -> f64 {
        self.chaotic_urges.get(kind)
    }

    fn defaults() -> Self {
        Self {
            biological_to_psychological_weight: 0.7,
            emotional_amplification: 1.0,
            urge_sensitivities: Urges {
                survival: 0.6,
                social: 0.4,
                achievement: 0.4,
                exploration: 0.3,
                reproduction: 0.3,
            },
            chaos_bounds: Urges {
                survival: 0.1,
                social: 0.1,
                achievement: 0.1,
                exploration: 0.1,
                reproduction: 0.1,
            },
            entropy_level: 0.3,
            stress_amplification: 1.0,
            baseline_threshold: 0.4,
            threshold_decay_rate: 0.1,
            threshold_recovery_rate: 0.15,
            autonomy_drive: 0.5,
            compliance_tendency: 0.5,
            attention_threshold: 0.2,
            interrupt_urgency_weight: default_interrupt_urgency_weight(),
            interrupt_chaos_weight: default_interrupt_chaos_weight(),
            interrupt_context_modulation: default_interrupt_context_modulation(),
            interrupt_cooldown_ms: DEFAULT_INTERRUPT_COOLDOWN_MS,
            urges: Urges {
                survival: 0.3,
                social: 0.2,
                achievement: 0.2,
                exploration: 0.2,
                reproduction: 0.1,
            },
            chaotic_urges: Urges {
                survival: 0.3,
                social: 0.2,
                achievement: 0.2,
                exploration: 0.2,
                reproduction: 0.1,
            },
            willpower_capacity: 0.5,
            willpower_available: false,
            dominant_urge: None,
            interrupted: false,
            ms_since_last_interrupt: cooldown_fully_elapsed(),
        }
    }
}

fn nz(value: f32, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One hour: willpower rates are per hour.
    const HOUR: f64 = crate::humans::rates::HOUR_YEARS;
    use crate::humans::attention::AttentionSnapshot;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::needs::NeedsSnapshot;
    use crate::humans::social_cognition::SocialCognitionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("core_systems_test");
        HumanProfile::from_canonical_schema(HumanId::new(7), schema)
    }

    fn rng() -> RngRegistry {
        RngRegistry::new([9u8; 32])
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        assert!(snapshot.urges.survival > 0.0);
        assert!(!snapshot.willpower_available);
    }

    #[test]
    fn high_hunger_and_thirst_raise_survival_urge() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 1.0;
        needs.thirst = 1.0;
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let registry = rng();

        let stepped = snapshot.step(
            HumanId::new(7),
            10,
            &needs,
            &emotion,
            &social,
            &attention,
            &registry,
            1.0,
        );

        assert!(stepped.urges.survival > snapshot.urges.survival);
    }

    #[test]
    fn same_seed_and_tick_produce_identical_chaos() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let registry = rng();

        let a = snapshot.step(
            HumanId::new(7),
            100,
            &needs,
            &emotion,
            &social,
            &attention,
            &registry,
            1.0,
        );
        let b = snapshot.step(
            HumanId::new(7),
            100,
            &needs,
            &emotion,
            &social,
            &attention,
            &registry,
            1.0,
        );

        assert_eq!(a.chaotic_urges.survival, b.chaotic_urges.survival);
        assert_eq!(a.chaotic_urges.reproduction, b.chaotic_urges.reproduction);
    }

    #[test]
    fn different_ticks_produce_different_chaos() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let registry = rng();

        let a = snapshot.step(
            HumanId::new(7),
            1,
            &needs,
            &emotion,
            &social,
            &attention,
            &registry,
            1.0,
        );
        let b = snapshot.step(
            HumanId::new(7),
            2,
            &needs,
            &emotion,
            &social,
            &attention,
            &registry,
            1.0,
        );

        assert_ne!(a.chaotic_urges.survival, b.chaotic_urges.survival);
    }

    #[test]
    fn extreme_urgency_clears_willpower_gate_with_attention() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 1.0;
        needs.thirst = 1.0;
        needs.fatigue = 0.0;
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let mut attention = AttentionSnapshot::from_profile(&profile());
        attention.available = 1.0;
        let registry = rng();

        let mut stepped = snapshot.clone();
        for tick in 0..30 {
            stepped = stepped.step(
                HumanId::new(7),
                tick,
                &needs,
                &emotion,
                &social,
                &attention,
                &registry,
                HOUR,
            );
            if stepped.willpower_available {
                break;
            }
        }

        assert!(stepped.willpower_available);
        assert_eq!(stepped.dominant_urge, Some(UrgeKind::Survival));
    }

    #[test]
    fn no_attention_blocks_gate_even_at_high_urgency() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 1.0;
        needs.thirst = 1.0;
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let mut attention = AttentionSnapshot::from_profile(&profile());
        attention.available = 0.0;
        let registry = rng();

        let mut stepped = snapshot.clone();
        for tick in 0..30 {
            stepped = stepped.step(
                HumanId::new(7),
                tick,
                &needs,
                &emotion,
                &social,
                &attention,
                &registry,
                HOUR,
            );
        }

        assert!(!stepped.willpower_available);
    }

    #[test]
    fn willpower_capacity_depletes_under_sustained_urgency() {
        let snapshot = CoreSystemsSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 1.0;
        needs.thirst = 1.0;
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let registry = rng();

        let mut stepped = snapshot.clone();
        for tick in 0..10 {
            stepped = stepped.step(
                HumanId::new(7),
                tick,
                &needs,
                &emotion,
                &social,
                &attention,
                &registry,
                HOUR,
            );
        }

        assert!(stepped.willpower_capacity <= snapshot.willpower_capacity);
    }

    /// A human already focused on `Social` who becomes starved and parched,
    /// so a stronger urge challenges the held one.
    fn step_with_urgent_challenger(
        snapshot: &CoreSystemsSnapshot,
        dt_years: f64,
    ) -> CoreSystemsSnapshot {
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 1.0;
        needs.thirst = 1.0;
        needs.fatigue = 0.0;
        let emotion = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let mut attention = AttentionSnapshot::from_profile(&profile());
        attention.available = 1.0;
        snapshot.step(
            HumanId::new(7),
            3,
            &needs,
            &emotion,
            &social,
            &attention,
            &rng(),
            dt_years,
        )
    }

    fn focused_on_social() -> CoreSystemsSnapshot {
        let mut snapshot = CoreSystemsSnapshot::from_profile(&profile());
        snapshot.dominant_urge = Some(UrgeKind::Social);
        snapshot.willpower_capacity = 1.5;
        snapshot
    }

    #[test]
    fn stronger_urge_interrupts_held_focus_once_cooldown_elapsed() {
        let stepped = step_with_urgent_challenger(&focused_on_social(), HOUR);

        let (challenger, _) = stepped.chaotic_urges.dominant();
        assert_ne!(challenger, UrgeKind::Social);
        assert!(stepped.interrupted);
        assert_eq!(stepped.dominant_urge, Some(challenger));
        assert_eq!(stepped.ms_since_last_interrupt, 0.0);
    }

    #[test]
    fn interrupt_is_refused_inside_the_cooldown_window() {
        let mut snapshot = focused_on_social();
        snapshot.ms_since_last_interrupt = 0.0;
        let one_ms_in_years = 1.0 / MS_PER_YEAR;

        let stepped = step_with_urgent_challenger(&snapshot, one_ms_in_years);

        let (challenger, _) = stepped.chaotic_urges.dominant();
        assert!(!stepped.interrupted);
        assert_ne!(stepped.dominant_urge, Some(challenger));
        assert!((stepped.ms_since_last_interrupt - 1.0).abs() < 1e-6);
    }

    #[test]
    fn high_context_modulation_lets_held_focus_resist() {
        let mut snapshot = focused_on_social();
        snapshot.interrupt_context_modulation = 50.0;

        let stepped = step_with_urgent_challenger(&snapshot, HOUR);

        let (challenger, _) = stepped.chaotic_urges.dominant();
        assert!(!stepped.interrupted);
        assert_ne!(stepped.dominant_urge, Some(challenger));
    }

    #[test]
    fn canon_interrupt_processing_fields_are_read() {
        let mut schema = HumanSchema::canonical_minimal("core_systems_interrupts");
        schema
            .core_systems
            .willsys
            .interrupt_processing
            .context_modulation = 0.9;
        schema
            .core_systems
            .willsys
            .interrupt_processing
            .interrupt_cooldown = 1200;
        let profile = HumanProfile::from_canonical_schema(HumanId::new(8), schema);

        let snapshot = CoreSystemsSnapshot::from_profile(&profile);

        assert!((snapshot.interrupt_context_modulation - 0.9).abs() < 1e-6);
        assert_eq!(snapshot.interrupt_cooldown_ms, 1200);
    }
}
