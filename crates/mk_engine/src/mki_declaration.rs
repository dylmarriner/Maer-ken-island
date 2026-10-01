use crate::long_horizon_verification::{
    run_all_verifications, VerificationHorizon, VerificationResults,
};
use crate::transparent_ui::{TransparentUISystem, UIConfig};
/**
 * Phase 8 MK-I Declaration
 *
 * MK-I EXISTS IF AND ONLY IF:
 * - 100 kyr / 1 Myr / 10 Myr runs pass
 * - Determinism holds
 * - Conservation closes within defined bounds
 * - No sapience leakage
 * - Biosphere can go extinct
 * - UI reports everything transparently
 */
use mk_core::canon::{CanonDerived, CanonLocked};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MK-I Declaration Status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MKIStatus {
    NotDeclared,
    Pending,
    Exists,
    Failed(Vec<FailureReason>),
}

/// Failure reasons for MK-I declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FailureReason {
    VerificationFailed {
        horizon: VerificationHorizon,
        reason: String,
    },
    DeterminismBroken {
        reason: String,
    },
    ConservationViolation {
        element: String,
        drift: f64,
        threshold: f64,
    },
    SapienceLeakage {
        max_intelligence: f64,
        ceiling: f64,
    },
    ExtinctionImpossible {
        reason: String,
    },
    UITransparencyViolation {
        reason: String,
    },
}

/// MK-I Declaration System
#[derive(Debug, Clone)]
pub struct MKIDeclaration {
    pub status: MKIStatus,
    pub canon: CanonLocked,
    pub verification_results: HashMap<VerificationHorizon, VerificationResults>,
    pub ui_system: TransparentUISystem,
}

impl MKIDeclaration {
    /// Create new MK-I declaration system
    pub fn new(canon: CanonLocked) -> Self {
        let ui_config = UIConfig::default();
        let ui_system = TransparentUISystem::new(ui_config);

        Self {
            status: MKIStatus::NotDeclared,
            canon,
            verification_results: HashMap::new(),
            ui_system,
        }
    }

    /// Execute complete MK-I declaration process
    pub fn declare_mki(&mut self) -> Result<(), Vec<FailureReason>> {
        self.status = MKIStatus::Pending;

        let all_results = run_all_verifications();
        for result in &all_results {
            self.verification_results
                .insert(result.horizon, result.clone());
        }

        let mut failures = Vec::new();
        if let Err(err) = self.check_verification_compliance() {
            // Iterate in horizon order, not HashMap iteration order
            // (randomized per-process): this loop's push order determines
            // `failures`'s element order, which is stored verbatim into the
            // persisted `self.status` field below.
            let mut ordered_results: Vec<_> = self.verification_results.iter().collect();
            ordered_results.sort_by_key(|(horizon, _)| **horizon);
            for (horizon, result) in ordered_results {
                if !result.success
                    || !result
                        .observables
                        .validate_mandatory_observables()
                        .is_empty()
                {
                    failures.push(FailureReason::VerificationFailed {
                        horizon: *horizon,
                        reason: err.clone(),
                    });
                }
            }
        }
        if let Err(err) = self.check_determinism() {
            failures.push(FailureReason::DeterminismBroken { reason: err });
        }
        if let Err(err) = self.check_conservation_closure() {
            let (element, drift) = self
                .conservation_failure_details()
                .unwrap_or_else(|| ("multiple".to_string(), 0.0));
            failures.push(FailureReason::ConservationViolation {
                element,
                drift,
                threshold: 0.01,
            });
            failures.push(FailureReason::VerificationFailed {
                horizon: VerificationHorizon::Kyr100,
                reason: err,
            });
        }
        if let Err(err) = self.check_sapience_leakage() {
            failures.push(FailureReason::SapienceLeakage {
                max_intelligence: self
                    .verification_results
                    .values()
                    .map(|r| r.observables.intelligence_max)
                    .fold(0.0, f64::max),
                ceiling: crate::biosphere::genetics::PRE_SAPIENT_CEILING,
            });
            failures.push(FailureReason::VerificationFailed {
                horizon: VerificationHorizon::Kyr100,
                reason: err,
            });
        }
        if let Err(err) = self.check_extinction_capability() {
            failures.push(FailureReason::ExtinctionImpossible { reason: err });
        }
        if let Err(err) = self.check_ui_transparency() {
            failures.push(FailureReason::UITransparencyViolation { reason: err });
        }

        if !failures.is_empty() {
            self.status = MKIStatus::Failed(failures.clone());
            return Err(failures);
        }

        self.status = MKIStatus::Exists;
        Ok(())
    }

    /// Get current MKI status
    pub fn get_status(&self) -> &MKIStatus {
        &self.status
    }

    fn ordered_verification_results(&self) -> Vec<(&VerificationHorizon, &VerificationResults)> {
        let mut results: Vec<_> = self.verification_results.iter().collect();
        results.sort_by_key(|(horizon, _)| **horizon);
        results
    }

    /// Check verification compliance
    pub fn check_verification_compliance(&self) -> Result<(), String> {
        let mut issues = Vec::new();
        for (horizon, result) in self.ordered_verification_results() {
            if !result.success {
                issues.push(format!("Verification failed for horizon: {:?}", horizon));
            }
            let observable_issues = result.observables.validate_mandatory_observables();
            if !observable_issues.is_empty() {
                issues.push(format!(
                    "Mandatory observables failed for {:?}: {}",
                    horizon,
                    observable_issues.join(", ")
                ));
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues.join(" | "))
        }
    }

