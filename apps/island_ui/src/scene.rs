//! Where things are, once the island has been turned into something a
//! renderer can draw.
//!
//! Two frames, because one cannot serve both. The island is about
//! 2,400 km across and a bedroom is four metres; a single scale that put
//! the island on screen would put both founders inside one floating-point
//! step of each other, and a single scale that drew the bedroom would put
//! the far coast two hundred million units away, where an `f32` has
//! metres of granularity left.
//!
//! So: a **regional** frame for looking at the island, where one unit is
//! ten kilometres and height is exaggerated; and an **estate** frame for
//! looking at the founders, where one unit is one metre and the origin
//! floats to the estate patch so the numbers stay small. [`regional_of`]
//! and [`estate_of`] put a point into each, and
//! [`estate_origin_in_regional`] is the transform between them.
//!
//! Nothing here touches Bevy, and nothing here is ever hashed. Rendering
//! is one-way: the simulation does not know this exists.

use mk_island_api::{Person, Terrain, World};

/// Metres per render unit in the regional frame.
///
/// Ten kilometres. The island is 2,401 km across, which comes to 240
/// units -- a scene a camera can orbit without a near plane fighting a far
/// one, and small enough that `f32` keeps sub-millimetre precision
/// everywhere in it.
pub const REGIONAL_METRES_PER_UNIT: f64 = 10_000.0;

/// How much taller than life the regional frame draws relief.
///
/// The island's relief is about 3 km against its 2,401 km width. At true
/// scale that is 0.3 units on a 240-unit map -- a flat sheet with a
/// coastline painted on it. Twenty times is enough to read the ranges and
/// is a presentation choice, not a measurement: the inspector always shows
/// metres from the terrain, never from this.
pub const REGIONAL_VERTICAL_EXAGGERATION: f64 = 20.0;

/// Metres per render unit in the estate frame. One, so a four-metre room
/// is four units and a person stands at their own height.
pub const ESTATE_METRES_PER_UNIT: f64 = 1.0;

/// A point in render space: x east, y up, z **south**.
///
/// Right-handed with y up, which is what every renderer this will meet
/// expects, and that is why z is south rather than north: the domain's
/// y runs north, and negating it is what keeps the handedness. A frame
/// that got this wrong would draw a mirror image of the island, which is
/// the kind of mistake that looks fine until somebody tries to find a
/// headland.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Distance to another point, in whatever units the frame uses.
    pub fn distance(&self, other: &Point) -> f32 {
        let (dx, dy, dz) = (self.x - other.x, self.y - other.y, self.z - other.z);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Domain metres into the regional frame.
///
/// The frame is centred on the island rather than cornered at its
/// south-west, so an orbit camera turns around the island instead of
/// around its corner.
pub fn regional_of(terrain: &Terrain, x_m: f64, y_m: f64, elevation_m: f64) -> Point {
    let (width_m, height_m) = terrain.extent_m();
    Point::new(
        ((x_m - width_m / 2.0) / REGIONAL_METRES_PER_UNIT) as f32,
        ((elevation_m * REGIONAL_VERTICAL_EXAGGERATION) / REGIONAL_METRES_PER_UNIT) as f32,
        (-(y_m - height_m / 2.0) / REGIONAL_METRES_PER_UNIT) as f32,
    )
}

/// Domain metres into the estate frame, relative to a floating origin.
///
/// `origin_m` is the patch's own corner. Keeping it out of the numbers is
/// the point: the estate is 114 km east and 124 km north of the domain's
/// corner, and metres measured from there lose precision a renderer can
/// see on a four-metre room.
pub fn estate_of(origin_m: (f64, f64), x_m: f64, y_m: f64, height_m: f64) -> Point {
    Point::new(
        ((x_m - origin_m.0) / ESTATE_METRES_PER_UNIT) as f32,
        (height_m / ESTATE_METRES_PER_UNIT) as f32,
        (-(y_m - origin_m.1) / ESTATE_METRES_PER_UNIT) as f32,
    )
}

/// Where the estate frame's origin sits in the regional one, so a camera
/// can fly between them without either frame knowing about the other.
pub fn estate_origin_in_regional(
    terrain: &Terrain,
    origin_m: (f64, f64),
    elevation_m: f64,
) -> Point {
    regional_of(terrain, origin_m.0, origin_m.1, elevation_m)
}

/// One islander, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedPerson {
    pub agent_id: String,
    pub alive: bool,
    pub asleep: bool,
    /// Where they stand in the regional frame.
    pub regional: Point,
    /// And in the estate frame, when they are on the patch. A person out
    /// on the island has no estate position and must not be given one: a
    /// renderer drawing them at the patch origin would put somebody who is
    /// two hundred kilometres away in the founders' yard.
    pub estate: Option<Point>,
    /// The room they are in, by the layout's own label.
    pub space: Option<String>,
}

