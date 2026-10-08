//! Headless previews of the island region (Phase 1 Task 6).
//!
//! Renderers return file names and bytes; nothing here reads clocks,
//! writes timestamps or mutates simulation state, so the same profile and
//! seed always give the same bytes. Later phases add subcommands for the
//! physical world, life and the running world.

pub mod font;
pub mod life;

use std::collections::BTreeMap;

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::deposits::{deposit_models, DepositKind};
use mk_engine::regional::geology::Lithology;
use mk_engine::regional::geophysics::{
    generate_regional_geophysics, RegionalGeophysics, RegionalGeophysicsError,
};
use mk_engine::regional::hydrology::{discharge_m3_s, RIVER_MIN_DISCHARGE_M3_S};
use mk_engine::regional::levels::sample_coarse_at_medium;
use mk_engine::regional::physical::{RegionalPhysicalError, RegionalPhysicalState};
use mk_engine::regional::shape::component_sizes;
use mk_engine::tectonics::BoundaryType;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};
use serde_json::{json, Value};

/// A rendered file: name relative to the output directory, and its bytes.
pub type Output = (String, Vec<u8>);

/// An RGB image.
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

impl Image {
    fn new(width: usize, height: usize, fill: [u8; 3]) -> Self {
        Self {
            width,
            height,
            rgb: fill
                .iter()
                .cycle()
                .take(3 * width * height)
                .copied()
                .collect(),
        }
    }

    fn set(&mut self, x: usize, y: usize, c: [u8; 3]) {
        if x < self.width && y < self.height {
            let k = 3 * (y * self.width + x);
            self.rgb[k..k + 3].copy_from_slice(&c);
        }
    }

    fn get(&self, x: usize, y: usize) -> [u8; 3] {
        let k = 3 * (y * self.width + x);
        [self.rgb[k], self.rgb[k + 1], self.rgb[k + 2]]
    }

    /// A filled square of side `2·r + 1` centred on (x, y).
    fn mark(&mut self, x: usize, y: usize, r: usize, c: [u8; 3]) {
        for dy in 0..=2 * r {
            for dx in 0..=2 * r {
                if let (Some(px), Some(py)) = ((x + dx).checked_sub(r), (y + dy).checked_sub(r)) {
                    self.set(px, py, c);
                }
            }
        }
    }

    /// Box-averaged downscale by an integer factor.
    pub fn downscale(&self, factor: usize) -> Image {
        let (w, h) = (self.width / factor, self.height / factor);
        let mut out = Image::new(w, h, [0, 0, 0]);
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 3];
                for dy in 0..factor {
                    for dx in 0..factor {
                        let p = self.get(x * factor + dx, y * factor + dy);
                        for k in 0..3 {
                            sum[k] += u32::from(p[k]);
                        }
                    }
                }
                let n = (factor * factor) as u32;
                out.set(
                    x,
                    y,
                    [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8],
                );
            }
        }
        out
    }

    /// PNG bytes: RGB8, default compression, no text or time chunks.
    pub fn png(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width as u32, self.height as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("png header");
            writer.write_image_data(&self.rgb).expect("png data");
        }
        bytes
    }
}

/// Medium-grid image with north up: row 0 (south) is the bottom line.
fn grid_image(domain: &IslandDomain, colour: impl Fn(usize, usize) -> [u8; 3]) -> Image {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let mut img = Image::new(cols, rows, [0, 0, 0]);
    for r in 0..rows {
        for c in 0..cols {
            img.set(c, rows - 1 - r, colour(r, c));
        }
    }
    img
}

fn lerp(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|k| (f64::from(a[k]) + (f64::from(b[k]) - f64::from(a[k])) * t).round() as u8)
}

