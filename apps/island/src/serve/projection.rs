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
    pub buildings: usize,
    pub items: usize,
    pub battery_charge_kwh: f64,
    pub battery_capacity_kwh: f64,
    pub fuel_litres: f64,
}

/// The ground the estate stands on.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Land {
    pub trees: usize,
    pub patch_carbon_kgc: f64,
    pub island_biomass_kgc: f64,
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
    ) -> Self {
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
            digest: Digest {
                current: digest.at_tick == life.tick,
                ..digest
            },
        }
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
    Estate {
        cell: life.placed.location,
        buildings: life.placed.layout.buildings.len(),
        items: life.placed.layout.items.len(),
        battery_charge_kwh: energy.batteries.iter().map(|b| b.charge_kwh).sum(),
        battery_capacity_kwh: energy.batteries.iter().map(|b| b.capacity_kwh).sum(),
        fuel_litres: energy.fuel_stores.iter().map(|s| s.litres).sum(),
    }
}

fn land(life: &IslandLife) -> Land {
    Land {
        trees: life.vegetation.trees.len(),
        patch_carbon_kgc: life.vegetation.total_carbon_kgc(),
        island_biomass_kgc: life.ecology.total_biomass_kgc(&life.domain),
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
