//! Real time and real energy for human work (Phase 3 Task 3b).
//!
//! Upstream actions are instantaneous and free. Here every task has a
//! duration, an output rate and an energy cost drawn from the Phase-0c
//! reference packs (`fixtures/reference/labour`, `humans/physiology`):
//!
//! - **Time and output** per action and tool: felling a 30 cm tree takes the
//!   packs' minutes for the tool (stone axe to chainsaw), digging, mining and
//!   gold panning follow the packs' per-worker daily rates, timber-frame
//!   building the packs' person-hours per m².
//! - **Energy**: expenditure is `BMR × MET × hours`, with BMR from the body
//!   by Mifflin–St Jeor and the MET of the activity from the packs
//!   (Compendium of Physical Activities). [`effort_for_met`] turns a MET into
//!   the needs model's `EffortFocus::activity`, so heavy labour drains
//!   glucose and water at realistic rates.
//! - **Walking** takes real time: the reference speed on the flat, Naismith's
//!   rule for ascent, slowed when the load passes the sustainable fraction
//!   of body mass; loads above the packs' maximum are refused.
//!
//! [`ActiveTask`] carries a task across ticks, delivering output as the
//! work progresses.

use mk_core::human::BiologicalSex;

use crate::humans::needs::EffortFocus;
use crate::humans::HumanBeing;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// Activity level (MET) the needs model's drain rates are calibrated for:
/// light daily activity.
pub const BASELINE_MET: f64 = 1.5;
/// Working hours in a labour day the packs' per-day rates refer to.
const WORKING_DAY_HOURS: f64 = 8.0;
/// Reference tree diameter (cm) of the felling times.
const REFERENCE_TREE_DIAMETER_CM: f64 = 30.0;

#[derive(Debug, Clone, PartialEq)]
pub enum LabourError {
    /// A reference item the model needs is missing from the packs.
    MissingReference(String),
    /// A load above what a body of this mass can carry.
    OverLoad { carried_kg: f64, max_kg: f64 },
}

impl std::fmt::Display for LabourError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingReference(k) => write!(f, "reference item {k} is missing"),
            Self::OverLoad { carried_kg, max_kg } => {
                write!(
                    f,
                    "a load of {carried_kg:.1} kg exceeds the {max_kg:.1} kg limit"
                )
            }
        }
    }
}

impl std::error::Error for LabourError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabourTool {
    StoneAxe,
    SteelAxe,
    CrosscutSaw,
    Chainsaw,
    PickAndShovel,
    GoldPan,
    SluiceBox,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DigMaterial {
    SoftSoil,
    HardClayOrGravel,
    SoftRock,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabourAction {
    FellTree {
        diameter_cm: f64,
    },
    Dig {
        material: DigMaterial,
        volume_m3: f64,
    },
    MineHardRock {
        tonnes: f64,
    },
    PanGravel {
        grade_g_m3: f64,
        coarse_gold: bool,
    },
    BuildTimberFrame {
        floor_m2: f64,
    },
}

/// One kind of work with its tool: how long, what it yields, what it costs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskSpec {
    pub action: LabourAction,
    pub tool: LabourTool,
    pub duration_s: f64,
    pub met: f64,
    /// Output over the whole task (trees, m³ dug, tonnes mined, grams of
    /// gold, m² built).
    pub output: f64,
    pub output_unit: &'static str,
}

impl TaskSpec {
    /// Output per hour of work.
    pub fn output_per_hour(&self) -> f64 {
        if self.duration_s > 0.0 {
            self.output / (self.duration_s / 3_600.0)
        } else {
            0.0
        }
    }
}

/// A (min, max) range and its middle, from a reference item.
#[derive(Debug, Clone, Copy)]
struct Range(f64, f64);

impl Range {
    fn mid(self) -> f64 {
        0.5 * (self.0 + self.1)
    }
}

/// The reference data the labour model runs on.
#[derive(Debug, Clone)]
pub struct LabourTable {
    set: ReferenceLibrary,
}

impl LabourTable {
    pub fn from_reference(set: ReferenceLibrary) -> Self {
        Self { set }
    }

    /// Load the repository's reference packs.
    pub fn load_default() -> Result<Self, LabourError> {
        ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map(Self::from_reference)
            .map_err(|e| LabourError::MissingReference(e.to_string()))
    }