/// Hypsometric land tints and depth-shaded ocean.
pub fn elevation_colour(e: f64) -> [u8; 3] {
    if e <= 0.0 {
        let depth = -e;
        if depth < 200.0 {
            lerp([160, 210, 235], [90, 160, 210], depth / 200.0)
        } else {
            lerp(
                [90, 160, 210],
                [10, 30, 80],
                ((depth - 200.0) / 5_800.0).sqrt(),
            )
        }
    } else {
        let stops: [(f64, [u8; 3]); 6] = [
            (0.0, [70, 140, 70]),
            (300.0, [130, 175, 90]),
            (1_000.0, [200, 185, 120]),
            (2_000.0, [150, 110, 80]),
            (3_000.0, [190, 190, 190]),
            (4_000.0, [255, 255, 255]),
        ];
        for w in stops.windows(2) {
            if e <= w[1].0 {
                return lerp(w[0].1, w[1].1, (e - w[0].0) / (w[1].0 - w[0].0));
            }
        }
        stops[5].1
    }
}

fn lithology_colour(l: Lithology) -> [u8; 3] {
    use Lithology::*;
    match l {
        OceanicBasalt => [40, 60, 90],
        ArcAndesite => [200, 80, 60],
        Rhyolite => [240, 140, 140],
        Granite => [230, 120, 200],
        Pegmatite => [255, 200, 255],
        Greywacke => [130, 140, 120],
        Schist => [90, 160, 110],
        Gneiss => [60, 110, 70],
        Marble => [235, 235, 220],
        Limestone => [140, 200, 230],
        Sandstone => [235, 210, 140],
        Mudstone => [150, 130, 110],
        CoalMeasures => [40, 40, 40],
        Evaporite => [255, 255, 160],
        Ultramafic => [20, 120, 60],
        CratonicGneiss => [180, 90, 40],
        Kimberlite => [0, 255, 255],
    }
}

fn deposit_colour(k: DepositKind) -> [u8; 3] {
    let i = DepositKind::ALL.iter().position(|&x| x == k).unwrap_or(0) as u64;
    let h = blake3::hash(&i.to_le_bytes());
    let b = h.as_bytes();
    match k {
        DepositKind::OrogenicGold | DepositKind::EpithermalGoldSilver => [255, 210, 0],
        DepositKind::Kimberlite => [0, 255, 255],
        _ => [96 + b[0] % 160, 96 + b[1] % 160, 96 + b[2] % 160],
    }
}

fn plate_colour(id: u8) -> [u8; 3] {
    let h = blake3::hash(&[b"plate".as_slice(), &[id]].concat());
    let b = h.as_bytes();
    [60 + b[0] % 180, 60 + b[1] % 180, 60 + b[2] % 180]
}

/// Draws the ocean-buffer rectangle.
fn buffer_outline(img: &mut Image, domain: &IslandDomain) {
    let medium = DomainLevel::Medium;
    let b = (domain.profile().minimum_ocean_buffer_m / domain.cell_size_m(medium)).round() as usize;
    let (w, h) = (img.width, img.height);
    if 2 * b >= w || 2 * b >= h {
        return;
    }
    for x in b..w - b {
        img.set(x, b, [255, 80, 80]);
        img.set(x, h - 1 - b, [255, 80, 80]);
    }
    for y in b..h - b {
        img.set(b, y, [255, 80, 80]);
        img.set(w - 1 - b, y, [255, 80, 80]);
    }
}

/// The elevation image with the buffer outline.
pub fn elevation_image(domain: &IslandDomain, g: &RegionalGeophysics) -> Image {
    let mut img = grid_image(domain, |r, c| elevation_colour(*g.elevation_m.get(r, c)));
    buffer_outline(&mut img, domain);
    img
}

/// The top of the biomass ramp, in kgC/m². Standing producer carbon on
/// this island's land tops out just under it -- the densest medium cell
/// measures 18.6 -- so the scale is the island's own range rather than a
/// round number picked to be safe.
pub const BIOMASS_FULL_KGC_M2: f64 = 20.0;

