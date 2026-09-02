use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use curve25519_dalek::montgomery::MontgomeryPoint;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    process::Child,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    flight_recorder::{
        EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
        FlightRecorderEventType,
    },
    kernel::{KernelActor, KernelEventType, NewKernelEvent},
    process_ledger::{
        ActiveProcessLifecycle, LedgerBatcher, ProcessEngineKind, ProcessStart, Reclaim,
        ReclaimTrigger, StopRecordOutcome,
    },
    sandbox::palmistry_watcher::{
        PalmistrySpawnSpec, PalmistryWatcherAdapter, SpawnedPalmistry, PALMISTRY_WATCHER_ADAPTER_ID,
    },
    storage::surreal::{
        SurrealPalmistryStore, SurrealPalmistryVerifier, SurrealStorage,
    },
    swarm_orchestration::model_lane::{
        ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState, ModelLaneStore,
        NewModelLaneDiagnosticTierStatus,
    },
};

const LEDGER_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const OBSERVATION_MAX_AGE_MS: u64 = 5_000;
const OBSERVATION_MAX_FUTURE_SKEW_MS: u64 = 2_000;
const SURVIVOR_MAX_AGE_MS: u64 = 30 * 24 * 60 * 60 * 1_000;
const SURVIVOR_MAX_FUTURE_SKEW_MS: u64 = 2 * 60 * 1_000;
const OWNER_WP: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PalmistryLaunchRequest {
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub parent_pid: u32,
    pub ring: PathBuf,
    pub survivor_dir: PathBuf,
    pub panic_signal: PathBuf,
    pub panic_ack: PathBuf,
    pub shutdown_signal: PathBuf,
    pub ready_signal: PathBuf,
    /// Ephemeral X25519-compatible public key used only to seal the response's
    /// Argus secret. The signing secret itself never enters Axum's JSON body.
    pub transport_public_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PalmistryLaunchReceipt {
    pub session_id: Uuid,
    pub process_uuid: Uuid,
    pub os_pid: u32,
    pub sandbox_adapter_id: &'static str,
    pub ledger_start_durable: bool,
    pub os_creation_time_100ns: u64,
}

#[derive(Debug, Serialize)]
struct TransportSigningSecretEnvelope {
    server_public_key: String,
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, Serialize)]
struct PalmistryLaunchResponse {
    #[serde(flatten)]
    receipt: PalmistryLaunchReceipt,
    argus_signing_secret_envelope: TransportSigningSecretEnvelope,
}

#[derive(Clone)]
struct SessionSigningSecret(Arc<Zeroizing<[u8; 32]>>);

impl SessionSigningSecret {
    fn generate() -> Self {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let mut bytes = [0_u8; 32];
        bytes[..16].copy_from_slice(first.as_bytes());
        bytes[16..].copy_from_slice(second.as_bytes());
        Self(Arc::new(Zeroizing::new(bytes)))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0.as_ref()[..]
    }

    fn seal_for_transport(
        &self,
        request: &PalmistryLaunchRequest,
    ) -> Result<TransportSigningSecretEnvelope, PalmistryLaunchError> {
        let client_public = decode_hex_array::<32>(&request.transport_public_key)
            .map_err(|_| PalmistryLaunchError::bad_request("invalid transport public key"))?;
        let mut server_secret = Zeroizing::new([0_u8; 32]);
        getrandom::getrandom(server_secret.as_mut()).map_err(|error| {
            PalmistryLaunchError::unavailable(format!(
                "cannot generate Palmistry transport key: {error}"
            ))
        })?;
        let server_public = MontgomeryPoint::mul_base_clamped(*server_secret);
        let shared = Zeroizing::new(
            MontgomeryPoint(client_public)
                .mul_clamped(*server_secret)
                .to_bytes(),
        );
        let key = palmistry_transport_key(&shared, request.session_id, request.launch_nonce);
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref()).map_err(|_| {
            PalmistryLaunchError::unavailable("cannot initialize Palmistry transport cipher")
        })?;
        let mut nonce = [0_u8; 12];
        getrandom::getrandom(&mut nonce).map_err(|error| {
            PalmistryLaunchError::unavailable(format!(
                "cannot generate Palmistry transport nonce: {error}"
            ))
        })?;
        let mut ciphertext = Zeroizing::new(self.0.as_ref().to_vec());
        cipher
            .encrypt_in_place(
                Nonce::from_slice(&nonce),
                palmistry_transport_aad(request.session_id, request.launch_nonce).as_slice(),
                &mut *ciphertext,
            )
            .map_err(|_| {
                PalmistryLaunchError::unavailable("cannot seal Palmistry signing secret")
            })?;
        Ok(TransportSigningSecretEnvelope {
            server_public_key: hex::encode(server_public.as_bytes()),
            nonce: hex::encode(nonce),
            ciphertext: hex::encode(ciphertext.as_slice()),
        })
    }
}

fn palmistry_transport_key(
    shared: &[u8; 32],
    session_id: Uuid,
    launch_nonce: Uuid,
) -> Zeroizing<[u8; 32]> {
    let mut digest = Sha256::new();
    digest.update(b"hsk.palmistry.transport-key@1\0");
    digest.update(shared);
    digest.update(session_id.as_bytes());
    digest.update(launch_nonce.as_bytes());
    Zeroizing::new(digest.finalize().into())
}

fn palmistry_transport_aad(session_id: Uuid, launch_nonce: Uuid) -> [u8; 32] {
    let mut aad = [0_u8; 32];
    aad[..16].copy_from_slice(session_id.as_bytes());
    aad[16..].copy_from_slice(launch_nonce.as_bytes());
    aad
}

fn decode_hex_array<const N: usize>(value: &str) -> Result<[u8; N], hex::FromHexError> {
    let mut bytes = [0_u8; N];
    hex::decode_to_slice(value, &mut bytes)?;
    Ok(bytes)
}

impl fmt::Debug for SessionSigningSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionSigningSecret([REDACTED])")
    }
}

#[derive(Clone)]
struct WatcherSigningSecret(Arc<Zeroizing<[u8; 32]>>);

impl WatcherSigningSecret {
    fn generate() -> Result<Self, PalmistryLaunchError> {
        let mut bytes = [0_u8; 32];
        getrandom::getrandom(&mut bytes).map_err(|error| {
            PalmistryLaunchError::unavailable(format!(
                "cannot generate Palmistry watcher signing key: {error}"
            ))
        })?;
        Ok(Self(Arc::new(Zeroizing::new(bytes))))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0.as_ref()[..]
    }

    fn verifying_key_bytes(&self) -> [u8; 32] {
        SigningKey::from_bytes(self.0.as_ref())
            .verifying_key()
            .to_bytes()
    }
}

impl fmt::Debug for WatcherSigningSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WatcherSigningSecret([REDACTED])")
    }
}

#[derive(Clone)]
pub struct PalmistryLaunchState {
    ledger: LedgerBatcher,
    recorder: Arc<dyn FlightRecorder>,
    model_lane_store: ModelLaneStore,
    palmistry_store: SurrealPalmistryStore,
    reclaim: Arc<Reclaim>,
    active: Arc<Mutex<HashMap<Uuid, LaunchSlot>>>,
}

#[derive(Clone)]
enum LaunchSlot {
    Launching(PalmistryLaunchRequest),
    Active {
        request: PalmistryLaunchRequest,
        receipt: PalmistryLaunchReceipt,
        signing_secret: SessionSigningSecret,
        watcher_verifying_key: [u8; 32],
        diagnostic_envelopes: std::collections::VecDeque<DiagnosticEnvelope>,
        lifecycle_ownership: WatcherLifecycleOwnership,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatcherLifecycleOwnership {
    AttachedReaper,
    DetachedReattached,
}

/// Exact child ownership for one launch attempt until ownership is transferred
/// to the dedicated reaper. `std::process::Child` does not kill on Drop, so a
/// cancelled launch future must hold an explicit kill-and-join guard.
struct PalmistryLaunchAttempt {
    child: Option<Child>,
}

impl PalmistryLaunchAttempt {
    fn new(spawned: SpawnedPalmistry) -> (Self, String, u64) {
        (
            Self {
                child: Some(spawned.child),
            },
            spawned.executable_sha256,
            spawned.os_creation_time_100ns,
        )
    }

    fn child(&self) -> &Child {
        self.child
            .as_ref()
            .expect("Palmistry launch attempt retains child ownership")
    }

    fn take_child(&mut self) -> Child {
        self.child
            .take()
            .expect("Palmistry child ownership transfers exactly once")
    }

    fn terminate_and_wait(&mut self) -> Option<i32> {
        let mut child = self.child.take()?;
        let _ = child.kill();
        child.wait().ok().and_then(|status| status.code())
    }
}

impl Drop for PalmistryLaunchAttempt {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.terminate_and_wait();
        }
    }
}

struct LaunchingSlotGuard {
    active: Arc<Mutex<HashMap<Uuid, LaunchSlot>>>,
    request: PalmistryLaunchRequest,
}

impl LaunchingSlotGuard {
    fn new(active: Arc<Mutex<HashMap<Uuid, LaunchSlot>>>, request: PalmistryLaunchRequest) -> Self {
        Self { active, request }
    }
}

impl Drop for LaunchingSlotGuard {
    fn drop(&mut self) {
        remove_matching_launching_slot(&self.active, &self.request);
    }
}

