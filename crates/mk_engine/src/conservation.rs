//! Conservation stocks, per-step bookings, and the per-tick stock audit.
//!
//! The flux ledger is double-entry: every entry debits one reservoir and
//! credits another, so reading the ledger alone can never reveal a model
//! that changes a stock without recording it. This module closes that gap.
//! It measures the world's interior stocks, books each subsystem's real
//! change to them in SI units, and checks, once per tick, that every
//! interior reservoir's measured change equals its recorded net inflow
//! ([`mk_core::flux::Ledger::stock_residuals`]).
//!
//! Stocks measured:
//! - **Energy**: the surface layer's heat content, `Σ C·A·T` per cell, where
//!   `C` is the areal heat capacity [`crate::climate::surface_heat_capacity_j_m2_k`]
//!   and `A` the cell's exact area. Land cells form
//!   [`Reservoir::SurfaceEnergy`] and ocean cells form [`Reservoir::OceanHeat`].
//! - **Water**: soil water, `Σ storage_mm · A` in kg (1 mm over 1 m² is
//!   1 kg), as [`Reservoir::SoilWater`].
//!
//! - **Carbon**: atmospheric CO₂ as [`Reservoir::AtmosCO2`], the
//!   concentration (ppm) times [`atmospheric_carbon_kg_per_ppm`]; and living
//!   biomass as [`Reservoir::BiomassCarbon`], every species' population
//!   times its body mass times [`CARBON_FRACTION_OF_WET_BODY_MASS`], in kg C.
//!
//!   Dead organic matter is [`Reservoir::DetritusCarbon`], the biosphere's
//!   `detritus_carbon_kg`.
//! - **Oxygen**, in kg O₂: free atmospheric O₂ ([`Reservoir::AtmosO2`], the
//!   climate state's `atmospheric_o2_kg`, starting from the canon
//!   composition) and the O₂ bound in atmospheric CO₂ (on
//!   [`Reservoir::AtmosCO2`], its carbon times [`O2_PER_CARBON`]).
//!   Photosynthesis (`CO₂ + H₂O → CH₂O + O₂`) frees exactly the O₂ bound to
//!   the carbon it fixes, and aerobic decomposition binds it back; CO₂
//!   exchanged with the crust or an operator carries its oxygen.
//!
//! Carbon moves between these stocks in four ways. Net biomass growth draws
//! CO₂ out of the air (photosynthesis), and net biomass loss (mortality)
//! moves the dead carbon into detritus ([`exchange_biomass_carbon`]).
//! Detritus decomposes back to CO₂ at a temperature-dependent first-order
//! rate ([`decompose_detritus`]). The climate step's carbon cycle moves CO₂
//! between the air and the crust (volcanic outgassing against silicate
//! weathering), so its change is booked against [`Reservoir::CrustCarbon`]. Operator interventions
//! enter and leave through [`Reservoir::OperatorIntervention`]. The model
//! has no dissolved ocean carbon stock.
//!
//! Soil nitrogen and phosphorus also exchange with the outside
//! ([`exchange_external_nutrients`]). Fixation draws nitrogen from
//! [`Reservoir::AtmosN2`], and denitrification and leaching return it there.
//! Rock weathering releases phosphorus from [`Reservoir::CrustPhosphorus`],
//! and occlusion and burial return it there. Both are boundaries.

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir, ReservoirStocks};
use mk_core::grid::{Grid2, GridSpec};
use serde::{Deserialize, Serialize};

/// Flux kinds whose stocks are measured and audited every tick.
pub const AUDITED_KINDS: [FluxKind; 6] = [
    FluxKind::Energy,
    FluxKind::Water,
    FluxKind::Carbon,
    FluxKind::Oxygen,
    FluxKind::Nitrogen,
    FluxKind::Phosphorus,
];

/// Mean dry-air molar mass, kg/mol (`NATURE_CONSTANTS.md` § 2.1).
pub const DRY_AIR_MOLAR_MASS_KG_MOL: f64 = 0.0296;

/// Molar mass of carbon, kg/mol.
pub const CARBON_MOLAR_MASS_KG_MOL: f64 = 0.012011;

/// Molar mass of O₂, kg/mol.
pub const O2_MOLAR_MASS_KG_MOL: f64 = 0.031998;

/// Kilograms of O₂ bound to (in CO₂) or freed from (by photosynthesis) each
/// kilogram of carbon.
pub const O2_PER_CARBON: f64 = O2_MOLAR_MASS_KG_MOL / CARBON_MOLAR_MASS_KG_MOL;

/// Starting O₂ mole fraction of dry air (`NATURE_CONSTANTS.md` § 2.2).
pub const ATMOSPHERIC_O2_MOLE_FRACTION: f64 = 0.2460;

/// Carbon per kg of living (wet) body mass: tissue is about 30 % dry matter,
/// and dry matter about 47.5 % carbon (the same dry-mass carbon fraction the
/// biosphere uses for plant production).
pub const CARBON_FRACTION_OF_WET_BODY_MASS: f64 = 0.30 * 0.475;

/// Largest acceptable residual, as a fraction of the reservoir's stock. The
/// stocks are sums over every cell (heat content is ~10²⁶ J), so an absolute
/// tolerance would be meaningless; 1e-9 is far above f64 summation error and
/// far below any real unbooked change.
pub const RELATIVE_TOLERANCE: f64 = 1e-9;

/// The heat reservoir a cell's surface layer belongs to.
pub fn heat_reservoir(elevation_m: f64) -> Reservoir {
    if elevation_m <= 0.0 {
        Reservoir::OceanHeat
    } else {
        Reservoir::SurfaceEnergy
    }
}

fn cell_area(spec: &GridSpec, row: usize, radius_m: f64) -> f64 {
    spec.cell_area_at_row_m2(row, radius_m).unwrap_or(0.0)
}

fn grid_value(grid: &Grid2<f64>, row: usize, col: usize) -> f64 {
    grid.get_safe(row, col).copied().unwrap_or(0.0)
}

/// Heat content (J) of each heat reservoir for a temperature field.
pub fn heat_content_j(
    spec: &GridSpec,
    radius_m: f64,
    temperature_k: &Grid2<f64>,
    elevation_m: &Grid2<f64>,
) -> (f64, f64) {
    let (mut land, mut ocean) = (0.0, 0.0);
    for row in 0..spec.nlat {
        let area = cell_area(spec, row, radius_m);
        for col in 0..spec.nlon {
            let height = grid_value(elevation_m, row, col);
            let heat = crate::climate::surface_heat_capacity_j_m2_k(height)
                * area
                * grid_value(temperature_k, row, col);
            match heat_reservoir(height) {
                Reservoir::OceanHeat => ocean += heat,
                _ => land += heat,
            }
        }
    }
    (land, ocean)
}

/// Soil water (kg) held in a hydrology state.
pub fn soil_water_kg(
    spec: &GridSpec,
    radius_m: f64,
    soil: &Grid2<crate::hydrology::SoilWater>,
) -> f64 {
    let mut total = 0.0;
    for row in 0..spec.nlat {
        let area = cell_area(spec, row, radius_m);
        for col in 0..spec.nlon {
            if let Some(cell) = soil.get_safe(row, col) {
                total += cell.storage_mm * area;
            }
        }
    }
    total
}

/// Moles of dry air in the atmosphere, `P₀ · 4πR² / g / M_air`.
fn atmospheric_moles(canon: &mk_core::canon::CanonLocked) -> f64 {
    let radius = canon.planet_radius_m;
    let air_mass_kg = canon.sea_level_pressure_pa * 4.0 * std::f64::consts::PI * radius * radius
        / canon.surface_gravity_m_s2;
    air_mass_kg / DRY_AIR_MOLAR_MASS_KG_MOL
}

/// Carbon (kg) in one ppm of atmospheric CO₂: the atmosphere's moles times
/// 10⁻⁶ times the molar mass of carbon.
pub fn atmospheric_carbon_kg_per_ppm(canon: &mk_core::canon::CanonLocked) -> f64 {
    atmospheric_moles(canon) * 1e-6 * CARBON_MOLAR_MASS_KG_MOL
}

/// Free O₂ (kg) in an atmosphere of the canon composition.
pub fn canon_atmospheric_o2_kg(canon: &mk_core::canon::CanonLocked) -> f64 {
    atmospheric_moles(canon) * ATMOSPHERIC_O2_MOLE_FRACTION * O2_MOLAR_MASS_KG_MOL
}

