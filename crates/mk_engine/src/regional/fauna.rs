//! A first grazer layer for the island: one herbivore carbon pool per land
//! cell that eats a capped share of the cell's plant production.
//!
//! **Every parameter below is an assumption, not a sourced value** (the
//! repository's reference packs have no herbivore data); they are recorded as
//! deviation D41. The structure is chosen to be stable and to account for
//! every kilogram of carbon, not to be a validated ecology:
//!
//! - Intake is `min(INTAKE_PER_YEAR * H, MAX_INTAKE_FRACTION_OF_NPP * NPP)`,
//!   so the herbivores can never take more than a fixed share of what the
//!   plants produce (the plants' equilibrium moves by at most that share) and
//!   there are no predator-prey oscillations: intake does not depend on the
//!   plants' standing biomass.
//! - Of every kilogram eaten, [`NET_PRODUCTION_EFFICIENCY`] becomes new
//!   herbivore tissue, [`FAECES_FRACTION`] goes to detritus, and the rest is
//!   respired. Deaths ([`MORTALITY_PER_YEAR`]) go to detritus.
//! - The equilibrium is `H* = EFF * MAX_INTAKE_FRACTION * NPP / MORTALITY`
//!   (0.00625 x NPP with the values below: about 3 g C/m2 at an NPP of 0.5 kg
//!   C/m2/yr, the right order of magnitude for grazer biomass).
//!
//! Not modelled: predators, fish, birds, migration or dispersal (so a cell
//! whose herbivores have died out stays empty even if plants return), seasons,
//! hunting by people (nothing in the household hunts), and the fauna's own
//! oxygen.

/// Carbon eaten per unit herbivore carbon per year when food is not limiting
/// (about 3% of body mass a day). Assumption.
pub const INTAKE_PER_YEAR: f64 = 12.0;
/// The most of a cell's NPP the herbivores may eat. Assumption.
pub const MAX_INTAKE_FRACTION_OF_NPP: f64 = 0.05;
/// Share of the carbon eaten that becomes herbivore tissue (the "10% rule"
/// is the usual order; half of that here). Assumption.
pub const NET_PRODUCTION_EFFICIENCY: f64 = 0.05;
/// Share of the carbon eaten that leaves undigested as faeces. Assumption.
pub const FAECES_FRACTION: f64 = 0.5;
/// Annual death rate of herbivore carbon (a mean life of 2.5 years). Assumption.
pub const MORTALITY_PER_YEAR: f64 = 0.4;

/// The herbivore density a cell with `npp_kgc_m2_yr` settles at.
pub fn equilibrium_kgc_m2(npp_kgc_m2_yr: f64) -> f64 {
    NET_PRODUCTION_EFFICIENCY * MAX_INTAKE_FRACTION_OF_NPP * npp_kgc_m2_yr.max(0.0)
        / MORTALITY_PER_YEAR
}

/// Plant carbon the herbivores in a cell eat per year (kg C/m2/yr).
pub fn intake_kgc_m2_yr(herbivore_kgc_m2: f64, npp_kgc_m2_yr: f64) -> f64 {
    (INTAKE_PER_YEAR * herbivore_kgc_m2.max(0.0))
        .min(MAX_INTAKE_FRACTION_OF_NPP * npp_kgc_m2_yr.max(0.0))
}

/// Where one cell's carbon went over one step (kg C/m2).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CellFlows {
    /// Eaten from the plants.
    pub eaten: f64,
    /// Into new herbivore tissue.
    pub growth: f64,
    /// Out as faeces, to detritus.
    pub faeces: f64,
    /// Respired to the air.
    pub respired: f64,
    /// Dead herbivores, to detritus.
    pub deaths: f64,
}

