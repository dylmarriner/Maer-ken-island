//! What the island looks like to anything that is not the simulation.
//!
//! Every type here was lifted out of the dashboard server's own projection
//! module when the frontend moved off the backend's machine. The comments
//! came with them, because what a field does not mean matters as much as
//! what it does — `Tree::kind` is not a species, `Economy::resource_nodes`
//! is a count that is always zero and says so — and a client reading these
//! needs that as much as the server writing them did.
//!
//! Every type is `Deserialize` as well as `Serialize`. The server only ever
//! writes them and the clients only ever read them, but they are compiled
//! from this one definition at both ends, so neither can drift from the
//! other without the other failing to build.

use serde::{Deserialize, Serialize};

/// The island at one moment: the body of `GET /api/world`.
///
/// The heavy parts of the island — every human's full record, a quarter of
/// a million stems, the properties, the economy, the timeline — are
/// deliberately **not** here. Each has its own endpoint, because a page
/// polling the clock once a second has no use for them and a desktop
/// client across a network has even less. `records_at_tick` and
/// `economy_at_tick` say how old those separate views are, so a frontend
/// can tell a reader that a record is an island-hour behind rather than
/// implying it is live.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct World {
    /// False before the first step has been published, so a frontend
    /// loaded in the first moments says so rather than drawing an empty
    /// island.
    pub running: bool,
    pub clock: Clock,
    pub people: Vec<Person>,
    pub estate: Estate,
    pub land: Land,
    pub stocks: Stocks,
    /// The tick the per-person records were last copied at.
    pub records_at_tick: u64,
    /// The tick the economy was last copied at.
    pub economy_at_tick: u64,
    pub digest: Digest,
}

/// A state digest and the moment it describes.
///
/// Two islands showing the same digest at the same tick are the same
/// island, which is what makes a frontend's reading worth comparing
/// against a headless run's.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Digest {
    pub value: String,
    pub at_tick: u64,
    /// True when this is the island as it is now rather than as it was at
    /// the last refresh, so a frontend never implies the digest is live
    /// when it is a few steps behind.
    pub current: bool,
}

/// The island's clock, and how fast it is being turned.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Clock {
    pub sim_time_s: u64,
    pub tick: u64,
    pub days: f64,
    pub hour_of_day: f64,
    /// 36 on this world, not 24. A frontend that assumed Earth's day would
    /// draw the sun in the wrong place for a third of it.
    pub day_length_hours: f64,
    /// Simulated seconds per real second, as measured over the last steps.
    /// `None` until enough steps have run to measure one.
    pub achieved_speed: Option<f64>,
    /// What was asked for: `real`, `max`, or a multiplier. Kept beside the
    /// achieved figure rather than instead of it, because an island that
    /// cannot keep up with what was asked should say so.
    pub requested_speed: String,
}

/// One islander, as a map and a roster see them.
///
/// There is no `name`: a human in the world has an `agent_id` and nothing
/// else to be called. Inventing a display name here would be fabricated
/// state. The id is what there is, so the id is what is shown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub agent_id: String,
    pub alive: bool,
    pub asleep: bool,
    pub age_years: f64,
    /// Where they are on the estate, by the layout's own label for the
    /// space — "Bedroom", "Workshop" — rather than a space id. Absent when
    /// they are off the estate patch entirely.
    pub space: Option<String>,
    pub position_m: Option<(f64, f64)>,
    /// The medium-grid cell they stand on, so a map can draw them.
    ///
    /// Served rather than derived by the frontend: turning metres into a
    /// cell is the server's arithmetic, and the one time a coordinate
    /// conversion happened somewhere it did not belong, a birth came out
    /// 0.716 degrees from the equator.
    pub cell: Option<(usize, usize)>,
    /// Carbon in the body (kg), the measure the island's own acceptance
    /// test holds the founders to.
    pub body_carbon_kg: Option<f64>,
    /// How tall they are, in metres -- the island's own figure, drawn for
    /// each person from a sex-specific distribution when they were made.
    ///
    /// Served so a renderer stands a person at their own height rather
    /// than one it chose for everybody. Every frontend used to put every
    /// head at 1.7 m.
    ///
    /// `#[serde(default)]` for backends written before this was sent; it
    /// reads 0.0 there, which a frontend treats as "not said" rather than
    /// as somebody with no height.
    #[serde(default)]
    pub height_m: f64,
}

