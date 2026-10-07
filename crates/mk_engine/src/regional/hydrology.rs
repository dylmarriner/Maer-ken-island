//! Regional hydrology (Phase 2 Task 5) on the medium grid.
//!
//! Upstream's bucket model (infiltration, evaporation, deep drainage,
//! hillslope drainage: `hydrology::infiltration_capacity`,
//! `potential_evaporation` and the constants, unchanged) with the island's
//! geometry: flat cell area, `cell_size_m` for slopes (upstream hard-codes
//! 50 km), the domain's latitudes for humidity, and **no longitude wrap**.
//!
//! Upstream moves water one cell per step, so on 2 km cells a 150 km river
//! would take ~75 days to reach the sea. Here each cell's hillslope runoff
//! enters a **channel network** (a [`FlowNetwork`], built once per terrain)
//! and is delivered downstream within the step, as real rivers (~1 m/s,
//! 86 km/day) are fast against a daily step. The network fills
//! depressions (priority flood) so noise pits drain; a filled depression
//! deeper than [`LAKE_MIN_DEPTH_M`] is a **lake** that stores water up to
//! its spill level and spills the rest downstream. Water leaving into the
//! sea or off a domain edge is `HydrologyBudget::surface_to_ocean_kg`.
//! Only land cells (~6% of the grid) are iterated, so a step costs
//! milliseconds, not the ~7 s a dense 1.15 M-cell pass would.
//!
//! The coarse climate and weather are sampled at each land cell by
//! bilinear interpolation (`levels::sample_coarse_at_medium`); no medium
//! copies are built.

use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

use super::levels::sample_coarse_at_medium;
use crate::climate::ClimateState;
use crate::hydrology::{
    infiltration_capacity, potential_evaporation, HydrologyBudget, HydrologyState, SoilWater,
    DEEP_DRAINAGE_PER_DAY, FIELD_CAPACITY_MM, SURFACE_DRAINAGE_PER_DAY,
    UNLIMITED_EVAPORATION_MOISTURE,
};
use crate::weather::{relative_humidity, WeatherState};

const SECONDS_PER_DAY: f64 = 86_400.0;
/// Smallest filled depression (m) that counts as a lake; shallower noise
/// pits are drained straight through.
pub const LAKE_MIN_DEPTH_M: f64 = 0.5;
/// Strictly descending slope (m per cell) the priority flood adds so
/// every filled cell has a lower neighbour.
const FILL_EPSILON_M: f64 = 1.0e-4;
/// Discharge (m³/s) above which a channel cell counts as a river.
pub const RIVER_MIN_DISCHARGE_M3_S: f64 = 1.0;

/// Where a land cell's channel water goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Receiver {
    /// Another land cell (flat index).
    Cell(u32),
    /// The sea (a neighbouring cell at or below sea level).
    Sea,
    /// Off the edge of the domain.
    Edge,
    /// Not a land cell.
    None,
}

/// The drainage network of one terrain: built once per terrain, reused
/// every step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNetwork {
    rows: usize,
    cols: usize,
    /// Land cells from upstream to downstream (flat indices).
    order: Vec<u32>,
    receiver: Vec<Receiver>,
    /// Depth (m) of the filled depression a cell lies in, 0 outside lakes.
    lake_depth_m: Vec<f32>,
}

/// A min-heap key: elevation then index, totally ordered.
#[derive(PartialEq)]
struct Level(f64, u32);
impl Eq for Level {}
impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Level {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0).then(self.1.cmp(&other.1))
    }
}

