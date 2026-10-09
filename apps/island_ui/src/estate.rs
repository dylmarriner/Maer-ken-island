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
use mk_island_api::{EstateLayout, Rect};

/// How tall a building is drawn, in metres.
///
/// The layout gives footprints and no heights, because the simulation has
/// no use for one: nothing on this island cares how tall the workshop is.
/// Three metres is a single storey and the key says it is a drawing
/// convention rather than a measurement.
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
    /// How big it is, in metres. For a building and a room this is the
    /// real footprint; for an item it is the size it is drawn at, which
    /// the simulation does not specify.
    pub size_m: (f32, f32, f32),
    /// Turned, in degrees, about the vertical.
    pub rotation_deg: f32,
    /// The model, if one is named for it. `is_really_this` false means a
    /// placeholder, and the key says so.
    pub model: Option<Model>,
}

/// How big a thing of each kind is drawn, in metres.
///
/// Invented, and labelled as invented: the island records that Gem-D owns
/// a computer and where it stands, not that it is 60 cm wide. These are
/// the sizes such a thing is, so a room full of them reads correctly, and
/// nothing downstream treats them as measurements.
///
/// Not an omission that could be fixed by reading harder, either.
/// `PropertyItem` in the engine carries `id`, `kind`, `name`,
/// `location`, `quantity`, `room` and `attaches_to`, and no dimension of
/// any sort -- nothing on this island cares how wide a workbench is, so
/// nothing records it. A renderer has to invent a size or draw nothing,
/// and inventing one *here*, where it is named as a drawing convention,
/// is the version that cannot be mistaken for the simulation's own
/// number: `PlacedItem` crosses the wire without a size field, so there
/// is nothing for a reader to confuse this with.
fn drawn_size(kind: &str) -> (f32, f32, f32) {
    match kind {
        "Computer" => (0.6, 0.6, 0.6),
        "HouseholdItem" => (0.6, 0.9, 0.6),
        "BuildingEquipment" => (1.2, 1.0, 0.8),
        "ArmouryItem" => (0.8, 1.2, 0.5),
        "Vehicle" => (4.8, 1.8, 2.0),
        "VehicleAttachment" => (2.0, 1.0, 1.5),
        "ShedTool" => (0.5, 1.0, 0.3),
        _ => (0.6, 0.6, 0.6),
    }
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
        .map(|building| Placed {
            name: building.name.clone(),
            kind: building.kind.clone(),
            at: centre(&building.rect_m, origin, f64::from(WALL_HEIGHT_M) / 2.0),
            size_m: (
                building.rect_m.width_m() as f32,
                WALL_HEIGHT_M,
                building.rect_m.depth_m() as f32,
            ),
            rotation_deg: building.rotation_deg as f32,
            model: Some(property::building(&building.kind)),
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

/// Everything standing on the estate, the four machines in the computer
/// room included.
pub fn things(layout: &EstateLayout) -> Vec<Placed> {
    let origin = (layout.patch.x0, layout.patch.y0);
    layout
        .items
        .iter()
        .map(|item| {
            let size = drawn_size(&item.kind);
            Placed {
                name: item.name.clone(),
                kind: item.kind.clone(),
                // Standing on the floor rather than sunk into it.
                at: estate_of(
                    origin,
                    item.position_m.0,
                    item.position_m.1,
                    f64::from(size.1) / 2.0 + f64::from(FLOOR_THICKNESS_M),
                ),
                size_m: size,
                rotation_deg: 0.0,
                model: Some(property::item(&item.kind, &item.name)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use mk_island_api::{BuildingFootprint, PlacedItem, SpaceRect};

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
                },
                BuildingFootprint {
                    name: "Tool Shed".to_string(),
                    kind: "Shed".to_string(),
                    rect_m: rect(113_975.0, 124_010.0, 113_981.0, 124_015.0),
                    rotation_deg: 0.0,
                },
            ],
            spaces: vec![SpaceRect {
                id: 7,
                label: "Computer Room".to_string(),
                kind: "Room".to_string(),
                rect_m: rect(113_964.0, 124_012.0, 113_968.0, 124_016.0),
            }],
            items: vec![
                PlacedItem {
                    name: "Gem-D's Computer (gaming, 3 monitors)".to_string(),
                    kind: "Computer".to_string(),
                    position_m: (113_965.0, 124_013.0),
                    space: Some("Computer Room".to_string()),
                },
                PlacedItem {
                    name: "Local Archive Server".to_string(),
                    kind: "Computer".to_string(),
                    position_m: (113_967.0, 124_015.0),
                    space: Some("Computer Room".to_string()),
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
    fn a_building_is_drawn_at_its_real_footprint() {
        let placed = buildings(&layout());
        let house = placed.iter().find(|p| p.kind == "House").unwrap();
        assert_eq!(house.size_m.0, 12.0, "the house is 12 m across");
        assert_eq!(house.size_m.2, 10.0, "and 10 m deep");
        assert_eq!(house.rotation_deg, 30.0, "and turned as the layout says");
        assert_eq!(house.size_m.1, WALL_HEIGHT_M);
        // Standing on the ground rather than half-buried in it.
        assert_eq!(house.at.y, WALL_HEIGHT_M / 2.0);
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
            let model = machine.model.expect("a machine has a model");
            assert!(model.is_really_this, "{} has no model", machine.name);
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
}
