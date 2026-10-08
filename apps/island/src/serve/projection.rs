//! Phase 4 Task 6: what the dashboard is allowed to see of the island.
//!
//! The simulation owns its state on one thread and nothing else may touch
//! it. After each step it publishes one of these — a plain, serializable
//! description of the world at that moment — and every GET handler reads
//! only this. A request can never block a step, and a half-stepped island
//! can never be rendered.
//!
//! What goes in is what somebody looking at the page needs. The whole state
//! is far too large to publish on every step (a quarter of a million stems
//! alone), so the vegetation is a count and a mass, not a list of trees.

use std::collections::BTreeMap;
use std::sync::Arc;

use mk_engine::humans::dialogue::ConversationRelationship;
use mk_engine::humans::HumanBeing;
use mk_engine::regional::estate_layout::Space;
use mk_engine::regional::life::IslandLife;
use serde::Serialize;

/// How many of the island's recent conversations the dashboard shows.
///
/// `HumanSystem` keeps 500. Serving all of them would be a wall of text
/// nobody reads, and the ones worth reading are the recent ones, so the
/// page takes this many from the newest end and says so.
pub const CONVERSATIONS_SHOWN: usize = 40;

/// The island at one moment, as the dashboard sees it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct IslandProjection {
    /// False before the first step has been published, so a page loaded in
    /// the first moments says so rather than showing an empty island.
    pub running: bool,
    pub clock: Clock,
    pub people: Vec<Person>,
    pub estate: Estate,
    pub land: Land,
    pub stocks: Stocks,
    /// Every islander's full canonical record, by agent id.
    ///
    /// Not in the JSON: this is what the per-person page reads, one at a
    /// time, and dumping a whole population into every `/api/world` reply
    /// would be absurd. Behind an `Arc` because a projection is cloned on
    /// every read and these are large.
    ///
    /// Refreshed on the digest's cadence rather than every step, and that
    /// is a choice about scale rather than a present necessity: measured,
    /// copying every record costs 336 µs for the two founders — about
    /// 170 µs each — against a **4.3 ms** step on the real island
    /// (`benchmarks/phase4_cost.md`; the 1.2 ms once quoted here was
    /// measured on a 50-tree patch). Cheap now; at the town-sized
    /// population of Phase 4b it would be tens of milliseconds per step for
    /// records nobody is reading. `records_at_tick` says how old they are
    /// and the page says so too, so the cadence can change later without
    /// anything having been claimed that was not true.
    ///
    /// A creation refreshes them immediately regardless, because creating
    /// somebody and then being told they do not exist would be absurd.
    #[serde(skip)]
    pub records: Arc<BTreeMap<String, HumanBeing>>,
    pub records_at_tick: u64,
    /// The estate's properties: buildings, items, who owns them and where
    /// they stand.
    ///
    /// Behind an `Arc` and built once, because this does not change: the
    /// properties are placed at bootstrap and nothing in Phase 4 moves a
    /// building or sells a quad bike. When something does, this becomes a
    /// per-step or per-cadence refresh like `records`, and the shape here
    /// does not have to change for that.
    #[serde(skip)]
    pub properties: Arc<Vec<Property>>,
    /// The resource economy: what has been built and what it recorded.
    ///
    /// Refreshed on the digest's cadence, like `records`, and refreshed at
    /// once when a command lands — a structure an operator just built
    /// should be there when they look, not an hour of island time later.
    #[serde(skip)]
    pub economy: Arc<Economy>,
    pub economy_at_tick: u64,
    /// Everything that has reached the island from outside, in order.
    ///
    /// Refreshed when a command lands rather than on a cadence, because
    /// that is exactly when it changes — and it is the one view where being
    /// an hour of island time behind would be plainly wrong, since a person
    /// looks at it to see what they just did.
    #[serde(skip)]
    pub timeline: Arc<Vec<TimelineEntry>>,
    /// The island's recent conversations, newest first.
    ///
    /// Skipped from the status body like the other heavy views: a page
    /// polling the clock every second has no use for forty conversations
    /// with it, and `/api/conversations` serves them on their own.
    #[serde(skip)]
    pub conversations: Arc<Vec<Conversation>>,
    /// Every individual stem the island holds, republished on its own
    /// cadence -- `TREES_EVERY`, a simulated day, rather than the hourly
    /// one the records and the economy ride. The patch holds about
    /// 200,000 of them at 56 bytes apiece, so this copy is eleven
    /// megabytes where the conversation feed is forty short strings;
    /// `tests/tree_cost.rs` measures it.
    ///
    /// Skipped from the status body, and harder than the other skips: a
    /// page polling the clock every second must not be handed two hundred
    /// thousand stems with it. `/api/trees` serves the ones inside a box.
    #[serde(skip)]
    pub trees: Arc<Vec<mk_engine::regional::local_vegetation::TreeInstance>>,
    /// The canonical state digest, and the tick it was taken at.
    ///
    /// Two islands showing the same digest at the same tick are the same
    /// island, which is what makes a dashboard reading worth comparing
    /// against a headless run's. It is not refreshed every step: hashing
    /// it walks a quarter of a million stems and two 1,152,000-cell grids,
    /// which measured at about 580 ms against a 3 ms step — publishing it
    /// every step made the island run two hundred times slower than it
    /// needed to, for a number nobody reads that often.
    pub digest: Digest,
}

