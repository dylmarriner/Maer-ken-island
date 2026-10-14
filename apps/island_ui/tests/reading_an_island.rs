//! The desktop application reading an island, both ways it can.
//!
//! This is the test that matters most for what this application is: a
//! frontend that runs on a different computer from the island. It starts
//! a real backend, reads it the way the application does, and checks that
//! what comes back is enough to draw -- terrain the ground is made from,
//! people with places, the estate's own things.
//!
//! It runs headlessly, because everything it exercises is headless. The
//! renderer is the only part that needs a GPU, and this container has
//! none; `docs/island/RENDER_STACK.md` says which parts that leaves
//! unverified rather than leaving it to be assumed.

use island_ui::scene;
use island_ui::source::{Island, LocalBackend};
use island_ui::terrain;

fn repo(path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// A small island: the same one the dashboard's own suite uses, for the
/// same reason. The grids rather than the trees are what cost, and this
/// container has four cores.
fn a_small_island() -> (tempfile::TempDir, LocalBackend) {
    let data = tempfile::tempdir().expect("a data directory");
    let scenario = repo("fixtures/island/default_scenario.json");
    // Written out with the small profile, because `LocalBackend` takes a
    // path rather than a scenario -- which is what `island serve` takes
    // too, and keeping them the same is the point.
    let mut loaded = mk_island::IslandScenario::load(&scenario).expect("the scenario loads");
    loaded.estate_patch.tree_cap = 50;
    loaded.profile = mk_island::IslandProfile::test_small();
    let mut seed = [0u8; 32];
    seed[..4].copy_from_slice(&16u32.to_le_bytes());
    loaded.seed = seed;
    // The canon path in the fixture is relative to the repository, and
    // this scenario is about to be written somewhere else. Made absolute
    // rather than copying the canon file beside it: there is one canon
    // and a second copy of it is a second thing that can drift.
    loaded.canon_path = repo("fixtures/island/canon.json");
    let small = data.path().join("small_scenario.json");
    std::fs::write(
        &small,
        serde_json::to_string(&loaded).expect("the scenario serializes"),
    )
    .expect("the scenario is written");

    let backend = LocalBackend::start(
        Some(&small),
        None,
        &data.path().join("people"),
        island::serve::sim::SimSpeed::Times(600),
    )
    .expect("the island starts");
    (data, backend)
}

#[test]
fn the_application_reads_an_island_and_has_enough_to_draw_it() {
    let (_data, backend) = a_small_island();
    let address = backend.address.clone();
    let island = Island::embedded(backend).expect("the application connects to it");

    // Everything the application was told it could do.
    assert!(island.capabilities().world, "there is an island here");
    assert!(island.capabilities().elevation);
    assert!(island.is_local());
    assert_eq!(island.address(), address);

    // The ground, which is fetched once because it cannot change.
    let ground = island.terrain().expect("terrain").clone();
    assert!(ground.rows > 0 && ground.cols > 0);
    assert_eq!(ground.elevation_m.len(), ground.rows * ground.cols);
    assert_eq!(ground.land.len(), ground.rows * ground.cols);
    let (lowest, highest) = ground.relief_m().expect("the island has relief");
    assert!(lowest < 0.0 && highest > 0.0, "{lowest} to {highest}");
    assert!(
        ground.land.iter().any(|l| *l) && ground.land.iter().any(|l| !*l),
        "an island is land with sea around it"
    );

    // And it makes meshes.
    let chunks = terrain::chunks(&ground);
    assert!(!chunks.is_empty());
    let vertices: usize = chunks.iter().map(|c| c.positions.len()).sum();
    let triangles: usize = chunks.iter().map(|c| c.indices.len() / 3).sum();
    assert!(vertices > 0 && triangles > 0);
    for chunk in &chunks {
        assert!(chunk
            .indices
            .iter()
            .all(|i| (*i as usize) < chunk.positions.len()));
    }
    println!(
        "{} chunks, {vertices} vertices, {triangles} triangles",
        chunks.len()
    );

    // Wait for the poller to have read the island at least once. It runs
    // on its own thread precisely so a frame never waits on it, which
    // means a test has to.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let snapshot = loop {
        let snapshot = island.snapshot();
        if snapshot.ever_read {
            break snapshot;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the poller never read the island: {:?}",
            snapshot.trouble
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    assert_eq!(snapshot.trouble, None, "the first read failed");

    // The founders, with places on the ground.
    let ids: Vec<&str> = snapshot
        .world
        .people
        .iter()
        .map(|p| p.agent_id.as_str())
        .collect();
    assert!(ids.contains(&"Gem-D") && ids.contains(&"Gem-K"), "{ids:?}");

    let estate_origin = (
        snapshot.world.estate.cell.1 as f64 * snapshot.world.land.cell_size_m,
        snapshot.world.estate.cell.0 as f64 * snapshot.world.land.cell_size_m,
    );
    let placed = scene::place_people(&ground, &snapshot.world, estate_origin);
    assert_eq!(
        placed.len(),
        snapshot.world.people.len(),
        "somebody on the island could not be placed on it"
    );
    for person in &placed {
        assert!(
            person.estate.is_some(),
            "{} starts on the estate and should have metres",
            person.agent_id
        );
        assert!(
            person.regional.y.is_finite()
                && person.regional.x.is_finite()
                && person.regional.z.is_finite(),
            "{person:?}"
        );
    }

    // And the estate's own things, the computer room included -- which is
    // the part of this that the goal names.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let properties = loop {
        let snapshot = island.snapshot();
        if !snapshot.properties.is_empty() {
            break snapshot.properties;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the slower views never arrived: {:?}",
            snapshot.trouble
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    // Only this island's estate. `/api/properties` also carries an
    // unowned homestead template with no location -- the first version of
    // this test counted both and found eight computers, which is the
    // inventory having two records rather than the room having eight
    // machines.
    let here: Vec<&mk_island_api::Property> =
        properties.iter().filter(|p| p.on_this_island).collect();
    assert_eq!(here.len(), 1, "one estate stands on this island: {here:?}");
    let computers: Vec<&mk_island_api::Item> = here
        .iter()
        .flat_map(|p| &p.items)
        .filter(|item| item.kind == "Computer")
        .collect();
    assert_eq!(
        computers.len(),
        4,
        "the computer room holds four machines: {computers:?}"
    );
    for machine in &computers {
        let model = island_ui::property::item(&machine.kind, &machine.name)
            .unwrap_or_else(|| panic!("{} has no model of its own", machine.name));
        assert!(
            repo("assets").join(model.path).is_file(),
            "{} is drawn from {}, which is not there",
            machine.name,
            model.path
        );
    }

    // Every building on the estate, likewise: a model, or no model and on
    // the list that says so -- never a silent substitution.
    for building in here.iter().flat_map(|p| &p.buildings) {
        if let Some(model) = island_ui::property::building(&building.kind) {
            assert!(
                repo("assets").join(model.path).is_file(),
                "{} is drawn from {}, which is not there",
                building.name,
                model.path
            );
        } else {
            assert!(
                island_ui::property::WITHOUT_MODELS.contains(&building.kind.as_str()),
                "{} has no model and is not on the list that says so",
                building.kind
            );
        }
    }

    // Every space in the served layout becomes exactly one floor, under
    // its own label and kind, and it lies inside a building's footprint;
    // every thing stands on the estate patch (Phase 5 Task 3 Step 3).
    let layout = island
        .estate()
        .expect("the island serves its estate layout");
    let floors = island_ui::estate::rooms(layout);
    assert_eq!(floors.len(), layout.spaces.len(), "one floor per space");
    assert!(!layout.spaces.is_empty());
    // Checked on what is drawn, in the estate frame, not on the layout's
    // own rectangles: a mistake in turning the layout into the frame (the
    // wrong origin, a flipped axis) is exactly what this has to catch.
    let drawn_buildings = island_ui::estate::buildings(layout);
    // Is the point (x, z) of the estate frame on this drawn box's
    // footprint, the box turned about the vertical as it is drawn?
    let on = |b: &island_ui::estate::Placed, x: f32, z: f32| {
        let (sin, cos) = b.rotation_deg.to_radians().sin_cos();
        let (dx, dz) = (x - b.at.x, z - b.at.z);
        // Back into the box's own frame (the inverse of Bevy's turn).
        let (lx, lz) = (dx * cos - dz * sin, dx * sin + dz * cos);
        lx.abs() <= b.size_m.0 / 2.0 + 1e-3 && lz.abs() <= b.size_m.2 / 2.0 + 1e-3
    };
    // The floor's four corners as drawn, turned with it: its footprint, not
    // only its centre, so a width and depth swapped about a centre that
    // stays put is caught too.
    let corners = |f: &island_ui::estate::Placed| {
        let (sin, cos) = f.rotation_deg.to_radians().sin_cos();
        let (hx, hz) = (f.size_m.0 / 2.0, f.size_m.2 / 2.0);
        [(-hx, -hz), (-hx, hz), (hx, -hz), (hx, hz)]
            .map(|(lx, lz)| (f.at.x + lx * cos + lz * sin, f.at.z - lx * sin + lz * cos))
    };
    for (space, floor) in layout.spaces.iter().zip(&floors) {
        assert_eq!((&floor.name, &floor.kind), (&space.label, &space.kind));
        assert!(
            drawn_buildings
                .iter()
                .any(|b| corners(floor).iter().all(|&(x, z)| on(b, x, z))),
            "the floor of {} ({}) is drawn at {:?}, {:?} across, not wholly in any drawn building",
            space.label,
            space.kind,
            floor.at,
            floor.size_m
        );
    }
    // And the wood around the yard arrives stem by stem, each standing
    // in the box it was asked for (Phase 5 Task 3 Step 2a).
    let (x0, y0, x1, y1) = island_ui::stems::neighbourhood(layout);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let trees = loop {
        if let Some(trees) = island.snapshot().trees.as_ref().clone() {
            break trees;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the stems never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(trees.shown <= island_ui::stems::STEM_CAP);
    for tree in &trees.trees {
        assert!(
            tree.x_m >= x0 && tree.x_m <= x1 && tree.y_m >= y0 && tree.y_m <= y1,
            "stem {} is outside the box it was asked for",
            tree.id
        );
    }
    let drawn = island_ui::stems::stems(&trees, (layout.patch.x0, layout.patch.y0));
    assert!(drawn.len() <= trees.trees.len());
    let things = island_ui::estate::things(layout);
    assert_eq!(things.len(), layout.items.len(), "one drawn thing per item");
    // The patch as drawn: its south-west corner is the frame's origin and
    // north is -z.
    let origin = (layout.patch.x0, layout.patch.y0);
    let far = island_ui::scene::estate_of(origin, layout.patch.x1, layout.patch.y1, 0.0);
    for thing in &things {
        assert!(
            (0.0..=far.x).contains(&thing.at.x) && (far.z..=0.0).contains(&thing.at.z),
            "{} is drawn at {:?}, off the patch drawn from (0, 0) to ({}, {})",
            thing.name,
            thing.at,
            far.x,
            far.z
        );
    }

    // And a founder, read the way the person inspector reads them (Phase 5
    // Task 4): every section the backend builds, what they are doing and
    // thinking, their family, and where they are on the running island.
    let reply = island
        .client()
        .human("Gem-D")
        .expect("the backend serves a founder");
    let gem_d = island_ui::human_inspector::read(&reply).expect("the inspector reads them");
    let ids: Vec<&str> = gem_d.sections.iter().map(|s| s.id.as_str()).collect();
    for wanted in [
        "identity", "vitals", "needs", "feeling", "choosing", "family",
    ] {
        assert!(ids.contains(&wanted), "no {wanted} section in {ids:?}");
    }
    let family = gem_d.sections.iter().find(|s| s.id == "family").unwrap();
    assert!(
        family.readings.iter().any(|r| r.label == "Father"),
        "{family:?}"
    );
    assert!(
        gem_d.whereabouts.is_some(),
        "a running island knows where a founder is"
    );
}

#[test]
fn an_island_that_is_not_there_is_reported_rather_than_waited_on() {
    // A desktop application pointed at the wrong address must say so at
    // once. Nothing is listening on port 1 and nothing will be.
    // `Island` has no `Debug` -- it holds a live connection and a thread
    // handle, and a `{:?}` of it would be noise -- so this matches rather
    // than using `expect_err`.
    let Err(err) = Island::remote("http://127.0.0.1:1", None) else {
        panic!("something answered on port 1");
    };
    assert!(err.worth_retrying(), "{err}");
    assert!(
        err.to_string().contains("Could not reach the island"),
        "{err}"
    );
}

#[test]
fn closing_the_application_stops_the_island_it_was_running() {
    // The embedded backend and its simulation thread have to go when the
    // window does, or quitting leaves an island stepping in a process
    // nobody can see.
    let (_data, backend) = a_small_island();
    let address = backend.address.clone();
    {
        let island = Island::embedded(backend).expect("connects");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !island.snapshot().ever_read {
            assert!(std::time::Instant::now() < deadline, "never read");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    // Dropped. The port should stop answering.
    let gone = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let still_there = mk_island_client::IslandClient::new(&address, None)
            .version()
            .is_ok();
        if !still_there {
            break;
        }
        assert!(
            std::time::Instant::now() < gone,
            "{address} is still serving after the application let go of it"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