    /// Check determinism
    pub fn check_determinism(&self) -> Result<(), String> {
        // Verify hash chain continuity across all verification results
        for (horizon, result) in self.ordered_verification_results() {
            if !result.observables.hash_chain_continuity {
                return Err(format!("Determinism violation in horizon: {:?}", horizon));
            }
        }
        Ok(())
    }

    /// Check conservation closure
    pub fn check_conservation_closure(&self) -> Result<(), String> {
        let drift_threshold = 0.01;
        for (horizon, result) in self.ordered_verification_results() {
            let drift = &result.observables.conservation_drift;
            if !result.observables.conservation_closure
                || drift.energy_drift.abs() > drift_threshold
                || drift.water_drift.abs() > drift_threshold
                || drift.carbon_drift.abs() > drift_threshold
                || drift.oxygen_drift.abs() > drift_threshold
                || drift.nitrogen_drift.abs() > drift_threshold
                || drift.phosphorus_drift.abs() > drift_threshold
            {
                return Err(format!("Conservation violation in horizon: {:?}", horizon));
            }
        }
        Ok(())
    }

    fn conservation_failure_details(&self) -> Option<(String, f64)> {
        for (_, result) in self.ordered_verification_results() {
            let drift = &result.observables.conservation_drift;
            let candidates = [
                ("energy", drift.energy_drift),
                ("water", drift.water_drift),
                ("carbon", drift.carbon_drift),
                ("oxygen", drift.oxygen_drift),
                ("nitrogen", drift.nitrogen_drift),
                ("phosphorus", drift.phosphorus_drift),
            ];
            if !result.observables.conservation_closure {
                return candidates
                    .iter()
                    .copied()
                    .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                    .map(|(element, value)| (element.to_string(), value));
            }
            if let Some((element, value)) = candidates
                .iter()
                .copied()
                .find(|(_, value)| value.abs() > 0.01)
            {
                return Some((element.to_string(), value));
            }
        }
        None
    }

    /// Check sapience leakage
    pub fn check_sapience_leakage(&self) -> Result<(), String> {
        let ceiling = crate::biosphere::genetics::PRE_SAPIENT_CEILING;
        for (horizon, result) in self.ordered_verification_results() {
            if result.observables.intelligence_max > ceiling {
                return Err(format!(
                    "Sapience leakage: {} > {} in horizon: {:?}",
                    result.observables.intelligence_max, ceiling, horizon
                ));
            }
        }
        Ok(())
    }

    /// Check extinction capability
    pub fn check_extinction_capability(&self) -> Result<(), String> {
        for (horizon, result) in self.ordered_verification_results() {
            if result.observables.extinction_count == 0 {
                return Err(format!("No extinction events in horizon: {:?}", horizon));
            }
        }
        Ok(())
    }

    /// Check UI transparency
    pub fn check_ui_transparency(&self) -> Result<(), String> {
        // Verify UI component has all required observables
        if self.ui_system.get_observables().is_empty() {
            return Err("UI component not providing observables".to_string());
        }
        Ok(())
    }

    /// Generate verification report
    pub fn generate_verification_report(&self, results: &VerificationResults) -> String {
        format!(
            "Verification Report for {:?}\nSuccess: {}\nIntelligence Max: {}\nConservation: {}\nHash Chain: {}\nExtinction Count: {}",
            results.horizon,
            results.success,
            results.observables.intelligence_max,
            results.observables.conservation_closure,
            results.observables.hash_chain_continuity,
            results.observables.extinction_count
        )
    }

    /// Generate canon digest
    pub fn generate_canon_digest(&self) -> String {
        let digest = CanonDerived::from(&self.canon).canon_digest;
        format!("blake3:{}", blake3::Hash::from(digest).to_hex())
    }

    /// Generate governance documentation
    pub fn generate_governance_documentation(&self) -> String {
        format!(
            "MK-I Governance Documentation\nStatus: {:?}\nVerification Horizons: {:?}\nUI Component: Active\nRead-only by default: {}\nExplicit modes required: {}\nFull traceability: {}",
            self.status,
            self.verification_results.keys().collect::<Vec<_>>(),
            self.ui_system.config.read_only_by_default,
            self.ui_system.config.require_explicit_modes,
            self.ui_system.config.full_traceability,
        )
    }

    /// Get declaration artifacts
    pub fn get_artifacts(&self) -> DeclarationArtifacts {
        let verification_summary = self
            .verification_results
            .iter()
            .map(|(h, r)| format!("{:?}: {}", h, r.success))
            .collect::<Vec<_>>()
            .join(", ");

        DeclarationArtifacts {
            status: self.status.clone(),
            verification_summary: if verification_summary.is_empty() {
                "no verification horizons executed".to_string()
            } else {
                verification_summary
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MKIError;

/// Artifacts generated during MK-I declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarationArtifacts {
    pub status: MKIStatus,
    pub verification_summary: String,
}

/// Compliance report for Phase 8
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub passes: bool,
    pub findings: Vec<String>,
}