/// Base detritus decomposition rate `k_dec,base`, per year
/// (`BIOSPHERE_CONSTANTS.md` § 9.1).
pub const DECOMPOSITION_RATE_PER_YEAR: f64 = 0.52;

/// Reference temperature of the decomposition Q10 response: the canon
/// near-surface reference temperature `T_ref` (`NATURE_CONSTANTS.md` § 2.1).
pub const DECOMPOSITION_REFERENCE_TEMPERATURE_K: f64 = 286.0;

/// Decomposition warmth sensitivity `Q10,dec` (`BIOSPHERE_CONSTANTS.md` § 9.1).
pub const DECOMPOSITION_Q10: f64 = 2.1;

/// Canon decomposition moisture optimum `W_opt,dec` (`BIOSPHERE_CONSTANTS.md`
/// § 9.1).
pub const DECOMPOSITION_MOISTURE_OPTIMUM: f64 = 0.61;

/// Width `σ_W` of the peaked moisture response (§ 7.4), which canon leaves
/// unvalued. Decomposition falls to half rate ≈0.31 either side of the
/// optimum (≈0.30 and ≈0.92 of saturation, as in Earth soil respiration
/// curves), so `σ_W = 0.31 / √(2 ln 2)`. See
/// `docs/plans/2026-09-29-biogeochemistry-constants.md`.
pub const DECOMPOSITION_MOISTURE_WIDTH: f64 = 0.263;

/// Biological nitrogen fixation per kg C of land NPP, kg N. Earth's natural
/// terrestrial fixation (≈100 Tg N/yr) over its land NPP (≈58 Pg C/yr):
/// fixers are energy-limited, so fixation follows productivity.
pub const NITROGEN_FIXATION_PER_NPP_CARBON: f64 = 100.0e9 / 58.0e12;

/// First-order loss of bioavailable soil nitrogen to denitrification and
/// leaching, per year: Earth's pre-industrial steady state, where loss
/// balances ≈100 Tg N/yr of fixation from ≈0.62 kg N/m² over 1.49e14 m².
pub const NITROGEN_LOSS_RATE_PER_YEAR: f64 = 100.0e9 / (0.62 * 1.49e14);

/// Rock weathering release of phosphorus at the reference temperature and
/// runoff, kg P per m² of land per year: Earth's ≈17.5 Tg P/yr over
/// 1.49e14 m² of land.
pub const PHOSPHORUS_WEATHERING_KG_M2_YR: f64 = 17.5e9 / 1.49e14;

/// Runoff at which [`PHOSPHORUS_WEATHERING_KG_M2_YR`] applies: Earth's mean
/// land runoff, ≈40 000 km³/yr over its land, in mm/day.
pub const PHOSPHORUS_WEATHERING_REFERENCE_RUNOFF_MM_DAY: f64 = 0.74;

/// Weathering temperature sensitivity per K and its reference temperature:
/// the same Arrhenius-like response the climate's silicate weathering uses.
pub const PHOSPHORUS_WEATHERING_TEMPERATURE_SENSITIVITY: f64 = 0.07;
pub const PHOSPHORUS_WEATHERING_REFERENCE_TEMPERATURE_K: f64 = 288.0;

/// First-order loss of bioavailable soil phosphorus to occlusion and
/// leaching, per year: Earth's steady state, balancing ≈17.5 Tg P/yr of
/// weathering from ≈0.041 kg P/m² over 1.49e14 m².
pub const PHOSPHORUS_LOSS_RATE_PER_YEAR: f64 = 17.5e9 / (0.041 * 1.49e14);

/// Molar mass of nitrogen, kg/mol.
pub const NITROGEN_MOLAR_MASS_KG_MOL: f64 = 0.014007;

/// Molar mass of phosphorus, kg/mol.
pub const PHOSPHORUS_MOLAR_MASS_KG_MOL: f64 = 0.030974;

/// Mean active soil nitrogen density, kg N/m² of land
/// (`BIOSPHERE_CONSTANTS.md` § 3.2).
pub const SOIL_NITROGEN_DENSITY_KG_M2: f64 = 0.62;

/// Mean active soil phosphorus density, kg P/m² of land (§ 3.2).
pub const SOIL_PHOSPHORUS_DENSITY_KG_M2: f64 = 0.041;

/// Producer (flora) biomass C:P, mol/mol (§ 3.3).
pub const PRODUCER_CP_RATIO: f64 = 240.0;

/// Consumer (fauna) biomass C:P, mol/mol (§ 3.3).
pub const CONSUMER_CP_RATIO: f64 = 78.0;

/// Share of dead producer biomass routed to detritus, `f_prod→det`
/// (`BIOSPHERE_CONSTANTS.md` § 9.2); the rest is respired to CO₂.
pub const PRODUCER_MORTALITY_TO_DETRITUS: f64 = 0.94;

/// Share of dead consumer biomass routed to detritus, `f_cons→det` (§ 9.2).
pub const CONSUMER_MORTALITY_TO_DETRITUS: f64 = 0.89;

/// Whether a species category is a heterotroph: fauna and the fungoid
/// decomposers take consumer stoichiometry and mortality routing; trees and
/// flowers are producers.
pub fn is_consumer_category(category: &crate::biosphere::SpeciesCategory) -> bool {
    crate::biosphere::is_fauna_category(category)
        || matches!(category, crate::biosphere::SpeciesCategory::Fungoid)
}

/// Carbon (kg) held in living producer and consumer biomass, in that order.
pub fn biomass_carbon_by_class_kg(species: &[crate::biosphere::Species]) -> [f64; 2] {
    let mut classes = [0.0, 0.0];
    for s in species {
        let carbon = s.population_size as f64
            * s.representative_genome.body_mass_kg()
            * CARBON_FRACTION_OF_WET_BODY_MASS;
        classes[usize::from(is_consumer_category(&s.category))] += carbon;
    }
    classes
}

/// Carbon a biosphere step's mortality released: all of it, and the part
/// routed to detritus.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DeadCarbon {
    /// Dead biomass carbon, kg C.
    pub total_kg: f64,
    /// The part of it that entered detritus, kg C.
    pub to_detritus_kg: f64,
}

/// Carbon (kg) held in living biomass.
pub fn biomass_carbon_kg(species: &[crate::biosphere::Species]) -> f64 {
    species
        .iter()
        .map(|s| {
            s.population_size as f64
                * s.representative_genome.body_mass_kg()
                * CARBON_FRACTION_OF_WET_BODY_MASS
        })
        .sum()
}

/// Nitrogen and phosphorus (kg) held in living biomass: each species'
/// carbon at the canon producer or consumer C:N and C:P ratio.
pub fn biomass_nutrients_kg(
    species: &[crate::biosphere::Species],
    canon: &mk_core::canon::CanonLocked,
) -> (f64, f64) {
    let (mut nitrogen, mut phosphorus) = (0.0, 0.0);
    for s in species {
        let carbon_mol = s.population_size as f64
            * s.representative_genome.body_mass_kg()
            * CARBON_FRACTION_OF_WET_BODY_MASS
            / CARBON_MOLAR_MASS_KG_MOL;
        let (cn, cp) = if is_consumer_category(&s.category) {
            (canon.cn_ratio_cons, CONSUMER_CP_RATIO)
        } else {
            (canon.cn_ratio_prod, PRODUCER_CP_RATIO)
        };
        nitrogen += carbon_mol / cn * NITROGEN_MOLAR_MASS_KG_MOL;
        phosphorus += carbon_mol / cp * PHOSPHORUS_MOLAR_MASS_KG_MOL;
    }
    (nitrogen, phosphorus)
}

/// Bioavailable soil nitrogen and phosphorus (kg) of a world of this
/// geography: the canon's mean active soil densities over its land area.
pub fn canon_soil_nutrients_kg(
    spec: &GridSpec,
    radius_m: f64,
    elevation_m: &Grid2<f64>,
) -> (f64, f64) {
    let mut land_m2 = 0.0;
    for row in 0..spec.nlat {
        let area = cell_area(spec, row, radius_m);
        for col in 0..spec.nlon {
            if grid_value(elevation_m, row, col) > 0.0 {
                land_m2 += area;
            }
        }
    }
    (
        land_m2 * SOIL_NITROGEN_DENSITY_KG_M2,
        land_m2 * SOIL_PHOSPHORUS_DENSITY_KG_M2,
    )
}

