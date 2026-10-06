//! The regional island profile: domain extent, grid resolutions, land
//! target, ocean buffer, shape and geology requirements.
//!
//! Every value is configurable and validated before anything is allocated
//! (`docs/superpowers/plans/2026-10-01-island-domain-geophysics.md`,
//! Task 1). The island canon (`fixtures/island/canon.json`) is the planet;
//! the profile is the region of it that is simulated in detail.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Profile format version this build understands.
pub const PROFILE_VERSION: u32 = 1;

/// Planet radius of the island canon (m): Marr'Kena's 19,113 km, kept by
/// the Phase 0c canon (`fixtures/island/canon.json`).
pub const ISLAND_CANON_RADIUS_M: f64 = 1.9113e7;

/// Measurable requirements on the island's outline (Task 4 metrics).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeRequirements {
    /// Largest allowed `4πA/P²` (a circle is 1): keeps the coast indented.
    pub max_compactness: f64,
    /// Largest allowed land area / convex-hull area: rules out blobs and
    /// ellipses.
    pub max_convexity: f64,
    pub min_major_headlands: u32,
    pub min_major_bays: u32,
}

impl Default for ShapeRequirements {
    fn default() -> Self {
        Self {
            max_compactness: 0.30,
            max_convexity: 0.80,
            min_major_headlands: 3,
            min_major_bays: 3,
        }
    }
}

/// Measured outline of a land mask (Phase 1 Task 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeMetrics {
    pub area_m2: f64,
    /// Length of the marching-squares coastline contour (m).
    pub perimeter_m: f64,
    /// `4πA/P²` (a circle is 1).
    pub compactness: f64,
    /// Land area over convex-hull area.
    pub convexity: f64,
    pub major_headlands: u32,
    pub major_bays: u32,
}

impl ShapeRequirements {
    /// The requirements a measured shape fails, as readable reasons (empty
    /// when it passes).
    pub fn unmet(&self, m: &ShapeMetrics) -> Vec<String> {
        let mut reasons = Vec::new();
        if m.compactness > self.max_compactness {
            reasons.push(format!(
                "compactness {:.3} exceeds {:.3}",
                m.compactness, self.max_compactness
            ));
        }
        if m.convexity > self.max_convexity {
            reasons.push(format!(
                "convexity {:.3} exceeds {:.3}",
                m.convexity, self.max_convexity
            ));
        }
        if m.major_headlands < self.min_major_headlands {
            reasons.push(format!(
                "{} major headlands, need {}",
                m.major_headlands, self.min_major_headlands
            ));
        }
        if m.major_bays < self.min_major_bays {
            reasons.push(format!(
                "{} major bays, need {}",
                m.major_bays, self.min_major_bays
            ));
        }
        reasons
    }
}

/// The island's deep geology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeologyProfile {
    /// Whether a fragment of Archean continental crust underlies part of
    /// the island. Diamondiferous kimberlites occur only on cratons older
    /// than ~2.5 Ga (Clifford's rule; Clifford 1966, Earth Planet. Sci.
    /// Lett. 1:421); without one the island is a young arc with no primary
    /// diamonds.
    pub ancient_basement: bool,
    /// Age of that basement (Ma).
    pub basement_age_ma: f64,
    /// Fraction of the land area it underlies.
    pub basement_area_fraction: f64,
}

impl Default for GeologyProfile {
    fn default() -> Self {
        Self {
            ancient_basement: true,
            basement_age_ma: 2_700.0,
            basement_area_fraction: 0.08,
        }
    }
}

/// The simulated region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandProfile {
    pub version: u32,
    /// East–west extent (m).
    pub width_m: f64,
    /// North–south extent (m).
    pub height_m: f64,
    /// Atmosphere and ocean grid cell (m).
    pub coarse_cell_m: f64,
    /// Terrain, river, ecology and resource grid cell (m).
    pub medium_cell_m: f64,
    /// Latitude and longitude at the domain centre (degrees).
    pub reference_latitude_deg: f64,
    pub reference_longitude_deg: f64,
    /// Planet radius used to convert metres to latitude and longitude.
    pub planet_radius_m: f64,
    pub target_land_area_m2: f64,
    /// Accepted relative error of the land area (0.05 = ±5%).
    pub land_area_tolerance_fraction: f64,
    /// Least distance from any coast to every domain edge (m).
    pub minimum_ocean_buffer_m: f64,
    /// Width of the sea band beyond the coast that medium-grid processes
    /// still iterate (m).
    pub coastal_band_m: f64,
    pub shape: ShapeRequirements,
    pub geology: GeologyProfile,
}

