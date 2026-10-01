use crate::long_horizon_verification::{VerificationHorizon, VerificationResults};
use crate::world_integration::WorldState;
/// Phase 7 Transparent UI / Release Readiness
///
/// Exposes truth without contaminating simulation authority.
/// Implements read-only by default, explicit modes, and full traceability.
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UI System with transparent observability
#[derive(Debug, Clone)]
pub struct TransparentUISystem {
    pub config: UIConfig,
    pub permissions: PermissionManager,
    pub current_mode: UIMode,
    pub world_state: Option<WorldState>,
    pub audit_trail: UIAuditTrail,
    /// Applies permitted interventions to `world_state`.
    ///
    /// Separate from `permissions`: `permissions` gates *this UI's* operation
    /// names, while the executor carries the typed
    /// `mk_interventions::InterventionPermissions` capability set and does
    /// the mutation.
    pub executor: crate::interventions::InterventionExecutor,
}

/// UI Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub read_only_by_default: bool,
    pub require_explicit_modes: bool,
    pub full_traceability: bool,
    pub no_hidden_state: bool,
    pub show_failures_honestly: bool,
    pub tick_precision: TickPrecision,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            read_only_by_default: true,
            require_explicit_modes: true,
            full_traceability: true,
            no_hidden_state: true,
            show_failures_honestly: true,
            tick_precision: TickPrecision::Exact,
        }
    }
}

/// Tick precision for traceability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TickPrecision {
    Exact,
    Approximate,
    Range,
}

/// UI Operation Modes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UIMode {
    Observe,   // Read-only observation
    Scrub,     // Scrub through timeline
    Intervene, // Gated intervention
}

/// Permission Manager for UI operations
#[derive(Debug, Clone)]
pub struct PermissionManager {
    pub intervention_permissions: HashMap<String, InterventionPermission>,
    pub audit_required: bool,
    pub gated_operations: Vec<String>,
}

/// Intervention permission levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterventionPermission {
    Forbidden,
    ReadOnly,
    Gated { requires_audit: bool },
    Allowed,
}

/// UI Audit Trail
#[derive(Debug, Clone, Serialize)]
pub struct UIAuditTrail {
    pub operations: Vec<UIOperation>,
    pub permission_violations: Vec<PermissionViolation>,
    pub traceability_log: Vec<TraceabilityEntry>,
}

/// UI Operation record
#[derive(Debug, Clone, Serialize)]
pub struct UIOperation {
    pub timestamp: std::time::SystemTime,
    pub mode: UIMode,
    pub operation: String,
    pub parameters: HashMap<String, String>,
    pub tick: Option<Tick>,
    pub source_trace: SourceTrace,
    pub outcome: OperationOutcome,
}

/// Source trace for every number
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTrace {
    pub module: String,
    pub function: String,
    pub tick: Tick,
    pub calculation_path: Vec<String>,
    pub raw_value: String,
    pub derivation: String,
}

/// Operation outcome
#[derive(Debug, Clone, Serialize)]
pub enum OperationOutcome {
    Success { result: String },
    LoggedAndGated { result: String },
    Failure { error: String },
    PermissionDenied { reason: String },
    AuditRequired { pending_review: String },
}

/// Permission violation record
#[derive(Debug, Clone, Serialize)]
pub struct PermissionViolation {
    pub timestamp: std::time::SystemTime,
    pub attempted_operation: String,
    pub required_permission: String,
    pub actual_permission: InterventionPermission,
    pub tick: Option<Tick>,
}

/// Traceability entry
#[derive(Debug, Clone, Serialize)]
pub struct TraceabilityEntry {
    pub tick: Tick,
    pub observable: String,
    pub value: String,
    pub source_module: String,
    pub source_function: String,
    pub calculation: String,
    pub dependencies: Vec<String>,
}

impl TransparentUISystem {
    pub fn new(config: UIConfig) -> Self {
        Self {
            permissions: PermissionManager::new(),
            current_mode: UIMode::Observe,
            world_state: None,
            executor: crate::interventions::InterventionExecutor::new(
                mk_interventions::InterventionPermissions::default(),
            ),
            audit_trail: UIAuditTrail::new(),
            config,
        }
    }

