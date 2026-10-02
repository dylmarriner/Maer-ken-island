//! Bridge to the external `apps/computer-service` Node process, giving
//! humans real web search and email (`ActionKind::WebSearch` /
//! `ActionKind::SendEmail`). See
//! `docs/superpowers/specs/2026-09-11-computer-access-integration-design.md`.
//!
//! Deliberately **not** part of `WorldState`'s serialized/hashed state: real
//! network results are non-deterministic by design. `WorldState::computer_bridge`
//! is `#[serde(skip)]` and only ever set by `mk serve` via
//! `WorldState::with_computer_bridge` after construction/load — a snapshot
//! loaded for `replay`/`audit`/`verify` never calls that setter, so this
//! feature has zero effect on those paths regardless of whether a live
//! human sent real email in the run that produced the snapshot.
//!
//! Query/recipient content is derived here from already-existing human
//! state, not carried as data on `ActionKind` itself — the same pattern
//! `ActionKind::Code` uses (gated by an `AgentWorldObservation` affordance
//! field, not parameterized), which keeps `ActionKind` `Copy`/`Eq` and the
//! learned-preference table (`autonomy::ACTIONS`) a small fixed action
//! space rather than an unbounded one keyed by search query text.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use thiserror::Error;

/// Glucose fraction (0.0-1.0 scale, see `NeedsSnapshot`) consumed by a
/// successful web search. Calibrated against `NeedsSnapshot::step`'s
/// per-year drain rate (~1.0/year at baseline sensitivity): a search costs
/// roughly a week's worth of passive drain, deducted only on success.
pub const WEB_SEARCH_GLUCOSE_COST: f64 = 0.02;
/// As `WEB_SEARCH_GLUCOSE_COST`, for a successful sent email.
pub const EMAIL_GLUCOSE_COST: f64 = 0.03;

/// Minimum ticks between two web searches by the same human.
pub const WEB_SEARCH_COOLDOWN_TICKS: u64 = 50;
/// Minimum ticks between two sent emails by the same human.
pub const EMAIL_COOLDOWN_TICKS: u64 = 100;
/// Rolling window (in ticks) over which `EMAIL_QUOTA_MAX` applies.
// Tick-based window, not wall-clock: this sim's tick duration
// (`CanonLocked::dt_seconds`) is configurable per world and not anchored to
// a real calendar day, so a literal "24 hours" quota has no single
// tick-count translation. Revisit if a world-relative "days" helper
// (`mk_core::time`) becomes the standard unit for spam-prevention windows.
pub const EMAIL_QUOTA_WINDOW_TICKS: u64 = 1_000;
pub const EMAIL_QUOTA_MAX: u32 = 10;

/// Bounded eviction limit for `ComputerInteractionMemory`'s two queues,
/// matching the `CONVERSATION_HISTORY_MAX_ENTRIES` pattern used elsewhere
/// in `humans::mod`.
pub const COMPUTER_MEMORY_MAX_ENTRIES: usize = 500;

/// Real internet contacts a human may email, resolved to an actual address
/// only by `apps/computer-service` (this crate never sees or stores a real
/// email address — separation of concerns from the design doc's Security
/// Considerations section).
pub const KNOWN_CONTACTS: [&str; 2] = ["dylan", "kirsty"];

#[derive(Debug, Clone, Error)]
pub enum BridgeError {
    #[error("computer-service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("network error talking to computer-service: {0}")]
    NetworkError(String),
    #[error("computer-service authentication error: {0}")]
    AuthenticationError(String),
    #[error("computer-service rate limit exceeded")]
    RateLimitError,
    #[error("computer-service request timed out")]
    TimeoutError,
    #[error("computer-service returned an invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub snippet: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub query: String,
    pub results: Vec<SearchResultItem>,
    /// 0.0-1.0 subjective satisfaction with the results, reported by
    /// `apps/computer-service` (result count/relevance heuristic).
    pub satisfaction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailResult {
    pub message_id: String,
    pub to: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerState {
    pub agent_id: String,
    pub powered_on: bool,
    pub unread_count: u32,
}

/// Trait boundary so `HumanSystem::step` can be exercised with a
/// `MockComputerBridge` in unit tests without any real network access, and
/// so a real `HttpComputerBridge` never leaks into determinism/replay
/// paths that only ever see `Option::None`.
pub trait ComputerBridge: Send + Sync {
    fn web_search(&self, agent_id: &str, query: &str) -> Result<WebSearchResult, BridgeError>;
    fn send_email(
        &self,
        agent_id: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<EmailResult, BridgeError>;
    #[allow(dead_code)]
    fn get_computer_state(&self, agent_id: &str) -> Result<ComputerState, BridgeError>;
}

/// `Clone + Debug` wrapper around `Arc<dyn ComputerBridge>` so
/// `WorldState` (which derives both) doesn't need every `ComputerBridge`
/// implementor to implement `Debug` itself — this wrapper's own `Debug`
/// impl always prints a fixed marker string rather than delegating to the
/// wrapped bridge.
#[derive(Clone)]
pub struct ComputerBridgeHandle(pub std::sync::Arc<dyn ComputerBridge>);

impl std::fmt::Debug for ComputerBridgeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ComputerBridgeHandle(<opaque bridge>)")
    }
}

impl ComputerBridgeHandle {
    pub fn new(bridge: impl ComputerBridge + 'static) -> Self {
        Self(std::sync::Arc::new(bridge))
    }