/// A state digest and the moment it describes.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Digest {
    pub value: String,
    pub at_tick: u64,
    /// True when this is the island as it is now rather than as it was at
    /// the last refresh, so a page never implies the digest is live when
    /// it is a few steps behind.
    pub current: bool,
}

/// Simulated time. The island's day is the canon's rotation period — 36
/// hours — so a clock that counted in 24s would be wrong here.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Clock {
    pub sim_time_s: u64,
    pub tick: u64,
    pub days: f64,
    /// Hours into the local day, 0 up to the day's own length.
    pub hour_of_day: f64,
    pub day_length_hours: f64,
    /// How many simulated seconds are passing per real second, measured
    /// over recent steps. `None` until enough steps have run to say.
    pub achieved_speed: Option<f64>,
    /// What was asked for, so a page can show that the island is not
    /// keeping up rather than quietly running slow.
    pub requested_speed: String,
}

/// One person, at the level of detail a roster shows.
///
/// There is no `name`: a human in the world has an `agent_id` and nothing
/// else to be called. The dashboard's own creator keeps a side-table of
/// typed-in names for the people it made, but the island's founders were
/// never typed in, and inventing a display name here would be exactly the
/// fabricated state `HUMAN_SCOPE.md` says not to add to make a page look
/// finished. The id is what there is, so the id is what is shown.
#[derive(Debug, Clone, Serialize)]
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
    /// The medium-grid cell they stand on, so the map can draw them.
    ///
    /// Served rather than derived on the page: turning metres into a cell
    /// is the server's arithmetic, and the one time this branch let a
    /// coordinate conversion happen somewhere it did not belong, a birth
    /// came out 0.716 degrees from the equator.
    pub cell: Option<(usize, usize)>,
    /// Carbon in the body (kg), which is the measure the island's own
    /// acceptance test holds the founders to.
    pub body_carbon_kg: Option<f64>,
}

/// The founders' estate.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Estate {
    /// Medium-grid cell of the estate's block.
    pub cell: (usize, usize),
    /// Where that cell is on the planet, in **degrees**.
    ///
    /// Here because an intervention is aimed with an upstream `Location`,
    /// which is a latitude and longitude, and a page that could only name
    /// cells had no way to ask for anywhere at all. Degrees, not radians:
    /// `IslandDomain::lat_lon_at_m` answers in radians because its other
    /// caller does trigonometry with the result, and passing those through
    /// as degrees is a mistake this code has already made once.
    pub latitude: f64,
    pub longitude: f64,
    pub buildings: usize,
    pub items: usize,
    /// Every space somebody can be put in, by the layout's own id and
    /// label. The creator page offers exactly these, so it can never ask
    /// for a room the island will refuse.
    pub spaces: Vec<SpaceChoice>,
    pub battery_charge_kwh: f64,
    pub battery_capacity_kwh: f64,
    pub fuel_litres: f64,
}

