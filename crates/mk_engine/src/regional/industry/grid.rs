//! The island's electricity grid (Phase 4b Task 3).
//!
//! Lines are three-phase overhead conductors: carrying `P` at line voltage
//! `V` (unity power factor) draws `I = P / (sqrt(3) V)` per phase and loses
//! `3 I^2 R = P^2 R / V^2` in a line of resistance `R` per conductor, and
//! drops `sqrt(3) I R` of voltage. A line is designed at the lowest standard
//! voltage (IEC 60038: 400 V low voltage, 11 kV distribution) and the
//! lightest standard conductor (BS 215 Part 2 aluminium conductor
//! steel-reinforced) whose voltage drop at its rated power stays within the
//! 5% AS/NZS 3000 allows.
//!
//! A [`Grid`] is radial, as rural distribution is: every node but the root
//! hangs from one parent by one line. Each step generators are dispatched
//! in merit order (the order they are given: run-of-river hydro, whose
//! water is spilled if unused, before anything that burns fuel) until the
//! loads and the line losses are met; when they cannot be, the lowest
//! priority loads are shed first.

use serde::{Deserialize, Serialize};

/// Largest voltage drop a line may have at its rated power, as a share of
/// its nominal voltage (AS/NZS 3000:2018, clause 3.6.2).
pub const MAX_VOLTAGE_DROP_FRACTION: f64 = 0.05;
/// Standard three-phase line voltages (V), lowest first (IEC 60038): low
/// voltage and the 11 kV of rural distribution.
pub const STANDARD_LINE_VOLTAGES_V: [f64; 2] = [400.0, 11_000.0];

/// A BS 215 Part 2 ACSR conductor: code name and maximum DC resistance at
/// 20 C (ohm/km), lightest first (De Angeli Prodotti conductor data sheet,
/// British sizes).
pub const ACSR_CONDUCTORS: [(&str, f64); 13] = [
    ("Mole", 2.7060),
    ("Squirrel", 1.3700),
    ("Gopher", 1.0930),
    ("Weasel", 0.9077),
    ("Fox", 0.7822),
    ("Ferret", 0.6766),
    ("Rabbit", 0.5426),
    ("Mink", 0.4545),
    ("Skunk", 0.4568),
    ("Beaver", 0.3825),
    ("Horse", 0.3936),
    ("Racoon", 0.3623),
    ("Otter", 0.3419),
];

/// A three-phase overhead line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub from: usize,
    pub to: usize,
    pub voltage_v: f64,
    pub length_km: f64,
    pub conductor: String,
    /// Resistance of one conductor over the line's length (ohm).
    pub resistance_ohm: f64,
}

impl Line {
    /// Power lost (kW) carrying `kw` along the line.
    pub fn loss_kw(&self, kw: f64) -> f64 {
        let w = kw.max(0.0) * 1_000.0;
        w * w * self.resistance_ohm / (self.voltage_v * self.voltage_v) / 1_000.0
    }

    /// Voltage drop (V) carrying `kw`.
    pub fn voltage_drop_v(&self, kw: f64) -> f64 {
        let current = kw.max(0.0) * 1_000.0 / (3f64.sqrt() * self.voltage_v);
        3f64.sqrt() * current * self.resistance_ohm
    }

    /// The power (kW) that arrives when `sent_kw` is sent.
    pub fn delivered_kw(&self, sent_kw: f64) -> f64 {
        (sent_kw - self.loss_kw(sent_kw)).max(0.0)
    }

    /// The power (kW) that must be sent for `arriving_kw` to arrive, or
    /// `None` if no amount can (the line's losses outgrow what it carries).
    pub fn sent_for_kw(&self, arriving_kw: f64) -> Option<f64> {
        if arriving_kw <= 0.0 {
            return Some(0.0);
        }
        // a s^2 - s + p = 0 with a = R / V^2 (per kW).
        let a = self.resistance_ohm * 1_000.0 / (self.voltage_v * self.voltage_v);
        if a <= 0.0 {
            return Some(arriving_kw);
        }
        let disc = 1.0 - 4.0 * a * arriving_kw;
        (disc >= 0.0).then(|| (1.0 - disc.sqrt()) / (2.0 * a))
    }

