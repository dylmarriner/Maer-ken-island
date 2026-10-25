//! The town: the people who work the island's enterprises, and their
//! households (Phase 4b Task 8).
//!
//! The town has no target size (owner, 2026-10-15). It starts with each
//! enterprise's smallest staff (`Opening`, sourced per enterprise) plus
//! their households, and grows only as the businesses hire. This module
//! founds those first households from the scenario's seed:
//!
//! - **Each worker** is of working age, with sex and age drawn from New
//!   Zealand's estimated resident population by five-year band (Stats NZ,
//!   2018 ERP, `age_sex_structure_2018_erp`), restricted to the bands from
//!   20 to 64.
//! - **Their household** has a size drawn from the 2023 Census household
//!   size distribution (`household_size_distribution_2023`; eight or more
//!   counts as eight).
//! - **A household of two or more** has the worker's partner, of the other
//!   sex and from the worker's own age band, and children for the rest.
//!   A child's age is weighted by the Coale-Trussell natural fertility
//!   schedule at the parent's age when the child was born
//!   (`natural_marital_fertility`, 20 to 49), times the population at the
//!   child's age; a child is under 18. A household whose parent is too
//!   young for any child gets fewer members.
//!
//! Every draw is keyed by the seed and the person's place in the town, so
//! the same scenario founds the same town. The departures (no couples among
//! staff, no other relatives or flatmates, partners in one band) are D67.
//!
//! Who lives where (owner, 2026-10-13): everybody here lives in the town,
//! never in the founders' house.

use crate::validation::{ReferenceDomain, ReferenceLibrary, ReferenceTable};
use serde::{Deserialize, Serialize};

/// A job the town must fill: `staff` people in `role` at `enterprise`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opening {
    pub enterprise: String,
    pub role: String,
    pub staff: u32,
}

/// The town's general practice at its smallest: the single GP the smallest
/// New Zealand rural practices run with (HTRHN 2024,
/// `rural_gp_practice_smallest`) and nurses at the average rural practice's
/// ratio of nurse to GP FTE (HTRHN 2023, `rural_gp_practice_average_fte`),
/// at least one.
pub fn general_practice(
    library: &ReferenceLibrary,
    enterprise: &str,
) -> Result<Vec<Opening>, String> {
    let item = |key: &str| {
        library
            .item(ReferenceDomain::Industry, key)
            .ok_or_else(|| format!("industry/{key} is missing"))
    };
    let gps = item("rural_gp_practice_smallest")?
        .point()
        .ok_or("rural_gp_practice_smallest is not a point")?;
    let average = item("rural_gp_practice_average_fte")?
        .table()
        .ok_or("rural_gp_practice_average_fte is not a table")?;
    let fte = |row: &str| {
        average
            .get(row, "value")
            .ok_or_else(|| format!("rural_gp_practice_average_fte/{row} is missing"))
    };
    let nurses = (gps * fte("nurse_fte")? / fte("gp_fte")?).round().max(1.0);
    Ok(vec![
        Opening {
            enterprise: enterprise.into(),
            role: "general practitioner".into(),
            staff: gps as u32,
        },
        Opening {
            enterprise: enterprise.into(),
            role: "practice nurse".into(),
            staff: nurses as u32,
        },
    ])
}

/// A person founded in the town.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Townsperson {
    /// `town-<household>-<member>`: unique and stable for a seed.
    pub id: String,
    pub male: bool,
    pub age_years: f64,
    pub household: u32,
    /// The enterprise and role they were hired for, if they work.
    pub job: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Household {
    pub id: u32,
    /// Member ids, the worker first.
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Town {
    pub people: Vec<Townsperson>,
    pub households: Vec<Household>,
}

/// An age band `[from, to)` in years with a weight.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Band {
    from: f64,
    to: f64,
    weight: f64,
}

/// The cited tables the town is founded from.
#[derive(Clone, Debug, PartialEq)]
pub struct Demography {
    /// Population by band, men then women.
    population: [Vec<Band>; 2],
    /// Households by size (1..=8).
    household_sizes: Vec<(u32, f64)>,
    /// Births per woman-year by the mother's age band.
    fertility: Vec<Band>,
}

/// The working ages: workers are drawn from these bands.
const WORKING_FROM: f64 = 20.0;
const WORKING_TO: f64 = 65.0;
/// A New Zealand child may start school at this age.
const SCHOOL_FROM: f64 = 5.0;
/// A dependent child is under this age.
const CHILD_UNDER: f64 = 18.0;
/// The oldest band's upper end when the table leaves it open (`90+`).
const OPEN_BAND_YEARS: f64 = 10.0;

