//! Small hydropower (Phase 4b Tasks 1 and 3).
//!
//! A plant takes water from a river at an intake, carries it down a
//! penstock and passes it through a turbine at a powerhouse below, so its
//! electrical output is
//!
//! `P = rho g Q (H_g - h_f(Q)) e_t(Q) e_g`
//!
//! (RETScreen 2004, Small Hydro, Eq. 7): `Q` the flow through the turbine,
//! `H_g` the gross head, `h_f` the penstock's friction loss (Darcy-Weisbach,
//! friction factor from Swamee & Jain 1976), `e_t` the turbine's efficiency
//! at that flow (RETScreen Appendix A, curves fitted to manufacturers'
//! data) and `e_g` the generator's (ESHA 2004 Table 6.8). RETScreen's
//! transformer and parasitic losses are user inputs with no reference
//! value; the plant here has neither (it feeds the estate at generator
//! voltage).
//!
//! The turbine is chosen among the types whose head range takes the net
//! head (ESHA 2004 Table 6.3) as the one that makes the most energy from
//! the river's own flows. A Turgo has no RETScreen curve of its own; it
//! takes the Pelton's shape at its own best efficiency (ESHA Table 6.7:
//! 0.85 against a one-jet Pelton's 0.89).

use serde::{Deserialize, Serialize};

/// Density of fresh water (kg/m^3), as RETScreen takes it.
pub const WATER_DENSITY_KG_M3: f64 = 1_000.0;
/// Standard gravity (m/s^2).
pub const GRAVITY_M_S2: f64 = 9.80665;
/// Kinematic viscosity of fresh water at 10 C (m^2/s; Anton Paar's measured
/// table, 1.3063 mm^2/s).
pub const WATER_KINEMATIC_VISCOSITY_M2_S: f64 = 1.3063e-6;
/// Absolute roughness of a plastic penstock wall (m): drawn tubing and PVC,
/// 0.0015 mm (Moody 1944, as tabulated).
pub const PLASTIC_PIPE_ROUGHNESS_M: f64 = 1.5e-6;
/// Absolute roughness of a commercial steel penstock (m): 0.046 mm.
pub const STEEL_PIPE_ROUGHNESS_M: f64 = 4.6e-5;
/// Penstock friction loss at design flow as a share of the gross head:
/// RETScreen's validation case (5%).
pub const DESIGN_HYDRAULIC_LOSS_FRACTION: f64 = 0.05;
/// Share of the time the firm flow is available (RETScreen: usually 95%).
pub const FIRM_FLOW_EXCEEDANCE: f64 = 0.95;
/// Flow always left in the river, as a share of its mean: Tennant (1976)'s
/// 30%, the base flow that sustains good habitat for most aquatic life.
pub const ENVIRONMENTAL_FLOW_FRACTION: f64 = 0.30;
/// RETScreen's turbine manufacture/design coefficient, its default (range
/// 2.8-6.1).
const MANUFACTURE_COEFFICIENT: f64 = 4.5;

/// ESHA 2004 Table 6.3: net head range (m) of each turbine type.
const KAPLAN_HEAD_M: (f64, f64) = (2.0, 40.0);
const FRANCIS_HEAD_M: (f64, f64) = (25.0, 350.0);
const PELTON_HEAD_M: (f64, f64) = (50.0, 1_300.0);
const CROSSFLOW_HEAD_M: (f64, f64) = (5.0, 200.0);
const TURGO_HEAD_M: (f64, f64) = (50.0, 250.0);
/// ESHA 2004 Table 6.7: best efficiencies of a Turgo and a one-jet Pelton.
const TURGO_BEST_EFFICIENCY: f64 = 0.85;
const PELTON_ONE_JET_BEST_EFFICIENCY: f64 = 0.89;
/// RETScreen lets a Pelton have one to six jets.
const MAX_PELTON_JETS: u8 = 6;
/// ESHA 2004 Table 6.8: best generator efficiency by rated power (kW).
const GENERATOR_EFFICIENCY: [(f64, f64); 6] = [
    (10.0, 0.910),
    (50.0, 0.940),
    (100.0, 0.950),
    (250.0, 0.955),
    (500.0, 0.960),
    (1_000.0, 0.970),
];

