use super::evolution::Species;
use super::genetics::Genome;
/// Apex predator limits and fragility for MARR'KENA animals
///
/// Implements hard caps on predator populations, energy requirements,
/// and anti-civilization safeguards to prevent proto-civilization emergence.
use mk_core::rng::{RngExt, RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

/// Maximum population density for apex predators (individuals per km²)
pub const APEX_PREDATOR_MAX_DENSITY: f64 = 0.01;

/// Minimum territory size per apex predator (km²)
pub const APEX_PREDATOR_MIN_TERRITORY: f64 = 100.0;

/// Energy requirement multiplier for apex predators (higher than other trophic levels)
pub const APEX_PREDATOR_ENERGY_MULTIPLIER: f64 = 2.5;

/// Intelligence ceiling specifically for apex predators (lower than general ceiling)
pub const APEX_PREDATOR_INTELLIGENCE_CEILING: f64 = 0.25;

/// Cooperation limit for apex predators (prevents complex social structures)
pub const APEX_PREDATOR_MAX_COOPERATION_SIZE: u32 = 3;

/// Tool use prohibition threshold (any tool-like behavior triggers collapse)
pub const TOOL_USE_PROHIBITION_THRESHOLD: f64 = 0.1;

/// Apex predator fragility factors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApexPredatorFragility {
    /// Energy scarcity fragility (0-1: how vulnerable to food shortages)
    pub energy_scarcity_fragility: f64,

    /// Disease susceptibility (0-1: how vulnerable to disease)
    pub disease_susceptibility: f64,

    /// Reproductive fragility (0-1: how vulnerable to reproductive failure)
    pub reproductive_fragility: f64,

    /// Environmental fragility (0-1: how vulnerable to environmental changes)
    pub environmental_fragility: f64,

    /// Social fragility (0-1: how vulnerable to social disruption)
    pub social_fragility: f64,

    /// Genetic fragility (0-1: how vulnerable to genetic problems)
    pub genetic_fragility: f64,
}

/// Apex predator population constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApexPredatorConstraints {
    /// Live species this population tracks (0 for a free-standing
    /// population built directly from a genome).
    #[serde(default)]
    pub species_id: u64,

    /// Current population size
    pub population_size: u64,

    /// Territory size (km²)
    pub territory_size: f64,

    /// Population density (individuals per km²)
    pub population_density: f64,

    /// Energy requirement (J/day per individual)
    pub energy_requirement_j_per_day: f64,

    /// Available energy in territory (J/day)
    pub available_energy_j_per_day: f64,

    /// Energy balance ratio (available/required)
    pub energy_balance_ratio: f64,

    /// Cooperation group size
    pub cooperation_group_size: u32,

    /// Tool use propensity (0-1: higher = more likely to use tools)
    pub tool_use_propensity: f64,

    /// Intelligence index
    pub intelligence_index: f64,

    /// Fragility factors
    pub fragility: ApexPredatorFragility,
}

impl ApexPredatorConstraints {
    /// Create apex predator constraints from genome
    pub fn from_genome(genome: &Genome, territory_size: f64) -> Self {
        let intelligence_index = genome.intelligence_index();

        // Calculate energy requirements
        let basal_rate = genome.basal_metabolic_rate_w();
        let daily_energy_requirement = basal_rate * 86400.0 * APEX_PREDATOR_ENERGY_MULTIPLIER;

        // Initialize population at sustainable level
        let sustainable_population =
            Self::calculate_sustainable_population(territory_size, daily_energy_requirement);

        // Calculate fragility factors
        let fragility = Self::calculate_fragility_factors(genome);

        // Calculate tool use propensity (based on intelligence)
        let tool_use_propensity = Self::calculate_tool_use_propensity(intelligence_index);

        Self {
            species_id: 0,
            population_size: sustainable_population,
            territory_size,
            population_density: sustainable_population as f64 / territory_size,
            energy_requirement_j_per_day: daily_energy_requirement,
            available_energy_j_per_day: 0.0, // Will be updated by environment
            energy_balance_ratio: 0.0,       // Will be calculated
            cooperation_group_size: 1,       // Start solitary
            tool_use_propensity,
            intelligence_index,
            fragility,
        }
    }

