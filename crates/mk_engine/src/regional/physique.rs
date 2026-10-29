//! Every living person's inherited genome and trained body (realism R12,
//! R13): the island's record of who each person is born as and what their
//! own activity has made of them.
//!
//! The genome is drawn once, when a person first joins the island: a
//! founder's from the population, a child's from its parents' (when both
//! parents' genomes are known; otherwise the child is treated as a founder).
//! It scales the person's untrained baselines (aerobic capacity, strength,
//! lean mass) by their inherited and environmental deviation.
//!
//! Fitness then follows only what the person actually did: each second of
//! work is recorded at its MET and load, and at the end of each local day
//! the day's work trains them, or its absence detrains them. Nothing here
//! chooses what anyone does. It keeps the consequences their own choices
//! have for their body.

use super::fitness::{Fitness, TrainingModels};
use super::genome::{Genome, GenomeMap, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The trait names the genome pack must carry for the baselines.
pub const AEROBIC_TRAIT: &str = "vo2max";
pub const STRENGTH_TRAIT: &str = "strength";

/// Every person's genome and fitness, keyed by agent id.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Physiques {
    pub genomes: BTreeMap<String, Genome>,
    pub fitness: BTreeMap<String, Fitness>,
    /// The local day whose work is being recorded.
    pub day: u64,
}

/// The models the physiques run on, loaded from the reference packs.
#[derive(Clone, Debug)]
pub struct PhysiqueModels {
    pub training: TrainingModels,
    pub genome: GenomeMap,
}

impl PhysiqueModels {
    pub fn load_default() -> Result<Self, String> {
        let library =
            crate::validation::ReferenceLibrary::load(&crate::validation::default_reference_dir())
                .map_err(|e| e.to_string())?;
        Ok(Self {
            training: TrainingModels::from_reference(&library)?,
            genome: GenomeMap::from_reference(&library)?,
        })
    }
}

impl Physiques {
    /// Give `id` a genome and an untrained body if they have none yet.
    /// `parents` are the mother's and father's ids, when the runtime
    /// recorded a birth.
    #[allow(clippy::too_many_arguments)]
    pub fn admit(
        &mut self,
        models: &PhysiqueModels,
        seed: &[u8; 32],
        id: &str,
        male: bool,
        age_years: f64,
        mass_kg: f64,
        parents: Option<(&str, &str)>,
    ) {
        if self.genomes.contains_key(id) {
            return;
        }
        let mut stream = Stream::new(seed, &format!("genome:{id}"));
        let genome =
            match parents.and_then(|(m, f)| Some((self.genomes.get(m)?, self.genomes.get(f)?))) {
                Some((mother, father)) => models.genome.conceive(mother, father, male, &mut stream),
                None => models.genome.founder(male, &mut stream),
            };
        let scale = |name: &str| models.genome.trait_multiplier(&genome, name);
        // Lean mass has no cited spread of its own and follows strength (D66).
        let strength = scale(STRENGTH_TRAIT);
        let inherited = [scale(AEROBIC_TRAIT), strength, strength];
        let body = Fitness::untrained(&models.training, !male, age_years, mass_kg, inherited);
        self.genomes.insert(id.to_string(), genome);
        self.fitness.insert(id.to_string(), body);
    }

    /// Record `seconds` of work at `met`, carrying or lifting `load_kg`.
    pub fn record(
        &mut self,
        models: &PhysiqueModels,
        id: &str,
        met: f64,
        seconds: f64,
        load_kg: f64,
    ) {
        if let Some(f) = self.fitness.get_mut(id) {
            f.record(&models.training, met, seconds, load_kg);
        }
    }

    /// Close every local day that has ended by `day`: each living person's
    /// recorded work trains them. `bodies` gives each person's sex, age
    /// (years) and present body mass (kg).
    pub fn close_days(
        &mut self,
        models: &PhysiqueModels,
        day: u64,
        bodies: &[(String, bool, f64, f64)],
    ) {
        while self.day < day {
            for (id, male, age_years, mass_kg) in bodies {
                if let Some(f) = self.fitness.get_mut(id) {
                    f.end_day(&models.training, !male, *age_years, *mass_kg);
                }
            }
            self.day += 1;
        }
    }

