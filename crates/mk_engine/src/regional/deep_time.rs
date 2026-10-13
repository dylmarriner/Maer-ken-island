//! Deep-time spin-up (Phase 4 Task 2b): centuries of the living world before
//! anybody lives on it, so the island starts with its plants, grazers and
//! soils where their own dynamics put them rather than where the bootstrap
//! guessed.
//!
//! The bootstrap lays the plants down at `NPP × residence` and the soil at
//! RothC's steady state for plant litter alone. Neither is where the island
//! settles: the grazers eat part of every year's production, so the plants
//! stand lower, and their faeces and carcasses reach the soil in different
//! compartments from leaf litter, so the soil's pools move for centuries
//! (RothC's humus turns over in about fifty years). The spin-up runs exactly
//! the step the living island runs ([`super::ecology::grazed_cell_step`]),
//! monthly, for as many local years as the scenario asks.
//!
//! What it does not step, and why (D20):
//!
//! - **Weather.** Ecology reads the climatology, not the hour's weather: NPP
//!   comes from each cell's annual-mean climate and the soil's temperature
//!   modifier from its year-average, both measured over the physical
//!   bootstrap's spin-up orbits. Nothing in deep time changes terrain or
//!   cover's effect on climate, so re-measuring it every century, as the plan
//!   proposed, would measure the same climatology again.
//! - **Rivers and lakes.** The physical bootstrap already runs them to their
//!   seasonal cycle, and nothing here changes the terrain they drain.
//! - **Geology.** The island has no erosion or uplift model; the plates are
//!   fixed over any horizon a run covers.
//!
//! Every cell is independent, so each runs its whole spin-up in one tight
//! loop on its own core, and the result is identical at any thread count.
//! Humans are never fast-forwarded: [`spin_up`] takes the ecology alone, and
//! [`super::world::IslandWorldState::spin_up`] refuses once anyone lives.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use mk_core::canon::CanonLocked;
use mk_island::IslandScenario;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::ecology::{grazed_cell_step, RegionalEcologyState};

/// Steps per local year: one per climatology month.
pub const STEPS_PER_YEAR: u32 = crate::climate::CLIMATOLOGY_PHASE_BINS as u32;

/// The window the equilibrium trends are measured over (local years).
pub const TREND_WINDOW_YEARS: u32 = 100;

/// An Earth year, the unit the ecology's rates are written in.
const EARTH_YEAR_S: f64 = 365.25 * 86_400.0;

/// What the spin-up was asked to do. Part of the cache key.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpinUpSettings {
    pub years: u32,
    pub steps_per_year: u32,
    /// The local year (s), the canon's orbital period.
    pub year_seconds: f64,
}

impl SpinUpSettings {
    pub fn new(canon: &CanonLocked, years: u32) -> Self {
        Self {
            years,
            steps_per_year: STEPS_PER_YEAR,
            year_seconds: canon.orbital_period_s,
        }
    }
}

/// Totals over the island (kg C) and their change over the last
/// [`TREND_WINDOW_YEARS`], as a fraction per century (positive is rising).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EquilibriumMetrics {
    pub soil_carbon_kgc: f64,
    pub biomass_kgc: f64,
    pub herbivore_kgc: f64,
    pub soil_carbon_trend_per_century: f64,
    pub biomass_trend_per_century: f64,
    pub herbivore_trend_per_century: f64,
    /// The same totals before the spin-up began.
    pub start_soil_carbon_kgc: f64,
    pub start_biomass_kgc: f64,
    pub start_herbivore_kgc: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpinUpReport {
    pub settings: SpinUpSettings,
    /// Wall-clock time the spin-up took, zero when it came from the cache.
    pub wall_seconds: f64,
    pub from_cache: bool,
    pub equilibrium: EquilibriumMetrics,
}

#[derive(Debug)]
pub enum DeepTimeError {
    /// The ecology has no soil to spin up (an island must install it first).
    NoSoil,
    /// Somebody already lives on the island.
    HumansExist(usize),
    Io(std::io::Error),
    Corrupt(String),
}

