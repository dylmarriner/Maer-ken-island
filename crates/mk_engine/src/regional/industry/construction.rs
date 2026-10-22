//! Building a timber-framed house (Phase 4b Task 7b, timber chain).
//!
//! The house is MBIE's reference single-storey NZS 3604 house (thinkstep
//! 2022, Table 2-2): 209 m² of slab floor, 156 m² of envelope wall and
//! 169 m² of internal wall, each framed in 90 × 45 mm timber. Timber is
//! 18% of a framed wall's area (Table 6-2), so a wall holds its area ×
//! 0.18 × 0.090 m of sawn timber. The roof trusses, floor and the rest are
//! not counted yet: no source read gives their timber.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const HOUSE_AREAS: &str = "reference_house_areas";
const WALLS: [&str; 2] = [
    "envelope_wall_brick_veneer_over_90x45_timber_frame_10mm_plasterboard",
    "internal_walls_90x45_timber_frame_10mm_plasterboard_both_sides",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseModel {
    pub floor_m2: f64,
    pub framed_wall_m2: f64,
    pub framing_ratio: f64,
    pub stud_depth_m: f64,
}

impl HouseModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let item = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MissingReference(key.into()))
        };
        let point = |key: &str| {
            item(key)?
                .point()
                .ok_or_else(|| MissingReference(key.into()))
        };
        let areas = item(HOUSE_AREAS)?
            .table()
            .ok_or_else(|| MissingReference(HOUSE_AREAS.into()))?;
        let area = |row: &str| {
            areas
                .get(row, "area_m2")
                .ok_or_else(|| MissingReference(format!("{HOUSE_AREAS}/{row}")))
        };
        Ok(Self {
            floor_m2: area("floor_concrete_slab_on_grade")?,
            framed_wall_m2: area(WALLS[0])? + area(WALLS[1])?,
            framing_ratio: point("wall_framing_ratio")?,
            stud_depth_m: point("wall_frame_stud_depth_mm")? / 1_000.0,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    /// m³ of sawn timber in the house's wall frames.
    pub fn wall_framing_m3(&self) -> f64 {
        self.framed_wall_m2 * self.framing_ratio * self.stud_depth_m
    }
}

/// Sawn timber in stock and the houses framed from it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Builder {
    pub timber_m3: f64,
    /// Tonnes of carbon in the stock.
    pub timber_carbon_t: f64,
    pub houses: u32,
    /// Tonnes of carbon in the frames of the houses built.
    pub houses_carbon_t: f64,
}

impl Builder {
    pub fn receive(&mut self, timber_m3: f64, carbon_t: f64) {
        self.timber_m3 += timber_m3;
        self.timber_carbon_t += carbon_t;
    }

    /// Frames as many houses as the stock allows; returns how many.
    pub fn build(&mut self, model: &HouseModel) -> u32 {
        let need = model.wall_framing_m3();
        let mut built = 0;
        while self.timber_m3 >= need {
            let carbon = self.timber_carbon_t * need / self.timber_m3;
            self.timber_m3 -= need;
            self.timber_carbon_t -= carbon;
            self.houses_carbon_t += carbon;
            built += 1;
        }
        self.houses += built;
        built
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_house_frames_its_walls_from_the_pack() {
        let m = HouseModel::load_default().expect("the pack loads");
        assert_eq!(m.floor_m2, 209.0);
        assert_eq!(m.framed_wall_m2, 325.0);
        assert!((m.wall_framing_m3() - 325.0 * 0.18 * 0.09).abs() < 1e-9);
    }

    #[test]
    fn a_builder_frames_what_its_stock_allows_and_keeps_the_carbon() {
        let m = HouseModel::load_default().expect("the pack loads");
        let mut b = Builder::default();
        b.receive(2.5 * m.wall_framing_m3(), 1.0);
        assert_eq!(b.build(&m), 2);
        assert!((b.timber_m3 - 0.5 * m.wall_framing_m3()).abs() < 1e-9);
        assert!((b.timber_carbon_t + b.houses_carbon_t - 1.0).abs() < 1e-12);
        assert!((b.houses_carbon_t - 0.8).abs() < 1e-12);
    }
}