/// The founders' estate.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Estate {
    /// Medium-grid cell of the estate's block.
    pub cell: (usize, usize),
    /// Where that cell is on the planet, in **degrees** — the units the
    /// intervention and creation forms take. `IslandDomain::lat_lon_at_m`
    /// answers in radians, and passing those through as degrees is a
    /// mistake this code has already made once.
    pub latitude: f64,
    pub longitude: f64,
    pub buildings: usize,
    pub items: usize,
    /// Every space somebody can be put in, by the layout's own id and
    /// label. A frontend offers exactly these, so it can never ask for a
    /// room the island will refuse.
    pub spaces: Vec<SpaceChoice>,
    pub battery_charge_kwh: f64,
    pub battery_capacity_kwh: f64,
    pub fuel_litres: f64,
}

/// A place on the estate a person can be created in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpaceChoice {
    pub id: u32,
    pub label: String,
    pub kind: String,
}

/// The ground the estate stands on, and the island around it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Land {
    pub trees: usize,
    pub patch_carbon_kgc: f64,
    pub island_biomass_kgc: f64,
    /// The medium grid's shape, so a frontend offering a cell can say what
    /// the numbers may be rather than letting somebody guess and be
    /// refused.
    pub rows: usize,
    pub cols: usize,
    /// How many of those cells are land. The rest are sea, and nobody can
    /// be created there.
    pub land_cells: usize,
    /// How wide one of those cells is, in metres, so a frontend can turn
    /// domain metres into picture pixels without holding a copy of the
    /// engine's 2 km and hoping it never changes.
    pub cell_size_m: f64,
}

/// What the household has moved, and whether its books closed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stocks {
    pub human_carbon_kg: f64,
    pub material_carbon_kg: f64,
    pub material_water_kg: f64,
    pub audits_closed: u64,
    pub food_shortfalls: u64,
    pub water_shortfalls: u64,
}

/// A property: who owns it, where it stands, what is on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub owners: Vec<String>,
    /// The medium cell it stands on, when it has a fixed place.
    pub cell: Option<(usize, usize)>,
    /// Whether it stands on this island's estate cell. The inventory
    /// carries an unowned homestead template with no location, and saying
    /// which is which is cheaper than leaving a reader to work it out.
    pub on_this_island: bool,
    pub buildings: Vec<Building>,
    pub items: Vec<Item>,
}

/// A building on a property. `kind` is a `PropertyBuildingKind` in words:
/// `House`, `Shed`, `Workshop`, `Armoury`, `ComputerRoom`, `Garage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub name: String,
    pub kind: String,
}

/// A thing on a property. `kind` is a `PropertyItemKind` in words:
/// `Vehicle`, `VehicleAttachment`, `ShedTool`, `BuildingEquipment`,
/// `ArmouryItem`, `Computer`, `HouseholdItem`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub kind: String,
}

/// The resource economy, as far as the island has one.
///
/// `resource_nodes` is a count rather than a list because it is always
/// zero: the island does not seed resource nodes from its biomes the way a
/// planetary world does, so there is nothing to list. It is reported rather
/// than hidden, because "no nodes" is a fact about the island and leaving
/// the field out would make an empty economy look like a missing endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Economy {
    pub resource_nodes: usize,
    pub structures: Vec<Structure>,
    pub events: Vec<EconomyEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Structure {
    pub id: u64,
    pub cell: (i32, i32),
    pub recipe: String,
    pub material_cost: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomyEntry {
    pub tick: u64,
    pub by: String,
    pub kind: String,
    pub subject: String,
    pub quantity: u32,
}

/// One thing that reached the island from outside, in the order it applied.
///
/// This is the replay log the island actually keeps, not a history of what
/// the islanders did: nothing here is a person going to bed or felling a
/// tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub tick: u64,
    pub sequence: u64,
    pub what: String,
}

/// One thing a person said.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationLine {
    pub speaker_id: String,
    pub speaker_name: String,
    pub text: String,
}

/// A conversation between two islanders, as it happened.
///
/// The engine generates these from state it has already computed — the
/// speaker's real emotion, their actual internal monologue, what the
/// listener said to them last time — and never from a language model. A
/// frontend shows the lines as the engine wrote them: a conversation
/// rewritten on the way to the screen is no longer evidence of what the
/// simulation did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    /// When the two of them started talking.
    pub tick: u64,
    /// When the most recent line was said. Equal to `tick` for an
    /// exchange that lasted one tick; an exchange that carried on is one
    /// conversation spanning `tick..=last_tick`, not one per tick.
    #[serde(default)]
    pub last_tick: u64,
    /// `founders`, `parent and child`, `siblings` or `neighbours` — the
    /// register the engine picked, in words rather than an enum name.
    pub relationship: String,
    /// The most recent lines of the exchange, newest last — at most
    /// `LINES_SHOWN` of them. A conversation that ran all day holds
    /// about a thousand lines and nothing useful is served by sending
    /// them all to a dashboard card.
    pub lines: Vec<ConversationLine>,
    /// How many lines the whole exchange holds, which is what `lines` is
    /// a tail of. A frontend that shows fewer lines than this should say
    /// so rather than let a reader think the conversation was short.
    ///
    /// `#[serde(default)]` for backends written before conversations
    /// could span ticks; those send no field and it reads 0, which a
    /// frontend treats as "no count given" rather than "no lines".
    #[serde(default)]
    pub lines_said: usize,
}

