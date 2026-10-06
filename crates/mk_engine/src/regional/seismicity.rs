//! Earthquakes from regional fault stress (Phase 1 Task 3b).
//!
//! Faults are the plate boundaries of [`super::tectonics`], one per pair of
//! plates. Each accumulates seismic moment at
//! `M0_rate = μ · L · W · v · coupling` (Brune 1968; Savage & Simpson 1997),
//! where `v` is the plates' relative speed. Each fault draws its sequence
//! of mainshock magnitudes from a truncated Gutenberg–Richter distribution
//! with b = 1 (fixtures/reference/geology `gutenberg_richter_b`) between
//! [`MIN_MAGNITUDE`] and the largest rupture the fault can host; the next
//! event nucleates when the stored moment reaches its moment (a
//! time-predictable renewal: stress loads, then releases). So the
//! long-run moment release balances tectonic loading, and the
//! magnitude–frequency follows Gutenberg–Richter.
//!
//! Rupture length, area and slip follow Wells & Coppersmith (1994), Bull.
//! Seismol. Soc. Am. 84:974. Mainshocks of Mw ≥ [`AFTERSHOCK_PARENT_MIN_MW`]
//! trigger aftershocks whose rate decays as Omori–Utsu `(c + t)^-p`
//! (p = 1.1, c = 0.05 d; Utsu et al. 1995) and whose largest is ~1.2 units
//! smaller (Båth's law).
//!
//! Every draw is keyed by fault and event index, never by the step, so the
//! catalogue does not depend on how time is divided into steps. Each fault
//! takes one 64-bit key from the world's `RngRegistry` when the fault
//! system is built; per-event draws hash (key, purpose, index) with
//! SplitMix64, which costs nanoseconds where building a ChaCha stream per
//! draw cost microseconds (a 10,000-year catalogue holds millions of
//! events).

use mk_core::grid::Grid2;
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use mk_island::{DomainLevel, IslandDomain};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};

use super::tectonics::RegionalPlate;
use crate::tectonics::{BoundaryType, PlateCell};

/// Crustal shear modulus (Pa); the standard value for moment calculations.
pub const SHEAR_MODULUS_PA: f64 = 3.0e10;
/// Smallest magnitude simulated. Events below it carry under 1% of the
/// moment for b = 1 and are not individually modelled.
pub const MIN_MAGNITUDE: f64 = 4.0;
/// Gutenberg–Richter b-value of every fault (global mean; Frohlich & Davis
/// 1993).
pub const B_VALUE: f64 = 1.0;
/// Mainshocks at or above this magnitude produce aftershock sequences.
pub const AFTERSHOCK_PARENT_MIN_MW: f64 = 5.5;
/// Båth's law gap between a mainshock and its largest aftershock.
const BATH_DELTA_M: f64 = 1.2;
/// Omori–Utsu parameters (Utsu et al. 1995).
pub const OMORI_P: f64 = 1.1;
const OMORI_C_SECONDS: f64 = 0.05 * 86_400.0;
/// Aftershocks are generated over this horizon after the mainshock.
const AFTERSHOCK_HORIZON_SECONDS: f64 = 365.25 * 86_400.0;
const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

/// RNG epochs: one stream per purpose.
const EPOCH_MAGNITUDE: u32 = 0x5345_4931;
const EPOCH_LOCATION: u32 = 0x5345_4932;
const EPOCH_DEPTH: u32 = 0x5345_4933;
const EPOCH_AFTERSHOCK: u32 = 0x5345_4934;
const EPOCH_JITTER_X: u32 = 0x5345_4935;
const EPOCH_JITTER_Y: u32 = 0x5345_4936;
/// Aftershock draw indices carry this bit; mainshock indices never do.
const AFTERSHOCK_INDEX_BIT: u64 = 1 << 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultKind {
    Thrust,
    Normal,
    StrikeSlip,
}