    /// Load world state for observation
    pub fn load_world_state(&mut self, world_state: WorldState) -> Result<(), UIError> {
        self.world_state = Some(world_state);

        self.audit_trail.operations.push(UIOperation {
            timestamp: std::time::SystemTime::now(),
            mode: self.current_mode.clone(),
            operation: "load_world_state".to_string(),
            parameters: HashMap::new(),
            tick: self.world_state.as_ref().map(|w| w.tick),
            source_trace: SourceTrace {
                module: "transparent_ui".to_string(),
                function: "load_world_state".to_string(),
                tick: self.world_state.as_ref().map(|w| w.tick).unwrap_or(0),
                calculation_path: vec!["world_state".to_string()],
                raw_value: "loaded".to_string(),
                derivation: "World state loaded into UI".to_string(),
            },
            outcome: OperationOutcome::Success {
                result: "World state loaded successfully".to_string(),
            },
        });

        Ok(())
    }

    /// Switch UI mode with permission checking
    pub fn switch_mode(&mut self, new_mode: UIMode) -> Result<(), UIError> {
        let operation = format!("switch_mode_{:?}", new_mode);

        if !self.permissions.can_switch_mode(&new_mode) {
            self.audit_trail
                .permission_violations
                .push(PermissionViolation {
                    timestamp: std::time::SystemTime::now(),
                    attempted_operation: operation.clone(),
                    required_permission: format!("mode_switch_{:?}", new_mode),
                    actual_permission: InterventionPermission::Forbidden,
                    tick: self.world_state.as_ref().map(|w| w.tick),
                });

            return Err(UIError::PermissionDenied(format!(
                "Cannot switch to {:?} mode",
                new_mode
            )));
        }

        let old_mode = self.current_mode.clone();
        let new_mode_clone = new_mode.clone();
        self.current_mode = new_mode;

        self.audit_trail.operations.push(UIOperation {
            timestamp: std::time::SystemTime::now(),
            mode: old_mode.clone(),
            operation,
            parameters: HashMap::new(),
            tick: self.world_state.as_ref().map(|w| w.tick),
            source_trace: SourceTrace {
                module: "transparent_ui".to_string(),
                function: "switch_mode".to_string(),
                tick: self.world_state.as_ref().map(|w| w.tick).unwrap_or(0),
                calculation_path: vec!["mode".to_string()],
                raw_value: format!("{:?}", new_mode_clone),
                derivation: "UI mode switched".to_string(),
            },
            outcome: OperationOutcome::Success {
                result: format!("Switched from {:?} to {:?}", old_mode, new_mode_clone),
            },
        });

        Ok(())
    }

    /// Get observable value with full traceability
    pub fn get_observable(&self, observable: &str) -> Result<TraceableValue, UIError> {
        let world_state = self
            .world_state
            .as_ref()
            .ok_or(UIError::NoWorldStateLoaded)?;

        let (value, source) = match observable {
            "tick" => (
                world_state.tick.to_string(),
                SourceTrace {
                    module: "world_integration".to_string(),
                    function: "WorldState::tick".to_string(),
                    tick: world_state.tick,
                    calculation_path: vec!["tick".to_string()],
                    raw_value: world_state.tick.to_string(),
                    derivation: "Current simulation tick".to_string(),
                },
            ),
            "species_count" => (
                world_state.biosphere_state.species.len().to_string(),
                SourceTrace {
                    module: "biosphere".to_string(),
                    function: "BiosphereSystem::species".to_string(),
                    tick: world_state.tick,
                    calculation_path: vec![
                        "biosphere".to_string(),
                        "species".to_string(),
                        "len".to_string(),
                    ],
                    raw_value: world_state.biosphere_state.species.len().to_string(),
                    derivation: "Number of species in biosphere".to_string(),
                },
            ),
            "total_population" => {
                let total = world_state
                    .biosphere_state
                    .species
                    .iter()
                    .map(|s| s.population_size)
                    .sum::<u64>();
                (
                    total.to_string(),
                    SourceTrace {
                        module: "biosphere".to_string(),
                        function: "Species::population_size".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec![
                            "biosphere".to_string(),
                            "species".to_string(),
                            "population_size".to_string(),
                            "sum".to_string(),
                        ],
                        raw_value: total.to_string(),
                        derivation: "Sum of all species populations".to_string(),
                    },
                )
            }
            "max_intelligence" => {
                let max_intelligence = world_state
                    .biosphere_state
                    .species
                    .iter()
                    .map(|s| s.representative_genome.intelligence_index())
                    .fold(0.0, f64::max);
                (
                    max_intelligence.to_string(),
                    SourceTrace {
                        module: "genetics".to_string(),
                        function: "Genome::intelligence_index".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec![
                            "biosphere".to_string(),
                            "species".to_string(),
                            "genome".to_string(),
                            "intelligence_index".to_string(),
                            "max".to_string(),
                        ],
                        raw_value: max_intelligence.to_string(),
                        derivation: "Maximum intelligence index across all species".to_string(),
                    },
                )
            }
            "energy_closure" => (
                world_state.audit_trail.energy_closure.to_string(),
                SourceTrace {
                    module: "world_integration".to_string(),
                    function: "AuditTrail::energy_closure".to_string(),
                    tick: world_state.tick,
                    calculation_path: vec!["audit_trail".to_string(), "energy_closure".to_string()],
                    raw_value: world_state.audit_trail.energy_closure.to_string(),
                    derivation: "Energy conservation closure error".to_string(),
                },
            ),
            _ => return Err(UIError::ObservableNotFound(observable.to_string())),
        };

        Ok(TraceableValue {
            value,
            source,
            observable: observable.to_string(),
        })
    }

