//! Deterministic inter-human conversation generation for the transparency
//! dashboard's dialogue feed.
//!
//! Like [`super::thought`], this is composition over real already-computed
//! per-tick state — never an LLM call. Each line says something true about
//! its speaker:
//! - the opener greets in a way that fits the relationship, recalls what the
//!   listener said last time these two spoke (from the speaker's own
//!   persistent `conversation_history`), then voices the speaker's current
//!   internal monologue ([`super::thought::generate_thought`]);
//! - the reply reacts to the opener's emotional tone, scaled by the
//!   listener's real empathy, then voices the listener's own monologue.
//!
//! The one genuine degree of freedom (which of several equally valid
//! phrasings to use for a greeting, question or acknowledgement) is
//! resolved via the seeded keyed-stream [`RngRegistry`] under
//! [`SubsystemId::Dialogue`], so a conversation between the same two humans
//! at the same tick is always identical across runs.
//!
//! Each [`ConversationEvent`] is appended to both participants' own
//! persistent history by [`super::HumanSystem::step_dialogue`], because
//! humans remember what the other says — and the next opener draws on it.

use super::HumanBeing;
use mk_core::human::HumanId;
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

/// The kind of relationship between the two participants, used to pick a
/// relationship-appropriate register. Derived by the caller from real
/// lineage data (`birth_records`) or the founder identities, not guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationRelationship {
    /// Gem-D and Gem-K — the founding pair every other human descends from.
    Founders,
    ParentChild,
    Siblings,
    /// Any other pairing (unrelated, distant kin, etc.) — the generic tone.
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueLine {
    pub speaker_id: HumanId,
    pub speaker_name: String,
    pub text: String,
    /// What the speaker said about themselves in this line (their voiced
    /// monologue), without greeting or quotation — what the listener
    /// remembers of it.
    #[serde(default)]
    pub gist: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEvent {
    /// When they started talking.
    ///
    /// Not when this line was said: an exchange that carries on across
    /// ticks keeps the tick it opened on, which is what makes it one
    /// conversation rather than one per minute. See `last_tick`.
    pub tick: Tick,
    pub participant_a_id: HumanId,
    pub participant_b_id: HumanId,
    pub relationship: ConversationRelationship,
    pub lines: Vec<DialogueLine>,
    /// When the most recent line was said.
    ///
    /// Equal to `tick` for an exchange that lasted a single tick, and
    /// later for one still going. `HumanSystem::step_dialogue` uses it
    /// to decide whether the pair are continuing or starting again:
    /// talking on the tick after the last line continues this
    /// conversation, and any longer gap begins another.
    ///
    /// `#[serde(default)]` because snapshots written before conversations
    /// could span ticks have no such field; those load with `last_tick`
    /// at 0, which simply means the first exchange after a load starts a
    /// new conversation rather than joining a pre-existing one. That is
    /// the right answer for a world that was paused and resumed anyway.
    #[serde(default)]
    pub last_tick: Tick,
}

/// Emotions that read as a good mood when dominant.
const POSITIVE_EMOTIONS: &[&str] = &[
    "joy",
    "love",
    "pride",
    "awe",
    "hope",
    "relief",
    "gratitude",
    "admiration",
    "triumph",
    "contentment",
    "curiosity",
];

/// Below this dominant-emotion intensity the speaker sounds even-keeled.
const NOTICEABLE_EMOTION: f64 = 0.25;

/// Empathy level at or above which a listener responds warmly to distress.
const EMPATHIC: f64 = 0.5;

fn greeting_templates(relationship: ConversationRelationship) -> &'static [&'static str] {
    match relationship {
        ConversationRelationship::Founders => {
            &["{name}, love.", "{name}.", "There you are, {name}."]
        }
        ConversationRelationship::ParentChild => {
            &["{name}, come here a moment.", "{name}.", "Hey, {name}."]
        }
        ConversationRelationship::Siblings => &["Hey, {name}.", "{name}!", "Oi, {name}."],
        ConversationRelationship::Other => {
            &["Hello, {name}.", "Good to see you, {name}.", "{name}."]
        }
    }
}

