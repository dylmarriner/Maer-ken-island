//! Green timber drying in the air (`WORK_REMAINING.md` §1a, D40).
//!
//! A felled log holds the water the tree drew up. Left in the air it loses
//! it toward the wood's **equilibrium moisture content** for that air: the
//! Wood Handbook's equation (4-5) (Simpson 1973), which the packs carry
//! with its Celsius coefficients and which reproduces the handbook's own
//! Table 4-2 to its printed precision.
//!
//! How fast is fitted, not assumed. Simpson & Hart (2000, FPL-GTR-121)
//! tabulate how long 29 mm northern red oak, stacked green at 80%, takes
//! to reach 30, 25 and 20% in Madison for every half-month of the year.
//! Their simulation adjusts its diffusion coefficient with the saturated
//! vapour pressure of water; here the moisture content above equilibrium
//! decays at
//!
//! ```text
//! k = k0 (e_s(T) / e_s(20 °C))^p (L_ref / L)^2
//! ```
//!
//! with `k0` and `p` fitted to all 72 of the table's times, integrating
//! through the year's conditions (`tests::the_fit_reproduces_the_madison_table`
//! runs that check), `L_ref` the board's half-thickness and `L` the
//! diffusion length of the piece, its radius for a round log: drying by
//! diffusion takes four times as long at twice the thickness.
//!
//! What it leaves out (D40): the end grain, through which a short piece
//! dries faster than its radius says; the bark, which slows a whole log;
//! species (the rate is northern red oak's); rain wetting a log again
//! (wood here only ever dries toward the air).

use crate::validation::{ReferenceDomain, ReferenceLibrary};

use super::timber::Log;

/// The equilibrium-moisture equation's parameters, each `c0 + c1 T + c2 T²`
/// with `T` in °C, in the order W, K, K1, K2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodDrying {
    pub emc: [[f64; 3]; 4],
    /// The fitted rate for the reference board at 20 °C (per day).
    pub k0_per_day: f64,
    /// How the rate follows the saturated vapour pressure.
    pub vapour_pressure_exponent: f64,
    /// Half the reference board's thickness (m): its diffusion length.
    pub reference_half_thickness_m: f64,
}

const DAY_S: f64 = 86_400.0;

/// Saturated vapour pressure of water (Pa) at `t_c` °C: Bolton (1980),
/// Mon. Weather Rev. 108:1046, as the boundary forcing uses.
pub fn saturation_vapour_pressure_pa(t_c: f64) -> f64 {
    611.2 * (17.67 * t_c / (t_c + 243.5)).exp()
}

