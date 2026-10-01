//! TECTONICS MODULE — PHASE 2
//!
//! Purpose
//! - Plate dynamics and internal heat flow for Marr'Kena MK-I
//! - Plate motion, subduction, ridge spreading
//! - Lithospheric cooling and mantle convection
//!
//! Invariants
//! - All arithmetic uses f64 for continuous quantities
//! - 11 major + 19 minor plates (per PLANET_CONSTANTS.md)
//! - Plate velocity ~2-10 cm/yr (deterministic from seed)
//! - Lithospheric age/thickness determines density and heat flow
//! - Same tick + RNG seed → same plate configuration (deterministic)
//! - Ledger-tracked: InternalHeat → CrustHeat
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.9
//! - PLANET_CONSTANTS.md § 9 (plate counts)
//! - NATURE_CONSTANTS.md (thermal properties)

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

/// Lithospheric plate type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlateType {
    /// Continental crust
    Continental,
    /// Oceanic crust
    Oceanic,
}

/// Plate boundary type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryType {
    /// Constructive: ridge spreading (new crust created)
    Ridge,
    /// Destructive: subduction zone (crust consumed)
    Subduction,
    /// Conservative: transform/strike-slip (crust slides)
    Transform,
    /// Convergent continent-continent boundary: neither side subducts, so
    /// the crust thickens into a mountain belt.
    Collision,
}

/// Plate velocity vector
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlateVelocity {
    /// Velocity (cm/yr)
    pub magnitude: f64,
    /// Direction (radians, 0 = north)
    pub direction: f64,
}

impl PlateVelocity {
    /// Get velocity components
    pub fn components(&self) -> (f64, f64) {
        let u = self.magnitude * self.direction.sin();
        let v = self.magnitude * self.direction.cos();
        (u, v)
    }
}

/// Plate configuration at a grid cell
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlateCell {
    /// Plate ID (0-30 for 31 total plates)
    pub plate_id: u8,
    /// Crust type
    pub crust_type: PlateType,
    /// Lithospheric age (Ma, million years)
    pub age_ma: f64,
    /// Lithospheric thickness (km)
    pub thickness_km: f64,
    /// Crustal thickness (km): about 7 km for oceanic crust, 30–45 km for
    /// continents, and more where continents collide. Continental elevation
    /// follows from it by Airy isostasy.
    #[serde(default = "default_crust_thickness_km")]
    pub crust_thickness_km: f64,
    /// Boundary type at this cell (if on boundary)
    pub boundary: Option<BoundaryType>,
}

/// Tectonics state for a simulation tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicsState {
    pub plates: Grid2<PlateCell>,
    pub heat_flow: Grid2<f64>,
    /// Persistent surface relief (m) added on top of the isostatic
    /// elevation: operator terrain sculpting (uplift, excavation). `None`
    /// until the first edit. The plate field is regenerated every tectonic
    /// step, so this layer is what keeps authored landforms in the world.
    #[serde(default)]
    pub surface_relief_m: Option<Grid2<f64>>,
    /// Height (m) of the sea surface above the isostatic zero, set so the
    /// ocean covers canon `ocean_fraction` of the planet's area (sea level
    /// is fixed by the ocean's volume; canon records the resulting
    /// coverage). Elevations are reported relative to it.
    #[serde(default)]
    pub sea_level_m: f64,
}

impl TectonicsState {
    /// Create new tectonics state
    pub fn new(width: usize, height: usize) -> Self {
        let spec = mk_core::grid::GridSpec::new(width, height);
        TectonicsState {
            plates: Grid2::new(&spec, PlateCell::new(0, PlateType::Continental, 100.0)),
            heat_flow: Grid2::new(&spec, 50.0),
            surface_relief_m: None,
            sea_level_m: 0.0,
        }
    }