fn band(label: &str) -> Result<(f64, f64), String> {
    if let Some(from) = label.strip_suffix('+') {
        let from: f64 = from.parse().map_err(|_| format!("age band {label}"))?;
        return Ok((from, from + OPEN_BAND_YEARS));
    }
    let (a, b) = label
        .split_once('-')
        .ok_or_else(|| format!("age band {label}"))?;
    let a: f64 = a.parse().map_err(|_| format!("age band {label}"))?;
    let b: f64 = b.parse().map_err(|_| format!("age band {label}"))?;
    Ok((a, b + 1.0))
}

fn bands(t: &ReferenceTable, col: &str) -> Result<Vec<Band>, String> {
    t.column(col)
        .ok_or_else(|| format!("demography column {col} is missing"))?
        .into_iter()
        .map(|(label, weight)| {
            let (from, to) = band(label)?;
            Ok(Band { from, to, weight })
        })
        .collect()
}

impl Demography {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let table = |key: &str| {
            library
                .item(ReferenceDomain::Humans, key)
                .and_then(|i| i.table())
                .ok_or_else(|| format!("humans/{key} is missing"))
        };
        let ages = table("age_sex_structure_2018_erp")?;
        let sizes = table("household_size_distribution_2023")?
            .column("households")
            .ok_or("household sizes are missing")?
            .into_iter()
            .map(|(label, n)| {
                let size: u32 = label
                    .split('_')
                    .next()
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("household size {label}"))?;
                Ok((size, n))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            population: [bands(ages, "male")?, bands(ages, "female")?],
            household_sizes: sizes,
            fertility: bands(table("natural_marital_fertility")?, "births_per_woman_year")?,
        })
    }

}

/// Uniform draws keyed by the seed and a label.
struct Draws {
    key: [u8; 32],
    label: String,
    n: u64,
}

impl Draws {
    fn new(seed: &[u8; 32], label: String) -> Self {
        Self {
            key: *seed,
            label,
            n: 0,
        }
    }

    /// In [0, 1).
    fn uniform(&mut self) -> f64 {
        let mut h = blake3::Hasher::new_keyed(&self.key);
        h.update(b"town:");
        h.update(self.label.as_bytes());
        h.update(&self.n.to_le_bytes());
        self.n += 1;
        let bytes: [u8; 8] = h.finalize().as_bytes()[..8].try_into().unwrap();
        (u64::from_le_bytes(bytes) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// An index into `weights`, in proportion to them; `None` if all are 0.
    fn pick(&mut self, weights: &[f64]) -> Option<usize> {
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            return None;
        }
        let mut x = self.uniform() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return Some(i);
            }
            x -= w;
        }
        weights.iter().rposition(|w| *w > 0.0)
    }
}

/// Found the town for `openings`: one household per worker.
/// The town at its founding: the given enterprises' openings, the smallest
/// general practice and a school staffed for the children all of them bring.
pub fn found_from_reference(
    library: &ReferenceLibrary,
    openings: &[Opening],
    seed: &[u8; 32],
) -> Result<Town, String> {
    let demography = Demography::from_reference(library)?;
    let mut all = openings.to_vec();
    all.extend(general_practice(library, HEALTH)?);
    let mut town = found(&all, &demography, seed);
    let students = library
        .item(
            ReferenceDomain::Industry,
            "small_school_students_per_teacher",
        )
        .and_then(|i| i.point())
        .ok_or("industry/small_school_students_per_teacher is missing")?;
    town.staff_school(SCHOOL, students, &demography, seed);
    Ok(town)
}

/// The town's health and school businesses, Gem-D and Gem-K's like every
/// other in the town (owner, 2026-10-13).
pub const HEALTH: &str = "Gem Health";
pub const SCHOOL: &str = "Gem School";

pub fn found(openings: &[Opening], demography: &Demography, seed: &[u8; 32]) -> Town {
    let mut town = Town::default();
    for opening in openings {
        for _ in 0..opening.staff {
            town.settle(&opening.enterprise, &opening.role, demography, seed);
        }
    }
    town
}

impl Town {
    /// Children of school age: from 5, when a New Zealand child may start
    /// school, to the end of childhood here (D67).
    pub fn school_age(&self) -> usize {
        self.people
            .iter()
            .filter(|p| p.job.is_none() && (SCHOOL_FROM..CHILD_UNDER).contains(&p.age_years))
            .count()
    }

