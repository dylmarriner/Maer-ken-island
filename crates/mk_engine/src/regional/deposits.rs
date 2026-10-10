//! Primary mineral deposits (Phase 1 Task 5).
//!
//! Each deposit kind forms only in its host rocks and setting. Its number
//! is a density per 10,000 km² of permissive ground (USGS three-part
//! assessments express undiscovered deposits this way; Singer & Menzie
//! 2010, *Quantitative Mineral Resource Assessments*), its tonnage and
//! grade are lognormal about medians from USGS grade–tonnage models (Cox &
//! Singer 1986, Bulletin 1693; `fixtures/reference/geology`
//! `grade_tonnage_medians`) with log10 spreads of 0.6–0.9 in tonnage.
//! Kinds without a published model in the reference pack are marked as
//! order-of-magnitude in their `source`. Placement draws are hashes of the
//! world key and the cell, so the list is a pure function of its inputs.
//!
//! Secondary deposits (placer gold, ironsand, beach heavy minerals) need
//! rivers and coasts and are added in Phase 3.

use crate::materials::catalogue::CatalogueItem;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};
use serde::{Deserialize, Serialize};

use super::geology::Lithology;
use super::terrain::{hash01, GeologicalSetting, Volcano};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DepositKind {
    OrogenicGold,
    EpithermalGoldSilver,
    PorphyryCopper,
    VolcanogenicMassiveSulfide,
    SedimentHostedLeadZinc,
    TinTungsten,
    Pegmatite,
    Skarn,
    Ultramafic,
    MetamorphicGems,
    BandedIronFormation,
    Kimberlite,
    VolcanicSilicaGems,
    VolcanicSulfur,
    Coal,
    Petroleum,
    Evaporite,
    Phosphate,
    Uranium,
    BuildingStone,
}

impl DepositKind {
    pub const ALL: [DepositKind; 20] = [
        Self::OrogenicGold,
        Self::EpithermalGoldSilver,
        Self::PorphyryCopper,
        Self::VolcanogenicMassiveSulfide,
        Self::SedimentHostedLeadZinc,
        Self::TinTungsten,
        Self::Pegmatite,
        Self::Skarn,
        Self::Ultramafic,
        Self::MetamorphicGems,
        Self::BandedIronFormation,
        Self::Kimberlite,
        Self::VolcanicSilicaGems,
        Self::VolcanicSulfur,
        Self::Coal,
        Self::Petroleum,
        Self::Evaporite,
        Self::Phosphate,
        Self::Uranium,
        Self::BuildingStone,
    ];

    /// The catalogue materials the deposit yields, its ore first.
    pub fn yields(self) -> &'static [CatalogueItem] {
        use CatalogueItem::*;
        use DepositKind::*;
        match self {
            OrogenicGold => &[GoldOre, QuartzCrystal],
            EpithermalGoldSilver => &[GoldOre, SilverOre, Cinnabar, QuartzCrystal, Amethyst],
            PorphyryCopper => &[CopperOre, Molybdenite, GoldOre],
            VolcanogenicMassiveSulfide => &[CopperOre, ZincOre, LeadOre, SilverOre, Pyrite],
            SedimentHostedLeadZinc => &[LeadOre, ZincOre, SilverOre],
            TinTungsten => &[TinOre, TungstenOre],
            // Beryl's gem variety is the emerald; the rest are the
            // pegmatite's own minerals.
            Pegmatite => &[
                QuartzCrystal,
                Feldspar,
                Mica,
                Emerald,
                Topaz,
                Tourmaline,
                LithiumOre,
            ],
            // An iron skarn's ore is magnetite.
            Skarn => &[MagnetiteOre, CopperOre, TungstenOre, Garnet],
            Ultramafic => &[
                Chromite,
                NickelOre,
                PlatinumGroupConcentrate,
                Nephrite,
                Serpentine,
            ],
            MetamorphicGems => &[Garnet, Ruby, Sapphire, Kyanite, Graphite],
            // Banded iron formations carry both hematite and magnetite.
            BandedIronFormation => &[IronOre, MagnetiteOre],
            Kimberlite => &[Diamond],
            VolcanicSilicaGems => &[Obsidian, Opal, Agate, Chalcedony, Amethyst, Pumice, Zeolite],
            VolcanicSulfur => &[Sulfur],
            DepositKind::Coal => &[CatalogueItem::Coal],
            Petroleum => &[CrudeOil, NaturalGas],
            Evaporite => &[RockSalt, Gypsum],
            Phosphate => &[PhosphateRock],
            Uranium => &[Uraninite],
            BuildingStone => &[
                Granite, Basalt, Limestone, Marble, Sandstone, Slate, Flint, Kaolin,
            ],
        }
    }
}

