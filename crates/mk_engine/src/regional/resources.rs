//! The island's mineral resources as places to work (Phase 3 Task 2).
//!
//! [`IslandResources::survey`] lists every place a mineral can be taken
//! from, each with the tonnes it holds and their grade:
//!
//! - **Primary deposits**, one site each, from Phase 1's
//!   [`MineralDeposit`]s with their tonnage, grade and depth.
//! - **Placer gold**, one site downstream of each primary deposit that
//!   yields gold. The descriptive model of placer Au-PGE (Yeend 1986,
//!   USGS Bulletin 1693 model 39a) sites placers "where gradients flatten
//!   and river velocities lessen", so the gold settles at the first river
//!   cell below its source whose outgoing reach is gentler than its
//!   incoming one, or in a lake; gold that meets the sea first lies on the
//!   beach at the river's mouth (the model includes beach placers). Size
//!   and grade are lognormal about the 65-deposit grade-tonnage model
//!   (Orris & Bliss 1986; `placer_gold_grade_tonnage`). No distance-decay
//!   of grade is applied: the model gives none, and alluvial grade can be
//!   near constant for kilometres.
//! - **Titanomagnetite ironsand**, at the mouth of every river whose
//!   catchment holds arc andesite: New Zealand's west-coast North Island
//!   ironsands are shed by the andesite of western Taranaki and
//!   concentrated by currents, waves and wind (Christie et al. 2024). Each
//!   holds the mean onshore deposit of that coast
//!   (`titanomagnetite_ironsand_resources`), as concentrate.
//!
//! Nothing here regenerates: a site's tonnes only go down. Placers in
//! reality recharge slowly from upstream erosion, which the island has no
//! model of (D56).
//!
//! Work by hand is real work from [`LabourTable`]: a day with a gold pan or
//! sluice box washes the packs' volume of gravel at the site's grade. A
//! primary deposit lies at depth, and no shaft model exists yet, so hand
//! work on one is refused as [`ResourceError::Buried`]; petroleum needs a
//! drill rig nobody on the island can make (D19).

use serde::{Deserialize, Serialize};

use mk_island::{DomainLevel, IslandDomain};

use super::deposits::{normal, DepositKind, MineralDeposit};
use super::geology::Lithology;
use super::hydrology::Receiver;
use super::labour::{LabourError, LabourTable, LabourTool, TaskSpec};
use super::physical::RegionalPhysicalState;
use super::terrain::{hash01, terrain_key};
use crate::materials::catalogue::CatalogueItem;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// One standard normal deviate spans this many log10 units between the
/// 90th and 10th percentiles: 2 × 1.2816.
const P90_TO_P10_SD: f64 = 2.0 * 1.281_551_565_545;
/// Draws beyond this many standard deviations are clamped, as the primary
/// deposits' are.
const MAX_SD: f64 = 3.0;

/// What kind of place a site is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SiteKind {
    /// A primary deposit of Phase 1.
    Primary(DepositKind),
    /// Alluvial or beach placer gold below a gold-bearing deposit.
    PlacerGold,
    /// Alluvial diamonds below a kimberlite pipe.
    PlacerDiamond,
    /// Titanomagnetite beach sand at a river mouth.
    Ironsand,
}

impl SiteKind {
    /// The material the site yields first: its ore.
    pub fn ore(self) -> CatalogueItem {
        match self {
            Self::Primary(kind) => kind.yields()[0],
            Self::PlacerGold => CatalogueItem::PlacerGold,
            Self::PlacerDiamond => CatalogueItem::Diamond,
            Self::Ironsand => CatalogueItem::Ironsand,
        }
    }

    /// Unit of the site's grade.
    pub fn grade_unit(self) -> &'static str {
        match self {
            Self::Primary(kind) => {
                super::deposits::deposit_models()[DepositKind::ALL
                    .iter()
                    .position(|k| *k == kind)
                    .expect("every deposit kind has a model")]
                .grade_unit
            }
            Self::PlacerGold => "g/t Au",
            Self::PlacerDiamond => "ct/m³",
            Self::Ironsand => "% Fe in concentrate",
        }
    }
}

