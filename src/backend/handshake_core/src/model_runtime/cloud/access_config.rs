//! MT-015: Operator cloud-model access configuration service.
//!
//! WP-KERNEL-004 built the cloud backends (BYOK runtimes, OS-keychain
//! secrets vault, official CLI bridge, consent gate). What was missing was
//! an operator-facing way to REACH them: a place to type a BYOK API key or
//! to see / start CLI-bridge (subscription-plan) login. This module is the
//! backend half of that surface. The native egui settings dialog
//! (`handshake_native::settings_dialog`) is the operator-facing half; it
//! talks to the [`crate::api::model_access`] HTTP routes that wrap this
//! service.
//!
//! ## Security invariants (MT-015 pre-implementation review decisions)
//!
//! * BYOK keys live ONLY in the OS-keychain vault
//!   ([`super::secrets_vault::OsKeychainSecretsVault`]). This service takes
//!   the key as a [`secrecy::SecretString`], exposes it exactly once at the
//!   `vault.put` boundary, and holds no copy. Nothing here logs, returns,
//!   serialises, or `Debug`-prints key material.
//! * Storing a key creates NO [`super::consent_gate::ConsentGate`] entry and
//!   NO ConsentReceipt (decision MED CONSENT). Configuring access is not the
//!   same as consenting to a cloud send; the first lane launch still hits the
//!   MT-006 fail-closed consent boundary.
//! * The enumeration surface is NON-SECRET. It reports each offered provider
//!   as `configured` / `unavailable` (mapping a missing key —
//!   "ProviderNotConfigured" — to `unavailable`, never an error) WITHOUT
//!   returning key material. It never calls `vault.list_lanes` (the OS
//!   keychain has no portable enumeration) and never uses SQLite.
//! * Gemini is not offered. It is not a variant of [`ByokProvider`] or
//!   [`CliBridgeProvider`], so the exclusion is enforced by construction,
//!   not by a runtime filter that a caller could bypass. The reset brief
//!   §6.11 lists Gemini as a valid cloud lane; the operator forbids offering
//!   it here because its CLI is being discontinued. That divergence is
//!   recorded so it is not silently "restored" later.
//! * CLI-bridge login is operator-initiated and uses ONLY the provider's own
//!   official login command (surfaced by [`CliBridgeProvider::login_command`])
//!   launched in a visible terminal by the native shell. Handshake never
//!   captures, stores, or automates the provider credentials that login
//!   establishes. Live logged-in / expired detection is DEFERRED per provider
//!   (no official whoami probe exists on the bridge yet).

use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use thiserror::Error;

use super::secrets_vault::{SecretsVault, SecretsVaultError};

/// BYOK providers Handshake OFFERS an operator-facing API-key entry for.
///
/// Adding a variant here is the ONLY way a provider becomes offerable for
/// BYOK, so Gemini's exclusion is a compile-time fact rather than a runtime
/// filter. The operator's own primary path is subscription plans via the CLI
/// bridge (see [`CliBridgeProvider`]); BYOK is available for other users.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ByokProvider {
    Anthropic,
    OpenAi,
}

impl ByokProvider {
    /// Every offered BYOK provider, in stable display order. Gemini is
    /// deliberately absent.
    pub const OFFERED: [ByokProvider; 2] = [ByokProvider::Anthropic, ByokProvider::OpenAi];

    /// Stable wire/id string (lower-kebab). Used in routes + the native
    /// author_id targets.
    pub fn id(self) -> &'static str {
        match self {
            ByokProvider::Anthropic => "anthropic",
            ByokProvider::OpenAi => "openai",
        }
    }

    /// Operator-facing label.
    pub fn label(self) -> &'static str {
        match self {
            ByokProvider::Anthropic => "Anthropic (Claude)",
            ByokProvider::OpenAi => "OpenAI (GPT)",
        }
    }

    /// The vault lane id the provider's key is stored under. Stable so the
    /// MT-006 CloudLane/BYOK backend can fetch the same lane via
    /// [`super::secrets_vault::VaultApiKeyProvider`].
    pub fn vault_lane(self) -> &'static str {
        match self {
            ByokProvider::Anthropic => "cloud-byok-anthropic",
            ByokProvider::OpenAi => "cloud-byok-openai",
        }
    }

    /// Parse a provider id. Returns `None` for unknown ids AND for ids that
    /// are deliberately not offered (e.g. `"gemini"`), so an excluded
    /// provider can never be configured through this surface.
    pub fn from_id(id: &str) -> Option<ByokProvider> {
        match id {
            "anthropic" => Some(ByokProvider::Anthropic),
            "openai" => Some(ByokProvider::OpenAi),
            _ => None,
        }
    }
}