    /// Get biosphere observatory data
    pub fn get_biosphere_observatory(&self) -> Result<BiosphereObservatory, UIError> {
        let world_state = self
            .world_state
            .as_ref()
            .ok_or(UIError::NoWorldStateLoaded)?;

        let species_data: Vec<SpeciesObservation> = world_state
            .biosphere_state
            .species
            .iter()
            .enumerate()
            .map(|(i, species)| {
                let genome = &species.representative_genome;
                SpeciesObservation {
                    species_id: species.species_id,
                    species_name: species.species_name.clone(),
                    population_size: species.population_size,
                    geographic_range: species.geographic_range,
                    intelligence_index: genome.intelligence_index(),
                    is_marine: genome.structural.environmental_adaptation == 0,
                    is_terrestrial: genome.structural.environmental_adaptation == 1,
                    body_mass_kg: genome.body_mass_kg(),
                    neural_capacity: genome.neural.processing_capacity,
                    metabolic_rate: genome.metabolic.basal_rate,
                    tick: world_state.tick,
                    source_trace: SourceTrace {
                        module: "biosphere".to_string(),
                        function: "BiosphereSystem::species".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["species".to_string(), format!("index_{}", i)],
                        raw_value: format!("species_{}", species.species_id),
                        derivation: "Individual species observation".to_string(),
                    },
                }
            })
            .collect();

        Ok(BiosphereObservatory {
            tick: world_state.tick,
            total_species: species_data.len(),
            total_population: species_data.iter().map(|s| s.population_size).sum(),
            max_intelligence: species_data
                .iter()
                .map(|s| s.intelligence_index)
                .fold(0.0, f64::max),
            marine_species_count: species_data.iter().filter(|s| s.is_marine).count(),
            terrestrial_species_count: species_data.iter().filter(|s| s.is_terrestrial).count(),
            species: species_data,
            intelligence_ceiling_status: world_state.audit_trail.intelligence_ceiling_status,
            source_trace: SourceTrace {
                module: "biosphere_observatory".to_string(),
                function: "get_biosphere_observatory".to_string(),
                tick: world_state.tick,
                calculation_path: vec!["biosphere".to_string(), "observatory".to_string()],
                raw_value: "biosphere_snapshot".to_string(),
                derivation: "Complete biosphere observation".to_string(),
            },
        })
    }

    /// Get verification observatory data
    pub fn get_verification_observatory(
        &self,
        results: &VerificationResults,
    ) -> VerificationObservatory {
        VerificationObservatory {
            horizon: results.horizon,
            status: if results.success {
                "PASSED".to_string()
            } else {
                "FAILED".to_string()
            },
            observables: results.observables.clone(),
            time_points: results.time_series.len(),
            final_hash: results.final_hash,
            determinism_proof: results.determinism_proof.clone(),
            errors: results.errors.clone(),
            warnings: results.warnings.clone(),
            source_trace: SourceTrace {
                module: "verification_observatory".to_string(),
                function: "get_verification_observatory".to_string(),
                tick: self.world_state.as_ref().map(|w| w.tick).unwrap_or(0),
                calculation_path: vec!["verification".to_string(), "results".to_string()],
                raw_value: format!("verification_{:?}", results.horizon),
                derivation: "Verification results observation".to_string(),
            },
        }
    }