/// One place on the island a mineral can be taken from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSite {
    /// Index in [`IslandResources::sites`].
    pub id: u32,
    pub kind: SiteKind,
    /// Medium-grid cell.
    pub cell: (usize, usize),
    /// Depth of the top of the material (m): 0 for placers and sand.
    pub depth_m: f64,
    /// Tonnes the site held when surveyed.
    pub tonnage_t: f64,
    /// Tonnes still in place.
    pub remaining_t: f64,
    /// Grade in [`SiteKind::grade_unit`].
    pub grade: f64,
    /// The primary deposit a placer was shed from.
    pub source_deposit: Option<u32>,
}

impl ResourceSite {
    pub fn is_exhausted(&self) -> bool {
        self.remaining_t <= 0.0
    }
}

/// Why a site could not be worked.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceError {
    UnknownSite(u32),
    /// Nothing is left.
    Exhausted(u32),
    /// The material lies `depth_m` below ground and hand tools cannot
    /// reach it until shafts exist.
    Buried {
        site: u32,
        depth_m: f64,
    },
    /// Oil and gas come up only through a drill rig (D19).
    NeedsDrillRig(u32),
    /// The tool cannot work this kind of site.
    WrongTool {
        site: u32,
        tool: LabourTool,
    },
    Labour(LabourError),
    MissingReference(String),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSite(id) => write!(f, "there is no resource site {id}"),
            Self::Exhausted(id) => write!(f, "resource site {id} is worked out"),
            Self::Buried { site, depth_m } => write!(
                f,
                "resource site {site} lies {depth_m:.0} m down, beyond hand tools"
            ),
            Self::NeedsDrillRig(id) => write!(f, "resource site {id} needs a drill rig"),
            Self::WrongTool { site, tool } => {
                write!(f, "a {tool:?} cannot work resource site {site}")
            }
            Self::Labour(e) => write!(f, "{e}"),
            Self::MissingReference(key) => write!(f, "reference item {key} is missing"),
        }
    }
}

impl std::error::Error for ResourceError {}

impl From<LabourError> for ResourceError {
    fn from(e: LabourError) -> Self {
        Self::Labour(e)
    }
}

/// Size and grade of the secondary deposits, from the reference packs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecondaryModels {
    pub placer_median_t: f64,
    pub placer_tonnage_log10_sd: f64,
    pub placer_median_g_t: f64,
    pub placer_grade_log10_sd: f64,
    /// Wet sand and gravel (t/m³) the placer model converts volume with.
    pub placer_gravel_t_m3: f64,
    /// Diamond placers (Bliss 1992, model 39d): median volume (m³) and
    /// grade (ct/m³), with their log10 spreads.
    pub diamond_median_m3: f64,
    pub diamond_volume_log10_sd: f64,
    pub diamond_median_ct_m3: f64,
    pub diamond_grade_log10_sd: f64,
    /// Mean onshore ironsand deposit (t of concentrate).
    pub ironsand_mean_t: f64,
    pub ironsand_concentrate_fe_pct: f64,
}

impl SecondaryModels {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, ResourceError> {
        let item = |key: &str| {
            library
                .item(ReferenceDomain::Geology, key)
                .ok_or_else(|| ResourceError::MissingReference(key.into()))
        };
        let cell = |key: &str, row: &str, column: &str| {
            item(key)?
                .table()
                .and_then(|t| t.get(row, column))
                .ok_or_else(|| ResourceError::MissingReference(format!("{key}/{row}/{column}")))
        };
        let point = |key: &str| {
            item(key)?
                .point()
                .ok_or_else(|| ResourceError::MissingReference(key.into()))
        };
        const PLACER: &str = "placer_gold_grade_tonnage";
        const DIAMOND: &str = "diamond_placer_grade_volume";
        let spread_of = |key: &str, row: &str| -> Result<f64, ResourceError> {
            Ok((cell(key, row, "p10")?.log10() - cell(key, row, "p90")?.log10()) / P90_TO_P10_SD)
        };
        let spread = |row: &str| spread_of(PLACER, row);
        let ironsand = item("titanomagnetite_ironsand_resources")?
            .table()
            .ok_or_else(|| ResourceError::MissingReference("ironsand table".into()))?;
        let deposits: f64 = ironsand
            .column("deposits")
            .into_iter()
            .flatten()
            .map(|(_, v)| v)
            .sum();
        let magnetics_mt: f64 = ironsand
            .column("magnetics_mt")
            .into_iter()
            .flatten()
            .map(|(_, v)| v)
            .sum();
        if deposits <= 0.0 {
            return Err(ResourceError::MissingReference("ironsand deposits".into()));
        }
        Ok(Self {
            placer_median_t: cell(PLACER, "tonnage_mt", "p50")? * 1e6,
            placer_tonnage_log10_sd: spread("tonnage_mt")?,
            placer_median_g_t: cell(PLACER, "au_g_t", "p50")?,
            placer_grade_log10_sd: spread("au_g_t")?,
            placer_gravel_t_m3: point("placer_gravel_density")?,
            diamond_median_m3: cell(DIAMOND, "volume_mm3", "p50")? * 1e6,
            diamond_volume_log10_sd: spread_of(DIAMOND, "volume_mm3")?,
            diamond_median_ct_m3: cell(DIAMOND, "diamond_ct_m3", "p50")?,
            diamond_grade_log10_sd: spread_of(DIAMOND, "diamond_ct_m3")?,
            ironsand_mean_t: magnetics_mt / deposits * 1e6,
            ironsand_concentrate_fe_pct: point("titanomagnetite_concentrate_fe")?,
        })
    }

