//! The founders' estate, as something to put on screen.
//!
//! At the regional scale one unit is ten kilometres, so a four-metre room
//! is 0.0004 of a unit and nothing about the estate can be seen at all.
//! This builds it in the **estate frame**, where one unit is one metre,
//! measured from the patch's own corner so the numbers stay small.
//!
//! What is drawn is the simulation's own geometry: a building is its
//! footprint rectangle, a room is its rectangle, a thing is at the metres
//! it stands on. The models from `assets/` are decoration placed on top of
//! that, never a substitute for it -- a model at the wrong scale makes an
//! odd-looking shed, and a footprint taken from a model would make the
//! island disagree with itself.
//!
//! Pure data, so it is testable on a machine with no graphics.

use crate::property::{self, Model};
use crate::scene::{estate_of, Point};
use crate::stems::Stem;
use mk_island_api::{EstateLayout, Rect};

/// How tall a building is drawn when the island does not say.
///
/// It does say now -- `BuildingFootprint::height_m` carries a real
/// height with a real source, and a shed is six metres because the
/// tractor in it is 3.6. This remains only for an older backend that
/// serves a layout without heights, where drawing nothing would be
/// worse than drawing a storey.
pub const WALL_HEIGHT_M: f32 = 3.0;

/// And a room, drawn as a floor rather than a box so the things standing
/// on it are visible from above.
pub const FLOOR_THICKNESS_M: f32 = 0.1;

/// Something to draw on the estate, in the estate frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// What it is called, for an inspector and for the key.
    pub name: String,
    /// `House`, `Computer`, `Bedroom` — the kind or the label, whichever
    /// the thing has.
    pub kind: String,
    /// The centre of the thing, in the estate frame.
    pub at: Point,
    /// How big it is, in metres, as (across, up, deep) in this frame.
    ///
    /// Real throughout now: a building and a room carry their footprint
    /// from the layout, and an item carries the island's own dimensions
    /// from `organisms::dimensions` -- the maker's figures where the
    /// thing is a named machine. Nothing here is this renderer's
    /// invention any more.
    pub size_m: (f32, f32, f32),
    /// Turned, in degrees, about the vertical.
    pub rotation_deg: f32,
    /// The model, if one exists for it. `None` for the three kinds nothing
    /// models -- `property::WITHOUT_MODELS` -- which are drawn at the size
    /// the island gives them and nothing else.
    pub model: Option<Model>,
}

fn centre(rect: &Rect, origin_m: (f64, f64), height_m: f64) -> Point {
    let (x, y) = rect.centre_m();
    estate_of(origin_m, x, y, height_m)
}

/// The estate's buildings, placed.
///
/// The computer room is **not** among them, and that is the island's own
/// doing rather than an omission here: `house_plan` lays its space out
/// inside the House's footprint, so it is a building to the engine and a
/// room on the ground. It comes back from [`rooms`] instead, which is
/// where a renderer should look for it.
pub fn buildings(layout: &EstateLayout) -> Vec<Placed> {
    let origin = (layout.patch.x0, layout.patch.y0);
    layout
        .buildings
        .iter()
        .map(|building| {
            // The island's height where it has one. A zero means an
            // older backend that does not serve heights at all, not a
            // building of no height, so the storey is the fallback.
            let height = if building.height_m > 0.0 {
                building.height_m as f32
            } else {
                WALL_HEIGHT_M
            };
            Placed {
                name: building.name.clone(),
                kind: building.kind.clone(),
                at: centre(&building.rect_m, origin, f64::from(height) / 2.0),
                size_m: (
                    building.rect_m.width_m() as f32,
                    height,
                    building.rect_m.depth_m() as f32,
                ),
                rotation_deg: building.rotation_deg as f32,
                model: property::building(&building.kind),
            }
        })
        .collect()
}

/// The rooms and zones inside them, as floors.
pub fn rooms(layout: &EstateLayout) -> Vec<Placed> {
    let origin = (layout.patch.x0, layout.patch.y0);
    layout
        .spaces
        .iter()
        .map(|space| Placed {
            name: space.label.clone(),
            kind: space.kind.clone(),
            // Just above the ground, so a floor is not in a depth fight
            // with the yard it sits on.
            at: centre(&space.rect_m, origin, f64::from(FLOOR_THICKNESS_M)),
            size_m: (
                space.rect_m.width_m() as f32,
                FLOOR_THICKNESS_M,
                space.rect_m.depth_m() as f32,
            ),
            rotation_deg: 0.0,
            model: None,
        })
        .collect()
}

