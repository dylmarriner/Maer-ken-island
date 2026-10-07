//! The island scenario (Phase 3 Task 4): everything that defines one run
//! of the island beyond the physics: the profile and seed, the canon, the
//! simulation cadences, the high-detail estate patch, the estate's energy
//! supply and whether founders and estate exist at all.
//!
//! Data only. It is read from JSON (`fixtures/island/default_scenario.json`)
//! and validated; nothing here runs the simulation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::profile::{IslandProfile, IslandProfileError};

/// The scenario format this code reads.
pub const SCENARIO_VERSION: u32 = 1;

#[derive(Debug)]
pub enum IslandScenarioError {
    Io(PathBuf, std::io::Error),
    Parse(PathBuf, serde_json::Error),
    UnsupportedVersion(u32),
    InvalidSeed,
    InvalidCadence(String),
    InvalidEstatePatch(String),
    InvalidEnergy(String),
    Profile(IslandProfileError),
}

impl std::fmt::Display for IslandScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "cannot read {}: {e}", p.display()),
            Self::Parse(p, e) => write!(f, "cannot parse {}: {e}", p.display()),
            Self::UnsupportedVersion(v) => {
                write!(f, "scenario version {v} is not {SCENARIO_VERSION}")
            }
            Self::InvalidSeed => write!(f, "seed must be 64 hexadecimal characters"),
            Self::InvalidCadence(why) => write!(f, "invalid cadence: {why}"),
            Self::InvalidEstatePatch(why) => write!(f, "invalid estate patch: {why}"),
            Self::InvalidEnergy(why) => write!(f, "invalid estate energy: {why}"),
            Self::Profile(e) => write!(f, "invalid profile: {e}"),
        }
    }
}

impl std::error::Error for IslandScenarioError {}

/// How often each system steps, in simulated seconds. Slower systems step
/// on multiples of the human step, so one scheduler tick can serve all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IslandCadenceProfile {
    pub human_seconds: u64,
    pub weather_ocean_seconds: u64,
    pub hydrology_ecology_resource_seconds: u64,
    pub geophysics_seconds: u64,
    pub human_store_seconds: u64,
}

impl Default for IslandCadenceProfile {
    fn default() -> Self {
        Self {
            human_seconds: 60,
            weather_ocean_seconds: 3_600,
            hydrology_ecology_resource_seconds: 21_600,
            geophysics_seconds: 86_400,
            human_store_seconds: 3_600,
        }
    }
}

impl IslandCadenceProfile {
    /// Every cadence must be positive and a multiple of the human step.
    pub fn validate(&self) -> Result<(), IslandScenarioError> {
        let bad = |why: String| Err(IslandScenarioError::InvalidCadence(why));
        if self.human_seconds == 0 {
            return bad("human_seconds must be positive".into());
        }
        for (name, value) in [
            ("weather_ocean_seconds", self.weather_ocean_seconds),
            (
                "hydrology_ecology_resource_seconds",
                self.hydrology_ecology_resource_seconds,
            ),
            ("geophysics_seconds", self.geophysics_seconds),
            ("human_store_seconds", self.human_store_seconds),
        ] {
            if value == 0 {
                return bad(format!("{name} must be positive"));
            }
            if value % self.human_seconds != 0 {
                return bad(format!(
                    "{name} ({value}) is not a multiple of human_seconds ({})",
                    self.human_seconds
                ));
            }
        }
        Ok(())
    }
}

/// The high-detail estate patch: a square of `extent_m` at `cell_size_m`,
/// with individually simulated trees within `individual_tree_radius_m` of
/// the estate centre (at most `tree_cap`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EstatePatchConfig {
    pub extent_m: f64,
    pub cell_size_m: f64,
    pub individual_tree_radius_m: f64,
    pub tree_cap: usize,
}

impl Default for EstatePatchConfig {
    fn default() -> Self {
        Self {
            extent_m: 4_000.0,
            cell_size_m: 5.0,
            individual_tree_radius_m: 1_000.0,
            tree_cap: 200_000,
        }
    }
}

