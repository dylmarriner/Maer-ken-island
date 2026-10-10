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
