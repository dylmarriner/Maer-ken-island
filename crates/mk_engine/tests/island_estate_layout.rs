//! Phase 3 Task 5: the founders' estate laid out in metres on the island.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_engine::organisms::property::{PropertyBuildingKind, StarterProperty};
use mk_engine::regional::estate_layout::{
    layout_estate, EstateLayout, EstateLayoutError, Space, SpaceKind, MAX_SLOPE,
    MIN_BUILDABLE_ELEVATION_M,
};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile, LocalPatchSpec};

const M: DomainLevel = DomainLevel::Medium;
const SEED: [u8; 32] = [5u8; 32];

struct Base {
    domain: IslandDomain,
    physical: RegionalPhysicalState,
    property: StarterProperty,
    patch: LocalPatchSpec,
    layout: EstateLayout,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// A 4 km patch (5 m cells) centred on `(x, y)` metres.
fn patch_at(domain: &IslandDomain, x: f64, y: f64) -> Option<LocalPatchSpec> {
    domain.local_patch(x, y, 4_000.0, 5.0).ok()
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().unwrap();
        let domain = IslandDomain::from_profile(profile).unwrap();
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, seed).unwrap();
        let property = StarterProperty::founders_estate((0, 0));
        // The first gentle lowland site (raster order) the layout accepts.
        let elev = &physical.geophysics.elevation_m;
        let (rows, cols) = (domain.rows(M), domain.cols(M));
        let mut found = None;
        'scan: for r in (3..rows - 3).step_by(5) {
            for c in (3..cols - 3).step_by(5) {
                let h = *elev.get(r, c);
                if !(5.0..150.0).contains(&h) {
                    continue;
                }
                let (x, y) = domain.cell_center_m(M, r, c);
                let Some(patch) = patch_at(&domain, x, y) else {
                    continue;
                };
                if let Ok(layout) = layout_estate(&property, &physical, &domain, &patch, SEED) {
                    found = Some((patch, layout));
                    break 'scan;
                }
            }
        }
        let (patch, layout) = found.expect("the island has a buildable estate site");
        Base {
            domain,
            physical,
            property,
            patch,
            layout,
        }
    })
}

#[test]
fn every_item_is_placed_once_in_the_space_its_label_maps_to() {
    let b = base();
    let l = &b.layout;
    assert_eq!(l.property_id, b.property.id);
    assert_eq!(l.items.len(), b.property.items.len());
    let space_label = |s: Space| match s {
        Space::Inside(id) => l.spaces.iter().find(|x| x.id == id).unwrap(),
        Space::Outdoors => panic!("an estate item was left outdoors"),
    };
    for item in &b.property.items {
        let placed: Vec<_> = l.items.iter().filter(|p| p.item_id == item.id).collect();
        assert_eq!(
            placed.len(),
            1,
            "item {} placed {} times",
            item.id,
            placed.len()
        );
        let p = placed[0];
        let sp = space_label(p.space);
        let building = b
            .property
            .buildings
            .iter()
            .find(|x| x.id == item.location)
            .unwrap();
        match building.kind {
            PropertyBuildingKind::House | PropertyBuildingKind::Shed => {
                if let Some(room) = &item.room {
                    assert_eq!(&sp.label, room, "{} ({})", item.name, item.id);
                }
                assert_eq!(sp.building_id, building.id);
            }
            PropertyBuildingKind::ComputerRoom => assert_eq!(sp.label, "Computer Room"),
            _ => {
                assert_eq!(sp.kind, SpaceKind::WholeBuilding);
                assert_eq!(sp.building_id, building.id);
            }
        }
        // `space_at` round-trips the placement.
        assert_eq!(l.space_at(p.position_m), p.space, "item {}", item.id);
    }
}

