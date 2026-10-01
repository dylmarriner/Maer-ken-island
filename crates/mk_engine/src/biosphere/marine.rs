//! Marine net primary production (phytoplankton), per ocean cell.
//!
//! Productivity is the product of a maximum rate and three limitation
//! factors, each from live world state:
//!
//! - **Light**: the VGPM light-saturation form `E / (E + 4.1)` (Behrenfeld &
//!   Falkowski 1997), with `E` the annual-mean photosynthetically active
//!   radiation reaching the sea surface, in mol photons m⁻² d⁻¹. `E` comes
//!   from the canon solar constant and obliquity via
//!   [`crate::climate::daily_mean_insolation`] averaged over the year.
//! - **Temperature**: the VGPM optimal assimilation efficiency `P^B_opt(T)`
//!   (Behrenfeld & Falkowski 1997, their 7th-order fit: 1.13 below −1 °C,
//!   rising to ≈6.6 mgC mgChl⁻¹ h⁻¹ near 20 °C and 4.0 above 28.5 °C),
//!   normalised by its peak. Water at or below the seawater freezing point
//!   is ice-covered and produces nothing.
//! - **Nutrients**: a Michaelis–Menten term on the rate at which deep,
//!   nutrient-rich water reaches the surface. That supply is the Ekman
//!   upwelling velocity from the wind-driven Ekman transport
//!   `M = τ × ẑ / (ρ₀ f)`, plus vertical mixing over continental shelves.
//!   In the open ocean the supply is Ekman pumping, the divergence of `M`.
//!   At a coast, transport cannot cross the shoreline, so offshore transport
//!   upwells within about one baroclinic Rossby radius (`c / |f|`) of the
//!   coast. Coastal cells are split into that upwelling strip and the rest
//!   of the cell, and their NPP is the area-weighted mean, so the result
//!   does not depend on grid resolution.
//!   Stratified open ocean keeps a regenerated-nutrient baseline, so that
//!   oligotrophic gyres are low but not zero.

use crate::weather::WindVector;
use mk_core::grid::Grid2;

/// Upper annual NPP of the most productive upwelling systems (Peru,
/// Benguela), kgC m⁻² yr⁻¹.
pub const MARINE_NPP_MAX_KGC_M2_YR: f64 = 1.5;
/// VGPM light-saturation constant, mol photons m⁻² d⁻¹.
pub const LIGHT_SATURATION_MOL_M2_D: f64 = 4.1;
/// Fraction of top-of-atmosphere shortwave reaching the surface, global
/// mean including cloud (Trenberth, Fasullo & Kiehl 2009: 184 of 341 W m⁻²).
pub const SURFACE_TRANSMISSION: f64 = 0.54;
/// Sea-surface shortwave albedo.
pub const OCEAN_ALBEDO: f64 = 0.06;
/// Fraction of shortwave energy in the 400–700 nm PAR band.
pub const PAR_FRACTION: f64 = 0.45;
/// mol photons m⁻² d⁻¹ delivered by 1 W m⁻² of PAR for a day
/// (4.57 µmol J⁻¹ × 86 400 s).
pub const PAR_MOL_PER_W_DAY: f64 = 4.57e-6 * 86_400.0;
/// Freezing point of seawater at 35 PSU, K.
pub const SEAWATER_FREEZING_K: f64 = 271.35;
/// Air density at sea level, kg m⁻³.
pub const AIR_DENSITY_KG_M3: f64 = 1.225;
/// Neutral drag coefficient for wind stress over the sea (Large & Pond 1981).
pub const DRAG_COEFFICIENT: f64 = 1.3e-3;
/// Reference seawater density, kg m⁻³.
pub const SEAWATER_DENSITY_KG_M3: f64 = 1025.0;
/// First baroclinic gravity-wave speed setting the coastal upwelling
/// width `c / |f|` (Chelton et al. 1998: 2–3 m s⁻¹ at mid-latitudes).
pub const BAROCLINIC_WAVE_SPEED_M_S: f64 = 2.5;
/// Ekman theory breaks down as the Coriolis parameter vanishes; transport
/// is evaluated no closer to the equator than this latitude.
pub const MIN_EKMAN_LATITUDE_DEG: f64 = 5.0;
/// Water shallower than this is continental shelf, where tidal and wind
/// mixing reach the bottom, m.
pub const SHELF_DEPTH_M: f64 = 200.0;
/// Nutrient supply from shelf mixing, expressed as an equivalent upwelling
/// velocity, m d⁻¹.
pub const SHELF_SUPPLY_M_D: f64 = 1.0;
/// Supply at which nutrient limitation is half relieved, m d⁻¹ (typical
/// coastal upwelling runs at 1–10 m d⁻¹).
pub const SUPPLY_HALF_SATURATION_M_D: f64 = 1.0;
/// Nutrient factor of stratified open ocean from regenerated nutrients.
pub const OLIGOTROPHIC_NUTRIENT_FACTOR: f64 = 0.1;
/// Declination samples used to average insolation over the year.
const YEAR_SAMPLES: usize = 48;

