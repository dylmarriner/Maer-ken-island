use crate::agents::GridPosition;
use crate::biosphere::{is_fauna_category, Sex, Species, SpeciesCategory};
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

/// Bound on the total number of tracked beings across all species. Keeps
/// reproduction (see [`OrganismSystem::step`]) from growing this system
/// past what a 128-founder seed plus deterministic births can sanely track.
const MAX_TOTAL_BEINGS: usize = 256;
/// Per-species cap on simultaneously alive beings, independent of the
/// global bound above.
const MAX_PER_SPECIES: usize = 8;
/// Breeding rate (per year) of a species with an alive male/female pair.
const REPRODUCTION_RATE_PER_YEAR: f64 = 1.0;
/// Kleiber-type allometric lifespan for animals: `years = A · mass^0.2`.
const ALLOMETRIC_LIFESPAN_YEARS: f64 = 11.8;
const ALLOMETRIC_LIFESPAN_EXPONENT: f64 = 0.2;
/// Typical lifespans (years) of rooted categories.
const TREE_LIFESPAN_YEARS: f64 = 200.0;
const FLOWER_LIFESPAN_YEARS: f64 = 1.0;
const FUNGOID_LIFESPAN_YEARS: f64 = 5.0;
/// Energy a flyer burns per day aloft over open water (fraction of full).
const FLIGHT_ENERGY_COST_PER_DAY: f64 = 0.2;
/// Energy a being regains per day in habitat it can feed in.
const FORAGING_RECOVERY_PER_DAY: f64 = 0.5;
/// Daily travel distance of walking and amphibious animals (km/day),
/// `DAY_RANGE_KM_PER_KG_QUARTER · mass^0.25`: day range scales with the
/// quarter power of body mass (Carbone et al. 2005), about 1 km/day at 1 kg
/// and 5.6 km/day at a tonne.
const DAY_RANGE_KM_PER_KG_QUARTER: f64 = 1.0;
/// Sustained daily travel of flyers and swimmers (km/day).
const FLYING_KM_PER_DAY: f64 = 50.0;
const SWIMMING_KM_PER_DAY: f64 = 30.0;
/// Keyed-RNG salt for the cell-crossing roll.
const CELL_CROSSING_SALT: u32 = 45_020;

/// A bounded set of inspectable fauna/flora instances derived from live
/// species. The biodiversity catalogue remains population-level; these are
/// the engine-owned beings required by the 3D ground view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismBeing {
    pub id: u64,
    pub species_id: u64,
    pub species_name: String,
    pub category: SpeciesCategory,
    pub position: GridPosition,
    pub lifecycle: LifecycleState,
    pub movement: MovementClass,
    pub energy: f32,
    /// Real biological sex for fauna categories (see
    /// [`crate::biosphere::is_fauna_category`]); `None` for the stationary
    /// plant/fungal categories (`Tree`/`Flower`/`Fungoid`), which reproduce
    /// asexually rather than via sexual pairing.
    pub sex: Option<Sex>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifecycleState {
    Alive,
    Dead,
}

/// How a being can move, from its species' body plan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MovementClass {
    /// Swims; confined to ocean cells.
    Aquatic,
    /// Moves between land and water.
    Amphibious,
    /// Flies over any cell.
    Flying,
    /// Walks; confined to land cells.
    Terrestrial,
    /// Rooted in place (trees, flowering plants, fungi); never moves.
    Sessile,
}

impl MovementClass {
    /// Whether a being of this class can occupy a cell that is (or is not)
    /// ocean.
    fn can_occupy(self, is_ocean: bool) -> bool {
        match self {
            MovementClass::Aquatic => is_ocean,
            MovementClass::Terrestrial | MovementClass::Sessile => !is_ocean,
            MovementClass::Amphibious | MovementClass::Flying => true,
        }
    }
}

/// The physical world a being moves through: which cells are ocean and how
/// productive each land cell is.
pub struct OrganismTerrain<'a> {
    /// Surface elevation (m); negative is ocean.
    pub elevation: &'a mk_core::grid::Grid2<f64>,
    /// Terrestrial NPP per cell (kgC m⁻² yr⁻¹), `row * nlon + col`; empty
    /// before the first biosphere step.
    pub npp_kgc_m2_yr: &'a [f64],
}