impl WoodDrying {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let table = |key: &str| {
            library
                .item(ReferenceDomain::Labour, key)
                .and_then(|i| i.table())
                .ok_or_else(|| format!("labour/{key} is missing"))
        };
        let emc_table = table("emc_equation_celsius")?;
        let mut emc = [[0.0; 3]; 4];
        for (row, name) in emc.iter_mut().zip(["W", "K", "K1", "K2"]) {
            for (c, col) in row.iter_mut().zip(["c0", "c1", "c2"]) {
                *c = emc_table
                    .get(name, col)
                    .ok_or_else(|| format!("emc_equation_celsius {name}/{col} is missing"))?;
            }
        }
        let fit = table("air_drying_rate_fit")?;
        let board = table("air_drying_reference_board")?;
        let value = |t: &crate::validation::ReferenceTable, row: &str| {
            t.get(row, "value")
                .ok_or_else(|| format!("{row} is missing"))
        };
        Ok(Self {
            emc,
            k0_per_day: value(fit, "k0_per_day_at_20c")?,
            vapour_pressure_exponent: value(fit, "vapour_pressure_exponent")?,
            reference_half_thickness_m: value(board, "thickness_mm")? / 2_000.0,
        })
    }

    /// Equilibrium moisture content (water per kg of oven-dry wood) in air
    /// at `t_c` °C and relative humidity `rh` (0-1).
    pub fn equilibrium_moisture(&self, t_c: f64, rh: f64) -> f64 {
        let at = |i: usize| self.emc[i][0] + self.emc[i][1] * t_c + self.emc[i][2] * t_c * t_c;
        let (w, k, k1, k2) = (at(0), at(1), at(2), at(3));
        // Saturated air would put the equation's pole at Kh = 1; the
        // handbook's own table stops at 98%.
        let h = rh.clamp(0.0, 0.98);
        let kh = k * h;
        let percent = 1_800.0 / w
            * (kh / (1.0 - kh)
                + (k1 * kh + 2.0 * k1 * k2 * kh * kh) / (1.0 + k1 * kh + k1 * k2 * kh * kh));
        (percent / 100.0).max(0.0)
    }

    /// The rate (per second) at which a piece with diffusion length
    /// `half_thickness_m` approaches equilibrium at `t_c` °C.
    pub fn rate_per_s(&self, t_c: f64, half_thickness_m: f64) -> f64 {
        if !(half_thickness_m.is_finite() && half_thickness_m > 0.0) {
            return 0.0;
        }
        let vapour = (saturation_vapour_pressure_pa(t_c) / saturation_vapour_pressure_pa(20.0))
            .powf(self.vapour_pressure_exponent);
        let thickness = (self.reference_half_thickness_m / half_thickness_m).powi(2);
        self.k0_per_day * vapour * thickness / DAY_S
    }

    /// Dry `log` for `dt_s` in air at `t_c` °C and `rh`; returns the water
    /// it gave up (kg). A log already at or below equilibrium is left as it
    /// is: nothing here wets wood.
    pub fn dry(&self, log: &mut Log, t_c: f64, rh: f64, dt_s: f64) -> f64 {
        if !(dt_s.is_finite() && dt_s > 0.0) || log.dry_kg <= 0.0 {
            return 0.0;
        }
        let equilibrium = self.equilibrium_moisture(t_c, rh);
        let now = log.moisture_content();
        if now <= equilibrium {
            return 0.0;
        }
        let k = self.rate_per_s(t_c, log.diameter_m / 2.0);
        let next = equilibrium + (now - equilibrium) * (-k * dt_s).exp();
        let lost = ((now - next) * log.dry_kg).clamp(0.0, log.water_kg);
        log.water_kg -= lost;
        lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> ReferenceLibrary {
        ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap()
    }

    fn drying() -> WoodDrying {
        WoodDrying::from_reference(&library()).unwrap()
    }

    fn table(key: &str) -> crate::validation::ReferenceTable {
        library()
            .item(ReferenceDomain::Labour, key)
            .and_then(|i| i.table())
            .cloned()
            .unwrap()
    }

    #[test]
    fn the_equation_reproduces_the_handbooks_table() {
        let d = drying();
        let points = table("emc_table_points");
        for row in &points.rows {
            let (t, rh, expected) = (row.values[0], row.values[1], row.values[2]);
            let got = 100.0 * d.equilibrium_moisture(t, rh);
            // The table prints one decimal.
            assert!(
                (got - expected).abs() <= 0.06,
                "{}: {got:.2}% against {expected}%",
                row.label
            );
        }
    }

    /// The day-by-day drying the fit was made with, from the Madison
    /// table's own year: the conditions of each half-month, linearly
    /// between the listed dates.
    fn madison_days_to(d: &WoodDrying, rows: &[(f64, f64, f64)], start: f64, target: f64) -> f64 {
        let at = |day: f64| {
            let day = day.rem_euclid(365.0);
            let n = rows.len();
            let i = rows.iter().rposition(|r| r.0 <= day).unwrap_or(n - 1);
            let (a, b) = (rows[i], rows[(i + 1) % n]);
            let span = (b.0 - a.0).rem_euclid(365.0);
            let f = ((day - a.0).rem_euclid(365.0) / span).clamp(0.0, 1.0);
            (a.1 + f * (b.1 - a.1), a.2 + f * (b.2 - a.2))
        };
        let board = table("air_drying_reference_board");
        let green = board.get("green_moisture_percent", "value").unwrap() / 100.0;
        let half = d.reference_half_thickness_m;
        let (mut mc, mut t) = (green, 0.0);
        let dt_days = 0.25;
        while t < 1_000.0 {
            let (t_c, rh) = at(start + t);
            let e = d.equilibrium_moisture(t_c, rh);
            let k = d.rate_per_s(t_c, half) * DAY_S;
            mc = e + (mc - e) * (-k * dt_days).exp();
            t += dt_days;
            if mc <= target {
                return t;
            }
        }
        f64::INFINITY
    }

    #[test]
    fn the_fit_reproduces_the_madison_table() {
        let d = drying();
        let madison = table("air_drying_madison_red_oak");
        let rows: Vec<(f64, f64, f64)> = madison
            .rows
            .iter()
            .map(|r| (r.values[0], (r.values[1] - 32.0) / 1.8, r.values[2] / 100.0))
            .collect();
        let mut errors = Vec::new();
        for r in &madison.rows {
            for (target, col) in [(0.30, 3), (0.25, 4), (0.20, 5)] {
                let model = madison_days_to(&d, &rows, r.values[0], target);
                errors.push((model / r.values[col] - 1.0).abs());
            }
        }
        errors.sort_by(f64::total_cmp);
        let median = errors[errors.len() / 2];
        let worst = errors[errors.len() - 1];
        assert!(median < 0.05, "median error {median:.3}");
        assert!(worst < 0.25, "worst error {worst:.3}");
    }

    fn log(diameter_m: f64, mc: f64) -> Log {
        Log {
            id: 1,
            position_m: (0.0, 0.0),
            heading_rad: 0.0,
            length_m: 3.0,
            diameter_m,
            dry_kg: 100.0,
            carbon_kgc: 50.0,
            water_kg: 100.0 * mc,
            place: super::super::timber::LogPlace::Stored,
        }
    }

    #[test]
    fn warm_dry_air_dries_faster_than_cold_wet_air_and_never_below_equilibrium() {
        let d = drying();
        let week = 7.0 * DAY_S;
        let mut warm = log(0.1, 0.8);
        let mut cold = log(0.1, 0.8);
        let warm_lost = d.dry(&mut warm, 25.0, 0.5, week);
        let cold_lost = d.dry(&mut cold, 2.0, 0.9, week);
        assert!(
            warm_lost > 2.0 * cold_lost,
            "{warm_lost} against {cold_lost}"
        );
        // Years in warm dry air: it settles at, not below, equilibrium.
        let mut long = log(0.05, 0.8);
        for _ in 0..200 {
            d.dry(&mut long, 25.0, 0.5, 30.0 * DAY_S);
        }
        let floor = d.equilibrium_moisture(25.0, 0.5);
        assert!(long.moisture_content() >= floor - 1e-12);
        assert!(long.moisture_content() - floor < 1e-3);
        // Wood below equilibrium gains nothing.
        let mut dry = log(0.1, 0.05);
        assert_eq!(d.dry(&mut dry, 10.0, 0.95, week), 0.0);
        assert_eq!(dry.moisture_content(), 0.05);
    }

    #[test]
    fn twice_the_thickness_takes_four_times_as_long() {
        let d = drying();
        let thin = d.rate_per_s(15.0, 0.05);
        let thick = d.rate_per_s(15.0, 0.10);
        assert!((thin / thick - 4.0).abs() < 1e-9);
    }

    #[test]
    fn steps_compose() {
        let d = drying();
        let mut one = log(0.2, 0.8);
        let mut many = log(0.2, 0.8);
        d.dry(&mut one, 12.0, 0.7, 24.0 * 3_600.0);
        for _ in 0..24 {
            d.dry(&mut many, 12.0, 0.7, 3_600.0);
        }
        assert!((one.water_kg - many.water_kg).abs() < 1e-9);
    }
}