impl EstatePatchConfig {
    pub fn validate(&self) -> Result<(), IslandScenarioError> {
        let bad = |why: &str| Err(IslandScenarioError::InvalidEstatePatch(why.into()));
        if !(self.extent_m.is_finite() && self.extent_m > 0.0) {
            return bad("extent_m must be positive");
        }
        if !(self.cell_size_m.is_finite() && self.cell_size_m > 0.0) {
            return bad("cell_size_m must be positive");
        }
        let n = self.extent_m / self.cell_size_m;
        if (n - n.round()).abs() > 1e-9 {
            return bad("cell_size_m must divide extent_m");
        }
        if !(self.individual_tree_radius_m.is_finite()
            && self.individual_tree_radius_m >= 0.0
            && self.individual_tree_radius_m <= 0.5 * self.extent_m)
        {
            return bad("individual_tree_radius_m must lie within the patch");
        }
        if self.tree_cap == 0 {
            return bad("tree_cap must be positive");
        }
        Ok(())
    }
}

/// The estate's initial energy supply (owner to confirm; Phase 4b adds the
/// river micro-hydro plant, making the generator the backup).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EstateEnergyConfig {
    pub generator_rated_kw: f64,
    pub diesel_litres: f64,
    pub petrol_litres: f64,
    pub solar_rated_kw: f64,
    pub battery_kwh: f64,
}

impl Default for EstateEnergyConfig {
    fn default() -> Self {
        Self {
            generator_rated_kw: 10.0,
            diesel_litres: 2_000.0,
            petrol_litres: 400.0,
            solar_rated_kw: 5.0,
            battery_kwh: 10.0,
        }
    }
}

impl EstateEnergyConfig {
    pub fn validate(&self) -> Result<(), IslandScenarioError> {
        for (name, v) in [
            ("generator_rated_kw", self.generator_rated_kw),
            ("diesel_litres", self.diesel_litres),
            ("petrol_litres", self.petrol_litres),
            ("solar_rated_kw", self.solar_rated_kw),
            ("battery_kwh", self.battery_kwh),
        ] {
            if !(v.is_finite() && v >= 0.0) {
                return Err(IslandScenarioError::InvalidEnergy(format!(
                    "{name} must be finite and not negative"
                )));
            }
        }
        Ok(())
    }
}

mod hex_seed {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(seed: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&seed.iter().map(|b| format!("{b:02x}")).collect::<String>())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let text = String::deserialize(d)?;
        let bad = || serde::de::Error::custom("seed must be 64 hexadecimal characters");
        if text.len() != 64 || !text.is_ascii() {
            return Err(bad());
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
        }
        Ok(out)
    }
}

/// One island run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandScenario {
    pub version: u32,
    #[serde(with = "hex_seed")]
    pub seed: [u8; 32],
    /// Path to the canon, relative to the repository root unless absolute.
    pub canon_path: PathBuf,
    pub profile: IslandProfile,
    pub cadences: IslandCadenceProfile,
    pub estate_patch: EstatePatchConfig,
    pub estate_energy: EstateEnergyConfig,
    pub founders_enabled: bool,
    pub estate_enabled: bool,
}

impl IslandScenario {
    /// The default scenario around `profile` and `seed`.
    pub fn new(profile: IslandProfile, seed: [u8; 32]) -> Self {
        Self {
            version: SCENARIO_VERSION,
            seed,
            canon_path: PathBuf::from("fixtures/island/canon.json"),
            profile,
            cadences: IslandCadenceProfile::default(),
            estate_patch: EstatePatchConfig::default(),
            estate_energy: EstateEnergyConfig::default(),
            founders_enabled: true,
            estate_enabled: true,
        }
    }

    pub fn validate(&self) -> Result<(), IslandScenarioError> {
        if self.version != SCENARIO_VERSION {
            return Err(IslandScenarioError::UnsupportedVersion(self.version));
        }
        self.cadences.validate()?;
        self.estate_patch.validate()?;
        self.estate_energy.validate()?;
        self.profile
            .validate()
            .map_err(IslandScenarioError::Profile)
    }

    pub fn load(path: &Path) -> Result<Self, IslandScenarioError> {
        let bytes = std::fs::read(path).map_err(|e| IslandScenarioError::Io(path.into(), e))?;
        let scenario: Self = serde_json::from_slice(&bytes)
            .map_err(|e| IslandScenarioError::Parse(path.into(), e))?;
        scenario.validate()?;
        Ok(scenario)
    }
}
