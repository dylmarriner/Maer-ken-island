//! Phase 1 Tasks 1 and 1b: the regional profile and coordinate contract.

use std::path::PathBuf;

use mk_island::{DomainLevel, IslandDomain, IslandDomainError, IslandProfile, IslandProfileError};

const EPS: f64 = 1e-9;

fn domain(profile: IslandProfile) -> IslandDomain {
    IslandDomain::from_profile(profile).expect("valid profile")
}

#[test]
fn default_profile_has_the_planned_values_and_grids() {
    let p = IslandProfile::default_nz_scale();
    assert_eq!(p.version, 1);
    assert_eq!((p.width_m, p.height_m), (2_400_000.0, 1_920_000.0));
    assert_eq!((p.coarse_cell_m, p.medium_cell_m), (12_000.0, 2_000.0));
    assert_eq!(
        (p.reference_latitude_deg, p.reference_longitude_deg),
        (-41.0, 174.0)
    );
    assert_eq!(p.target_land_area_m2, 268_000.0e6);
    assert_eq!(p.minimum_ocean_buffer_m, 300_000.0);
    let (lo, hi) = p.land_area_bounds_m2();
    assert!((lo - 254_600.0e6).abs() < 1.0 && (hi - 281_400.0e6).abs() < 1.0);
    assert!(p.geology.ancient_basement);

    let d = domain(p);
    assert_eq!(
        (d.rows(DomainLevel::Coarse), d.cols(DomainLevel::Coarse)),
        (160, 200)
    );
    assert_eq!(
        (d.rows(DomainLevel::Medium), d.cols(DomainLevel::Medium)),
        (960, 1_200)
    );
    let spec = d.storage_spec(DomainLevel::Medium);
    assert_eq!((spec.nlat, spec.nlon), (960, 1_200));
    assert_eq!(d.cell_area_m2(DomainLevel::Coarse), 144.0e6);
    assert_eq!(d.cell_area_m2(DomainLevel::Medium), 4.0e6);
    assert_eq!(d.active_cells(DomainLevel::Medium).len(), 960 * 1_200);
}

#[test]
fn fallback_and_small_test_profiles_have_their_grids() {
    let f = domain(IslandProfile::fallback_nz_scale());
    assert_eq!(
        (f.rows(DomainLevel::Coarse), f.cols(DomainLevel::Coarse)),
        (80, 100)
    );
    assert_eq!(
        (f.rows(DomainLevel::Medium), f.cols(DomainLevel::Medium)),
        (480, 600)
    );

    let s = domain(IslandProfile::test_small());
    assert_eq!(
        (s.rows(DomainLevel::Coarse), s.cols(DomainLevel::Coarse)),
        (32, 40)
    );
    assert_eq!(
        (s.rows(DomainLevel::Medium), s.cols(DomainLevel::Medium)),
        (192, 240)
    );
    assert_eq!(s.profile().minimum_ocean_buffer_m, 60_000.0);
    assert_eq!(s.profile().target_land_area_m2, 8_000.0e6);
}

#[test]
fn the_small_profile_fixture_is_test_small() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/island/test_small_profile.json");
    assert_eq!(
        IslandProfile::load(&path).expect("fixture loads"),
        IslandProfile::test_small()
    );
}

#[test]
fn row_zero_is_south_and_the_centre_is_the_reference_point() {
    let d = domain(IslandProfile::default_nz_scale());
    for level in [DomainLevel::Coarse, DomainLevel::Medium] {
        let south = d.latitude_rad_for_row(level, 0);
        let north = d.latitude_rad_for_row(level, d.rows(level) - 1);
        assert!(south < north);
        let west = d.longitude_rad_for_col(level, 0);
        let east = d.longitude_rad_for_col(level, d.cols(level) - 1);
        assert!(west < east);
    }
    let (lat, lon) = d.lat_lon_at_m(1_200_000.0, 960_000.0);
    assert!((lat - (-41.0f64).to_radians()).abs() < EPS);
    assert!((lon - 174.0f64.to_radians()).abs() < EPS);
    // Latitude is linear in metres: one radius-metre per radian.
    let (lat_n, _) = d.lat_lon_at_m(1_200_000.0, 960_000.0 + 19_113.0);
    assert!((lat_n - lat - 1.0e-3).abs() < 1e-12);
}

#[test]
fn coordinates_round_trip_through_cells() {
    let d = domain(IslandProfile::default_nz_scale());
    for level in [DomainLevel::Coarse, DomainLevel::Medium] {
        for (row, col) in [(0, 0), (17, 3), (d.rows(level) - 1, d.cols(level) - 1)] {
            let (x, y) = d.cell_center_m(level, row, col);
            assert_eq!(d.cell_containing_m(level, x, y), Some((row, col)));
        }
    }
    assert_eq!(d.cell_containing_m(DomainLevel::Medium, -1.0, 5.0), None);
    assert_eq!(
        d.cell_containing_m(DomainLevel::Medium, 2_400_000.0, 5.0),
        None
    );
    assert_eq!(
        d.cell_containing_m(DomainLevel::Medium, f64::NAN, 5.0),
        None
    );
}