    /// Add `delta_m` of relief at `(row, col)`, creating the relief layer on
    /// first use. Out-of-range cells are ignored and reported as `false`.
    pub fn add_surface_relief(&mut self, row: usize, col: usize, delta_m: f64) -> bool {
        if row >= self.plates.nlat() || col >= self.plates.nlon() {
            return false;
        }
        let spec = mk_core::grid::GridSpec::new(self.plates.nlat(), self.plates.nlon());
        let relief = self
            .surface_relief_m
            .get_or_insert_with(|| Grid2::new(&spec, 0.0));
        *relief.get_mut(row, col) += delta_m;
        true
    }

    /// Compatibility accessor returning (plates, heat_flow) tuple when older modules expect `plate_config` field
    pub fn plate_config_tuple(&self) -> (Grid2<PlateCell>, Grid2<f64>) {
        (self.plates.clone(), self.heat_flow.clone())
    }

    /// Backwards-compatible field accessor so older code that expects `plate_config` can use `.plate_config()`
    pub fn plate_config(&self) -> (Grid2<PlateCell>, Grid2<f64>) {
        (self.plates.clone(), self.heat_flow.clone())
    }

    /// Get fraction of grid cells that are constructive boundaries (ridges)
    pub fn ridge_activity(&self) -> f64 {
        let ridge_count = self
            .plates
            .data()
            .iter()
            .filter(|c| c.boundary == Some(BoundaryType::Ridge))
            .count();
        ridge_count as f64 / self.plates.data().len() as f64
    }

    /// Get elevation grid derived from plate thickness and isostasy
    /// Elevation = reference_level + isostatic_adjustment
    pub fn get_elevation_grid(&self) -> Grid2<f64> {
        let spec = mk_core::grid::GridSpec::new(self.plates.nlat(), self.plates.nlon());
        let mut elevation = Grid2::new(&spec, 0.0);

        for y in 0..self.plates.nlat() {
            for x in 0..self.plates.nlon() {
                *elevation.get_mut(y, x) =
                    self.plates.get(y, x).isostatic_elevation_m() - self.sea_level_m;
            }
        }

        if let Some(relief) = &self.surface_relief_m {
            if relief.nlat() == elevation.nlat() && relief.nlon() == elevation.nlon() {
                for y in 0..elevation.nlat() {
                    for x in 0..elevation.nlon() {
                        *elevation.get_mut(y, x) += *relief.get(y, x);
                    }
                }
            }
        }

        elevation
    }
}

/// Mantle density beneath the crust (kg/m³), for Airy isostasy.
const MANTLE_DENSITY_KG_M3: f64 = 3300.0;
/// Continental crustal thickness whose isostatic surface sits at sea level
/// (km). From Airy isostasy on Earth's continents: a mean elevation of
/// ~840 m over a mean crust of ~38 km.
const SEA_LEVEL_CONTINENTAL_CRUST_KM: f64 = 33.4;
/// Oceanic crust thickness (km).
const OCEANIC_CRUST_KM: f64 = 7.0;
/// Ridge-crest depth (m) and half-space subsidence coefficient (m/√Ma) of
/// the Parsons & Sclater (1977) age–depth relation, valid to 70 Ma.
const RIDGE_CREST_DEPTH_M: f64 = 2500.0;
const SUBSIDENCE_M_PER_SQRT_MA: f64 = 350.0;
/// Plate-model asymptote for older seafloor: `d = 6400 − 3200·e^(−t/62.8)`.
const OLD_SEAFLOOR_ASYMPTOTE_M: f64 = 6400.0;
const OLD_SEAFLOOR_AMPLITUDE_M: f64 = 3200.0;
const OLD_SEAFLOOR_TIMESCALE_MA: f64 = 62.8;
const HALF_SPACE_LIMIT_MA: f64 = 70.0;

fn default_crust_thickness_km() -> f64 {
    SEA_LEVEL_CONTINENTAL_CRUST_KM
}

/// Seafloor depth (m, positive down) for lithosphere of `age_ma`: the
/// half-space cooling curve to 70 Ma, the plate-model asymptote after.
pub fn seafloor_depth_m(age_ma: f64) -> f64 {
    let age = age_ma.max(0.0);
    if age < HALF_SPACE_LIMIT_MA {
        RIDGE_CREST_DEPTH_M + SUBSIDENCE_M_PER_SQRT_MA * age.sqrt()
    } else {
        OLD_SEAFLOOR_ASYMPTOTE_M
            - OLD_SEAFLOOR_AMPLITUDE_M * (-age / OLD_SEAFLOOR_TIMESCALE_MA).exp()
    }
}

