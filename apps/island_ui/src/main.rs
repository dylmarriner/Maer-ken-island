//! `island-ui` — Maer-Ken Island as a desktop application.
//!
//! It reads an island and draws it. The island can be on this machine or
//! on another one, and the application does not care which: `--server`
//! points it at a backend somewhere else, `--scenario` starts one here,
//! and both are read through the same client over the same HTTP. That is
//! the whole architecture, and it is in `source.rs` rather than here.
//!
//! This file is the window. Everything it can do without a GPU lives in
//! the library beside it and is tested there; what is left is the part
//! that turns tested values into Bevy entities. `docs/island/RENDER_STACK.md`
//! says plainly which half is which, and which half has never been run on
//! the machine this was written on, because that machine has no graphics
//! hardware at all.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_egui::egui::{self, LayerId, Ui, UiBuilder};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use island_ui::camera::Orbit;
use island_ui::estate;
use island_ui::scene::{self, Point};
use island_ui::source::{Island, LocalBackend, Snapshot};
use island_ui::{property, terrain};

const USAGE: &str = "\
island-ui --server <url> [--token <token>]
island-ui (--scenario <path> | --snapshot <path>) [--data-dir <dir>] [--speed real|max|<n>]

  --server URL     read an island served by `island serve` somewhere else.
                   This is the ordinary way to run it: the simulation is a
                   backend on its own machine and this is one of several
                   screens looking at it.
  --token TOKEN    the token that backend wants, if it wants one. A control
                   token reads and writes; a read token only reads. Which
                   of the two this is cannot be known before it is used --
                   `/api/version` says a token is wanted, not what the one
                   you hold is for -- so the controls stay live and a read
                   token's refusal is shown under them in the island's own
                   words.
  --scenario PATH  run an island here instead, in this process. It is the
                   same backend, on a loopback port nothing else can reach,
                   read through the same client -- so there is one way in
                   and the local case is proven by the remote one.
  --snapshot PATH  carry on a saved island, here, the same way.
  --data-dir DIR   where a locally-run island keeps its people
                   (default ./island-data)
  --assets DIR     where the models live (default: ISLAND_ASSETS, then
                   ./assets, then an assets/ beside this binary or one
                   directory above it). Without them the estate still
                   draws -- every building, room and thing is a box at the
                   simulation's own metres -- but it draws without the
                   models.
  --speed SPEED    real (the default), max, or a multiplier like 60. Only
                   for an island run here; a remote one is paced by
                   whoever started it, and the controls can change it.
  --view WHICH     island (the default) or estate: which of the two frames
                   to open on. The panel switches between them at any
                   time; this is for opening straight onto the estate, and
                   for a headless render that has nobody to press it.

The island itself is never in this process unless you asked for it with
--scenario or --snapshot. Nothing drawn here is ever hashed, and the
simulation does not know this application exists.";

fn help() -> ! {
    println!("{USAGE}");
    std::process::exit(0);
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}

/// Where the models are.
///
/// Searched rather than assumed, because the three places this runs from
/// are different: a developer's checkout (`./assets`), a release tree
/// built by `deploy/build-release.sh` (`assets/` beside the binary), and
/// a `cargo run` from the workspace root (also `./assets`).
///
/// Never fatal. A missing asset root means the estate draws as boxes at
/// the simulation's own metres, which is most of what it is: the models
/// are decoration hung on that geometry.
fn asset_root(asked: Option<String>) -> std::path::PathBuf {
    if let Some(asked) = asked {
        return std::path::PathBuf::from(asked);
    }
    if let Some(from_env) = std::env::var_os("ISLAND_ASSETS") {
        return std::path::PathBuf::from(from_env);
    }
    let beside_the_binary = std::env::current_exe().ok().and_then(|exe| {
        let dir = exe.parent()?.to_path_buf();
        [dir.join("assets"), dir.join("../assets")]
            .into_iter()
            .find(|candidate| candidate.is_dir())
    });
    let here = std::path::PathBuf::from("assets");
    let found = if here.is_dir() {
        here
    } else {
        beside_the_binary.unwrap_or(here)
    };
    // Absolute, always. Bevy resolves a *relative* asset path against the
    // binary's own directory, not the working directory -- so `assets`
    // from a `cargo run` in the workspace root became
    // `target/debug/assets`, and every model failed to load with "Path
    // not found" while the directory sat where it always was. The estate
    // still drew, because its geometry is the simulation's and the models
    // are hung on it, which is how that went unnoticed for one frame.
    found
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(found))
}