impl WatcherLifecycleOwnership {
    fn has_attached_reaper(self) -> bool {
        matches!(self, Self::AttachedReaper)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticEnvelope {
    behavior_id: String,
    run_id: String,
    lane_id: String,
    heartbeat_counter: u64,
    correlation_hmac: String,
}

impl PalmistryLaunchState {
    pub fn new(
        ledger: LedgerBatcher,
        recorder: Arc<dyn FlightRecorder>,
        surreal_storage: SurrealStorage,
        exact_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        reclaim: Arc<Reclaim>,
    ) -> Self {
        let resource_scope = crate::swarm_orchestration::resource_scope::ResourceScope::new(
            exact_scope.owner_account_id,
            exact_scope.actor_principal_id,
        )
        .with_session(exact_scope.authenticated_session_id)
        .with_access_space(exact_scope.access_space_id)
        .with_workspace(exact_scope.workspace_id.clone());
        Self {
            ledger,
            recorder,
            model_lane_store: ModelLaneStore::new_scoped(
                surreal_storage.clone(),
                resource_scope,
            ),
            palmistry_store: SurrealPalmistryStore::new_exact(surreal_storage, exact_scope),
            reclaim,
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn active_count(&self) -> usize {
        self.active.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticObservationRequest {
    diagnostics_session_id: Uuid,
    behavior_id: String,
    run_id: String,
    lane_id: String,
    heartbeat_counter: u64,
    proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PalmistryObservationRecord {
    schema_id: String,
    session_id: Uuid,
    launch_nonce: Uuid,
    heartbeat_counter: u64,
    watcher_pid: u32,
    observed_at_unix_ms: u64,
    behavior_observations: Vec<BehaviorObservationRecord>,
    watcher_creation_time_100ns: u64,
    source_proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BehaviorObservationRecord {
    mechanism: String,
    heartbeat_counter: u64,
    observed_at_unix_ms: u64,
    correlation_hmac: String,
}

#[derive(Debug, Clone)]
struct DurableWatcherVerifier {
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: i64,
    watcher_pid: i64,
    watcher_creation_time_100ns: i64,
    process_uuid: Uuid,
    executable_sha256: String,
    verifying_key_hex: String,
}

impl DurableWatcherVerifier {
    fn verifying_key(&self) -> Result<[u8; 32], PalmistryLaunchError> {
        hex::decode(self.verifying_key_hex.trim())
            .map_err(|_| PalmistryLaunchError::bad_request("invalid durable Palmistry verifier"))?
            .try_into()
            .map_err(|_| PalmistryLaunchError::bad_request("invalid durable verifier length"))
    }
}

impl From<SurrealPalmistryVerifier> for DurableWatcherVerifier {
    fn from(verifier: SurrealPalmistryVerifier) -> Self {
        Self {
            session_id: verifier.session_id,
            launch_nonce: verifier.launch_nonce,
            parent_pid: verifier.parent_pid,
            watcher_pid: verifier.watcher_pid,
            watcher_creation_time_100ns: verifier.watcher_creation_time_100ns,
            process_uuid: verifier.process_uuid,
            executable_sha256: verifier.executable_sha256,
            verifying_key_hex: verifier.verifying_key_hex,
        }
    }
}

impl From<&DurableWatcherVerifier> for SurrealPalmistryVerifier {
    fn from(verifier: &DurableWatcherVerifier) -> Self {
        Self {
            session_id: verifier.session_id,
            launch_nonce: verifier.launch_nonce,
            parent_pid: verifier.parent_pid,
            watcher_pid: verifier.watcher_pid,
            watcher_creation_time_100ns: verifier.watcher_creation_time_100ns,
            process_uuid: verifier.process_uuid,
            executable_sha256: verifier.executable_sha256.clone(),
            verifying_key_hex: verifier.verifying_key_hex.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArgusActionReceiptRequest {
    diagnostics_session_id: Uuid,
    action_id: String,
    action: String,
    connection_id: String,
    agent_id: String,
    agent_label: String,
    window_id: String,
    author_id: String,
    before_revision: u64,
    after_revision: Option<u64>,
    status: String,
    proof: String,
}

#[derive(Debug, Clone, Serialize)]
struct ArgusActionDurabilityReceipt {
    event_ledger_event_id: String,
    flight_recorder_event_id: Option<Uuid>,
    flight_recorder_mirrored: bool,
    durable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecoveredSurvivorSummary {
    pub record_id: Uuid,
    pub source_session_id: Uuid,
    pub kind: String,
    pub observed_at_unix_ms: u64,
    pub parent_pid: u32,
    pub parent_exit_code: Option<u32>,
    pub heartbeat_stale_ms: Option<u64>,
    pub os_hung_window_confirmed: bool,
    pub minidump_status: String,
    pub imported: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PalmistryRecoverRequest {
    current_session_id: Uuid,
    launch_nonce: Uuid,
    summary: RecoveredSurvivorSummary,
    proof: String,
}

#[derive(Debug)]
struct PalmistryLaunchError {
    status: StatusCode,
    code: &'static str,
    detail: String,
}

impl PalmistryLaunchError {
    fn bad_request(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "PALMISTRY_LAUNCH_INVALID",
            detail: detail.into(),
        }
    }

    fn unavailable(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "PALMISTRY_LAUNCH_UNAVAILABLE",
            detail: detail.into(),
        }
    }

    fn bad_request_io(stage: &'static str, source: std::io::Error) -> Self {
        Self::bad_request(format!(
            "Palmistry request validation stage `{stage}` failed ({:?}): {source}",
            source.kind()
        ))
    }
}

impl IntoResponse for PalmistryLaunchError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"error": self.code, "detail": self.detail})),
        )
            .into_response()
    }
}

async fn launch(
    State(state): State<PalmistryLaunchState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    Json(request): Json<PalmistryLaunchRequest>,
) -> Result<Json<PalmistryLaunchResponse>, PalmistryLaunchError> {
    validate_request(&request)?;
    authenticate_native_launch(peer_addr, &request)?;
    {
        let mut active = state.active.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = active.get(&request.session_id) {
            return match existing {
                LaunchSlot::Active {
                    request: existing_request,
                    receipt,
                    signing_secret,
                    ..
                } if same_launch_identity(existing_request, &request) => {
                    launch_response(receipt.clone(), signing_secret, &request)
                }
                LaunchSlot::Launching(existing_request)
                    if same_launch_identity(existing_request, &request) =>
                {
                    Err(PalmistryLaunchError::unavailable(
                        "identical Palmistry launch is still starting",
                    ))
                }
                _ => Err(PalmistryLaunchError::bad_request(
                    "session_id is already bound to a different Palmistry launch identity",
                )),
            };
        }
        active.insert(request.session_id, LaunchSlot::Launching(request.clone()));
    }
    // The owned task is deliberately detached from the request future. If the
    // client disconnects or Axum drops this handler, the launch transaction
    // still reaches one terminal state: Active, or exact Launching-slot removal.
    let transaction_state = state.clone();
    let transaction_request = request.clone();
    let launch_result = tokio::spawn(async move {
        let _launching_slot_guard = LaunchingSlotGuard::new(
            Arc::clone(&transaction_state.active),
            transaction_request.clone(),
        );
        let result = match reattach_durable_launch(&transaction_state, &transaction_request).await {
            Ok(Some(reattached)) => Ok(reattached),
            Ok(None) => launch_validated(&transaction_state, &transaction_request).await,
            Err(error) => Err(error),
        };
        result
    })
    .await
    .map_err(|error| {
        PalmistryLaunchError::unavailable(format!(
            "Palmistry launch transaction task failed: {error}"
        ))
    })?;
    let (receipt, signing_secret) = launch_result?;
    launch_response(receipt, &signing_secret, &request)
}

fn same_launch_identity(left: &PalmistryLaunchRequest, right: &PalmistryLaunchRequest) -> bool {
    left.session_id == right.session_id
        && left.launch_nonce == right.launch_nonce
        && left.parent_pid == right.parent_pid
        && left.ring == right.ring
        && left.survivor_dir == right.survivor_dir
        && left.panic_signal == right.panic_signal
        && left.panic_ack == right.panic_ack
        && left.shutdown_signal == right.shutdown_signal
        && left.ready_signal == right.ready_signal
}

fn remove_matching_launching_slot(
    active: &Arc<Mutex<HashMap<Uuid, LaunchSlot>>>,
    request: &PalmistryLaunchRequest,
) -> bool {
    let mut active = active.lock().unwrap_or_else(|error| error.into_inner());
    let matches = matches!(
        active.get(&request.session_id),
        Some(LaunchSlot::Launching(existing)) if same_launch_identity(existing, request)
    );
    if matches {
        active.remove(&request.session_id);
    }
    matches
}

fn launch_response(
    receipt: PalmistryLaunchReceipt,
    signing_secret: &SessionSigningSecret,
    request: &PalmistryLaunchRequest,
) -> Result<Json<PalmistryLaunchResponse>, PalmistryLaunchError> {
    Ok(Json(PalmistryLaunchResponse {
        receipt,
        argus_signing_secret_envelope: signing_secret.seal_for_transport(request)?,
    }))
}

async fn reattach_durable_launch(
    state: &PalmistryLaunchState,
    request: &PalmistryLaunchRequest,
) -> Result<Option<(PalmistryLaunchReceipt, SessionSigningSecret)>, PalmistryLaunchError> {
    let verifiers: Vec<DurableWatcherVerifier> = state
        .palmistry_store
        .active_for_session(request.session_id)
    .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?
        .into_iter()
        .map(Into::into)
        .collect();
    if verifiers.is_empty() {
        return Ok(None);
    }
    let [verifier] = verifiers.as_slice() else {
        return Err(PalmistryLaunchError::unavailable(
            "durable Palmistry launch identity is ambiguous",
        ));
    };
    if verifier.launch_nonce != request.launch_nonce
        || verifier.parent_pid != i64::from(request.parent_pid)
    {
        return Err(PalmistryLaunchError::bad_request(
            "active durable Palmistry launch belongs to a different native identity",
        ));
    }
    verify_durable_watcher_identity(state, verifier).await?;
    let watcher_pid = u32::try_from(verifier.watcher_pid)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid durable watcher pid"))?;
    let actual_creation =
        crate::sandbox::handshake_native::process_creation_time_100ns(watcher_pid)
            .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
    if actual_creation
        != u64::try_from(verifier.watcher_creation_time_100ns)
            .map_err(|_| PalmistryLaunchError::bad_request("invalid durable creation identity"))?
    {
        return Err(PalmistryLaunchError::unavailable(
            "durable Palmistry pid was reused; relaunch requires reconciled STOP",
        ));
    }
    let receipt = PalmistryLaunchReceipt {
        session_id: request.session_id,
        process_uuid: verifier.process_uuid,
        os_pid: watcher_pid,
        sandbox_adapter_id: PALMISTRY_WATCHER_ADAPTER_ID,
        ledger_start_durable: true,
        os_creation_time_100ns: actual_creation,
    };
    let signing_secret = SessionSigningSecret::generate();
    state
        .active
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(
            request.session_id,
            LaunchSlot::Active {
                request: request.clone(),
                receipt: receipt.clone(),
                signing_secret: signing_secret.clone(),
                watcher_verifying_key: verifier.verifying_key()?,
                diagnostic_envelopes: std::collections::VecDeque::new(),
                lifecycle_ownership: WatcherLifecycleOwnership::DetachedReattached,
            },
        );
    Ok(Some((receipt, signing_secret)))
}

async fn launch_validated(
    state: &PalmistryLaunchState,
    request: &PalmistryLaunchRequest,
) -> Result<(PalmistryLaunchReceipt, SessionSigningSecret), PalmistryLaunchError> {
    let reservation = state
        .ledger
        .try_reserve_lifecycles(1)
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?
        .pop()
        .ok_or_else(|| PalmistryLaunchError::unavailable("missing lifecycle reservation"))?;
    let watcher_signing_secret = WatcherSigningSecret::generate()?;
    let watcher_verifying_key = watcher_signing_secret.verifying_key_bytes();
    let spec = PalmistrySpawnSpec {
        session_id: request.session_id,
        launch_nonce: request.launch_nonce,
        parent_pid: request.parent_pid,
        ring: request.ring.clone(),
        survivor_dir: request.survivor_dir.clone(),
        panic_signal: request.panic_signal.clone(),
        panic_ack: request.panic_ack.clone(),
        shutdown_signal: request.shutdown_signal.clone(),
        ready_signal: request.ready_signal.clone(),
        watcher_signing_secret: Arc::clone(&watcher_signing_secret.0),
    };
    let spawned = PalmistryWatcherAdapter::spawn(&spec)
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
    let (mut launch_attempt, executable_sha256, os_creation_time_100ns) =
        PalmistryLaunchAttempt::new(spawned);
    let os_pid = launch_attempt.child().id();
    let process_uuid = Uuid::now_v7();
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "handshake-native",
        Some(OWNER_WP.to_owned()),
    )
    .with_process_uuid(process_uuid)
    .with_os_pid(os_pid)
    .with_parent_session_id(request.session_id.to_string())
    .with_sandbox_adapter_id(PALMISTRY_WATCHER_ADAPTER_ID)
    .with_sandbox_internal_id(process_uuid.to_string())
    .with_mt_id("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1-MT-011")
    .with_sandbox_capabilities_snapshot(json!({
        "network": "not_os_enforced; Palmistry has no network code path",
        "stdio": "stdout/stderr null; stdin is a closed one-shot inherited key pipe",
        "lifecycle": "observer_survives_monitored_parent",
        "quiet": true,
        "isolation_boundary": false
    }))
    .with_metadata_jsonb(json!({
        "watcher": "palmistry",
        "monitored_parent_pid": request.parent_pid,
        "executable_sha256": executable_sha256.clone(),
        "os_creation_time_100ns": os_creation_time_100ns,
        "ring_schema": "hsk.internal_diagnostics.snapshot@1"
    }));
    let (lifecycle, durable_start) = match reservation.begin_with_durable_ack(start) {
        Ok(value) => value,
        Err(error) => {
            launch_attempt.terminate_and_wait();
            return Err(PalmistryLaunchError::unavailable(error.to_string()));
        }
    };
    if let Err(error) = durable_start.wait(LEDGER_ACK_TIMEOUT).await {
        lifecycle.leave_open_for_reconciliation();
        launch_attempt.terminate_and_wait();
        return Err(PalmistryLaunchError::unavailable(error.to_string()));
    }
    let register_verifier = state
        .palmistry_store
        .register(SurrealPalmistryVerifier {
            session_id: request.session_id,
            launch_nonce: request.launch_nonce,
            parent_pid: i64::from(request.parent_pid),
            watcher_pid: i64::from(os_pid),
            watcher_creation_time_100ns: i64::try_from(os_creation_time_100ns).map_err(|_| {
                PalmistryLaunchError::unavailable(
                    "Palmistry creation identity exceeds embedded integer range",
                )
            })?,
            process_uuid,
            executable_sha256: executable_sha256.clone(),
            verifying_key_hex: hex::encode(watcher_verifying_key),
        })
        .await;
    if let Err(error) = register_verifier {
        lifecycle.leave_open_for_reconciliation();
        launch_attempt.terminate_and_wait();
        return Err(PalmistryLaunchError::unavailable(format!(
            "cannot durably register Palmistry public verifier: {error}"
        )));
    }
    if let Err(error) = wait_for_readiness(
        request,
        os_pid,
        os_creation_time_100ns,
        &watcher_verifying_key,
    )
    .await
    {
        let exit_code = launch_attempt.terminate_and_wait();
        match lifecycle
            .stop_with_durable_ack(exit_code, "palmistry-readiness-failed", LEDGER_ACK_TIMEOUT)
            .await
        {
            Ok(StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped) => {}
            Ok(
                StopRecordOutcome::LeftOpenForReconciliation
                | StopRecordOutcome::DurabilityUnconfirmed,
            )
            | Err(_) => {
                lifecycle.leave_open_for_reconciliation();
            }
        }
        let _ = state
            .palmistry_store
            .retire_exact(request.session_id, request.launch_nonce, process_uuid)
            .await;
        return Err(error);
    }

    let receipt = PalmistryLaunchReceipt {
        session_id: request.session_id,
        process_uuid,
        os_pid,
        sandbox_adapter_id: PALMISTRY_WATCHER_ADAPTER_ID,
        ledger_start_durable: true,
        os_creation_time_100ns,
    };
    let signing_secret = SessionSigningSecret::generate();
    let active = Arc::clone(&state.active);
    let palmistry_store = state.palmistry_store.clone();
    let runtime_handle = tokio::runtime::Handle::current();
    let session_id = request.session_id;
    let launch_nonce = request.launch_nonce;
    let reaper_process_uuid = process_uuid;
    let (reaper_tx, reaper_rx) =
        std::sync::mpsc::sync_channel::<(std::process::Child, ActiveProcessLifecycle)>(1);
    let reaper = std::thread::Builder::new()
        .name(format!("palmistry-reaper-{session_id}"))
        .spawn(move || {
            let Ok((mut child, lifecycle)) = reaper_rx.recv() else {
                return;
            };
            let result = child.wait();
            if finish_lifecycle(&runtime_handle, lifecycle, result) {
                match runtime_handle.block_on(palmistry_store.retire_exact(
                    session_id,
                    launch_nonce,
                    reaper_process_uuid,
                )) {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::warn!(
                            %session_id,
                            %launch_nonce,
                            process_uuid = %reaper_process_uuid,
                            "Palmistry watcher STOP is durable but its exact verifier row is missing"
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            %session_id,
                            %launch_nonce,
                            process_uuid = %reaper_process_uuid,
                            error = %error,
                            "Palmistry watcher STOP is durable but verifier retirement needs reconciliation"
                        );
                    }
                }
            }
            // Child exit is authoritative for in-memory liveness even when the
            // durable STOP/retirement path failed. Never keep serving a cached
            // Active receipt for an exact watcher the reaper has joined.
            remove_matching_active_slot(
                &active,
                session_id,
                launch_nonce,
                reaper_process_uuid,
            );
        });
    if let Err(error) = reaper {
        launch_attempt.terminate_and_wait();
        lifecycle.leave_open_for_reconciliation();
        return Err(PalmistryLaunchError::unavailable(error.to_string()));
    }
    state
        .active
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(
            session_id,
            LaunchSlot::Active {
                request: request.clone(),
                receipt: receipt.clone(),
                signing_secret: signing_secret.clone(),
                watcher_verifying_key,
                diagnostic_envelopes: std::collections::VecDeque::new(),
                lifecycle_ownership: WatcherLifecycleOwnership::AttachedReaper,
            },
        );
    if let Err(error) = reaper_tx.send((launch_attempt.take_child(), lifecycle)) {
        let (mut child, lifecycle) = error.0;
        let _ = child.kill();
        let _ = child.wait();
        lifecycle.leave_open_for_reconciliation();
        state
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&session_id);
        return Err(PalmistryLaunchError::unavailable(
            "Palmistry reaper stopped before ownership transfer",
        ));
    }
    Ok((receipt, signing_secret))
}

fn finish_lifecycle(
    runtime_handle: &tokio::runtime::Handle,
    lifecycle: ActiveProcessLifecycle,
    result: std::io::Result<std::process::ExitStatus>,
) -> bool {
    let Ok(status) = result else {
        lifecycle.leave_open_for_reconciliation();
        return false;
    };
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime_handle.block_on(lifecycle.stop_with_durable_ack(
            status.code(),
            "palmistry-watcher-exited",
            LEDGER_ACK_TIMEOUT,
        ))
    }));
    match stop {
        Ok(Ok(StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped)) => true,
        Ok(Ok(
            StopRecordOutcome::LeftOpenForReconciliation | StopRecordOutcome::DurabilityUnconfirmed,
        ))
        | Ok(Err(_))
        | Err(_) => {
            lifecycle.leave_open_for_reconciliation();
            false
        }
    }
}