    pub fn as_bridge(&self) -> &dyn ComputerBridge {
        self.0.as_ref()
    }
}

/// Real HTTP client for `apps/computer-service`. Blocking (`ureq`), matching
/// `HumanSystem::step`'s fully synchronous per-tick loop — see module docs
/// on why this crate does not pull in an async runtime for one bridge.
pub struct HttpComputerBridge {
    base_url: String,
    agent: ureq::Agent,
    /// Shared secret sent as `Authorization: Bearer` when the service
    /// requires one (`COMPUTER_SERVICE_TOKEN`).
    token: Option<String>,
}

const MAX_RETRIES: u32 = 3;

/// Whether repeating a request after a failure can change the outcome of the
/// real world, which decides which failures may be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPolicy {
    /// Repeating the request is harmless (e.g. a search): retry on timeout
    /// and on rate limiting.
    Idempotent,
    /// Repeating the request may repeat its effect (e.g. sending an email):
    /// retry only on rate limiting, which means the request was not
    /// accepted. A timeout is never retried because the request may have
    /// been delivered.
    NonIdempotent,
}

impl RetryPolicy {
    fn should_retry(self, err: &BridgeError) -> bool {
        match (self, err) {
            (_, BridgeError::RateLimitError) => true,
            (RetryPolicy::Idempotent, BridgeError::TimeoutError) => true,
            _ => false,
        }
    }
}

/// Exponential backoff used between attempts: 200, 400, 800 ms, ...
fn backoff_delay(retry: u32) -> Duration {
    Duration::from_millis(200 * 2u64.pow(retry))
}

/// Run `attempt` up to [`MAX_RETRIES`] times, sleeping `backoff(n)` after the
/// n-th failure that `policy` allows retrying (never after the last attempt).
/// A failure the policy does not allow retrying is returned immediately, and
/// when attempts run out the final attempt's real error is returned.
pub fn retry_with<T>(
    policy: RetryPolicy,
    backoff: impl Fn(u32) -> Duration,
    mut attempt: impl FnMut() -> Result<T, BridgeError>,
) -> Result<T, BridgeError> {
    let mut retry = 0;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(err) if policy.should_retry(&err) && retry + 1 < MAX_RETRIES => {
                std::thread::sleep(backoff(retry));
                retry += 1;
            }
            Err(err) => return Err(err),
        }
    }
}
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

impl HttpComputerBridge {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            agent: ureq::AgentBuilder::new().timeout(REQUEST_TIMEOUT).build(),
            token: None,
        }
    }

    /// Send `token` as a bearer credential on every request.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        self.token = (!token.trim().is_empty()).then_some(token);
        self
    }

    fn authorized(&self, request: ureq::Request) -> ureq::Request {
        match &self.token {
            Some(token) => request.set("Authorization", &format!("Bearer {token}")),
            None => request,
        }
    }

    /// Construct from environment, only when explicitly opted in via
    /// `COMPUTER_ACTIONS_ENABLED=1` — absent that, humans never attempt
    /// `WebSearch`/`SendEmail` regardless of whether something happens to be
    /// listening on `COMPUTER_SERVICE_URL`. Called only by `mk serve`; never
    /// by `replay`/`audit`/`verify`/test construction paths.
    pub fn from_env() -> Option<Self> {
        if std::env::var("COMPUTER_ACTIONS_ENABLED").as_deref() != Ok("1") {
            return None;
        }
        let base_url = std::env::var("COMPUTER_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8002".to_string());
        let bridge = Self::new(base_url);
        Some(match std::env::var("COMPUTER_SERVICE_TOKEN") {
            Ok(token) => bridge.with_token(token),
            Err(_) => bridge,
        })
    }

    fn with_retry<T>(
        &self,
        policy: RetryPolicy,
        attempt: impl FnMut() -> Result<T, BridgeError>,
    ) -> Result<T, BridgeError> {
        retry_with(policy, backoff_delay, attempt)
    }

    fn classify_error(err: ureq::Error) -> BridgeError {
        match err {
            ureq::Error::Status(401, _) | ureq::Error::Status(403, _) => {
                BridgeError::AuthenticationError("computer-service rejected credentials".into())
            }
            ureq::Error::Status(429, _) => BridgeError::RateLimitError,
            ureq::Error::Status(code, _) => BridgeError::ServiceUnavailable(format!("HTTP {code}")),
            ureq::Error::Transport(transport) => {
                if transport.kind() == ureq::ErrorKind::Io
                    && transport.to_string().to_lowercase().contains("timed out")
                {
                    BridgeError::TimeoutError
                } else {
                    BridgeError::NetworkError(transport.to_string())
                }
            }
        }
    }
}

