//! Phase 2 Task 4: regional ocean and tides.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_core::grid::{Grid2, GridSpec};
use mk_engine::climate::{ClimateForcing, ClimateState};
use mk_engine::ocean::{OceanColumn, OceanForcing, OceanState};
use mk_engine::regional::boundary::sample_regional_boundaries_with_background;
use mk_engine::regional::climate::{regional_coriolis, step_regional_climate};
use mk_engine::regional::edge::EDGE_RELAX_CELLS;
use mk_engine::regional::ocean::step_regional_ocean;
use mk_engine::regional::tides::step_regional_tides;
use mk_engine::regional::weather::step_regional_weather;
use mk_engine::regional::zonal::{ZonalBackgroundState, DEFAULT_BAND_COUNT};
use mk_island::{DomainLevel, Edge, IslandDomain, IslandProfile, RegionalBoundaryState};

const C: DomainLevel = DomainLevel::Coarse;

struct World {
    canon: CanonLocked,
    domain: IslandDomain,
    elevation: Grid2<f64>,
    climate: ClimateState,
    weather: mk_engine::weather::WeatherState,
    boundaries: RegionalBoundaryState,
}

fn world() -> World {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    let canon = CanonLocked::load(&path).expect("island canon");
    let domain = IslandDomain::from_profile(IslandProfile::default_nz_scale()).unwrap();
    let spec = domain.storage_spec(C);
    // A 300 km square island in open ocean.
    let elevation = Grid2::from_data(
        &spec,
        (0..spec.nlat * spec.nlon)
            .map(|i| {
                let (r, c) = (i / spec.nlon, i % spec.nlon);
                if (60..110).contains(&r) && (70..130).contains(&c) {
                    500.0
                } else {
                    -3000.0
                }
            })
            .collect(),
    );
    let background = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    let boundaries =
        sample_regional_boundaries_with_background([5u8; 32], &canon, &domain, &background, 0.0);
    let orbit = mk_engine::orbit::step_orbit(&canon, 0.0);
    let rotation = mk_engine::rotation::step_rotation(0.0, orbit.mean_anomaly, &canon);
    let geothermal = Grid2::new(&spec, 0.09);
    let climate = step_regional_climate(
        &ClimateState::default_for_grid(&spec),
        &ClimateForcing {
            toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
            solar_declination_rad: rotation.subsolar_latitude,
            albedo: canon.albedo_baseline,
            surface_pressure_pa: canon.sea_level_pressure_pa,
            surface_gravity_m_s2: canon.surface_gravity_m_s2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: 1.0e11,
            elevation_m: &elevation,
            dt_seconds: f64::INFINITY,
        },
        &domain,
        &background,
        &boundaries.atmosphere,
        &boundaries.ocean,
        // Calm: no air carried in, the energy balance on its own.
        &mk_core::grid::Grid2::new(
            &spec,
            mk_engine::weather::WindVector {
                u_east: 0.0,
                v_north: 0.0,
            },
        ),
    );
    let weather = step_regional_weather(
        &canon,
        &climate,
        &elevation,
        &domain,
        &background,
        &boundaries.atmosphere,
    );
    World {
        canon,
        domain,
        elevation,
        climate,
        weather,
        boundaries,
    }
}

fn ocean_step(w: &World, boundary: &mk_island::OceanBoundaryForcing) -> OceanState {
    let spec = w.domain.storage_spec(C);
    let previous = Grid2::new(&spec, OceanColumn::new(0.0, 0.0, 0.0));
    let coriolis = regional_coriolis(&w.domain, C, &w.canon);
    step_regional_ocean(
        &w.canon,
        &previous,
        &OceanForcing {
            wind: &w.weather.wind,
            climate: &w.climate,
            precipitation_mm_day: &w.weather.precipitation,
            coriolis: &coriolis,
            elevation_m: &w.elevation,
            dt_seconds: 86_400.0,
        },
        &w.domain,
        boundary,
    )
}

