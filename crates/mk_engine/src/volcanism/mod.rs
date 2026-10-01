//! VOLCANISM MODULE — PHASE 2
//!
//! Purpose
//! - Volcanic outgassing, heat flux, atmospheric contribution
//! - Mantle degassing: CO₂, H₂O, N₂, etc.
//! - Volcanic eruption frequency and magnitude
//!
//! Invariants
//! - All arithmetic uses f64 for continuous quantities
//! - Outgassing rates tied to plate boundaries (ridge, subduction, hotspots)
//! - Mantle heat flux distributed to crust
//! - Same tick + plate configuration → same volcanism (deterministic)
//! - Ledger-tracked: MantleHeat → VulcanicHeat, OutgassedCarbon, etc.
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.10
//! - PLANET_CONSTANTS.md § 9 (plate dynamics)
//! - NATURE_CONSTANTS.md (mantle properties, outgassing)

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

/// Volcanic gas composition (moles per square meter per year)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolcanicGases {
    /// CO₂ outgassing (mol/m²/yr)
    pub co2_mol_m2_yr: f64,
    /// H₂O outgassing (mol/m²/yr)
    pub h2o_mol_m2_yr: f64,
    /// N₂ outgassing (mol/m²/yr)
    pub n2_mol_m2_yr: f64,
    /// SO₂ outgassing (mol/m²/yr)
    pub so2_mol_m2_yr: f64,
}

impl VolcanicGases {
    /// Total molar outgassing rate
    pub fn total_mol_m2_yr(&self) -> f64 {
        self.co2_mol_m2_yr + self.h2o_mol_m2_yr + self.n2_mol_m2_yr + self.so2_mol_m2_yr
    }
}

/// Volcanic state at a grid cell
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolcanicCell {
    /// Eruption probability (0-1) in next timestep
    pub eruption_prob: f64,
    /// Outgassing rate (mol/m²/yr)
    pub outgassing: VolcanicGases,
    /// Heat flux from volcanism (mW/m²)
    pub heat_flux_mw_m2: f64,
    /// Distance to nearest plate boundary (km). Where no neighbouring cell
    /// is a boundary this is the grid spacing, a lower bound.
    pub distance_to_boundary_km: f64,
    /// Fraction of the cell's area that is volcanic terrain: a volcanic arc
    /// or rift belt along the boundary through the cell, or a hotspot's
    /// shield. A cell is volcanic ground only where this dominates
    /// ([`Self::dominates_cell`]); on a coarse grid a belt a few hundred km
    /// wide covers only part of a cell.
    #[serde(default)]
    pub volcanic_area_fraction: f64,
}

/// Width (km) of the active volcanic belt along each boundary type:
/// subduction arcs, and rift zones where a ridge runs through a continent
/// (or the neovolcanic zone of an oceanic ridge). Collisions and transforms
/// carry no volcanic belt.
const ARC_BELT_WIDTH_KM: f64 = 150.0;
const CONTINENTAL_RIFT_BELT_WIDTH_KM: f64 = 60.0;
const OCEANIC_RIDGE_BELT_WIDTH_KM: f64 = 20.0;
/// Radius (km) of a hotspot's volcanic shield or plateau.
const HOTSPOT_SHIELD_RADIUS_KM: f64 = 150.0;
/// Area fraction above which a cell counts as volcanic ground.
const VOLCANIC_DOMINANCE_FRACTION: f64 = 0.5;

/// Volcanism state for Phase 2 integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcanismState {
    pub volcanism: Grid2<VolcanicCell>,
    pub outgassed_co2: Grid2<f64>,
    /// Cumulative fraction of the mantle's initial volatile inventory that
    /// has degassed. Starts at [`INITIAL_DEGASSED_FRACTION`] and grows with
    /// simulated time.
    #[serde(default = "initial_degassed_fraction")]
    pub degassed_fraction: f64,
}

/// Degassed fraction of the mantle volatile inventory when a world is
/// created (canon records no planet age; ~30% matches a mature, Earth-like
/// mantle).
pub const INITIAL_DEGASSED_FRACTION: f64 = 0.3;
/// e-folding time of mantle degassing (years).
const DEGASSING_TIMESCALE_YEARS: f64 = 1.5e9;
/// Share of qualifying hot, boundary-distant oceanic cells that host a
/// long-lived mantle plume (hotspot).
const HOTSPOT_PROBABILITY: f64 = 1.0 / 7.0;
/// Keyed RNG salt for hotspot placement. Drawn per cell at tick 0: plumes
/// are persistent features of the mantle, not re-rolled each step.
const HOTSPOT_SALT: u32 = 0x484F_5453;

fn initial_degassed_fraction() -> f64 {
    INITIAL_DEGASSED_FRACTION
}

const REFERENCE_MANTLE_TEMP_K: f64 = 1600.0;