impl ComputerBridge for HttpComputerBridge {
    fn web_search(&self, agent_id: &str, query: &str) -> Result<WebSearchResult, BridgeError> {
        self.with_retry(RetryPolicy::Idempotent, || {
            let url = format!("{}/api/computer/{}/search", self.base_url, agent_id);
            self.authorized(self.agent.post(&url))
                .send_json(ureq::json!({ "query": query }))
                .map_err(Self::classify_error)?
                .into_json::<WebSearchResult>()
                .map_err(|e| BridgeError::InvalidResponse(e.to_string()))
        })
    }

    fn send_email(
        &self,
        agent_id: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<EmailResult, BridgeError> {
        self.with_retry(RetryPolicy::NonIdempotent, || {
            let url = format!("{}/api/computer/{}/email", self.base_url, agent_id);
            self.authorized(self.agent.post(&url))
                .send_json(ureq::json!({ "to": to, "subject": subject, "body": body }))
                .map_err(Self::classify_error)?
                .into_json::<EmailResult>()
                .map_err(|e| BridgeError::InvalidResponse(e.to_string()))
        })
    }

    fn get_computer_state(&self, agent_id: &str) -> Result<ComputerState, BridgeError> {
        let url = format!("{}/api/computer/{}/state", self.base_url, agent_id);
        self.authorized(self.agent.get(&url))
            .call()
            .map_err(Self::classify_error)?
            .into_json::<ComputerState>()
            .map_err(|e| BridgeError::InvalidResponse(e.to_string()))
    }
}

/// Deterministic per-human record of computer interactions — persisted
/// through the normal `HumanBeing`/human-storage snapshot path like
/// `conversation_history`, even though the *content* it stores originated
/// from a non-deterministic real-world call. Cooldown/quota fields make the
/// gating decisions themselves reproducible given the same history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputerInteractionMemory {
    pub web_searches: VecDeque<WebSearchMemory>,
    pub email_interactions: VecDeque<EmailMemory>,
    #[serde(default)]
    last_search_tick: Option<u64>,
    #[serde(default)]
    last_email_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchMemory {
    pub timestamp: u64,
    pub query: String,
    pub result_count: usize,
    pub satisfaction: f64,
    pub succeeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMemory {
    pub timestamp: u64,
    pub recipient: String,
    pub subject: String,
    pub sent_successfully: bool,
}

impl ComputerInteractionMemory {
    fn search_on_cooldown(&self, tick: u64) -> bool {
        self.last_search_tick
            .is_some_and(|last| tick.saturating_sub(last) < WEB_SEARCH_COOLDOWN_TICKS)
    }

    fn email_on_cooldown(&self, tick: u64) -> bool {
        self.last_email_tick
            .is_some_and(|last| tick.saturating_sub(last) < EMAIL_COOLDOWN_TICKS)
    }

    fn email_quota_exceeded(&self, tick: u64) -> bool {
        self.email_interactions
            .iter()
            .filter(|e| tick.saturating_sub(e.timestamp) <= EMAIL_QUOTA_WINDOW_TICKS)
            .count() as u32
            >= EMAIL_QUOTA_MAX
    }

    /// Ticks since this human last emailed `recipient`, or `None` if never.
    fn ticks_since_last_contact(&self, recipient: &str, tick: u64) -> Option<u64> {
        self.email_interactions
            .iter()
            .rev()
            .find(|e| e.recipient == recipient)
            .map(|e| tick.saturating_sub(e.timestamp))
    }

    fn record_search(&mut self, tick: u64, memory: WebSearchMemory) {
        self.last_search_tick = Some(tick);
        push_bounded(&mut self.web_searches, COMPUTER_MEMORY_MAX_ENTRIES, memory);
    }

    fn record_email(&mut self, tick: u64, memory: EmailMemory) {
        self.last_email_tick = Some(tick);
        push_bounded(
            &mut self.email_interactions,
            COMPUTER_MEMORY_MAX_ENTRIES,
            memory,
        );
    }
}

fn push_bounded<T>(queue: &mut VecDeque<T>, max_entries: usize, item: T) {
    queue.push_back(item);
    while queue.len() > max_entries {
        queue.pop_front();
    }
}

/// Unlockable recipes a human can research, with the phrase they would
/// search for. The baseline survival tier is always known, so only the
/// invented tier (`resource_economy::recipe_unlock_threshold` > 0) appears.
const RESEARCHABLE_RECIPES: [(crate::resource_economy::RecipeId, &str); 4] = [
    (
        crate::resource_economy::RecipeId::IronAxe,
        "how to forge an iron axe",
    ),
    (
        crate::resource_economy::RecipeId::IronPickaxe,
        "how to forge an iron pickaxe",
    ),
    (
        crate::resource_economy::RecipeId::Cart,
        "how to build a wooden cart",
    ),
    (
        crate::resource_economy::RecipeId::StoneHouse,
        "how to build a stone house",
    ),
];

/// Needs at or above this level become something worth looking up.
const PRESSING_NEED: f64 = 0.5;
/// A negative emotion at or above this magnitude prompts a coping search.
const PRESSING_EMOTION: f64 = 0.4;
/// How many of this human's most recent queries are not repeated.
const RECENT_QUERY_WINDOW: usize = 3;

/// Emotions a person would look for help with, and how they'd phrase it.
fn coping_query(emotion: &str) -> Option<&'static str> {
    Some(match emotion {
        "sadness" | "despair" => "how to cope with sadness",
        "fear" => "how to overcome fear",
        "anger" | "hate" | "resentment" => "how to calm down when angry",
        "shame" | "guilt" | "humiliation" | "embarrassment" => "how to forgive yourself",
        "jealousy" | "envy" => "how to deal with jealousy",
        "boredom" => "new things to try when bored",
        "disappointment" => "how to handle disappointment",
        _ => return None,
    })
}

/// What this human currently wants to find out, most pressing first,
/// derived from their own state: bodily needs, strong negative emotion,
/// and the next invention their accumulated knowledge is approaching.
fn information_needs(human: &super::HumanBeing) -> Vec<&'static str> {
    let mut needs: Vec<(f64, &'static str)> = Vec::new();
    if human.needs.thirst >= PRESSING_NEED {
        needs.push((human.needs.thirst, "how to find safe drinking water"));
    }
    if human.needs.hunger >= PRESSING_NEED {
        needs.push((human.needs.hunger, "edible wild plants and foraging"));
    }
    if human.needs.fatigue >= PRESSING_NEED {
        needs.push((human.needs.fatigue, "how to sleep better"));
    }
    let (emotion, intensity) = human.emotion.current.dominant();
    if intensity.abs() >= PRESSING_EMOTION {
        if let Some(query) = coping_query(emotion) {
            needs.push((intensity.abs(), query));
        }
    }
    // Curiosity drives research: the nearest still-locked invention, weighted
    // by how close the human's knowledge is to unlocking it.
    let knowledge = human.technology.accumulated_knowledge;
    if let Some((threshold, query)) = RESEARCHABLE_RECIPES
        .iter()
        .map(|(id, query)| {
            (
                crate::resource_economy::recipe_unlock_threshold(*id),
                *query,
            )
        })
        .filter(|(threshold, _)| *threshold > knowledge)
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    {
        let progress = (knowledge / threshold).clamp(0.0, 1.0);
        needs.push((
            progress * human.emotion.current.curiosity.abs().max(0.25),
            query,
        ));
    }
    needs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    needs.into_iter().map(|(_, query)| query).collect()
}

/// The query this human searches for now: their most pressing information
/// need that they have not looked up in their last few searches. `None`
/// when they have nothing new to look up, in which case no search happens.
fn query_for(human: &super::HumanBeing) -> Option<&'static str> {
    let recent: Vec<&str> = human
        .computer_interaction_memory
        .web_searches
        .iter()
        .rev()
        .take(RECENT_QUERY_WINDOW)
        .map(|memory| memory.query.as_str())
        .collect();
    information_needs(human)
        .into_iter()
        .find(|query| !recent.contains(query))
}

