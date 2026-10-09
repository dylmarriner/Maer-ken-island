//! The island's wire schema: one definition of every shape that crosses
//! the network, shared by the server that produces it and every frontend
//! that reads it.
//!
//! This crate exists because the island stopped being one program. The
//! backend simulates and serves; a web page on one machine and a desktop
//! application on another read it. Three programs that each wrote their own
//! idea of what `/api/world` looks like would agree right up until one of
//! them changed, and then disagree silently — the frontend would show a
//! field as missing rather than fail, which is the worst of the available
//! behaviours. So the types live here once, `Serialize` for the server and
//! `Deserialize` for the clients, and a change to the schema is a change
//! both ends recompile against.
//!
//! It depends on `serde` and nothing else. No engine, no simulation: a
//! client must be able to link this without building the island.

#![forbid(unsafe_code)]

pub mod request;
pub mod terrain;
pub mod version;
pub mod wire;

pub use request::{
    Accepted, ControlRequest, CreateHumanRequest, InterventionRequest, InterventionTarget, Outcome,
    Refusal,
};
pub use terrain::{Terrain, TerrainError};
pub use version::{Capabilities, ServerVersion, API_VERSION};
pub use wire::{
    Building, Cell, Clock, Conversation, ConversationLine, Digest, Economy, EconomyEntry, Estate,
    Item, Land, Person, Property, SpaceChoice, Stocks, Structure, TimelineEntry, Tree, Trees,
    World,
};