/// Advance one cell's herbivore carbon by `dt_years`. Returns the new density
/// and the flows; `eaten == growth + faeces + respired` and
/// `new == old + growth - deaths` hold exactly.
pub fn step_cell(herbivore_kgc_m2: f64, npp_kgc_m2_yr: f64, dt_years: f64) -> (f64, CellFlows) {
    let h = herbivore_kgc_m2.max(0.0);
    if !(dt_years.is_finite() && dt_years > 0.0) || npp_kgc_m2_yr <= 0.0 {
        // No plants here: whatever lives there dies back.
        let deaths = h;
        return (
            0.0,
            CellFlows {
                deaths,
                ..CellFlows::default()
            },
        );
    }
    let eaten = intake_kgc_m2_yr(h, npp_kgc_m2_yr) * dt_years;
    let growth = NET_PRODUCTION_EFFICIENCY * eaten;
    let faeces = FAECES_FRACTION * eaten;
    let respired = eaten - growth - faeces;
    // Exact solution of dH/dt = -m H over the step for the deaths, so a long
    // step cannot overshoot below zero.
    let deaths = h * (1.0 - (-MORTALITY_PER_YEAR * dt_years).exp());
    let new = (h + growth - deaths).max(0.0);
    (
        new,
        CellFlows {
            eaten,
            growth,
            faeces,
            respired,
            deaths,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kilogram_eaten_is_accounted_for_and_the_pool_balances() {
        for &(h, npp, dt) in &[
            (0.0, 0.5, 0.01),
            (0.002, 0.5, 0.01),
            (0.5, 0.5, 0.25),
            (0.003, 0.05, 1.0),
        ] {
            let (new, f) = step_cell(h, npp, dt);
            assert!(
                (f.eaten - (f.growth + f.faeces + f.respired)).abs() < 1e-15,
                "{f:?}"
            );
            assert!(
                (new - (h + f.growth - f.deaths)).abs() < 1e-15 || new == 0.0,
                "{h} -> {new} {f:?}"
            );
            assert!(f.respired >= 0.0 && f.faeces >= 0.0 && f.growth >= 0.0 && f.deaths >= 0.0);
        }
    }

    #[test]
    fn herbivores_never_eat_more_than_their_share_of_production() {
        for npp in [0.01, 0.1, 0.5, 1.2] {
            for h in [0.0, 1e-4, 1e-2, 1.0, 100.0] {
                let eaten_per_year = intake_kgc_m2_yr(h, npp);
                assert!(
                    eaten_per_year <= MAX_INTAKE_FRACTION_OF_NPP * npp + 1e-15,
                    "{h} {npp}"
                );
            }
        }
    }

    #[test]
    fn any_start_settles_at_the_equilibrium_without_oscillating() {
        let npp = 0.5;
        let want = equilibrium_kgc_m2(npp);
        // An empty cell stays empty: there is no dispersal to recolonise it.
        assert_eq!(step_cell(0.0, npp, 1.0).0, 0.0);
        for start in [want * 0.01, want, want * 10.0] {
            let (mut h, mut last) = (start, start);
            let mut turned = 0;
            let mut rising = None::<bool>;
            for _ in 0..(40 * 365) {
                let (next, _) = step_cell(h, npp, 1.0 / 365.0);
                let now_rising = next > last;
                if rising.is_some() && rising != Some(now_rising) && (next - last).abs() > 1e-12 {
                    turned += 1;
                }
                rising = Some(now_rising);
                last = h;
                h = next;
            }
            assert!(
                (h - want).abs() < 1e-3 * want.max(1e-9) + 1e-6,
                "start {start}: {h} vs {want}"
            );
            assert!(
                turned <= 1,
                "start {start} changed direction {turned} times"
            );
        }
    }

    #[test]
    fn a_step_length_does_not_change_the_answer_much() {
        let (daily, _) = (0..365).fold((0.001, ()), |(h, _), _| {
            (step_cell(h, 0.5, 1.0 / 365.0).0, ())
        });
        let (quarterly, _) = (0..4).fold((0.001, ()), |(h, _), _| (step_cell(h, 0.5, 0.25).0, ()));
        assert!(
            (daily - quarterly).abs() / daily < 0.05,
            "{daily} vs {quarterly}"
        );
    }

    #[test]
    fn without_plants_the_herbivores_die_back_to_detritus() {
        let (new, f) = step_cell(0.01, 0.0, 0.1);
        assert_eq!(new, 0.0);
        assert_eq!(f.deaths, 0.01);
        assert_eq!(f.eaten, 0.0);
    }
}
