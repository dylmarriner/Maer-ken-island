//! Phase 1 Task 7: the owner's island, locked.
//!
//! Loads `fixtures/island/default_profile.json` (with its pinned seed),
//! builds the regional geophysics twice and checks the Gate 1 contract:
//! identical results, area window, one landmass, open ocean in the buffer,
//! the shape requirements, finite tectonic and volcanic state, and the
//! deposits the owner saw in the gallery
//! (`docs/previews/phase1/gallery/gallery.json`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::geophysics::{
    bootstrap_regional_geophysics, primary_land_component, RegionalGeophysics,
};
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn build() -> (IslandProfile, IslandDomain, RegionalGeophysics) {
    let canon = CanonLocked::load(&repo("fixtures/island/canon.json")).expect("island canon");
    let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json"))
        .expect("default profile");
    let seed = profile
        .seed_bytes()
        .expect("the default profile pins the owner's seed");
    let domain = IslandDomain::from_profile(profile.clone()).unwrap();
    let boundaries = sample_regional_boundaries(seed, &canon, &domain, 0.0);
    let g = bootstrap_regional_geophysics(&canon, &domain, &boundaries, seed)
        .expect("the owner's island is valid");
    (profile, domain, g)
}

#[test]
fn the_owners_island_is_valid_deterministic_and_as_shown_in_the_gallery() {
    let (profile, domain, a) = build();
    let (_, _, b) = build();
    let bytes = |g: &RegionalGeophysics| blake3::hash(&bincode::serialize(g).expect("serialises"));
    assert_eq!(bytes(&a), bytes(&b), "two builds differ");

    let (lo, hi) = profile.land_area_bounds_m2();
    assert!(a.land_area_m2 >= lo && a.land_area_m2 <= hi);
    assert_eq!(
        primary_land_component(&a.elevation_m).data(),
        a.land_mask.data()
    );
    let medium = DomainLevel::Medium;
    for r in 0..domain.rows(medium) {
        for c in 0..domain.cols(medium) {
            if domain.is_edge_buffer_cell(medium, r, c) {
                assert!(!*a.land_mask.get(r, c), "land in the buffer at ({r}, {c})");
            }
        }
    }
    assert!(profile.shape.unmet(&a.shape).is_empty());
    assert!(a.tectonics.heat_flow.data().iter().all(|q| q.is_finite()));
    assert!(a
        .volcanism
        .volcanism
        .data()
        .iter()
        .all(|v| v.eruption_prob.is_finite() && v.heat_flux_mw_m2.is_finite()));
    assert!(a.elevation_m.data().iter().all(|e| e.is_finite()));

    // The deposits match the gallery entry the owner chose from.
    let gallery: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo("docs/previews/phase1/gallery/gallery.json")).unwrap(),
    )
    .unwrap();
    let seed_hex = profile.seed.clone().unwrap();
    let entry = gallery["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["seed"] == seed_hex.as_str())
        .expect("the chosen seed is in the gallery");
    assert!(entry["pass"].as_bool().unwrap());
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for d in &a.deposits {
        *counts.entry(format!("{:?}", d.kind)).or_default() += 1;
    }
    for (kind, value) in entry["deposits"].as_object().unwrap() {
        let shown = value["count"].as_u64().unwrap();
        assert_eq!(counts.get(kind).copied().unwrap_or(0), shown, "{kind}");
    }
    assert_eq!(
        entry["has_diamonds"].as_bool().unwrap(),
        counts.contains_key("Kimberlite")
    );
}