    fn table_range(
        &self,
        domain: ReferenceDomain,
        key: &str,
        row: &str,
    ) -> Result<Range, LabourError> {
        let missing = || LabourError::MissingReference(format!("{key}/{row}"));
        let table = self
            .set
            .item(domain, key)
            .and_then(|i| i.table())
            .ok_or_else(missing)?;
        Ok(Range(
            table.get(row, "min").ok_or_else(missing)?,
            table.get(row, "max").ok_or_else(missing)?,
        ))
    }

    fn range(
        &self,
        domain: ReferenceDomain,
        key: &str,
    ) -> Result<(Range, Option<f64>), LabourError> {
        let item = self
            .set
            .item(domain, key)
            .ok_or_else(|| LabourError::MissingReference(key.into()))?;
        let (lo, hi) = item
            .range()
            .ok_or_else(|| LabourError::MissingReference(key.into()))?;
        Ok((Range(lo, hi), item.central()))
    }

    fn scalar(&self, domain: ReferenceDomain, key: &str, row: &str) -> Result<f64, LabourError> {
        self.set
            .item(domain, key)
            .and_then(|i| i.table())
            .and_then(|t| t.get(row, "value"))
            .ok_or_else(|| LabourError::MissingReference(format!("{key}/{row}")))
    }

    /// Middle of an activity's MET range (Compendium of Physical Activities).
    pub fn met(&self, activity: &str) -> Result<f64, LabourError> {
        Ok(self
            .table_range(ReferenceDomain::Humans, "met_by_activity", activity)?
            .mid())
    }

    /// The task of felling a tree of `diameter_cm` with `tool`: the packs'
    /// time for a 30 cm tree, scaled by the trunk's cross-section.
    pub fn fell_tree(&self, tool: LabourTool, diameter_cm: f64) -> Result<TaskSpec, LabourError> {
        let (row, activity) = match tool {
            LabourTool::StoneAxe => ("stone_axe", "felling_trees_hand_axe"),
            LabourTool::SteelAxe => ("steel_axe", "felling_trees_hand_axe"),
            LabourTool::CrosscutSaw => ("two_person_crosscut_saw", "felling_trees_hand_axe"),
            LabourTool::Chainsaw => ("chainsaw", "chainsaw_felling"),
            _ => return Err(LabourError::MissingReference("a felling tool".into())),
        };
        let minutes = self
            .table_range(ReferenceDomain::Labour, "felling_time_30cm_tree", row)?
            .mid();
        let scale = (diameter_cm.max(1.0) / REFERENCE_TREE_DIAMETER_CM).powi(2);
        Ok(TaskSpec {
            action: LabourAction::FellTree { diameter_cm },
            tool,
            duration_s: minutes * 60.0 * scale,
            met: self.met(activity)?,
            output: 1.0,
            output_unit: "tree",
        })
    }

    /// Digging `volume_m3` of `material` by pick and shovel at the packs'
    /// per-worker daily rate.
    pub fn dig(&self, material: DigMaterial, volume_m3: f64) -> Result<TaskSpec, LabourError> {
        let row = match material {
            DigMaterial::SoftSoil => "soft_soil",
            DigMaterial::HardClayOrGravel => "hard_clay_or_gravel",
            DigMaterial::SoftRock => "soft_rock",
        };
        let per_day = self
            .table_range(ReferenceDomain::Labour, "hand_excavation_rate", row)?
            .mid();
        Ok(TaskSpec {
            action: LabourAction::Dig {
                material,
                volume_m3,
            },
            tool: LabourTool::PickAndShovel,
            duration_s: volume_m3 / per_day * WORKING_DAY_HOURS * 3_600.0,
            met: self.met("digging_shoveling")?,
            output: volume_m3,
            output_unit: "m3",
        })
    }

    /// Breaking and hauling `tonnes` of hard rock by hand.
    pub fn mine_hard_rock(&self, tonnes: f64) -> Result<TaskSpec, LabourError> {
        let (range, typical) = self.range(ReferenceDomain::Labour, "artisanal_hard_rock_mining")?;
        let per_day = typical.unwrap_or_else(|| range.mid());
        Ok(TaskSpec {
            action: LabourAction::MineHardRock { tonnes },
            tool: LabourTool::PickAndShovel,
            duration_s: tonnes / per_day * WORKING_DAY_HOURS * 3_600.0,
            met: self.met("hand_mining")?,
            output: tonnes,
            output_unit: "t",
        })
    }