    /// Attempt intervention (always gated)
    pub fn attempt_intervention(
        &mut self,
        intervention: InterventionRequest,
    ) -> Result<InterventionResult, UIError> {
        // ALWAYS check permissions first to log violations, even if mode is wrong
        let permission = self
            .permissions
            .check_intervention_permission(&intervention.operation);

        if permission == InterventionPermission::Forbidden {
            self.audit_trail
                .permission_violations
                .push(PermissionViolation {
                    timestamp: std::time::SystemTime::now(),
                    attempted_operation: intervention.operation.clone(),
                    required_permission: "intervention_allowed".to_string(),
                    actual_permission: InterventionPermission::Forbidden,
                    tick: self.world_state.as_ref().map(|w| w.tick),
                });
        }

        if self.current_mode != UIMode::Intervene {
            return Err(UIError::WrongMode(
                "Intervention requires Intervene mode".to_string(),
            ));
        }

        match permission {
            InterventionPermission::Forbidden => {
                return Err(UIError::PermissionDenied(format!(
                    "Intervention '{}' is forbidden",
                    intervention.operation
                )));
            }
            InterventionPermission::ReadOnly => {
                return Err(UIError::PermissionDenied(
                    "Read-only mode - interventions not allowed".to_string(),
                ));
            }
            InterventionPermission::Gated { requires_audit } => {
                if requires_audit {
                    return Err(UIError::AuditRequired(
                        "Intervention requires audit approval".to_string(),
                    ));
                }
            }
            InterventionPermission::Allowed => {
                // Proceed with intervention
            }
        }

        // Log intervention attempt
        self.audit_trail.operations.push(UIOperation {
            timestamp: std::time::SystemTime::now(),
            mode: self.current_mode.clone(),
            operation: format!("intervention_{}", intervention.operation),
            parameters: intervention.parameters.clone(),
            tick: self.world_state.as_ref().map(|w| w.tick),
            source_trace: SourceTrace {
                module: "intervention".to_string(),
                function: "attempt_intervention".to_string(),
                tick: self.world_state.as_ref().map(|w| w.tick).unwrap_or(0),
                calculation_path: vec!["intervention".to_string()],
                raw_value: format!("{:?}", intervention),
                derivation: "Intervention attempt logged".to_string(),
            },
            outcome: OperationOutcome::LoggedAndGated {
                result: "Intervention accepted; applying to world state".to_string(),
            },
        });

        let audit_id = self.next_audit_id();

        // A gated-but-not-audit-required operation is deliberately logged
        // without being applied: that is what "gated" means. It is reported
        // as `LoggedAndGated` so the caller can tell it apart from an
        // applied intervention, rather than both paths returning the same
        // value.
        if matches!(permission, InterventionPermission::Gated { .. }) {
            return Ok(InterventionResult {
                status: InterventionStatus::LoggedAndGated,
                message: "Intervention logged under gating; no world mutation performed."
                    .to_string(),
                audit_id,
            });
        }

        // `Allowed`: actually apply it.
        let world = self
            .world_state
            .as_mut()
            .ok_or(UIError::NoWorldStateLoaded)?;

        let executor = self.executor.clone();
        match executor.execute(world, &intervention.action) {
            Ok(outcome) => {
                let summary = outcome.summary();
                if let Some(operation) = self.audit_trail.operations.last_mut() {
                    operation.outcome = OperationOutcome::Success {
                        result: summary.clone(),
                    };
                }
                Ok(InterventionResult {
                    status: InterventionStatus::Applied {
                        summary: summary.clone(),
                    },
                    message: format!("Intervention applied: {summary}"),
                    audit_id,
                })
            }
            Err(err) => {
                // A failed intervention is recorded as a failure, not
                // silently reported as gated success.
                let message = err.to_string();
                if let Some(operation) = self.audit_trail.operations.last_mut() {
                    operation.outcome = OperationOutcome::Failure {
                        error: message.clone(),
                    };
                }
                Err(UIError::InterventionFailed(message))
            }
        }
    }