/// Behrenfeld & Falkowski (1997) `P^B_opt(T)`, mgC mgChl⁻¹ h⁻¹.
pub fn optimal_assimilation_efficiency(celsius: f64) -> f64 {
    const COEFFICIENTS: [f64; 8] = [
        1.2956, 2.749e-1, 6.17e-2, -2.05e-2, 2.462e-3, -1.348e-4, 3.4132e-6, -3.27e-8,
    ];
    if celsius < -1.0 {
        return 1.13;
    }
    if celsius > 28.5 {
        return 4.0;
    }
    COEFFICIENTS
        .iter()
        .rev()
        .fold(0.0, |acc, coefficient| acc * celsius + coefficient)
}

/// Peak of [`optimal_assimilation_efficiency`] over its valid range.
fn peak_assimilation_efficiency() -> f64 {
    (0..=295)
        .map(|tenth| optimal_assimilation_efficiency(-1.0 + tenth as f64 * 0.1))
        .fold(0.0, f64::max)
}

/// Temperature factor in [0, 1]: 0 under sea ice, 1 at the optimum.
pub fn temperature_factor(sea_surface_temperature_k: f64) -> f64 {
    if sea_surface_temperature_k <= SEAWATER_FREEZING_K {
        return 0.0;
    }
    optimal_assimilation_efficiency(sea_surface_temperature_k - 273.15)
        / peak_assimilation_efficiency()
}

/// VGPM light factor for annual-mean surface PAR (mol photons m⁻² d⁻¹).
pub fn light_factor(par_mol_m2_d: f64) -> f64 {
    let par = par_mol_m2_d.max(0.0);
    par / (par + LIGHT_SATURATION_MOL_M2_D)
}

/// Nutrient factor for a supply rate of deep water to the surface (m d⁻¹).
pub fn nutrient_factor(supply_m_d: f64) -> f64 {
    let supply = supply_m_d.max(0.0);
    OLIGOTROPHIC_NUTRIENT_FACTOR
        + (1.0 - OLIGOTROPHIC_NUTRIENT_FACTOR) * supply / (supply + SUPPLY_HALF_SATURATION_M_D)
}

/// Annual-mean surface PAR at `latitude_rad`, mol photons m⁻² d⁻¹.
pub fn annual_mean_surface_par(
    solar_constant_w_m2: f64,
    obliquity_rad: f64,
    latitude_rad: f64,
) -> f64 {
    let toa = (0..YEAR_SAMPLES)
        .map(|k| {
            let season = std::f64::consts::TAU * (k as f64 + 0.5) / YEAR_SAMPLES as f64;
            crate::climate::daily_mean_insolation(
                solar_constant_w_m2,
                latitude_rad,
                obliquity_rad * season.sin(),
            )
        })
        .sum::<f64>()
        / YEAR_SAMPLES as f64;
    toa * SURFACE_TRANSMISSION * (1.0 - OCEAN_ALBEDO) * PAR_FRACTION * PAR_MOL_PER_W_DAY
}

/// World fields the marine model reads, fixed for one production update.
pub struct MarineInputs<'a> {
    pub wind: &'a Grid2<WindVector>,
    pub elevation: &'a Grid2<f64>,
    pub planet_radius_m: f64,
    pub rotation_rate_rad_s: f64,
}