/// Why a profile cannot be used.
#[derive(Debug)]
pub enum IslandProfileError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    UnsupportedVersion(u32),
    Invalid { field: &'static str, reason: String },
}

impl fmt::Display for IslandProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "could not read island profile: {err}"),
            Self::Parse(err) => write!(f, "could not parse island profile: {err}"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "island profile version {v} is not supported (this build reads version {PROFILE_VERSION})"
            ),
            Self::Invalid { field, reason } => write!(f, "invalid island profile {field}: {reason}"),
        }
    }
}

impl std::error::Error for IslandProfileError {}

fn invalid(field: &'static str, reason: impl Into<String>) -> IslandProfileError {
    IslandProfileError::Invalid {
        field,
        reason: reason.into(),
    }
}

/// Whether `extent` is a whole number of `cell`s.
fn divides(extent: f64, cell: f64) -> bool {
    let n = extent / cell;
    n >= 1.0 && (n - n.round()).abs() < 1e-9 && (n.round() * cell - extent).abs() < 1e-6
}

impl IslandProfile {
    /// The New-Zealand-sized island: 2,400 km × 1,920 km, 12 km coarse and
    /// 2 km medium cells (`160 × 200` and `960 × 1,200`), centred at
    /// 41° S 174° E, 268,000 km² of land ±5% inside a 300 km ocean buffer.
    pub fn default_nz_scale() -> Self {
        Self {
            version: PROFILE_VERSION,
            width_m: 2_400_000.0,
            height_m: 1_920_000.0,
            coarse_cell_m: 12_000.0,
            medium_cell_m: 2_000.0,
            reference_latitude_deg: -41.0,
            reference_longitude_deg: 174.0,
            planet_radius_m: ISLAND_CANON_RADIUS_M,
            target_land_area_m2: 268_000.0e6,
            land_area_tolerance_fraction: 0.05,
            minimum_ocean_buffer_m: 300_000.0,
            coastal_band_m: 20_000.0,
            shape: ShapeRequirements::default(),
            geology: GeologyProfile::default(),
        }
    }

    /// The documented performance fallback: the same island at 24 km and
    /// 4 km cells (`80 × 100` and `480 × 600`).
    pub fn fallback_nz_scale() -> Self {
        Self {
            coarse_cell_m: 24_000.0,
            medium_cell_m: 4_000.0,
            ..Self::default_nz_scale()
        }
    }

    /// The small island functional tests use (Task 1b): 480 km × 384 km,
    /// `32 × 40` coarse and `192 × 240` medium cells, 8,000 km² of land
    /// inside a 60 km buffer.
    pub fn test_small() -> Self {
        Self {
            width_m: 480_000.0,
            height_m: 384_000.0,
            target_land_area_m2: 8_000.0e6,
            minimum_ocean_buffer_m: 60_000.0,
            ..Self::default_nz_scale()
        }
    }

    /// Reads a profile, rejecting an unknown version before validating.
    pub fn load(path: &Path) -> Result<Self, IslandProfileError> {
        let bytes = std::fs::read(path).map_err(IslandProfileError::Io)?;
        let raw: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(IslandProfileError::Parse)?;
        if let Some(version) = raw.get("version").and_then(|v| v.as_u64()) {
            if version != u64::from(PROFILE_VERSION) {
                return Err(IslandProfileError::UnsupportedVersion(
                    version.min(u32::MAX as u64) as u32,
                ));
            }
        }
        let profile: Self = serde_json::from_value(raw).map_err(IslandProfileError::Parse)?;
        profile.validate()?;
        Ok(profile)
    }