    /// Monotonic audit identifier.
    ///
    /// Derived from the audit trail's own length and the world tick rather
    /// than from wall-clock seconds: the previous `SystemTime`-based id
    /// collided for every intervention within the same second and could not
    /// be reproduced on replay.
    fn next_audit_id(&self) -> String {
        let tick = self.world_state.as_ref().map(|w| w.tick).unwrap_or(0);
        format!("audit_t{}_{}", tick, self.audit_trail.operations.len())
    }

    /// Get all available observables from current world state
    pub fn get_observables(&self) -> HashMap<String, TraceableValue> {
        let mut observables = HashMap::new();

        if let Some(world_state) = &self.world_state {
            // Core observables
            observables.insert(
                "tick".to_string(),
                TraceableValue {
                    value: world_state.tick.to_string(),
                    source: SourceTrace {
                        module: "world_integration".to_string(),
                        function: "WorldState::tick".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["tick".to_string()],
                        raw_value: world_state.tick.to_string(),
                        derivation: "Current simulation tick".to_string(),
                    },
                    observable: "tick".to_string(),
                },
            );

            observables.insert(
                "species_count".to_string(),
                TraceableValue {
                    value: world_state.biosphere_state.species.len().to_string(),
                    source: SourceTrace {
                        module: "biosphere".to_string(),
                        function: "BiosphereSystem::species".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["species".to_string()],
                        raw_value: world_state.biosphere_state.species.len().to_string(),
                        derivation: "Number of species in biosphere".to_string(),
                    },
                    observable: "species_count".to_string(),
                },
            );

            observables.insert(
                "total_population".to_string(),
                TraceableValue {
                    value: world_state
                        .biosphere_state
                        .statistics
                        .total_population
                        .to_string(),
                    source: SourceTrace {
                        module: "biosphere".to_string(),
                        function: "BiosphereStatistics::total_population".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec![
                            "statistics".to_string(),
                            "total_population".to_string(),
                        ],
                        raw_value: world_state
                            .biosphere_state
                            .statistics
                            .total_population
                            .to_string(),
                        derivation: "Total population across all species".to_string(),
                    },
                    observable: "total_population".to_string(),
                },
            );

            // Climate observables
            let temp = world_state.climate_state.global_temperature();
            observables.insert(
                "global_temperature".to_string(),
                TraceableValue {
                    value: temp.to_string(),
                    source: SourceTrace {
                        module: "climate".to_string(),
                        function: "ClimateState::global_temperature".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["global_temperature".to_string()],
                        raw_value: temp.to_string(),
                        derivation: "Global average temperature".to_string(),
                    },
                    observable: "global_temperature".to_string(),
                },
            );

            let co2 = world_state.climate_state.co2_concentration();
            observables.insert(
                "co2_concentration".to_string(),
                TraceableValue {
                    value: co2.to_string(),
                    source: SourceTrace {
                        module: "climate".to_string(),
                        function: "ClimateState::co2_concentration".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["co2_concentration".to_string()],
                        raw_value: co2.to_string(),
                        derivation: "Atmospheric CO2 concentration".to_string(),
                    },
                    observable: "co2_concentration".to_string(),
                },
            );

            // Audit observables
            observables.insert(
                "intelligence_ceiling_status".to_string(),
                TraceableValue {
                    value: world_state
                        .audit_trail
                        .intelligence_ceiling_status
                        .to_string(),
                    source: SourceTrace {
                        module: "audit".to_string(),
                        function: "AuditTrail::intelligence_ceiling_status".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["intelligence_ceiling_status".to_string()],
                        raw_value: world_state
                            .audit_trail
                            .intelligence_ceiling_status
                            .to_string(),
                        derivation: "Status of intelligence ceiling enforcement".to_string(),
                    },
                    observable: "intelligence_ceiling_status".to_string(),
                },
            );

            observables.insert(
                "hash_chain_continuity".to_string(),
                TraceableValue {
                    value: world_state.audit_trail.hash_chain_continuity.to_string(),
                    source: SourceTrace {
                        module: "audit".to_string(),
                        function: "AuditTrail::hash_chain_continuity".to_string(),
                        tick: world_state.tick,
                        calculation_path: vec!["hash_chain_continuity".to_string()],
                        raw_value: world_state.audit_trail.hash_chain_continuity.to_string(),
                        derivation: "Hash chain continuity verification status".to_string(),
                    },
                    observable: "hash_chain_continuity".to_string(),
                },
            );
        }

        observables
    }
}

