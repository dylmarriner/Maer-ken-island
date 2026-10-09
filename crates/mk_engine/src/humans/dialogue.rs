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
use crate::agents::ActionKind;
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

/// Ways of saying out loud what `action` was, for a speaker who is not
/// reading their own inner voice aloud.
///
/// The first of each set is the monologue's own wording, so nothing is
/// lost; the rest are the same thing said as people say it. This is
/// where D36 is fixed, and it is phrasing rather than content: the state
/// behind every one of these is identical, because inventing a *different
/// state* to get variety would be inventing what the island did.
fn spoken_action(action: ActionKind) -> &'static [&'static str] {
    match action {
        ActionKind::Idle => &[
            "I'm just taking a moment",
            "I'm having a breather",
            "I'm not doing much, to be honest",
            "I'm just standing here a while",
        ],
        ActionKind::Rest => &[
            "I could really use some rest",
            "I'm worn out",
            "I need to sit down for a bit",
            "I've not got much left in me today",
        ],
        ActionKind::SeekWater => &[
            "I need to find water",
            "I'm going to look for water",
            "I want a drink, properly",
            "I should fill something before it gets late",
        ],
        ActionKind::SeekFood => &[
            "I should find something to eat",
            "I'm going to see what there is to eat",
            "I want to get something in me",
            "I could do with a meal",
        ],
        ActionKind::SeekShelter => &[
            "I should get to shelter",
            "I want to be under a roof",
            "I'm heading in before this turns",
            "I'd rather not be out in this",
        ],
        ActionKind::SocialApproach => &[
            "I want to talk to someone",
            "I was hoping to find you",
            "I've been wanting a word with somebody",
            "I came looking for company",
        ],
        ActionKind::Explore => &[
            "I feel like looking around",
            "I want to see what's out there",
            "I've a mind to wander a bit",
            "I keep wondering what's past the ridge",
        ],
        ActionKind::Gather => &[
            "I should gather some resources",
            "I'm collecting what I can",
            "I want to bring a load back",
            "there's plenty worth picking up out there",
        ],
        ActionKind::Mine => &[
            "I should get back to mining",
            "I've stone to get out",
            "I want to finish what I started at the face",
            "there's more down there than I've taken",
        ],
        ActionKind::Move => &[
            "I need to move on from here",
            "I'm not staying here",
            "I want to be somewhere else",
            "it's time I shifted",
        ],
        ActionKind::Build => &[
            "I want to keep building",
            "I've work to finish on the build",
            "I want to see it standing",
            "there's more to put up before I stop",
        ],
        ActionKind::Craft => &[
            "I should get back to crafting",
            "I've something half-made waiting",
            "I want to finish what's on the bench",
            "I'd like to make something properly for once",
        ],
        ActionKind::Transfer => &[
            "I should hand this off to someone",
            "I've something for you, if you want it",
            "I'd rather this went to somebody who needs it",
            "I'm passing this on",
        ],
        ActionKind::Harm => &[
            "I want to hurt you",
            "I'm not going to pretend I'm calm about this",
            "something in me wants to swing",
            "stay back from me",
        ],
        ActionKind::Code => &[
            "I want to get back to my work on the computer",
            "I've something running I want to finish",
            "the machine's waiting on me",
            "I left it half-written and it's been nagging at me",
        ],
        ActionKind::Intimacy => &[
            "I want to be close to someone right now",
            "I'd like you near me",
            "I don't want to be on my own tonight",
            "come here a minute",
        ],
        ActionKind::WebSearch => &[
            "I want to look something up",
            "there's something I want to find out",
            "I've a question I can't answer myself",
            "I keep meaning to search for it",
        ],
        ActionKind::SendEmail => &[
            "I should reach out to someone",
            "I owe somebody a message",
            "I've been meaning to write",
            "there's a letter I keep not sending",
        ],
    }
}

/// How a need is said aloud, as against how it is thought.
///
/// The monologue says "I'm increasingly feeling my hunger", which is a
/// sentence nobody says. These are. They keep *which* need it is,
/// because dropping that to get variety would lose what the speaker
/// actually told the listener -- the point is to vary the wording, not
/// to say less.
fn spoken_need(name: &str, value: f64) -> &'static [&'static str] {
    let pressing = value >= 0.75;
    match (name, pressing) {
        ("hunger", false) => &[
            "I'm getting hungry",
            "I could eat",
            "my stomach's starting to talk to me",
        ],
        ("hunger", true) => &[
            "I'm starving, honestly",
            "I can't think past how hungry I am",
            "I need to eat, properly, soon",
        ],
        ("thirst", false) => &["I'm thirsty", "I could do with a drink", "my mouth's dry"],
        ("thirst", true) => &[
            "I'm parched",
            "I need water, not in a minute",
            "I can't get past how dry I am",
        ],
        ("fatigue", false) => &["I'm tired", "I'm flagging a bit", "I've been going a while"],
        (_, true) => &[
            "I'm exhausted",
            "I'm dead on my feet",
            "I can't keep my eyes open much longer",
        ],
        (_, false) => &[
            "it's starting to get to me",
            "I'm feeling it",
            "it's been building all day",
        ],
    }
}