/// How one deposit kind forms and how big and rich it is.
#[derive(Debug, Clone, Copy)]
pub struct DepositModel {
    pub kind: DepositKind,
    /// Deposits per 10,000 km² of permissive ground.
    pub density_per_10k_km2: f64,
    pub median_tonnage_t: f64,
    pub tonnage_log10_sd: f64,
    pub median_grade: f64,
    pub grade_log10_sd: f64,
    /// Unit of `grade`.
    pub grade_unit: &'static str,
    pub depth_m: (f64, f64),
    pub source: &'static str,
}

const CS86: &str = "Cox & Singer 1986, USGS Bulletin 1693";
const OOM: &str = "order of magnitude (no grade-tonnage model in the reference pack)";

/// The model of every kind, in [`DepositKind::ALL`] order.
pub fn deposit_models() -> [DepositModel; 20] {
    use DepositKind::*;
    let m = |kind, density, tonnage, t_sd, grade, g_sd, unit, depth, source| DepositModel {
        kind,
        density_per_10k_km2: density,
        median_tonnage_t: tonnage,
        tonnage_log10_sd: t_sd,
        median_grade: grade,
        grade_log10_sd: g_sd,
        grade_unit: unit,
        depth_m: depth,
        source,
    };
    [
        // Low-sulfide Au-quartz veins: 0.03 Mt at 16 g/t.
        m(
            OrogenicGold,
            6.0,
            3.0e4,
            0.9,
            16.0,
            0.3,
            "g/t Au",
            (0.0, 500.0),
            CS86,
        ),
        // Comstock epithermal veins: 0.77 Mt at 7.5 g/t Au (110 g/t Ag).
        m(
            EpithermalGoldSilver,
            3.0,
            7.7e5,
            0.8,
            7.5,
            0.4,
            "g/t Au",
            (0.0, 300.0),
            CS86,
        ),
        // Porphyry Cu: 140 Mt at 0.54% Cu.
        m(
            PorphyryCopper,
            1.5,
            1.4e8,
            0.6,
            0.54,
            0.2,
            "% Cu",
            (0.0, 1_000.0),
            CS86,
        ),
        // Kuroko VMS: 1.5 Mt at 1.3% Cu (2.0% Zn).
        m(
            VolcanogenicMassiveSulfide,
            1.0,
            1.5e6,
            0.8,
            1.3,
            0.3,
            "% Cu",
            (0.0, 500.0),
            CS86,
        ),
        // SEDEX Zn-Pb: 15 Mt at 5.6% Zn (2.8% Pb).
        m(
            SedimentHostedLeadZinc,
            0.5,
            1.5e7,
            0.6,
            5.6,
            0.25,
            "% Zn",
            (0.0, 800.0),
            CS86,
        ),
        // Sn greisen: 7.2 Mt at 0.28% Sn.
        m(
            TinTungsten,
            1.0,
            7.2e6,
            0.7,
            0.28,
            0.3,
            "% Sn",
            (0.0, 500.0),
            CS86,
        ),
        m(
            Pegmatite,
            8.0,
            1.0e6,
            0.8,
            1.0,
            0.25,
            "% Li2O",
            (0.0, 200.0),
            OOM,
        ),
        // Fe skarn: 7.2 Mt at 49% Fe.
        m(
            Skarn,
            // Permissive ground is only the granite–carbonate contact zone,
            // so its density is high: ~1 skarn per 50 km² of contact.
            200.0,
            7.2e6,
            0.7,
            49.0,
            0.05,
            "% Fe",
            (0.0, 500.0),
            CS86,
        ),
        m(
            Ultramafic,
            3.0,
            1.0e6,
            0.8,
            1.0,
            0.3,
            "% Ni",
            (0.0, 300.0),
            OOM,
        ),
        m(
            MetamorphicGems,
            2.0,
            1.0e4,
            0.8,
            5.0,
            0.4,
            "kg/t gem rough",
            (0.0, 100.0),
            OOM,
        ),
        // Superior-type BIF: 170 Mt at 53% Fe.
        m(
            BandedIronFormation,
            2.0,
            1.7e8,
            0.6,
            53.0,
            0.05,
            "% Fe",
            (0.0, 300.0),
            CS86,
        ),
        // Economic pipes 10-100 Mt at 0.1-1.5 ct/t; most pipes are poor.
        // One pipe per kimberlite cell (2,500 per 10,000 km² of 4 km² cells).
        m(
            Kimberlite,
            2_500.0,
            2.6e7,
            0.6,
            5.0,
            0.8,
            "ct/100 t",
            (0.0, 200.0),
            CS86,
        ),
        m(
            VolcanicSilicaGems,
            5.0,
            1.0e5,
            0.8,
            1.0,
            0.5,
            "kg/t gem material",
            (0.0, 50.0),
            OOM,
        ),
        // About one fumarolic sulfur body per volcano (8 km radius = 200 km²).
        m(
            VolcanicSulfur,
            50.0,
            5.0e5,
            0.7,
            30.0,
            0.2,
            "% S",
            (0.0, 20.0),
            OOM,
        ),
        m(
            Coal,
            40.0,
            5.0e7,
            0.8,
            60.0,
            0.1,
            "% fixed carbon",
            (0.0, 600.0),
            OOM,
        ),
        // Oil window ~2-4 km (Tissot & Welte 1984).
        m(
            Petroleum,
            1.5,
            1.0e7,
            0.9,
            15.0,
            0.2,
            "% porosity",
            (1_500.0, 4_000.0),
            OOM,
        ),
        m(
            Evaporite,
            30.0,
            1.0e8,
            0.7,
            95.0,
            0.01,
            "% NaCl",
            (100.0, 1_500.0),
            OOM,
        ),
        // Upwelling phosphorite: 200 Mt at 24% P2O5.
        m(
            Phosphate,
            1.0,
            2.0e8,
            0.6,
            24.0,
            0.15,
            "% P2O5",
            (0.0, 200.0),
            CS86,
        ),
        m(
            Uranium,
            0.8,
            5.0e5,
            0.8,
            0.1,
            0.4,
            "% U3O8",
            (0.0, 500.0),
            OOM,
        ),
        m(
            BuildingStone,
            50.0,
            3.0e7,
            0.6,
            100.0,
            0.0,
            "% usable stone",
            (0.0, 50.0),
            OOM,
        ),
    ]
}