/// Traceable value with source information
#[derive(Debug, Clone, Serialize)]
pub struct TraceableValue {
    pub value: String,
    pub source: SourceTrace,
    pub observable: String,
}

/// Biosphere observation data
#[derive(Debug, Clone, Serialize)]
pub struct BiosphereObservatory {
    pub tick: Tick,
    pub total_species: usize,
    pub total_population: u64,
    pub max_intelligence: f64,
    pub marine_species_count: usize,
    pub terrestrial_species_count: usize,
    pub species: Vec<SpeciesObservation>,
    pub intelligence_ceiling_status: bool,
    pub source_trace: SourceTrace,
}

/// Individual species observation
#[derive(Debug, Clone, Serialize)]
pub struct SpeciesObservation {
    pub species_id: u64,
    pub species_name: String,
    pub population_size: u64,
    pub geographic_range: f64,
    pub intelligence_index: f64,
    pub is_marine: bool,
    pub is_terrestrial: bool,
    pub body_mass_kg: f64,
    pub neural_capacity: u8,
    pub metabolic_rate: u8,
    pub tick: Tick,
    pub source_trace: SourceTrace,
}

/// Verification observatory data
#[derive(Debug, Clone, Serialize)]
pub struct VerificationObservatory {
    pub horizon: VerificationHorizon,
    pub status: String,
    pub observables: crate::long_horizon_verification::VerificationObservables,
    pub time_points: usize,
    pub final_hash: [u8; 32],
    pub determinism_proof: crate::long_horizon_verification::DeterminismProof,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub source_trace: SourceTrace,
}

/// Intervention request
///
/// `action` is the typed intervention that will actually be applied by
/// [`crate::interventions::InterventionExecutor`]. `operation` remains the
/// permission key and audit label; `parameters` is supplementary context for
/// the audit trail only and is never the source of the executed effect.
#[derive(Debug, Clone, Serialize)]
pub struct InterventionRequest {
    pub operation: String,
    pub parameters: HashMap<String, String>,
    pub justification: String,
    pub requester: String,
    pub action: mk_interventions::InterventionAction,
}

/// Intervention result
#[derive(Debug, Clone, Serialize)]
pub struct InterventionResult {
    pub status: InterventionStatus,
    pub message: String,
    pub audit_id: String,
}

/// Intervention status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterventionStatus {
    /// The intervention was applied to world state. `summary` describes the
    /// concrete change that was made.
    Applied {
        summary: String,
    },
    LoggedAndGated,
    Rejected,
    RequiresApproval,
}

impl PermissionManager {
    pub fn new() -> Self {
        let mut permissions = HashMap::new();

        // Default permissions - most interventions are forbidden
        permissions.insert(
            "modify_species".to_string(),
            InterventionPermission::Forbidden,
        );
        permissions.insert(
            "alter_climate".to_string(),
            InterventionPermission::Forbidden,
        );
        permissions.insert(
            "change_intelligence".to_string(),
            InterventionPermission::Forbidden,
        );
        permissions.insert(
            "adjust_evolution".to_string(),
            InterventionPermission::Forbidden,
        );

        Self {
            intervention_permissions: permissions,
            audit_required: true,
            gated_operations: vec![
                "modify_species".to_string(),
                "alter_climate".to_string(),
                "change_intelligence".to_string(),
            ],
        }
    }

    pub fn can_switch_mode(&self, mode: &UIMode) -> bool {
        match mode {
            UIMode::Observe => true,
            UIMode::Scrub => true,
            UIMode::Intervene => true, // Always allowed but gated
        }
    }

    pub fn check_intervention_permission(&self, operation: &str) -> InterventionPermission {
        self.intervention_permissions
            .get(operation)
            .cloned()
            .unwrap_or(InterventionPermission::Forbidden)
    }

    /// Grant `operation` unconditionally.
    ///
    /// Without this (and [`PermissionManager::gate`]) every operation is
    /// `Forbidden` and the `Allowed` branch of
    /// [`TransparentUISystem::attempt_intervention`] is unreachable, which
    /// is exactly the state the 2026-09-18 audit recorded.
    pub fn allow(&mut self, operation: impl Into<String>) -> &mut Self {
        let operation = operation.into();
        self.gated_operations.retain(|op| op != &operation);
        self.intervention_permissions
            .insert(operation, InterventionPermission::Allowed);
        self
    }

