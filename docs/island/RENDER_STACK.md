# The desktop application's render stack

What `island-ui` is built on, what it draws, and — the part that matters
most when reading this — which half of it has been verified and which half
has not, and why.

## Pins

| Thing | Version | Why this one |
|---|---|---|
| `bevy` | 0.19.1 | 0.20 requires Rust 1.97.1 and this workspace is pinned to 1.97.0 in `rust-toolchain.toml`. |
| `bevy_egui` | 0.42 | The release that targets Bevy 0.19. |
| `egui` | 0.36 | Whatever `bevy_egui` 0.42 brings. |
| Rust | 1.97.0 | `rust-toolchain.toml`, unchanged. |

### Features, and why the list is not the default one

`bevy`'s default features pull in `bevy_audio`, which needs **alsa**, and
`bevy_gilrs`, which needs **libudev**. This island has no sound and no
gamepads, and the development packages for either are not on a plain
server — `libasound2-dev` and `libudev-dev` are not even in this
container's package lists. So the features are named explicitly and both
are left out.

What is left builds from source with no system development packages at
all. Measured: a clean build of the 392-crate dependency tree took
**9 minutes 19 seconds** and 1.9 GB of target directory on four cores.

`bevy_world_serialization` is in the list because of a rename: in Bevy 0.19
a glTF scene is a `WorldAsset` and the component that spawns one is
`WorldAssetRoot`. It used to be `SceneRoot`, and "scene" now means the new
BSN scene system instead.

Wayland is deliberately not enabled. It needs `libwayland-dev` at build
time; X11 goes through `x11-dl`, which loads at run time and therefore
builds anywhere.

## What is drawn

| Layer | Where it comes from | Where the code is |
|---|---|---|
| Ground | `/api/elevation.bin`, chunked into 64×64-cell meshes | `src/terrain.rs` |
| Coast | Land and sea triangles split by the island's own land mask, not by height | `src/terrain.rs`, `chunk_mesh` in `src/main.rs` |
| Islanders | `/api/world`, placed in two frames | `src/scene.rs` |
| The estate's things | `/api/properties`, mapped to the Maer-Ken models | `src/property.rs` |
| Camera | Orbit over the map | `src/camera.rs` |
| Panels | Clock, controls, roster, one person, the computer room, conversations | `panels` in `src/main.rs` |

### Two frames, and why one will not do

The island is about **2,400 km** across and a bedroom is **four metres**.
One scale cannot hold both:

- At a scale that puts the island on screen, both founders are inside one
  floating-point step of each other.
- At a scale that draws the bedroom, the far coast is two hundred million
  units away, where an `f32` has whole metres of granularity left.

So `scene.rs` has a **regional** frame (1 unit = 10 km, relief exaggerated
20×) and an **estate** frame (1 unit = 1 m, origin floating to the patch).
The exaggeration is a presentation choice and the inspector always shows
metres from the terrain, never from the drawing.

The floating origin is not a precaution, it is a measurement: the estate
stands 113,967 m east and 124,015 m north of the domain's south-west
corner, and an `f32` metre at that distance has a step of **7.8 mm** —
coarse enough to see on a four-metre room. From the patch's own corner the
numbers are single-digit metres and the step is half a micrometre.
`the_estate_frame_keeps_precision_a_bedroom_needs` asserts the 7.8 mm
figure, so this paragraph cannot drift from the arithmetic.

### Chunking

The medium grid is 1,200 × 960 cells. One mesh would be 1,152,000 quads in
a single buffer that no culler can do anything with, because it is one
object. 64×64-cell chunks give **19 × 15 = 285** meshes, of which a camera
over the estate draws a handful.

Chunks share a row and a column of **vertices** with their neighbours. That
is not waste: chunks that stopped at their own last cell would leave a
crack of missing triangles right through the island.

## What has been verified

### It draws the island

`scripts/render-smoke.sh` runs the application on a virtual screen
against an island it starts itself, photographs a frame and asserts the
picture is of something. It runs in CI on every push and the frame is
uploaded as an artifact.

That check exists because of what it found. **The first frame this
application ever drew was nearly black**, and two real defects were in it,
neither of which any unit test could have seen:

