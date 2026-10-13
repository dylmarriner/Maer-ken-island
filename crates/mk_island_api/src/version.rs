//! What a frontend asks first: which schema this server speaks, and what
//! it can do.
//!
//! Once the frontend stopped living on the backend's machine, the two
//! stopped being upgraded together. A desktop application installed on
//! somebody's laptop in March talks to a backend redeployed in June, and
//! the only question that matters is whether they still agree about the
//! shapes in [`crate::wire`]. `GET /api/version` is how it finds out
//! before it draws anything, instead of discovering it one missing field
//! at a time.

use serde::{Deserialize, Serialize};

/// The wire schema's version. Bumped when a change to [`crate::wire`] or
/// [`crate::request`] would make an older client read the new answers
/// wrongly: a removed field, a renamed one, a changed meaning or unit.
///
/// **Not** bumped for an added field. Every type here deserializes with
/// serde's default behaviour of ignoring what it does not know, so an
/// older client reading a newer server simply does not see the new field,
/// which is exactly what it did before the field existed.
pub const API_VERSION: u32 = 1;

/// The body of `GET /api/version`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerVersion {
    /// [`API_VERSION`] as this server was built with it.
    pub api_version: u32,
    /// The oldest schema version this server still answers correctly.
    /// A client whose own `API_VERSION` is at least this and no more than
    /// `api_version` can talk to it.
    pub api_version_minimum: u32,
    /// The `island` crate's own version, so a bug report can say which
    /// build it was.
    pub server_version: String,
    pub capabilities: Capabilities,
}

impl ServerVersion {
    /// Whether a client built against `client_api_version` can read this
    /// server, and why not when it cannot.
    ///
    /// Returns the sentence to show a person. It names both numbers,
    /// because "incompatible" without them sends somebody to the logs.
    pub fn compatibility(&self, client_api_version: u32) -> Result<(), String> {
        if client_api_version < self.api_version_minimum {
            return Err(format!(
                "This frontend speaks island API v{client_api_version} and the server at the \
                 other end has dropped support for anything below v{}. Update the frontend.",
                self.api_version_minimum
            ));
        }
        if client_api_version > self.api_version {
            return Err(format!(
                "This frontend speaks island API v{client_api_version} and the server at the \
                 other end speaks v{}. Update the server, or point the frontend at one that \
                 has been.",
                self.api_version
            ));
        }
        Ok(())
    }
}

/// What this particular server can do, as opposed to what the schema
/// allows.
///
/// A frontend reads these to decide what to offer rather than offering
/// everything and letting half of it fail. `island serve` without
/// `--scenario` has a stored population and no island at all, and a
/// desktop client pointed at one should say so on its first screen rather
/// than drawing an empty sea.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    /// An island is running: `/api/world` and everything under it answer.
    /// False on a human-only bootstrap, where only the stored population
    /// is served.
    pub world: bool,
    /// Which of the three control modes the server is in: `token`,
    /// `loopback` or `disabled`. A frontend uses this to decide whether to
    /// ask for a token, to offer writes freely, or to grey them out and
    /// say why.
    pub writes: String,
    /// Reads need a bearer token too. False on a server that is open to
    /// anyone who can reach it, which is the default and is correct on a
    /// trusted network and wrong off one.
    pub reads_need_token: bool,
    /// Browser origins allowed to call this server, for a web frontend
    /// hosted somewhere else. Empty means same-origin only, which is what
    /// a browser enforces without being told anything.
    pub allowed_origins: Vec<String>,
    /// The terrain is served as binary for a client that renders a
    /// heightfield rather than a picture.
    pub elevation: bool,
    /// A real computer bridge is attached, so humans may do the two
    /// actions that reach the outside world. Its results are deliberately
    /// outside the deterministic digest.
    pub computer_service: bool,
    /// A snapshot directory was configured, so `Snapshot` will write one
    /// instead of being refused.
    pub snapshots: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(min: u32, current: u32) -> ServerVersion {
        ServerVersion {
            api_version: current,
            api_version_minimum: min,
            server_version: "0.1.0".to_string(),
            capabilities: Capabilities::default(),
        }
    }

    #[test]
    fn a_client_inside_the_range_is_compatible() {
        assert!(server(1, 3).compatibility(1).is_ok());
        assert!(server(1, 3).compatibility(3).is_ok());
    }

    #[test]
    fn a_client_too_old_is_told_which_version_to_reach() {
        let message = server(2, 3).compatibility(1).unwrap_err();
        assert!(message.contains("v1"), "{message}");
        assert!(message.contains("v2"), "{message}");
        assert!(message.contains("Update the frontend"), "{message}");
    }

    #[test]
    fn a_client_newer_than_its_server_is_told_to_update_the_server() {
        let message = server(1, 2).compatibility(3).unwrap_err();
        assert!(message.contains("Update the server"), "{message}");
    }
}