/// Surface water (kg): standing land water plus the runoff in transit
/// between cells (mm over the cell it left).
pub fn surface_water_kg(
    spec: &GridSpec,
    radius_m: f64,
    hydrology: &crate::hydrology::HydrologyState,
) -> f64 {
    let routed = &hydrology.routed_outflow_mm;
    let mut total = 0.0;
    for row in 0..spec.nlat {
        let area = cell_area(spec, row, radius_m);
        for col in 0..spec.nlon {
            let standing = grid_value(&hydrology.surface_water, row, col).max(0.0);
            let in_transit = routed.get(row * spec.nlon + col).copied().unwrap_or(0.0);
            total += (standing + in_transit) * area;
        }
    }
    total
}

/// Measure every audited stock of `world`.
pub fn measure(world: &crate::world_integration::WorldState) -> ReservoirStocks {
    let spec = &world.grid_spec;
    let radius = world.canon.planet_radius_m;
    let (land, ocean) = heat_content_j(
        spec,
        radius,
        &world.climate_state.surface_temperature,
        &world.elevation_grid,
    );
    let mut stocks = ReservoirStocks::new();
    stocks.set(Reservoir::SurfaceEnergy, FluxKind::Energy, land);
    stocks.set(Reservoir::OceanHeat, FluxKind::Energy, ocean);
    stocks.set(
        Reservoir::SoilWater,
        FluxKind::Water,
        soil_water_kg(spec, radius, &world.hydrology_state.soil_water),
    );
    stocks.set(
        Reservoir::SurfaceWater,
        FluxKind::Water,
        surface_water_kg(spec, radius, &world.hydrology_state),
    );
    let co2_carbon_kg =
        world.climate_state.co2_concentration * atmospheric_carbon_kg_per_ppm(&world.canon);
    stocks.set(Reservoir::AtmosCO2, FluxKind::Carbon, co2_carbon_kg);
    // Oxygen: the O₂ bound in atmospheric CO₂, and the free O₂.
    stocks.set(
        Reservoir::AtmosCO2,
        FluxKind::Oxygen,
        co2_carbon_kg * O2_PER_CARBON,
    );
    stocks.set(
        Reservoir::AtmosO2,
        FluxKind::Oxygen,
        world.climate_state.atmospheric_o2_kg.unwrap_or(0.0),
    );
    stocks.set(
        Reservoir::BiomassCarbon,
        FluxKind::Carbon,
        biomass_carbon_kg(&world.biosphere_state.species),
    );
    stocks.set(
        Reservoir::DetritusCarbon,
        FluxKind::Carbon,
        world.biosphere_state.detritus_carbon_kg,
    );
    let biosphere = &world.biosphere_state;
    let (biomass_n, biomass_p) = biomass_nutrients_kg(&biosphere.species, &world.canon);
    for (reservoir, kind, kg) in [
        (Reservoir::BiomassNitrogen, FluxKind::Nitrogen, biomass_n),
        (
            Reservoir::DetritusNitrogen,
            FluxKind::Nitrogen,
            biosphere.detritus_nitrogen_kg,
        ),
        (
            Reservoir::SoilNitrogen,
            FluxKind::Nitrogen,
            biosphere.soil_nitrogen_kg.unwrap_or(0.0),
        ),
        (
            Reservoir::BiomassPhosphorus,
            FluxKind::Phosphorus,
            biomass_p,
        ),
        (
            Reservoir::DetritusPhosphorus,
            FluxKind::Phosphorus,
            biosphere.detritus_phosphorus_kg,
        ),
        (
            Reservoir::SoilPhosphorus,
            FluxKind::Phosphorus,
            biosphere.soil_phosphorus_kg.unwrap_or(0.0),
        ),
    ] {
        stocks.set(reservoir, kind, kg);
    }
    stocks
}

/// Book the nitrogen and phosphorus a biosphere step moved into or out of
/// living biomass.
///
/// `before` is [`biomass_nutrients_kg`] at the start of the step and `dead`
/// what [`exchange_biomass_carbon`] routed. Growth takes its nutrients up
/// from the soil. What dying biomass loses goes into detritus up to what the
/// detrital carbon holds at the detritus C:N ratio (and the detrital share
/// of its phosphorus); the rest mineralizes straight back to the soil.
pub fn exchange_biomass_nutrients(
    ledger: &mut Ledger,
    biosphere: &mut crate::biosphere::BiosphereSystem,
    canon: &mk_core::canon::CanonLocked,
    before: (f64, f64),
    dead: DeadCarbon,
) {
    let (nitrogen, phosphorus) = biomass_nutrients_kg(&biosphere.species, canon);
    let detritus_n_capacity = dead.to_detritus_kg / CARBON_MOLAR_MASS_KG_MOL / canon.cn_ratio_det
        * NITROGEN_MOLAR_MASS_KG_MOL;
    let detrital_share = if dead.total_kg > 0.0 {
        (dead.to_detritus_kg / dead.total_kg).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let nitrogen_change = nitrogen - before.0;
    let phosphorus_change = phosphorus - before.1;
    for (kind, change, to_detritus_limit, biomass, detritus, soil) in [
        (
            FluxKind::Nitrogen,
            nitrogen_change,
            detritus_n_capacity,
            Reservoir::BiomassNitrogen,
            Reservoir::DetritusNitrogen,
            Reservoir::SoilNitrogen,
        ),
        (
            FluxKind::Phosphorus,
            phosphorus_change,
            (-phosphorus_change).max(0.0) * detrital_share,
            Reservoir::BiomassPhosphorus,
            Reservoir::DetritusPhosphorus,
            Reservoir::SoilPhosphorus,
        ),
    ] {
        if !change.is_finite() || change == 0.0 {
            continue;
        }
        let (detritus_kg, soil_kg) = match kind {
            FluxKind::Nitrogen => (
                &mut biosphere.detritus_nitrogen_kg,
                &mut biosphere.soil_nitrogen_kg,
            ),
            _ => (
                &mut biosphere.detritus_phosphorus_kg,
                &mut biosphere.soil_phosphorus_kg,
            ),
        };
        if change > 0.0 {
            if let Some(soil_kg) = soil_kg.as_mut() {
                *soil_kg -= change;
            }
            ledger.push(FluxEntry::new(soil, biomass, change, kind));
        } else {
            let dead = -change;
            let into_detritus = dead.min(to_detritus_limit);
            let mineralized = dead - into_detritus;
            *detritus_kg += into_detritus;
            if let Some(soil_kg) = soil_kg.as_mut() {
                *soil_kg += mineralized;
            }
            for (sink, amount) in [(detritus, into_detritus), (soil, mineralized)] {
                if amount > 0.0 {
                    ledger.push(FluxEntry::new(biomass, sink, amount, kind));
                }
            }
        }
    }
}

/// Move `kg` of carbon into (positive) or out of (negative) the
/// atmosphere. The CO₂ concentration is carried state the carbon cycle
/// steps from its previous value, so the change persists.
fn add_atmospheric_carbon(climate: &mut crate::climate::ClimateState, kg: f64, kg_per_ppm: f64) {
    climate.co2_concentration += kg / kg_per_ppm;
}

/// Book the carbon a biosphere step added to or removed from living
/// biomass, class by class.
///
/// `before` is [`biomass_carbon_by_class_kg`] at the start of the step.
/// Growth takes its carbon out of [`Reservoir::AtmosCO2`] (photosynthesis).
/// Loss is mortality, routed by the canon fractions
/// ([`PRODUCER_MORTALITY_TO_DETRITUS`], [`CONSUMER_MORTALITY_TO_DETRITUS`];
/// `BIOSPHERE_EQUATIONS.md` § 6.3): that share moves into
/// `detritus_carbon_kg` ([`Reservoir::DetritusCarbon`]) and decomposes
/// later, and the rest is respired to CO₂ at once.
pub fn exchange_biomass_carbon(
    ledger: &mut Ledger,
    climate: &mut crate::climate::ClimateState,
    biosphere: &mut crate::biosphere::BiosphereSystem,
    kg_per_ppm: f64,
    before: [f64; 2],
) -> DeadCarbon {
    let mut dead = DeadCarbon::default();
    if kg_per_ppm <= 0.0 {
        return dead;
    }
    let after = biomass_carbon_by_class_kg(&biosphere.species);
    for (class, to_detritus_fraction) in [
        PRODUCER_MORTALITY_TO_DETRITUS,
        CONSUMER_MORTALITY_TO_DETRITUS,
    ]
    .into_iter()
    .enumerate()
    {
        let change = after[class] - before[class];
        if change == 0.0 || !change.is_finite() {
            continue;
        }
        if change > 0.0 {
            add_atmospheric_carbon(climate, -change, kg_per_ppm);
            ledger.push(FluxEntry::new(
                Reservoir::AtmosCO2,
                Reservoir::BiomassCarbon,
                change,
                FluxKind::Carbon,
            ));
            // CO₂ + H₂O → CH₂O + O₂: the O₂ that was bound to the fixed
            // carbon is freed into the air (the water's oxygen goes into the
            // biomass).
            exchange_bound_oxygen(ledger, climate, change * O2_PER_CARBON);
            continue;
        }
        let died = -change;
        let to_detritus = died * to_detritus_fraction;
        let respired = died - to_detritus;
        dead.total_kg += died;
        dead.to_detritus_kg += to_detritus;
        biosphere.detritus_carbon_kg += to_detritus;
        ledger.push(FluxEntry::new(
            Reservoir::BiomassCarbon,
            Reservoir::DetritusCarbon,
            to_detritus,
            FluxKind::Carbon,
        ));
        if respired > 0.0 {
            add_atmospheric_carbon(climate, respired, kg_per_ppm);
            ledger.push(FluxEntry::new(
                Reservoir::BiomassCarbon,
                Reservoir::AtmosCO2,
                respired,
                FluxKind::Carbon,
            ));
            exchange_bound_oxygen(ledger, climate, -respired * O2_PER_CARBON);
        }
    }
    dead
}

/// Canon peaked moisture response of decomposition, `f_W` (§ 7.4).
pub fn decomposition_moisture_response(soil_moisture: f64) -> f64 {
    let offset = soil_moisture.clamp(0.0, 1.0) - DECOMPOSITION_MOISTURE_OPTIMUM;
    (-(offset * offset) / (2.0 * DECOMPOSITION_MOISTURE_WIDTH * DECOMPOSITION_MOISTURE_WIDTH)).exp()
}

/// The land fields the external nutrient fluxes read, one value per cell.
pub struct LandNutrientDrivers<'a> {
    pub spec: &'a GridSpec,
    pub radius_m: f64,
    pub elevation_m: &'a Grid2<f64>,
    pub surface_temperature_k: &'a Grid2<f64>,
    /// Runoff, mm/day.
    pub runoff_mm_day: &'a Grid2<f64>,
    /// Net primary production, kg C m⁻² yr⁻¹, `GridSpec` row-major.
    pub npp_kgc_m2_yr: &'a [f64],
}

