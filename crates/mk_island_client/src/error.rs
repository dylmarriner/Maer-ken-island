//! What can go wrong between a frontend and the island, in words somebody
//! can act on.
//!
//! The distinction that matters is not HTTP's. A frontend needs to tell
//! apart four things, because the person in front of it does something
//! different about each: the backend is not there, the backend is there
//! and said no, the backend is there and is a different version, and the
//! backend answered something this client could not read. A bare status
//! code tells none of them apart.

use mk_island_api::Refusal;

#[derive(Debug)]
pub enum ClientError {
    /// Nothing answered. The address is wrong, the backend is not
    /// running, or the network between them is not working.
    Unreachable { url: String, reason: String },
    /// It answered, and said no. `refusal` carries the island's own
    /// sentences; `status` is kept because 401 and 404 call for different
    /// things from a frontend even when the words are similar.
    Refused { status: u16, refusal: Refusal },
    /// No island is running behind this backend: it was started without
    /// `--scenario`, so it has a stored population and nothing else.
    ///
    /// Its own variant because it is not a failure. A frontend should say
    /// "this backend is not simulating an island" and offer what it can,
    /// rather than showing an error.
    NoIsland { what: String },
    /// This client and that server do not speak the same schema.
    Incompatible(String),
    /// It answered something this client could not read. A proxy serving
    /// an error page, a truncated download, a field that changed shape.
    Malformed { url: String, reason: String },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Unreachable { url, reason } => write!(
                f,
                "Could not reach the island at {url}: {reason}. Check that the backend is \
                 running and that this address is the one it is bound to."
            ),
            ClientError::Refused { status, refusal } => {
                let said = refusal.message();
                let said = if said.is_empty() {
                    format!("it answered {status} and said nothing")
                } else {
                    said
                };
                match refusal.writes_mode.as_deref() {
                    Some("disabled") => write!(
                        f,
                        "{said} That backend takes no changes from off its own machine at all: \
                         it was started without a control token on a non-loopback bind, and no \
                         token will work until it is restarted with one."
                    ),
                    Some("token") => write!(f, "{said} Check the control token."),
                    _ => write!(f, "{said}"),
                }
            }
            ClientError::NoIsland { what } => write!(
                f,
                "That backend is not simulating an island, so it has no {what}. It was started \
                 without --scenario: it keeps a stored population and nothing more."
            ),
            ClientError::Incompatible(reason) => write!(f, "{reason}"),
            ClientError::Malformed { url, reason } => write!(
                f,
                "The answer from {url} was not what this client reads: {reason}. That usually \
                 means something between here and the island replaced it -- a proxy's error \
                 page, or a login portal."
            ),
        }
    }
}

impl std::error::Error for ClientError {}

impl ClientError {
    /// Whether retrying unchanged could plausibly work.
    ///
    /// Only the network. A refusal, a version mismatch and an unreadable
    /// answer will all say exactly the same thing next time, and a client
    /// that retried them would hide a configuration problem behind a
    /// spinner.
    pub fn worth_retrying(&self) -> bool {
        matches!(self, ClientError::Unreachable { .. })
    }

    /// Whether a token would fix it.
    pub fn needs_a_token(&self) -> bool {
        matches!(self, ClientError::Refused { status: 401, .. })
    }
}