impl PlateCell {
    /// Surface elevation (m, relative to sea level) of this column before
    /// operator relief. Oceanic lithosphere subsides as it cools with age
    /// ([`seafloor_depth_m`]); oceanic crust thickened past normal (plateaus)
    /// rises by Airy isostasy under water. Continental elevation is Airy
    /// isostasy on the crust: each km of crust above
    /// [`SEA_LEVEL_CONTINENTAL_CRUST_KM`] raises the surface by
    /// `1 − ρc/ρm` km.
    pub fn isostatic_elevation_m(&self) -> f64 {
        let buoyancy = 1.0 - self.density() / MANTLE_DENSITY_KG_M3;
        match self.crust_type {
            PlateType::Continental => {
                (self.crust_thickness_km - SEA_LEVEL_CONTINENTAL_CRUST_KM) * 1000.0 * buoyancy
            }
            PlateType::Oceanic => {
                // Under water the column's buoyancy works against the
                // displaced sea water rather than air.
                let water_loaded = buoyancy / (1.0 - 1025.0 / MANTLE_DENSITY_KG_M3);
                -seafloor_depth_m(self.age_ma)
                    + (self.crust_thickness_km - OCEANIC_CRUST_KM) * 1000.0 * water_loaded
            }
        }
    }

    /// Create new plate cell
    pub fn new(plate_id: u8, crust_type: PlateType, age_ma: f64) -> Self {
        // Lithospheric thickness scales with sqrt(age)
        // Young oceanic: ~6 km, 80 Ma old: ~100 km
        let thickness_km = if crust_type == PlateType::Oceanic {
            2.32 * age_ma.sqrt().min(140.0)
        } else {
            // Continental crust thickens with age via accretion/orogeny,
            // same "age drives thickness" principle as oceanic above.
            // Deterministic (age_ma itself comes from a lat/lon hash in
            // `step_tectonics`, not wall-clock or rand::random()).
            // Centered on age_ma=150 so it can go thinner (rifted margin)
            // or thicker (old craton) than the flat 120 km baseline this
            // replaces; feeds `get_elevation_grid`'s isostasy formula
            // (elevation = 200 + (thickness_km - 120) * 2) to produce real
            // beach/lowland/upland relief instead of one uniform elevation.
            (120.0 + (age_ma - 150.0)).clamp(20.0, 220.0)
        };

        let crust_thickness_km = if crust_type == PlateType::Oceanic {
            OCEANIC_CRUST_KM
        } else {
            // Young, rifted continental margins are thin; old cratons thick
            // (roughly 32–42 km over the 50–250 Ma continental ages here).
            32.0 + ((age_ma - 50.0) / 200.0).clamp(0.0, 1.0) * 10.0
        };

        PlateCell {
            plate_id,
            crust_type,
            age_ma,
            thickness_km,
            crust_thickness_km,
            boundary: None,
        }
    }

    /// Compute lithospheric density
    /// Get plate density - used for isostasy calculations
    pub fn density(&self) -> f64 {
        match self.crust_type {
            PlateType::Continental => 2700.0, // kg/m³
            PlateType::Oceanic => 3000.0,     // kg/m³
        }
    }
}