    /// `id`'s present strength (kg of grip), if the island tracks them.
    pub fn strength_kg(&self, id: &str) -> Option<f64> {
        self.fitness.get(id).map(|f| f.strength_kg)
    }

    /// Forget someone who died or left.
    pub fn forget(&mut self, id: &str) {
        self.fitness.remove(id);
        // The genome stays: a child born after its father died is still
        // conceived from him. It holds no material and moves no ledger.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn models() -> PhysiqueModels {
        PhysiqueModels::load_default().expect("the training and genome packs load")
    }

    #[test]
    fn a_child_of_two_admitted_parents_is_conceived_from_them() {
        let m = models();
        let mut p = Physiques::default();
        p.admit(&m, &[7; 32], "mum", false, 30.0, 62.0, None);
        p.admit(&m, &[7; 32], "dad", true, 32.0, 80.0, None);
        p.admit(&m, &[7; 32], "kid", true, 0.0, 3.4, Some(("mum", "dad")));
        let kid = &p.genomes["kid"];
        assert!(kid.male());
        assert!(kid.de_novo > 0, "a child carries new mutations");
        let orphan = {
            let mut q = Physiques::default();
            q.admit(&m, &[7; 32], "kid", true, 0.0, 3.4, Some(("mum", "dad")));
            q.genomes["kid"].de_novo
        };
        assert_eq!(orphan, 0, "parents unknown to the island: a founder");
    }

    #[test]
    fn admission_replays_and_is_once_only() {
        let m = models();
        let mut a = Physiques::default();
        let mut b = Physiques::default();
        a.admit(&m, &[9; 32], "x", false, 25.0, 60.0, None);
        b.admit(&m, &[9; 32], "x", false, 25.0, 60.0, None);
        assert_eq!(a, b);
        let before = a.clone();
        a.admit(&m, &[9; 32], "x", false, 25.0, 70.0, None);
        assert_eq!(a, before);
    }

    #[test]
    fn a_labourer_grows_stronger_than_someone_who_sits() {
        let m = models();
        let mut p = Physiques::default();
        p.admit(&m, &[3; 32], "worker", true, 30.0, 80.0, None);
        p.admit(&m, &[3; 32], "sitter", true, 30.0, 80.0, None);
        let start = p.fitness["worker"].strength_kg;
        let sitter_start = p.fitness["sitter"].strength_kg;
        let heavy = start * m.training.strength_load_fraction * 1.1;
        let t = &m.training;
        let hard_met = 1.1 * t.aerobic_intensity_fraction * p.fitness["worker"].vo2max_ml_kg_min
            / t.met_oxygen_ml_kg_min;
        let bodies = vec![
            ("worker".to_string(), true, 30.0, 80.0),
            ("sitter".to_string(), true, 30.0, 80.0),
        ];
        for day in 1..=60 {
            // Four hours of hard work and a heavy lift, three days in seven.
            if day % 7 < 3 {
                p.record(&m, "worker", hard_met, 4.0 * 3600.0, heavy);
            }
            p.close_days(&m, day, &bodies);
        }
        let worker = &p.fitness["worker"];
        let sitter = &p.fitness["sitter"];
        assert!(worker.strength_kg > start, "training builds strength");
        assert!(
            worker.vo2max_ml_kg_min > worker.baseline_vo2max,
            "and stamina"
        );
        assert!(worker.lean_kg > worker.baseline_lean_kg, "and muscle");
        assert!(
            worker.fat_kg(80.0) < 80.0 - worker.baseline_lean_kg,
            "and less fat at the same mass"
        );
        assert!(
            (sitter.strength_kg - sitter_start).abs() < 1e-9,
            "sitting changes nothing"
        );
        assert!((sitter.lean_kg - sitter.baseline_lean_kg).abs() < 1e-9);
    }
}