/// A small-hydro turbine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurbineKind {
    /// Double-regulated (adjustable runner and guide vanes).
    Kaplan,
    /// Fixed-blade propeller.
    Propeller,
    Francis,
    Pelton {
        jets: u8,
    },
    Turgo,
    Crossflow,
}

impl TurbineKind {
    /// The turbine types whose head range (ESHA 2004 Table 6.3) takes
    /// `net_head_m`; a Pelton once per jet count.
    pub fn for_head(net_head_m: f64) -> Vec<TurbineKind> {
        let within = |(lo, hi): (f64, f64)| net_head_m > lo && net_head_m < hi;
        let mut kinds = Vec::new();
        if within(KAPLAN_HEAD_M) {
            kinds.push(TurbineKind::Kaplan);
            kinds.push(TurbineKind::Propeller);
        }
        if within(FRANCIS_HEAD_M) {
            kinds.push(TurbineKind::Francis);
        }
        if within(PELTON_HEAD_M) {
            kinds.extend((1..=MAX_PELTON_JETS).map(|jets| TurbineKind::Pelton { jets }));
        }
        if within(TURGO_HEAD_M) {
            kinds.push(TurbineKind::Turgo);
        }
        if within(CROSSFLOW_HEAD_M) {
            kinds.push(TurbineKind::Crossflow);
        }
        kinds
    }
}

/// One turbine's efficiency curve (RETScreen 2004, Appendix A).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TurbineCurve {
    pub kind: TurbineKind,
    /// Flow at rated head and full gate opening (m^3/s): the most the
    /// turbine takes.
    pub design_flow_m3_s: f64,
    /// Flow of best efficiency (m^3/s).
    pub peak_flow_m3_s: f64,
    /// Best efficiency.
    pub peak_efficiency: f64,
    /// Efficiency at design flow (Francis only; equals the peak otherwise).
    pub full_load_efficiency: f64,
    /// Specific speed based on flow (reaction turbines; zero otherwise).
    pub specific_speed: f64,
}

/// Rotational speed (rpm) of a Pelton of `jets` jets at `head_m` and
/// `design_flow_m3_s` (RETScreen Appendix A): `31 (h / (Qd / j))^0.5`, the
/// jet's specific speed held constant. Read with the flow divided into the
/// head, it reproduces ESHA 2004's worked example (400 m, 0.42 m^3/s, one
/// jet: about 1,000 rpm).
pub fn pelton_speed_rpm(head_m: f64, design_flow_m3_s: f64, jets: u8) -> f64 {
    let per_jet = design_flow_m3_s / f64::from(jets.max(1));
    31.0 * (head_m / per_jet.max(1e-12)).sqrt()
}

/// Reaction-turbine runner throat diameter (m) for a design flow.
fn reaction_runner_diameter_m(design_flow_m3_s: f64) -> f64 {
    let d = 0.46 * design_flow_m3_s.powf(0.473);
    if d < 1.8 {
        d
    } else {
        0.41 * design_flow_m3_s.powf(0.473)
    }
}

