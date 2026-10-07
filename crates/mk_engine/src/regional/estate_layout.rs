//! The founders' estate in metres (Phase 3 Task 5).
//!
//! Upstream stores the whole estate as one grid cell with room *labels*
//! only. This lays it out on the island's high-detail patch (5 m cells):
//! a buildable site, five building footprints (the computer room is a room
//! *inside* the House), rooms and zones, doors, a metric position for every
//! item, and a door graph to route through. It derives everything from the
//! `StarterProperty` and never defines items of its own.
//!
//! Patch terrain is the medium-cell elevation, bilinearly interpolated,
//! plus bounded deterministic detail ([`DETAIL_AMPLITUDE_M`]); it never
//! changes the regional elevation. A cell is **buildable** if it is dry
//! land (above [`MIN_BUILDABLE_ELEVATION_M`]), not in a lake or a river,
//! and no steeper than [`MAX_SLOPE`] (2 m rise over the patch's 5 m
//! spacing: upstream's `MAX_CLIMB_HEIGHT_M` applied at patch resolution).
//!
//! Site selection is deterministic: of all positions where the whole
//! cluster of buildings fits on buildable cells, the one nearest the
//! patch centre (ties broken by scan order).

use std::collections::{BTreeMap, VecDeque};

use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain, LocalPatchSpec};
use serde::{Deserialize, Serialize};

use super::hydrology::{discharge_m3_s, RIVER_MIN_DISCHARGE_M3_S};
use super::physical::RegionalPhysicalState;
use crate::organisms::property::{PropertyBuildingKind, StarterProperty};

/// Highest rise per metre a building may stand on: 2 m over 5 m.
pub const MAX_SLOPE: f64 = 2.0 / 5.0;
/// Lowest ground (m above sea level) counted as dry.
pub const MIN_BUILDABLE_ELEVATION_M: f64 = 1.0;
/// Peak size (m) of the deterministic detail added to patch terrain.
pub const DETAIL_AMPLITUDE_M: f64 = 0.4;
/// Wavelength (m) of that detail.
const DETAIL_WAVELENGTH_M: f64 = 20.0;
/// Gap (m) between buildings, and margin (m) kept clear of unbuildable
/// ground around each.
const BUILDING_GAP_M: f64 = 8.0;
const SITE_MARGIN_M: f64 = 2.0;
/// Room and hall dimensions (m) in the House.
const ROOM_WIDTH_M: f64 = 5.0;
const ROOM_DEPTH_M: f64 = 5.0;
const HALL_DEPTH_M: f64 = 2.5;
const SHED_DEPTH_M: f64 = 6.0;
const ZONE_MIN_WIDTH_M: f64 = 4.0;
const YARD_MARGIN_M: f64 = 10.0;

#[derive(Debug, Clone, PartialEq)]
pub enum EstateLayoutError {
    /// No position on the patch fits every building on buildable ground.
    NoBuildableSite,
    /// The property lacks something the layout needs.
    InvalidProperty(String),
}

impl std::fmt::Display for EstateLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoBuildableSite => write!(f, "no buildable site for the estate on the patch"),
            Self::InvalidProperty(why) => write!(f, "invalid estate property: {why}"),
        }
    }
}

impl std::error::Error for EstateLayoutError {}

