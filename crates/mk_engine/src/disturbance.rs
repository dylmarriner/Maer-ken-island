//! Discrete environmental disturbances (fire, flood, eruption, impact,
//! disease, drought).
//!
//! Before this module, `WorldState::disturbance_state` was a unit struct — a
//! named placeholder that carried no information and so could not be the
//! target of `mk_interventions::InterventionAction::TriggerDisturbance`. A
//! disturbance here is a real, bounded, decaying record: it is placed at a
//! grid cell, it has an intensity that falls off over a finite lifetime, and
//! [`step_disturbances`] applies its effect to the subsystem it physically
//! belongs to while pushing the matching conserved-quantity flux into the
//! ledger.
//!
//! Effects are deliberately modelled through the world's *existing* state
//! (soil water, precipitation, surface temperature, species populations,
//! volcanic heat) rather than by inventing parallel fields, so a disturbance
//! is visible to every projection and audit that already reads those fields.
//!
//! Intensity is a dimensionless `0.0..=1.0` severity, not a physical unit;
//! each effect converts it into that subsystem's units at the point of use
//! and documents the conversion. Every hazard that strikes a place also
//! kills the exposed share of the biosphere, the canon loss `F = χ · B`
//! (`BIOSPHERE_EQUATIONS.md` § 16), with intensity as the fraction of the
//! exposed population killed.

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};
use mk_core::grid::GridSpec;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

/// The kind of disturbance, mirroring
/// `mk_interventions::DisturbanceType` one-to-one.
///
/// Kept as its own engine-side type so the engine does not force every
/// consumer of `WorldState` to depend on the intervention contract crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisturbanceKind {
    Fire,
    Flood,
    VolcanicEruption,
    MeteorImpact,
    Disease,
    Drought,
}

impl DisturbanceKind {
    /// Stable name, used in audit entries and UI labels.
    pub fn as_str(&self) -> &'static str {
        match self {
            DisturbanceKind::Fire => "Fire",
            DisturbanceKind::Flood => "Flood",
            DisturbanceKind::VolcanicEruption => "VolcanicEruption",
            DisturbanceKind::MeteorImpact => "MeteorImpact",
            DisturbanceKind::Disease => "Disease",
            DisturbanceKind::Drought => "Drought",
        }
    }

    /// How many ticks this kind of disturbance remains active.
    ///
    /// These are model parameters chosen so that the ordering matches the
    /// real-world persistence ordering of these hazards (an impact and an
    /// eruption outlast a flood; a drought outlasts a fire), not measured
    /// durations.
    pub fn duration_ticks(&self) -> u64 {
        match self {
            DisturbanceKind::Fire => 40,
            DisturbanceKind::Flood => 20,
            DisturbanceKind::VolcanicEruption => 120,
            DisturbanceKind::MeteorImpact => 200,
            DisturbanceKind::Disease => 150,
            DisturbanceKind::Drought => 300,
        }
    }

    /// Radius, in grid cells, over which the disturbance acts.
    pub fn radius_cells(&self) -> usize {
        match self {
            DisturbanceKind::Fire => 1,
            DisturbanceKind::Flood => 1,
            DisturbanceKind::VolcanicEruption => 2,
            DisturbanceKind::MeteorImpact => 3,
            // Disease follows hosts, not geography; it is applied to species
            // populations rather than to cells, so its cell radius is 0.
            DisturbanceKind::Disease => 0,
            DisturbanceKind::Drought => 3,
        }
    }
}

/// A single active disturbance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveDisturbance {
    pub kind: DisturbanceKind,
    /// Grid cell the disturbance is centred on.
    pub row: usize,
    pub col: usize,
    /// Severity at onset, `0.0..=1.0`.
    pub intensity: f64,
    /// Tick the disturbance was created.
    pub onset_tick: Tick,
    /// Tick after which the disturbance is retired.
    pub expiry_tick: Tick,
}

