//! Phase 4b Task 8: the town at its founding, from the cited packs alone.

use mk_engine::regional::town::{self, Opening};
use mk_engine::validation::{default_reference_dir, ReferenceLibrary};

fn library() -> ReferenceLibrary {
    ReferenceLibrary::load(&default_reference_dir()).expect("the reference packs load")
}

#[test]
fn the_founding_town_has_a_practice_a_school_and_their_households() {
    let openings = [Opening {
        enterprise: "Gem Petroleum".into(),
        role: "operator".into(),
        staff: 12,
    }];
    let founded = town::found_from_reference(&library(), &openings, &[3; 32]).unwrap();
    let staff = |enterprise: &str| {
        founded
            .people
            .iter()
            .filter(|p| p.job.as_ref().is_some_and(|j| j.0 == enterprise))
            .count()
    };
    assert_eq!(staff("Gem Petroleum"), 12);
    // One GP and their nurses (HTRHN), and a teacher per 25 children (MoE).
    assert_eq!(staff(town::HEALTH), 3);
    let teachers = staff(town::SCHOOL);
    assert!(teachers >= 1);
    assert!(teachers as f64 >= founded.school_age() as f64 / 25.0);

    // Every worker heads their own household, and everyone belongs to one.
    let workers = founded.people.iter().filter(|p| p.job.is_some()).count();
    assert_eq!(founded.households.len(), workers);
    let members: usize = founded.households.iter().map(|h| h.members.len()).sum();
    assert_eq!(members, founded.people.len());
}

#[test]
fn the_town_founds_the_same_way_from_the_same_seed() {
    let a = town::found_from_reference(&library(), &[], &[5; 32]).unwrap();
    let b = town::found_from_reference(&library(), &[], &[5; 32]).unwrap();
    assert_eq!(a, b);
    let c = town::found_from_reference(&library(), &[], &[6; 32]).unwrap();
    assert_ne!(a, c);
}