/// CLI-bridge providers Handshake offers subscription-plan login status for.
/// This is the operator's PRIMARY path (Claude Code + GPT/Codex via the
/// official CLI bridge). Gemini is not a variant (excluded by construction).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CliBridgeProvider {
    ClaudeCode,
    Codex,
}

impl CliBridgeProvider {
    /// Every offered CLI-bridge provider, in stable display order.
    pub const OFFERED: [CliBridgeProvider; 2] =
        [CliBridgeProvider::ClaudeCode, CliBridgeProvider::Codex];

    pub fn id(self) -> &'static str {
        match self {
            CliBridgeProvider::ClaudeCode => "claude_code",
            CliBridgeProvider::Codex => "codex",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CliBridgeProvider::ClaudeCode => "Claude Code",
            CliBridgeProvider::Codex => "GPT / Codex CLI",
        }
    }

    pub fn from_id(id: &str) -> Option<CliBridgeProvider> {
        match id {
            "claude_code" => Some(CliBridgeProvider::ClaudeCode),
            "codex" => Some(CliBridgeProvider::Codex),
            _ => None,
        }
    }

    /// The provider's OWN official login command, to be launched
    /// operator-initiated in a visible terminal by the native shell.
    ///
    /// Handshake NEVER captures or stores the credentials this command
    /// establishes; it only starts the provider's official interactive flow.
    /// The exact login sub-command is best-effort (there is no network to
    /// re-verify vendor docs here); the operator sees the full command before
    /// it runs and can adjust it. Getting the sub-command wrong is a
    /// usability nit, not a security issue — no credential is handled by
    /// Handshake either way.
    pub fn login_command(self) -> OfficialLoginCommand {
        match self {
            CliBridgeProvider::ClaudeCode => OfficialLoginCommand {
                program: "claude",
                args: &["/login"],
                hint: "Starts the official Claude Code CLI login. Handshake stores no credential; \
                       your Claude subscription session lives in the Claude Code CLI.",
            },
            CliBridgeProvider::Codex => OfficialLoginCommand {
                program: "codex",
                args: &["login"],
                hint: "Starts the official Codex/GPT CLI login. Handshake stores no credential; \
                       your ChatGPT/Codex session lives in the Codex CLI.",
            },
        }
    }
}

/// A provider's own official login command, launched operator-initiated in a
/// visible terminal. Non-secret static data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct OfficialLoginCommand {
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub hint: &'static str,
}

/// Non-secret access status for one provider. `ProviderNotConfigured`
/// (a missing key) maps to [`ProviderAccessStatus::Unavailable`], never to an
/// error, so the MT-012 model picker can list an un-keyed provider as simply
/// unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAccessStatus {
    Configured,
    Unavailable,
}

/// One non-secret BYOK enumeration row (never carries key material).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ByokAccessRow {
    pub provider: &'static str,
    pub label: &'static str,
    pub status: ProviderAccessStatus,
}

/// One non-secret CLI-bridge enumeration row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CliBridgeAccessRow {
    pub provider: &'static str,
    pub label: &'static str,
    pub login: OfficialLoginCommand,
}

/// The full non-secret enumeration surface consumed by MT-012's model picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CloudAccessEnumeration {
    pub byok: Vec<ByokAccessRow>,
    pub cli_bridge: Vec<CliBridgeAccessRow>,
    /// Providers deliberately NOT offered here, surfaced so the exclusion is
    /// visible (and testable) rather than a silent gap.
    pub excluded: Vec<&'static str>,
}

/// NON-SECRET view of which providers are configured. Implementations answer
/// per-provider status WITHOUT returning key material; they never call
/// `vault.list_lanes` and never touch SQLite.
pub trait ProviderAccessRegistry: Send + Sync {
    fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus;
}