impl ActiveDisturbance {
    /// Create a disturbance centred on `(row, col)`.
    ///
    /// `intensity` is clamped into `0.0..=1.0`; a non-finite intensity is
    /// treated as zero rather than propagating a `NaN` into world state.
    pub fn new(
        kind: DisturbanceKind,
        row: usize,
        col: usize,
        intensity: f64,
        onset_tick: Tick,
    ) -> Self {
        let intensity = if intensity.is_finite() {
            intensity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            kind,
            row,
            col,
            intensity,
            onset_tick,
            expiry_tick: onset_tick.saturating_add(kind.duration_ticks()),
        }
    }

    /// Whether this disturbance is still active at `tick`.
    pub fn is_active(&self, tick: Tick) -> bool {
        tick < self.expiry_tick
    }

    /// Intensity at `tick`, decaying linearly from onset to expiry.
    ///
    /// Linear decay is the simplest form that is monotone and reaches exactly
    /// zero at expiry, so a disturbance never leaves a residual effect after
    /// it is retired.
    pub fn current_intensity(&self, tick: Tick) -> f64 {
        if !self.is_active(tick) {
            return 0.0;
        }
        let total = self.expiry_tick.saturating_sub(self.onset_tick);
        if total == 0 {
            return 0.0;
        }
        let elapsed = tick.saturating_sub(self.onset_tick);
        let remaining = total.saturating_sub(elapsed) as f64 / total as f64;
        self.intensity * remaining
    }

    /// Cells this disturbance acts on, clamped to the grid.
    ///
    /// Longitude wraps (the grid is cyclic east-west); latitude does not (the
    /// poles are edges, not seams).
    pub fn affected_cells(&self, spec: &GridSpec) -> Vec<(usize, usize)> {
        let radius = self.kind.radius_cells();
        let mut cells = Vec::new();
        let radius_i = radius as isize;
        for drow in -radius_i..=radius_i {
            for dcol in -radius_i..=radius_i {
                let row = self.row as isize + drow;
                if row < 0 || row >= spec.nlat() as isize {
                    continue;
                }
                let col = (self.col as isize + dcol).rem_euclid(spec.nlon() as isize);
                cells.push((row as usize, col as usize));
            }
        }
        cells
    }
}

/// Active disturbances carried in `WorldState`.
///
/// `#[serde(default)]` on the field in `WorldState` keeps snapshots written
/// before this type had content loadable — the same convention used for
/// `ClimateState::absorbed_flux_w_m2`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DisturbanceState {
    pub active: Vec<ActiveDisturbance>,
    /// Count of disturbances retired over the run, for audit reporting.
    pub retired_count: u64,
}

impl DisturbanceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new disturbance.
    pub fn push(&mut self, disturbance: ActiveDisturbance) {
        self.active.push(disturbance);
    }

    /// Drop disturbances whose lifetime has elapsed.
    pub fn retire_expired(&mut self, tick: Tick) {
        let before = self.active.len();
        self.active.retain(|d| d.is_active(tick));
        self.retired_count += (before - self.active.len()) as u64;
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn len(&self) -> usize {
        self.active.len()
    }
}

/// What a single disturbance did this tick, for audit and UI reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisturbanceEffect {
    pub kind: DisturbanceKind,
    pub row: usize,
    pub col: usize,
    pub intensity: f64,
    /// Human-readable description of the concrete change applied.
    pub summary: String,
}

