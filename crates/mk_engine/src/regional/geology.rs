//! Rock types of the island (Phase 1 Task 5).
//!
//! A lithology map on the medium grid from the same tectonic setting that
//! built the terrain ([`super::terrain::GeologicalSetting`]), so rocks and
//! landforms agree. Rules follow the architecture of real convergent
//! margins (New Zealand's in particular):
//!
//! - oceanic plates are basalt;
//! - next to a convergent boundary on the overriding or colliding side,
//!   metamorphic grade falls with distance: gneiss and high-grade schist
//!   against the fault (the Alpine Schist), schist, then low-grade
//!   greywacke (the Torlesse terranes), with marble where limestone was
//!   caught up, and ultramafic slivers along the suture (the Dun Mountain
//!   ophiolite belt);
//! - the volcanic arc 100–200 km behind the margin is andesite with
//!   rhyolitic centres; granite batholiths stand further back with
//!   pegmatite at their margins;
//! - continental rifts are rhyolite and rift-fill sandstone;
//! - elsewhere subsiding continental crust holds sedimentary basins:
//!   limestone and mudstone on shallow shelves, sandstone, coal measures in
//!   low wet basins and evaporites in closed ones and buried under shelves;
//! - granite batholiths carry marble roof pendants at their margins, the
//!   carbonate contacts where skarns form;
//! - with an ancient basement, a fragment of Archean craton is cratonic
//!   gneiss whose thick-lithosphere interior hosts kimberlite pipes
//!   (Clifford's rule).
//!
//! Length thresholds scale with the landform scale, like the terrain.

use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};
use serde::{Deserialize, Serialize};

use super::par::par_map_cells;
use super::terrain::{fbm, hash01, GeologicalSetting, MarginRole, Volcano};
use crate::tectonics::PlateType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Lithology {
    OceanicBasalt,
    ArcAndesite,
    Rhyolite,
    Granite,
    Pegmatite,
    Greywacke,
    Schist,
    Gneiss,
    Marble,
    Limestone,
    Sandstone,
    Mudstone,
    CoalMeasures,
    Evaporite,
    Ultramafic,
    CratonicGneiss,
    Kimberlite,
}

/// Distances (km at full scale) of the margin's metamorphic and magmatic
/// belts from the plate boundary. Alpine Schist grade rises to
/// amphibolite facies within ~20 km of the Alpine Fault (Grapes & Watanabe
/// 1992); arc–trench gaps are 100–200 km (Gill 1981); arc plutons and
/// Cordilleran batholiths lie ~60–300 km inboard (the Sierra Nevada and
/// Coast Mountains batholiths; NZ's Median Batholith).
const GNEISS_KM: f64 = 20.0;
const SCHIST_KM: f64 = 45.0;
const GREYWACKE_KM: f64 = 110.0;
const SUTURE_KM: f64 = 8.0;
const ARC_BELT_KM: (f64, f64) = (100.0, 200.0);
const BATHOLITH_KM: (f64, f64) = (70.0, 250.0);
const RIFT_KM: f64 = 25.0;
/// Wavelength (km at full scale) of the noise that patches rock bodies.
const BODY_WAVELENGTH_KM: f64 = 25.0;
/// Fraction of granite cells on a batholith margin that are pegmatite.
const PEGMATITE_RIM_FRACTION: f64 = 0.3;
/// Kimberlite pipes: only within the thick-lithosphere interior of the
/// craton (inner 70% of its radius; cratonic keels exceed 150 km, their
/// margins thin), and in ~1.2% of those 2 km cells. Kimberlite fields hold
/// tens to hundreds of pipes over thousands of km² (Lac de Gras: ~300 pipes
/// in ~30,000 km²; Kjarsgaard 2007).
const KIMBERLITE_CORE_FRACTION: f64 = 0.7;
const KIMBERLITE_CELL_FRACTION: f64 = 0.012;

/// The ancient basement fragment, if the profile has one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Basement {
    pub centre_m: (f64, f64),
    pub radius_m: f64,
}

/// Places the basement on continental land of the overriding side, as far
/// from the convergent margin as possible, sized to the profile's fraction
/// of the island's land area.
pub fn place_basement(
    domain: &IslandDomain,
    setting: &GeologicalSetting,
    land: &Grid2<bool>,
    land_area_m2: f64,
    key: u64,
) -> Option<Basement> {
    let g = &domain.profile().geology;
    if !g.ancient_basement || g.basement_area_fraction <= 0.0 {
        return None;
    }
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let mut best: Option<(f64, (f64, f64))> = None;
    for r in 0..rows {
        for c in 0..cols {
            if !*land.get(r, c) {
                continue;
            }
            let (x, y) = domain.cell_center_m(medium, r, c);
            let s = setting.at(x, y);
            if s.crust != PlateType::Continental || s.role == MarginRole::Downgoing {
                continue;
            }
            // Far from the margin, with a little hashed jitter to break ties.
            let score = s.margin_km + 5.0 * hash01(key, 0x6261_7365, (r * cols + c) as u64);
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, (x, y)));
            }
        }
    }
    let (_, centre_m) = best?;
    let radius_m = (g.basement_area_fraction * land_area_m2 / std::f64::consts::PI).sqrt();
    Some(Basement { centre_m, radius_m })
}

