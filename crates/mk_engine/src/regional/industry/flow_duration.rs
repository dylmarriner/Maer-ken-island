//! Flow-duration curves of the island's rivers (Phase 4b Task 2).
//!
//! A hydro site is judged on how its river's flow is distributed over a
//! year, not on one day's flow: RETScreen (2004, §2.2) describes it by the
//! flow-duration curve, the flows equalled or exceeded 0, 5, ..., 100% of
//! the time, and takes the firm flow as the one available 95% of the time.
//! The curves here come from a simulated year of the island's own
//! hydrology (the spin-up's last orbit), one per perennial medium cell: a
//! cell whose river never ran dry that year.

use serde::{Deserialize, Serialize};

/// Points on a flow-duration curve: exceedance 0%, 5%, ..., 100%.
pub const DURATION_POINTS: usize = 21;

/// One cell's flow-duration curve (m^3/s), exceedance 0% (the highest
/// flow of the year) to 100% (the lowest), and its mean.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlowDuration {
    pub flows_m3_s: [f32; DURATION_POINTS],
    pub mean_m3_s: f32,
}

impl FlowDuration {
    /// From a year of equally spaced flow samples; `None` if the river ran
    /// dry at any of them (it is not perennial) or there are none.
    pub fn from_samples(samples_m3_s: &[f64]) -> Option<Self> {
        if samples_m3_s.is_empty() || samples_m3_s.iter().any(|&q| q <= 0.0 || !q.is_finite()) {
            return None;
        }
        let mut sorted = samples_m3_s.to_vec();
        // Descending: exceedance rises along the vector.
        sorted.sort_by(|a, b| b.total_cmp(a));
        let n = sorted.len();
        let mut flows = [0.0f32; DURATION_POINTS];
        for (k, f) in flows.iter_mut().enumerate() {
            // Weibull plotting position: sample i (0-based) is exceeded or
            // equalled (i + 1) / (n + 1) of the time; interpolate linearly.
            let p = k as f64 / (DURATION_POINTS - 1) as f64;
            let x = (p * (n + 1) as f64 - 1.0).clamp(0.0, (n - 1) as f64);
            let i = x.floor() as usize;
            let j = (i + 1).min(n - 1);
            let t = x - i as f64;
            *f = (sorted[i] * (1.0 - t) + sorted[j] * t) as f32;
        }
        let mean = samples_m3_s.iter().sum::<f64>() / n as f64;
        Some(Self {
            flows_m3_s: flows,
            mean_m3_s: mean as f32,
        })
    }

    /// The flow (m^3/s) equalled or exceeded `exceedance` (0-1) of the
    /// time, interpolated between the curve's points.
    pub fn flow_at(&self, exceedance: f64) -> f64 {
        let x = exceedance.clamp(0.0, 1.0) * (DURATION_POINTS - 1) as f64;
        let i = x.floor() as usize;
        let j = (i + 1).min(DURATION_POINTS - 1);
        let t = x - i as f64;
        f64::from(self.flows_m3_s[i]) * (1.0 - t) + f64::from(self.flows_m3_s[j]) * t
    }

    /// The curve's 21 flows as equally likely samples of the year (m^3/s),
    /// for averaging a plant's output over it.
    pub fn samples_m3_s(&self) -> Vec<f64> {
        self.flows_m3_s.iter().map(|&q| f64::from(q)).collect()
    }
}

/// Flow-duration curves of every perennial medium cell, by flat index.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RiverFlowRecord {
    pub cells: std::collections::BTreeMap<u32, FlowDuration>,
}

impl RiverFlowRecord {
    /// From a year of per-step discharge (m^3/s) of every medium cell
    /// (`steps[s][cell]`).
    pub fn from_year(steps: &[Vec<f32>]) -> Self {
        let n = steps.first().map_or(0, Vec::len);
        let cells: Vec<usize> = (0..n).collect();
        Self::from_year_at(steps, &cells)
    }

    /// As [`from_year`](Self::from_year) where `steps[s][k]` is the
    /// discharge of medium cell `cells[k]`.
    pub fn from_year_at(steps: &[Vec<f32>], cells: &[usize]) -> Self {
        let mut record = std::collections::BTreeMap::new();
        let mut series = Vec::with_capacity(steps.len());
        for (k, &cell) in cells.iter().enumerate() {
            series.clear();
            series.extend(steps.iter().map(|s| f64::from(s[k])));
            if let Some(fd) = FlowDuration::from_samples(&series) {
                record.insert(cell as u32, fd);
            }
        }
        Self { cells: record }
    }

    pub fn get(&self, cell: usize) -> Option<&FlowDuration> {
        self.cells.get(&(cell as u32))
    }

    /// Whether a river runs through medium cell `cell` all year: it is
    /// perennial in the record and its mean flow reaches the hydrology's
    /// river threshold. A property of the island's year, not of whichever
    /// step it is asked on, so what is built on it -- habitats, deposits --
    /// does not change with the weather.
    pub fn is_river(&self, cell: usize) -> bool {
        self.get(cell).is_some_and(|flow| {
            f64::from(flow.mean_m3_s) >= crate::regional::hydrology::RIVER_MIN_DISCHARGE_M3_S
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_falls_from_the_flood_to_the_low_flow() {
        let year: Vec<f64> = (1..=54).map(f64::from).collect();
        let fd = FlowDuration::from_samples(&year).unwrap();
        assert_eq!(fd.flows_m3_s[0], 54.0);
        assert_eq!(fd.flows_m3_s[20], 1.0);
        assert!(fd.flows_m3_s.windows(2).all(|w| w[0] >= w[1]));
        assert!((fd.flow_at(0.5) - 27.5).abs() < 0.6);
        assert!((f64::from(fd.mean_m3_s) - 27.5).abs() < 1e-6);
        // Firm flow (95%) is near the year's low end.
        assert!(fd.flow_at(0.95) < 4.0);
        // A river that ran dry once is not perennial.
        let mut dry = year.clone();
        dry[10] = 0.0;
        assert!(FlowDuration::from_samples(&dry).is_none());
    }

    #[test]
    fn the_record_keeps_only_perennial_cells() {
        let steps = vec![vec![1.0f32, 0.0, 2.0], vec![2.0, 0.5, 3.0]];
        let r = RiverFlowRecord::from_year(&steps);
        assert!(r.get(0).is_some() && r.get(1).is_none() && r.get(2).is_some());
    }
}