/// A place on the estate a person can be created in.
#[derive(Debug, Clone, Serialize)]
pub struct SpaceChoice {
    pub id: u32,
    pub label: String,
    pub kind: String,
}

/// The ground the estate stands on, and the island around it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Land {
    pub trees: usize,
    pub patch_carbon_kgc: f64,
    pub island_biomass_kgc: f64,
    /// The medium grid's shape, so a page offering a cell can say what the
    /// numbers may be rather than letting somebody guess and be refused.
    pub rows: usize,
    pub cols: usize,
    /// How many of those cells are land. The rest are sea, and nobody can
    /// be created there.
    pub land_cells: usize,
    /// How wide one of those cells is, in metres.
    ///
    /// Here so the viewer can turn domain metres into picture pixels. The
    /// map is one pixel per cell and the trees are in metres, and without
    /// this the page would have to hold a copy of the engine's 2 km and
    /// hope it never changes.
    pub cell_size_m: f64,
}

/// What the household has moved, and whether its books closed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Stocks {
    pub human_carbon_kg: f64,
    pub material_carbon_kg: f64,
    pub material_water_kg: f64,
    pub audits_closed: u64,
    pub food_shortfalls: u64,
    pub water_shortfalls: u64,
}

/// The parts of a projection that are refreshed on their own schedule
/// rather than rebuilt every step.
///
/// Together rather than as six parameters, because they travel together:
/// the loop holds exactly this and hands it over whole, and `of` would
/// otherwise take eleven arguments nobody could read in order.
#[derive(Clone)]
pub struct Views {
    pub records: Arc<BTreeMap<String, HumanBeing>>,
    pub records_at_tick: u64,
    pub conversations: Arc<Vec<Conversation>>,
    pub trees: Arc<Vec<mk_engine::regional::local_vegetation::TreeInstance>>,
    pub properties: Arc<Vec<Property>>,
    pub economy: Arc<Economy>,
    pub economy_at_tick: u64,
    pub timeline: Arc<Vec<TimelineEntry>>,
}

impl Views {
    /// Every view, built fresh. Used once at startup; the loop then
    /// refreshes each part on its own schedule.
    pub fn of(life: &IslandLife) -> Self {
        Self {
            records: IslandProjection::records_now(life),
            records_at_tick: life.tick,
            conversations: IslandProjection::conversations_now(life),
            trees: IslandProjection::trees_now(life),
            properties: IslandProjection::properties_now(life),
            economy: IslandProjection::economy_now(life),
            economy_at_tick: life.tick,
            timeline: IslandProjection::timeline_now(life),
        }
    }
}

/// What a recipe is called, for somebody reading rather than parsing.
///
/// `StructureKind::display_name` already exists for exactly this, and the
/// four buildable recipes are the four structure kinds, so a page shows
/// "Wooden Shelter" rather than the enum's `WoodenShelter`. Anything that
/// is not one of those four falls back to its own name, which is still
/// better than nothing and does not pretend to a nicety it has not got.
fn recipe_name(recipe: mk_engine::resource_economy::RecipeId) -> String {
    use mk_engine::resource_economy::RecipeId;
    use mk_interventions::StructureKind;
    match recipe {
        RecipeId::WoodenShelter => StructureKind::WoodenShelter.display_name().to_string(),
        RecipeId::Workshop => StructureKind::Workshop.display_name().to_string(),
        RecipeId::Storage => StructureKind::Storage.display_name().to_string(),
        RecipeId::StoneHouse => StructureKind::StoneHouse.display_name().to_string(),
        other => format!("{other:?}"),
    }
}

/// An economy event's subject, named the way the rest of the page names it.
///
/// `subject` is free text the economy writes for itself, and for a
/// construction it is a `RecipeId` through `{:?}`. Left alone, the same
/// building read "Stone House" in the structures table and "StoneHouse" one
/// table below it, which is the sort of thing that makes a reader wonder
/// whether they are looking at two different things. Only the four
/// buildable structures are renamed, and only for display: the event
/// itself is untouched.
fn readable_subject(subject: &str) -> String {
    mk_interventions::StructureKind::ALL
        .iter()
        .find(|kind| format!("{kind:?}") == subject)
        .map(|kind| kind.display_name().to_string())
        .unwrap_or_else(|| subject.to_string())
}