#[test]
fn edge_buffer_band_is_exactly_the_buffer() {
    let d = domain(IslandProfile::default_nz_scale());
    // 300 km / 12 km = 25 coarse cells on every side.
    assert!(d.is_edge_buffer_cell(DomainLevel::Coarse, 80, 24));
    assert!(!d.is_edge_buffer_cell(DomainLevel::Coarse, 80, 25));
    assert!(!d.is_edge_buffer_cell(DomainLevel::Coarse, 80, 174));
    assert!(d.is_edge_buffer_cell(DomainLevel::Coarse, 80, 175));
    assert!(d.is_edge_buffer_cell(DomainLevel::Coarse, 24, 100));
    assert!(!d.is_edge_buffer_cell(DomainLevel::Coarse, 25, 100));
    assert!(d.is_edge_buffer_cell(DomainLevel::Coarse, 135, 100));
    // 300 km / 2 km = 150 medium cells.
    assert!(d.is_edge_buffer_cell(DomainLevel::Medium, 480, 149));
    assert!(!d.is_edge_buffer_cell(DomainLevel::Medium, 480, 150));
}

#[test]
fn active_cells_can_be_narrowed_but_not_out_of_range() {
    let mut d = domain(IslandProfile::test_small());
    d.set_active_cells(DomainLevel::Medium, vec![(5, 5), (1, 2), (5, 5)])
        .unwrap();
    assert_eq!(d.active_cells(DomainLevel::Medium), &[(1, 2), (5, 5)]);
    assert!(matches!(
        d.set_active_cells(DomainLevel::Medium, vec![(192, 0)]),
        Err(IslandDomainError::CellOutOfRange { .. })
    ));
}

#[test]
fn local_patches_are_bounded_and_inside_the_domain() {
    let d = domain(IslandProfile::default_nz_scale());
    let estate = d.local_patch(1_000_000.0, 900_000.0, 4_000.0, 5.0).unwrap();
    assert_eq!((estate.rows, estate.cols), (800, 800));
    assert_eq!(
        (estate.origin_x_m, estate.origin_y_m),
        (998_000.0, 898_000.0)
    );
    assert_eq!(estate.cell_center_m(0, 0), (998_002.5, 898_002.5));

    for (cx, cy, extent, cell) in [
        (1_000.0, 900_000.0, 4_000.0, 5.0),      // crosses the west edge
        (1_000_000.0, 900_000.0, 4_000.0, 3.0),  // not a whole number of cells
        (1_000_000.0, 900_000.0, -4_000.0, 5.0), // negative extent
        (1_000_000.0, 900_000.0, 4_000.0, 0.0),  // zero cell
        (1_000_000.0, 900_000.0, 40_000.0, 5.0), // too many cells
        (f64::NAN, 900_000.0, 4_000.0, 5.0),
    ] {
        assert!(
            matches!(
                d.local_patch(cx, cy, extent, cell),
                Err(IslandDomainError::InvalidPatch(_))
            ),
            "{cx} {cy} {extent} {cell}"
        );
    }
}

#[test]
fn profiles_round_trip_through_json() {
    let p = IslandProfile::default_nz_scale();
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(serde_json::from_str::<IslandProfile>(&json).unwrap(), p);
}

#[test]
fn unknown_versions_are_rejected_on_load() {
    let dir = std::env::temp_dir().join(format!("mk_island_version_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.json");
    let mut value = serde_json::to_value(IslandProfile::default_nz_scale()).unwrap();
    value["version"] = 2.into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(
        IslandProfile::load(&path),
        Err(IslandProfileError::UnsupportedVersion(2))
    ));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn invalid_profiles_fail_before_allocation() {
    let base = IslandProfile::default_nz_scale;
    let cases: Vec<(&str, IslandProfile)> = vec![
        (
            "zero width",
            IslandProfile {
                width_m: 0.0,
                ..base()
            },
        ),
        (
            "negative height",
            IslandProfile {
                height_m: -1.0,
                ..base()
            },
        ),
        (
            "nan cell",
            IslandProfile {
                medium_cell_m: f64::NAN,
                ..base()
            },
        ),
        (
            "coarse not dividing",
            IslandProfile {
                coarse_cell_m: 7_000.0,
                ..base()
            },
        ),
        (
            "medium not dividing coarse",
            IslandProfile {
                medium_cell_m: 5_000.0,
                ..base()
            },
        ),
        (
            "zero land",
            IslandProfile {
                target_land_area_m2: 0.0,
                ..base()
            },
        ),
        (
            "land does not fit",
            IslandProfile {
                target_land_area_m2: 3.0e12,
                ..base()
            },
        ),
        (
            "zero buffer",
            IslandProfile {
                minimum_ocean_buffer_m: 0.0,
                ..base()
            },
        ),
        (
            "buffer eats domain",
            IslandProfile {
                minimum_ocean_buffer_m: 1_000_000.0,
                ..base()
            },
        ),
        (
            "tolerance",
            IslandProfile {
                land_area_tolerance_fraction: 0.0,
                ..base()
            },
        ),
        (
            "latitude",
            IslandProfile {
                reference_latitude_deg: 95.0,
                ..base()
            },
        ),
        (
            "reaches a pole",
            IslandProfile {
                planet_radius_m: 1.0e6,
                ..base()
            },
        ),
        (
            "radius",
            IslandProfile {
                planet_radius_m: 0.0,
                ..base()
            },
        ),
        (
            "version",
            IslandProfile {
                version: 9,
                ..base()
            },
        ),
    ];
    for (name, profile) in cases {
        assert!(profile.validate().is_err(), "{name} should be rejected");
        assert!(IslandDomain::from_profile(profile).is_err(), "{name}");
    }
    let mut shape = base();
    shape.shape.max_convexity = 1.5;
    assert!(shape.validate().is_err());
    let mut geology = base();
    geology.geology.basement_area_fraction = 0.9;
    assert!(geology.validate().is_err());
}