- **Every terrain triangle was wound so its geometric normal pointed
  down.** x runs east and z runs *south* in this frame, so the obvious
  `[i, up, right]` — which reads as counter-clockwise — gives a normal of
  −y. All 285 chunks were backface-culled from any camera above them. The
  shading normals in `Chunk::normals` were correct throughout and did not
  help, because culling is decided by vertex order.
  `every_triangle_faces_the_sky` now holds it.
- **`bevy_egui` was taken with `default-features = false`,** which drops
  `default_fonts`. egui had no font data, so every panel drew as an empty
  grey box. The feature is back and named with a comment saying why.

Measured, the frames either side of those fixes:

| frame | colours | land | sea | light text |
|---|---|---|---|---|
| broken | 497 | 0.0% | 0.0% | 0.00% |
| drawing | 3,964 | 1.1% | 14.8% | 0.08% |
| drawing, embedded island | 4,904 | 2.1% | 29.6% | 0.10% |

The smoke test's thresholds sit between those columns and well away from
both: 1,500 colours, 0.2% land, 5% sea, 0.02% text. Reverting the winding
alone fails it with three named reasons, which was checked rather than
assumed.

What a rendered frame shows, against the full 1,200 × 960 island: the
landmass with its relief, the sea floor around it with the trench and the
outer rise, the domain's edge buffer, both egui panels with the clock, the
digest, the speed controls and the founders' roster, and the simulation
running — 59.9× of the 60× asked for in the remote case, 926.5× with
`--scenario` flat out.

### Everything that is not the renderer

Tested on a machine with no graphics at all, which is most of the
application:

- The projection, both frames and the transform between them (`scene.rs`).
- Chunking: coverage, shared edges, **winding**, normals on flat ground
  and on a slope, land/sea from the mask rather than from a height, and
  determinism (`terrain.rs`).
- The camera: distance, pitch limits, multiplicative zoom, pan scaling
  with zoom, clip planes that follow it (`camera.rs`).
- The model table: every building kind and item kind answered, every model
  it names present in `assets/`, the four machines in the computer room
  distinct (`property.rs`).
- Reading a real island end to end, both ways:
  `tests/reading_an_island.rs`.

### Still not verified

- **How it feels.** Nobody has used this with a mouse. The camera's
  sensitivities, the panel layout at other window sizes and whether the
  controls are pleasant are unanswered, and a screenshot cannot answer
  them.
- **Frame rate on real hardware.** Everything above was rendered by
  **lavapipe**, a software rasteriser, which Bevy warns about on startup
  and which is slower than any GPU by orders of magnitude. That it draws
  at all on a CPU is a good sign for a GPU; it is not a measurement of
  one, and no frame-rate figure is claimed here.
- **Anything beyond the first frame under interaction.** The smoke test
  photographs a settled frame; it does not drag, zoom or click.

### A correction worth recording

An earlier version of this document said the container could not render at
all: no display, no Vulkan driver, no EGL, nothing installable. The first
two were true and the third was not. `apt-cache policy` reported no
candidate for `mesa-vulkan-drivers` because **the package lists were
stale** — a plain `apt-get update` made lavapipe, `libegl1` and
`libxkbcommon-x11-0` all available, and Xvfb was installed the whole time.

The conclusion was reported honestly from what was checked, and what was
checked was not enough. "This cannot be verified here" deserves the same
scepticism as any other claim, and it got it one step too late.

## One way in

`--scenario` does not get its own path to the island. It starts the same
backend `island serve` starts, on `127.0.0.1:0`, and reads it through the
same `mk_island_client` a remote one uses.

That costs a loopback round trip — microseconds against a 4.3 ms step — and
buys the thing worth having: the local case is proven by the remote one,
and there is no second reader to drift from the first. It also means a
locally-run island can be opened in a browser at the same time, which the
message above suggests.

## CI

`apps/island_ui` is built, linted, tested **and rendered** in the
`desktop` job of `.github/workflows/ci.yml`. The render step installs
Xvfb, ImageMagick and `mesa-vulkan-drivers`, runs
`scripts/render-smoke.sh`, and uploads the frame as an artifact whether it
passed or failed — a picture is the fastest way to see what went wrong
with a picture.

Nothing is installed to *build* it. That is a separate claim and the job
keeps them separate: if the build step ever needs a system package, a
Bevy feature has crept back in. The job also asserts `cargo tree -p island`
contains no `bevy`, so the headless backend stays buildable on a machine
with no graphics packages at all.
