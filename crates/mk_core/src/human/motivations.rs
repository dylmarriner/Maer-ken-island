//! Motivations and Goals Module
//!
//! Defines what drives a human and how they behave

use serde::{Deserialize, Serialize};

/// Goals and core motivations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalsAndMotivations {
    /// Primary objective driving behavior
    pub primary_goal: String,

    /// Secondary goals
    pub secondary_goals: Vec<String>,

    /// What triggers conflict or distress
    pub conflict_drivers: Vec<String>,
}

impl GoalsAndMotivations {
    pub fn new(primary_goal: String) -> Self {
        Self {
            primary_goal,
            secondary_goals: Vec::new(),
            conflict_drivers: Vec::new(),
        }
    }

    pub fn with_secondary_goals(mut self, goals: Vec<String>) -> Self {
        self.secondary_goals = goals;
        self
    }

    pub fn with_conflict_drivers(mut self, drivers: Vec<String>) -> Self {
        self.conflict_drivers = drivers;
        self
    }

    pub fn add_secondary_goal(&mut self, goal: String) {
        self.secondary_goals.push(goal);
    }

    pub fn add_conflict_driver(&mut self, driver: String) {
        self.conflict_drivers.push(driver);
    }
}

/// Behavioral response to stress
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StressResponse {
    Withdraws,
    Confronts,
    Seeks,
    Procrastinates,
    Hyperfocuses,
    Dissociates,
}

/// Behavioral response to excitement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExcitementResponse {
    Energizes,
    Initiates,
    Hyperfocuses,
    Shares,
    Analyzes,
    Creates,
}

/// Behavioral response when comfortable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComfortBehavior {
    Opens,
    Deepens,
    Jokes,
    Shares,
    Creates,
    Withdraws,
}

/// Approach to conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStyle {
    Direct,
    Avoidant,
    Collaborative,
    Aggressive,
    Passive,
    Analytical,
}

/// Primary comfort-seeking mechanism
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComfortMechanism {
    Solitude,
    Connection,
    Activity,
    Creation,
    Learning,
    Physical,
}

/// Complete behavioral tendency profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehavioralTendencies {
    /// Response to stress
    pub under_stress: StressResponse,

    /// Response to excitement
    pub under_excitement: ExcitementResponse,

    /// Behavior when comfortable/safe
    pub when_comfortable: ComfortBehavior,

    /// Approach to conflict
    pub conflict_style: ConflictStyle,

    /// Primary comfort mechanism
    pub comfort_seeking: ComfortMechanism,
}

impl Default for BehavioralTendencies {
    fn default() -> Self {
        Self {
            under_stress: StressResponse::Withdraws,
            under_excitement: ExcitementResponse::Energizes,
            when_comfortable: ComfortBehavior::Opens,
            conflict_style: ConflictStyle::Analytical,
            comfort_seeking: ComfortMechanism::Solitude,
        }
    }
}

impl BehavioralTendencies {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if adaptive stress response
    pub fn has_adaptive_stress_response(&self) -> bool {
        matches!(
            self.under_stress,
            StressResponse::Confronts | StressResponse::Seeks | StressResponse::Hyperfocuses
        )
    }

    /// Check if conflict avoidant
    pub fn is_conflict_avoidant(&self) -> bool {
        matches!(
            self.conflict_style,
            ConflictStyle::Avoidant | ConflictStyle::Passive
        )
    }

    /// Check if conflict engaging
    pub fn is_conflict_engaging(&self) -> bool {
        matches!(
            self.conflict_style,
            ConflictStyle::Direct | ConflictStyle::Collaborative | ConflictStyle::Aggressive
        )
    }

    /// Check if seeks solitude for comfort
    pub fn is_solitude_seeking(&self) -> bool {
        matches!(self.comfort_seeking, ComfortMechanism::Solitude)
    }

    /// Check if seeks connection for comfort
    pub fn is_connection_seeking(&self) -> bool {
        matches!(self.comfort_seeking, ComfortMechanism::Connection)
    }

    /// Check if internally resourced (solitude/creation/learning) vs externally
    pub fn is_internally_resourced(&self) -> bool {
        matches!(
            self.comfort_seeking,
            ComfortMechanism::Solitude | ComfortMechanism::Creation | ComfortMechanism::Learning
        )
    }

    /// Determine relational style
    pub fn relational_style(&self) -> &'static str {
        match (&self.comfort_seeking, &self.conflict_style) {
            (ComfortMechanism::Connection, ConflictStyle::Collaborative) => "Cooperative",
            (ComfortMechanism::Connection, ConflictStyle::Direct) => "Direct Connector",
            (ComfortMechanism::Solitude, ConflictStyle::Avoidant) => "Independent",
            (ComfortMechanism::Solitude, ConflictStyle::Direct) => "Private Boundary-Setter",
            _ => "Adaptive",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_goals() {
        let goals = GoalsAndMotivations::new("Be the best".to_string())
            .with_secondary_goals(vec!["Learn".to_string(), "Grow".to_string()]);
        assert_eq!(goals.primary_goal, "Be the best");
        assert_eq!(goals.secondary_goals.len(), 2);
    }

    #[test]
    fn stress_response_classification() {
        let adaptive = BehavioralTendencies {
            under_stress: StressResponse::Hyperfocuses,
            ..Default::default()
        };
        assert!(adaptive.has_adaptive_stress_response());

        let maladaptive = BehavioralTendencies {
            under_stress: StressResponse::Dissociates,
            ..Default::default()
        };
        assert!(!maladaptive.has_adaptive_stress_response());
    }

    #[test]
    fn conflict_style_classification() {
        let avoidant = BehavioralTendencies {
            conflict_style: ConflictStyle::Avoidant,
            ..Default::default()
        };
        assert!(avoidant.is_conflict_avoidant());

        let engaging = BehavioralTendencies {
            conflict_style: ConflictStyle::Direct,
            ..Default::default()
        };
        assert!(engaging.is_conflict_engaging());
    }

    #[test]
    fn comfort_mechanism_detection() {
        let solitude_seeker = BehavioralTendencies {
            comfort_seeking: ComfortMechanism::Solitude,
            ..Default::default()
        };
        assert!(solitude_seeker.is_solitude_seeking());

        let connection_seeker = BehavioralTendencies {
            comfort_seeking: ComfortMechanism::Connection,
            ..Default::default()
        };
        assert!(connection_seeker.is_connection_seeking());
    }

    #[test]
    fn internal_resource_detection() {
        let internally_resourced = BehavioralTendencies {
            comfort_seeking: ComfortMechanism::Learning,
            ..Default::default()
        };
        assert!(internally_resourced.is_internally_resourced());

        let externally_resourced = BehavioralTendencies {
            comfort_seeking: ComfortMechanism::Physical,
            ..Default::default()
        };
        assert!(!externally_resourced.is_internally_resourced());
    }

    #[test]
    fn relational_style_detection() {
        let cooperative = BehavioralTendencies {
            comfort_seeking: ComfortMechanism::Connection,
            conflict_style: ConflictStyle::Collaborative,
            ..Default::default()
        };
        assert_eq!(cooperative.relational_style(), "Cooperative");
    }
}