    pub fn load_default() -> Result<Self, ResourceError> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| ResourceError::MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }
}

/// What a day's work at a site took out.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteWork {
    pub task: TaskSpec,
    /// Tonnes of material moved from the site.
    pub moved_t: f64,
    /// What the work yields and how much of it (kg).
    pub product: CatalogueItem,
    pub product_kg: f64,
}

/// Every mineral resource site on the island.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandResources {
    sites: Vec<ResourceSite>,
}

/// Whether a primary deposit sheds gold into the rivers below it.
fn sheds_gold(kind: DepositKind) -> bool {
    kind.yields().contains(&CatalogueItem::GoldOre)
}

impl IslandResources {
    /// Survey `physical`'s island: primary deposits, then placer gold and
    /// ironsand from its rivers and coast. A pure function of its inputs.
    pub fn survey(
        physical: &RegionalPhysicalState,
        domain: &IslandDomain,
        seed: &[u8; 32],
        models: &SecondaryModels,
    ) -> Self {
        let geo = &physical.geophysics;
        let key = terrain_key(seed) ^ 0x7265_736f_7572_6365;
        let mut sites: Vec<ResourceSite> = geo
            .deposits
            .iter()
            .map(|d: &MineralDeposit| ResourceSite {
                id: 0,
                kind: SiteKind::Primary(d.kind),
                cell: d.cell,
                depth_m: d.depth_m,
                tonnage_t: d.tonnage_t,
                remaining_t: d.tonnage_t,
                grade: d.grade,
                source_deposit: None,
            })
            .collect();

        let channels = Channels::of(physical, domain);
        for deposit in geo.deposits.iter().filter(|d| sheds_gold(d.kind)) {
            let Some(cell) = channels.placer_trap(deposit.cell, true) else {
                continue;
            };
            let u = |n: u64| hash01(key, n, u64::from(deposit.id));
            let draw = |sd: f64, a: u64, b: u64| {
                10f64.powf(sd * normal(u(a), u(b)).clamp(-MAX_SD, MAX_SD))
            };
            let tonnage_t = models.placer_median_t * draw(models.placer_tonnage_log10_sd, 1, 2);
            sites.push(ResourceSite {
                id: 0,
                kind: SiteKind::PlacerGold,
                cell,
                depth_m: 0.0,
                tonnage_t,
                remaining_t: tonnage_t,
                grade: models.placer_median_g_t * draw(models.placer_grade_log10_sd, 3, 4),
                source_deposit: Some(deposit.id),
            });
        }

        // Model 39d excludes beach placers: diamonds stop where the river
        // slows inland, not at its mouth.
        for deposit in geo
            .deposits
            .iter()
            .filter(|d| d.kind == DepositKind::Kimberlite)
        {
            let Some(cell) = channels.placer_trap(deposit.cell, false) else {
                continue;
            };
            let u = |n: u64| hash01(key, n, u64::from(deposit.id) ^ 0x6469_616d);
            let draw = |sd: f64, a: u64, b: u64| {
                10f64.powf(sd * normal(u(a), u(b)).clamp(-MAX_SD, MAX_SD))
            };
            let tonnage_t = models.diamond_median_m3
                * draw(models.diamond_volume_log10_sd, 1, 2)
                * models.placer_gravel_t_m3;
            sites.push(ResourceSite {
                id: 0,
                kind: SiteKind::PlacerDiamond,
                cell,
                depth_m: 0.0,
                tonnage_t,
                remaining_t: tonnage_t,
                grade: models.diamond_median_ct_m3 * draw(models.diamond_grade_log10_sd, 3, 4),
                source_deposit: Some(deposit.id),
            });
        }

        for cell in channels.andesite_river_mouths(&geo.lithology) {
            sites.push(ResourceSite {
                id: 0,
                kind: SiteKind::Ironsand,
                cell,
                depth_m: 0.0,
                tonnage_t: models.ironsand_mean_t,
                remaining_t: models.ironsand_mean_t,
                grade: models.ironsand_concentrate_fe_pct,
                source_deposit: None,
            });
        }

        sites.sort_by(|a, b| {
            a.cell
                .cmp(&b.cell)
                .then(a.kind.cmp(&b.kind))
                .then(a.source_deposit.cmp(&b.source_deposit))
        });
        for (id, site) in sites.iter_mut().enumerate() {
            site.id = id as u32;
        }
        Self { sites }
    }

