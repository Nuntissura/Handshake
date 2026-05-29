//! MT-036: Environment manifest.
//!
//! Acceptance: manifest explains run context **without exposing secrets**.
//!
//! The manifest accepts a key/value bag but enforces a static allowlist of
//! field names that may appear in an exportable manifest. Any other key is
//! rejected at construction time and surfaces as a typed error, so secret
//! material (`API_KEY`, `*_TOKEN`, `*_PASSWORD`, etc.) cannot accidentally
//! be persisted. The allowlist itself is intentionally small and visible.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const ENVIRONMENT_FIELD_ALLOWLIST: &[&str] = &[
    "os_family",
    "os_version",
    "arch",
    "host_kernel",
    "rustc_version",
    "handshake_core_version",
    "adapter_tier",
    "git_commit_short",
    "git_branch",
    "ci_provider",
    "timezone",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentManifest {
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentManifestError {
    KeyNotAllowed { key: String },
    EmptyKey,
}

impl std::fmt::Display for EnvironmentManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyNotAllowed { key } => write!(
                f,
                "environment manifest key '{key}' is not in ENVIRONMENT_FIELD_ALLOWLIST"
            ),
            Self::EmptyKey => write!(f, "environment manifest key must not be empty"),
        }
    }
}

impl std::error::Error for EnvironmentManifestError {}

impl EnvironmentManifest {
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), EnvironmentManifestError> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(EnvironmentManifestError::EmptyKey);
        }
        if !ENVIRONMENT_FIELD_ALLOWLIST.contains(&key.as_str()) {
            return Err(EnvironmentManifestError::KeyNotAllowed { key });
        }
        self.fields.insert(key, value.into());
        Ok(())
    }
}

impl Default for EnvironmentManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlisted_keys_accepted() {
        let mut m = EnvironmentManifest::new();
        m.insert("os_family", "windows").unwrap();
        m.insert("arch", "x86_64").unwrap();
        m.insert("rustc_version", "1.85.0").unwrap();
        assert_eq!(m.fields.len(), 3);
    }

    #[test]
    fn secret_like_keys_rejected() {
        let mut m = EnvironmentManifest::new();
        for bad in [
            "API_KEY",
            "SECRET_TOKEN",
            "password",
            "AWS_SECRET_ACCESS_KEY",
        ] {
            let err = m.insert(bad, "value").unwrap_err();
            assert!(matches!(
                err,
                EnvironmentManifestError::KeyNotAllowed { .. }
            ));
        }
        assert!(m.fields.is_empty());
    }

    #[test]
    fn empty_key_rejected() {
        let mut m = EnvironmentManifest::new();
        assert_eq!(
            m.insert("", "x").unwrap_err(),
            EnvironmentManifestError::EmptyKey
        );
    }
}
