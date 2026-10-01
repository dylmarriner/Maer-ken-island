//! Deterministic per-human "thought" text generation for the transparency
//! dashboard's live internal-monologue display.
//!
//! This is template selection over real already-computed per-tick state
//! (the real executed action from [`super::autonomy::AutonomousMind`], the
//! dominant need drive from [`super::needs`], and the dominant emotion from
//! [`super::emotion`], and the presenting symptom of the most active
//! [`super::pathology`]) — never an LLM call, never wall-clock, never live
//! network. The only degree of freedom is which template phrasing to pick
//! for a given state, and that choice is a pure function of the state
//! itself (need/emotion magnitude, [`super::consciousness`]'s
//! `narrative_coherence`), not RNG — so the same human at the same tick
//! always produces the same thought.
//!
//! Reads `human.autonomous_mind.last_action.kind`, not
//! `human.formal_predictive_processing.selected_action`: the monologue
//! narrates what the human actually *did*, and `AutonomousMind` is the
//! system whose choice is executed against the world each tick.
//! `formal_predictive_processing`'s sampled action is an internal
//! active-inference policy over need-relief affordances that informs
//! learning but is not itself executed, so narrating it would describe an
//! intention rather than the action taken.

use super::HumanBeing;
use crate::agents::ActionKind;

/// This human's current dominant unmet survival drive, as `(name, value)`,
/// where `value` is the drive's own 0.0-1.0 intensity (not a derived
/// deficit — [`super::needs::NeedsSnapshot`]'s hunger/thirst/fatigue fields
/// already rise as the underlying reserve depletes).
fn dominant_need(human: &HumanBeing) -> (&'static str, f64) {
    let needs = &human.needs;
    [
        ("hunger", needs.hunger),
        ("thirst", needs.thirst),
        ("fatigue", needs.fatigue),
    ]
    .into_iter()
    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    .expect("fixed 3-element array is never empty")
}

fn action_phrase(action: ActionKind) -> &'static str {
    match action {
        ActionKind::Idle => "I'm just taking a moment",
        ActionKind::Rest => "I could really use some rest",
        ActionKind::SeekWater => "I need to find water",
        ActionKind::SeekFood => "I should find something to eat",
        ActionKind::SeekShelter => "I should get to shelter",
        ActionKind::SocialApproach => "I want to talk to someone",
        ActionKind::Explore => "I feel like looking around",
        ActionKind::Gather => "I should gather some resources",
        ActionKind::Mine => "I should get back to mining",
        ActionKind::Move => "I need to move on from here",
        ActionKind::Build => "I want to keep building",
        ActionKind::Craft => "I should get back to crafting",
        ActionKind::Transfer => "I should hand this off to someone",
        ActionKind::Harm => "I want to hurt you",
        ActionKind::Code => "I want to get back to my work on the computer",
        ActionKind::Intimacy => "I want to be close to someone right now",
        ActionKind::WebSearch => "I want to look something up",
        ActionKind::SendEmail => "I should reach out to someone",
    }
}

/// Outcome clause for the two computer actions, read from already-computed
/// per-tick state (`human.last_action_success` plus whichever of
/// `last_web_search_result`/`last_email_result` this action populated) —
/// same "pure function of already-stepped state" contract as the rest of
/// this module. `None` for every other action or before either action has
/// ever run once.
fn computer_action_outcome_clause(human: &HumanBeing) -> Option<String> {
    match human.autonomous_mind.last_action.kind {
        ActionKind::WebSearch => {
            if human.last_action_success {
                let result = human.last_web_search_result.as_ref()?;
                Some(format!(
                    "I searched for '{}' and found {} results",
                    result.query,
                    result.results.len()
                ))
            } else {
                Some("I tried to search but the computer isn't responding".to_string())
            }
        }
        ActionKind::SendEmail => {
            if human.last_action_success {
                let result = human.last_email_result.as_ref()?;
                Some(format!(
                    "I sent an email to {} about '{}'",
                    result.to, result.subject
                ))
            } else {
                Some("I tried to email someone but the connection failed".to_string())
            }
        }
        _ => None,
    }
}

