//! MT-128 (part 1/2): Operator Secrets Vault for cloud API keys.
//!
//! Per HBR-INT-005 lane normalisation: cloud API keys MUST be
//! retrievable per-call from a vault, never logged, and the vault
//! itself is operator-managed. The production impl wraps the OS
//! keychain via the `keyring` crate (deferred to follow-on
//! alongside the dependency add); the in-memory impl in this
//! module backs unit tests and early integration.
//!
//! The vault is the concrete impl that satisfies the
//! [`super::openai_byok::ApiKeyProvider`] trait shape; the
//! per-runtime `Arc<dyn ApiKeyProvider>` constructor argument
//! accepts any `SecretsVault::api_key_provider_for(&self, lane:
//! &str)` adapter, including the in-memory test impl.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use super::openai_byok::{ApiKeyProvider, OpenAiByokError};

#[derive(Debug, Error)]
pub enum SecretsVaultError {
    #[error("lane id must not be empty")]
    EmptyLaneId,
    #[error("secret value must not be empty")]
    EmptySecretValue,
    #[error("no secret stored for lane {0}")]
    NoSecretForLane(String),
    #[error("internal vault lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("keychain backend error: {0}")]
    KeychainBackend(String),
}

/// Vault storage abstraction. Concrete impls:
/// - [`InMemorySecretsVault`] (this module; tests + early integration).
/// - OsKeychainSecretsVault (follow-on, wraps the `keyring` crate).
pub trait SecretsVault: Send + Sync {
    fn put(&self, lane: &str, secret: String) -> Result<(), SecretsVaultError>;
    fn get(&self, lane: &str) -> Result<String, SecretsVaultError>;
    fn delete(&self, lane: &str) -> Result<(), SecretsVaultError>;
    fn list_lanes(&self) -> Result<Vec<String>, SecretsVaultError>;
}

/// In-memory impl. NOT for production: secrets live in process
/// memory only. The OS-keychain impl is the production target.
pub struct InMemorySecretsVault {
    state: RwLock<HashMap<String, String>>,
}

impl Default for InMemorySecretsVault {
    fn default() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }
}

impl SecretsVault for InMemorySecretsVault {
    fn put(&self, lane: &str, secret: String) -> Result<(), SecretsVaultError> {
        if lane.trim().is_empty() {
            return Err(SecretsVaultError::EmptyLaneId);
        }
        if secret.is_empty() {
            return Err(SecretsVaultError::EmptySecretValue);
        }
        let mut guard = self
            .state
            .write()
            .map_err(|err| SecretsVaultError::LockPoisoned(err.to_string()))?;
        guard.insert(lane.to_string(), secret);
        Ok(())
    }

    fn get(&self, lane: &str) -> Result<String, SecretsVaultError> {
        if lane.trim().is_empty() {
            return Err(SecretsVaultError::EmptyLaneId);
        }
        let guard = self
            .state
            .read()
            .map_err(|err| SecretsVaultError::LockPoisoned(err.to_string()))?;
        guard
            .get(lane)
            .cloned()
            .ok_or_else(|| SecretsVaultError::NoSecretForLane(lane.to_string()))
    }

    fn delete(&self, lane: &str) -> Result<(), SecretsVaultError> {
        if lane.trim().is_empty() {
            return Err(SecretsVaultError::EmptyLaneId);
        }
        let mut guard = self
            .state
            .write()
            .map_err(|err| SecretsVaultError::LockPoisoned(err.to_string()))?;
        guard.remove(lane);
        Ok(())
    }

    fn list_lanes(&self) -> Result<Vec<String>, SecretsVaultError> {
        let guard = self
            .state
            .read()
            .map_err(|err| SecretsVaultError::LockPoisoned(err.to_string()))?;
        let mut lanes: Vec<String> = guard.keys().cloned().collect();
        lanes.sort();
        Ok(lanes)
    }
}

