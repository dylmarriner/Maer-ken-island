//! Physical materials (Phase 3 Task 3): carbon, oxygen and water through
//! the economy.
//!
//! Upstream gathering, burning, eating and drinking move no matter. Here
//! every transfer is booked in the flux ledger and held in a stock, so
//! nothing a human gathers appears from nowhere or vanishes:
//!
//! - **Gathering biotic material** (wood, food plants, fibre) takes carbon
//!   out of the cell's biomass (`BiomassCarbon → MaterialCarbon`) and cannot
//!   take more than the cell holds; regrowth is the ecology's NPP.
//! - **Fossil and carbonate** material (coal, limestone) comes from the
//!   crust (`CrustCarbon → MaterialCarbon`).
//! - **Hunting** removes animals from their population (a fractional
//!   accumulator keeps the integer count exact) and moves the meat's carbon,
//!   the rest of the carcass going to detritus.
//! - **Burning** and **calcining** return carbon to the air
//!   (`MaterialCarbon → AtmosCO2`). Burning binds free oxygen (`C + O₂ →
//!   CO₂`, 2.664 kg O₂ per kg C); calcination's CO₂ carries the carbonate's
//!   own oxygen. Making charcoal releases the carbon the char does not keep.
//! - **Eating** moves food carbon into the body (`→ HumanCarbon`);
//!   **respiration** returns it as CO₂ at a respiratory quotient of 0.85,
//!   binding oxygen; a body's carbon goes to detritus at death.
//! - **Water** is drawn from the cell's standing water or river flow
//!   (`SurfaceWater`/`Rivers → MaterialWater`) and returns to the air and
//!   soil by respiration, sweat and excretion.
//! - **Building** keeps a material's carbon in `MaterialCarbon`, attributed
//!   to the structure, until demolition or decay returns it to detritus.
//!
//! Embedded water in foods and the water oxygen of metabolism are not
//! booked. Species populations are not debited when biomass is gathered
//! (the island's species are not seeded yet; Task 1b).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};

use super::ecology::RegionalEcologyState;
use crate::conservation::O2_PER_CARBON;
use crate::hydrology::HydrologyState;

/// Where a material's carbon comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Biotic,
    Animal,
    Fossil,
    Carbonate,
    Mineral,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Material {
    Wood,
    Charcoal,
    Coal,
    Limestone,
    Quicklime,
    PlantFood,
    Meat,
    Fibre,
    Water,
}

pub const ALL_MATERIALS: [Material; 9] = [
    Material::Wood,
    Material::Charcoal,
    Material::Coal,
    Material::Limestone,
    Material::Quicklime,
    Material::PlantFood,
    Material::Meat,
    Material::Fibre,
    Material::Water,
];

/// What a kilogram of a material is made of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Composition {
    /// Carbon per kg of the material as held (kg C).
    pub carbon_per_kg: f64,
    /// Water per kg of the material as held (kg).
    pub water_per_kg: f64,
    pub origin: Origin,
    pub source: &'static str,
}

