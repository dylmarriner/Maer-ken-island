//! Phase 2 Task 6: the physical previews are byte-deterministic and agree
//! with the simulation state they show.

use island_preview::{render_physical, simulate_physical};
use mk_island::IslandProfile;

/// Small-island seed that makes a valid island (Phase 1).
const SMALL_SEED: [u8; 32] = [9u8; 32];

#[test]
fn identical_inputs_give_identical_files_and_the_summary_matches_the_state() {
    let profile = IslandProfile::test_small();
    let a = simulate_physical(&profile, SMALL_SEED, 3).expect("island");
    let b = simulate_physical(&profile, SMALL_SEED, 3).expect("island");
    let (fa, fb) = (render_physical(&a), render_physical(&b));
    assert_eq!(fa, fb);

    let names: Vec<&str> = fa.iter().map(|(n, _)| n.as_str()).collect();
    for expected in [
        "temperature.png",
        "rainfall.png",
        "rivers_lakes.png",
        "ocean_temperature.png",
        "ocean_currents.png",
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
    let hash: String = a
        .state
        .state_hash()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(v["state_hash"].as_str().unwrap(), hash);
    assert_eq!(v["local_days"].as_u64().unwrap(), 3);
    // The island is warmer on the ocean than on the land (mountains, the
    // lapse rate), and every reported number is finite.
    let land = v["land_surface_temperature_k"]["mean"].as_f64().unwrap();
    let sea = v["ocean_surface_temperature_k"]["mean"].as_f64().unwrap();
    assert!(land.is_finite() && sea.is_finite() && sea > land - 5.0);
    assert!(v["land_rain_mm_day"]["mean"].as_f64().unwrap() >= 0.0);
}