fn remove_matching_active_slot(
    active: &Arc<Mutex<HashMap<Uuid, LaunchSlot>>>,
    session_id: Uuid,
    launch_nonce: Uuid,
    process_uuid: Uuid,
) -> bool {
    let mut active = active.lock().unwrap_or_else(|error| error.into_inner());
    let matches = matches!(
        active.get(&session_id),
        Some(LaunchSlot::Active {
            request,
            receipt,
            ..
        }) if request.launch_nonce == launch_nonce && receipt.process_uuid == process_uuid
    );
    if matches {
        active.remove(&session_id);
    }
    matches
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadyRecord {
    schema_id: String,
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: u32,
    watcher_pid: u32,
    watcher_creation_time_100ns: u64,
    source_proof: String,
}

async fn wait_for_readiness(
    request: &PalmistryLaunchRequest,
    watcher_pid: u32,
    watcher_creation_time_100ns: u64,
    watcher_verifying_key: &[u8; 32],
) -> Result<(), PalmistryLaunchError> {
    let started = Instant::now();
    while started.elapsed() < READY_TIMEOUT {
        if let Ok(bytes) = fs::read(&request.ready_signal) {
            let ready: ReadyRecord = serde_json::from_slice(&bytes)
                .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
            if ready.schema_id == "hsk.palmistry.ready@1"
                && ready.session_id == request.session_id
                && ready.launch_nonce == request.launch_nonce
                && ready.parent_pid == request.parent_pid
                && ready.watcher_pid == watcher_pid
                && ready.watcher_creation_time_100ns == watcher_creation_time_100ns
                && verify_watcher_source_proof(&ready, &ready.source_proof, watcher_verifying_key)
                    .is_ok()
            {
                let _ = fs::remove_file(&request.ready_signal);
                return Ok(());
            }
            return Err(PalmistryLaunchError::unavailable(
                "Palmistry readiness identity mismatch",
            ));
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    Err(PalmistryLaunchError::unavailable(
        "Palmistry did not become ready within the bounded startup interval",
    ))
}

fn verify_watcher_source_proof<T: Serialize>(
    record: &T,
    source_proof: &str,
    watcher_verifying_key: &[u8; 32],
) -> Result<(), PalmistryLaunchError> {
    if source_proof.len() != 128 || !source_proof.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PalmistryLaunchError::bad_request(
            "invalid Palmistry watcher source proof",
        ));
    }
    let mut value = serde_json::to_value(record)
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    value
        .as_object_mut()
        .ok_or_else(|| PalmistryLaunchError::bad_request("Palmistry record is not an object"))?
        .insert(
            "source_proof".to_owned(),
            serde_json::Value::String(String::new()),
        );
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    let proof: [u8; 64] = hex::decode(source_proof)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Palmistry source proof"))?
        .try_into()
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Palmistry signature length"))?;
    let verifying_key = VerifyingKey::from_bytes(watcher_verifying_key)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Palmistry verifying key"))?;
    verifying_key
        .verify_strict(&bytes, &Signature::from_bytes(&proof))
        .map_err(|_| PalmistryLaunchError::bad_request("Palmistry source proof mismatch"))
}

async fn recover_survivor(
    State(state): State<PalmistryLaunchState>,
    Json(request): Json<PalmistryRecoverRequest>,
) -> Result<Json<serde_json::Value>, PalmistryLaunchError> {
    let signing_secret = {
        let active = state.active.lock().unwrap_or_else(|e| e.into_inner());
        let Some(LaunchSlot::Active {
            request: launch,
            signing_secret,
            ..
        }) = active.get(&request.current_session_id)
        else {
            return Err(PalmistryLaunchError::bad_request(
                "recovery importer is not bound to an active Palmistry session",
            ));
        };
        if launch.launch_nonce != request.launch_nonce {
            return Err(PalmistryLaunchError::bad_request(
                "recovery importer nonce mismatch",
            ));
        }
        signing_secret.clone()
    };
    validate_summary(&request.summary)?;
    verify_recovery_proof(&request, signing_secret.as_bytes())?;
    let filter = EventFilter {
        event_id: Some(request.summary.record_id),
        ..EventFilter::default()
    };
    let existing = state
        .recorder
        .list_events(filter)
        .await
        .map_err(|e| PalmistryLaunchError::unavailable(e.to_string()))?;
    if !existing.is_empty() {
        let summary_digest = recovery_summary_digest(&request.summary);
        let matching_event = existing
            .iter()
            .find(|event| {
                event.event_type == FlightRecorderEventType::Diagnostic
                    && event.actor_id == "palmistry"
                    && event.trace_id == request.summary.source_session_id
                    && event
                        .payload
                        .get("diagnostic_id")
                        .and_then(|value| value.as_str())
                        == Some("palmistry.survivor_recovered")
                    && event
                        .payload
                        .get("verified_recovery_summary_sha256")
                        .and_then(|value| value.as_str())
                        == Some(summary_digest.as_str())
            })
            .ok_or_else(|| {
                PalmistryLaunchError::bad_request(
                    "recovery record_id collides with a non-matching durable event",
                )
            })?;
        let source_process_uuid = matching_event
            .payload
            .get("source_process_uuid")
            .and_then(|value| value.as_str())
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| {
                PalmistryLaunchError::bad_request(
                    "durable recovery event is missing its exact source process identity",
                )
            })?;
        let source_launch_nonce = matching_event
            .payload
            .get("source_launch_nonce")
            .and_then(|value| value.as_str())
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| {
                PalmistryLaunchError::bad_request(
                    "durable recovery event is missing its exact source launch identity",
                )
            })?;
        let source_shutdown_signal = {
            let active = state.active.lock().unwrap_or_else(|e| e.into_inner());
            match active.get(&request.summary.source_session_id) {
                Some(LaunchSlot::Active { request, .. }) => request.shutdown_signal.clone(),
                _ => diagnostics_root().join(format!(
                    "shutdown-{}.signal",
                    request.summary.source_session_id
                )),
            }
        };
        let cleanup_complete = shutdown_recovered_source(
            &state,
            &request,
            &source_shutdown_signal,
            Some(source_process_uuid),
        )
        .await;
        if cleanup_complete {
            retire_recovered_source(
                &state,
                request.summary.source_session_id,
                source_launch_nonce,
                source_process_uuid,
            )
            .await?;
        }
        return Ok(Json(json!({
            "imported": true,
            "idempotent": true,
            "cleanup_pending": !cleanup_complete
        })));
    }
    let source_verifiers: Vec<DurableWatcherVerifier> = state
        .palmistry_store
        .active_for_session(request.summary.source_session_id)
        .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?
        .into_iter()
        .map(Into::into)
        .collect();
    let [source_verifier] = source_verifiers.as_slice() else {
        return Err(PalmistryLaunchError::bad_request(
            "source Palmistry verifier is missing or ambiguous",
        ));
    };
    verify_durable_watcher_identity(&state, source_verifier).await?;
    let source_survivor_dir = diagnostics_root().join("survivors");
    let durable_summary = read_survivor_summary(
        &source_survivor_dir,
        source_verifier,
        request.summary.observed_at_unix_ms,
        request.summary.record_id,
    )?;
    if durable_summary != request.summary {
        return Err(PalmistryLaunchError::bad_request(
            "recovery summary does not match the durable Palmistry survivor artifact",
        ));
    }
    let payload = json!({
        "diagnostic_id": "palmistry.survivor_recovered",
        "type": "palmistry.survivor_recovered",
        "record_id": request.summary.record_id,
        "kind": request.summary.kind,
        "observed_at_unix_ms": request.summary.observed_at_unix_ms,
        "parent_pid": request.summary.parent_pid,
        "parent_exit_code": request.summary.parent_exit_code,
        "heartbeat_stale_ms": request.summary.heartbeat_stale_ms,
        "os_hung_window_confirmed": request.summary.os_hung_window_confirmed,
        "minidump_status": request.summary.minidump_status,
        "source_launch_nonce": source_verifier.launch_nonce,
        "source_process_uuid": source_verifier.process_uuid,
        "verified_recovery_summary_sha256": recovery_summary_digest(&request.summary),
    });
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::Diagnostic,
        FlightRecorderActor::System,
        request.summary.source_session_id,
        payload,
    )
    .with_actor_id("palmistry");
    event.event_id = request.summary.record_id;
    state
        .recorder
        .record_event(event)
        .await
        .map_err(|e| PalmistryLaunchError::unavailable(e.to_string()))?;
    let source_shutdown_signal = diagnostics_root().join(format!(
        "shutdown-{}.signal",
        request.summary.source_session_id
    ));
    let cleanup_complete = shutdown_recovered_source(
        &state,
        &request,
        &source_shutdown_signal,
        Some(source_verifier.process_uuid),
    )
    .await;
    if cleanup_complete {
        retire_recovered_source(
            &state,
            source_verifier.session_id,
            source_verifier.launch_nonce,
            source_verifier.process_uuid,
        )
        .await?;
    }
    Ok(Json(json!({
        "imported": true,
        "idempotent": false,
        "cleanup_pending": !cleanup_complete
    })))
}