    /// The lowest standard voltage and lightest standard conductor that
    /// carry `rated_kw` over `length_km` within the allowed voltage drop.
    pub fn design(from: usize, to: usize, rated_kw: f64, length_km: f64) -> Self {
        for &voltage_v in &STANDARD_LINE_VOLTAGES_V {
            for &(name, ohm_per_km) in &ACSR_CONDUCTORS {
                let line = Line {
                    from,
                    to,
                    voltage_v,
                    length_km,
                    conductor: name.to_string(),
                    resistance_ohm: ohm_per_km * length_km,
                };
                if line.voltage_drop_v(rated_kw) <= MAX_VOLTAGE_DROP_FRACTION * voltage_v {
                    return line;
                }
            }
        }
        let (name, ohm_per_km) = ACSR_CONDUCTORS[ACSR_CONDUCTORS.len() - 1];
        let voltage_v = STANDARD_LINE_VOLTAGES_V[STANDARD_LINE_VOLTAGES_V.len() - 1];
        Line {
            from,
            to,
            voltage_v,
            length_km,
            conductor: name.to_string(),
            resistance_ohm: ohm_per_km * length_km,
        }
    }
}

/// How essential a load is: under a shortfall the lowest are shed first
/// (Phase 4b plan: refinery, workshops, households, street lighting, in
/// that order of priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LoadPriority {
    StreetLighting,
    Household,
    Workshop,
    Refinery,
}

/// A load at a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridLoad {
    pub node: usize,
    pub kw: f64,
    pub priority: LoadPriority,
}

/// What a generator at a node can give this step (kW).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridSupply {
    pub node: usize,
    pub available_kw: f64,
}

/// A radial network: node 0 is the root; every other node has exactly one
/// line to its parent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub nodes: Vec<String>,
    pub lines: Vec<Line>,
}

/// One step's dispatch.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dispatch {
    /// Output of each supply (kW), in the order given.
    pub generated_kw: Vec<f64>,
    /// Whether each load is served, in the order given.
    pub served: Vec<bool>,
    pub line_loss_kw: f64,
}

impl Grid {
    /// The line joining `node` to its parent, if it has one.
    fn parent_line(&self, node: usize) -> Option<&Line> {
        self.lines.iter().find(|l| l.to == node)
    }

    /// Path from `node` up to the root, as line indices.
    fn path_to_root(&self, node: usize) -> Vec<usize> {
        let mut path = Vec::new();
        let mut here = node;
        while let Some((i, line)) = self.lines.iter().enumerate().find(|(_, l)| l.to == here) {
            path.push(i);
            here = line.from;
            if path.len() > self.lines.len() {
                break;
            }
        }
        path
    }

    /// Net injection (kW, + out of the node) for given served loads and
    /// supply outputs, then the flow on every line and the total loss,
    /// solved by iterating the losses into the flows. Flow on a line is
    /// toward the root when positive.
    fn losses_kw(&self, injection_kw: &[f64]) -> (f64, Vec<f64>) {
        let mut loss_at = vec![0.0; self.nodes.len()];
        let mut flows = vec![0.0; self.lines.len()];
        for _ in 0..50 {
            flows.iter_mut().for_each(|f| *f = 0.0);
            for (node, &inj) in injection_kw.iter().enumerate() {
                let net = inj - loss_at[node];
                for li in self.path_to_root(node) {
                    flows[li] += net;
                }
            }
            let mut next = vec![0.0; self.nodes.len()];
            for (li, line) in self.lines.iter().enumerate() {
                // A line's loss is drawn at its child end.
                next[line.to] += line.loss_kw(flows[li].abs());
            }
            let converged = next
                .iter()
                .zip(&loss_at)
                .all(|(a, b)| (a - b).abs() <= 1e-9);
            loss_at = next;
            if converged {
                break;
            }
        }
        (loss_at.iter().sum(), flows)
    }

    /// Dispatch `supplies` in merit order to serve `loads`, shedding the
    /// lowest-priority loads (then the latest given) when the supply cannot
    /// cover them and the line losses. Generation balances served load plus
    /// losses.
    pub fn dispatch(&self, supplies: &[GridSupply], loads: &[GridLoad]) -> Dispatch {
        let mut order: Vec<usize> = (0..loads.len()).collect();
        // Highest priority first; among equals, the order given.
        order.sort_by(|&a, &b| loads[b].priority.cmp(&loads[a].priority).then(a.cmp(&b)));
        let mut served = vec![true; loads.len()];
        let mut shed_from = order.len();
        loop {
            let demand: Vec<(usize, f64)> = order[..shed_from]
                .iter()
                .map(|&i| (loads[i].node, loads[i].kw.max(0.0)))
                .collect();
            if let Some((generated, loss)) = self.balance(supplies, &demand) {
                for &i in &order[shed_from..] {
                    served[i] = false;
                }
                return Dispatch {
                    generated_kw: generated,
                    served,
                    line_loss_kw: loss,
                };
            }
            if shed_from == 0 {
                served.iter_mut().for_each(|s| *s = false);
                return Dispatch {
                    generated_kw: vec![0.0; supplies.len()],
                    served,
                    line_loss_kw: 0.0,
                };
            }
            shed_from -= 1;
        }
    }