    /// Calculate sustainable population for given territory
    fn calculate_sustainable_population(
        territory_size: f64,
        _energy_requirement_per_individual: f64,
    ) -> u64 {
        let max_population_by_density = (territory_size * APEX_PREDATOR_MAX_DENSITY) as u64;
        let max_population_by_territory = (territory_size / APEX_PREDATOR_MIN_TERRITORY) as u64;

        // Take the more restrictive limit
        max_population_by_density.min(max_population_by_territory)
    }

    /// Calculate fragility factors based on genome traits
    fn calculate_fragility_factors(genome: &Genome) -> ApexPredatorFragility {
        let body_mass = genome.body_mass_kg();
        let neural_complexity = genome.neural.intel_index;

        // Larger predators are more vulnerable to energy scarcity
        let energy_scarcity_fragility = (body_mass / 1000.0).min(1.0);

        // Higher neural complexity increases disease susceptibility
        let disease_susceptibility = (neural_complexity as f64 / 15.0).min(1.0);

        // Large body size increases reproductive fragility
        let reproductive_fragility = (body_mass / 500.0).min(1.0);

        // Specialized predators are more environmentally fragile
        let environmental_fragility = match genome.structural.environmental_adaptation {
            0 => 0.8, // Marine specialists
            1 => 0.6, // Terrestrial specialists
            2 => 0.4, // Amphibious generalists
            3 => 0.3, // Generalists
            _ => 0.5,
        };

        // Solitary predators have high social fragility
        let social_fragility = 0.9;

        // Small populations have high genetic fragility
        let genetic_fragility = 0.8;

        ApexPredatorFragility {
            energy_scarcity_fragility,
            disease_susceptibility,
            reproductive_fragility,
            environmental_fragility,
            social_fragility,
            genetic_fragility,
        }
    }

    /// Calculate tool use propensity (should be very low)
    fn calculate_tool_use_propensity(intelligence_index: f64) -> f64 {
        // Tool use propensity increases with intelligence but is heavily penalized
        let base_propensity = intelligence_index / 10000.0;

        let _ = intelligence_index; // Ensure used efficiently

        // Apply heavy penalty to prevent tool use emergence
        let penalty_factor = 0.005; // 99.5% reduction (safer margin)

        base_propensity * penalty_factor
    }

    /// Update available energy from environment
    pub fn update_available_energy(&mut self, environmental_energy_density: f64) {
        self.available_energy_j_per_day = environmental_energy_density * self.territory_size * 0.1; // 10% capture efficiency
        self.energy_balance_ratio = self.available_energy_j_per_day
            / (self.energy_requirement_j_per_day * self.population_size as f64);
    }

    /// Check if population should collapse due to energy scarcity
    pub fn should_collapse_energy(&self) -> bool {
        self.energy_balance_ratio < 0.5 || self.fragility.energy_scarcity_fragility >= 0.7
    }

    /// Check if population should collapse due to disease
    pub fn should_collapse_disease(&self, rng: &RngRegistry, draw: ApexDraw) -> bool {
        let mut rng_disease = rng.stream(draw.key(20));

        // Disease outbreak probability increases with population density and susceptibility
        let outbreak_probability =
            self.population_density * self.fragility.disease_susceptibility * 0.1;

        rng_disease.gen_f64_01() < outbreak_probability
    }

    /// Check if population should collapse due to reproductive failure
    pub fn should_collapse_reproduction(&self) -> bool {
        self.energy_balance_ratio < 0.3 && self.fragility.reproductive_fragility > 0.8
    }

    /// Check if population should collapse due to environmental changes
    pub fn should_collapse_environmental(&self) -> bool {
        self.energy_balance_ratio < 0.4 && self.fragility.environmental_fragility > 0.6
    }

    /// Check if tool use should trigger population collapse
    pub fn should_collapse_tool_use(&self, rng: &RngRegistry, draw: ApexDraw) -> bool {
        // Any significant tool use triggers immediate collapse
        if self.tool_use_propensity > TOOL_USE_PROHIBITION_THRESHOLD {
            return true;
        }

        // Discovery is driven by propensity, with a keyed draw adding
        // per-population, per-step variance around it.
        let discovery_roll = rng.stream(draw.key(21)).gen_f64_01();

        let discovery_probability = self.tool_use_propensity * 0.01 * (0.9 + 0.2 * discovery_roll);
        let threshold = 0.95; // Only trigger at extreme propensity levels
        discovery_probability > threshold
    }