const NEIGHBOURS: [(i64, i64); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

/// The lowest of a cell's 8 neighbours, if lower than the cell. Domain
/// edges are edges: nothing wraps.
pub fn regional_downhill_neighbour(
    topography: &Grid2<f64>,
    row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    let (rows, cols) = (topography.nlat() as i64, topography.nlon() as i64);
    let here = *topography.get(row, col);
    let mut best: Option<((usize, usize), f64)> = None;
    for (dr, dc) in NEIGHBOURS {
        let (r, c) = (row as i64 + dr, col as i64 + dc);
        if r < 0 || c < 0 || r >= rows || c >= cols {
            continue;
        }
        let h = *topography.get(r as usize, c as usize);
        if h < here && best.is_none_or(|(_, lowest)| h < lowest) {
            best = Some(((r as usize, c as usize), h));
        }
    }
    best.map(|(cell, _)| cell)
}

impl FlowNetwork {
    /// Build the network of `topography` (m, sea level 0): fill depressions
    /// by priority flood from the coast and the domain edge, then point
    /// every land cell at its steepest-descent neighbour on the filled
    /// surface.
    pub fn build(topography: &Grid2<f64>) -> Self {
        let (rows, cols) = (topography.nlat(), topography.nlon());
        let n = rows * cols;
        let elev = topography.data();
        let land = |i: usize| elev[i] > 0.0;
        let index = |r: i64, c: i64| -> Option<usize> {
            (r >= 0 && c >= 0 && (r as usize) < rows && (c as usize) < cols)
                .then(|| r as usize * cols + c as usize)
        };

        let mut filled = elev.to_vec();
        let mut closed = vec![false; n];
        let mut heap = BinaryHeap::new();
        // Seeds: land cells on the domain edge or next to the sea: water
        // leaves the network there.
        for i in (0..n).filter(|&i| land(i)) {
            let (r, c) = ((i / cols) as i64, (i % cols) as i64);
            let on_edge = r == 0 || c == 0 || r + 1 == rows as i64 || c + 1 == cols as i64;
            let by_sea = NEIGHBOURS
                .iter()
                .filter_map(|&(dr, dc)| index(r + dr, c + dc))
                .any(|j| !land(j));
            if on_edge || by_sea {
                closed[i] = true;
                heap.push(Reverse(Level(elev[i], i as u32)));
            }
        }
        while let Some(Reverse(Level(level, i))) = heap.pop() {
            let (r, c) = ((i as usize / cols) as i64, (i as usize % cols) as i64);
            for (dr, dc) in NEIGHBOURS {
                let Some(j) = index(r + dr, c + dc) else {
                    continue;
                };
                if closed[j] || !land(j) {
                    continue;
                }
                closed[j] = true;
                filled[j] = elev[j].max(level + FILL_EPSILON_M);
                heap.push(Reverse(Level(filled[j], j as u32)));
            }
        }

        // Receivers: steepest descent on the filled surface; the sea
        // counts at its own (negative) elevation, the edge as a sink for
        // an edge cell with nothing lower.
        let mut receiver = vec![Receiver::None; n];
        let mut lake_depth_m = vec![0.0_f32; n];
        let mut order: Vec<u32> = Vec::new();
        for i in (0..n).filter(|&i| land(i)) {
            let (r, c) = ((i / cols) as i64, (i % cols) as i64);
            let mut best: Option<(Receiver, f64)> = None;
            for (dr, dc) in NEIGHBOURS {
                let Some(j) = index(r + dr, c + dc) else {
                    continue;
                };
                let dist = if dr != 0 && dc != 0 {
                    std::f64::consts::SQRT_2
                } else {
                    1.0
                };
                let level = if land(j) { filled[j] } else { elev[j].min(0.0) };
                let drop = (filled[i] - level) / dist;
                if level < filled[i] && best.is_none_or(|(_, d)| drop > d) {
                    let to = if land(j) {
                        Receiver::Cell(j as u32)
                    } else {
                        Receiver::Sea
                    };
                    best = Some((to, drop));
                }
            }
            receiver[i] = best.map_or(Receiver::Edge, |(to, _)| to);
            let depth = filled[i] - elev[i];
            if depth >= LAKE_MIN_DEPTH_M {
                lake_depth_m[i] = depth as f32;
            }
            order.push(i as u32);
        }
        order.sort_by(|&a, &b| {
            filled[b as usize]
                .total_cmp(&filled[a as usize])
                .then(a.cmp(&b))
        });
        Self {
            rows,
            cols,
            order,
            receiver,
            lake_depth_m,
        }
    }

    pub fn land_cells(&self) -> usize {
        self.order.len()
    }

    pub fn receiver_of(&self, row: usize, col: usize) -> Receiver {
        self.receiver[row * self.cols + col]
    }

    /// Depth of the lake a cell lies in (m), 0 if none.
    pub fn lake_depth_m(&self, row: usize, col: usize) -> f64 {
        f64::from(self.lake_depth_m[row * self.cols + col])
    }

    /// Land cells whose water leaves the domain (into the sea or off an
    /// edge): `(flat index, to_sea)`.
    pub fn outlets(&self) -> impl Iterator<Item = (usize, bool)> + '_ {
        self.order
            .iter()
            .map(|&i| i as usize)
            .filter_map(|i| match self.receiver[i] {
                Receiver::Sea => Some((i, true)),
                Receiver::Edge => Some((i, false)),
                _ => None,
            })
    }
}

/// A hydrology state for `topography` with half-full soils and **full
/// lakes** (the long-run state: lakes fill until they spill).
pub fn bootstrap_regional_hydrology(
    network: &FlowNetwork,
    domain: &IslandDomain,
) -> HydrologyState {
    let spec = domain.storage_spec(DomainLevel::Medium);
    let mut state = HydrologyState::new(&spec, 0.5);
    for &i in &network.order {
        let depth = f64::from(network.lake_depth_m[i as usize]);
        if depth > 0.0 {
            let (r, c) = (i as usize / network.cols, i as usize % network.cols);
            state.surface_water.set(r, c, depth * 1_000.0);
        }
    }
    state
}