/// Adapter that exposes a [`SecretsVault`] lane as an
/// [`ApiKeyProvider`] for the BYOK runtimes (MT-125 / MT-126).
/// Construct one per lane and pass `Arc::new(adapter)` to
/// `OpenAiByokRuntime::new` / `AnthropicByokRuntime::new`.
pub struct VaultApiKeyProvider<V: SecretsVault> {
    vault: Arc<V>,
    lane: String,
}

impl<V: SecretsVault> VaultApiKeyProvider<V> {
    pub fn new(vault: Arc<V>, lane: impl Into<String>) -> Self {
        Self {
            vault,
            lane: lane.into(),
        }
    }
}

impl<V: SecretsVault> std::fmt::Debug for VaultApiKeyProvider<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultApiKeyProvider")
            .field("lane", &self.lane)
            .field("vault", &"<Arc<dyn SecretsVault>>")
            .finish()
    }
}

impl<V: SecretsVault + 'static> ApiKeyProvider for VaultApiKeyProvider<V> {
    fn fetch_api_key(&self) -> Result<String, OpenAiByokError> {
        self.vault
            .get(&self.lane)
            .map_err(|err| OpenAiByokError::ApiKeyFetch(format!("{err}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_round_trips_secret_value() {
        let vault = InMemorySecretsVault::default();
        vault.put("openai", "sk-test-value".to_string()).unwrap();
        let retrieved = vault.get("openai").unwrap();
        assert_eq!(retrieved, "sk-test-value");
    }

    #[test]
    fn get_unknown_lane_returns_no_secret_for_lane_error() {
        let vault = InMemorySecretsVault::default();
        let err = vault.get("unknown").expect_err("no secret");
        assert!(matches!(err, SecretsVaultError::NoSecretForLane(_)));
    }

    #[test]
    fn empty_lane_id_rejected_on_put_get_delete() {
        let vault = InMemorySecretsVault::default();
        assert!(matches!(
            vault.put("", "x".to_string()).unwrap_err(),
            SecretsVaultError::EmptyLaneId
        ));
        assert!(matches!(
            vault.get(" ").unwrap_err(),
            SecretsVaultError::EmptyLaneId
        ));
        assert!(matches!(
            vault.delete(" ").unwrap_err(),
            SecretsVaultError::EmptyLaneId
        ));
    }

    #[test]
    fn empty_secret_value_rejected() {
        let vault = InMemorySecretsVault::default();
        let err = vault.put("lane", "".to_string()).unwrap_err();
        assert!(matches!(err, SecretsVaultError::EmptySecretValue));
    }

    #[test]
    fn delete_removes_secret() {
        let vault = InMemorySecretsVault::default();
        vault.put("lane-a", "v".to_string()).unwrap();
        vault.delete("lane-a").unwrap();
        assert!(matches!(
            vault.get("lane-a").unwrap_err(),
            SecretsVaultError::NoSecretForLane(_)
        ));
    }

    #[test]
    fn list_lanes_returns_sorted_keys() {
        let vault = InMemorySecretsVault::default();
        vault.put("b", "v".to_string()).unwrap();
        vault.put("a", "v".to_string()).unwrap();
        vault.put("c", "v".to_string()).unwrap();
        assert_eq!(
            vault.list_lanes().unwrap(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn vault_api_key_provider_proxies_to_lane() {
        let vault = Arc::new(InMemorySecretsVault::default());
        vault.put("openai", "sk-proxy".to_string()).unwrap();
        let provider = VaultApiKeyProvider::new(vault.clone(), "openai");
        let fetched = provider.fetch_api_key().expect("fetch");
        assert_eq!(fetched, "sk-proxy");
    }

    #[test]
    fn vault_api_key_provider_debug_does_not_render_secret() {
        let vault = Arc::new(InMemorySecretsVault::default());
        vault.put("openai", "sk-DO-NOT-LOG-THIS".to_string()).unwrap();
        let provider = VaultApiKeyProvider::new(vault, "openai");
        let dbg = format!("{provider:?}");
        assert!(!dbg.contains("sk-DO-NOT-LOG-THIS"));
        assert!(dbg.contains("openai"));
    }
}