impl TurbineCurve {
    /// The curve of a `kind` turbine designed for `design_flow_m3_s` at
    /// `rated_head_m` (gross head less the hydraulic loss at design flow).
    pub fn new(kind: TurbineKind, design_flow_m3_s: f64, rated_head_m: f64) -> Self {
        let qd = design_flow_m3_s.max(1e-9);
        let h = rated_head_m.max(1e-9);
        let rm = 0.005 * MANUFACTURE_COEFFICIENT - 0.0305;
        let pelton = |jets: u8| {
            let j = f64::from(jets);
            let n = pelton_speed_rpm(h, qd, jets);
            let d = 49.4 * h.sqrt() * j.powf(0.02) / n;
            (0.864 * d.powf(0.04), (0.662 + 0.001 * j) * qd)
        };
        let (peak_efficiency, peak_flow, full_load, nq) = match kind {
            TurbineKind::Francis => {
                let nq = 600.0 * h.powf(-0.5);
                let d = reaction_runner_diameter_m(qd);
                let e_nq = ((nq - 56.0) / 256.0).powi(2);
                let e_d = (0.081 + e_nq) * (1.0 - 0.789 * d.powf(-0.2));
                let ep = 0.919 - e_nq + e_d + rm;
                let drop = 0.0072 * nq.powf(0.4);
                (ep, 0.65 * qd * nq.powf(0.05), (1.0 - drop) * ep, nq)
            }
            TurbineKind::Kaplan | TurbineKind::Propeller => {
                let nq = 800.0 * h.powf(-0.5);
                let d = reaction_runner_diameter_m(qd);
                let e_nq = ((nq - 170.0) / 700.0).powi(2);
                let e_d = (0.095 + e_nq) * (1.0 - 0.789 * d.powf(-0.2));
                let ep = 0.905 - e_nq + e_d + rm;
                let qp = if kind == TurbineKind::Kaplan {
                    0.75 * qd
                } else {
                    qd
                };
                (ep, qp, ep, nq)
            }
            TurbineKind::Pelton { jets } => {
                let (ep, qp) = pelton(jets.clamp(1, MAX_PELTON_JETS));
                (ep, qp, ep, 0.0)
            }
            TurbineKind::Turgo => {
                let (ep, qp) = pelton(1);
                let ep = ep * TURGO_BEST_EFFICIENCY / PELTON_ONE_JET_BEST_EFFICIENCY;
                (ep, qp, ep, 0.0)
            }
            TurbineKind::Crossflow => (0.79, qd, 0.79, 0.0),
        };
        Self {
            kind,
            design_flow_m3_s: qd,
            peak_flow_m3_s: peak_flow.min(qd),
            peak_efficiency,
            full_load_efficiency: full_load,
            specific_speed: nq,
        }
    }

    /// Efficiency at a flow of `flow_m3_s` through the turbine (capped at
    /// the design flow); zero where the curve falls to nothing, which is
    /// below the flow the turbine can run on.
    pub fn efficiency(&self, flow_m3_s: f64) -> f64 {
        if flow_m3_s <= 0.0 {
            return 0.0;
        }
        let q = flow_m3_s.min(self.design_flow_m3_s);
        let (qd, qp, ep) = (
            self.design_flow_m3_s,
            self.peak_flow_m3_s,
            self.peak_efficiency,
        );
        let below = (qp - q) / qp;
        let e = match self.kind {
            TurbineKind::Francis => {
                if q <= qp {
                    (1.0 - 1.25 * below.powf(3.94 - 0.0195 * self.specific_speed)) * ep
                } else {
                    let x = (q - qp) / (qd - qp);
                    ep - x * x * (ep - self.full_load_efficiency)
                }
            }
            TurbineKind::Kaplan => (1.0 - 3.5 * below.powi(6)) * ep,
            TurbineKind::Propeller => (1.0 - 1.25 * below.max(0.0).powf(1.13)) * ep,
            TurbineKind::Pelton { jets } => {
                let j = f64::from(jets.clamp(1, MAX_PELTON_JETS));
                (1.0 - (1.31 + 0.025 * j) * below.abs().powf(5.6 + 0.4 * j)) * ep
            }
            TurbineKind::Turgo => (1.0 - 1.335 * below.abs().powf(6.0)) * ep,
            TurbineKind::Crossflow => {
                let x = (qd - q) / qp;
                0.79 - 0.15 * x - 1.37 * x.powi(14)
            }
        };
        e.clamp(0.0, 1.0)
    }
}

/// Best efficiency of a small generator rated `rated_kw` (ESHA 2004 Table
/// 6.8), interpolated in the logarithm of the rating and held at the
/// table's ends.
pub fn generator_efficiency(rated_kw: f64) -> f64 {
    let first = GENERATOR_EFFICIENCY[0];
    let last = GENERATOR_EFFICIENCY[GENERATOR_EFFICIENCY.len() - 1];
    if rated_kw <= first.0 {
        return first.1;
    }
    if rated_kw >= last.0 {
        return last.1;
    }
    for pair in GENERATOR_EFFICIENCY.windows(2) {
        let ((k0, e0), (k1, e1)) = (pair[0], pair[1]);
        if rated_kw <= k1 {
            let t = (rated_kw.ln() - k0.ln()) / (k1.ln() - k0.ln());
            return e0 + (e1 - e0) * t;
        }
    }
    last.1
}