    /// Permit `operation` but require audit approval before it applies.
    pub fn gate(&mut self, operation: impl Into<String>, requires_audit: bool) -> &mut Self {
        let operation = operation.into();
        if !self.gated_operations.contains(&operation) {
            self.gated_operations.push(operation.clone());
        }
        self.intervention_permissions
            .insert(operation, InterventionPermission::Gated { requires_audit });
        self
    }

    /// Forbid `operation`.
    pub fn forbid(&mut self, operation: impl Into<String>) -> &mut Self {
        let operation = operation.into();
        self.gated_operations.retain(|op| op != &operation);
        self.intervention_permissions
            .insert(operation, InterventionPermission::Forbidden);
        self
    }

    /// Whether any operation can currently be applied.
    ///
    /// Useful for a UI that needs to tell the operator honestly that nothing
    /// is executable, rather than offering controls that cannot work.
    pub fn any_allowed(&self) -> bool {
        self.intervention_permissions
            .values()
            .any(|permission| matches!(permission, InterventionPermission::Allowed))
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UIAuditTrail {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            permission_violations: Vec::new(),
            traceability_log: Vec::new(),
        }
    }

    pub fn add_traceability_entry(&mut self, entry: TraceabilityEntry) {
        self.traceability_log.push(entry);
    }

    pub fn get_operations_for_tick(&self, tick: Tick) -> Vec<&UIOperation> {
        self.operations
            .iter()
            .filter(|op| op.tick == Some(tick))
            .collect()
    }

    pub fn has_permission_violations(&self) -> bool {
        !self.permission_violations.is_empty()
    }
}

impl Default for UIAuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

/// UI Error types
#[derive(Debug, Clone)]
pub enum UIError {
    NoWorldStateLoaded,
    PermissionDenied(String),
    ObservableNotFound(String),
    WrongMode(String),
    AuditRequired(String),
    InvalidParameter(String),
    /// The intervention was permitted and attempted, but the executor
    /// refused or failed it. Carries the executor's own reason.
    InterventionFailed(String),
}

impl std::fmt::Display for UIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UIError::NoWorldStateLoaded => write!(f, "No world state loaded"),
            UIError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            UIError::ObservableNotFound(name) => write!(f, "Observable '{}' not found", name),
            UIError::WrongMode(msg) => write!(f, "Wrong mode: {}", msg),
            UIError::AuditRequired(msg) => write!(f, "Audit required: {}", msg),
            UIError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            UIError::InterventionFailed(msg) => write!(f, "Intervention failed: {}", msg),
        }
    }
}

