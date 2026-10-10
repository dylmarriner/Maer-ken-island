//! PHASE 2–4 MARR'KENA ENGINE
//!
//! Purpose
//! - Implement planet-level physical systems for Marr'Kena MK-I
//! - Implement animal-only biosphere (Phase 3)
//! - Implement deep-time evolution (Phase 4)
//! - Deterministic, ledger-tracked, audit-compliant
//!
//! Invariants
//! - Phase 2 (physics): Q32.32 fixed-point or integer arithmetic
//! - Phase 3–4 (biosphere/evolution): floating-point with deterministic RNG seeding
//! - All energy transfers must be ledger-tracked
//! - Determinism enforced: same tick + RNG seed → same state
//! - No world/planet boundary contamination
//! - Intelligence ceiling: hard-enforced at 0.40 (pre-sapient)
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md (binding execution plan)
//! - MARRKENA_PLANET_EXECUTION_MAP.md (biosphere integration)
//! - PLANET_CONSTANTS.md (orbital & planetary parameters)
//! - COSMOS_CONSTANTS.md (stellar context)
//! - LAW_CONSTANTS.md (world-level time authority)

pub mod biosphere;
pub mod chronicle;
pub mod climate;
pub mod conservation;
pub mod disturbance;
pub mod governance;
pub mod hydrology;
pub mod insolation;
pub mod interventions;
pub mod io;
pub mod long_horizon_verification;
pub mod materials;
pub mod mki_declaration;
pub mod ocean;
pub mod orbit;
pub mod organisms;
pub mod perception;
pub mod phase7_release_artifacts;
pub mod physics;
pub mod planet;
pub mod regional;
pub mod replay;
pub mod resource_economy;
pub mod rotation;
pub mod runtime;
pub mod sim;
pub mod tectonics;
pub mod tides;
pub mod topology;
pub mod transparent_ui;
pub mod validation;
pub mod verification;
pub mod volcanism;
pub mod weather;
pub mod world_integration;

mod agents;
pub mod humans;

pub use biosphere::{BiosphereStatistics, BiosphereSystem};
pub use climate::{step_climate, ClimateState};
pub use hydrology::{step_hydrology, HydrologyState, SoilWater};
pub use insolation::{step_insolation, InsolationField};
pub use long_horizon_verification::{
    generate_final_report, run_all_verifications, run_all_verifications_for_release_artifacts,
    run_release_verifications_for_horizons, LongHorizonVerifier, VerificationConfig,
    VerificationHorizon, VerificationResults, WorldStepMode,
};
pub use mki_declaration::{ComplianceReport, DeclarationArtifacts, MKIDeclaration, MKIStatus};
pub use ocean::{step_ocean, OceanColumn, OceanVelocity};
pub use orbit::{solve_kepler, step_orbit, KeplerError, OrbitState};
pub use organisms::{OrganismSchema, SchemaValidationError};
pub use phase7_release_artifacts::{
    emit_phase7_artifacts, emit_phase7_artifacts_for_horizons, verification_results_to_json,
    Phase7ArtifactEmitterMeta,
};
pub use planet::{step_planet, PlanetFields};
pub use replay::{ReplayManager, ReplayOptions, ReplaySession, ReplayStatistics};
pub use resource_economy::{
    EconomyEvent, EconomyEventKind, RecipeId, ResourceEconomyState, ResourceNode, ResourceNodeKind,
    ToolInstance, ToolKind,
};
pub use rotation::{step_rotation, RotationState};
pub use sim::WorldState;
pub use tectonics::{step_tectonics, PlateCell, PlateType};
pub use tides::{step_tides, TidalState};
pub use transparent_ui::{
    BiosphereObservatory, TraceableValue, TransparentUISystem, UIConfig, UIMode,
    VerificationObservatory,
};
pub use volcanism::{step_volcanism, VolcanicCell, VolcanicGases};
pub use weather::{step_weather, WeatherState, WindVector};
pub use world_integration::WorldSnapshot;

// Event database exports
pub use io::{
    get_event_database,
    init_event_database,
    is_event_database_initialized,
    log_human_born,
    log_human_died,
    log_human_reproduced,
    log_species_created,
    log_species_extinct,
    log_species_mutated,
    print_universe_summary,
    CognitionStorage,
    CultureStorage,
    DevelopmentStorage,
    EntityType,
    EventDatabase,
    EventType,
    GeneticsStorage,
    HumanProfileStorage,
    // Human storage
    HumanStorage,
    HumanStorageError,
    LanguageStorage,
    LifecycleStorage,
    PersonalityStorage,
    SocialStorage,
    TechnologyStorage,
    UniverseEvent,
};