impl VolcanismState {
    /// Get aggregate eruption intensity proxy from the outgassed CO₂ field.
    pub fn get_eruption_intensity(&self) -> f64 {
        self.outgassed_co2.average()
    }
}

impl VolcanicCell {
    /// Create new volcanic cell
    pub fn new(distance_to_boundary_km: f64, is_hot_spot: bool) -> Self {
        // Eruption probability inversely proportional to distance from boundary
        let eruption_prob = if is_hot_spot {
            0.01 // Hotspots erupt ~1% per year
        } else if distance_to_boundary_km < 50.0 {
            0.1 * (50.0 - distance_to_boundary_km) / 50.0 // Up to 10% near boundaries
        } else {
            0.001 // Low background rate
        };

        let outgassing = if is_hot_spot || distance_to_boundary_km < 100.0 {
            VolcanicGases {
                co2_mol_m2_yr: 1.0 + (100.0 - distance_to_boundary_km).max(0.0) * 0.05,
                h2o_mol_m2_yr: 5.0 + (100.0 - distance_to_boundary_km).max(0.0) * 0.1,
                n2_mol_m2_yr: 0.1,
                so2_mol_m2_yr: 0.01,
            }
        } else {
            VolcanicGases {
                co2_mol_m2_yr: 0.1,
                h2o_mol_m2_yr: 0.5,
                n2_mol_m2_yr: 0.01,
                so2_mol_m2_yr: 0.001,
            }
        };

        let heat_flux = if distance_to_boundary_km < 50.0 {
            100.0 - distance_to_boundary_km.min(50.0) // 50-100 mW/m²
        } else {
            10.0
        };

        VolcanicCell {
            eruption_prob,
            outgassing,
            heat_flux_mw_m2: heat_flux,
            distance_to_boundary_km,
            volcanic_area_fraction: 0.0,
        }
    }

    /// Whether volcanic terrain covers most of this cell.
    pub fn dominates_cell(&self) -> bool {
        self.volcanic_area_fraction >= VOLCANIC_DOMINANCE_FRACTION
    }
}

/// Share of a cell of `cell_width_km` × `cell_height_km` covered by volcanic
/// terrain: a belt along the boundary through it, and/or a hotspot shield.
fn volcanic_area_fraction(
    plate: &super::tectonics::PlateCell,
    is_hotspot: bool,
    cell_width_km: f64,
    cell_height_km: f64,
) -> f64 {
    use super::tectonics::{BoundaryType, PlateType};
    let cell_area_km2 = (cell_width_km * cell_height_km).max(1.0);
    let belt_width_km = match plate.boundary {
        Some(BoundaryType::Subduction) => ARC_BELT_WIDTH_KM,
        Some(BoundaryType::Ridge) if plate.crust_type == PlateType::Continental => {
            CONTINENTAL_RIFT_BELT_WIDTH_KM
        }
        Some(BoundaryType::Ridge) => OCEANIC_RIDGE_BELT_WIDTH_KM,
        Some(BoundaryType::Collision) | Some(BoundaryType::Transform) | None => 0.0,
    };
    // The belt crosses the cell along its longer side.
    let belt_km2 = belt_width_km * cell_width_km.max(cell_height_km);
    let shield_km2 = if is_hotspot {
        std::f64::consts::PI * HOTSPOT_SHIELD_RADIUS_KM * HOTSPOT_SHIELD_RADIUS_KM
    } else {
        0.0
    };
    ((belt_km2 + shield_km2) / cell_area_km2).clamp(0.0, 1.0)
}

/// Mantle outgassing rate based on temperature and plate activity
///
/// Higher temperature and ridge activity → higher outgassing
fn mantle_outgassing(
    mantle_temp_k: f64,
    boundary_heat_flow: f64,
    degassing_factor: f64, // Cumulative degassing fraction [0, 1]
) -> f64 {
    // Reference mantle temp ~1600 K
    let ref_temp = 1600.0;
    let temp_anomaly = mantle_temp_k - ref_temp;

    // Outgassing scales with temperature anomaly (exponentially)
    let temp_dependence = 0.1 * (temp_anomaly / 100.0).min(5.0); // Saturation at hot

    // Boundary heat flow directly drives outgassing
    let boundary_dependence = boundary_heat_flow / 100.0; // Normalized to ridge heat

    // Degassing reduces future outgassing
    let degassing_reduction = (1.0 - degassing_factor * 0.5).max(0.1);

    (temp_dependence + boundary_dependence) * degassing_reduction
}

/// Compute volcanic CO₂ flux
///
/// Returns CO₂ molar flux (mol/m²/yr)
fn volcanic_co2_flux(
    mantle_outgassing_rate: f64,
    c_fraction: f64, // CO₂ as fraction of outgassing
) -> f64 {
    mantle_outgassing_rate * c_fraction * 100.0
}