/// The walls of each room, as an outline: its four corners on the floor,
/// in order round it, for a line drawing rather than a solid.
///
/// Only `Room`s have walls; a `Zone` is part of an open floor (a bay of
/// the shed) and a `WholeBuilding` is the building's own box. Drawn as
/// lines because the island gives a room's extent and nothing about how
/// thick or tall its walls are; a line claims neither.
pub fn room_outlines(layout: &EstateLayout) -> Vec<(String, [Point; 4])> {
    let origin = (layout.patch.x0, layout.patch.y0);
    // Just above the floor's top face: a floor is drawn centred
    // `FLOOR_THICKNESS_M` up and that thick, so its top is at one and a
    // half thicknesses, and a line any lower is inside it and hidden.
    let floor = 2.0 * f64::from(FLOOR_THICKNESS_M);
    layout
        .spaces
        .iter()
        .filter(|space| space.kind == "Room")
        .map(|space| {
            let r = &space.rect_m;
            let corner = |x: f64, y: f64| estate_of(origin, x, y, floor);
            (
                space.label.clone(),
                [
                    corner(r.x0, r.y0),
                    corner(r.x1, r.y0),
                    corner(r.x1, r.y1),
                    corner(r.x0, r.y1),
                ],
            )
        })
        .collect()
}

/// Where each door is, on the floor in the estate frame, and how tall a
/// marker for it may stand: the eaves of the building it is in (the
/// island's own height), or a storey where it is in none.
pub fn door_marks(layout: &EstateLayout) -> Vec<(Point, f32)> {
    let origin = (layout.patch.x0, layout.patch.y0);
    layout
        .doors
        .iter()
        .map(|door| {
            let (x, y) = door.position_m;
            let eaves = layout
                .buildings
                .iter()
                .filter(|b| b.height_m > 0.0)
                // A door in an outside wall sits on its edge: inclusive.
                .find(|b| {
                    let r = &b.rect_m;
                    (r.x0.min(r.x1)..=r.x0.max(r.x1)).contains(&x)
                        && (r.y0.min(r.y1)..=r.y0.max(r.y1)).contains(&y)
                })
                .map_or(WALL_HEIGHT_M, |b| b.height_m as f32);
            (estate_of(origin, x, y, 0.0), eaves)
        })
        .collect()
}

/// Where a door in `label` leads, in words: the other side's label, or
/// "outdoors".
fn doors_of(layout: &EstateLayout, label: &str) -> Vec<String> {
    layout
        .doors
        .iter()
        .filter_map(|door| {
            let here = Some(label);
            if door.a.as_deref() == here {
                Some(door.b.clone())
            } else if door.b.as_deref() == here {
                Some(door.a.clone())
            } else {
                None
            }
        })
        .map(|other| other.map_or_else(|| "outdoors".to_string(), |o| format!("the {o}")))
        .collect()
}

/// Everything standing on the estate, the four machines in the computer
/// room included.
pub fn things(layout: &EstateLayout) -> Vec<Placed> {
    let origin = (layout.patch.x0, layout.patch.y0);
    layout
        .items
        .iter()
        .map(|item| {
            // The island's own figure, not this renderer's. `size_m`
            // comes from `organisms::dimensions` -- a Ford Raptor is
            // 5.381 m long because Ford says so -- and the only thing
            // done to it here is the axis swap: the wire gives length,
            // width and height, and this frame wants x across, y up, z
            // deep.
            let (length, width, height) = item.size_m;
            let size = (length as f32, height as f32, width as f32);
            Placed {
                name: item.name.clone(),
                kind: item.kind.clone(),
                // Standing on the floor rather than sunk into it.
                at: estate_of(
                    origin,
                    item.position_m.0,
                    item.position_m.1,
                    height / 2.0 + f64::from(FLOOR_THICKNESS_M),
                ),
                size_m: size,
                rotation_deg: 0.0,
                model: property::item(&item.kind, &item.name),
            }
        })
        .collect()
}

/// The cleared ground the buildings stand on.
pub fn yard(layout: &EstateLayout) -> Placed {
    let origin = (layout.patch.x0, layout.patch.y0);
    Placed {
        name: "Yard".to_string(),
        kind: "Yard".to_string(),
        at: centre(&layout.yard, origin, 0.0),
        size_m: (
            layout.yard.width_m() as f32,
            0.05,
            layout.yard.depth_m() as f32,
        ),
        rotation_deg: 0.0,
        model: None,
    }
}