/// Subject and body of an email from `human` to `recipient`, composed from
/// how long it has been since they last wrote, how they feel, what is on
/// their mind (their internal monologue) and what they last read about.
fn compose_email(human: &super::HumanBeing, recipient: &str, tick: u64) -> (String, String) {
    let memory = &human.computer_interaction_memory;
    let (emotion, intensity) = human.emotion.current.dominant();
    let positive = matches!(
        emotion,
        "joy"
            | "love"
            | "pride"
            | "awe"
            | "hope"
            | "relief"
            | "gratitude"
            | "admiration"
            | "triumph"
            | "contentment"
            | "curiosity"
    );
    let subject = match memory.ticks_since_last_contact(recipient, tick) {
        None => format!("Hello from {}", human.agent_id()),
        Some(gap) if gap >= EMAIL_QUOTA_WINDOW_TICKS => "It's been a while".to_string(),
        Some(_) if intensity.abs() >= PRESSING_EMOTION && positive => "Some good news".to_string(),
        Some(_) if intensity.abs() >= PRESSING_EMOTION => "Could use a friend".to_string(),
        Some(_) => "Checking in".to_string(),
    };

    let mut greeting: Vec<char> = recipient.chars().collect();
    if let Some(first) = greeting.first_mut() {
        *first = first.to_ascii_uppercase();
    }
    let mut paragraphs = vec![
        format!("Hi {},", greeting.into_iter().collect::<String>()),
        super::thought::generate_thought(human),
    ];
    if let Some(search) = memory.web_searches.iter().rev().find(|s| s.succeeded) {
        let mut line = format!("I've been reading about {}.", search.query);
        if let Some(top) = human
            .last_web_search_result
            .as_ref()
            .filter(|result| result.query == search.query)
            .and_then(|result| result.results.first())
        {
            line.push_str(&format!(
                " The most useful thing I found was \"{}\".",
                top.title
            ));
        }
        paragraphs.push(line);
    }
    paragraphs.push(format!("— {}", human.agent_id()));
    (subject, paragraphs.join("\n\n"))
}

