//! Immune Snapshot - innate/adaptive immunity, system stress, pathogen load.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `ImmuneSystemSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Coupled to `needs`/`body`: fatigue and poor nutrition suppress immune
//! function, mirroring real physiology, rather than immunity sitting
//! static and disconnected from the rest of the survival state.
//!
//! Canon's `pathogens`/`immune_responses` are per-entity arrays (each
//! pathogen with a type/virulence/replication_rate/load, each response
//! targeting one). This snapshot tracks them as real per-tick entities
//! rather than the single aggregate `infection_load` scalar it previously
//! stood in for — `active_pathogen_count`/`infection_load` are now
//! read-outs derived from `pathogens`, not independent state. Spawn timing
//! uses this project's seeded `RngRegistry` (deterministic per human/tick),
//! never `rand::random`. Canon's per-pathogen `location`/`Coordinates3D`
//! is left at its zero default: no body-position model exists yet to place
//! pathogens honestly, the same reasoning `proprioception.rs` documents
//! for kinematic pose fields.

use mk_core::human::HumanProfile;
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathogenType {
    Virus,
    Bacteria,
    Fungus,
    Parasite,
}

impl PathogenType {
    const ALL: [PathogenType; 4] = [
        PathogenType::Virus,
        PathogenType::Bacteria,
        PathogenType::Fungus,
        PathogenType::Parasite,
    ];