/// Production registry: derives `configured` / `unavailable` from the presence
/// of a key in the OS-keychain vault for the provider's lane. The OS keychain
/// exposes no presence-only probe, so this reads the secret value to decide
/// presence and DROPS it immediately — it never returns, logs, or stores the
/// key. `NoSecretForLane` (ProviderNotConfigured) maps to `Unavailable`; any
/// other vault error also maps to `Unavailable` (fail-closed: a backend error
/// must not make an unconfigured provider look available, nor abort the whole
/// enumeration).
pub struct VaultBackedAccessRegistry {
    vault: Arc<dyn SecretsVault>,
}

impl VaultBackedAccessRegistry {
    pub fn new(vault: Arc<dyn SecretsVault>) -> Self {
        Self { vault }
    }
}

impl ProviderAccessRegistry for VaultBackedAccessRegistry {
    fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus {
        match self.vault.get(provider.vault_lane()) {
            Ok(secret) => {
                // Presence only: drop the key immediately; never surface it.
                drop(secret);
                ProviderAccessStatus::Configured
            }
            Err(_) => ProviderAccessStatus::Unavailable,
        }
    }
}

/// In-memory NON-SECRET registry used by unit tests to prove the enumeration
/// mapping in isolation from the OS keychain. Holds only which providers are
/// marked configured (booleans), never key material.
#[derive(Default)]
pub struct InMemoryAccessRegistry {
    configured: RwLock<BTreeSet<ByokProvider>>,
}

impl InMemoryAccessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_configured(&self, provider: ByokProvider, configured: bool) {
        let mut guard = self.configured.write().expect("registry lock");
        if configured {
            guard.insert(provider);
        } else {
            guard.remove(&provider);
        }
    }
}

impl ProviderAccessRegistry for InMemoryAccessRegistry {
    fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus {
        if self.configured.read().expect("registry lock").contains(&provider) {
            ProviderAccessStatus::Configured
        } else {
            ProviderAccessStatus::Unavailable
        }
    }
}

/// Enumerate every offered BYOK provider's non-secret status. Gemini can never
/// appear because it is not in [`ByokProvider::OFFERED`].
pub fn enumerate_byok(registry: &dyn ProviderAccessRegistry) -> Vec<ByokAccessRow> {
    ByokProvider::OFFERED
        .iter()
        .map(|provider| ByokAccessRow {
            provider: provider.id(),
            label: provider.label(),
            status: registry.byok_status(*provider),
        })
        .collect()
}

/// Enumerate every offered CLI-bridge provider with its official login command.
pub fn enumerate_cli_bridge() -> Vec<CliBridgeAccessRow> {
    CliBridgeProvider::OFFERED
        .iter()
        .map(|provider| CliBridgeAccessRow {
            provider: provider.id(),
            label: provider.label(),
            login: provider.login_command(),
        })
        .collect()
}

/// Build the full non-secret enumeration surface for the model picker.
pub fn enumerate(registry: &dyn ProviderAccessRegistry) -> CloudAccessEnumeration {
    CloudAccessEnumeration {
        byok: enumerate_byok(registry),
        cli_bridge: enumerate_cli_bridge(),
        // Deliberately not offered (reset brief §6.11 divergence — CLI
        // deprecating). Surfaced so the exclusion is visible + testable.
        excluded: vec!["gemini"],
    }
}

#[derive(Debug, Error)]
pub enum AccessConfigError {
    #[error("provider {0} is not offered for BYOK configuration")]
    ProviderNotOffered(String),
    #[error("API key must not be empty")]
    EmptyKey,
    #[error(
        "OS keychain feature is not enabled; refusing to persist a cloud key \
         (there is no plaintext fallback)"
    )]
    KeychainUnavailable,
    #[error("secrets vault error: {0}")]
    Vault(#[from] SecretsVaultError),
}

/// The operator cloud-model access service: the thin, security-critical seam
/// between the operator config surface and the OS-keychain vault.
///
/// It holds `Arc<dyn SecretsVault>` (no key material) plus a stable
/// `vault_kind` label so a leak test can assert the wired vault is the OS
/// keychain and not the in-memory impl. Its `Debug` never renders a key
/// because it never holds one.
pub struct CloudModelAccess {
    vault: Arc<dyn SecretsVault>,
    vault_kind: &'static str,
}