    /// One working day of washing placer gravel of `grade_g_m3` (g of gold
    /// per m³ in place) with a pan or a sluice: yield is grade × volume ×
    /// recovery, all from the packs.
    pub fn pan_gravel(
        &self,
        tool: LabourTool,
        grade_g_m3: f64,
        coarse_gold: bool,
    ) -> Result<TaskSpec, LabourError> {
        let (rate_row, recovery_row) = match (tool, coarse_gold) {
            (LabourTool::GoldPan, true) => ("gold_pan", "pan_coarse_gold"),
            (LabourTool::GoldPan, false) => ("gold_pan", "pan_fine_gold"),
            (LabourTool::SluiceBox, true) => ("sluice_box", "sluice_coarse_gold"),
            (LabourTool::SluiceBox, false) => ("sluice_box", "sluice_fine_gold"),
            _ => return Err(LabourError::MissingReference("a panning tool".into())),
        };
        let volume = self
            .table_range(ReferenceDomain::Labour, "gold_panning_throughput", rate_row)?
            .mid();
        let recovery = self
            .table_range(
                ReferenceDomain::Labour,
                "placer_gold_recovery",
                recovery_row,
            )?
            .mid();
        Ok(TaskSpec {
            action: LabourAction::PanGravel {
                grade_g_m3,
                coarse_gold,
            },
            tool,
            duration_s: WORKING_DAY_HOURS * 3_600.0,
            met: self.met("gold_panning_in_stream")?,
            output: grade_g_m3.max(0.0) * volume * recovery,
            output_unit: "g gold",
        })
    }

    /// Building `floor_m2` of timber-frame house; hand tools take roughly
    /// twice the packs' skilled-with-power-tools hours.
    pub fn build_timber_frame(
        &self,
        floor_m2: f64,
        power_tools: bool,
    ) -> Result<TaskSpec, LabourError> {
        let (range, typical) =
            self.range(ReferenceDomain::Labour, "timber_frame_construction_labour")?;
        let hours_per_m2 =
            typical.unwrap_or_else(|| range.mid()) * if power_tools { 1.0 } else { 2.0 };
        Ok(TaskSpec {
            action: LabourAction::BuildTimberFrame { floor_m2 },
            tool: LabourTool::PickAndShovel,
            duration_s: floor_m2 * hours_per_m2 * 3_600.0,
            met: self.met("carpentry_general")?,
            output: floor_m2,
            output_unit: "m2",
        })
    }

    /// Basal metabolic rate (kcal/day), Mifflin–St Jeor with the packs' terms.
    pub fn bmr_kcal_per_day(&self, body: &LabourBody) -> Result<f64, LabourError> {
        let h = ReferenceDomain::Humans;
        let key = "bmr_mifflin_st_jeor";
        let sex_constant = match body.sex {
            BiologicalSex::Male => self.scalar(h, key, "male_constant")?,
            _ => self.scalar(h, key, "female_constant")?,
        };
        Ok(self.scalar(h, key, "per_kg")? * body.weight_kg
            + self.scalar(h, key, "per_cm")? * body.height_cm
            + self.scalar(h, key, "per_year")? * body.age_years
            + sex_constant)
    }

    /// Energy (kcal) of `seconds` at `met`: `BMR/24 h × MET × hours`.
    pub fn energy_kcal(
        &self,
        body: &LabourBody,
        met: f64,
        seconds: f64,
    ) -> Result<f64, LabourError> {
        Ok(self.bmr_kcal_per_day(body)? / 24.0 * met.max(0.0) * seconds.max(0.0) / 3_600.0)
    }