impl FaultKind {
    /// Seismogenic (locked) depth (km), dip (degrees) and interseismic
    /// coupling. Megathrusts lock to ~40 km on a shallow ~15° dip (Hayes et
    /// al. 2018, Slab2); continental crust is seismogenic to ~15 km;
    /// ridges and their normal faults are mostly aseismic (coupling ~0.1;
    /// Bird & Kagan 2004, Bull. Seismol. Soc. Am. 94:2380), transforms
    /// ~0.8, subduction interfaces ~0.5 on average.
    fn geometry(self, boundary: BoundaryType) -> (f64, f64, f64) {
        match (self, boundary) {
            (FaultKind::Thrust, BoundaryType::Subduction) => (40.0, 15.0, 0.5),
            (FaultKind::Thrust, _) => (20.0, 30.0, 0.6),
            (FaultKind::Normal, _) => (10.0, 60.0, 0.1),
            (FaultKind::StrikeSlip, _) => (15.0, 90.0, 0.8),
        }
    }
}

/// One fault: a plate boundary between two plates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fault {
    pub id: u32,
    pub plates: (u8, u8),
    pub boundary: BoundaryType,
    pub kind: FaultKind,
    /// Coarse cells along the fault, row-major order.
    pub trace_cells: Vec<(usize, usize)>,
    pub length_km: f64,
    pub slip_rate_mm_yr: f64,
    pub locked_depth_km: f64,
    pub dip_deg: f64,
    pub coupling: f64,
    /// Largest magnitude the whole fault can rupture in.
    pub max_magnitude_mw: f64,
    /// Shear stress stored on the locked fault (MPa), the moment deficit
    /// spread over its area and width.
    pub stress_mpa: f64,
    /// Seismic moment stored since the last event (N·m).
    pub moment_deficit_nm: f64,
    /// Mainshocks released so far (the next draw's index).
    pub events_released: u64,
    /// Moment of the next mainshock (N·m), drawn ahead.
    pub next_event_moment_nm: f64,
    pub next_event_magnitude_mw: f64,
    /// Key of this fault's deterministic draws (from the world RNG).
    pub stream_key: u64,
}

/// SplitMix64 finaliser (Steele, Lea & Flood 2014).
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Fault {
    /// A uniform draw in [0, 1) for `purpose` and `index`.
    fn uniform(&self, purpose: u32, index: u64) -> f64 {
        let word = mix(mix(self.stream_key ^ u64::from(purpose)) ^ index);
        (word >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Down-dip locked width (m).
    fn locked_width_m(&self) -> f64 {
        self.locked_depth_km * 1000.0 / self.dip_deg.to_radians().sin()
    }

    /// Seismic moment loading rate (N·m per second).
    pub fn moment_rate_nm_s(&self) -> f64 {
        SHEAR_MODULUS_PA
            * self.length_km
            * 1000.0
            * self.locked_width_m()
            * self.slip_rate_mm_yr
            * 1e-3
            * self.coupling
            / SECONDS_PER_YEAR
    }
}

/// An earthquake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Earthquake {
    pub fault_id: u32,
    /// Simulated time of the event (s).
    pub time_s: f64,
    /// Domain metres (east, north).
    pub epicentre_m: (f64, f64),
    pub depth_km: f64,
    pub magnitude_mw: f64,
    pub moment_nm: f64,
    pub slip_m: f64,
    pub rupture_length_km: f64,
    pub is_aftershock: bool,
}

/// Every fault, the clock and the queue of scheduled aftershocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaultSystem {
    pub faults: Vec<Fault>,
    pub time_s: f64,
    /// Aftershocks scheduled but not yet occurred, sorted by time.
    pub pending_aftershocks: Vec<Earthquake>,
    /// Total moment released so far (N·m).
    pub released_moment_nm: f64,
    /// Total moment loaded so far (N·m).
    pub loaded_moment_nm: f64,
}

/// Moment magnitude to seismic moment (N·m): Hanks & Kanamori (1979).
pub fn moment_from_magnitude(mw: f64) -> f64 {
    10f64.powf(1.5 * mw + 9.1)
}