/// Composition of a material, with the source of each value.
pub fn composition(m: Material) -> Composition {
    use Material::{Charcoal, Coal, Fibre, Limestone, Meat, PlantFood, Quicklime, Wood};
    use Origin::{Animal, Biotic, Carbonate, Fossil, Mineral};
    match m {
        // Dry wood is ~50% carbon (Lamlom & Savidge 2003: 47-54%); air-dry
        // wood holds ~15% water, so 0.85 x 0.50.
        Wood => Composition {
            carbon_per_kg: 0.425,
            water_per_kg: 0.15,
            origin: Biotic,
            source: "Lamlom & Savidge 2003, Biomass Bioenergy 25:381; air-dry 15% moisture",
        },
        // Charcoal: 75-85% fixed carbon (FAO 1987, Simple technologies for charcoal making).
        Charcoal => Composition {
            carbon_per_kg: 0.80,
            water_per_kg: 0.03,
            origin: Biotic,
            source: "FAO Forestry Paper 41 (1987): 75-85% fixed carbon",
        },
        // Bituminous coal: ~75% carbon as received (ASTM D388 ranks; EIA).
        Coal => Composition {
            carbon_per_kg: 0.75,
            water_per_kg: 0.05,
            origin: Fossil,
            source: "bituminous coal, ~75% C as received (US EIA)",
        },
        // CaCO3: 12.011 / 100.087 by mass.
        Limestone => Composition {
            carbon_per_kg: 0.12,
            water_per_kg: 0.0,
            origin: Carbonate,
            source: "stoichiometry of CaCO3 (12.011/100.087)",
        },
        Quicklime => Composition {
            carbon_per_kg: 0.0,
            water_per_kg: 0.0,
            origin: Mineral,
            source: "CaO has no carbon",
        },
        // Mixed fruit, nuts and roots: ~75% water, dry matter ~45% carbon.
        PlantFood => Composition {
            carbon_per_kg: 0.1125,
            water_per_kg: 0.75,
            origin: Biotic,
            source: "~75% water x 25% dry matter x 45% C (USDA FoodData Central averages)",
        },
        // Lean meat: ~21% protein (C 53%) + ~5% fat (C 77%), ~70% water.
        Meat => Composition {
            carbon_per_kg: 0.15,
            water_per_kg: 0.70,
            origin: Animal,
            source: "USDA lean beef: 21% protein x 53% C + 5% fat x 77% C",
        },
        // Air-dried bast and leaf fibre: cellulose 44% C, ~10% moisture.
        Fibre => Composition {
            carbon_per_kg: 0.40,
            water_per_kg: 0.10,
            origin: Biotic,
            source: "cellulose 44% C x 0.9 dry",
        },
        Material::Water => Composition {
            carbon_per_kg: 0.0,
            water_per_kg: 1.0,
            origin: Origin::Water,
            source: "H2O",
        },
    }
}

/// Carbon in a whole animal body (kg C per kg): ~18% of fresh mass
/// (Heymsfield et al. 2007, body composition).
pub const WHOLE_BODY_CARBON_FRACTION: f64 = 0.18;
/// Meat as a share of an animal's live mass (dressed carcass, lean).
pub const EDIBLE_FRACTION: f64 = 0.5;
/// Charcoal mass per mass of oven-dry wood in an earth kiln: the middle of
/// the labour pack's 0.10-0.20.
pub const CHARCOAL_YIELD_BY_DRY_MASS: f64 = 0.15;
/// CaO per CaCO3 by mass: 56.077 / 100.087.
pub const QUICKLIME_PER_LIMESTONE: f64 = 56.077 / 100.087;
/// Respiratory quotient of a mixed diet.
pub const RESPIRATORY_QUOTIENT: f64 = 0.85;
/// Oxygen consumed per kcal of energy (kg): 4.86 kcal per litre of O2 at
/// 1.429 g/L (Weir 1949 / Compendium).
pub const O2_KG_PER_KCAL: f64 = 1.429e-3 / 4.86;
/// Carbon respired per kcal (kg C): `RQ × O2 moles × 12.011 g`.
pub fn respired_carbon_kg_per_kcal() -> f64 {
    RESPIRATORY_QUOTIENT * O2_KG_PER_KCAL / 0.031_998 * 0.012_011
}
/// Share of a river's flow a household may draw per day.
pub const RIVER_WITHDRAWAL_FRACTION: f64 = 0.01;
/// Service life (years) of a timber structure: decay moves its carbon to
/// detritus at this e-folding time.
pub const TIMBER_STRUCTURE_LIFETIME_YEARS: f64 = 60.0;
/// Carbon in a body (kg C per kg of body mass).
pub const BODY_CARBON_PER_KG: f64 = WHOLE_BODY_CARBON_FRACTION;

#[derive(Debug, Clone, PartialEq)]
pub enum MaterialError {
    /// The cell holds no biomass to harvest.
    NothingToHarvest,
    /// The cell has no water to draw.
    DryCell,
    /// Not enough of a material in stock.
    Insufficient {
        material: Material,
        have_kg: f64,
        need_kg: f64,
    },
    /// The material cannot be used this way.
    WrongMaterial(Material),
    NoSuchHuman(String),
    NoSuchStructure(u64),
    /// A recipe would create carbon from nothing.
    CreatesCarbon {
        out_kgc: f64,
        in_kgc: f64,
    },
    /// No animals left to hunt.
    NoAnimals,
}

impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MaterialError {}

