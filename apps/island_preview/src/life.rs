//! The `life` subcommand (Phase 3 Task 9 Step 5): biomes, resources, the
//! founders' estate plan and the estate's trees, from the integrated island.

use std::collections::BTreeMap;

use mk_core::biomes::BiomeType;
use mk_engine::organisms::property::PropertyBuildingKind;
use mk_engine::regional::deposits::DepositKind;
use mk_engine::regional::estate_layout::Rect;
use mk_engine::regional::life::IslandLife;
use mk_island::DomainLevel;
use serde_json::json;

use super::{buffer_outline, grid_image, lerp, mark_deposits, Image, Output};

pub fn biome_colour(b: BiomeType) -> [u8; 3] {
    use BiomeType::*;
    match b {
        DeepOcean => [10, 30, 80],
        ShallowOcean => [40, 90, 160],
        CoastalWaters => [90, 160, 210],
        ReefSea => [60, 190, 200],
        Desert => [230, 205, 140],
        SemiDesert => [210, 190, 130],
        Savanna => [200, 190, 90],
        Grassland => [150, 200, 90],
        Shrubland => [140, 160, 90],
        Tundra => [170, 180, 150],
        TropicalRainforest => [10, 100, 40],
        TropicalDryForest => [60, 130, 50],
        TemperateForest => [30, 120, 60],
        BorealForest => [40, 100, 90],
        Woodland => [90, 150, 70],
        MontaneForest => [50, 110, 80],
        Alpine => [190, 190, 190],
        Wetland => [80, 150, 150],
        River => [70, 130, 230],
        Volcanic => [140, 60, 50],
        IceSheet => [245, 250, 255],
    }
}

fn building_colour(kind: PropertyBuildingKind) -> [u8; 3] {
    match kind {
        PropertyBuildingKind::House => [205, 150, 105],
        PropertyBuildingKind::Shed => [150, 120, 90],
        PropertyBuildingKind::Workshop => [120, 125, 170],
        PropertyBuildingKind::Armoury => [165, 90, 90],
        PropertyBuildingKind::ComputerRoom => [90, 170, 200],
        PropertyBuildingKind::Garage => [110, 155, 120],
    }
}

impl Image {
    fn fill_rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
        for y in y0.max(0)..y1.min(self.height as i64) {
            for x in x0.max(0)..x1.min(self.width as i64) {
                self.set(x as usize, y as usize, c);
            }
        }
    }

    fn outline_rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
        self.line(x0, y0, x1, y0, c);
        self.line(x1, y0, x1, y1, c);
        self.line(x1, y1, x0, y1, c);
        self.line(x0, y1, x0, y0, c);
    }
}

/// Estate-block marker on a medium-grid image (north up).
fn mark_estate(img: &mut Image, life: &IslandLife) {
    let rows = life.domain.rows(DomainLevel::Medium);
    let (row, col) = life.placed.location;
    img.mark(col + 1, rows - 1 - (row + 1), 4, [255, 0, 255]);
}

fn biomes_image(life: &IslandLife) -> Image {
    let mut img = grid_image(&life.domain, |r, c| {
        biome_colour(*life.ecology.biome_grid.get(r, c))
    });
    buffer_outline(&mut img, &life.domain);
    mark_estate(&mut img, life);
    img
}

fn resources_image(life: &IslandLife) -> Image {
    let g = &life.physical.geophysics;
    let mut img = grid_image(&life.domain, |r, c| {
        let e = *g.elevation_m.get(r, c);
        if e <= 0.0 {
            return [15, 25, 50];
        }
        match life.ecology.biome_grid.get(r, c) {
            BiomeType::River => [70, 130, 230],
            BiomeType::CoastalWaters => [120, 200, 255],
            _ => {
                let t = (e / 1_500.0).clamp(0.0, 1.0);
                lerp([60, 60, 55], [170, 170, 160], t)
            }
        }
    });
    mark_deposits(&mut img, &life.domain, g, &DepositKind::ALL, 1, 1);
    mark_estate(&mut img, life);
    img
}

