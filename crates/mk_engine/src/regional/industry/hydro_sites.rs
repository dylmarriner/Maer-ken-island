//! Where a run-of-river hydro plant can go (Phase 4b Task 2).
//!
//! A plant's powerhouse sits on a perennial river cell and its intake on
//! the same river upstream: the water runs down a penstock laid along the
//! valley and returns to the river at the tailrace. The head is the fall of
//! the river between the two cells (medium-cell mean elevations), the
//! penstock as long as the river's path between them.
//!
//! A site *supports* a design when the plant can take its full design flow
//! at the firm flow (the intake's flow 95% of the year, RETScreen) after
//! leaving the river its environmental flow (30% of the intake's mean,
//! Tennant 1976). The search walks upstream along the main stem (the
//! upstream neighbour carrying the most water) and returns the shortest
//! penstock that supports the design: the least pipe for the power.

use mk_island::{DomainLevel, IslandDomain};
use serde::{Deserialize, Serialize};

use super::flow_duration::FlowDuration;
use super::hydro::{
    design_hydro, HydroDesign, DESIGN_HYDRAULIC_LOSS_FRACTION, ENVIRONMENTAL_FLOW_FRACTION,
    FIRM_FLOW_EXCEEDANCE, GRAVITY_M_S2, WATER_DENSITY_KG_M3,
};
use crate::regional::hydrology::Receiver;
use crate::regional::physical::RegionalPhysicalState;

/// A stage-1 plant's target output (kW): the estate's peak load of about
/// 10-15 kW with margin (Phase 4b plan defaults, owner decision).
pub const ESTATE_HYDRO_KW: f64 = 25.0;
/// A stage-2 plant's target output (kW) for a town of about 200 people
/// (Phase 4b plan defaults, owner decision).
pub const TOWN_HYDRO_KW: f64 = 2_000.0;

/// A run-of-river site and the plant designed for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydroSite {
    /// Medium cell (row, col) of the intake.
    pub intake_cell: (usize, usize),
    /// Medium cell (row, col) of the powerhouse and tailrace.
    pub powerhouse_cell: (usize, usize),
    pub head_m: f64,
    pub penstock_length_m: f64,
    pub design_flow_m3_s: f64,
    /// The intake's flow 95% of the year (m^3/s).
    pub firm_flow_m3_s: f64,
    /// The intake's mean flow (m^3/s).
    pub mean_flow_m3_s: f64,
    /// Always left in the river at the intake (m^3/s).
    pub environmental_flow_m3_s: f64,
    pub design: HydroDesign,
}

fn flow_at<'a>(
    physical: &'a RegionalPhysicalState,
    domain: &IslandDomain,
    (r, c): (usize, usize),
) -> Option<&'a FlowDuration> {
    physical
        .river_flows
        .get(r * domain.cols(DomainLevel::Medium) + c)
}

/// The upstream neighbour of `cell` on its river's main stem: of the
/// perennial cells draining into it, the one with the greatest mean flow.
fn main_stem_upstream(
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    (r, c): (usize, usize),
) -> Option<(usize, usize)> {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let here = (r * cols + c) as u32;
    let network = physical.flow_network();
    let mut best: Option<((usize, usize), f32)> = None;
    for dr in -1i64..=1 {
        for dc in -1i64..=1 {
            let (rr, cc) = (r as i64 + dr, c as i64 + dc);
            if (dr, dc) == (0, 0) || rr < 0 || cc < 0 || rr as usize >= rows || cc as usize >= cols
            {
                continue;
            }
            let cell = (rr as usize, cc as usize);
            if network.receiver_of(cell.0, cell.1) != Receiver::Cell(here) {
                continue;
            }
            if let Some(fd) = flow_at(physical, domain, cell) {
                if best.is_none_or(|(_, q)| fd.mean_m3_s > q) {
                    best = Some((cell, fd.mean_m3_s));
                }
            }
        }
    }
    best.map(|(cell, _)| cell)
}