/// A property on the island, or belonging to it.
#[derive(Debug, Clone, serde::Serialize)]
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct Building {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Economy {
    pub resource_nodes: usize,
    pub structures: Vec<Structure>,
    pub events: Vec<EconomyEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Structure {
    pub id: u64,
    pub cell: (i32, i32),
    pub recipe: String,
    pub material_cost: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EconomyEntry {
    pub tick: u64,
    pub by: String,
    pub kind: String,
    pub subject: String,
    pub quantity: u32,
}

/// One thing that reached the island from outside.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineEntry {
    pub tick: u64,
    pub sequence: u64,
    pub what: String,
}

/// A command in a line, for a timeline a person reads.
fn describe(command: &mk_engine::regional::commands::IslandCommand) -> String {
    use mk_engine::regional::commands::{ControlCommand, IslandCommand};
    match command {
        IslandCommand::CreateHuman(request) => {
            format!("created {}", request.name)
        }
        IslandCommand::Intervention(action) => {
            mk_engine::regional::interventions::action_name(action).to_string()
        }
        IslandCommand::Control(control) => match control {
            ControlCommand::Pause => "paused".to_string(),
            ControlCommand::Resume => "resumed".to_string(),
            ControlCommand::Step(n) => format!("stepped {n}"),
            // Only recorded once a snapshot has actually been written
            // (`sim::apply` saves first and records second), so the past
            // tense is earned. It did not used to be: this line read the
            // same while nothing anywhere wrote a snapshot.
            ControlCommand::Snapshot => "wrote a snapshot".to_string(),
            ControlCommand::SetSpeed(speed) => format!("set the speed to {speed}"),
        },
    }
}

/// One thing a person said, in a conversation somebody can read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationLine {
    pub speaker_id: String,
    pub speaker_name: String,
    pub text: String,
}

/// A conversation between two islanders, as it happened.
///
/// The engine generates these from state it has already computed — the
/// speaker's real emotion, their actual internal monologue, what the
/// listener said to them last time — and never from a language model. The
/// page shows the lines as the engine wrote them, for the same reason the
/// server notes are shown rather than paraphrased: a conversation
/// rewritten on the way to the screen is no longer evidence of what the
/// simulation did.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Conversation {
    pub tick: u64,
    /// `founders`, `parent and child`, `siblings` or `neighbours` — the
    /// register the engine picked, in words rather than an enum name.
    pub relationship: String,
    pub lines: Vec<ConversationLine>,
}

/// How the two speakers are related, for somebody reading rather than
/// parsing. `Other` is every pairing that is not kin, which on this island
/// means two people who were near each other and one of them reached out,
/// so "neighbours" says what it is without claiming more.
fn relationship_name(relationship: ConversationRelationship) -> &'static str {
    match relationship {
        ConversationRelationship::Founders => "founders",
        ConversationRelationship::ParentChild => "parent and child",
        ConversationRelationship::Siblings => "siblings",
        ConversationRelationship::Other => "neighbours",
    }
}

/// The island's terrain, copied once so a click can be answered without
/// disturbing the simulation.
///
/// Terrain is the one part of the world that cannot change -- the island
/// refuses `SculptTerrain` and `SmoothTerrain` -- which is what makes a
/// copy safe. Everything else the dashboard shows goes through the
/// published projection precisely because it does change.
#[derive(Debug)]
pub struct Terrain {
    pub domain: mk_island::IslandDomain,
    pub elevation_m: mk_core::grid::Grid2<f64>,
    pub land_mask: mk_core::grid::Grid2<bool>,
    /// The high-detail patch: south-west corner and size, domain metres.
    /// Fixed by the scenario, so it belongs here with the terrain.
    pub patch: (f64, f64, f64, f64),
    pub individual_centre_m: (f64, f64),
    pub individual_radius_m: f64,
    /// The estate's yard, as domain metres: west, south, east, north.
    ///
    /// Here because it is a hole in the wood. `seed_local_vegetation`
    /// refuses to place a stem inside the yard, and the individual radius
    /// is measured from the yard's own centre, so zooming to the middle of
    /// the individually-modelled wood lands you in a clearing with nothing
    /// in it. A viewer that did not know about the yard could only show
    /// that as an unexplained empty square.
    pub yard: (f64, f64, f64, f64),
}

