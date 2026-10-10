//! Phase 2 Task 6: the coupled regional physical tick.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const C: DomainLevel = DomainLevel::Coarse;
const M: DomainLevel = DomainLevel::Medium;
const DAY: u64 = 86_400;

struct Base {
    canon: Arc<CanonLocked>,
    domain: IslandDomain,
    seed: [u8; 32],
    state: RegionalPhysicalState,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// The owner's island, bootstrapped once and cloned per test.
fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().expect("pinned seed");
        let domain = IslandDomain::from_profile(profile).unwrap();
        let state = RegionalPhysicalState::bootstrap(&canon, &domain, seed).expect("bootstrap");
        Base {
            canon,
            domain,
            seed,
            state,
        }
    })
}

/// Soil and standing water over every medium cell (kg).
fn water_kg(b: &Base, s: &RegionalPhysicalState) -> f64 {
    let a = b.domain.cell_area_m2(M);
    (s.hydrology
        .soil_water
        .data()
        .iter()
        .map(|w| w.storage_mm)
        .sum::<f64>()
        + s.hydrology.surface_water.data().iter().sum::<f64>())
        * a
}

fn run(b: &Base, days: u64) -> (RegionalPhysicalState, Vec<f64>) {
    let mut s = b.state.clone();
    let mut closure_errors = Vec::new();
    for day in 1..=days {
        let before = water_kg(b, &s);
        s.step(&b.canon, &b.domain, b.seed, (day * DAY) as f64, day, DAY)
            .expect("step");
        let bud = s.hydrology.budget;
        let expected = bud.precipitation_kg
            - bud.surface_evaporation_kg
            - bud.soil_evaporation_kg
            - bud.deep_drainage_kg
            - bud.surface_to_ocean_kg
            + bud.ocean_to_sea_soil_kg;
        closure_errors
            .push((water_kg(b, &s) - before - expected).abs() / bud.precipitation_kg.max(1.0));
    }
    (s, closure_errors)
}

#[test]
fn thirty_days_change_the_weather_stay_finite_conserve_water_and_replay_exactly() {
    let b = base();
    let start = b.state.state_hash();
    let (a, closure) = run(b, 30);

    // Bootstrap left the world on its seasonal cycle at t = 0.
    assert_eq!(b.state.sim_time_seconds, 0.0);
    assert_ne!(a.state_hash(), start, "thirty days changed nothing");
    assert_eq!(a.sim_time_seconds, 30.0 * DAY as f64);

    // Weather, ocean and hydrology all moved.
    let moved = |x: &[f64], y: &[f64]| x.iter().zip(y).any(|(p, q)| p != q);
    assert!(moved(
        a.weather.precipitation.data(),
        b.state.weather.precipitation.data()
    ));
    assert!(moved(
        &a.ocean
            .columns
            .data()
            .iter()
            .map(|c| c.surface_temp)
            .collect::<Vec<_>>(),
        &b.state
            .ocean
            .columns
            .data()
            .iter()
            .map(|c| c.surface_temp)
            .collect::<Vec<_>>()
    ));
    assert!(moved(
        &a.hydrology
            .soil_water
            .data()
            .iter()
            .map(|w| w.storage_mm)
            .collect::<Vec<_>>(),
        &b.state
            .hydrology
            .soil_water
            .data()
            .iter()
            .map(|w| w.storage_mm)
            .collect::<Vec<_>>()
    ));

    // Everything finite; the land mask (and so the terrain) is preserved.
    for g in [
        &a.climate.surface_temperature,
        &a.climate.atmos_temperature,
        &a.weather.precipitation,
        &a.weather.moisture,
        &a.diurnal_offset_k,
    ] {
        assert!(g.data().iter().all(|v| v.is_finite()));
    }
    assert!(a
        .ocean
        .columns
        .data()
        .iter()
        .all(|c| c.salinity.is_finite() && c.density.is_finite()));
    assert_eq!(
        a.geophysics.land_mask.data(),
        b.state.geophysics.land_mask.data()
    );
    // Land cells hold no ocean column; ocean cells do.
    let (rows, cols) = (b.domain.rows(C), b.domain.cols(C));
    for r in 0..rows {
        for c in 0..cols {
            let depth = a.ocean.columns.get(r, c).depth;
            if *a.coarse_elevation_m().get(r, c) > 0.0 {
                assert_eq!(depth, 0.0);
            } else {
                assert!(depth > 0.0);
            }
        }
    }

    // The water budget closed on every step (soil + standing water =
    // rain - evaporation - drainage - to the sea).
    let worst = closure.iter().cloned().fold(0.0, f64::max);
    assert!(worst <= 1e-9, "worst budget closure error {worst:e}");
    assert!(a.river_water_to_ocean_kg >= 0.0);

    // Identical replay: the same final bits.
    let (again, _) = run(b, 30);
    assert_eq!(a.state_hash(), again.state_hash());
}

