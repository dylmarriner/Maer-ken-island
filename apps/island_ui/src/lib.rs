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

#[cfg(test)]
mod adapter_independence {
    /// Nothing that builds what is drawn may ask what it is drawn on.
    ///
    /// This is the test that carries "it works under lavapipe" across to
    /// "it works on a GPU", and it is worth being precise about why,
    /// because that step is otherwise an assumption and this container
    /// has no GPU to check it against.
    ///
    /// `wgpu` is the abstraction: the same commands go to a software
    /// rasteriser and to a discrete card, and the application does not
    /// choose between them. So if no code in this library branches on
    /// which adapter is present -- no fast path, no fallback geometry,
    /// no `cfg!` gate -- then what the smoke tests prove under lavapipe
    /// is a property of the *meshes and the camera*, which are the same
    /// meshes and the same camera on any adapter.
    ///
    /// The moment somebody adds `if discrete_gpu { … } else { … }` that
    /// stops being true and the frame rates in `RENDER_STACK.md` stop
    /// transferring. This fails then, which is the point: it is not
    /// guarding a style, it is guarding an argument.
    ///
    /// `main.rs` is deliberately not covered. It *does* read the adapter
    /// -- to say which one it got and to warn when it is a software one
    /// -- and that is reporting, not rendering. The modules below are
    /// where every vertex, normal, colour and transform is decided.
    #[test]
    fn nothing_that_builds_a_frame_asks_what_it_is_drawn_on() {
        const SOURCES: [(&str, &str); 6] = [
            ("terrain.rs", include_str!("terrain.rs")),
            ("scene.rs", include_str!("scene.rs")),
            ("estate.rs", include_str!("estate.rs")),
            ("camera.rs", include_str!("camera.rs")),
            ("property.rs", include_str!("property.rs")),
            ("source.rs", include_str!("source.rs")),
        ];
        // Words that would mean a decision was made about the adapter.
        const ASKING: [&str; 7] = [
            "device_type",
            "AdapterInfo",
            "RenderAdapter",
            "DiscreteGpu",
            "IntegratedGpu",
            "llvmpipe",
            "lavapipe",
        ];
        for (name, text) in SOURCES {
            for asked in ASKING {
                assert!(
                    !text.contains(asked),
                    "{name} mentions `{asked}`. If the geometry now depends on which adapter \
                     is present, then measuring it under a software rasteriser no longer says \
                     anything about a GPU, and `docs/island/RENDER_STACK.md` is making a claim \
                     it cannot support."
                );
            }
        }
    }
}