/// Step the regional hydrology over `dt_seconds`, building the network of
/// `topography` first. The coupled tick builds the network once per
/// terrain and calls [`step_regional_hydrology_on`].
pub fn step_regional_hydrology(
    canon: &CanonLocked,
    previous: &HydrologyState,
    weather: &WeatherState,
    climate: &ClimateState,
    topography: &Grid2<f64>,
    domain: &IslandDomain,
    dt_seconds: f64,
) -> HydrologyState {
    let network = FlowNetwork::build(topography);
    step_regional_hydrology_on(
        canon, previous, weather, climate, topography, domain, &network, dt_seconds,
    )
}

/// [`step_regional_hydrology`] on a prebuilt `network`. `weather` and
/// `climate` are **coarse** and sampled at each land cell. A zero or
/// negative step returns `previous` with an empty budget.
#[allow(clippy::too_many_arguments)]
pub fn step_regional_hydrology_on(
    canon: &CanonLocked,
    previous: &HydrologyState,
    weather: &WeatherState,
    climate: &ClimateState,
    topography: &Grid2<f64>,
    domain: &IslandDomain,
    network: &FlowNetwork,
    dt_seconds: f64,
) -> HydrologyState {
    if !(dt_seconds.is_finite() && dt_seconds > 0.0) {
        return HydrologyState {
            budget: HydrologyBudget::default(),
            ..previous.clone()
        };
    }
    let medium = DomainLevel::Medium;
    let (rows, cols) = (network.rows, network.cols);
    let size = domain.cell_size_m(medium);
    let cell_m2 = domain.cell_area_m2(medium);
    let dt_days = dt_seconds / SECONDS_PER_DAY;
    let elev = topography.data();

    let mut soil_water = previous.soil_water.clone();
    let mut surface_water = previous.surface_water.clone();
    let mut runoff = Grid2::new(&domain.storage_spec(medium), 0.0);
    let mut generated_runoff = Grid2::new(&domain.storage_spec(medium), 0.0);
    let mut infiltration = Grid2::new(&domain.storage_spec(medium), 0.0);
    let mut evaporation = Grid2::new(&domain.storage_spec(medium), 0.0);
    let mut budget = HydrologyBudget::default();

    // Cells that were land and are now sea: their soil is held saturated
    // and their standing water joins the ocean.
    for (i, &height) in elev.iter().enumerate().take(rows * cols) {
        if height > 0.0 {
            continue;
        }
        let (r, c) = (i / cols, i % cols);
        let soil = *soil_water.get(r, c);
        let standing = surface_water.get(r, c).max(0.0);
        if soil.storage_mm != FIELD_CAPACITY_MM || standing != 0.0 {
            budget.ocean_to_sea_soil_kg += (FIELD_CAPACITY_MM - soil.storage_mm) * cell_m2;
            budget.surface_to_ocean_kg += standing * cell_m2;
            soil_water.set(r, c, SoilWater::new(1.0));
            surface_water.set(r, c, 0.0);
        }
    }

    // Channel water (mm over a cell) arriving at each land cell from
    // upstream within this step.
    let mut channel_in = vec![0.0_f64; rows * cols];
    let coarse_get = |f: &dyn Fn(usize, usize) -> f64, r: usize, c: usize| {
        sample_coarse_at_medium(f, domain, r, c)
    };
    for &i in &network.order {
        let i = i as usize;
        let (row, col) = (i / cols, i % cols);
        let elevation = elev[i];
        let prior_soil = *soil_water.get(row, col);
        let precip = coarse_get(&|r, c| weather.precipitation.get(r, c).max(0.0), row, col);
        let temp = coarse_get(&|r, c| *climate.surface_temperature.get(r, c), row, col);
        let wind = {
            let u = coarse_get(&|r, c| weather.wind.get(r, c).u_east, row, col);
            let v = coarse_get(&|r, c| weather.wind.get(r, c).v_north, row, col);
            u.hypot(v)
        };
        let rh = relative_humidity(domain.latitude_rad_for_row(medium, row), false);

        let to = network.receiver[i];
        let slope = match to {
            Receiver::Cell(j) => {
                let j = j as usize;
                let dist = if (j / cols != row) && (j % cols != col) {
                    size * std::f64::consts::SQRT_2
                } else {
                    size
                };
                ((elevation - elev[j].max(0.0)) / dist).max(0.0)
            }
            Receiver::Sea | Receiver::Edge => (elevation / size).max(0.0),
            Receiver::None => 0.0,
        };

        let mut storage = prior_soil.storage_mm.max(0.0);
        let excess = (storage - FIELD_CAPACITY_MM).max(0.0);
        storage -= excess;
        let lake_cap_mm = f64::from(network.lake_depth_m[i]) * 1_000.0;
        let is_lake = lake_cap_mm > 0.0;
        let mut standing = surface_water.get(row, col).max(0.0)
            + excess
            + precip * dt_days
            // A lake receives the river water that reaches it; elsewhere
            // river water passes through in the channel.
            + if is_lake { channel_in[i] } else { 0.0 };

        let capacity = infiltration_capacity(storage / FIELD_CAPACITY_MM, slope) * dt_days;
        let infiltrated = standing
            .min(capacity)
            .min(FIELD_CAPACITY_MM - storage)
            .max(0.0);
        standing -= infiltrated;
        storage += infiltrated;

        let potential =
            potential_evaporation(temp, wind, rh, canon.sea_level_pressure_pa) * dt_days;
        let from_surface = standing.min(potential);
        standing -= from_surface;
        let moisture_limit =
            (storage / (UNLIMITED_EVAPORATION_MOISTURE * FIELD_CAPACITY_MM)).min(1.0);
        let from_soil = ((potential - from_surface) * moisture_limit)
            .min(storage)
            .max(0.0);
        storage -= from_soil;
        let drained = storage * (1.0 - (-DEEP_DRAINAGE_PER_DAY * dt_days).exp());
        storage -= drained;

        // Hillslope drainage into the channel; a lake keeps water up to
        // its spill level and releases the rest.
        let hillslope = if is_lake {
            (standing - lake_cap_mm).max(0.0)
        } else {
            let rate = SURFACE_DRAINAGE_PER_DAY * (1.0 + 50.0 * slope);
            standing * (1.0 - (-rate * dt_days).exp())
        };
        standing -= hillslope;
        let discharge = hillslope + if is_lake { 0.0 } else { channel_in[i] };

        match to {
            Receiver::Cell(j) => channel_in[j as usize] += discharge,
            Receiver::Sea | Receiver::Edge => budget.surface_to_ocean_kg += discharge * cell_m2,
            Receiver::None => {}
        }

        soil_water.set(
            row,
            col,
            SoilWater {
                moisture_fraction: (storage / FIELD_CAPACITY_MM).clamp(0.0, 1.0),
                storage_mm: storage,
            },
        );
        surface_water.set(row, col, standing);
        budget.precipitation_kg += precip * dt_days * cell_m2;
        budget.infiltration_kg += infiltrated * cell_m2;
        budget.soil_excess_kg += excess * cell_m2;
        budget.surface_evaporation_kg += from_surface * cell_m2;
        budget.soil_evaporation_kg += from_soil * cell_m2;
        budget.deep_drainage_kg += drained * cell_m2;
        infiltration.set(row, col, infiltrated / dt_days);
        evaporation.set(row, col, (from_surface + from_soil) / dt_days);
        generated_runoff.set(row, col, hillslope / dt_days);
        runoff.set(row, col, discharge / dt_days);
    }

    HydrologyState {
        soil_water,
        runoff,
        generated_runoff: Some(generated_runoff),
        infiltration,
        evaporation,
        surface_water,
        routed_outflow_mm: Vec::new(),
        budget,
    }
}