/// Pick which known contact to email: whoever this human has gone longest
/// without contacting (ties favor the first `KNOWN_CONTACTS` entry).
fn contact_for(memory: &ComputerInteractionMemory, tick: u64) -> &'static str {
    KNOWN_CONTACTS
        .iter()
        .max_by_key(|contact| {
            memory
                .ticks_since_last_contact(contact, tick)
                .unwrap_or(u64::MAX)
        })
        .copied()
        .unwrap_or(KNOWN_CONTACTS[0])
}

/// Execute the currently-selected `WebSearch`/`SendEmail` action against
/// `bridge`, applying resource costs only on success and always recording
/// the attempt in `human.computer_interaction_memory`. Returns whether the
/// action succeeded (feeds `human.last_action_success`, same contract as
/// `ResourceEconomyState::apply_human_action`).
///
/// Called from `HumanSystem::step` only when `human.economy_action.kind`
/// is one of the two computer actions — mirrors the existing
/// `ActionKind::Code` post-check in that loop.
pub fn execute_computer_action(
    human: &mut super::HumanBeing,
    bridge: Option<&dyn ComputerBridge>,
    tick: u64,
) -> bool {
    let Some(bridge) = bridge else {
        return false;
    };
    let agent_id = human.agent_id().to_string();
    match human.economy_action.kind {
        crate::agents::ActionKind::WebSearch => {
            if human.computer_interaction_memory.search_on_cooldown(tick)
                || human.needs.glucose < WEB_SEARCH_GLUCOSE_COST
            {
                return false;
            }
            let Some(query) = query_for(human) else {
                return false;
            };
            let query = query.to_string();
            match bridge.web_search(&agent_id, &query) {
                Ok(result) => {
                    human.needs.glucose -= WEB_SEARCH_GLUCOSE_COST;
                    human.computer_interaction_memory.record_search(
                        tick,
                        WebSearchMemory {
                            timestamp: tick,
                            query: query.clone(),
                            result_count: result.results.len(),
                            satisfaction: result.satisfaction,
                            succeeded: true,
                        },
                    );
                    human.last_web_search_result = Some(result);
                    true
                }
                Err(_) => {
                    human.computer_interaction_memory.record_search(
                        tick,
                        WebSearchMemory {
                            timestamp: tick,
                            query,
                            result_count: 0,
                            satisfaction: 0.0,
                            succeeded: false,
                        },
                    );
                    false
                }
            }
        }
        crate::agents::ActionKind::SendEmail => {
            if human.computer_interaction_memory.email_on_cooldown(tick)
                || human.computer_interaction_memory.email_quota_exceeded(tick)
                || human.needs.glucose < EMAIL_GLUCOSE_COST
            {
                return false;
            }
            let recipient = contact_for(&human.computer_interaction_memory, tick).to_string();
            let (subject, body) = compose_email(human, &recipient, tick);
            match bridge.send_email(&agent_id, &recipient, &subject, &body) {
                Ok(result) => {
                    human.needs.glucose -= EMAIL_GLUCOSE_COST;
                    human.computer_interaction_memory.record_email(
                        tick,
                        EmailMemory {
                            timestamp: tick,
                            recipient: recipient.clone(),
                            subject: subject.clone(),
                            sent_successfully: true,
                        },
                    );
                    human.last_email_result = Some(result);
                    true
                }
                Err(_) => {
                    human.computer_interaction_memory.record_email(
                        tick,
                        EmailMemory {
                            timestamp: tick,
                            recipient,
                            subject,
                            sent_successfully: false,
                        },
                    );
                    false
                }
            }
        }
        _ => false,
    }
}