/// Darcy friction factor of turbulent pipe flow (Swamee & Jain 1976) at
/// Reynolds number `reynolds` and relative roughness `roughness / D`;
/// laminar `64 / Re` below the transition.
pub fn friction_factor(reynolds: f64, relative_roughness: f64) -> f64 {
    if reynolds <= 0.0 {
        return 0.0;
    }
    if reynolds < 2_300.0 {
        return 64.0 / reynolds;
    }
    let log = (relative_roughness / 3.7 + 5.74 / reynolds.powf(0.9)).log10();
    0.25 / (log * log)
}

/// A penstock: the pressure pipe from intake to turbine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Penstock {
    pub length_m: f64,
    pub diameter_m: f64,
    pub roughness_m: f64,
}

impl Penstock {
    /// Friction head loss (m) at `flow_m3_s`, by Darcy-Weisbach.
    pub fn head_loss_m(&self, flow_m3_s: f64) -> f64 {
        if flow_m3_s <= 0.0 || self.diameter_m <= 0.0 {
            return 0.0;
        }
        let area = std::f64::consts::PI * self.diameter_m * self.diameter_m / 4.0;
        let v = flow_m3_s / area;
        let re = v * self.diameter_m / WATER_KINEMATIC_VISCOSITY_M2_S;
        let f = friction_factor(re, self.roughness_m / self.diameter_m);
        f * self.length_m / self.diameter_m * v * v / (2.0 * GRAVITY_M_S2)
    }

    /// The narrowest penstock of `length_m` and wall `roughness_m` that
    /// loses no more than `loss_fraction` of `gross_head_m` at
    /// `design_flow_m3_s` (found by bisection on the diameter: the loss
    /// falls monotonically as the pipe widens).
    pub fn sized(
        length_m: f64,
        roughness_m: f64,
        gross_head_m: f64,
        design_flow_m3_s: f64,
        loss_fraction: f64,
    ) -> Self {
        let allowed = loss_fraction * gross_head_m;
        let pipe = |d: f64| Penstock {
            length_m,
            diameter_m: d,
            roughness_m,
        };
        let (mut lo, mut hi) = (1e-3, 1.0);
        while pipe(hi).head_loss_m(design_flow_m3_s) > allowed && hi < 100.0 {
            hi *= 2.0;
        }
        for _ in 0..100 {
            let mid = 0.5 * (lo + hi);
            if pipe(mid).head_loss_m(design_flow_m3_s) > allowed {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        pipe(hi)
    }
}

/// A designed run-of-river plant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HydroDesign {
    pub gross_head_m: f64,
    pub penstock: Penstock,
    pub turbine: TurbineCurve,
    pub generator_efficiency: f64,
    /// Electrical output at design flow (kW).
    pub rated_kw: f64,
}

impl HydroDesign {
    /// Electrical output (kW) with `flow_m3_s` through the turbine (capped
    /// at the design flow).
    pub fn output_kw(&self, flow_m3_s: f64) -> f64 {
        let q = flow_m3_s.clamp(0.0, self.turbine.design_flow_m3_s);
        let net = (self.gross_head_m - self.penstock.head_loss_m(q)).max(0.0);
        WATER_DENSITY_KG_M3
            * GRAVITY_M_S2
            * q
            * net
            * self.turbine.efficiency(q)
            * self.generator_efficiency
            / 1_000.0
    }

    /// Flow the plant takes (m^3/s) from a river carrying `river_m3_s`,
    /// leaving `environmental_m3_s` in it: none if what is left would not
    /// turn the turbine.
    pub fn diverted_m3_s(&self, river_m3_s: f64, environmental_m3_s: f64) -> f64 {
        let q = (river_m3_s - environmental_m3_s).clamp(0.0, self.turbine.design_flow_m3_s);
        if self.turbine.efficiency(q) > 0.0 {
            q
        } else {
            0.0
        }
    }