/// Millimetres of water a maximum-intensity flood delivers to a cell per
/// tick. Model parameter, sized relative to the 300 mm field capacity that
/// [`crate::hydrology::SoilWater`] uses.
const FLOOD_WATER_MM: f64 = 60.0;
/// Fraction of a cell's stored soil water a maximum-intensity drought
/// removes per tick. Model parameter.
const DROUGHT_SOIL_WATER_FRACTION: f64 = 0.1;
/// Soil field capacity in millimetres, matching
/// [`crate::hydrology::SoilWater::new`].
const SOIL_FIELD_CAPACITY_MM: f64 = 300.0;
/// Kelvin of surface warming at maximum eruption/impact intensity. Model
/// parameter, scaled by decaying intensity at the point of use.
const THERMAL_DISTURBANCE_KELVIN: f64 = 12.0;
/// Fraction of a species' population a maximum-intensity disease outbreak
/// removes per tick. Model parameter.
const DISEASE_MORTALITY_FRACTION: f64 = 0.02;
/// Fraction of a cell's precipitation a maximum-intensity drought suppresses.
const DROUGHT_PRECIPITATION_FRACTION: f64 = 0.8;
/// Fraction of a cell's moisture a maximum-intensity fire drives off.
const FIRE_MOISTURE_FRACTION: f64 = 0.3;

/// Apply every active disturbance's effect for this tick.
///
/// Called from the world pipeline after the physical subsystems have stepped,
/// so a disturbance perturbs the tick's computed state rather than being
/// overwritten by it. Every conserved-quantity change is pushed to `ledger`
/// against [`Reservoir::OperatorIntervention`] — disturbances are exogenous
/// events, so their mass/energy genuinely crosses the system boundary and
/// must be recorded there for [`Ledger::audit`] to close.
///
/// Returns one [`DisturbanceEffect`] per disturbance that did something.
#[allow(clippy::too_many_arguments)]
pub fn step_disturbances(
    state: &mut DisturbanceState,
    tick: Tick,
    spec: &GridSpec,
    elevation: &mk_core::grid::Grid2<f64>,
    climate: &mut crate::climate::ClimateState,
    weather: &mut crate::weather::WeatherState,
    hydrology: &mut crate::hydrology::HydrologyState,
    biosphere: &mut crate::biosphere::BiosphereSystem,
    ledger: &mut Ledger,
) -> Vec<DisturbanceEffect> {
    state.retire_expired(tick);

    let mut effects = Vec::new();
    // Sorted iteration so multi-disturbance ticks apply in a stable order and
    // the resulting world hash does not depend on insertion history.
    let mut ordered: Vec<ActiveDisturbance> = state.active.clone();
    ordered.sort_by(|a, b| {
        (a.onset_tick, a.row, a.col, a.kind.as_str()).cmp(&(
            b.onset_tick,
            b.row,
            b.col,
            b.kind.as_str(),
        ))
    });

    for disturbance in &ordered {
        let intensity = disturbance.current_intensity(tick);
        if intensity <= 0.0 {
            continue;
        }
        // Canon biomass loss `F = χ · B` (`BIOSPHERE_EQUATIONS.md` § 16.1–16.2)
        // for every hazard that strikes a place. Disease acts on the whole
        // population instead, in `apply_disease`.
        let killed = match disturbance.kind {
            DisturbanceKind::Disease => 0,
            kind => apply_exposure_mortality(
                disturbance,
                intensity,
                spec,
                elevation,
                biosphere,
                !matches!(kind, DisturbanceKind::Fire | DisturbanceKind::Drought),
            ),
        };
        let mut effect = match disturbance.kind {
            DisturbanceKind::Fire => apply_fire(disturbance, intensity, spec, weather),
            DisturbanceKind::Flood => apply_flood(disturbance, intensity, spec, hydrology, ledger),
            DisturbanceKind::VolcanicEruption => apply_thermal(
                disturbance,
                intensity,
                spec,
                elevation,
                climate,
                ledger,
                Reservoir::VolcanicHeat,
            ),
            DisturbanceKind::MeteorImpact => apply_thermal(
                disturbance,
                intensity,
                spec,
                elevation,
                climate,
                ledger,
                Reservoir::ImpactorKinetic,
            ),
            DisturbanceKind::Disease => apply_disease(disturbance, intensity, biosphere),
            DisturbanceKind::Drought => {
                apply_drought(disturbance, intensity, spec, weather, hydrology, ledger)
            }
        };
        if killed > 0 {
            effect.summary = format!("{}; killed {killed} individuals", effect.summary);
        }
        effects.push(effect);
    }

    effects
}