#[test]
fn five_footprints_the_computer_room_inside_the_house_reached_only_through_it() {
    let b = base();
    let l = &b.layout;
    assert_eq!(l.buildings.len(), 5);
    let kinds: Vec<_> = l.buildings.iter().map(|f| f.kind).collect();
    for k in [
        PropertyBuildingKind::House,
        PropertyBuildingKind::Shed,
        PropertyBuildingKind::Workshop,
        PropertyBuildingKind::Armoury,
        PropertyBuildingKind::Garage,
    ] {
        assert!(kinds.contains(&k), "{k:?} footprint missing");
    }
    assert!(!kinds.contains(&PropertyBuildingKind::ComputerRoom));
    for (i, a) in l.buildings.iter().enumerate() {
        for c in &l.buildings[i + 1..] {
            assert!(
                !a.rect_m.intersects(&c.rect_m),
                "{:?} overlaps {:?}",
                a.kind,
                c.kind
            );
        }
    }

    let house = l
        .buildings
        .iter()
        .find(|f| f.kind == PropertyBuildingKind::House)
        .unwrap();
    let computer = l
        .spaces
        .iter()
        .find(|s| s.label == "Computer Room")
        .unwrap();
    assert!(house.rect_m.contains_rect(&computer.rect_m));
    let computer_space = Space::Inside(computer.id);
    // Its only door opens into the House hall; none to the outdoors.
    let doors: Vec<_> = l
        .doors
        .iter()
        .filter(|d| d.a == computer_space || d.b == computer_space)
        .collect();
    assert_eq!(doors.len(), 1);
    let other = if doors[0].a == computer_space {
        doors[0].b
    } else {
        doors[0].a
    };
    let hall = l
        .spaces
        .iter()
        .find(|s| s.building_id == house.building_id && s.label == "General")
        .unwrap();
    assert_eq!(other, Space::Inside(hall.id));
    // Reaching it from outdoors goes through the House.
    let path = l.route(Space::Outdoors, computer_space).expect("reachable");
    assert_eq!(path.first(), Some(&Space::Outdoors));
    assert!(
        path.contains(&Space::Inside(hall.id)) && path.len() == 3,
        "{path:?}"
    );
}

#[test]
fn every_space_is_routable_footprints_stand_on_buildable_ground_and_nothing_overlaps() {
    let b = base();
    let l = &b.layout;
    for s in &l.spaces {
        assert!(
            l.route(Space::Outdoors, Space::Inside(s.id)).is_some(),
            "{} not reachable",
            s.label
        );
    }
    // Spaces do not overlap, and each lies inside its building's footprint.
    for (i, a) in l.spaces.iter().enumerate() {
        for c in &l.spaces[i + 1..] {
            assert!(
                !a.rect_m.intersects(&c.rect_m),
                "{} overlaps {}",
                a.label,
                c.label
            );
        }
        assert!(
            l.buildings
                .iter()
                .any(|f| f.rect_m.contains_rect(&a.rect_m)),
            "{}",
            a.label
        );
    }
    // Footprints are dry land under the slope limit, on the patch terrain.
    let patch = &l.patch;
    for f in &l.buildings {
        for r in 0..patch.rows {
            for c in 0..patch.cols {
                let (x, y) = patch.cell_center_m(r, c);
                if !f.rect_m.contains(x, y) {
                    continue;
                }
                let h = *l.terrain_m.get(r, c);
                assert!(h >= MIN_BUILDABLE_ELEVATION_M, "{:?} on {h} m", f.kind);
                for (dr, dc) in [(0i64, 1i64), (1, 0)] {
                    let (nr, nc) = (r as i64 + dr, c as i64 + dc);
                    if (nr as usize) < patch.rows && (nc as usize) < patch.cols {
                        let slope = (l.terrain_m.get(nr as usize, nc as usize) - h).abs()
                            / patch.cell_size_m;
                        assert!(slope <= MAX_SLOPE + 1e-12, "{:?} slope {slope}", f.kind);
                    }
                }
            }
        }
    }
    // Patch terrain stays within the detail bound of the regional field.
    let elev = &b.physical.geophysics.elevation_m;
    let size = b.domain.cell_size_m(M);
    let (mr, mc) = (
        ((patch.origin_y_m + 0.5 * patch.height_m) / size) as usize,
        ((patch.origin_x_m + 0.5 * patch.width_m) / size) as usize,
    );
    let regional = *elev.get(mr, mc);
    let centre = *l.terrain_m.get(patch.rows / 2, patch.cols / 2);
    assert!(
        (centre - regional).abs() < 60.0,
        "patch {centre} vs regional {regional}"
    );
}

