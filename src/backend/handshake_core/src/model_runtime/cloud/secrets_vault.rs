//! MT-128: Operator Secrets Vault for cloud API keys.
//!
//! Per HBR-INT-005 lane normalisation: cloud API keys MUST be
//! retrievable per-call from a vault, never logged, and the vault
//! itself is operator-managed. The production impl
//! ([`OsKeychainSecretsVault`]) wraps the OS keychain via the
//! `keyring` crate (Windows Credential Manager, macOS Keychain,
//! Linux Secret Service + keyutils). The in-memory impl
//! ([`InMemorySecretsVault`]) backs early integration and tests
//! where touching the real OS keychain is not desired.
//!
//! Both impls satisfy the [`SecretsVault`] trait; either can be
//! wrapped by a [`VaultApiKeyProvider`] adapter to feed the
//! per-runtime `Arc<dyn ApiKeyProvider>` constructor argument of
//! the BYOK runtimes (MT-125 / MT-126).

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
/// - [`OsKeychainSecretsVault`] (this module; production, wraps the
///   `keyring` crate against the host OS credential store).
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

/// Default service namespace used by [`OsKeychainSecretsVault`]
/// when wiring against the host OS credential store. The vault
/// passes `(service_namespace, lane)` to `keyring::Entry::new`,
/// so collisions across Handshake installs on the same host are
/// avoided by varying the namespace (e.g. with an operator-chosen
/// suffix or a per-install UUID).
pub const HANDSHAKE_KEYCHAIN_SERVICE: &str = "handshake-cloud-keys";

/// Production [`SecretsVault`] impl backed by the host OS
/// credential store via the `keyring` crate.
///
/// Storage targets per platform (selected at compile time via the
/// `keyring` crate features wired in `Cargo.toml`):
/// - Windows: Windows Credential Manager (Generic Credentials).
/// - macOS / iOS: Apple Keychain.
/// - Linux / FreeBSD / OpenBSD: Secret Service (D-Bus) layered
///   over kernel keyutils for combo persistence.
///
/// The vault does not cache secrets in process memory: every
/// [`SecretsVault::get`] call round-trips through the OS keychain
/// API. Errors from the underlying `keyring::Entry` API are
/// mapped to [`SecretsVaultError::KeychainBackend`], except for
/// `keyring::Error::NoEntry` which maps to
/// [`SecretsVaultError::NoSecretForLane`].
pub struct OsKeychainSecretsVault {
    service_namespace: String,
}

impl OsKeychainSecretsVault {
    /// Construct a vault that maps lane ids onto entries in the
    /// host OS credential store under the supplied service
    /// namespace. Pass [`HANDSHAKE_KEYCHAIN_SERVICE`] for the
    /// default namespace, or a per-install suffix to isolate
    /// multiple Handshake installs on the same host.
    pub fn new(service_namespace: impl Into<String>) -> Self {
        Self {
            service_namespace: service_namespace.into(),
        }
    }

    /// Expose the configured service namespace. Useful for
    /// diagnostics and for tests that need to interrogate the
    /// keychain through the same `keyring::Entry` API.
    pub fn service_namespace(&self) -> &str {
        &self.service_namespace
    }

    /// Build a fresh `keyring::Entry` for the given lane. Each
    /// call materialises a new entry: the underlying credential
    /// builder uses `(service_namespace, lane)` as the identity
    /// pair so repeated calls resolve to the same OS credential.
    fn entry(&self, lane: &str) -> Result<keyring::Entry, SecretsVaultError> {
        keyring::Entry::new(&self.service_namespace, lane)
            .map_err(|err| SecretsVaultError::KeychainBackend(err.to_string()))
    }
}

impl SecretsVault for OsKeychainSecretsVault {
    fn put(&self, lane: &str, secret: String) -> Result<(), SecretsVaultError> {
        if lane.trim().is_empty() {
            return Err(SecretsVaultError::EmptyLaneId);
        }
        if secret.is_empty() {
            return Err(SecretsVaultError::EmptySecretValue);
        }
        let entry = self.entry(lane)?;
        entry
            .set_password(&secret)
            .map_err(|err| SecretsVaultError::KeychainBackend(err.to_string()))
    }