/// An axis-aligned rectangle in domain metres, half-open: `x0 <= x < x1`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    fn new(x0: f64, y0: f64, w: f64, h: f64) -> Self {
        Self {
            x0,
            y0,
            x1: x0 + w,
            y1: y0 + h,
        }
    }
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
    pub fn contains_rect(&self, o: &Rect) -> bool {
        self.x0 <= o.x0 && self.y0 <= o.y0 && o.x1 <= self.x1 && o.y1 <= self.y1
    }
    fn translated(&self, dx: f64, dy: f64) -> Rect {
        Rect {
            x0: self.x0 + dx,
            y0: self.y0 + dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }
    fn expanded(&self, m: f64) -> Rect {
        Rect {
            x0: self.x0 - m,
            y0: self.y0 - m,
            x1: self.x1 + m,
            y1: self.y1 + m,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SpaceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Space {
    Outdoors,
    Inside(SpaceId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SpaceKind {
    Room,
    Zone,
    WholeBuilding,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildingFootprint {
    pub building_id: u64,
    pub kind: PropertyBuildingKind,
    pub rect_m: Rect,
    pub rotation_deg: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpaceLayout {
    pub id: SpaceId,
    /// The upstream building the space belongs to. The computer room is
    /// inside the House's footprint but keeps its own upstream building id.
    pub building_id: u64,
    pub label: String,
    pub kind: SpaceKind,
    pub rect_m: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Door {
    pub a: Space,
    pub b: Space,
    pub position_m: (f64, f64),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemPlacement {
    pub property_id: u64,
    pub item_id: u64,
    pub space: Space,
    pub position_m: (f64, f64),
}

#[derive(Debug, Clone, Serialize)]
pub struct EstateLayout {
    pub property_id: u64,
    pub patch: LocalPatchSpec,
    /// Patch terrain (m above sea level), `rows × cols` at the patch's cell size.
    pub terrain_m: Grid2<f64>,
    pub buildings: Vec<BuildingFootprint>,
    pub spaces: Vec<SpaceLayout>,
    pub doors: Vec<Door>,
    pub items: Vec<ItemPlacement>,
    pub yard: Rect,
}

impl EstateLayout {
    /// The space a point is in.
    pub fn space_at(&self, position_m: (f64, f64)) -> Space {
        self.spaces
            .iter()
            .find(|s| s.rect_m.contains(position_m.0, position_m.1))
            .map_or(Space::Outdoors, |s| Space::Inside(s.id))
    }

    /// The fewest-doors path from one space to another, both included, or
    /// `None` if the door graph does not connect them.
    pub fn route(&self, from: Space, to: Space) -> Option<Vec<Space>> {
        if from == to {
            return Some(vec![from]);
        }
        let mut previous: BTreeMap<Space, Space> = BTreeMap::new();
        let mut queue = VecDeque::from([from]);
        let mut seen = std::collections::HashSet::from([from]);
        while let Some(here) = queue.pop_front() {
            for door in &self.doors {
                let next = if door.a == here {
                    door.b
                } else if door.b == here {
                    door.a
                } else {
                    continue;
                };
                if seen.insert(next) {
                    previous.insert(next, here);
                    if next == to {
                        let mut path = vec![to];
                        let mut at = to;
                        while let Some(&p) = previous.get(&at) {
                            path.push(p);
                            at = p;
                        }
                        path.reverse();
                        return Some(path);
                    }
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

impl PartialOrd for Space {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Space {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let key = |s: &Space| match s {
            Space::Outdoors => 0u64,
            Space::Inside(id) => 1 + u64::from(id.0),
        };
        key(self).cmp(&key(other))
    }
}

fn unit(seed: &[u8; 32], ix: i64, iy: i64) -> f64 {
    let mut h = blake3::Hasher::new();
    h.update(b"mk-island-estate-detail");
    h.update(seed);
    h.update(&ix.to_le_bytes());
    h.update(&iy.to_le_bytes());
    let b = h.finalize();
    let w = u64::from_le_bytes(b.as_bytes()[..8].try_into().expect("8 bytes"));
    (w >> 11) as f64 / (1u64 << 53) as f64
}

/// Medium elevation at a point, bilinear between cell centres.
fn medium_elevation(elev: &Grid2<f64>, size: f64, x: f64, y: f64) -> f64 {
    let (rows, cols) = (elev.nlat(), elev.nlon());
    let axis = |m: f64, n: usize| {
        let v = (m / size - 0.5).clamp(0.0, (n - 1) as f64);
        let i = (v.floor() as usize).min(n - 1);
        (i, (i + 1).min(n - 1), v - i as f64)
    };
    let (r0, r1, fr) = axis(y, rows);
    let (c0, c1, fc) = axis(x, cols);
    let lo = elev.get(r0, c0) * (1.0 - fc) + elev.get(r0, c1) * fc;
    let hi = elev.get(r1, c0) * (1.0 - fc) + elev.get(r1, c1) * fc;
    lo * (1.0 - fr) + hi * fr
}

/// Patch terrain: interpolated medium elevation plus smooth value noise of
/// at most [`DETAIL_AMPLITUDE_M`].
fn patch_terrain(
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    patch: &LocalPatchSpec,
    seed: &[u8; 32],
) -> Grid2<f64> {
    let size = domain.cell_size_m(DomainLevel::Medium);
    let elev = &physical.geophysics.elevation_m;
    let spec = mk_core::grid::GridSpec::new(patch.rows, patch.cols);
    let mut data = Vec::with_capacity(patch.rows * patch.cols);
    for r in 0..patch.rows {
        for c in 0..patch.cols {
            let (x, y) = patch.cell_center_m(r, c);
            let (gx, gy) = (x / DETAIL_WAVELENGTH_M, y / DETAIL_WAVELENGTH_M);
            let (ix, iy) = (gx.floor() as i64, gy.floor() as i64);
            let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
            let s = |t: f64| t * t * (3.0 - 2.0 * t);
            let v = |dx: i64, dy: i64| 2.0 * unit(seed, ix + dx, iy + dy) - 1.0;
            let lo = v(0, 0) * (1.0 - s(fx)) + v(1, 0) * s(fx);
            let hi = v(0, 1) * (1.0 - s(fx)) + v(1, 1) * s(fx);
            let detail = DETAIL_AMPLITUDE_M * (lo * (1.0 - s(fy)) + hi * s(fy));
            data.push(medium_elevation(elev, size, x, y) + detail);
        }
    }
    Grid2::from_data(&spec, data)
}

/// Buildable cells of the patch.
fn buildable_cells(
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    patch: &LocalPatchSpec,
    terrain: &Grid2<f64>,
) -> Vec<bool> {
    let medium = DomainLevel::Medium;
    let size = domain.cell_size_m(medium);
    let net = physical.flow_network();
    let (rows, cols) = (patch.rows, patch.cols);
    let mut ok = vec![false; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let h = *terrain.get(r, c);
            if h < MIN_BUILDABLE_ELEVATION_M {
                continue;
            }
            let (x, y) = patch.cell_center_m(r, c);
            let (mr, mc) = (
                ((y / size) as usize).min(domain.rows(medium) - 1),
                ((x / size) as usize).min(domain.cols(medium) - 1),
            );
            if net.lake_depth_m(mr, mc) > 0.0
                || discharge_m3_s(&physical.hydrology, domain, mr, mc) >= RIVER_MIN_DISCHARGE_M3_S
            {
                continue;
            }
            // Steepest rise to a 4-neighbour, per metre.
            let mut steepest: f64 = 0.0;
            for (dr, dc) in [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)] {
                let (nr, nc) = (r as i64 + dr, c as i64 + dc);
                if nr >= 0 && nc >= 0 && (nr as usize) < rows && (nc as usize) < cols {
                    steepest = steepest
                        .max((terrain.get(nr as usize, nc as usize) - h).abs() / patch.cell_size_m);
                }
            }
            ok[r * cols + c] = steepest <= MAX_SLOPE;
        }
    }
    ok
}

/// 2-D prefix sums of `1` for every unbuildable cell, for O(1) rectangle
/// queries.
struct Unbuildable {
    sums: Vec<u32>,
    rows: usize,
    cols: usize,
}

impl Unbuildable {
    fn new(ok: &[bool], rows: usize, cols: usize) -> Self {
        let w = cols + 1;
        let mut sums = vec![0u32; (rows + 1) * w];
        for r in 0..rows {
            for c in 0..cols {
                sums[(r + 1) * w + c + 1] =
                    u32::from(!ok[r * cols + c]) + sums[r * w + c + 1] + sums[(r + 1) * w + c]
                        - sums[r * w + c];
            }
        }
        Self { sums, rows, cols }
    }
    /// Unbuildable cells in cell rows `r0..r1`, columns `c0..c1`.
    fn count(&self, r0: usize, c0: usize, r1: usize, c1: usize) -> u32 {
        let w = self.cols + 1;
        self.sums[r1 * w + c1] + self.sums[r0 * w + c0]
            - self.sums[r0 * w + c1]
            - self.sums[r1 * w + c0]
    }
    fn rect_is_clear(&self, patch: &LocalPatchSpec, rect: &Rect) -> bool {
        let to_cell = |m: f64, origin: f64, n: usize, floor: bool| {
            let v = (m - origin) / patch.cell_size_m;
            let v = if floor { v.floor() } else { v.ceil() };
            if v < 0.0 || v as usize > n {
                None
            } else {
                Some(v as usize)
            }
        };
        let (Some(c0), Some(c1), Some(r0), Some(r1)) = (
            to_cell(rect.x0, patch.origin_x_m, self.cols, true),
            to_cell(rect.x1, patch.origin_x_m, self.cols, false),
            to_cell(rect.y0, patch.origin_y_m, self.rows, true),
            to_cell(rect.y1, patch.origin_y_m, self.rows, false),
        ) else {
            return false;
        };
        self.count(r0, c0, r1, c1) == 0
    }
}

/// A door in a building plan: the spaces it joins (`None` is outdoors, an
/// index is into the plan's spaces) and its position.
type PlanDoor = (Option<usize>, Option<usize>, (f64, f64));

/// A building's plan, relative to its own south-west corner.
struct Plan {
    building_id: u64,
    kind: PropertyBuildingKind,
    width: f64,
    depth: f64,
    spaces: Vec<(String, u64, SpaceKind, Rect)>,
    doors: Vec<PlanDoor>,
}

/// Distinct sub-grouping labels of a building's items in first-appearance
/// (item id) order.
fn labels_of(property: &StarterProperty, building_id: u64) -> Vec<String> {
    let mut seen = Vec::new();
    let mut items: Vec<_> = property
        .items
        .iter()
        .filter(|i| i.location == building_id)
        .collect();
    items.sort_by_key(|i| i.id);
    for item in items {
        if let Some(room) = &item.room {
            if !seen.contains(room) {
                seen.push(room.clone());
            }
        }
    }
    seen
}

fn items_in(property: &StarterProperty, building_id: u64) -> usize {
    property
        .items
        .iter()
        .filter(|i| i.location == building_id)
        .count()
}

fn house_plan(
    property: &StarterProperty,
    house_id: u64,
    computer_room_id: u64,
) -> Result<Plan, EstateLayoutError> {
    let labels: Vec<String> = labels_of(property, house_id)
        .into_iter()
        .filter(|l| l != "General")
        .collect();
    // Room entries: (label, owning upstream building id).
    let mut rooms: Vec<(String, u64)> = labels.into_iter().map(|l| (l, house_id)).collect();
    rooms.push(("Computer Room".to_string(), computer_room_id));
    let per_row = rooms.len().div_ceil(2);
    let width = per_row as f64 * ROOM_WIDTH_M;
    let depth = 2.0 * ROOM_DEPTH_M + HALL_DEPTH_M;
    let mut spaces = vec![(
        "General".to_string(),
        house_id,
        SpaceKind::Zone,
        Rect::new(0.0, ROOM_DEPTH_M, width, HALL_DEPTH_M),
    )];
    let mut doors = vec![(None, Some(0), (0.0, ROOM_DEPTH_M + 0.5 * HALL_DEPTH_M))];
    for (i, (label, owner)) in rooms.into_iter().enumerate() {
        let (slot, north) = (i % per_row, i >= per_row);
        let x0 = slot as f64 * ROOM_WIDTH_M;
        let y0 = if north {
            ROOM_DEPTH_M + HALL_DEPTH_M
        } else {
            0.0
        };
        spaces.push((
            label,
            owner,
            SpaceKind::Room,
            Rect::new(x0, y0, ROOM_WIDTH_M, ROOM_DEPTH_M),
        ));
        let door_y = if north { y0 } else { y0 + ROOM_DEPTH_M };
        doors.push((
            Some(spaces.len() - 1),
            Some(0),
            (x0 + 0.5 * ROOM_WIDTH_M, door_y),
        ));
    }
    Ok(Plan {
        building_id: house_id,
        kind: PropertyBuildingKind::House,
        width,
        depth,
        spaces,
        doors,
    })
}

fn shed_plan(property: &StarterProperty, shed_id: u64) -> Plan {
    let mut labels = labels_of(property, shed_id);
    if labels.is_empty() {
        labels.push("General".to_string());
    }
    let mut spaces = Vec::new();
    let mut doors = Vec::new();
    let mut x = 0.0;
    for (i, label) in labels.iter().enumerate() {
        let count = property
            .items
            .iter()
            .filter(|it| it.location == shed_id && it.room.as_deref() == Some(label.as_str()))
            .count();
        let w = ZONE_MIN_WIDTH_M.max(count as f64);
        spaces.push((
            label.clone(),
            shed_id,
            SpaceKind::Zone,
            Rect::new(x, 0.0, w, SHED_DEPTH_M),
        ));
        if i == 0 {
            doors.push((None, Some(0), (x + 0.5 * w, 0.0)));
        } else {
            doors.push((Some(i - 1), Some(i), (x, 0.5 * SHED_DEPTH_M)));
        }
        x += w;
    }
    Plan {
        building_id: shed_id,
        kind: PropertyBuildingKind::Shed,
        width: x,
        depth: SHED_DEPTH_M,
        spaces,
        doors,
    }
}

/// A single-space building sized from its item count: `per_item_m2` of floor
/// each, at least `min_m2`, at a 1.6 : 1 aspect.
fn whole_plan(
    property: &StarterProperty,
    building: &crate::organisms::property::PropertyBuilding,
    per_item_m2: f64,
    min_m2: f64,
) -> Plan {
    let area = min_m2.max(per_item_m2 * items_in(property, building.id) as f64);
    let depth = (area / 1.6).sqrt().ceil();
    let width = (area / depth).ceil();
    Plan {
        building_id: building.id,
        kind: building.kind,
        width,
        depth,
        spaces: vec![(
            building.name.clone(),
            building.id,
            SpaceKind::WholeBuilding,
            Rect::new(0.0, 0.0, width, depth),
        )],
        doors: vec![(None, Some(0), (0.5 * width, 0.0))],
    }
}

/// Lay out `property` on `patch`: choose the site, place the five
/// buildings, subdivide them, hang the doors and place every item.
pub fn layout_estate(
    property: &StarterProperty,
    physical: &RegionalPhysicalState,
    domain: &IslandDomain,
    patch: &LocalPatchSpec,
    seed: [u8; 32],
) -> Result<EstateLayout, EstateLayoutError> {
    use PropertyBuildingKind as K;
    let find = |kind: K| {
        property
            .buildings
            .iter()
            .find(|b| b.kind == kind)
            .ok_or_else(|| EstateLayoutError::InvalidProperty(format!("no {kind:?} building")))
    };
    let (house, shed, workshop, armoury, garage, computer) = (
        find(K::House)?,
        find(K::Shed)?,
        find(K::Workshop)?,
        find(K::Armoury)?,
        find(K::Garage)?,
        find(K::ComputerRoom)?,
    );

    let plans = [
        house_plan(property, house.id, computer.id)?,
        shed_plan(property, shed.id),
        whole_plan(property, workshop, 3.0, 60.0),
        whole_plan(property, armoury, 2.0, 30.0),
        whole_plan(property, garage, 14.0, 120.0),
    ];

    // Cluster: House, Shed, Workshop in a row; Armoury, Garage in a second
    // row to the south, all [`BUILDING_GAP_M`] apart. Offsets are from the
    // cluster's south-west corner.
    let row2_h = plans[3].depth.max(plans[4].depth);
    let row1_h = plans[0].depth.max(plans[1].depth).max(plans[2].depth);
    let mut offsets = [(0.0, 0.0); 5];
    let mut x = 0.0;
    for i in 0..3 {
        offsets[i] = (x, row2_h + BUILDING_GAP_M);
        x += plans[i].width + BUILDING_GAP_M;
    }
    let row1_w = x - BUILDING_GAP_M;
    let mut x = 0.0;
    for i in 3..5 {
        offsets[i] = (x, 0.0);
        x += plans[i].width + BUILDING_GAP_M;
    }
    let row2_w = x - BUILDING_GAP_M;
    let (cluster_w, cluster_h) = (row1_w.max(row2_w), row1_h + BUILDING_GAP_M + row2_h);

    // Site: the buildable position nearest the patch centre.
    let terrain = patch_terrain(physical, domain, patch, &seed);
    let ok = buildable_cells(physical, domain, patch, &terrain);
    let unbuildable = Unbuildable::new(&ok, patch.rows, patch.cols);
    let (cx, cy) = (
        patch.origin_x_m + 0.5 * patch.width_m,
        patch.origin_y_m + 0.5 * patch.height_m,
    );
    let rect_at = |ox: f64, oy: f64, i: usize| {
        Rect::new(
            ox + offsets[i].0,
            oy + offsets[i].1,
            plans[i].width,
            plans[i].depth,
        )
    };
    let mut best: Option<(f64, f64, f64)> = None;
    for r in 0..patch.rows {
        let oy = patch.origin_y_m + r as f64 * patch.cell_size_m;
        for c in 0..patch.cols {
            let ox = patch.origin_x_m + c as f64 * patch.cell_size_m;
            let distance = (ox + 0.5 * cluster_w - cx).hypot(oy + 0.5 * cluster_h - cy);
            if best.is_some_and(|(d, _, _)| distance >= d) {
                continue;
            }
            if (0..5).all(|i| {
                unbuildable.rect_is_clear(patch, &rect_at(ox, oy, i).expanded(SITE_MARGIN_M))
            }) {
                best = Some((distance, ox, oy));
            }
        }
    }
    let (_, ox, oy) = best.ok_or(EstateLayoutError::NoBuildableSite)?;

    // Footprints, spaces and doors in world metres.
    let mut buildings = Vec::new();
    let mut spaces = Vec::new();
    let mut doors = Vec::new();
    for (i, plan) in plans.iter().enumerate() {
        let (dx, dy) = (ox + offsets[i].0, oy + offsets[i].1);
        buildings.push(BuildingFootprint {
            building_id: plan.building_id,
            kind: plan.kind,
            rect_m: Rect::new(dx, dy, plan.width, plan.depth),
            rotation_deg: 0.0,
        });
        let first = spaces.len() as u32;
        for (j, (label, owner, kind, rect)) in plan.spaces.iter().enumerate() {
            spaces.push(SpaceLayout {
                id: SpaceId(first + j as u32),
                building_id: *owner,
                label: label.clone(),
                kind: *kind,
                rect_m: rect.translated(dx, dy),
            });
        }
        let id = |k: Option<usize>| {
            k.map_or(Space::Outdoors, |j| {
                Space::Inside(SpaceId(first + j as u32))
            })
        };
        for (a, b, (px, py)) in &plan.doors {
            doors.push(Door {
                a: id(*a),
                b: id(*b),
                position_m: (dx + px, dy + py),
            });
        }
    }

    // Items: each into the space its building and label map to, on a
    // lattice inside the space's rectangle, in item-id order.
    let building_kind = |id: u64| {
        property
            .buildings
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.kind)
    };
    let space_of = |item: &crate::organisms::property::PropertyItem| -> Option<SpaceId> {
        let kind = building_kind(item.location)?;
        let label = item.room.as_deref();
        let find_space = |building: u64, label: &str| {
            spaces
                .iter()
                .find(|s| s.building_id == building && s.label == label)
                .map(|s| s.id)
        };
        match kind {
            K::House => label
                .and_then(|l| find_space(house.id, l))
                .or_else(|| find_space(house.id, "General")),
            K::Shed => label.and_then(|l| find_space(shed.id, l)).or_else(|| {
                spaces
                    .iter()
                    .find(|s| s.building_id == shed.id)
                    .map(|s| s.id)
            }),
            K::ComputerRoom => find_space(computer.id, "Computer Room"),
            K::Workshop | K::Armoury | K::Garage => spaces
                .iter()
                .find(|s| s.building_id == item.location)
                .map(|s| s.id),
        }
    };
    let mut by_space: BTreeMap<Option<SpaceId>, Vec<u64>> = BTreeMap::new();
    let mut ordered: Vec<_> = property.items.iter().collect();
    ordered.sort_by_key(|i| i.id);
    for item in &ordered {
        by_space.entry(space_of(item)).or_default().push(item.id);
    }
    let mut items = Vec::new();
    for (space, ids) in by_space {
        let rect = match space {
            Some(id) => spaces.iter().find(|s| s.id == id).map(|s| s.rect_m),
            None => None,
        };
        // Items with no mapped building stand in the yard.
        let yard_rect = Rect::new(ox, oy, cluster_w, cluster_h).expanded(YARD_MARGIN_M);
        let area = rect.unwrap_or(yard_rect);
        let inset = Rect {
            x0: area.x0 + 0.25,
            y0: area.y0 + 0.25,
            x1: area.x1 - 0.25,
            y1: area.y1 - 0.25,
        };
        let n = ids.len();
        let cols = ((n as f64 * inset.width() / inset.height()).sqrt().ceil() as usize).clamp(1, n);
        let rows = n.div_ceil(cols);
        for (k, item_id) in ids.into_iter().enumerate() {
            let (r, c) = (k / cols, k % cols);
            items.push(ItemPlacement {
                property_id: property.id,
                item_id,
                space: space.map_or(Space::Outdoors, Space::Inside),
                position_m: (
                    inset.x0 + (c as f64 + 0.5) / cols as f64 * inset.width(),
                    inset.y0 + (r as f64 + 0.5) / rows as f64 * inset.height(),
                ),
            });
        }
    }
    items.sort_by_key(|p| p.item_id);

    let yard = Rect::new(ox, oy, cluster_w, cluster_h).expanded(YARD_MARGIN_M);
    Ok(EstateLayout {
        property_id: property.id,
        patch: patch.clone(),
        terrain_m: terrain,
        buildings,
        spaces,
        doors,
        items,
        yard,
    })
}
