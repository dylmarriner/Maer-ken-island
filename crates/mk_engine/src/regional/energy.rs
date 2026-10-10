//! Fuel and electricity for the estate (Phase 3 Task 4b).
//!
//! Upstream property has vehicles, power tools and computers but nothing
//! that runs them. Here every load needs energy:
//!
//! - **Electricity**: loads draw rated power only while in use. Supply is
//!   solar (from the regional insolation and cloud at the estate), then the
//!   battery, then the generator burning fuel. When supply runs out the
//!   loads stop (a computer loses power, so loses access).
//! - **Vehicles** carry a tank. Road vehicles burn fuel per km at the
//!   reference 4WD figures for the surface (EPA, `fixtures/reference/
//!   labour`); tractors and the excavator burn fuel per engine-hour at the
//!   reference specific consumption of 0.27 L/kWh. An empty tank does not
//!   move.
//! - **Burning fuel** books carbon (crust → atmosphere, kg C) and the free
//!   oxygen it binds into CO₂ (kg O₂) in the flux ledger, from the fuel's
//!   stoichiometry. The hydrogen's water is not booked until Task 3's
//!   material ledger exists.
//!
//! - **Lighting**: every room of the house is wired for lamps, sized by
//!   the lumen method to give ordinary room light (Gooley et al. 2011's
//!   < 200 lux) over the room's own floor. A room's lamps are on while
//!   someone is awake in it and daylight is dimmer than they are, and
//!   draw from the same supply as everything else.
//!
//! There is no refinery: fuel is finite until Phase 4b's production route
//! (deviation D16).

use std::collections::BTreeSet;

use super::estate_layout::{EstateLayout, SpaceId};

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};
use mk_island::EstateEnergyConfig;
use serde::{Deserialize, Serialize};

use crate::conservation::O2_PER_CARBON;
use crate::organisms::property::{PropertyBuildingKind, PropertyItemKind, StarterProperty};

/// Lower heating value of diesel (kWh per litre): 35.9 MJ/L.
pub const DIESEL_KWH_PER_LITRE: f64 = 9.97;
/// Shaft-to-electric efficiency of a small diesel genset at working load.
pub const GENERATOR_EFFICIENCY: f64 = 0.30;
/// Round-trip losses are split evenly between charging and discharging.
const BATTERY_ONE_WAY_EFFICIENCY: f64 = 0.95;
/// Clear-sky transmittance of the atmosphere to the surface (global
/// irradiance over top-of-atmosphere), for a mid-latitude sea-level site.
const CLEAR_SKY_TRANSMITTANCE: f64 = 0.75;
/// Cloud's reduction of surface irradiance at full cover.
const OVERCAST_REDUCTION: f64 = 0.75;
/// Standard-test-condition irradiance of a panel rating (W/m²).
const STC_IRRADIANCE_W_M2: f64 = 1_000.0;
/// Panel plus inverter and wiring efficiency in the field.
const SOLAR_SYSTEM_EFFICIENCY: f64 = 0.85;
/// Fraction of its tank a vehicle starts with, drawn from the fuel store,
/// so the vehicles do not empty the store the generator needs.
const INITIAL_TANK_FRACTION: f64 = 0.25;
/// Rated power (kW) while in use of each load class.
const COMPUTER_KW: f64 = 0.2;
const POWER_TOOL_KW: f64 = 1.5;
/// Illuminance of ordinary room light at the eye (lux): the room-light
/// condition of Gooley et al. (2011), J. Clin. Endocrinol. Metab. 96:E463,
/// "< 200 lux", at its upper bound (`fixtures/reference/humans`,
/// `room_light_illuminance`).
pub const ROOM_LIGHT_LUX: f64 = 200.0;
/// Luminous efficacy of the lamps (lm/W): the ENERGY STAR Lamps v2.0
/// minimum for an omnidirectional LED lamp (`fixtures/reference/labour`,
/// `led_lamp_efficacy`).
pub const LED_LAMP_LUMENS_PER_W: f64 = 80.0;
/// Fraction of lamp output that reaches the working plane of a small
/// domestic room: the lumen method's utilisation factor, CIBSE (2012)
/// typical 0.5 (`room_utilisation_factor`).
pub const ROOM_UTILISATION_FACTOR: f64 = 0.5;

