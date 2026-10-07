//! Phase 3 Task 9 Step 5: the life previews are byte-deterministic and
//! agree with the simulated state they show.

use std::path::PathBuf;

use island_preview::bootstrap_life;
use island_preview::life::render_life;
use mk_island::IslandScenario;

#[test]
fn identical_inputs_give_identical_files_and_the_summary_matches_the_state() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/island/default_scenario.json");
    let load = || bootstrap_life(IslandScenario::load(&path).unwrap()).expect("island");
    let (a, b) = (load(), load());
    let (fa, fb) = (render_life(&a), render_life(&b));
    assert_eq!(fa, fb);
    let names: Vec<&str> = fa.iter().map(|(n, _)| n.as_str()).collect();
    for expected in [
        "biomes.png",
        "resources.png",
        "estate_plan.png",
        "estate_trees.png",
        "summary.json",
    ] {
        assert!(names.contains(&expected), "{expected} missing");
    }
    for (name, bytes) in &fa {
        if name.ends_with(".png") {
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{name}");
            assert!(
                !bytes.windows(4).any(|w| w == b"tIME" || w == b"tEXt"),
                "{name} has metadata"
            );
        }
    }
    let summary = &fa.iter().find(|(n, _)| n == "summary.json").unwrap().1;
    let v: serde_json::Value = serde_json::from_slice(summary).unwrap();
    let digest: String = a
        .state_digest()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(v["state_digest"].as_str().unwrap(), digest);
    assert_eq!(
        v["patch_trees"].as_u64().unwrap() as usize,
        a.vegetation.trees.len()
    );
    assert_eq!(v["buildings"].as_u64().unwrap(), 5);
    assert_eq!(
        v["items"].as_u64().unwrap() as usize,
        a.placed.layout.items.len()
    );
}