/// Whether there is a desktop to put a window on.
///
/// The two variables winit itself looks at. Checking them is not a
/// guarantee that a window will open -- a broken X server sets `DISPLAY`
/// too -- but it catches the ordinary headless case, which is the one
/// somebody hits by accident.
fn has_a_display() -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"]
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

/// The island, kept where Bevy's systems can reach it.
#[derive(Resource)]
struct TheIsland(Island);

/// The last snapshot, copied out of the poller once a frame so every
/// system in that frame sees the same island.
#[derive(Resource, Default)]
struct Latest(Snapshot);

/// The camera's state, from the tested arithmetic in `camera.rs`.
#[derive(Resource, Default)]
struct View(Orbit);

/// Where the estate patch's corner is, in domain metres: the floating
/// origin of the estate frame.
#[derive(Resource, Default)]
struct EstateOrigin((f64, f64));

/// What the panels are showing and what somebody has typed into them.
#[derive(Resource, Default)]
struct Panels {
    selected: Option<String>,
    said: Option<String>,
    show_conversations: bool,
    show_estate: bool,
}

/// Marks an entity that stands for an islander, so it can be moved rather
/// than respawned every frame.
#[derive(Component)]
struct Islander(String);

/// Marks the ground, so the estate view can hide it.
#[derive(Component)]
struct Ground;

/// Marks everything on the estate, drawn in metres rather than in ten-
/// kilometre units, and hidden unless the estate is what is being looked
/// at.
#[derive(Component)]
struct OnTheEstate;

/// Which of the two frames is being shown.
///
/// Two views rather than one, because one scale cannot hold an island
/// 2,400 km across and a four-metre room: see `scene.rs`. Switching hides
/// one set of entities and shows the other, and moves the camera, rather
/// than rescaling anything -- a rescaled estate would still be measured
/// in six-figure metres and still lose millimetres to `f32`.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
enum Showing {
    #[default]
    Island,
    Estate,
}

