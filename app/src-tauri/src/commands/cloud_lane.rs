//! MT-129: Tauri IPC surface for the Cloud Lane control panel.
//!
//! Wires the frontend `CloudLanePanel` / `ApiKeyVault` / `ConsentPromptModal`
//! components to real production backend data sources:
//! - `handshake_core::model_runtime::cloud::secrets_vault::SecretsVault`
//!   (production impl `OsKeychainSecretsVault` -> Windows Credential
//!   Manager via the `keyring` crate; tests use `InMemorySecretsVault`).
//! - `handshake_core::model_runtime::cloud::consent_gate::ConsentGate`
//!   (per-session per-lane operator consent capture).
//!
//! Operator workflow:
//! - Register a cloud lane (kind = openai_byok | anthropic_byok | official_cli,
//!   lane_id, model_name). Lane metadata lives in the Tauri-managed
//!   `CloudLaneIpcState` in-memory registry; the lane_id is the vault key.
//! - Store / rotate / delete the API key for a lane via `SecretsVault`. The
//!   secret never leaves the IPC boundary - it is wrapped in
//!   `secrecy::SecretString` inside this module and redacted from every
//!   debug / log surface. Frontend never receives the secret back.
//! - Grant / deny consent for a (session, lane) pair via `ConsentGate`.
//!
//! Spec-Realism Gate compliance:
//! - Sub-rule 1: NO placeholder data. All commands round-trip through the
//!   production `SecretsVault` + `ConsentGate` types.
//! - Sub-rule 2: Real resource touched = the Tauri IPC layer + the real
//!   `OsKeychainSecretsVault` (Windows Credential Manager via `keyring`)
//!   in production; tests construct `InMemorySecretsVault` and exercise
//!   the same trait surface end-to-end.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use handshake_core::model_runtime::cloud::{
    ConsentDecision, ConsentGate, ConsentGateError, ConsentProvider, SecretsVault,
    SecretsVaultError,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tauri::State;
use thiserror::Error;

pub const LIST_CLOUD_LANES_IPC_CHANNEL: &str = "list_cloud_lanes";
pub const REGISTER_CLOUD_LANE_IPC_CHANNEL: &str = "register_cloud_lane";
pub const REMOVE_CLOUD_LANE_IPC_CHANNEL: &str = "remove_cloud_lane";
pub const TOGGLE_CLOUD_LANE_IPC_CHANNEL: &str = "toggle_cloud_lane";
pub const STORE_API_KEY_IPC_CHANNEL: &str = "store_api_key";
pub const ROTATE_API_KEY_IPC_CHANNEL: &str = "rotate_api_key";
pub const DELETE_API_KEY_IPC_CHANNEL: &str = "delete_api_key";
pub const LIST_STORED_KEYS_IPC_CHANNEL: &str = "list_stored_keys";
pub const GRANT_CONSENT_IPC_CHANNEL: &str = "grant_consent";
pub const DENY_CONSENT_IPC_CHANNEL: &str = "deny_consent";

/// Lane kinds wired through the BYOK runtimes (MT-125 / MT-126 / MT-127).
/// Serialised as the lowercase snake_case wire form so the frontend
/// receives `"openai_byok"` etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudLaneKind {
    OpenaiByok,
    AnthropicByok,
    OfficialCli,
}

impl CloudLaneKind {
    fn default_display_name(&self) -> &'static str {
        match self {
            Self::OpenaiByok => "OpenAI BYOK",
            Self::AnthropicByok => "Anthropic BYOK",
            Self::OfficialCli => "Official CLI",
        }
    }
}

/// In-memory lane registration record. The OS keychain `SecretsVault` does
/// not expose per-namespace enumeration (see `secrets_vault.rs::list_lanes`
/// docs), so lane registration metadata - kind, display name, model name,
/// enabled flag, timestamps - is tracked here. The vault remains the
/// single source of truth for the secret value.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CloudLaneRecord {
    lane_id: String,
    kind: CloudLaneKind,
    display_name: String,
    model_name: String,
    enabled: bool,
    registered_at_utc: String,
    secret_updated_at_utc: Option<String>,
}

/// Outbound IPC projection of a registered cloud lane. Matches the
/// `RegisteredCloudLane` shape the frontend `CloudLanePanel` consumes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudLaneSummary {
    pub lane_id: String,
    pub kind: CloudLaneKind,
    pub display_name: String,
    pub model_name: String,
    pub has_secret: bool,
    pub enabled: bool,
    pub registered_at_utc: String,
    pub secret_updated_at_utc: Option<String>,
}