    /// Check if intelligence exceeds apex predator ceiling
    pub fn exceeds_intelligence_ceiling(&self) -> bool {
        self.intelligence_index > APEX_PREDATOR_INTELLIGENCE_CEILING
    }

    /// Check if cooperation exceeds allowed limits
    pub fn exceeds_cooperation_limit(&self) -> bool {
        self.cooperation_group_size > APEX_PREDATOR_MAX_COOPERATION_SIZE
    }

    /// Calculate overall collapse probability
    pub fn calculate_collapse_probability(&self, rng: &RngRegistry, draw: ApexDraw) -> f64 {
        let mut probability: f64 = 0.0;

        if self.should_collapse_energy() {
            probability += 0.3;
        }

        if self.should_collapse_disease(rng, draw) {
            probability += 0.2;
        }

        if self.should_collapse_reproduction() {
            probability += 0.2;
        }

        if self.should_collapse_environmental() {
            probability += 0.15;
        }

        if self.should_collapse_tool_use(rng, draw) {
            probability += 0.5; // Tool use is catastrophic
        }

        if self.exceeds_intelligence_ceiling() {
            probability += 0.4; // Intelligence breach is catastrophic
        }

        if self.exceeds_cooperation_limit() {
            probability += 0.3; // Cooperation breach is serious
        }

        probability.min(1.0)
    }

    /// Apply population collapse
    pub fn apply_collapse(&mut self, collapse_type: CollapseType) {
        match collapse_type {
            CollapseType::Energy => {
                // Reduce population to sustainable level
                let sustainable_pop = (self.population_size as f64 * 0.1) as u64;
                self.population_size = sustainable_pop.max(1);
            }
            CollapseType::Disease => {
                // Random population reduction
                let survival_rate = 0.3; // 70% mortality
                self.population_size = (self.population_size as f64 * survival_rate) as u64;
            }
            CollapseType::Reproduction => {
                // Gradual decline
                self.population_size = (self.population_size as f64 * 0.8) as u64;
            }
            CollapseType::Environmental => {
                // Severe reduction
                self.population_size = (self.population_size as f64 * 0.2) as u64;
            }
            CollapseType::ToolUse => {
                // Complete collapse - tool use is forbidden
                self.population_size = 0;
            }
            CollapseType::Intelligence => {
                // Complete collapse - intelligence breach is forbidden
                self.population_size = 0;
            }
            CollapseType::Cooperation => {
                // Break up large groups
                self.cooperation_group_size = 1;
                self.population_size = (self.population_size as f64 * 0.5) as u64;
            }
        }

        // Update density after population change
        self.population_density = self.population_size as f64 / self.territory_size;
    }

    /// Update cooperation group size based on population
    pub fn update_cooperation(&mut self) {
        // Apex predators naturally form small groups or remain solitary
        if self.population_size <= 1 {
            self.cooperation_group_size = 1;
        } else if self.population_size <= 3 {
            self.cooperation_group_size = self.population_size as u32;
        } else {
            // Larger populations still form small groups
            self.cooperation_group_size = APEX_PREDATOR_MAX_COOPERATION_SIZE;
        }
    }

    /// Check if apex predator constraints are satisfied
    pub fn is_viable(&self) -> bool {
        self.population_size > 0
            && self.population_density <= APEX_PREDATOR_MAX_DENSITY
            && self.energy_balance_ratio >= 0.3
            && !self.exceeds_intelligence_ceiling()
            && !self.exceeds_cooperation_limit()
            && self.tool_use_propensity <= TOOL_USE_PROHIBITION_THRESHOLD
    }
}

/// Identifies one population's random draws in one step, so every
/// population and every step gets independent values.
#[derive(Debug, Clone, Copy)]
pub struct ApexDraw {
    pub lineage: u32,
    pub step: u64,
}

impl ApexDraw {
    fn key(&self, salt: u32) -> RngKey {
        RngKey::new(SubsystemId::Biosphere, salt, self.lineage, self.step)
    }
}