    pub fn sites(&self) -> &[ResourceSite] {
        &self.sites
    }

    pub fn site(&self, id: u32) -> Option<&ResourceSite> {
        self.sites.get(id as usize)
    }

    /// The sites in one medium cell.
    pub fn sites_in(&self, cell: (usize, usize)) -> impl Iterator<Item = &ResourceSite> + '_ {
        self.sites.iter().filter(move |s| s.cell == cell)
    }

    /// Take up to `tonnes` from a site; returns the tonnes actually taken,
    /// which is less once the site runs out. Sites never refill.
    pub fn remove(&mut self, id: u32, tonnes: f64) -> Result<f64, ResourceError> {
        let site = self
            .sites
            .get_mut(id as usize)
            .ok_or(ResourceError::UnknownSite(id))?;
        if site.is_exhausted() {
            return Err(ResourceError::Exhausted(id));
        }
        let taken = tonnes.max(0.0).min(site.remaining_t);
        site.remaining_t -= taken;
        Ok(taken)
    }

    /// One working day at a site with `tool`. Only placer gold can be
    /// worked by hand so far: a pan or a sluice washes the day's gravel at
    /// the site's grade (`LabourTable::pan_gravel`), and the gravel leaves
    /// the site.
    pub fn work_day(
        &mut self,
        id: u32,
        tool: LabourTool,
        coarse_gold: bool,
        labour: &LabourTable,
        models: &SecondaryModels,
    ) -> Result<SiteWork, ResourceError> {
        let site = self.site(id).ok_or(ResourceError::UnknownSite(id))?;
        if site.is_exhausted() {
            return Err(ResourceError::Exhausted(id));
        }
        match site.kind {
            SiteKind::Primary(DepositKind::Petroleum) => Err(ResourceError::NeedsDrillRig(id)),
            SiteKind::Primary(_) => Err(ResourceError::Buried {
                site: id,
                depth_m: site.depth_m,
            }),
            // Diamonds need a jig or a pan's concentrate sorted by eye; no
            // pack times either yet.
            SiteKind::Ironsand | SiteKind::PlacerDiamond => {
                Err(ResourceError::WrongTool { site: id, tool })
            }
            SiteKind::PlacerGold => {
                if !matches!(tool, LabourTool::GoldPan | LabourTool::SluiceBox) {
                    return Err(ResourceError::WrongTool { site: id, tool });
                }
                let gravel_t_m3 = models.placer_gravel_t_m3;
                let full = labour.pan_gravel(tool, site.grade * gravel_t_m3, coarse_gold)?;
                // The day's gravel, cut short if the site runs out first.
                let day_m3 = labour.gravel_washed_m3_per_day(tool)?;
                let moved_t = self.remove(id, day_m3 * gravel_t_m3)?;
                let share = if day_m3 > 0.0 {
                    moved_t / (day_m3 * gravel_t_m3)
                } else {
                    0.0
                };
                let task = TaskSpec {
                    duration_s: full.duration_s * share,
                    output: full.output * share,
                    ..full
                };
                Ok(SiteWork {
                    product: CatalogueItem::PlacerGold,
                    product_kg: task.output / 1_000.0,
                    moved_t,
                    task,
                })
            }
        }
    }
}