/// The lithology of every medium cell.
pub fn generate_lithology(
    domain: &IslandDomain,
    setting: &GeologicalSetting,
    volcanoes: &[Volcano],
    elevation_m: &Grid2<f64>,
    basement: Option<Basement>,
    key: u64,
) -> Grid2<Lithology> {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    // Scaled widths never fall below 1.5 cells, or narrow belts would
    // vanish from a small island.
    let floor_km = 1.5 * domain.cell_size_m(medium) / 1000.0;
    let k = |v: f64| (v * setting.scale).max(floor_km);
    let wavelength = BODY_WAVELENGTH_KM * 1000.0 * setting.scale;
    let noise = |salt: u64, x: f64, y: f64| fbm(key ^ salt, x, y, wavelength, 3);
    let cells: Vec<(usize, usize)> = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .collect();

    let mut rocks: Vec<Lithology> = par_map_cells(&cells, |r, c| {
        use Lithology::*;
        let (x, y) = domain.cell_center_m(medium, r, c);
        let e = *elevation_m.get(r, c);
        if let Some(b) = basement {
            let d = (x - b.centre_m.0).hypot(y - b.centre_m.1);
            // A ragged rather than circular outline.
            let ragged = d * (1.0 + 0.25 * noise(0xB1, x, y));
            if ragged < b.radius_m {
                let i = (r * cols + c) as u64;
                if ragged < KIMBERLITE_CORE_FRACTION * b.radius_m
                    && hash01(key, 0x6b69_6d62, i) < KIMBERLITE_CELL_FRACTION
                {
                    return Kimberlite;
                }
                return CratonicGneiss;
            }
        }
        if let Some((n, v)) = volcanoes
            .iter()
            .enumerate()
            .find(|(_, v)| (x - v.x_m).hypot(y - v.y_m) < v.radius_m)
        {
            // About a third of arc centres are rhyolitic calderas (Taupō);
            // rift volcanoes are rhyolitic.
            let rhyolitic = v.rift || hash01(key, 0x7268_796f, n as u64) < 0.3;
            return if rhyolitic { Rhyolite } else { ArcAndesite };
        }
        let s = setting.at(x, y);
        let d = s.margin_km;
        if s.crust == PlateType::Oceanic && s.role != MarginRole::Overriding {
            return OceanicBasalt;
        }
        let orogenic = matches!(s.role, MarginRole::Overriding | MarginRole::Colliding);
        if orogenic {
            if d < k(SUTURE_KM) && noise(0x11, x, y) > 0.3 {
                return Ultramafic;
            }
            let metamorphic = if d < k(GNEISS_KM) {
                Some(if noise(0x12, x, y) > 0.0 {
                    Gneiss
                } else {
                    Schist
                })
            } else if d < k(SCHIST_KM) {
                Some(Schist)
            } else {
                None
            };
            if let Some(m) = metamorphic {
                return if noise(0x13, x, y) > 0.55 { Marble } else { m };
            }
            if (k(BATHOLITH_KM.0)..=k(BATHOLITH_KM.1)).contains(&d) {
                let pluton = noise(0x16, x, y);
                if pluton > 0.35 {
                    return Granite;
                }
                // Roof pendants of limestone baked to marble at pluton
                // margins: where skarns form.
                if pluton > 0.25 && noise(0x13, x, y) > 0.3 {
                    return Marble;
                }
            }
            if s.role == MarginRole::Overriding
                && (k(ARC_BELT_KM.0)..=k(ARC_BELT_KM.1)).contains(&d)
                && noise(0x14, x, y) > 0.2
            {
                return if noise(0x15, x, y) > 0.5 {
                    Rhyolite
                } else {
                    ArcAndesite
                };
            }
            if d < k(GREYWACKE_KM) {
                return Greywacke;
            }
        }
        if s.rift_km < k(RIFT_KM) {
            return if noise(0x17, x, y) > 0.0 {
                Rhyolite
            } else {
                Sandstone
            };
        }
        // Sedimentary cover of continental crust.
        if e < -200.0 {
            Mudstone
        } else if e < 0.0 {
            // Salt basins also lie buried under shelves.
            if noise(0x1A, x, y) > 0.7 {
                Evaporite
            } else if noise(0x18, x, y) > 0.0 {
                Limestone
            } else {
                Mudstone
            }
        } else if e < 250.0 {
            if noise(0x19, x, y) > 0.45 {
                CoalMeasures
            } else if noise(0x1A, x, y) > 0.6 {
                Evaporite
            } else if noise(0x1B, x, y) > 0.0 {
                Sandstone
            } else {
                Mudstone
            }
        } else {
            Greywacke
        }
    });

    // Pegmatite at batholith margins.
    let snapshot = rocks.clone();
    for r in 0..rows {
        for c in 0..cols {
            let i = r * cols + c;
            if snapshot[i] != Lithology::Granite {
                continue;
            }
            let rim = [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)]
                .iter()
                .any(|&(dr, dc)| {
                    let (rr, cc) = (r as i64 + dr, c as i64 + dc);
                    rr >= 0
                        && cc >= 0
                        && (rr as usize) < rows
                        && (cc as usize) < cols
                        && snapshot[rr as usize * cols + cc as usize] != Lithology::Granite
                });
            if rim && hash01(key, 0x7065_676d, i as u64) < PEGMATITE_RIM_FRACTION {
                rocks[i] = Lithology::Pegmatite;
            }
        }
    }
    Grid2::from_data(&domain.storage_spec(medium), rocks)
}