/// What a square metre of standing producer carbon is coloured.
///
/// Square-rooted, not linear, and that is a decision rather than a
/// flourish. The island's land is heavily skewed: the median medium cell
/// carries 0.8 kgC/m² and the densest 18.6, so a linear ramp puts half the
/// island inside the bottom twenty-fifth of its own scale and draws the
/// whole interior as one flat tan. The square root is still a monotonic
/// function of a real measurement -- darker is always more carbon -- but
/// it spends its range where the island actually is. The viewer's key says
/// the scale is square-rooted and gives both ends, because a colour
/// nobody can read back into a number is decoration.
///
/// Sea is left to the caller: biomass over water is zero, and colouring it
/// as bare ground would draw the ocean as a desert.
pub fn biomass_colour(kgc_m2: f64) -> [u8; 3] {
    const BARE: [u8; 3] = [196, 186, 150];
    const SPARSE: [u8; 3] = [154, 176, 104];
    const CLOSED: [u8; 3] = [28, 82, 44];
    let t = (kgc_m2.max(0.0) / BIOMASS_FULL_KGC_M2)
        .clamp(0.0, 1.0)
        .sqrt();
    if t < 0.5 {
        lerp(BARE, SPARSE, t / 0.5)
    } else {
        lerp(SPARSE, CLOSED, (t - 0.5) / 0.5)
    }
}

/// The island's standing vegetation on the medium grid, north up.
///
/// The companion to [`elevation_image`], and the layer that makes the rest
/// of the island's vegetation visible at all: individual stems exist only
/// in a wood a kilometre across, and everything beyond it is this.
pub fn biomass_image(
    domain: &IslandDomain,
    ecology: &mk_engine::regional::ecology::RegionalEcologyState,
    g: &RegionalGeophysics,
) -> Image {
    const SEA: [u8; 3] = [24, 48, 74];
    let mut img = grid_image(domain, |r, c| {
        if *g.land_mask.get(r, c) {
            biomass_colour(*ecology.biomass_kgc_m2.get(r, c))
        } else {
            SEA
        }
    });
    buffer_outline(&mut img, domain);
    img
}

/// The estate's 4 km patch at its own 5 m resolution, north up.
///
/// Four hundred times finer than [`biomass_image`] over the same ground,
/// which is the whole reason it exists: the medium grid draws the patch as
/// four flat squares, and the stand cover underneath has a value every
/// five metres.
pub fn stand_image(patch: &mk_engine::regional::local_vegetation::LocalVegetationPatch) -> Image {
    let spec = patch.patch_spec();
    let (rows, cols) = (spec.rows, spec.cols);
    let area = spec.cell_size_m * spec.cell_size_m;
    let mut img = Image::new(cols, rows, [0, 0, 0]);
    for r in 0..rows {
        for c in 0..cols {
            let cell = patch.stands.get(r, c);
            img.set(c, rows - 1 - r, biomass_colour(cell.biomass_kgc / area));
        }
    }
    img
}

/// Marks deposits of `kinds` on an image of the medium grid scaled down by
/// `factor`.
fn mark_deposits(
    img: &mut Image,
    domain: &IslandDomain,
    g: &RegionalGeophysics,
    kinds: &[DepositKind],
    factor: usize,
    r: usize,
) {
    let rows = domain.rows(DomainLevel::Medium);
    for d in &g.deposits {
        if kinds.contains(&d.kind) {
            let (row, col) = d.cell;
            img.mark(
                col / factor,
                (rows - 1 - row) / factor,
                r,
                deposit_colour(d.kind),
            );
        }
    }
}

/// Deposit counts and ore tonnage per kind.
fn deposit_summary(g: &RegionalGeophysics) -> BTreeMap<String, Value> {
    let mut by_kind: BTreeMap<String, (u64, f64)> = BTreeMap::new();
    for kind in DepositKind::ALL {
        by_kind.insert(format!("{kind:?}"), (0, 0.0));
    }
    for d in &g.deposits {
        let e = by_kind
            .get_mut(&format!("{:?}", d.kind))
            .expect("every kind listed");
        e.0 += 1;
        e.1 += d.tonnage_t;
    }
    let units: BTreeMap<String, &str> = deposit_models()
        .iter()
        .map(|m| (format!("{:?}", m.kind), m.grade_unit))
        .collect();
    by_kind
        .into_iter()
        .map(|(k, (count, tonnage))| {
            let unit = units[&k];
            (
                k,
                json!({ "count": count, "ore_tonnage_t": tonnage, "grade_unit": unit }),
            )
        })
        .collect()
}