/// A game population: integer animals with an exact fractional accumulator.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimalPopulation {
    pub count: u64,
    pub body_mass_kg: f64,
    fractional_deaths: f64,
}

impl AnimalPopulation {
    pub fn new(count: u64, body_mass_kg: f64) -> Self {
        Self {
            count,
            body_mass_kg,
            fractional_deaths: 0.0,
        }
    }
    /// Carbon in the living population (kg C).
    pub fn carbon_kgc(&self) -> f64 {
        (self.count as f64 - self.fractional_deaths)
            * self.body_mass_kg
            * WHOLE_BODY_CARBON_FRACTION
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Body {
    carbon_kg: f64,
    water_kg: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Structure {
    material: Material,
    kg: f64,
}

/// Everything the economy holds as physical material.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialLedger {
    stock: BTreeMap<Material, f64>,
    structures: BTreeMap<u64, Structure>,
    bodies: BTreeMap<String, Body>,
    next_structure: u64,
}

fn flow(ledger: &mut Ledger, kind: FluxKind, from: Reservoir, to: Reservoir, amount: f64) {
    if amount > 0.0 {
        ledger.push(FluxEntry::new(from, to, amount, kind));
    }
}

impl MaterialLedger {
    pub fn new() -> Self {
        Self {
            next_structure: 1,
            ..Self::default()
        }
    }

    pub fn stock_kg(&self, m: Material) -> f64 {
        self.stock.get(&m).copied().unwrap_or(0.0)
    }

    fn add(&mut self, m: Material, kg: f64) {
        *self.stock.entry(m).or_insert(0.0) += kg;
    }

    fn take(&mut self, m: Material, kg: f64) -> Result<(), MaterialError> {
        let have = self.stock_kg(m);
        if have + 1e-12 < kg {
            return Err(MaterialError::Insufficient {
                material: m,
                have_kg: have,
                need_kg: kg,
            });
        }
        self.add(m, -kg.min(have));
        Ok(())
    }

    /// Carbon held in `MaterialCarbon` (stock and structures), `HumanCarbon`
    /// (bodies) or water held in `MaterialWater` (stock and bodies), kg.
    pub fn stock_of(&self, reservoir: Reservoir) -> f64 {
        match reservoir {
            Reservoir::MaterialCarbon => {
                self.stock
                    .iter()
                    .map(|(m, kg)| kg * composition(*m).carbon_per_kg)
                    .sum::<f64>()
                    + self
                        .structures
                        .values()
                        .map(|s| s.kg * composition(s.material).carbon_per_kg)
                        .sum::<f64>()
            }
            Reservoir::HumanCarbon => self.bodies.values().map(|b| b.carbon_kg).sum(),
            Reservoir::MaterialWater => {
                self.stock_kg(Material::Water)
                    + self.bodies.values().map(|b| b.water_kg).sum::<f64>()
            }
            _ => 0.0,
        }
    }

    /// Gather biotic material from a cell's biomass.
    pub fn gather_biotic(
        &mut self,
        material: Material,
        kg_wanted: f64,
        ecology: &mut RegionalEcologyState,
        cell: (usize, usize),
        cell_area_m2: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        let c = composition(material);
        if c.origin != Origin::Biotic || c.carbon_per_kg <= 0.0 {
            return Err(MaterialError::WrongMaterial(material));
        }
        let held_kgc = ecology.biomass_kgc_m2.get(cell.0, cell.1) * cell_area_m2;
        let kg = kg_wanted.max(0.0).min(held_kgc / c.carbon_per_kg);
        if kg <= 0.0 {
            return Err(MaterialError::NothingToHarvest);
        }
        let kgc = kg * c.carbon_per_kg;
        let biomass = ecology.biomass_kgc_m2.get_mut(cell.0, cell.1);
        *biomass = (*biomass - kgc / cell_area_m2).max(0.0);
        self.add(material, kg);
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::BiomassCarbon,
            Reservoir::MaterialCarbon,
            kgc,
        );
        Ok(kg)
    }

    /// Mine coal or quarry limestone: carbon from the crust.
    pub fn gather_fossil(
        &mut self,
        material: Material,
        kg: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        let c = composition(material);
        if !matches!(c.origin, Origin::Fossil | Origin::Carbonate) {
            return Err(MaterialError::WrongMaterial(material));
        }
        let kg = kg.max(0.0);
        self.add(material, kg);
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::CrustCarbon,
            Reservoir::MaterialCarbon,
            kg * c.carbon_per_kg,
        );
        Ok(kg)
    }