/// One deposit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MineralDeposit {
    pub id: u32,
    pub kind: DepositKind,
    /// Medium-grid cell.
    pub cell: (usize, usize),
    /// Depth of the top of the orebody (m).
    pub depth_m: f64,
    pub tonnage_t: f64,
    /// Grade in the model's unit ([`DepositModel::grade_unit`]).
    pub grade: f64,
    pub host: Lithology,
}

/// What a cell offers a deposit to form.
struct Site {
    rock: Lithology,
    elevation_m: f64,
    margin_km: f64,
    near_volcano_km: f64,
    contact_with_carbonate: bool,
}

/// The host rocks a deposit kind may sit in (its setting rules come on
/// top of these).
pub fn allowed_hosts(kind: DepositKind) -> &'static [Lithology] {
    use DepositKind as K;
    use Lithology::*;
    match kind {
        K::OrogenicGold => &[Greywacke, Schist],
        K::EpithermalGoldSilver => &[ArcAndesite, Rhyolite],
        K::PorphyryCopper | K::TinTungsten | K::Skarn => &[Granite],
        K::VolcanogenicMassiveSulfide => &[OceanicBasalt, ArcAndesite],
        K::SedimentHostedLeadZinc | K::Phosphate => &[Limestone, Mudstone],
        K::Pegmatite => &[Pegmatite],
        K::Ultramafic => &[Ultramafic],
        K::MetamorphicGems => &[Schist, Gneiss, Marble],
        K::BandedIronFormation => &[CratonicGneiss],
        K::Kimberlite => &[Kimberlite],
        K::VolcanicSilicaGems => &[Rhyolite, ArcAndesite, OceanicBasalt],
        // Fumaroles alter whatever rock the volcano stands on.
        K::VolcanicSulfur => &[
            OceanicBasalt,
            ArcAndesite,
            Rhyolite,
            Granite,
            Pegmatite,
            Greywacke,
            Schist,
            Gneiss,
            Marble,
            Limestone,
            Sandstone,
            Mudstone,
            CoalMeasures,
            Evaporite,
            Ultramafic,
            CratonicGneiss,
            Kimberlite,
        ],
        K::Coal => &[CoalMeasures],
        K::Petroleum => &[Mudstone, Sandstone],
        K::Evaporite => &[Evaporite],
        K::Uranium => &[Granite, CratonicGneiss],
        K::BuildingStone => &[
            Granite,
            OceanicBasalt,
            ArcAndesite,
            Limestone,
            Marble,
            Sandstone,
            Schist,
            Mudstone,
            Greywacke,
        ],
    }
}