/// Gem-bearing deposit kinds present.
fn gems(g: &RegionalGeophysics) -> Vec<&'static str> {
    let has = |k: DepositKind| g.deposits.iter().any(|d| d.kind == k);
    let mut out = Vec::new();
    if has(DepositKind::Kimberlite) {
        out.push("diamond");
    }
    if has(DepositKind::Pegmatite) {
        out.extend(["emerald", "topaz", "tourmaline", "quartz crystal"]);
    }
    if has(DepositKind::MetamorphicGems) {
        out.extend(["garnet", "ruby", "sapphire"]);
    }
    if has(DepositKind::VolcanicSilicaGems) {
        out.extend(["opal", "agate", "amethyst", "obsidian"]);
    }
    if has(DepositKind::EpithermalGoldSilver) {
        out.push("amethyst");
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn has_gold(g: &RegionalGeophysics) -> bool {
    g.deposits.iter().any(|d| {
        matches!(
            d.kind,
            DepositKind::OrogenicGold
                | DepositKind::EpithermalGoldSilver
                | DepositKind::PorphyryCopper
        )
    })
}

/// `summary.json` for one island.
pub fn summary(domain: &IslandDomain, g: &RegionalGeophysics, seed: &[u8; 32]) -> Value {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let components = component_sizes(g.land_mask.data(), rows, cols, false).len();
    let data = g.elevation_m.data();
    let plates: std::collections::BTreeSet<u8> = g
        .tectonics
        .plates
        .data()
        .iter()
        .map(|p| p.plate_id)
        .collect();
    json!({
        "seed": hex::encode(seed),
        "land_area_km2": g.land_area_m2 / 1e6,
        "sea_level_offset_m": g.sea_level_offset_m,
        "land_components": components,
        "min_elevation_m": data.iter().cloned().fold(f64::INFINITY, f64::min),
        "max_elevation_m": data.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "plate_count": plates.len(),
        "volcano_count": g.volcanoes.len(),
        "shape": g.shape,
        "shape_unmet": domain.profile().shape.unmet(&g.shape),
        "has_gold": has_gold(g),
        "has_diamonds": g.deposits.iter().any(|d| d.kind == DepositKind::Kimberlite),
        "gems": gems(g),
        "deposits": deposit_summary(g),
    })
}

/// Loads the island canon from the repository fixtures.
pub fn island_canon() -> CanonLocked {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("fixtures/island/canon.json")
}

/// Generates the island (without the shape check) for a profile and seed.
pub fn generate(
    profile: &IslandProfile,
    seed: [u8; 32],
) -> (
    IslandDomain,
    Result<RegionalGeophysics, RegionalGeophysicsError>,
) {
    let canon = island_canon();
    let domain = IslandDomain::from_profile(profile.clone()).expect("valid profile");
    let boundaries = sample_regional_boundaries(seed, &canon, &domain, 0.0);
    let g = generate_regional_geophysics(&canon, &domain, &boundaries, seed);
    (domain, g)
}

/// The `geophysics` subcommand's files.
pub fn render_geophysics(
    domain: &IslandDomain,
    g: &RegionalGeophysics,
    seed: &[u8; 32],
) -> Vec<Output> {
    let medium = DomainLevel::Medium;
    let coarse_m = domain.cell_size_m(DomainLevel::Coarse);
    let medium_m = domain.cell_size_m(medium);
    let land = grid_image(domain, |r, c| {
        if *g.land_mask.get(r, c) {
            [255, 255, 255]
        } else {
            [0, 0, 0]
        }
    });
    let plates = grid_image(domain, |r, c| {
        let (x, y) = domain.cell_center_m(medium, r, c);
        let (pr, pc) = ((y / coarse_m) as usize, (x / coarse_m) as usize);
        let cell = g.tectonics.plates.get(pr, pc);
        let base = plate_colour(cell.plate_id);
        let tinted = match cell.boundary {
            Some(BoundaryType::Subduction) => [200, 30, 30],
            Some(BoundaryType::Collision) => [140, 0, 140],
            Some(BoundaryType::Ridge) => [30, 30, 200],
            Some(BoundaryType::Transform) => [30, 30, 30],
            None => base,
        };
        if *g.land_mask.get(r, c) {
            lerp(tinted, [255, 255, 255], 0.35)
        } else {
            tinted
        }
    });
    let geology = grid_image(domain, |r, c| lithology_colour(*g.lithology.get(r, c)));
    let mut deposits = grid_image(domain, |r, c| {
        if *g.land_mask.get(r, c) {
            [70, 70, 70]
        } else {
            [15, 20, 35]
        }
    });
    mark_deposits(&mut deposits, domain, g, &DepositKind::ALL, 1, 1);
    let _ = medium_m;
    vec![
        ("elevation.png".into(), elevation_image(domain, g).png()),
        ("land_mask.png".into(), land.png()),
        ("plates.png".into(), plates.png()),
        ("geology.png".into(), geology.png()),
        ("deposits.png".into(), deposits.png()),
        (
            "summary.json".into(),
            serde_json::to_vec_pretty(&summary(domain, g, seed)).expect("summary serialises"),
        ),
    ]
}

/// Gallery candidate `index`: `blake3("island-gallery" ‖ index)`.
pub fn gallery_seed(index: u32) -> [u8; 32] {
    *blake3::hash(&[b"island-gallery".as_slice(), &index.to_le_bytes()].concat()).as_bytes()
}

/// Thumbnail downscale factor: the full island's 1,200 × 960 cells become
/// 300 × 240 pixels.
const THUMB_FACTOR: usize = 4;
const GALLERY_COLUMNS: usize = 6;
const LABEL_HEIGHT: usize = 18;

/// The `gallery` subcommand's files.
pub fn render_gallery(profile: &IslandProfile, seeds: u32) -> Vec<Output> {
    let mut outputs = Vec::new();
    let mut entries = Vec::new();
    let mut tiles: Vec<(u32, Option<Image>, bool, bool, bool)> = Vec::new();
    for index in 0..seeds {
        let seed = gallery_seed(index);
        let hex_seed = hex::encode(seed);
        let (domain, result) = generate(profile, seed);
        match result {
            Ok(g) => {
                let unmet = profile.shape.unmet(&g.shape);
                let mut thumb = elevation_image(&domain, &g).downscale(THUMB_FACTOR);
                mark_deposits(
                    &mut thumb,
                    &domain,
                    &g,
                    &[DepositKind::OrogenicGold, DepositKind::EpithermalGoldSilver],
                    THUMB_FACTOR,
                    1,
                );
                mark_deposits(
                    &mut thumb,
                    &domain,
                    &g,
                    &[DepositKind::Kimberlite],
                    THUMB_FACTOR,
                    1,
                );
                outputs.push((format!("seed-{hex_seed}.png"), thumb.png()));
                let gold = has_gold(&g);
                let diamonds = g.deposits.iter().any(|d| d.kind == DepositKind::Kimberlite);
                entries.push(json!({
                    "index": index,
                    "seed": hex_seed,
                    "pass": unmet.is_empty(),
                    "reason": if unmet.is_empty() { Value::Null } else { json!(unmet.join("; ")) },
                    "land_area_km2": g.land_area_m2 / 1e6,
                    "shape": g.shape,
                    "has_gold": gold,
                    "has_diamonds": diamonds,
                    "gems": gems(&g),
                    "deposits": deposit_summary(&g),
                }));
                tiles.push((index, Some(thumb), unmet.is_empty(), gold, diamonds));
            }
            Err(e) => {
                entries.push(json!({
                    "index": index,
                    "seed": hex_seed,
                    "pass": false,
                    "reason": e.to_string(),
                }));
                tiles.push((index, None, false, false, false));
            }
        }
    }

    // Contact sheet.
    let domain = IslandDomain::from_profile(profile.clone()).expect("valid profile");
    let medium = DomainLevel::Medium;
    let (tw, th) = (
        domain.cols(medium) / THUMB_FACTOR,
        domain.rows(medium) / THUMB_FACTOR,
    );
    let columns = GALLERY_COLUMNS.min(tiles.len().max(1));
    let rows = tiles.len().div_ceil(columns).max(1);
    let (cell_w, cell_h) = (tw + 4, th + LABEL_HEIGHT + 4);
    let mut sheet = Image::new(columns * cell_w, rows * cell_h, [20, 20, 20]);
    for (n, (index, thumb, pass, gold, diamonds)) in tiles.iter().enumerate() {
        let (ox, oy) = ((n % columns) * cell_w + 2, (n / columns) * cell_h + 2);
        if let Some(t) = thumb {
            for y in 0..th.min(t.height) {
                for x in 0..tw.min(t.width) {
                    sheet.set(ox + x, oy + y, t.get(x, y));
                }
            }
        }
        let mut label = index.to_string();
        if *gold {
            label.push_str(" G");
        }
        if *diamonds {
            label.push_str(" D");
        }
        if !pass {
            label.push_str(" X");
        }
        let colour = if *pass {
            [255, 255, 255]
        } else {
            [255, 90, 90]
        };
        let (w, h) = (sheet.width, sheet.height);
        font::draw_text(&mut sheet.rgb, w, h, ox + 2, oy + th + 3, &label, colour, 2);
    }
    outputs.push(("gallery.png".into(), sheet.png()));
    outputs.push((
        "gallery.json".into(),
        serde_json::to_vec_pretty(&json!({ "profile": profile, "candidates": entries }))
            .expect("gallery serialises"),
    ));
    outputs
}

/// A simulated stretch of the island's physical state, with the means the
/// previews show.
pub struct PhysicalRun {
    pub domain: IslandDomain,
    pub state: RegionalPhysicalState,
    pub seed: [u8; 32],
    pub days: u64,
    /// Mean surface temperature (K) and rain (mm/day) per coarse cell over
    /// the run.
    pub mean_temperature_k: Vec<f64>,
    pub mean_rain_mm_day: Vec<f64>,
}

/// Bootstraps the island for `profile` and `seed` (which must make a valid
/// island) and runs `days` local days of the physical tick.
pub fn simulate_physical(
    profile: &IslandProfile,
    seed: [u8; 32],
    days: u64,
) -> Result<PhysicalRun, RegionalPhysicalError> {
    let canon = Arc::new(island_canon());
    let domain = IslandDomain::from_profile(profile.clone()).expect("valid profile");
    let mut state = RegionalPhysicalState::bootstrap(&canon, &domain, seed)?;
    let n = domain.rows(DomainLevel::Coarse) * domain.cols(DomainLevel::Coarse);
    let (mut temperature, mut rain) = (vec![0.0; n], vec![0.0; n]);
    let dt = canon.rotation_period_s as u64;
    for day in 1..=days {
        state.step(&canon, &domain, seed, (day * dt) as f64, day, dt)?;
        for i in 0..n {
            temperature[i] += state.climate.surface_temperature.data()[i];
            rain[i] += state.weather.precipitation.data()[i];
        }
    }
    let scale = 1.0 / days.max(1) as f64;
    Ok(PhysicalRun {
        domain,
        state,
        seed,
        days,
        mean_temperature_k: temperature.into_iter().map(|v| v * scale).collect(),
        mean_rain_mm_day: rain.into_iter().map(|v| v * scale).collect(),
    })
}

fn ramp(stops: &[(f64, [u8; 3])], v: f64) -> [u8; 3] {
    if v <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        if v <= w[1].0 {
            return lerp(w[0].1, w[1].1, (v - w[0].0) / (w[1].0 - w[0].0));
        }
    }
    stops[stops.len() - 1].1
}

