//! mk_interventions - Typed intervention system for Maer'Ken Observatory
//!
//! This crate provides the typed intervention contracts for the UI to interact
//! with the simulation engine. All interventions flow through this system
//! to ensure full provenance and auditability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod operator;

/// Unique identifier for an intervention
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterventionId(pub String);

impl InterventionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for InterventionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Author of an intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionAuthor {
    pub user_id: String,
    pub session_id: String,
    pub display_name: String,
}

/// Source state reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceState {
    pub tick: u64,
    pub hash: [u8; 32],
    pub snapshot_id: Option<String>,
}

/// Resulting state after intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultState {
    pub tick: u64,
    pub hash: [u8; 32],
    pub snapshot_id: Option<String>,
    pub acknowledged: bool,
}

/// Core intervention record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    pub id: InterventionId,
    pub author: InterventionAuthor,
    pub timestamp: DateTime<Utc>,
    pub source: SourceState,
    pub action: InterventionAction,
    pub result: Option<ResultState>,
    pub status: InterventionStatus,
    pub error_message: Option<String>,
}

impl Intervention {
    /// Build the record without validating `action`; only [`Self::try_new`]
    /// calls it, so every intervention record is validated.
    fn new_unchecked(
        author: InterventionAuthor,
        source: SourceState,
        action: InterventionAction,
    ) -> Self {
        Self {
            id: InterventionId::new(),
            author,
            timestamp: Utc::now(),
            source,
            action,
            result: None,
            status: InterventionStatus::Pending,
            error_message: None,
        }
    }

    /// Build a validated intervention record: `action` must pass
    /// [`validate_intervention`].
    pub fn try_new(
        author: InterventionAuthor,
        source: SourceState,
        action: InterventionAction,
    ) -> Result<Self, ValidationResult> {
        let validation = validate_intervention(&action);
        if !validation.valid {
            return Err(validation);
        }
        Ok(Self::new_unchecked(author, source, action))
    }

    pub fn acknowledge(&mut self, result: ResultState) {
        self.result = Some(result);
        self.status = InterventionStatus::Acknowledged;
    }

    pub fn reject(&mut self, error: String) {
        self.status = InterventionStatus::Rejected;
        self.error_message = Some(error);
    }
}

/// Types of interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionAction {
    // Simulation controls
    Pause,
    Resume,
    Step {
        ticks: u64,
    },
    Scrub {
        target_tick: u64,
    },

    // Branching
    Branch {
        name: String,
        source_snapshot: String,
    },
    Fork {
        name: String,
        modifications: Vec<ScenarioModification>,
    },

    // Resource injections
    InjectResource {
        resource_type: ResourceType,
        amount: f32,
        location: Location,
    },
    InjectBiomass {
        biomass_type: BiomassType,
        amount: f32,
        region: Region,
    },
    InjectEnergy {
        energy_type: EnergyType,
        amount: f32,
        location: Location,
    },

    // Environmental modifications
    ModifyClimate {
        parameter: ClimateParameter,
        value: f32,
        region: Option<Region>,
    },
    TriggerDisturbance {
        disturbance_type: DisturbanceType,
        intensity: f32,
        location: Location,
    },
    /// Raise (positive) or lower (negative) the land surface across a
    /// region, peaking at `elevation_delta_m` at the centre and easing to
    /// zero at the region's edge.
    SculptTerrain {
        elevation_delta_m: f32,
        region: Region,
    },
    /// Relax the land surface across a region toward each cell's local
    /// neighbourhood mean; `strength` in `0..=1` is the fraction of the gap
    /// closed at the region centre.
    SmoothTerrain {
        strength: f32,
        region: Region,
    },

    // Scenario modifications
    ModifyScenario {
        parameter: String,
        value: ScenarioValue,
    },

    // Construction: place a real engine structure (one of the resource
    // economy's buildable recipes) at a surface location.
    ConstructStructure {
        structure: StructureKind,
        location: Location,
    },

    // Human interventions
    SpawnHuman {
        /// Biological-sex template: `"male"` or `"female"`.
        template_id: String,
        location: Location,
        /// Authored identity for the new human (Foundry spawns). Without it
        /// the engine's canonical newborn-profile defaults apply.
        #[serde(default)]
        profile: Option<HumanSpawnProfile>,
    },
    RemoveHuman {
        human_id: String,
    },
}