/// Deepest water (m) over which onshore-type deposits are generated: the
/// continental shelf edge. Deeper ore is beyond any reach the island's
/// people will have; only seafloor deposit types form there.
const SHELF_EDGE_M: f64 = -200.0;
/// Deepest water (m) over which petroleum is generated: shelf and upper
/// slope, the reach of jack-up and platform rigs (~500 m).
const PETROLEUM_MAX_WATER_DEPTH_M: f64 = -500.0;

fn permits_site(kind: DepositKind, s: &Site, scale: f64) -> bool {
    use DepositKind as K;
    use Lithology::*;
    let offshore_kind = matches!(
        kind,
        K::VolcanogenicMassiveSulfide | K::Petroleum | K::Phosphate | K::Evaporite
    );
    if !offshore_kind && s.elevation_m < SHELF_EDGE_M {
        return false;
    }
    match kind {
        K::OrogenicGold => matches!(s.rock, Greywacke | Schist) && s.margin_km <= 150.0 * scale,
        K::EpithermalGoldSilver => {
            matches!(s.rock, ArcAndesite | Rhyolite) && s.near_volcano_km <= 30.0
        }
        K::PorphyryCopper => s.rock == Granite && s.margin_km <= 350.0 * scale,
        K::VolcanogenicMassiveSulfide => {
            s.rock == OceanicBasalt || (s.rock == ArcAndesite && s.elevation_m < 0.0)
        }
        K::SedimentHostedLeadZinc => matches!(s.rock, Limestone | Mudstone),
        K::TinTungsten => s.rock == Granite,
        K::Pegmatite => s.rock == Pegmatite,
        K::Skarn => s.rock == Granite && s.contact_with_carbonate,
        K::Ultramafic => s.rock == Ultramafic,
        K::MetamorphicGems => matches!(s.rock, Schist | Gneiss | Marble),
        K::BandedIronFormation => s.rock == CratonicGneiss,
        K::Kimberlite => s.rock == Kimberlite,
        K::VolcanicSilicaGems => matches!(s.rock, Rhyolite | ArcAndesite | OceanicBasalt),
        K::VolcanicSulfur => s.near_volcano_km <= 8.0,
        K::Coal => s.rock == CoalMeasures,
        K::Petroleum => {
            // Shelf and upper slope only: within reach of shallow-water rigs.
            matches!(s.rock, Mudstone | Sandstone)
                && (PETROLEUM_MAX_WATER_DEPTH_M..100.0).contains(&s.elevation_m)
        }
        K::Evaporite => s.rock == Evaporite,
        K::Phosphate => {
            matches!(s.rock, Limestone | Mudstone) && (-300.0..50.0).contains(&s.elevation_m)
        }
        K::Uranium => matches!(s.rock, Granite | CratonicGneiss),
        K::BuildingStone => {
            s.elevation_m > 0.0
                && matches!(
                    s.rock,
                    Granite
                        | OceanicBasalt
                        | ArcAndesite
                        | Limestone
                        | Marble
                        | Sandstone
                        | Schist
                        | Mudstone
                        | Greywacke
                )
        }
    }
}