impl Terrain {
    /// Copy the island's terrain. Called once, at startup.
    pub fn of(life: &IslandLife) -> Self {
        Self {
            domain: life.domain.clone(),
            elevation_m: life.physical.geophysics.elevation_m.clone(),
            land_mask: life.physical.geophysics.land_mask.clone(),
            patch: {
                let p = life.vegetation.patch_spec();
                (p.origin_x_m, p.origin_y_m, p.width_m, p.height_m)
            },
            individual_centre_m: life.vegetation.centre_m(),
            individual_radius_m: life.vegetation.individual_radius_m,
            yard: {
                let y = life.placed.layout.yard;
                (y.x0, y.y0, y.x1, y.y1)
            },
        }
    }
}

/// A stem's id, stirred, so that taking one in n of them samples the wood
/// rather than the order it was generated in.
///
/// SplitMix64's finalising mix, which is a few multiplies and shifts and
/// spreads neighbouring ids right across the range. It is not randomness
/// and must not be: the same stem hashes the same way on every request and
/// in every process, so two people looking at the same box see the same
/// trees, and so does a test.
fn scattered(id: u64) -> u64 {
    let mut z = id.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Individual trees inside a box of domain metres, thinned to `cap`.
///
/// Capped because the island holds about 200,000 stems and a viewer zoomed
/// out over all of them wants a sample, not the lot: 200,000 of anything
/// is megabytes and draws as a solid block anyway.
///
/// Thinned on a hash of the stem's id rather than on its place in the
/// list, and that is not fussiness -- both of the obvious ways are wrong.
///
/// The stems are generated in an order that tracks position, so taking the
/// first `cap` of them takes a patch of ground rather than a sample of the
/// wood. Taking every nth fixes that and buys a worse problem, which the
/// viewer showed rather than argued: the generation order is near-periodic
/// in space, a stride beats against that period, and a 4 km view of the
/// estate came out in vertical stripes and chevrons that are nowhere in
/// the island. A hash has no period to beat against. It is still a pure
/// function of the stem and not randomness, so the same box always answers
/// with the same trees -- for two people looking at one wood, and for a
/// test.
///
/// Zoom in far enough that fewer than `cap` stand in view and the thinning
/// stops: every stem in the box comes back, which is the point of the
/// whole thing.
///
/// `in_box` says how many really stood there, so a thinned view can never
/// be read as a thin wood.
pub fn trees_in(
    terrain: &Terrain,
    all: &[mk_engine::regional::local_vegetation::TreeInstance],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    cap: usize,
) -> Trees {
    let (lo_x, hi_x) = (x0.min(x1), x0.max(x1));
    let (lo_y, hi_y) = (y0.min(y1), y0.max(y1));
    let in_view = |t: &&mk_engine::regional::local_vegetation::TreeInstance| {
        t.position_m.0 >= lo_x
            && t.position_m.0 <= hi_x
            && t.position_m.1 >= lo_y
            && t.position_m.1 <= hi_y
    };

    let mut total = 0usize;
    let mut in_box = 0usize;
    for t in all.iter().filter(|t| t.alive) {
        total += 1;
        if in_view(&t) {
            in_box += 1;
        }
    }

    // One stem in `stride` is kept, chosen by its hash, so the sample is
    // spread over the box instead of over the list. The `take` is the hard
    // cap: a hash keeps about `in_box / stride` of them rather than exactly
    // that many, and the caller was promised no more than it asked for.
    let stride = in_box.div_ceil(cap.max(1)).max(1) as u64;
    let trees: Vec<Tree> = all
        .iter()
        .filter(|t| t.alive)
        .filter(in_view)
        .filter(|t| scattered(t.id).is_multiple_of(stride))
        .take(cap)
        .map(|t| Tree {
            id: t.id,
            kind: format!("{:?}", t.kind),
            x_m: t.position_m.0,
            y_m: t.position_m.1,
            height_m: t.height_m,
            stem_diameter_m: t.stem_diameter_m,
        })
        .collect();
    Trees {
        shown: trees.len(),
        trees,
        in_box,
        patch: terrain.patch,
        individual_centre_m: terrain.individual_centre_m,
        individual_radius_m: terrain.individual_radius_m,
        yard: terrain.yard,
        total,
    }
}

/// What is at one medium-grid cell, or `None` if it is off the island.
pub fn cell_of(terrain: &Terrain, row: usize, col: usize) -> Option<Cell> {
    use mk_island::DomainLevel::Medium;
    let (rows, cols) = (terrain.domain.rows(Medium), terrain.domain.cols(Medium));
    if row >= rows || col >= cols {
        return None;
    }
    let (x_m, y_m) = terrain.domain.cell_center_m(Medium, row, col);
    // `lat_lon_at_m` answers in radians -- it is the function that put a
    // birth 0.716 degrees from the equator earlier in this branch.
    let (latitude, longitude) = terrain.domain.lat_lon_at_m(x_m, y_m);
    Some(Cell {
        row,
        col,
        latitude: latitude.to_degrees(),
        longitude: longitude.to_degrees(),
        land: *terrain.land_mask.get(row, col),
        elevation_m: *terrain.elevation_m.get(row, col),
        buildable: mk_engine::regional::geophysics::is_buildable_cell(
            &terrain.elevation_m,
            &terrain.land_mask,
            terrain.domain.cell_size_m(Medium),
            row,
            col,
        ),
    })
}

/// One tree, as the map draws it.
///
/// Position is domain metres, the same frame everything else on the map
/// uses. `kind` is `Tree`, `Shrub` or `Grass` and **not a species**: the
/// engine's `TreeInstance` carries no species, and the species system in
/// `organisms/runtime.rs` is not wired into the island's vegetation.
/// Island-scale species is Phase 3 Task 1b, still open.
#[derive(Debug, Clone, serde::Serialize)]
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
/// `centre_m` the engine holds a `TreeInstance` per stem above a minimum
/// diameter; outside it — including the rest of the 4 km patch and the
/// whole island beyond — vegetation is stand cover and biomass per cell.
/// A viewer that did not say so would let somebody read an empty island
/// as a treeless one.
#[derive(Debug, Clone, serde::Serialize)]
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
    /// cleared rectangle the engine places no stem inside.
    pub yard: (f64, f64, f64, f64),
    /// Every individual stem the island holds, in or out of view.
    pub total: usize,
}