/// The external nutrient fluxes one step moved, in kg.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExternalNutrientFluxes {
    pub nitrogen_fixed_kg: f64,
    pub nitrogen_lost_kg: f64,
    pub phosphorus_weathered_kg: f64,
    pub phosphorus_lost_kg: f64,
}

/// Move the soil nutrient pools' external fluxes over `dt_seconds` and book
/// them (`BIOSPHERE_EQUATIONS.md` § 2.3–2.4): `F_fix,N` and `F_loss,N` against
/// atmospheric N₂, `F_weather,P` and `F_burial,P` against rock phosphorus.
///
/// Every land cell fixes nitrogen in proportion to its NPP
/// ([`NITROGEN_FIXATION_PER_NPP_CARBON`]) and weathers phosphorus at
/// [`PHOSPHORUS_WEATHERING_KG_M2_YR`] scaled by `e^(0.07·(T − 288 K))` and by
/// its runoff relative to Earth's. Losses are first order in each pool, and
/// `dS/dt = I − k·S` is integrated exactly, so any step length lands on the
/// same state. The values come from Earth and are rescaled per unit
/// area (`docs/plans/2026-09-29-biogeochemistry-constants.md`). Does nothing
/// before the soil pools are sized.
pub fn exchange_external_nutrients(
    ledger: &mut Ledger,
    biosphere: &mut crate::biosphere::BiosphereSystem,
    land: &LandNutrientDrivers<'_>,
    dt_seconds: f64,
) -> ExternalNutrientFluxes {
    let mut fluxes = ExternalNutrientFluxes::default();
    if !dt_seconds.is_finite() || dt_seconds <= 0.0 {
        return fluxes;
    }
    let (Some(soil_n), Some(soil_p)) = (biosphere.soil_nitrogen_kg, biosphere.soil_phosphorus_kg)
    else {
        return fluxes;
    };
    let years = dt_seconds / (365.25 * 86_400.0);
    let spec = land.spec;
    let (mut npp_kgc_yr, mut weathering_kg_yr) = (0.0, 0.0);
    for row in 0..spec.nlat {
        let area = cell_area(spec, row, land.radius_m);
        for col in 0..spec.nlon {
            if grid_value(land.elevation_m, row, col) <= 0.0 {
                continue;
            }
            let npp = land
                .npp_kgc_m2_yr
                .get(row * spec.nlon + col)
                .copied()
                .unwrap_or(0.0);
            if npp.is_finite() {
                npp_kgc_yr += npp.max(0.0) * area;
            }
            let temperature = grid_value(land.surface_temperature_k, row, col);
            let runoff = grid_value(land.runoff_mm_day, row, col).max(0.0);
            let rate = PHOSPHORUS_WEATHERING_KG_M2_YR
                * (PHOSPHORUS_WEATHERING_TEMPERATURE_SENSITIVITY
                    * (temperature - PHOSPHORUS_WEATHERING_REFERENCE_TEMPERATURE_K))
                    .exp()
                * runoff
                / PHOSPHORUS_WEATHERING_REFERENCE_RUNOFF_MM_DAY;
            if rate.is_finite() {
                weathering_kg_yr += rate * area;
            }
        }
    }
    // `dS/dt = I − k·S` integrated exactly over the step:
    // `S(t) = I/k + (S₀ − I/k)·e^(−k·t)`, so the loss is what the input and
    // the starting pool leave behind.
    let relax = |pool: f64, input_kg_yr: f64, k_per_yr: f64| {
        let equilibrium = input_kg_yr / k_per_yr;
        let end = equilibrium + (pool - equilibrium) * (-k_per_yr * years).exp();
        let input = input_kg_yr * years;
        (input, (pool + input - end).max(0.0))
    };
    let nitrogen_input_kg_yr = NITROGEN_FIXATION_PER_NPP_CARBON * npp_kgc_yr;
    (fluxes.nitrogen_fixed_kg, fluxes.nitrogen_lost_kg) = relax(
        soil_n.max(0.0),
        nitrogen_input_kg_yr,
        NITROGEN_LOSS_RATE_PER_YEAR,
    );
    (fluxes.phosphorus_weathered_kg, fluxes.phosphorus_lost_kg) = relax(
        soil_p.max(0.0),
        weathering_kg_yr,
        PHOSPHORUS_LOSS_RATE_PER_YEAR,
    );

    biosphere.soil_nitrogen_kg = Some(soil_n + fluxes.nitrogen_fixed_kg - fluxes.nitrogen_lost_kg);
    biosphere.soil_phosphorus_kg =
        Some(soil_p + fluxes.phosphorus_weathered_kg - fluxes.phosphorus_lost_kg);
    for (source, sink, amount, kind) in [
        (
            Reservoir::AtmosN2,
            Reservoir::SoilNitrogen,
            fluxes.nitrogen_fixed_kg,
            FluxKind::Nitrogen,
        ),
        (
            Reservoir::SoilNitrogen,
            Reservoir::AtmosN2,
            fluxes.nitrogen_lost_kg,
            FluxKind::Nitrogen,
        ),
        (
            Reservoir::CrustPhosphorus,
            Reservoir::SoilPhosphorus,
            fluxes.phosphorus_weathered_kg,
            FluxKind::Phosphorus,
        ),
        (
            Reservoir::SoilPhosphorus,
            Reservoir::CrustPhosphorus,
            fluxes.phosphorus_lost_kg,
            FluxKind::Phosphorus,
        ),
    ] {
        if amount > 0.0 {
            ledger.push(FluxEntry::new(source, sink, amount, kind));
        }
    }
    fluxes
}

