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

use mk_engine::humans::HumanBeing;
use mk_engine::regional::estate_layout::Space;
use mk_engine::regional::life::IslandLife;
use serde::Serialize;

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
            ControlCommand::Snapshot => "wrote a snapshot".to_string(),
            ControlCommand::SetSpeed(speed) => format!("set the speed to {speed}"),
        },
    }
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