fn edge(b: &mk_island::OceanBoundaryForcing, e: Edge) -> &mk_island::EdgeOceanForcing {
    b.edges.iter().find(|x| x.edge == e).unwrap()
}

#[test]
fn land_has_no_column_and_edge_cells_take_the_boundary_forcing() {
    let w = world();
    let s = ocean_step(&w, &w.boundaries.ocean);
    let (rows, cols) = (w.domain.rows(C), w.domain.cols(C));
    for r in 0..rows {
        for c in 0..cols {
            let col = s.columns.get(r, c);
            let cur = s.currents.get(r, c);
            if *w.elevation.get(r, c) > 0.0 {
                assert_eq!(col.depth, 0.0);
                assert_eq!((cur.u_east, cur.v_north), (0.0, 0.0));
            } else {
                assert!((col.depth - 3000.0).abs() < 1e-9);
                assert!(col.salinity.is_finite() && col.density > 900.0);
            }
        }
    }
    // Edge ring cells clear of the corners (where two edges blend by
    // weight) equal the boundary exactly: salinity and the inflow normal
    // to the edge (m/s -> cm/s, into the domain).
    let b = &w.boundaries.ocean;
    for i in EDGE_RELAX_CELLS..cols - EDGE_RELAX_CELLS {
        let south = edge(b, Edge::South);
        let north = edge(b, Edge::North);
        assert!((s.columns.get(0, i).salinity - south.salinity_psu[i]).abs() < 1e-9);
        assert!((s.currents.get(0, i).v_north - 100.0 * south.inflow_m_s[i]).abs() < 1e-9);
        assert!((s.columns.get(rows - 1, i).salinity - north.salinity_psu[i]).abs() < 1e-9);
        assert!((s.currents.get(rows - 1, i).v_north + 100.0 * north.inflow_m_s[i]).abs() < 1e-9);
    }
    for i in EDGE_RELAX_CELLS..rows - EDGE_RELAX_CELLS {
        let west = edge(b, Edge::West);
        let east = edge(b, Edge::East);
        assert!((s.columns.get(i, 0).salinity - west.salinity_psu[i]).abs() < 1e-9);
        assert!((s.currents.get(i, 0).u_east - 100.0 * west.inflow_m_s[i]).abs() < 1e-9);
        assert!((s.currents.get(i, cols - 1).u_east + 100.0 * east.inflow_m_s[i]).abs() < 1e-9);
    }
}

#[test]
fn nothing_wraps_and_the_interior_is_beyond_the_edge_forcing() {
    let w = world();
    let base = ocean_step(&w, &w.boundaries.ocean);
    let mut changed = w.boundaries.ocean.clone();
    for e in changed.edges.iter_mut().filter(|e| e.edge == Edge::East) {
        e.salinity_psu.iter_mut().for_each(|v| *v = 41.0);
        e.inflow_m_s.iter_mut().for_each(|v| *v = 0.9);
    }
    let alt = ocean_step(&w, &changed);
    let (rows, cols) = (w.domain.rows(C), w.domain.cols(C));
    let mut east_moved = false;
    for r in 0..rows {
        for c in 0..cols {
            let same = base.columns.get(r, c).salinity == alt.columns.get(r, c).salinity
                && base.currents.get(r, c).u_east == alt.currents.get(r, c).u_east;
            if c < cols - EDGE_RELAX_CELLS {
                assert!(same, "({r},{c}) changed by the east edge");
            } else if !same {
                east_moved = true;
            }
        }
    }
    assert!(
        east_moved,
        "the east edge forcing had no effect on its own cells"
    );
}

#[test]
fn extreme_edge_forcing_is_clamped_and_the_step_is_deterministic() {
    let w = world();
    let mut wild = w.boundaries.ocean.clone();
    for e in &mut wild.edges {
        e.salinity_psu.iter_mut().for_each(|v| *v = 1.0e9);
        e.inflow_m_s.iter_mut().for_each(|v| *v = -1.0e9);
    }
    let s = ocean_step(&w, &wild);
    assert!(s
        .columns
        .data()
        .iter()
        .all(|c| (0.0..=45.0).contains(&c.salinity) && c.density.is_finite()));
    assert!(s
        .currents
        .data()
        .iter()
        .all(|v| v.u_east.abs() <= 500.0 + 1e-9 && v.v_north.abs() <= 500.0 + 1e-9));
    let a = serde_json::to_string(&ocean_step(&w, &w.boundaries.ocean)).unwrap();
    assert_eq!(
        a,
        serde_json::to_string(&ocean_step(&w, &w.boundaries.ocean)).unwrap()
    );
}

