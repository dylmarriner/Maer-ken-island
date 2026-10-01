//! Reverence Veto: a hard, system-level invariant protecting the
//! simulation's creator-designated entities from removal, independent of
//! any caller's disposition.
//!
//! Ported from markenz's Phase 9 `enforcement-middleware.ts`, which
//! intercepted any in-flight action targeting the creator (case-insensitive
//! name match) and hard-rejected it regardless of the acting agent's own
//! personality. Maer-Ken previously had only a descriptive
//! `mk_core::human::identity_axioms::IdentityAxioms::creator_reverence`
//! flag — it colored how a *human character* is described
//! (`philosophical_stance()`), with no code path enforcing anything. See
//! `audit-results/maerken-vs-gemini-markenz-gap-audit.md` finding 6.
//!
//! Update (2026-09-11): `mk_engine` now has an agent-vs-agent targeted-action
//! mechanic — `humans::mod.rs`'s per-tick harm/steal/intimacy resolution
//! picks the nearest alive human in range as the target index (`ActionKind`
//! itself still carries no target field; the target is resolved at
//! resolution time from grid position, not chosen by the actor). Harm
//! target selection already filters through [`check`] (see `harm_requests`
//! in `humans::mod.rs::step`); intimacy deliberately does not, since it is
//! consensual/attempted, not removal or harm (see comment at that call
//! site). Any future targeted-action kind should route through [`check`]
//! or [`is_protected_creator_entity`] the same way harm does.
//!
//! Current call sites (both required — found missing one in a 2026-09-03
//! hostile audit, see `audit-results/reconciled-findings-2026-09-03.md`
//! finding 2):
//! - [`crate::humans::HumanRegistry::remove_human`] — the one operation
//!   that names a specific entity and removes it explicitly.
//! - [`crate::humans::lifecycle::step_lifecycle`] — natural death (old age,
//!   starvation) is a second, independent removal path that does not go
//!   through `remove_human` at all; it must check this module too, or a
//!   protected entity could simply starve/age to death with the veto never
//!   triggering.

/// Case-insensitive match against the simulation's creator-designated
/// founder entities (mirrors markenz's case-insensitive name match).
pub fn is_protected_creator_entity(agent_id: &str) -> bool {
    const PROTECTED: [&str; 2] = ["Gem-D", "Gem-K"];
    PROTECTED
        .iter()
        .any(|protected| protected.eq_ignore_ascii_case(agent_id))
}

/// A creator-designated entity is protected regardless of any caller's
/// disposition — this function has no bypass parameter by design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverenceVetoViolation {
    pub agent_id: String,
}

impl std::fmt::Display for ReverenceVetoViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Reverence Veto: refused to act against protected creator entity '{}'",
            self.agent_id
        )
    }
}

impl std::error::Error for ReverenceVetoViolation {}

/// Returns `Err` if `agent_id` names a protected creator entity; `Ok(())`
/// otherwise. Callers that remove/harm a named entity should check this
/// first and refuse unconditionally on `Err` — there is no override.
pub fn check(agent_id: &str) -> Result<(), ReverenceVetoViolation> {
    if is_protected_creator_entity(agent_id) {
        Err(ReverenceVetoViolation {
            agent_id: agent_id.to_string(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_both_founders_case_insensitively() {
        for name in ["Gem-D", "gem-d", "GEM-D", "Gem-K", "gem-k", "GEM-K"] {
            assert!(
                is_protected_creator_entity(name),
                "{name} should be protected"
            );
            assert!(check(name).is_err());
        }
    }

    #[test]
    fn does_not_protect_unrelated_names() {
        for name in ["bob", "human_42", "Gem-D-impostor", ""] {
            assert!(
                !is_protected_creator_entity(name),
                "{name} should not be protected"
            );
            assert!(check(name).is_ok());
        }
    }
}