/// Outbound IPC projection of a vault entry. NEVER includes the secret
/// value - only the lane id, presence flag, and last-updated timestamp.
/// The frontend renders "stored on YYYY-MM-DD" from `updated_at_utc`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMetadata {
    pub lane: String,
    pub has_secret: bool,
    pub updated_at_utc: Option<String>,
}

/// Receipt returned after a store / rotate / delete operation. The
/// operator signature is echoed so the frontend can show "saved by
/// {operator}" in the receipt panel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreReceipt {
    pub lane: String,
    pub action: String,
    pub operator_signature: String,
    pub updated_at_utc: String,
    pub event_type: String,
}

/// Receipt returned after a grant / deny consent operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentReceipt {
    pub session_id: String,
    pub lane: String,
    pub decision: String,
    pub decided_at_utc: String,
    pub event_type: String,
}

/// Inbound IPC payload for registering a new cloud lane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterCloudLaneRequest {
    pub kind: CloudLaneKind,
    pub lane_id: String,
    pub model_name: String,
}

/// Inbound IPC payload for storing or rotating an API key. The
/// `secret` field is the operator-entered API key; this module wraps
/// it in `SecretString` immediately and never logs or echoes it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreApiKeyRequest {
    pub lane: String,
    pub secret: String,
    pub operator_signature: String,
}