/// Wells & Coppersmith (1994), all slip types: surface rupture length (km),
/// rupture area (km²) and average displacement (m) for a magnitude.
pub fn rupture_scaling(mw: f64) -> (f64, f64, f64) {
    let length_km = 10f64.powf((mw - 5.08) / 1.16);
    let area_km2 = 10f64.powf((mw - 4.07) / 0.98);
    let slip_m = 10f64.powf((mw - 6.93) / 0.82);
    (length_km, area_km2, slip_m)
}

/// A magnitude from the truncated Gutenberg–Richter distribution on
/// `[MIN_MAGNITUDE, max]`, by inverse CDF of uniform `u`.
fn gutenberg_richter(u: f64, max: f64) -> f64 {
    let beta = B_VALUE * std::f64::consts::LN_10;
    let span = (max - MIN_MAGNITUDE).max(0.0);
    let tail = (-beta * span).exp();
    MIN_MAGNITUDE - (1.0 - u * (1.0 - tail)).ln() / beta
}

/// RNG epoch of the per-fault stream keys.
const EPOCH_FAULT_KEY: u32 = 0x5345_4930;

fn draw_next(fault: &mut Fault) {
    let u = fault.uniform(EPOCH_MAGNITUDE, fault.events_released);
    let mw = gutenberg_richter(u, fault.max_magnitude_mw);
    fault.next_event_magnitude_mw = mw;
    fault.next_event_moment_nm = moment_from_magnitude(mw);
}

/// Plate pair -> (lower-id side's contact cells, cell counts per boundary
/// kind: subduction, collision, ridge, transform).
type Contacts = std::collections::BTreeMap<(u8, u8), (Vec<(usize, usize)>, [usize; 4])>;

fn boundary_kind(boundary: BoundaryType) -> FaultKind {
    match boundary {
        BoundaryType::Subduction | BoundaryType::Collision => FaultKind::Thrust,
        BoundaryType::Ridge => FaultKind::Normal,
        BoundaryType::Transform => FaultKind::StrikeSlip,
    }
}

impl FaultSystem {
    /// One fault per pair of plates that meet, typed by the dominant
    /// boundary kind along their contact, slipping at their relative speed.
    pub fn from_tectonics(
        domain: &IslandDomain,
        plates_grid: &Grid2<PlateCell>,
        plates: &[RegionalPlate],
        rng: &RngRegistry,
    ) -> Self {
        let level = DomainLevel::Coarse;
        let (rows, cols) = (domain.rows(level), domain.cols(level));
        let cell_km = domain.cell_size_m(level) / 1000.0;
        // (pair) -> (cells, counts per boundary kind)
        let mut contacts: Contacts = Contacts::new();
        for r in 0..rows {
            for c in 0..cols {
                let here = plates_grid.get(r, c);
                let Some(boundary) = here.boundary else {
                    continue;
                };
                // The neighbouring plate: the first different plate across
                // an edge (east, west, north, south order).
                let other =
                    [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)]
                        .iter()
                        .find_map(|&(dr, dc)| {
                            let (rr, cc) = (r as i64 + dr, c as i64 + dc);
                            if rr < 0 || cc < 0 || rr as usize >= rows || cc as usize >= cols {
                                return None;
                            }
                            let id = plates_grid.get(rr as usize, cc as usize).plate_id;
                            (id != here.plate_id).then_some(id)
                        });
                let Some(other) = other else { continue };
                let pair = (here.plate_id.min(other), here.plate_id.max(other));
                let entry = contacts.entry(pair).or_default();
                // Count each contact once: only the lower-id side's cells.
                if here.plate_id == pair.0 {
                    entry.0.push((r, c));
                }
                let k = match boundary {
                    BoundaryType::Subduction => 0,
                    BoundaryType::Collision => 1,
                    BoundaryType::Ridge => 2,
                    BoundaryType::Transform => 3,
                };
                entry.1[k] += 1;
            }
        }