impl MarineInputs<'_> {
    fn is_ocean(&self, row: usize, col: usize) -> bool {
        *self.elevation.get(row, col) < 0.0
    }

    /// Ekman volume transport per unit width (m² s⁻¹) at a cell, zero on
    /// land. `M = τ × ẑ / (ρ₀ f)`: 90° to the right of the wind stress in
    /// the northern hemisphere and to the left in the southern.
    fn ekman_transport(&self, row: usize, col: usize, lat_rad: f64) -> (f64, f64) {
        if !self.is_ocean(row, col) {
            return (0.0, 0.0);
        }
        let wind = self.wind.get(row, col);
        let speed = (wind.u_east * wind.u_east + wind.v_north * wind.v_north).sqrt();
        let (tau_x, tau_y) = (
            AIR_DENSITY_KG_M3 * DRAG_COEFFICIENT * speed * wind.u_east,
            AIR_DENSITY_KG_M3 * DRAG_COEFFICIENT * speed * wind.v_north,
        );
        let min_lat = MIN_EKMAN_LATITUDE_DEG.to_radians();
        let effective_lat = if lat_rad.abs() < min_lat {
            min_lat.copysign(if lat_rad == 0.0 { 1.0 } else { lat_rad })
        } else {
            lat_rad
        };
        let f = 2.0 * self.rotation_rate_rad_s * effective_lat.sin();
        if f == 0.0 {
            return (0.0, 0.0);
        }
        (
            tau_y / (SEAWATER_DENSITY_KG_M3 * f),
            -tau_x / (SEAWATER_DENSITY_KG_M3 * f),
        )
    }

    fn coriolis(&self, lat_rad: f64) -> f64 {
        let min_lat = MIN_EKMAN_LATITUDE_DEG.to_radians();
        let magnitude = lat_rad.abs().max(min_lat);
        2.0 * self.rotation_rate_rad_s * magnitude.sin()
    }

    /// Nutrient supply at an ocean cell, split into the open part of the cell
    /// and its coastal upwelling strip.
    fn supply(&self, row: usize, col: usize) -> CellSupply {
        let (nlat, nlon) = (self.elevation.nlat(), self.elevation.nlon());
        if nlat < 2 || nlon < 2 {
            return CellSupply::default();
        }
        let lat = |r: usize| {
            (r as f64 + 0.5) / nlat as f64 * std::f64::consts::PI - std::f64::consts::FRAC_PI_2
        };
        let lat_here = lat(row);
        let radius = self.planet_radius_m;
        let dlon = std::f64::consts::TAU / nlon as f64;
        let dlat = std::f64::consts::PI / nlat as f64;
        let cos_here = lat_here.cos().max(1e-3);
        let (mx, my) = self.ekman_transport(row, col, lat_here);

        // Interior Ekman pumping: divergence of M with coastal faces treated
        // as zero-gradient, so only the wind-stress curl contributes.
        let neighbour = |r: usize, c: usize| {
            if self.is_ocean(r, c) {
                self.ekman_transport(r, c, lat(r))
            } else {
                (mx, my)
            }
        };
        let (east, west) = ((col + 1) % nlon, (col + nlon - 1) % nlon);
        let (north, south) = (row.min(nlat - 2) + 1, row.max(1) - 1);
        let dmx =
            (neighbour(row, east).0 - neighbour(row, west).0) / (2.0 * dlon * radius * cos_here);
        let dmy = (neighbour(north, col).1 * lat(north).cos()
            - neighbour(south, col).1 * lat(south).cos())
            / ((north - south) as f64 * dlat * radius * cos_here);
        let interior_m_d = (dmx + dmy) * 86_400.0;

        // Coastal upwelling: net offshore transport across each coastline
        // face (m³ s⁻¹), upwelled within one Rossby radius of that coast.
        let rossby_radius = BAROCLINIC_WAVE_SPEED_M_S / self.coriolis(lat_here);
        let ew_face_length = dlat * radius;
        let ns_face_length = dlon * radius * cos_here;
        let mut offshore_m3_s = 0.0;
        let mut strip_area_m2 = 0.0;
        for (is_land, offshore, face_length) in [
            (!self.is_ocean(row, east), -mx, ew_face_length),
            (!self.is_ocean(row, west), mx, ew_face_length),
            (
                row + 1 < nlat && !self.is_ocean(row + 1, col),
                -my,
                ns_face_length,
            ),
            (row > 0 && !self.is_ocean(row - 1, col), my, ns_face_length),
        ] {
            if is_land {
                offshore_m3_s += offshore * face_length;
                strip_area_m2 += rossby_radius * face_length;
            }
        }
        let cell_area_m2 = radius * radius * cos_here * dlon * dlat;
        let strip_fraction = (strip_area_m2 / cell_area_m2).min(1.0);
        let strip_m_d = if strip_area_m2 > 0.0 {
            interior_m_d + offshore_m3_s / strip_area_m2 * 86_400.0
        } else {
            interior_m_d
        };
        CellSupply {
            interior_m_d,
            strip_m_d,
            strip_fraction,
        }
    }
}

/// Upwelling supply at one ocean cell, m d⁻¹.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct CellSupply {
    /// Ekman pumping over the open part of the cell.
    interior_m_d: f64,
    /// Upwelling within the coastal strip (pumping plus offshore transport).
    strip_m_d: f64,
    /// Share of the cell's area that is coastal upwelling strip.
    strip_fraction: f64,
}