/// One spoken line from `mind`, chosen with the pair's own seeded draw.
///
/// This is deliberately *not* `thought::generate_thought`. A spoken line
/// used to be the internal monologue verbatim, and because that function
/// is contractually a pure function of state with no RNG -- the dashboard
/// needs a monologue that does not flicker -- the same state forced the
/// same sentence. A founder's day of 2,042 lines held 32 distinct ones,
/// each said about sixty-four times. That was D36.
///
/// The determinism that mattered is kept and nothing is weakened: the
/// draw comes from the same seeded `RngRegistry` and the same per-pair,
/// per-tick key the greetings and questions have always used, so the same
/// two people at the same tick still say exactly the same thing. What
/// changes is that two ticks with identical state no longer have to.
fn said_aloud(mind: &super::thought::Mind, rng: &RngRegistry, key: RngKey) -> String {
    // A computer outcome quotes a real query or recipient; there is
    // nothing to vary in a fact, so it is said as it is.
    if let Some(outcome) = &mind.outcome {
        return format!("{outcome}.");
    }
    let mut clauses = vec![pick(rng, key, spoken_action(mind.action)).to_string()];
    if let Some((name, value)) = mind.need {
        // A second draw, so the need's phrasing is not locked to the
        // action's -- otherwise the pair would move together and the
        // variety would be a quarter of what it looks like.
        let need_key = RngKey::new(key.subsystem, key.chunk, key.epoch + 64, key.tick);
        clauses.push(pick(rng, need_key, spoken_need(name, value)).to_string());
    }
    sentence(&clauses.join(", and "))
}