    fn get(&self, lane: &str) -> Result<String, SecretsVaultError> {
        if lane.trim().is_empty() {
            return Err(SecretsVaultError::EmptyLaneId);
        }
        let entry = self.entry(lane)?;
        match entry.get_password() {
            Ok(secret) => Ok(secret),
            Err(keyring::Error::NoEntry) => {
                Err(SecretsVaultError::NoSecretForLane(lane.to_string()))
            }
            Err(err) => Err(SecretsVaultError::KeychainBackend(err.to_string())),
        }
    }

    fn delete(&self, lane: &str) -> Result<(), SecretsVaultError> {
        if lane.trim().is_empty() {
            return Err(SecretsVaultError::EmptyLaneId);
        }
        let entry = self.entry(lane)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            // Idempotent delete: NoEntry is success-equivalent so
            // callers can safely call delete repeatedly during
            // cleanup or operator-initiated key rotation.
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(SecretsVaultError::KeychainBackend(err.to_string())),
        }
    }

    fn list_lanes(&self) -> Result<Vec<String>, SecretsVaultError> {
        // The OS keychain backends exposed by the `keyring` crate
        // do not provide a portable enumeration API: Windows
        // Credential Manager lacks a per-namespace iterator at
        // this abstraction layer, and the macOS / Linux backends
        // require platform-specific code paths outside the
        // crate's trait surface. Callers that need lane discovery
        // should maintain a parallel lane registry (e.g. in the
        // Postgres event ledger or a Handshake-owned file) and
        // treat the vault as the secret-value store only.
        Ok(Vec::new())
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

    // -----------------------------------------------------------
    // OsKeychainSecretsVault unit tests
    //
    // These tests exercise the production code path (which goes
    // through `keyring::Entry::new` + `set_password` / `get_password`
    // / `delete_credential`) but reroute the underlying credential
    // builder to the keyring crate's mock backend so the host OS
    // credential store is never touched during `cargo test`.
    //
    // The mock backend uses `CredentialPersistence::EntryOnly`:
    // each `Entry::new(svc, user)` materialises a fresh
    // `MockCredential` with no shared state across calls. That is
    // perfect for validating the wiring (lane / secret guards,
    // error mapping for `NoEntry`, idempotent delete, listing) but
    // cannot be used to demonstrate a put-then-get round-trip
    // through fresh entries. The real round-trip is covered by
    // the platform-gated integration test in
    // `tests/cloud_os_keychain_vault_tests.rs`, which talks to the
    // actual OS credential store.
    // -----------------------------------------------------------

    use std::sync::Once;

    /// Install the keyring mock backend exactly once per test
    /// process. `set_default_credential_builder` itself tolerates
    /// repeated calls, but routing through `Once` keeps the
    /// install side-effect bounded and explicit.
    fn install_mock_keyring_backend() {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        });
    }

    #[test]
    fn os_keychain_vault_construct_records_service_namespace() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-ctor-test");
        assert_eq!(vault.service_namespace(), "handshake-os-keychain-ctor-test");
    }

    #[test]
    fn os_keychain_vault_default_service_constant_is_stable() {
        // Pin the namespace constant so the integration test and
        // any operator-facing docs stay in sync.
        assert_eq!(HANDSHAKE_KEYCHAIN_SERVICE, "handshake-cloud-keys");
    }

    #[test]
    fn os_keychain_vault_rejects_empty_lane_on_put() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-empty-lane-put");
        let err = vault
            .put("", "sk-value".to_string())
            .expect_err("empty lane");
        assert!(matches!(err, SecretsVaultError::EmptyLaneId));
        // Whitespace-only lane is also rejected.
        let err = vault
            .put("   ", "sk-value".to_string())
            .expect_err("whitespace lane");
        assert!(matches!(err, SecretsVaultError::EmptyLaneId));
    }

    #[test]
    fn os_keychain_vault_rejects_empty_lane_on_get_and_delete() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-empty-lane-getdel");
        assert!(matches!(
            vault.get("").unwrap_err(),
            SecretsVaultError::EmptyLaneId
        ));
        assert!(matches!(
            vault.delete(" ").unwrap_err(),
            SecretsVaultError::EmptyLaneId
        ));
    }

    #[test]
    fn os_keychain_vault_rejects_empty_secret_value() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-empty-secret");
        let err = vault
            .put("openai-empty-secret", "".to_string())
            .expect_err("empty secret");
        assert!(matches!(err, SecretsVaultError::EmptySecretValue));
    }

    #[test]
    fn os_keychain_vault_get_unknown_lane_returns_no_secret_for_lane() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-get-unknown");
        // Mock backend treats every fresh entry as having no
        // password; the vault must map `keyring::Error::NoEntry`
        // to `SecretsVaultError::NoSecretForLane(lane)` and
        // preserve the lane id in the error payload.
        let err = vault
            .get("openai-unknown-lane")
            .expect_err("unknown lane should error");
        match err {
            SecretsVaultError::NoSecretForLane(lane) => {
                assert_eq!(lane, "openai-unknown-lane");
            }
            other => panic!("expected NoSecretForLane, got {other:?}"),
        }
    }

    #[test]
    fn os_keychain_vault_delete_on_missing_lane_is_idempotent() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-delete-missing");
        // Operator-initiated key rotation issues delete first;
        // a non-existent lane must succeed so the rotation
        // workflow is not blocked on prior state.
        vault
            .delete("never-stored-lane")
            .expect("delete on missing lane must be idempotent");
        // Calling delete twice in a row also stays Ok.
        vault
            .delete("never-stored-lane")
            .expect("repeat delete on missing lane must be idempotent");
    }

    #[test]
    fn os_keychain_vault_put_against_mock_backend_returns_ok() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-put-ok");
        // The mock backend's CredentialPersistence is EntryOnly,
        // so we cannot read the value back through a fresh entry
        // in this unit test. We can still pin that the production
        // put() path drives `set_password` without surfacing an
        // error, which exercises the full validation + entry-
        // construction + set_password code path against the real
        // `keyring::Entry` API. The full put/get/delete round
        // trip is covered by the platform-gated integration test.
        vault
            .put("openai-put-ok", "sk-mock-stored".to_string())
            .expect("put must succeed against mock backend");
    }

    #[test]
    fn os_keychain_vault_list_lanes_returns_empty_per_doc_contract() {
        install_mock_keyring_backend();
        let vault = OsKeychainSecretsVault::new("handshake-os-keychain-list");
        // Documented contract: list_lanes always returns an empty
        // Vec on the OS keychain backend; callers must keep a
        // parallel lane registry if they need discovery.
        let lanes = vault.list_lanes().expect("list_lanes must succeed");
        assert!(lanes.is_empty(), "expected empty Vec, got {lanes:?}");
    }

    #[test]
    fn os_keychain_vault_is_send_and_sync_for_arc_dyn_use() {
        // Sanity: the production vault must be `Send + Sync`
        // because the cloud BYOK runtimes hand it around as
        // `Arc<dyn SecretsVault>` across async task boundaries.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OsKeychainSecretsVault>();
    }

    #[test]
    fn os_keychain_vault_satisfies_secrets_vault_object_safety() {
        // Pin object-safety: a downstream caller must be able to
        // erase the concrete type behind `Arc<dyn SecretsVault>`
        // exactly the way `VaultApiKeyProvider` does, so the
        // BYOK runtimes can swap impls at runtime.
        install_mock_keyring_backend();
        let vault: Arc<dyn SecretsVault> =
            Arc::new(OsKeychainSecretsVault::new("handshake-os-keychain-objsafe"));
        let err = vault
            .get("lane-that-does-not-exist")
            .expect_err("dyn dispatch on get must surface NoSecretForLane");
        assert!(matches!(err, SecretsVaultError::NoSecretForLane(_)));
    }
}
