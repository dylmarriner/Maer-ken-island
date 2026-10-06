//! Validation against real-world data (`docs/island/REALISM.md` §2).
//!
//! The reference packs under `fixtures/reference/` hold the published values
//! that island subsystems are compared with; `reference` loads and checks
//! them.

pub mod reference;

pub use reference::{
    default_reference_dir, Confidence, ReferenceDomain, ReferenceError, ReferenceItem,
    ReferenceLibrary, ReferencePack, ReferenceSource, ReferenceTable, ReferenceValue, TableRow,
};