/// Step volcanism for one tick
///
/// Compute outgassing, heat flow, eruption probability
impl VolcanismState {
    /// Planet-wide volcanic CO2 outgassing (mol/yr): each cell's
    /// `co2_mol_m2_yr` over its area.
    pub fn total_co2_mol_yr(&self, grid_spec: &mk_core::grid::GridSpec) -> f64 {
        self.volcanism
            .indexed_iter()
            .map(|(row, _, cell)| {
                cell.outgassing.co2_mol_m2_yr.max(0.0)
                    * grid_spec
                        .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
                        .unwrap_or(0.0)
            })
            .sum()
    }

    /// Geothermal heat flux per cell in W/m² (the stored field is mW/m²).
    pub fn heat_flux_w_m2(&self, grid_spec: &mk_core::grid::GridSpec) -> Grid2<f64> {
        let data = self
            .volcanism
            .data()
            .iter()
            .map(|cell| cell.heat_flux_mw_m2 / 1000.0)
            .collect();
        Grid2::from_data(grid_spec, data)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn step_volcanism(
    _canon: &CanonLocked,
    _tick: Tick,
    plate_config: &Grid2<super::tectonics::PlateCell>,
    heat_flow: &Grid2<f64>,
    previous_degassed_fraction: f64,
    dt_seconds: f64,
    rng: &mk_core::rng::RngRegistry,
    grid_spec: &mk_core::grid::GridSpec,
) -> VolcanismState {
    let grid_rows = grid_spec.nlat;
    let grid_cols = grid_spec.nlon;
    let planet_radius_km = mk_core::grid::CANON_PLANET_RADIUS_M / 1000.0;

    let mut volcanism = Grid2::new(grid_spec, VolcanicCell::new(500.0, false));
    let mut outgassed_co2 = Grid2::new(grid_spec, 0.0);

    // Cumulative degassing advances with simulated time.
    let dt_years = dt_seconds.max(0.0) / (365.25 * 86_400.0);
    let degassing_factor = (1.0
        - (1.0 - previous_degassed_fraction.clamp(0.0, 1.0))
            * (-dt_years / DEGASSING_TIMESCALE_YEARS).exp())
    .clamp(0.0, 1.0);

    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let plate = *plate_config.get(row, col);
            let heat = *heat_flow.get(row, col);

            // The current world model exposes local mantle heat flow rather than a
            // separate temperature field. Convert that live signal into a bounded
            // thermal proxy so temperature-dependent outgassing remains active.
            let mantle_temp =
                (REFERENCE_MANTLE_TEMP_K + (heat - 100.0) * 0.5).clamp(1200.0, 2200.0);

            // Cell dimensions on the sphere (km): rows are evenly spaced in
            // latitude, columns shrink with cos(latitude).
            let cell_height_km = std::f64::consts::PI * planet_radius_km / grid_rows as f64;
            let cell_width_km =
                (std::f64::consts::TAU * planet_radius_km * grid_spec.lat_rad(row).cos().abs()
                    / grid_cols as f64)
                    .max(1.0);

            // Nearest-boundary distance from the local Moore neighbourhood,
            // at the real spacing between cell centres.
            let distance_to_boundary: f64 = if plate.boundary.is_some() {
                0.0
            } else {
                let row_start = row.saturating_sub(1);
                let row_end = (row + 1).min(grid_rows - 1);
                let mut min_dist_km = cell_width_km.min(cell_height_km);
                let mut found = false;
                for neighbor_row in row_start..=row_end {
                    for dc in [-1i64, 0, 1] {
                        let neighbor_col = (col as i64 + dc).rem_euclid(grid_cols as i64) as usize;
                        if neighbor_row == row && dc == 0 {
                            continue;
                        }
                        if plate_config
                            .get(neighbor_row, neighbor_col)
                            .boundary
                            .is_some()
                        {
                            let dy = neighbor_row.abs_diff(row) as f64 * cell_height_km;
                            let dx = dc.unsigned_abs() as f64 * cell_width_km;
                            let d = (dx * dx + dy * dy).sqrt();
                            min_dist_km = if found { min_dist_km.min(d) } else { d };
                            found = true;
                        }
                    }
                }
                min_dist_km
            };

            // Hotspots favor hot oceanic lithosphere away from active plate boundaries.
            let is_hotspot = plate.crust_type == super::tectonics::PlateType::Oceanic
                && distance_to_boundary > 75.0
                && heat > 120.0
                && rng.gen_f64_01(mk_core::rng::RngKey::new(
                    mk_core::rng::SubsystemId::Volcanism,
                    HOTSPOT_SALT,
                    (row * grid_cols + col) as u32,
                    0,
                )) < HOTSPOT_PROBABILITY;

            // Create volcanic cell
            let mut vol_cell = VolcanicCell::new(distance_to_boundary, is_hotspot);
            vol_cell.volcanic_area_fraction =
                volcanic_area_fraction(&plate, is_hotspot, cell_width_km, cell_height_km);

            // Update outgassing based on mantle state and boundary heat
            let mantle_outgassing_rate = mantle_outgassing(mantle_temp, heat, degassing_factor);

            // Adjust outgassing in the cell
            let adjustment = mantle_outgassing_rate / 10.0; // Scale factor
            vol_cell.outgassing.co2_mol_m2_yr *= (1.0 + adjustment).min(10.0);
            vol_cell.outgassing.h2o_mol_m2_yr *= (1.0 + adjustment).min(10.0);

            // Update heat flux
            vol_cell.heat_flux_mw_m2 = (heat * 0.1).clamp(5.0, 200.0);

            // Compute CO₂ flux
            let co2_flux = volcanic_co2_flux(mantle_outgassing_rate, 0.5); // 50% CO₂ by molar fraction
            outgassed_co2.set(row, col, co2_flux);

            volcanism.set(row, col, vol_cell);
        }
    }