impl std::error::Error for UIError {}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::canon::CanonLocked;
    use mk_interventions::{InterventionAction, Location, ResourceType};
    use std::sync::Arc;

    fn request(operation: &str, action: InterventionAction) -> InterventionRequest {
        InterventionRequest {
            operation: operation.to_string(),
            parameters: HashMap::new(),
            justification: "test".to_string(),
            requester: "test-operator".to_string(),
            action,
        }
    }

    fn system() -> TransparentUISystem {
        let mut ui = TransparentUISystem::new(UIConfig::default());
        ui.load_world_state(WorldState::new(Arc::new(CanonLocked::default()), [3u8; 32]))
            .expect("world loads");
        ui.current_mode = UIMode::Intervene;
        ui
    }

    fn co2_action(ppm: f32) -> InterventionAction {
        InterventionAction::ModifyScenario {
            parameter: "climate.co2_concentration_ppm".to_string(),
            value: mk_interventions::ScenarioValue::Float(ppm),
        }
    }

    #[test]
    fn default_permissions_forbid_every_operation() {
        let permissions = PermissionManager::new();
        assert!(!permissions.any_allowed());
        assert_eq!(
            permissions.check_intervention_permission("alter_climate"),
            InterventionPermission::Forbidden
        );
    }

    #[test]
    fn allowed_intervention_actually_mutates_world_state() {
        let mut ui = system();
        ui.permissions.allow("alter_climate");

        let before = ui
            .world_state
            .as_ref()
            .expect("world present")
            .climate_state
            .co2_concentration;

        let result = ui
            .attempt_intervention(request("alter_climate", co2_action(640.0)))
            .expect("allowed intervention succeeds");

        match result.status {
            InterventionStatus::Applied { ref summary } => {
                assert!(
                    summary.contains("640"),
                    "summary should name the value: {summary}"
                );
            }
            other => panic!("expected Applied, got {other:?}"),
        }

        let after = ui
            .world_state
            .as_ref()
            .expect("world present")
            .climate_state
            .co2_concentration;
        assert_ne!(before, after, "the Allowed path must not be a no-op");
        assert!((after - 640.0).abs() < 1e-9);
    }

    #[test]
    fn gated_intervention_is_logged_without_mutating() {
        let mut ui = system();
        ui.permissions.gate("alter_climate", false);

        let before = ui
            .world_state
            .as_ref()
            .expect("world present")
            .climate_state
            .co2_concentration;

        let result = ui
            .attempt_intervention(request("alter_climate", co2_action(900.0)))
            .expect("gated intervention returns Ok");

        assert_eq!(result.status, InterventionStatus::LoggedAndGated);
        assert_eq!(
            ui.world_state
                .as_ref()
                .expect("world present")
                .climate_state
                .co2_concentration,
            before,
            "a gated intervention must not mutate"
        );
    }

    #[test]
    fn audit_required_gate_blocks_before_execution() {
        let mut ui = system();
        ui.permissions.gate("alter_climate", true);
        let err = ui
            .attempt_intervention(request("alter_climate", co2_action(900.0)))
            .expect_err("audit approval is required");
        assert!(matches!(err, UIError::AuditRequired(_)));
    }

    #[test]
    fn forbidden_intervention_is_denied_and_recorded_as_a_violation() {
        let mut ui = system();
        // `alter_climate` is Forbidden by default.
        let err = ui
            .attempt_intervention(request("alter_climate", co2_action(900.0)))
            .expect_err("forbidden operations are denied");
        assert!(matches!(err, UIError::PermissionDenied(_)));
        assert_eq!(ui.audit_trail.permission_violations.len(), 1);
    }

    #[test]
    fn intervention_outside_intervene_mode_is_rejected() {
        let mut ui = system();
        ui.permissions.allow("alter_climate");
        ui.current_mode = UIMode::Observe;
        let err = ui
            .attempt_intervention(request("alter_climate", co2_action(640.0)))
            .expect_err("wrong mode");
        assert!(matches!(err, UIError::WrongMode(_)));
    }

    #[test]
    fn executor_failure_surfaces_as_an_error_not_a_fake_success() {
        let mut ui = system();
        ui.permissions.allow("alter_climate");
        let err = ui
            .attempt_intervention(request(
                "alter_climate",
                InterventionAction::ModifyScenario {
                    parameter: "nonexistent.parameter".to_string(),
                    value: mk_interventions::ScenarioValue::Float(1.0),
                },
            ))
            .expect_err("unknown parameter must fail");
        assert!(matches!(err, UIError::InterventionFailed(_)));
        // And the audit trail records the failure rather than a success.
        assert!(matches!(
            ui.audit_trail.operations.last().map(|op| &op.outcome),
            Some(OperationOutcome::Failure { .. })
        ));
    }

    #[test]
    fn invalid_action_is_rejected_by_the_executor_validation_gate() {
        let mut ui = system();
        ui.permissions.allow("inject");
        let err = ui
            .attempt_intervention(request(
                "inject",
                InterventionAction::InjectResource {
                    resource_type: ResourceType::Water,
                    amount: f32::NAN,
                    location: Location::new(0.0, 0.0),
                },
            ))
            .expect_err("NaN amount must fail validation");
        assert!(matches!(err, UIError::InterventionFailed(_)));
    }

    #[test]
    fn audit_ids_are_unique_within_a_session() {
        let mut ui = system();
        ui.permissions.allow("alter_climate");
        let first = ui
            .attempt_intervention(request("alter_climate", co2_action(500.0)))
            .expect("first succeeds")
            .audit_id;
        let second = ui
            .attempt_intervention(request("alter_climate", co2_action(600.0)))
            .expect("second succeeds")
            .audit_id;
        assert_ne!(
            first, second,
            "two interventions in the same second must not share an audit id"
        );
    }

    #[test]
    fn permission_can_be_revoked_again() {
        let mut ui = system();
        ui.permissions.allow("alter_climate");
        assert!(ui.permissions.any_allowed());
        ui.permissions.forbid("alter_climate");
        assert!(!ui.permissions.any_allowed());
        assert!(ui
            .attempt_intervention(request("alter_climate", co2_action(640.0)))
            .is_err());
    }
}
