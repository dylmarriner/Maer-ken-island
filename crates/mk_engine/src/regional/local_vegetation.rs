//! Individual trees around the estate (Phase 3 Task 7).
//!
//! The regional ecology holds producer carbon per 2 km cell
//! (`biomass_kgc_m2`). The estate's 4 km patch (5 m cells, the 2 × 2 block of
//! medium cells) resolves it: trees with a stem of at least
//! [`MIN_STEM_DIAMETER_M`] within `individual_radius_m` of the estate
//! centre are individual [`TreeInstance`]s; everything else (shrubs, grass,
//! small stems, undergrowth, and all vegetation beyond the radius) is stand
//! cover per patch cell. Nothing is invented: per medium cell, trees plus
//! stands carry exactly that cell's biomass over the ground the estate does
//! not cover, so the patch and the field never disagree.
//!
//! Individual stems are placed by thinned, jittered lattice sampling (blue
//! noise, not clumped), at a density set by the cell's biomass and a
//! reverse-J stem-size distribution; their carbon follows the pantropical
//! allometry of Chave et al. (2014), `AGB = 0.0673 (ρ D² H)^0.976`. If seeding
//! would exceed the tree `cap`, the radius shrinks (halving the excess area
//! each pass) and the displaced carbon stays in the stands.
//!
//! The patch follows the field: [`LocalVegetationPatch::step`] rescales
//! each cell's pieces to the cell's current biomass (growth) and kills trees
//! at a background hazard, moving their carbon into the stands. Felling
//! debits the field by the stem's carbon.

use mk_core::biomes::BiomeType;
use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};
use mk_core::grid::{Grid2, GridSpec};
use mk_island::{DomainLevel, EstatePatchConfig, IslandDomain, LocalPatchSpec};

use super::ecology::RegionalEcologyState;
use super::estate_layout::{EstateLayout, Rect};
use crate::organisms::vegetation::PlantKind;

/// Smallest stem (m) that is an individual tree: 10 cm.
pub const MIN_STEM_DIAMETER_M: f64 = 0.10;
/// Carbon fraction of dry mass.
const CARBON_FRACTION: f64 = 0.475;
/// Wood density (g/cm³) of a temperate/boreal mix.
const WOOD_DENSITY_G_CM3: f64 = 0.55;
/// Share of a tree-biome cell's biomass in stems.
const STEM_CARBON_FRACTION: f64 = 0.75;
/// Mean stem carbon above the 10 cm minimum, as a multiple of the
/// minimum's carbon (a reverse-J size distribution).
const MEAN_EXCESS_OVER_MIN: f64 = 6.0;
/// Background tree mortality: one in 150 per year.
const TREE_LIFESPAN_YEARS: f64 = 150.0;
const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;
/// Usable wood as a share of a felled stem's dry mass (stem and large
/// branches; the rest is brash and roots left behind).
const USABLE_WOOD_FRACTION: f64 = 0.7;

#[derive(Debug, Clone, PartialEq)]
pub enum LocalVegetationError {
    /// The layout's patch does not sit on a 2 × 2 block of medium cells.
    PatchIsNotAnEstateBlock,
    NoSuchTree(u64),
}

impl std::fmt::Display for LocalVegetationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PatchIsNotAnEstateBlock => {
                write!(f, "the patch is not a 2 x 2 block of medium cells")
            }
            Self::NoSuchTree(id) => write!(f, "no tree {id}"),
        }
    }
}