/// Energy content of prey tissue (J/kg, typical vertebrate muscle/fat mix).
const PREY_ENERGY_J_PER_KG: f64 = 7.0e6;
/// Fraction of standing prey biomass produced (and so harvestable without
/// depleting the stock) per year.
const PREY_ANNUAL_PRODUCTION_FRACTION: f64 = 0.3;
/// Mortality rate (per year) of a population receiving no food at all; a
/// full-deficit large carnivore population halves in about two months.
const STARVATION_MORTALITY_PER_YEAR: f64 = 4.0;
/// Minimum body mass (kg) for a carnivore species to occupy the apex tier.
const APEX_MIN_BODY_MASS_KG: f64 = 100.0;

/// Whether a live species occupies the apex-predator tier: a large
/// carnivore.
pub fn is_apex_species(species: &Species) -> bool {
    matches!(
        species.category,
        crate::biosphere::evolution::SpeciesCategory::Carnivore
    ) && species.representative_genome.body_mass_kg() >= APEX_MIN_BODY_MASS_KG
}

/// Whether a live species is animal prey available to apex predators.
fn is_prey_species(species: &Species) -> bool {
    use crate::biosphere::evolution::SpeciesCategory as C;
    !is_apex_species(species)
        && matches!(
            species.category,
            C::Aquatic
                | C::Flying
                | C::Bird
                | C::Insect
                | C::Herbivore
                | C::Omnivore
                | C::Carnivore
                | C::Amphibious
        )
}

/// Types of population collapses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollapseType {
    Energy,
    Disease,
    Reproduction,
    Environmental,
    ToolUse,
    Intelligence,
    Cooperation,
}

/// Probability over a step of `dt_years` of an event whose probability per
/// year is `annual`: `1 − (1 − annual)^dt`, so the steps of one year compose
/// back to `annual`.
fn step_probability(annual: f64, dt_years: f64) -> f64 {
    1.0 - (1.0 - annual.clamp(0.0, 1.0)).powf(dt_years.max(0.0))
}

/// Apex predator system manager
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApexPredatorSystem {
    /// All apex predator populations
    pub predator_populations: Vec<ApexPredatorConstraints>,

    /// Global apex predator population cap
    pub global_population_cap: u64,

    /// Total ecosystem energy available to apex predators
    pub total_apex_energy_j_per_day: f64,

    /// Habitable territory the whole apex tier ranges over (km²).
    #[serde(default = "default_total_territory_km2")]
    pub total_territory_km2: f64,

    /// Steps taken, keying every draw.
    #[serde(default)]
    pub step_index: u64,
}

fn default_total_territory_km2() -> f64 {
    10_000.0
}

impl ApexPredatorSystem {
    /// Create new apex predator system
    pub fn new(total_territory_km2: f64) -> Self {
        let global_population_cap = (total_territory_km2 * APEX_PREDATOR_MAX_DENSITY) as u64;

        Self {
            predator_populations: Vec::new(),
            global_population_cap,
            total_apex_energy_j_per_day: 0.0,
            total_territory_km2,
            step_index: 0,
        }
    }

    /// Add apex predator population
    pub fn add_population(&mut self, population: ApexPredatorConstraints) -> Result<(), String> {
        // Check global population cap
        let total_current_population = self
            .predator_populations
            .iter()
            .map(|p| p.population_size)
            .sum::<u64>();

        if total_current_population + population.population_size > self.global_population_cap {
            return Err("Global apex predator population cap exceeded".to_string());
        }

        self.predator_populations.push(population);
        Ok(())
    }