impl std::fmt::Debug for CloudModelAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudModelAccess")
            .field("vault", &"<Arc<dyn SecretsVault>>")
            .field("vault_kind", &self.vault_kind)
            .finish()
    }
}

impl CloudModelAccess {
    /// Wire the service to an explicit vault. `vault_kind` is a stable label
    /// (e.g. `"OsKeychainSecretsVault"`) used by the leak test to prove the
    /// production wiring is the OS keychain. Prefer [`Self::production`] in
    /// app code.
    pub fn with_vault(vault: Arc<dyn SecretsVault>, vault_kind: &'static str) -> Self {
        Self { vault, vault_kind }
    }

    /// Production wiring. With the default `os-keychain` feature this returns a
    /// service backed by [`super::secrets_vault::OsKeychainSecretsVault`].
    /// WITHOUT `os-keychain` it REFUSES (returns
    /// [`AccessConfigError::KeychainUnavailable`]) rather than silently
    /// falling back to an in-memory / plaintext store — a cloud key must never
    /// be persisted outside the OS keychain.
    pub fn production() -> Result<Self, AccessConfigError> {
        #[cfg(feature = "os-keychain")]
        {
            let vault = super::secrets_vault::OsKeychainSecretsVault::new(
                super::secrets_vault::HANDSHAKE_KEYCHAIN_SERVICE,
            );
            Ok(Self::with_vault(Arc::new(vault), "OsKeychainSecretsVault"))
        }
        #[cfg(not(feature = "os-keychain"))]
        {
            Err(AccessConfigError::KeychainUnavailable)
        }
    }