/// Decompose detritus to CO₂ over `dt_seconds` and book it.
///
/// First-order decay, `D · (1 − e^(−k·t))`, at the canon decomposition flux
/// `k = k_dec,base · Q10^((T − T_ref)/10) · f_O2` (`BIOSPHERE_EQUATIONS.md`
/// § 7.2–7.5) times the peaked moisture response
/// `f_W = exp(−(W − W_opt)² / 2σ_W²)` (§ 7.4). `T` is the mean surface
/// temperature, `W` the area-weighted mean land soil moisture fraction, and
/// `f_O2 = 1` because the atmosphere is oxic. Returns the kg C decomposed.
pub fn decompose_detritus(
    ledger: &mut Ledger,
    climate: &mut crate::climate::ClimateState,
    biosphere: &mut crate::biosphere::BiosphereSystem,
    kg_per_ppm: f64,
    soil_moisture: f64,
    dt_seconds: f64,
) -> f64 {
    let stock = biosphere.detritus_carbon_kg;
    if stock <= 0.0 || !dt_seconds.is_finite() || dt_seconds <= 0.0 || kg_per_ppm <= 0.0 {
        return 0.0;
    }
    let temperature_k = climate.surface_temperature.average();
    let rate = DECOMPOSITION_RATE_PER_YEAR
        * DECOMPOSITION_Q10.powf((temperature_k - DECOMPOSITION_REFERENCE_TEMPERATURE_K) / 10.0)
        * decomposition_moisture_response(soil_moisture);
    let years = dt_seconds / (365.25 * 86_400.0);
    let decomposed = stock * -(-rate * years).exp_m1();
    if !decomposed.is_finite() || decomposed <= 0.0 {
        return 0.0;
    }
    let decomposed = decomposed.min(stock);
    biosphere.detritus_carbon_kg = stock - decomposed;
    // Decomposition mineralizes the detritus' nutrients at the same rate.
    let fraction = decomposed / stock;
    for (kind, detritus, soil) in [
        (
            FluxKind::Nitrogen,
            Reservoir::DetritusNitrogen,
            Reservoir::SoilNitrogen,
        ),
        (
            FluxKind::Phosphorus,
            Reservoir::DetritusPhosphorus,
            Reservoir::SoilPhosphorus,
        ),
    ] {
        let (detritus_kg, soil_kg) = match kind {
            FluxKind::Nitrogen => (
                &mut biosphere.detritus_nitrogen_kg,
                &mut biosphere.soil_nitrogen_kg,
            ),
            _ => (
                &mut biosphere.detritus_phosphorus_kg,
                &mut biosphere.soil_phosphorus_kg,
            ),
        };
        let released = *detritus_kg * fraction;
        if released > 0.0 {
            *detritus_kg -= released;
            if let Some(soil_kg) = soil_kg.as_mut() {
                *soil_kg += released;
            }
            ledger.push(FluxEntry::new(detritus, soil, released, kind));
        }
    }
    add_atmospheric_carbon(climate, decomposed, kg_per_ppm);
    ledger.push(FluxEntry::new(
        Reservoir::DetritusCarbon,
        Reservoir::AtmosCO2,
        decomposed,
        FluxKind::Carbon,
    ));
    // Aerobic decomposition binds free O₂ back into CO₂.
    exchange_bound_oxygen(ledger, climate, -decomposed * O2_PER_CARBON);
    decomposed
}

/// Free `kg` of O₂ from atmospheric CO₂ (positive) or bind it back into CO₂
/// (negative), and book it.
fn exchange_bound_oxygen(ledger: &mut Ledger, climate: &mut crate::climate::ClimateState, kg: f64) {
    if let Some(o2) = climate.atmospheric_o2_kg.as_mut() {
        *o2 += kg;
    }
    book_signed(
        ledger,
        FluxKind::Oxygen,
        Reservoir::AtmosCO2,
        Reservoir::AtmosO2,
        Reservoir::AtmosCO2,
        kg,
    );
}

/// Book a change in atmospheric CO₂ (ppm) against `boundary`, which
/// supplies it when the concentration rose and receives it when it fell.
pub fn book_atmospheric_co2_change(
    ledger: &mut Ledger,
    boundary: Reservoir,
    kg_per_ppm: f64,
    before_ppm: f64,
    after_ppm: f64,
) {
    let carbon_kg = (after_ppm - before_ppm) * kg_per_ppm;
    book_signed(
        ledger,
        FluxKind::Carbon,
        boundary,
        Reservoir::AtmosCO2,
        boundary,
        carbon_kg,
    );
    // The CO₂ carries its oxygen with it.
    book_signed(
        ledger,
        FluxKind::Oxygen,
        boundary,
        Reservoir::AtmosCO2,
        boundary,
        carbon_kg * O2_PER_CARBON,
    );
}

/// Book `amount` of `kind` into `reservoir` from `source` when positive, or
/// out of `reservoir` to `sink` when negative. Zero books nothing.
pub fn book_signed(
    ledger: &mut Ledger,
    kind: FluxKind,
    source: Reservoir,
    reservoir: Reservoir,
    sink: Reservoir,
    amount: f64,
) {
    if amount > 0.0 {
        ledger.push(FluxEntry::new(source, reservoir, amount, kind));
    } else if amount < 0.0 {
        ledger.push(FluxEntry::new(reservoir, sink, -amount, kind));
    }
}

/// Book the climate step's change to surface heat.
///
/// For each heat reservoir, geothermal heat from the planet's interior
/// ([`Reservoir::VolcanicHeat`]) enters at its real rate over the step. The
/// tides step has already booked tidal heating into the ocean. The rest of
/// the measured change is the climate model's net radiative exchange: net
/// absorbed sunlight in from [`Reservoir::TOAInsolation`] when the store
/// gained heat, net longwave loss to [`Reservoir::SpaceRadiation`] when it
/// lost heat.
#[allow(clippy::too_many_arguments)]
pub fn book_climate_step(
    ledger: &mut Ledger,
    spec: &GridSpec,
    radius_m: f64,
    elevation_m: &Grid2<f64>,
    before_k: &Grid2<f64>,
    after_k: &Grid2<f64>,
    geothermal_w_m2: &Grid2<f64>,
    tidal_heating_j: f64,
    dt_seconds: f64,
) {
    let (land_before, ocean_before) = heat_content_j(spec, radius_m, before_k, elevation_m);
    let (land_after, ocean_after) = heat_content_j(spec, radius_m, after_k, elevation_m);

    let (mut geo_land, mut geo_ocean) = (0.0, 0.0);
    for row in 0..spec.nlat {
        let area = cell_area(spec, row, radius_m);
        for col in 0..spec.nlon {
            let joules = grid_value(geothermal_w_m2, row, col).max(0.0) * area * dt_seconds;
            match heat_reservoir(grid_value(elevation_m, row, col)) {
                Reservoir::OceanHeat => geo_ocean += joules,
                _ => geo_land += joules,
            }
        }
    }

    for (reservoir, change, geothermal, tidal) in [
        (
            Reservoir::SurfaceEnergy,
            land_after - land_before,
            geo_land,
            0.0,
        ),
        (
            Reservoir::OceanHeat,
            ocean_after - ocean_before,
            geo_ocean,
            tidal_heating_j,
        ),
    ] {
        if geothermal > 0.0 {
            ledger.push(FluxEntry::new(
                Reservoir::VolcanicHeat,
                reservoir,
                geothermal,
                FluxKind::Energy,
            ));
        }
        book_signed(
            ledger,
            FluxKind::Energy,
            Reservoir::TOAInsolation,
            reservoir,
            Reservoir::SpaceRadiation,
            change - geothermal - tidal,
        );
    }
}