impl OrganismTerrain<'_> {
    fn nlat(&self) -> usize {
        self.elevation.nlat()
    }

    fn nlon(&self) -> usize {
        self.elevation.nlon()
    }

    fn is_ocean(&self, row: usize, col: usize) -> bool {
        *self.elevation.get(row, col) < 0.0
    }

    /// How attractive a cell is to a being of `movement` class: on land,
    /// its primary productivity (food); at sea, shallow shelf water, where
    /// light reaches the bottom and marine life concentrates.
    fn suitability(&self, movement: MovementClass, row: usize, col: usize) -> f64 {
        let elevation = *self.elevation.get(row, col);
        if elevation < 0.0 {
            match movement {
                MovementClass::Aquatic | MovementClass::Amphibious => {
                    1.0 / (1.0 + -elevation / 200.0)
                }
                _ => 0.0,
            }
        } else {
            self.npp_kgc_m2_yr
                .get(row * self.nlon() + col)
                .copied()
                .unwrap_or(0.0)
        }
    }

    /// Every cell a being of `movement` class can occupy.
    fn habitable_cells(&self, movement: MovementClass) -> Vec<(usize, usize)> {
        (0..self.nlat())
            .flat_map(|row| (0..self.nlon()).map(move |col| (row, col)))
            .filter(|&(row, col)| movement.can_occupy(self.is_ocean(row, col)))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrganismSystem {
    pub beings: Vec<OrganismBeing>,
    pub next_id: u64,
}

impl OrganismSystem {
    pub fn new() -> Self {
        Self {
            beings: Vec::new(),
            next_id: 1,
        }
    }

    /// Create inspectable beings for each of the first bounded species.
    /// Species population remains authoritative for aggregate ecology. Fauna
    /// categories (see [`is_fauna_category`]) get one male + one female
    /// founding pair with real, RNG-derived sex (a mate must already exist
    /// for [`OrganismSystem::reproduce`] to ever trigger); the stationary
    /// plant/fungal categories get a single asexual representative, since
    /// they reproduce via spores/seeds/division, not pairing.
    pub fn seed_from_species(&mut self, species: &[Species], terrain: &OrganismTerrain<'_>) {
        if !self.beings.is_empty() {
            return;
        }
        for (index, species) in species.iter().take(64).enumerate() {
            // Each species starts on a cell of its own medium (walkers and
            // plants on land, swimmers at sea), chosen deterministically.
            let movement = movement_class(&species.category);
            let cells = terrain.habitable_cells(movement);
            let (row, col) = if cells.is_empty() {
                (0, 0)
            } else {
                cells[(index * 37 + species.species_id as usize * 101) % cells.len()]
            };
            let (row, col) = (row as i32, col as i32);

            let sexes: &[Option<Sex>] = if is_fauna_category(&species.category) {
                &[Some(Sex::Male), Some(Sex::Female)]
            } else {
                &[None]
            };
            for sex in sexes {
                self.beings.push(OrganismBeing {
                    id: self.next_id,
                    species_id: species.species_id,
                    species_name: species.species_name.clone(),
                    category: species.category.clone(),
                    position: GridPosition::new(row, col),
                    lifecycle: LifecycleState::Alive,
                    movement,
                    energy: 1.0,
                    sex: *sex,
                });
                self.next_id += 1;
            }
        }
    }

    /// Age, feed and move every living being, then roll reproduction.
    ///
    /// Each being faces its expected lifespan as a constant hazard over
    /// `dt_years` and dies with its species if that species is gone. Flyers
    /// over open water tire; any other being in its own habitat recovers
    /// energy, and an exhausted being dies. The dead are removed.
    ///
    /// A mobile being then considers staying put or moving to one of its four
    /// neighbouring cells (longitude wraps) that its movement class can
    /// occupy, and takes the most suitable — the most productive land, or
    /// the shallowest sea — with a small keyed-random preference so equally
    /// good cells are chosen between rather than always in the same order.
    /// Sessile beings never move.
    pub fn step(
        &mut self,
        tick: u64,
        dt_years: f64,
        terrain: &OrganismTerrain<'_>,
        species: &[Species],
        rng: &RngRegistry,
    ) {
        let topology = crate::topology::GridTopology::planetary(
            &mk_core::grid::GridSpec::new(terrain.nlat(), terrain.nlon()),
            mk_core::grid::CANON_PLANET_RADIUS_M,
        );
        self.step_on(tick, dt_years, terrain, &topology, species, rng);
    }

    /// [`step`](Self::step) on any [`GridTopology`](crate::topology::GridTopology):
    /// on the island nothing wraps east/west and a being at the edge has
    /// no neighbour beyond it.
    pub fn step_on(
        &mut self,
        tick: u64,
        dt_years: f64,
        terrain: &OrganismTerrain<'_>,
        topology: &crate::topology::GridTopology,
        species: &[Species],
        rng: &RngRegistry,
    ) {
        let (nlat, nlon) = (terrain.nlat() as i32, terrain.nlon() as i32);
        if nlat == 0 || nlon == 0 {
            return;
        }
        let dt = dt_years.max(0.0);
        let dt_days = dt * 365.25;
        for being in &mut self.beings {
            if being.lifecycle != LifecycleState::Alive {
                continue;
            }
            let Some(lifespan) = species
                .iter()
                .find(|sp| sp.species_id == being.species_id)
                .map(expected_lifespan_years)
            else {
                being.lifecycle = LifecycleState::Dead;
                continue;
            };
            let death_probability = 1.0 - (-dt / lifespan.max(0.1)).exp();
            let death_roll = rng.gen_f64_01(RngKey::new(
                SubsystemId::Biosphere,
                45_011,
                being.id as u32,
                tick,
            ));
            if death_roll < death_probability {
                being.lifecycle = LifecycleState::Dead;
                continue;
            }
            let row = being.position.row.clamp(0, nlat - 1) as usize;
            let col = topology.resolve_col(being.position.col);
            let over_water = terrain.is_ocean(row, col);
            let energy = if being.movement == MovementClass::Flying && over_water {
                f64::from(being.energy) - FLIGHT_ENERGY_COST_PER_DAY * dt_days
            } else {
                f64::from(being.energy) + FORAGING_RECOVERY_PER_DAY * dt_days
            };
            being.energy = energy.clamp(0.0, 1.0) as f32;
            if being.energy <= 0.0 {
                being.lifecycle = LifecycleState::Dead;
            }
        }
        self.beings
            .retain(|being| being.lifecycle == LifecycleState::Alive);

        let body_mass: std::collections::HashMap<u64, f64> = species
            .iter()
            .map(|sp| (sp.species_id, sp.representative_genome.body_mass_kg()))
            .collect();
        for being in &mut self.beings {
            if being.lifecycle != LifecycleState::Alive || being.movement == MovementClass::Sessile
            {
                continue;
            }
            // A step covers only part of a cell: the chance of reaching a
            // neighbour is the distance travelled over the cell's width.
            let mass = body_mass.get(&being.species_id).copied().unwrap_or(1.0);
            let crossing = cell_crossing_probability(
                being.movement,
                mass,
                being.position.row.clamp(0, nlat - 1) as usize,
                dt_days,
                topology,
            );
            let roll = rng.gen_f64_01(RngKey::new(
                SubsystemId::Biosphere,
                CELL_CROSSING_SALT,
                being.id as u32,
                tick,
            ));
            if roll >= crossing {
                continue;
            }
            let here = (
                being.position.row.clamp(0, nlat - 1),
                topology.resolve_col(being.position.col) as i32,
            );
            let candidates = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .filter_map(|(dr, dc)| {
                    topology
                        .neighbour(here.0 as usize, here.1 as usize, dr, dc)
                        .map(|(row, col)| (row as i32, col as i32))
                })
                .filter(|&(row, col)| {
                    being
                        .movement
                        .can_occupy(terrain.is_ocean(row as usize, col as usize))
                });
            let mut best = here;
            let mut best_score = f64::NEG_INFINITY;
            for (slot, (row, col)) in candidates.enumerate() {
                let key = RngKey::new(
                    SubsystemId::Biosphere,
                    45_004 + slot as u32,
                    being.id as u32,
                    tick,
                );
                let score = terrain.suitability(being.movement, row as usize, col as usize)
                    + 0.05 * rng.gen_f64_01(key);
                if score > best_score {
                    best_score = score;
                    best = (row, col);
                }
            }
            being.position = GridPosition::new(best.0, best.1);
        }

        self.reproduce(tick, dt, rng);
    }

    /// Deterministically spawns offspring for fauna species with an alive
    /// male and an alive female present, under both the per-species and
    /// global population caps.
    fn reproduce(&mut self, tick: u64, dt_years: f64, rng: &RngRegistry) {
        if self.beings.len() >= MAX_TOTAL_BEINGS {
            return;
        }
        let birth_probability = 1.0 - (-REPRODUCTION_RATE_PER_YEAR * dt_years).exp();

        use std::collections::HashMap;
        /// One species' living tracked beings, as reproduction sees them.
        struct SpeciesTally {
            count: usize,
            has_male: bool,
            has_female: bool,
            name: String,
            category: SpeciesCategory,
            movement: MovementClass,
            /// Where offspring are born: the first living female's cell.
            birthplace: GridPosition,
        }
        let mut per_species: HashMap<u64, SpeciesTally> = HashMap::new();
        for being in &self.beings {
            if being.lifecycle != LifecycleState::Alive {
                continue;
            }
            let tally = per_species.entry(being.species_id).or_insert(SpeciesTally {
                count: 0,
                has_male: false,
                has_female: false,
                name: being.species_name.clone(),
                category: being.category.clone(),
                movement: being.movement,
                birthplace: being.position,
            });
            tally.count += 1;
            match being.sex {
                Some(Sex::Male) => tally.has_male = true,
                Some(Sex::Female) => {
                    if !tally.has_female {
                        tally.birthplace = being.position;
                    }
                    tally.has_female = true;
                }
                None => {}
            }
        }

        // Iterate in species_id order, not HashMap order: HashMap iteration
        // order is randomized per-process, and the `break` below on hitting
        // MAX_TOTAL_BEINGS means iteration order determines *which* species
        // get to reproduce near the cap — a real state divergence between
        // otherwise-identical runs, not a cosmetic ordering difference.
        let mut ordered_species: Vec<_> = per_species.iter().collect();
        ordered_species.sort_by_key(|(species_id, _)| **species_id);

        let mut births = Vec::new();
        for (species_id, tally) in ordered_species {
            if tally.count >= MAX_PER_SPECIES || !tally.has_male || !tally.has_female {
                continue;
            }
            if self.beings.len() + births.len() >= MAX_TOTAL_BEINGS {
                break;
            }
            let roll_key = RngKey::new(SubsystemId::Biosphere, 45_002, *species_id as u32, tick);
            if rng.gen_f64_01(roll_key) >= birth_probability {
                continue;
            }
            let sex_key = RngKey::new(SubsystemId::Biosphere, 45_003, *species_id as u32, tick);
            let sex = if rng.gen_f64_01(sex_key) < 0.5 {
                Sex::Male
            } else {
                Sex::Female
            };
            births.push(OrganismBeing {
                id: 0,
                species_id: *species_id,
                species_name: tally.name.clone(),
                category: tally.category.clone(),
                position: tally.birthplace,
                lifecycle: LifecycleState::Alive,
                movement: tally.movement,
                energy: 1.0,
                sex: Some(sex),
            });
        }

        for mut birth in births {
            birth.id = self.next_id;
            self.next_id += 1;
            self.beings.push(birth);
        }
    }
}

/// Daily travel distance (km) of a being of `movement` class and body `mass_kg`.
fn travel_km_per_day(movement: MovementClass, mass_kg: f64) -> f64 {
    match movement {
        MovementClass::Terrestrial | MovementClass::Amphibious => {
            DAY_RANGE_KM_PER_KG_QUARTER * mass_kg.max(0.001).powf(0.25)
        }
        MovementClass::Flying => FLYING_KM_PER_DAY,
        MovementClass::Aquatic => SWIMMING_KM_PER_DAY,
        MovementClass::Sessile => 0.0,
    }
}

/// Chance that a being reaches a neighbouring cell over `dt_days`: its
/// travel over the width of a cell in `row`, capped at 1.
fn cell_crossing_probability(
    movement: MovementClass,
    mass_kg: f64,
    row: usize,
    dt_days: f64,
    topology: &crate::topology::GridTopology,
) -> f64 {
    let width_km = topology.cell_width_m(row) / 1000.0;
    if width_km <= 0.0 {
        return 1.0;
    }
    (travel_km_per_day(movement, mass_kg) * dt_days.max(0.0) / width_km).clamp(0.0, 1.0)
}

/// Expected lifespan (years): Kleiber allometry on body mass for animals,
/// typical values for rooted categories.
fn expected_lifespan_years(species: &Species) -> f64 {
    match species.category {
        SpeciesCategory::Tree => TREE_LIFESPAN_YEARS,
        SpeciesCategory::Flower => FLOWER_LIFESPAN_YEARS,
        SpeciesCategory::Fungoid => FUNGOID_LIFESPAN_YEARS,
        _ => {
            ALLOMETRIC_LIFESPAN_YEARS
                * species
                    .representative_genome
                    .body_mass_kg()
                    .max(0.001)
                    .powf(ALLOMETRIC_LIFESPAN_EXPONENT)
        }
    }
}

fn movement_class(category: &SpeciesCategory) -> MovementClass {
    match category {
        SpeciesCategory::Aquatic => MovementClass::Aquatic,
        SpeciesCategory::Amphibious => MovementClass::Amphibious,
        SpeciesCategory::Flying | SpeciesCategory::Bird | SpeciesCategory::Bee => {
            MovementClass::Flying
        }
        SpeciesCategory::Tree | SpeciesCategory::Flower | SpeciesCategory::Fungoid => {
            MovementClass::Sessile
        }
        SpeciesCategory::Insect
        | SpeciesCategory::Herbivore
        | SpeciesCategory::Omnivore
        | SpeciesCategory::Carnivore => MovementClass::Terrestrial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The planetary topology of a test grid.
    fn topo(spec: &mk_core::grid::GridSpec) -> crate::topology::GridTopology {
        crate::topology::GridTopology::planetary(spec, mk_core::grid::CANON_PLANET_RADIUS_M)
    }
    use crate::biosphere::genetics::Genome;
    use mk_core::rng::RngRegistry;

    const DAY_YEARS: f64 = 1.0 / 365.25;

    /// A 32x64 world: the southern half (rows 0..16) ocean, the northern
    /// half land, with land productivity rising northward.
    fn half_ocean() -> (mk_core::grid::Grid2<f64>, Vec<f64>) {
        let spec = mk_core::grid::GridSpec::new(32, 64);
        let mut elevation = mk_core::grid::Grid2::new(&spec, 100.0);
        let mut npp = vec![0.0; 32 * 64];
        for row in 0..32 {
            for col in 0..64 {
                if row < 16 {
                    *elevation.get_mut(row, col) = -1000.0;
                } else {
                    npp[row * 64 + col] = row as f64 / 32.0;
                }
            }
        }
        (elevation, npp)
    }

    fn species(id: u64, category: SpeciesCategory, rng: &RngRegistry) -> Species {
        let mut s = Species::new(
            id,
            Genome::new(id, rng, mk_core::ids::KenzIeSubclass::Alpha),
            100,
            0.5,
        );
        s.category = category;
        s
    }

    #[test]
    fn beings_start_and_stay_in_their_own_medium() {
        let rng = RngRegistry::new([7u8; 32]);
        let (elevation, npp) = half_ocean();
        let terrain = OrganismTerrain {
            elevation: &elevation,
            npp_kgc_m2_yr: &npp,
        };
        let mut system = OrganismSystem::new();
        let living = [
            species(1, SpeciesCategory::Aquatic, &rng),
            species(2, SpeciesCategory::Herbivore, &rng),
        ];
        system.seed_from_species(&living, &terrain);
        for tick in 0..200u64 {
            system.step(tick, DAY_YEARS, &terrain, &living, &rng);
            for being in &system.beings {
                let ocean = being.position.row < 16;
                match being.movement {
                    MovementClass::Aquatic => assert!(ocean, "a swimmer walked onto land"),
                    MovementClass::Terrestrial => assert!(!ocean, "a walker swam out to sea"),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn plants_never_move() {
        let rng = RngRegistry::new([9u8; 32]);
        let (elevation, npp) = half_ocean();
        let terrain = OrganismTerrain {
            elevation: &elevation,
            npp_kgc_m2_yr: &npp,
        };
        let mut system = OrganismSystem::new();
        let living = [species(3, SpeciesCategory::Tree, &rng)];
        system.seed_from_species(&living, &terrain);
        let before = system.beings[0].position;
        for tick in 0..50u64 {
            system.step(tick, DAY_YEARS, &terrain, &living, &rng);
        }
        assert_eq!(system.beings[0].movement, MovementClass::Sessile);
        assert_eq!(system.beings[0].position, before);
    }

    #[test]
    fn grazers_move_toward_productive_land() {
        let rng = RngRegistry::new([5u8; 32]);
        let (elevation, npp) = half_ocean();
        let terrain = OrganismTerrain {
            elevation: &elevation,
            npp_kgc_m2_yr: &npp,
        };
        let mut system = OrganismSystem::new();
        // A one-tonne grazer ranges ~5.6 km a day, about 2,000 km a year:
        // a ~1,900 km cell each yearly step.
        let mut grazer_species = species(4, SpeciesCategory::Herbivore, &rng);
        grazer_species
            .representative_genome
            .structural
            .size_modifier = 11;
        let living = [grazer_species];
        system.seed_from_species(&living, &terrain);
        for being in &mut system.beings {
            being.position = GridPosition::new(16, 10);
        }
        for tick in 0..12u64 {
            system.step(tick, 1.0, &terrain, &living, &rng);
        }
        // Founders may die of age over the dozen years; the herd, offspring
        // included (born at their mother's cell), has climbed the gradient.
        assert!(!system.beings.is_empty(), "the herd died out");
        for being in &system.beings {
            assert!(
                being.position.row > 20,
                "a grazer on a productivity gradient should climb it: {:?}",
                being.position
            );
        }
        let highest = system.beings.iter().map(|b| b.position.row).max().unwrap();
        assert!(
            highest > 25,
            "the herd reaches the productive north: row {highest}"
        );
    }

    #[test]
    fn beings_cross_a_continental_cell_at_their_own_pace() {
        let grid = mk_core::grid::GridSpec::new(32, 64);
        let day = 1.0;
        let mouse =
            cell_crossing_probability(MovementClass::Terrestrial, 0.02, 16, day, &topo(&grid));
        let bison =
            cell_crossing_probability(MovementClass::Terrestrial, 1000.0, 16, day, &topo(&grid));
        let bird = cell_crossing_probability(MovementClass::Flying, 0.02, 16, day, &topo(&grid));
        // A day's travel covers a small share of a ~1,900 km cell.
        assert!(
            mouse < bison && bison < bird && bird < 0.05,
            "{mouse} {bison} {bird}"
        );
        // Rooted beings never cross; long steps saturate at certainty.
        assert_eq!(
            cell_crossing_probability(MovementClass::Sessile, 10.0, 16, 365.0, &topo(&grid)),
            0.0
        );
        assert_eq!(
            cell_crossing_probability(MovementClass::Flying, 1.0, 16, 365.0, &topo(&grid)),
            1.0
        );
    }

    #[test]
    fn fauna_species_get_real_sex_flora_do_not() {
        let rng = RngRegistry::new([11u8; 32]);
        let mut fauna = Species::new(
            1,
            Genome::new(1, &rng, mk_core::ids::KenzIeSubclass::Alpha),
            100,
            0.5,
        );
        fauna.category = SpeciesCategory::Herbivore;
        let mut flora = Species::new(
            2,
            Genome::new(2, &rng, mk_core::ids::KenzIeSubclass::Alpha),
            100,
            0.5,
        );
        flora.category = SpeciesCategory::Tree;

        let (elevation, npp) = half_ocean();
        let terrain = OrganismTerrain {
            elevation: &elevation,
            npp_kgc_m2_yr: &npp,
        };
        let mut system = OrganismSystem::new();
        system.seed_from_species(&[fauna, flora], &terrain);

        let fauna_being = system.beings.iter().find(|b| b.species_id == 1).unwrap();
        let flora_being = system.beings.iter().find(|b| b.species_id == 2).unwrap();
        assert!(fauna_being.sex.is_some());
        assert!(flora_being.sex.is_none());
    }

    #[test]
    fn reproduction_never_exceeds_per_species_cap() {
        let rng = RngRegistry::new([21u8; 32]);
        let mut species = Species::new(
            1,
            Genome::new(1, &rng, mk_core::ids::KenzIeSubclass::Alpha),
            100,
            0.5,
        );
        species.category = SpeciesCategory::Herbivore;

        let (elevation, npp) = half_ocean();
        let terrain = OrganismTerrain {
            elevation: &elevation,
            npp_kgc_m2_yr: &npp,
        };
        let mut system = OrganismSystem::new();
        let living = [species];
        system.seed_from_species(&living, &terrain);
        for tick in 0..500u64 {
            system.step(tick, 0.1, &terrain, &living, &rng);
        }

        let alive = system
            .beings
            .iter()
            .filter(|b| b.lifecycle == LifecycleState::Alive && b.species_id == 1)
            .count();
        assert!(alive <= MAX_PER_SPECIES);
    }

    #[test]
    fn beings_age_out_and_die_with_their_species() {
        let rng = RngRegistry::new([9u8; 32]);
        let mut mouse = species(3, SpeciesCategory::Herbivore, &rng);
        mouse.representative_genome.structural.size_modifier = 0; // 10 kg
        let spec = mk_core::grid::GridSpec::new(8, 8);
        let land = mk_core::grid::Grid2::new(&spec, 100.0);
        let npp = vec![0.5; 64];
        let terrain = OrganismTerrain {
            elevation: &land,
            npp_kgc_m2_yr: &npp,
        };

        let mut extinct = OrganismSystem::new();
        extinct.seed_from_species(std::slice::from_ref(&mouse), &terrain);
        extinct.step(1, DAY_YEARS, &terrain, &[], &rng);
        assert!(
            extinct.beings.is_empty(),
            "an extinct species leaves no living individuals"
        );

        let mut aging = OrganismSystem::new();
        aging.seed_from_species(std::slice::from_ref(&mouse), &terrain);
        let founders: Vec<u64> = aging.beings.iter().map(|b| b.id).collect();
        for tick in 0..400u64 {
            aging.step(tick, 1.0, &terrain, std::slice::from_ref(&mouse), &rng);
        }
        assert!(
            aging.beings.iter().all(|b| !founders.contains(&b.id)),
            "no founder outlives four centuries"
        );
    }

    #[test]
    fn flyers_stranded_over_open_water_tire_and_die() {
        let rng = RngRegistry::new([13u8; 32]);
        let bird = species(5, SpeciesCategory::Bird, &rng);
        let spec = mk_core::grid::GridSpec::new(4, 4);
        let sea = mk_core::grid::Grid2::new(&spec, -1000.0);
        let npp = vec![0.0; 16];
        let terrain = OrganismTerrain {
            elevation: &sea,
            npp_kgc_m2_yr: &npp,
        };
        let mut system = OrganismSystem::new();
        system.seed_from_species(std::slice::from_ref(&bird), &terrain);
        let founders: Vec<u64> = system.beings.iter().map(|b| b.id).collect();
        for tick in 0..10u64 {
            system.step(tick, DAY_YEARS, &terrain, std::slice::from_ref(&bird), &rng);
        }
        assert!(system.beings.iter().all(|b| !founders.contains(&b.id)));
    }
}
