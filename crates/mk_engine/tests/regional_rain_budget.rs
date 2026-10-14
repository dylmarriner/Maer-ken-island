//! D38 diagnosis: where the island's land rain comes from over a year.
//!
//! For every coarse land cell, a year's rain is split into the zonal
//! background's rate at its latitude (what the open sea gets), the rain
//! the weather's land-against-sea pattern gives before any orography, and
//! the final rain after the orographic vapour budget. Run with
//! `cargo test --release -p mk_engine --test regional_rain_budget -- --ignored --nocapture`.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::climate::{coarse_latitudes, distance_to_sea_m};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::weather::step_regional_weather;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn quantiles(mut v: Vec<f64>) -> String {
    v.sort_by(f64::total_cmp);
    let q = |p: f64| v[((v.len() - 1) as f64 * p).round() as usize];
    format!(
        "p10 {:7.0}  p25 {:7.0}  median {:7.0}  p75 {:7.0}  p90 {:7.0}  mean {:7.0}",
        q(0.1),
        q(0.25),
        q(0.5),
        q(0.75),
        q(0.9),
        v.iter().sum::<f64>() / v.len() as f64
    )
}

#[test]
#[ignore = "diagnostic: a year of weather, release only"]
fn slow_where_the_island_s_land_rain_comes_from() {
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
    let seed = profile.seed_bytes().unwrap();
    let domain = IslandDomain::from_profile(profile).unwrap();
    let mut s = RegionalPhysicalState::bootstrap(&canon, &domain, seed).unwrap();
    let coarse = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(coarse), domain.cols(coarse));
    let n = rows * cols;
    let elev = s.coarse_elevation_m().clone();
    let latitudes = coarse_latitudes(&domain);
    let local_day = canon.rotation_period_s as u64;
    let steps = (canon.orbital_period_s / canon.rotation_period_s).round() as u64;
    let per_earth_day = local_day as f64 / 86_400.0;

    let (mut background, mut pattern, mut fin) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for day in 1..=steps {
        s.step(
            &canon,
            &domain,
            seed,
            (day * local_day) as f64,
            day,
            local_day,
        )
        .unwrap();
        let base = step_regional_weather(
            &canon,
            &s.climate,
            &elev,
            &domain,
            &s.zonal_background,
            &s.boundaries.atmosphere,
        );
        for i in 0..n {
            background[i] +=
                s.zonal_background.precipitation_at(latitudes[i / cols]) * per_earth_day;
            pattern[i] += base.precipitation.data()[i] * per_earth_day;
            fin[i] += s.weather.precipitation.data()[i] * per_earth_day;
        }
    }

    let land: Vec<usize> = (0..n).filter(|&i| elev.data()[i] > 0.0).collect();
    let sea: Vec<usize> = (0..n).filter(|&i| elev.data()[i] <= 0.0).collect();
    let pick = |v: &[f64], cells: &[usize]| cells.iter().map(|&i| v[i]).collect::<Vec<_>>();
    println!("land cells {}, sea cells {}", land.len(), sea.len());
    println!("sea  final      {}", quantiles(pick(&fin, &sea)));
    println!("land background {}", quantiles(pick(&background, &land)));
    println!("land pattern    {}", quantiles(pick(&pattern, &land)));
    println!("land final      {}", quantiles(pick(&fin, &land)));
    println!(
        "land pattern / background {}",
        quantiles(
            land.iter()
                .map(|&i| pattern[i] / background[i] * 1000.0)
                .collect()
        )
    );
    println!(
        "land final / pattern      {}",
        quantiles(land.iter().map(|&i| fin[i] / pattern[i] * 1000.0).collect())
    );
    println!("(ratios in thousandths)");

    // By distance from the sea and by height.
    let to_sea = distance_to_sea_m(&elev, domain.cell_size_m(coarse));
    for (lo, hi) in [(0.0, 25e3), (25e3, 50e3), (50e3, 100e3), (100e3, 1e9)] {
        let cells: Vec<usize> = land
            .iter()
            .copied()
            .filter(|&i| (lo..hi).contains(&to_sea[i]))
            .collect();
        if cells.is_empty() {
            continue;
        }
        println!(
            "{:>4.0}-{:<4.0} km from sea ({:5} cells): pattern median {:5.0}, final {}",
            lo / 1e3,
            (hi / 1e3).min(9999.0),
            cells.len(),
            {
                let mut p = pick(&pattern, &cells);
                p.sort_by(f64::total_cmp);
                p[p.len() / 2]
            },
            quantiles(pick(&fin, &cells))
        );
    }
    for (lo, hi) in [(0.0, 300.0), (300.0, 800.0), (800.0, 1500.0), (1500.0, 9e3)] {
        let cells: Vec<usize> = land
            .iter()
            .copied()
            .filter(|&i| (lo..hi).contains(elev.data().get(i).unwrap()))
            .collect();
        if cells.is_empty() {
            continue;
        }
        println!(
            "{lo:>5.0}-{hi:<5.0} m ({:5} cells): final {}",
            cells.len(),
            quantiles(pick(&fin, &cells))
        );
    }
}