impl Showing {
    /// What `--view` was asked for, or the island when it was not asked.
    ///
    /// An unknown word is refused rather than quietly defaulting: a
    /// person who typed `--view estates` wants to be told, not to be
    /// shown the island and left wondering why the flag did nothing.
    fn asked_for(flag: Option<String>) -> Self {
        match flag.as_deref() {
            None | Some("island") => Showing::Island,
            Some("estate") => Showing::Estate,
            Some(other) => {
                eprintln!("--view takes `island` or `estate`, not `{other}`.");
                usage()
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--help" | "-h" | "help"))
    {
        help();
    }

    let island = match (
        flag(&args, "--server"),
        flag(&args, "--scenario"),
        flag(&args, "--snapshot"),
    ) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            eprintln!(
                "--server reads an island somewhere else; --scenario runs one here. Pick one.\n"
            );
            usage()
        }
        (Some(address), None, None) => {
            let token = flag(&args, "--token").or_else(|| std::env::var("ISLAND_TOKEN").ok());
            println!("Reading the island at {address}.");
            Island::remote(&address, token)
        }
        (None, None, None) => {
            eprintln!(
                "island-ui needs an island: --server to read one, or --scenario to run one.\n"
            );
            usage()
        }
        (None, scenario, snapshot) => {
            let data_dir = flag(&args, "--data-dir").unwrap_or_else(|| "island-data".to_string());
            let speed = match flag(&args, "--speed") {
                None => island::serve::sim::SimSpeed::RealTime,
                Some(text) => text.parse().unwrap_or_else(|e: String| {
                    eprintln!("{e}\n");
                    usage()
                }),
            };
            println!("Starting an island here. This takes a moment on the full one.");
            let local = LocalBackend::start(
                scenario.as_deref().map(std::path::Path::new),
                snapshot.as_deref().map(std::path::Path::new),
                std::path::Path::new(&data_dir),
                speed,
            )
            .unwrap_or_else(|err| {
                eprintln!("{err}");
                std::process::exit(1);
            });
            println!(
                "It is at {}, on loopback, for this process alone.",
                local.address
            );
            Island::embedded(local)
        }
    };

    let island = island.unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });

    if !island.capabilities().world {
        eprintln!(
            "That backend is not simulating an island -- it was started without --scenario, so it \
             has a stored population and nothing to draw. Start it with a scenario, or point this \
             at one that has been."
        );
        std::process::exit(1);
    }

    // Checked before Bevy is assembled, because without it winit panics
    // with a sixteen-frame backtrace about an event loop -- which is a
    // true statement and a useless one for somebody who has just run this
    // over SSH. Measured: that is exactly what happened on the machine
    // this was written on.
    //
    // After the island is read rather than before, deliberately: finding
    // out that the backend is reachable and has a world is useful even on
    // a machine that cannot draw it, and a headless operator checking a
    // connection gets a real answer before the refusal.
    if !has_a_display() {
        eprintln!(
            "island-ui draws a window and this machine has no display: neither DISPLAY nor \
             WAYLAND_DISPLAY is set. The island at {} is reachable and has a world -- it is \
             only the drawing that cannot happen here. Run this where there is a desktop, or \
             read the same island in a browser: `island serve --frontend-only --backend {}`.",
            island.address(),
            island.address()
        );
        std::process::exit(1);
    }

    let terrain = island.terrain().expect("an island has terrain").clone();
    let (width_m, height_m) = terrain.extent_m();
    println!(
        "{} x {} cells of {:.0} m: {:.0} x {:.0} km of island.",
        terrain.rows,
        terrain.cols,
        terrain.cell_size_m,
        width_m / 1000.0,
        height_m / 1000.0
    );

    let assets = asset_root(flag(&args, "--assets"));
    println!("Models from {}.", assets.display());

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("Maer-Ken Island — {}", island.address()),
                        // Wider than Bevy's 1280x720 default, because two side
                        // panels and a map between them is what this draws, and
                        // at 1280 the map is the narrowest of the three.
                        resolution: bevy::window::WindowResolution::new(1600, 1000),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .insert_resource(TheIsland(island))
        .insert_resource(Latest::default())
        .insert_resource(View::default())
        .insert_resource(EstateOrigin::default())
        .insert_resource(Panels::default())
        .insert_resource(Showing::asked_for(flag(&args, "--view")))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.04, 0.08)))
        // Spawning a glTF scene goes through `bevy_world_serialization`,
        // which looks every component up in the type registry and panics
        // on one it does not find. With Bevy's default features that
        // registration happens inside plugins this build leaves out, so
        // the types the models actually carry are registered here. The
        // alternative is turning features back on for the registry
        // alone, which would drag alsa and libudev back with them.
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<Visibility>()
        .register_type::<InheritedVisibility>()
        .register_type::<ViewVisibility>()
        .register_type::<Name>()
        .register_type::<bevy::camera::primitives::Aabb>()
        .register_type::<Mesh3d>()
        .register_type::<Children>()
        .register_type::<ChildOf>()
        .register_type::<TransformTreeChanged>()
        // The glTF loader hangs these on the entities it makes, so a
        // model carries them whether or not this application reads them.
        .register_type::<bevy::gltf::GltfExtras>()
        .register_type::<bevy::gltf::GltfSceneExtras>()
        .register_type::<bevy::gltf::GltfSceneName>()
        .register_type::<bevy::gltf::GltfMeshExtras>()
        .register_type::<bevy::gltf::GltfMeshName>()
        .register_type::<bevy::gltf::GltfMaterialExtras>()
        .register_type::<bevy::gltf::GltfMaterialName>()
        .add_systems(Startup, (build_ground, build_the_estate, light_the_island))
        .add_systems(
            Update,
            (
                read_the_island,
                drive_the_camera,
                place_islanders,
                show_what_was_chosen,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, panels)
        .run();
}