/// Authored identity for a spawned human.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanSpawnProfile {
    /// Display name; also the basis of the human's `agent_id`.
    pub name: String,
    /// RFC 3339 birth instant.
    pub birth_timestamp: String,
    pub birth_latitude: f64,
    pub birth_longitude: f64,
    /// Current age in years.
    pub age_years: f64,
    pub height_cm: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    pub skin_tone: String,
}

/// Largest single terrain edit, in metres of relief at the region centre.
pub const MAX_SCULPT_DELTA_M: f32 = 10_000.0;

/// Oldest age a spawned human may be authored at.
pub const MAX_SPAWN_AGE_YEARS: f64 = 130.0;

/// Authorable standing-height range for a spawned human, in centimetres.
pub const SPAWN_HEIGHT_RANGE_CM: std::ops::RangeInclusive<f64> = 40.0..=272.0;

/// Structures an operator can construct. Each variant is one of the resource
/// economy's buildable recipes, so a constructed structure is the same kind of
/// world object an agent builds from gathered materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureKind {
    WoodenShelter,
    Workshop,
    Storage,
    StoneHouse,
}

impl StructureKind {
    /// Every constructible structure, in palette display order.
    pub const ALL: [StructureKind; 4] = [
        StructureKind::WoodenShelter,
        StructureKind::Workshop,
        StructureKind::Storage,
        StructureKind::StoneHouse,
    ];

    /// Operator-facing name.
    pub fn display_name(self) -> &'static str {
        match self {
            StructureKind::WoodenShelter => "Wooden Shelter",
            StructureKind::Workshop => "Workshop",
            StructureKind::Storage => "Storage",
            StructureKind::StoneHouse => "Stone House",
        }
    }

    /// Inverse of [`StructureKind::display_name`].
    pub fn from_display_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.display_name() == name)
    }
}

/// Resource types for injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Minerals,
    Water,
    Nutrients,
    Energy,
    Organic,
}

/// Biomass types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiomassType {
    Producers,
    Consumers,
    Apex,
    Decomposers,
}

/// Energy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyType {
    Solar,
    Thermal,
    Chemical,
}

/// Climate parameters an intervention can set. Each names a real field of
/// the engine's climate/weather state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClimateParameter {
    /// Surface temperature, kelvin (absolute).
    Temperature,
    /// Precipitation rate (absolute).
    Precipitation,
    /// Wind speed, m/s (direction preserved).
    WindSpeed,
    /// Atmospheric moisture (absolute).
    Humidity,
}

/// Disturbance types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisturbanceType {
    Fire,
    Flood,
    VolcanicEruption,
    MeteorImpact,
    Disease,
    Drought,
}

/// Location in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f32,
    pub longitude: f32,
    pub altitude: Option<f32>,
}

impl Location {
    pub fn new(lat: f32, lon: f32) -> Self {
        Self {
            latitude: lat,
            longitude: lon,
            altitude: None,
        }
    }

    pub fn with_altitude(lat: f32, lon: f32, alt: f32) -> Self {
        Self {
            latitude: lat,
            longitude: lon,
            altitude: Some(alt),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.latitude >= -90.0
            && self.latitude <= 90.0
            && self.longitude >= -180.0
            && self.longitude <= 180.0
    }
}

/// Region definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub center: Location,
    pub radius_km: f32,
}

impl Region {
    pub fn new(center: Location, radius_km: f32) -> Self {
        Self { center, radius_km }
    }
}

/// Scenario modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioModification {
    pub parameter: String,
    pub old_value: ScenarioValue,
    pub new_value: ScenarioValue,
}

/// Scenario value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    String(String),
}

/// Intervention status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterventionStatus {
    Pending,
    Acknowledged,
    Rejected,
    Failed,
}

/// Intervention log - append-only record
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterventionLog {
    pub interventions: Vec<Intervention>,
}

impl InterventionLog {
    pub fn new() -> Self {
        Self {
            interventions: Vec::new(),
        }
    }

    pub fn push(&mut self, intervention: Intervention) {
        self.interventions.push(intervention);
    }