    /// COORDINATED STEP: track every live apex species as a constrained
    /// population, feed it the energy the live prey biomass can supply, apply
    /// collapses and caps, and write the resulting populations back.
    pub fn step_apex_predators(
        &mut self,
        species: &mut Vec<Species>,
        dt_years: f64,
        rng: &mut RngRegistry,
    ) -> Result<(), String> {
        let apex_ids: Vec<u64> = species
            .iter()
            .filter(|sp| is_apex_species(sp) && sp.population_size > 0)
            .map(|sp| sp.species_id)
            .collect();
        self.predator_populations
            .retain(|p| p.species_id != 0 && apex_ids.contains(&p.species_id));
        let territory_each = self.total_territory_km2 / apex_ids.len().max(1) as f64;
        for sp in species
            .iter()
            .filter(|sp| apex_ids.contains(&sp.species_id))
        {
            let index = match self
                .predator_populations
                .iter()
                .position(|p| p.species_id == sp.species_id)
            {
                Some(index) => index,
                None => {
                    let mut population = ApexPredatorConstraints::from_genome(
                        &sp.representative_genome,
                        territory_each,
                    );
                    population.species_id = sp.species_id;
                    self.predator_populations.push(population);
                    self.predator_populations.len() - 1
                }
            };
            let population = &mut self.predator_populations[index];
            population.territory_size = territory_each;
            population.population_size = sp.population_size;
            population.population_density = sp.population_size as f64 / territory_each;
        }

        // Energy the live prey biomass can sustainably supply per day, per km².
        self.total_apex_energy_j_per_day = species
            .iter()
            .filter(|sp| is_prey_species(sp))
            .map(|sp| {
                sp.population_size as f64
                    * sp.representative_genome.body_mass_kg()
                    * PREY_ENERGY_J_PER_KG
                    * PREY_ANNUAL_PRODUCTION_FRACTION
                    / 365.25
            })
            .sum();
        let energy_density = self.total_apex_energy_j_per_day / self.total_territory_km2.max(1.0);

        self.update(rng, energy_density, dt_years);
        self.apply_starvation(dt_years);

        for sp in species.iter_mut() {
            if let Some(population) = self
                .predator_populations
                .iter()
                .find(|p| p.species_id == sp.species_id)
            {
                sp.population_size = population.population_size;
            } else if apex_ids.contains(&sp.species_id) {
                // Its population collapsed to zero and was removed.
                sp.population_size = 0;
            }
        }
        species.retain(|sp| sp.population_size > 0);
        Ok(())
    }

    /// Update all predator populations against the prey energy available
    /// per km² per day, over a step of `dt_years`.
    ///
    /// A population's collapse probability is its risk *per year*. A
    /// population whose annual risk passes one half is in crisis, and over
    /// the step it collapses with probability `1 − (1 − p)^dt`, so how
    /// often predators collapse does not depend on how finely time is
    /// stepped.
    pub fn update(&mut self, rng: &RngRegistry, environmental_energy_density: f64, dt_years: f64) {
        self.step_index += 1;
        let step = self.step_index;
        // Update available energy for all populations
        for population in &mut self.predator_populations {
            population.update_available_energy(environmental_energy_density);
            population.update_cooperation();
        }

        // Check for collapses
        let mut collapses_to_apply = Vec::new();

        for (i, population) in self.predator_populations.iter().enumerate() {
            let draw = ApexDraw {
                lineage: (population.species_id as u32) ^ (i as u32).rotate_left(16),
                step,
            };
            let collapse_probability = population.calculate_collapse_probability(rng, draw);

            if collapse_probability > 0.5 {
                let step_probability = step_probability(collapse_probability, dt_years);
                let mut rng_collapse = rng.stream(draw.key(22));
                if rng_collapse.gen_f64_01() < step_probability {
                    // Determine collapse type
                    let collapse_type = Self::calculate_collapse_type(population, rng, draw);
                    collapses_to_apply.push((i, collapse_type));
                }
            }
        }

        // Apply collapses
        for (i, collapse_type) in collapses_to_apply {
            self.predator_populations[i].apply_collapse(collapse_type);
        }

        // Remove extinct populations
        self.predator_populations.retain(|p| p.population_size > 0);

        // Enforce global population cap
        self.enforce_global_cap();
    }