/// River discharge (m³/s) through a cell, from the last step's `runoff`
/// (mm/day over the cell).
pub fn discharge_m3_s(
    state: &HydrologyState,
    domain: &IslandDomain,
    row: usize,
    col: usize,
) -> f64 {
    state.runoff.get(row, col) / 1_000.0 * domain.cell_area_m2(DomainLevel::Medium)
        / SECONDS_PER_DAY
}

/// The fresh water (kg) the last step delivered to each coarse ocean cell
/// at the network's outlets: the sum, not the mean, of the medium cells'
/// discharge into it, so the coarse ocean gets exactly what the land lost
/// (`budget.surface_to_ocean_kg`, less any water from newly flooded
/// cells).
pub fn freshwater_to_coarse_ocean_kg(
    state: &HydrologyState,
    network: &FlowNetwork,
    domain: &IslandDomain,
    dt_seconds: f64,
) -> Grid2<f64> {
    let coarse = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(coarse), domain.cols(coarse));
    let k = (domain.cell_size_m(coarse) / domain.cell_size_m(DomainLevel::Medium)).round() as usize;
    let cell_m2 = domain.cell_area_m2(DomainLevel::Medium);
    let dt_days = dt_seconds / SECONDS_PER_DAY;
    let mut out = vec![0.0; rows * cols];
    for (i, _) in network.outlets() {
        let (r, c) = (i / network.cols, i % network.cols);
        let kg = state.runoff.get(r, c) * dt_days * cell_m2;
        out[(r / k).min(rows - 1) * cols + (c / k).min(cols - 1)] += kg;
    }
    Grid2::from_data(&domain.storage_spec(coarse), out)
}