/// The drainage the secondary deposits follow.
struct Channels<'a> {
    physical: &'a RegionalPhysicalState,
    domain: &'a IslandDomain,
    rows: usize,
    cols: usize,
}

impl<'a> Channels<'a> {
    fn of(physical: &'a RegionalPhysicalState, domain: &'a IslandDomain) -> Self {
        let m = DomainLevel::Medium;
        Self {
            physical,
            domain,
            rows: domain.rows(m),
            cols: domain.cols(m),
        }
    }

    fn is_river(&self, (r, c): (usize, usize)) -> bool {
        self.physical.river_flows.is_river(r * self.cols + c)
    }

    fn receiver(&self, (r, c): (usize, usize)) -> Receiver {
        self.physical.flow_network().receiver_of(r, c)
    }

    fn cell_of(&self, i: usize) -> (usize, usize) {
        (i / self.cols, i % self.cols)
    }

    /// Fall per metre from `from` to `to` on the terrain.
    fn gradient(&self, from: (usize, usize), to: (usize, usize)) -> f64 {
        let elevation = &self.physical.geophysics.elevation_m;
        let step = if from.0 != to.0 && from.1 != to.1 {
            std::f64::consts::SQRT_2
        } else {
            1.0
        };
        let run = step * self.domain.cell_size_m(DomainLevel::Medium);
        (elevation.get(from.0, from.1) - elevation.get(to.0, to.1)) / run
    }

    /// Where gold shed at `source` settles: walking downstream, the first
    /// river cell in a lake or whose outgoing reach is gentler than its
    /// incoming one; the river's mouth if the sea comes first; nowhere if
    /// the water leaves the domain or no river carries it.
    fn placer_trap(&self, source: (usize, usize), beach: bool) -> Option<(usize, usize)> {
        let network = self.physical.flow_network();
        let mut here = source;
        let mut incoming: Option<f64> = None;
        // A drainage path visits each land cell at most once.
        for _ in 0..self.rows * self.cols {
            match self.receiver(here) {
                Receiver::Cell(next) => {
                    let next = self.cell_of(next as usize);
                    let outgoing = self.gradient(here, next);
                    if here != source && self.is_river(here) {
                        let still = network.lake_depth_m(here.0, here.1) > 0.0;
                        if still || incoming.is_some_and(|g| outgoing < g) {
                            return Some(here);
                        }
                    }
                    incoming = Some(outgoing);
                    here = next;
                }
                // Gold a river carries to the sea lies on the beach at its
                // mouth; gold shed straight off a coastal outcrop is a
                // residual deposit, which model 39a excludes.
                Receiver::Sea => {
                    return (beach && here != source && self.is_river(here)).then_some(here);
                }
                Receiver::Edge | Receiver::None => return None,
            }
        }
        None
    }

    /// The mouths of rivers whose catchments hold arc andesite.
    fn andesite_river_mouths(
        &self,
        lithology: &mk_core::grid::Grid2<Lithology>,
    ) -> Vec<(usize, usize)> {
        let network = self.physical.flow_network();
        let rocks = lithology.data();
        let mut drains_andesite = vec![false; self.rows * self.cols];
        let mut mouths = Vec::new();
        for i in network.upstream_to_downstream() {
            if rocks[i] == Lithology::ArcAndesite {
                drains_andesite[i] = true;
            }
            let cell = self.cell_of(i);
            match self.receiver(cell) {
                Receiver::Cell(next) => {
                    if drains_andesite[i] {
                        drains_andesite[next as usize] = true;
                    }
                }
                Receiver::Sea => {
                    if drains_andesite[i] && self.is_river(cell) {
                        mouths.push(cell);
                    }
                }
                Receiver::Edge | Receiver::None => {}
            }
        }
        mouths.sort_unstable();
        mouths
    }
}
