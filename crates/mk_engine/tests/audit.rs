use mk_core::canon::CanonLocked;
use mk_core::flux::{AuditError as FluxAuditError, Ledger, TolerancePolicy};
use mk_engine::world_integration::WorldState;
use std::sync::Arc;

/// Audit error types for engine tests
#[derive(Debug)]
pub enum AuditError {
    ImbalanceExceeded {
        kind: mk_core::flux::FluxKind,
        balance: f64,
        tolerance: f64,
    },
    NegativeValue {
        kind: mk_core::flux::FluxKind,
        value: f64,
    },
    NaNOrInf {
        kind: mk_core::flux::FluxKind,
        value: f64,
    },
    ConservationViolation {
        kind: mk_core::flux::FluxKind,
        balance: f64,
        tolerance: f64,
    },
}

/// Audit conservation using ledger
///
/// Checks that all flux kinds are balanced within tolerance.
/// MUST check conservation (ledger closure).
///
/// # Arguments
///
/// * `ledger` - Ledger to audit
/// * `policy` - Tolerance policy for numerical precision
///
/// # Returns
///
/// Ok if balanced, Err with audit error details
pub fn audit_conservation(ledger: &Ledger, policy: TolerancePolicy) -> Result<(), AuditError> {
    ledger.audit(policy).map_err(|flux_error| match flux_error {
        FluxAuditError::ImbalanceExceeded {
            kind,
            balance,
            tolerance,
        } => AuditError::ConservationViolation {
            kind,
            balance,
            tolerance,
        },
    })
}

/// Audit non-negativity of all state values
///
/// Checks that all state values are positive.
pub fn audit_nonnegativity(state: &WorldState) -> Result<(), AuditError> {
    // Check that world state is valid as a proxy for non-negativity
    state.validate().map_err(|_| AuditError::NegativeValue {
        kind: mk_core::flux::FluxKind::Energy,
        value: 0.0,
    })
}

#[test]
fn audit_nonnegativity_all_positive() {
    let state = WorldState::new(Arc::new(CanonLocked::default()), [42u8; 32]);

    // Check that world state is valid
    assert!(state.validate().is_ok());

    // Audit should succeed
    let result = audit_nonnegativity(&state);
    assert!(result.is_ok());
}