/// The estate at 8 px per metre, north up, with a margin around the yard.
fn estate_plan_image(life: &IslandLife) -> Image {
    const PX_PER_M: f64 = 8.0;
    const MARGIN_M: f64 = 20.0;
    let layout = &life.placed.layout;
    let yard: Rect = layout.yard;
    let (x0, y0) = (yard.x0 - MARGIN_M, yard.y0 - MARGIN_M);
    let (w_m, h_m) = (
        yard.width() + 2.0 * MARGIN_M,
        yard.height() + 2.0 * MARGIN_M,
    );
    let (w, h) = ((w_m * PX_PER_M) as usize, (h_m * PX_PER_M) as usize);
    let px = |x: f64| ((x - x0) * PX_PER_M) as i64;
    let py = |y: f64| (((y0 + h_m) - y) * PX_PER_M) as i64;

    let patch = &layout.patch;
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for v in layout.terrain_m.data() {
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    let mut img = Image::new(w, h, [0, 0, 0]);
    for iy in 0..h {
        for ix in 0..w {
            let (x, y) = (x0 + ix as f64 / PX_PER_M, y0 + h_m - iy as f64 / PX_PER_M);
            let (c, r) = (
                ((x - patch.origin_x_m) / patch.cell_size_m) as usize,
                ((y - patch.origin_y_m) / patch.cell_size_m) as usize,
            );
            let shade = if r < patch.rows && c < patch.cols {
                ((layout.terrain_m.get(r, c) - lo) / (hi - lo).max(1e-9)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            img.set(ix, iy, lerp([70, 95, 60], [120, 150, 100], shade));
        }
    }
    img.outline_rect(
        px(yard.x0),
        py(yard.y1),
        px(yard.x1),
        py(yard.y0),
        [230, 230, 120],
    );
    for f in &layout.buildings {
        let r = f.rect_m;
        img.fill_rect(
            px(r.x0),
            py(r.y1),
            px(r.x1),
            py(r.y0),
            building_colour(f.kind),
        );
    }
    for s in &layout.spaces {
        let r = s.rect_m;
        img.outline_rect(px(r.x0), py(r.y1), px(r.x1) - 1, py(r.y0) - 1, [40, 30, 20]);
    }
    for f in &layout.buildings {
        let r = f.rect_m;
        img.outline_rect(px(r.x0), py(r.y1), px(r.x1) - 1, py(r.y0) - 1, [0, 0, 0]);
    }
    for d in &layout.doors {
        img.mark(
            px(d.position_m.0).max(0) as usize,
            py(d.position_m.1).max(0) as usize,
            3,
            [220, 30, 30],
        );
    }
    for it in &layout.items {
        img.mark(
            px(it.position_m.0).max(0) as usize,
            py(it.position_m.1).max(0) as usize,
            1,
            [250, 220, 40],
        );
    }
    for p in life.positions.0.values() {
        let (cx, cy) = (
            px(p.position_m.0).max(0) as usize,
            py(p.position_m.1).max(0) as usize,
        );
        img.mark(cx, cy, 6, [0, 0, 0]);
        img.mark(cx, cy, 4, [255, 255, 255]);
    }
    img
}

/// The 4 km patch at 5 m per pixel: stand cover shaded by carbon, trees as
/// dots sized by stem, the yard dark.
fn estate_trees_image(life: &IslandLife) -> Image {
    let v = &life.vegetation;
    let patch = &life.placed.layout.patch;
    let (rows, cols) = (patch.rows, patch.cols);
    let max_stand = v
        .stands
        .data()
        .iter()
        .map(|s| s.biomass_kgc)
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let mut img = Image::new(cols, rows, [0, 0, 0]);
    for r in 0..rows {
        for c in 0..cols {
            let s = v.stands.get(r, c);
            let t = (s.biomass_kgc / max_stand).clamp(0.0, 1.0);
            let colour = if s.biomass_kgc <= 0.0 {
                [20, 20, 20]
            } else {
                lerp([150, 190, 100], biome_colour(s.biome), 0.35 + 0.65 * t)
            };
            img.set(c, rows - 1 - r, colour);
        }
    }
    for t in &v.trees {
        let (c, r) = (
            ((t.position_m.0 - patch.origin_x_m) / patch.cell_size_m) as usize,
            ((t.position_m.1 - patch.origin_y_m) / patch.cell_size_m) as usize,
        );
        let shade = (t.stem_diameter_m / 0.8).clamp(0.0, 1.0);
        img.set(
            c.min(cols - 1),
            rows - 1 - r.min(rows - 1),
            lerp([20, 90, 30], [5, 40, 15], shade),
        );
    }
    let yard = life.placed.layout.yard;
    let px = |x: f64| ((x - patch.origin_x_m) / patch.cell_size_m) as i64;
    let py = |y: f64| rows as i64 - 1 - ((y - patch.origin_y_m) / patch.cell_size_m) as i64;
    img.outline_rect(
        px(yard.x0),
        py(yard.y1),
        px(yard.x1),
        py(yard.y0),
        [255, 0, 255],
    );
    img
}

/// The `life` subcommand's files.
pub fn render_life(life: &IslandLife) -> Vec<Output> {
    let mut area_km2: BTreeMap<String, f64> = BTreeMap::new();
    let cell_km2 = life.domain.cell_area_m2(DomainLevel::Medium) / 1e6;
    for b in life.ecology.biome_grid.data() {
        *area_km2.entry(format!("{b:?}")).or_insert(0.0) += cell_km2;
    }
    let (row, col) = life.placed.location;
    let summary = json!({
        "estate_cell": [row, col],
        "biome_area_km2": area_km2,
        "land_biomass_kgc": life.ecology.total_biomass_kgc(&life.domain),
        "patch_trees": life.vegetation.trees.len(),
        "patch_individual_radius_m": life.vegetation.individual_radius_m,
        "patch_carbon_kgc": life.vegetation.total_carbon_kgc(),
        "buildings": life.placed.layout.buildings.len(),
        "spaces": life.placed.layout.spaces.len(),
        "doors": life.placed.layout.doors.len(),
        "items": life.placed.layout.items.len(),
        "deposits": life.physical.geophysics.deposits.len(),
        "founders": life.positions.0.keys().collect::<Vec<_>>(),
        "state_digest": life.state_digest().iter().map(|b| format!("{b:02x}")).collect::<String>(),
    });
    vec![
        ("biomes.png".into(), biomes_image(life).png()),
        ("resources.png".into(), resources_image(life).png()),
        ("estate_plan.png".into(), estate_plan_image(life).png()),
        ("estate_trees.png".into(), estate_trees_image(life).png()),
        (
            "summary.json".into(),
            serde_json::to_vec_pretty(&summary).expect("summary serialises"),
        ),
    ]
}