/// Standard normal from two uniforms (Box–Muller).
fn normal(u1: f64, u2: f64) -> f64 {
    (-2.0 * u1.max(1e-300).ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// Every primary deposit on the island region, ordered by cell then kind.
pub fn generate_primary_deposits(
    domain: &IslandDomain,
    lithology: &Grid2<Lithology>,
    elevation_m: &Grid2<f64>,
    setting: &GeologicalSetting,
    volcanoes: &[Volcano],
    key: u64,
) -> Vec<MineralDeposit> {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let cell_km2 = domain.cell_area_m2(medium) / 1e6;
    let rocks = lithology.data();

    let sites: Vec<Site> = (0..rows * cols)
        .map(|i| {
            let (r, c) = (i / cols, i % cols);
            let (x, y) = domain.cell_center_m(medium, r, c);
            let near_volcano_km = volcanoes
                .iter()
                .map(|v| (x - v.x_m).hypot(y - v.y_m) / 1000.0)
                .fold(f64::INFINITY, f64::min);
            let contact_with_carbonate =
                [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)]
                    .iter()
                    .any(|&(dr, dc)| {
                        let (rr, cc) = (r as i64 + dr, c as i64 + dc);
                        rr >= 0
                            && cc >= 0
                            && (rr as usize) < rows
                            && (cc as usize) < cols
                            && matches!(
                                rocks[rr as usize * cols + cc as usize],
                                Lithology::Limestone | Lithology::Marble
                            )
                    });
            Site {
                rock: rocks[i],
                elevation_m: *elevation_m.get(r, c),
                margin_km: setting.at(x, y).margin_km,
                near_volcano_km,
                contact_with_carbonate,
            }
        })
        .collect();

    let mut deposits = Vec::new();
    for (k, model) in deposit_models().iter().enumerate() {
        let salt = 0x6465_7000 + k as u64;
        let mut eligible: Vec<(f64, usize)> = sites
            .iter()
            .enumerate()
            .filter(|(_, s)| permits_site(model.kind, s, setting.scale))
            .map(|(i, _)| (hash01(key, salt, i as u64), i))
            .collect();
        let expected = model.density_per_10k_km2 * eligible.len() as f64 * cell_km2 / 10_000.0;
        let mut count = expected.floor() as usize;
        if hash01(key, salt ^ 0xC0, 0) < expected.fract() {
            count += 1;
        }
        let count = count.min(eligible.len());
        // The `count` cells with the smallest hashes.
        eligible.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for &(_, i) in &eligible[..count] {
            let u = |n: u64| hash01(key, salt ^ (n << 32), i as u64);
            let tonnage_t = model.median_tonnage_t
                * 10f64.powf(model.tonnage_log10_sd * normal(u(1), u(2)).clamp(-3.0, 3.0));
            let grade = model.median_grade
                * 10f64.powf(model.grade_log10_sd * normal(u(3), u(4)).clamp(-3.0, 3.0));
            let depth_m = model.depth_m.0 + u(5) * (model.depth_m.1 - model.depth_m.0);
            deposits.push(MineralDeposit {
                id: 0,
                kind: model.kind,
                cell: (i / cols, i % cols),
                depth_m,
                tonnage_t,
                grade,
                host: rocks[i],
            });
        }
    }
    deposits.sort_by(|a, b| a.cell.cmp(&b.cell).then(a.kind.cmp(&b.kind)));
    for (id, d) in deposits.iter_mut().enumerate() {
        d.id = id as u32;
    }
    deposits
}