#[test]
fn a_zero_step_changes_nothing() {
    let b = base();
    let mut s = b.state.clone();
    s.step(&b.canon, &b.domain, b.seed, 5.0 * DAY as f64, 0, 0)
        .unwrap();
    assert_eq!(s.state_hash(), b.state.state_hash());
}

#[test]
fn the_physical_state_allocates_no_planetary_grid() {
    let b = base();
    let s = &b.state;
    let (rows_c, cols_c) = (b.domain.rows(C), b.domain.cols(C));
    for (name, nlat, nlon) in [
        (
            "climate",
            s.climate.surface_temperature.nlat(),
            s.climate.surface_temperature.nlon(),
        ),
        (
            "weather",
            s.weather.precipitation.nlat(),
            s.weather.precipitation.nlon(),
        ),
        ("ocean", s.ocean.columns.nlat(), s.ocean.columns.nlon()),
        ("tides", s.tides.potential.nlat(), s.tides.potential.nlon()),
        (
            "insolation",
            s.insolation.toa_w_m2.nlat(),
            s.insolation.toa_w_m2.nlon(),
        ),
    ] {
        assert_eq!(
            (nlat, nlon),
            (rows_c, cols_c),
            "{name} is not on the coarse grid"
        );
    }
    assert_eq!(s.hydrology.soil_water.nlat(), b.domain.rows(M));
    // Land is a small share of the medium grid, so hydrology iterates
    // ~6% of its cells.
    let land = s.flow_network().land_cells() as f64;
    assert!(
        land / (b.domain.rows(M) * b.domain.cols(M)) as f64 <= 0.10,
        "land cells {land}"
    );
}