/// Kill the exposed share of each species: the canon disturbance loss
/// `F_j = χ_j · B_j` (`BIOSPHERE_EQUATIONS.md` § 16.1–16.2).
///
/// The exposure fraction `χ` of a species is the share of its habitat the
/// disturbance covers (affected land area over all land for terrestrial
/// species, affected ocean over all ocean for marine species, and affected
/// area over the planet for the rest), times the disturbance's intensity,
/// the fraction of the exposed population it kills. `reaches_ocean` is false
/// for hazards that only strike land (fire, drought). The dead biomass is
/// routed to detritus and the air by the biosphere carbon exchange that
/// wraps the disturbance step. Returns the individuals killed.
fn apply_exposure_mortality(
    disturbance: &ActiveDisturbance,
    intensity: f64,
    spec: &GridSpec,
    elevation: &mk_core::grid::Grid2<f64>,
    biosphere: &mut crate::biosphere::BiosphereSystem,
    reaches_ocean: bool,
) -> u64 {
    let area = |row: usize| {
        spec.cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
            .unwrap_or(0.0)
    };
    let is_land =
        |row: usize, col: usize| elevation.get_safe(row, col).copied().unwrap_or(0.0) > 0.0;
    let (mut land_total, mut ocean_total) = (0.0, 0.0);
    for row in 0..spec.nlat {
        for col in 0..spec.nlon {
            if is_land(row, col) {
                land_total += area(row);
            } else {
                ocean_total += area(row);
            }
        }
    }
    let (mut land_hit, mut ocean_hit) = (0.0, 0.0);
    for (row, col) in disturbance.affected_cells(spec) {
        if is_land(row, col) {
            land_hit += area(row);
        } else if reaches_ocean {
            ocean_hit += area(row);
        }
    }
    let share = |hit: f64, total: f64| {
        if total > 0.0 {
            (hit / total).min(1.0)
        } else {
            0.0
        }
    };
    let land_exposure = share(land_hit, land_total);
    let ocean_exposure = share(ocean_hit, ocean_total);
    let any_exposure = share(land_hit + ocean_hit, land_total + ocean_total);

    let severity = intensity.clamp(0.0, 1.0);
    let mut killed = 0u64;
    for species in &mut biosphere.species {
        let exposure = if species.is_terrestrial() {
            land_exposure
        } else if species.is_marine() {
            ocean_exposure
        } else {
            any_exposure
        };
        let removed = (species.population_size as f64 * exposure * severity).floor() as u64;
        species.population_size = species.population_size.saturating_sub(removed);
        killed += removed;
    }
    killed
}

fn apply_fire(
    disturbance: &ActiveDisturbance,
    intensity: f64,
    spec: &GridSpec,
    weather: &mut crate::weather::WeatherState,
) -> DisturbanceEffect {
    let mut water_released = 0.0;
    for (row, col) in disturbance.affected_cells(spec) {
        if let Some(moisture) = weather.moisture.get_mut_safe(row, col) {
            let removed = *moisture * FIRE_MOISTURE_FRACTION * intensity;
            *moisture -= removed;
            water_released += removed;
        }
    }
    // The fire dries the air column's moisture (`weather.moisture`), which is
    // part of the atmosphere, a water-cycle boundary reservoir: no audited
    // store changes, so there is nothing to book.
    DisturbanceEffect {
        kind: disturbance.kind,
        row: disturbance.row,
        col: disturbance.col,
        intensity,
        summary: format!("dried {water_released:.4} moisture units across burn area"),
    }
}

