//! The estate's own trees, stem by stem (Phase 5 Task 3 Step 2a).
//!
//! The island models every tree in the estate's patch individually, and
//! `/api/trees` serves them with their real height and stem diameter. This
//! turns what was served into trunks for the estate frame: a cylinder at
//! each stem's position, as tall and as thick as the stem is. Crowns are
//! not drawn: nothing the island serves says how wide one is, and a
//! canopy of invented size would look like data.
//!
//! Which trees: those around the yard, where the founders work and fell.
//! The server spreads a capped request evenly over the box rather than
//! taking the first stems in its list, so a dense wood is thinned, never
//! clipped to one corner.

use mk_island_api::{EstateLayout, Trees};

use crate::scene::{estate_of, Point};

/// How far around the yard the stems are drawn (m).
pub const NEIGHBOURHOOD_M: f64 = 250.0;

/// The most stems asked for at once: enough to read a wood around the yard
/// as a wood, few enough to draw as one entity each.
pub const STEM_CAP: usize = 3_000;

/// How often the stems are asked for again. They change only when
/// somebody fells one, and that takes the felling pack's minutes.
pub const TREE_POLL: std::time::Duration = std::time::Duration::from_secs(30);

/// The box (domain metres: west, south, east, north) whose stems are
/// drawn: the yard and [`NEIGHBOURHOOD_M`] around it, kept on the patch.
pub fn neighbourhood(layout: &EstateLayout) -> (f64, f64, f64, f64) {
    let (p, y) = (&layout.patch, &layout.yard);
    let clamp = |v: f64, a: f64, b: f64| v.clamp(a.min(b), a.max(b));
    (
        clamp(y.x0.min(y.x1) - NEIGHBOURHOOD_M, p.x0, p.x1),
        clamp(y.y0.min(y.y1) - NEIGHBOURHOOD_M, p.y0, p.y1),
        clamp(y.x0.max(y.x1) + NEIGHBOURHOOD_M, p.x0, p.x1),
        clamp(y.y0.max(y.y1) + NEIGHBOURHOOD_M, p.y0, p.y1),
    )
}

/// One trunk, ready to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stem {
    pub id: u64,
    /// The foot of the stem, in the estate frame.
    pub base: Point,
    pub height_m: f32,
    pub radius_m: f32,
    pub shrub: bool,
}

/// The trunks among `trees`, in the estate frame whose origin is
/// `origin_m` (the patch corner). Grass is not a stem, and a stem with no
/// height or no thickness is not drawn.
pub fn stems(trees: &Trees, origin_m: (f64, f64)) -> Vec<Stem> {
    trees
        .trees
        .iter()
        .filter(|t| t.kind != "Grass")
        .filter(|t| t.height_m > 0.0 && t.stem_diameter_m > 0.0)
        .map(|t| Stem {
            id: t.id,
            base: estate_of(origin_m, t.x_m, t.y_m, 0.0),
            height_m: t.height_m as f32,
            radius_m: (t.stem_diameter_m / 2.0) as f32,
            shrub: t.kind == "Shrub",
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_island_api::{Rect, Tree};

    fn layout() -> EstateLayout {
        EstateLayout {
            patch: Rect {
                x0: 1_000.0,
                y0: 2_000.0,
                x1: 5_000.0,
                y1: 6_000.0,
            },
            yard: Rect {
                x0: 1_100.0,
                y0: 3_900.0,
                x1: 1_200.0,
                y1: 4_000.0,
            },
            buildings: vec![],
            spaces: vec![],
            items: vec![],
            doors: vec![],
        }
    }

    fn tree(id: u64, kind: &str, x: f64, y: f64, h: f64, d: f64) -> Tree {
        Tree {
            id,
            kind: kind.into(),
            x_m: x,
            y_m: y,
            height_m: h,
            stem_diameter_m: d,
        }
    }

    #[test]
    fn the_neighbourhood_is_the_yard_and_its_surroundings_kept_on_the_patch() {
        // 250 m around the yard, but the yard is 100 m from the patch's
        // west edge, so the box stops there.
        assert_eq!(
            neighbourhood(&layout()),
            (1_000.0, 3_650.0, 1_450.0, 4_250.0)
        );
    }

    #[test]
    fn a_stem_is_drawn_where_it_stands_as_tall_and_thick_as_it_is() {
        let trees = Trees {
            trees: vec![
                tree(1, "Tree", 1_150.0, 4_100.0, 24.0, 0.42),
                tree(2, "Shrub", 1_120.0, 3_880.0, 2.5, 0.05),
                tree(3, "Grass", 1_130.0, 3_890.0, 0.4, 0.0),
                tree(4, "Tree", 1_140.0, 3_895.0, 0.0, 0.3),
            ],
            ..Default::default()
        };
        let drawn = stems(&trees, (1_000.0, 2_000.0));
        assert_eq!(
            drawn.len(),
            2,
            "grass and a stem with no height are not trunks"
        );
        let oak = drawn[0];
        assert_eq!(oak.id, 1);
        // 150 m east and 2,100 m north of the corner: z is south.
        assert_eq!(oak.base, Point::new(150.0, 0.0, -2_100.0));
        assert_eq!((oak.height_m, oak.radius_m), (24.0, 0.21));
        assert!(!oak.shrub && drawn[1].shrub);
    }
}