    /// Stable label of the wired vault impl (for leak-test assertions).
    pub fn vault_kind(&self) -> &'static str {
        self.vault_kind
    }

    /// A non-secret registry view over this service's vault, for enumeration.
    pub fn registry(&self) -> VaultBackedAccessRegistry {
        VaultBackedAccessRegistry::new(self.vault.clone())
    }

    /// Store a BYOK API key for `provider` ONLY in the OS-keychain vault.
    ///
    /// The key is exposed exactly once, at the `vault.put` boundary; no copy is
    /// retained. This creates NO ConsentGate entry and NO ConsentReceipt — the
    /// first cloud lane launch still hits the MT-006 fail-closed consent gate.
    pub fn store_byok_key(
        &self,
        provider: ByokProvider,
        api_key: &SecretString,
    ) -> Result<(), AccessConfigError> {
        if api_key.expose_secret().trim().is_empty() {
            return Err(AccessConfigError::EmptyKey);
        }
        // Expose once, hand straight to the vault. `vault.put` moves the value
        // into the OS credential store; nothing here keeps it.
        self.vault
            .put(provider.vault_lane(), api_key.expose_secret().to_owned())?;
        Ok(())
    }

    /// Remove (or rotate) a provider's BYOK key. Idempotent — removing a key
    /// that was never stored succeeds — so a leaked key can be rotated without
    /// first checking whether one exists.
    pub fn remove_byok_key(&self, provider: ByokProvider) -> Result<(), AccessConfigError> {
        self.vault.delete(provider.vault_lane())?;
        Ok(())
    }

    /// Fetch a stored BYOK key back out of the vault for USE by the cloud
    /// backend (round-trip). Returns the key on success or a vault error. This
    /// is the ONLY method that returns key material and exists so the MT-006
    /// CloudLane/BYOK backend (via `VaultApiKeyProvider`) — and the leak test —
    /// can prove the key still round-trips. Callers MUST NOT log the result.
    pub fn fetch_byok_key(
        &self,
        provider: ByokProvider,
    ) -> Result<String, AccessConfigError> {
        Ok(self.vault.get(provider.vault_lane())?)
    }

    /// Non-secret status for one BYOK provider (ProviderNotConfigured ->
    /// Unavailable). Convenience wrapper over the vault-backed registry.
    pub fn byok_status(&self, provider: ByokProvider) -> ProviderAccessStatus {
        self.registry().byok_status(provider)
    }

    /// The full non-secret enumeration surface.
    pub fn enumerate(&self) -> CloudAccessEnumeration {
        enumerate(&self.registry())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Provider identity + Gemini exclusion (enforced by construction).
    // ------------------------------------------------------------------

    #[test]
    fn byok_providers_are_anthropic_and_openai_only_no_gemini() {
        let ids: Vec<&str> = ByokProvider::OFFERED.iter().map(|p| p.id()).collect();
        assert_eq!(ids, vec!["anthropic", "openai"]);
        assert!(!ids.contains(&"gemini"), "Gemini must never be offered for BYOK");
        // No id string can parse to a Gemini provider (there is no variant).
        assert!(ByokProvider::from_id("gemini").is_none());
        assert!(ByokProvider::from_id("google").is_none());
        assert!(ByokProvider::from_id("").is_none());
    }

    #[test]
    fn cli_bridge_providers_are_claude_and_codex_only_no_gemini() {
        let ids: Vec<&str> = CliBridgeProvider::OFFERED.iter().map(|p| p.id()).collect();
        assert_eq!(ids, vec!["claude_code", "codex"]);
        assert!(CliBridgeProvider::from_id("gemini_cli").is_none());
        assert!(CliBridgeProvider::from_id("gemini").is_none());
    }

    #[test]
    fn byok_vault_lanes_are_stable_and_distinct() {
        assert_eq!(ByokProvider::Anthropic.vault_lane(), "cloud-byok-anthropic");
        assert_eq!(ByokProvider::OpenAi.vault_lane(), "cloud-byok-openai");
        assert_ne!(
            ByokProvider::Anthropic.vault_lane(),
            ByokProvider::OpenAi.vault_lane()
        );
    }

    #[test]
    fn cli_bridge_login_commands_are_the_providers_own_official_commands() {
        let claude = CliBridgeProvider::ClaudeCode.login_command();
        assert_eq!(claude.program, "claude");
        let codex = CliBridgeProvider::Codex.login_command();
        assert_eq!(codex.program, "codex");
    }

    // ------------------------------------------------------------------
    // Enumeration API: ProviderNotConfigured -> Unavailable, no Gemini.
    // ------------------------------------------------------------------

    #[test]
    fn enumeration_maps_unconfigured_to_unavailable_and_configured_to_configured() {
        let registry = InMemoryAccessRegistry::new();
        // Nothing configured yet: both providers unavailable, not an error.
        let rows = enumerate_byok(&registry);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(
                row.status,
                ProviderAccessStatus::Unavailable,
                "unconfigured {} must be unavailable",
                row.provider
            );
        }

        // Configure OpenAI only.
        registry.set_configured(ByokProvider::OpenAi, true);
        let rows = enumerate_byok(&registry);
        let openai = rows.iter().find(|r| r.provider == "openai").unwrap();
        let anthropic = rows.iter().find(|r| r.provider == "anthropic").unwrap();
        assert_eq!(openai.status, ProviderAccessStatus::Configured);
        assert_eq!(anthropic.status, ProviderAccessStatus::Unavailable);
    }

    #[test]
    fn full_enumeration_never_offers_gemini_and_records_the_exclusion() {
        let registry = InMemoryAccessRegistry::new();
        let enumeration = enumerate(&registry);
        // BYOK rows carry no Gemini.
        assert!(enumeration.byok.iter().all(|r| r.provider != "gemini"));
        // CLI-bridge rows carry no Gemini.
        assert!(enumeration
            .cli_bridge
            .iter()
            .all(|r| r.provider != "gemini" && r.provider != "gemini_cli"));
        // The exclusion is explicit + visible.
        assert!(enumeration.excluded.contains(&"gemini"));
    }

    #[test]
    fn enumeration_is_json_serialisable_for_the_model_picker() {
        let registry = InMemoryAccessRegistry::new();
        registry.set_configured(ByokProvider::Anthropic, true);
        let enumeration = enumerate(&registry);
        let json = serde_json::to_value(&enumeration).expect("serialises");
        assert_eq!(json["byok"][0]["provider"], "anthropic");
        assert_eq!(json["byok"][0]["status"], "configured");
        assert_eq!(json["byok"][1]["status"], "unavailable");
        assert_eq!(json["excluded"][0], "gemini");
    }

    // ------------------------------------------------------------------
    // Store / round-trip / remove against the in-memory vault (fast unit
    // coverage; the real OS-keychain round-trip + leak proof is in
    // tests/cloud_byok_access_config_leak_tests.rs).
    // ------------------------------------------------------------------

    fn in_memory_service() -> CloudModelAccess {
        let vault = Arc::new(super::super::secrets_vault::InMemorySecretsVault::default());
        CloudModelAccess::with_vault(vault, "InMemorySecretsVault")
    }

    #[test]
    fn store_then_fetch_round_trips_the_key_only_through_the_vault() {
        let service = in_memory_service();
        let key = SecretString::from("sk-canary-unit".to_string());
        service.store_byok_key(ByokProvider::OpenAi, &key).unwrap();
        assert_eq!(
            service.fetch_byok_key(ByokProvider::OpenAi).unwrap(),
            "sk-canary-unit"
        );
        assert_eq!(
            service.byok_status(ByokProvider::OpenAi),
            ProviderAccessStatus::Configured
        );
        assert_eq!(
            service.byok_status(ByokProvider::Anthropic),
            ProviderAccessStatus::Unavailable
        );
    }

    #[test]
    fn empty_key_is_rejected_and_not_stored() {
        let service = in_memory_service();
        let err = service
            .store_byok_key(ByokProvider::OpenAi, &SecretString::from("   ".to_string()))
            .expect_err("empty key rejected");
        assert!(matches!(err, AccessConfigError::EmptyKey));
        assert_eq!(
            service.byok_status(ByokProvider::OpenAi),
            ProviderAccessStatus::Unavailable
        );
    }

    #[test]
    fn remove_is_idempotent_for_rotation() {
        let service = in_memory_service();
        // Remove a never-stored key: must succeed (rotation-safe).
        service.remove_byok_key(ByokProvider::Anthropic).unwrap();
        let key = SecretString::from("sk-rotate-me".to_string());
        service.store_byok_key(ByokProvider::Anthropic, &key).unwrap();
        service.remove_byok_key(ByokProvider::Anthropic).unwrap();
        assert_eq!(
            service.byok_status(ByokProvider::Anthropic),
            ProviderAccessStatus::Unavailable
        );
    }

    #[test]
    fn cloud_model_access_debug_never_holds_or_prints_key_material() {
        let service = in_memory_service();
        service
            .store_byok_key(
                ByokProvider::OpenAi,
                &SecretString::from("sk-DO-NOT-PRINT".to_string()),
            )
            .unwrap();
        let dbg = format!("{service:?}");
        assert!(!dbg.contains("sk-DO-NOT-PRINT"));
        assert!(dbg.contains("InMemorySecretsVault"));
    }

    #[test]
    fn saving_a_key_creates_no_consent_and_first_launch_still_fails_closed() {
        // Saving a BYOK key must NOT pre-approve consent. The access service holds no ConsentGate, so
        // storing a key cannot create an approval; we prove it behaviourally — a fresh gate (as at the
        // first cloud lane launch) still fails closed under a deny provider AFTER the key is stored.
        use super::super::consent_gate::{
            ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider,
        };
        struct Deny;
        impl ConsentProvider for Deny {
            fn prompt_for_decision(
                &self,
                _session: &str,
                _lane: &str,
            ) -> Result<ConsentDecision, ConsentGateError> {
                Ok(ConsentDecision::Denied)
            }
        }

        let service = in_memory_service();
        service
            .store_byok_key(
                ByokProvider::OpenAi,
                &SecretString::from("sk-no-consent".to_string()),
            )
            .unwrap();

        // The key is stored + usable...
        assert_eq!(
            service.byok_status(ByokProvider::OpenAi),
            ProviderAccessStatus::Configured
        );
        // ...but the first lane launch still hits the fail-closed MT-006 gate (no approval was created).
        let gate = ConsentGate::new();
        let first_launch =
            gate.check_or_prompt("session-mt015", ByokProvider::OpenAi.id(), &Deny);
        assert!(
            matches!(first_launch, Err(ConsentGateError::ConsentDenied { .. })),
            "saving a key must not grant consent; first launch must fail closed"
        );
    }
}