/// Build the ground once. Terrain cannot change -- the island refuses
/// every terrain edit -- so this runs at startup and never again.
fn build_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    island: Res<TheIsland>,
) {
    let Some(ground) = island.0.terrain() else {
        return;
    };
    let land = materials.add(StandardMaterial {
        base_color: Color::srgb(0.33, 0.42, 0.26),
        perceptual_roughness: 0.95,
        ..default()
    });
    let sea = materials.add(StandardMaterial {
        base_color: Color::srgb(0.07, 0.17, 0.31),
        perceptual_roughness: 0.25,
        ..default()
    });

    let chunks = terrain::chunks(ground);
    let count = chunks.len();
    for chunk in chunks {
        // A chunk is drawn twice: once as land and once as sea, each with
        // only its own triangles. Splitting by vertex would need one
        // material per vertex, which is not a thing; splitting the
        // triangles is, and it means a coastline is a real edge rather
        // than a blend between two colours.
        for (is_land, material) in [(true, land.clone()), (false, sea.clone())] {
            let Some(mesh) = chunk_mesh(&chunk, is_land) else {
                continue;
            };
            commands.spawn((
                Ground,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::IDENTITY,
            ));
        }
    }
    info!("{count} chunks of ground");
}

/// One chunk's triangles, kept to the land ones or the sea ones.
///
/// `None` when none of them qualify, so an all-sea chunk does not spawn an
/// empty land mesh and vice versa. An empty mesh is not free: it is a draw
/// call and an entity, and on 285 chunks that is 285 of each for nothing.
fn chunk_mesh(chunk: &terrain::Chunk, want_land: bool) -> Option<Mesh> {
    let kept: Vec<u32> = chunk
        .indices
        .chunks_exact(3)
        .filter(|tri| {
            // A triangle belongs to the land if any of its corners do, so
            // the coastline is drawn rather than falling in the gap
            // between the two meshes.
            tri.iter()
                .any(|i| chunk.land.get(*i as usize).copied().unwrap_or(false))
                == want_land
        })
        .flatten()
        .copied()
        .collect();
    if kept.is_empty() {
        return None;
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        chunk
            .positions
            .iter()
            .map(|p| [p.x, p.y, p.z])
            .collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, chunk.normals.clone());
    mesh.insert_indices(Indices::U32(kept));
    Some(mesh)
}

/// The founders' estate, in metres, hidden until somebody asks for it.
///
/// Built once at startup: the buildings are placed when the island
/// bootstraps and nothing on this island moves one.
///
/// Every shape here is the simulation's own -- a building is its
/// footprint, a room is its rectangle, a thing is at the metres it stands
/// on. The models from `assets/` are hung on those shapes rather than
/// consulted for them: a model at the wrong scale makes an odd-looking
/// shed, and a footprint taken from a model would make the island
/// disagree with itself.
fn build_the_estate(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    island: Res<TheIsland>,
) {
    let Some(layout) = island.0.estate() else {
        warn!("this island served no estate layout, so there is nothing to draw of the estate");
        return;
    };

    let box_for = |size: (f32, f32, f32)| Cuboid::new(size.0, size.1, size.2).mesh().build();
    let colour = |r: f32, g: f32, b: f32, alpha: f32| StandardMaterial {
        base_color: Color::srgba(r, g, b, alpha),
        alpha_mode: if alpha < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        perceptual_roughness: 0.9,
        ..default()
    };

    // The cleared ground first, so everything else stands on something.
    let yard = estate::yard(layout);
    commands.spawn((
        OnTheEstate,
        Mesh3d(meshes.add(box_for(yard.size_m))),
        MeshMaterial3d(materials.add(colour(0.42, 0.44, 0.33, 1.0))),
        Transform::from_xyz(yard.at.x, yard.at.y, yard.at.z),
        Visibility::Hidden,
    ));

    // Buildings as their real footprints, translucent so the rooms and
    // the things inside them can be seen without a cutaway.
    for building in estate::buildings(layout) {
        commands.spawn((
            OnTheEstate,
            Mesh3d(meshes.add(box_for(building.size_m))),
            MeshMaterial3d(materials.add(colour(0.72, 0.68, 0.58, 0.35))),
            Transform::from_xyz(building.at.x, building.at.y, building.at.z)
                .with_rotation(Quat::from_rotation_y(building.rotation_deg.to_radians())),
            Visibility::Hidden,
        ));
    }

    for room in estate::rooms(layout) {
        // The computer room gets its own colour, because it is the room
        // this application exists to be able to show.
        let is_computers = room.name.contains("Computer");
        commands.spawn((
            OnTheEstate,
            Mesh3d(meshes.add(box_for(room.size_m))),
            MeshMaterial3d(materials.add(if is_computers {
                colour(0.30, 0.42, 0.55, 1.0)
            } else {
                colour(0.55, 0.50, 0.44, 1.0)
            })),
            Transform::from_xyz(room.at.x, room.at.y, room.at.z),
            Visibility::Hidden,
        ));
    }

    let mut things = 0usize;
    let mut modelled = 0usize;
    for thing in estate::things(layout) {
        let at = Transform::from_xyz(thing.at.x, thing.at.y, thing.at.z);
        // A box at the size such a thing is, always: the model may be at
        // any scale, and a thing that is only a model is a thing that
        // vanishes when the model does not load.
        commands.spawn((
            OnTheEstate,
            Mesh3d(meshes.add(box_for(thing.size_m))),
            MeshMaterial3d(materials.add(match thing.kind.as_str() {
                "Computer" => colour(0.85, 0.87, 0.92, 1.0),
                "Vehicle" => colour(0.55, 0.35, 0.25, 1.0),
                _ => colour(0.62, 0.58, 0.50, 1.0),
            })),
            at,
            Visibility::Hidden,
        ));
        things += 1;
        if let Some(model) = thing.model.filter(|m| m.is_really_this) {
            commands.spawn((
                OnTheEstate,
                WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(model.path))),
                at,
                Visibility::Hidden,
            ));
            modelled += 1;
        }
    }
    info!(
        "the estate: {} buildings, {} rooms, {things} things ({modelled} with models of their own)",
        layout.buildings.len(),
        layout.spaces.len()
    );
}