async fn verify_durable_watcher_identity(
    state: &PalmistryLaunchState,
    source_verifier: &DurableWatcherVerifier,
) -> Result<(), PalmistryLaunchError> {
    let verifier = SurrealPalmistryVerifier::from(source_verifier);
    let matches = state
        .palmistry_store
        .active_process_matches(&verifier)
        .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
    if !matches {
        return Err(PalmistryLaunchError::bad_request(
            "durable Palmistry verifier does not match ProcessOwnershipLedger launch identity",
        ));
    }
    Ok(())
}

async fn shutdown_recovered_source(
    state: &PalmistryLaunchState,
    request: &PalmistryRecoverRequest,
    source_shutdown_signal: &Path,
    source_process_uuid: Option<Uuid>,
) -> bool {
    if !source_shutdown_is_allowed(
        request.summary.source_session_id,
        request.current_session_id,
    ) {
        // Importing a freeze from the currently running watcher must never retire or stop that
        // verifier. A later freeze episode needs the same live signing identity.
        return false;
    }
    let lifecycle_ownership = {
        let active = state
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match active.get(&request.summary.source_session_id) {
            Some(LaunchSlot::Active {
                lifecycle_ownership,
                ..
            }) => Some(*lifecycle_ownership),
            _ => None,
        }
    };
    let has_attached_reaper = source_has_attached_reaper(lifecycle_ownership);
    let guarded_reclaim_allowed = source_allows_guarded_reclaim(lifecycle_ownership);
    let shutdown_signal_written = match fs::write(
        source_shutdown_signal,
        b"backend_authenticated_recovery\n",
    ) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(
                source_session_id = %request.summary.source_session_id,
                error = %error,
                has_attached_reaper,
                "Palmistry survivor was imported but its authenticated shutdown signal could not be written"
            );
            if !guarded_reclaim_allowed {
                return false;
            }
            false
        }
    };
    if shutdown_signal_written {
        let started = Instant::now();
        let graceful_wait = if has_attached_reaper {
            Duration::from_secs(4)
        } else {
            Duration::from_millis(250)
        };
        while started.elapsed() < graceful_wait {
            if let Some(process_uuid) = source_process_uuid {
                match palmistry_stop_is_durable(state, process_uuid).await {
                    Ok(true) => return true,
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(
                            source_session_id = %request.summary.source_session_id,
                            process_uuid = %process_uuid,
                            error = %error,
                            "Palmistry survivor cleanup could not read durable STOP state"
                        );
                        return false;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    if !guarded_reclaim_allowed {
        return false;
    }
    let Some(process_uuid) = source_process_uuid else {
        return false;
    };
    if let Err(error) = state
        .reclaim
        .run_process(
            &request.summary.source_session_id.to_string(),
            process_uuid,
            ReclaimTrigger::Restart,
        )
        .await
    {
        tracing::warn!(
            source_session_id = %request.summary.source_session_id,
            process_uuid = %process_uuid,
            error = %error,
            "Palmistry detached restart cleanup did not complete"
        );
        return false;
    }
    match palmistry_stop_is_durable(state, process_uuid).await {
        Ok(stopped) => stopped,
        Err(error) => {
            tracing::warn!(
                source_session_id = %request.summary.source_session_id,
                process_uuid = %process_uuid,
                error = %error,
                "Palmistry detached cleanup completed without readable durable STOP proof"
            );
            false
        }
    }
}

fn source_has_attached_reaper(lifecycle_ownership: Option<WatcherLifecycleOwnership>) -> bool {
    lifecycle_ownership.is_some_and(WatcherLifecycleOwnership::has_attached_reaper)
}

fn source_allows_guarded_reclaim(lifecycle_ownership: Option<WatcherLifecycleOwnership>) -> bool {
    !source_has_attached_reaper(lifecycle_ownership)
}

fn source_shutdown_is_allowed(source_session_id: Uuid, current_session_id: Uuid) -> bool {
    source_session_id != current_session_id
}

async fn palmistry_stop_is_durable(
    state: &PalmistryLaunchState,
    process_uuid: Uuid,
) -> Result<bool, PalmistryLaunchError> {
    state
        .palmistry_store
        .stop_is_durable(process_uuid)
        .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))
}

async fn retire_recovered_source(
    state: &PalmistryLaunchState,
    session_id: Uuid,
    launch_nonce: Uuid,
    process_uuid: Uuid,
) -> Result<(), PalmistryLaunchError> {
    if !palmistry_stop_is_durable(state, process_uuid)
        .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?
    {
        return Err(PalmistryLaunchError::unavailable(
            "Palmistry source retirement requires durable STOP",
        ));
    }
    let verifier_retired = state
        .palmistry_store
        .retire_exact(session_id, launch_nonce, process_uuid)
        .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
    if !verifier_retired {
        return Err(PalmistryLaunchError::unavailable(
            "Palmistry source verifier identity is missing during retirement",
        ));
    }
    remove_matching_active_slot(&state.active, session_id, launch_nonce, process_uuid);
    Ok(())
}

fn recovery_proof_bytes(request: &PalmistryRecoverRequest) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        request.current_session_id.to_string(),
        request.launch_nonce.to_string(),
        request.summary.record_id.to_string(),
        request.summary.source_session_id.to_string(),
        request.summary.kind.clone(),
        request.summary.observed_at_unix_ms.to_string(),
        request.summary.parent_pid.to_string(),
        request
            .summary
            .parent_exit_code
            .map(|value| value.to_string())
            .unwrap_or_default(),
        request
            .summary
            .heartbeat_stale_ms
            .map(|value| value.to_string())
            .unwrap_or_default(),
        request.summary.os_hung_window_confirmed.to_string(),
        request.summary.minidump_status.clone(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes
}

fn recovery_summary_digest(summary: &RecoveredSurvivorSummary) -> String {
    let mut bytes = Vec::new();
    for value in [
        summary.record_id.to_string(),
        summary.source_session_id.to_string(),
        summary.kind.clone(),
        summary.observed_at_unix_ms.to_string(),
        summary.parent_pid.to_string(),
        summary
            .parent_exit_code
            .map(|value| value.to_string())
            .unwrap_or_default(),
        summary
            .heartbeat_stale_ms
            .map(|value| value.to_string())
            .unwrap_or_default(),
        summary.os_hung_window_confirmed.to_string(),
        summary.minidump_status.clone(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    hex::encode(Sha256::digest(bytes))
}

fn verify_recovery_proof(
    request: &PalmistryRecoverRequest,
    signing_secret: &[u8],
) -> Result<(), PalmistryLaunchError> {
    if request.proof.len() != 64 || !request.proof.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PalmistryLaunchError::bad_request(
            "invalid Palmistry recovery proof",
        ));
    }
    let proof = hex::decode(&request.proof)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Palmistry recovery proof"))?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(signing_secret)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Palmistry recovery proof key"))?;
    mac.update(&recovery_proof_bytes(request));
    mac.verify_slice(&proof)
        .map_err(|_| PalmistryLaunchError::bad_request("Palmistry recovery proof mismatch"))
}

fn read_survivor_summary(
    survivor_dir: &Path,
    source_verifier: &DurableWatcherVerifier,
    observed_at_unix_ms: u64,
    record_id: Uuid,
) -> Result<RecoveredSurvivorSummary, PalmistryLaunchError> {
    // Caller-authenticated timestamp plus record id form the writer's exact durable filename;
    // recovery never performs an attacker-amplifiable full-directory scan.
    let path = survivor_dir.join(format!("survivor-{observed_at_unix_ms}-{record_id}.json"));
    if is_symlink_or_reparse(&path)
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?
    {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry survivor artifact may not be a symlink or reparse point",
        ));
    }
    if fs::metadata(&path)
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?
        .len()
        > 2 * 1024 * 1024
    {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry survivor artifact exceeds the 2 MiB bound",
        ));
    }
    let mut value: serde_json::Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?,
    )
    .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    if value.get("schema_id").and_then(|value| value.as_str()) != Some("hsk.palmistry.survivor@1") {
        return Err(PalmistryLaunchError::bad_request(
            "invalid Palmistry survivor schema",
        ));
    }
    let source_proof = value
        .get("source_proof")
        .and_then(|value| value.as_str())
        .ok_or_else(|| PalmistryLaunchError::bad_request("missing survivor source proof"))?
        .to_owned();
    let proof: [u8; 64] = hex::decode(&source_proof)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid survivor source proof"))?
        .try_into()
        .map_err(|_| PalmistryLaunchError::bad_request("invalid survivor signature length"))?;
    value
        .as_object_mut()
        .ok_or_else(|| PalmistryLaunchError::bad_request("survivor artifact is not an object"))?
        .insert(
            "source_proof".to_owned(),
            serde_json::Value::String(String::new()),
        );
    let canonical = serde_json::to_vec(&value)
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    let verifying_key = VerifyingKey::from_bytes(&source_verifier.verifying_key()?)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid durable Palmistry verifier"))?;
    verifying_key
        .verify_strict(&canonical, &Signature::from_bytes(&proof))
        .map_err(|_| PalmistryLaunchError::bad_request("survivor source proof mismatch"))?;
    let expected_session = source_verifier.session_id.to_string();
    let expected_nonce = source_verifier.launch_nonce.to_string();
    if value.get("session_id").and_then(|value| value.as_str()) != Some(expected_session.as_str())
        || value.get("launch_nonce").and_then(|value| value.as_str())
            != Some(expected_nonce.as_str())
        || value.get("watcher_pid").and_then(|value| value.as_u64())
            != u64::try_from(source_verifier.watcher_pid).ok()
        || value
            .get("watcher_creation_time_100ns")
            .and_then(|value| value.as_u64())
            != u64::try_from(source_verifier.watcher_creation_time_100ns).ok()
        || value.get("parent_pid").and_then(|value| value.as_u64())
            != u64::try_from(source_verifier.parent_pid).ok()
    {
        return Err(PalmistryLaunchError::bad_request(
            "survivor source process identity does not match the active Palmistry launch",
        ));
    }
    let string = |field: &str| {
        value
            .get(field)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .ok_or_else(|| PalmistryLaunchError::bad_request(format!("missing survivor {field}")))
    };
    let unsigned = |field: &str| {
        value
            .get(field)
            .and_then(|value| value.as_u64())
            .ok_or_else(|| PalmistryLaunchError::bad_request(format!("missing survivor {field}")))
    };
    let summary = RecoveredSurvivorSummary {
        record_id: Uuid::parse_str(&string("record_id")?)
            .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?,
        source_session_id: Uuid::parse_str(&string("session_id")?)
            .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?,
        kind: string("kind")?,
        observed_at_unix_ms: unsigned("observed_at_unix_ms")?,
        parent_pid: u32::try_from(unsigned("parent_pid")?)
            .map_err(|_| PalmistryLaunchError::bad_request("survivor parent_pid overflow"))?,
        parent_exit_code: value
            .get("parent_exit_code")
            .and_then(|value| value.as_u64())
            .map(u32::try_from)
            .transpose()
            .map_err(|_| PalmistryLaunchError::bad_request("survivor exit code overflow"))?,
        heartbeat_stale_ms: value
            .get("heartbeat_stale_ms")
            .and_then(|value| value.as_u64()),
        os_hung_window_confirmed: value
            .get("os_hung_window_confirmed")
            .and_then(|value| value.as_bool())
            .ok_or_else(|| {
                PalmistryLaunchError::bad_request("missing survivor hung-window verdict")
            })?,
        minidump_status: string("minidump_status")?,
        imported: false,
    };
    validate_summary(&summary)?;
    Ok(summary)
}