    /// Hire teachers for `enterprise` until there is one for every
    /// `students_per_teacher` children of school age (the Ministry of
    /// Education's small-school curriculum staffing,
    /// `small_school_students_per_teacher`), counting the teachers' own
    /// children, and at least one.
    pub fn staff_school(
        &mut self,
        enterprise: &str,
        students_per_teacher: f64,
        demography: &Demography,
        seed: &[u8; 32],
    ) {
        loop {
            let teachers = self
                .people
                .iter()
                .filter(|p| {
                    p.job
                        .as_ref()
                        .is_some_and(|j| j.0 == enterprise && j.1 == "teacher")
                })
                .count();
            let needed = ((self.school_age() as f64 / students_per_teacher).ceil() as usize).max(1);
            if teachers >= needed {
                break;
            }
            self.settle(enterprise, "teacher", demography, seed);
        }
    }

    /// One worker hired for `role` at `enterprise`, and their household.
    pub fn settle(
        &mut self,
        enterprise: &str,
        role: &str,
        demography: &Demography,
        seed: &[u8; 32],
    ) {
        let household = self.households.len() as u32;
        let mut d = Draws::new(seed, format!("household:{household}"));
        let mut members = Vec::new();
        let mut person = |member: usize, male: bool, age: f64, job: Option<(String, String)>| {
            let id = format!("town-{household}-{member}");
            members.push(id.clone());
            Townsperson {
                id,
                male,
                age_years: age,
                household,
                job,
            }
        };

        // The worker: a working-age band by sex, then an age within it.
        let working: Vec<(bool, Band)> = [true, false]
            .into_iter()
            .flat_map(|male| {
                demography.population[usize::from(!male)]
                    .iter()
                    .filter(|b| b.from >= WORKING_FROM && b.to <= WORKING_TO)
                    .map(move |b| (male, *b))
            })
            .collect();
        let weights: Vec<f64> = working.iter().map(|(_, b)| b.weight).collect();
        let (male, b) = working[d.pick(&weights).expect("working ages are tabled")];
        let age = b.from + d.uniform() * (b.to - b.from);
        let mut people = vec![person(
            0,
            male,
            age,
            Some((enterprise.to_string(), role.to_string())),
        )];

        let sizes: Vec<f64> = demography.household_sizes.iter().map(|s| s.1).collect();
        let size =
            demography.household_sizes[d.pick(&sizes).expect("household sizes are tabled")].0;
        if size >= 2 {
            let partner_age = b.from + d.uniform() * (b.to - b.from);
            people.push(person(1, !male, partner_age, None));
            // Children are drawn over the exact intervals where both the
            // mother's fertility schedule and the child's population band
            // apply. Sampling a whole integer-year bucket first could push
            // the final birth age outside its selected fertility band.
            let mother_age = if male { partner_age } else { age };
            for member in 2..size as usize {
                let child_male = d.uniform() < 0.5;
                let mut intervals = Vec::new();
                let mut weights = Vec::new();
                for fertility in &demography.fertility {
                    if fertility.weight <= 0.0 {
                        continue;
                    }
                    for population in &demography.population[usize::from(!child_male)] {
                        if population.weight <= 0.0 || population.to <= population.from {
                            continue;
                        }
                        let from = (mother_age - fertility.to).max(population.from).max(0.0);
                        let to = (mother_age - fertility.from)
                            .min(population.to)
                            .min(CHILD_UNDER);
                        if to > from {
                            intervals.push((from, to));
                            // Integrate the piecewise-constant fertility and
                            // population density over the actual age range.
                            weights.push(
                                (to - from) * fertility.weight * population.weight
                                    / (population.to - population.from),
                            );
                        }
                    }
                }
                let Some(i) = d.pick(&weights) else { break };
                let (from, to) = intervals[i];
                let child_age = from + d.uniform() * (to - from);
                people.push(person(member, child_male, child_age, None));
            }
        }
        self.people.extend(people);
        self.households.push(Household {
            id: household,
            members,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn demography() -> Demography {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reference");
        Demography::from_reference(&ReferenceLibrary::load(&root).unwrap()).unwrap()
    }

    fn openings(n: u32) -> Vec<Opening> {
        vec![
            Opening {
                enterprise: "Gem Health".into(),
                role: "general practitioner".into(),
                staff: 2,
            },
            Opening {
                enterprise: "Gem Petroleum".into(),
                role: "operator".into(),
                staff: n,
            },
        ]
    }

    #[test]
    fn every_opening_is_filled_by_a_working_age_adult() {
        let town = found(&openings(30), &demography(), &[7; 32]);
        let workers: Vec<_> = town.people.iter().filter(|p| p.job.is_some()).collect();
        assert_eq!(workers.len(), 32);
        assert_eq!(town.households.len(), 32);
        for w in workers {
            assert!((WORKING_FROM..WORKING_TO).contains(&w.age_years), "{w:?}");
        }
        let gps = town
            .people
            .iter()
            .filter(|p| {
                p.job
                    .as_ref()
                    .is_some_and(|j| j.1 == "general practitioner")
            })
            .count();
        assert_eq!(gps, 2);
    }

    #[test]
    fn the_smallest_practice_is_one_gp_and_their_nurses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reference");
        let library = ReferenceLibrary::load(&root).unwrap();
        let practice = general_practice(&library, "Gem Health").unwrap();
        assert_eq!(practice[0].staff, 1);
        // 3.89 nurse FTE to 2.49 GP FTE: two nurses for one GP.
        assert_eq!(practice[1].staff, 2);
    }

    #[test]
    fn the_school_hires_a_teacher_for_every_25_children() {
        let d = demography();
        let mut town = found(&openings(60), &d, &[9; 32]);
        town.staff_school("Gem School", 25.0, &d, &[9; 32]);
        let teachers = town
            .people
            .iter()
            .filter(|p| p.job.as_ref().is_some_and(|j| j.1 == "teacher"))
            .count();
        let children = town.school_age();
        assert!(teachers >= 1);
        assert!(teachers as f64 >= children as f64 / 25.0);
        // No more teachers than the children need.
        assert!((teachers - 1) as f64 * 25.0 < children.max(1) as f64);
    }

    #[test]
    fn school_caretakers_do_not_satisfy_the_teacher_requirement() {
        let d = demography();
        let mut town = found(
            &[Opening {
                enterprise: SCHOOL.into(),
                role: "caretaker".into(),
                staff: 1,
            }],
            &d,
            &[19; 32],
        );
        town.staff_school(SCHOOL, 25.0, &d, &[19; 32]);
        let teachers = town
            .people
            .iter()
            .filter(|p| {
                p.job
                    .as_ref()
                    .is_some_and(|j| j.0 == SCHOOL && j.1 == "teacher")
            })
            .count();
        assert!(
            teachers >= 1,
            "non-teaching staff cannot fill teacher positions"
        );
        assert!(teachers as f64 * 25.0 >= town.school_age() as f64);
    }

    #[test]
    fn the_same_seed_founds_the_same_town() {
        let d = demography();
        assert_eq!(
            found(&openings(10), &d, &[3; 32]),
            found(&openings(10), &d, &[3; 32])
        );
        assert_ne!(
            found(&openings(10), &d, &[3; 32]),
            found(&openings(10), &d, &[4; 32])
        );
    }

    #[test]
    fn household_sizes_follow_the_census() {
        let d = demography();
        let town = found(&openings(4_000), &d, &[11; 32]);
        let total: f64 = d.household_sizes.iter().map(|s| s.1).sum();
        // Ones and twos never fall short: a single worker and a couple
        // always fit. Their shares match the census within sampling error.
        for size in [1usize, 2] {
            let want = d
                .household_sizes
                .iter()
                .find(|s| s.0 as usize == size)
                .unwrap()
                .1
                / total;
            let got = town
                .households
                .iter()
                .filter(|h| h.members.len() == size)
                .count() as f64
                / town.households.len() as f64;
            assert!((got - want).abs() < 0.03, "size {size}: {got} vs {want}");
        }
        // Larger households are never larger than drawn.
        assert!(town.households.iter().all(|h| h.members.len() <= 8));
    }

    #[test]
    fn children_are_born_to_mothers_of_fertile_age() {
        let d = demography();
        let town = found(&openings(1500), &d, &[5; 32]);
        for h in &town.households {
            let members: Vec<&Townsperson> = h
                .members
                .iter()
                .map(|id| town.people.iter().find(|p| &p.id == id).unwrap())
                .collect();
            if members.len() < 3 {
                continue;
            }
            let mother = members[..2].iter().find(|p| !p.male).unwrap();
            for child in &members[2..] {
                assert!(child.age_years < CHILD_UNDER);
                let at_birth = mother.age_years - child.age_years;
                assert!(
                    d.fertility.iter().any(|band| {
                        (band.from..band.to).contains(&at_birth) && band.weight > 0.0
                    }),
                    "child born when mother was {at_birth}, outside actual fertility bands"
                );
            }
        }
    }
}