/// Make `clause` a sentence: a capital at the front, a full stop at the
/// back.
///
/// The phrasings above are written as clauses because most of them read
/// better mid-sentence, and whichever is drawn first has to start one.
/// Without this the dashboard showed `... "I want to be close to someone
/// right now." come here a minute`, which is the kind of thing a test
/// does not see and a reader cannot miss.
fn sentence(clause: &str) -> String {
    let mut chars = clause.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => format!(
            "{}{}.",
            first.to_uppercase(),
            chars.as_str().trim_end_matches('.')
        ),
    }
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

    // Opener: greeting, recollection, what they say about themselves,
    // question.
    //
    // The spoken part and the remembered part are now different strings
    // from the same state. `a_gist` stays the monologue, because that is
    // what a listener remembers of somebody -- the substance, not the
    // wording -- and it is what `remembered_gist` quotes back later.
    // `a_said` is how it came out of their mouth this time. Before D36
    // these were one string, so a speaker said their inner voice verbatim
    // and said it identically whenever their state repeated.
    let a_mind = super::thought::read(a);
    let a_gist = super::thought::generate_thought(a);
    let a_said = said_aloud(&a_mind, rng_registry, key(3));
    let mut opener = vec![pick(rng_registry, key(0), greeting_templates(relationship))
        .replace("{name}", b.agent_id())];
    if let Some(recalled) = remembered_gist(a, b) {
        // Quoted as said: the gist is the listener's own first person.
        opener.push(format!("Last time you told me, \"{recalled}\""));
    }
    opener.push(a_said);
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
    let b_mind = super::thought::read(b);
    let b_gist = super::thought::generate_thought(b);
    let b_said = said_aloud(&b_mind, rng_registry, key(4));

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
                text: format!("{reaction} {b_said}"),
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
    fn what_a_listener_remembers_of_a_speaker_is_that_speakers_own_monologue() {
        // This used to assert that the spoken line *contained* the
        // monologue verbatim, which is what D36 turned out to be: a
        // person saying their inner voice word for word, and saying it
        // identically whenever their state repeated. The spoken line is
        // its own rendering now. What a listener carries away is still
        // the monologue, because what you remember of somebody is the
        // substance rather than the wording, and `remembered_gist` quotes
        // exactly this back in a later conversation.
        let a = HumanBeing::new("dialogue-e".to_string(), BiologicalSex::Female);
        let b = HumanBeing::new("dialogue-f".to_string(), BiologicalSex::Male);
        let event = generate_conversation(&a, &b, ConversationRelationship::Other, &registry(), 3);

        let a_thought = super::super::thought::generate_thought(&a);
        let b_thought = super::super::thought::generate_thought(&b);
        assert!(
            event.lines[0].text.contains("dialogue-f"),
            "greets the listener"
        );
        assert_eq!(event.lines[0].gist, a_thought, "a's memory is a's mind");
        assert_eq!(event.lines[1].gist, b_thought, "b's memory is b's mind");
        for line in &event.lines {
            // An opener ends on its question, a reply on a statement, so
            // either closing mark is right -- what matters is that a line
            // is finished rather than trailing off.
            assert!(
                line.text.ends_with('.') || line.text.ends_with('?'),
                "a spoken line is a finished sentence: {}",
                line.text
            );
            assert!(!line.text.trim().is_empty());
        }
    }

    #[test]
    fn a_spoken_line_starts_with_a_capital_whatever_phrasing_was_drawn() {
        // Most of the phrasings read better as clauses, so plenty of them
        // begin in lower case; whichever is drawn first has to open a
        // sentence. The browser caught this one, not a test: the People
        // page showed `... "I want to be close to someone right now."
        // come here a minute`.
        assert_eq!(sentence("come here a minute"), "Come here a minute.");
        assert_eq!(sentence("I'm tired"), "I'm tired.");
        assert_eq!(sentence("I'm tired."), "I'm tired.", "no doubled stop");
        assert_eq!(sentence(""), "", "nothing said is nothing shown");

        // Asserted on the rendering itself rather than on the assembled
        // line: a line also carries the listener's name, and a test
        // agent is called "dialogue-l", which is lower case for reasons
        // that have nothing to do with this.
        let a = HumanBeing::new("dialogue-k".to_string(), BiologicalSex::Female);
        let rng = registry();
        let mind = super::super::thought::read(&a);
        for tick in 0..200u64 {
            let key = RngKey::new(SubsystemId::Dialogue, 7, 3, tick);
            let said = said_aloud(&mind, &rng, key);
            let first = said.chars().next().expect("something was said");
            assert!(
                !first.is_lowercase(),
                "a spoken sentence begins in lower case at tick {tick}: {said}"
            );
            assert!(said.ends_with('.'), "and ends without a stop: {said}");
        }
    }

    /// Every phrasing, rendered, must open a sentence.
    ///
    /// Exhaustive on purpose. The first version of this test walked four
    /// hundred ticks of a simulated pair and passed *with the fix
    /// reverted*, because fresh test humans only ever reached phrasings
    /// that already began with a capital. A guard that cannot fail is
    /// worse than no guard: it reports safety it has not checked. This
    /// one enumerates every action and every phrasing, so a new
    /// lower-case clause cannot slip in behind it.
    #[test]
    fn every_spoken_phrasing_renders_as_a_sentence() {
        use crate::agents::ActionKind;

        const EVERY_ACTION: [ActionKind; 19] = [
            ActionKind::Idle,
            ActionKind::Rest,
            ActionKind::SeekWater,
            ActionKind::SeekFood,
            ActionKind::SeekShelter,
            ActionKind::SocialApproach,
            ActionKind::Explore,
            ActionKind::Gather,
            ActionKind::Mine,
            ActionKind::Move,
            ActionKind::Build,
            ActionKind::Craft,
            ActionKind::Transfer,
            ActionKind::Harm,
            ActionKind::Code,
            ActionKind::Intimacy,
            ActionKind::WebSearch,
            ActionKind::SendEmail,
            ActionKind::Idle,
        ];

        let mut checked = 0;
        for action in EVERY_ACTION {
            for phrasing in spoken_action(action) {
                let said = sentence(phrasing);
                let first = said.chars().next().expect("a phrasing is not empty");
                assert!(
                    !first.is_lowercase(),
                    "{action:?} phrasing renders in lower case: {said}"
                );
                assert!(
                    said.ends_with('.'),
                    "{action:?} phrasing has no stop: {said}"
                );
                assert!(
                    !said.ends_with(".."),
                    "{action:?} phrasing doubled its stop: {said}"
                );
                checked += 1;
            }
        }
        for name in ["hunger", "thirst", "fatigue", "something-else"] {
            for pressing in [0.4, 0.9] {
                for phrasing in spoken_need(name, pressing) {
                    // A need is never the first clause on its own, but it
                    // can be if an action phrase is ever dropped, and it
                    // costs nothing to hold it to the same rule.
                    let said = sentence(phrasing);
                    assert!(!said.chars().next().expect("not empty").is_lowercase());
                    checked += 1;
                }
            }
        }
        assert!(checked > 70, "only {checked} phrasings were checked");
    }

    /// D36, from the side that matters: a speaker whose state has not
    /// moved does not say the same sentence over and over.
    ///
    /// This is the test that would have failed before speech was given
    /// its own phrasings. The human here is never stepped, so their state
    /// is byte-identical at every tick -- which used to force one
    /// sentence, because the monologue is contractually a pure function
    /// of state with no RNG and the spoken line *was* the monologue.
    #[test]
    fn a_speaker_whose_state_has_not_moved_still_does_not_repeat_one_sentence() {
        let a = HumanBeing::new("dialogue-i".to_string(), BiologicalSex::Female);
        let b = HumanBeing::new("dialogue-j".to_string(), BiologicalSex::Male);
        let rng = registry();

        let said: std::collections::BTreeSet<String> = (0..200)
            .map(|tick| {
                generate_conversation(&a, &b, ConversationRelationship::Other, &rng, tick).lines[0]
                    .text
                    .clone()
            })
            .collect();

        assert!(
            said.len() > 1,
            "two hundred ticks of identical state produced one sentence: {said:?}"
        );
        // And the monologue they are all renderings of has not moved,
        // which is what makes this a phrasing fix rather than the state
        // being quietly shaken up to manufacture variety.
        let thought = super::super::thought::generate_thought(&a);
        assert_eq!(
            super::super::thought::generate_thought(&a),
            thought,
            "the mind behind the words is unchanged"
        );
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