fn validate_summary(summary: &RecoveredSurvivorSummary) -> Result<(), PalmistryLaunchError> {
    const KINDS: &[&str] = &["panic", "unexpected_exit", "gui_freeze"];
    const DUMP_STATUSES: &[&str] = &[
        "written",
        "failed_while_running",
        "failed_after_exit",
        "unsupported",
        "not_requested",
    ];
    let now = unix_ms_now();
    let timestamp_fresh = survivor_timestamp_is_fresh(now, summary.observed_at_unix_ms);
    if summary.record_id.is_nil()
        || summary.source_session_id.is_nil()
        || !KINDS.contains(&summary.kind.as_str())
        || !DUMP_STATUSES.contains(&summary.minidump_status.as_str())
        || !timestamp_fresh
    {
        return Err(PalmistryLaunchError::bad_request(
            "invalid sanitized Palmistry survivor summary",
        ));
    }
    Ok(())
}

fn unix_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn survivor_timestamp_is_fresh(now: u64, observed_at: u64) -> bool {
    observed_at <= now.saturating_add(SURVIVOR_MAX_FUTURE_SKEW_MS)
        && now.saturating_sub(observed_at) <= SURVIVOR_MAX_AGE_MS
}

fn authenticate_native_launch(
    peer_addr: SocketAddr,
    request: &PalmistryLaunchRequest,
) -> Result<(), PalmistryLaunchError> {
    if !peer_addr.ip().is_loopback() {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry launch must originate from a loopback connection",
        ));
    }
    let owner_pid = tcp_connection_owner_pid(peer_addr)?;
    if owner_pid != request.parent_pid {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry launch parent_pid does not own the authenticated TCP connection",
        ));
    }
    verify_handshake_native_process(owner_pid)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn tcp_connection_owner_pid(peer_addr: SocketAddr) -> Result<u32, PalmistryLaunchError> {
    use std::ffi::c_void;

    const AF_INET: u32 = 2;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    const MIB_TCP_STATE_ESTAB: u32 = 5;
    const TCP_TABLE_OWNER_PID_CONNECTIONS: u32 = 4;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct TcpRowOwnerPid {
        state: u32,
        local_addr: u32,
        local_port: u32,
        remote_addr: u32,
        remote_port: u32,
        owning_pid: u32,
    }

    #[link(name = "iphlpapi")]
    unsafe extern "system" {
        fn GetExtendedTcpTable(
            table: *mut c_void,
            size: *mut u32,
            order: i32,
            address_family: u32,
            table_class: u32,
            reserved: u32,
        ) -> u32;
    }

    let SocketAddr::V4(peer) = peer_addr else {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry launch requires an IPv4 loopback connection",
        ));
    };
    let mut byte_len = 0_u32;
    let first = unsafe {
        GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut byte_len,
            0,
            AF_INET,
            TCP_TABLE_OWNER_PID_CONNECTIONS,
            0,
        )
    };
    if first != ERROR_INSUFFICIENT_BUFFER || byte_len < std::mem::size_of::<u32>() as u32 {
        return Err(PalmistryLaunchError::unavailable(format!(
            "cannot size the Windows TCP ownership table (status {first})"
        )));
    }
    let word_len = (byte_len as usize).div_ceil(std::mem::size_of::<u32>());
    let mut words = vec![0_u32; word_len];
    let status = unsafe {
        GetExtendedTcpTable(
            words.as_mut_ptr().cast::<c_void>(),
            &mut byte_len,
            0,
            AF_INET,
            TCP_TABLE_OWNER_PID_CONNECTIONS,
            0,
        )
    };
    if status != 0 {
        return Err(PalmistryLaunchError::unavailable(format!(
            "cannot read the Windows TCP ownership table (status {status})"
        )));
    }
    let row_count = words[0] as usize;
    let required = std::mem::size_of::<u32>()
        .checked_add(
            row_count
                .checked_mul(std::mem::size_of::<TcpRowOwnerPid>())
                .ok_or_else(|| PalmistryLaunchError::unavailable("TCP ownership table overflow"))?,
        )
        .ok_or_else(|| PalmistryLaunchError::unavailable("TCP ownership table overflow"))?;
    if required > byte_len as usize {
        return Err(PalmistryLaunchError::unavailable(
            "Windows TCP ownership table was truncated",
        ));
    }
    let rows = unsafe {
        std::slice::from_raw_parts(words.as_ptr().add(1).cast::<TcpRowOwnerPid>(), row_count)
    };
    let expected_addr = peer.ip().octets();
    let expected_port = peer.port();
    let mut owner = None;
    for row in rows {
        if row.state != MIB_TCP_STATE_ESTAB
            || row.local_addr.to_ne_bytes() != expected_addr
            || u16::from_be(row.local_port as u16) != expected_port
            || !Ipv4Addr::from(row.remote_addr.to_ne_bytes()).is_loopback()
        {
            continue;
        }
        if owner.replace(row.owning_pid).is_some() {
            return Err(PalmistryLaunchError::unavailable(
                "ambiguous Windows TCP ownership identity",
            ));
        }
    }
    owner.ok_or_else(|| {
        PalmistryLaunchError::bad_request(
            "Palmistry launch connection has no authenticated Windows process owner",
        )
    })
}

#[cfg(not(target_os = "windows"))]
fn tcp_connection_owner_pid(_peer_addr: SocketAddr) -> Result<u32, PalmistryLaunchError> {
    Err(PalmistryLaunchError::unavailable(
        "authenticated Palmistry launch is currently supported only on Windows",
    ))
}

#[cfg(target_os = "windows")]
fn verify_handshake_native_process(pid: u32) -> Result<(), PalmistryLaunchError> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::Threading::{
            GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    };

    struct ProcessHandle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(PalmistryLaunchError::bad_request(format!(
            "cannot open authenticated native pid {pid}: {}",
            std::io::Error::last_os_error()
        )));
    }
    let handle = ProcessHandle(handle);
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe { GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(PalmistryLaunchError::bad_request(format!(
            "cannot read authenticated native process generation: {}",
            std::io::Error::last_os_error()
        )));
    }
    let mut path = vec![0_u16; 32_768];
    let mut path_len = path.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, path.as_mut_ptr(), &mut path_len) } == 0 {
        return Err(PalmistryLaunchError::bad_request(format!(
            "cannot read authenticated native executable path: {}",
            std::io::Error::last_os_error()
        )));
    }
    path.truncate(path_len as usize);
    let actual = PathBuf::from(String::from_utf16(&path).map_err(|error| {
        PalmistryLaunchError::bad_request(format!(
            "authenticated native executable path is invalid UTF-16: {error}"
        ))
    })?);
    let actual = fs::canonicalize(&actual)
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    let expected = expected_handshake_native_executable()?;
    if !actual
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
    {
        return Err(PalmistryLaunchError::bad_request(
            "authenticated TCP owner is not the configured handshake-native executable",
        ));
    }
    let actual_hash = Sha256::digest(
        fs::read(&actual).map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?,
    );
    let expected_hash = Sha256::digest(
        fs::read(&expected)
            .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?,
    );
    if actual_hash != expected_hash {
        return Err(PalmistryLaunchError::bad_request(
            "authenticated native executable hash does not match the configured executable",
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn expected_handshake_native_executable() -> Result<PathBuf, PalmistryLaunchError> {
    let configured = std::env::var_os("HSK_HANDSHAKE_NATIVE_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(Path::to_path_buf))
                .unwrap_or_default()
                .join("handshake-native.exe")
        });
    if configured.file_name().and_then(|value| value.to_str()) != Some("handshake-native.exe") {
        return Err(PalmistryLaunchError::unavailable(
            "configured native executable must be named handshake-native.exe",
        ));
    }
    fs::canonicalize(configured)
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))
}

#[cfg(not(target_os = "windows"))]
fn verify_handshake_native_process(_pid: u32) -> Result<(), PalmistryLaunchError> {
    Err(PalmistryLaunchError::unavailable(
        "authenticated Palmistry launch is currently supported only on Windows",
    ))
}