        let velocity = |id: u8| {
            plates
                .iter()
                .find(|p| p.id == id)
                .map(|p| (p.velocity_east_m_yr, p.velocity_north_m_yr))
                .unwrap_or((0.0, 0.0))
        };
        let mut faults = Vec::new();
        for ((a, b), (cells, counts)) in contacts {
            if cells.is_empty() {
                continue;
            }
            let dominant = [
                BoundaryType::Subduction,
                BoundaryType::Collision,
                BoundaryType::Ridge,
                BoundaryType::Transform,
            ][(0..4)
                .max_by_key(|&k| (counts[k], 3 - k))
                .expect("four kinds")];
            let kind = boundary_kind(dominant);
            let (depth, dip, coupling) = kind.geometry(dominant);
            let (va, vb) = (velocity(a), velocity(b));
            let slip_rate_mm_yr = (va.0 - vb.0).hypot(va.1 - vb.1) * 1000.0;
            let length_km = cells.len() as f64 * cell_km;
            // The largest rupture spans the whole fault (Wells & Coppersmith
            // SRL relation inverted), capped at Mw 9.5.
            let max_magnitude_mw =
                (5.08 + 1.16 * length_km.log10()).clamp(MIN_MAGNITUDE + 0.5, 9.5);
            let mut fault = Fault {
                id: faults.len() as u32,
                plates: (a, b),
                boundary: dominant,
                kind,
                trace_cells: cells,
                length_km,
                slip_rate_mm_yr,
                locked_depth_km: depth,
                dip_deg: dip,
                coupling,
                max_magnitude_mw,
                stress_mpa: 0.0,
                moment_deficit_nm: 0.0,
                events_released: 0,
                next_event_moment_nm: 0.0,
                next_event_magnitude_mw: 0.0,
                stream_key: rng
                    .stream(RngKey::new(
                        SubsystemId::Tectonics,
                        faults.len() as u32,
                        EPOCH_FAULT_KEY,
                        0,
                    ))
                    .next_u64(),
            };
            draw_next(&mut fault);
            faults.push(fault);
        }
        Self {
            faults,
            time_s: 0.0,
            pending_aftershocks: Vec::new(),
            released_moment_nm: 0.0,
            loaded_moment_nm: 0.0,
        }
    }
}

/// Where along a fault an event happens, and how deep.
fn locate(domain_cell_m: f64, fault: &Fault, epoch_index: u64) -> ((f64, f64), f64) {
    let u = fault.uniform(EPOCH_LOCATION, epoch_index);
    let (r, c) = fault.trace_cells
        [((u * fault.trace_cells.len() as f64) as usize).min(fault.trace_cells.len() - 1)];
    let jx = fault.uniform(EPOCH_JITTER_X, epoch_index) - 0.5;
    let jy = fault.uniform(EPOCH_JITTER_Y, epoch_index) - 0.5;
    let x = (c as f64 + 0.5 + jx) * domain_cell_m;
    let y = (r as f64 + 0.5 + jy) * domain_cell_m;
    let ud = fault.uniform(EPOCH_DEPTH, epoch_index);
    let depth = 2.0 + ud * (fault.locked_depth_km - 2.0).max(1.0);
    ((x, y), depth)
}

fn earthquake(
    fault: &Fault,
    time_s: f64,
    mw: f64,
    at: ((f64, f64), f64),
    aftershock: bool,
) -> Earthquake {
    let (length_km, _, slip_m) = rupture_scaling(mw);
    Earthquake {
        fault_id: fault.id,
        time_s,
        epicentre_m: at.0,
        depth_km: at.1,
        magnitude_mw: mw,
        moment_nm: moment_from_magnitude(mw),
        slip_m,
        rupture_length_km: length_km.min(fault.length_km),
        is_aftershock: aftershock,
    }
}