    /// Determine type of collapse
    fn calculate_collapse_type(
        population: &ApexPredatorConstraints,
        rng: &RngRegistry,
        draw: ApexDraw,
    ) -> CollapseType {
        let mut rng_collapse = rng.stream(draw.key(23));

        // Check for catastrophic breaches first
        if population.should_collapse_tool_use(rng, draw) {
            return CollapseType::ToolUse;
        }

        if population.exceeds_intelligence_ceiling() {
            return CollapseType::Intelligence;
        }

        if population.exceeds_cooperation_limit() {
            return CollapseType::Cooperation;
        }

        // Check for other collapse types
        let roll = rng_collapse.gen_f64_01();

        if population.should_collapse_energy() && roll < 0.4 {
            CollapseType::Energy
        } else if population.should_collapse_disease(rng, draw) && roll < 0.6 {
            CollapseType::Disease
        } else if population.should_collapse_reproduction() && roll < 0.8 {
            CollapseType::Reproduction
        } else {
            CollapseType::Environmental
        }
    }

    /// Energy-limited mortality: a population whose prey cannot cover its
    /// requirement declines at a rate proportional to the shortfall, so
    /// predators without enough prey die off over months rather than
    /// persisting indefinitely.
    fn apply_starvation(&mut self, dt_years: f64) {
        for population in &mut self.predator_populations {
            let shortfall = (1.0 - population.energy_balance_ratio).clamp(0.0, 1.0);
            if shortfall == 0.0 {
                continue;
            }
            let survival = (-STARVATION_MORTALITY_PER_YEAR * shortfall * dt_years.max(0.0)).exp();
            population.population_size = (population.population_size as f64 * survival) as u64;
            population.population_density =
                population.population_size as f64 / population.territory_size.max(1e-9);
        }
        self.predator_populations.retain(|p| p.population_size > 0);
    }

    /// Enforce global population cap
    fn enforce_global_cap(&mut self) {
        let total_current = self
            .predator_populations
            .iter()
            .map(|p| p.population_size)
            .sum::<u64>();

        if total_current > self.global_population_cap {
            let excess = total_current - self.global_population_cap;
            let reduction_ratio = 1.0 - (excess as f64 / total_current as f64);

            for population in &mut self.predator_populations {
                population.population_size =
                    (population.population_size as f64 * reduction_ratio) as u64;
                population.population_density =
                    population.population_size as f64 / population.territory_size;
            }
        }
    }

    /// Get total apex predator population
    pub fn total_population(&self) -> u64 {
        self.predator_populations
            .iter()
            .map(|p| p.population_size)
            .sum()
    }

    /// Check if system is stable
    pub fn is_stable(&self) -> bool {
        self.total_population() <= self.global_population_cap
            && self.predator_populations.iter().all(|p| p.is_viable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosphere::genetics::Genome;
    use mk_core::ids::KenzIeSubclass;
    use mk_core::rng::RngRegistry;

    #[test]
    fn test_apex_predator_constraints() {
        let rng = RngRegistry::new([0u8; 32]);
        let genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);

        let constraints = ApexPredatorConstraints::from_genome(&genome, 1000.0);

        assert!(constraints.population_size > 0);
        assert!(constraints.population_density <= APEX_PREDATOR_MAX_DENSITY);
        assert!(constraints.territory_size >= APEX_PREDATOR_MIN_TERRITORY);
        assert!(constraints.tool_use_propensity <= TOOL_USE_PROHIBITION_THRESHOLD);
        assert!(constraints.intelligence_index <= APEX_PREDATOR_INTELLIGENCE_CEILING);
    }

    #[test]
    fn test_energy_collapse() {
        let rng = RngRegistry::new([0u8; 32]);
        let genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);

        let mut constraints = ApexPredatorConstraints::from_genome(&genome, 1000.0);

        // Set very low available energy
        constraints.update_available_energy(0.001); // Much lower to trigger collapse

        // Also increase fragility to ensure collapse
        constraints.fragility.energy_scarcity_fragility = 0.8;

        assert!(constraints.should_collapse_energy());
    }

    #[test]
    fn test_tool_use_prohibition() {
        let rng = RngRegistry::new([0u8; 32]);
        let genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);

        let mut constraints = ApexPredatorConstraints::from_genome(&genome, 1000.0);

        // Artificially increase tool use propensity to test prohibition
        constraints.tool_use_propensity = TOOL_USE_PROHIBITION_THRESHOLD + 0.1;