fn validate_request(request: &PalmistryLaunchRequest) -> Result<(), PalmistryLaunchError> {
    if request.parent_pid == 0 || request.session_id.is_nil() || request.launch_nonce.is_nil() {
        return Err(PalmistryLaunchError::bad_request(
            "parent_pid must be non-zero",
        ));
    }
    decode_hex_array::<32>(&request.transport_public_key)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid transport public key"))?;
    for path in [
        &request.ring,
        &request.survivor_dir,
        &request.panic_signal,
        &request.panic_ack,
        &request.shutdown_signal,
        &request.ready_signal,
    ] {
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(PalmistryLaunchError::bad_request(
                "Palmistry paths must be absolute and contain no parent traversal",
            ));
        }
    }
    let root = request
        .ring
        .parent()
        .ok_or_else(|| PalmistryLaunchError::bad_request("ring has no parent"))?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| PalmistryLaunchError::bad_request_io("root-canonicalize", error))?;
    let canonical_expected = fs::canonicalize(diagnostics_root()).map_err(|error| {
        PalmistryLaunchError::bad_request_io("expected-root-canonicalize", error)
    })?;
    if canonical_root != canonical_expected
        || is_symlink_or_reparse(root).map_err(|error| {
            PalmistryLaunchError::bad_request_io("root-reparse-inspection", error)
        })?
    {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry diagnostics root is not the canonical owned diagnostics root",
        ));
    }
    let session = request.session_id.to_string();
    let expected = [
        (&request.ring, format!("ring-{session}.bin")),
        (
            &request.panic_signal,
            format!("panic-{session}.signal.json"),
        ),
        (&request.panic_ack, format!("panic-{session}.ack")),
        (
            &request.shutdown_signal,
            format!("shutdown-{session}.signal"),
        ),
        (&request.ready_signal, format!("ready-{session}.json")),
    ];
    for (path, file_name) in expected {
        if path.parent() != Some(root)
            || path.file_name().and_then(|value| value.to_str()) != Some(file_name.as_str())
        {
            return Err(PalmistryLaunchError::bad_request(
                "Palmistry paths do not match the session-scoped diagnostics root",
            ));
        }
    }
    let ring_is_file = fs::metadata(&request.ring)
        .map_err(|error| PalmistryLaunchError::bad_request_io("ring-metadata", error))?
        .is_file();
    if request.survivor_dir != root.join("survivors") || !ring_is_file {
        return Err(PalmistryLaunchError::bad_request(
            "survivor directory or diagnostics ring is invalid",
        ));
    }
    if fs::canonicalize(&request.survivor_dir).map_err(|error| {
        PalmistryLaunchError::bad_request_io("survivor-directory-canonicalize", error)
    })? != canonical_root.join("survivors")
        || is_symlink_or_reparse(&request.survivor_dir).map_err(|error| {
            PalmistryLaunchError::bad_request_io("survivor-directory-reparse-inspection", error)
        })?
        || is_symlink_or_reparse(&request.ring).map_err(|error| {
            PalmistryLaunchError::bad_request_io("ring-reparse-inspection", error)
        })?
    {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry paths may not use symlink or reparse redirection",
        ));
    }
    for path in [
        &request.panic_signal,
        &request.panic_ack,
        &request.shutdown_signal,
        &request.ready_signal,
    ] {
        if path.try_exists().map_err(|error| {
            PalmistryLaunchError::bad_request_io("signal-existence-check", error)
        })? && is_symlink_or_reparse(path).map_err(|error| {
            PalmistryLaunchError::bad_request_io("signal-reparse-inspection", error)
        })? {
            return Err(PalmistryLaunchError::bad_request(
                "Palmistry signal path may not be a symlink or reparse point",
            ));
        }
    }
    let snapshot = read_ring_snapshot(&request.ring)?;
    if snapshot.session_id != request.session_id
        || snapshot.launch_nonce != request.launch_nonce
        || snapshot.process_id != request.parent_pid
        || snapshot.schema_id != "hsk.internal_diagnostics.snapshot@1"
        || snapshot.ring_version != 1
    {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry ring identity does not match the authenticated launch",
        ));
    }
    if request.ready_signal.try_exists().map_err(|error| {
        PalmistryLaunchError::bad_request_io("ready-signal-existence-check", error)
    })? {
        fs::remove_file(&request.ready_signal).map_err(|error| {
            PalmistryLaunchError::bad_request_io("stale-ready-signal-removal", error)
        })?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct RingSnapshotIdentity {
    schema_id: String,
    ring_version: u32,
    session_id: Uuid,
    launch_nonce: Uuid,
    process_id: u32,
    heartbeat_counter: u64,
    heartbeat_unix_ms: u64,
}

fn read_ring_snapshot(path: &Path) -> Result<RingSnapshotIdentity, PalmistryLaunchError> {
    const HEADER: usize = 128;
    const SLOT: usize = 128 * 1024;
    const TOTAL: usize = HEADER + SLOT * 2;
    let bytes =
        fs::read(path).map_err(|error| PalmistryLaunchError::bad_request_io("ring-read", error))?;
    if bytes.len() != TOTAL || &bytes[..8] != b"HSKIDG01" {
        return Err(PalmistryLaunchError::bad_request(
            "invalid diagnostics ring",
        ));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let active = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let generation = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    if version != 1 || active > 1 || generation == 0 {
        return Err(PalmistryLaunchError::bad_request(
            "invalid diagnostics ring header",
        ));
    }
    let offset = HEADER + active * SLOT;
    let slot_generation = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
    let len = u32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
    if slot_generation != generation || len > SLOT - 44 {
        return Err(PalmistryLaunchError::bad_request(
            "unstable diagnostics ring slot",
        ));
    }
    let payload = &bytes[offset + 44..offset + 44 + len];
    if Sha256::digest(payload).as_slice() != &bytes[offset + 12..offset + 44] {
        return Err(PalmistryLaunchError::bad_request(
            "diagnostics ring hash mismatch",
        ));
    }
    serde_json::from_slice(payload).map_err(|e| PalmistryLaunchError::bad_request(e.to_string()))
}

fn diagnostics_root() -> PathBuf {
    if let Some(path) = std::env::var_os("HANDSHAKE_DIAGNOSTICS_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local).join("Handshake").join("diagnostics");
    }
    std::env::temp_dir().join("handshake").join("diagnostics")
}

fn is_symlink_or_reparse(path: &Path) -> std::io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        return Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0);
    }
    #[cfg(not(target_os = "windows"))]
    Ok(false)
}

async fn record_argus_receipt(
    State(state): State<PalmistryLaunchState>,
    Json(request): Json<ArgusActionReceiptRequest>,
) -> Result<Json<ArgusActionDurabilityReceipt>, PalmistryLaunchError> {
    validate_argus_receipt(&request)?;
    let flight_recorder_event_id = argus_action_uuid(&request.action_id)?;
    let signing_secret = {
        let active = state
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(LaunchSlot::Active { signing_secret, .. }) =
            active.get(&request.diagnostics_session_id)
        else {
            return Err(PalmistryLaunchError::bad_request(
                "Argus receipt is not bound to an active diagnostics session",
            ));
        };
        signing_secret.clone()
    };
    verify_argus_proof(&request, signing_secret.as_bytes())?;
    let payload = json!({
        "kind": "argus_action_receipt",
        "action_id": request.action_id,
        "action": request.action,
        "connection_id": request.connection_id,
        "agent_id": request.agent_id,
        "agent_label": request.agent_label,
        "diagnostics_session_id": request.diagnostics_session_id,
        "window_id": request.window_id,
        "author_id": request.author_id,
        "before_revision": request.before_revision,
        "after_revision": request.after_revision,
        "status": request.status,
        "authority_source": "embedded_surrealdb_kernel_event_ledger",
    });
    let kernel_event = NewKernelEvent::builder(
        format!("argus:{}", request.connection_id),
        format!("argus:{}", request.connection_id),
        KernelEventType::ToolResultRecorded,
        KernelActor::ToolGate(request.agent_id.clone()),
    )
    .aggregate("argus_action", request.action_id.clone())
    .idempotency_key(format!(
        "argus-action-receipt:{}:{}",
        request.diagnostics_session_id, request.action_id
    ))
    .correlation_id(request.action_id.clone())
    .source_component("handshake_native_argus")
    .payload(payload.clone())
    .build()
    .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    let event_ledger_event_id = state
        .palmistry_store
        .append_kernel_event(kernel_event)
        .await
        .map_err(|error| {
            PalmistryLaunchError::unavailable(format!(
                "Argus EventLedger receipt append failed: {error}"
            ))
        })?;
    let mirror_exists = !state
        .recorder
        .list_events(EventFilter {
            event_id: Some(flight_recorder_event_id),
            ..EventFilter::default()
        })
        .await
        .unwrap_or_default()
        .is_empty();
    let mut flight_event = FlightRecorderEvent::new(
        FlightRecorderEventType::Diagnostic,
        FlightRecorderActor::Agent,
        request.diagnostics_session_id,
        json!({
            "diagnostic_id": "argus.action_receipt",
            "type": "argus.action_receipt",
            "event_ledger_event_id": event_ledger_event_id.clone(),
            "receipt": payload,
        }),
    )
    .with_actor_id(request.agent_id);
    flight_event.event_id = flight_recorder_event_id;
    flight_event.trace_id = request.diagnostics_session_id;
    let flight_recorder_mirrored =
        mirror_exists || state.recorder.record_event(flight_event).await.is_ok();
    Ok(Json(ArgusActionDurabilityReceipt {
        event_ledger_event_id,
        flight_recorder_event_id: flight_recorder_mirrored.then_some(flight_recorder_event_id),
        flight_recorder_mirrored,
        durable: true,
    }))
}

fn argus_proof_bytes(request: &ArgusActionReceiptRequest) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        request.diagnostics_session_id.to_string(),
        request.action_id.clone(),
        request.action.clone(),
        request.connection_id.clone(),
        request.agent_id.clone(),
        request.agent_label.clone(),
        request.window_id.clone(),
        request.author_id.clone(),
        request.before_revision.to_string(),
        request
            .after_revision
            .map(|value| value.to_string())
            .unwrap_or_default(),
        request.status.clone(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes
}

fn verify_argus_proof(
    request: &ArgusActionReceiptRequest,
    signing_secret: &[u8],
) -> Result<(), PalmistryLaunchError> {
    let proof = hex::decode(&request.proof)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Argus receipt proof"))?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(signing_secret)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid Argus proof key"))?;
    mac.update(&argus_proof_bytes(request));
    mac.verify_slice(&proof)
        .map_err(|_| PalmistryLaunchError::bad_request("Argus receipt proof mismatch"))
}

fn validate_argus_receipt(request: &ArgusActionReceiptRequest) -> Result<(), PalmistryLaunchError> {
    fn bounded_identifier(value: &str, max: usize) -> bool {
        !value.is_empty()
            && value.len() <= max
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':')
            })
    }
    fn bounded_graphic(value: &str, max: usize) -> bool {
        !value.is_empty() && value.len() <= max && value.bytes().all(|byte| byte.is_ascii_graphic())
    }
    if !bounded_identifier(&request.action_id, 96)
        || !matches!(
            request.action.as_str(),
            "argus.click" | "argus.set_value" | "argus.show_context_menu"
        )
        || !bounded_identifier(&request.connection_id, 96)
        || request.diagnostics_session_id.is_nil()
        || !bounded_identifier(&request.agent_id, 96)
        || !bounded_graphic(&request.agent_label, 96)
        || !bounded_graphic(&request.window_id, 96)
        || !bounded_graphic(&request.author_id, 256)
        || !matches!(request.status.as_str(), "applied" | "failed")
        || (request.status == "applied"
            && !request
                .after_revision
                .is_some_and(|revision| revision > request.before_revision))
        || request
            .after_revision
            .is_some_and(|revision| revision < request.before_revision)
        || request.proof.len() != 64
        || !request.proof.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(PalmistryLaunchError::bad_request(
            "invalid sanitized Argus action receipt",
        ));
    }
    argus_action_uuid(&request.action_id)?;
    Ok(())
}

fn argus_action_uuid(action_id: &str) -> Result<Uuid, PalmistryLaunchError> {
    action_id
        .strip_prefix("argus-action-")
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| PalmistryLaunchError::bad_request("invalid Argus action UUID"))
}