/// One simulated year (360 local days of 36 h) of the owner's island,
/// checked against the Phase-0c climate and hydrology reference packs.
/// Each tolerance is stated where it is applied. Slow tier:
/// `cargo test --release -- --ignored slow_`.
#[test]
#[ignore = "slow: one simulated year"]
fn slow_a_year_of_island_weather_matches_the_reference_packs() {
    use mk_engine::regional::climate::distance_to_sea_m;

    let b = base();
    let local_day = b.canon.rotation_period_s as u64;
    let steps = (b.canon.orbital_period_s / b.canon.rotation_period_s).round() as u64;
    let (rows, cols) = (b.domain.rows(C), b.domain.cols(C));
    let n = rows * cols;
    let elev = b.state.coarse_elevation_m().clone();
    let interior = |r: usize, c: usize| r >= 40 && r < rows - 40 && c >= 40 && c < cols - 40;

    let mut s = b.state.clone();
    let (mut temp, mut rain, mut u, mut v) =
        (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let mut sst_by_day = Vec::new();
    // Monthly mean surface temperature per cell: twelve thirty-day months.
    let mut monthly = vec![vec![0.0; n]; 12];
    let (mut precip_kg, mut to_sea_kg, mut deep_kg) = (0.0, 0.0, 0.0);
    for day in 1..=steps {
        s.step(
            &b.canon,
            &b.domain,
            b.seed,
            (day * local_day) as f64,
            day,
            local_day,
        )
        .expect("step");
        let (mut sst, mut count) = (0.0, 0.0);
        for r in 0..rows {
            for c in 0..cols {
                let i = r * cols + c;
                temp[i] += s.climate.surface_temperature.data()[i] / steps as f64;
                let month = (((day - 1) * 12) / steps).min(11) as usize;
                monthly[month][i] += s.climate.surface_temperature.data()[i] * 12.0 / steps as f64;
                rain[i] += s.weather.precipitation.data()[i] * (local_day as f64 / 86_400.0);
                let w = s.weather.wind.get(r, c);
                u[i] += w.u_east / steps as f64;
                v[i] += w.v_north / steps as f64;
                if *elev.get(r, c) < -200.0 && interior(r, c) {
                    sst += s.climate.surface_temperature.get(r, c);
                    count += 1.0;
                }
            }
        }
        sst_by_day.push(sst / count);
        precip_kg += s.hydrology.budget.precipitation_kg;
        to_sea_kg += s.hydrology.budget.surface_to_ocean_kg;
        deep_kg += s.hydrology.budget.deep_drainage_kg;
    }

    // 1. Lapse rate: within each row, annual-mean land temperature falls
    //    with elevation at 6.5 K/km (US Standard Atmosphere). Tolerance
    //    +-1 K/km: 12 km cells average peaks, and the land's radiative
    //    response is not exactly linear.
    let (mut cov, mut var) = (0.0, 0.0);
    for r in 0..rows {
        let land: Vec<usize> = (0..cols).filter(|&c| *elev.get(r, c) > 0.0).collect();
        if land.len() < 3 {
            continue;
        }
        let (mh, mt) = (
            land.iter().map(|&c| *elev.get(r, c)).sum::<f64>() / land.len() as f64,
            land.iter().map(|&c| temp[r * cols + c]).sum::<f64>() / land.len() as f64,
        );
        for &c in &land {
            cov += (elev.get(r, c) - mh) * (temp[r * cols + c] - mt);
            var += (elev.get(r, c) - mh).powi(2);
        }
    }
    let lapse_k_km = -1000.0 * cov / var;
    println!("lapse rate {lapse_k_km:.2} K/km");
    assert!((5.5..=7.5).contains(&lapse_k_km), "lapse {lapse_k_km}");

    // 2. Diurnal range by surface (Dai 1999, Geiger 2009, Kawai & Wada
    //    2007 in the climate pack, 24 h values). A 36 h day widens the
    //    land range, so the land upper bounds are x1.5.
    let range = s.diurnal_range_k(&b.domain, &b.canon);
    let sea = distance_to_sea_m(&elev, b.domain.cell_size_m(C));
    let mean_where = |pred: &dyn Fn(usize) -> bool| {
        let (mut sum, mut k) = (0.0, 0.0);
        for i in 0..n {
            if pred(i) {
                sum += range.data()[i];
                k += 1.0;
            }
        }
        (k > 0.0).then(|| sum / k)
    };
    let ocean_dtr = mean_where(&|i| elev.data()[i] <= 0.0).unwrap();
    let coastal_dtr = mean_where(&|i| elev.data()[i] > 0.0 && sea[i] <= 24_000.0);
    let inland_dtr = mean_where(&|i| elev.data()[i] > 0.0 && sea[i] >= 60_000.0);
    println!("DTR ocean {ocean_dtr:.2}, coastal {coastal_dtr:?}, inland {inland_dtr:?} K");
    assert!((0.1..=0.6).contains(&ocean_dtr), "ocean DTR {ocean_dtr}");
    if let Some(c) = coastal_dtr {
        assert!((3.0..=13.5).contains(&c), "coastal DTR {c}");
    }
    if let Some(i) = inland_dtr {
        assert!((6.0..=22.0).contains(&i), "inland DTR {i}");
    }

    // 3. Windward/leeward rain over the ranges: the reference ratio for a
    //    >2 km barrier is 3-8 (Southern Alps ~5), applied as is.
    let mut wind_up = (0.0, 0.0, 0.0, 0.0);
    for r in 1..rows - 1 {
        for c in 1..cols - 1 {
            let i = r * cols + c;
            if *elev.get(r, c) < 300.0 {
                continue;
            }
            let size = b.domain.cell_size_m(C);
            let dx = (elev.get(r, c + 1) - elev.get(r, c - 1)) / (2.0 * size);
            let dy = (elev.get(r + 1, c) - elev.get(r - 1, c)) / (2.0 * size);
            let lift = u[i] * dx + v[i] * dy;
            if lift > 0.0 {
                wind_up.0 += rain[i];
                wind_up.1 += 1.0;
            } else if lift < 0.0 {
                wind_up.2 += rain[i];
                wind_up.3 += 1.0;
            }
        }
    }
    if wind_up.1 > 0.0 && wind_up.3 > 0.0 {
        let ratio = (wind_up.0 / wind_up.1) / (wind_up.2 / wind_up.3);
        println!(
            "windward/leeward {ratio:.2} ({} / {} cells)",
            wind_up.1, wind_up.3
        );
        assert!((3.0..=8.0).contains(&ratio), "windward/leeward {ratio}");
    }

    // 4. Open-ocean SST seasonal range at 40-45 degrees: 3-6 K observed
    //    (Reynolds 2007), applied as is. (Its phase lags the solstice,
    //    D25; its amplitude does not.)
    let sst_range = sst_by_day.iter().cloned().fold(f64::MIN, f64::max)
        - sst_by_day.iter().cloned().fold(f64::MAX, f64::min);
    println!("SST seasonal range {sst_range:.2} K");
    assert!((3.0..=6.0).contains(&sst_range), "SST range {sst_range}");

    // 6. The year over lowland: warmest minus coldest monthly mean, median
    //    over land cells below 300 m. New Zealand's lowland stations at
    //    the same latitudes run 7-16 K (NIWA; `maritime_land_annual_
    //    temperature_range`), the top of that only in sheltered inland
    //    basins; tolerance x1.25 on the top, for 12 km cells that average
    //    coast and interior. Before deviation D37's maritime air this was
    //    ~46 K.
    let mut lowland_ranges: Vec<f64> = (0..n)
        .filter(|&i| elev.data()[i] > 0.0 && elev.data()[i] < 300.0)
        .map(|i| {
            let months: Vec<f64> = monthly.iter().map(|m| m[i]).collect();
            months.iter().cloned().fold(f64::MIN, f64::max)
                - months.iter().cloned().fold(f64::MAX, f64::min)
        })
        .collect();
    lowland_ranges.sort_by(f64::total_cmp);
    let median_range = lowland_ranges[lowland_ranges.len() / 2];
    println!(
        "lowland annual range: median {median_range:.1} K over {} cells (min {:.1}, max {:.1})",
        lowland_ranges.len(),
        lowland_ranges[0],
        lowland_ranges[lowland_ranges.len() - 1]
    );
    assert!(
        (7.0..=16.0 * 1.25).contains(&median_range),
        "lowland annual range {median_range} K"
    );

    // 5. Runoff: of the rain on land, the share that reaches the sea or
    //    recharges groundwater (Budyko, Fu omega 2.6) is 0.2-0.85 across
    //    aridity 0.25-2 in the hydrology pack.
    let runoff_ratio = (to_sea_kg + deep_kg) / precip_kg;
    println!(
        "runoff ratio {runoff_ratio:.2} (sea {:.2e} kg, deep {:.2e} kg, rain {:.2e} kg)",
        to_sea_kg, deep_kg, precip_kg
    );
    assert!(
        (0.2..=0.85).contains(&runoff_ratio),
        "runoff ratio {runoff_ratio}"
    );
}