fn apply_flood(
    disturbance: &ActiveDisturbance,
    intensity: f64,
    spec: &GridSpec,
    hydrology: &mut crate::hydrology::HydrologyState,
    ledger: &mut Ledger,
) -> DisturbanceEffect {
    let added_mm = FLOOD_WATER_MM * intensity;
    let mut water_added_mm = 0.0;
    let (mut into_soil_kg, mut overflow_kg) = (0.0, 0.0);
    let mut cells = 0usize;
    for (row, col) in disturbance.affected_cells(spec) {
        let Some(soil) = hydrology.soil_water.get_mut_safe(row, col) else {
            continue;
        };
        // 1 mm of water over 1 m² is 1 kg.
        let area = spec
            .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
            .unwrap_or(0.0);
        // Soil takes water up to field capacity; the rest becomes standing
        // surface water, which is how the hydrology module already
        // distinguishes the two stores.
        let headroom = (SOIL_FIELD_CAPACITY_MM - soil.storage_mm).max(0.0);
        let into_soil = added_mm.min(headroom);
        soil.storage_mm += into_soil;
        soil.moisture_fraction = (soil.storage_mm / SOIL_FIELD_CAPACITY_MM).clamp(0.0, 1.0);

        let overflow = added_mm - into_soil;
        if overflow > 0.0 {
            if let Some(surface) = hydrology.surface_water.get_mut_safe(row, col) {
                *surface += overflow;
            }
        }
        into_soil_kg += into_soil * area;
        overflow_kg += overflow * area;
        water_added_mm += added_mm;
        cells += 1;
    }
    // Floodwater arrives from outside the modelled system. What the soil
    // keeps enters the soil store; the overflow stands as surface water,
    // which the hydrology step then drains downhill.
    for (sink, amount) in [
        (Reservoir::SoilWater, into_soil_kg),
        (Reservoir::SurfaceWater, overflow_kg),
    ] {
        if amount > 0.0 {
            ledger.push(FluxEntry::new(
                Reservoir::OperatorIntervention,
                sink,
                amount,
                FluxKind::Water,
            ));
        }
    }
    DisturbanceEffect {
        kind: disturbance.kind,
        row: disturbance.row,
        col: disturbance.col,
        intensity,
        summary: format!("added {water_added_mm:.3} mm of floodwater across {cells} cells"),
    }
}

fn apply_thermal(
    disturbance: &ActiveDisturbance,
    intensity: f64,
    spec: &GridSpec,
    elevation: &mk_core::grid::Grid2<f64>,
    climate: &mut crate::climate::ClimateState,
    ledger: &mut Ledger,
    source: Reservoir,
) -> DisturbanceEffect {
    let delta_k = THERMAL_DISTURBANCE_KELVIN * intensity;
    let mut cells_warmed = 0usize;
    let (mut land_j, mut ocean_j) = (0.0, 0.0);
    for (row, col) in disturbance.affected_cells(spec) {
        if let Some(temp) = climate.surface_temperature.get_mut_safe(row, col) {
            *temp += delta_k;
            cells_warmed += 1;
            // Energy that warming takes: ΔT × this cell's true area × the
            // areal heat capacity of its surface (ocean or land).
            let area = spec
                .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
                .unwrap_or(0.0);
            let height = elevation.get_safe(row, col).copied().unwrap_or(0.0);
            let joules = delta_k * area * crate::climate::surface_heat_capacity_j_m2_k(height);
            match crate::conservation::heat_reservoir(height) {
                Reservoir::OceanHeat => ocean_j += joules,
                _ => land_j += joules,
            }
        }
    }
    // The heat enters the surface layer that it warmed, from its source. The
    // climate step then radiates it away over the following ticks, and books
    // that loss against the cooling it measures.
    for (reservoir, joules) in [
        (Reservoir::SurfaceEnergy, land_j),
        (Reservoir::OceanHeat, ocean_j),
    ] {
        if joules > 0.0 {
            ledger.push(FluxEntry::new(source, reservoir, joules, FluxKind::Energy));
        }
    }
    DisturbanceEffect {
        kind: disturbance.kind,
        row: disturbance.row,
        col: disturbance.col,
        intensity,
        summary: format!("warmed {cells_warmed} cells by {delta_k:.3} K"),
    }
}