/// One cell of the island, for somebody who clicked on the map.
#[derive(Debug, Clone, serde::Serialize)]
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

impl IslandProjection {
    /// Describe an island, carrying `digest` forward from an earlier step.
    ///
    /// Everything but the digest is cheap: this walks the people and reads
    /// a handful of totals, and never copies the grids or the stems. The
    /// digest is the one expensive part, so the caller decides when to pay
    /// for a fresh one and this carries the last one in between.
    pub fn of(
        life: &IslandLife,
        requested_speed: &str,
        achieved_speed: Option<f64>,
        digest: Digest,
        views: Views,
    ) -> Self {
        let Views {
            records,
            records_at_tick,
            properties,
            economy,
            economy_at_tick,
            timeline,
            conversations,
            trees,
        } = views;
        let day_s = life.canon.rotation_period_s;
        Self {
            running: true,
            clock: Clock {
                sim_time_s: life.sim_time_s,
                tick: life.tick,
                days: life.sim_time_s as f64 / day_s,
                hour_of_day: (life.sim_time_s as f64 % day_s) / 3_600.0,
                day_length_hours: day_s / 3_600.0,
                achieved_speed,
                requested_speed: requested_speed.to_string(),
            },
            people: people(life),
            estate: estate(life),
            land: land(life),
            stocks: stocks(life),
            records,
            records_at_tick,
            properties,
            economy,
            economy_at_tick,
            timeline,
            conversations,
            trees,
            digest: Digest {
                current: digest.at_tick == life.tick,
                ..digest
            },
        }
    }

