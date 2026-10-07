//! Who may change anything through the dashboard. Ported from upstream
//! `apps/mk_cli/src/web/server.rs` (`ControlAuth`): a configured token always
//! wins; without one, writes are allowed only on a loopback bind, and a
//! non-loopback bind without a token refuses every write.

/// Environment variable holding the dashboard's bearer token.
pub const TOKEN_ENV: &str = "ISLAND_CONTROL_TOKEN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlAuth {
    /// Require `Authorization: Bearer <token>`.
    BearerToken(String),
    /// No token configured, bound to loopback: only this machine can reach it.
    LoopbackOnly,
    /// No token configured but reachable off-host: refuse every write.
    Disabled,
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
            ControlAuth::BearerToken(expected) => header
                .and_then(|value| {
                    let (scheme, rest) = (value.get(..7)?, value.get(7..)?);
                    scheme.eq_ignore_ascii_case("Bearer ").then_some(rest)
                })
                .is_some_and(|token| {
                    constant_time_eq(token.trim().as_bytes(), expected.as_bytes())
                }),
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