/// Marine NPP in kgC m⁻² yr⁻¹ for the ocean cell at `row`/`col`.
pub fn marine_npp_kgc_m2_yr(
    inputs: &MarineInputs<'_>,
    sea_surface_temperature_k: f64,
    row: usize,
    col: usize,
    surface_par_mol_m2_d: f64,
) -> f64 {
    let shelf = if -*inputs.elevation.get(row, col) < SHELF_DEPTH_M {
        SHELF_SUPPLY_M_D
    } else {
        0.0
    };
    let supply = inputs.supply(row, col);
    let nutrients = (1.0 - supply.strip_fraction) * nutrient_factor(supply.interior_m_d + shelf)
        + supply.strip_fraction * nutrient_factor(supply.strip_m_d + shelf);
    MARINE_NPP_MAX_KGC_M2_YR
        * light_factor(surface_par_mol_m2_d)
        * temperature_factor(sea_surface_temperature_k)
        * nutrients
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factors_follow_their_published_forms() {
        assert_eq!(temperature_factor(SEAWATER_FREEZING_K - 1.0), 0.0);
        // The published curve peaks near 20 °C at ~6.6 and is continuous
        // with its fixed values at both ends of the fitted range.
        assert!((optimal_assimilation_efficiency(20.0) - 6.62).abs() < 0.01);
        assert!((optimal_assimilation_efficiency(-1.0) - 1.13).abs() < 0.03);
        assert!((optimal_assimilation_efficiency(28.5) - 4.0).abs() < 0.03);
        assert!(temperature_factor(293.15) > 0.99);
        assert!(temperature_factor(283.15) < temperature_factor(293.15));
        assert!(temperature_factor(301.65) < temperature_factor(293.15));
        assert!((light_factor(LIGHT_SATURATION_MOL_M2_D) - 0.5).abs() < 1e-12);
        assert!((nutrient_factor(0.0) - OLIGOTROPHIC_NUTRIENT_FACTOR).abs() < 1e-12);
        assert!(nutrient_factor(10.0) > 0.9);
    }

    #[test]
    fn surface_par_is_highest_in_the_tropics_and_earthlike_in_magnitude() {
        let obliquity = 23.44f64.to_radians();
        let equator = annual_mean_surface_par(1361.0, obliquity, 0.0);
        let pole = annual_mean_surface_par(1361.0, obliquity, 89f64.to_radians());
        assert!(equator > pole);
        // Observed tropical annual-mean surface PAR is ~40-50 mol m⁻² d⁻¹.
        assert!(equator > 30.0 && equator < 60.0, "equatorial PAR {equator}");
    }

    #[test]
    fn coastal_upwelling_and_shelves_raise_productivity() {
        // Northern-hemisphere row with a coast to the east. A southward
        // (equatorward) wind drives Ekman transport to its right, i.e. west,
        // away from the coast: the classic eastern-boundary upwelling of
        // California or Portugal.
        let spec = mk_core::grid::GridSpec::new(16, 32);
        let row = 11; // ~34° N
        let calm_wind = Grid2::new(
            &spec,
            WindVector {
                u_east: 0.0,
                v_north: 0.0,
            },
        );
        let equatorward = Grid2::new(
            &spec,
            WindVector {
                u_east: 0.0,
                v_north: -8.0,
            },
        );
        let mut elevation = Grid2::new(&spec, -4000.0);
        *elevation.get_mut(row, 11) = 500.0;
        let omega = std::f64::consts::TAU / 86_164.0;
        let inputs = |wind| MarineInputs {
            wind,
            elevation: &elevation,
            planet_radius_m: 6.371e6,
            rotation_rate_rad_s: omega,
        };
        let npp = |wind, col| marine_npp_kgc_m2_yr(&inputs(wind), 293.15, row, col, 40.0);

        let calm = npp(&calm_wind, 10);
        let coastal = npp(&equatorward, 10);
        let offshore = npp(&equatorward, 5);
        assert!(
            coastal > offshore,
            "coastal {coastal} vs offshore {offshore}"
        );
        assert!(
            (offshore - calm).abs() < 1e-9,
            "uniform open-ocean wind has no divergence"
        );
        let supply = inputs(&equatorward).supply(row, 10);
        assert!(
            supply.strip_m_d > 1.0 && supply.strip_m_d < 20.0,
            "coastal strip upwelling {} m/day",
            supply.strip_m_d
        );
        assert!(supply.strip_fraction > 0.0 && supply.strip_fraction < 0.2);
        assert!(coastal <= MARINE_NPP_MAX_KGC_M2_YR);

        *elevation.get_mut(row, 20) = -100.0;
        let shelf_inputs = MarineInputs {
            wind: &calm_wind,
            elevation: &elevation,
            planet_radius_m: 6.371e6,
            rotation_rate_rad_s: omega,
        };
        assert!(
            marine_npp_kgc_m2_yr(&shelf_inputs, 293.15, row, 20, 40.0) > calm,
            "shelf mixing"
        );
    }
}
