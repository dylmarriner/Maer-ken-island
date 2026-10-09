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

## What has been verified, and what has not

This is the important section, and it is short because the division is
clean.

**Verified, by tests that run on a machine with no graphics at all:**

- The projection, both frames and the transform between them (`scene.rs`).
- Chunking: coverage, shared edges, winding, normals on flat ground and on
  a slope, land/sea from the mask rather than from a height, and that the
  same island gives the same chunks every time (`terrain.rs`).
- The camera: distance, pitch limits, multiplicative zoom, pan scaling with
  zoom, clip planes that follow it (`camera.rs`).
- The model table: every building kind and item kind answered, every model
  it names present in `assets/`, the four machines in the computer room
  distinct, and a created human marked as wearing a founder's face
  (`property.rs`).
- Reading a real island end to end, both ways the application can:
  `tests/reading_an_island.rs` starts a real backend, reads it through the
  real client, meshes the real terrain, places the real founders and finds
  the real computers.
- Both command-line paths in the real binary, against a live island:
  `--server` against a token-gated backend on another port, and
  `--scenario` with an island in-process on a loopback port.

**Not verified here, and this container cannot verify it:**

- **Anything a GPU does.** No window has ever been opened by this binary on
  the machine it was written on. That is not an oversight and not a
  shortcut — the container has no display, no Vulkan driver (there is no
  ICD at all, and `mesa-vulkan-drivers` is not installable), and no EGL, so
  `wgpu` cannot obtain an adapter by any path. There is no software
  rasterizer to fall back to.

  What that leaves unproven: that the meshes look right, that the materials
  are sensible, that the egui panels lay out, that the camera controls feel
  like anything, and that the frame rate is acceptable. Every one of those
  is a real question and none of them is answered.

The binary knows this about itself. Run with no `DISPLAY` or
`WAYLAND_DISPLAY` it says so in a sentence and exits 1, rather than letting
winit panic with a sixteen-frame backtrace about an event loop — which is
what it did at first, measured on this machine. It checks *after* reading
the island, so a headless operator still finds out whether the backend is
reachable and has a world:

```
$ island-ui --server http://127.0.0.1:8101 --token rd
Reading the island at http://127.0.0.1:8101.
island-ui draws a window and this machine has no display: neither DISPLAY nor
WAYLAND_DISPLAY is set. The island at http://127.0.0.1:8101 is reachable and
has a world -- it is only the drawing that cannot happen here. Run this where
there is a desktop, or read the same island in a browser:
`island serve --frontend-only --backend http://127.0.0.1:8101`.
```

**Before this is relied on as a desktop application, somebody has to run it
on a machine with a screen.** The web dashboard has been driven in Chromium
against a split deployment and the island draws; this has not, and nothing
in this repository should be read as claiming otherwise.

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

`apps/island_ui` is built, and its tests run, in the `desktop` job of
`.github/workflows/ci.yml`. **No windowed test runs in CI**, for the same
reason none runs here. The job also asserts `cargo tree -p island` contains
no `bevy`, so the headless binary stays buildable on a machine with no
graphics packages.
