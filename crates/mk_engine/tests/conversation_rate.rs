//! How often the islanders actually talk, and what that does to the memory
//! that is supposed to hold a lifetime of it.
//!
//! D35's open half is the rate: waking family pairs are matched on every
//! tick, which is once a simulated minute. Whether that should be a
//! cooldown, a chosen action, or something else is a decision about the
//! model. This measures the consequence so the decision can be made on
//! numbers instead of impressions.
//!
//! The number that matters is `CONVERSATION_HISTORY_MAX_ENTRIES`, whose own
//! comment says it is "sized like a lifetime of remembered exchanges rather
//! than a shared display ring buffer, per the user's explicit
//! 'conversations should be remembered, not a rolling log' constraint". At
//! one conversation per tick that intent and that cap cannot both hold.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::humans::CONVERSATION_HISTORY_MAX_ENTRIES;
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
    println!("cap: {CONVERSATION_HISTORY_MAX_ENTRIES}");
    if most > 0 {
        let days = CONVERSATION_HISTORY_MAX_ENTRIES as f64 / most as f64;
        println!("the cap holds about {days:.1} of these days before it evicts");
    }

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
    for event in &gem_d.conversation_history {
        for line in &event.lines {
            lines += 1;
            words += line.text.split_whitespace().count();
        }
    }
    let per_founder = words as f64;
    println!("\nGem-D: {lines} lines, {words} words across the day");
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

    // Asserting the shape of the problem, not a target. If a day's talking
    // ever stops filling a meaningful share of a cap meant to last a
    // lifetime, the rate has been given one and this test should be
    // revisited along with D35.
    assert!(
        most > 0,
        "the founders are adjacent kin and should talk at all"
    );
    assert!(
        most * 10 > CONVERSATION_HISTORY_MAX_ENTRIES,
        "a single day fills {most} of {CONVERSATION_HISTORY_MAX_ENTRIES}, under a tenth of the \
         cap; if that is now true the conversation rate has been given a limit and D35's open \
         half may be closed"
    );
}