/// Place everyone in the world.
///
/// A person with neither metres nor a cell is skipped rather than placed
/// at the origin. The island does give both for anyone on it; somebody
/// with neither is somebody this frontend does not know where to draw, and
/// drawing them in the sea off the south-west corner would be inventing a
/// position.
pub fn place_people(
    terrain: &Terrain,
    world: &World,
    estate_origin_m: (f64, f64),
) -> Vec<PlacedPerson> {
    world
        .people
        .iter()
        .filter_map(|person| place_person(terrain, person, estate_origin_m))
        .collect()
}

fn place_person(
    terrain: &Terrain,
    person: &Person,
    estate_origin_m: (f64, f64),
) -> Option<PlacedPerson> {
    let (x_m, y_m) = match (person.position_m, person.cell) {
        (Some(metres), _) => metres,
        (None, Some((row, col))) => terrain.centre_m(row, col),
        (None, None) => return None,
    };
    let (row, col) = terrain.cell_at_m(x_m, y_m)?;
    let ground_m = f64::from(terrain.elevation_at(row, col)?.max(0.0));
    // The top of their head above the ground they stand on: their own
    // height, as the island drew it for them.
    let standing_m = stature_m(person);
    Some(PlacedPerson {
        agent_id: person.agent_id.clone(),
        alive: person.alive,
        asleep: person.asleep,
        regional: regional_of(terrain, x_m, y_m, ground_m + standing_m),
        // Only where they really have metres. A cell is 2 km across, so a
        // person known only by cell has no place in a frame where one unit
        // is a metre.
        estate: person
            .position_m
            .map(|(x, y)| estate_of(estate_origin_m, x, y, standing_m)),
        space: person.space.clone(),
    })
}

/// The mean adult height of the population the island draws people from,
/// for a backend too old to say how tall somebody is.
///
/// The island's own figures rather than a renderer's: `individual.rs`
/// draws adult height from the NCD Risk Factor Collaboration's 2016 global
/// means, 171.0 cm for men and 159.0 cm for women, and this is the middle
/// of the two. Every frontend used to stand every person at 1.7 m, which
/// was nobody's height in particular and the island's for no one.
const POPULATION_MEAN_HEIGHT_M: f64 = (1.710 + 1.590) / 2.0;

