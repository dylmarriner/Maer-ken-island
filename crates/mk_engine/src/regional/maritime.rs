//! Maritime air over the island (deviation D37).
//!
//! Upstream's energy balance relaxes every cell toward its own radiative
//! equilibrium, so a land cell -- with three-hundredths of the ocean's heat
//! capacity -- follows the season almost as fast as the sun moves. On a
//! continent's interior that is nearly right. On an island it is not: the
//! air over the land arrived from the sea, and it keeps the sea's
//! temperature for hundreds of kilometres, warming the winter and cooling
//! the summer of everything it crosses.
//!
//! This carries that air. Over the sea the boundary layer takes the sea's
//! surface temperature. Over land it is carried downwind and exchanges
//! sensible heat with the ground by the bulk aerodynamic formula,
//! `H = rho c_p C_H U (T_s - T_a)`, so a column of depth `h` moving at `U`
//! approaches the ground's temperature over a distance
//! `L = h / C_H` -- independent of the wind speed, which sets only how
//! quickly the air goes, not how far it gets. With the textbook mixed
//! layer and transfer coefficient below that is about 830 km: air crossing
//! an island a few hundred kilometres wide keeps most of the sea in it.
//! The ground in turn is held toward that air with a conductance
//! `rho c_p C_H U` against its radiative restoring, in
//! [`crate::climate::step_climate_on`].
//!
//! The ground is coupled to that air by sensible heat only. In the grey
//! one-layer balance the downwelling longwave also comes from the air
//! overhead, which over an island is the sea's air rather than the local
//! column; that coupling (about `2 A sigma T^3`, 2 W/m^2/K) is not added, so
//! the land still follows its own radiation a little more than it should.

use mk_core::grid::Grid2;

use super::climate::LAPSE_RATE_K_PER_M;
use crate::weather::WindVector;

/// Depth of the well-mixed boundary layer the sea air fills (m): the
/// daytime convective boundary layer is 1-2 km deep over land (Stull 1988,
/// *An Introduction to Boundary Layer Meteorology*, §1.3), and the lower
/// end is taken because the nocturnal layer is shallower.
pub const BOUNDARY_LAYER_DEPTH_M: f64 = 1_000.0;
/// Bulk transfer coefficient for sensible heat at 10 m in near-neutral
/// conditions (Garratt 1992, *The Atmospheric Boundary Layer*, §4.2: about
/// 1.0-1.5 x 10^-3 over open, moderately rough land).
pub const HEAT_TRANSFER_COEFFICIENT: f64 = 1.2e-3;
/// Specific heat of dry air at constant pressure (J/kg/K).
pub const AIR_SPECIFIC_HEAT_J_KG_K: f64 = 1_004.0;
/// Specific gas constant of dry air (J/kg/K).
pub const DRY_AIR_GAS_CONSTANT_J_KG_K: f64 = 287.05;
/// The upwind solve stops when a full round of its four sweeps moves no
/// cell's value by more than this (K for temperature; relative humidity
/// converges far inside it in absolute terms too).
const CONVERGED: f64 = 1e-9;

/// Distance (m) over which carried air approaches the ground's temperature.
pub fn adjustment_length_m() -> f64 {
    BOUNDARY_LAYER_DEPTH_M / HEAT_TRANSFER_COEFFICIENT
}

/// Sensible-heat conductance between the ground and the air above it
/// (W/m^2/K): `rho c_p C_H U`, with air density from the surface pressure
/// and the air's own temperature.
pub fn sensible_conductance_w_m2_k(pressure_pa: f64, air_k: f64, wind_m_s: f64) -> f64 {
    let density = pressure_pa / (DRY_AIR_GAS_CONSTANT_J_KG_K * air_k.max(1.0));
    density * AIR_SPECIFIC_HEAT_J_KG_K * HEAT_TRANSFER_COEFFICIENT * wind_m_s.max(0.0)
}

