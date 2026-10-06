//! Phase 1 Task 6: previews are byte-deterministic and agree with the
//! simulation state they show.

use island_preview::{generate, render_gallery, render_geophysics};
use mk_island::IslandProfile;

const SMALL_SEED: [u8; 32] = [4u8; 32];

#[test]
fn identical_inputs_give_identical_files() {
    let profile = IslandProfile::test_small();
    let (d1, g1) = generate(&profile, SMALL_SEED);
    let (d2, g2) = generate(&profile, SMALL_SEED);
    let a = render_geophysics(&d1, &g1.unwrap(), &SMALL_SEED);
    let b = render_geophysics(&d2, &g2.unwrap(), &SMALL_SEED);
    assert_eq!(a, b);
    let names: Vec<&str> = a.iter().map(|(n, _)| n.as_str()).collect();
    for expected in [
        "elevation.png",
        "land_mask.png",
        "plates.png",
        "geology.png",
        "deposits.png",
        "summary.json",
    ] {
        assert!(names.contains(&expected), "{expected} missing");
    }
    for (name, bytes) in &a {
        if name.ends_with(".png") {
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{name}");
            assert!(
                !bytes.windows(4).any(|w| w == b"tIME" || w == b"tEXt"),
                "{name} has metadata"
            );
        }
    }
}

#[test]
fn summary_area_equals_the_simulated_land_area() {
    let profile = IslandProfile::test_small();
    let (d, g) = generate(&profile, SMALL_SEED);
    let g = g.unwrap();
    let files = render_geophysics(&d, &g, &SMALL_SEED);
    let summary = &files.iter().find(|(n, _)| n == "summary.json").unwrap().1;
    let v: serde_json::Value = serde_json::from_slice(summary).unwrap();
    assert_eq!(v["land_area_km2"].as_f64().unwrap(), g.land_area_m2 / 1e6);
    assert_eq!(v["land_components"].as_u64().unwrap(), 1);
    let total: u64 = v["deposits"]
        .as_object()
        .unwrap()
        .values()
        .map(|d| d["count"].as_u64().unwrap())
        .sum();
    assert_eq!(total as usize, g.deposits.len());
}

#[test]
fn the_gallery_lists_failing_seeds_with_reasons() {
    let files = render_gallery(&IslandProfile::test_small(), 8);
    let json = &files.iter().find(|(n, _)| n == "gallery.json").unwrap().1;
    let v: serde_json::Value = serde_json::from_slice(json).unwrap();
    let candidates = v["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 8);
    let failing: Vec<_> = candidates
        .iter()
        .filter(|c| !c["pass"].as_bool().unwrap())
        .collect();
    assert!(
        !failing.is_empty(),
        "expected some of 8 small seeds to fail"
    );
    for f in failing {
        assert!(f["reason"].as_str().is_some_and(|r| !r.is_empty()), "{f}");
    }
    assert!(files.iter().any(|(n, _)| n == "gallery.png"));
    assert_eq!(render_gallery(&IslandProfile::test_small(), 8), files);
}
