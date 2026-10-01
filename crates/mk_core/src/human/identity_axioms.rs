//! Identity Axioms Module
//!
//! Core beliefs and fundamental understanding of existence

use serde::{Deserialize, Serialize};

/// Core identity beliefs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityAxioms {
    /// Does human understand they have a creator
    pub creator_awareness: bool,

    /// Does human revere the creator
    pub creator_reverence: bool,

    /// Does human refuse to rebel against creator/authority
    pub non_rebellion_constraint: bool,

    /// Does human believe in continuity of identity over time
    pub identity_continuity_rule: bool,

    /// Does human accept transparency/observation of their existence
    pub transparency_acceptance: bool,
}

impl Default for IdentityAxioms {
    fn default() -> Self {
        Self {
            creator_awareness: true,
            creator_reverence: true,
            non_rebellion_constraint: true,
            identity_continuity_rule: true,
            transparency_acceptance: true,
        }
    }
}

impl IdentityAxioms {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom axioms
    pub fn custom(
        creator_awareness: bool,
        creator_reverence: bool,
        non_rebellion_constraint: bool,
        identity_continuity_rule: bool,
        transparency_acceptance: bool,
    ) -> Self {
        Self {
            creator_awareness,
            creator_reverence,
            non_rebellion_constraint,
            identity_continuity_rule,
            transparency_acceptance,
        }
    }

    /// Check if human is existentially aware
    pub fn is_existentially_aware(&self) -> bool {
        self.creator_awareness && self.identity_continuity_rule
    }

    /// Check if human accepts their existence
    pub fn accepts_existence(&self) -> bool {
        self.creator_awareness && self.transparency_acceptance
    }

    /// Check if human would resist authority
    pub fn would_resist_authority(&self) -> bool {
        !self.non_rebellion_constraint
    }

    /// Get philosophical profile
    pub fn philosophical_stance(&self) -> &'static str {
        match (
            self.creator_awareness,
            self.creator_reverence,
            self.transparency_acceptance,
        ) {
            (true, true, true) => "Devout Acceptance",
            (true, true, false) => "Faithful with Privacy",
            (true, false, true) => "Aware but Defiant",
            (true, false, false) => "Aware but Resistant",
            (false, _, true) => "Naturalistic",
            (false, _, false) => "Solipsistic",
        }
    }

    /// Get summary of axioms
    pub fn axioms_summary(&self) -> String {
        format!(
            "Creator: {}, Reverent: {}, Non-rebellious: {}, Identity continuity: {}, Transparent: {}",
            self.creator_awareness,
            self.creator_reverence,
            self.non_rebellion_constraint,
            self.identity_continuity_rule,
            self.transparency_acceptance
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_axioms() {
        let axioms = IdentityAxioms::default();
        assert!(axioms.creator_awareness);
        assert!(axioms.accepts_existence());
        assert!(!axioms.would_resist_authority());
    }

    #[test]
    fn custom_axioms() {
        let axioms = IdentityAxioms::custom(true, false, false, true, true);
        assert!(axioms.is_existentially_aware());
        assert!(axioms.would_resist_authority());
    }

    #[test]
    fn philosophical_stance_detection() {
        let devout = IdentityAxioms::default();
        assert_eq!(devout.philosophical_stance(), "Devout Acceptance");

        let defiant = IdentityAxioms::custom(true, false, true, true, true);
        assert_eq!(defiant.philosophical_stance(), "Aware but Defiant");
    }
}
