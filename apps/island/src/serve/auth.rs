//! Who may change anything through the dashboard. Ported from upstream
//! `apps/mk_cli/src/web/server.rs` (`ControlAuth`): a configured token always
//! wins; without one, writes are allowed only on a loopback bind, and a
//! non-loopback bind without a token refuses every write.

/// Environment variable holding the dashboard's bearer token.
pub const TOKEN_ENV: &str = "ISLAND_CONTROL_TOKEN";

/// Environment variable holding a token every read must also carry.
///
/// Separate from [`TOKEN_ENV`] because the two answer different questions.
/// The control token decides who may change the island; this one decides
/// who may look at it at all. On a loopback bind the second question does
/// not arise and reads are open, which is the default and is right. The
/// moment the backend is bound off-host -- which is the whole point of a
/// frontend on another machine -- "anyone who can reach the port" becomes
/// an answer somebody has to choose deliberately rather than inherit.
pub const READ_TOKEN_ENV: &str = "ISLAND_READ_TOKEN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlAuth {
    /// Require `Authorization: Bearer <token>`.
    BearerToken(String),
    /// No token configured, bound to loopback: only this machine can reach it.
    LoopbackOnly,
    /// No token configured but reachable off-host: refuse every write.
    Disabled,
}

/// Who may read the island.
///
/// Reads were open to anyone who could reach the port for as long as the
/// dashboard and the island were the same machine. They still are by
/// default, because on a loopback bind that is exactly as open as the
/// machine itself. A backend serving frontends across a network is a
/// different thing, and this is how its operator says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadAuth {
    /// Anyone who can reach the server may read it.
    Open,
    /// Reads need `Authorization: Bearer <token>`.
    BearerToken(String),
}

impl ReadAuth {
    pub fn resolve(token: Option<String>) -> Self {
        match token {
            Some(token) if !token.trim().is_empty() => ReadAuth::BearerToken(token),
            _ => ReadAuth::Open,
        }
    }

    /// Whether a request carrying this `Authorization` header may read.
    ///
    /// The control *token* counts. An operator holding the token that lets
    /// them pause the island should not need a second one to watch it, and
    /// making them carry both would mean a desktop client holding two
    /// secrets to do one job.
    ///
    /// Only the token. `ControlAuth::permits` answers true for everyone
    /// under `LoopbackOnly`, because there the question it is answering is
    /// "is this machine allowed to write", and on loopback everyone is
    /// this machine. That is a statement about network position, and this
    /// gate is about a credential; deferring to it would have made any
    /// bearer token at all open a server whose reads were locked. A test
    /// below holds that line, because it was crossed once already.
    pub fn permits(&self, header: Option<&str>, control: &ControlAuth) -> bool {
        match self {
            ReadAuth::Open => true,
            ReadAuth::BearerToken(expected) => match bearer(header) {
                None => false,
                Some(given) => {
                    constant_time_eq(given.as_bytes(), expected.as_bytes())
                        || matches!(control, ControlAuth::BearerToken(_)) && control.permits(header)
                }
            },
        }
    }

    pub fn needs_token(&self) -> bool {
        matches!(self, ReadAuth::BearerToken(_))
    }

    pub fn describe(&self) -> &'static str {
        match self {
            ReadAuth::Open => "reading is open to anyone who can reach this server",
            ReadAuth::BearerToken(_) => "reading needs the read token",
        }
    }
}

/// The token out of an `Authorization: Bearer <token>` header, trimmed.
///
/// One place, because two different parsers for one header is two
/// different ideas of what counts as a token.
fn bearer(header: Option<&str>) -> Option<&str> {
    let value = header?;
    let (scheme, rest) = (value.get(..7)?, value.get(7..)?);
    scheme.eq_ignore_ascii_case("Bearer ").then(|| rest.trim())
}

impl ControlAuth {
    pub fn resolve(token: Option<String>, bind_address: std::net::IpAddr) -> Self {
        match token {
            Some(token) if !token.trim().is_empty() => ControlAuth::BearerToken(token),
            _ if bind_address.is_loopback() => ControlAuth::LoopbackOnly,
            _ => ControlAuth::Disabled,
        }
    }

    /// Whether a request carrying this `Authorization` header may write.
    pub fn permits(&self, header: Option<&str>) -> bool {
        match self {
            ControlAuth::BearerToken(expected) => bearer(header)
                .is_some_and(|token| constant_time_eq(token.as_bytes(), expected.as_bytes())),
            ControlAuth::LoopbackOnly => true,
            ControlAuth::Disabled => false,
        }
    }