/// Test double — canned responses, no network. Used by `humans::mod` unit
/// tests and by `computer_bridge`'s own tests.
#[cfg(test)]
pub struct MockComputerBridge {
    pub search_result: std::sync::Mutex<Option<Result<WebSearchResult, BridgeError>>>,
    pub email_result: std::sync::Mutex<Option<Result<EmailResult, BridgeError>>>,
    pub calls: AtomicU32,
}

#[cfg(test)]
impl MockComputerBridge {
    pub fn always_succeeds() -> Self {
        Self {
            search_result: std::sync::Mutex::new(Some(Ok(WebSearchResult {
                query: "test".into(),
                results: vec![SearchResultItem {
                    title: "Result".into(),
                    snippet: "snippet".into(),
                    url: "https://example.com".into(),
                }],
                satisfaction: 0.8,
            }))),
            email_result: std::sync::Mutex::new(Some(Ok(EmailResult {
                message_id: "msg-1".into(),
                to: "dylan".into(),
                subject: "Checking in".into(),
            }))),
            calls: AtomicU32::new(0),
        }
    }

    pub fn always_fails() -> Self {
        Self {
            search_result: std::sync::Mutex::new(Some(Err(BridgeError::ServiceUnavailable(
                "down".into(),
            )))),
            email_result: std::sync::Mutex::new(Some(Err(BridgeError::ServiceUnavailable(
                "down".into(),
            )))),
            calls: AtomicU32::new(0),
        }
    }
}

