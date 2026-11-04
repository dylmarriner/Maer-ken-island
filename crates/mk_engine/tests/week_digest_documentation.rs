//! Keep the pinned founders' week digest consistent across active project guidance.

const ACCEPTANCE_SOURCE: &str = include_str!("island_life_acceptance.rs");
const WORK_REMAINING: &str = include_str!("../../../WORK_REMAINING.md");
const TESTING_GUIDE: &str = include_str!("../../../docs/TESTING.md");
const ISLAND_STATE: &str = include_str!("../../../docs/handoff/STATE_OF_THE_ISLAND.md");
const ACTIVE_HANDOFF: &str = include_str!("../../../HANDOFF.md");

fn pinned_week_digest() -> &'static str {
    const PREFIX: &str = "const WEEK_DIGEST: &str = \"";
    let line = ACCEPTANCE_SOURCE
        .lines()
        .find(|line| line.starts_with(PREFIX))
        .expect("the week acceptance test must declare exactly one canonical digest line");
    line.strip_prefix(PREFIX)
        .and_then(|value| value.strip_suffix("\";"))
        .expect("the week digest declaration must be a quoted string literal")
}

#[test]
fn active_week_digest_documentation_matches_the_acceptance_pin() {
    let digest = pinned_week_digest();
    assert_eq!(
        digest.len(),
        64,
        "the week digest must be a full BLAKE3 hex value"
    );
    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "the week digest must be lowercase hexadecimal"
    );

    for (path, document) in [
        ("WORK_REMAINING.md", WORK_REMAINING),
        ("docs/TESTING.md", TESTING_GUIDE),
        ("docs/handoff/STATE_OF_THE_ISLAND.md", ISLAND_STATE),
        ("HANDOFF.md", ACTIVE_HANDOFF),
    ] {
        assert!(
            document.contains(digest),
            "{path} must publish the exact WEEK_DIGEST from island_life_acceptance.rs"
        );
    }
}