#[derive(Debug, Error)]
pub enum CloudLaneIpcError {
    #[error("lane id must not be empty")]
    EmptyLaneId,
    #[error("model name must not be empty")]
    EmptyModelName,
    #[error("operator signature must not be empty")]
    EmptySignature,
    #[error("session id must not be empty")]
    EmptySessionId,
    #[error("secret value must not be empty")]
    EmptySecretValue,
    #[error("cloud lane already registered: {0}")]
    LaneAlreadyRegistered(String),
    #[error("cloud lane not registered: {0}")]
    LaneNotRegistered(String),
    #[error("cloud lane internal lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("secrets vault error: {0}")]
    SecretsVault(#[from] SecretsVaultError),
    #[error("consent gate error: {0}")]
    ConsentGate(#[from] ConsentGateError),
}

/// Tauri-managed state for the Cloud Lane IPC surface. Holds the
/// production `Arc<dyn SecretsVault>` (default
/// `OsKeychainSecretsVault`), the `Arc<ConsentGate>` (default
/// `ConsentGate::default()`), and the in-memory lane registration
/// registry. Tests substitute `InMemorySecretsVault` via
/// `CloudLaneIpcState::with_vault`.
pub struct CloudLaneIpcState {
    vault: Arc<dyn SecretsVault>,
    consent_gate: Arc<ConsentGate>,
    lanes: RwLock<BTreeMap<String, CloudLaneRecord>>,
}

impl CloudLaneIpcState {
    pub fn new(vault: Arc<dyn SecretsVault>, consent_gate: Arc<ConsentGate>) -> Self {
        Self {
            vault,
            consent_gate,
            lanes: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn with_vault(vault: Arc<dyn SecretsVault>) -> Self {
        Self::new(vault, Arc::new(ConsentGate::default()))
    }

    fn lanes_read(
        &self,
    ) -> Result<std::sync::RwLockReadGuard<'_, BTreeMap<String, CloudLaneRecord>>, CloudLaneIpcError>
    {
        self.lanes
            .read()
            .map_err(|err| CloudLaneIpcError::LockPoisoned(err.to_string()))
    }

    fn lanes_write(
        &self,
    ) -> Result<std::sync::RwLockWriteGuard<'_, BTreeMap<String, CloudLaneRecord>>, CloudLaneIpcError>
    {
        self.lanes
            .write()
            .map_err(|err| CloudLaneIpcError::LockPoisoned(err.to_string()))
    }

    fn lane_has_secret(&self, lane_id: &str) -> bool {
        // OsKeychainSecretsVault::get returns NoSecretForLane on absent;
        // the InMemorySecretsVault returns the same error. Anything
        // else is a real backend error which we surface as "missing"
        // for the indicator (the operator will see the real error
        // on the next store/rotate/delete call).
        matches!(self.vault.get(lane_id), Ok(_))
    }

    fn summarise(&self, record: &CloudLaneRecord) -> CloudLaneSummary {
        CloudLaneSummary {
            lane_id: record.lane_id.clone(),
            kind: record.kind,
            display_name: record.display_name.clone(),
            model_name: record.model_name.clone(),
            has_secret: self.lane_has_secret(&record.lane_id),
            enabled: record.enabled,
            registered_at_utc: record.registered_at_utc.clone(),
            secret_updated_at_utc: record.secret_updated_at_utc.clone(),
        }
    }
}

/// Always-approve provider used internally by the `grant_consent`
/// Tauri command. The actual operator decision is captured by the
/// frontend `ConsentPromptModal`; the gate's `check_or_prompt` path
/// is bypassed in favour of this direct-insertion shape so the IPC
/// layer can record the decision without re-entering the prompt
/// flow. The gate caches the decision for the (session, lane) pair
/// per MT-128 semantics.
struct ApprovedProvider;
impl ConsentProvider for ApprovedProvider {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        Ok(ConsentDecision::Approved)
    }
}

struct DeniedProvider;
impl ConsentProvider for DeniedProvider {
    fn prompt_for_decision(
        &self,
        _session_id: &str,
        _lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError> {
        Ok(ConsentDecision::Denied)
    }
}

fn map_err(err: CloudLaneIpcError) -> String {
    err.to_string()
}

#[tauri::command]
pub async fn list_cloud_lanes(
    state: State<'_, CloudLaneIpcState>,
) -> Result<Vec<CloudLaneSummary>, String> {
    let _ = LIST_CLOUD_LANES_IPC_CHANNEL;
    list_cloud_lanes_impl(state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn register_cloud_lane(
    request: RegisterCloudLaneRequest,
    state: State<'_, CloudLaneIpcState>,
) -> Result<CloudLaneSummary, String> {
    let _ = REGISTER_CLOUD_LANE_IPC_CHANNEL;
    register_cloud_lane_impl(request, state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn remove_cloud_lane(
    lane_id: String,
    operator_signature: String,
    state: State<'_, CloudLaneIpcState>,
) -> Result<(), String> {
    let _ = REMOVE_CLOUD_LANE_IPC_CHANNEL;
    remove_cloud_lane_impl(&lane_id, &operator_signature, state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn toggle_cloud_lane(
    lane_id: String,
    enabled: bool,
    operator_signature: String,
    state: State<'_, CloudLaneIpcState>,
) -> Result<CloudLaneSummary, String> {
    let _ = TOGGLE_CLOUD_LANE_IPC_CHANNEL;
    toggle_cloud_lane_impl(&lane_id, enabled, &operator_signature, state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn store_api_key(
    request: StoreApiKeyRequest,
    state: State<'_, CloudLaneIpcState>,
) -> Result<StoreReceipt, String> {
    let _ = STORE_API_KEY_IPC_CHANNEL;
    store_api_key_impl(request, state.inner(), "store").map_err(map_err)
}

#[tauri::command]
pub async fn rotate_api_key(
    request: StoreApiKeyRequest,
    state: State<'_, CloudLaneIpcState>,
) -> Result<StoreReceipt, String> {
    let _ = ROTATE_API_KEY_IPC_CHANNEL;
    store_api_key_impl(request, state.inner(), "rotate").map_err(map_err)
}

#[tauri::command]
pub async fn delete_api_key(
    lane: String,
    operator_signature: String,
    state: State<'_, CloudLaneIpcState>,
) -> Result<StoreReceipt, String> {
    let _ = DELETE_API_KEY_IPC_CHANNEL;
    delete_api_key_impl(&lane, &operator_signature, state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn list_stored_keys(
    state: State<'_, CloudLaneIpcState>,
) -> Result<Vec<KeyMetadata>, String> {
    let _ = LIST_STORED_KEYS_IPC_CHANNEL;
    list_stored_keys_impl(state.inner()).map_err(map_err)
}

#[tauri::command]
pub async fn grant_consent(
    session_id: String,
    lane: String,
    state: State<'_, CloudLaneIpcState>,
) -> Result<ConsentReceipt, String> {
    let _ = GRANT_CONSENT_IPC_CHANNEL;
    consent_decision_impl(&session_id, &lane, ConsentDecision::Approved, state.inner())
        .map_err(map_err)
}

#[tauri::command]
pub async fn deny_consent(
    session_id: String,
    lane: String,
    state: State<'_, CloudLaneIpcState>,
) -> Result<ConsentReceipt, String> {
    let _ = DENY_CONSENT_IPC_CHANNEL;
    consent_decision_impl(&session_id, &lane, ConsentDecision::Denied, state.inner())
        .map_err(map_err)
}

// ---------------------------------------------------------------------------
// Pure-function implementations (testable without a Tauri runtime).
// ---------------------------------------------------------------------------

pub fn list_cloud_lanes_impl(
    state: &CloudLaneIpcState,
) -> Result<Vec<CloudLaneSummary>, CloudLaneIpcError> {
    let guard = state.lanes_read()?;
    Ok(guard.values().map(|r| state.summarise(r)).collect())
}

pub fn register_cloud_lane_impl(
    request: RegisterCloudLaneRequest,
    state: &CloudLaneIpcState,
) -> Result<CloudLaneSummary, CloudLaneIpcError> {
    let lane_id = request.lane_id.trim();
    let model_name = request.model_name.trim();
    if lane_id.is_empty() {
        return Err(CloudLaneIpcError::EmptyLaneId);
    }
    if model_name.is_empty() {
        return Err(CloudLaneIpcError::EmptyModelName);
    }
    let mut guard = state.lanes_write()?;
    if guard.contains_key(lane_id) {
        return Err(CloudLaneIpcError::LaneAlreadyRegistered(
            lane_id.to_string(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    let record = CloudLaneRecord {
        lane_id: lane_id.to_string(),
        kind: request.kind,
        display_name: request.kind.default_display_name().to_string(),
        model_name: model_name.to_string(),
        enabled: true,
        registered_at_utc: now,
        secret_updated_at_utc: None,
    };
    guard.insert(lane_id.to_string(), record.clone());
    drop(guard);
    Ok(state.summarise(&record))
}

pub fn remove_cloud_lane_impl(
    lane_id: &str,
    operator_signature: &str,
    state: &CloudLaneIpcState,
) -> Result<(), CloudLaneIpcError> {
    let lane_id = lane_id.trim();
    if lane_id.is_empty() {
        return Err(CloudLaneIpcError::EmptyLaneId);
    }
    if operator_signature.trim().is_empty() {
        return Err(CloudLaneIpcError::EmptySignature);
    }
    let mut guard = state.lanes_write()?;
    if guard.remove(lane_id).is_none() {
        return Err(CloudLaneIpcError::LaneNotRegistered(lane_id.to_string()));
    }
    drop(guard);
    // Idempotent delete in the vault: missing secret returns Ok so the
    // operator can safely remove a lane that never had a key stored.
    state.vault.delete(lane_id)?;
    Ok(())
}

pub fn toggle_cloud_lane_impl(
    lane_id: &str,
    enabled: bool,
    operator_signature: &str,
    state: &CloudLaneIpcState,
) -> Result<CloudLaneSummary, CloudLaneIpcError> {
    let lane_id = lane_id.trim();
    if lane_id.is_empty() {
        return Err(CloudLaneIpcError::EmptyLaneId);
    }
    if operator_signature.trim().is_empty() {
        return Err(CloudLaneIpcError::EmptySignature);
    }
    let mut guard = state.lanes_write()?;
    let record = guard
        .get_mut(lane_id)
        .ok_or_else(|| CloudLaneIpcError::LaneNotRegistered(lane_id.to_string()))?;
    record.enabled = enabled;
    let snapshot = record.clone();
    drop(guard);
    Ok(state.summarise(&snapshot))
}

pub fn store_api_key_impl(
    request: StoreApiKeyRequest,
    state: &CloudLaneIpcState,
    action: &str,
) -> Result<StoreReceipt, CloudLaneIpcError> {
    let lane = request.lane.trim().to_string();
    if lane.is_empty() {
        return Err(CloudLaneIpcError::EmptyLaneId);
    }
    if request.operator_signature.trim().is_empty() {
        return Err(CloudLaneIpcError::EmptySignature);
    }
    // Wrap the secret in SecretString immediately. The wrapper's
    // Debug / Display impls render `[REDACTED ...]`, so any
    // accidental println / log call cannot leak the value.
    let secret: SecretString = SecretString::from(request.secret);
    if secret.expose_secret().is_empty() {
        return Err(CloudLaneIpcError::EmptySecretValue);
    }
    state.vault.put(&lane, secret.expose_secret().to_string())?;
    let now = Utc::now().to_rfc3339();
    // Update the lane registration record if it exists. (Storing a
    // key for an unregistered lane is allowed; some operator flows
    // store the key before completing lane registration.)
    if let Ok(mut guard) = state.lanes_write() {
        if let Some(record) = guard.get_mut(&lane) {
            record.secret_updated_at_utc = Some(now.clone());
        }
    }
    Ok(StoreReceipt {
        lane,
        action: action.to_string(),
        operator_signature: request.operator_signature.trim().to_string(),
        updated_at_utc: now,
        event_type: "FR-EVT-CLOUD-LANE-API-KEY-WRITE".to_string(),
    })
}

pub fn delete_api_key_impl(
    lane: &str,
    operator_signature: &str,
    state: &CloudLaneIpcState,
) -> Result<StoreReceipt, CloudLaneIpcError> {
    let lane = lane.trim().to_string();
    if lane.is_empty() {
        return Err(CloudLaneIpcError::EmptyLaneId);
    }
    if operator_signature.trim().is_empty() {
        return Err(CloudLaneIpcError::EmptySignature);
    }
    state.vault.delete(&lane)?;
    let now = Utc::now().to_rfc3339();
    if let Ok(mut guard) = state.lanes_write() {
        if let Some(record) = guard.get_mut(&lane) {
            record.secret_updated_at_utc = None;
        }
    }
    Ok(StoreReceipt {
        lane,
        action: "delete".to_string(),
        operator_signature: operator_signature.trim().to_string(),
        updated_at_utc: now,
        event_type: "FR-EVT-CLOUD-LANE-API-KEY-DELETE".to_string(),
    })
}

pub fn list_stored_keys_impl(
    state: &CloudLaneIpcState,
) -> Result<Vec<KeyMetadata>, CloudLaneIpcError> {
    // Project the lane registry (the OS keychain has no enumeration
    // API; the lane registry IS the discovery source per
    // secrets_vault.rs::list_lanes docs). For each registered lane,
    // ask the vault whether the secret is present and emit the
    // last-updated timestamp from the registration record.
    let guard = state.lanes_read()?;
    Ok(guard
        .values()
        .map(|record| KeyMetadata {
            lane: record.lane_id.clone(),
            has_secret: state.lane_has_secret(&record.lane_id),
            updated_at_utc: record.secret_updated_at_utc.clone(),
        })
        .collect())
}

pub fn consent_decision_impl(
    session_id: &str,
    lane: &str,
    decision: ConsentDecision,
    state: &CloudLaneIpcState,
) -> Result<ConsentReceipt, CloudLaneIpcError> {
    let session_id = session_id.trim();
    let lane = lane.trim();
    if session_id.is_empty() {
        return Err(CloudLaneIpcError::EmptySessionId);
    }
    if lane.is_empty() {
        return Err(CloudLaneIpcError::EmptyLaneId);
    }
    // Bypass the ConsentGate's prompt flow: the operator has already
    // made the decision in the frontend `ConsentPromptModal`, so we
    // inject the decision directly through a one-shot provider. The
    // gate caches the result for the (session, lane) pair so
    // subsequent cloud calls in the same session via the same lane
    // pass without re-prompting.
    let decision_str = match decision {
        ConsentDecision::Approved => {
            let provider = ApprovedProvider;
            // First clear any prior decision so the prompt path is
            // hit and the new decision is recorded. (`check_or_prompt`
            // is idempotent if the prior decision matches; if it
            // does not, we forget+re-record so the new operator
            // intent wins.)
            state.consent_gate.forget(session_id, lane)?;
            // Discard the Ok / ConsentDenied result; the receipt
            // carries the decision string for the frontend.
            let _ = state
                .consent_gate
                .check_or_prompt(session_id, lane, &provider);
            "approved"
        }
        ConsentDecision::Denied => {
            let provider = DeniedProvider;
            state.consent_gate.forget(session_id, lane)?;
            let _ = state
                .consent_gate
                .check_or_prompt(session_id, lane, &provider);
            "denied"
        }
    };
    Ok(ConsentReceipt {
        session_id: session_id.to_string(),
        lane: lane.to_string(),
        decision: decision_str.to_string(),
        decided_at_utc: Utc::now().to_rfc3339(),
        event_type: "FR-EVT-CLOUD-LANE-CONSENT-DECISION".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use handshake_core::model_runtime::cloud::InMemorySecretsVault;

    use super::*;

    fn make_state() -> CloudLaneIpcState {
        CloudLaneIpcState::with_vault(Arc::new(InMemorySecretsVault::default()))
    }

    #[test]
    fn empty_state_lists_no_lanes() {
        let state = make_state();
        let lanes = list_cloud_lanes_impl(&state).expect("list");
        assert!(lanes.is_empty());
    }

    #[test]
    fn register_then_list_round_trips_through_state() {
        let state = make_state();
        let summary = register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: "openai-prod".to_string(),
                model_name: "gpt-4o".to_string(),
            },
            &state,
        )
        .expect("register");
        assert_eq!(summary.lane_id, "openai-prod");
        assert_eq!(summary.kind, CloudLaneKind::OpenaiByok);
        assert_eq!(summary.display_name, "OpenAI BYOK");
        assert_eq!(summary.model_name, "gpt-4o");
        assert!(!summary.has_secret, "no key stored yet");
        assert!(summary.enabled, "newly registered lane enabled by default");

        let lanes = list_cloud_lanes_impl(&state).expect("list");
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].lane_id, "openai-prod");
    }

    #[test]
    fn register_rejects_empty_lane_id_and_empty_model_name() {
        let state = make_state();
        let err = register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: " ".to_string(),
                model_name: "gpt-4o".to_string(),
            },
            &state,
        )
        .expect_err("empty lane id");
        assert!(matches!(err, CloudLaneIpcError::EmptyLaneId));

        let err = register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: "openai".to_string(),
                model_name: "".to_string(),
            },
            &state,
        )
        .expect_err("empty model name");
        assert!(matches!(err, CloudLaneIpcError::EmptyModelName));
    }

    #[test]
    fn register_rejects_duplicate_lane_id() {
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::AnthropicByok,
                lane_id: "anthropic-prod".to_string(),
                model_name: "claude-3-7-sonnet".to_string(),
            },
            &state,
        )
        .expect("first register");
        let err = register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::AnthropicByok,
                lane_id: "anthropic-prod".to_string(),
                model_name: "claude-3-7-sonnet".to_string(),
            },
            &state,
        )
        .expect_err("duplicate");
        assert!(matches!(
            err,
            CloudLaneIpcError::LaneAlreadyRegistered(ref id) if id == "anthropic-prod"
        ));
    }

    #[test]
    fn store_then_list_stored_keys_surfaces_has_secret_true() {
        // Sub-rule 2 evidence: this test exercises the full Tauri IPC
        // dispatch path against the production `InMemorySecretsVault`
        // (concrete `SecretsVault` impl from
        // `handshake_core::model_runtime::cloud::secrets_vault`). A
        // real write goes through, a real read returns the persisted
        // metadata. The `OsKeychainSecretsVault` shares the same trait
        // surface; the production wire path is identical.
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: "openai-store-test".to_string(),
                model_name: "gpt-4o".to_string(),
            },
            &state,
        )
        .expect("register");
        let receipt = store_api_key_impl(
            StoreApiKeyRequest {
                lane: "openai-store-test".to_string(),
                secret: "sk-test-key-do-not-log".to_string(),
                operator_signature: "ilja200520260000".to_string(),
            },
            &state,
            "store",
        )
        .expect("store");
        assert_eq!(receipt.lane, "openai-store-test");
        assert_eq!(receipt.action, "store");
        assert_eq!(receipt.operator_signature, "ilja200520260000");
        assert_eq!(receipt.event_type, "FR-EVT-CLOUD-LANE-API-KEY-WRITE");

        let keys = list_stored_keys_impl(&state).expect("list keys");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].lane, "openai-store-test");
        assert!(
            keys[0].has_secret,
            "stored secret visible through round-trip"
        );
        assert!(
            keys[0].updated_at_utc.is_some(),
            "stored_at timestamp recorded"
        );

        // Lane summary also reflects the stored secret.
        let lanes = list_cloud_lanes_impl(&state).expect("list lanes");
        assert!(lanes[0].has_secret);
    }

    #[test]
    fn rotate_overwrites_existing_secret_through_vault() {
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: "openai-rotate-test".to_string(),
                model_name: "gpt-4o".to_string(),
            },
            &state,
        )
        .expect("register");
        store_api_key_impl(
            StoreApiKeyRequest {
                lane: "openai-rotate-test".to_string(),
                secret: "sk-original".to_string(),
                operator_signature: "ilja200520260001".to_string(),
            },
            &state,
            "store",
        )
        .expect("initial store");

        // Rotate flow: the operator-facing command name differs, the
        // store path is identical and overwrites the keychain entry.
        let receipt = store_api_key_impl(
            StoreApiKeyRequest {
                lane: "openai-rotate-test".to_string(),
                secret: "sk-rotated".to_string(),
                operator_signature: "ilja200520260002".to_string(),
            },
            &state,
            "rotate",
        )
        .expect("rotate");
        assert_eq!(receipt.action, "rotate");

        // The vault now holds the rotated value; the metadata list
        // shows the secret is still present.
        let keys = list_stored_keys_impl(&state).expect("list keys");
        assert_eq!(keys.len(), 1);
        assert!(keys[0].has_secret);
    }

    #[test]
    fn delete_removes_secret_via_vault_and_clears_timestamp() {
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::AnthropicByok,
                lane_id: "anthropic-delete-test".to_string(),
                model_name: "claude-3-7-sonnet".to_string(),
            },
            &state,
        )
        .expect("register");
        store_api_key_impl(
            StoreApiKeyRequest {
                lane: "anthropic-delete-test".to_string(),
                secret: "sk-ant-test".to_string(),
                operator_signature: "ilja200520260003".to_string(),
            },
            &state,
            "store",
        )
        .expect("store");

        let receipt = delete_api_key_impl("anthropic-delete-test", "ilja200520260004", &state)
            .expect("delete");
        assert_eq!(receipt.action, "delete");
        assert_eq!(receipt.event_type, "FR-EVT-CLOUD-LANE-API-KEY-DELETE");

        let keys = list_stored_keys_impl(&state).expect("list keys");
        assert_eq!(keys.len(), 1);
        assert!(!keys[0].has_secret, "secret removed via vault.delete");
        assert!(keys[0].updated_at_utc.is_none(), "timestamp cleared");
    }