#[cfg(test)]
impl ComputerBridge for MockComputerBridge {
    fn web_search(&self, _agent_id: &str, query: &str) -> Result<WebSearchResult, BridgeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.search_result.lock().unwrap().clone() {
            Some(Ok(mut result)) => {
                result.query = query.to_string();
                Ok(result)
            }
            Some(Err(e)) => Err(e),
            None => Err(BridgeError::ServiceUnavailable(
                "no mock response set".into(),
            )),
        }
    }

    fn send_email(
        &self,
        _agent_id: &str,
        to: &str,
        subject: &str,
        _body: &str,
    ) -> Result<EmailResult, BridgeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.email_result.lock().unwrap().clone() {
            Some(Ok(mut result)) => {
                result.to = to.to_string();
                result.subject = subject.to_string();
                Ok(result)
            }
            Some(Err(e)) => Err(e),
            None => Err(BridgeError::ServiceUnavailable(
                "no mock response set".into(),
            )),
        }
    }

    fn get_computer_state(&self, agent_id: &str) -> Result<ComputerState, BridgeError> {
        Ok(ComputerState {
            agent_id: agent_id.to_string(),
            powered_on: true,
            unread_count: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{retry_with, BridgeError, RetryPolicy, MAX_RETRIES};
    use std::cell::Cell;
    use std::time::Duration;

    fn no_wait(_: u32) -> Duration {
        Duration::ZERO
    }

    /// Fails with `err()` for the first `failures` calls, then succeeds.
    fn run(
        policy: RetryPolicy,
        failures: u32,
        err: fn() -> BridgeError,
    ) -> (Result<u32, BridgeError>, u32) {
        let calls = Cell::new(0);
        let result = retry_with(policy, no_wait, || {
            calls.set(calls.get() + 1);
            if calls.get() <= failures {
                Err(err())
            } else {
                Ok(calls.get())
            }
        });
        (result, calls.get())
    }

    #[test]
    fn idempotent_requests_retry_timeouts_then_succeed() {
        let (result, calls) = run(RetryPolicy::Idempotent, 2, || BridgeError::TimeoutError);
        assert_eq!(result.unwrap(), 3);
        assert_eq!(calls, 3);
    }

    #[test]
    fn non_idempotent_requests_never_retry_timeouts() {
        let (result, calls) = run(RetryPolicy::NonIdempotent, 1, || BridgeError::TimeoutError);
        assert!(matches!(result, Err(BridgeError::TimeoutError)));
        assert_eq!(calls, 1, "a timed-out email may have been sent");
    }

    #[test]
    fn non_idempotent_requests_retry_rate_limits() {
        let (result, calls) = run(RetryPolicy::NonIdempotent, 2, || {
            BridgeError::RateLimitError
        });
        assert_eq!(result.unwrap(), 3);
        assert_eq!(calls, 3);
    }

    #[test]
    fn exhausted_retries_return_the_real_error() {
        let (result, calls) = run(RetryPolicy::Idempotent, 99, || BridgeError::TimeoutError);
        assert!(matches!(result, Err(BridgeError::TimeoutError)));
        assert_eq!(calls, MAX_RETRIES);

        let (result, calls) = run(RetryPolicy::Idempotent, 99, || BridgeError::RateLimitError);
        assert!(matches!(result, Err(BridgeError::RateLimitError)));
        assert_eq!(calls, MAX_RETRIES);
    }

    #[test]
    fn other_errors_are_not_retried() {
        let (result, calls) = run(RetryPolicy::Idempotent, 99, || {
            BridgeError::ServiceUnavailable("HTTP 500".into())
        });
        assert!(matches!(result, Err(BridgeError::ServiceUnavailable(_))));
        assert_eq!(calls, 1);
    }

    #[test]
    fn backoff_runs_between_attempts_only() {
        let waits = Cell::new(0);
        let calls = Cell::new(0);
        let _: Result<(), _> = retry_with(
            RetryPolicy::Idempotent,
            |_| {
                waits.set(waits.get() + 1);
                Duration::ZERO
            },
            || {
                calls.set(calls.get() + 1);
                Err(BridgeError::TimeoutError)
            },
        );
        assert_eq!(calls.get(), MAX_RETRIES);
        assert_eq!(waits.get(), MAX_RETRIES - 1);
    }

    use super::*;
    use crate::humans::HumanBeing;
    use mk_core::human::BiologicalSex;

    fn human_with_glucose(glucose: f64) -> HumanBeing {
        let mut human = HumanBeing::new("gem-d".to_string(), BiologicalSex::Male);
        human.needs.glucose = glucose;
        human
    }

    #[test]
    fn web_search_success_deducts_glucose_and_records_memory() {
        let bridge = MockComputerBridge::always_succeeds();
        let mut human = human_with_glucose(0.5);
        human.economy_action.kind = crate::agents::ActionKind::WebSearch;
        let success = execute_computer_action(&mut human, Some(&bridge), 100);
        assert!(success);
        assert!((human.needs.glucose - (0.5 - WEB_SEARCH_GLUCOSE_COST)).abs() < 1e-9);
        assert_eq!(human.computer_interaction_memory.web_searches.len(), 1);
        assert!(human.last_web_search_result.is_some());
    }

    #[test]
    fn web_search_failure_does_not_deduct_glucose() {
        let bridge = MockComputerBridge::always_fails();
        let mut human = human_with_glucose(0.5);
        human.economy_action.kind = crate::agents::ActionKind::WebSearch;
        let success = execute_computer_action(&mut human, Some(&bridge), 100);
        assert!(!success);
        assert!((human.needs.glucose - 0.5).abs() < 1e-9);
        assert_eq!(human.computer_interaction_memory.web_searches.len(), 1);
        assert!(!human.computer_interaction_memory.web_searches[0].succeeded);
    }

    #[test]
    fn web_search_on_cooldown_never_calls_bridge() {
        let bridge = MockComputerBridge::always_succeeds();
        let mut human = human_with_glucose(0.5);
        human.economy_action.kind = crate::agents::ActionKind::WebSearch;
        assert!(execute_computer_action(&mut human, Some(&bridge), 100));
        let success = execute_computer_action(&mut human, Some(&bridge), 110);
        assert!(!success);
        assert_eq!(bridge.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn no_bridge_means_action_always_fails_without_touching_memory() {
        let mut human = human_with_glucose(0.5);
        human.economy_action.kind = crate::agents::ActionKind::SendEmail;
        let success = execute_computer_action(&mut human, None, 100);
        assert!(!success);
        assert!(human
            .computer_interaction_memory
            .email_interactions
            .is_empty());
    }

    #[test]
    fn insufficient_glucose_blocks_email_without_calling_bridge() {
        let bridge = MockComputerBridge::always_succeeds();
        let mut human = human_with_glucose(0.0);
        human.economy_action.kind = crate::agents::ActionKind::SendEmail;
        let success = execute_computer_action(&mut human, Some(&bridge), 100);
        assert!(!success);
        assert_eq!(bridge.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn email_quota_blocks_after_max_within_window() {
        let bridge = MockComputerBridge::always_succeeds();
        let mut human = human_with_glucose(1.0);
        human.economy_action.kind = crate::agents::ActionKind::SendEmail;
        for i in 0..EMAIL_QUOTA_MAX {
            let tick = 100 + i as u64 * EMAIL_COOLDOWN_TICKS;
            assert!(execute_computer_action(&mut human, Some(&bridge), tick));
        }
        let over_quota_tick = 100 + EMAIL_QUOTA_MAX as u64 * EMAIL_COOLDOWN_TICKS;
        assert!(!execute_computer_action(
            &mut human,
            Some(&bridge),
            over_quota_tick
        ));
    }

    #[test]
    fn contact_rotation_favors_longest_since_contact() {
        let mut memory = ComputerInteractionMemory::default();
        memory.record_email(
            100,
            EmailMemory {
                timestamp: 100,
                recipient: "dylan".to_string(),
                subject: "hi".to_string(),
                sent_successfully: true,
            },
        );
        assert_eq!(contact_for(&memory, 500), "kirsty");
    }

    #[test]
    fn search_query_follows_the_most_pressing_need() {
        let mut human = human_with_glucose(0.5);
        human.needs.thirst = 0.9;
        human.needs.hunger = 0.6;
        assert_eq!(query_for(&human), Some("how to find safe drinking water"));
        human.needs.thirst = 0.0;
        assert_eq!(query_for(&human), Some("edible wild plants and foraging"));
    }

    #[test]
    fn search_research_targets_the_next_locked_invention() {
        let mut human = human_with_glucose(0.5);
        human.needs.thirst = 0.0;
        human.needs.hunger = 0.0;
        human.needs.fatigue = 0.0;
        human.emotion.current = human.emotion.baseline.clone();
        human.technology.accumulated_knowledge = 10.0; // iron tools unlocked at 5
        assert_eq!(query_for(&human), Some("how to build a wooden cart"));
        human.technology.accumulated_knowledge = 100.0; // everything unlocked
        assert_eq!(query_for(&human), None);
    }

    #[test]
    fn recent_queries_are_not_repeated() {
        let mut human = human_with_glucose(0.5);
        human.needs.thirst = 0.9;
        human.computer_interaction_memory.record_search(
            10,
            WebSearchMemory {
                timestamp: 10,
                query: "how to find safe drinking water".to_string(),
                result_count: 1,
                satisfaction: 0.5,
                succeeded: true,
            },
        );
        assert_ne!(query_for(&human), Some("how to find safe drinking water"));
    }

    #[test]
    fn email_is_composed_from_the_humans_own_state() {
        let mut human = human_with_glucose(0.5);
        human.computer_interaction_memory.record_search(
            10,
            WebSearchMemory {
                timestamp: 10,
                query: "how to build a wooden cart".to_string(),
                result_count: 1,
                satisfaction: 0.5,
                succeeded: true,
            },
        );
        let (subject, body) = compose_email(&human, "kirsty", 20);
        assert_eq!(subject, "Hello from gem-d");
        assert!(body.starts_with("Hi Kirsty,"));
        assert!(body.contains(&crate::humans::thought::generate_thought(&human)));
        assert!(body.contains("I've been reading about how to build a wooden cart."));
        assert!(body.ends_with("— gem-d"));

        human.computer_interaction_memory.record_email(
            20,
            EmailMemory {
                timestamp: 20,
                recipient: "kirsty".to_string(),
                subject,
                sent_successfully: true,
            },
        );
        let (later, _) = compose_email(&human, "kirsty", 20 + EMAIL_QUOTA_WINDOW_TICKS);
        assert_eq!(later, "It's been a while");
    }
}

#[cfg(test)]
mod http_bridge_auth_tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    /// Serve one HTTP request, returning the request head the bridge sent.
    fn serve_once(listener: TcpListener) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut head = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("read");
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            let body = r#"{"agent_id":"Gem-D","powered_on":true,"unread_count":3}"#;
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("write");
            head
        })
    }

    #[test]
    fn a_configured_token_is_sent_as_a_bearer_header() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let server = serve_once(listener);

        let state = HttpComputerBridge::new(base)
            .with_token("s3cret")
            .get_computer_state("Gem-D")
            .expect("state");
        let head = server.join().expect("server").to_ascii_lowercase();

        assert!(head.contains("authorization: bearer s3cret"), "{head}");
        assert!(state.powered_on);
    }

    #[test]
    fn a_blank_token_sends_no_header() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let server = serve_once(listener);

        HttpComputerBridge::new(base)
            .with_token("  ")
            .get_computer_state("Gem-D")
            .expect("state");
        let head = server.join().expect("server").to_ascii_lowercase();

        assert!(!head.contains("authorization"), "{head}");
    }
}