    /// How quickly this pathogen type grows once established, before
    /// immune defenses are applied.
    /// Load growth per day at full virulence, before immune clearance.
    fn base_replication_rate(self) -> f64 {
        match self {
            PathogenType::Virus => 0.35,
            PathogenType::Bacteria => 0.25,
            PathogenType::Fungus => 0.12,
            PathogenType::Parasite => 0.08,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pathogen {
    pub id: String,
    pub pathogen_type: PathogenType,
    pub virulence: f64,
    pub replication_rate: f64,
    pub load: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImmuneResponseType {
    Inflammation,
    Fever,
    Antibody,
    CellMediated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmuneResponse {
    pub response_type: ImmuneResponseType,
    pub target_pathogen_id: String,
    pub intensity: f64,
    pub effectiveness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmuneSnapshot {
    // --- innate immunity ---
    pub macrophages: f64,
    pub neutrophils: f64,
    pub nk_cells: f64,
    pub complement: f64,
    pub inflammation: f64,

    // --- adaptive immunity ---
    pub t_cells: f64,
    pub b_cells: f64,
    pub memory_cells: f64,
    pub vaccination_count: usize,

    // --- overall ---
    pub system_stress: f64,
    pub autoimmunity_risk: f64,
    pub active_pathogen_count: usize,
    pub immune_response_count: usize,

    /// Total load across [`Self::pathogens`] — a readout, not independent
    /// state.
    pub infection_load: f64,

    /// Real per-entity pathogen population. Individual pathogens spawn
    /// (opportunistic infection taking hold under weak defenses) and clear
    /// (defenses winning) rather than infection being a single undifferentiated
    /// scalar.
    pub pathogens: Vec<Pathogen>,
    /// One real response per active pathogen, recomputed each tick from
    /// current defense strength against that specific pathogen.
    pub immune_responses: Vec<ImmuneResponse>,
}

const IMMUNE_SPAWN_EPOCH: u32 = 9001;
const IMMUNE_TYPE_EPOCH: u32 = 9002;
const IMMUNE_VIRULENCE_EPOCH: u32 = 9003;

impl ImmuneSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let immune = &schema.immune_system;

        Self {
            macrophages: nonzero_or(immune.innate_immunity.macrophages as f64, 0.7),
            neutrophils: nonzero_or(immune.innate_immunity.neutrophils as f64, 0.7),
            nk_cells: nonzero_or(immune.innate_immunity.nk_cells as f64, 0.6),
            complement: nonzero_or(immune.innate_immunity.complement as f64, 0.6),
            inflammation: immune.innate_immunity.inflammation as f64,

            t_cells: nonzero_or(immune.adaptive_immunity.t_cells as f64, 0.6),
            b_cells: nonzero_or(immune.adaptive_immunity.b_cells as f64, 0.6),
            memory_cells: immune.adaptive_immunity.memory_cells as f64,
            vaccination_count: immune.adaptive_immunity.vaccination_history.len(),

            system_stress: immune.system_stress as f64,
            autoimmunity_risk: immune.autoimmunity_risk as f64,
            active_pathogen_count: immune.pathogens.len(),
            immune_response_count: immune.immune_responses.len(),
            infection_load: immune
                .pathogens
                .iter()
                .map(|p| p.load as f64)
                .sum::<f64>()
                .max(immune.pathogens.len() as f64),
            pathogens: immune
                .pathogens
                .iter()
                .enumerate()
                .map(|(i, p)| Pathogen {
                    id: if p.id.is_empty() {
                        format!("canon-{i}")
                    } else {
                        p.id.clone()
                    },
                    pathogen_type: match p.pathogen_type {
                        Some(mk_core::human::schema::PathogenTypeSchema::Virus) | None => {
                            PathogenType::Virus
                        }
                        Some(mk_core::human::schema::PathogenTypeSchema::Bacteria) => {
                            PathogenType::Bacteria
                        }
                        Some(mk_core::human::schema::PathogenTypeSchema::Fungus) => {
                            PathogenType::Fungus
                        }
                        Some(mk_core::human::schema::PathogenTypeSchema::Parasite)
                        | Some(mk_core::human::schema::PathogenTypeSchema::Toxin) => {
                            PathogenType::Parasite
                        }
                    },
                    virulence: nonzero_or(p.virulence as f64, 0.3),
                    replication_rate: nonzero_or(p.replication_rate as f64, 0.2),
                    load: nonzero_or(p.load as f64, 1.0),
                })
                .collect(),
            immune_responses: Vec::new(),
        }
    }

    /// Step immune function forward, suppressed by fatigue/malnutrition and
    /// recovering with rest/nutrition — the same coupling real immune
    /// systems have to sleep and diet, instead of immunity being an inert
    /// number nothing else touches. `human_id`/`tick`/`rng` drive
    /// deterministic opportunistic-infection spawn timing.
    pub fn step(
        &self,
        needs: &super::needs::NeedsSnapshot,
        human_id: mk_core::human::HumanId,
        tick: u64,
        rng: &RngRegistry,
        dt_years: f64,
    ) -> Self {
        use super::rates::{
            elapsed, event_probability, relaxation_fraction, DAY_YEARS, MONTH_YEARS,
        };
        let days = elapsed(dt_years, DAY_YEARS);

        // Suppression factor: 0 = fully suppressed, 1 = no suppression.
        let suppression =
            ((needs.glucose + needs.hydration) / 2.0 * (1.0 - needs.fatigue)).clamp(0.1, 1.0);

        // Immune cell populations relax toward their (suppressed) baseline
        // at 0.5 per day: counts respond to deprivation over days.
        let recovery = relaxation_fraction(0.5, dt_years, DAY_YEARS);
        let recover = |current: f64, baseline: f64| -> f64 {
            let target = baseline * suppression;
            (current + (target - current) * recovery).clamp(0.0, 1.0)
        };

        let macrophages = recover(self.macrophages, 0.7);
        let neutrophils = recover(self.neutrophils, 0.7);
        let nk_cells = recover(self.nk_cells, 0.6);
        let complement = recover(self.complement, 0.6);
        let t_cells = recover(self.t_cells, 0.6);
        let b_cells = recover(self.b_cells, 0.6);

        // System stress rises with sustained low suppression factor (rates
        // per day).
        let system_stress = (self.system_stress + (1.0 - suppression) * 0.3 * days
            - suppression * 0.1 * days)
            .clamp(0.0, 1.0);

        let defense_strength = (macrophages * 0.3 + t_cells * 0.4 + nk_cells * 0.3).clamp(0.0, 1.0);

        // Existing pathogens: replicate against defense strength, generating
        // a real response for each, and clear once load hits zero.
        let mut pathogens: Vec<Pathogen> = self
            .pathogens
            .iter()
            .map(|p| {
                // Replication and clearance are both per day.
                let growth = p.replication_rate * p.virulence * days;
                let clearance = defense_strength * 0.4 * days;
                Pathogen {
                    load: (p.load + growth - clearance).max(0.0),
                    ..p.clone()
                }
            })
            .filter(|p| p.load > 0.01)
            .collect();

        // Opportunistic new infection: weak defenses under sustained
        // deprivation let a new pathogen take hold. Deterministic per
        // (human, tick) via the project's seeded RngRegistry — never
        // `rand::random`.
        let human_chunk = (human_id.0 & 0xFFFF_FFFF) as u32;
        let spawn_roll = rng.gen_f64_range(
            RngKey::new(SubsystemId::Humans, human_chunk, IMMUNE_SPAWN_EPOCH, tick),
            0.0,
            1.0,
        );
        // Infection hazard per month: none for a well-fed human with healthy
        // defenses, up to ~0.15 a month (about two a year) when deprivation
        // suppresses them.
        let hazard_per_month = ((1.0 - suppression) * 0.15 - defense_strength * 0.1).max(0.0);
        let spawn_chance = event_probability(hazard_per_month, dt_years, MONTH_YEARS);
        if spawn_roll < spawn_chance {
            let type_roll = rng.gen_f64_range(
                RngKey::new(SubsystemId::Humans, human_chunk, IMMUNE_TYPE_EPOCH, tick),
                0.0,
                PathogenType::ALL.len() as f64,
            );
            let pathogen_type =
                PathogenType::ALL[(type_roll as usize).min(PathogenType::ALL.len() - 1)];
            pathogens.push(Pathogen {
                id: format!("p-{human_chunk:x}-{tick}"),
                pathogen_type,
                virulence: rng.gen_f64_range(
                    RngKey::new(
                        SubsystemId::Humans,
                        human_chunk,
                        IMMUNE_VIRULENCE_EPOCH,
                        tick,
                    ),
                    0.2,
                    0.8,
                ),
                replication_rate: pathogen_type.base_replication_rate(),
                load: 0.1,
            });
        }

        let immune_responses: Vec<ImmuneResponse> = pathogens
            .iter()
            .map(|p| {
                let (response_type, effectiveness) = match p.pathogen_type {
                    PathogenType::Virus => {
                        (ImmuneResponseType::Antibody, t_cells * 0.6 + b_cells * 0.4)
                    }
                    PathogenType::Bacteria => (
                        ImmuneResponseType::CellMediated,
                        macrophages * 0.5 + neutrophils * 0.5,
                    ),
                    PathogenType::Fungus => (ImmuneResponseType::Inflammation, complement * 0.6),
                    PathogenType::Parasite => {
                        (ImmuneResponseType::Fever, nk_cells * 0.5 + t_cells * 0.5)
                    }
                };
                ImmuneResponse {
                    response_type,
                    target_pathogen_id: p.id.clone(),
                    intensity: p.load.clamp(0.0, 1.0),
                    effectiveness: effectiveness.clamp(0.0, 1.0),
                }
            })
            .collect();

        let infection_load: f64 = pathogens.iter().map(|p| p.load).sum();
        let active_pathogen_count = pathogens.len();
        let immune_response_count = immune_responses.len();

        Self {
            macrophages,
            neutrophils,
            nk_cells,
            complement,
            t_cells,
            b_cells,
            system_stress,
            infection_load,
            active_pathogen_count,
            immune_response_count,
            pathogens,
            immune_responses,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            macrophages: 0.7,
            neutrophils: 0.7,
            nk_cells: 0.6,
            complement: 0.6,
            inflammation: 0.0,
            t_cells: 0.6,
            b_cells: 0.6,
            memory_cells: 0.0,
            vaccination_count: 0,
            system_stress: 0.0,
            autoimmunity_risk: 0.0,
            active_pathogen_count: 0,
            immune_response_count: 0,
            infection_load: 0.0,
            pathogens: Vec::new(),
            immune_responses: Vec::new(),
        }
    }
}

fn nonzero_or(value: f64, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanId, HumanSchema};
    use mk_core::rng::RngRegistry;

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("immune_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    fn rng() -> RngRegistry {
        RngRegistry::new([7u8; 32])
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = ImmuneSnapshot::from_profile(&profile());
        assert!(snapshot.macrophages > 0.0);
        assert_eq!(snapshot.active_pathogen_count, 0);
    }

    #[test]
    fn step_suppresses_immunity_under_poor_needs() {
        use crate::agents::AgentWorldObservation;
        use crate::humans::needs::{EffortFocus, NeedsSnapshot};

        let good_needs = NeedsSnapshot::from_profile(&profile());
        let starved_observation = AgentWorldObservation {
            caloric_access: 0.0,
            hydration_access: 0.0,
            shelter_quality: 0.0,
            ..AgentWorldObservation::default()
        };
        let mut poor_needs = good_needs.clone();
        for _ in 0..20 {
            poor_needs = poor_needs.step(&starved_observation, 1.0, EffortFocus::none(), 1.0);
        }

        let rng = rng();
        let snapshot = ImmuneSnapshot::from_profile(&profile());
        let stable = snapshot.step(&good_needs, HumanId::new(1), 0, &rng, 1.0);
        let stressed = snapshot.step(&poor_needs, HumanId::new(1), 0, &rng, 1.0);

        assert!(stressed.system_stress >= stable.system_stress);
    }

    #[test]
    fn sustained_deprivation_makes_active_pathogen_count_rise() {
        use crate::agents::AgentWorldObservation;
        use crate::humans::needs::{EffortFocus, NeedsSnapshot};

        let good_needs = NeedsSnapshot::from_profile(&profile());
        let starved_observation = AgentWorldObservation {
            caloric_access: 0.0,
            hydration_access: 0.0,
            shelter_quality: 0.0,
            ..AgentWorldObservation::default()
        };
        let mut poor_needs = good_needs.clone();
        let rng = rng();

        let healthy_start = ImmuneSnapshot::from_profile(&profile());
        let mut sick = healthy_start.clone();
        for tick in 0..200u64 {
            poor_needs = poor_needs.step(&starved_observation, 1.0, EffortFocus::none(), 1.0);
            sick = sick.step(&poor_needs, HumanId::new(1), tick, &rng, 1.0);
        }

        assert!(sick.active_pathogen_count > healthy_start.active_pathogen_count);
        assert!(sick.infection_load > 0.0);
        assert!(!sick.immune_responses.is_empty());
    }

    #[test]
    fn recovery_clears_pathogens_over_time() {
        use crate::humans::needs::NeedsSnapshot;

        let good_needs = NeedsSnapshot::from_profile(&profile());
        let mut sick = ImmuneSnapshot::from_profile(&profile());
        sick.pathogens = vec![
            Pathogen {
                id: "seed-1".into(),
                pathogen_type: PathogenType::Bacteria,
                virulence: 0.6,
                replication_rate: 0.2,
                load: 1.5,
            },
            Pathogen {
                id: "seed-2".into(),
                pathogen_type: PathogenType::Virus,
                virulence: 0.5,
                replication_rate: 0.3,
                load: 1.5,
            },
        ];
        sick.infection_load = 3.0;
        sick.active_pathogen_count = 2;

        let rng = rng();
        let mut recovering = sick.clone();
        for tick in 0..60u64 {
            recovering = recovering.step(&good_needs, HumanId::new(1), tick, &rng, 1.0);
        }

        assert!(recovering.infection_load < sick.infection_load);
    }
}