    /// Hunt `meat_kg` of meat from `population`. Animals die in whole
    /// numbers as the fractional need accumulates; the carbon of the animals
    /// the meat stands for leaves biomass now, the meat's share into
    /// `MaterialCarbon` and the carcass into detritus.
    pub fn hunt(
        &mut self,
        population: &mut AnimalPopulation,
        meat_kg: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        if population.count == 0 || meat_kg <= 0.0 {
            return Err(MaterialError::NoAnimals);
        }
        let animals = meat_kg / (population.body_mass_kg * EDIBLE_FRACTION);
        population.fractional_deaths += animals;
        let deaths = (population.fractional_deaths.floor() as u64).min(population.count);
        population.count -= deaths;
        population.fractional_deaths -= deaths as f64;
        let total_c = animals * population.body_mass_kg * WHOLE_BODY_CARBON_FRACTION;
        let meat_c = meat_kg * composition(Material::Meat).carbon_per_kg;
        self.add(Material::Meat, meat_kg);
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::BiomassCarbon,
            Reservoir::MaterialCarbon,
            meat_c,
        );
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::BiomassCarbon,
            Reservoir::DetritusCarbon,
            (total_c - meat_c).max(0.0),
        );
        Ok(meat_kg)
    }

    /// Burn fuel (wood, charcoal, coal): carbon to CO₂, binding free oxygen.
    pub fn burn(
        &mut self,
        material: Material,
        kg: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        if !matches!(
            material,
            Material::Wood | Material::Charcoal | Material::Coal
        ) {
            return Err(MaterialError::WrongMaterial(material));
        }
        self.take(material, kg)?;
        let kgc = kg * composition(material).carbon_per_kg;
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::MaterialCarbon,
            Reservoir::AtmosCO2,
            kgc,
        );
        flow(
            ledger,
            FluxKind::Oxygen,
            Reservoir::AtmosO2,
            Reservoir::AtmosCO2,
            kgc * O2_PER_CARBON,
        );
        Ok(kgc)
    }

    /// Calcine limestone to quicklime: the carbonate's carbon leaves as CO₂
    /// carrying its own oxygen.
    pub fn calcine(
        &mut self,
        limestone_kg: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        self.take(Material::Limestone, limestone_kg)?;
        let kgc = limestone_kg * composition(Material::Limestone).carbon_per_kg;
        self.add(Material::Quicklime, limestone_kg * QUICKLIME_PER_LIMESTONE);
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::MaterialCarbon,
            Reservoir::AtmosCO2,
            kgc,
        );
        flow(
            ledger,
            FluxKind::Oxygen,
            Reservoir::CrustCarbon,
            Reservoir::AtmosCO2,
            kgc * O2_PER_CARBON,
        );
        Ok(kgc)
    }

    /// Char wood into charcoal: the char keeps its carbon, the rest of the
    /// wood's carbon is released as CO₂.
    pub fn make_charcoal(
        &mut self,
        wood_kg: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        self.take(Material::Wood, wood_kg)?;
        let wood = composition(Material::Wood);
        let dry = wood_kg * (1.0 - wood.water_per_kg);
        let char_kg = CHARCOAL_YIELD_BY_DRY_MASS * dry;
        let char_c = char_kg * composition(Material::Charcoal).carbon_per_kg;
        let released = wood_kg * wood.carbon_per_kg - char_c;
        self.add(Material::Charcoal, char_kg);
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::MaterialCarbon,
            Reservoir::AtmosCO2,
            released,
        );
        flow(
            ledger,
            FluxKind::Oxygen,
            Reservoir::AtmosO2,
            Reservoir::AtmosCO2,
            released * O2_PER_CARBON,
        );
        Ok(char_kg)
    }

    /// Craft: `inputs` become `outputs`. Carbon may only be lost (offcuts go
    /// to detritus), never created.
    pub fn craft(
        &mut self,
        inputs: &[(Material, f64)],
        outputs: &[(Material, f64)],
        ledger: &mut Ledger,
    ) -> Result<(), MaterialError> {
        let c = |list: &[(Material, f64)]| {
            list.iter()
                .map(|(m, kg)| kg * composition(*m).carbon_per_kg)
                .sum::<f64>()
        };
        let (in_c, out_c) = (c(inputs), c(outputs));
        if out_c > in_c + 1e-9 {
            return Err(MaterialError::CreatesCarbon {
                out_kgc: out_c,
                in_kgc: in_c,
            });
        }
        for (m, kg) in inputs {
            if self.stock_kg(*m) + 1e-12 < *kg {
                return Err(MaterialError::Insufficient {
                    material: *m,
                    have_kg: self.stock_kg(*m),
                    need_kg: *kg,
                });
            }
        }
        for (m, kg) in inputs {
            self.take(*m, *kg)?;
        }
        for (m, kg) in outputs {
            self.add(*m, *kg);
        }
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::MaterialCarbon,
            Reservoir::DetritusCarbon,
            in_c - out_c,
        );
        Ok(())
    }

    /// Draw `kg` of water from a cell: standing water first, then a small
    /// share of the river flow. Refused if the cell has none.
    pub fn gather_water(
        &mut self,
        kg: f64,
        hydrology: &mut HydrologyState,
        cell: (usize, usize),
        cell_area_m2: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        let (r, c) = cell;
        let standing_kg = hydrology.surface_water.get(r, c).max(0.0) * cell_area_m2;
        let river_kg =
            hydrology.runoff.get(r, c).max(0.0) * cell_area_m2 * RIVER_WITHDRAWAL_FRACTION;
        let take = kg.max(0.0).min(standing_kg + river_kg);
        if take <= 0.0 {
            return Err(MaterialError::DryCell);
        }
        let from_standing = take.min(standing_kg);
        let from_river = take - from_standing;
        hydrology.surface_water.set(
            r,
            c,
            hydrology.surface_water.get(r, c) - from_standing / cell_area_m2,
        );
        hydrology.runoff.set(
            r,
            c,
            hydrology.runoff.get(r, c) - from_river / (cell_area_m2 * RIVER_WITHDRAWAL_FRACTION),
        );
        self.add(Material::Water, take);
        flow(
            ledger,
            FluxKind::Water,
            Reservoir::SurfaceWater,
            Reservoir::MaterialWater,
            from_standing,
        );
        flow(
            ledger,
            FluxKind::Water,
            Reservoir::Rivers,
            Reservoir::MaterialWater,
            from_river,
        );
        Ok(take)
    }

    /// A body enters the ledger with its own carbon (18% of its mass).
    /// The carbon in one human's body (kg), or `None` if they were never
    /// registered. Eating adds to it and respiring draws it down, so it is
    /// the physical answer to whether someone is actually being fed — which
    /// the needs model, running separately, cannot give.
    pub fn body_carbon_kg(&self, id: &str) -> Option<f64> {
        self.bodies.get(id).map(|body| body.carbon_kg)
    }

    pub fn register_human(&mut self, id: &str, weight_kg: f64) {
        self.bodies.entry(id.to_string()).or_insert(Body {
            carbon_kg: weight_kg * BODY_CARBON_PER_KG,
            water_kg: 0.0,
        });
    }

    fn body_mut(&mut self, id: &str) -> Result<&mut Body, MaterialError> {
        self.bodies
            .get_mut(id)
            .ok_or_else(|| MaterialError::NoSuchHuman(id.into()))
    }

    /// Eat food: its carbon moves into the body.
    pub fn eat(
        &mut self,
        id: &str,
        material: Material,
        kg: f64,
        ledger: &mut Ledger,
    ) -> Result<(), MaterialError> {
        if !matches!(material, Material::PlantFood | Material::Meat) {
            return Err(MaterialError::WrongMaterial(material));
        }
        self.body_mut(id)?;
        self.take(material, kg)?;
        let kgc = kg * composition(material).carbon_per_kg;
        self.body_mut(id)?.carbon_kg += kgc;
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::MaterialCarbon,
            Reservoir::HumanCarbon,
            kgc,
        );
        Ok(())
    }

    /// Respire `kcal` of energy: body carbon returns to the air as CO₂ at
    /// the respiratory quotient, binding oxygen. Returns the carbon respired.
    pub fn respire(
        &mut self,
        id: &str,
        kcal: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        let wanted = kcal.max(0.0) * respired_carbon_kg_per_kcal();
        let body = self.body_mut(id)?;
        let kgc = wanted.min(body.carbon_kg);
        body.carbon_kg -= kgc;
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::HumanCarbon,
            Reservoir::AtmosCO2,
            kgc,
        );
        flow(
            ledger,
            FluxKind::Oxygen,
            Reservoir::AtmosO2,
            Reservoir::AtmosCO2,
            kgc * O2_PER_CARBON,
        );
        Ok(kgc)
    }

    /// Drink water from stock into the body.
    pub fn drink(&mut self, id: &str, kg: f64) -> Result<(), MaterialError> {
        self.body_mut(id)?;
        self.take(Material::Water, kg)?;
        self.body_mut(id)?.water_kg += kg;
        Ok(())
    }

    /// Lose body water: `to_soil_fraction` as excretion to the soil, the
    /// rest as breath and sweat to the air.
    pub fn lose_water(
        &mut self,
        id: &str,
        kg: f64,
        to_soil_fraction: f64,
        ledger: &mut Ledger,
    ) -> Result<f64, MaterialError> {
        let body = self.body_mut(id)?;
        let lost = kg.max(0.0).min(body.water_kg);
        body.water_kg -= lost;
        let soil = lost * to_soil_fraction.clamp(0.0, 1.0);
        flow(
            ledger,
            FluxKind::Water,
            Reservoir::MaterialWater,
            Reservoir::SoilWater,
            soil,
        );
        flow(
            ledger,
            FluxKind::Water,
            Reservoir::MaterialWater,
            Reservoir::Atmosphere,
            lost - soil,
        );
        Ok(lost)
    }

    /// A body dies: its carbon goes to detritus, its water to the soil.
    pub fn on_death(&mut self, id: &str, ledger: &mut Ledger) -> Result<(), MaterialError> {
        let body = self
            .bodies
            .remove(id)
            .ok_or_else(|| MaterialError::NoSuchHuman(id.into()))?;
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::HumanCarbon,
            Reservoir::DetritusCarbon,
            body.carbon_kg,
        );
        flow(
            ledger,
            FluxKind::Water,
            Reservoir::MaterialWater,
            Reservoir::SoilWater,
            body.water_kg,
        );
        Ok(())
    }

    /// Build a structure from stock: the carbon stays as `MaterialCarbon`,
    /// attributed to the structure.
    pub fn build(&mut self, material: Material, kg: f64) -> Result<u64, MaterialError> {
        self.take(material, kg)?;
        let id = self.next_structure.max(1);
        self.next_structure = id + 1;
        self.structures.insert(id, Structure { material, kg });
        Ok(id)
    }

    /// Demolish: the structure's carbon goes to detritus.
    pub fn demolish(&mut self, id: u64, ledger: &mut Ledger) -> Result<(), MaterialError> {
        let s = self
            .structures
            .remove(&id)
            .ok_or(MaterialError::NoSuchStructure(id))?;
        flow(
            ledger,
            FluxKind::Carbon,
            Reservoir::MaterialCarbon,
            Reservoir::DetritusCarbon,
            s.kg * composition(s.material).carbon_per_kg,
        );
        Ok(())
    }

    /// Age the structures by `dt_years`: each loses the fraction
    /// `1 - exp(-dt / lifetime)` of its mass, its carbon to detritus.
    pub fn decay_structures(&mut self, dt_years: f64, ledger: &mut Ledger) {
        if dt_years <= 0.0 {
            return;
        }
        let lose = 1.0 - (-dt_years / TIMBER_STRUCTURE_LIFETIME_YEARS).exp();
        for s in self.structures.values_mut() {
            let gone = s.kg * lose;
            s.kg -= gone;
            flow(
                ledger,
                FluxKind::Carbon,
                Reservoir::MaterialCarbon,
                Reservoir::DetritusCarbon,
                gone * composition(s.material).carbon_per_kg,
            );
        }
    }

    pub fn structure_count(&self) -> usize {
        self.structures.len()
    }
}