/// Book the hydrology step's water budget ([`crate::hydrology::HydrologyBudget`]).
///
/// Rain enters surface water from the atmosphere; it infiltrates the soil,
/// evaporates from either store, drains from the soil to groundwater (booked
/// to [`Reservoir::Rivers`], the boundary the recharge feeds), and runoff
/// reaching the coast joins the ocean. Sea cells' soil is held saturated by
/// the ocean.
pub fn book_hydrology_step(ledger: &mut Ledger, budget: &crate::hydrology::HydrologyBudget) {
    for (source, sink, amount) in [
        (
            Reservoir::Atmosphere,
            Reservoir::SurfaceWater,
            budget.precipitation_kg,
        ),
        (
            Reservoir::SurfaceWater,
            Reservoir::SoilWater,
            budget.infiltration_kg,
        ),
        (
            Reservoir::SoilWater,
            Reservoir::SurfaceWater,
            budget.soil_excess_kg,
        ),
        (
            Reservoir::SurfaceWater,
            Reservoir::Atmosphere,
            budget.surface_evaporation_kg,
        ),
        (
            Reservoir::SoilWater,
            Reservoir::Atmosphere,
            budget.soil_evaporation_kg,
        ),
        (
            Reservoir::SoilWater,
            Reservoir::Rivers,
            budget.deep_drainage_kg,
        ),
        (
            Reservoir::SurfaceWater,
            Reservoir::Ocean,
            budget.surface_to_ocean_kg,
        ),
    ] {
        if amount > 0.0 {
            ledger.push(FluxEntry::new(source, sink, amount, FluxKind::Water));
        }
    }
    book_signed(
        ledger,
        FluxKind::Water,
        Reservoir::Ocean,
        Reservoir::SoilWater,
        Reservoir::Ocean,
        budget.ocean_to_sea_soil_kg,
    );
}

/// Result of one tick's stock audit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StockAuditReport {
    /// Tick the audit ran on.
    pub tick: u64,
    /// Whether every audited reservoir closed within [`RELATIVE_TOLERANCE`].
    pub conserved: bool,
    /// Largest |residual| / stock over the energy reservoirs.
    pub energy_relative_residual: f64,
    /// Largest |residual| / stock over the water reservoirs.
    pub water_relative_residual: f64,
    /// Largest |residual| / stock over the carbon reservoirs.
    #[serde(default)]
    pub carbon_relative_residual: f64,
    /// Largest |residual| / stock over the oxygen reservoirs.
    #[serde(default)]
    pub oxygen_relative_residual: f64,
    /// Largest |residual| / stock over the nitrogen reservoirs.
    #[serde(default)]
    pub nitrogen_relative_residual: f64,
    /// Largest |residual| / stock over the phosphorus reservoirs.
    #[serde(default)]
    pub phosphorus_relative_residual: f64,
    /// Human-readable description of the first failure, if any.
    pub failure: Option<String>,
}