/// Thermal structure of lithosphere
///
/// Heat flow depends on age and boundary type
fn heat_flow(plate_cell: &PlateCell, boundary_type: Option<BoundaryType>) -> f64 {
    let base_heat = match boundary_type {
        None => {
            // Conductive cooling: Q ~ 1/sqrt(age)
            // Young ridges: ~350 mW/m², old cratons: ~50 mW/m²
            if plate_cell.age_ma < 1.0 {
                350.0
            } else {
                350.0 / plate_cell.age_ma.sqrt()
            }
        }
        Some(BoundaryType::Ridge) => {
            // Ridge spreading: high heat flow ~500+ mW/m²
            500.0 + 100.0 * (1.0 / (plate_cell.age_ma + 0.1))
        }
        Some(BoundaryType::Subduction) => {
            // Subduction zone: low heat flow (cold descending slab) ~50-100 mW/m²
            75.0
        }
        Some(BoundaryType::Transform) => {
            // Transform: moderate heat flow from friction
            200.0
        }
        Some(BoundaryType::Collision) => {
            // Thickened continental crust: radiogenic heating raises heat
            // flow modestly above the stable-continent background.
            90.0
        }
    };

    base_heat.clamp(20.0, 700.0)
}

/// A plate's velocity: a persistent property of the plate (speed 1-15
/// cm/yr, realistic for terrestrial planets, and a heading), derived
/// deterministically from its id so it is the same every step.
pub fn plate_velocity(plate_id: u8) -> PlateVelocity {
    // SplitMix64 scramble of the plate id: two independent uniform
    // fractions, one for speed and one for heading.
    let mut x = (plate_id as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    let speed_fraction = (x & 0xFFFF_FFFF) as f64 / u32::MAX as f64;
    let heading_fraction = (x >> 32) as f64 / u32::MAX as f64;

    PlateVelocity {
        magnitude: 1.0 + speed_fraction * 14.0,
        direction: heading_fraction * std::f64::consts::TAU,
    }
}

/// Crustal thickening (km) at a continent-continent collision.
const COLLISION_THICKENING_KM: f64 = 25.0;

/// Classify the boundary a cell sits on, if any, from the relative motion
/// of its plate and the neighbouring plate(s) across it: diverging →
/// ridge; converging → subduction where oceanic crust can go under (the
/// oceanic side subducts, the overriding side carries the volcanic arc),
/// collision where both sides are continental; mostly sliding past →
/// transform. The neighbour with the strongest relative motion decides.
fn classify_boundary(plates: &Grid2<PlateCell>, row: usize, col: usize) -> Option<BoundaryType> {
    let (nlat, nlon) = (plates.nlat(), plates.nlon());
    let here = plates.get(row, col);
    let (ue, un) = plate_velocity(here.plate_id).components();
    // (neighbour, unit normal pointing toward it in east/north); row 0 is
    // the south pole, so row + 1 is north.
    let mut neighbours: Vec<((usize, usize), (f64, f64))> = vec![
        ((row, (col + 1) % nlon), (1.0, 0.0)),
        ((row, (col + nlon - 1) % nlon), (-1.0, 0.0)),
    ];
    if row + 1 < nlat {
        neighbours.push(((row + 1, col), (0.0, 1.0)));
    }
    if row > 0 {
        neighbours.push(((row - 1, col), (0.0, -1.0)));
    }

    let mut strongest: Option<(f64, BoundaryType)> = None;
    for ((r, c), (ne, nn)) in neighbours {
        let other = plates.get(r, c);
        if other.plate_id == here.plate_id {
            continue;
        }
        let (ve, vn) = plate_velocity(other.plate_id).components();
        let (re, rn) = (ue - ve, un - vn);
        // Positive: this plate moves toward the neighbour (convergent).
        let closing = re * ne + rn * nn;
        let sliding = (re * nn - rn * ne).abs();
        let kind = if sliding > closing.abs() {
            BoundaryType::Transform
        } else if closing < 0.0 {
            BoundaryType::Ridge
        } else if here.crust_type == PlateType::Continental
            && other.crust_type == PlateType::Continental
        {
            BoundaryType::Collision
        } else {
            BoundaryType::Subduction
        };
        let strength = closing.hypot(sliding);
        if strongest.is_none_or(|(best, _)| strength > best) {
            strongest = Some((strength, kind));
        }
    }
    strongest.map(|(_, kind)| kind)
}

/// Step tectonics for one tick
///
/// Compute plate configuration, heat flow, boundary types
pub fn step_tectonics(
    canon: &CanonLocked,
    _tick: Tick,
    grid_spec: &mk_core::grid::GridSpec,
) -> TectonicsState {
    let grid_rows = grid_spec.nlat;
    let grid_cols = grid_spec.nlon;

    let mut plates = Grid2::new(grid_spec, PlateCell::new(0, PlateType::Oceanic, 50.0));
    let mut heat_flow_field = Grid2::new(grid_spec, 0.0);

    // 11 major + 19 minor = 30 plates, mapped to grid
    let total_plates: usize = 30;
    // Fixed, deterministic seed positions (Fibonacci sphere lattice — pure
    // closed-form math, no rand::random() or wall-clock) so plate_id is
    // assigned by nearest seed (Voronoi regions) instead of an independent
    // per-cell hash. The previous per-cell hash produced salt-and-pepper
    // noise (adjacent cells could land on wildly different plates/ages/
    // thicknesses), which meant adjacent-vertex elevation jumps large
    // enough to flip `slope_normal`'s output between lit and unlit,
    // rendering as chaotic black/white stripe bands rather than terrain.
    // Voronoi seeding gives spatially contiguous plates, matching how real
    // tectonic plates look and removing that per-cell discontinuity.
    let seeds: Vec<(f64, f64)> = (0..total_plates)
        .map(|i| {
            let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
            let y = 1.0 - 2.0 * (i as f64) / ((total_plates - 1).max(1) as f64);
            let theta = golden_angle * i as f64;
            let radius = (1.0 - y * y).max(0.0).sqrt();
            let (sx, sz) = (theta.cos() * radius, theta.sin() * radius);
            (y.clamp(-1.0, 1.0).asin(), sz.atan2(sx))
        })
        .collect();

    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let lat = grid_spec.lat_rad(row);
            let lon = grid_spec.lon_rad(col);
            let (clat, clon) = (lat.cos(), lon.cos());
            let (slat, slon) = (lat.sin(), lon.sin());
            let cell_vec = (clat * clon, slat, clat * slon);

            // Nearest seed by dot product (max dot = min angular distance).
            let mut plate_id: u8 = 0;
            let mut best_dot = f64::MIN;
            for (i, &(slat_i, slon_i)) in seeds.iter().enumerate() {
                let (sc, ss) = (slat_i.cos(), slat_i.sin());
                let (scl, ssl) = (slon_i.cos(), slon_i.sin());
                let seed_vec = (sc * scl, ss, sc * ssl);
                let dot =
                    cell_vec.0 * seed_vec.0 + cell_vec.1 * seed_vec.1 + cell_vec.2 * seed_vec.2;
                if dot > best_dot {
                    best_dot = dot;
                    plate_id = i as u8;
                }
            }

            // Per-cell hash retained only for sub-plate texture (small
            // thickness jitter below), not for plate/type/age assignment —
            // those must be uniform per plate_id so a single physical plate
            // doesn't flip crust type or age cell-to-cell.
            let lat_hash = ((lat * 1000.0) as u64).wrapping_mul(73856093);
            let lon_hash = ((lon * 1000.0) as u64).wrapping_mul(19349663);

            // Plate type: deterministic per plate_id, ~1/3 continental.
            let is_continental = plate_id.is_multiple_of(3);
            let crust_type = if is_continental {
                PlateType::Continental
            } else {
                PlateType::Oceanic
            };

            // Lithospheric age: deterministic per plate_id (uniform across
            // one plate, as real crust age is), not per-cell.
            let age_ma = if crust_type == PlateType::Continental {
                50.0 + ((plate_id as u64 * 37) % 2000) as f64 / 10.0 // 50-250 Ma
            } else {
                0.5 + ((plate_id as u64 * 53) % 1000) as f64 / 5.0 // 0.5-200 Ma
            };

            let mut plate_cell = PlateCell::new(plate_id, crust_type, age_ma);
            // Small deterministic sub-plate thickness jitter (+/-2.5 km) so
            // a plate's interior isn't perfectly flat, without being large
            // enough to reproduce the old per-cell chaos.
            let jitter_km = ((lat_hash ^ lon_hash) % 100) as f64 * 0.05 - 2.5;
            plate_cell.thickness_km = (plate_cell.thickness_km + jitter_km).max(1.0);
            if crust_type == PlateType::Continental {
                plate_cell.crust_thickness_km += jitter_km;
            }

            plates.set(row, col, plate_cell);
        }
    }

    // Second pass: boundaries from neighbouring plates' relative motion,
    // then the heat flow each boundary type produces.
    let unclassified = plates.clone();
    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let mut plate_cell = *unclassified.get(row, col);
            plate_cell.boundary = classify_boundary(&unclassified, row, col);
            if plate_cell.boundary == Some(BoundaryType::Collision) {
                plate_cell.thickness_km += COLLISION_THICKENING_KM;
                plate_cell.crust_thickness_km += COLLISION_THICKENING_KM;
            }
            let heat = heat_flow(&plate_cell, plate_cell.boundary);
            plates.set(row, col, plate_cell);
            heat_flow_field.set(row, col, heat);
        }
    }

    let sea_level_m = sea_level_for_ocean_fraction(&plates, canon.ocean_fraction, grid_spec);
    TectonicsState {
        plates,
        heat_flow: heat_flow_field,
        surface_relief_m: None,
        sea_level_m,
    }
}

