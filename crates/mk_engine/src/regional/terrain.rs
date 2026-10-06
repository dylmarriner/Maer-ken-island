//! Raw island terrain from geology (Phase 1 Task 4, Step 2).
//!
//! Elevation is built from the tectonic and volcanic state, not from a
//! template or a real coastline:
//!
//! - **Base:** continental plates are a submerged continental plateau
//!   (Zealandia, the continent New Zealand stands on, is 94% under water at
//!   ~1 km depth; Mortimer et al. 2017, GSA Today 27(3)); oceanic plates
//!   lie at the age–depth of their seafloor (`seafloor_depth_m`).
//! - **Convergent margins:** across a subduction margin the downgoing plate
//!   has a trench and outer rise; the overriding plate a forearc and a main
//!   range ~70 km inland of the plate boundary; collisions raise both
//!   sides. Range height varies along strike (Southern Alps ~3 km, North
//!   Island axial ranges ~1.5 km).
//! - **Volcanoes:** stratovolcanoes on the overriding plate 100–200 km
//!   behind the trench (arc–trench gaps; Gill 1981), and in continental
//!   rifts; a back-arc rift subsides (the Taupō Volcanic Zone graben).
//! - **Texture:** domain-warped multi-octave noise gives headlands, bays,
//!   inlets and secondary ranges; a box smoothing stands in for erosion;
//!   ridged noise scaled by uplift adds valleys to the mountains.
//!
//! Every random value hashes the terrain key with lattice or cell indices
//! (SplitMix64), so terrain is a pure function of its inputs and
//! independent of the thread count.

use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

use super::par::par_map_cells;
use super::shape::chamfer_distance;
use super::tectonics::RegionalPlate;
use crate::tectonics::{seafloor_depth_m, BoundaryType, PlateType, TectonicsState};
use crate::volcanism::VolcanismState;

/// Depth of submerged continental crust away from margins (m).
const CONTINENTAL_PLATEAU_M: f64 = -1_400.0;
/// Mean main-range uplift of a convergent margin before noise (m).
const RANGE_UPLIFT_M: f64 = 2_700.0;
/// Distance of the main range from the plate boundary, and its half-width
/// (km): the Southern Alps crest lies ~20-40 km east of the Alpine Fault,
/// the North Island axial ranges ~100 km from the Hikurangi trench.
const RANGE_OFFSET_KM: f64 = 60.0;
const RANGE_HALF_WIDTH_KM: f64 = 60.0;
/// Trench depth below the surrounding seafloor and its distance seaward of
/// the boundary (km).
const TRENCH_DEPTH_M: f64 = 2_500.0;
const TRENCH_OFFSET_KM: f64 = 15.0;
/// Arc–trench gap (km) where arc volcanoes stand.
const ARC_GAP_KM: (f64, f64) = (100.0, 200.0);
/// Share of eligible coarse cells holding a volcano.
const VOLCANO_PROBABILITY: f64 = 0.10;
/// Stratovolcano relief (m) and basal radius (km): Ruapehu ~2 km relief on
/// a ~15 km radius, Taranaki 2.5 km on ~20 km.
const VOLCANO_RELIEF_M: (f64, f64) = (800.0, 2_400.0);
const VOLCANO_RADIUS_KM: (f64, f64) = (10.0, 22.0);
/// Back-arc rift graben subsidence (m) and half-width (km).
const RIFT_SUBSIDENCE_M: f64 = 500.0;
const RIFT_HALF_WIDTH_KM: f64 = 30.0;
/// Large-scale relief noise: amplitude (m) and longest wavelength (km).
const NOISE_AMPLITUDE_M: f64 = 750.0;
const NOISE_WAVELENGTH_KM: f64 = 420.0;
const NOISE_OCTAVES: u32 = 7;
/// Domain warp (km) that bends coasts into headlands and bays.
const WARP_KM: f64 = 70.0;
/// Mountain valley texture amplitude at full uplift (m).
const RIDGE_TEXTURE_M: f64 = 700.0;
/// Land area the horizontal length scales above are set for (m²): the
/// NZ-scale island. A smaller target (the functional-test island) shrinks
/// every landform length by √(target / reference), so it has the same
/// shape vocabulary at a smaller size.
const REFERENCE_LAND_AREA_M2: f64 = 268_000.0e6;
/// Width (fraction of the smaller domain side) inside the ocean buffer over
/// which terrain is drawn down toward the abyss, so coasts end naturally
/// instead of against the buffer's straight edge.
const EDGE_TAPER_FRACTION: f64 = 0.15;
/// Depth (m) the taper reaches at the buffer's inner edge.
const EDGE_TAPER_DEPTH_M: f64 = 4_000.0;