/// How far along a ray, from `from` in the direction `along`, it first
/// meets `placed`'s box, or `None` if it misses.
///
/// The box is the one drawn: centred on `at`, `size_m` across, up and
/// deep, turned `rotation_deg` about the vertical the way Bevy's
/// `Quat::from_rotation_y` turns it. The ray is turned back into the box's
/// own frame and clipped against its three pairs of faces. A ray that
/// starts inside the box hits it at distance zero.
pub fn ray_hits(from: Point, along: Point, placed: &Placed) -> Option<f32> {
    let (sin, cos) = placed.rotation_deg.to_radians().sin_cos();
    // The inverse of a turn by `a` about +y: x' = x cos a - z sin a,
    // z' = x sin a + z cos a.
    let unturn = |x: f32, z: f32| (x * cos - z * sin, x * sin + z * cos);
    let (ox, oz) = unturn(from.x - placed.at.x, from.z - placed.at.z);
    let oy = from.y - placed.at.y;
    let (dx, dz) = unturn(along.x, along.z);
    let dy = along.y;

    let mut near = f32::NEG_INFINITY;
    let mut far = f32::INFINITY;
    let half = (
        placed.size_m.0 / 2.0,
        placed.size_m.1 / 2.0,
        placed.size_m.2 / 2.0,
    );
    for (o, d, h) in [(ox, dx, half.0), (oy, dy, half.1), (oz, dz, half.2)] {
        if d.abs() < f32::EPSILON {
            // Parallel to this pair of faces: inside the slab or never.
            if o.abs() > h {
                return None;
            }
            continue;
        }
        let (a, b) = ((-h - o) / d, (h - o) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    if far < near.max(0.0) {
        return None;
    }
    Some(near.max(0.0))
}

/// What a click on the estate chose, by its index in [`things`], [`rooms`]
/// or [`buildings`] -- which is also its index in the layout's `items`,
/// `spaces` or `buildings`, because each of those maps one to one -- or a
/// tree by the island's own id for it, which outlives any one read of the
/// trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    Thing(usize),
    Room(usize),
    Building(usize),
    Tree(u64),
}

/// The narrowest a trunk is taken to be when clicked (m across). A sapling
/// a few centimetres thick is a pixel or two at any useful distance; this
/// is how near the pointer has to be, not a claim about the tree, and the
/// tree is still drawn and described at its own diameter.
pub const TRUNK_PICK_WIDTH_M: f32 = 1.0;

/// The box a click on `stem` is tested against: the trunk as drawn,
/// upright from its foot, at least [`TRUNK_PICK_WIDTH_M`] across.
pub fn trunk_target(stem: &Stem) -> Placed {
    let across = (2.0 * stem.radius_m).max(TRUNK_PICK_WIDTH_M);
    Placed {
        name: format!("Tree {}", stem.id),
        kind: if stem.shrub { "Shrub" } else { "Tree" }.to_string(),
        at: Point::new(stem.base.x, stem.base.y + stem.height_m / 2.0, stem.base.z),
        size_m: (across, stem.height_m, across),
        rotation_deg: 0.0,
        model: None,
    }
}

/// What a ray picks on the estate.
///
/// The smallest kind of thing the ray meets wins, and the nearest of that
/// kind: a thing before a room, a room before a building. That is not the
/// nearest box overall, deliberately -- the buildings are drawn
/// translucent so that what is inside them can be seen, and a click on a
/// computer seen through the house's roof means the computer, not the
/// roof. The yard is never picked; it is the ground everything stands on.
///
/// A tree among `stems` (the trunks drawn) is weighed by distance rather
/// than ranked: it is picked only when the ray meets it before any room
/// and before any building. A room is seen through its building's
/// translucent walls, so it still wins over the building in front of it;
/// a tree behind a building does not, or that part of the building could
/// never be clicked.
pub fn pick(layout: &EstateLayout, stems: &[Stem], from: Point, along: Point) -> Option<Picked> {
    let nearest = |placed: &[Placed]| {
        placed
            .iter()
            .enumerate()
            .filter_map(|(i, p)| ray_hits(from, along, p).map(|t| (i, t)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    };
    if let Some((i, _)) = nearest(&things(layout)) {
        return Some(Picked::Thing(i));
    }
    let trunks: Vec<Placed> = stems.iter().map(trunk_target).collect();
    let room = nearest(&rooms(layout));
    let building = nearest(&buildings(layout));
    let in_front = |t: f32, other: Option<(usize, f32)>| other.is_none_or(|(_, d)| t < d);
    if let Some((i, t)) = nearest(&trunks) {
        if in_front(t, room) && in_front(t, building) {
            return Some(Picked::Tree(stems[i].id));
        }
    }
    room.map(|(i, _)| Picked::Room(i))
        .or(building.map(|(i, _)| Picked::Building(i)))
}

/// What the inspector says about something picked on the estate: the
/// island's own words and figures, with their sources where it gives them.
///
/// `None` when the index no longer names anything, which happens only if
/// the layout changed under a selection.
pub fn describe(layout: &EstateLayout, picked: Picked) -> Option<Vec<String>> {
    let origin = (layout.patch.x0, layout.patch.y0);
    let from_corner = |(x, y): (f64, f64)| {
        format!(
            "{:.1}, {:.1} m from the patch corner",
            x - origin.0,
            y - origin.1
        )
    };
    let lines = match picked {
        // A tree is described from the trees the island served, which the
        // layout does not hold: see `stems::describe`.
        Picked::Tree(_) => return None,
        Picked::Thing(i) => {
            let item = layout.items.get(i)?;
            let (length, width, height) = item.size_m;
            let mut lines = vec![
                item.name.clone(),
                format!("A {}", words(&item.kind)),
                match &item.space {
                    Some(space) => format!("In the {space}"),
                    None => "Outdoors".to_string(),
                },
                format!("At {}", from_corner(item.position_m)),
                format!("{length:.2} m long, {width:.2} m wide, {height:.2} m high"),
            ];
            if !item.size_source.is_empty() {
                lines.push(format!("Size: {}", item.size_source));
            }
            lines
        }
        Picked::Room(i) => {
            let space = layout.spaces.get(i)?;
            let r = &space.rect_m;
            let inside: Vec<&str> = layout
                .items_in(&space.label)
                .map(|item| item.name.as_str())
                .collect();
            let doors = doors_of(layout, &space.label);
            vec![
                space.label.clone(),
                format!("A {}", words(&space.kind)),
                format!(
                    "{:.1} x {:.1} m, {:.1} m² of floor",
                    r.width_m(),
                    r.depth_m(),
                    r.width_m() * r.depth_m()
                ),
                format!("Centred {}", from_corner(r.centre_m())),
                match inside.len() {
                    0 => "Nothing placed in it".to_string(),
                    n => format!("{n} things in it: {}", inside.join(", ")),
                },
                match doors.len() {
                    0 => "No door".to_string(),
                    1 => format!("A door to {}", doors[0]),
                    n => format!("{n} doors: to {}", doors.join(", to ")),
                },
            ]
        }
        Picked::Building(i) => {
            let building = layout.buildings.get(i)?;
            let r = &building.rect_m;
            let mut lines = vec![
                building.name.clone(),
                format!("A {}", words(&building.kind)),
                format!(
                    "{:.1} x {:.1} m on the ground, {:.1} m² of footprint",
                    r.width_m(),
                    r.depth_m(),
                    r.width_m() * r.depth_m()
                ),
                format!("Centred {}", from_corner(r.centre_m())),
            ];
            if building.rotation_deg != 0.0 {
                lines.push(format!("Turned {:.0}°", building.rotation_deg));
            }
            if building.height_m > 0.0 {
                lines.push(format!("{:.1} m to the eaves", building.height_m));
                if !building.height_source.is_empty() {
                    lines.push(format!("Height: {}", building.height_source));
                }
            } else {
                lines.push("Its height is not served; drawn a storey high".to_string());
            }
            lines
        }
    };
    Some(lines)
}

/// `ShedTool` as "shed tool", `ComputerRoom` as "computer room".
fn words(kind: &str) -> String {
    let mut out = String::with_capacity(kind.len() + 4);
    for (i, c) in kind.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// Where a camera should sit to see the whole estate, in the estate
/// frame: what to look at, and how far away.
///
/// Far enough that the widest side fits a 45-degree field of view, with a
/// little room. `None` when the estate covers no ground, which would mean
/// a layout with nothing in it.
pub fn framing(layout: &EstateLayout) -> Option<(Point, f32)> {
    let extent = layout.extent_m()?;
    let origin = (layout.patch.x0, layout.patch.y0);
    let widest = extent.width_m().max(extent.depth_m()) as f32;
    // half-angle of 45 degrees is 22.5; tan(22.5) ~ 0.4142
    let distance = (widest / 2.0) / 0.4142 * 1.3;
    Some((centre(&extent, origin, 0.0), distance.max(10.0)))
}

/// Where the camera should be to look at one picked thing up close: its
/// centre, and far enough that its widest side fits the 45-degree view
/// with the same room [`framing`] leaves around the whole estate.
///
/// For a room or a building this is the interior view: the buildings are
/// drawn translucent, so from here the room's furniture, tools, vehicles
/// and machines are what fills the picture. A tree is framed by its foot
/// and height. `None` when the pick no longer names anything.
pub fn framing_of(layout: &EstateLayout, stems: &[Stem], picked: Picked) -> Option<(Point, f32)> {
    let fit = |placed: &Placed| {
        let widest = placed.size_m.0.max(placed.size_m.1).max(placed.size_m.2);
        // tan(22.5 degrees), the half-angle of the view.
        let distance = (widest / 2.0) / 0.4142 * 1.3;
        (placed.at, distance.max(MIN_LOOK_M))
    };
    match picked {
        Picked::Thing(i) => things(layout).get(i).map(fit),
        Picked::Room(i) => rooms(layout).get(i).map(fit),
        Picked::Building(i) => buildings(layout).get(i).map(fit),
        Picked::Tree(id) => stems
            .iter()
            .find(|stem| stem.id == id)
            .map(|stem| fit(&trunk_target(stem))),
    }
}

/// Where the camera stands to be inside a room rather than looking down on
/// it: the room's centre at half the height of the building it is in (the
/// middle of the storey, so the floor and the things on it are in view and
/// no eye height is invented), and a distance of a quarter of the room's
/// shorter side, so the eye stays inside the walls whichever way it turns
/// while it looks level. `None` when the index names no room.
pub fn inside(layout: &EstateLayout, room: usize) -> Option<(Point, f32)> {
    let space = layout.spaces.get(room)?;
    let (x, y) = space.rect_m.centre_m();
    let height = layout
        .buildings
        .iter()
        .find(|b| (b.rect_m.x0..=b.rect_m.x1).contains(&x) && (b.rect_m.y0..=b.rect_m.y1).contains(&y))
        .map(|b| b.height_m as f32)
        .filter(|h| *h > 0.0)
        .unwrap_or(WALL_HEIGHT_M);
    let origin = (layout.patch.x0, layout.patch.y0);
    let shorter = space.rect_m.width_m().min(space.rect_m.depth_m()) as f32;
    Some((
        estate_of(origin, x, y, f64::from(height) / 2.0),
        shorter / 4.0,
    ))
}

/// The nearest the camera is brought to something it is asked to look at
/// (m): close enough to read a desk, not so close that a small thing
/// fills the view with one face of its box.
pub const MIN_LOOK_M: f32 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;
    use mk_island_api::{BuildingFootprint, Doorway, PlacedItem, SpaceRect};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    /// An estate shaped like the real one: a patch corner a long way from
    /// the domain's, a yard, a house with a computer room inside it, and
    /// the four machines.
    fn layout() -> EstateLayout {
        EstateLayout {
            patch: rect(113_000.0, 124_000.0, 117_000.0, 128_000.0),
            yard: rect(113_950.0, 124_000.0, 113_990.0, 124_040.0),
            buildings: vec![
                BuildingFootprint {
                    name: "Homestead".to_string(),
                    kind: "House".to_string(),
                    rect_m: rect(113_960.0, 124_010.0, 113_972.0, 124_020.0),
                    rotation_deg: 30.0,
                    // What the island serves for a single storey.
                    height_m: 3.0,
                    height_source: "a single-storey dwelling at the eaves".to_string(),
                },
                BuildingFootprint {
                    name: "Tool Shed".to_string(),
                    kind: "Shed".to_string(),
                    rect_m: rect(113_975.0, 124_010.0, 113_981.0, 124_015.0),
                    rotation_deg: 0.0,
                    // Tall enough for the tractor that lives in it.
                    height_m: 6.0,
                    height_source: "a machinery shed".to_string(),
                },
            ],
            spaces: vec![SpaceRect {
                id: 7,
                label: "Computer Room".to_string(),
                kind: "Room".to_string(),
                rect_m: rect(113_964.0, 124_012.0, 113_968.0, 124_016.0),
            }],
            doors: vec![
                // The computer room's door, in its south wall.
                Doorway {
                    a: Some("Computer Room".to_string()),
                    b: None,
                    position_m: (113_966.0, 124_012.0),
                },
                // The shed's, on its south edge.
                Doorway {
                    a: None,
                    b: Some("Tool Shed".to_string()),
                    position_m: (113_978.0, 124_010.0),
                },
                // A gate out on the open yard, in no building.
                Doorway {
                    a: None,
                    b: None,
                    position_m: (113_955.0, 124_030.0),
                },
            ],
            items: vec![
                PlacedItem {
                    name: "Gem-D's Computer (gaming, 3 monitors)".to_string(),
                    kind: "Computer".to_string(),
                    position_m: (113_965.0, 124_013.0),
                    space: Some("Computer Room".to_string()),
                    // What the island serves for a desktop tower.
                    size_m: (0.5, 0.25, 0.45),
                    size_source: "convention: a desktop tower case".to_string(),
                },
                PlacedItem {
                    name: "Local Archive Server".to_string(),
                    kind: "Computer".to_string(),
                    position_m: (113_967.0, 124_015.0),
                    space: Some("Computer Room".to_string()),
                    size_m: (0.5, 0.25, 0.45),
                    size_source: "convention: a desktop tower case".to_string(),
                },
            ],
        }
    }

    #[test]
    fn the_estate_lands_near_the_origin_of_its_own_frame() {
        // The whole reason the frame floats: measured from the domain's
        // corner these are six-figure metres, where an `f32` has 7.8 mm
        // of granularity. Measured from the patch they are hundreds.
        let placed = buildings(&layout());
        for thing in &placed {
            assert!(
                thing.at.x.abs() < 4_000.0 && thing.at.z.abs() < 4_000.0,
                "{} is at {:?}, which is not inside its own patch",
                thing.name,
                thing.at
            );
        }
    }

    #[test]
    fn a_building_is_drawn_at_its_real_footprint_and_its_real_height() {
        let placed = buildings(&layout());
        let house = placed.iter().find(|p| p.kind == "House").unwrap();
        assert_eq!(house.size_m.0, 12.0, "the house is 12 m across");
        assert_eq!(house.size_m.2, 10.0, "and 10 m deep");
        assert_eq!(house.rotation_deg, 30.0, "and turned as the layout says");
        assert_eq!(house.size_m.1, 3.0, "and the height the island served");
        // Standing on the ground rather than half-buried in it.
        assert_eq!(house.at.y, 1.5);

        // The shed is taller than the house, which is the whole point:
        // one holds people and the other holds a 3.6 m tractor. Drawing
        // both at a flat three metres was how that went unnoticed.
        let shed = placed.iter().find(|p| p.kind == "Shed").unwrap();
        assert_eq!(shed.size_m.1, 6.0);
        assert!(shed.size_m.1 > house.size_m.1);
    }

    #[test]
    fn a_layout_with_no_heights_still_draws_a_building() {
        // An older backend serves no `height_m` at all, which arrives as
        // zero. A building of no height is a building nobody can see, so
        // the storey is used instead -- the frontend degrades rather
        // than vanishing.
        let mut old = layout();
        for building in &mut old.buildings {
            building.height_m = 0.0;
            building.height_source = String::new();
        }
        for placed in buildings(&old) {
            assert_eq!(placed.size_m.1, WALL_HEIGHT_M, "{}", placed.name);
            assert!(placed.at.y > 0.0, "{} is buried", placed.name);
        }
    }

    #[test]
    fn the_computer_room_is_a_room_and_not_a_building() {
        // The island's own doing: `house_plan` lays its space out inside
        // the House's footprint, so it has no footprint of its own. A
        // renderer that looked for one would draw nothing, which is what
        // this test exists to stop.
        let layout = layout();
        assert!(!buildings(&layout).iter().any(|b| b.kind == "ComputerRoom"));
        let room = rooms(&layout)
            .into_iter()
            .find(|r| r.name == "Computer Room")
            .expect("the computer room is a room");
        assert_eq!(room.size_m.0, 4.0);
        assert_eq!(room.size_m.2, 4.0);

        // And it is inside the house.
        let house = buildings(&layout)
            .into_iter()
            .find(|b| b.kind == "House")
            .unwrap();
        assert!(
            (room.at.x - house.at.x).abs() < house.size_m.0
                && (room.at.z - house.at.z).abs() < house.size_m.2,
            "the room at {:?} is not inside the house at {:?}",
            room.at,
            house.at
        );
    }

    #[test]
    fn the_machines_stand_on_the_floor_of_the_room_they_are_in() {
        let layout = layout();
        let things = things(&layout);
        let room = rooms(&layout)
            .into_iter()
            .find(|r| r.name == "Computer Room")
            .unwrap();
        let machines: Vec<&Placed> = things.iter().filter(|t| t.kind == "Computer").collect();
        assert_eq!(machines.len(), 2);
        for machine in machines {
            assert!(
                machine.at.y > 0.0,
                "{} is sunk into the floor",
                machine.name
            );
            assert!(
                (machine.at.x - room.at.x).abs() <= room.size_m.0,
                "{} at {:?} is not in the room at {:?}",
                machine.name,
                machine.at,
                room.at
            );
            assert!(machine.model.is_some(), "{} has no model", machine.name);
        }
    }

    #[test]
    fn a_camera_asked_to_frame_the_estate_can_see_all_of_it() {
        let layout = layout();
        let (focus, distance) = framing(&layout).expect("the estate covers ground");
        let extent = layout.extent_m().unwrap();
        // The view at 45 degrees has to be at least as wide as the estate.
        let across = 2.0 * distance * (std::f32::consts::FRAC_PI_8).tan();
        assert!(
            f64::from(across) >= extent.width_m().max(extent.depth_m()),
            "a {across} m view cannot hold a {} m estate",
            extent.width_m().max(extent.depth_m())
        );
        // And it is looking at the middle of it, not at the patch corner.
        assert!(focus.x > 0.0 && focus.z < 0.0, "{focus:?}");
    }

    #[test]
    fn an_empty_estate_is_not_framed_at_all() {
        assert_eq!(framing(&EstateLayout::default()), None);
    }

    /// Straight down from well above a point of the estate frame.
    fn from_above(x: f32, z: f32) -> (Point, Point) {
        (Point::new(x, 100.0, z), Point::new(0.0, -1.0, 0.0))
    }

    #[test]
    fn a_click_on_a_machine_picks_the_machine_not_the_room_or_the_house() {
        // The machine is inside the room, which is inside the house: three
        // boxes under one click. The smallest is what was meant.
        let layout = layout();
        let machine = &things(&layout)[0];
        let (from, along) = from_above(machine.at.x, machine.at.z);
        assert_eq!(pick(&layout, &[], from, along), Some(Picked::Thing(0)));
    }

    #[test]
    fn a_click_on_bare_floor_picks_the_room_and_outside_it_the_building() {
        let layout = layout();
        // The computer room's floor between its two machines: the room
        // is 4 m square and they stand a metre in from opposite corners.
        let room = &rooms(&layout)[0];
        let (from, along) = from_above(room.at.x, room.at.z);
        assert_eq!(pick(&layout, &[], from, along), Some(Picked::Room(0)));

        // The tool shed has no rooms in this layout, so its roof is all
        // there is to pick.
        let shed = &buildings(&layout)[1];
        let (from, along) = from_above(shed.at.x, shed.at.z);
        assert_eq!(pick(&layout, &[], from, along), Some(Picked::Building(1)));
    }

    #[test]
    fn a_click_on_a_trunk_picks_the_tree_and_a_tree_in_front_of_a_room_wins() {
        let layout = layout();
        let room = &rooms(&layout)[0];
        // A 10 cm sapling standing on the open yard, well clear of the
        // buildings, is still clickable a few tens of centimetres off.
        let sapling = Stem {
            id: 41,
            base: Point::new(room.at.x - 40.0, 0.0, room.at.z),
            height_m: 3.0,
            radius_m: 0.05,
            shrub: false,
        };
        let (from, along) = from_above(sapling.base.x + 0.3, sapling.base.z);
        assert_eq!(
            pick(&layout, &[sapling], from, along),
            Some(Picked::Tree(41))
        );
        // Seen from the side, a tall tree between the eye and the room is
        // the one in front; with the eye between them, the room is.
        let oak = Stem {
            id: 42,
            base: Point::new(room.at.x - 10.0, 0.0, room.at.z),
            height_m: 20.0,
            radius_m: 0.4,
            shrub: false,
        };
        let along = Point::new(1.0, 0.0, 0.0);
        let behind_oak = Point::new(room.at.x - 30.0, room.at.y, room.at.z);
        assert_eq!(
            pick(&layout, &[oak], behind_oak, along),
            Some(Picked::Tree(42))
        );
        let past_oak = Point::new(room.at.x - 5.0, room.at.y, room.at.z);
        assert_eq!(
            pick(&layout, &[oak], past_oak, along),
            Some(Picked::Room(0))
        );

        // A tree behind the shed, seen past it, is hidden by it: the
        // shed is what the click meets first. Looked at from the south
        // (+z) towards the north, clear of the house to the west.
        let shed = &buildings(&layout)[1];
        let hidden = Stem {
            id: 43,
            base: Point::new(shed.at.x, 0.0, shed.at.z - 20.0),
            height_m: 20.0,
            radius_m: 0.4,
            shrub: false,
        };
        let north = Point::new(0.0, 0.0, -1.0);
        let south_of_shed = Point::new(shed.at.x, 1.0, shed.at.z + 20.0);
        assert_eq!(
            pick(&layout, &[hidden], south_of_shed, north),
            Some(Picked::Building(1))
        );
        // And with nothing in the way, the same tree is picked.
        let north_of_shed = Point::new(shed.at.x, 1.0, shed.at.z - 10.0);
        assert_eq!(
            pick(&layout, &[hidden], north_of_shed, north),
            Some(Picked::Tree(43))
        );
    }

    #[test]
    fn looking_inside_a_room_frames_the_room_not_the_estate() {
        let layout = layout();
        let room = &rooms(&layout)[0];
        let (focus, distance) = framing_of(&layout, &[], Picked::Room(0)).unwrap();
        assert_eq!(focus, room.at);
        // The 4 m room fits the 45-degree view, and is far closer than the
        // whole estate's framing.
        let across = 2.0 * distance * (std::f32::consts::FRAC_PI_8).tan();
        assert!(across >= 4.0, "a {across} m view cannot hold the 4 m room");
        assert!(distance < framing(&layout).unwrap().1);
        // A small thing is not looked at from inside its own box.
        let (_, near) = framing_of(&layout, &[], Picked::Thing(0)).unwrap();
        assert_eq!(near, MIN_LOOK_M);
        // A tree no longer drawn, or an index past the end, frames nothing.
        assert_eq!(framing_of(&layout, &[], Picked::Tree(9)), None);
        assert_eq!(framing_of(&layout, &[], Picked::Room(99)), None);
    }

    #[test]
    fn stepping_inside_a_room_keeps_the_eye_within_its_walls() {
        let layout = layout();
        let room = &rooms(&layout)[0];
        let (focus, distance) = inside(&layout, 0).unwrap();
        // Over the room's centre, half way up the 3 m homestead.
        assert_eq!((focus.x, focus.z), (room.at.x, room.at.z));
        assert!((focus.y - 1.5).abs() < 1e-4, "{}", focus.y);
        // The 4 m room: a 1 m distance, so a level eye turned any way is
        // inside it.
        assert!((distance - 1.0).abs() < 1e-4);
        for k in 0..8 {
            let orbit = crate::camera::Orbit {
                focus,
                distance,
                yaw: k as f32 * std::f32::consts::FRAC_PI_4,
                pitch: 0.0,
            };
            let eye = orbit.eye();
            assert!((eye.x - room.at.x).abs() < 2.0 && (eye.z - room.at.z).abs() < 2.0);
        }
        assert_eq!(inside(&layout, 9), None);
    }

    #[test]
    fn a_room_is_outlined_on_its_floor_and_a_zone_or_whole_building_is_not() {
        let mut layout = layout();
        layout.spaces.push(SpaceRect {
            id: 8,
            label: "Bay 1".to_string(),
            kind: "Zone".to_string(),
            rect_m: rect(113_975.0, 124_010.0, 113_978.0, 124_015.0),
        });
        let outlines = room_outlines(&layout);
        assert_eq!(outlines.len(), 1, "the zone has no walls");
        let (name, corners) = &outlines[0];
        assert_eq!(name, "Computer Room");
        // Round the room in order, each side its 4 m, on the floor.
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let side = ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
            assert!((side - 4.0).abs() < 1e-3, "side {i} is {side} m");
            // Above the floor's top face, which is at 1.5 thicknesses.
            assert!(a.y > 1.5 * FLOOR_THICKNESS_M, "{a:?} is inside the floor");
        }
    }

    #[test]
    fn a_door_is_marked_where_the_island_hangs_it_up_to_its_buildings_eaves() {
        let layout = layout();
        let marks = door_marks(&layout);
        assert_eq!(marks.len(), 3);
        let origin = (layout.patch.x0, layout.patch.y0);
        assert_eq!(marks[0].0, estate_of(origin, 113_966.0, 124_012.0, 0.0));
        // In the house, the shed, and neither: the island's two eaves
        // heights, and a storey for the gate in the open.
        assert_eq!(
            marks.iter().map(|m| m.1).collect::<Vec<_>>(),
            [3.0, 6.0, WALL_HEIGHT_M]
        );
    }

    #[test]
    fn a_click_on_the_open_yard_picks_nothing() {
        let layout = layout();
        // The yard's south-west corner, clear of both buildings.
        let corner = estate_of(
            (layout.patch.x0, layout.patch.y0),
            113_951.0,
            124_001.0,
            0.0,
        );
        let (from, along) = from_above(corner.x, corner.z);
        assert_eq!(pick(&layout, &[], from, along), None);
    }

    #[test]
    fn a_turned_box_is_hit_where_it_is_drawn_not_where_it_would_be_unturned() {
        // Four metres long and half a metre wide, turned a quarter: drawn
        // along z, so a point 1.5 m along z is on it and 1.5 m along x is
        // not. Unturned, those two answers swap.
        let beam = Placed {
            name: "Beam".to_string(),
            kind: "Beam".to_string(),
            at: Point::new(0.0, 0.5, 0.0),
            size_m: (4.0, 1.0, 0.5),
            rotation_deg: 90.0,
            model: None,
        };
        let (from, along) = from_above(0.0, 1.5);
        assert!(ray_hits(from, along, &beam).is_some());
        let (from, along) = from_above(1.5, 0.0);
        assert_eq!(ray_hits(from, along, &beam), None);

        let unturned = Placed {
            rotation_deg: 0.0,
            ..beam
        };
        let (from, along) = from_above(1.5, 0.0);
        assert!(ray_hits(from, along, &unturned).is_some());
        let (from, along) = from_above(0.0, 1.5);
        assert_eq!(ray_hits(from, along, &unturned), None);
    }

    #[test]
    fn a_ray_meets_a_box_at_its_near_face_and_never_behind_its_start() {
        let cube = Placed {
            name: "Cube".to_string(),
            kind: "Cube".to_string(),
            at: Point::new(0.0, 0.0, 0.0),
            size_m: (2.0, 2.0, 2.0),
            rotation_deg: 0.0,
            model: None,
        };
        let toward = ray_hits(
            Point::new(-10.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            &cube,
        );
        assert_eq!(toward, Some(9.0), "the near face is at x = -1");
        // The same line pointed the other way has the cube behind it.
        let away = ray_hits(
            Point::new(-10.0, 0.0, 0.0),
            Point::new(-1.0, 0.0, 0.0),
            &cube,
        );
        assert_eq!(away, None);
        // Starting inside, it is already there.
        let inside = ray_hits(Point::new(0.5, 0.0, 0.0), Point::new(0.0, 0.0, 1.0), &cube);
        assert_eq!(inside, Some(0.0));
    }

    #[test]
    fn the_inspector_says_what_the_island_says_about_what_was_picked() {
        let layout = layout();
        let machine = describe(&layout, Picked::Thing(0)).unwrap();
        assert_eq!(machine[0], "Gem-D's Computer (gaming, 3 monitors)");
        assert!(machine.contains(&"A computer".to_string()));
        assert!(machine.contains(&"In the Computer Room".to_string()));
        assert!(machine
            .iter()
            .any(|l| l == "0.50 m long, 0.25 m wide, 0.45 m high"));
        assert!(
            machine.iter().any(|l| l.contains("a desktop tower case")),
            "a figure comes with where it came from: {machine:?}"
        );

        let room = describe(&layout, Picked::Room(0)).unwrap();
        assert!(room.contains(&"4.0 x 4.0 m, 16.0 m² of floor".to_string()));
        assert!(
            room.iter().any(|l| l.starts_with("2 things in it")),
            "{room:?}"
        );
        assert!(room.contains(&"A door to outdoors".to_string()), "{room:?}");

        let house = describe(&layout, Picked::Building(0)).unwrap();
        assert!(house.contains(&"A house".to_string()));
        assert!(house.contains(&"Turned 30°".to_string()));
        assert!(house.contains(&"3.0 m to the eaves".to_string()));

        // A stale index is nothing, not a panic.
        assert_eq!(describe(&layout, Picked::Thing(99)), None);
    }

    #[test]
    fn a_kind_in_camel_case_is_said_in_words() {
        assert_eq!(words("ComputerRoom"), "computer room");
        assert_eq!(words("VehicleAttachment"), "vehicle attachment");
        assert_eq!(words("House"), "house");
    }
}