    /// A stable word for the mode, for API clients and tests. `describe` is
    /// the sentence a person reads; this is what code branches on.
    pub fn mode(&self) -> &'static str {
        match self {
            ControlAuth::BearerToken(_) => "token",
            ControlAuth::LoopbackOnly => "loopback",
            ControlAuth::Disabled => "disabled",
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            ControlAuth::BearerToken(_) => "the control token is required",
            ControlAuth::LoopbackOnly => "anyone on this machine can create people",
            ControlAuth::Disabled => "creating people is off: set a control token first",
        }
    }
}

/// Comparison that does not stop at the first differing byte, so a token
/// cannot be recovered by timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
    const LAN: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));

    #[test]
    fn a_token_always_wins() {
        let auth = ControlAuth::resolve(Some("s3cret".into()), LAN);
        assert!(auth.permits(Some("Bearer s3cret")));
        assert!(auth.permits(Some("bearer s3cret")));
        assert!(!auth.permits(Some("Bearer wrong")));
        assert!(!auth.permits(None));
    }

    #[test]
    fn no_token_allows_loopback_and_refuses_lan() {
        assert!(ControlAuth::resolve(None, LOOPBACK).permits(None));
        assert!(!ControlAuth::resolve(None, LAN).permits(Some("Bearer anything")));
        assert_eq!(
            ControlAuth::resolve(Some("  ".into()), LAN),
            ControlAuth::Disabled
        );
    }

    #[test]
    fn each_mode_has_a_word_and_a_sentence() {
        for auth in [
            ControlAuth::resolve(Some("t".into()), LAN),
            ControlAuth::resolve(None, LOOPBACK),
            ControlAuth::resolve(None, LAN),
        ] {
            assert!(!auth.mode().is_empty());
            assert!(auth.describe().len() > 10, "{:?}", auth.describe());
            assert!(
                !auth.describe().contains('_'),
                "{:?} reads like a variable name",
                auth.describe()
            );
        }
        assert_eq!(ControlAuth::resolve(None, LOOPBACK).mode(), "loopback");
    }

    #[test]
    fn constant_time_eq_matches_only_identical_bytes() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}

#[cfg(test)]
mod read_tests {
    use super::*;

    const CONTROL: &str = "control-secret";
    const READING: &str = "read-secret";

    fn control() -> ControlAuth {
        ControlAuth::BearerToken(CONTROL.to_string())
    }

    #[test]
    fn without_a_read_token_anyone_who_can_reach_it_may_read() {
        let reads = ReadAuth::resolve(None);
        assert_eq!(reads, ReadAuth::Open);
        assert!(reads.permits(None, &control()));
        assert!(!reads.needs_token());
    }

    #[test]
    fn an_empty_read_token_is_no_token_at_all() {
        // Otherwise `ISLAND_READ_TOKEN=` in a unit file would lock out
        // every client with a secret nobody can type.
        assert_eq!(ReadAuth::resolve(Some("   ".into())), ReadAuth::Open);
    }

    #[test]
    fn with_a_read_token_a_request_without_one_is_refused() {
        let reads = ReadAuth::resolve(Some(READING.into()));
        assert!(!reads.permits(None, &control()));
        assert!(!reads.permits(Some("Bearer wrong"), &control()));
        assert!(
            !reads.permits(Some(READING), &control()),
            "the scheme matters"
        );
        assert!(reads.permits(Some("Bearer read-secret"), &control()));
        assert!(reads.permits(Some("bearer read-secret"), &control()));
    }

    #[test]
    fn the_control_token_is_enough_to_read_with() {
        // An operator holding the token that pauses the island should not
        // need a second one to watch it.
        let reads = ReadAuth::resolve(Some(READING.into()));
        assert!(reads.permits(Some("Bearer control-secret"), &control()));
    }

    #[test]
    fn a_loopback_control_mode_does_not_open_a_locked_read() {
        // `ControlAuth::LoopbackOnly::permits` answers true for everyone,
        // because on loopback everyone is this machine. That must not leak
        // into the read gate: a server with a read token and no control
        // token still refuses an unauthenticated read.
        let reads = ReadAuth::resolve(Some(READING.into()));
        let loopback = ControlAuth::LoopbackOnly;
        assert!(!reads.permits(None, &loopback));
        assert!(!reads.permits(Some("Bearer wrong"), &loopback));
        assert!(reads.permits(Some("Bearer read-secret"), &loopback));
    }

    #[test]
    fn a_bearer_token_is_read_the_same_way_for_reads_and_writes() {
        assert_eq!(bearer(Some("Bearer  padded  ")), Some("padded"));
        assert_eq!(bearer(Some("Basic abc")), None);
        assert_eq!(bearer(Some("Bearer")), None);
        assert_eq!(bearer(None), None);
    }
}