async fn record_model_lane_observation(
    State(state): State<PalmistryLaunchState>,
    Json(request): Json<DiagnosticObservationRequest>,
) -> Result<Json<serde_json::Value>, PalmistryLaunchError> {
    let (launch, receipt, signing_secret, watcher_verifying_key) = {
        let active = state
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(LaunchSlot::Active {
            request: launch,
            receipt,
            signing_secret,
            watcher_verifying_key,
            ..
        }) = active.get(&request.diagnostics_session_id)
        else {
            return Err(PalmistryLaunchError::bad_request(
                "observation is not bound to an active diagnostics session",
            ));
        };
        (
            launch.clone(),
            receipt.clone(),
            signing_secret.clone(),
            *watcher_verifying_key,
        )
    };
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(signing_secret.as_bytes())
        .map_err(|_| PalmistryLaunchError::bad_request("invalid observation proof key"))?;
    for value in [
        request.diagnostics_session_id.to_string(),
        request.behavior_id.clone(),
        request.run_id.clone(),
        request.lane_id.clone(),
        request.heartbeat_counter.to_string(),
    ] {
        mac.update(&(value.len() as u64).to_le_bytes());
        mac.update(value.as_bytes());
    }
    let proof = hex::decode(&request.proof)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid observation proof"))?;
    mac.verify_slice(&proof)
        .map_err(|_| PalmistryLaunchError::bad_request("observation proof mismatch"))?;

    if request.behavior_id != "HBR-INT-009" || request.heartbeat_counter == 0 {
        return Err(PalmistryLaunchError::bad_request(
            "invalid diagnostic behavior observation scope",
        ));
    }
    let now_unix_ms = current_unix_ms();
    let ring = read_ring_snapshot(&launch.ring)?;
    if ring.heartbeat_counter < request.heartbeat_counter
        || !timestamp_is_fresh(now_unix_ms, ring.heartbeat_unix_ms)
    {
        return Err(PalmistryLaunchError::bad_request(
            "native heartbeat has not reached the observation envelope",
        ));
    }
    let observation_path = launch
        .ring
        .parent()
        .ok_or_else(|| PalmistryLaunchError::bad_request("ring has no parent"))?
        .join(format!("observation-{}.json", launch.session_id));
    let observation: PalmistryObservationRecord = serde_json::from_slice(
        &fs::read(observation_path)
            .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?,
    )
    .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    verify_watcher_source_proof(
        &observation,
        &observation.source_proof,
        &watcher_verifying_key,
    )?;
    if observation.schema_id != "hsk.palmistry.observation@1"
        || observation.session_id != launch.session_id
        || observation.launch_nonce != launch.launch_nonce
        || observation.watcher_pid != receipt.os_pid
        || observation.watcher_creation_time_100ns != receipt.os_creation_time_100ns
        || observation.heartbeat_counter < request.heartbeat_counter
        || !timestamp_is_fresh(now_unix_ms, observation.observed_at_unix_ms)
    {
        return Err(PalmistryLaunchError::bad_request(
            "Palmistry readback does not match the native observation envelope",
        ));
    }
    let behavior_observation = observation
        .behavior_observations
        .iter()
        .find(|value| {
            value.mechanism == "model_lane_behavior_observation"
                && value.heartbeat_counter == request.heartbeat_counter
                && timestamp_is_fresh(now_unix_ms, value.observed_at_unix_ms)
        })
        .ok_or_else(|| {
            PalmistryLaunchError::bad_request(
                "Palmistry readback lacks the exact fresh behavior observation",
            )
        })?;
    verify_behavior_correlation(&request, behavior_observation, signing_secret.as_bytes())?;

    let replay = state
        .model_lane_store
        .replay_run(&request.run_id)
        .await
        .map_err(|error| PalmistryLaunchError::bad_request(error.to_string()))?;
    if !replay
        .lanes
        .iter()
        .any(|lane| lane.lane_id == request.lane_id)
    {
        return Err(PalmistryLaunchError::bad_request(
            "lane_id does not belong to the server-resolved run",
        ));
    }
    let requested_envelope = DiagnosticEnvelope {
        behavior_id: request.behavior_id.clone(),
        run_id: request.run_id.clone(),
        lane_id: request.lane_id.clone(),
        heartbeat_counter: request.heartbeat_counter,
        correlation_hmac: behavior_observation.correlation_hmac.clone(),
    };
    {
        let mut active = state
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(LaunchSlot::Active {
            diagnostic_envelopes,
            ..
        }) = active.get_mut(&request.diagnostics_session_id)
        else {
            return Err(PalmistryLaunchError::bad_request(
                "diagnostics session ended before scope registration",
            ));
        };
        if !diagnostic_envelopes.contains(&requested_envelope) {
            if diagnostic_envelopes.len() == 256 {
                diagnostic_envelopes.pop_front();
            }
            diagnostic_envelopes.push_back(requested_envelope);
        }
    }
    let work_packet_id = replay
        .run
        .work_packet_id
        .clone()
        .ok_or_else(|| PalmistryLaunchError::bad_request("run has no work_packet_id"))?;
    let micro_task_id = replay
        .run
        .micro_task_id
        .clone()
        .ok_or_else(|| PalmistryLaunchError::bad_request("run has no micro_task_id"))?;
    let task_board_id = replay
        .run
        .task_board_id
        .clone()
        .ok_or_else(|| PalmistryLaunchError::bad_request("run has no task_board_id"))?;
    for (tier, evidence_ref, producer) in [
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            format!(
                "internal-diagnostics://session/{}/heartbeat/{}",
                launch.session_id, request.heartbeat_counter
            ),
            "native_ring",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            format!(
                "palmistry-observation://session/{}/heartbeat/{}",
                launch.session_id, request.heartbeat_counter
            ),
            "palmistry_watcher",
        ),
    ] {
        let tier_label = tier.as_str();
        state.model_lane_store.record_diagnostic_tier_status(NewModelLaneDiagnosticTierStatus {
            diagnostic_status_id: format!(
                "diag-{}-{}-{tier_label}", request.run_id, request.lane_id
            ),
            behavior_id: request.behavior_id.clone(),
            run_id: request.run_id.clone(),
            tier,
            state: ModelLaneDiagnosticTierState::Wired,
            reason: format!("{producer} produced and read back the correlated observation"),
            evidence_ref,
            follow_up_ref: None,
            event_ledger_stream_id: replay.run.event_ledger_stream_id.clone(),
            work_packet_id: work_packet_id.clone(),
            micro_task_id: micro_task_id.clone(),
            task_board_id: task_board_id.clone(),
            owner_session: replay.run.owner_session.clone(),
            idempotency_key: format!(
                "diagnostic-observation::{}::{}::{tier_label}",
                request.run_id, request.lane_id
            ),
            diagnostic_payload: json!({"producer": producer, "run_id": request.run_id, "lane_id": request.lane_id, "diagnostics_session_id": launch.session_id, "heartbeat_counter": request.heartbeat_counter}),
        }).await.map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
    }
    let posture = state
        .model_lane_store
        .validate_diagnostic_tier_posture(&request.run_id, &request.behavior_id)
        .await
        .map_err(|error| PalmistryLaunchError::unavailable(error.to_string()))?;
    Ok(Json(json!({"recorded": true, "posture": posture})))
}

