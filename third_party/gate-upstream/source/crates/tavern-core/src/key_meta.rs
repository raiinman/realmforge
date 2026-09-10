// SPDX-License-Identifier: AGPL-3.0-only

//! Signing key metadata sidecar format.
//!
//! Each signing key has a `.meta` file (TOML) alongside it with project,
//! environment, and purpose information. The server reads this at startup
//! to warn when a demo key is used in a non-demo context.

use serde::Deserialize;

/// Metadata for a signing key, loaded from a `.meta` sidecar file.
#[derive(Debug, Clone, Deserialize)]
pub struct KeyMeta {
    pub project: String,
    pub environment: String,
    pub kid: String,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl KeyMeta {
    /// Load metadata from a `.meta` file alongside the key path.
    /// Returns `None` if the file doesn't exist (not an error — production
    /// keys may not have a sidecar).
    pub fn load(key_path: &std::path::Path) -> Option<Self> {
        let meta_path = format!("{}.meta", key_path.display());
        let content = std::fs::read_to_string(&meta_path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Whether this is a demo/test key (not for production).
    pub fn is_demo(&self) -> bool {
        matches!(
            self.environment.as_str(),
            "demo" | "test" | "development" | "dev"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_demo_meta() {
        // The keys/signing.pem.meta sidecar shipped with the repo.
        let key_path = std::path::Path::new("../../keys/signing.pem");
        let meta = KeyMeta::load(key_path).expect("should load demo key metadata");
        assert_eq!(meta.project, "tavern");
        assert_eq!(meta.environment, "demo");
        assert_eq!(meta.kid, "tavern-1");
        assert!(meta.is_demo());
    }

    #[test]
    fn load_missing_meta() {
        let key_path = std::path::Path::new("/nonexistent/key.pem");
        assert!(KeyMeta::load(key_path).is_none());
    }
}