/// Electric power (kW) of lamps giving [`ROOM_LIGHT_LUX`] over `floor_m2`:
/// the lumen method, `E A = F UF`, at the lamps' efficacy.
pub fn room_lighting_kw(floor_m2: f64) -> f64 {
    ROOM_LIGHT_LUX * floor_m2.max(0.0) / (LED_LAMP_LUMENS_PER_W * ROOM_UTILISATION_FACTOR) / 1_000.0
}

/// Diesel density and composition (average C₁₂H₂₃) and petrol (C₈H₁₅).
const MOLAR_MASS_C: f64 = 12.011;
const MOLAR_MASS_H: f64 = 1.008;
const MOLAR_MASS_O2: f64 = 31.998;
const MOLAR_MASS_CO2: f64 = 44.009;
const MOLAR_MASS_H2O: f64 = 18.015;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FuelKind {
    Diesel,
    Petrol,
}

impl FuelKind {
    /// (carbon atoms, hydrogen atoms, density kg/L) of the average molecule.
    fn composition(self) -> (f64, f64, f64) {
        match self {
            FuelKind::Diesel => (12.0, 23.0, 0.832),
            FuelKind::Petrol => (8.0, 15.0, 0.745),
        }
    }

    fn mol_per_litre(self) -> f64 {
        let (c, h, density) = self.composition();
        1000.0 * density / (c * MOLAR_MASS_C + h * MOLAR_MASS_H)
    }

    /// Carbon in a litre of fuel (kg C).
    pub fn carbon_kg_per_litre(self) -> f64 {
        self.mol_per_litre() * self.composition().0 * MOLAR_MASS_C / 1000.0
    }

    /// CO₂ a litre makes (kg).
    pub fn co2_kg_per_litre(self) -> f64 {
        self.mol_per_litre() * self.composition().0 * MOLAR_MASS_CO2 / 1000.0
    }

    /// Oxygen a litre consumes (kg): `x + y/4` mol O₂ per mol fuel.
    pub fn o2_kg_per_litre(self) -> f64 {
        let (c, h, _) = self.composition();
        self.mol_per_litre() * (c + h / 4.0) * MOLAR_MASS_O2 / 1000.0
    }