    /// The estate's properties, as they stand.
    pub fn properties_now(life: &IslandLife) -> Arc<Vec<Property>> {
        Arc::new(
            life.placed
                .property
                .properties
                .iter()
                .map(|p| Property {
                    name: p.name.clone(),
                    owners: p.owner_agent_ids.clone(),
                    cell: p.location,
                    on_this_island: p.location == Some(life.placed.location),
                    buildings: p
                        .buildings
                        .iter()
                        .map(|b| Building {
                            name: b.name.clone(),
                            kind: format!("{:?}", b.kind),
                        })
                        .collect(),
                    items: p
                        .items
                        .iter()
                        .map(|i| Item {
                            name: i.name.clone(),
                            kind: format!("{:?}", i.kind),
                        })
                        .collect(),
                })
                .collect(),
        )
    }

    /// What the resource economy holds, and what it has recorded.
    pub fn economy_now(life: &IslandLife) -> Arc<Economy> {
        Arc::new(Economy {
            resource_nodes: life.economy.nodes.len(),
            structures: life
                .economy
                .structures
                .iter()
                .map(|s| Structure {
                    id: s.id,
                    cell: (s.position.row, s.position.col),
                    recipe: recipe_name(s.recipe),
                    material_cost: s.material_cost,
                })
                .collect(),
            events: life
                .economy
                .events
                .iter()
                .map(|e| EconomyEntry {
                    tick: e.tick,
                    by: e.agent_id.clone(),
                    kind: format!("{:?}", e.kind),
                    subject: readable_subject(&e.subject),
                    quantity: e.quantity,
                })
                .collect(),
        })
    }

    /// Everything that has reached this island from outside, in order.
    ///
    /// Built from the replay log rather than from a separate record, so the
    /// page and `island replay` are reading the same history — a timeline
    /// that could disagree with the log would be a second account of the
    /// same run.
    pub fn timeline_now(life: &IslandLife) -> Arc<Vec<TimelineEntry>> {
        Arc::new(
            life.replay_log()
                .entries
                .iter()
                .map(|entry| TimelineEntry {
                    tick: entry.tick,
                    sequence: entry.sequence,
                    what: describe(&entry.command),
                })
                .collect(),
        )
    }

    /// Copy every islander's record. Expensive — see [`IslandProjection`].
    pub fn records_now(life: &IslandLife) -> Arc<BTreeMap<String, HumanBeing>> {
        Arc::new(
            life.humans
                .registry
                .iter()
                .map(|human| (human.agent_id().to_string(), human.clone()))
                .collect(),
        )
    }

    /// The island's recent conversations, newest first.
    ///
    /// `HumanSystem` keeps a bounded feed of the last
    /// `CONVERSATION_LOG_MAX_ENTRIES` exchanges across the whole
    /// population. This copies the most recent `CONVERSATIONS_SHOWN` of
    /// them and reverses the order, because somebody opening the page
    /// wants what was just said, not what was said five hundred
    /// conversations ago. Cheap: a bounded deque of short strings, nothing
    /// like the per-human record copy.
    pub fn conversations_now(life: &IslandLife) -> Arc<Vec<Conversation>> {
        let all: Vec<&mk_engine::humans::dialogue::ConversationEvent> =
            life.humans.conversation_log().collect();
        Arc::new(
            all.iter()
                .rev()
                .take(CONVERSATIONS_SHOWN)
                .map(|event| Conversation {
                    tick: event.tick,
                    relationship: relationship_name(event.relationship).to_string(),
                    lines: event
                        .lines
                        .iter()
                        .map(|line| ConversationLine {
                            speaker_id: line.speaker_id.to_string(),
                            speaker_name: line.speaker_name.clone(),
                            text: line.text.clone(),
                        })
                        .collect(),
                })
                .collect(),
        )
    }

    /// Copy the island's individual stems.
    pub fn trees_now(
        life: &IslandLife,
    ) -> Arc<Vec<mk_engine::regional::local_vegetation::TreeInstance>> {
        Arc::new(life.vegetation.trees.clone())
    }

