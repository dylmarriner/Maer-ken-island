//! A genome every person inherits, as on Earth (realism R13).
//!
//! Each person carries two copies of every chromosome, one from each
//! parent. A copy is a double helix: one strand of bases (A, C, G, T) and
//! its complement, which pairing fixes (A with T, C with G), so only the
//! forward strand is stored. Along each chromosome a locus stands every
//! `locus_spacing_bp` base pairs of the real GRCh38 length; the genome is
//! sampled at that spacing rather than carried in full (D66).
//!
//! A child is made from one gamete of each parent. A gamete takes each
//! chromosome from the parent's two copies, switching between them at
//! crossovers drawn on the parent's sex-specific genetic map (Poisson, a
//! mean of one crossover per 100 cM), and mutates each locus at the cited
//! rate per base pair per generation times the base pairs the locus stands
//! for. The father gives an X to a daughter and his Y to a son, matching
//! the child's sex as the human runtime decided it.
//!
//! Heritable traits are polygenic: each trait reads a fixed set of loci,
//! counting the copies of its effect base. The genetic score is that count
//! standardised for the founders' base frequencies, and a trait's
//! deviation from the population is `sqrt(h2) * genetic + sqrt(1 - h2) *
//! environment`, so in the founders the trait varies with exactly its cited
//! heritability. Children inherit the score through their parents' loci and
//! draw their own environment.

use crate::validation::{ReferenceDomain, ReferenceLibrary};
use serde::{Deserialize, Serialize};

/// A base on the forward strand: 0 = A, 1 = C, 2 = G, 3 = T.
pub type Base = u8;

/// The base paired with `base` on the other strand.
pub fn complement(base: Base) -> Base {
    3 - (base & 3)
}

/// One chromosome of the reference map.
#[derive(Clone, Debug, PartialEq)]
pub struct Chromosome {
    pub name: String,
    pub bp: u64,
    pub cm_female: f64,
    pub cm_male: f64,
}

/// One heritable trait: its heritability, the spread of its deviation
/// (coefficient of variation), and every `trait_count`-th locus from its
/// offset is one of its loci.
#[derive(Clone, Debug, PartialEq)]
pub struct HeritableTrait {
    pub name: String,
    pub heritability: f64,
    pub cv: f64,
}

/// The cited map (`fixtures/reference/humans/genome.json`).
#[derive(Clone, Debug, PartialEq)]
pub struct GenomeMap {
    /// Autosomes 1-22, then X, then Y. A mother's two X recombine along
    /// the X's share of her map, taken as its length's share (D66).
    pub chromosomes: Vec<Chromosome>,
    pub mutation_rate_per_bp: f64,
    pub locus_spacing_bp: u64,
    pub traits: Vec<HeritableTrait>,
}

/// One copy of every chromosome. `chromosomes[22]` is the X or the Y.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Haplotype {
    pub y: bool,
    pub chromosomes: Vec<Vec<Base>>,
}

/// A person's genome: the copy from their mother and the one from their
/// father, and the environment draw for each trait.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub maternal: Haplotype,
    pub paternal: Haplotype,
    pub environment: Vec<f64>,
    /// De novo mutations this person was conceived with.
    pub de_novo: u32,
}

/// A named, seeded stream of uniforms, so each draw is reproducible.
#[derive(Clone, Debug)]
pub struct Stream {
    key: [u8; 32],
    counter: u64,
}

impl Stream {
    pub fn new(seed: &[u8; 32], label: &str) -> Self {
        let mut h = blake3::Hasher::new();
        h.update(b"mk-island-genome");
        h.update(seed);
        h.update(label.as_bytes());
        Self {
            key: *h.finalize().as_bytes(),
            counter: 0,
        }
    }