    /// Checks every value; nothing is allocated from an invalid profile.
    pub fn validate(&self) -> Result<(), IslandProfileError> {
        if self.version != PROFILE_VERSION {
            return Err(IslandProfileError::UnsupportedVersion(self.version));
        }
        let positive = |field: &'static str, v: f64| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(invalid(field, format!("{v} must be finite and positive")))
            }
        };
        positive("width_m", self.width_m)?;
        positive("height_m", self.height_m)?;
        positive("coarse_cell_m", self.coarse_cell_m)?;
        positive("medium_cell_m", self.medium_cell_m)?;
        positive("planet_radius_m", self.planet_radius_m)?;
        positive("target_land_area_m2", self.target_land_area_m2)?;
        positive("minimum_ocean_buffer_m", self.minimum_ocean_buffer_m)?;
        if !(self.coastal_band_m.is_finite() && self.coastal_band_m >= 0.0) {
            return Err(invalid("coastal_band_m", "must be finite and not negative"));
        }
        for (field, cell) in [
            ("coarse_cell_m", self.coarse_cell_m),
            ("medium_cell_m", self.medium_cell_m),
        ] {
            if !divides(self.width_m, cell) || !divides(self.height_m, cell) {
                return Err(invalid(
                    field,
                    format!(
                        "{cell} m does not divide the {} m × {} m domain",
                        self.width_m, self.height_m
                    ),
                ));
            }
        }
        if !divides(self.coarse_cell_m, self.medium_cell_m) {
            return Err(invalid("medium_cell_m", "must divide the coarse cell"));
        }
        if !(self.reference_latitude_deg.is_finite() && self.reference_latitude_deg.abs() <= 80.0) {
            return Err(invalid("reference_latitude_deg", "must lie within ±80°"));
        }
        if !(self.reference_longitude_deg.is_finite()
            && self.reference_longitude_deg.abs() <= 180.0)
        {
            return Err(invalid("reference_longitude_deg", "must lie within ±180°"));
        }
        let half_height_deg = (0.5 * self.height_m / self.planet_radius_m).to_degrees();
        if self.reference_latitude_deg.abs() + half_height_deg >= 89.0 {
            return Err(invalid("height_m", "the domain would reach a pole"));
        }
        if self.width_m / (self.planet_radius_m * self.reference_latitude_deg.to_radians().cos())
            >= std::f64::consts::PI
        {
            return Err(invalid("width_m", "the domain spans half the planet"));
        }
        if 2.0 * self.minimum_ocean_buffer_m >= self.width_m.min(self.height_m) {
            return Err(invalid("minimum_ocean_buffer_m", "leaves no room for land"));
        }
        let interior = (self.width_m - 2.0 * self.minimum_ocean_buffer_m)
            * (self.height_m - 2.0 * self.minimum_ocean_buffer_m);
        if self.target_land_area_m2 >= interior {
            return Err(invalid(
                "target_land_area_m2",
                format!(
                    "{} m² does not fit inside the buffer ({interior} m²)",
                    self.target_land_area_m2
                ),
            ));
        }
        if !(self.land_area_tolerance_fraction.is_finite()
            && self.land_area_tolerance_fraction > 0.0
            && self.land_area_tolerance_fraction < 0.5)
        {
            return Err(invalid(
                "land_area_tolerance_fraction",
                "must lie in (0, 0.5)",
            ));
        }
        let s = &self.shape;
        for (field, v) in [
            ("shape.max_compactness", s.max_compactness),
            ("shape.max_convexity", s.max_convexity),
        ] {
            if !(v.is_finite() && v > 0.0 && v <= 1.0) {
                return Err(invalid(field, format!("{v} must lie in (0, 1]")));
            }
        }
        if s.min_major_headlands > 64 || s.min_major_bays > 64 {
            return Err(invalid(
                "shape",
                "at most 64 major headlands or bays can be required",
            ));
        }
        let g = &self.geology;
        if !(g.basement_age_ma.is_finite()
            && g.basement_age_ma > 0.0
            && g.basement_age_ma < 4_600.0)
        {
            return Err(invalid(
                "geology.basement_age_ma",
                "must lie in (0, 4600) Ma",
            ));
        }
        if !(g.basement_area_fraction.is_finite()
            && (0.0..=0.5).contains(&g.basement_area_fraction))
        {
            return Err(invalid(
                "geology.basement_area_fraction",
                "must lie in [0, 0.5]",
            ));
        }
        Ok(())
    }

    /// Accepted land area range (m²).
    pub fn land_area_bounds_m2(&self) -> (f64, f64) {
        let t = self.land_area_tolerance_fraction;
        (
            self.target_land_area_m2 * (1.0 - t),
            self.target_land_area_m2 * (1.0 + t),
        )
    }
}