        assert!(constraints.should_collapse_tool_use(
            &rng,
            ApexDraw {
                lineage: 1,
                step: 1
            }
        ));
    }

    #[test]
    fn test_intelligence_ceiling() {
        let rng = RngRegistry::new([0u8; 32]);
        let genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);

        let mut constraints = ApexPredatorConstraints::from_genome(&genome, 1000.0);

        // Should not exceed ceiling initially
        assert!(!constraints.exceeds_intelligence_ceiling());

        // Artificially set high intelligence to test ceiling
        constraints.intelligence_index = APEX_PREDATOR_INTELLIGENCE_CEILING + 100.0;
        assert!(constraints.exceeds_intelligence_ceiling());
    }

    #[test]
    fn collapse_risk_per_year_does_not_depend_on_the_step_length() {
        for annual in [0.6, 0.9] {
            let yearly = step_probability(annual, 1.0);
            // Surviving 365 daily steps is as likely as surviving one year.
            let daily = step_probability(annual, 1.0 / 365.0);
            let survive_daily = (1.0 - daily).powi(365);
            assert!((yearly - annual).abs() < 1e-12);
            assert!(((1.0 - survive_daily) - annual).abs() < 1e-9);
        }
        assert_eq!(step_probability(0.7, 0.0), 0.0);
    }

    #[test]
    fn test_apex_predator_system() {
        let mut system = ApexPredatorSystem::new(10000.0);

        let rng = RngRegistry::new([0u8; 32]);
        let _genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);

        // Test with a fresh apex manager
        let mut manager = ApexPredatorSystem::new(10000.0);
        let genome_clean = Genome::new(2, &rng, KenzIeSubclass::Alpha);
        let mut population_clean = ApexPredatorConstraints::from_genome(&genome_clean, 1000.0);

        // Override energy requirement to reasonable level for test
        population_clean.energy_requirement_j_per_day = 1000.0; // Much lower than default

        // Update population with high energy first to ensure viability
        population_clean.update_available_energy(1000.0);

        // Should be able to add clean population
        assert!(manager.add_population(population_clean).is_ok());

        // Manager should be stable with clean population after energy update
        assert!(manager.is_stable());

        // Update system
        system.update(&rng, 100.0, 1.0);

        // Should still be stable with good energy
        assert!(system.is_stable());
    }

    fn live_species(
        id: u64,
        category: crate::biosphere::evolution::SpeciesCategory,
        size: u8,
        population: u64,
    ) -> Species {
        let rng = RngRegistry::new([3u8; 32]);
        let mut genome = Genome::new(id, &rng, KenzIeSubclass::Alpha);
        genome.structural.size_modifier = size;
        let mut species = Species::new(id, genome, population, 1.0);
        species.category = category;
        species
    }

    #[test]
    fn live_apex_species_are_tracked_and_fed_by_prey_biomass() {
        use crate::biosphere::evolution::SpeciesCategory;
        let mut rng = RngRegistry::new([1u8; 32]);
        let mut system = ApexPredatorSystem::new(1.0e6);
        let mut species = vec![
            live_species(1, SpeciesCategory::Carnivore, 8, 50),
            live_species(2, SpeciesCategory::Herbivore, 8, 2_000_000),
        ];

        system
            .step_apex_predators(&mut species, 0.1, &mut rng)
            .unwrap();

        assert_eq!(system.predator_populations.len(), 1);
        assert_eq!(system.predator_populations[0].species_id, 1);
        assert!(system.total_apex_energy_j_per_day > 0.0);
        assert!(system.predator_populations[0].available_energy_j_per_day > 0.0);
    }

    #[test]
    fn apex_predators_without_prey_starve() {
        use crate::biosphere::evolution::SpeciesCategory;
        let mut rng = RngRegistry::new([1u8; 32]);
        let mut system = ApexPredatorSystem::new(1.0e6);
        let mut species = vec![live_species(1, SpeciesCategory::Carnivore, 8, 5_000)];

        for _ in 0..20 {
            system
                .step_apex_predators(&mut species, 0.1, &mut rng)
                .unwrap();
        }

        let remaining = species.first().map(|sp| sp.population_size).unwrap_or(0);
        assert!(
            remaining < 5_000,
            "no prey must mean decline, got {remaining}"
        );
        assert_eq!(system.total_apex_energy_j_per_day, 0.0);
    }
}
