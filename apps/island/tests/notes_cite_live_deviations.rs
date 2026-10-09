//! A note that points at a deviation must point at an open one.
//!
//! The conversations feed carries a paragraph telling the reader why the
//! lines repeat, and it names the deviation that explains it. That
//! paragraph has gone stale twice. It told readers the islanders
//! conversed "awake or asleep" for as long as it took to notice, after
//! the sleep gate had already stopped them; and it cited D35 after D35
//! was resolved, so a reader following the citation found a row saying
//! the thing was fixed.
//!
//! Both were found by opening the page in a browser, which is not a
//! thing that happens on every commit. A note explaining a limitation
//! the code no longer has is worse than no note at all, because nothing
//! about it looks wrong -- it is fluent, specific, and false, and a
//! reader has no way to tell. So the one part of it a machine can check
//! is checked: if it cites a deviation, that deviation must still be
//! open.
//!
//! This cannot tell whether the prose is true. It can tell that the row
//! it sends people to has not been closed underneath it, which is the
//! failure that actually happened.

use std::path::PathBuf;

fn deviations() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs/island/DEVIATIONS.md");
    std::fs::read_to_string(path).expect("the deviation register exists")
}

/// The status column of row `id`, as the register writes it.
fn status_of(id: &str) -> Option<String> {
    let register = deviations();
    let row = register
        .lines()
        .find(|line| line.starts_with(&format!("| {id} |")))?;
    let cells: Vec<&str> = row.trim().trim_matches('|').split('|').collect();
    Some(cells.last()?.trim().to_string())
}

/// Every `Dnn` the served note mentions.
fn cited(note: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes: Vec<char> = note.chars().collect();
    for (i, c) in bytes.iter().enumerate() {
        if *c != 'D' {
            continue;
        }
        let digits: String = bytes[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            found.push(format!("D{digits}"));
        }
    }
    found
}

#[test]
fn the_conversations_note_cites_a_deviation_that_is_still_open() {
    // Read from the served JSON rather than from the constant, because
    // what a reader sees is what is served. A constant renamed or no
    // longer used would pass a test written against the constant.
    let note = island::serve::server::conversations_note();
    // Citing nothing is allowed, and became the right answer the first
    // time this test fired in anger: D36 was fixed, so the note stopped
    // having a known limitation to point at. An earlier version of this
    // demanded a citation always exist, which would have forced a note to
    // keep pointing somewhere after there was nowhere true to point --
    // the failure this file exists to prevent, wearing the other face.
    for id in cited(note) {
        let status = status_of(&id)
            .unwrap_or_else(|| panic!("the note cites {id}, which is not in the register"));
        assert_eq!(
            status, "Open",
            "the note sends a reader to {id}, which the register now calls {status}. Either the \
             limitation it describes is gone and the note should say so, or it has moved to \
             another row and the note should cite that one."
        );
    }
}

#[test]
fn the_note_does_not_still_claim_the_islanders_talk_in_their_sleep() {
    // The exact sentence that was wrong, named so it cannot come back by
    // a copy-paste from an old branch. `step_dialogue` gates on being
    // awake and `one_sleeper_is_enough_to_stop_a_conversation` holds it
    // there, so this claim is false and was visible on the dashboard.
    let note = island::serve::server::conversations_note();
    assert!(
        !note.contains("awake or asleep"),
        "the note still tells readers the islanders converse in their sleep:\n{note}"
    );
}