/// Surface temperature colours: fixed 270-305 K range so previews compare.
pub fn temperature_colour(k: f64) -> [u8; 3] {
    ramp(
        &[
            (270.0, [40, 70, 200]),
            (280.0, [60, 190, 220]),
            (288.0, [230, 230, 100]),
            (296.0, [240, 140, 40]),
            (305.0, [190, 30, 30]),
        ],
        k,
    )
}

/// Rain colours (mm/day): white through blue to purple.
pub fn rain_colour(mm_day: f64) -> [u8; 3] {
    ramp(
        &[
            (0.0, [245, 245, 235]),
            (2.0, [170, 210, 240]),
            (5.0, [60, 120, 220]),
            (10.0, [90, 40, 180]),
            (25.0, [50, 0, 90]),
        ],
        mm_day,
    )
}

impl Image {
    /// A straight line, Bresenham.
    fn line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            if x >= 0 && y >= 0 {
                self.set(x as usize, y as usize, c);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

/// The `physical` subcommand's files.
pub fn render_physical(run: &PhysicalRun) -> Vec<Output> {
    let (domain, s) = (&run.domain, &run.state);
    let coarse = DomainLevel::Coarse;
    let medium = DomainLevel::Medium;
    let cols_c = domain.cols(coarse);
    let land = |r: usize, c: usize| *s.geophysics.elevation_m.get(r, c) > 0.0;
    // A coarse field at a medium cell, bilinear.
    let at = |f: &dyn Fn(usize, usize) -> f64, r: usize, c: usize| {
        sample_coarse_at_medium(f, domain, r, c)
    };
    let shade = |col: [u8; 3], is_land: bool| {
        if is_land {
            lerp(col, [255, 255, 255], 0.12)
        } else {
            col
        }
    };

    let temperature = grid_image(domain, |r, c| {
        let k = at(&|a, b| run.mean_temperature_k[a * cols_c + b], r, c);
        shade(temperature_colour(k), land(r, c))
    });
    let rainfall = grid_image(domain, |r, c| {
        let mm = at(&|a, b| run.mean_rain_mm_day[a * cols_c + b], r, c);
        let col = rain_colour(mm);
        if land(r, c) {
            col
        } else {
            lerp(col, [60, 60, 80], 0.35)
        }
    });
    let net = s.flow_network();
    let mut rivers = grid_image(domain, |r, c| {
        if !land(r, c) {
            return [15, 25, 50];
        }
        if net.lake_depth_m(r, c) > 0.0 {
            return [120, 200, 255];
        }
        let q = discharge_m3_s(&s.hydrology, domain, r, c);
        if q >= RIVER_MIN_DISCHARGE_M3_S {
            let t = (q.log10() / 3.0).clamp(0.0, 1.0);
            return lerp([140, 230, 255], [0, 40, 200], t);
        }
        [95, 95, 85]
    });
    buffer_outline(&mut rivers, domain);
    let sst = |a: usize, b: usize| s.ocean.columns.get(a, b).surface_temp;
    let mut ocean_t = grid_image(domain, |r, c| {
        if land(r, c) {
            [70, 70, 70]
        } else {
            temperature_colour(at(&sst, r, c))
        }
    });
    let mut currents = grid_image(domain, |r, c| {
        if land(r, c) {
            [70, 70, 70]
        } else {
            lerp(temperature_colour(at(&sst, r, c)), [20, 20, 30], 0.6)
        }
    });
    // Current vectors: one arrow per 6 x 6 coarse cells, scaled so the
    // strongest drawn current is 28 px (the scale is in summary.json).
    let (rows_m, step) = (domain.rows(medium), 6usize);
    let px_per_coarse = (domain.cell_size_m(coarse) / domain.cell_size_m(medium)).round() as usize;
    let sampled: Vec<(usize, usize)> = (step / 2..domain.rows(coarse))
        .step_by(step)
        .flat_map(|r| (step / 2..cols_c).step_by(step).map(move |c| (r, c)))
        .filter(|&(r, c)| s.ocean.columns.get(r, c).depth > 0.0)
        .collect();
    // The strongest interior current sets the scale: the edge cells are
    // forced to the boundary inflow and would dominate it.
    let (rows_c, margin) = (domain.rows(coarse), 8usize);
    let strongest_cm_s = sampled
        .iter()
        .filter(|&&(r, c)| r >= margin && c >= margin && r + margin < rows_c && c + margin < cols_c)
        .map(|&(r, c)| {
            let v = s.ocean.currents.get(r, c);
            v.u_east.hypot(v.v_north)
        })
        .fold(0.0_f64, f64::max);
    let px_per_cm_s = if strongest_cm_s > 0.0 {
        28.0 / strongest_cm_s
    } else {
        0.0
    };
    for &(r, c) in &sampled {
        let v = s.ocean.currents.get(r, c);
        let (x0, y0) = (
            ((c as f64 + 0.5) * px_per_coarse as f64) as i64,
            (rows_m as f64 - (r as f64 + 0.5) * px_per_coarse as f64) as i64,
        );
        let (x1, y1) = (
            x0 + (v.u_east * px_per_cm_s).round() as i64,
            y0 - (v.v_north * px_per_cm_s).round() as i64,
        );
        currents.line(x0, y0, x1, y1, [255, 255, 255]);
        currents.mark(x1.max(0) as usize, y1.max(0) as usize, 1, [255, 220, 0]);
    }
    buffer_outline(&mut ocean_t, domain);

    let stats = |values: &dyn Fn(usize) -> Option<f64>| {
        let v: Vec<f64> = (0..run.mean_rain_mm_day.len()).filter_map(values).collect();
        (
            v.iter().cloned().fold(f64::MAX, f64::min),
            v.iter().cloned().fold(f64::MIN, f64::max),
            v.iter().sum::<f64>() / v.len().max(1) as f64,
        )
    };
    let is_land_c = |i: usize| *s.coarse_elevation_m().data().get(i).unwrap_or(&0.0) > 0.0;
    let (tl_min, tl_max, tl_mean) = stats(&|i| is_land_c(i).then_some(run.mean_temperature_k[i]));
    let (to_min, to_max, to_mean) =
        stats(&|i| (!is_land_c(i)).then_some(run.mean_temperature_k[i]));
    let (_, rl_max, rl_mean) = stats(&|i| is_land_c(i).then_some(run.mean_rain_mm_day[i]));
    let rivers_cells = net.outlets().count();
    let river_cells = (0..domain.rows(medium))
        .flat_map(|r| (0..domain.cols(medium)).map(move |c| (r, c)))
        .filter(|&(r, c)| {
            land(r, c) && discharge_m3_s(&s.hydrology, domain, r, c) >= RIVER_MIN_DISCHARGE_M3_S
        })
        .count();
    let lake_cells = (0..domain.rows(medium))
        .flat_map(|r| (0..domain.cols(medium)).map(move |c| (r, c)))
        .filter(|&(r, c)| land(r, c) && net.lake_depth_m(r, c) > 0.0)
        .count();
    let summary = json!({
        "seed": run.seed.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "local_days": run.days,
        "land_surface_temperature_k": {"min": tl_min, "max": tl_max, "mean": tl_mean},
        "ocean_surface_temperature_k": {"min": to_min, "max": to_max, "mean": to_mean},
        "land_rain_mm_day": {"max": rl_max, "mean": rl_mean},
        "strongest_interior_current_cm_s": strongest_cm_s,
        "river_cells": river_cells,
        "lake_cells": lake_cells,
        "coastal_outlets": rivers_cells,
        "fresh_water_to_ocean_kg_last_step": s.river_water_to_ocean_kg,
        "state_hash": s.state_hash().iter().map(|b| format!("{b:02x}")).collect::<String>(),
    });
    vec![
        ("temperature.png".into(), temperature.png()),
        ("rainfall.png".into(), rainfall.png()),
        ("rivers_lakes.png".into(), rivers.png()),
        ("ocean_temperature.png".into(), ocean_t.png()),
        ("ocean_currents.png".into(), currents.png()),
        (
            "summary.json".into(),
            serde_json::to_vec_pretty(&summary).expect("summary serialises"),
        ),
    ]
}

/// Bootstraps the integrated island for `scenario_path` (the `life`
/// subcommand's input).
pub fn bootstrap_life(
    scenario: mk_island::IslandScenario,
) -> Result<mk_engine::regional::life::IslandLife, mk_engine::regional::life::IslandLifeError> {
    mk_engine::regional::life::IslandLife::bootstrap(scenario, Arc::new(island_canon()))
}
