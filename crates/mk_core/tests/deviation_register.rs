//! Keeps `docs/island/DEVIATIONS.md` well formed: every deviation names the
//! phase that owns it and a status from `docs/island/REALISM.md` §4.

use std::path::PathBuf;

const PHASES: [&str; 9] = ["0", "0b", "0c", "1", "2", "3", "4", "4b", "5"];
const STATUSES: [&str; 3] = ["Open", "Accepted", "Resolved"];

fn rows() -> Vec<Vec<String>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/island/DEVIATIONS.md");
    let text = std::fs::read_to_string(path).expect("deviation register exists");
    text.lines()
        .filter(|line| line.starts_with("| D"))
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

#[test]
fn every_deviation_has_seven_columns_an_owning_phase_and_a_status() {
    let rows = rows();
    assert!(!rows.is_empty(), "no deviations found");
    for row in &rows {
        assert_eq!(row.len(), 7, "{row:?}");
        assert!(
            PHASES.contains(&row[5].as_str()),
            "unknown phase in {row:?}"
        );
        assert!(
            STATUSES.contains(&row[6].as_str()),
            "unknown status in {row:?}"
        );
        for cell in &row[..5] {
            assert!(!cell.is_empty(), "empty cell in {row:?}");
        }
    }
}

#[test]
fn deviation_ids_are_unique_and_sequential() {
    let ids: Vec<String> = rows().into_iter().map(|row| row[0].clone()).collect();
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(id, &format!("D{}", index + 1));
    }
}
