//! Maer-Ken Island regional contracts: the island profile, the regional
//! domain geometry and boundary-forcing data. Data only — this
//! crate depends on `mk_core` and never on `mk_engine`.

pub mod boundary;
pub mod domain;
pub mod profile;
pub mod scenario;

pub use boundary::{
    AstronomyForcing, AtmosphereBoundaryForcing, CrustKind, Edge, EdgeAtmosphereForcing,
    EdgeOceanForcing, FarFieldPlate, OceanBoundaryForcing, RegionalBoundaryState,
    TectonicBoundaryForcing,
};
pub use domain::{
    DomainLevel, IslandDomain, IslandDomainError, LocalPatchSpec, MAX_LOCAL_PATCH_CELLS,
};
pub use profile::{
    GeologyProfile, IslandProfile, IslandProfileError, ShapeMetrics, ShapeRequirements,
    ISLAND_CANON_RADIUS_M, PROFILE_VERSION,
};
pub use scenario::{
    EstateEnergyConfig, EstatePatchConfig, IslandCadenceProfile, IslandScenario,
    IslandScenarioError, SCENARIO_VERSION,
};