/// Everything drawn in one frame, asked for by the marker that says which.
///
/// Three of these in one signature, and Bevy insists each excludes the
/// other two: without the `Without`s the borrow checker cannot know the
/// three sets are disjoint and the system will not build. Named rather
/// than written out because clippy is right that the written-out form is
/// unreadable, and because naming it puts the rule -- one marker, the
/// other two excluded -- in one place instead of three.
type OnlyIn<'w, 's, Marker, NotA, NotB> =
    Query<'w, 's, &'static mut Visibility, (With<Marker>, Without<NotA>, Without<NotB>)>;

/// Show one frame and hide the other, and move the camera with it.
fn show_what_was_chosen(
    showing: Res<Showing>,
    island: Res<TheIsland>,
    mut view: ResMut<View>,
    mut ground: OnlyIn<Ground, OnTheEstate, Islander>,
    mut estate_parts: OnlyIn<OnTheEstate, Ground, Islander>,
    mut islanders: OnlyIn<Islander, Ground, OnTheEstate>,
) {
    if !showing.is_changed() {
        return;
    }
    let on_the_island = *showing == Showing::Island;
    let show = |visible: bool| {
        if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    for mut visibility in &mut ground {
        *visibility = show(on_the_island);
    }
    for mut visibility in &mut islanders {
        *visibility = show(on_the_island);
    }
    for mut visibility in &mut estate_parts {
        *visibility = show(!on_the_island);
    }

    match *showing {
        Showing::Island => view.0.look_at(Point::new(0.0, 0.0, 0.0), 320.0),
        Showing::Estate => {
            if let Some((focus, distance)) = island.0.estate().and_then(estate::framing) {
                view.0.look_at(focus, distance);
                // Lower than the island view: a yard is read from an
                // angle, not from overhead.
                view.0.pitch = 0.6;
            }
        }
    }
}

/// The sun, and enough ambient light that a shaded slope is still legible.
fn light_the_island(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(60.0, 120.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // `GlobalAmbientLight` rather than `AmbientLight`: in 0.19 the latter
    // is a per-camera component and only the former is a resource.
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.7, 0.8, 1.0),
        brightness: 220.0,
        ..default()
    });
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 200.0, 240.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Copy what the poller last read, once a frame.
fn read_the_island(
    island: Res<TheIsland>,
    mut latest: ResMut<Latest>,
    mut origin: ResMut<EstateOrigin>,
) {
    latest.0 = island.0.snapshot();
    if origin.0 == (0.0, 0.0) && latest.0.ever_read {
        let land = &latest.0.world.land;
        origin.0 = (
            latest.0.world.estate.cell.1 as f64 * land.cell_size_m,
            latest.0.world.estate.cell.0 as f64 * land.cell_size_m,
        );
    }
}

