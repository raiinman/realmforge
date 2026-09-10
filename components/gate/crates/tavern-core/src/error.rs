// SPDX-License-Identifier: AGPL-3.0-only

//! Crate-wide error type.

use thiserror::Error;

/// Errors returned by `tavern-core`.
///
/// One variant per failure class. The enum grows as the crate gains
/// functionality; crypto variants are added alongside the SRP and JWT modules
/// in later milestones.
#[derive(Debug, Error)]
pub enum Error {
    /// A required configuration value was not provided.
    #[error("missing required configuration key: {0}")]
    MissingConfig(&'static str),

    /// A configuration value was present but invalid.
    #[error("invalid configuration for {key}: {reason}")]
    InvalidConfig { key: &'static str, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_variant_mentions_key() {
        let err = Error::MissingConfig("DATABASE_URL");
        let msg = err.to_string();
        assert!(msg.contains("DATABASE_URL"), "message was: {msg}");
    }

    #[test]
    fn invalid_variant_mentions_key_and_reason() {
        let err = Error::InvalidConfig {
            key: "BIND_ADDR",
            reason: "not a socket address".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("BIND_ADDR"), "message was: {msg}");
        assert!(msg.contains("not a socket address"), "message was: {msg}");
    }
}