impl std::fmt::Display for DeepTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSoil => write!(f, "the ecology has no soil installed"),
            Self::HumansExist(n) => write!(
                f,
                "{n} people live on the island; humans are never fast-forwarded"
            ),
            Self::Io(e) => write!(f, "spin-up cache: {e}"),
            Self::Corrupt(why) => write!(f, "spin-up cache is corrupt: {why}"),
        }
    }
}

impl std::error::Error for DeepTimeError {}

/// Run the island's living world for `settings.years` local years, each in
/// `settings.steps_per_year` steps, in place.
pub fn spin_up(
    ecology: &mut RegionalEcologyState,
    cell_area_m2: f64,
    settings: SpinUpSettings,
) -> Result<SpinUpReport, DeepTimeError> {
    let started = std::time::Instant::now();
    let soil = ecology.soil.as_mut().ok_or(DeepTimeError::NoSoil)?;
    let params = soil.params;
    let biomes = ecology.biome_grid.data();
    let npp = ecology.npp_kgc_m2_yr.data();
    let factor = &soil.temperature_factor;
    let dt_years = settings.year_seconds / settings.steps_per_year.max(1) as f64 / EARTH_YEAR_S;
    let checkpoint = settings.years.saturating_sub(TREND_WINDOW_YEARS);
    let mut respired = vec![0.0; biomes.len()];

    // Per cell: [soil, biomass, grazers] at the start, at the checkpoint
    // and at the end.
    let samples: Vec<[[f64; 3]; 3]> = ecology
        .biomass_kgc_m2
        .data_mut()
        .par_iter_mut()
        .zip(ecology.herbivore_kgc_m2.data_mut().par_iter_mut())
        .zip(soil.pools.par_iter_mut())
        .zip(respired.par_iter_mut())
        .enumerate()
        .map(|(i, (((b, g), pools), respired))| {
            let sample = |b: f64, g: f64, pools: &[f64; 4]| [pools.iter().sum::<f64>(), b, g];
            let mut out = [sample(*b, *g, pools); 3];
            if params.plant_dpm_share(biomes[i]).is_none() && *b <= 0.0 && *g <= 0.0 {
                return out;
            }
            for year in 0..settings.years {
                if year == checkpoint {
                    out[1] = sample(*b, *g, pools);
                }
                for _ in 0..settings.steps_per_year {
                    let (_, r) = grazed_cell_step(
                        biomes[i],
                        npp[i],
                        b,
                        g,
                        Some((&params, pools, factor[i])),
                        dt_years,
                    );
                    *respired += r;
                }
            }
            if settings.years == checkpoint {
                out[1] = sample(*b, *g, pools);
            }
            out[2] = sample(*b, *g, pools);
            out
        })
        .collect();
    soil.respired_kgc_m2 += crate::regional::par::det_sum(&respired);

    let total = |when: usize, what: usize| {
        let values: Vec<f64> = samples.iter().map(|s| s[when][what]).collect();
        crate::regional::par::det_sum(&values) * cell_area_m2
    };
    let window = settings.years.clamp(1, TREND_WINDOW_YEARS) as f64;
    let trend = |what: usize| {
        let (then, now) = (total(1, what), total(2, what));
        if now > 0.0 {
            (now - then) / now * (100.0 / window)
        } else {
            0.0
        }
    };
    Ok(SpinUpReport {
        settings,
        wall_seconds: started.elapsed().as_secs_f64(),
        from_cache: false,
        equilibrium: EquilibriumMetrics {
            soil_carbon_kgc: total(2, 0),
            biomass_kgc: total(2, 1),
            herbivore_kgc: total(2, 2),
            soil_carbon_trend_per_century: trend(0),
            biomass_trend_per_century: trend(1),
            herbivore_trend_per_century: trend(2),
            start_soil_carbon_kgc: total(0, 0),
            start_biomass_kgc: total(0, 1),
            start_herbivore_kgc: total(0, 2),
        },
    })
}

