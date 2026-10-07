//! Regional ocean (Phase 2 Task 4) on the coarse grid.
//!
//! Upstream's seawater density, bulk-formula evaporation, freshwater
//! exchange and wind-driven current (`ocean::step_ocean_on`) with the
//! domain's latitudes. SST comes from the regional climate (Task 3); this
//! step then relaxes the outermost cells' **salinity** and **currents**
//! toward the ocean boundary forcing (the edge's inflow normal to it),
//! and recomputes their density. Nothing reads across an edge: upstream's
//! ocean has no advection, and the relaxation is one-sided from the edge.

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, Edge, IslandDomain, OceanBoundaryForcing};

use super::climate::coarse_latitudes;
use super::edge::relax_to_edges;
use crate::ocean::{ocean_density, step_ocean_on, OceanColumn, OceanForcing, OceanState};

/// Salinity range (PSU) an edge may force: fresh to a hypersaline sea.
const SALINITY_RANGE_PSU: (f64, f64) = (0.0, 45.0);
/// Largest current (m/s) an edge may force.
const MAX_EDGE_CURRENT_M_S: f64 = 5.0;

fn edge_values<'a>(
    boundary: &'a OceanBoundaryForcing,
    edge: Edge,
    index: usize,
    values: impl Fn(&'a mk_island::EdgeOceanForcing) -> &'a [f64],
) -> Option<f64> {
    boundary
        .edges
        .iter()
        .find(|e| e.edge == edge)
        .and_then(|e| values(e).get(index).copied())
        .filter(|v| v.is_finite())
}

/// Step the regional ocean on the coarse grid. `forcing`'s grids are
/// coarse-level and its `elevation_m` decides land (no column) and sea.
pub fn step_regional_ocean(
    canon: &CanonLocked,
    previous: &Grid2<OceanColumn>,
    forcing: &OceanForcing<'_>,
    domain: &IslandDomain,
    ocean_boundary: &OceanBoundaryForcing,
) -> OceanState {
    let spec = domain.storage_spec(DomainLevel::Coarse);
    let (rows, cols) = (spec.nlat, spec.nlon);
    let mut state = step_ocean_on(canon, previous, forcing, &coarse_latitudes(domain), &spec);

    let sea: Vec<bool> = state.columns.data().iter().map(|c| c.depth > 0.0).collect();
    let mut salinity: Vec<f64> = state.columns.data().iter().map(|c| c.salinity).collect();
    relax_to_edges(&mut salinity, rows, cols, |edge, i| {
        edge_values(ocean_boundary, edge, i, |e| &e.salinity_psu)
            .map(|v| v.clamp(SALINITY_RANGE_PSU.0, SALINITY_RANGE_PSU.1))
    });

    // Edge currents: the inflow normal to an edge, in cm/s. West and
    // south edges flow in toward +x/+y, east and north toward -x/-y.
    let inflow_cm_s = |edge: Edge, i: usize| {
        edge_values(ocean_boundary, edge, i, |e| &e.inflow_m_s)
            .map(|v| 100.0 * v.clamp(-MAX_EDGE_CURRENT_M_S, MAX_EDGE_CURRENT_M_S))
    };
    let mut u: Vec<f64> = state.currents.data().iter().map(|v| v.u_east).collect();
    let mut v: Vec<f64> = state.currents.data().iter().map(|v| v.v_north).collect();
    relax_to_edges(&mut u, rows, cols, |edge, i| match edge {
        Edge::West => inflow_cm_s(edge, i),
        Edge::East => inflow_cm_s(edge, i).map(|x| -x),
        _ => None,
    });
    relax_to_edges(&mut v, rows, cols, |edge, i| match edge {
        Edge::South => inflow_cm_s(edge, i),
        Edge::North => inflow_cm_s(edge, i).map(|x| -x),
        _ => None,
    });

    for (idx, &is_sea) in sea.iter().enumerate() {
        let (r, c) = (idx / cols, idx % cols);
        if is_sea {
            let col = state.columns.get_mut(r, c);
            col.salinity = salinity[idx];
            col.density = ocean_density(col.surface_temp, col.salinity);
            let cur = state.currents.get_mut(r, c);
            cur.u_east = u[idx];
            cur.v_north = v[idx];
        }
        // Land keeps no water column and no current.
    }
    state
}