    /// A uniform value in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        let mut h = blake3::Hasher::new_keyed(&self.key);
        h.update(&self.counter.to_le_bytes());
        self.counter += 1;
        let word = u64::from_le_bytes(h.finalize().as_bytes()[..8].try_into().expect("8 bytes"));
        (word >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A standard normal value (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.uniform();
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// A Poisson count of mean `mean` (inversion; means here are small).
    pub fn poisson(&mut self, mean: f64) -> u32 {
        let mut k = 0;
        let mut p = (-mean).exp();
        let mut cum = p;
        let u = self.uniform();
        while u > cum && k < 10_000 {
            k += 1;
            p *= mean / f64::from(k);
            cum += p;
        }
        k
    }

    fn base(&mut self) -> Base {
        (self.uniform() * 4.0) as Base & 3
    }
}

const X: usize = 22;
const Y: usize = 23;

impl GenomeMap {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let table = |key: &str| {
            library
                .item(ReferenceDomain::Humans, key)
                .and_then(|i| i.table())
                .ok_or_else(|| format!("humans reference item {key} missing or not a table"))
        };
        let lengths = table("chromosome_lengths")?;
        let need = |v: Option<f64>, what: &str| v.ok_or_else(|| format!("genome pack: {what}"));
        if lengths.rows.len() != 24 {
            return Err(format!(
                "chromosome_lengths has {} rows, not 1-22, X, Y",
                lengths.rows.len()
            ));
        }
        let bps = lengths
            .rows
            .iter()
            .map(|r| Ok((r.label.clone(), need(lengths.get(&r.label, "bp"), "bp")? as u64)))
            .collect::<Result<Vec<_>, String>>()?;
        // The map length of each autosome is its share by length of the
        // sex's crossovers per meiosis (D66: no per-chromosome map).
        let crossovers = table("crossovers_per_meiosis")?;
        let female = need(crossovers.get("autosomal", "female"), "female crossovers")?;
        let male = need(crossovers.get("autosomal", "male"), "male crossovers")?;
        let autosomal_bp: u64 = bps[..X].iter().map(|(_, bp)| bp).sum();
        let chromosomes = bps
            .into_iter()
            .enumerate()
            .map(|(i, (name, bp))| {
                // A father's X and Y do not recombine here (D66).
                let share = if i <= X {
                    bp as f64 / autosomal_bp as f64
                } else {
                    0.0
                };
                Chromosome {
                    name,
                    bp,
                    cm_female: 100.0 * female * share,
                    cm_male: 100.0 * male * share,
                }
            })
            .collect();
        let rate = table("de_novo_mutation_rate")?;
        let traits_t = table("trait_heritability")?;
        let mut traits = Vec::new();
        for row in &traits_t.rows {
            traits.push(HeritableTrait {
                name: row.label.clone(),
                heritability: need(traits_t.get(&row.label, "heritability"), "heritability")?,
                cv: need(traits_t.get(&row.label, "cv"), "cv")?,
            });
        }
        Ok(Self {
            chromosomes,
            mutation_rate_per_bp: rate
                .get("per_bp_per_generation", "value")
                .ok_or("de_novo_mutation_rate: per_bp_per_generation")?,
            locus_spacing_bp: LOCUS_SPACING_BP,
            traits,
        })
    }

    /// Loci carried on chromosome `c`.
    pub fn loci(&self, c: usize) -> usize {
        (self.chromosomes[c].bp / self.locus_spacing_bp).max(1) as usize
    }

    fn sex_slot(y: bool) -> usize {
        if y {
            Y
        } else {
            X
        }
    }

    /// A founder's copy, its bases drawn evenly (D66: no founder population
    /// structure).
    pub fn founder_haplotype(&self, y: bool, s: &mut Stream) -> Haplotype {
        let chromosomes = (0..=X)
            .map(|c| {
                let c = if c == X { Self::sex_slot(y) } else { c };
                (0..self.loci(c)).map(|_| s.base()).collect()
            })
            .collect();
        Haplotype { y, chromosomes }
    }

    /// A founder: two unrelated copies and their own environment.
    pub fn founder(&self, male: bool, s: &mut Stream) -> Genome {
        Genome {
            maternal: self.founder_haplotype(false, s),
            paternal: self.founder_haplotype(male, s),
            environment: self.traits.iter().map(|_| s.normal()).collect(),
            de_novo: 0,
        }
    }

    /// One gamete of `parent`: recombined on the parent's map and mutated.
    /// `female` says which map; `y` asks a father for his Y. Returns the
    /// copy and the mutations it carries.
    pub fn gamete(&self, parent: &Genome, female: bool, y: bool, s: &mut Stream) -> (Haplotype, u32) {
        let mut mutations = 0;
        let mut chromosomes = Vec::with_capacity(X + 1);
        for c in 0..=X {
            let (a, b) = (&parent.maternal.chromosomes[c], &parent.paternal.chromosomes[c]);
            let mut copy: Vec<Base> = if c < X || female {
                // Autosomes, and a mother's two X: recombine.
                let ch = &self.chromosomes[c];
                let cm = if female { ch.cm_female } else { ch.cm_male };
                let n = a.len().min(b.len());
                let mut cuts: Vec<usize> = (0..s.poisson(cm / 100.0))
                    .map(|_| (s.uniform() * n as f64) as usize)
                    .collect();
                cuts.sort_unstable();
                let mut from_b = s.uniform() < 0.5;
                let mut next = cuts.iter().peekable();
                (0..n)
                    .map(|i| {
                        while next.peek().is_some_and(|&&cut| cut <= i) {
                            next.next();
                            from_b = !from_b;
                        }
                        if from_b {
                            b[i]
                        } else {
                            a[i]
                        }
                    })
                    .collect()
            } else if y {
                // A father's Y came from his father (D66: the pseudo-
                // autosomal regions are not recombined).
                parent.paternal.chromosomes[X].clone()
            } else {
                parent.maternal.chromosomes[X].clone()
            };
            let per_locus = self.mutation_rate_per_bp * self.locus_spacing_bp as f64;
            for base in copy.iter_mut() {
                if s.uniform() < per_locus {
                    *base = (*base + 1 + (s.uniform() * 3.0) as Base) & 3;
                    mutations += 1;
                }
            }
            chromosomes.push(copy);
        }
        let y = !female && y;
        (Haplotype { y, chromosomes }, mutations)
    }

    /// A child of `mother` and `father`, of the sex the runtime gave it.
    pub fn conceive(&self, mother: &Genome, father: &Genome, male: bool, s: &mut Stream) -> Genome {
        let (maternal, m) = self.gamete(mother, true, false, s);
        let (paternal, p) = self.gamete(father, false, male, s);
        Genome {
            maternal,
            paternal,
            environment: self.traits.iter().map(|_| s.normal()).collect(),
            de_novo: m + p,
        }
    }

    /// The standardised genetic score of trait `t`: the copies of each of
    /// its loci's effect base (C), centred and scaled for the founders'
    /// even base frequencies (p = 1/4, variance 2p(1-p) per locus).
    pub fn genetic_score(&self, g: &Genome, t: usize) -> f64 {
        let k = self.traits.len();
        let (mut sum, mut n) = (0.0, 0usize);
        let mut index = 0usize;
        for c in 0..X {
            for i in 0..self.loci(c) {
                if index % k == t {
                    let dose = u8::from(g.maternal.chromosomes[c][i] == 1)
                        + u8::from(g.paternal.chromosomes[c][i] == 1);
                    sum += f64::from(dose) - 0.5;
                    n += 1;
                }
                index += 1;
            }
        }
        if n == 0 {
            return 0.0;
        }
        sum / (n as f64 * 0.375).sqrt()
    }

    /// The multiplier trait `name` applies to the population's value for
    /// this person: `1 + cv * (sqrt(h2) genetic + sqrt(1 - h2) environment)`.
    pub fn trait_multiplier(&self, g: &Genome, name: &str) -> f64 {
        let Some(t) = self.traits.iter().position(|x| x.name == name) else {
            return 1.0;
        };
        let tr = &self.traits[t];
        let z = tr.heritability.sqrt() * self.genetic_score(g, t)
            + (1.0 - tr.heritability).sqrt() * g.environment.get(t).copied().unwrap_or(0.0);
        (1.0 + tr.cv * z).max(0.1)
    }

    /// Base pairs this genome's two copies stand for.
    pub fn diploid_bp(&self, g: &Genome) -> u64 {
        let auto: u64 = self.chromosomes[..X].iter().map(|c| c.bp).sum();
        let sex = |h: &Haplotype| self.chromosomes[Self::sex_slot(h.y)].bp;
        2 * auto + sex(&g.maternal) + sex(&g.paternal)
    }
}

/// One locus for each megabase (D66).
pub const LOCUS_SPACING_BP: u64 = 1_000_000;

impl Genome {
    /// Male if the father's copy carries a Y.
    pub fn male(&self) -> bool {
        self.paternal.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn map() -> GenomeMap {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reference");
        let library = ReferenceLibrary::load(&root).unwrap();
        GenomeMap::from_reference(&library).unwrap()
    }

    #[test]
    fn the_strands_pair() {
        for b in 0..4 {
            assert_ne!(complement(b), b);
            assert_eq!(complement(complement(b)), b);
        }
        assert_eq!(complement(0), 3, "A pairs with T");
        assert_eq!(complement(1), 2, "C pairs with G");
    }

    #[test]
    fn a_genome_spans_the_reference_assembly() {
        let m = map();
        let mut s = Stream::new(&[1; 32], "t");
        let man = m.founder(true, &mut s);
        let woman = m.founder(false, &mut s);
        assert!(man.male() && !woman.male());
        let loci: usize = woman.maternal.chromosomes.iter().map(Vec::len).sum();
        let haploid: u64 = m.chromosomes[..23].iter().map(|c| c.bp).sum();
        let whole = haploid / m.locus_spacing_bp;
        assert!(whole - loci as u64 <= 23, "one locus a megabase: {loci} of {whole}");
        assert!(m.diploid_bp(&woman) > m.diploid_bp(&man), "an X is longer than a Y");
    }

    #[test]
    fn crossovers_follow_each_sex_s_map() {
        let m = map();
        let mut s = Stream::new(&[2; 32], "t");
        // Parents whose two copies differ at every locus, so each switch
        // between them in a gamete is a crossover.
        let mut parent = m.founder(false, &mut s);
        for c in 0..23 {
            for (a, b) in parent.paternal.chromosomes[c]
                .iter_mut()
                .zip(&parent.maternal.chromosomes[c])
            {
                *a = complement(*b);
            }
        }
        let mut mapped = |female: bool| {
            let mut m2 = m.clone();
            m2.mutation_rate_per_bp = 0.0;
            let trials = 400;
            let mut switches = 0usize;
            for _ in 0..trials {
                let (g, _) = m2.gamete(&parent, female, false, &mut s);
                for c in 0..22 {
                    let from = &parent.maternal.chromosomes[c];
                    let g = &g.chromosomes[c];
                    switches += g
                        .windows(2)
                        .zip(from.windows(2))
                        .filter(|(w, f)| (w[0] == f[0]) != (w[1] == f[1]))
                        .count();
                }
            }
            switches as f64 / trials as f64 * 100.0
        };
        let (female, male) = (mapped(true), mapped(false));
        let total = |f: fn(&Chromosome) -> f64| m.chromosomes[..22].iter().map(f).sum::<f64>();
        // Two crossovers between neighbouring loci undo each other, so the
        // counted map runs a little short of the drawn one.
        assert!((female / total(|c| c.cm_female) - 1.0).abs() < 0.08, "{female}");
        assert!((male / total(|c| c.cm_male) - 1.0).abs() < 0.08, "{male}");
        assert!(female > male, "the female map is longer");
    }

    #[test]
    fn a_child_carries_the_cited_number_of_new_mutations() {
        let m = map();
        let mut s = Stream::new(&[3; 32], "t");
        let mum = m.founder(false, &mut s);
        let dad = m.founder(true, &mut s);
        let n = 200;
        let total: u32 = (0..n).map(|_| m.conceive(&mum, &dad, n % 2 == 0, &mut s).de_novo).sum();
        let mean = f64::from(total) / f64::from(n);
        let loci: usize = mum.maternal.chromosomes.iter().map(Vec::len).sum::<usize>() * 2;
        let expected = loci as f64 * m.mutation_rate_per_bp * m.locus_spacing_bp as f64;
        assert!((mean / expected - 1.0).abs() < 0.1, "{mean} vs {expected}");
    }

    #[test]
    fn a_child_takes_half_from_each_parent_and_the_father_decides_nothing_of_sex() {
        let m = map();
        let mut s = Stream::new(&[4; 32], "t");
        let mum = m.founder(false, &mut s);
        let dad = m.founder(true, &mut s);
        let son = m.conceive(&mum, &dad, true, &mut s);
        let daughter = m.conceive(&mum, &dad, false, &mut s);
        assert!(son.male() && !daughter.male());
        assert_eq!(son.paternal.chromosomes[22].len(), m.loci(23), "a son has his father's Y");
        assert_eq!(daughter.paternal.chromosomes[22].len(), m.loci(22));
        // Every autosomal locus of the child's maternal copy is one of the
        // mother's two bases, mutations aside.
        let foreign = (0..22)
            .flat_map(|c| (0..m.loci(c)).map(move |i| (c, i)))
            .filter(|&(c, i)| {
                let b = son.maternal.chromosomes[c][i];
                b != mum.maternal.chromosomes[c][i] && b != mum.paternal.chromosomes[c][i]
            })
            .count();
        assert!(foreign as u32 <= son.de_novo);
    }

    #[test]
    fn founders_vary_with_the_cited_heritability() {
        let m = map();
        let mut s = Stream::new(&[5; 32], "t");
        // Parent-offspring regression on the mid-parent value estimates h2.
        let t = 0;
        let h2 = m.traits[t].heritability;
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for _ in 0..300 {
            let mum = m.founder(false, &mut s);
            let dad = m.founder(true, &mut s);
            let kid = m.conceive(&mum, &dad, false, &mut s);
            let z = |g: &Genome| {
                h2.sqrt() * m.genetic_score(g, t) + (1.0 - h2).sqrt() * g.environment[t]
            };
            xs.push((z(&mum) + z(&dad)) / 2.0);
            ys.push(z(&kid));
        }
        let n = xs.len() as f64;
        let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
        let cov: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum::<f64>() / n;
        let var: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum::<f64>() / n;
        let slope = cov / var;
        assert!((slope - h2).abs() < 0.2, "slope {slope} against h2 {h2}");
    }
}