/// Audit the tick's ledger against stocks measured at its start and end.
pub fn audit(
    ledger: &Ledger,
    tick: u64,
    before: &ReservoirStocks,
    after: &ReservoirStocks,
) -> StockAuditReport {
    let mut report = StockAuditReport {
        tick,
        conserved: true,
        ..StockAuditReport::default()
    };
    for kind in AUDITED_KINDS {
        let residuals = match ledger.stock_residuals(kind, before, after) {
            Ok(residuals) => residuals,
            Err(err) => {
                report.conserved = false;
                report.failure.get_or_insert_with(|| err.to_string());
                continue;
            }
        };
        let mut worst: f64 = 0.0;
        for (reservoir, residual) in residuals {
            let scale = before
                .get(reservoir, kind)
                .unwrap_or(0.0)
                .abs()
                .max(after.get(reservoir, kind).unwrap_or(0.0).abs())
                .max(1.0);
            let relative = residual.abs() / scale;
            if !relative.is_finite() || relative > RELATIVE_TOLERANCE {
                report.conserved = false;
                report.failure.get_or_insert_with(|| {
                    format!(
                        "{kind} not conserved in {reservoir}: residual {residual:e} \
                         ({relative:e} of the stock)"
                    )
                });
            }
            worst = worst.max(relative);
        }
        match kind {
            FluxKind::Energy => report.energy_relative_residual = worst,
            FluxKind::Water => report.water_relative_residual = worst,
            FluxKind::Carbon => report.carbon_relative_residual = worst,
            FluxKind::Oxygen => report.oxygen_relative_residual = worst,
            FluxKind::Nitrogen => report.nitrogen_relative_residual = worst,
            FluxKind::Phosphorus => report.phosphorus_relative_residual = worst,
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> GridSpec {
        GridSpec::new(4, 8)
    }

    fn nutrient_world(spec: &GridSpec) -> (Grid2<f64>, Grid2<f64>, Grid2<f64>, Vec<f64>) {
        // Every cell land, at Earth's reference temperature and runoff and
        // Earth's mean land NPP.
        let elevation = Grid2::new(spec, 100.0);
        let temperature = Grid2::new(spec, PHOSPHORUS_WEATHERING_REFERENCE_TEMPERATURE_K);
        let runoff = Grid2::new(spec, PHOSPHORUS_WEATHERING_REFERENCE_RUNOFF_MM_DAY);
        let npp = vec![58.0e12 / 1.49e14; spec.nlat * spec.nlon];
        (elevation, temperature, runoff, npp)
    }

    fn land_area(spec: &GridSpec, radius_m: f64) -> f64 {
        (0..spec.nlat)
            .map(|row| cell_area(spec, row, radius_m) * spec.nlon as f64)
            .sum()
    }

    #[test]
    fn moisture_response_peaks_at_the_optimum_and_halves_about_0_31_away() {
        assert!(
            (decomposition_moisture_response(DECOMPOSITION_MOISTURE_OPTIMUM) - 1.0).abs() < 1e-15
        );
        for w in [
            DECOMPOSITION_MOISTURE_OPTIMUM - 0.31,
            DECOMPOSITION_MOISTURE_OPTIMUM + 0.31,
        ] {
            assert!(
                (decomposition_moisture_response(w) - 0.5).abs() < 0.005,
                "{w}"
            );
        }
        assert!(decomposition_moisture_response(0.0) < decomposition_moisture_response(0.3));
    }

    #[test]
    fn earth_conditions_reproduce_earths_external_nutrient_fluxes() {
        let spec = spec();
        let radius = 6.371e6;
        let (elevation, temperature, runoff, npp) = nutrient_world(&spec);
        let land = land_area(&spec, radius);
        let mut biosphere = crate::biosphere::BiosphereSystem {
            soil_nitrogen_kg: Some(SOIL_NITROGEN_DENSITY_KG_M2 * land),
            soil_phosphorus_kg: Some(SOIL_PHOSPHORUS_DENSITY_KG_M2 * land),
            ..Default::default()
        };
        let mut ledger = Ledger::new();
        let year = 365.25 * 86_400.0;
        let drivers = LandNutrientDrivers {
            spec: &spec,
            radius_m: radius,
            elevation_m: &elevation,
            surface_temperature_k: &temperature,
            runoff_mm_day: &runoff,
            npp_kgc_m2_yr: &npp,
        };
        let fluxes = exchange_external_nutrients(&mut ledger, &mut biosphere, &drivers, year);

        // Per m² of land, Earth's fixation (≈0.67 g N) and weathering
        // (≈0.117 g P) per year.
        assert!((fluxes.nitrogen_fixed_kg / land - 100.0e9 / 1.49e14).abs() < 1e-12);
        assert!(
            (fluxes.phosphorus_weathered_kg / land - PHOSPHORUS_WEATHERING_KG_M2_YR).abs() < 1e-15
        );
        // At Earth's soil densities the first-order losses nearly balance.
        assert!((fluxes.nitrogen_lost_kg / fluxes.nitrogen_fixed_kg - 1.0).abs() < 1e-9);
        assert!((fluxes.phosphorus_lost_kg / fluxes.phosphorus_weathered_kg - 1.0).abs() < 1e-9);

        // Every flux is booked against a boundary, and the soil pools'
        // recorded net inflow is their actual change.
        for (kind, reservoir, before, after, boundary) in [
            (
                FluxKind::Nitrogen,
                Reservoir::SoilNitrogen,
                SOIL_NITROGEN_DENSITY_KG_M2 * land,
                biosphere.soil_nitrogen_kg.unwrap(),
                Reservoir::AtmosN2,
            ),
            (
                FluxKind::Phosphorus,
                Reservoir::SoilPhosphorus,
                SOIL_PHOSPHORUS_DENSITY_KG_M2 * land,
                biosphere.soil_phosphorus_kg.unwrap(),
                Reservoir::CrustPhosphorus,
            ),
        ] {
            assert!(boundary.is_boundary());
            let recorded = ledger.net_flow_by_reservoir(kind)[&reservoir];
            assert!(((after - before) - recorded).abs() <= before * 1e-15);
        }
    }

    #[test]
    fn warmth_speeds_and_drought_slows_phosphorus_weathering() {
        let spec = spec();
        let radius = 6.371e6;
        let (elevation, temperature, runoff, npp) = nutrient_world(&spec);
        let weathered = |temperature: &Grid2<f64>, runoff: &Grid2<f64>| {
            let mut biosphere = crate::biosphere::BiosphereSystem {
                soil_nitrogen_kg: Some(0.0),
                soil_phosphorus_kg: Some(0.0),
                ..Default::default()
            };
            let drivers = LandNutrientDrivers {
                spec: &spec,
                radius_m: radius,
                elevation_m: &elevation,
                surface_temperature_k: temperature,
                runoff_mm_day: runoff,
                npp_kgc_m2_yr: &npp,
            };
            exchange_external_nutrients(&mut Ledger::new(), &mut biosphere, &drivers, 86_400.0)
                .phosphorus_weathered_kg
        };
        let base = weathered(&temperature, &runoff);
        let warm = weathered(&Grid2::new(&spec, 298.0), &runoff);
        let dry = weathered(&temperature, &Grid2::new(&spec, 0.37));
        assert!((warm / base - (0.7f64).exp()).abs() < 1e-12);
        assert!((dry / base - 0.5).abs() < 1e-12);
    }

    #[test]
    fn one_long_step_equals_many_short_ones() {
        let spec = spec();
        let radius = 6.371e6;
        let (elevation, temperature, runoff, npp) = nutrient_world(&spec);
        let drivers = LandNutrientDrivers {
            spec: &spec,
            radius_m: radius,
            elevation_m: &elevation,
            surface_temperature_k: &temperature,
            runoff_mm_day: &runoff,
            npp_kgc_m2_yr: &npp,
        };
        let fresh = || crate::biosphere::BiosphereSystem {
            soil_nitrogen_kg: Some(1.0e12),
            soil_phosphorus_kg: Some(1.0e10),
            ..Default::default()
        };
        let decade = 10.0 * 365.25 * 86_400.0;
        let mut once = fresh();
        exchange_external_nutrients(&mut Ledger::new(), &mut once, &drivers, decade);
        let mut often = fresh();
        for _ in 0..3650 {
            exchange_external_nutrients(&mut Ledger::new(), &mut often, &drivers, decade / 3650.0);
        }
        let close = |a: f64, b: f64| (a / b - 1.0).abs() < 1e-9;
        assert!(close(
            once.soil_nitrogen_kg.unwrap(),
            often.soil_nitrogen_kg.unwrap()
        ));
        assert!(close(
            once.soil_phosphorus_kg.unwrap(),
            often.soil_phosphorus_kg.unwrap()
        ));
    }

    #[test]
    fn soil_pools_relax_to_input_over_loss_rate() {
        let spec = spec();
        let radius = 6.371e6;
        let (elevation, temperature, runoff, npp) = nutrient_world(&spec);
        let land = land_area(&spec, radius);
        let mut biosphere = crate::biosphere::BiosphereSystem {
            soil_nitrogen_kg: Some(0.0),
            soil_phosphorus_kg: Some(0.0),
            ..Default::default()
        };
        let drivers = LandNutrientDrivers {
            spec: &spec,
            radius_m: radius,
            elevation_m: &elevation,
            surface_temperature_k: &temperature,
            runoff_mm_day: &runoff,
            npp_kgc_m2_yr: &npp,
        };
        let century = 100.0 * 365.25 * 86_400.0;
        for _ in 0..200 {
            exchange_external_nutrients(&mut Ledger::new(), &mut biosphere, &drivers, century);
        }
        // Twenty thousand years: both pools sit at their steady state, which
        // at Earth's inputs is Earth's soil density.
        let n = biosphere.soil_nitrogen_kg.unwrap() / land;
        let p = biosphere.soil_phosphorus_kg.unwrap() / land;
        assert!((n / SOIL_NITROGEN_DENSITY_KG_M2 - 1.0).abs() < 1e-9, "{n}");
        assert!(
            (p / SOIL_PHOSPHORUS_DENSITY_KG_M2 - 1.0).abs() < 1e-9,
            "{p}"
        );
    }

    #[test]
    fn heat_content_splits_land_and_ocean() {
        let spec = spec();
        let mut elevation = Grid2::new(&spec, 100.0);
        elevation.set(0, 0, -500.0);
        let temps = Grid2::new(&spec, 288.0);
        let (land, ocean) = heat_content_j(&spec, 1.0e6, &temps, &elevation);
        assert!(land > 0.0 && ocean > 0.0);
        let area = spec.cell_area_at_row_m2(0, 1.0e6).unwrap();
        let expected_ocean = crate::climate::surface_heat_capacity_j_m2_k(-500.0) * area * 288.0;
        assert!((ocean - expected_ocean).abs() / expected_ocean < 1e-12);
    }

    #[test]
    fn climate_booking_closes_the_heat_budget() {
        let spec = spec();
        let radius = 1.0e6;
        let mut elevation = Grid2::new(&spec, 100.0);
        elevation.set(1, 1, -200.0);
        let before = Grid2::new(&spec, 280.0);
        let mut after = Grid2::new(&spec, 281.0);
        after.set(2, 3, 279.0);
        let geothermal = Grid2::new(&spec, 0.09);
        let mut ledger = Ledger::new();
        book_climate_step(
            &mut ledger,
            &spec,
            radius,
            &elevation,
            &before,
            &after,
            &geothermal,
            0.0,
            3600.0,
        );
        let stocks = |t: &Grid2<f64>| {
            let (land, ocean) = heat_content_j(&spec, radius, t, &elevation);
            let mut s = ReservoirStocks::new();
            s.set(Reservoir::SurfaceEnergy, FluxKind::Energy, land);
            s.set(Reservoir::OceanHeat, FluxKind::Energy, ocean);
            s
        };
        let residuals = ledger
            .stock_residuals(FluxKind::Energy, &stocks(&before), &stocks(&after))
            .unwrap();
        for (_, r) in residuals {
            assert!(r.abs() < 1.0, "residual {r}");
        }
    }

    #[test]
    fn audit_flags_an_unbooked_change() {
        let mut before = ReservoirStocks::new();
        before.set(Reservoir::SoilWater, FluxKind::Water, 1.0e15);
        before.set(Reservoir::SurfaceEnergy, FluxKind::Energy, 1.0e24);
        let mut after = before.clone();
        after.set(Reservoir::SoilWater, FluxKind::Water, 1.001e15);
        let report = audit(&Ledger::new(), 3, &before, &after);
        assert!(!report.conserved);
        assert!(report.water_relative_residual > RELATIVE_TOLERANCE);
        assert!(report.failure.unwrap().contains("SoilWater"));

        let clean = audit(&Ledger::new(), 3, &before, &before);
        assert!(clean.conserved, "{:?}", clean.failure);
    }

    #[test]
    fn signed_booking_picks_direction() {
        let mut ledger = Ledger::new();
        book_signed(
            &mut ledger,
            FluxKind::Energy,
            Reservoir::TOAInsolation,
            Reservoir::SurfaceEnergy,
            Reservoir::SpaceRadiation,
            -5.0,
        );
        let entry = ledger.entries()[0];
        assert_eq!(entry.source, Reservoir::SurfaceEnergy);
        assert_eq!(entry.sink, Reservoir::SpaceRadiation);
        assert_eq!(entry.amount, 5.0);
    }

    #[test]
    fn one_ppm_of_co2_is_the_atmospheres_moles_times_carbon_mass() {
        let canon = mk_core::canon::CanonLocked::default();
        let per_ppm = atmospheric_carbon_kg_per_ppm(&canon);
        // 182 kPa over a 1.9113e7 m planet at 19.62 m/s²: ~4.26e19 kg of
        // air, ~1.44e21 mol, so one ppm holds ~1.73e13 kg of carbon.
        assert!((per_ppm / 1.73e13 - 1.0).abs() < 0.01, "{per_ppm:e}");
    }

    #[test]
    fn biomass_growth_comes_out_of_the_air() {
        let spec = spec();
        let mut climate = crate::climate::ClimateState::new(&spec, 288.0);
        let mut world = crate::world_integration::WorldState::new(
            std::sync::Arc::new(mk_core::canon::CanonLocked::default()),
            [3u8; 32],
        );
        world.step_world(3600).unwrap();
        let biosphere = &mut world.biosphere_state;
        assert!(!biosphere.species.is_empty());
        let before_total = biomass_carbon_kg(&biosphere.species);
        let before = biomass_carbon_by_class_kg(&biosphere.species);
        biosphere.species[0].population_size += 1000;
        let gained = biomass_carbon_kg(&biosphere.species) - before_total;
        assert!(gained > 0.0);

        let per_ppm = 1.0e13;
        let mut ledger = Ledger::new();
        exchange_biomass_carbon(&mut ledger, &mut climate, biosphere, per_ppm, before);
        let drawdown_ppm = gained / per_ppm;
        assert!((280.0 - climate.co2_concentration - drawdown_ppm).abs() < 1e-12);
        let entry = ledger.entries()[0];
        assert_eq!(entry.source, Reservoir::AtmosCO2);
        assert_eq!(entry.sink, Reservoir::BiomassCarbon);
        assert_eq!(entry.amount, gained);
    }

    #[test]
    fn mortality_feeds_detritus_which_decays_back_to_the_air() {
        let mut world = crate::world_integration::WorldState::new(
            std::sync::Arc::new(mk_core::canon::CanonLocked::default()),
            [3u8; 32],
        );
        world.step_world(3600).unwrap();
        let biosphere = &mut world.biosphere_state;
        biosphere.detritus_carbon_kg = 0.0;
        let before_total = biomass_carbon_kg(&biosphere.species);
        let before = biomass_carbon_by_class_kg(&biosphere.species);
        let index = biosphere
            .species
            .iter()
            .position(|s| !is_consumer_category(&s.category) && s.population_size >= 1000)
            .expect("a producer species with 1000 individuals");
        biosphere.species[index].population_size -= 1000;
        let died = before_total - biomass_carbon_kg(&biosphere.species);

        let per_ppm = 1.0e13;
        let mut climate =
            crate::climate::ClimateState::new(&spec(), DECOMPOSITION_REFERENCE_TEMPERATURE_K);
        let mut ledger = Ledger::new();
        let co2_before = climate.co2_concentration;
        let dead = exchange_biomass_carbon(&mut ledger, &mut climate, biosphere, per_ppm, before);
        // Dead producer tissue: 94 % to detritus, 6 % respired at once.
        let to_detritus = died * PRODUCER_MORTALITY_TO_DETRITUS;
        assert!((biosphere.detritus_carbon_kg / to_detritus - 1.0).abs() < 1e-12);
        assert!((dead.total_kg / died - 1.0).abs() < 1e-12);
        let respired_ppm = (died - to_detritus) / per_ppm;
        let gained_ppm = climate.co2_concentration - co2_before;
        // The gain is read off 280 ppm, so allow its rounding.
        assert!(
            (gained_ppm - respired_ppm).abs()
                <= respired_ppm * 1e-9 + 4.0 * co2_before * f64::EPSILON
        );
        let died = biosphere.detritus_carbon_kg;

        // Measure the release from zero, so it is not lost in the rounding
        // of 280 ppm.
        climate.co2_concentration = 0.0;
        // At the reference temperature, one year decays 1 − e^(−k).
        let year = 365.25 * 86_400.0;
        let decomposed = decompose_detritus(
            &mut ledger,
            &mut climate,
            biosphere,
            per_ppm,
            DECOMPOSITION_MOISTURE_OPTIMUM,
            year,
        );
        let expected = died * (1.0 - (-DECOMPOSITION_RATE_PER_YEAR).exp());
        assert!((decomposed / expected - 1.0).abs() < 1e-12);
        assert!((biosphere.detritus_carbon_kg - (died - decomposed)).abs() <= died * 1e-15);
        assert!((climate.co2_concentration * per_ppm / decomposed - 1.0).abs() < 1e-12);

        // Ten kelvin warmer, decay runs Q10 times faster.
        let mut warm = crate::climate::ClimateState::new(
            &spec(),
            DECOMPOSITION_REFERENCE_TEMPERATURE_K + 10.0,
        );
        let mut pool = crate::biosphere::BiosphereSystem {
            detritus_carbon_kg: 1.0e6,
            ..Default::default()
        };
        let fast = decompose_detritus(
            &mut ledger,
            &mut warm,
            &mut pool,
            per_ppm,
            DECOMPOSITION_MOISTURE_OPTIMUM,
            year / 100.0,
        );
        let slow_rate = -(-DECOMPOSITION_RATE_PER_YEAR / 100.0).exp_m1();
        let fast_rate = -(-DECOMPOSITION_Q10 * DECOMPOSITION_RATE_PER_YEAR / 100.0).exp_m1();
        assert!((fast / 1.0e6 - fast_rate).abs() < 1e-12 && fast_rate > 2.09 * slow_rate);
    }

    #[test]
    fn dying_tissue_sends_detritus_its_c_n_share_and_mineralizes_the_rest() {
        let canon = mk_core::canon::CanonLocked::default();
        let mut world = crate::world_integration::WorldState::new(
            std::sync::Arc::new(canon.clone()),
            [3u8; 32],
        );
        world.step_world(3600).unwrap();
        let biosphere = &mut world.biosphere_state;
        let index = biosphere
            .species
            .iter()
            .position(|s| {
                crate::biosphere::is_fauna_category(&s.category) && s.population_size >= 1000
            })
            .expect("a fauna species with 1000 individuals");
        let carbon_before = biomass_carbon_kg(&biosphere.species);
        let classes_before = biomass_carbon_by_class_kg(&biosphere.species);
        let nutrients_before = biomass_nutrients_kg(&biosphere.species, &canon);
        let (detritus_n0, detritus_p0) = (
            biosphere.detritus_nitrogen_kg,
            biosphere.detritus_phosphorus_kg,
        );
        biosphere.species[index].population_size -= 1000;
        let died = carbon_before - biomass_carbon_kg(&biosphere.species);
        let (n_after, p_after) = biomass_nutrients_kg(&biosphere.species, &canon);
        let (dead_n, dead_p) = (nutrients_before.0 - n_after, nutrients_before.1 - p_after);

        let mut ledger = Ledger::new();
        let mut climate = crate::climate::ClimateState::new(&spec(), 288.0);
        let dead =
            exchange_biomass_carbon(&mut ledger, &mut climate, biosphere, 1.0e13, classes_before);
        assert!((dead.total_kg / died - 1.0).abs() < 1e-12);
        exchange_biomass_nutrients(&mut ledger, biosphere, &canon, nutrients_before, dead);

        // Consumer tissue (C:N 7.2) is richer in nitrogen than detritus (C:N
        // 24), and only 89 % of it becomes detritus: detritus keeps
        // 0.89 · 7.2/24 of its nitrogen and the rest mineralizes.
        let into_detritus: f64 = ledger
            .entries()
            .iter()
            .filter(|e| {
                e.source == Reservoir::BiomassNitrogen && e.sink == Reservoir::DetritusNitrogen
            })
            .map(|e| e.amount)
            .sum();
        assert!(biosphere.detritus_nitrogen_kg >= detritus_n0);
        // `dead_n` is the difference of two large biomass totals, so it
        // carries ~1e-9 relative cancellation error; the ledger amount does
        // not.
        let expected =
            dead_n * CONSUMER_MORTALITY_TO_DETRITUS * canon.cn_ratio_cons / canon.cn_ratio_det;
        assert!(
            (into_detritus / expected - 1.0).abs() < 1e-7,
            "{into_detritus:e} vs {expected:e}"
        );
        let mineralized: f64 = ledger
            .entries()
            .iter()
            .filter(|e| e.source == Reservoir::BiomassNitrogen && e.sink == Reservoir::SoilNitrogen)
            .map(|e| e.amount)
            .sum();
        assert!(((into_detritus + mineralized) / dead_n - 1.0).abs() < 1e-7);
        let detrital_p = dead_p * CONSUMER_MORTALITY_TO_DETRITUS;
        assert!(
            (biosphere.detritus_phosphorus_kg - detritus_p0 - detrital_p).abs() <= dead_p * 1e-9
        );
    }
}