    /// Time (s) to walk `distance_m` with `ascent_m` of climb carrying
    /// `load_kg`: the comfortable speed on the flat, an hour more per
    /// 600 m of ascent (Naismith), and a slowdown that grows as the load
    /// passes the sustainable fraction of body mass. A load above the
    /// packs' maximum fraction is refused.
    pub fn walking_time_s(
        &self,
        body: &LabourBody,
        distance_m: f64,
        ascent_m: f64,
        load_kg: f64,
    ) -> Result<f64, LabourError> {
        let max_kg = self.max_load_kg(body)?;
        if load_kg > max_kg {
            return Err(LabourError::OverLoad {
                carried_kg: load_kg,
                max_kg,
            });
        }
        let (_, typical) = self.range(ReferenceDomain::Humans, "comfortable_walking_speed")?;
        let speed = typical.unwrap_or(1.4);
        let ascent_per_hour = self.scalar(
            ReferenceDomain::Humans,
            "naismith_rule",
            "ascent_m_per_hour",
        )?;
        let sustainable_kg = self.sustainable_load_kg(body)?;
        // Beyond the sustainable load the pace falls linearly to 70% at the maximum.
        let over = ((load_kg - sustainable_kg) / (max_kg - sustainable_kg).max(f64::MIN_POSITIVE))
            .clamp(0.0, 1.0);
        let pace = 1.0 - 0.3 * over;
        Ok((distance_m.max(0.0) / speed + ascent_m.max(0.0) / ascent_per_hour * 3_600.0) / pace)
    }

    /// Load (kg) a body can carry for a day: the packs' typical fraction of
    /// body mass.
    pub fn sustainable_load_kg(&self, body: &LabourBody) -> Result<f64, LabourError> {
        let (_, typical) = self.range(ReferenceDomain::Humans, "sustainable_load_fraction")?;
        Ok(body.weight_kg * typical.unwrap_or(0.33))
    }

    /// The most a body may carry at all: the top of the packs' range.
    pub fn max_load_kg(&self, body: &LabourBody) -> Result<f64, LabourError> {
        let (range, _) = self.range(ReferenceDomain::Humans, "sustainable_load_fraction")?;
        Ok(body.weight_kg * range.1)
    }

    /// Refuse a carried load above the maximum.
    pub fn check_carry(&self, body: &LabourBody, load_kg: f64) -> Result<(), LabourError> {
        let max_kg = self.max_load_kg(body)?;
        if load_kg > max_kg {
            Err(LabourError::OverLoad {
                carried_kg: load_kg,
                max_kg,
            })
        } else {
            Ok(())
        }
    }
}

/// What the energy and walking models need of a body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabourBody {
    pub sex: BiologicalSex,
    pub age_years: f64,
    pub height_cm: f64,
    pub weight_kg: f64,
}

impl LabourBody {
    pub fn from_human(human: &HumanBeing) -> Self {
        Self {
            sex: *human.biological_sex(),
            age_years: human.development.age_years,
            height_cm: human.body.height_cm,
            weight_kg: human.body.weight_kg,
        }
    }
}

/// The needs model's effort for an activity of `met`: its drain scales with
/// the activity above [`BASELINE_MET`].
pub fn effort_for_met(met: f64) -> EffortFocus {
    EffortFocus {
        activity: (met / BASELINE_MET).max(1.0),
        ..EffortFocus::none()
    }
}

/// Energy (kcal) of a day described as `(met, hours)` blocks.
pub fn day_energy_kcal(
    table: &LabourTable,
    body: &LabourBody,
    blocks: &[(f64, f64)],
) -> Result<f64, LabourError> {
    let mut total = 0.0;
    for &(met, hours) in blocks {
        total += table.energy_kcal(body, met, hours * 3_600.0)?;
    }
    Ok(total)
}

/// What an [`ActiveTask`] did in one advance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskProgress {
    /// Output delivered by this advance.
    pub delivered: f64,
    /// Seconds of work done (≤ the time offered).
    pub worked_s: f64,
    pub finished: bool,
}

/// A task in progress, carried across human ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveTask {
    pub spec: TaskSpec,
    pub progress_s: f64,
}

impl ActiveTask {
    pub fn new(spec: TaskSpec) -> Self {
        Self {
            spec,
            progress_s: 0.0,
        }
    }

    pub fn finished(&self) -> bool {
        self.progress_s >= self.spec.duration_s
    }

    /// Work for up to `dt_s`; output is delivered in proportion to the work
    /// done, and none beyond the task's end.
    pub fn advance(&mut self, dt_s: f64) -> TaskProgress {
        if !(dt_s.is_finite() && dt_s > 0.0) || self.spec.duration_s <= 0.0 {
            return TaskProgress {
                delivered: 0.0,
                worked_s: 0.0,
                finished: self.finished(),
            };
        }
        let worked = dt_s.min(self.spec.duration_s - self.progress_s).max(0.0);
        self.progress_s += worked;
        TaskProgress {
            delivered: self.spec.output * worked / self.spec.duration_s,
            worked_s: worked,
            finished: self.finished(),
        }
    }
}
