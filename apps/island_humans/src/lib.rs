use chrono::{DateTime, Utc};
use mk_core::human::{astrology::GeoCoordinates, BiologicalSex, HumanStatus};
use mk_engine::humans::{HumanBeing, HumanSystem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanSummary {
    pub agent_id: String,
    pub human_id: String,
    pub biological_sex: String,
    pub status: String,
    pub age_years: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateHumanError {
    DuplicateAgentId(String),
}

pub struct IslandHumanPopulation {
    system: HumanSystem,
}

impl Default for IslandHumanPopulation {
    fn default() -> Self {
        Self::with_founders()
    }
}

impl IslandHumanPopulation {
    pub fn with_founders() -> Self {
        let mut system = HumanSystem::new();
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_d_founder());
        system
            .registry
            .add_human_no_storage(HumanBeing::gem_k_founder());
        Self { system }
    }

    pub fn empty() -> Self {
        Self {
            system: HumanSystem::new(),
        }
    }

    pub fn create_human(
        &mut self,
        agent_id: impl Into<String>,
        biological_sex: BiologicalSex,
        birth: DateTime<Utc>,
        birthplace: GeoCoordinates,
        location: &str,
    ) -> Result<HumanSummary, CreateHumanError> {
        let agent_id = agent_id.into();
        if self.system.registry.get_human(&agent_id).is_some() {
            return Err(CreateHumanError::DuplicateAgentId(agent_id));
        }
        let human = HumanBeing::new_born_at(
            agent_id.clone(),
            biological_sex,
            birth,
            birthplace,
            location,
        );
        let summary = summarize(&human);
        self.system.registry.add_human_no_storage(human);
        Ok(summary)
    }

    pub fn get(&self, agent_id: &str) -> Option<&HumanBeing> {
        self.system.registry.get_human(agent_id)
    }

    pub fn len(&self) -> usize {
        self.system.registry.count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn summaries(&self) -> Vec<HumanSummary> {
        self.system.registry.iter().map(summarize).collect()
    }

    pub fn system(&self) -> &HumanSystem {
        &self.system
    }

    pub fn system_mut(&mut self) -> &mut HumanSystem {
        &mut self.system
    }
}

pub fn summarize(human: &HumanBeing) -> HumanSummary {
    HumanSummary {
        agent_id: human.agent_id().to_string(),
        human_id: human.profile.human_id.to_string(),
        biological_sex: match human.biological_sex() {
            BiologicalSex::Male => "male",
            BiologicalSex::Female => "female",
            BiologicalSex::Neutral => "neutral",
        }
        .to_string(),
        status: match human.profile.status {
            HumanStatus::Alive => "alive",
            HumanStatus::Dead => "dead",
            HumanStatus::Dormant => "dormant",
        }
        .to_string(),
        age_years: human.development.age_years,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn birth() -> DateTime<Utc> {
        "2000-01-02T03:04:05Z".parse().unwrap()
    }

    #[test]
    fn canonical_founders_are_recreated() {
        let population = IslandHumanPopulation::with_founders();
        assert_eq!(population.len(), 2);
        assert!(population.get("Gem-D").is_some());
        assert!(population.get("Gem-K").is_some());
    }

    #[test]
    fn generated_human_is_deterministic_for_same_inputs() {
        let coords = GeoCoordinates {
            latitude: -36.85,
            longitude: 174.76,
        };
        let a = HumanBeing::new_born_at(
            "islander-001".into(),
            BiologicalSex::Female,
            birth(),
            coords,
            "Maer-Ken Island",
        );
        let b = HumanBeing::new_born_at(
            "islander-001".into(),
            BiologicalSex::Female,
            birth(),
            coords,
            "Maer-Ken Island",
        );
        assert_eq!(a.profile.human_id, b.profile.human_id);
        assert_eq!(
            serde_json::to_value(&a.profile).unwrap(),
            serde_json::to_value(&b.profile).unwrap()
        );
    }

    #[test]
    fn duplicate_agent_ids_are_rejected() {
        let mut population = IslandHumanPopulation::empty();
        let coords = GeoCoordinates {
            latitude: 0.0,
            longitude: 0.0,
        };
        population
            .create_human(
                "same-person",
                BiologicalSex::Male,
                birth(),
                coords,
                "Maer-Ken Island",
            )
            .unwrap();
        assert_eq!(
            population.create_human(
                "same-person",
                BiologicalSex::Male,
                birth(),
                coords,
                "Maer-Ken Island",
            ),
            Err(CreateHumanError::DuplicateAgentId("same-person".into()))
        );
    }
}