    /// Mean output (kW) over a set of equally likely river flows, leaving
    /// `environmental_m3_s` in the river.
    pub fn mean_output_kw(&self, river_flows_m3_s: &[f64], environmental_m3_s: f64) -> f64 {
        if river_flows_m3_s.is_empty() {
            return 0.0;
        }
        river_flows_m3_s
            .iter()
            .map(|&q| self.output_kw(self.diverted_m3_s(q, environmental_m3_s)))
            .sum::<f64>()
            / river_flows_m3_s.len() as f64
    }
}

/// Design a plant to deliver `target_kw` at design flow from `gross_head_m`
/// through a plastic penstock of `penstock_length_m`, choosing among the
/// turbines whose head range takes the rated head the one that makes the
/// most energy from the river's `river_flows_m3_s` (equally likely
/// samples, such as a year of steps) with `environmental_m3_s` left in it.
/// `None` where no turbine takes the head.
pub fn design_hydro(
    target_kw: f64,
    gross_head_m: f64,
    penstock_length_m: f64,
    river_flows_m3_s: &[f64],
    environmental_m3_s: f64,
) -> Option<HydroDesign> {
    let rated_head = gross_head_m * (1.0 - DESIGN_HYDRAULIC_LOSS_FRACTION);
    let mut best: Option<(f64, HydroDesign)> = None;
    for kind in TurbineKind::for_head(rated_head) {
        // The design flow that delivers the target, iterated because the
        // turbine's and generator's efficiencies depend on it.
        let mut qd = target_kw * 1_000.0 / (WATER_DENSITY_KG_M3 * GRAVITY_M_S2 * rated_head);
        let mut design = None;
        for _ in 0..50 {
            let turbine = TurbineCurve::new(kind, qd, rated_head);
            let eta = turbine.efficiency(qd) * generator_efficiency(target_kw);
            if eta <= 0.0 {
                break;
            }
            let next =
                target_kw * 1_000.0 / (WATER_DENSITY_KG_M3 * GRAVITY_M_S2 * rated_head * eta);
            let converged = (next - qd).abs() <= 1e-12 * qd;
            qd = next;
            let penstock = Penstock::sized(
                penstock_length_m,
                PLASTIC_PIPE_ROUGHNESS_M,
                gross_head_m,
                qd,
                DESIGN_HYDRAULIC_LOSS_FRACTION,
            );
            let mut d = HydroDesign {
                gross_head_m,
                penstock,
                turbine: TurbineCurve::new(kind, qd, rated_head),
                generator_efficiency: generator_efficiency(target_kw),
                rated_kw: 0.0,
            };
            d.rated_kw = d.output_kw(qd);
            design = Some(d);
            if converged {
                break;
            }
        }
        let Some(design) = design else { continue };
        let energy = design.mean_output_kw(river_flows_m3_s, environmental_m3_s);
        if best.as_ref().is_none_or(|(e, _)| energy > *e) {
            best = Some((energy, design));
        }
    }
    best.map(|(_, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_select_the_turbines_esha_lists() {
        assert_eq!(
            TurbineKind::for_head(10.0),
            vec![
                TurbineKind::Kaplan,
                TurbineKind::Propeller,
                TurbineKind::Crossflow
            ]
        );
        let h100 = TurbineKind::for_head(100.0);
        assert!(h100.contains(&TurbineKind::Francis));
        assert!(h100.contains(&TurbineKind::Pelton { jets: 1 }));
        assert!(h100.contains(&TurbineKind::Turgo));
        assert!(!h100.contains(&TurbineKind::Kaplan));
        assert_eq!(TurbineKind::for_head(1.0), vec![]);
    }

    #[test]
    fn curves_peak_where_retscreen_puts_them_and_near_esha_best() {
        let kaplan = TurbineCurve::new(TurbineKind::Kaplan, 2.0, 15.0);
        assert!((kaplan.peak_flow_m3_s - 1.5).abs() < 1e-12);
        let e_peak = kaplan.efficiency(1.5);
        assert!(e_peak >= kaplan.efficiency(1.0) && e_peak >= kaplan.efficiency(2.0));
        // ESHA's best Kaplan is 0.93; RETScreen's fitted peak for a small
        // runner sits a little under it.
        assert!((0.85..=0.93).contains(&e_peak), "{e_peak}");
        let pelton = TurbineCurve::new(TurbineKind::Pelton { jets: 1 }, 0.1, 100.0);
        assert!(
            (0.80..=0.90).contains(&pelton.peak_efficiency),
            "{}",
            pelton.peak_efficiency
        );
        // A multi-jet Pelton holds its efficiency to lower flows.
        let four = TurbineCurve::new(TurbineKind::Pelton { jets: 4 }, 0.1, 100.0);
        assert!(four.efficiency(0.02) > pelton.efficiency(0.02));
        // Cross-flow: 0.79 at design flow (RETScreen).
        let cross = TurbineCurve::new(TurbineKind::Crossflow, 0.5, 20.0);
        assert!((cross.efficiency(0.5) - 0.79).abs() < 1e-12);
        // No flow, no efficiency; a propeller falls off fast below design.
        assert_eq!(kaplan.efficiency(0.0), 0.0);
        let prop = TurbineCurve::new(TurbineKind::Propeller, 2.0, 15.0);
        assert!(prop.efficiency(1.0) < kaplan.efficiency(1.0));
    }

    #[test]
    fn the_pelton_runs_at_the_speed_of_eshas_example() {
        let n = pelton_speed_rpm(400.0, 0.42, 1);
        assert!((900.0..=1_050.0).contains(&n), "{n}");
        // A bigger runner is the more efficient one.
        let small = TurbineCurve::new(TurbineKind::Pelton { jets: 1 }, 0.05, 100.0);
        let large = TurbineCurve::new(TurbineKind::Pelton { jets: 1 }, 5.0, 100.0);
        assert!(large.peak_efficiency > small.peak_efficiency);
    }

    #[test]
    fn generator_efficiency_follows_esha() {
        assert_eq!(generator_efficiency(5.0), 0.91);
        assert_eq!(generator_efficiency(50.0), 0.94);
        let mid = generator_efficiency(25.0);
        assert!(mid > 0.91 && mid < 0.94);
        assert_eq!(generator_efficiency(5_000.0), 0.97);
    }

    #[test]
    fn penstock_loss_is_darcy_weisbach_and_sizing_meets_the_budget() {
        // Fully rough steel at high Re approaches von Karman's limit.
        let f = friction_factor(1e8, 1e-3);
        let karman = 1.0 / (-2.0 * (1e-3f64 / 3.7).log10()).powi(2);
        assert!((f - karman).abs() / karman < 0.02, "{f} vs {karman}");
        let p = Penstock::sized(500.0, PLASTIC_PIPE_ROUGHNESS_M, 80.0, 0.05, 0.05);
        assert!((p.head_loss_m(0.05) - 4.0).abs() < 1e-6);
        // Half the flow, about a quarter of the loss.
        let ratio = p.head_loss_m(0.025) / p.head_loss_m(0.05);
        assert!((0.25..0.32).contains(&ratio), "{ratio}");
    }

    #[test]
    fn a_design_delivers_its_target_and_leaves_the_river_its_share() {
        let flows: Vec<f64> = (0..20).map(|k| 0.05 + 0.02 * f64::from(k)).collect();
        let d = design_hydro(25.0, 80.0, 600.0, &flows, 0.03).unwrap();
        assert!((d.rated_kw - 25.0).abs() < 0.05, "{}", d.rated_kw);
        // P = rho g Q H_net e_t e_g at the design flow.
        let q = d.turbine.design_flow_m3_s;
        let p = WATER_DENSITY_KG_M3
            * GRAVITY_M_S2
            * q
            * (80.0 - d.penstock.head_loss_m(q))
            * d.turbine.efficiency(q)
            * d.generator_efficiency
            / 1_000.0;
        assert!((d.output_kw(q) - p).abs() < 1e-9);
        // Nothing diverted below the environmental flow; a flood is capped.
        assert_eq!(d.diverted_m3_s(0.02, 0.03), 0.0);
        assert_eq!(d.diverted_m3_s(10.0, 0.03), q);
        assert!(d.output_kw(10.0) <= d.rated_kw + 1e-9);
    }
}
