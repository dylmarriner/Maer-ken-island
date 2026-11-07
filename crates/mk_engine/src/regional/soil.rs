//! Soil carbon: what plants shed and grazers leave behind rots (realism
//! programme R0, deviation D43).
//!
//! Before this, the island's plants turned over their biomass (litterfall,
//! `B / residence` in the ecology step) and the grazers dropped faeces and
//! died, and all of that carbon simply left the books. It now goes into the
//! soil under the cell and decomposes by the Rothamsted Carbon Model
//! (RothC-26.3; Coleman, Prout & Milne 2024), every constant read from
//! `fixtures/reference/ecology/rothc.json`:
//!
//! - incoming plant carbon is split between decomposable (DPM) and resistant
//!   (RPM) plant material by vegetation, faeces as farmyard manure, and a
//!   carcass as decomposable material;
//! - each of the four active compartments (DPM, RPM, microbial biomass BIO,
//!   humified HUM) declines as `Y exp(-a b c k t)`;
//! - what decomposes is split into CO2 and new BIO + HUM by the clay ratio
//!   `x`, and the BIO + HUM 46 : 54.
//!
//! Simplified, and recorded in D43: the moisture modifier `b` is 1 (the
//! island has no topsoil moisture deficit); the clay content is
//! Rothamsted's own 23.4% (the island has no soil texture); the temperature
//! modifier is each cell's year-average of RothC's monthly factor over its
//! climatology, so the slow pools turn over at the right annual rate but the
//! fast pool's seasonal timing is lost; there is no inert organic matter
//! (IOM) pool; and the soil starts at RothC's steady state for the cell's
//! production rather than from a history.

use serde::{Deserialize, Serialize};

use mk_core::biomes::BiomeType;

use crate::validation::{ReferenceDomain, ReferenceLibrary};

const DPM: usize = 0;
const RPM: usize = 1;
const BIO: usize = 2;
const HUM: usize = 3;

/// RothC's constants, from the packs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RothC {
    /// Rate constants (1/yr): DPM, RPM, BIO, HUM.
    pub k: [f64; 4],
    /// Temperature factor `a = num / (1 + exp(exp_coeff / (T + offset)))`.
    pub temp_num: f64,
    pub temp_exp: f64,
    pub temp_offset_c: f64,
    pub cover_vegetated: f64,
    pub cover_bare: f64,
    /// Plant input split into DPM for woodland and for grassland and scrub.
    pub dpm_woodland: f64,
    pub dpm_scrub: f64,
    /// Farmyard manure split (DPM, RPM, HUM).
    pub manure: [f64; 3],
    /// Share of decomposed carbon that forms BIO + HUM, `1 / (x + 1)`.
    pub to_biohum: f64,
    pub bio_share: f64,
}