/// SplitMix64 finaliser.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The terrain key for a world seed.
pub fn terrain_key(seed: &[u8; 32]) -> u64 {
    let digest = blake3::hash(&[b"mk-island-terrain".as_slice(), seed].concat());
    u64::from_le_bytes(digest.as_bytes()[..8].try_into().expect("8 bytes"))
}

/// Uniform in [0, 1) for a key and two indices.
fn hash01(key: u64, a: u64, b: u64) -> f64 {
    (mix(mix(key ^ a) ^ b) >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth value noise in [-1, 1] at lattice coordinates (x, y).
fn value_noise(key: u64, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v) = (fade(fx), fade(fy));
    let corner = |ix: f64, iy: f64| 2.0 * hash01(key, ix as i64 as u64, iy as i64 as u64) - 1.0;
    let a = corner(x0, y0);
    let b = corner(x0 + 1.0, y0);
    let c = corner(x0, y0 + 1.0);
    let d = corner(x0 + 1.0, y0 + 1.0);
    let top = a + (b - a) * u;
    let bottom = c + (d - c) * u;
    top + (bottom - top) * v
}

/// Fractal Brownian motion in about [-1, 1].
fn fbm(key: u64, x_m: f64, y_m: f64, wavelength_m: f64, octaves: u32) -> f64 {
    let (mut sum, mut norm, mut amp, mut freq) = (0.0, 0.0, 1.0, 1.0 / wavelength_m);
    for o in 0..octaves {
        let k = mix(key ^ (u64::from(o) << 56));
        sum += amp * value_noise(k, x_m * freq, y_m * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Ridged multifractal in [0, 1]: sharp crests and V-shaped valleys.
fn ridged(key: u64, x_m: f64, y_m: f64, wavelength_m: f64, octaves: u32) -> f64 {
    let (mut sum, mut norm, mut amp, mut freq) = (0.0, 0.0, 1.0, 1.0 / wavelength_m);
    for o in 0..octaves {
        let k = mix(key ^ (u64::from(o) << 48) ^ 0xA1);
        let n = 1.0 - value_noise(k, x_m * freq, y_m * freq).abs();
        sum += amp * n * n;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// How a plate behaves at convergent margins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarginRole {
    None,
    Overriding,
    Downgoing,
    Colliding,
}

/// Each plate's role at the convergent contacts it takes part in.
/// Overriding plates are continental (or, ocean against ocean, the older
/// plate subducts).
fn margin_roles(tectonics: &TectonicsState, plates: &[RegionalPlate]) -> Vec<MarginRole> {
    let mut roles = vec![MarginRole::None; plates.len()];
    let grid = &tectonics.plates;
    let (rows, cols) = (grid.nlat(), grid.nlon());
    let by_id = |id: u8| plates.iter().position(|p| p.id == id);
    for r in 0..rows {
        for c in 0..cols {
            let here = grid.get(r, c);
            let kind = match here.boundary {
                Some(k @ (BoundaryType::Subduction | BoundaryType::Collision)) => k,
                _ => continue,
            };
            for (dr, dc) in [(0i64, 1i64), (1, 0)] {
                let (rr, cc) = (r as i64 + dr, c as i64 + dc);
                if rr as usize >= rows || cc as usize >= cols {
                    continue;
                }
                let other = grid.get(rr as usize, cc as usize);
                if other.plate_id == here.plate_id {
                    continue;
                }
                let (Some(a), Some(b)) = (by_id(here.plate_id), by_id(other.plate_id)) else {
                    continue;
                };
                let (pa, pb) = (&plates[a], &plates[b]);
                if kind == BoundaryType::Collision
                    || (pa.crust == PlateType::Continental && pb.crust == PlateType::Continental)
                {
                    for i in [a, b] {
                        if roles[i] == MarginRole::None {
                            roles[i] = MarginRole::Colliding;
                        }
                    }
                    continue;
                }
                let a_overrides = match (pa.crust, pb.crust) {
                    (PlateType::Continental, PlateType::Oceanic) => true,
                    (PlateType::Oceanic, PlateType::Continental) => false,
                    _ => pa.age_ma < pb.age_ma,
                };
                let (over, down) = if a_overrides { (a, b) } else { (b, a) };
                // Overriding wins over any other role: the range belongs to
                // the plate that stays on top.
                roles[over] = MarginRole::Overriding;
                if roles[down] == MarginRole::None {
                    roles[down] = MarginRole::Downgoing;
                }
            }
        }
    }
    roles
}

/// Bilinear sample of a coarse field at domain metres.
fn bilinear(field: &[f64], rows: usize, cols: usize, cell_m: f64, x: f64, y: f64) -> f64 {
    let gx = (x / cell_m - 0.5).clamp(0.0, (cols - 1) as f64);
    let gy = (y / cell_m - 0.5).clamp(0.0, (rows - 1) as f64);
    let (c0, r0) = (gx.floor() as usize, gy.floor() as usize);
    let (c1, r1) = ((c0 + 1).min(cols - 1), (r0 + 1).min(rows - 1));
    let (fx, fy) = (gx - c0 as f64, gy - r0 as f64);
    let at = |r: usize, c: usize| field[r * cols + c];
    let top = at(r0, c0) + (at(r0, c1) - at(r0, c0)) * fx;
    let bottom = at(r1, c0) + (at(r1, c1) - at(r1, c0)) * fx;
    top + (bottom - top) * fy
}

/// 3×3 box smoothing, `passes` times (edges replicate).
fn smooth(field: &mut [f64], rows: usize, cols: usize, passes: usize) {
    let mut next = vec![0.0; field.len()];
    for _ in 0..passes {
        for r in 0..rows {
            for c in 0..cols {
                let mut sum = 0.0;
                for dr in -1i64..=1 {
                    for dc in -1i64..=1 {
                        let rr = (r as i64 + dr).clamp(0, rows as i64 - 1) as usize;
                        let cc = (c as i64 + dc).clamp(0, cols as i64 - 1) as usize;
                        sum += field[rr * cols + cc];
                    }
                }
                next[r * cols + c] = sum / 9.0;
            }
        }
        field.copy_from_slice(&next);
    }
}

/// A volcano: centre (m), relief (m) and basal radius (m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volcano {
    pub x_m: f64,
    pub y_m: f64,
    pub relief_m: f64,
    pub radius_m: f64,
}

/// Raw (unfitted) elevation on the medium grid, in metres relative to an
/// arbitrary datum; [`super::geophysics::fit_sea_level_to_target`] sets
/// sea level. Also returns the volcanoes placed.
pub fn raw_elevation(
    domain: &IslandDomain,
    tectonics: &TectonicsState,
    volcanism: &VolcanismState,
    plates: &[RegionalPlate],
    key: u64,
) -> (Grid2<f64>, Vec<Volcano>) {
    let coarse = DomainLevel::Coarse;
    let (cr, cc) = (domain.rows(coarse), domain.cols(coarse));
    let coarse_m = domain.cell_size_m(coarse);
    let grid = &tectonics.plates;

    // Coarse base depth, smoothed so plate edges are not cliffs.
    let mut base: Vec<f64> = grid
        .data()
        .iter()
        .map(|cell| match cell.crust_type {
            PlateType::Continental => CONTINENTAL_PLATEAU_M,
            PlateType::Oceanic => -seafloor_depth_m(cell.age_ma),
        })
        .collect();
    smooth(&mut base, cr, cc, 3);

    // Distances (km) to convergent and to rift (ridge in continental
    // crust) boundary cells, on the coarse grid.
    let convergent: Vec<bool> = grid
        .data()
        .iter()
        .map(|c| {
            matches!(
                c.boundary,
                Some(BoundaryType::Subduction | BoundaryType::Collision)
            )
        })
        .collect();
    let rift: Vec<bool> = grid
        .data()
        .iter()
        .map(|c| c.boundary == Some(BoundaryType::Ridge) && c.crust_type == PlateType::Continental)
        .collect();
    let to_margin: Vec<f64> = chamfer_distance(&convergent, cr, cc, false)
        .iter()
        .map(|d| d * coarse_m / 1000.0)
        .collect();
    let to_rift: Vec<f64> = chamfer_distance(&rift, cr, cc, false)
        .iter()
        .map(|d| d * coarse_m / 1000.0)
        .collect();
    let roles = margin_roles(tectonics, plates);
    let role_of = |id: u8| {
        plates
            .iter()
            .position(|p| p.id == id)
            .map_or(MarginRole::None, |i| roles[i])
    };

    let scale = (domain.profile().target_land_area_m2 / REFERENCE_LAND_AREA_M2).sqrt();
    // Volcanoes: arc volcanoes on overriding plates in the arc–trench gap,
    // rift volcanoes in continental rifts with volcanic ground.
    let mut volcanoes = Vec::new();
    for r in 0..cr {
        for c in 0..cc {
            let i = r * cc + c;
            let cell = grid.get(r, c);
            let arc = role_of(cell.plate_id) == MarginRole::Overriding
                && (ARC_GAP_KM.0 * scale..=ARC_GAP_KM.1 * scale).contains(&to_margin[i]);
            let rift_volcanic = rift[i] && volcanism.volcanism.get(r, c).dominates_cell();
            if !(arc || rift_volcanic) || hash01(key, 0x766f_6c63, i as u64) >= VOLCANO_PROBABILITY
            {
                continue;
            }
            let u = |salt: u64| hash01(key, salt, i as u64);
            volcanoes.push(Volcano {
                x_m: (c as f64 + u(1)) * coarse_m,
                y_m: (r as f64 + u(2)) * coarse_m,
                relief_m: VOLCANO_RELIEF_M.0 + u(3) * (VOLCANO_RELIEF_M.1 - VOLCANO_RELIEF_M.0),
                radius_m: 1000.0
                    * (VOLCANO_RADIUS_KM.0 + u(4) * (VOLCANO_RADIUS_KM.1 - VOLCANO_RADIUS_KM.0)),
            });
        }
    }

    let profile = domain.profile();
    let medium = DomainLevel::Medium;
    let (mr, mc) = (domain.rows(medium), domain.cols(medium));
    let cells: Vec<(usize, usize)> = (0..mr).flat_map(|r| (0..mc).map(move |c| (r, c))).collect();
    let warp_m = WARP_KM * 1000.0 * scale;
    let noise_wavelength_m = NOISE_WAVELENGTH_KM * 1000.0 * scale;
    let km = |v: f64| v * scale;
    let (warp_key, noise_key, strike_key, plate_key) =
        (mix(key ^ 1), mix(key ^ 2), mix(key ^ 3), mix(key ^ 4));

    let uplift_of = |x: f64, y: f64| -> (f64, f64) {
        // Plate membership read through a small warp so plate edges on the
        // medium grid are not 12 km stair steps.
        let jx = x + 8_000.0
            * scale
            * value_noise(plate_key, x / (30_000.0 * scale), y / (30_000.0 * scale));
        let jy = y + 8_000.0
            * scale
            * value_noise(
                plate_key ^ 9,
                x / (30_000.0 * scale),
                y / (30_000.0 * scale),
            );
        let (pr, pc) = (
            ((jy / coarse_m) as usize).min(cr - 1),
            ((jx / coarse_m) as usize).min(cc - 1),
        );
        let d = bilinear(&to_margin, cr, cc, coarse_m, x, y);
        let d_rift = bilinear(&to_rift, cr, cc, coarse_m, x, y);
        // Along-strike variation of range height, 0.55-1.35.
        let strike = 0.95 + 0.4 * fbm(strike_key, x, y, 350_000.0 * scale, 3);
        let a = RANGE_UPLIFT_M * strike;
        let mut u = match role_of(grid.get(pr, pc).plate_id) {
            MarginRole::Overriding => {
                a * (-((d - km(RANGE_OFFSET_KM)) / km(RANGE_HALF_WIDTH_KM)).powi(2)).exp()
                    + 0.35 * a * (-(d / km(220.0)).powi(2)).exp()
            }
            MarginRole::Downgoing => {
                -TRENCH_DEPTH_M * (-((d - km(TRENCH_OFFSET_KM)) / km(20.0)).powi(2)).exp()
                    + 300.0 * (-((d - km(80.0)) / km(40.0)).powi(2)).exp()
            }
            MarginRole::Colliding => a * (-(d / km(75.0)).powi(2)).exp(),
            MarginRole::None => 0.0,
        };
        u -= RIFT_SUBSIDENCE_M * (-(d_rift / km(RIFT_HALF_WIDTH_KM)).powi(2)).exp();
        (u, (u / RANGE_UPLIFT_M).clamp(0.0, 1.0))
    };

    // Base + margin relief + warped large-scale noise.
    let mut elevation: Vec<f64> = par_map_cells(&cells, |r, c| {
        let (x, y) = domain.cell_center_m(medium, r, c);
        let wx = x + warp_m * fbm(warp_key, x, y, 2.0 * noise_wavelength_m, 4);
        let wy = y + warp_m * fbm(warp_key ^ 7, x, y, 2.0 * noise_wavelength_m, 4);
        let b = bilinear(&base, cr, cc, coarse_m, wx, wy);
        let (u, _) = uplift_of(wx, wy);
        b + u + NOISE_AMPLITUDE_M * fbm(noise_key, wx, wy, noise_wavelength_m, NOISE_OCTAVES)
    });
    // Erosion stand-in: smooth, then cut valleys into the uplifted ground.
    smooth(&mut elevation, mr, mc, 2);
    let ridge_key = mix(key ^ 5);
    let texture: Vec<f64> = par_map_cells(&cells, |r, c| {
        let (x, y) = domain.cell_center_m(medium, r, c);
        let (_, mountain) = uplift_of(x, y);
        RIDGE_TEXTURE_M * mountain * (ridged(ridge_key, x, y, 60_000.0 * scale, 5) - 0.5)
    });
    // Draw the terrain down toward the ocean buffer.
    let taper_m = EDGE_TAPER_FRACTION * profile.width_m.min(profile.height_m);
    let buffer_m = profile.minimum_ocean_buffer_m;
    let taper: Vec<f64> = par_map_cells(&cells, |r, c| {
        let (x, y) = domain.cell_center_m(medium, r, c);
        let edge = x.min(y).min(profile.width_m - x).min(profile.height_m - y);
        let t = ((edge - buffer_m) / taper_m).clamp(0.0, 1.0);
        EDGE_TAPER_DEPTH_M * (1.0 - t).powi(2)
    });
    for ((e, t), d) in elevation.iter_mut().zip(&texture).zip(&taper) {
        *e += t - d;
    }

    // Volcanic cones on top.
    let cell_m = domain.cell_size_m(medium);
    for v in &volcanoes {
        let (r0, r1) = (
            ((v.y_m - v.radius_m) / cell_m).floor().max(0.0) as usize,
            (((v.y_m + v.radius_m) / cell_m).ceil() as usize).min(mr),
        );
        let (c0, c1) = (
            ((v.x_m - v.radius_m) / cell_m).floor().max(0.0) as usize,
            (((v.x_m + v.radius_m) / cell_m).ceil() as usize).min(mc),
        );
        for r in r0..r1 {
            for c in c0..c1 {
                let (x, y) = domain.cell_center_m(medium, r, c);
                let t = 1.0 - (x - v.x_m).hypot(y - v.y_m) / v.radius_m;
                if t > 0.0 {
                    elevation[r * mc + c] += v.relief_m * t.powf(1.6);
                }
            }
        }
    }

    let spec = domain.storage_spec(medium);
    (Grid2::from_data(&spec, elevation), volcanoes)
}
