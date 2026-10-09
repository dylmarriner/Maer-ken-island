//! How often the islanders actually talk, and what that does to the memory
//! that is supposed to hold a lifetime of it.
//!
//! This began as a measurement of D35's open half: waking family pairs were
//! matched on every tick, once a simulated minute, and each minute was filed
//! as a separate remembered conversation. The measurement is what decided
//! the fix. Counted in *words* rather than conversations the rate was never
//! the defect -- the founders talk about 1.8x as much as the average person
//! Mehl et al. recorded, which is inside the range those recorders observed.
//! What was wrong was the division: an unbroken afternoon was being cut into
//! minute-long fragments and each fragment remembered separately.
//!
//! So `HumanSystem::step_dialogue` now carries an exchange on when the same
//! pair speak again on the next tick, and this test guards the result from
//! both sides. It asserts that the founders talk at all, that a day no
//! longer eats the cap and fits inside the line budget, and it prints the
//! word count beside a real measurement of human speech so the rate stays
//! arguable on numbers.
//!
//! The number that matters is no longer `CONVERSATION_HISTORY_MAX_ENTRIES`.
//! Merging does not reduce how much is said -- a day is 2,042 lines either
//! way -- so once an entry became a whole exchange, an entry count stopped
//! bounding anything and `CONVERSATION_HISTORY_MAX_LINES` is the real
//! limit. This prints what a day costs against it, in days, because days
//! are the unit the question was always asked in.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::humans::{CONVERSATION_HISTORY_MAX_ENTRIES, CONVERSATION_HISTORY_MAX_LINES};
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

const DAY: u64 = 86_400;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn life() -> IslandLife {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).expect("the island bootstraps")
}

#[test]
#[ignore = "slow: one simulated day on the full island"]
fn slow_how_long_a_lifetime_of_remembered_conversation_actually_lasts() {
    let mut w = life();
    w.advance(DAY).expect("a day passes");

    let mut per_human = Vec::new();
    for id in ["Gem-D", "Gem-K"] {
        let human = w.humans.registry.get_human(id).expect("a founder");
        per_human.push((id, human.conversation_history.len()));
    }

    println!("\n== One simulated day (36 h on Marr'Kena) ==\n");
    for (id, held) in &per_human {
        println!("{id}: {held} conversations remembered");
    }
    let most = per_human.iter().map(|(_, n)| *n).max().expect("a founder");
    println!(
        "caps: {CONVERSATION_HISTORY_MAX_ENTRIES} entries, {CONVERSATION_HISTORY_MAX_LINES} lines"
    );

    // And the same day in words, which is the figure a real measurement
    // of human talking can be set beside. Mehl et al., *Science* 317:82
    // (2007), put a wearable recorder on 396 people and found about
    // **16,000 words a day** each -- women 16,215, men 15,669, the
    // difference not significant -- over roughly 17 waking hours. That
    // is every word spoken to anybody; on this island the founders have
    // only each other, so all of it would go to one listener.
    //
    // Marr'Kena's day is 36 hours against Earth's 24, so a like-for-like
    // comparison scales the human figure by the waking hours actually
    // available here rather than comparing a long day to a short one.
    // From one founder's own remembered history rather than the shared
    // display feed, which is a bounded ring and would undercount a day.
    let gem_d = w.humans.registry.get_human("Gem-D").expect("a founder");
    let mut words = 0usize;
    let mut lines = 0usize;
    // How much of it is the same thing said again. The lines are
    // composed from state that barely moves in sixty seconds, so a day
    // of talking draws on a small stock of sentences; counting the
    // distinct ones says how small. This is D36's figure.
    let mut said = std::collections::BTreeSet::new();
    for event in &gem_d.conversation_history {
        for line in &event.lines {
            lines += 1;
            words += line.text.split_whitespace().count();
            said.insert(line.text.clone());
        }
    }
    let per_founder = words as f64;
    println!("\nGem-D: {lines} lines, {words} words across the day");
    println!(
        "{} of those {lines} lines are distinct sentences",
        said.len()
    );
    println!("about {per_founder:.0} words in a 36-hour day");
    // 17 waking hours in Mehl's 24-hour day; the island's day is 36, so
    // the hours awake scale with it unless the sleep model says
    // otherwise. This is the comparison, stated so a reader can argue
    // with the scaling rather than having it buried.
    let human_equivalent = 16_000.0 * (36.0 / 24.0);
    println!(
        "a person measured by Mehl et al. (2007) would speak about {human_equivalent:.0} words \
         in a day of this length"
    );
    if per_founder > 0.0 {
        println!(
            "the island is at {:.1}x that",
            per_founder / human_equivalent
        );
    }

    assert!(
        most > 0,
        "the founders are adjacent kin and should talk at all"
    );

    // The guard that D35's open half stays closed. Before the merge a
    // single day filled about a thousand of the two thousand entries, so
    // a cap meant to hold a lifetime held two days. A day's talking is
    // now a couple of unbroken exchanges -- the waking stretches either
    // side of sleep -- and the cap holds hundreds of days.
    //
    // A tenth of the cap is the line the earlier version of this test
    // drew, from the other side: it failed while a day took *more* than
    // a tenth. Keeping the same line keeps the two readings comparable
    // rather than inventing a new threshold to pass.
    assert!(
        most * 10 <= CONVERSATION_HISTORY_MAX_ENTRIES,
        "a single day fills {most} of {CONVERSATION_HISTORY_MAX_ENTRIES} entries, over a tenth of \
         the cap; conversations are being fragmented again"
    );

    // Entries stopped being the real bound when an entry became a whole
    // exchange, so this is the one that decides how long the memory
    // reaches: lines. Printed in days, which is the unit the question was
    // ever asked in.
    println!(
        "a day costs {lines} lines of a {CONVERSATION_HISTORY_MAX_LINES}-line budget, which \
         holds about {:.1} days",
        CONVERSATION_HISTORY_MAX_LINES as f64 / lines.max(1) as f64
    );
    assert!(
        lines < CONVERSATION_HISTORY_MAX_LINES,
        "a single day's talking is {lines} lines against a {CONVERSATION_HISTORY_MAX_LINES}-line \
         budget, so a person cannot remember even one day of it"
    );

    // And from the other side, so this cannot be passed by the founders
    // falling silent: the day has to hold a real amount of talking. One
    // line a minute of waking life would be far more than anyone says;
    // this only asks that the day is not empty of words.
    assert!(
        words > 1_000,
        "a whole day produced only {words} words, which is not two people living together"
    );
}
