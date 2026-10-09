//! Maer-Ken Island as a desktop application, minus the window.
//!
//! Everything that is not the renderer lives here: where the island comes
//! from, how its state becomes a scene, how the ground becomes meshes,
//! where the camera is, and which model stands for which thing. All of it
//! is ordinary Rust with no Bevy in it, which is what lets it be tested on
//! a machine with no graphics at all -- and this one has none.
//!
//! `main.rs` is the window: it reads these, hands the results to Bevy, and
//! does nothing else. That split is deliberate and it is where the line
//! between "tested" and "not tested here" falls, which
//! `docs/island/RENDER_STACK.md` states plainly rather than leaving to be
//! discovered.

#![forbid(unsafe_code)]

pub mod camera;
pub mod estate;
pub mod property;
pub mod scene;
pub mod source;
pub mod terrain;

/// The build this binary came from.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