fn need_clause(need_name: &str, need_value: f64) -> Option<String> {
    if need_value < 0.35 {
        return None;
    }
    let intensity = if need_value >= 0.75 {
        "desperately"
    } else {
        "increasingly"
    };
    Some(format!("I'm {intensity} feeling my {need_name}"))
}

/// The most active pathology's presenting canon symptom, or its name when
/// canon lists no symptoms for it.
fn pathology_clause(human: &HumanBeing) -> Option<String> {
    let pathology = human.pathology.most_active()?;
    Some(match pathology.presenting_symptom() {
        Some(symptom) => format!("I can't shake the {symptom}"),
        None => format!("the {} has a hold on me", pathology.name),
    })
}

fn emotion_clause(emotion_name: &str, emotion_value: f64) -> Option<String> {
    if emotion_value.abs() < 0.25 {
        return None;
    }
    let article = match emotion_name.chars().next() {
        Some('a') | Some('e') | Some('i') | Some('o') | Some('u') => "an",
        _ => "a",
    };
    Some(format!(
        "there's {article} {emotion_name} sitting under all of it"
    ))
}

/// Generate this tick's internal-monologue line for `human`. Pure function
/// of `human`'s own already-stepped state — safe to call any number of
/// times per tick without side effects or drift.
pub fn generate_thought(human: &HumanBeing) -> String {
    let action = human.autonomous_mind.last_action.kind;
    let (need_name, need_value) = dominant_need(human);
    let (emotion_name, emotion_value) = human.emotion.current.dominant();
    let verbose = human.consciousness.narrative_coherence >= 0.5;

    let mut clauses: Vec<String> = Vec::with_capacity(4);
    if let Some(outcome) = computer_action_outcome_clause(human) {
        clauses.push(outcome);
    } else {
        clauses.push(action_phrase(action).to_string());
    }
    if verbose {
        if let Some(need) = need_clause(need_name, need_value) {
            clauses.push(need);
        }
        if let Some(emotion) = emotion_clause(emotion_name, emotion_value) {
            clauses.push(emotion);
        }
        if let Some(pathology) = pathology_clause(human) {
            clauses.push(pathology);
        }
    }

    format!("{}.", clauses.join(", and "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::{BiologicalSex, HumanBeing};

    #[test]
    fn same_state_produces_same_thought() {
        let human = HumanBeing::new("thought-test".to_string(), BiologicalSex::Female);
        let a = generate_thought(&human);
        let b = generate_thought(&human);
        assert_eq!(a, b);
    }

    #[test]
    fn thought_always_ends_with_a_sentence() {
        let human = HumanBeing::new("thought-test-2".to_string(), BiologicalSex::Male);
        let thought = generate_thought(&human);
        assert!(thought.ends_with('.'));
        assert!(!thought.is_empty());
    }

    #[test]
    fn an_active_pathology_voices_its_presenting_symptom() {
        let mut human = HumanBeing::new("thought-test-4".to_string(), BiologicalSex::Female);
        human.consciousness.narrative_coherence = 1.0;
        let pathology = &mut human.pathology.pathologies[0];
        pathology.risk = 0.7;
        pathology.symptoms.emotional = vec!["dread".to_string()];
        let thought = generate_thought(&human);
        assert!(thought.contains("I can't shake the dread"), "{thought}");

        human.pathology.pathologies[0].risk = 0.1;
        assert!(!generate_thought(&human).contains("dread"));
    }

    #[test]
    fn thought_reflects_the_real_executed_action_not_a_frozen_default() {
        // Regression test: this previously read
        // `human.formal_predictive_processing.selected_action`, an internal
        // policy sample rather than the executed action, so the monologue's
        // opening clause could contradict what the human was actually doing
        // via `AutonomousMind`.
        let mut human = HumanBeing::new("thought-test-3".to_string(), BiologicalSex::Female);
        human.autonomous_mind.last_action = crate::agents::AgentAction {
            kind: crate::agents::ActionKind::Explore,
            intensity: 1.0,
        };
        let thought = generate_thought(&human);
        assert!(
            thought.starts_with("I feel like looking around"),
            "expected the monologue to open with the Explore phrase, got: {thought}"
        );
    }
}
