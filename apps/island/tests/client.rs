//! The typed client against the real backend.
//!
//! `mk_island_client` is what a desktop application on another machine
//! uses to read this island, and it could be tested against a mock. It is
//! not, because a mock would only prove that this client agrees with what
//! I believe the server sends. Every bug worth catching here is a
//! disagreement between the two -- a field named `entries` where the
//! client asks for `timeline`, an economy whose fields are at the top
//! level rather than nested -- and only the real server can find those.
//! Two of them were in the client the first time this ran.

mod common;
use common::{a_remote_backend, a_running_dashboard};

use island::serve::auth::{ControlAuth, ReadAuth};
use island::serve::server::ServeConfig;
use mk_island_api::{ControlRequest, CreateHumanRequest, InterventionRequest, InterventionTarget};
use mk_island_client::{ClientError, IslandClient};

fn at(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// Wait for a command to stop being `Queued`, or give up saying so.
///
/// The island applies commands between steps, and this fixture is paced
/// rather than flat out, so an answer takes a moment. Polling with a
/// deadline rather than sleeping a fixed time: on a loaded four-core
/// container a fixed sleep is either too short to be reliable or long
/// enough to make the suite crawl.
fn settled(island: &IslandClient, command: u64) -> mk_island_api::Outcome {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        match island.outcome(command) {
            Ok(mk_island_api::Outcome::Queued) | Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Ok(outcome) => return outcome,
            Err(err) => panic!("command {command} never settled: {err}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_reads_the_whole_island_through_the_shared_schema() {
    let (world, port, server) = a_running_dashboard().await;
    let base = at(port);

    // Everything here runs on a blocking thread, because the client is
    // blocking and this is a tokio test: calling it on the runtime's own
    // thread would deadlock the server it is talking to.
    let read = tokio::task::spawn_blocking(move || {
        let (island, version) =
            IslandClient::connect(&base, None).expect("the backend negotiates a version");
        assert!(version.capabilities.world);

        let world = island.world().expect("the world reads");
        assert!(world.running || world.clock.tick == 0);
        assert!(world.land.cols * world.land.rows > 0);
        // The founders are `Gem-D` and `Gem-K` on the island itself. The
        // `HUM-000001` ids belong to the *stored* Phase-0b population,
        // which is a different set of people in a different place; this
        // test asserted the wrong one first.
        let ids: Vec<&str> = world.people.iter().map(|p| p.agent_id.as_str()).collect();
        assert!(ids.contains(&"Gem-D") && ids.contains(&"Gem-K"), "{ids:?}");
        assert!(
            !world.estate.spaces.is_empty(),
            "the estate's rooms are what a creator offers"
        );

        // Each of these is a differently-shaped answer, and each was a
        // chance for the client to ask for the wrong field.
        let properties = island.properties().expect("properties read");
        assert!(
            properties.iter().any(|p| p.on_this_island),
            "one of the properties is this island's estate"
        );
        assert!(
            properties
                .iter()
                .flat_map(|p| &p.items)
                .any(|item| item.kind == "Computer"),
            "the computer room's machines are property items"
        );

        let economy = island.economy().expect("economy reads");
        assert_eq!(
            economy.resource_nodes, 0,
            "the island seeds none, and says so rather than hiding the count"
        );

        let timeline = island.timeline().expect("timeline reads");
        assert!(timeline.is_empty(), "nothing has reached this island yet");

        let _conversations = island.conversations().expect("conversations read");

        let terrain = island.terrain().expect("terrain reads");
        assert_eq!(terrain.rows, world.land.rows);
        assert_eq!(terrain.cols, world.land.cols);
        assert_eq!(f64::from(terrain.cell_size_m), world.land.cell_size_m);
        assert_eq!(
            terrain.land.iter().filter(|l| **l).count(),
            world.land.land_cells
        );
        let (lowest, highest) = terrain.relief_m().expect("the island has relief");
        assert!(lowest < 0.0 && highest > 0.0, "{lowest} to {highest}");

        // The grids and the per-cell endpoint have to agree, because a
        // renderer reads one and an inspector reads the other.
        let (row, col) = (terrain.rows / 3, terrain.cols / 3);
        let cell = island.cell(row, col).expect("a cell reads");
        assert_eq!(cell.land, terrain.is_land(row, col).unwrap());
        assert!(
            (cell.elevation_m - f64::from(terrain.elevation_at(row, col).unwrap())).abs() < 0.5
        );

        let (cx, cy) = (world.estate.cell.1, world.estate.cell.0);
        let _ = (cx, cy);
        let trees = island
            .trees_in(f64::MIN, f64::MIN, f64::MAX, f64::MAX, 100)
            .expect("trees read");
        assert!(trees.shown <= 100, "the cap was not honoured");
        assert!(trees.total >= trees.in_box);

        let png = island.map_png().expect("the map reads");
        assert_eq!(&png[1..4], b"PNG", "that is not a PNG");

        let roster = island.roster().expect("the roster reads");
        assert_eq!(
            roster
                .iter()
                .map(|p| p.agent_id.as_str())
                .collect::<Vec<_>>(),
            ids,
            "the roster and the world must be the same people"
        );
        assert!(
            roster.iter().all(|p| p.space.is_some()),
            "both founders start in their own bedrooms"
        );
    })
    .await;

    world.stop();
    server.abort();
    read.expect("the reads ran");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_changes_the_island_and_is_told_what_happened() {
    let (world, port, server) = a_running_dashboard().await;
    let base = at(port);

    let wrote = tokio::task::spawn_blocking(move || {
        let (island, _) = IslandClient::connect(&base, None).expect("connects");
        let before = island.world().expect("reads");

        // Create somebody in a room of the estate, through exactly the
        // path the dashboard's own creator uses.
        let room = before.estate.spaces[0].id;
        let accepted = island
            .create_human(&CreateHumanRequest::in_space(
                "Hine Moana",
                "female",
                "1992-11-03T10:15:00+13:00",
                33.9,
                room,
            ))
            .expect("the creation is accepted");
        assert!(accepted.poll.contains(&accepted.command.to_string()));
        match settled(&island, accepted.command) {
            mk_island_api::Outcome::Created { agent_id, .. } => {
                let record = island.human(&agent_id).expect("the new person reads back");
                assert!(record.to_string().contains(&agent_id));
            }
            other => panic!("the island did not create them: {other:?}"),
        }

        // Pause it, and see the clock stop.
        let accepted = island
            .control(&ControlRequest::Pause)
            .expect("pause is accepted");
        assert!(matches!(
            settled(&island, accepted.command),
            mk_island_api::Outcome::Noted
        ));
        let held = island.world().expect("reads").clock.tick;
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert_eq!(
            island.world().expect("reads").clock.tick,
            held,
            "a paused island kept stepping"
        );
        island
            .control(&ControlRequest::Resume)
            .expect("resume is accepted");

        // An intervention the island applies, and one it refuses by name.
        let here = InterventionTarget::new(before.estate.latitude, before.estate.longitude);
        let accepted = island
            .intervene(&InterventionRequest::inject_water(100.0, here))
            .expect("the intervention is accepted");
        match settled(&island, accepted.command) {
            mk_island_api::Outcome::Intervened { summary, .. } => {
                assert!(summary.contains("water"), "{summary}");
            }
            other => panic!("water should have gone in: {other:?}"),
        }

        let accepted = island
            .intervene(&InterventionRequest::sculpt_terrain(
                10.0,
                here.region(25.0),
            ))
            .expect("even a refusal is accepted for application");
        match settled(&island, accepted.command) {
            mk_island_api::Outcome::Refused { problems } => {
                assert!(
                    !problems.is_empty(),
                    "a refusal with no reason is the thing the island promises not to do"
                );
            }
            other => panic!("terrain is canon and must be refused: {other:?}"),
        }

        // And the timeline now carries all of it, in order.
        let timeline = island.timeline().expect("timeline reads");
        assert!(
            timeline.len() >= 4,
            "the replay log should hold every one of those: {timeline:?}"
        );
    })
    .await;

    world.stop();
    server.abort();
    wrote.expect("the writes ran");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_without_the_token_is_told_which_one_it_needs() {
    let config = ServeConfig {
        control: ControlAuth::BearerToken("control".to_string()),
        reads: ReadAuth::BearerToken("reading".to_string()),
        ..ServeConfig::local(ControlAuth::Disabled)
    };
    let (world, port, server) = a_remote_backend(config).await;
    let base = at(port);

    let checked = tokio::task::spawn_blocking(move || {
        // Version negotiation works with no token at all, which is the
        // whole reason it is outside the gate: a frontend has to be able
        // to find out it needs one.
        let (anonymous, version) = IslandClient::connect(&base, None).expect("version is open");
        assert!(version.capabilities.reads_need_token);

        let err = anonymous.world().expect_err("reads are locked");
        assert!(err.needs_a_token(), "{err}");
        assert!(!err.worth_retrying(), "retrying will not find a token");
        assert!(
            err.to_string().contains("token"),
            "the sentence has to say what to do: {err}"
        );

        // The read token opens reading and not writing.
        let reader = anonymous.with_token(Some("reading".to_string()));
        let world = reader.world().expect("reads with the read token");
        let room = world.estate.spaces[0].id;
        let err = reader
            .create_human(&CreateHumanRequest::in_space(
                "Nobody",
                "female",
                "1992-11-03T10:15:00+13:00",
                30.0,
                room,
            ))
            .expect_err("a read token is not a licence to write");
        assert!(err.needs_a_token(), "{err}");
        assert!(
            err.to_string().contains("control token"),
            "it must say which token: {err}"
        );

        // The control token does both.
        let operator = anonymous.with_token(Some("control".to_string()));
        operator.world().expect("the control token reads too");
        operator
            .control(&ControlRequest::Pause)
            .expect("and writes");
    })
    .await;

    world.stop();
    server.abort();
    checked.expect("the checks ran");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_backend_that_is_not_there_is_not_reported_as_a_refusal() {
    // The distinction a frontend has to draw before it can say anything
    // useful: nobody answered, as against somebody answered and said no.
    let unreachable = tokio::task::spawn_blocking(|| {
        // Port 1 on loopback: nothing is listening and nothing will be.
        let err = IslandClient::connect("http://127.0.0.1:1", None)
            .expect_err("nothing is listening there");
        assert!(matches!(err, ClientError::Unreachable { .. }), "{err:?}");
        assert!(err.worth_retrying(), "a dead backend may come back");
        assert!(!err.needs_a_token());
        assert!(
            err.to_string().contains("Could not reach the island"),
            "{err}"
        );
    })
    .await;
    unreachable.expect("the check ran");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_backend_with_no_island_says_so_rather_than_failing() {
    // `island serve` without `--scenario` is a real configuration: a
    // stored population and no world. A client should say that in those
    // words rather than showing an error.
    let data_dir = tempfile::tempdir().unwrap();
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [13u8; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = island::serve::server::routes_with_config(
        std::sync::Arc::new(std::sync::Mutex::new(population)),
        None,
        ServeConfig::local(ControlAuth::LoopbackOnly),
    );
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });
    let base = at(port);

    let checked = tokio::task::spawn_blocking(move || {
        let (island, version) = IslandClient::connect(&base, None).expect("connects");
        assert!(!version.capabilities.world, "there is no island here");

        for err in [
            island.world().expect_err("no world"),
            island.properties().expect_err("no properties"),
            island.terrain().expect_err("no terrain"),
        ] {
            assert!(matches!(err, ClientError::NoIsland { .. }), "{err:?}");
            assert!(
                err.to_string().contains("--scenario"),
                "it should say how to get one: {err}"
            );
        }
    })
    .await;

    server.abort();
    checked.expect("the check ran");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_estate_is_served_as_geometry_a_renderer_can_place() {
    // The desktop application draws the founders' estate: the house, the
    // workshop, the computer room, and the four machines inside it. None
    // of that is drawable from `/api/properties`, which says what exists
    // and not where it is.
    let (world, port, server) = a_running_dashboard().await;
    let base = at(port);

    let read = tokio::task::spawn_blocking(move || {
        let (island, _) = IslandClient::connect(&base, None).expect("connects");
        let layout = island.estate_layout().expect("the layout reads");

        // Buildings, with real extents rather than points.
        assert!(
            layout.buildings.len() >= 5,
            "the estate has a house, a shed, a workshop, an armoury and a computer room: {:?}",
            layout.buildings.iter().map(|b| &b.kind).collect::<Vec<_>>()
        );
        for building in &layout.buildings {
            assert!(
                building.rect_m.width_m() > 1.0 && building.rect_m.depth_m() > 1.0,
                "{} is {} x {} m",
                building.name,
                building.rect_m.width_m(),
                building.rect_m.depth_m()
            );
            let model = island_ui_model(&building.kind);
            assert!(!model.is_empty(), "{} has no model path", building.kind);
        }
        // The computer room is a *building* to the engine and a *room
        // inside the house* on the ground: `house_plan` is given the
        // computer building's id and lays its space out within the
        // house's footprint. So it has no footprint of its own, and a
        // renderer that looked for one would find nothing and draw
        // nothing. `assets/CREDITS.md` and `island_ui`'s model table both
        // already said this; the first version of this test did not
        // believe them.
        assert!(
            !layout.buildings.iter().any(|b| b.kind == "ComputerRoom"),
            "the computer room should be a room, not a footprint: {:?}",
            layout.buildings.iter().map(|b| &b.kind).collect::<Vec<_>>()
        );
        let house = layout
            .buildings
            .iter()
            .find(|b| b.kind == "House")
            .expect("the homestead");
        let room = layout
            .spaces
            .iter()
            .find(|s| s.label == "Computer Room")
            .expect("the computer room is a space");
        let (rx, ry) = room.rect_m.centre_m();
        assert!(
            rx >= house.rect_m.x0.min(house.rect_m.x1) - 1.0
                && rx <= house.rect_m.x0.max(house.rect_m.x1) + 1.0
                && ry >= house.rect_m.y0.min(house.rect_m.y1) - 1.0
                && ry <= house.rect_m.y0.max(house.rect_m.y1) + 1.0,
            "the computer room is at {rx}, {ry}, which is not inside the house ({:?})",
            house.rect_m
        );

        // The rooms inside them.
        assert!(
            layout.spaces.iter().any(|s| s.label.contains("Computer")),
            "the computer room is a space too: {:?}",
            layout.spaces.iter().map(|s| &s.label).collect::<Vec<_>>()
        );
        assert!(
            layout.spaces.iter().any(|s| s.label.contains("Bedroom")),
            "the founders start in their own bedrooms, so those are rooms"
        );

        // And the things in them, with the join from placement to name
        // already done.
        let computers: Vec<&mk_island_api::PlacedItem> = layout
            .items
            .iter()
            .filter(|item| item.kind == "Computer")
            .collect();
        assert_eq!(
            computers.len(),
            4,
            "four machines, each at its own metres: {computers:?}"
        );
        for machine in &computers {
            assert_eq!(
                machine.space.as_deref(),
                Some("Computer Room"),
                "{} is not in the computer room",
                machine.name
            );
            // Inside the room it belongs to, which is the whole point of
            // serving metres rather than a room name.
            let (x, y) = machine.position_m;
            assert!(
                x >= room.rect_m.x0.min(room.rect_m.x1) - 1.0
                    && x <= room.rect_m.x0.max(room.rect_m.x1) + 1.0
                    && y >= room.rect_m.y0.min(room.rect_m.y1) - 1.0
                    && y <= room.rect_m.y0.max(room.rect_m.y1) + 1.0,
                "{} stands at {x}, {y}, outside the room it is in ({:?})",
                machine.name,
                room.rect_m
            );
        }
        // `items_in` finds everything in that room, which is more than
        // the machines: the first version of this expected four and got
        // six. What the other two are is printed rather than guessed at.
        let in_the_room: Vec<&mk_island_api::PlacedItem> =
            layout.items_in("Computer Room").collect();
        println!(
            "the computer room holds: {:?}",
            in_the_room
                .iter()
                .map(|i| format!("{} ({})", i.name, i.kind))
                .collect::<Vec<_>>()
        );
        assert!(
            in_the_room.len() >= computers.len(),
            "every machine is in the room: {} against {}",
            in_the_room.len(),
            computers.len()
        );
        assert_eq!(
            in_the_room.iter().filter(|i| i.kind == "Computer").count(),
            4,
            "and four of them are the machines"
        );

        // The yard, and the ground everything stands on.
        assert!(layout.yard.width_m() > 0.0 && layout.yard.depth_m() > 0.0);
        let extent = layout.extent_m().expect("the estate covers some ground");
        assert!(extent.width_m() > 0.0 && extent.depth_m() > 0.0);
        assert!(
            extent.x0 >= layout.patch.x0 - 1.0 && extent.x1 <= layout.patch.x1 + 1.0,
            "the estate should be inside its own patch: {extent:?} against {:?}",
            layout.patch
        );
    })
    .await;

    world.stop();
    server.abort();
    read.expect("the reads ran");
}

/// The model `island_ui` would draw a building kind with.
///
/// Spelled out here rather than depending on `island_ui` from this crate:
/// the backend has no business linking a renderer, and what this test
/// needs is only that the kinds the island serves are kinds that table
/// knows. `island_ui`'s own tests check the files exist.
fn island_ui_model(kind: &str) -> &'static str {
    match kind {
        "House" => "property/buildings/homestead_house.glb",
        "Shed" => "property/buildings/equipment_shed.glb",
        "Workshop" => "property/buildings/building_workshop.glb",
        "Armoury" => "property/buildings/secure_armoury.glb",
        "ComputerRoom" => "property/buildings/computer_room.glb",
        "Garage" => "property/placeholder.glb",
        _ => "",
    }
}