/// Mouse and keyboard into the camera, through the tested arithmetic.
fn drive_the_camera(
    mut view: ResMut<View>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut contexts: EguiContexts,
) {
    // A drag that started on a panel belongs to the panel. Without this
    // the island spins while somebody drags a slider.
    let over_ui = contexts
        .ctx_mut()
        .map(|ctx| ctx.egui_wants_pointer_input())
        .unwrap_or(false);

    let mut turn = Vec2::ZERO;
    for moved in motion.read() {
        turn += moved.delta;
    }
    let mut zoom = 0.0_f32;
    for scrolled in wheel.read() {
        zoom += scrolled.y;
    }

    if !over_ui {
        if buttons.pressed(MouseButton::Left) {
            view.0.turn(-turn.x * 0.005, turn.y * 0.005);
        }
        if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
            view.0.pan(-turn.x * 0.001, turn.y * 0.001);
        }
        if zoom != 0.0 {
            // Exponential, so one notch is the same proportion whether the
            // camera is over the island or over a bedroom.
            view.0.zoom(0.9_f32.powf(zoom));
        }
    }

    // Keys work whether or not the pointer is over a panel: they are how
    // somebody gets back to a known view after losing themselves.
    if keys.just_pressed(KeyCode::KeyI) {
        view.0.look_at(Point::new(0.0, 0.0, 0.0), 320.0);
    }

    if let Ok(mut transform) = camera.single_mut() {
        let eye = view.0.eye();
        let focus = view.0.focus;
        *transform = Transform::from_xyz(eye.x, eye.y, eye.z)
            .looking_at(Vec3::new(focus.x, focus.y, focus.z), Vec3::Y);
    }
}

/// Move the islanders to where the island says they are.
///
/// Moved rather than respawned: a person is the same person between
/// frames, and despawning and respawning them every frame would throw away
/// whatever the renderer had cached about them several times a second.
fn place_islanders(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    island: Res<TheIsland>,
    latest: Res<Latest>,
    origin: Res<EstateOrigin>,
    mut existing: Query<(Entity, &Islander, &mut Transform)>,
) {
    let Some(ground) = island.0.terrain() else {
        return;
    };
    let placed = scene::place_people(ground, &latest.0.world, origin.0);

    let mut seen: Vec<&str> = Vec::with_capacity(placed.len());
    for person in &placed {
        seen.push(&person.agent_id);
        let found = existing
            .iter_mut()
            .find(|(_, islander, _)| islander.0 == person.agent_id);
        let at = Vec3::new(person.regional.x, person.regional.y, person.regional.z);
        match found {
            Some((_, _, mut transform)) => transform.translation = at,
            None => {
                // A marker rather than a model, at this scale: one render
                // unit is ten kilometres, so a person at true size is a
                // fifth of a micrometre and would be invisible whatever
                // was drawn. The estate view draws them properly; this is
                // "somebody is here".
                let mesh = meshes.add(Sphere::new(0.6).mesh().build());
                let colour = if person.asleep {
                    Color::srgb(0.55, 0.45, 0.85)
                } else {
                    Color::srgb(1.0, 0.83, 0.3)
                };
                commands.spawn((
                    Islander(person.agent_id.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: colour,
                        emissive: LinearRgba::rgb(0.3, 0.25, 0.08),
                        ..default()
                    })),
                    Transform::from_translation(at),
                ));
            }
        }
    }

    // Anybody the island has stopped reporting -- removed, or dead and
    // cleared -- goes with them, rather than being left standing where
    // they last were.
    for (entity, islander, _) in existing.iter() {
        if !seen.contains(&islander.0.as_str()) {
            commands.entity(entity).despawn();
        }
    }
}