    VolcanismState {
        volcanism,
        outgassed_co2,
        degassed_fraction: degassing_factor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volcanic_gases_total() {
        let gases = VolcanicGases {
            co2_mol_m2_yr: 1.0,
            h2o_mol_m2_yr: 2.0,
            n2_mol_m2_yr: 3.0,
            so2_mol_m2_yr: 4.0,
        };
        assert_eq!(gases.total_mol_m2_yr(), 10.0);
    }

    #[test]
    fn volcanic_cell_near_boundary() {
        let near = VolcanicCell::new(10.0, false);
        let far = VolcanicCell::new(500.0, false);

        assert!(
            near.eruption_prob > far.eruption_prob,
            "Cells near boundaries should erupt more frequently"
        );
    }

    #[test]
    fn hotspot_higher_activity() {
        let hotspot = VolcanicCell::new(200.0, true);
        let regular = VolcanicCell::new(200.0, false);

        assert!(hotspot.eruption_prob > regular.eruption_prob);
    }

    #[test]
    fn outgassing_increases_with_temp_anomaly() {
        let low_temp = mantle_outgassing(1500.0, 100.0, 0.3);
        let high_temp = mantle_outgassing(1700.0, 100.0, 0.3);

        assert!(high_temp > low_temp, "Hotter mantle should outgas more");
    }

    #[test]
    fn outgassing_reduced_by_degassing() {
        let fresh = mantle_outgassing(1600.0, 100.0, 0.0); // Fresh planet
        let degassed = mantle_outgassing(1600.0, 100.0, 0.9); // Well-degassed

        assert!(fresh > degassed, "Degassed planets outgas less");
    }

    #[test]
    fn degassing_accumulates_with_simulated_time() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(8, 16);
        let tectonics = crate::tectonics::step_tectonics(&canon, 0, &grid_spec);
        let rng = mk_core::rng::RngRegistry::new([2u8; 32]);
        let step = |previous: f64, dt_years: f64| {
            step_volcanism(
                &canon,
                0,
                &tectonics.plates,
                &tectonics.heat_flow,
                previous,
                dt_years * 365.25 * 86_400.0,
                &rng,
                &grid_spec,
            )
            .degassed_fraction
        };
        assert!((step(INITIAL_DEGASSED_FRACTION, 0.0) - INITIAL_DEGASSED_FRACTION).abs() < 1e-12);
        let after_a_gyr = step(INITIAL_DEGASSED_FRACTION, 1.0e9);
        assert!(after_a_gyr > INITIAL_DEGASSED_FRACTION && after_a_gyr < 1.0);
    }

    #[test]
    fn volcanic_ground_needs_a_belt_that_covers_the_cell() {
        use crate::tectonics::{BoundaryType, PlateCell, PlateType};
        let mut arc = PlateCell::new(0, PlateType::Oceanic, 20.0);
        arc.boundary = Some(BoundaryType::Subduction);
        let mut collision = PlateCell::new(0, PlateType::Continental, 100.0);
        collision.boundary = Some(BoundaryType::Collision);

        // A 150 km arc covers most of a 200 km cell but little of a 1900 km one.
        assert!(volcanic_area_fraction(&arc, false, 200.0, 200.0) >= 0.5);
        assert!(volcanic_area_fraction(&arc, false, 1900.0, 1900.0) < 0.1);
        // Mountain-building collisions are not volcanic.
        assert_eq!(volcanic_area_fraction(&collision, false, 200.0, 200.0), 0.0);
        // A hotspot shield dominates a small cell.
        let interior = PlateCell::new(0, PlateType::Oceanic, 20.0);
        assert!(volcanic_area_fraction(&interior, true, 250.0, 250.0) >= 0.5);
    }
}