fn question_templates(relationship: ConversationRelationship) -> &'static [&'static str] {
    match relationship {
        ConversationRelationship::Founders => &["How are you holding up?", "How are you, really?"],
        ConversationRelationship::ParentChild => {
            &["How are you doing?", "Tell me how you're feeling."]
        }
        ConversationRelationship::Siblings => &["You alright?", "What about you?"],
        ConversationRelationship::Other => &["How's it going?", "How are you keeping?"],
    }
}

fn pick<'a>(rng: &RngRegistry, key: RngKey, options: &'a [&'a str]) -> &'a str {
    let index = (rng.gen_u32(key) as usize) % options.len();
    options[index]
}

/// Symmetric per-pair RNG chunk: identical regardless of argument order, so
/// the conversation is deterministic no matter which participant is passed
/// as `a` vs `b`.
fn pair_chunk(a: HumanId, b: HumanId) -> u32 {
    let (lo, hi) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
    ((lo & 0xFFFF) as u32) ^ (((hi & 0xFFFF) as u32) << 16)
}

/// Emotional tone of `human` right now: +1 good, -1 bad, 0 even.
fn tone(human: &HumanBeing) -> i8 {
    let (emotion, intensity) = human.emotion.current.dominant();
    if intensity.abs() < NOTICEABLE_EMOTION {
        0
    } else if POSITIVE_EMOTIONS.contains(&emotion) && intensity > 0.0 {
        1
    } else {
        -1
    }
}

/// What `listener` last said to `speaker`, as the speaker remembers it.
fn remembered_gist<'a>(speaker: &'a HumanBeing, listener: &HumanBeing) -> Option<&'a str> {
    let listener_id = listener.profile.human_id;
    speaker
        .conversation_history
        .iter()
        .rev()
        .flat_map(|event| event.lines.iter().rev())
        .find(|line| line.speaker_id == listener_id && !line.gist.is_empty())
        .map(|line| line.gist.as_str())
}