fn apply_disease(
    disturbance: &ActiveDisturbance,
    intensity: f64,
    biosphere: &mut crate::biosphere::BiosphereSystem,
) -> DisturbanceEffect {
    let mortality = DISEASE_MORTALITY_FRACTION * intensity;
    let mut removed_total = 0u64;
    for species in &mut biosphere.species {
        let removed = (species.population_size as f64 * mortality).floor() as u64;
        species.population_size = species.population_size.saturating_sub(removed);
        removed_total += removed;
    }
    DisturbanceEffect {
        kind: disturbance.kind,
        row: disturbance.row,
        col: disturbance.col,
        intensity,
        summary: format!("removed {removed_total} individuals across the biosphere"),
    }
}

fn apply_drought(
    disturbance: &ActiveDisturbance,
    intensity: f64,
    spec: &GridSpec,
    weather: &mut crate::weather::WeatherState,
    hydrology: &mut crate::hydrology::HydrologyState,
    ledger: &mut Ledger,
) -> DisturbanceEffect {
    let mut water_removed_mm = 0.0;
    let mut water_removed_kg = 0.0;
    for (row, col) in disturbance.affected_cells(spec) {
        let area = spec
            .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
            .unwrap_or(0.0);
        if let Some(precip) = weather.precipitation.get_mut_safe(row, col) {
            *precip *= 1.0 - (DROUGHT_PRECIPITATION_FRACTION * intensity).min(1.0);
        }
        if let Some(soil) = hydrology.soil_water.get_mut_safe(row, col) {
            let removed = soil.storage_mm * DROUGHT_SOIL_WATER_FRACTION * intensity;
            soil.storage_mm = (soil.storage_mm - removed).max(0.0);
            soil.moisture_fraction = (soil.storage_mm / SOIL_FIELD_CAPACITY_MM).clamp(0.0, 1.0);
            water_removed_mm += removed;
            water_removed_kg += removed * area;
        }
    }
    if water_removed_kg > 0.0 {
        // Water lost to evaporative demand leaves for the atmosphere, a
        // water-cycle boundary reservoir.
        ledger.push(FluxEntry::new(
            Reservoir::SoilWater,
            Reservoir::Atmosphere,
            water_removed_kg,
            FluxKind::Water,
        ));
    }
    DisturbanceEffect {
        kind: disturbance.kind,
        row: disturbance.row,
        col: disturbance.col,
        intensity,
        summary: format!(
            "suppressed precipitation and dried {water_removed_mm:.3} mm of soil water"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> GridSpec {
        GridSpec::new(32, 64)
    }

    fn climate(spec: &GridSpec) -> crate::climate::ClimateState {
        crate::climate::ClimateState::new(spec, 288.0)
    }

    fn hydrology(spec: &GridSpec) -> crate::hydrology::HydrologyState {
        crate::hydrology::HydrologyState::new(spec, 0.4)
    }

    fn biosphere() -> crate::biosphere::BiosphereSystem {
        let canon = std::sync::Arc::new(mk_core::canon::CanonLocked::default());
        crate::biosphere::BiosphereSystem::new(canon, 42)
    }

    #[test]
    fn intensity_decays_to_zero_at_expiry() {
        let d = ActiveDisturbance::new(DisturbanceKind::Fire, 5, 5, 1.0, 100);
        assert!((d.current_intensity(100) - 1.0).abs() < 1e-9);
        let mid = d.current_intensity(100 + DisturbanceKind::Fire.duration_ticks() / 2);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(d.current_intensity(d.expiry_tick), 0.0);
        assert_eq!(d.current_intensity(d.expiry_tick + 50), 0.0);
    }

    #[test]
    fn intensity_is_clamped_and_nan_safe() {
        assert_eq!(
            ActiveDisturbance::new(DisturbanceKind::Fire, 0, 0, 5.0, 0).intensity,
            1.0
        );
        assert_eq!(
            ActiveDisturbance::new(DisturbanceKind::Fire, 0, 0, -3.0, 0).intensity,
            0.0
        );
        assert_eq!(
            ActiveDisturbance::new(DisturbanceKind::Fire, 0, 0, f64::NAN, 0).intensity,
            0.0
        );
    }

    #[test]
    fn affected_cells_wrap_longitude_but_not_latitude() {
        let spec = spec();
        // At the prime meridian edge, the western cells wrap to the far side.
        let d = ActiveDisturbance::new(DisturbanceKind::MeteorImpact, 0, 0, 1.0, 0);
        let cells = d.affected_cells(&spec);
        assert!(cells
            .iter()
            .all(|(row, col)| spec.is_valid_index(*row, *col)));
        assert!(cells.iter().any(|(_, col)| *col == spec.nlon() - 1));
        // Row 0 is the south pole edge: nothing below it.
        assert!(cells.iter().all(|(row, _)| *row <= 3));
    }

    #[test]
    fn expired_disturbances_are_retired_and_counted() {
        let mut state = DisturbanceState::new();
        state.push(ActiveDisturbance::new(DisturbanceKind::Flood, 1, 1, 1.0, 0));
        assert_eq!(state.len(), 1);
        state.retire_expired(DisturbanceKind::Flood.duration_ticks());
        assert!(state.is_empty());
        assert_eq!(state.retired_count, 1);
    }

    #[test]
    fn flood_adds_soil_water_and_keeps_the_audit_closed() {
        let spec = spec();
        let mut hydrology = hydrology(&spec);
        let mut ledger = Ledger::new();
        let d = ActiveDisturbance::new(DisturbanceKind::Flood, 10, 10, 1.0, 0);
        let before = hydrology.soil_water.get(10, 10).storage_mm;
        let radius = mk_core::grid::CANON_PLANET_RADIUS_M;
        let soil_before = crate::conservation::soil_water_kg(&spec, radius, &hydrology.soil_water);

        let effect = apply_flood(&d, 1.0, &spec, &mut hydrology, &mut ledger);

        assert!(hydrology.soil_water.get(10, 10).storage_mm > before);
        assert!(effect.summary.contains("floodwater"));
        // The soil's measured gain is exactly what the ledger booked into it.
        let gained =
            crate::conservation::soil_water_kg(&spec, radius, &hydrology.soil_water) - soil_before;
        let booked = ledger.net_flow_by_reservoir(FluxKind::Water)[&Reservoir::SoilWater];
        assert!(gained > 0.0);
        assert!(
            (gained - booked).abs() <= 1e-9 * gained,
            "{gained} vs {booked}"
        );
    }

    #[test]
    fn fire_kills_the_exposed_share_of_land_life_and_spares_the_sea() {
        let spec = spec();
        let mut world = crate::world_integration::WorldState::new(
            std::sync::Arc::new(mk_core::canon::CanonLocked::default()),
            [7u8; 32],
        );
        world.step_world(3600).unwrap();
        // Half the planet land, half ocean, so the burn's land share is exact.
        let elevation = mk_core::grid::Grid2::from_data(
            &spec,
            (0..spec.nlat * spec.nlon)
                .map(|i| {
                    if i % spec.nlon < spec.nlon / 2 {
                        500.0
                    } else {
                        -3000.0
                    }
                })
                .collect(),
        );
        let biosphere = &mut world.biosphere_state;
        for species in &mut biosphere.species {
            species.population_size = 1_000_000;
        }
        let fire = ActiveDisturbance::new(DisturbanceKind::Fire, spec.nlat / 2, 2, 1.0, 0);
        let area = |row: usize| {
            spec.cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
                .unwrap()
        };
        let land_total: f64 = (0..spec.nlat)
            .map(|r| area(r) * (spec.nlon / 2) as f64)
            .sum();
        let burned: f64 = fire
            .affected_cells(&spec)
            .into_iter()
            .filter(|(_, c)| *c < spec.nlon / 2)
            .map(|(r, _)| area(r))
            .sum();
        let killed = apply_exposure_mortality(&fire, 0.5, &spec, &elevation, biosphere, false);
        assert!(killed > 0);
        let expected = (1_000_000.0 * burned / land_total * 0.5).floor() as u64;
        for species in &biosphere.species {
            if species.is_marine() {
                assert_eq!(
                    species.population_size, 1_000_000,
                    "fire does not reach the sea"
                );
            } else if species.is_terrestrial() {
                assert_eq!(species.population_size, 1_000_000 - expected);
            }
        }
    }

    #[test]
    fn disease_reduces_population_without_underflow() {
        let mut biosphere = biosphere();
        for species in &mut biosphere.species {
            species.population_size = 1;
        }
        let d = ActiveDisturbance::new(DisturbanceKind::Disease, 0, 0, 1.0, 0);
        // A single individual with a 2% mortality floor loses nobody, rather
        // than wrapping around on subtraction.
        let effect = apply_disease(&d, 1.0, &mut biosphere);
        assert!(biosphere.species.iter().all(|s| s.population_size == 1));
        assert!(effect.summary.contains("removed 0"));
    }

    #[test]
    fn thermal_heat_is_temperature_rise_times_true_area_times_heat_capacity() {
        let spec = spec();
        let d = ActiveDisturbance::new(DisturbanceKind::MeteorImpact, 16, 10, 1.0, 0);
        let heat_for = |height: f64| {
            let mut climate = climate(&spec);
            let mut ledger = Ledger::new();
            let elevation = mk_core::grid::Grid2::new(&spec, height);
            apply_thermal(
                &d,
                1.0,
                &spec,
                &elevation,
                &mut climate,
                &mut ledger,
                Reservoir::ImpactorKinetic,
            );
            // The heat goes into the surface store it warms, not straight
            // on to space: the climate step radiates it later.
            assert!(ledger
                .entries()
                .iter()
                .all(|e| e.sink != Reservoir::SpaceRadiation));
            ledger
                .entries()
                .iter()
                .filter(|e| e.source == Reservoir::ImpactorKinetic)
                .map(|e| e.amount)
                .sum::<f64>()
        };

        let expected_land: f64 = d
            .affected_cells(&spec)
            .into_iter()
            .map(|(row, _)| {
                THERMAL_DISTURBANCE_KELVIN
                    * spec
                        .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
                        .unwrap()
                    * crate::climate::surface_heat_capacity_j_m2_k(100.0)
            })
            .sum();
        let land = heat_for(100.0);
        let ocean = heat_for(-100.0);
        assert!((land / expected_land - 1.0).abs() < 1e-12);
        // Warming the ocean mixed layer by the same amount takes far more
        // energy than warming land.
        assert!(ocean > 50.0 * land);
    }

    #[test]
    fn step_applies_in_a_stable_order_regardless_of_insertion() {
        let spec = spec();
        let a = ActiveDisturbance::new(DisturbanceKind::Fire, 1, 1, 1.0, 0);
        let b = ActiveDisturbance::new(DisturbanceKind::Flood, 2, 2, 1.0, 0);

        let run = |first: &ActiveDisturbance, second: &ActiveDisturbance| {
            let mut state = DisturbanceState::new();
            state.push(first.clone());
            state.push(second.clone());
            let mut climate = climate(&spec);
            let mut weather = crate::weather::WeatherState::new(&spec);
            let mut hydrology = hydrology(&spec);
            let mut biosphere = biosphere();
            let mut ledger = Ledger::new();
            step_disturbances(
                &mut state,
                1,
                &spec,
                &mk_core::grid::Grid2::new(&spec, 100.0),
                &mut climate,
                &mut weather,
                &mut hydrology,
                &mut biosphere,
                &mut ledger,
            )
            .into_iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>()
        };

        assert_eq!(run(&a, &b), run(&b, &a));
    }
}