/// The cache key: canon, scenario (which carries the seed and the profile)
/// and spin-up settings, hashed together. Any change to any of them is a
/// different island, so a different file.
pub fn cache_key(
    canon: &CanonLocked,
    scenario: &IslandScenario,
    settings: SpinUpSettings,
) -> String {
    let mut h = blake3::Hasher::new();
    h.update(b"mk-island-spinup-v1");
    for part in [
        serde_json::to_vec(canon).expect("the canon serialises"),
        serde_json::to_vec(scenario).expect("the scenario serialises"),
        serde_json::to_vec(&settings).expect("settings serialise"),
    ] {
        h.update(&(part.len() as u64).to_le_bytes());
        h.update(&part);
    }
    h.finalize().to_hex().to_string()
}

/// Where a spun-up island is kept under `save_root`.
pub fn cache_path(save_root: &Path, key: &str) -> PathBuf {
    save_root.join("spinup").join(format!("{key}.snap"))
}

const MAGIC: &[u8; 8] = b"MKSPINv1";

#[derive(Serialize, Deserialize)]
struct Cached {
    key: String,
    report: SpinUpReport,
    ecology: RegionalEcologyState,
}

/// Save a spun-up ecology: `magic ‖ blake3(rest) ‖ deflated JSON`, written
/// beside the target, fsynced and renamed, so a crash leaves either the old
/// file or the new one.
pub fn save_cached(
    path: &Path,
    key: &str,
    report: &SpinUpReport,
    ecology: &RegionalEcologyState,
) -> Result<(), DeepTimeError> {
    let json = serde_json::to_vec(&Cached {
        key: key.to_string(),
        report: report.clone(),
        ecology: ecology.clone(),
    })
    .map_err(|e| DeepTimeError::Corrupt(e.to_string()))?;
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&json).map_err(DeepTimeError::Io)?;
    let body = enc.finish().map_err(DeepTimeError::Io)?;
    let dir = path
        .parent()
        .ok_or_else(|| DeepTimeError::Io(std::io::Error::other("cache path has no directory")))?;
    std::fs::create_dir_all(dir).map_err(DeepTimeError::Io)?;
    let tmp = path.with_extension("snap.tmp");
    {
        let mut f = std::fs::File::create(&tmp).map_err(DeepTimeError::Io)?;
        f.write_all(MAGIC).map_err(DeepTimeError::Io)?;
        f.write_all(blake3::hash(&body).as_bytes())
            .map_err(DeepTimeError::Io)?;
        f.write_all(&body).map_err(DeepTimeError::Io)?;
        f.sync_all().map_err(DeepTimeError::Io)?;
    }
    std::fs::rename(&tmp, path).map_err(DeepTimeError::Io)?;
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

/// Load a spun-up ecology saved under `key`. `Ok(None)` when there is no
/// file; an error when there is one and it is damaged or belongs to a
/// different key.
pub fn load_cached(
    path: &Path,
    key: &str,
) -> Result<Option<(SpinUpReport, RegionalEcologyState)>, DeepTimeError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(DeepTimeError::Io(e)),
    };
    if bytes.len() < MAGIC.len() + 32 || &bytes[..MAGIC.len()] != MAGIC {
        return Err(DeepTimeError::Corrupt("not a spin-up cache".into()));
    }
    let (digest, body) = bytes[MAGIC.len()..].split_at(32);
    if blake3::hash(body).as_bytes() != digest {
        return Err(DeepTimeError::Corrupt("digest mismatch".into()));
    }
    let mut json = Vec::new();
    flate2::read::DeflateDecoder::new(body)
        .read_to_end(&mut json)
        .map_err(|e| DeepTimeError::Corrupt(e.to_string()))?;
    let cached: Cached =
        serde_json::from_slice(&json).map_err(|e| DeepTimeError::Corrupt(e.to_string()))?;
    if cached.key != key {
        return Err(DeepTimeError::Corrupt(format!(
            "saved for {} not {key}",
            cached.key
        )));
    }
    let mut report = cached.report;
    report.from_cache = true;
    report.wall_seconds = 0.0;
    Ok(Some((report, cached.ecology)))
}