impl RothC {
    /// Reject invalid reference parameters before they can create or mutate soil state.
    fn assert_valid(&self) {
        assert!(
            self.k.iter().all(|k| k.is_finite() && *k > 0.0),
            "RothC rate constants must be finite and positive"
        );
        assert!(
            self.temp_num.is_finite() && self.temp_num > 0.0,
            "RothC temperature numerator must be finite and positive"
        );
        assert!(
            self.temp_exp.is_finite(),
            "RothC temperature exponent must be finite"
        );
        assert!(
            self.temp_offset_c.is_finite() && self.temp_offset_c > 0.0,
            "RothC temperature offset must be finite and positive"
        );
        assert!(
            self.cover_vegetated.is_finite() && self.cover_vegetated > 0.0,
            "RothC vegetated cover factor must be finite and positive"
        );
        assert!(
            self.cover_bare.is_finite() && self.cover_bare > 0.0,
            "RothC bare cover factor must be finite and positive"
        );
        assert!(
            (0.0..=1.0).contains(&self.dpm_woodland)
                && self.dpm_woodland.is_finite()
                && (0.0..=1.0).contains(&self.dpm_scrub)
                && self.dpm_scrub.is_finite(),
            "RothC plant DPM fractions must be finite and within [0, 1]"
        );
        assert!(
            self.manure
                .iter()
                .all(|fraction| fraction.is_finite() && *fraction >= 0.0),
            "RothC manure fractions must be finite and non-negative"
        );
        let manure_total: f64 = self.manure.iter().sum();
        assert!(
            manure_total.is_finite() && (manure_total - 1.0).abs() <= 1e-12,
            "RothC manure fractions must conserve incoming carbon"
        );
        assert!(
            self.to_biohum.is_finite() && (0.0..1.0).contains(&self.to_biohum),
            "RothC decomposed-carbon partition must be finite and within [0, 1)"
        );
        assert!(
            self.bio_share.is_finite() && (0.0..=1.0).contains(&self.bio_share),
            "RothC BIO share must be finite and within [0, 1]"
        );
    }

    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let get = |key: &str, row: &str, col: &str| {
            library
                .item(ReferenceDomain::Ecology, key)
                .and_then(|i| i.table())
                .and_then(|t| t.get(row, col))
                .ok_or_else(|| format!("{key}/{row}/{col}"))
        };
        let k = |row| get("rothc_rate_constants", row, "k");
        let temp = |row| get("rothc_temperature_factor", row, "value");
        let part = |row| get("rothc_co2_biohum_partition", row, "value");
        let split = |row, col| get("rothc_dpm_rpm_ratio", row, col);
        let clay = part("rothamsted_clay_percent")?;
        let x = part("s_scaling")?
            * (part("p")? + part("q")? * (-part("r_per_percent_clay")? * clay).exp());
        let params = Self {
            k: [k("dpm")?, k("rpm")?, k("bio")?, k("hum")?],
            temp_num: temp("a_numerator")?,
            temp_exp: temp("b_exponent")?,
            temp_offset_c: temp("c_offset_c")?,
            cover_vegetated: get("rothc_cover_factor", "vegetated", "c")?,
            cover_bare: get("rothc_cover_factor", "bare", "c")?,
            dpm_woodland: split("deciduous_or_tropical_woodland", "dpm_fraction")?,
            dpm_scrub: split("unimproved_grassland_and_scrub", "dpm_fraction")?,
            manure: [
                split("farmyard_manure", "dpm_fraction")?,
                split("farmyard_manure", "rpm_fraction")?,
                split("farmyard_manure", "hum_fraction")?,
            ],
            to_biohum: 1.0 / (x + 1.0),
            bio_share: part("bio_share_of_biohum")?,
        };
        params.assert_valid();
        Ok(params)
    }

    /// RothC's temperature modifier at a mean air temperature (°C). It is
    /// zero at and below -18.27 °C, where the formula's denominator turns.
    pub fn temperature_factor(&self, t_c: f64) -> f64 {
        assert!(t_c.is_finite(), "RothC temperature must be finite");
        let d = t_c + self.temp_offset_c;
        if d <= 0.0 {
            return 0.0;
        }
        self.temp_num / (1.0 + (self.temp_exp / d).exp())
    }

    /// The DPM share of plant carbon falling under `biome`, or `None` where
    /// nothing grows (water).
    pub fn plant_dpm_share(&self, biome: BiomeType) -> Option<f64> {
        use BiomeType::*;
        match biome {
            TropicalRainforest | TropicalDryForest | TemperateForest | MontaneForest
            | BorealForest | Woodland => Some(self.dpm_woodland),
            Shrubland | Savanna | Grassland | Tundra | Alpine | Desert | SemiDesert | Wetland
            | Volcanic | IceSheet => Some(self.dpm_scrub),
            River | DeepOcean | ShallowOcean | CoastalWaters | ReefSea => None,
        }
    }
}

/// What reaches one cell's soil in a step (kg C m⁻²).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SoilInputs {
    pub litter: f64,
    pub faeces: f64,
    pub carcasses: f64,
}

fn assert_soil_input(input: SoilInputs) {
    assert!(
        input.litter.is_finite() && input.litter >= 0.0,
        "soil litter input must be finite and non-negative"
    );
    assert!(
        input.faeces.is_finite() && input.faeces >= 0.0,
        "soil faeces input must be finite and non-negative"
    );
    assert!(
        input.carcasses.is_finite() && input.carcasses >= 0.0,
        "soil carcass input must be finite and non-negative"
    );
    assert!(
        (input.litter + input.faeces + input.carcasses).is_finite(),
        "total soil carbon input must be finite"
    );
}