fn verify_behavior_correlation(
    request: &DiagnosticObservationRequest,
    observation: &BehaviorObservationRecord,
    signing_secret: &[u8],
) -> Result<(), PalmistryLaunchError> {
    let proof = hex::decode(&observation.correlation_hmac).map_err(|_| {
        PalmistryLaunchError::bad_request("invalid behavior-observation correlation proof")
    })?;
    let mut bytes = Vec::new();
    for value in [
        request.behavior_id.clone(),
        request.run_id.clone(),
        request.lane_id.clone(),
        observation.heartbeat_counter.to_string(),
        observation.observed_at_unix_ms.to_string(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(signing_secret)
        .map_err(|_| PalmistryLaunchError::bad_request("invalid correlation proof key"))?;
    mac.update(&bytes);
    mac.verify_slice(&proof)
        .map_err(|_| PalmistryLaunchError::bad_request("behavior correlation proof mismatch"))
}

fn current_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn timestamp_is_fresh(now_unix_ms: u64, observed_at_unix_ms: u64) -> bool {
    observed_at_unix_ms <= now_unix_ms.saturating_add(OBSERVATION_MAX_FUTURE_SKEW_MS)
        && now_unix_ms.saturating_sub(observed_at_unix_ms) <= OBSERVATION_MAX_AGE_MS
}

pub fn routes(state: PalmistryLaunchState) -> Router {
    Router::new()
        .route("/internal-diagnostics/palmistry/start", post(launch))
        .route(
            "/internal-diagnostics/palmistry/recover",
            post(recover_survivor),
        )
        .route(
            "/internal-diagnostics/argus/action-receipt",
            post(record_argus_receipt),
        )
        .route(
            "/internal-diagnostics/model-lane/observe",
            post(record_model_lane_observation),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn valid_argus_receipt(action: &str) -> ArgusActionReceiptRequest {
        ArgusActionReceiptRequest {
            diagnostics_session_id: Uuid::now_v7(),
            action_id: format!("argus-action-{}", Uuid::now_v7()),
            action: action.to_owned(),
            connection_id: "mcp-connection-1".to_owned(),
            agent_id: "agent-a1b2c3d4".to_owned(),
            agent_label: "reviewer-lane-1".to_owned(),
            window_id: "main".to_owned(),
            author_id: "model-runtime.action.refresh".to_owned(),
            before_revision: 4,
            after_revision: Some(5),
            status: "applied".to_owned(),
            proof: "ab".repeat(32),
        }
    }

    #[test]
    fn request_validation_io_context_preserves_stage_kind_and_source_message() {
        let error = PalmistryLaunchError::bad_request_io(
            "ring-read",
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "access denied fixture",
            ),
        );

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "PALMISTRY_LAUNCH_INVALID");
        assert_eq!(
            error.detail,
            "Palmistry request validation stage `ring-read` failed (PermissionDenied): access denied fixture"
        );
    }

    #[test]
    fn observation_freshness_rejects_stale_and_far_future_timestamps() {
        let now = 10_000;
        assert!(timestamp_is_fresh(now, now));
        assert!(timestamp_is_fresh(
            now,
            now + OBSERVATION_MAX_FUTURE_SKEW_MS
        ));
        assert!(!timestamp_is_fresh(
            now,
            now + OBSERVATION_MAX_FUTURE_SKEW_MS + 1
        ));
        assert!(timestamp_is_fresh(now, now - OBSERVATION_MAX_AGE_MS));
        assert!(!timestamp_is_fresh(now, now - OBSERVATION_MAX_AGE_MS - 1));
    }

    #[test]
    fn watcher_ed25519_proof_rejects_tamper_and_wrong_key() {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let mut record = ReadyRecord {
            schema_id: "hsk.palmistry.ready@1".to_owned(),
            session_id: Uuid::now_v7(),
            launch_nonce: Uuid::now_v7(),
            parent_pid: 42,
            watcher_pid: 43,
            watcher_creation_time_100ns: 44,
            source_proof: String::new(),
        };
        let canonical = serde_json::to_vec(&record).expect("canonical ready record");
        record.source_proof = hex::encode(signing_key.sign(&canonical).to_bytes());
        assert!(verify_watcher_source_proof(
            &record,
            &record.source_proof,
            &signing_key.verifying_key().to_bytes()
        )
        .is_ok());

        let mut tampered = record.clone();
        tampered.parent_pid += 1;
        assert!(verify_watcher_source_proof(
            &tampered,
            &tampered.source_proof,
            &signing_key.verifying_key().to_bytes()
        )
        .is_err());
        let wrong_key = SigningKey::from_bytes(&[8_u8; 32]);
        assert!(verify_watcher_source_proof(
            &record,
            &record.source_proof,
            &wrong_key.verifying_key().to_bytes()
        )
        .is_err());
    }

    #[test]
    fn request_validation_rejects_cross_session_paths() {
        let session_id = Uuid::now_v7();
        let root = std::env::temp_dir().join("palmistry-api-validation");
        let request = PalmistryLaunchRequest {
            session_id,
            launch_nonce: Uuid::now_v7(),
            parent_pid: 42,
            ring: root.join(format!("ring-{session_id}.bin")),
            survivor_dir: root.join("survivors"),
            panic_signal: root.join("panic-wrong.signal.json"),
            panic_ack: root.join(format!("panic-{session_id}.ack")),
            shutdown_signal: root.join(format!("shutdown-{session_id}.signal")),
            ready_signal: root.join(format!("ready-{session_id}.json")),
            transport_public_key: "11".repeat(32),
        };
        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn exact_duplicate_identity_includes_nonce_pid_and_every_path() {
        let session_id = Uuid::now_v7();
        let root = PathBuf::from("C:/diagnostics");
        let request = PalmistryLaunchRequest {
            session_id,
            launch_nonce: Uuid::now_v7(),
            parent_pid: 42,
            ring: root.join(format!("ring-{session_id}.bin")),
            survivor_dir: root.join("survivors"),
            panic_signal: root.join(format!("panic-{session_id}.signal.json")),
            panic_ack: root.join(format!("panic-{session_id}.ack")),
            shutdown_signal: root.join(format!("shutdown-{session_id}.signal")),
            ready_signal: root.join(format!("ready-{session_id}.json")),
            transport_public_key: "11".repeat(32),
        };
        assert!(same_launch_identity(&request, &request.clone()));
        let mut rotated_transport = request.clone();
        rotated_transport.transport_public_key = "22".repeat(32);
        assert!(same_launch_identity(&request, &rotated_transport));
        let mut wrong_nonce = request.clone();
        wrong_nonce.launch_nonce = Uuid::now_v7();
        assert!(!same_launch_identity(&request, &wrong_nonce));
        let mut wrong_pid = request.clone();
        wrong_pid.parent_pid += 1;
        assert!(!same_launch_identity(&request, &wrong_pid));
        let mut wrong_path = request.clone();
        wrong_path.ready_signal = root.join("other-ready.json");
        assert!(!same_launch_identity(&request, &wrong_path));
    }

    #[test]
    fn launch_response_serializes_only_authenticated_ciphertext_not_signing_secret() {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let client_secret = [3_u8; 32];
        let request = PalmistryLaunchRequest {
            session_id,
            launch_nonce,
            parent_pid: 42,
            ring: PathBuf::from("C:/diagnostics/ring.bin"),
            survivor_dir: PathBuf::from("C:/diagnostics/survivors"),
            panic_signal: PathBuf::from("C:/diagnostics/panic.signal"),
            panic_ack: PathBuf::from("C:/diagnostics/panic.ack"),
            shutdown_signal: PathBuf::from("C:/diagnostics/shutdown.signal"),
            ready_signal: PathBuf::from("C:/diagnostics/ready.signal"),
            transport_public_key: hex::encode(
                MontgomeryPoint::mul_base_clamped(client_secret).as_bytes(),
            ),
        };
        let secret = SessionSigningSecret(Arc::new(Zeroizing::new([7_u8; 32])));
        let envelope = secret.seal_for_transport(&request).unwrap();
        let serialized = serde_json::to_string(&envelope).unwrap();
        assert!(!serialized.contains(&"07".repeat(32)));
        assert_eq!(envelope.ciphertext.len(), (32 + 16) * 2);
    }

    #[test]
    fn recovery_summary_is_a_strict_mechanical_allowlist() {
        let valid = RecoveredSurvivorSummary {
            record_id: Uuid::now_v7(),
            source_session_id: Uuid::now_v7(),
            kind: "unexpected_exit".to_owned(),
            observed_at_unix_ms: unix_ms_now(),
            parent_pid: 42,
            parent_exit_code: Some(0xC000_0005),
            heartbeat_stale_ms: None,
            os_hung_window_confirmed: false,
            minidump_status: "failed_after_exit".to_owned(),
            imported: false,
        };
        assert!(validate_summary(&valid).is_ok());
        let mut invalid = valid.clone();
        invalid.kind = "project_payload".to_owned();
        assert!(validate_summary(&invalid).is_err());
        assert_ne!(
            recovery_summary_digest(&valid),
            recovery_summary_digest(&invalid),
            "idempotency digest must bind every mechanical summary field"
        );
    }

    #[test]
    fn argus_receipt_is_a_strict_mechanical_allowlist() {
        let valid = valid_argus_receipt("argus.click");
        assert!(validate_argus_receipt(&valid).is_ok());
        let mut graphic_attribution = valid.clone();
        graphic_attribution.agent_id = "reviewer@lane/1".to_owned();
        assert!(validate_argus_receipt(&graphic_attribution).is_err());
        let mut injected = valid.clone();
        injected.author_id = "secret\nvalue".to_owned();
        assert!(validate_argus_receipt(&injected).is_err());
        let mut relabeled = valid.clone();
        relabeled.agent_label = "different-reviewer".to_owned();
        assert_ne!(
            argus_proof_bytes(&valid),
            argus_proof_bytes(&relabeled),
            "agent_label must be inside the authenticated receipt envelope"
        );
        let mut secret_field = serde_json::to_value(valid).expect("serialize receipt");
        secret_field["value"] = json!("canary-secret");
        assert!(serde_json::from_value::<ArgusActionReceiptRequest>(secret_field).is_err());
    }

    #[test]
    fn argus_receipt_accepts_durable_context_menu_action() {
        let context_menu = valid_argus_receipt("argus.show_context_menu");
        assert!(
            validate_argus_receipt(&context_menu).is_ok(),
            "the native canonical durable context-menu action must cross the Palmistry boundary"
        );
    }

    #[test]
    fn argus_receipt_rejects_unallowlisted_context_menu_lookalikes() {
        for action in [
            "argus.context_menu",
            "argus.show-context-menu",
            "argus.show_context_menu\n",
        ] {
            let receipt = valid_argus_receipt(action);
            assert!(
                validate_argus_receipt(&receipt).is_err(),
                "unexpected durable action was accepted: {action:?}"
            );
        }
    }

    #[test]
    fn same_session_freeze_import_never_authorizes_watcher_shutdown() {
        let session_id = Uuid::now_v7();
        assert!(!source_shutdown_is_allowed(session_id, session_id));
        assert!(source_shutdown_is_allowed(Uuid::now_v7(), session_id));
    }

    #[test]
    fn detached_reattach_never_claims_an_attached_reaper() {
        assert!(source_has_attached_reaper(Some(
            WatcherLifecycleOwnership::AttachedReaper
        )));
        assert!(!source_has_attached_reaper(Some(
            WatcherLifecycleOwnership::DetachedReattached
        )));
        assert!(!source_has_attached_reaper(None));
    }

    #[test]
    fn detached_source_allows_guarded_reclaim_when_shutdown_signal_is_unavailable() {
        assert!(!source_allows_guarded_reclaim(Some(
            WatcherLifecycleOwnership::AttachedReaper
        )));
        assert!(source_allows_guarded_reclaim(Some(
            WatcherLifecycleOwnership::DetachedReattached
        )));
        assert!(source_allows_guarded_reclaim(None));
    }

    #[test]
    fn matching_active_slot_removal_requires_nonce_and_process_uuid() {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let process_uuid = Uuid::now_v7();
        let root = PathBuf::from("C:/diagnostics");
        let request = PalmistryLaunchRequest {
            session_id,
            launch_nonce,
            parent_pid: 42,
            ring: root.join(format!("ring-{session_id}.bin")),
            survivor_dir: root.join("survivors"),
            panic_signal: root.join(format!("panic-{session_id}.signal.json")),
            panic_ack: root.join(format!("panic-{session_id}.ack")),
            shutdown_signal: root.join(format!("shutdown-{session_id}.signal")),
            ready_signal: root.join(format!("ready-{session_id}.json")),
            transport_public_key: "11".repeat(32),
        };
        let receipt = PalmistryLaunchReceipt {
            session_id,
            process_uuid,
            os_pid: 43,
            sandbox_adapter_id: "palmistry",
            ledger_start_durable: true,
            os_creation_time_100ns: 44,
        };
        let active = Arc::new(Mutex::new(HashMap::from([(
            session_id,
            LaunchSlot::Active {
                request,
                receipt,
                signing_secret: SessionSigningSecret(Arc::new(Zeroizing::new([7_u8; 32]))),
                watcher_verifying_key: [8_u8; 32],
                diagnostic_envelopes: std::collections::VecDeque::new(),
                lifecycle_ownership: WatcherLifecycleOwnership::DetachedReattached,
            },
        )])));

        assert!(!remove_matching_active_slot(
            &active,
            session_id,
            Uuid::now_v7(),
            process_uuid
        ));
        assert!(!remove_matching_active_slot(
            &active,
            session_id,
            launch_nonce,
            Uuid::now_v7()
        ));
        assert!(remove_matching_active_slot(
            &active,
            session_id,
            launch_nonce,
            process_uuid
        ));
        assert!(!active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains_key(&session_id));
    }

    #[tokio::test]
    async fn cancelled_request_waiter_does_not_cancel_owned_launch_slot_cleanup() {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let root = PathBuf::from("C:/diagnostics");
        let request = PalmistryLaunchRequest {
            session_id,
            launch_nonce,
            parent_pid: 42,
            ring: root.join(format!("ring-{session_id}.bin")),
            survivor_dir: root.join("survivors"),
            panic_signal: root.join(format!("panic-{session_id}.signal.json")),
            panic_ack: root.join(format!("panic-{session_id}.ack")),
            shutdown_signal: root.join(format!("shutdown-{session_id}.signal")),
            ready_signal: root.join(format!("ready-{session_id}.json")),
            transport_public_key: "11".repeat(32),
        };
        let active = Arc::new(Mutex::new(HashMap::from([(
            session_id,
            LaunchSlot::Launching(request.clone()),
        )])));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finish_tx, finish_rx) = tokio::sync::oneshot::channel::<()>();
        let transaction_active = Arc::clone(&active);
        let transaction_request = request.clone();
        let transaction = tokio::spawn(async move {
            let _slot_guard = LaunchingSlotGuard::new(transaction_active, transaction_request);
            let _ = started_tx.send(());
            let _ = finish_rx.await;
        });
        let waiter = tokio::spawn(async move {
            let _ = transaction.await;
        });
        started_rx.await.expect("owned transaction started");
        waiter.abort();
        let _ = waiter.await;
        finish_tx
            .send(())
            .expect("detached launch transaction remains alive");
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if !active
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .contains_key(&session_id)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("launching slot cleanup completed after waiter cancellation");
    }

    #[test]
    fn reaper_exit_removes_dead_cached_active_even_when_stop_is_not_durable() {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let process_uuid = Uuid::now_v7();
        let request = PalmistryLaunchRequest {
            session_id,
            launch_nonce,
            parent_pid: 42,
            ring: PathBuf::from("C:/diagnostics/ring.bin"),
            survivor_dir: PathBuf::from("C:/diagnostics/survivors"),
            panic_signal: PathBuf::from("C:/diagnostics/panic.signal"),
            panic_ack: PathBuf::from("C:/diagnostics/panic.ack"),
            shutdown_signal: PathBuf::from("C:/diagnostics/shutdown.signal"),
            ready_signal: PathBuf::from("C:/diagnostics/ready.signal"),
            transport_public_key: "11".repeat(32),
        };
        let active = Arc::new(Mutex::new(HashMap::from([(
            session_id,
            LaunchSlot::Active {
                request,
                receipt: PalmistryLaunchReceipt {
                    session_id,
                    process_uuid,
                    os_pid: 43,
                    sandbox_adapter_id: PALMISTRY_WATCHER_ADAPTER_ID,
                    ledger_start_durable: true,
                    os_creation_time_100ns: 44,
                },
                signing_secret: SessionSigningSecret(Arc::new(Zeroizing::new([7_u8; 32]))),
                watcher_verifying_key: [8_u8; 32],
                diagnostic_envelopes: std::collections::VecDeque::new(),
                lifecycle_ownership: WatcherLifecycleOwnership::AttachedReaper,
            },
        )])));

        // The durable STOP result gates verifier retirement, not in-memory
        // liveness. A joined child must disappear from the exact Active cache.
        assert!(remove_matching_active_slot(
            &active,
            session_id,
            launch_nonce,
            process_uuid
        ));
        assert!(!active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains_key(&session_id));
    }

    #[test]
    fn survivor_timestamp_rejects_stale_and_future_signed_records() {
        let now = 2 * SURVIVOR_MAX_AGE_MS;
        assert!(survivor_timestamp_is_fresh(now, now));
        assert!(!survivor_timestamp_is_fresh(
            now,
            now - SURVIVOR_MAX_AGE_MS - 1
        ));
        assert!(!survivor_timestamp_is_fresh(
            now,
            now + SURVIVOR_MAX_FUTURE_SKEW_MS + 1
        ));
    }

    #[test]
    fn verifier_migration_has_real_up_down_up_shape() {
        let up = include_str!("../../migrations/0360_palmistry_durable_verifier.sql");
        let down = include_str!("../../migrations/0360_palmistry_durable_verifier.down.sql");
        assert!(up.contains("CREATE TABLE IF NOT EXISTS palmistry_durable_verifier"));
        assert!(up.contains("CREATE UNIQUE INDEX IF NOT EXISTS"));
        assert!(down.contains("DROP INDEX IF EXISTS"));
        assert!(down.contains("DROP TABLE IF EXISTS palmistry_durable_verifier"));
    }

    #[test]
    fn reaper_contract_has_no_tokio_blocking_wait_and_wait_errors_stay_open() {
        let source = include_str!("palmistry.rs");
        let forbidden = ["spawn", "blocking"].join("_");
        assert!(!source.contains(&forbidden));
        assert!(source.contains("let Ok(status) = result else"));
        assert!(source.contains("lifecycle.leave_open_for_reconciliation();"));
        assert!(source.contains("PalmistryLaunchAttempt::new(spawned)"));
        assert!(source.contains("remove_matching_active_slot("));
    }
}