/// The panels: what the island is doing, and what can be done to it.
fn panels(
    mut contexts: EguiContexts,
    island: Res<TheIsland>,
    latest: Res<Latest>,
    mut panels: ResMut<Panels>,
    mut view: ResMut<View>,
    mut showing: ResMut<Showing>,
    origin: Res<EstateOrigin>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    // egui 0.36 folded `SidePanel` into one `Panel` that takes a `Ui`
    // rather than a `Context`, so the root `Ui` is built here. This is
    // exactly what `bevy_egui`'s own examples do.
    let ctx = ctx.clone();
    let mut viewport = Ui::new(
        ctx.clone(),
        "island-ui".into(),
        UiBuilder::new()
            .layer_id(LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let snapshot = &latest.0;
    let writes = island.0.capabilities().writes.clone();
    let may_write = writes != "disabled";

    egui::Panel::left("island")
        .default_size(320.0)
        .show(&mut viewport, |ui| {
            ui.heading("The island");
            ui.label(island.0.address().to_string());
            if island.0.is_local() {
                ui.label("Running in this process, on loopback.");
            }
            ui.separator();

            if let Some(trouble) = &snapshot.trouble {
                // Said rather than swallowed. An application that quietly
                // showed an island five minutes out of date would be worse
                // than one that said it had lost the connection.
                ui.colored_label(egui::Color32::from_rgb(220, 120, 120), trouble);
                ui.separator();
            }
            if !snapshot.ever_read {
                ui.label("Reading the island…");
                return;
            }

            let clock = &snapshot.world.clock;
            ui.label(format!("Tick {}", clock.tick));
            ui.label(format!(
                "Day {:.2}, hour {:.1} of {:.0}",
                clock.days, clock.hour_of_day, clock.day_length_hours
            ));
            match clock.achieved_speed {
                Some(achieved) => ui.label(format!(
                    "{} asked for, {achieved:.1}x achieved",
                    clock.requested_speed
                )),
                None => ui.label(format!("{} asked for", clock.requested_speed)),
            };
            ui.label(format!(
                "Digest {} at tick {}{}",
                snapshot.world.digest.value,
                snapshot.world.digest.at_tick,
                if snapshot.world.digest.current {
                    ""
                } else {
                    " (behind)"
                }
            ));

            ui.separator();
            ui.horizontal(|ui| {
                ui.add_enabled_ui(may_write, |ui| {
                    if ui.button("Pause").clicked() {
                        panels.said = Some(ask(&island.0, mk_island_api::ControlRequest::Pause));
                    }
                    if ui.button("Resume").clicked() {
                        panels.said = Some(ask(&island.0, mk_island_api::ControlRequest::Resume));
                    }
                    if ui.button("Step an hour").clicked() {
                        panels.said = Some(ask(
                            &island.0,
                            mk_island_api::ControlRequest::Step { ticks: 60 },
                        ));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(may_write, |ui| {
                    for speed in ["real", "60", "1440", "max"] {
                        if ui.button(speed).clicked() {
                            panels.said = Some(ask(
                                &island.0,
                                mk_island_api::ControlRequest::SetSpeed {
                                    speed: speed.to_string(),
                                },
                            ));
                        }
                    }
                });
            });
            if !may_write {
                ui.label("This backend takes no changes from here.");
            } else if writes == "token" && !island.0.client().has_token() {
                // Greyed-out buttons would be wrong -- the backend might
                // accept them -- and silence would be worse.
                ui.label("This backend wants a token for changes; none was given.");
            }
            if let Some(said) = &panels.said {
                ui.label(said.clone());
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Looking at");
                if ui
                    .selectable_label(*showing == Showing::Island, "the island")
                    .clicked()
                {
                    *showing = Showing::Island;
                }
                let has_estate = island.0.estate().is_some();
                ui.add_enabled_ui(has_estate, |ui| {
                    if ui
                        .selectable_label(*showing == Showing::Estate, "the estate")
                        .clicked()
                    {
                        *showing = Showing::Estate;
                    }
                });
            });
            if let Some(layout) = island.0.estate() {
                if *showing == Showing::Estate {
                    ui.label(format!(
                        "{} buildings, {} rooms, {} things",
                        layout.buildings.len(),
                        layout.spaces.len(),
                        layout.items.len()
                    ));
                    let machines = layout.items.iter().filter(|i| i.kind == "Computer").count();
                    ui.label(format!(
                        "{machines} machines in the computer room, drawn from Maer-Ken's models"
                    ));
                }
            }

            ui.separator();
            ui.heading("People");
            for person in &snapshot.world.people {
                let label = format!(
                    "{}{}{}",
                    person.agent_id,
                    if person.alive { "" } else { " (dead)" },
                    if person.asleep { " — asleep" } else { "" }
                );
                if ui
                    .selectable_label(panels.selected.as_deref() == Some(&person.agent_id), label)
                    .clicked()
                {
                    panels.selected = Some(person.agent_id.clone());
                    if let Some(ground) = island.0.terrain() {
                        if let Some(at) = scene::place_people(ground, &snapshot.world, origin.0)
                            .into_iter()
                            .find(|p| p.agent_id == person.agent_id)
                        {
                            view.0.look_at(at.regional, 2.0);
                        }
                    }
                }
            }
        });

    egui::Panel::right("detail")
        .default_size(340.0)
        .show(&mut viewport, |ui| {
            match panels.selected.as_ref().and_then(|id| {
                snapshot
                    .world
                    .people
                    .iter()
                    .find(|person| &person.agent_id == id)
            }) {
                None => {
                    ui.heading("Nobody chosen");
                    ui.label("Pick somebody on the left to read them.");
                }
                Some(person) => {
                    ui.heading(&person.agent_id);
                    ui.label(format!("{:.1} years old", person.age_years));
                    ui.label(if person.asleep { "Asleep" } else { "Awake" });
                    if let Some(space) = &person.space {
                        ui.label(format!("In the {space}"));
                    }
                    if let Some((x, y)) = person.position_m {
                        ui.label(format!("At {x:.1}, {y:.1} m"));
                    }
                    if let Some((row, col)) = person.cell {
                        ui.label(format!("Cell {row}, {col}"));
                    }
                    if let Some(carbon) = person.body_carbon_kg {
                        ui.label(format!("{carbon:.2} kg of body carbon"));
                    }
                    let model = property::human(&person.agent_id, true);
                    if !model.is_really_this {
                        ui.label(
                        "Drawn with a founder's model: nobody created later has one of their own.",
                    );
                    }
                }
            }

            ui.separator();
            ui.checkbox(&mut panels.show_estate, "The estate's things");
            if panels.show_estate {
                for property in snapshot.properties.iter().filter(|p| p.on_this_island) {
                    ui.label(format!(
                        "{} — {} buildings",
                        property.name,
                        property.buildings.len()
                    ));
                    for building in &property.buildings {
                        let model = property::building(&building.kind);
                        ui.label(format!(
                            "  {} ({}){}",
                            building.name,
                            building.kind,
                            if model.is_really_this {
                                ""
                            } else {
                                " — no model"
                            }
                        ));
                    }
                    for item in property.items.iter().filter(|i| i.kind == "Computer") {
                        ui.label(format!(
                            "  {} — {}",
                            item.name,
                            property::item(&item.kind, &item.name).path
                        ));
                    }
                }
            }

            ui.separator();
            ui.checkbox(&mut panels.show_conversations, "What they are saying");
            if panels.show_conversations {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for talk in snapshot.conversations.iter().take(12) {
                        ui.label(format!("tick {} — {}", talk.tick, talk.relationship));
                        for line in &talk.lines {
                            ui.label(format!("  {}: {}", line.speaker_name, line.text));
                        }
                    }
                });
            }
        });
}

/// Send a control command and give back a sentence about what happened.
///
/// Synchronous, unlike the reads: a button press is a thing somebody is
/// waiting on, it is one small request, and queueing it through the poller
/// would mean a press with no answer until the next tick.
fn ask(island: &Island, request: mk_island_api::ControlRequest) -> String {
    match island.client().control(&request) {
        Ok(accepted) => format!("Asked; command {}.", accepted.command),
        Err(err) => err.to_string(),
    }
}