/// The isostatic height at which the sea must stand for water to cover
/// `ocean_fraction` of the sphere's area: the area-weighted quantile of the
/// column heights.
fn sea_level_for_ocean_fraction(
    plates: &Grid2<PlateCell>,
    ocean_fraction: f64,
    grid_spec: &mk_core::grid::GridSpec,
) -> f64 {
    let mut columns: Vec<(f64, f64)> = Vec::with_capacity(grid_spec.nlat * grid_spec.nlon);
    for row in 0..grid_spec.nlat {
        let area = grid_spec
            .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
            .unwrap_or(0.0);
        for col in 0..grid_spec.nlon {
            columns.push((plates.get(row, col).isostatic_elevation_m(), area));
        }
    }
    if columns.is_empty() || !ocean_fraction.is_finite() {
        return 0.0;
    }
    columns.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = columns.iter().map(|(_, a)| a).sum();
    let target = ocean_fraction.clamp(0.0, 1.0) * total;
    let mut covered = 0.0;
    for (i, (height, area)) in columns.iter().enumerate() {
        covered += area;
        if covered >= target {
            // The sea surface lies between this column and the next one up,
            // so this column is submerged and the next stands dry.
            let next = columns.get(i + 1).map_or(*height + 1.0, |c| c.0);
            return 0.5 * (height + next);
        }
    }
    columns.last().map_or(0.0, |c| c.0 + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plate_cell_creation() {
        let cell = PlateCell::new(0, PlateType::Oceanic, 50.0);
        assert_eq!(cell.plate_id, 0);
        assert!(cell.thickness_km > 0.0);
        assert!(cell.thickness_km < 150.0);
    }

    #[test]
    fn oceanic_lithosphere_thickens_with_age() {
        let young = PlateCell::new(0, PlateType::Oceanic, 1.0);
        let old = PlateCell::new(0, PlateType::Oceanic, 100.0);

        assert!(old.thickness_km > young.thickness_km);
    }

    #[test]
    fn density_realistic() {
        let cont = PlateCell::new(0, PlateType::Continental, 50.0);
        let oce = PlateCell::new(0, PlateType::Oceanic, 50.0);

        let cont_rho = cont.density();
        let oce_rho = oce.density();

        assert!(cont_rho > 2600.0 && cont_rho < 2800.0);
        assert!(oce_rho > 2700.0 && oce_rho < 3300.0);
    }

    #[test]
    fn heat_flow_ridge_high() {
        let ridge = PlateCell::new(0, PlateType::Oceanic, 1.0);
        let q_ridge = heat_flow(&ridge, Some(BoundaryType::Ridge));

        let interior = PlateCell::new(0, PlateType::Oceanic, 50.0);
        let q_interior = heat_flow(&interior, None);

        assert!(q_ridge > q_interior);
    }

    #[test]
    fn plate_velocity_deterministic() {
        let vel1 = plate_velocity(5);
        let vel2 = plate_velocity(5);

        assert_eq!(vel1.magnitude, vel2.magnitude);
        assert_eq!(vel1.direction, vel2.direction);
    }

    #[test]
    fn boundaries_lie_only_where_plates_meet() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(32, 64);
        let state = step_tectonics(&canon, 0, &grid_spec);
        let (nlat, nlon) = (grid_spec.nlat, grid_spec.nlon);
        let mut boundary_cells = 0;
        for row in 0..nlat {
            for col in 0..nlon {
                let cell = state.plates.get(row, col);
                let touches_other_plate = [
                    Some((row, (col + 1) % nlon)),
                    Some((row, (col + nlon - 1) % nlon)),
                    (row + 1 < nlat).then_some((row + 1, col)),
                    row.checked_sub(1).map(|r| (r, col)),
                ]
                .into_iter()
                .flatten()
                .any(|(r, c)| state.plates.get(r, c).plate_id != cell.plate_id);
                assert_eq!(
                    cell.boundary.is_some(),
                    touches_other_plate,
                    "({row}, {col})"
                );
                boundary_cells += usize::from(cell.boundary.is_some());
            }
        }
        assert!(boundary_cells > 0);
    }

    #[test]
    fn plate_motion_is_a_persistent_varied_property() {
        let headings: Vec<f64> = (0..30).map(|id| plate_velocity(id).direction).collect();
        assert!(
            headings.iter().any(|h| *h > 1.0),
            "headings must not all be zero"
        );
        assert_eq!(plate_velocity(7).direction, plate_velocity(7).direction);
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(16, 32);
        let early = step_tectonics(&canon, 0, &grid_spec);
        let late = step_tectonics(&canon, 1_000_000, &grid_spec);
        assert_eq!(early.plates.data(), late.plates.data());
    }

    #[test]
    fn seafloor_deepens_with_age_and_continents_stand_above_it() {
        assert!((seafloor_depth_m(0.0) - 2500.0).abs() < 1e-9);
        assert!(seafloor_depth_m(50.0) > seafloor_depth_m(10.0));
        // The two branches meet near 70 Ma.
        assert!((seafloor_depth_m(69.999) - seafloor_depth_m(70.0)).abs() < 200.0);
        assert!(seafloor_depth_m(200.0) < 6400.0);

        let young = PlateCell::new(0, PlateType::Oceanic, 1.0);
        let old = PlateCell::new(0, PlateType::Oceanic, 150.0);
        assert!(old.isostatic_elevation_m() < young.isostatic_elevation_m());

        let mut craton = PlateCell::new(0, PlateType::Continental, 250.0);
        assert!(craton.isostatic_elevation_m() > 0.0);
        let normal = craton.isostatic_elevation_m();
        craton.crust_thickness_km += COLLISION_THICKENING_KM;
        let raised = craton.isostatic_elevation_m() - normal;
        // Airy: 25 km × (1 − 2700/3300) ≈ 4.5 km of uplift.
        assert!((raised - 4545.0).abs() < 10.0, "{raised}");
    }

    #[test]
    fn generated_world_has_earthlike_relief() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(32, 64);
        let elevation = step_tectonics(&canon, 0, &grid_spec).get_elevation_grid();
        let heights = elevation.data();
        let max = heights.iter().cloned().fold(f64::MIN, f64::max);
        let min = heights.iter().cloned().fold(f64::MAX, f64::min);
        assert!(max > 2000.0, "highest land {max} m");
        assert!(min < -4000.0, "deepest sea {min} m");

        // Sea level floods exactly canon's ocean fraction of the area.
        let mut ocean_area = 0.0;
        let mut area = 0.0;
        for row in 0..grid_spec.nlat {
            let a = grid_spec
                .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
                .unwrap();
            for col in 0..grid_spec.nlon {
                area += a;
                if *elevation.get(row, col) < 0.0 {
                    ocean_area += a;
                }
            }
        }
        let fraction = ocean_area / area;
        assert!((fraction - canon.ocean_fraction).abs() < 0.02, "{fraction}");
    }
}