/// One tree, as a map draws it.
///
/// Position is domain metres, the same frame everything else on the map
/// uses. `kind` is `Tree`, `Shrub` or `Grass` and **not a species**: the
/// engine's `TreeInstance` carries no species, and the species system in
/// `organisms/runtime.rs` is not wired into the island's vegetation.
/// Island-scale species is Phase 3 Task 1b, still open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tree {
    pub id: u64,
    pub kind: String,
    pub x_m: f64,
    pub y_m: f64,
    pub height_m: f64,
    pub stem_diameter_m: f64,
}

/// Where individual trees exist, and how many are in view.
///
/// The island does not model every tree. Within `individual_radius_m` of
/// `individual_centre_m` the engine holds a stem per tree above a minimum
/// diameter; outside it — including the rest of the 4 km patch and the
/// whole island beyond — vegetation is stand cover and biomass per cell. A
/// frontend that did not say so would let somebody read an empty island as
/// a treeless one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Trees {
    pub trees: Vec<Tree>,
    /// How many stood in the requested box, before the cap.
    pub in_box: usize,
    /// How many are being returned.
    pub shown: usize,
    /// The whole patch: south-west corner and size, in domain metres.
    pub patch: (f64, f64, f64, f64),
    /// Centre and radius of the individually-modelled area, domain metres.
    pub individual_centre_m: (f64, f64),
    pub individual_radius_m: f64,
    /// The estate's yard, west, south, east and north in domain metres: a
    /// cleared rectangle the engine places no stem inside, so the middle
    /// of the wood is a hole rather than an unexplained empty square.
    pub yard: (f64, f64, f64, f64),
    /// Every individual stem the island holds, in or out of view.
    pub total: usize,
}

/// One cell of the island, for somebody who clicked on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    /// Degrees, the same units the intervention and creation forms take,
    /// so a click can fill them in directly.
    pub latitude: f64,
    pub longitude: f64,
    pub land: bool,
    pub elevation_m: f64,
    /// Whether a structure could stand here, by the island's own rule
    /// (`MAX_BUILD_GRADIENT`) rather than upstream's, which admits nothing
    /// on a 2 km grid. Sea is never buildable.
    pub buildable: bool,
}

/// A rectangle on the estate patch, in domain metres: west, south, east,
/// north.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn width_m(&self) -> f64 {
        (self.x1 - self.x0).abs()
    }

    pub fn depth_m(&self) -> f64 {
        (self.y1 - self.y0).abs()
    }

    pub fn centre_m(&self) -> (f64, f64) {
        ((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0)
    }
}

/// A building on the estate, where it stands and how big it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingFootprint {
    pub name: String,
    /// A `PropertyBuildingKind` in words: `House`, `Shed`, `Workshop`,
    /// `Armoury`, `ComputerRoom`, `Garage`.
    pub kind: String,
    pub rect_m: Rect,
    pub rotation_deg: f64,
    /// How tall it is, in metres, to the eaves.
    ///
    /// The island's answer, not a renderer's convention. Every frontend
    /// drew buildings at a flat three metres before this, which put a
    /// 3.606 m Fendt 1000 Vario inside a 3 m shed and said nothing.
    #[serde(default)]
    pub height_m: f64,
    /// Where that height came from, in words a reader can check.
    #[serde(default)]
    pub height_source: String,
}

/// A room or zone inside a building, by the layout's own label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpaceRect {
    pub id: u32,
    pub label: String,
    /// `Room`, `Zone` or `WholeBuilding`.
    pub kind: String,
    pub rect_m: Rect,
}