/// Generate one two-line exchange between `a` and `b` at `tick`. Pure
/// function of `a`/`b`'s own already-stepped state (including what each
/// remembers of earlier conversations) plus the seeded registry — the same
/// inputs always produce the same conversation.
pub fn generate_conversation(
    a: &HumanBeing,
    b: &HumanBeing,
    relationship: ConversationRelationship,
    rng_registry: &RngRegistry,
    tick: Tick,
) -> ConversationEvent {
    let chunk = pair_chunk(a.profile.human_id, b.profile.human_id);
    let key = |epoch: u32| RngKey::new(SubsystemId::Dialogue, chunk, epoch, tick);

    // Opener: greeting, recollection, monologue, question.
    let a_gist = super::thought::generate_thought(a);
    let mut opener = vec![pick(rng_registry, key(0), greeting_templates(relationship))
        .replace("{name}", b.agent_id())];
    if let Some(recalled) = remembered_gist(a, b) {
        // Quoted as said: the gist is the listener's own first person.
        opener.push(format!("Last time you told me, \"{recalled}\""));
    }
    opener.push(a_gist.clone());
    opener.push(pick(rng_registry, key(1), question_templates(relationship)).to_string());

    // Reply: a reaction to how the opener sounded, then b's own monologue.
    let reaction = match tone(a) {
        -1 if b.social_cognition.empathy_level >= EMPATHIC => {
            format!("I'm sorry, {}. I'm here.", a.agent_id())
        }
        -1 => "We all have days like that.".to_string(),
        1 => "Good to hear it.".to_string(),
        _ => pick(
            rng_registry,
            key(2),
            &["Same as ever.", "Well enough.", "Mm."],
        )
        .to_string(),
    };
    let b_gist = super::thought::generate_thought(b);

    ConversationEvent {
        tick,
        // A freshly generated exchange begins and ends on this tick;
        // `step_dialogue` moves `last_tick` forward if the pair carry on.
        last_tick: tick,
        participant_a_id: a.profile.human_id,
        participant_b_id: b.profile.human_id,
        relationship,
        lines: vec![
            DialogueLine {
                speaker_id: a.profile.human_id,
                speaker_name: a.agent_id().to_string(),
                text: opener.join(" "),
                gist: a_gist,
            },
            DialogueLine {
                speaker_id: b.profile.human_id,
                speaker_name: b.agent_id().to_string(),
                text: format!("{reaction} {b_gist}"),
                gist: b_gist,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::BiologicalSex;

    fn registry() -> RngRegistry {
        RngRegistry::new([11u8; 32])
    }

    #[test]
    fn same_pair_same_tick_produces_same_conversation() {
        let a = HumanBeing::new("dialogue-a".to_string(), BiologicalSex::Female);
        let b = HumanBeing::new("dialogue-b".to_string(), BiologicalSex::Male);
        let rng = registry();

        let first = generate_conversation(&a, &b, ConversationRelationship::Other, &rng, 42);
        let second = generate_conversation(&a, &b, ConversationRelationship::Other, &rng, 42);

        assert_eq!(first.lines[0].text, second.lines[0].text);
        assert_eq!(first.lines[1].text, second.lines[1].text);
    }

    #[test]
    fn lines_voice_each_speakers_own_monologue() {
        let a = HumanBeing::new("dialogue-e".to_string(), BiologicalSex::Female);
        let b = HumanBeing::new("dialogue-f".to_string(), BiologicalSex::Male);
        let event = generate_conversation(&a, &b, ConversationRelationship::Other, &registry(), 3);

        let a_thought = super::super::thought::generate_thought(&a);
        let b_thought = super::super::thought::generate_thought(&b);
        assert!(
            event.lines[0].text.contains("dialogue-f"),
            "greets the listener"
        );
        assert!(event.lines[0].text.contains(&a_thought));
        assert_eq!(event.lines[0].gist, a_thought);
        assert!(event.lines[1].text.ends_with(&b_thought));
        assert_eq!(event.lines[1].gist, b_thought);
    }

    #[test]
    fn opener_recalls_what_the_listener_said_last_time() {
        let mut a = HumanBeing::new("dialogue-g".to_string(), BiologicalSex::Female);
        let b = HumanBeing::new("dialogue-h".to_string(), BiologicalSex::Male);
        let rng = registry();
        let mut earlier = generate_conversation(&a, &b, ConversationRelationship::Other, &rng, 1);
        earlier.lines[1].gist = "I found water by the ridge.".to_string();
        a.conversation_history.push_back(earlier);

        let later = generate_conversation(&a, &b, ConversationRelationship::Other, &rng, 2);

        assert!(later.lines[0]
            .text
            .contains("Last time you told me, \"I found water by the ridge.\""));
    }

    #[test]
    fn an_empathic_listener_answers_distress_warmly() {
        let mut a = HumanBeing::new("dialogue-i".to_string(), BiologicalSex::Female);
        let mut b = HumanBeing::new("dialogue-j".to_string(), BiologicalSex::Male);
        a.emotion.current.despair = 0.95;
        b.social_cognition.empathy_level = 0.9;
        let rng = registry();

        let warm = generate_conversation(&a, &b, ConversationRelationship::Siblings, &rng, 5);
        assert!(warm.lines[1].text.starts_with("I'm sorry, dialogue-i."));

        b.social_cognition.empathy_level = 0.1;
        let cool = generate_conversation(&a, &b, ConversationRelationship::Siblings, &rng, 5);
        assert!(cool.lines[1]
            .text
            .starts_with("We all have days like that."));
    }

    #[test]
    fn argument_order_does_not_change_the_pair_chunk() {
        let a = HumanBeing::new("dialogue-c".to_string(), BiologicalSex::Female);
        let b = HumanBeing::new("dialogue-d".to_string(), BiologicalSex::Male);
        assert_eq!(
            pair_chunk(a.profile.human_id, b.profile.human_id),
            pair_chunk(b.profile.human_id, a.profile.human_id)
        );
    }
}