/// How tall to draw `person`: what the island says, or -- only when an
/// older backend says nothing -- the mean of the population it draws from.
fn stature_m(person: &Person) -> f64 {
    if person.height_m > 0.0 {
        person.height_m
    } else {
        POPULATION_MEAN_HEIGHT_M
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain() -> Terrain {
        // 10 x 10 cells of 2 km: a 20 km square, enough to check the
        // arithmetic without pretending to be an island.
        let cells = 100;
        Terrain {
            rows: 10,
            cols: 10,
            cell_size_m: 2_000.0,
            elevation_m: (0..cells).map(|i| i as f32).collect(),
            land: (0..cells).map(|i| i % 3 != 0).collect(),
        }
    }

    #[test]
    fn the_centre_of_the_island_is_the_origin() {
        let t = terrain();
        let (w, h) = t.extent_m();
        let middle = regional_of(&t, w / 2.0, h / 2.0, 0.0);
        assert_eq!(middle, Point::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn north_is_negative_z_and_east_is_positive_x() {
        // A frame that got this wrong would draw a mirror image of the
        // island: everything in the right place and the wrong way round,
        // which looks fine until somebody tries to find a headland.
        let t = terrain();
        let (w, h) = t.extent_m();
        let east = regional_of(&t, w, h / 2.0, 0.0);
        let north = regional_of(&t, w / 2.0, h, 0.0);
        assert!(east.x > 0.0 && east.z == 0.0, "{east:?}");
        assert!(north.z < 0.0 && north.x == 0.0, "{north:?}");
    }

    #[test]
    fn the_whole_island_fits_in_a_scene_a_camera_can_hold() {
        // 20 km at 10 km per unit is two units across, one either side of
        // the centre. On the real island, 2,401 km is 240.
        let t = terrain();
        let (w, h) = t.extent_m();
        let corner = regional_of(&t, w, h, 0.0);
        assert_eq!(corner.x, 1.0);
        assert_eq!(corner.z, -1.0);
    }

    #[test]
    fn height_is_exaggerated_by_exactly_what_the_constant_says() {
        let t = terrain();
        let at = regional_of(&t, 0.0, 0.0, 3_000.0);
        let expected = (3_000.0 * REGIONAL_VERTICAL_EXAGGERATION / REGIONAL_METRES_PER_UNIT) as f32;
        assert_eq!(at.y, expected);
        // Six units of relief against 240 of width: readable, which is
        // the whole reason the exaggeration exists.
        assert!((at.y - 6.0).abs() < 1e-6, "{}", at.y);
    }

    #[test]
    fn the_estate_frame_is_metres_from_its_own_corner() {
        let origin = (113_000.0, 124_000.0);
        let at = estate_of(origin, 113_004.0, 124_003.0, 1.7);
        assert_eq!(at, Point::new(4.0, 1.7, -3.0));
    }

    #[test]
    fn the_estate_frame_keeps_precision_a_bedroom_needs() {
        // The real estate stands 113,967 m east and 124,015 m north of
        // the domain's south-west corner. An `f32` metre at that distance
        // has a step of **7.8 mm** -- measured, not estimated -- which is
        // coarse enough to see on a four-metre room: furniture against a
        // wall shifts visibly, and two things 5 mm apart become one.
        //
        // Measured from the patch's own corner instead, the numbers are
        // single-digit metres and the step is half a micrometre. That is
        // the whole reason the origin floats, and this test is the record
        // of the figure rather than an assertion that it feels better.
        //
        // The first version of this test used a centimetre and failed,
        // because a centimetre is just above that 7.8 mm step. A
        // millimetre is below it.
        let origin = (113_967.5, 124_015.0);
        let a = estate_of(origin, 113_967.500, 124_015.0, 0.0);
        let b = estate_of(origin, 113_967.501, 124_015.0, 0.0);
        assert_ne!(a.x, b.x, "a millimetre vanished from the estate frame");

        // Written as addition rather than as two literals: clippy reads
        // `113_967.501_f32` as excessive precision, which is precisely
        // the fact under test, and the arithmetic says it better anyway.
        // Adding a millimetre to a metre-measure at the estate's distance
        // from the domain corner changes nothing at all.
        let naive_a = 113_967.5_f32;
        let naive_b = naive_a + 0.001;
        assert_eq!(
            naive_a, naive_b,
            "a millimetre at the estate's distance from the domain corner is representable \
             after all, so the floating origin has stopped being necessary"
        );

        // And the step itself, so the comment above cannot drift from the
        // arithmetic under it.
        let step = f32::from_bits(naive_a.to_bits() + 1) - naive_a;
        assert!(
            (step - 0.0078125).abs() < 1e-9,
            "an f32 metre at the estate is {step} m, not the 7.8 mm this is written around"
        );
    }

    #[test]
    fn the_two_frames_agree_about_where_the_estate_is() {
        let t = terrain();
        let origin = (8_000.0, 6_000.0);
        let in_regional = estate_origin_in_regional(&t, origin, 0.0);
        assert_eq!(in_regional, regional_of(&t, origin.0, origin.1, 0.0));
        // And a point one metre east of the origin is one metre east in
        // both, once the regional scale is undone.
        let east_estate = estate_of(origin, origin.0 + 1.0, origin.1, 0.0);
        let east_regional = regional_of(&t, origin.0 + 1.0, origin.1, 0.0);
        let regional_metres = f64::from(east_regional.x - in_regional.x) * REGIONAL_METRES_PER_UNIT;
        assert!((regional_metres - f64::from(east_estate.x)).abs() < 1e-3);
    }

    fn a_person(position: Option<(f64, f64)>, cell: Option<(usize, usize)>) -> Person {
        Person {
            agent_id: "Gem-D".to_string(),
            alive: true,
            asleep: false,
            age_years: 25.0,
            space: Some("Gem-D's Bedroom".to_string()),
            position_m: position,
            cell,
            body_carbon_kg: Some(13.0),
            height_m: 1.83,
        }
    }

    #[test]
    fn somebody_with_metres_is_placed_in_both_frames() {
        let t = terrain();
        let world = World {
            people: vec![a_person(Some((8_000.0, 6_000.0)), Some((3, 4)))],
            ..Default::default()
        };
        let placed = place_people(&t, &world, (8_000.0, 6_000.0));
        assert_eq!(placed.len(), 1);
        assert_eq!(
            placed[0].estate,
            Some(Point::new(0.0, 1.83, 0.0)),
            "a person stands at their own height, not one chosen for everybody"
        );
        assert_eq!(placed[0].space.as_deref(), Some("Gem-D's Bedroom"));
    }

    #[test]
    fn two_people_of_different_heights_stand_at_different_heights() {
        let t = terrain();
        let mut tall = a_person(Some((8_000.0, 6_000.0)), Some((3, 4)));
        tall.height_m = 1.93;
        let mut short = a_person(Some((8_000.0, 6_000.0)), Some((3, 4)));
        short.agent_id = "Gem-K".to_string();
        short.height_m = 1.52;
        let world = World {
            people: vec![tall, short],
            ..Default::default()
        };
        let placed = place_people(&t, &world, (8_000.0, 6_000.0));
        let head = |id: &str| {
            placed
                .iter()
                .find(|p| p.agent_id == id)
                .and_then(|p| p.estate)
                .expect("placed on the estate")
                .y
        };
        assert!(
            (head("Gem-D") - head("Gem-K") - 0.41).abs() < 1e-5,
            "a 1.93 m person and a 1.52 m person must stand 41 cm apart at the head"
        );
    }

    #[test]
    fn a_backend_that_sends_no_height_gets_the_islands_own_mean() {
        let t = terrain();
        let mut unsaid = a_person(Some((8_000.0, 6_000.0)), Some((3, 4)));
        unsaid.height_m = 0.0;
        let world = World {
            people: vec![unsaid],
            ..Default::default()
        };
        let placed = place_people(&t, &world, (8_000.0, 6_000.0));
        let y = placed[0].estate.expect("placed").y;
        assert!(
            (f64::from(y) - 1.65).abs() < 1e-5,
            "the middle of the NCD-RisC means the island draws from, not 1.7: {y}"
        );
    }

    #[test]
    fn somebody_known_only_by_cell_gets_no_estate_position() {
        // A cell is 2 km across. Giving them one anyway would put a person
        // who could be anywhere in that square at an exact spot in the
        // founders' yard.
        let t = terrain();
        let world = World {
            people: vec![a_person(None, Some((3, 4)))],
            ..Default::default()
        };
        let placed = place_people(&t, &world, (8_000.0, 6_000.0));
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].estate, None);
        assert_ne!(placed[0].regional, Point::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn somebody_with_no_place_at_all_is_not_drawn() {
        let t = terrain();
        let world = World {
            people: vec![a_person(None, None)],
            ..Default::default()
        };
        assert!(place_people(&t, &world, (0.0, 0.0)).is_empty());
    }

    #[test]
    fn somebody_off_the_domain_is_not_drawn_at_its_edge() {
        let t = terrain();
        let (w, _) = t.extent_m();
        let world = World {
            people: vec![a_person(Some((w + 10_000.0, 0.0)), None)],
            ..Default::default()
        };
        assert!(
            place_people(&t, &world, (0.0, 0.0)).is_empty(),
            "a position off the island must not be clamped onto it"
        );
    }
}