    /// Generation that meets `demand` (node, kW) and the losses it causes,
    /// supplies taken in merit order; `None` if they cannot.
    fn balance(&self, supplies: &[GridSupply], demand: &[(usize, f64)]) -> Option<(Vec<f64>, f64)> {
        let total: f64 = demand.iter().map(|(_, kw)| kw).sum();
        let mut need = total;
        let mut generated = vec![0.0; supplies.len()];
        for _ in 0..50 {
            let mut left = need;
            for (g, s) in generated.iter_mut().zip(supplies) {
                *g = left.min(s.available_kw.max(0.0));
                left -= *g;
            }
            if left > 1e-9 {
                return None;
            }
            let mut injection = vec![0.0; self.nodes.len()];
            for (g, s) in generated.iter().zip(supplies) {
                injection[s.node] += g;
            }
            for &(node, kw) in demand {
                injection[node] -= kw;
            }
            let (loss, _) = self.losses_kw(&injection);
            let next = total + loss;
            if (next - need).abs() <= 1e-9 {
                return Some((generated, loss));
            }
            need = next;
        }
        None
    }

    /// Number of nodes below the root with no parent line (a malformed
    /// network has some).
    pub fn orphans(&self) -> usize {
        (1..self.nodes.len())
            .filter(|&n| self.parent_line(n).is_none())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_loses_i_squared_r_and_designs_to_the_voltage_drop() {
        let line = Line::design(0, 1, 25.0, 2.0);
        // 25 kW over 2 km cannot stay within 5% at 400 V on any of the
        // standard conductors (Otter: about 11%), so it goes to 11 kV, where
        // the lightest conductor will do.
        assert_eq!(line.voltage_v, 11_000.0);
        assert_eq!(line.conductor, "Mole");
        let i = 25_000.0 / (3f64.sqrt() * 11_000.0);
        let loss = 3.0 * i * i * line.resistance_ohm / 1_000.0;
        assert!((line.loss_kw(25.0) - loss).abs() < 1e-12);
        assert!(line.voltage_drop_v(25.0) <= 0.05 * 11_000.0);
        // A short low-voltage run stays at 400 V.
        let short = Line::design(0, 1, 5.0, 0.05);
        assert_eq!(short.voltage_v, 400.0);
        // Sent and delivered invert each other.
        let sent = line.sent_for_kw(20.0).unwrap();
        assert!((line.delivered_kw(sent) - 20.0).abs() < 1e-9);
    }

    fn feeder() -> Grid {
        // Root (powerhouse) - house - workshop, lossy enough to matter.
        let line = |from, to| Line {
            from,
            to,
            voltage_v: 400.0,
            length_km: 1.0,
            conductor: "Mole".into(),
            resistance_ohm: 0.2,
        };
        Grid {
            nodes: vec!["powerhouse".into(), "house".into(), "workshop".into()],
            lines: vec![line(0, 1), line(1, 2)],
        }
    }

    #[test]
    fn generation_meets_load_plus_losses_in_merit_order() {
        let grid = feeder();
        let supplies = [
            GridSupply {
                node: 0,
                available_kw: 20.0,
            },
            GridSupply {
                node: 1,
                available_kw: 50.0,
            },
        ];
        let loads = [
            GridLoad {
                node: 1,
                kw: 10.0,
                priority: LoadPriority::Household,
            },
            GridLoad {
                node: 2,
                kw: 15.0,
                priority: LoadPriority::Workshop,
            },
        ];
        let d = grid.dispatch(&supplies, &loads);
        assert_eq!(d.served, vec![true, true]);
        // Hydro first, at its limit; the rest from the second supply.
        assert!((d.generated_kw[0] - 20.0).abs() < 1e-9);
        let total: f64 = d.generated_kw.iter().sum();
        assert!(d.line_loss_kw > 0.0);
        assert!((total - (25.0 + d.line_loss_kw)).abs() < 1e-6);
    }

    #[test]
    fn a_shortfall_sheds_the_lowest_priority_first() {
        let grid = feeder();
        let supplies = [GridSupply {
            node: 0,
            available_kw: 18.0,
        }];
        let loads = [
            GridLoad {
                node: 1,
                kw: 5.0,
                priority: LoadPriority::StreetLighting,
            },
            GridLoad {
                node: 1,
                kw: 10.0,
                priority: LoadPriority::Household,
            },
            GridLoad {
                node: 2,
                kw: 6.0,
                priority: LoadPriority::Refinery,
            },
        ];
        let d = grid.dispatch(&supplies, &loads);
        assert_eq!(d.served, vec![false, true, true]);
        let total: f64 = d.generated_kw.iter().sum();
        assert!((total - (16.0 + d.line_loss_kw)).abs() < 1e-6);
        assert_eq!(grid.orphans(), 0);
    }
}