/// Aftershocks of a mainshock: `10^(b(M - ΔM_Båth - M_min))` events of
/// Mw ≥ M_min over the horizon, at Omori–Utsu-distributed delays.
fn aftershocks(fault: &Fault, main: &Earthquake, cell_m: f64) -> Vec<Earthquake> {
    let count = 10f64
        .powf(B_VALUE * (main.magnitude_mw - BATH_DELTA_M - MIN_MAGNITUDE))
        .floor() as u64;
    let largest = main.magnitude_mw - BATH_DELTA_M;
    let base = AFTERSHOCK_INDEX_BIT | (fault.events_released << 20);
    let (c, p, horizon) = (OMORI_C_SECONDS, OMORI_P, AFTERSHOCK_HORIZON_SECONDS);
    // Inverse CDF of the Omori–Utsu density on [0, horizon].
    let a = c.powf(1.0 - p);
    let b = (c + horizon).powf(1.0 - p);
    (0..count.min(5_000))
        .map(|i| {
            let index = base + 2 * i;
            let ut = fault.uniform(EPOCH_AFTERSHOCK, index);
            let delay = (a + ut * (b - a)).powf(1.0 / (1.0 - p)) - c;
            let um = fault.uniform(EPOCH_AFTERSHOCK, index + 1);
            let mw = if i == 0 {
                largest
            } else {
                gutenberg_richter(um, largest)
            };
            let at = locate(cell_m, fault, index);
            earthquake(fault, main.time_s + delay.max(0.0), mw, at, true)
        })
        .collect()
}

/// Advance every fault by `dt_seconds`: load moment, release mainshocks
/// whose moment has been reached (each at the instant it is reached), and
/// emit aftershocks falling inside the step. Returns the events in time
/// order (ties by fault id).
pub fn step_seismicity(
    faults: &mut FaultSystem,
    domain: &IslandDomain,
    dt_seconds: f64,
) -> Vec<Earthquake> {
    let dt = dt_seconds.max(0.0);
    let start = faults.time_s;
    let end = start + dt;
    let cell_m = domain.cell_size_m(DomainLevel::Coarse);
    let mut events = Vec::new();
    let mut new_aftershocks = Vec::new();

    for fault in &mut faults.faults {
        let rate = fault.moment_rate_nm_s();
        faults.loaded_moment_nm += rate * dt;
        let mut t = start;
        let mut deficit = fault.moment_deficit_nm;
        loop {
            let needed = fault.next_event_moment_nm - deficit;
            if rate <= 0.0 {
                break;
            }
            let wait = needed.max(0.0) / rate;
            if t + wait > end {
                deficit += rate * (end - t);
                break;
            }
            t += wait;
            deficit += rate * wait - fault.next_event_moment_nm;
            let at = locate(cell_m, fault, fault.events_released);
            let main = earthquake(fault, t, fault.next_event_magnitude_mw, at, false);
            faults.released_moment_nm += main.moment_nm;
            if main.magnitude_mw >= AFTERSHOCK_PARENT_MIN_MW {
                for aftershock in aftershocks(fault, &main, cell_m) {
                    // Aftershocks relieve stress the mainshock left behind.
                    deficit -= aftershock.moment_nm;
                    faults.released_moment_nm += aftershock.moment_nm;
                    new_aftershocks.push(aftershock);
                }
            }
            events.push(main);
            fault.events_released += 1;
            draw_next(fault);
        }
        fault.moment_deficit_nm = deficit;
        let area_m2 = fault.length_km * 1000.0 * fault.locked_width_m();
        fault.stress_mpa = deficit / (area_m2 * fault.locked_width_m()) / 1e6;
    }

    faults.pending_aftershocks.extend(new_aftershocks);
    faults.pending_aftershocks.sort_by(|a, b| {
        a.time_s
            .total_cmp(&b.time_s)
            .then(a.fault_id.cmp(&b.fault_id))
    });
    let due = faults
        .pending_aftershocks
        .partition_point(|e| e.time_s <= end);
    events.extend(faults.pending_aftershocks.drain(..due));
    events.sort_by(|a, b| {
        a.time_s
            .total_cmp(&b.time_s)
            .then(a.fault_id.cmp(&b.fault_id))
    });
    faults.time_s = end;
    events
}