impl std::error::Error for LocalVegetationError {}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeInstance {
    pub id: u64,
    pub kind: PlantKind,
    /// Domain metres.
    pub position_m: (f64, f64),
    pub height_m: f64,
    pub stem_diameter_m: f64,
    pub biomass_kgc: f64,
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandCover {
    pub biome: BiomeType,
    /// Carbon in the 5 m cell (kgC).
    pub biomass_kgc: f64,
    /// Stems of at least 10 cm per hectare the cell stands for.
    pub stem_density_per_ha: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodYield {
    /// Usable wood (kg dry mass).
    pub wood_kg: f64,
    /// The stem's carbon (kgC) debited from the cell and the ledger.
    pub carbon_kgc: f64,
}

#[derive(Debug, Clone)]
pub struct LocalVegetationPatch {
    pub trees: Vec<TreeInstance>,
    pub stands: Grid2<StandCover>,
    pub individual_radius_m: f64,
    pub cap: usize,
    patch: LocalPatchSpec,
    /// Medium cell (row, col) of the block's south-west cell.
    block: (usize, usize),
    medium_cell_m: f64,
    next_id: u64,
    seed: [u8; 32],
    centre_m: (f64, f64),
    excluded: Vec<bool>,
}

fn is_tree_biome(biome: BiomeType) -> bool {
    use BiomeType::*;
    matches!(
        biome,
        TropicalRainforest
            | TropicalDryForest
            | TemperateForest
            | BorealForest
            | Woodland
            | MontaneForest
    )
}

/// Tree height (m) from stem diameter (m): Chave et al. (2014)'s
/// height-diameter relation, `ln H = 0.893 + 0.76 ln D - 0.034 (ln D)²`
/// with D in cm (`fixtures/reference/ecology`, `chave_height_diameter`).
fn height_of(diameter_m: f64) -> f64 {
    let ln_d = (diameter_m * 100.0).max(1.0).ln();
    (0.893 + 0.76 * ln_d - 0.034 * ln_d * ln_d).exp()
}

/// Carbon (kgC) of a stem of `diameter_m` (Chave et al. 2014).
pub fn carbon_of_diameter(diameter_m: f64) -> f64 {
    let d_cm = diameter_m * 100.0;
    CARBON_FRACTION
        * 0.0673
        * (WOOD_DENSITY_G_CM3 * d_cm * d_cm * height_of(diameter_m)).powf(0.976)
}

/// The stem diameter (m) whose carbon is `carbon_kgc`, by bisection.
pub fn diameter_of_carbon(carbon_kgc: f64) -> f64 {
    let (mut lo, mut hi) = (0.0_f64, 3.0_f64);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if carbon_of_diameter(mid) < carbon_kgc {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn uniforms(seed: &[u8; 32], label: &[u8], ix: i64, iy: i64) -> [f64; 4] {
    let mut h = blake3::Hasher::new();
    h.update(b"mk-island-local-vegetation");
    h.update(seed);
    h.update(label);
    h.update(&ix.to_le_bytes());
    h.update(&iy.to_le_bytes());
    let b = h.finalize();
    let w = |k: usize| {
        let word = u64::from_le_bytes(b.as_bytes()[8 * k..8 * k + 8].try_into().expect("8 bytes"));
        (word >> 11) as f64 / (1u64 << 53) as f64
    };
    [w(0), w(1), w(2), w(3)]
}

impl LocalVegetationPatch {
    fn cell_index(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let (c, r) = (
            ((x - self.patch.origin_x_m) / self.patch.cell_size_m).floor(),
            ((y - self.patch.origin_y_m) / self.patch.cell_size_m).floor(),
        );
        (r >= 0.0 && c >= 0.0 && (r as usize) < self.patch.rows && (c as usize) < self.patch.cols)
            .then_some((r as usize, c as usize))
    }

    /// The block's medium cell under a domain point.
    fn medium_of(&self, x: f64, y: f64) -> (usize, usize) {
        let size = self.medium_cell_m;
        let (r, c) = ((y / size).floor() as usize, (x / size).floor() as usize);
        (
            r.clamp(self.block.0, self.block.0 + 1),
            c.clamp(self.block.1, self.block.1 + 1),
        )
    }

    /// Carbon (kgC) in the patch: trees plus stands.
    pub fn total_carbon_kgc(&self) -> f64 {
        self.trees
            .iter()
            .filter(|t| t.alive)
            .map(|t| t.biomass_kgc)
            .sum::<f64>()
            + self
                .stands
                .data()
                .iter()
                .map(|s| s.biomass_kgc)
                .sum::<f64>()
    }

    /// Carbon (kgC) the patch holds over the medium cell `(row, col)`.
    pub fn carbon_in_medium_cell_kgc(&self, row: usize, col: usize) -> f64 {
        let in_cell = |x: f64, y: f64| self.medium_of(x, y) == (row, col);
        let trees: f64 = self
            .trees
            .iter()
            .filter(|t| t.alive && in_cell(t.position_m.0, t.position_m.1))
            .map(|t| t.biomass_kgc)
            .sum();
        let mut stands = 0.0;
        for r in 0..self.patch.rows {
            for c in 0..self.patch.cols {
                let (x, y) = self.patch.cell_center_m(r, c);
                if in_cell(x, y) {
                    stands += self.stands.get(r, c).biomass_kgc;
                }
            }
        }
        trees + stands
    }

    /// Fell a tree: the stem leaves the patch, the cell's biomass field
    /// loses its carbon, and the ledger records it moving out of living
    /// biomass. Until the material ledger (Task 3) exists the carbon is
    /// booked into `DetritusCarbon`: it stays in the land carbon system.
    pub fn fell_tree(
        &mut self,
        tree_id: u64,
        ecology: &mut RegionalEcologyState,
        ledger: &mut Ledger,
    ) -> Result<WoodYield, LocalVegetationError> {
        let i = self
            .trees
            .iter()
            .position(|t| t.id == tree_id && t.alive)
            .ok_or(LocalVegetationError::NoSuchTree(tree_id))?;
        let tree = self.trees.remove(i);
        let (row, col) = self.medium_of(tree.position_m.0, tree.position_m.1);
        let area = self.medium_cell_m * self.medium_cell_m;
        let cell = ecology.biomass_kgc_m2.get_mut(row, col);
        *cell = (*cell - tree.biomass_kgc / area).max(0.0);
        ledger.push(FluxEntry::new(
            Reservoir::BiomassCarbon,
            Reservoir::DetritusCarbon,
            tree.biomass_kgc,
            FluxKind::Carbon,
        ));
        Ok(WoodYield {
            wood_kg: tree.biomass_kgc / CARBON_FRACTION * USABLE_WOOD_FRACTION,
            carbon_kgc: tree.biomass_kgc,
        })
    }

    /// Follow the field over `dt_seconds`: trees die at the background
    /// hazard (their carbon joins the stands), then every cell's pieces are
    /// rescaled to the cell's current biomass, so growth and decline in the
    /// ecology reach the individual stems.
    pub fn step(&mut self, ecology: &RegionalEcologyState, dt_seconds: u64) {
        if dt_seconds == 0 {
            return;
        }
        let dt_years = dt_seconds as f64 / SECONDS_PER_YEAR;
        let hazard = 1.0 - (-dt_years / TREE_LIFESPAN_YEARS).exp();
        let step_key = (self.next_id as i64).wrapping_add(dt_seconds as i64);
        let mut fallen: Vec<(f64, f64, f64)> = Vec::new();
        let seed = self.seed;
        self.trees.retain(|t| {
            let u = uniforms(&seed, b"mortality", t.id as i64, step_key)[0];
            if t.alive && u < hazard {
                fallen.push((t.position_m.0, t.position_m.1, t.biomass_kgc));
                false
            } else {
                true
            }
        });
        for (x, y, carbon) in fallen {
            if let Some((r, c)) = self.cell_index(x, y) {
                self.stands.get_mut(r, c).biomass_kgc += carbon;
            }
        }
        // Rescale each medium cell's pieces to the field.
        let area = self.medium_cell_m * self.medium_cell_m;
        for dr in 0..2 {
            for dc in 0..2 {
                let (row, col) = (self.block.0 + dr, self.block.1 + dc);
                let target =
                    ecology.biomass_kgc_m2.get(row, col) * area * self.included_share(row, col);
                let current = self.carbon_in_medium_cell_kgc(row, col);
                if current <= 0.0 || target <= 0.0 {
                    continue;
                }
                let factor = target / current;
                let (block, size) = (self.block, self.medium_cell_m);
                let medium_of = |x: f64, y: f64| {
                    let (r, c) = ((y / size).floor() as usize, (x / size).floor() as usize);
                    (r.clamp(block.0, block.0 + 1), c.clamp(block.1, block.1 + 1))
                };
                for t in &mut self.trees {
                    if medium_of(t.position_m.0, t.position_m.1) == (row, col) {
                        t.biomass_kgc *= factor;
                        t.stem_diameter_m = diameter_of_carbon(t.biomass_kgc);
                        t.height_m = height_of(t.stem_diameter_m);
                    }
                }
                for r in 0..self.patch.rows {
                    for c in 0..self.patch.cols {
                        let (x, y) = self.patch.cell_center_m(r, c);
                        if self.medium_of(x, y) == (row, col) {
                            self.stands.get_mut(r, c).biomass_kgc *= factor;
                        }
                    }
                }
            }
        }
    }

    /// Share of a medium cell's ground the vegetation covers (outside the
    /// estate's yard).
    fn included_share(&self, row: usize, col: usize) -> f64 {
        let (mut inside, mut total) = (0.0, 0.0);
        for r in 0..self.patch.rows {
            for c in 0..self.patch.cols {
                let (x, y) = self.patch.cell_center_m(r, c);
                if self.medium_of(x, y) == (row, col) {
                    total += 1.0;
                    if !self.excluded[r * self.patch.cols + c] {
                        inside += 1.0;
                    }
                }
            }
        }
        if total > 0.0 {
            inside / total
        } else {
            0.0
        }
    }
}

/// Seed the estate patch's vegetation from the ecology.
pub fn seed_local_vegetation(
    ecology: &RegionalEcologyState,
    layout: &EstateLayout,
    domain: &IslandDomain,
    config: &EstatePatchConfig,
    seed: [u8; 32],
) -> Result<LocalVegetationPatch, LocalVegetationError> {
    let medium = DomainLevel::Medium;
    let size = domain.cell_size_m(medium);
    let patch = layout.patch.clone();
    let (r0, c0) = (patch.origin_y_m / size, patch.origin_x_m / size);
    if (r0 - r0.round()).abs() > 1e-9
        || (c0 - c0.round()).abs() > 1e-9
        || (patch.width_m - 2.0 * size).abs() > 1e-9
        || (patch.height_m - 2.0 * size).abs() > 1e-9
    {
        return Err(LocalVegetationError::PatchIsNotAnEstateBlock);
    }
    let block = (r0.round() as usize, c0.round() as usize);
    let yard: Rect = layout.yard;
    let centre_m = (0.5 * (yard.x0 + yard.x1), 0.5 * (yard.y0 + yard.y1));
    let area_medium = size * size;
    let cell_m2 = patch.cell_size_m * patch.cell_size_m;
    let spec = GridSpec::new(patch.rows, patch.cols);

    let mut state = LocalVegetationPatch {
        trees: Vec::new(),
        stands: Grid2::new(
            &spec,
            StandCover {
                biome: BiomeType::DeepOcean,
                biomass_kgc: 0.0,
                stem_density_per_ha: 0.0,
            },
        ),
        individual_radius_m: config.individual_tree_radius_m,
        cap: config.tree_cap,
        block,
        medium_cell_m: size,
        next_id: 1,
        seed,
        centre_m,
        excluded: vec![false; patch.rows * patch.cols],
        patch: patch.clone(),
    };
    for r in 0..patch.rows {
        for c in 0..patch.cols {
            let (x, y) = patch.cell_center_m(r, c);
            state.excluded[r * patch.cols + c] = yard.contains(x, y);
        }
    }

    // Per medium cell: biome, target carbon, included ground.
    struct CellPlan {
        biome: BiomeType,
        carbon_total: f64,
        stem_density_kgc_m2: f64,
    }
    let plan = |row: usize, col: usize| {
        let biome = *ecology.biome_grid.get(row, col);
        let b = *ecology.biomass_kgc_m2.get(row, col);
        CellPlan {
            biome,
            carbon_total: b * area_medium,
            stem_density_kgc_m2: if is_tree_biome(biome) {
                STEM_CARBON_FRACTION * b
            } else {
                0.0
            },
        }
    };
    let plans = [
        [plan(block.0, block.1), plan(block.0, block.1 + 1)],
        [plan(block.0 + 1, block.1), plan(block.0 + 1, block.1 + 1)],
    ];
    let included_share = |row: usize, col: usize| state.included_share(row, col);
    let share = [
        [
            included_share(block.0, block.1),
            included_share(block.0, block.1 + 1),
        ],
        [
            included_share(block.0 + 1, block.1),
            included_share(block.0 + 1, block.1 + 1),
        ],
    ];

    // Individual stems, shrinking the radius until under the cap.
    let min_carbon = carbon_of_diameter(MIN_STEM_DIAMETER_M);
    let mean_tree_carbon = min_carbon * (1.0 + MEAN_EXCESS_OVER_MIN);
    let max_density = plans
        .iter()
        .flatten()
        .map(|p| p.stem_density_kgc_m2 / mean_tree_carbon)
        .fold(0.0_f64, f64::max);
    let mut radius = config.individual_tree_radius_m;
    let mut trees: Vec<TreeInstance>;
    let mut guard = 0;
    loop {
        trees = Vec::new();
        if max_density > 0.0 && radius > 0.0 {
            let spacing = (1.0 / max_density.sqrt()).clamp(2.0, 20.0);
            let n = (2.0 * radius / spacing).ceil() as i64;
            for i in 0..n {
                for j in 0..n {
                    let u = uniforms(&seed, b"stem", i, j);
                    let x = centre_m.0 - radius + (i as f64 + u[0]) * spacing;
                    let y = centre_m.1 - radius + (j as f64 + u[1]) * spacing;
                    if (x - centre_m.0).hypot(y - centre_m.1) > radius || yard.contains(x, y) {
                        continue;
                    }
                    let Some((pr, pc)) = state.cell_index(x, y) else {
                        continue;
                    };
                    let (mr, mc) = state.medium_of(x, y);
                    let cell = &plans[mr - block.0][mc - block.1];
                    let density = cell.stem_density_kgc_m2 / mean_tree_carbon;
                    if density <= 0.0 || u[2] >= density * spacing * spacing {
                        continue;
                    }
                    let _ = (pr, pc);
                    // Reverse-J size: minimum plus an exponential excess.
                    let excess = -(1.0 - u[3]).ln() * MEAN_EXCESS_OVER_MIN * min_carbon;
                    trees.push(TreeInstance {
                        id: 0,
                        kind: PlantKind::Tree,
                        position_m: (x, y),
                        height_m: 0.0,
                        stem_diameter_m: 0.0,
                        biomass_kgc: min_carbon + excess,
                        alive: true,
                    });
                }
            }
        }
        if trees.len() <= config.tree_cap || guard >= 60 {
            break;
        }
        // Halve the excess area each pass.
        let area = std::f64::consts::PI * radius * radius;
        let allowed = area * config.tree_cap as f64 / trees.len() as f64;
        radius = ((area - 0.5 * (area - allowed)) / std::f64::consts::PI).sqrt();
        guard += 1;
    }
    trees.truncate(config.tree_cap);
    state.individual_radius_m = radius;

    // Scale each cell's stems to exactly their share of the cell's stem
    // carbon, then derive size and height.
    for dr in 0..2 {
        for dc in 0..2 {
            let (row, col) = (block.0 + dr, block.1 + dc);
            // Fraction of the cell's included ground inside the radius.
            let (mut within, mut included) = (0.0, 0.0);
            for r in 0..patch.rows {
                for c in 0..patch.cols {
                    let (x, y) = patch.cell_center_m(r, c);
                    if state.medium_of(x, y) != (row, col) || state.excluded[r * patch.cols + c] {
                        continue;
                    }
                    included += 1.0;
                    if (x - centre_m.0).hypot(y - centre_m.1) <= radius {
                        within += 1.0;
                    }
                }
            }
            let cell = &plans[dr][dc];
            let want = cell.stem_density_kgc_m2
                * area_medium
                * share[dr][dc]
                * if included > 0.0 {
                    within / included
                } else {
                    0.0
                };
            let mine: Vec<usize> = (0..trees.len())
                .filter(|&k| {
                    state.medium_of(trees[k].position_m.0, trees[k].position_m.1) == (row, col)
                })
                .collect();
            let have: f64 = mine.iter().map(|&k| trees[k].biomass_kgc).sum();
            if have > 0.0 && want > 0.0 {
                // Scale only the excess over the minimum stem so every
                // stem stays at least 10 cm.
                let floor_total = min_carbon * mine.len() as f64;
                let excess_have = have - floor_total;
                let excess_want = (want - floor_total).max(0.0);
                for &k in &mine {
                    let excess = trees[k].biomass_kgc - min_carbon;
                    let scaled = if excess_have > 0.0 {
                        excess * excess_want / excess_have
                    } else {
                        0.0
                    };
                    trees[k].biomass_kgc = min_carbon + scaled;
                }
                // If even minimum stems exceed the cell's stem carbon,
                // drop the smallest until they fit.
                let mut total: f64 = mine.iter().map(|&k| trees[k].biomass_kgc).sum();
                let mut order = mine.clone();
                order.sort_by(|&a, &b| trees[a].biomass_kgc.total_cmp(&trees[b].biomass_kgc));
                for k in order {
                    if total <= want * (1.0 + 1e-9) {
                        break;
                    }
                    total -= trees[k].biomass_kgc;
                    trees[k].alive = false;
                }
            } else {
                for &k in &mine {
                    trees[k].alive = false;
                }
            }
        }
    }
    trees.retain(|t| t.alive);
    for t in &mut trees {
        t.id = state.next_id;
        state.next_id += 1;
        t.stem_diameter_m = diameter_of_carbon(t.biomass_kgc);
        t.height_m = height_of(t.stem_diameter_m);
    }
    state.trees = trees;

    // Stands carry the rest: per medium cell, (total - stems) spread over
    // the included ground, lighter where stems stand individually.
    let radius_m = state.individual_radius_m;
    for dr in 0..2 {
        for dc in 0..2 {
            let (row, col) = (block.0 + dr, block.1 + dc);
            let cell = &plans[dr][dc];
            let covered = cell.carbon_total * share[dr][dc];
            let stems: f64 = state
                .trees
                .iter()
                .filter(|t| state.medium_of(t.position_m.0, t.position_m.1) == (row, col))
                .map(|t| t.biomass_kgc)
                .sum();
            let remainder = (covered - stems).max(0.0);
            // Weight: full where stems are not individual, the non-stem
            // share where they are.
            let mut cells = Vec::new();
            let mut weight_sum = 0.0;
            for r in 0..patch.rows {
                for c in 0..patch.cols {
                    let (x, y) = patch.cell_center_m(r, c);
                    if state.medium_of(x, y) != (row, col) || state.excluded[r * patch.cols + c] {
                        continue;
                    }
                    let individual = cell.stem_density_kgc_m2 > 0.0
                        && (x - centre_m.0).hypot(y - centre_m.1) <= radius_m;
                    let w = if individual {
                        1.0 - STEM_CARBON_FRACTION
                    } else {
                        1.0
                    };
                    weight_sum += w;
                    cells.push((r, c, w));
                }
            }
            for (r, c, w) in cells {
                let carbon = if weight_sum > 0.0 {
                    remainder * w / weight_sum
                } else {
                    0.0
                };
                *state.stands.get_mut(r, c) = StandCover {
                    biome: cell.biome,
                    biomass_kgc: carbon,
                    stem_density_per_ha: if is_tree_biome(cell.biome) {
                        cell.stem_density_kgc_m2 / mean_tree_carbon * 10_000.0
                    } else {
                        0.0
                    },
                };
            }
        }
    }
    let _ = (state.centre_m, cell_m2);
    Ok(state)
}