    pub fn get(&self, id: &InterventionId) -> Option<&Intervention> {
        self.interventions.iter().find(|i| &i.id == id)
    }

    pub fn by_author(&self, user_id: &str) -> Vec<&Intervention> {
        self.interventions
            .iter()
            .filter(|i| i.author.user_id == user_id)
            .collect()
    }

    pub fn by_source_tick(&self, tick: u64) -> Vec<&Intervention> {
        self.interventions
            .iter()
            .filter(|i| i.source.tick == tick)
            .collect()
    }
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            warnings: Vec::new(),
            errors,
        }
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Validate an intervention before execution
pub fn validate_intervention(action: &InterventionAction) -> ValidationResult {
    match action {
        InterventionAction::InjectResource {
            amount, location, ..
        } => validate_amount_and_location(*amount, location, "Resource amount"),
        InterventionAction::InjectBiomass { amount, region, .. } => {
            validate_amount_and_region(*amount, region, "Biomass amount")
        }
        InterventionAction::InjectEnergy {
            amount, location, ..
        } => validate_amount_and_location(*amount, location, "Energy amount"),
        InterventionAction::ModifyClimate { value, region, .. } => {
            let mut result = validate_finite(*value, "Climate value");
            if let Some(region) = region {
                result = merge_validation(result, validate_region(region));
            }
            result
        }
        InterventionAction::TriggerDisturbance {
            intensity,
            location,
            ..
        } => {
            let result = validate_intensity(*intensity);
            merge_validation(result, validate_location(location))
        }
        InterventionAction::SpawnHuman {
            location, profile, ..
        } => {
            let mut result = validate_location(location);
            if let Some(profile) = profile {
                result = merge_validation(result, validate_spawn_profile(profile));
            }
            result
        }
        InterventionAction::ConstructStructure { location, .. } => validate_location(location),
        InterventionAction::SculptTerrain {
            elevation_delta_m,
            region,
        } => {
            let result =
                if elevation_delta_m.is_finite() && elevation_delta_m.abs() <= MAX_SCULPT_DELTA_M {
                    ValidationResult::valid()
                } else {
                    ValidationResult::invalid(vec![format!(
                    "Terrain elevation change must be finite and within ±{MAX_SCULPT_DELTA_M} m"
                )])
                };
            merge_validation(result, validate_region(region))
        }
        InterventionAction::SmoothTerrain { strength, region } => {
            merge_validation(validate_intensity(*strength), validate_region(region))
        }
        InterventionAction::ModifyScenario { .. } => {
            // Scenario modifications are allowed but logged
            ValidationResult::valid().with_warning("Scenario modification logged".to_string())
        }
        _ => ValidationResult::valid(),
    }
}

fn validate_spawn_profile(profile: &HumanSpawnProfile) -> ValidationResult {
    let mut errors = Vec::new();
    if !profile.name.chars().any(char::is_alphanumeric) {
        errors.push("Human name must contain at least one letter or digit".to_string());
    }
    if !profile.age_years.is_finite()
        || profile.age_years < 0.0
        || profile.age_years > MAX_SPAWN_AGE_YEARS
    {
        errors.push(format!(
            "Human age must be between 0 and {MAX_SPAWN_AGE_YEARS} years"
        ));
    }
    if !profile.height_cm.is_finite() || !SPAWN_HEIGHT_RANGE_CM.contains(&profile.height_cm) {
        errors.push(format!(
            "Human height must be between {} and {} cm",
            SPAWN_HEIGHT_RANGE_CM.start(),
            SPAWN_HEIGHT_RANGE_CM.end()
        ));
    }
    for (label, value) in [
        ("Build", &profile.build),
        ("Hair color", &profile.hair_color),
        ("Eye color", &profile.eye_color),
        ("Skin tone", &profile.skin_tone),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("{label} must not be empty"));
        }
    }
    if chrono::DateTime::parse_from_rfc3339(&profile.birth_timestamp).is_err() {
        errors.push("Birth timestamp must be RFC 3339".to_string());
    }
    let birthplace = Location::new(
        profile.birth_latitude as f32,
        profile.birth_longitude as f32,
    );
    if !profile.birth_latitude.is_finite()
        || !profile.birth_longitude.is_finite()
        || !birthplace.is_valid()
    {
        errors.push("Birthplace coordinates are out of range".to_string());
    }
    if errors.is_empty() {
        ValidationResult::valid()
    } else {
        ValidationResult::invalid(errors)
    }
}