    /// Hash the island now. Expensive — see [`Digest`].
    pub fn digest_now(life: &IslandLife) -> Digest {
        Digest {
            value: life
                .state_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            at_tick: life.tick,
            current: true,
        }
    }
}

fn people(life: &IslandLife) -> Vec<Person> {
    life.humans
        .registry
        .iter()
        .map(|human| {
            let id = human.agent_id();
            let at = life.positions.0.get(id);
            Person {
                agent_id: id.to_string(),
                alive: matches!(human.profile.status, mk_core::human::HumanStatus::Alive),
                asleep: human.circadian.asleep,
                age_years: human.development.age_years,
                space: at.map(|p| space_name(life, p.space)),
                position_m: at.map(|p| p.position_m),
                cell: at.and_then(|p| {
                    let size = life.domain.cell_size_m(mk_island::DomainLevel::Medium);
                    let (x, y) = p.position_m;
                    (x >= 0.0 && y >= 0.0).then(|| ((y / size) as usize, (x / size) as usize))
                }),
                body_carbon_kg: life.materials.body_carbon_kg(id),
            }
        })
        .collect()
}

/// What to call a place somebody is standing in.
///
/// The layout already labels every space the way a person would say it, so
/// this looks the label up rather than printing the id. A `SpaceId` with no
/// matching space cannot happen from a consistent layout, but saying so is
/// better than rendering `Inside(SpaceId(4))` at somebody.
fn space_name(life: &IslandLife, space: Space) -> String {
    match space {
        Space::Outdoors => "Outdoors".to_string(),
        Space::Inside(id) => life
            .placed
            .layout
            .spaces
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| format!("an unnamed space ({})", id.0)),
    }
}

fn estate(life: &IslandLife) -> Estate {
    let energy = &life.energy;
    let (latitude, longitude) = {
        let size = life.domain.cell_size_m(mk_island::DomainLevel::Medium);
        let (row, col) = life.placed.location;
        let (lat, lon) = life
            .domain
            .lat_lon_at_m((col as f64 + 0.5) * size, (row as f64 + 0.5) * size);
        (lat.to_degrees(), lon.to_degrees())
    };
    Estate {
        cell: life.placed.location,
        latitude,
        longitude,
        buildings: life.placed.layout.buildings.len(),
        items: life.placed.layout.items.len(),
        battery_charge_kwh: energy.batteries.iter().map(|b| b.charge_kwh).sum(),
        battery_capacity_kwh: energy.batteries.iter().map(|b| b.capacity_kwh).sum(),
        fuel_litres: energy.fuel_stores.iter().map(|s| s.litres).sum(),
        spaces: life
            .placed
            .layout
            .spaces
            .iter()
            .map(|s| SpaceChoice {
                id: s.id.0,
                label: s.label.clone(),
                kind: format!("{:?}", s.kind),
            })
            .collect(),
    }
}

fn land(life: &IslandLife) -> Land {
    use mk_island::DomainLevel::Medium;
    Land {
        trees: life.vegetation.trees.len(),
        patch_carbon_kgc: life.vegetation.total_carbon_kgc(),
        island_biomass_kgc: life.ecology.total_biomass_kgc(&life.domain),
        rows: life.domain.rows(Medium),
        cols: life.domain.cols(Medium),
        land_cells: life
            .physical
            .geophysics
            .land_mask
            .data()
            .iter()
            .filter(|&&land| land)
            .count(),
        cell_size_m: life.domain.cell_size_m(Medium),
    }
}

fn stocks(life: &IslandLife) -> Stocks {
    use mk_core::flux::Reservoir;
    Stocks {
        human_carbon_kg: life.materials.stock_of(Reservoir::HumanCarbon),
        material_carbon_kg: life.materials.stock_of(Reservoir::MaterialCarbon),
        material_water_kg: life.materials.stock_of(Reservoir::MaterialWater),
        audits_closed: life.audits_closed,
        food_shortfalls: life.shortfalls.food,
        water_shortfalls: life.shortfalls.water,
    }
}