fn assert_soil_pools(pools: &[f64; 4]) {
    assert!(
        pools.iter().all(|pool| pool.is_finite() && *pool >= 0.0),
        "RothC soil pools must be finite and non-negative"
    );
    assert!(
        pools.iter().sum::<f64>().is_finite(),
        "total RothC soil carbon must be finite"
    );
}

/// The soil's four active compartments under every medium cell (kg C m⁻²).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoilCarbon {
    pub params: RothC,
    pub pools: Vec<[f64; 4]>,
    /// Each cell's year-average temperature modifier `a`.
    pub temperature_factor: Vec<f64>,
    /// Carbon decomposed to CO2 since the soil was laid down (kg C m⁻²,
    /// summed over cells).
    pub respired_kgc_m2: f64,
}

impl SoilCarbon {
    /// RothC's steady state for every cell: what the pools hold when a
    /// constant `input` of plant carbon (kg C m⁻² yr⁻¹, the cell's
    /// production) has fallen for long enough, under the cell's temperature
    /// and cover. Each plant compartment holds input/(m k); all of it
    /// eventually decomposes, `D = I / (1 - e)` passes through in all, and
    /// BIO and HUM hold their shares of `e D` over their own `m k`.
    pub fn steady_state(
        params: RothC,
        biomes: &[BiomeType],
        input_kgc_m2_yr: &[f64],
        temperature_factor: Vec<f64>,
        vegetated: &[bool],
    ) -> Self {
        params.assert_valid();
        assert_eq!(
            biomes.len(),
            input_kgc_m2_yr.len(),
            "RothC biome and annual-input vectors must have equal lengths"
        );
        assert_eq!(
            biomes.len(),
            temperature_factor.len(),
            "RothC biome and temperature-factor vectors must have equal lengths"
        );
        assert_eq!(
            biomes.len(),
            vegetated.len(),
            "RothC biome and vegetation vectors must have equal lengths"
        );
        assert!(
            input_kgc_m2_yr
                .iter()
                .all(|input| input.is_finite() && *input >= 0.0),
            "RothC annual inputs must be finite and non-negative"
        );
        assert!(
            temperature_factor
                .iter()
                .all(|factor| factor.is_finite() && *factor >= 0.0),
            "RothC temperature factors must be finite and non-negative"
        );
        let pools = biomes
            .iter()
            .zip(input_kgc_m2_yr)
            .zip(&temperature_factor)
            .zip(vegetated)
            .map(|(((biome, input), a), veg)| {
                let Some(dpm_share) = params.plant_dpm_share(*biome) else {
                    return [0.0; 4];
                };
                let m = a * if *veg {
                    params.cover_vegetated
                } else {
                    params.cover_bare
                };
                assert!(
                    m.is_finite(),
                    "RothC steady-state decay modifier must be finite"
                );
                let input = *input;
                if m <= 0.0 || input <= 0.0 {
                    return [0.0; 4];
                }
                let e = params.to_biohum;
                let through = input / (1.0 - e);
                let pools = [
                    dpm_share * input / (m * params.k[DPM]),
                    (1.0 - dpm_share) * input / (m * params.k[RPM]),
                    params.bio_share * e * through / (m * params.k[BIO]),
                    (1.0 - params.bio_share) * e * through / (m * params.k[HUM]),
                ];
                assert_soil_pools(&pools);
                pools
            })
            .collect();
        Self {
            params,
            pools,
            temperature_factor,
            respired_kgc_m2: 0.0,
        }
    }

    /// Total soil carbon in a cell (kg C m⁻²).
    pub fn total(&self, i: usize) -> f64 {
        self.pools[i].iter().sum()
    }