/// The shortest-penstock site with its powerhouse at `powerhouse` that
/// supports a `target_kw` plant, walking upstream along the main stem;
/// `None` if the powerhouse cell is not on a perennial river or no intake
/// upstream supports the design.
pub fn site_for_powerhouse(
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    powerhouse: (usize, usize),
    target_kw: f64,
) -> Option<HydroSite> {
    flow_at(physical, domain, powerhouse)?;
    let medium = DomainLevel::Medium;
    let elevation = &physical.geophysics.elevation_m;
    let base = *elevation.get(powerhouse.0, powerhouse.1);
    let mut length_m = 0.0;
    let mut prev = powerhouse;
    let mut visited = 0usize;
    while let Some(intake) = main_stem_upstream(physical, domain, prev) {
        visited += 1;
        if visited > domain.rows(medium) * domain.cols(medium) {
            break;
        }
        let (x0, y0) = domain.cell_center_m(medium, prev.0, prev.1);
        let (x1, y1) = domain.cell_center_m(medium, intake.0, intake.1);
        length_m += (x1 - x0).hypot(y1 - y0);
        prev = intake;
        let head_m = elevation.get(intake.0, intake.1) - base;
        if head_m <= 0.0 {
            continue;
        }
        let fd = flow_at(physical, domain, intake)?;
        let mean = f64::from(fd.mean_m3_s);
        let environmental = ENVIRONMENTAL_FLOW_FRACTION * mean;
        let firm = fd.flow_at(FIRM_FLOW_EXCEEDANCE);
        // Even a lossless plant needs `P / (rho g H)` of flow at the rated
        // head; a reach that cannot give that is passed without a design.
        let rated_head = head_m * (1.0 - DESIGN_HYDRAULIC_LOSS_FRACTION);
        if firm - environmental
            < target_kw * 1_000.0 / (WATER_DENSITY_KG_M3 * GRAVITY_M_S2 * rated_head)
        {
            continue;
        }
        let Some(design) = design_hydro(
            target_kw,
            head_m,
            length_m,
            &fd.samples_m3_s(),
            environmental,
        ) else {
            continue;
        };
        let qd = design.turbine.design_flow_m3_s;
        if firm - environmental >= qd {
            return Some(HydroSite {
                intake_cell: intake,
                powerhouse_cell: powerhouse,
                head_m,
                penstock_length_m: length_m,
                design_flow_m3_s: qd,
                firm_flow_m3_s: firm,
                mean_flow_m3_s: mean,
                environmental_flow_m3_s: environmental,
                design,
            });
        }
    }
    None
}

/// Every medium cell downstream of `cell` along its river to the sea or the
/// domain's edge, nearest first.
pub fn downstream_of(
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    cell: (usize, usize),
) -> Vec<(usize, usize)> {
    let cols = domain.cols(DomainLevel::Medium);
    let network = physical.flow_network();
    let mut out = Vec::new();
    let mut here = cell;
    while let Receiver::Cell(next) = network.receiver_of(here.0, here.1) {
        let next = (next as usize / cols, next as usize % cols);
        if out.len() > domain.rows(DomainLevel::Medium) * cols {
            break;
        }
        out.push(next);
        here = next;
    }
    out
}

/// A site on the same river as `site` for a plant of `target_kw`: the
/// nearest powerhouse downstream of the first site's that supports it, else
/// one further up the main stem above its intake.
pub fn site_on_same_river(
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    site: &HydroSite,
    target_kw: f64,
) -> Option<HydroSite> {
    for cell in downstream_of(physical, domain, site.powerhouse_cell) {
        if let Some(s) = site_for_powerhouse(physical, domain, cell, target_kw) {
            return Some(s);
        }
    }
    let mut cell = site.intake_cell;
    while let Some(up) = main_stem_upstream(physical, domain, cell) {
        if let Some(s) = site_for_powerhouse(physical, domain, up, target_kw) {
            return Some(s);
        }
        cell = up;
    }
    None
}