/// A thing on the estate, at the metres it stands on.
///
/// The join between the layout's placements and the property inventory is
/// done on the server: a placement carries an item id and a client would
/// otherwise have to fetch the inventory and match them up to find out
/// that the thing at these metres is a computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedItem {
    pub name: String,
    /// A `PropertyItemKind` in words: `Vehicle`, `VehicleAttachment`,
    /// `ShedTool`, `BuildingEquipment`, `ArmouryItem`, `Computer`,
    /// `HouseholdItem`.
    pub kind: String,
    pub position_m: (f64, f64),
    /// The space it is in, by the layout's own label, or `None` outdoors.
    pub space: Option<String>,
    /// How big it really is: length, width, height in metres, along its
    /// own axes.
    ///
    /// The island's answer rather than a renderer's guess. Every frontend
    /// drawing this estate used to invent a size, which meant two of them
    /// could draw the same truck differently and neither would be wrong
    /// about anything the island had said. Now the island says.
    #[serde(default)]
    pub size_m: (f64, f64, f64),
    /// Where that figure came from, in words a reader can check: a
    /// citation for a named thing, or a sentence beginning "convention:"
    /// for one the table knows only by kind.
    ///
    /// Carried on the wire rather than kept server-side because a
    /// frontend showing a measurement should be able to show where it
    /// came from, and because a figure whose provenance is one hop away
    /// is a figure nobody checks.
    #[serde(default)]
    pub size_source: String,
}

/// The founders' estate as geometry: what stands where, in metres.
///
/// Served on its own rather than with the rest of the world because it
/// does not change -- the buildings are placed at bootstrap and nothing in
/// Phase 4 moves one -- and because it is the one thing a renderer needs
/// before it can draw the estate at all. A frontend fetches it once.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EstateLayout {
    /// The patch's south-west corner and size in domain metres, which is
    /// the floating origin an estate-scale frame measures from.
    pub patch: Rect,
    /// The cleared ground around the buildings. The engine places no stem
    /// inside it, so a renderer that did not know would draw a wood with
    /// an unexplained hole.
    pub yard: Rect,
    pub buildings: Vec<BuildingFootprint>,
    pub spaces: Vec<SpaceRect>,
    pub items: Vec<PlacedItem>,
}

impl EstateLayout {
    /// Everything in one space, for an interior view.
    pub fn items_in<'a>(&'a self, label: &'a str) -> impl Iterator<Item = &'a PlacedItem> + 'a {
        self.items
            .iter()
            .filter(move |item| item.space.as_deref() == Some(label))
    }

    /// The ground the estate covers, as a rectangle around everything on
    /// it. `None` when there is nothing.
    pub fn extent_m(&self) -> Option<Rect> {
        let mut found = false;
        let mut extent = Rect {
            x0: f64::MAX,
            y0: f64::MAX,
            x1: f64::MIN,
            y1: f64::MIN,
        };
        // A rectangle with no area is not ground. Without this check a
        // default-constructed layout -- no buildings, a yard of zeroes --
        // reports an extent at the origin, and a camera asked to frame
        // the estate obligingly frames nothing.
        let real = |r: &Rect| r.width_m() > 0.0 && r.depth_m() > 0.0;
        for rect in self
            .buildings
            .iter()
            .map(|b| b.rect_m)
            .chain([self.yard])
            .filter(real)
        {
            found = true;
            extent.x0 = extent.x0.min(rect.x0.min(rect.x1));
            extent.y0 = extent.y0.min(rect.y0.min(rect.y1));
            extent.x1 = extent.x1.max(rect.x0.max(rect.x1));
            extent.y1 = extent.y1.max(rect.y0.max(rect.y1));
        }
        found.then_some(extent)
    }
}

#[cfg(test)]
mod estate_tests {
    use super::*;

    #[test]
    fn an_estate_with_nothing_on_it_covers_no_ground() {
        // A default layout has a yard of zeroes. Counting it would give an
        // extent at the origin, and anything that framed the estate would
        // frame nothing and look as though it had worked.
        assert_eq!(EstateLayout::default().extent_m(), None);
    }

    #[test]
    fn the_extent_holds_every_building_and_the_yard() {
        let layout = EstateLayout {
            yard: Rect {
                x0: 10.0,
                y0: 10.0,
                x1: 30.0,
                y1: 30.0,
            },
            buildings: vec![BuildingFootprint {
                name: "Shed".to_string(),
                kind: "Shed".to_string(),
                rect_m: Rect {
                    x0: 35.0,
                    y0: 5.0,
                    x1: 41.0,
                    y1: 11.0,
                },
                rotation_deg: 0.0,
                height_m: 6.0,
                height_source: "a machinery shed".to_string(),
            }],
            ..Default::default()
        };
        let extent = layout.extent_m().expect("there is ground here");
        assert_eq!(
            extent,
            Rect {
                x0: 10.0,
                y0: 5.0,
                x1: 41.0,
                y1: 30.0
            }
        );
    }
}