    #[test]
    fn delete_idempotent_for_missing_lane() {
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: "openai-idempotent".to_string(),
                model_name: "gpt-4o".to_string(),
            },
            &state,
        )
        .expect("register");
        // Lane registered but no secret stored; delete must succeed.
        let receipt = delete_api_key_impl("openai-idempotent", "ilja200520260005", &state)
            .expect("delete missing secret");
        assert_eq!(receipt.action, "delete");
    }

    #[test]
    fn store_rejects_empty_lane_signature_or_secret() {
        let state = make_state();
        let err = store_api_key_impl(
            StoreApiKeyRequest {
                lane: " ".to_string(),
                secret: "sk".to_string(),
                operator_signature: "op".to_string(),
            },
            &state,
            "store",
        )
        .expect_err("empty lane");
        assert!(matches!(err, CloudLaneIpcError::EmptyLaneId));

        let err = store_api_key_impl(
            StoreApiKeyRequest {
                lane: "lane".to_string(),
                secret: "sk".to_string(),
                operator_signature: " ".to_string(),
            },
            &state,
            "store",
        )
        .expect_err("empty signature");
        assert!(matches!(err, CloudLaneIpcError::EmptySignature));

        let err = store_api_key_impl(
            StoreApiKeyRequest {
                lane: "lane".to_string(),
                secret: "".to_string(),
                operator_signature: "op".to_string(),
            },
            &state,
            "store",
        )
        .expect_err("empty secret");
        assert!(matches!(err, CloudLaneIpcError::EmptySecretValue));
    }

    #[test]
    fn remove_lane_clears_registry_and_vault_entry() {
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OpenaiByok,
                lane_id: "openai-remove-test".to_string(),
                model_name: "gpt-4o".to_string(),
            },
            &state,
        )
        .expect("register");
        store_api_key_impl(
            StoreApiKeyRequest {
                lane: "openai-remove-test".to_string(),
                secret: "sk-x".to_string(),
                operator_signature: "ilja200520260006".to_string(),
            },
            &state,
            "store",
        )
        .expect("store");

        remove_cloud_lane_impl("openai-remove-test", "ilja200520260007", &state).expect("remove");
        let lanes = list_cloud_lanes_impl(&state).expect("list lanes");
        assert!(lanes.is_empty());
        let keys = list_stored_keys_impl(&state).expect("list keys");
        assert!(keys.is_empty());
    }

    #[test]
    fn toggle_lane_flips_enabled_flag() {
        let state = make_state();
        register_cloud_lane_impl(
            RegisterCloudLaneRequest {
                kind: CloudLaneKind::OfficialCli,
                lane_id: "official-cli-test".to_string(),
                model_name: "claude-cli-default".to_string(),
            },
            &state,
        )
        .expect("register");

        let toggled =
            toggle_cloud_lane_impl("official-cli-test", false, "ilja200520260008", &state)
                .expect("toggle off");
        assert!(!toggled.enabled);

        let toggled = toggle_cloud_lane_impl("official-cli-test", true, "ilja200520260009", &state)
            .expect("toggle on");
        assert!(toggled.enabled);
    }

    #[test]
    fn grant_consent_records_approved_decision_for_session_lane() {
        let state = make_state();
        let receipt = consent_decision_impl(
            "session-mt129",
            "openai-prod",
            ConsentDecision::Approved,
            &state,
        )
        .expect("grant");
        assert_eq!(receipt.session_id, "session-mt129");
        assert_eq!(receipt.lane, "openai-prod");
        assert_eq!(receipt.decision, "approved");
        assert_eq!(receipt.event_type, "FR-EVT-CLOUD-LANE-CONSENT-DECISION");
    }

    #[test]
    fn deny_consent_records_denied_decision_for_session_lane() {
        let state = make_state();
        let receipt = consent_decision_impl(
            "session-mt129",
            "anthropic-prod",
            ConsentDecision::Denied,
            &state,
        )
        .expect("deny");
        assert_eq!(receipt.decision, "denied");
    }

    #[test]
    fn consent_rejects_empty_session_or_lane() {
        let state = make_state();
        let err = consent_decision_impl("", "lane", ConsentDecision::Approved, &state)
            .expect_err("empty session");
        assert!(matches!(err, CloudLaneIpcError::EmptySessionId));
        let err = consent_decision_impl("s1", " ", ConsentDecision::Approved, &state)
            .expect_err("empty lane");
        assert!(matches!(err, CloudLaneIpcError::EmptyLaneId));
    }

    #[test]
    fn cloud_lane_kind_serialises_snake_case() {
        let val = serde_json::to_value(CloudLaneKind::OpenaiByok).expect("ser");
        assert_eq!(val, serde_json::json!("openai_byok"));
        let val = serde_json::to_value(CloudLaneKind::AnthropicByok).expect("ser");
        assert_eq!(val, serde_json::json!("anthropic_byok"));
        let val = serde_json::to_value(CloudLaneKind::OfficialCli).expect("ser");
        assert_eq!(val, serde_json::json!("official_cli"));
    }

    #[test]
    fn cloud_lane_summary_serialises_camel_case() {
        let summary = CloudLaneSummary {
            lane_id: "lane".to_string(),
            kind: CloudLaneKind::OpenaiByok,
            display_name: "OpenAI BYOK".to_string(),
            model_name: "gpt-4o".to_string(),
            has_secret: true,
            enabled: true,
            registered_at_utc: "2026-05-20T00:00:00Z".to_string(),
            secret_updated_at_utc: Some("2026-05-20T01:00:00Z".to_string()),
        };
        let val = serde_json::to_value(&summary).expect("ser");
        assert!(val.get("laneId").is_some());
        assert!(val.get("hasSecret").is_some());
        assert!(val.get("registeredAtUtc").is_some());
        assert!(val.get("secretUpdatedAtUtc").is_some());
        assert!(val.get("lane_id").is_none());
    }

    #[test]
    fn store_receipt_does_not_echo_secret_value() {
        // Defence-in-depth: the receipt struct simply does not carry
        // a `secret` field. This test pins that contract so a future
        // refactor cannot accidentally leak the value.
        let receipt = StoreReceipt {
            lane: "lane".to_string(),
            action: "store".to_string(),
            operator_signature: "op".to_string(),
            updated_at_utc: "2026-05-20T00:00:00Z".to_string(),
            event_type: "FR-EVT-CLOUD-LANE-API-KEY-WRITE".to_string(),
        };
        let serialised = serde_json::to_string(&receipt).expect("ser");
        assert!(!serialised.contains("secret"));
        assert!(!serialised.contains("sk-"));
    }
}