/// Boundary-layer air temperature (K) over every coarse cell.
///
/// `surface_k` is the surface temperature, `elevation_m` each cell's mean
/// height (land is above zero), `wind` the wind, and `cell_m` the cell
/// size; rows run northward. Sea cells take their surface temperature. A
/// land cell takes the air arriving from upwind -- the cells it blows
/// from, weighted by the wind's components -- adjusted toward its own
/// ground over the distance it crosses. Where there is no wind, the air is
/// the ground's: nothing carries the sea in.
///
/// Air lifted over high ground cools as it rises, so the carrying is done
/// in sea-level-equivalent temperature (`T + Γ h`, at the regional lapse
/// rate) and each cell's air is returned at its own height.
pub fn boundary_layer_air_k(
    surface_k: &[f64],
    elevation_m: &[f64],
    wind: &Grid2<WindVector>,
    rows: usize,
    cols: usize,
    cell_m: f64,
) -> Vec<f64> {
    let lift = |i: usize| LAPSE_RATE_K_PER_M * elevation_m[i].max(0.0);
    let ground: Vec<f64> = (0..surface_k.len())
        .map(|i| surface_k[i] + lift(i))
        .collect();
    let land: Vec<bool> = elevation_m.iter().map(|&h| h > 0.0).collect();
    let mut air = carry_from_sea(&ground, &land, wind, rows, cols, cell_m);
    for (i, a) in air.iter_mut().enumerate() {
        *a -= lift(i);
    }
    air
}