    /// One step of `dt_years`: inputs arrive, then every compartment
    /// decomposes as RothC says. Returns the carbon respired as CO2 over the
    /// step, summed over cells (kg C m⁻²).
    pub fn step(
        &mut self,
        dt_years: f64,
        biomes: &[BiomeType],
        inputs: &[SoilInputs],
        vegetated: &[bool],
    ) -> f64 {
        let p = self.params;
        p.assert_valid();
        assert!(
            dt_years.is_finite() && dt_years >= 0.0,
            "RothC step duration must be finite and non-negative"
        );
        assert_eq!(
            self.pools.len(),
            biomes.len(),
            "RothC pool and biome vectors must have equal lengths"
        );
        assert_eq!(
            self.pools.len(),
            inputs.len(),
            "RothC pool and input vectors must have equal lengths"
        );
        assert_eq!(
            self.pools.len(),
            self.temperature_factor.len(),
            "RothC pool and temperature-factor vectors must have equal lengths"
        );
        assert_eq!(
            self.pools.len(),
            vegetated.len(),
            "RothC pool and vegetation vectors must have equal lengths"
        );
        assert!(
            self.respired_kgc_m2.is_finite() && self.respired_kgc_m2 >= 0.0,
            "cumulative soil respiration must be finite and non-negative"
        );
        // Validate every caller-supplied vector before mutating the first cell.
        for input in inputs {
            assert_soil_input(*input);
        }
        for factor in &self.temperature_factor {
            assert!(
                factor.is_finite() && *factor >= 0.0,
                "RothC temperature factors must be finite and non-negative"
            );
        }
        let mut respired = 0.0;
        for (i, pools) in self.pools.iter_mut().enumerate() {
            assert_soil_pools(pools);
            respired += p.step_cell_unchecked(
                pools,
                biomes[i],
                inputs[i],
                self.temperature_factor[i],
                vegetated[i],
                dt_years,
            );
            assert!(
                respired.is_finite(),
                "RothC cumulative step respiration must remain finite"
            );
        }
        let cumulative_respired = self.respired_kgc_m2 + respired;
        assert!(
            cumulative_respired.is_finite(),
            "cumulative soil respiration must remain finite"
        );
        self.respired_kgc_m2 = cumulative_respired;
        respired
    }
}

impl RothC {
    /// [`SoilCarbon::step`] for one cell's `pools` under temperature
    /// modifier `a`: returns the carbon respired (kg C m⁻²), zero where the
    /// biome has no soil.
    pub fn step_cell(
        &self,
        pools: &mut [f64; 4],
        biome: BiomeType,
        input: SoilInputs,
        a: f64,
        vegetated: bool,
        dt_years: f64,
    ) -> f64 {
        self.assert_valid();
        assert!(
            dt_years.is_finite() && dt_years >= 0.0,
            "RothC step duration must be finite and non-negative"
        );
        assert_soil_input(input);
        assert!(
            a.is_finite() && a >= 0.0,
            "RothC temperature factor must be finite and non-negative"
        );
        assert_soil_pools(pools);
        let incoming = input.litter + input.faeces + input.carcasses;
        assert!(
            (pools.iter().sum::<f64>() + incoming).is_finite(),
            "RothC soil carbon plus incoming material must remain finite"
        );
        self.step_cell_unchecked(pools, biome, input, a, vegetated, dt_years)
    }