fn validate_amount_and_location(amount: f32, location: &Location, label: &str) -> ValidationResult {
    merge_validation(
        validate_finite_non_negative(amount, label),
        validate_location(location),
    )
}

fn validate_amount_and_region(amount: f32, region: &Region, label: &str) -> ValidationResult {
    merge_validation(
        validate_finite_non_negative(amount, label),
        validate_region(region),
    )
}

fn validate_finite_non_negative(value: f32, label: &str) -> ValidationResult {
    if !value.is_finite() || value < 0.0 {
        ValidationResult::invalid(vec![format!("{label} must be finite and non-negative")])
    } else {
        ValidationResult::valid()
    }
}

fn validate_finite(value: f32, label: &str) -> ValidationResult {
    if value.is_finite() {
        ValidationResult::valid()
    } else {
        ValidationResult::invalid(vec![format!("{label} must be finite")])
    }
}

fn validate_location(location: &Location) -> ValidationResult {
    if location.is_valid() {
        ValidationResult::valid()
    } else {
        ValidationResult::invalid(vec![
            "Location latitude/longitude is out of range".to_string()
        ])
    }
}

fn validate_region(region: &Region) -> ValidationResult {
    merge_validation(
        validate_location(&region.center),
        if region.radius_km.is_finite() && region.radius_km > 0.0 {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(vec![
                "Region radius must be finite and greater than zero".to_string()
            ])
        },
    )
}

fn validate_intensity(intensity: f32) -> ValidationResult {
    if !intensity.is_finite() || !(0.0..=1.0).contains(&intensity) {
        ValidationResult::invalid(vec![
            "Intensity must be finite and within [0, 1]".to_string()
        ])
    } else {
        ValidationResult::valid()
    }
}

fn merge_validation(mut first: ValidationResult, second: ValidationResult) -> ValidationResult {
    first.valid &= second.valid;
    first.warnings.extend(second.warnings);
    first.errors.extend(second.errors);
    first
}

/// Intervention permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionPermissions {
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_branch: bool,
    pub can_inject_resources: bool,
    pub can_modify_climate: bool,
    pub can_trigger_disturbances: bool,
    pub can_modify_scenario: bool,
    pub can_spawn_humans: bool,
    pub can_construct_structures: bool,
    pub can_sculpt_terrain: bool,
}

impl Default for InterventionPermissions {
    fn default() -> Self {
        Self {
            can_pause: true,
            can_resume: true,
            can_branch: true,
            can_inject_resources: true,
            can_modify_climate: true,
            can_trigger_disturbances: true,
            can_modify_scenario: true,
            can_spawn_humans: false, // Requires higher permissions
            can_construct_structures: true,
            can_sculpt_terrain: true,
        }
    }
}

impl InterventionPermissions {
    pub fn can_execute(&self, action: &InterventionAction) -> bool {
        match action {
            InterventionAction::Pause => self.can_pause,
            InterventionAction::Resume => self.can_resume,
            InterventionAction::Step { .. } => self.can_resume,
            InterventionAction::Scrub { .. } => self.can_resume,
            InterventionAction::Branch { .. } => self.can_branch,
            InterventionAction::Fork { .. } => self.can_branch,
            InterventionAction::InjectResource { .. } => self.can_inject_resources,
            InterventionAction::InjectBiomass { .. } => self.can_inject_resources,
            InterventionAction::InjectEnergy { .. } => self.can_inject_resources,
            InterventionAction::ModifyClimate { .. } => self.can_modify_climate,
            InterventionAction::TriggerDisturbance { .. } => self.can_trigger_disturbances,
            InterventionAction::ModifyScenario { .. } => self.can_modify_scenario,
            InterventionAction::SpawnHuman { .. } => self.can_spawn_humans,
            InterventionAction::RemoveHuman { .. } => self.can_spawn_humans,
            InterventionAction::ConstructStructure { .. } => self.can_construct_structures,
            InterventionAction::SculptTerrain { .. } => self.can_sculpt_terrain,
            InterventionAction::SmoothTerrain { .. } => self.can_sculpt_terrain,
        }
    }
}