/// A property of the boundary-layer air carried from the sea along the
/// wind: `ground` holds, per cell, the sea's value over the sea and the
/// value the air would reach in equilibrium with the land over land. Each
/// land cell takes the air arriving from upwind -- the cells it blows
/// from, weighted by the wind's components -- moved toward its own ground
/// value over the distance it crosses, by `exp(-crossing / L)` with
/// `L = h / C` ([`adjustment_length_m`]). Heat and water vapour share `L`:
/// their bulk transfer coefficients are equal in near-neutral air (the
/// Reynolds analogy; Garratt 1992, §4.2). Where there is no wind, the air
/// is the ground's: nothing carries the sea in.
pub fn carry_from_sea(
    ground: &[f64],
    land: &[bool],
    wind: &Grid2<WindVector>,
    rows: usize,
    cols: usize,
    cell_m: f64,
) -> Vec<f64> {
    let length = adjustment_length_m();
    let mut air = ground.to_vec();
    // What each land cell takes from upwind is fixed for the whole solve --
    // the wind and the ground do not change while the air settles -- so the
    // neighbours, weights and the share kept across the cell are worked out
    // once rather than on every sweep.
    struct Link {
        cell: usize,
        from_col: usize,
        from_row: usize,
        u: f64,
        v: f64,
        weight: f64,
        kept: f64,
    }
    let links: Vec<Option<Link>> = (0..rows * cols)
        .map(|i| {
            if !land[i] {
                return None;
            }
            let (row, col) = (i / cols, i % cols);
            let w = *wind.get(row, col);
            let (u, v) = (w.u_east, w.v_north);
            let speed = u.hypot(v);
            let weight = u.abs() + v.abs();
            if speed <= f64::EPSILON || weight <= f64::EPSILON {
                // Calm: the cell keeps its own ground.
                return None;
            }
            // The neighbours the air comes from, clamped at the domain edge
            // (which is sea).
            let up_col = if u > 0.0 {
                col.saturating_sub(1)
            } else {
                (col + 1).min(cols - 1)
            };
            let up_row = if v > 0.0 {
                row.saturating_sub(1)
            } else {
                (row + 1).min(rows - 1)
            };
            // Path across the cell along the wind: one cell for a wind along
            // a grid axis, up to sqrt(2) on a diagonal.
            let crossing = cell_m * weight / speed;
            Some(Link {
                cell: i,
                from_col: row * cols + up_col,
                from_row: up_row * cols + col,
                u: u.abs(),
                v: v.abs(),
                weight,
                kept: (-crossing / length).exp(),
            })
        })
        .collect();
    let orders: [(bool, bool); 4] = [(false, false), (true, false), (false, true), (true, true)];
    // Fast sweeping: each round sweeps the grid in all four orderings, so
    // air moving along any quadrant of wind crosses it in one sweep; a
    // path the wind bends -- round a cyclone -- needs a round per bend.
    // Every land cell starts at its own ground, so a cell the sea's air has
    // not yet reached reads as continental: stopping early is not a small
    // error but the very one this module exists to remove (measured: two
    // rounds left the lowland year 24 K wide where a converged solve gives
    // 15). So it runs until nothing moves, and no path is longer than the
    // grid's rows plus columns.
    for _ in 0..(rows + cols).max(1) {
        let mut largest_change: f64 = 0.0;
        for (rows_back, cols_back) in orders {
            for ri in 0..rows {
                let row = if rows_back { rows - 1 - ri } else { ri };
                for ci in 0..cols {
                    let col = if cols_back { cols - 1 - ci } else { ci };
                    let Some(link) = &links[row * cols + col] else {
                        continue;
                    };
                    let i = link.cell;
                    let arriving =
                        (link.u * air[link.from_col] + link.v * air[link.from_row]) / link.weight;
                    let updated = ground[i] + (arriving - ground[i]) * link.kept;
                    largest_change = largest_change.max((updated - air[i]).abs());
                    air[i] = updated;
                }
            }
        }
        if largest_change <= CONVERGED {
            break;
        }
    }
    air
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::grid::GridSpec;

    fn westerly(rows: usize, cols: usize, u: f64) -> Grid2<WindVector> {
        Grid2::new(
            &GridSpec::new(rows, cols),
            WindVector {
                u_east: u,
                v_north: 0.0,
            },
        )
    }

    #[test]
    fn the_adjustment_length_is_the_layer_over_the_coefficient() {
        // 1,000 m / 1.2e-3 = 833 km.
        assert!((adjustment_length_m() - 833_333.333).abs() < 1.0);
        // rho c_p C_H U at 101,325 Pa and 288 K, 5 m/s: ~7.4 W/m^2/K.
        let g = sensible_conductance_w_m2_k(101_325.0, 288.15, 5.0);
        assert!((g - 7.37).abs() < 0.05, "{g}");
        assert_eq!(sensible_conductance_w_m2_k(101_325.0, 288.15, 0.0), 0.0);
    }

    #[test]
    fn sea_air_crossing_land_keeps_the_sea_and_lets_it_go_downwind() {
        // One row: two sea cells at 285 K, then 40 cells of land 12 km
        // wide at 250 K, under a westerly.
        let cols = 42;
        let mut surface = vec![250.0; cols];
        // Land a micrometre above the sea, so the lapse rate is out of it.
        let mut elevation = vec![1e-6; cols];
        for i in 0..2 {
            surface[i] = 285.0;
            elevation[i] = -100.0;
        }
        let air = boundary_layer_air_k(
            &surface,
            &elevation,
            &westerly(1, cols, 8.0),
            1,
            cols,
            12_000.0,
        );
        assert_eq!(air[0], 285.0);
        // Each 12 km cell keeps exp(-12/833) of the sea's excess, so the
        // first land cell is barely changed and the air cools steadily
        // downwind -- after 480 km it still holds exp(-480/833) of it.
        let excess = |i: usize| air[i] - 250.0;
        let first = 35.0 * (-12_000.0 / adjustment_length_m()).exp();
        assert!((excess(2) - first).abs() < 1e-6, "{}", excess(2));
        for i in 3..cols {
            assert!(air[i] < air[i - 1], "air warms downwind at {i}");
        }
        let last = 35.0 * (-480_000.0 / adjustment_length_m()).exp();
        assert!(
            (excess(cols - 1) - last).abs() < 1e-6,
            "{}",
            excess(cols - 1)
        );
    }

    #[test]
    fn the_wind_decides_which_coast_the_air_comes_from() {
        let cols = 12;
        let mut surface = vec![260.0; cols];
        let mut elevation = vec![5.0; cols];
        surface[0] = 285.0; // warm sea to the west
        surface[cols - 1] = 270.0; // cold sea to the east
        elevation[0] = -50.0;
        elevation[cols - 1] = -50.0;
        let from_west = boundary_layer_air_k(
            &surface,
            &elevation,
            &westerly(1, cols, 6.0),
            1,
            cols,
            12_000.0,
        );
        let from_east = boundary_layer_air_k(
            &surface,
            &elevation,
            &westerly(1, cols, -6.0),
            1,
            cols,
            12_000.0,
        );
        assert!(
            from_west[1] > from_east[1],
            "the west coast gets the west sea"
        );
        assert!(from_east[cols - 2] < from_west[cols - 2] + 1e-9);
        assert!(
            from_east[cols - 2] > 260.0,
            "the east coast gets the east sea"
        );
    }

    #[test]
    fn air_round_a_bend_arrives_from_the_sea_not_from_the_start_value() {
        // Sea along the south row; land above. The wind blows north up the
        // first column, then east along the top row, then south down the
        // last column -- a path with two bends, ending far from where it
        // entered. Every cell on it must carry the sea's air, adjusted only
        // by the distance it travelled.
        let (rows, cols) = (6, 6);
        let mut surface = vec![250.0; rows * cols];
        let mut elevation = vec![1e-6; rows * cols];
        for col in 0..cols {
            surface[col] = 285.0;
            elevation[col] = -50.0;
        }
        let mut wind = westerly(rows, cols, 0.0);
        for row in 1..rows {
            for col in 0..cols {
                let w = if col == 0 {
                    (0.0, 5.0)
                } else if row == rows - 1 {
                    (5.0, 0.0)
                } else if col == cols - 1 {
                    (0.0, -5.0)
                } else {
                    (5.0, 0.0)
                };
                *wind.get_mut(row, col) = WindVector {
                    u_east: w.0,
                    v_north: w.1,
                };
            }
        }
        let air = boundary_layer_air_k(&surface, &elevation, &wind, rows, cols, 12_000.0);
        // Down the last column the air has come the long way round: up five
        // cells, along five, and down to row `r` another `5 - r`. Each 12 km
        // keeps exp(-12/833) of the sea's 35 K excess -- exactly, with no
        // cell left at its 250 K starting value.
        for row in 1..rows {
            let travelled = (5 + 5 + (rows - 1 - row)) as f64 * 12_000.0;
            let want = 250.0 + 35.0 * (-travelled / adjustment_length_m()).exp();
            let a = air[row * cols + cols - 1];
            assert!((a - want).abs() < 1e-6, "row {row}: {a} against {want}");
        }
    }

    #[test]
    fn air_on_a_winding_path_is_carried_all_the_way() {
        // A serpentine: in from the sea at the south-west corner, east along
        // row 1, north, west along row 2, north, east along row 3, ... Each
        // reversal runs against some sweep order, so one round of sweeps
        // cannot carry the air to the end: this is the case that needs the
        // solve to run to convergence rather than a fixed count.
        let (rows, cols) = (8, 6);
        let mut surface = vec![250.0; rows * cols];
        let mut elevation = vec![1e-6; rows * cols];
        for col in 0..cols {
            surface[col] = 285.0;
            elevation[col] = -50.0;
        }
        let mut wind = westerly(rows, cols, 0.0);
        let mut order = Vec::new();
        for row in 1..rows {
            let east = row % 2 == 1;
            let along: Vec<usize> = if east {
                (0..cols).collect()
            } else {
                (0..cols).rev().collect()
            };
            for (k, &col) in along.iter().enumerate() {
                // The first cell of each row is entered from below.
                let w = if k == 0 {
                    (0.0, 5.0)
                } else if east {
                    (5.0, 0.0)
                } else {
                    (-5.0, 0.0)
                };
                *wind.get_mut(row, col) = WindVector {
                    u_east: w.0,
                    v_north: w.1,
                };
                order.push(row * cols + col);
            }
        }
        let air = boundary_layer_air_k(&surface, &elevation, &wind, rows, cols, 12_000.0);
        for (k, &i) in order.iter().enumerate() {
            let travelled = (k + 1) as f64 * 12_000.0;
            let want = 250.0 + 35.0 * (-travelled / adjustment_length_m()).exp();
            assert!(
                (air[i] - want).abs() < 1e-6,
                "cell {k} of the path: {} against {want}",
                air[i]
            );
        }
    }

    #[test]
    fn with_no_wind_the_air_is_the_grounds() {
        let surface = vec![285.0, 250.0, 250.0];
        let elevation = vec![-10.0, 100.0, 100.0];
        let air = boundary_layer_air_k(&surface, &elevation, &westerly(1, 3, 0.0), 1, 3, 12_000.0);
        assert_eq!(&air[1..], &[250.0, 250.0]);
    }

    #[test]
    fn air_lifted_onto_high_ground_cools_as_it_rises() {
        // Sea air at 285 K onto a 2,000 m plateau whose ground is exactly
        // the lapse rate below it: the air arrives at the ground's own
        // temperature, not the sea's.
        let surface = vec![285.0, 285.0 - 13.0];
        let elevation = vec![-10.0, 2_000.0];
        let air = boundary_layer_air_k(&surface, &elevation, &westerly(1, 2, 5.0), 1, 2, 12_000.0);
        assert!((air[1] - 272.0).abs() < 1e-9, "{}", air[1]);
    }
}