    fn step_cell_unchecked(
        &self,
        pools: &mut [f64; 4],
        biome: BiomeType,
        input: SoilInputs,
        a: f64,
        vegetated: bool,
        dt_years: f64,
    ) -> f64 {
        let p = self;
        let Some(dpm_share) = p.plant_dpm_share(biome) else {
            return 0.0;
        };
        let cover = if vegetated {
            p.cover_vegetated
        } else {
            p.cover_bare
        };
        let m = a * cover;
        assert!(m.is_finite(), "RothC decay modifier must remain finite");
        pools[DPM] += dpm_share * input.litter + p.manure[0] * input.faeces + input.carcasses;
        pools[RPM] += (1.0 - dpm_share) * input.litter + p.manure[1] * input.faeces;
        pools[HUM] += p.manure[2] * input.faeces;
        let mut decomposed = 0.0;
        for (pool, k) in pools.iter_mut().zip(p.k) {
            let lost = *pool * (1.0 - (-m * k * dt_years).exp());
            *pool -= lost;
            decomposed += lost;
        }
        let formed = decomposed * p.to_biohum;
        pools[BIO] += formed * p.bio_share;
        pools[HUM] += formed * (1.0 - p.bio_share);
        decomposed - formed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> RothC {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        RothC::from_reference(&library).unwrap()
    }

    #[test]
    fn the_packs_reproduce_the_documents_own_numbers() {
        let p = params();
        // Section 1.7: x = 3.51 at Rothamsted's 23.4% clay.
        let x = 1.0 / p.to_biohum - 1.0;
        assert!((x - 3.51).abs() < 0.01, "x = {x}");
        // The temperature modifier rises with temperature over the range
        // the island sees, and is zero where the formula stops applying.
        let mut last = 0.0;
        for t in [-10.0, 0.0, 5.0, 10.0, 20.0, 30.0] {
            let a = p.temperature_factor(t);
            assert!(a > last, "a({t}) = {a} did not rise");
            last = a;
        }
        assert_eq!(p.temperature_factor(-19.0), 0.0);
        assert_eq!(p.plant_dpm_share(BiomeType::TemperateForest), Some(0.2));
        assert_eq!(p.plant_dpm_share(BiomeType::DeepOcean), None);
    }

    /// Carbon is neither made nor lost: what was there plus what came in is
    /// what is there now plus what was respired.
    #[test]
    fn a_step_conserves_carbon() {
        let p = params();
        let biomes = [BiomeType::TemperateForest, BiomeType::Grassland];
        let mut soil =
            SoilCarbon::steady_state(p, &biomes, &[0.5, 0.3], vec![1.0, 1.4], &[true, false]);
        let before: f64 = (0..2).map(|i| soil.total(i)).sum();
        let inputs = [
            SoilInputs {
                litter: 0.01,
                faeces: 0.002,
                carcasses: 0.001,
            },
            SoilInputs::default(),
        ];
        let respired = soil.step(0.25, &biomes, &inputs, &[true, false]);
        let after: f64 = (0..2).map(|i| soil.total(i)).sum();
        assert!(((before + 0.013) - (after + respired)).abs() < 1e-12);
    }

    #[test]
    fn steady_state_rejects_mismatched_cell_vectors() {
        let p = params();
        let biomes = [BiomeType::TemperateForest];
        let result = std::panic::catch_unwind(|| {
            SoilCarbon::steady_state(p, &biomes, &[], vec![1.0], &[true]);
        });
        assert!(result.is_err());
    }

    #[test]
    fn step_preflights_all_inputs_before_mutating_any_cell() {
        use std::panic::AssertUnwindSafe;

        let p = params();
        let biomes = [BiomeType::TemperateForest, BiomeType::Grassland];
        let mut soil =
            SoilCarbon::steady_state(p, &biomes, &[0.5, 0.3], vec![1.0, 1.4], &[true, false]);
        let before = soil.clone();
        let inputs = [
            SoilInputs {
                litter: 0.01,
                ..SoilInputs::default()
            },
            SoilInputs {
                litter: f64::NAN,
                ..SoilInputs::default()
            },
        ];

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            soil.step(0.25, &biomes, &inputs, &[true, false]);
        }));

        assert!(result.is_err());
        assert_eq!(
            soil, before,
            "invalid input must not partially advance soil"
        );
    }

    #[test]
    fn step_cell_rejects_negative_duration_without_mutating_pools() {
        use std::panic::AssertUnwindSafe;

        let p = params();
        let mut pools = [1.0, 0.5, 0.2, 0.1];
        let before = pools;
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            p.step_cell(
                &mut pools,
                BiomeType::TemperateForest,
                SoilInputs::default(),
                1.0,
                true,
                -0.25,
            );
        }));

        assert!(result.is_err());
        assert_eq!(pools, before);
    }

    /// The steady state is one: fed its own input, it stays put.
    #[test]
    fn the_steady_state_holds_under_its_own_input() {
        let p = params();
        let biomes = [BiomeType::TemperateForest];
        let mut soil = SoilCarbon::steady_state(p, &biomes, &[0.6], vec![0.9], &[true]);
        let start = soil.pools[0];
        let dt = 1.0 / 1460.0;
        for _ in 0..(1460 * 5) {
            soil.step(
                dt,
                &biomes,
                &[SoilInputs {
                    litter: 0.6 * dt,
                    ..SoilInputs::default()
                }],
                &[true],
            );
        }
        for (now, then) in soil.pools[0].iter().zip(start) {
            assert!(
                // Inputs land before each step's decay, so the stepped
                // equilibrium sits about k dt / 2 off the continuous one.
                (now - then).abs() <= 1e-2 * then.max(1e-9),
                "{now} drifted from {then}"
            );
        }
    }
}