#[test]
fn layout_is_deterministic_and_an_unbuildable_patch_is_an_error() {
    let b = base();
    let again = layout_estate(&b.property, &b.physical, &b.domain, &b.patch, SEED).unwrap();
    assert_eq!(
        serde_json::to_string(&again).unwrap(),
        serde_json::to_string(&b.layout).unwrap()
    );
    // A different seed changes only the terrain detail, not the plan's
    // shape (same number of buildings, spaces, doors, items).
    let other = layout_estate(&b.property, &b.physical, &b.domain, &b.patch, [6u8; 32]).unwrap();
    assert_eq!(other.spaces.len(), b.layout.spaces.len());
    assert_eq!(other.items.len(), b.layout.items.len());

    // A patch in open sea has nowhere to build.
    let elev = &b.physical.geophysics.elevation_m;
    let (rows, cols) = (b.domain.rows(M), b.domain.cols(M));
    let sea = (0..rows)
        .step_by(7)
        .flat_map(|r| (0..cols).step_by(7).map(move |c| (r, c)))
        .find_map(|(r, c)| {
            let (x, y) = b.domain.cell_center_m(M, r, c);
            (*elev.get(r, c) < -200.0)
                .then(|| patch_at(&b.domain, x, y))
                .flatten()
        })
        .expect("open sea inside the domain");
    assert_eq!(
        layout_estate(&b.property, &b.physical, &b.domain, &sea, SEED).unwrap_err(),
        EstateLayoutError::NoBuildableSite
    );

    // A property without a House is rejected, not invented.
    let mut broken = b.property.clone();
    broken
        .buildings
        .retain(|x| x.kind != PropertyBuildingKind::House);
    assert!(matches!(
        layout_estate(&broken, &b.physical, &b.domain, &b.patch, SEED),
        Err(EstateLayoutError::InvalidProperty(_))
    ));
}

#[test]
fn a_river_takes_its_channel_and_esplanade_not_its_whole_cell() {
    use mk_engine::regional::ecology::river_width_m;
    use mk_engine::regional::estate_layout::ESPLANADE_RESERVE_M;
    use mk_engine::regional::hydrology::{discharge_m3_s, RIVER_MIN_DISCHARGE_M3_S};
    let b = base();
    let size = b.domain.cell_size_m(M);
    let medium_of = |patch: &LocalPatchSpec, r: usize, c: usize| {
        let (x, y) = patch.cell_center_m(r, c);
        ((y / size) as usize, (x / size) as usize)
    };
    // A lowland patch with a river through it that the estate still fits
    // on. Before the river kept to its channel, the whole 2 km cell a
    // 1 m^3/s stream crossed was unbuildable.
    let elev = &b.physical.geophysics.elevation_m;
    let (rows, cols) = (b.domain.rows(M), b.domain.cols(M));
    let mut checked = 0;
    for (r, c) in (3..rows - 3)
        .step_by(4)
        .flat_map(|r| (3..cols - 3).step_by(4).map(move |c| (r, c)))
    {
        let h = *elev.get(r, c);
        let q = discharge_m3_s(&b.physical.hydrology, &b.domain, r, c);
        if !(5.0..300.0).contains(&h) || q < RIVER_MIN_DISCHARGE_M3_S {
            continue;
        }
        let (x, y) = b.domain.cell_center_m(M, r, c);
        let Some(patch) = patch_at(&b.domain, x, y) else {
            continue;
        };
        let Ok(layout) = layout_estate(&b.property, &b.physical, &b.domain, &patch, SEED) else {
            continue;
        };
        // No footprint stands in the river's strip: within each river cell,
        // footprints sit above the lowest (width + 2 x 20 m) / 2 km share
        // of the patch cells, where the channel runs.
        let mut by_cell: std::collections::BTreeMap<(usize, usize), Vec<f64>> =
            std::collections::BTreeMap::new();
        for pr in 0..patch.rows {
            for pc in 0..patch.cols {
                by_cell
                    .entry(medium_of(&patch, pr, pc))
                    .or_default()
                    .push(*layout.terrain_m.get(pr, pc));
            }
        }
        for f in &layout.buildings {
            for pr in 0..patch.rows {
                for pc in 0..patch.cols {
                    let (px, py) = patch.cell_center_m(pr, pc);
                    if !f.rect_m.contains(px, py) {
                        continue;
                    }
                    let (mr, mc) = medium_of(&patch, pr, pc);
                    let q = discharge_m3_s(&b.physical.hydrology, &b.domain, mr, mc);
                    if q < RIVER_MIN_DISCHARGE_M3_S {
                        continue;
                    }
                    let mut heights = by_cell[&(mr, mc)].clone();
                    heights.sort_by(f64::total_cmp);
                    let share = ((river_width_m(q) + 2.0 * ESPLANADE_RESERVE_M) / size).min(1.0);
                    let taken = (heights.len() as f64 * share).ceil() as usize;
                    let channel_top = heights[taken.saturating_sub(1).min(heights.len() - 1)];
                    assert!(
                        *layout.terrain_m.get(pr, pc) >= channel_top,
                        "{:?} stands in a river's strip",
                        f.kind
                    );
                }
            }
        }
        checked += 1;
        if checked >= 3 {
            break;
        }
    }
    assert!(
        checked > 0,
        "no river-crossed lowland patch could take the estate"
    );
}