#[test]
fn tides_vary_across_the_domain_book_the_ledger_and_are_deterministic() {
    let w = world();
    let canon = Arc::new(w.canon.clone());
    let run = |t: f64| {
        let mut ledger = Ledger::new();
        let state = step_regional_tides(&canon, t, 3_600, &w.domain, &w.elevation, &mut ledger);
        (state, ledger)
    };
    let (state, ledger) = run(1.0e6);
    let (rows, cols) = (w.domain.rows(C), w.domain.cols(C));
    assert_eq!(
        (state.potential.nlat(), state.potential.nlon()),
        (rows, cols)
    );
    assert_eq!(state.equilibrium_height_m.len(), rows * cols);
    assert!(state
        .potential
        .data()
        .iter()
        .all(|p| (-0.5..=1.0).contains(p)));
    // Height varies with longitude (and latitude) across the domain.
    let (lo, hi) = state
        .equilibrium_height_m
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &h| (lo.min(h), hi.max(h)));
    assert!(hi - lo > 1e-3, "tide range across the domain {} m", hi - lo);
    let west_mean: f64 = (0..rows)
        .map(|r| state.equilibrium_height_m[r * cols])
        .sum::<f64>()
        / rows as f64;
    let east_mean: f64 = (0..rows)
        .map(|r| state.equilibrium_height_m[r * cols + cols - 1])
        .sum::<f64>()
        / rows as f64;
    assert!(
        (west_mean - east_mean).abs() > 1e-6,
        "no west-east variation"
    );
    // The phase is analytic: it moves with time.
    let (later, _) = run(1.0e6 + 0.25 * w.canon.moon_mckenz_period_s);
    assert_ne!(state.equilibrium_height_m, later.equilibrium_height_m);
    // Deterministic to the byte.
    assert_eq!(state, run(1.0e6).0);

    // The dissipation entry is the planet's tidal heating times the
    // ocean's share of the planet's surface, for the step.
    let planet = {
        let mut l = Ledger::new();
        mk_engine::tides::step_tides(&canon, 1.0e6, 3_600, &GridSpec::new(4, 8), &mut l)
            .heating_power_w
    };
    let ocean_cells = w.elevation.data().iter().filter(|&&h| h <= 0.0).count() as f64;
    let share = ocean_cells * w.domain.cell_area_m2(C)
        / (4.0 * std::f64::consts::PI * w.canon.planet_radius_m.powi(2));
    let entries: Vec<_> = ledger
        .entries()
        .iter()
        .filter(|e| e.source == Reservoir::TidalHeat && e.sink == Reservoir::OceanHeat)
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, FluxKind::Energy);
    let want = planet * share * 3_600.0;
    assert!(
        (entries[0].amount - want).abs() <= 1e-9 * want.abs().max(1.0),
        "{} vs {want}",
        entries[0].amount
    );
    assert!((state.heating_power_w - planet * share).abs() <= 1e-9 * planet);

    // The moons' phase is the one the boundary forcing carries.
    let t = 1.0e6;
    let b = sample_regional_boundaries_with_background(
        [5u8; 32],
        &w.canon,
        &w.domain,
        &ZonalBackgroundState::bootstrap(&w.canon, DEFAULT_BAND_COUNT),
        t,
    );
    assert_eq!(
        b.astronomy.moon_sub_longitudes_rad[0],
        mk_engine::tides::sub_lunar_longitude(
            t,
            w.canon.moon_mckenz_period_s,
            w.canon.rotation_period_s
        )
    );
}