    /// Water a litre makes (kg): `y/2` mol H₂O per mol fuel.
    pub fn water_kg_per_litre(self) -> f64 {
        let (_, h, _) = self.composition();
        self.mol_per_litre() * (h / 2.0) * MOLAR_MASS_H2O / 1000.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuelStore {
    pub fuel: FuelKind,
    pub litres: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Generator {
    pub rated_kw: f64,
    pub fuel: FuelKind,
    pub litres_per_kwh: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarArray {
    pub rated_kw: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battery {
    pub capacity_kwh: f64,
    pub charge_kwh: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Load {
    pub item_id: u64,
    pub name: String,
    pub kw: f64,
    pub in_use: bool,
}

/// One room's lamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomLighting {
    pub space: SpaceId,
    pub label: String,
    pub kw: f64,
    pub in_use: bool,
}

/// What draws in a step: an item, or a room's lamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Draw {
    Item(u64),
    Room(SpaceId),
}

/// How a vehicle burns fuel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VehicleEngine {
    /// Per km, at `class_factor` times the reference light-4WD figure for
    /// the surface.
    Road { class_factor: f64 },
    /// Per engine-hour, at the reference specific consumption over
    /// `rated_kw × load_fraction`.
    Work { rated_kw: f64, load_fraction: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstateVehicle {
    pub item_id: u64,
    pub name: String,
    pub fuel: FuelKind,
    pub tank_litres: f64,
    pub litres: f64,
    pub engine: VehicleEngine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    SealedRoad,
    GravelRoad,
    OffRoadTrack,
}

/// Reference light-4WD fuel consumption (L/100 km): the midpoints of the
/// EPA-based ranges in `fixtures/reference/labour` (sealed 9-14, gravel
/// 11-18, off-road 15-30).
pub fn reference_l_per_100km(surface: Surface) -> f64 {
    match surface {
        Surface::SealedRoad => 11.5,
        Surface::GravelRoad => 14.5,
        Surface::OffRoadTrack => 22.5,
    }
}

/// Reference specific fuel consumption of a diesel engine at work (L/kWh):
/// the typical value in `fixtures/reference/labour` (0.22-0.4).
pub const WORK_L_PER_KWH: f64 = 0.27;

/// Fuel, tank and engine of a named estate vehicle, or `None` for towed
/// trailers and the like. Tank sizes and engine ratings are the
/// manufacturers' published figures, rounded; consumption comes from the
/// reference pack.
fn vehicle_spec(name: &str) -> Option<(FuelKind, f64, VehicleEngine)> {
    use FuelKind::*;
    use VehicleEngine::*;
    Some(match name {
        "Polaris RZR 1000 Turbo" | "Polaris RZR 2000 PRO R" => {
            (Petrol, 38.0, Road { class_factor: 0.9 })
        }
        "Ford Raptor 4x4 Ute" => (Petrol, 136.0, Road { class_factor: 1.2 }),
        "Golf Cart" => (Petrol, 10.0, Road { class_factor: 0.25 }),
        "Fendt 900 Vario" => (
            Diesel,
            600.0,
            Work {
                rated_kw: 220.0,
                load_fraction: 0.5,
            },
        ),
        "Fendt 1000 Vario" => (
            Diesel,
            800.0,
            Work {
                rated_kw: 300.0,
                load_fraction: 0.5,
            },
        ),
        "30-Ton Excavator (CAT 330)" => (
            Diesel,
            600.0,
            Work {
                rated_kw: 200.0,
                load_fraction: 0.6,
            },
        ),
        _ => return None,
    })
}

/// Burn of a fuel: litres and what it made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Burn {
    pub fuel: FuelKind,
    pub litres: f64,
}

impl Burn {
    pub fn co2_kg(&self) -> f64 {
        self.litres * self.fuel.co2_kg_per_litre()
    }
}

/// Book a burn: carbon crust → atmosphere (kg C) and the free oxygen bound
/// into that CO₂ (kg O₂).
pub fn book_burn(ledger: &mut Ledger, burn: Burn) {
    if burn.litres <= 0.0 {
        return;
    }
    let carbon = burn.litres * burn.fuel.carbon_kg_per_litre();
    ledger.push(FluxEntry::new(
        Reservoir::CrustCarbon,
        Reservoir::AtmosCO2,
        carbon,
        FluxKind::Carbon,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosO2,
        Reservoir::AtmosCO2,
        carbon * O2_PER_CARBON,
        FluxKind::Oxygen,
    ));
}

/// Top-of-atmosphere insolation and cloud to a panel array's output (kW).
pub fn solar_output_kw(rated_kw: f64, toa_w_m2: f64, cloud_fraction: f64) -> f64 {
    let surface = toa_w_m2.max(0.0)
        * CLEAR_SKY_TRANSMITTANCE
        * (1.0 - OVERCAST_REDUCTION * cloud_fraction.clamp(0.0, 1.0));
    (rated_kw.max(0.0) * surface / STC_IRRADIANCE_W_M2 * SOLAR_SYSTEM_EFFICIENCY)
        .min(rated_kw.max(0.0))
}

/// What one electricity step did.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnergyStep {
    pub demand_kwh: f64,
    pub solar_used_kwh: f64,
    pub solar_curtailed_kwh: f64,
    pub battery_discharged_kwh: f64,
    pub generator_kwh: f64,
    pub fuel_burned_litres: f64,
    pub unmet_kwh: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstateEnergy {
    pub fuel_stores: Vec<FuelStore>,
    pub generators: Vec<Generator>,
    pub solar: Vec<SolarArray>,
    pub batteries: Vec<Battery>,
    pub loads: Vec<Load>,
    pub vehicles: Vec<EstateVehicle>,
    /// The house's lamps, room by room. `#[serde(default)]`: an estate
    /// saved before lighting was wired had none.
    #[serde(default)]
    pub lighting: Vec<RoomLighting>,
    /// Loads fully served in the last step.
    powered: BTreeSet<u64>,
    /// Rooms whose lamps were fully served in the last step.
    #[serde(default)]
    lit: BTreeSet<SpaceId>,
}

impl EstateEnergy {
    /// The estate's supply from the scenario's config and its loads and
    /// vehicles from the property's items. Batteries start full.
    pub fn from_config(config: &EstateEnergyConfig, property: &StarterProperty) -> Self {
        let mut loads = Vec::new();
        let mut vehicles = Vec::new();
        for item in &property.items {
            match item.kind {
                PropertyItemKind::Computer => loads.push(Load {
                    item_id: item.id,
                    name: item.name.clone(),
                    kw: COMPUTER_KW,
                    in_use: false,
                }),
                PropertyItemKind::BuildingEquipment | PropertyItemKind::ShedTool
                    if item.name.to_lowercase().contains("power")
                        || item.name.to_lowercase().contains("drill")
                        || item.name.to_lowercase().contains("saw")
                        || item.name.to_lowercase().contains("welder") =>
                {
                    loads.push(Load {
                        item_id: item.id,
                        name: item.name.clone(),
                        kw: POWER_TOOL_KW,
                        in_use: false,
                    })
                }
                PropertyItemKind::Vehicle => {
                    if let Some((fuel, tank, engine)) = vehicle_spec(&item.name) {
                        vehicles.push(EstateVehicle {
                            item_id: item.id,
                            name: item.name.clone(),
                            fuel,
                            tank_litres: tank,
                            litres: 0.0,
                            engine,
                        });
                    }
                }
                _ => {}
            }
        }
        let mut energy = Self {
            fuel_stores: vec![
                FuelStore {
                    fuel: FuelKind::Diesel,
                    litres: config.diesel_litres,
                },
                FuelStore {
                    fuel: FuelKind::Petrol,
                    litres: config.petrol_litres,
                },
            ],
            generators: vec![Generator {
                rated_kw: config.generator_rated_kw,
                fuel: FuelKind::Diesel,
                litres_per_kwh: 1.0 / (DIESEL_KWH_PER_LITRE * GENERATOR_EFFICIENCY),
            }],
            solar: vec![SolarArray {
                rated_kw: config.solar_rated_kw,
            }],
            batteries: vec![Battery {
                capacity_kwh: config.battery_kwh,
                charge_kwh: config.battery_kwh,
            }],
            loads,
            vehicles,
            lighting: Vec::new(),
            powered: BTreeSet::new(),
            lit: BTreeSet::new(),
        };
        // Vehicles start part-filled, drawn from the store.
        let ids: Vec<u64> = energy.vehicles.iter().map(|v| v.item_id).collect();
        for id in ids {
            let want = energy
                .vehicles
                .iter()
                .find(|v| v.item_id == id)
                .map_or(0.0, |v| v.tank_litres * INITIAL_TANK_FRACTION);
            energy.refuel(id, want);
        }
        energy
    }

    /// Wire lamps into every space of the house (its rooms, its hall and
    /// the computer room inside it), each sized by [`room_lighting_kw`]
    /// from the space's floor in `layout`. Other buildings are workplaces
    /// and stores, used by day, and are left unwired.
    pub fn wire_lighting(&mut self, layout: &EstateLayout, property: &StarterProperty) {
        self.lighting = layout
            .spaces
            .iter()
            .filter(|space| {
                property.buildings.iter().any(|b| {
                    b.id == space.building_id
                        && matches!(
                            b.kind,
                            PropertyBuildingKind::House | PropertyBuildingKind::ComputerRoom
                        )
                })
            })
            .map(|space| RoomLighting {
                space: space.id,
                label: space.label.clone(),
                kw: room_lighting_kw(
                    (space.rect_m.x1 - space.rect_m.x0) * (space.rect_m.y1 - space.rect_m.y0),
                ),
                in_use: false,
            })
            .collect();
        self.lit.clear();
    }

    /// Whether `space` has lamps.
    pub fn is_wired(&self, space: SpaceId) -> bool {
        self.lighting.iter().any(|l| l.space == space)
    }

    /// Switch a room's lamps on or off. A space without lamps is ignored.
    pub fn set_lit(&mut self, space: SpaceId, on: bool) {
        if let Some(room) = self.lighting.iter_mut().find(|l| l.space == space) {
            room.in_use = on;
        }
    }

    /// Whether a room's lamps are on and were fully served in the last step.
    pub fn is_lit(&self, space: SpaceId) -> bool {
        self.lighting.iter().any(|l| l.space == space && l.in_use) && self.lit.contains(&space)
    }

    fn store_mut(&mut self, fuel: FuelKind) -> Option<&mut FuelStore> {
        self.fuel_stores.iter_mut().find(|s| s.fuel == fuel)
    }

    /// Fuel in store (litres).
    pub fn stored_litres(&self, fuel: FuelKind) -> f64 {
        self.fuel_stores
            .iter()
            .filter(|s| s.fuel == fuel)
            .map(|s| s.litres)
            .sum()
    }

    pub fn set_in_use(&mut self, item_id: u64, in_use: bool) {
        if let Some(load) = self.loads.iter_mut().find(|l| l.item_id == item_id) {
            load.in_use = in_use;
        }
    }

    /// Whether the estate can supply a load right now: charge in a battery,
    /// fuel for a generator, or sun on the panels (`solar_kw`).
    pub fn has_stored_supply(&self, solar_kw: f64) -> bool {
        solar_kw > 0.0
            || self.batteries.iter().any(|b| b.charge_kwh > 1e-9)
            || self
                .generators
                .iter()
                .any(|g| self.stored_litres(g.fuel) > 0.0 && g.rated_kw > 0.0)
    }

    /// Whether a load is running: in use and fully served in the last step.
    pub fn has_power(&self, item_id: u64) -> bool {
        self.loads.iter().any(|l| l.item_id == item_id && l.in_use)
            && self.powered.contains(&item_id)
    }

    /// Supply the in-use loads for `dt_seconds`: solar (`solar_kw`
    /// available), then battery, then the generator. Item loads are served
    /// in item order, then the lit rooms in space order; one the supply
    /// cannot cover stops. Fuel burned is
    /// booked in `ledger`.
    pub fn step(&mut self, dt_seconds: f64, solar_kw: f64, ledger: &mut Ledger) -> EnergyStep {
        self.powered.clear();
        self.lit.clear();
        if !(dt_seconds.is_finite() && dt_seconds > 0.0) {
            return EnergyStep::default();
        }
        let hours = dt_seconds / 3_600.0;
        let mut active: Vec<(Draw, f64)> = self
            .loads
            .iter()
            .filter(|l| l.in_use)
            .map(|l| (Draw::Item(l.item_id), l.kw * hours))
            .chain(
                self.lighting
                    .iter()
                    .filter(|l| l.in_use)
                    .map(|l| (Draw::Room(l.space), l.kw * hours)),
            )
            .collect();
        active.sort_by_key(|(draw, _)| *draw);
        let demand_kwh: f64 = active.iter().map(|(_, kwh)| kwh).sum();
        let active_ids = active;

        let solar_kwh = solar_kw.max(0.0) * hours;
        let solar_used = solar_kwh.min(demand_kwh);
        let mut surplus = solar_kwh - solar_used;
        let mut deficit = demand_kwh - solar_used;

        // Surplus solar charges the batteries.
        for b in &mut self.batteries {
            if surplus <= 0.0 {
                break;
            }
            let stored = (surplus * BATTERY_ONE_WAY_EFFICIENCY).min(b.capacity_kwh - b.charge_kwh);
            b.charge_kwh += stored;
            surplus -= stored / BATTERY_ONE_WAY_EFFICIENCY;
        }
        let curtailed = surplus.max(0.0);

        // The deficit comes from the batteries...
        let mut battery_out = 0.0;
        for b in &mut self.batteries {
            if deficit <= 0.0 {
                break;
            }
            let take = deficit.min(b.charge_kwh * BATTERY_ONE_WAY_EFFICIENCY);
            b.charge_kwh -= take / BATTERY_ONE_WAY_EFFICIENCY;
            deficit -= take;
            battery_out += take;
        }

        // ...then from the generator, limited by rating and fuel.
        let (mut generator_kwh, mut burned) = (0.0, 0.0);
        let gens: Vec<Generator> = self.generators.clone();
        for g in gens {
            if deficit <= 1e-12 {
                break;
            }
            let available = self.stored_litres(g.fuel);
            let kwh = deficit
                .min(g.rated_kw * hours)
                .min(available / g.litres_per_kwh.max(f64::MIN_POSITIVE));
            if kwh <= 0.0 {
                continue;
            }
            let litres = kwh * g.litres_per_kwh;
            if let Some(store) = self.store_mut(g.fuel) {
                store.litres = (store.litres - litres).max(0.0);
            }
            book_burn(
                ledger,
                Burn {
                    fuel: g.fuel,
                    litres,
                },
            );
            deficit -= kwh;
            generator_kwh += kwh;
            burned += litres;
        }

        // Serve loads in order while the supply lasts.
        let supplied = demand_kwh - deficit.max(0.0);
        let mut left = supplied;
        for (draw, need) in active_ids {
            if need <= left + 1e-12 {
                left -= need;
                match draw {
                    Draw::Item(id) => {
                        self.powered.insert(id);
                    }
                    Draw::Room(space) => {
                        self.lit.insert(space);
                    }
                }
            }
        }
        EnergyStep {
            demand_kwh,
            solar_used_kwh: solar_used,
            solar_curtailed_kwh: curtailed,
            battery_discharged_kwh: battery_out,
            generator_kwh,
            fuel_burned_litres: burned,
            unmet_kwh: deficit.max(0.0),
        }
    }

    /// Move fuel from the store into a vehicle's tank, up to `litres`;
    /// returns what was transferred.
    pub fn refuel(&mut self, vehicle_id: u64, litres: f64) -> f64 {
        let Some(i) = self.vehicles.iter().position(|v| v.item_id == vehicle_id) else {
            return 0.0;
        };
        let (fuel, room) = (
            self.vehicles[i].fuel,
            self.vehicles[i].tank_litres - self.vehicles[i].litres,
        );
        let available = self.stored_litres(fuel);
        let moved = litres.max(0.0).min(room).min(available);
        if let Some(store) = self.store_mut(fuel) {
            store.litres -= moved;
        }
        self.vehicles[i].litres += moved;
        moved
    }

    /// Drive a road vehicle `km` on `surface`: the distance it actually
    /// covers before the tank empties. Fuel burned is booked.
    pub fn drive_km(
        &mut self,
        vehicle_id: u64,
        surface: Surface,
        km: f64,
        ledger: &mut Ledger,
    ) -> f64 {
        let Some(v) = self.vehicles.iter_mut().find(|v| v.item_id == vehicle_id) else {
            return 0.0;
        };
        let VehicleEngine::Road { class_factor } = v.engine else {
            return 0.0;
        };
        let per_km = reference_l_per_100km(surface) * class_factor / 100.0;
        let driven = km.max(0.0).min(v.litres / per_km.max(f64::MIN_POSITIVE));
        let litres = driven * per_km;
        v.litres -= litres;
        book_burn(
            ledger,
            Burn {
                fuel: v.fuel,
                litres,
            },
        );
        driven
    }

    /// Run a working machine for `hours`: the hours it actually runs
    /// before the tank empties.
    pub fn run_engine_hours(&mut self, vehicle_id: u64, hours: f64, ledger: &mut Ledger) -> f64 {
        let Some(v) = self.vehicles.iter_mut().find(|v| v.item_id == vehicle_id) else {
            return 0.0;
        };
        let VehicleEngine::Work {
            rated_kw,
            load_fraction,
        } = v.engine
        else {
            return 0.0;
        };
        let per_hour = rated_kw * load_fraction * WORK_L_PER_KWH;
        let run = hours
            .max(0.0)
            .min(v.litres / per_hour.max(f64::MIN_POSITIVE));
        let litres = run * per_hour;
        v.litres -= litres;
        book_burn(
            ledger,
            Burn {
                fuel: v.fuel,
                litres,
            },
        );
        run
    }
}

#[cfg(test)]
mod lighting_tests {
    use super::*;
    use crate::regional::estate_layout::{Rect, SpaceKind, SpaceLayout};
    use mk_core::flux::Ledger;

    fn building(property: &StarterProperty, kind: PropertyBuildingKind) -> u64 {
        property
            .buildings
            .iter()
            .find(|b| b.kind == kind)
            .unwrap_or_else(|| panic!("the estate has a {kind:?}"))
            .id
    }

    fn space(id: u32, building_id: u64, label: &str, w: f64, h: f64) -> SpaceLayout {
        SpaceLayout {
            id: SpaceId(id),
            building_id,
            label: label.to_string(),
            kind: SpaceKind::Room,
            rect_m: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: w,
                y1: h,
            },
        }
    }

    /// The founders' estate with a house of a 5 x 5 m lounge, the 5 x 5 m
    /// computer room inside it, and a shed.
    fn wired() -> (EstateEnergy, StarterProperty) {
        let property = StarterProperty::founders_estate((0, 0));
        let house = building(&property, PropertyBuildingKind::House);
        let computers = building(&property, PropertyBuildingKind::ComputerRoom);
        let shed = building(&property, PropertyBuildingKind::Shed);
        let layout = EstateLayout {
            property_id: 0,
            patch: mk_island::LocalPatchSpec {
                origin_x_m: 0.0,
                origin_y_m: 0.0,
                width_m: 5.0,
                height_m: 5.0,
                cell_size_m: 5.0,
                rows: 1,
                cols: 1,
            },
            terrain_m: mk_core::grid::Grid2::new(&mk_core::grid::GridSpec::new(1, 1), 0.0),
            buildings: Vec::new(),
            spaces: vec![
                space(1, house, "Lounge", 5.0, 5.0),
                space(2, computers, "Computer Room", 5.0, 5.0),
                space(3, shed, "Digging Tools", 4.0, 6.0),
            ],
            doors: Vec::new(),
            items: Vec::new(),
            yard: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 1.0,
                y1: 1.0,
            },
        };
        let mut energy = EstateEnergy::from_config(&EstateEnergyConfig::default(), &property);
        energy.wire_lighting(&layout, &property);
        (energy, property)
    }

    #[test]
    fn a_room_is_lit_by_the_lumen_method() {
        // 200 lux over 25 m^2 is 5,000 lm on the floor; at a utilisation
        // factor of 0.5 the lamps emit 10,000 lm, which at 80 lm/W is
        // 125 W.
        assert!((room_lighting_kw(25.0) - 0.125).abs() < 1e-12);
        assert_eq!(room_lighting_kw(-1.0), 0.0);
    }

    #[test]
    fn the_house_is_wired_and_the_shed_is_not() {
        let (energy, _) = wired();
        assert!(energy.is_wired(SpaceId(1)), "the lounge has lamps");
        assert!(
            energy.is_wired(SpaceId(2)),
            "the computer room is a room of the house"
        );
        assert!(!energy.is_wired(SpaceId(3)), "the shed is worked by day");
        assert!(energy.lighting.iter().all(|l| (l.kw - 0.125).abs() < 1e-12));
    }

    #[test]
    fn a_lit_room_draws_its_lamps_from_the_supply() {
        let (mut energy, _) = wired();
        let mut ledger = Ledger::new();
        energy.set_lit(SpaceId(1), true);
        energy.set_lit(SpaceId(3), true); // not wired: ignored
        let night = energy.step(3_600.0, 0.0, &mut ledger);
        assert!((night.demand_kwh - 0.125).abs() < 1e-12, "{night:?}");
        assert!((night.battery_discharged_kwh - 0.125).abs() < 1e-12);
        assert!(energy.is_lit(SpaceId(1)));
        assert!(!energy.is_lit(SpaceId(2)), "nobody switched it on");
        energy.set_lit(SpaceId(1), false);
        let dark = energy.step(3_600.0, 0.0, &mut ledger);
        assert_eq!(dark.demand_kwh, 0.0);
        assert!(!energy.is_lit(SpaceId(1)));
    }

    #[test]
    fn with_no_supply_left_the_lamps_go_out() {
        let (mut energy, _) = wired();
        let mut ledger = Ledger::new();
        for b in &mut energy.batteries {
            b.charge_kwh = 0.0;
        }
        for s in &mut energy.fuel_stores {
            s.litres = 0.0;
        }
        energy.set_lit(SpaceId(1), true);
        let step = energy.step(3_600.0, 0.0, &mut ledger);
        assert!((step.unmet_kwh - 0.125).abs() < 1e-12, "{step:?}");
        assert!(!energy.is_lit(SpaceId(1)));
    }

    #[test]
    fn an_estate_saved_before_lamps_reads_with_none() {
        let (energy, _) = wired();
        let mut json = serde_json::to_value(&energy).unwrap();
        let object = json.as_object_mut().unwrap();
        object.remove("lighting");
        object.remove("lit");
        let old: EstateEnergy = serde_json::from_value(json).unwrap();
        assert!(old.lighting.is_empty());
    }
}
