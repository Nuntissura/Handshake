use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use crate::storage::surreal::SurrealStorage;

use super::{
    LedgerEventKind, ProcessEngineKind, ProcessLedgerError, ProcessLedgerWriter,
    ProcessRuntimeOwner, ProcessStop, ReservedProcessStop, PROCESS_LEDGER_TABLE_NAME,
};

pub const EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID: &str = "hsk.embedded_runtime.instance@2";
pub const EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL: &str =
    "hsk.embedded_runtime.loopback_udp_exclusive@1";
pub const HANDSHAKE_HOST_SCOPE_ID_ENV: &str = "HANDSHAKE_HOST_SCOPE_ID";
/// Non-optional process-liveness evidence carried by every pid-less embedded
/// runtime START row.
///
/// The UDP socket is deliberately loopback-only and never carries traffic. Its
/// exclusive OS bind is the lease: process crash releases the port, while a
/// durable-store restart cannot. `host_scope_id` prevents a reclaimer connected
/// to shared durable storage from treating its own loopback namespace as another
/// host's namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedRuntimeInstanceDescriptor {
    #[serde(rename = "runtime_instance_id")]
    pub instance_id: Uuid,
    #[serde(rename = "runtime_host_scope_id")]
    pub host_scope_id: String,
    #[serde(rename = "runtime_lease_protocol")]
    pub lease_protocol: String,
    #[serde(rename = "runtime_lease_address")]
    pub loopback_address: IpAddr,
    #[serde(rename = "runtime_lease_port")]
    pub loopback_port: u16,
}

impl EmbeddedRuntimeInstanceDescriptor {
    pub fn metadata_fields(&self) -> Value {
        serde_json::json!({
            "runtime_instance_schema_id": EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
            "runtime_instance_id": self.instance_id.to_string(),
            "runtime_host_scope_id": self.host_scope_id.clone(),
            "runtime_lease_protocol": self.lease_protocol.clone(),
            "runtime_lease_address": self.loopback_address.to_string(),
            "runtime_lease_port": self.loopback_port,
        })
    }

    pub fn process_runtime_owner(&self) -> super::ProcessRuntimeOwner {
        super::ProcessRuntimeOwner {
            runtime_instance_id: self.instance_id,
            host_scope_id: self.host_scope_id.clone(),
            lease_schema_id: EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID.to_string(),
            lease_protocol: self.lease_protocol.clone(),
            lease_address: self.loopback_address.to_string(),
            lease_port: self.loopback_port,
        }
    }
}

/// An OS-owned liveness lease for one Handshake backend instance.
///
/// Keeping `_socket` alive keeps the exact loopback endpoint exclusively bound.
/// No database connection is held, so database restart/session loss cannot be
/// confused with process death and the application pool loses no capacity.
pub struct EmbeddedRuntimeInstanceLease {
    descriptor: EmbeddedRuntimeInstanceDescriptor,
    _socket: UdpSocket,
}

impl EmbeddedRuntimeInstanceLease {
    pub fn instance_id(&self) -> Uuid {
        self.descriptor.instance_id
    }

    pub fn descriptor(&self) -> &EmbeddedRuntimeInstanceDescriptor {
        &self.descriptor
    }

    pub async fn release(self) -> Result<(), ProcessLedgerError> {
        drop(self);
        Ok(())
    }
}

/// Resolve the explicit host identity used to keep loopback liveness evidence
/// local to one machine. Database endpoint identity is never used as a host
/// identity in the embedded-Surreal runtime.
pub fn resolve_embedded_runtime_host_scope() -> Result<String, ProcessLedgerError> {
    let explicit = std::env::var(HANDSHAKE_HOST_SCOPE_ID_ENV).ok();
    resolve_embedded_runtime_host_scope_with_override(explicit.as_deref())
}

pub fn resolve_embedded_runtime_host_scope_with_override(
    explicit_host_scope: Option<&str>,
) -> Result<String, ProcessLedgerError> {
    let host_scope = explicit_host_scope
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(format!(
                "embedded Surreal runtime requires explicit {HANDSHAKE_HOST_SCOPE_ID_ENV}"
            ))
        })?;
    if host_scope.len() > 256 {
        return Err(ProcessLedgerError::InvalidConfig(format!(
            "{HANDSHAKE_HOST_SCOPE_ID_ENV} exceeds 256 bytes"
        )));
    }
    Ok(host_scope.to_owned())
}

/// Acquire the process-lifetime OS lease before any model artifact is opened.
/// This is synchronous so callers cannot accidentally spawn work between lease
/// selection and ownership. The second-bind self-test fails closed on a platform
/// whose ordinary UDP bind semantics do not provide exclusivity.
pub fn acquire_embedded_runtime_instance_lease(
    instance_id: Uuid,
    host_scope_id: impl Into<String>,
) -> Result<EmbeddedRuntimeInstanceLease, ProcessLedgerError> {
    let host_scope_id = host_scope_id.into();
    if host_scope_id.trim().is_empty() {
        return Err(ProcessLedgerError::InvalidConfig(
            "embedded runtime host_scope_id must not be empty".to_string(),
        ));
    }
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|error| {
        ProcessLedgerError::Store(format!(
            "failed to bind embedded runtime loopback UDP lease: {error}"
        ))
    })?;
    let address = socket.local_addr().map_err(|error| {
        ProcessLedgerError::Store(format!(
            "failed to inspect embedded runtime loopback UDP lease: {error}"
        ))
    })?;
    verify_second_udp_bind_is_rejected(address)?;
    Ok(EmbeddedRuntimeInstanceLease {
        descriptor: EmbeddedRuntimeInstanceDescriptor {
            instance_id,
            host_scope_id,
            lease_protocol: EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL.to_string(),
            loopback_address: address.ip(),
            loopback_port: address.port(),
        },
        _socket: socket,
    })
}

fn verify_second_udp_bind_is_rejected(address: SocketAddr) -> Result<(), ProcessLedgerError> {
    match UdpSocket::bind(address) {
        Ok(second) => {
            drop(second);
            Err(ProcessLedgerError::Store(format!(
                "loopback UDP lease endpoint {address} accepted a second ordinary bind"
            )))
        }
        // The platform-specific error kind differs (notably AddrInUse vs
        // PermissionDenied on Windows). The invariant is simply that the exact
        // second ordinary bind was rejected after the first succeeded.
        Err(_) => Ok(()),
    }
}

#[derive(Debug)]
enum UdpLeaseClaim {
    Claimed(UdpSocket),
    Protected,
    Ambiguous(io::Error),
}

fn try_claim_udp_lease(descriptor: &EmbeddedRuntimeInstanceDescriptor) -> UdpLeaseClaim {
    let address = SocketAddr::new(descriptor.loopback_address, descriptor.loopback_port);
    match UdpSocket::bind(address) {
        Ok(socket) => match UdpSocket::bind(address) {
            Ok(second) => {
                drop(second);
                drop(socket);
                UdpLeaseClaim::Ambiguous(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "claimed UDP endpoint did not enforce exclusive second-bind rejection",
                ))
            }
            // As in acquisition, the platform-specific rejection kind is not
            // stable. The first exact bind succeeded and the second exact bind
            // failed, which is the OS-ownership invariant we need.
            Err(_) => UdpLeaseClaim::Claimed(socket),
        },
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => UdpLeaseClaim::Protected,
        Err(error) => UdpLeaseClaim::Ambiguous(error),
    }
}

pub(crate) fn runtime_owner_loopback_lease_is_free(owner: &ProcessRuntimeOwner) -> bool {
    let Some(loopback_address) = owner
        .lease_address
        .parse::<IpAddr>()
        .ok()
        .filter(IpAddr::is_loopback)
    else {
        return false;
    };
    let descriptor = EmbeddedRuntimeInstanceDescriptor {
        instance_id: owner.runtime_instance_id,
        host_scope_id: owner.host_scope_id.clone(),
        lease_protocol: owner.lease_protocol.clone(),
        loopback_address,
        loopback_port: owner.lease_port,
    };
    matches!(try_claim_udp_lease(&descriptor), UdpLeaseClaim::Claimed(_))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimTrigger {
    Close,
    Failure,
    Restart,
    Stale,
    OperatorCancel,
}

const RECLAIM_CLAIM_TTL: Duration = Duration::from_secs(30);
const RECLAIM_CLAIM_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const RECLAIM_KILL_TIMEOUT: Duration = Duration::from_secs(30);
const RECLAIM_STOP_ACK_TIMEOUT: Duration = Duration::from_secs(5);
/// Upper bound on crash-left `reclaim_kill_in_progress` rows one restart or
/// stale-owner recovery sweep examines per session per pass. The Surreal
/// provider rejects any limit outside `1..=64`, so this is also the ceiling.
/// Exported so the UserManual currency proof asserts the documented bound
/// against the compiled value instead of a prose literal (HBR-MAN-003).
pub const RECLAIM_IN_PROGRESS_RECOVERY_LIMIT: usize = 64;

#[derive(Default)]
struct ProcessKillFence {
    result: std::sync::Mutex<Option<Result<(), KillError>>>,
    completed: tokio::sync::Notify,
}

static PROCESS_KILL_FENCES: OnceLock<std::sync::Mutex<HashMap<Uuid, Arc<ProcessKillFence>>>> =
    OnceLock::new();

fn acquire_process_kill_fence(process_uuid: Uuid) -> (Arc<ProcessKillFence>, bool) {
    let fences = PROCESS_KILL_FENCES.get_or_init(Default::default);
    let mut fences = match fences.lock() {
        Ok(fences) => fences,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(existing) = fences.get(&process_uuid) {
        return (Arc::clone(existing), false);
    }
    let fence = Arc::new(ProcessKillFence::default());
    fences.insert(process_uuid, Arc::clone(&fence));
    (fence, true)
}

fn clear_process_kill_fence(process_uuid: Uuid, completed: &Arc<ProcessKillFence>) {
    let Some(fences) = PROCESS_KILL_FENCES.get() else {
        return;
    };
    let mut fences = match fences.lock() {
        Ok(fences) => fences,
        Err(poisoned) => poisoned.into_inner(),
    };
    if fences
        .get(&process_uuid)
        .is_some_and(|current| Arc::ptr_eq(current, completed))
    {
        fences.remove(&process_uuid);
    }
}

fn clear_completed_process_kill_fence(process_uuid: Uuid) {
    let Some(fences) = PROCESS_KILL_FENCES.get() else {
        return;
    };
    let completed = {
        let fences = match fences.lock() {
            Ok(fences) => fences,
            Err(poisoned) => poisoned.into_inner(),
        };
        fences.get(&process_uuid).cloned()
    };
    let Some(completed) = completed else {
        return;
    };
    let is_complete = match completed.result.lock() {
        Ok(result) => result.is_some(),
        Err(poisoned) => poisoned.into_inner().is_some(),
    };
    if is_complete {
        clear_process_kill_fence(process_uuid, &completed);
    }
}

/// Fenced ownership of one open lifecycle row. Both fields participate in
/// release, renewal, pending-stop, and final STOP transitions; a stale
/// claimant can therefore neither erase nor finalize a newer claim.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReclaimResourceScope {
    pub account_uuid: Uuid,
    pub actor_uuid: Uuid,
    pub session_uuid: Uuid,
    pub workspace_id: String,
    pub access_space_uuid: Uuid,
}

impl ReclaimResourceScope {
    pub fn try_from_stored(
        owner_account_id: &str,
        actor_principal_id: &str,
        authenticated_session_id: &str,
        workspace_id: &str,
        access_space_id: &str,
    ) -> Result<Self, ProcessLedgerError> {
        if workspace_id.trim().is_empty() {
            return Err(ProcessLedgerError::Store(
                "reclaim ResourceScope workspace_id is missing".to_owned(),
            ));
        }
        let parse = |name: &str, value: &str| {
            Uuid::parse_str(value).map_err(|_| {
                ProcessLedgerError::Store(format!(
                    "reclaim ResourceScope {name} is missing or invalid"
                ))
            })
        };
        Ok(Self {
            account_uuid: parse("owner_account_id", owner_account_id)?,
            actor_uuid: parse("actor_principal_id", actor_principal_id)?,
            session_uuid: parse("authenticated_session_id", authenticated_session_id)?,
            workspace_id: workspace_id.to_owned(),
            access_space_uuid: parse("access_space_id", access_space_id)?,
        })
    }

    /// Builds the reclaim scope from an exact five-field ResourceScope
    /// attribution. Reclaim never widens beyond the exact scope of the
    /// authority that owns the process rows.
    pub fn from_exact(
        exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    ) -> Result<Self, ProcessLedgerError> {
        Self::try_from_stored(
            &exact.owner_account_id.to_string(),
            &exact.actor_principal_id.to_string(),
            &exact.authenticated_session_id.to_string(),
            exact.workspace_id.as_str(),
            &exact.access_space_id.to_string(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimClaim {
    pub resource_scope: ReclaimResourceScope,
    pub claimant_uuid: Uuid,
    pub kill_operation_uuid: Uuid,
    pub generation: u64,
    pub claimed_at_unix_ms: i64,
    pub lease_expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimKillOperationStatus {
    NotStarted,
    InProgress,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimKillOperation {
    pub resource_scope: ReclaimResourceScope,
    pub process_uuid: Uuid,
    pub kill_operation_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "candidate", rename_all = "snake_case")]
pub enum ReclaimKillOperationCandidate {
    Operation {
        operation: ReclaimKillOperation,
    },
    Malformed {
        process_identity: String,
        kill_operation_identity: Option<String>,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimKillOperationSweep {
    pub operations: Vec<ReclaimKillOperationSweepEntry>,
    pub reclaim_report: Option<ReclaimReport>,
    pub reclaim_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimKillOperationSweepEntry {
    pub candidate: ReclaimKillOperationCandidate,
    pub outcome: ReclaimKillOperationSweepOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ReclaimKillOperationSweepOutcome {
    StateAdvanced {
        status: ReclaimKillOperationStatus,
    },
    StateOpen {
        status: ReclaimKillOperationStatus,
    },
    StatusQueryFailed {
        error: String,
    },
    StateTransitionFailed {
        status: ReclaimKillOperationStatus,
        error: String,
    },
    MalformedRecoveryRow {
        error: String,
    },
}

impl ReclaimKillOperationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

impl ReclaimTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Failure => "failure",
            Self::Restart => "restart",
            Self::Stale => "stale",
            Self::OperatorCancel => "operator_cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimableProcess {
    pub resource_scope: ReclaimResourceScope,
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    /// Nullable in the authority table (migration 0021) and genuinely absent for
    /// adapter-owned official-CLI probe children, so the reclaim view must not
    /// pretend every reclaimable row belongs to a coordinator session.
    pub parent_session_id: Option<String>,
    pub parent_process_id: Option<Uuid>,
    pub sandbox_adapter_id: Option<String>,
    pub sandbox_internal_id: Option<String>,
    pub engine_kind: ProcessEngineKind,
    pub started_at: DateTime<Utc>,
    pub model_artifact_sha256: Option<String>,
    pub work_profile_id: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub runtime_owner: Option<ProcessRuntimeOwner>,
    pub sandbox_capabilities_snapshot: serde_json::Value,
    pub metadata_jsonb: serde_json::Value,
    pub reclaim_claim: ReclaimClaim,
    /// A prior claimant already proved the kill. Recovery must persist STOP
    /// from the durable row and must never invoke the sandbox kill again.
    pub kill_succeeded_pending_stop: bool,
}

impl ReclaimableProcess {
    fn sync_reclaim_claim_metadata(&mut self) -> Result<(), ProcessLedgerError> {
        let claim = serde_json::to_value(&self.reclaim_claim).map_err(|error| {
            ProcessLedgerError::Store(format!(
                "failed to serialize reclaim claim for process_uuid {}: {error}",
                self.process_uuid
            ))
        })?;
        if let Some(metadata) = self.metadata_jsonb.as_object_mut() {
            metadata.insert("reclaim_claim".to_string(), claim);
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "reclaim metadata for process_uuid {} is not a JSON object",
                self.process_uuid
            )))
        }
    }

    pub fn reclaim_stop(&self, exit_code: i32) -> ProcessStop {
        let mut metadata_jsonb = self.metadata_jsonb.clone();
        if let Some(metadata) = metadata_jsonb.as_object_mut() {
            metadata.insert(
                "reclaim_pending_stop".to_string(),
                serde_json::json!({
                    "exit_code": exit_code,
                    "stop_reason": "reclaim",
                    "claimant_uuid": self.reclaim_claim.claimant_uuid,
                    "kill_operation_uuid": self.reclaim_claim.kill_operation_uuid,
                    "generation": self.reclaim_claim.generation,
                }),
            );
            metadata.insert(
                "reclaim_last_kill_operation".to_string(),
                serde_json::json!({
                    "kill_operation_uuid": self.reclaim_claim.kill_operation_uuid,
                    "status": "succeeded",
                }),
            );
        }
        ProcessStop {
            process_uuid: self.process_uuid,
            os_pid: self.os_pid,
            parent_session_id: self.parent_session_id.clone(),
            parent_process_id: self.parent_process_id,
            sandbox_adapter_id: self.sandbox_adapter_id.clone(),
            sandbox_internal_id: self.sandbox_internal_id.clone(),
            engine_kind: self.engine_kind,
            started_at: self.started_at,
            stopped_at: Utc::now(),
            exit_code: Some(exit_code),
            stop_reason: Some("reclaim".to_string()),
            model_artifact_sha256: self.model_artifact_sha256.clone(),
            work_profile_id: self.work_profile_id.clone(),
            owner_role: self.owner_role.clone(),
            owner_wp: self.owner_wp.clone(),
            role_id: self.role_id.clone(),
            wp_id: self.wp_id.clone(),
            mt_id: self.mt_id.clone(),
            runtime_owner: self.runtime_owner.clone(),
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot.clone(),
            metadata_jsonb,
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct KillError {
    message: String,
}

impl KillError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum KillOutcome {
    Killed,
    /// The kill completed, but store acknowledgement did not arrive within the
    /// bounded wait. The queued writer row remains retained for retry and the
    /// fenced durable row remains recoverable without another kill.
    KilledPendingStop {
        error: String,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedProcess {
    pub process_uuid: Uuid,
    pub engine_kind: ProcessEngineKind,
    pub sandbox_adapter_id: Option<String>,
    pub kill_result: KillOutcome,
    pub stop_event_kind: Option<LedgerEventKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimReport {
    pub session_id: String,
    pub trigger: ReclaimTrigger,
    pub processes_reclaimed: Vec<ReclaimedProcess>,
    pub total_duration_ms: u128,
}

#[async_trait]
pub trait ReclaimProcessStore: Send + Sync + 'static {
    async fn active_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError>;

    /// Claim one exact process without allowing a lane-level teardown fallback
    /// to kill healthy sibling lanes that share the same coordinator session.
    /// Stores may override this with a single-row query; the conservative
    /// default atomically claims the session set and immediately releases every
    /// non-target claim before returning the requested row.
    async fn active_process_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        process_uuid: Uuid,
    ) -> Result<Option<ReclaimableProcess>, ProcessLedgerError> {
        let claimed = self
            .active_processes_for_session(resource_scope, session_id)
            .await?;
        let mut target = None;
        let mut release_error = None;
        for process in claimed {
            if process.process_uuid == process_uuid {
                target = Some(process);
            } else if !process.kill_succeeded_pending_stop {
                if let Err(error) = self
                    .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
                    .await
                {
                    release_error.get_or_insert(error);
                }
            }
        }
        if let Some(error) = release_error {
            if let Some(target) = target.as_ref() {
                if !target.kill_succeeded_pending_stop {
                    let _ = self
                        .release_reclaim_claim(target.process_uuid, &target.reclaim_claim)
                        .await;
                }
            }
            return Err(error);
        }
        Ok(target)
    }

    /// MT-019 P-2 + HBR-QUIET-003: claim exactly one row by `process_uuid`,
    /// gated on an explicit `owner_runtime_instance_id`.
    ///
    /// This is the only claim path a RUNNING instance may use to reap its own
    /// mid-run orphan, because the row class it targets (an adapter-owned
    /// official-CLI probe child) carries no `parent_session_id` and is therefore
    /// invisible to every session-keyed claim. The owner predicate must be
    /// enforced inside the claim statement, not by the caller.
    ///
    /// There is deliberately NO delegating default: a store that cannot express
    /// the owner predicate must fail closed rather than silently widen the claim
    /// to another instance's processes.
    async fn active_owned_process(
        &self,
        _resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        owner_runtime_instance_id: Uuid,
    ) -> Result<Option<ReclaimableProcess>, ProcessLedgerError> {
        Err(ProcessLedgerError::InvalidConfig(format!(
            "reclaim store does not implement the owner-scoped single-process claim required to \
             reap process {process_uuid} owned by runtime instance {owner_runtime_instance_id}"
        )))
    }

    /// MT-019 P-4(c): claim a session's open rows while structurally excluding
    /// every row owned by `excluded_owner_runtime_instance_id` (the caller).
    ///
    /// Restart reclaim is the one trigger that intentionally acts on ANOTHER
    /// instance's rows, so it is also the one trigger that must never act on its
    /// own. There is deliberately no delegating default for the same reason as
    /// [`Self::active_owned_process`].
    async fn active_foreign_owner_processes_for_session(
        &self,
        _resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        _authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Err(ProcessLedgerError::InvalidConfig(format!(
            "reclaim store does not implement the foreign-owner session claim required to reap \
             restart orphans of session {session_id} while excluding runtime instance \
             {excluded_owner_runtime_instance_id}"
        )))
    }

    /// Claim only the rows whose exact runtime+host ownership was evaluated by
    /// the stale-session source. A session id is not an ownership boundary.
    async fn active_stale_owned_processes_for_session(
        &self,
        _resource_scope: &ReclaimResourceScope,
        _session_id: &str,
        _owner_runtime_instance_id: Uuid,
        _owner_host_scope_id: &str,
        _authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Err(ProcessLedgerError::InvalidConfig(
            "STALE_RECLAIM_STORE_OWNER_SCOPE_UNSUPPORTED".to_string(),
        ))
    }

    async fn renew_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError>;

    async fn mark_reclaim_kill_succeeded(
        &self,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn resolve_reclaim_kill_operation(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError>;

    async fn in_progress_kill_operations_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError>;

    async fn in_progress_kill_operations_for_stale_owner(
        &self,
        _resource_scope: &ReclaimResourceScope,
        _session_id: &str,
        _owner_runtime_instance_id: Uuid,
        _owner_host_scope_id: &str,
        _authorized_process_uuids: &[Uuid],
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Err(ProcessLedgerError::InvalidConfig(
            "STALE_RECLAIM_RECOVERY_OWNER_SCOPE_UNSUPPORTED".to_string(),
        ))
    }
}

#[async_trait]
pub trait SandboxKill: Send + Sync + 'static {
    /// Execute or coalesce one stable kill operation. Implementations must use
    /// `kill_operation_uuid` as their idempotency key across retries.
    async fn kill(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<(), KillError>;

    /// Query the adapter's authoritative idempotency record for crash recovery.
    async fn kill_operation_status(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError>;
}

/// Production process killer for ledger rows whose owning adapter exposes an
/// OS process identity. The ledger's immutable executable hash and launch
/// process-generation identity are checked before termination, so a reused PID
/// cannot redirect stale reclaim to another process generation.
#[derive(Clone)]
pub struct ProductionSandboxKill {
    storage: SurrealStorage,
    sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
}

impl ProductionSandboxKill {
    pub fn new(storage: SurrealStorage, _runtime: tokio::runtime::Handle) -> Self {
        let adapter_id =
            crate::sandbox::AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        let mut registry = crate::sandbox::SandboxAdapterRegistry::new(adapter_id);
        registry.register(Arc::new(
            crate::sandbox::HandshakeNativeSandboxAdapter::new(),
        ));
        Self::with_registry(storage, Arc::new(registry))
    }

    pub fn with_registry(
        storage: SurrealStorage,
        sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    ) -> Self {
        Self {
            storage,
            sandbox_registry,
        }
    }

    async fn identity(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
    ) -> Result<ProductionKillIdentity, KillError> {
        tokio::time::timeout(
            RECLAIM_KILL_TIMEOUT,
            load_production_kill_identity(&self.storage, resource_scope, process_uuid),
        )
        .await
        .map_err(|_| {
            KillError::new(format!(
                "production reclaim identity lookup timed out for process {process_uuid}"
            ))
        })?
        .map_err(|error| KillError::new(error.to_string()))
    }

    fn owning_adapter(
        &self,
        identity: &ProductionKillIdentity,
    ) -> Result<Arc<dyn crate::sandbox::SandboxAdapter>, KillError> {
        self.sandbox_registry
            .get(&identity.detached.handle.adapter_id)
            .ok_or_else(|| {
                KillError::new(format!(
                    "process {} owning sandbox adapter {} is not registered",
                    identity.detached.process_uuid, identity.detached.handle.adapter_id
                ))
            })
    }
}

#[derive(Debug, Clone)]
struct ProductionKillIdentity {
    stopped: bool,
    detached: crate::sandbox::DetachedProcessIdentity,
    kill_operation_uuid: Option<Uuid>,
}

#[derive(Debug, SurrealValue)]
struct ProductionKillIdentityBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ProductionKillIdentityRow {
    process_uuid: Uuid,
    os_pid: Option<i64>,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    metadata: Value,
}

const LOAD_PRODUCTION_KILL_IDENTITY: &str = r#"
SELECT process_uuid, os_pid, started_at, stopped_at, sandbox_adapter_id,
    sandbox_internal_id, metadata
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

async fn load_production_kill_identity(
    storage: &SurrealStorage,
    resource_scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) -> Result<ProductionKillIdentity, ProcessLedgerError> {
    let bindings = ProductionKillIdentityBindings {
        record: RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
        owner_account_id: resource_scope.account_uuid.to_string(),
        actor_principal_id: resource_scope.actor_uuid.to_string(),
        authenticated_session_id: resource_scope.session_uuid.to_string(),
        access_space_id: resource_scope.access_space_uuid.to_string(),
        workspace_id: resource_scope.workspace_id.clone(),
    };
    let row = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_first::<ProductionKillIdentityRow, _>(
                        LOAD_PRODUCTION_KILL_IDENTITY,
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| ProcessLedgerError::Store(error.to_string()))?
        .ok_or_else(|| {
            ProcessLedgerError::Store(format!(
                "production reclaim identity missing in the exact ResourceScope for process {process_uuid}"
            ))
        })?;
    if row.process_uuid != process_uuid {
        return Err(ProcessLedgerError::Store(format!(
            "production reclaim identity returned the wrong process for {process_uuid}"
        )));
    }
    let metadata = row.metadata;
    let executable_sha256 = metadata
        .get("effective_executable_sha256")
        .or_else(|| metadata.get("executable_sha256"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let os_creation_time_100ns = metadata
        .get("os_creation_time_100ns")
        .and_then(Value::as_u64);
    let kill_operation_uuid = metadata
        .get("reclaim_last_kill_operation")
        .and_then(|value| value.get("kill_operation_uuid"))
        .and_then(Value::as_str)
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "invalid production reclaim operation identity: {error}"
            ))
        })?;
    let os_pid = row.os_pid.map(stored_pid_to_u32).transpose()?;
    let adapter_id = row
        .sandbox_adapter_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ProcessLedgerError::Store(format!(
                "production reclaim identity for process {process_uuid} has no owning sandbox adapter"
            ))
        })?;
    let handle_id = metadata
        .get("sandbox_handle_id")
        .and_then(Value::as_str)
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "invalid production sandbox handle identity for process {process_uuid}: {error}"
            ))
        })?
        .unwrap_or(process_uuid);
    let sandbox_internal_id = row
        .sandbox_internal_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| process_uuid.to_string());
    Ok(ProductionKillIdentity {
        stopped: row.stopped_at.is_some(),
        detached: crate::sandbox::DetachedProcessIdentity {
            process_uuid,
            handle: crate::sandbox::ProcessHandle {
                id: handle_id,
                adapter_id: crate::sandbox::AdapterId::new(adapter_id),
                pid: os_pid,
                sandbox_internal_id,
                spawned_at_utc: row.started_at,
            },
            executable_sha256,
            os_creation_time_100ns,
        },
        kill_operation_uuid,
    })
}

#[async_trait]
impl SandboxKill for ProductionSandboxKill {
    async fn kill(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<(), KillError> {
        let identity = self.identity(resource_scope, process_uuid).await?;
        if identity.stopped {
            return Ok(());
        }
        if identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Err(KillError::new(format!(
                "process {process_uuid} kill operation does not match the durable reclaim fence"
            )));
        }
        let adapter = self.owning_adapter(&identity)?;
        adapter
            .reclaim_detached(&identity.detached, crate::sandbox::Signal::Kill)
            .await
            .map_err(|error| KillError::new(error.to_string()))
    }

    async fn kill_operation_status(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        let identity = self.identity(resource_scope, process_uuid).await?;
        if identity.stopped {
            return Ok(ReclaimKillOperationStatus::Succeeded);
        }
        if identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Ok(ReclaimKillOperationStatus::NotStarted);
        }
        let adapter = self.owning_adapter(&identity)?;
        let status = tokio::time::timeout(
            Duration::from_secs(10),
            adapter.detached_status(&identity.detached),
        )
        .await
        .map_err(|_| {
            KillError::new(format!(
                "owning adapter status timed out for process {process_uuid}"
            ))
        })?
        .map_err(|error| KillError::new(error.to_string()))?;
        Ok(match status {
            crate::sandbox::ProcessStatus::Running => ReclaimKillOperationStatus::InProgress,
            crate::sandbox::ProcessStatus::Exited { .. }
            | crate::sandbox::ProcessStatus::Killed { .. }
            | crate::sandbox::ProcessStatus::Orphaned => ReclaimKillOperationStatus::Succeeded,
            crate::sandbox::ProcessStatus::FailedToStart { .. } => {
                ReclaimKillOperationStatus::Failed
            }
        })
    }
}

#[async_trait]
pub trait ReclaimStopReservation: Send + 'static {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError>;
}

pub trait ReclaimStopWriter: Send + Sync + 'static {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError>;
}

pub struct Reclaim {
    store: Arc<dyn ReclaimProcessStore>,
    sandbox_kill: Arc<dyn SandboxKill>,
    stop_writer: Arc<dyn ReclaimStopWriter>,
    claim_renew_interval: Duration,
    kill_timeout: Duration,
    stop_ack_timeout: Duration,
}

impl Reclaim {
    pub fn new<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Self
    where
        S: ReclaimProcessStore,
        K: SandboxKill,
        W: ReclaimStopWriter,
    {
        Self {
            store,
            sandbox_kill,
            stop_writer,
            claim_renew_interval: RECLAIM_CLAIM_RENEW_INTERVAL,
            kill_timeout: RECLAIM_KILL_TIMEOUT,
            stop_ack_timeout: RECLAIM_STOP_ACK_TIMEOUT,
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn with_reclaim_timings_for_test(
        mut self,
        claim_renew_interval: Duration,
        stop_ack_timeout: Duration,
    ) -> Self {
        self.claim_renew_interval = claim_renew_interval;
        self.stop_ack_timeout = stop_ack_timeout;
        self
    }

    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn with_kill_timeout_for_test(mut self, kill_timeout: Duration) -> Self {
        self.kill_timeout = kill_timeout;
        self
    }

    pub async fn run(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self
            .store
            .active_processes_for_session(resource_scope, session_id)
            .await?;
        self.run_claimed(session_id, trigger, started, active).await
    }

    pub async fn run_process(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        process_uuid: Uuid,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self
            .store
            .active_process_for_session(resource_scope, session_id, process_uuid)
            .await?
            .into_iter()
            .collect();
        self.run_claimed(session_id, trigger, started, active).await
    }

    /// MT-019 F1/P-2: reap exactly one process this runtime instance owns,
    /// without needing a coordinator session id.
    ///
    /// This is the running-app reap path. It exists because the row class it
    /// targets — an adapter-owned official-CLI child left OPEN mid-run because
    /// its STOP could not be proven — carries no `parent_session_id`, so it is
    /// invisible to every session-keyed claim AND to `restart_sessions`, and was
    /// therefore not reaped until some later boot.
    ///
    /// `owner_runtime_instance_id` is enforced inside the claim statement, so
    /// this path structurally cannot reach another live instance's processes.
    pub async fn run_owned_process(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        owner_runtime_instance_id: Uuid,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let claimed = self
            .store
            .active_owned_process(resource_scope, process_uuid, owner_runtime_instance_id)
            .await?;
        let session_id = claimed
            .as_ref()
            .and_then(|process| process.parent_session_id.clone())
            .unwrap_or_else(|| format!("process-ledger://{process_uuid}"));
        let active = claimed.into_iter().collect();
        self.run_claimed(&session_id, trigger, started, active)
            .await
    }

    /// MT-019 P-4(c): Restart-triggered reclaim of one surfaced orphan session
    /// that structurally excludes rows owned by the calling instance.
    pub async fn run_restart_orphan_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self
            .store
            .active_foreign_owner_processes_for_session(
                resource_scope,
                session_id,
                excluded_owner_runtime_instance_id,
                authorized_process_uuids,
            )
            .await?;
        self.run_claimed(session_id, ReclaimTrigger::Restart, started, active)
            .await
    }

    /// Reclaim a stale session without widening the source's runtime+host
    /// ownership decision to foreign rows that happen to share the session id.
    pub async fn run_stale_owned_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self
            .store
            .active_stale_owned_processes_for_session(
                resource_scope,
                session_id,
                owner_runtime_instance_id,
                owner_host_scope_id,
                authorized_process_uuids,
            )
            .await?;
        self.run_claimed(session_id, ReclaimTrigger::Stale, started, active)
            .await
    }

    async fn run_claimed(
        &self,
        session_id: &str,
        trigger: ReclaimTrigger,
        started: std::time::Instant,
        active: Vec<ReclaimableProcess>,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let mut reclaimed = Vec::with_capacity(active.len());
        let mut active = active.into_iter();

        while let Some(mut process) = active.next() {
            if process.resource_scope != process.reclaim_claim.resource_scope {
                self.release_unprocessed_claims_after_abort(
                    std::iter::once(&process).chain(active.as_slice().iter()),
                    "claim ResourceScope mismatch",
                )
                .await;
                return Err(ProcessLedgerError::Store(format!(
                    "reclaim claim for process {} escaped its exact ResourceScope",
                    process.process_uuid
                )));
            }
            let reservation = match self.stop_writer.reserve_reclaim_stop() {
                Ok(reservation) => reservation,
                Err(error) => {
                    self.release_unprocessed_claims_after_abort(
                        std::iter::once(&process).chain(active.as_slice().iter()),
                        "STOP reservation rejection",
                    )
                    .await;
                    return Err(error);
                }
            };

            let (kill_result, stop_event_kind) = if process.kill_succeeded_pending_stop {
                let stop = process.reclaim_stop(-1);
                match reservation.persist(stop, self.stop_ack_timeout).await {
                    Ok(()) => {
                        clear_completed_process_kill_fence(process.process_uuid);
                        (KillOutcome::Killed, Some(LedgerEventKind::Stop))
                    }
                    Err(error) => (
                        KillOutcome::KilledPendingStop {
                            error: error.to_string(),
                        },
                        None,
                    ),
                }
            } else {
                if let Err(error) = self
                    .store
                    .mark_reclaim_kill_started(process.process_uuid, &process.reclaim_claim)
                    .await
                {
                    drop(reservation);
                    self.release_unprocessed_claims_after_abort(
                        std::iter::once(&process).chain(active.as_slice().iter()),
                        "kill-start fence rejection",
                    )
                    .await;
                    return Err(error);
                }
                let (kill, renewal_error, renewed_claim, kill_fence) =
                    self.kill_with_claim_renewal(&process).await?;
                process.reclaim_claim = renewed_claim;
                process.sync_reclaim_claim_metadata()?;
                match kill {
                    Ok(()) => {
                        let stop = process.reclaim_stop(-1);
                        let pending_mark_error = self
                            .store
                            .mark_reclaim_kill_succeeded(&stop, &process.reclaim_claim)
                            .await
                            .err();
                        let stop_persisted = reservation.persist(stop, self.stop_ack_timeout).await;
                        if renewal_error.is_none()
                            && pending_mark_error.is_none()
                            && stop_persisted.is_ok()
                        {
                            clear_process_kill_fence(process.process_uuid, &kill_fence);
                            (KillOutcome::Killed, Some(LedgerEventKind::Stop))
                        } else {
                            let mut errors = Vec::new();
                            if let Some(error) = renewal_error {
                                errors.push(format!("claim renewal failed: {error}"));
                            }
                            if let Some(error) = pending_mark_error {
                                errors.push(format!("pending-stop marker failed: {error}"));
                            }
                            if let Err(error) = stop_persisted {
                                errors.push(format!("STOP durability failed: {error}"));
                            } else {
                                errors.push(
                                    "STOP was durable but reclaim ownership continuity was not proven"
                                        .to_string(),
                                );
                            }
                            (
                                KillOutcome::KilledPendingStop {
                                    error: errors.join("; "),
                                },
                                None,
                            )
                        }
                    }
                    Err(error) => {
                        drop(reservation);
                        let release_result = self
                            .store
                            .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
                            .await;
                        // The process-global fence only coalesces one live kill
                        // attempt. It must never retain a completed failure just
                        // because durable claim release also failed; otherwise
                        // a later durable retry replays the stale in-memory error
                        // without invoking the owning adapter again.
                        clear_process_kill_fence(process.process_uuid, &kill_fence);
                        release_result?;
                        (
                            KillOutcome::Failed {
                                error: error.message().to_string(),
                            },
                            None,
                        )
                    }
                }
            };
            reclaimed.push(ReclaimedProcess {
                process_uuid: process.process_uuid,
                engine_kind: process.engine_kind,
                sandbox_adapter_id: process.sandbox_adapter_id,
                kill_result,
                stop_event_kind,
            });
        }

        Ok(ReclaimReport {
            session_id: session_id.to_string(),
            trigger,
            processes_reclaimed: reclaimed,
            total_duration_ms: started.elapsed().as_millis(),
        })
    }

    /// Reconcile a crash-left kill operation from the owning adapter's
    /// authoritative idempotency record. Unknown/in-progress evidence leaves
    /// durable state unchanged and truthfully open; terminal/not-started evidence
    /// advances the shared recovery state without trusting a caller-supplied
    /// success flag.
    pub async fn reconcile_kill_operation(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, ProcessLedgerError> {
        let status = self
            .sandbox_kill
            .kill_operation_status(resource_scope, process_uuid, kill_operation_uuid)
            .await
            .map_err(|error| {
                ProcessLedgerError::Store(format!(
                    "kill-operation status query failed for process {process_uuid} operation {kill_operation_uuid}: {error}"
                ))
            })?;
        if matches!(
            status,
            ReclaimKillOperationStatus::Succeeded
                | ReclaimKillOperationStatus::Failed
                | ReclaimKillOperationStatus::NotStarted
        ) {
            self.store
                .resolve_reclaim_kill_operation(
                    resource_scope,
                    process_uuid,
                    kill_operation_uuid,
                    status,
                )
                .await?;
        }
        Ok(status)
    }

    pub async fn reconcile_in_progress_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ReclaimKillOperationSweep, ProcessLedgerError> {
        let recoverable = self
            .store
            .in_progress_kill_operations_for_session(
                resource_scope,
                session_id,
                excluded_owner_runtime_instance_id,
                authorized_process_uuids,
                RECLAIM_IN_PROGRESS_RECOVERY_LIMIT,
            )
            .await?;
        self.reconcile_in_progress_candidates(resource_scope, session_id, recoverable, None, None)
            .await
    }

    pub async fn reconcile_in_progress_for_stale_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ReclaimKillOperationSweep, ProcessLedgerError> {
        let recoverable = self
            .store
            .in_progress_kill_operations_for_stale_owner(
                resource_scope,
                session_id,
                owner_runtime_instance_id,
                owner_host_scope_id,
                authorized_process_uuids,
                RECLAIM_IN_PROGRESS_RECOVERY_LIMIT,
            )
            .await?;
        self.reconcile_in_progress_candidates(
            resource_scope,
            session_id,
            recoverable,
            Some((owner_runtime_instance_id, owner_host_scope_id)),
            Some(authorized_process_uuids),
        )
        .await
    }

    async fn reconcile_in_progress_candidates(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        recoverable: Vec<ReclaimKillOperationCandidate>,
        stale_owner_scope: Option<(Uuid, &str)>,
        stale_authorized_process_uuids: Option<&[Uuid]>,
    ) -> Result<ReclaimKillOperationSweep, ProcessLedgerError> {
        let mut operations = Vec::with_capacity(recoverable.len());
        let mut state_advanced = false;
        for candidate in recoverable {
            let operation = match candidate {
                ReclaimKillOperationCandidate::Operation { operation } => operation,
                ReclaimKillOperationCandidate::Malformed {
                    process_identity,
                    kill_operation_identity,
                    error,
                } => {
                    operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Malformed {
                            process_identity,
                            kill_operation_identity,
                            error: error.clone(),
                        },
                        outcome: ReclaimKillOperationSweepOutcome::MalformedRecoveryRow { error },
                    });
                    continue;
                }
            };
            if operation.resource_scope != *resource_scope {
                return Err(ProcessLedgerError::Store(format!(
                    "reclaim recovery operation for process {} escaped its exact ResourceScope",
                    operation.process_uuid
                )));
            }
            let status = match self
                .sandbox_kill
                .kill_operation_status(
                    &operation.resource_scope,
                    operation.process_uuid,
                    operation.kill_operation_uuid,
                )
                .await
            {
                Ok(status) => status,
                Err(error) => {
                    operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Operation { operation },
                        outcome: ReclaimKillOperationSweepOutcome::StatusQueryFailed {
                            error: error.message().to_string(),
                        },
                    });
                    continue;
                }
            };
            if matches!(
                status,
                ReclaimKillOperationStatus::Succeeded
                    | ReclaimKillOperationStatus::Failed
                    | ReclaimKillOperationStatus::NotStarted
            ) {
                match self
                    .store
                    .resolve_reclaim_kill_operation(
                        &operation.resource_scope,
                        operation.process_uuid,
                        operation.kill_operation_uuid,
                        status,
                    )
                    .await
                {
                    Ok(()) => {
                        state_advanced = true;
                        operations.push(ReclaimKillOperationSweepEntry {
                            candidate: ReclaimKillOperationCandidate::Operation { operation },
                            outcome: ReclaimKillOperationSweepOutcome::StateAdvanced { status },
                        });
                    }
                    Err(error) => operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Operation { operation },
                        outcome: ReclaimKillOperationSweepOutcome::StateTransitionFailed {
                            status,
                            error: error.to_string(),
                        },
                    }),
                }
            } else {
                operations.push(ReclaimKillOperationSweepEntry {
                    candidate: ReclaimKillOperationCandidate::Operation { operation },
                    outcome: ReclaimKillOperationSweepOutcome::StateOpen { status },
                });
            }
        }
        let (reclaim_report, reclaim_error) = if state_advanced {
            let reclaim_result = match stale_owner_scope {
                Some((owner_runtime_instance_id, owner_host_scope_id)) => {
                    self.run_stale_owned_session(
                        resource_scope,
                        session_id,
                        owner_runtime_instance_id,
                        owner_host_scope_id,
                        stale_authorized_process_uuids.unwrap_or_default(),
                    )
                    .await
                }
                None => {
                    self.run(resource_scope, session_id, ReclaimTrigger::Stale)
                        .await
                }
            };
            match reclaim_result {
                Ok(report) => (Some(report), None),
                Err(error) => (None, Some(error.to_string())),
            }
        } else {
            (None, None)
        };
        Ok(ReclaimKillOperationSweep {
            operations,
            reclaim_report,
            reclaim_error,
        })
    }

    async fn release_unprocessed_claims_after_abort<'a>(
        &self,
        processes: impl Iterator<Item = &'a ReclaimableProcess>,
        context: &'static str,
    ) {
        for process in processes {
            if process.kill_succeeded_pending_stop {
                continue;
            }
            if let Err(error) = self
                .store
                .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
                .await
            {
                tracing::error!(
                    process_uuid = %process.process_uuid,
                    error = %error,
                    context,
                    "failed to release an unprocessed reclaim claim after abort"
                );
            }
        }
    }

    async fn kill_with_claim_renewal(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<
        (
            Result<(), KillError>,
            Option<ProcessLedgerError>,
            ReclaimClaim,
            Arc<ProcessKillFence>,
        ),
        ProcessLedgerError,
    > {
        let process_uuid = process.process_uuid;
        let (kill_fence, owns_kill) = acquire_process_kill_fence(process_uuid);
        let mut claim = process.reclaim_claim.clone();
        let mut renewal_error = None;
        let renew_every = if self.claim_renew_interval.is_zero() {
            Duration::from_millis(1)
        } else {
            self.claim_renew_interval.min(RECLAIM_CLAIM_TTL / 2)
        };
        let mut renewal = time::interval(renew_every);
        renewal.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // `interval` ticks immediately; the claim was just committed, so wait
        // one full interval before the first renewal.
        renewal.tick().await;
        let kill_timeout = if self.kill_timeout.is_zero() {
            Duration::from_millis(1)
        } else {
            self.kill_timeout
        };
        let kill_deadline = time::sleep(kill_timeout);
        tokio::pin!(kill_deadline);

        if owns_kill {
            let killer = Arc::clone(&self.sandbox_kill);
            let resource_scope = process.resource_scope.clone();
            let kill_operation_uuid = process.reclaim_claim.kill_operation_uuid;
            let mut kill_task = tokio::spawn(async move {
                killer
                    .kill(&resource_scope, process_uuid, kill_operation_uuid)
                    .await
            });
            loop {
                tokio::select! {
                    joined = &mut kill_task => {
                    let result = joined.unwrap_or_else(|error| {
                        Err(KillError::new(format!(
                            "reclaim kill task for process {process_uuid} failed to join: {error}"
                        )))
                    });
                    match kill_fence.result.lock() {
                        Ok(mut published) => *published = Some(result.clone()),
                        Err(poisoned) => *poisoned.into_inner() = Some(result.clone()),
                    }
                    kill_fence.completed.notify_waiters();
                    return Ok((result, renewal_error, claim, kill_fence));
                    }
                    _ = renewal.tick() => {
                        match self.store.renew_reclaim_claim(process_uuid, &claim).await {
                            Ok(renewed) => {
                                claim = renewed;
                                renewal_error = None;
                            }
                            Err(error) => renewal_error = Some(error),
                        }
                    }
                    _ = &mut kill_deadline => {
                        kill_task.abort();
                        let _ = (&mut kill_task).await;
                        let result = Err(KillError::new(format!(
                            "reclaim kill operation for process {process_uuid} exceeded {}ms",
                            kill_timeout.as_millis()
                        )));
                        match kill_fence.result.lock() {
                            Ok(mut published) => *published = Some(result.clone()),
                            Err(poisoned) => *poisoned.into_inner() = Some(result.clone()),
                        }
                        kill_fence.completed.notify_waiters();
                        return Ok((result, renewal_error, claim, kill_fence));
                    }
                }
            }
        } else {
            loop {
                let completed = kill_fence.completed.notified();
                tokio::pin!(completed);
                if let Some(result) = match kill_fence.result.lock() {
                    Ok(result) => result.clone(),
                    Err(poisoned) => poisoned.into_inner().clone(),
                } {
                    return Ok((result, renewal_error, claim, Arc::clone(&kill_fence)));
                }
                tokio::select! {
                    _ = &mut completed => {}
                    _ = renewal.tick() => {
                        match self.store.renew_reclaim_claim(process_uuid, &claim).await {
                            Ok(renewed) => {
                                claim = renewed;
                                renewal_error = None;
                            }
                            Err(error) => renewal_error = Some(error),
                        }
                    }
                    _ = &mut kill_deadline => {
                        return Ok((
                            Err(KillError::new(format!(
                                "coalesced reclaim wait for process {process_uuid} exceeded {}ms",
                                kill_timeout.as_millis()
                            ))),
                            renewal_error,
                            claim,
                            Arc::clone(&kill_fence),
                        ));
                    }
                }
            }
        }
    }
}

pub fn reclaim_handle<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Reclaim
where
    S: ReclaimProcessStore,
    K: SandboxKill,
    W: ReclaimStopWriter,
{
    Reclaim::new(store, sandbox_kill, stop_writer)
}

struct WriterReclaimStopReservation {
    reserved: ReservedProcessStop,
}

#[async_trait]
impl ReclaimStopReservation for WriterReclaimStopReservation {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.reserved
            .commit_with_durable_ack(stop)?
            .wait(timeout)
            .await
    }
}

impl ReclaimStopWriter for ProcessLedgerWriter {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterReclaimStopReservation {
            reserved: self.try_reserve_reclaim_stop()?,
        }))
    }
}

impl ReclaimStopWriter for super::LedgerBatcher {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterReclaimStopReservation {
            reserved: self.try_reserve_reclaim_stop()?,
        }))
    }
}

fn stored_pid_to_u32(value: i64) -> Result<u32, ProcessLedgerError> {
    u32::try_from(value)
        .map_err(|_| ProcessLedgerError::Store(format!("invalid os_pid in reclaim query: {value}")))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaleSessionProcessSet {
    pub resource_scope: ReclaimResourceScope,
    pub session_id: String,
    pub authorized_process_uuids: Vec<Uuid>,
}

#[async_trait]
pub trait StaleSessionSource: Send + Sync + 'static {
    async fn stale_sessions(&self, ttl: Duration) -> Result<Vec<String>, ProcessLedgerError>;

    async fn stale_session_process_sets(
        &self,
        _ttl: Duration,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        Err(ProcessLedgerError::InvalidConfig(
            "STALE_RECLAIM_PROCESS_SET_REQUIRED".to_string(),
        ))
    }

    async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(Vec::new())
    }

    async fn restart_session_process_sets(
        &self,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        Err(ProcessLedgerError::InvalidConfig(
            "RESTART_RECLAIM_EXACT_SCOPE_PROCESS_SET_REQUIRED".to_owned(),
        ))
    }

    /// The runtime instance whose open rows a restart pass must never claim.
    ///
    /// MT-019 P-4(c): when a source knows its own instance identity, the restart
    /// reclaim binds it as an explicit `owner_runtime_instance_id <> self`
    /// predicate inside the claim statement instead of relying only on the
    /// surfacing-level veto in [`Self::restart_sessions`].
    fn self_runtime_instance_id(&self) -> Option<Uuid> {
        None
    }

    /// Exact owner boundary used by stale-session selection. Callers must carry
    /// both values into the atomic claim instead of widening back to session id.
    fn self_runtime_owner_scope(&self) -> Option<(Uuid, String)> {
        None
    }

    fn require_runtime_owner_scope(&self) -> Result<(Uuid, String), ProcessLedgerError> {
        self.self_runtime_owner_scope().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig("STALE_RECLAIM_OWNER_SCOPE_REQUIRED".to_string())
        })
    }
}

/// Test-only override for the default dead-owner confirmation gap.
///
/// `ProcessReclaimRuntime::production_with_lease` composes its own stale-session
/// source internally, so a proof that must drive the REAL boot composition has no
/// other seam to shorten the corroboration window with. Only ever set to
/// `Duration::ZERO` (legacy single-sample) by proofs whose subject is not the
/// two-sample gate itself; the gate has its own dedicated proof that configures
/// the gap explicitly on the source instead of through this override.
#[cfg(feature = "test-utils")]
static DEAD_OWNER_CONFIRMATION_GAP_OVERRIDE_MS: std::sync::atomic::AtomicI64 =
    std::sync::atomic::AtomicI64::new(-1);

/// Set (or clear) the process-wide dead-owner confirmation gap override.
#[doc(hidden)]
#[cfg(feature = "test-utils")]
pub fn set_dead_owner_confirmation_gap_override_for_test(gap: Option<Duration>) {
    let encoded = gap
        .and_then(|gap| i64::try_from(gap.as_millis()).ok())
        .unwrap_or(-1);
    DEAD_OWNER_CONFIRMATION_GAP_OVERRIDE_MS.store(encoded, std::sync::atomic::Ordering::SeqCst);
}

pub(crate) fn default_dead_owner_confirmation_gap() -> Duration {
    #[cfg(feature = "test-utils")]
    {
        let override_ms =
            DEAD_OWNER_CONFIRMATION_GAP_OVERRIDE_MS.load(std::sync::atomic::Ordering::SeqCst);
        if override_ms >= 0 {
            return Duration::from_millis(override_ms as u64);
        }
    }
    StalenessReclaimConfig::default().scan_interval
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StalenessReclaimConfig {
    pub ttl: Duration,
    pub scan_interval: Duration,
}

impl StalenessReclaimConfig {
    pub fn normalized(self) -> Self {
        Self {
            ttl: if self.ttl.is_zero() {
                Duration::from_secs(300)
            } else {
                self.ttl
            },
            scan_interval: if self.scan_interval.is_zero() {
                Duration::from_secs(30)
            } else {
                self.scan_interval
            },
        }
    }
}

impl Default for StalenessReclaimConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300),
            scan_interval: Duration::from_secs(30),
        }
    }
}

/// Durable evidence produced by one composed restart-reconcile pass.
#[derive(Debug, Default, Clone)]
pub struct RestartOrphanBootReconcileReport {
    /// Sessions surfaced by [`StaleSessionSource::restart_sessions`] whose open
    /// process rows were reconciled this pass.
    pub sessions_reconciled: usize,
    /// Process rows this pass actually reclaimed: [`KillOutcome::Killed`] or
    /// [`KillOutcome::KilledPendingStop`] only.
    ///
    /// MT-019 F5: this previously counted every [`ReclaimedProcess`], so a
    /// [`KillOutcome::Failed`] row — a process that is still running and whose
    /// START row is still OPEN — was reported as reclaimed.
    pub processes_reclaimed: usize,
    /// Process rows whose kill did NOT succeed. Their claim was released, their
    /// fence cleared, and no STOP was written, so they remain truthfully open and
    /// idempotently retryable by a later pass. Non-zero here means boot completed
    /// with known-unreaped processes (see the F3 resilient-boot contract).
    pub processes_kill_failed: usize,
    /// The per-session Restart reclaim reports, in surfaced order.
    pub reclaim_reports: Vec<ReclaimReport>,
    /// MT-019 F6: reclaim errors observed INSIDE the in-progress kill-operation
    /// sweep ([`ReclaimKillOperationSweep::reclaim_error`]). The sweep returns
    /// `Ok` while carrying this field, so the boot call site used to drop it
    /// silently. It is recorded rather than escalated: escalating it would
    /// convert the recorded fail-open boot contract into a fail-closed one.
    pub sweep_reclaim_errors: Vec<String>,
    /// Non-fatal per-session errors tolerated under
    /// [`RestartOrphanReconcileErrorPolicy::LogAndContinue`].
    pub session_errors: Vec<String>,
    /// The pass stopped early because the caller's cancellation hook fired.
    pub cancelled: bool,
}

impl RestartOrphanBootReconcileReport {
    fn record(&mut self, reclaim_report: ReclaimReport) {
        self.sessions_reconciled += 1;
        for reclaimed in &reclaim_report.processes_reclaimed {
            match reclaimed.kill_result {
                KillOutcome::Killed | KillOutcome::KilledPendingStop { .. } => {
                    self.processes_reclaimed += 1
                }
                KillOutcome::Failed { .. } => self.processes_kill_failed += 1,
            }
        }
        self.reclaim_reports.push(reclaim_report);
    }
}

/// How one restart-reconcile pass treats a per-session error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartOrphanReconcileErrorPolicy {
    /// Boot semantics: the first surfacing-scan, in-progress-reconcile, or
    /// reclaim error aborts the pass and propagates so the caller can fail closed
    /// instead of continuing as if reconciliation completed.
    AbortOnFirstError,
    /// Periodic-task semantics: a single bad session must not silently retire the
    /// long-running reclaimer, so per-session errors are recorded and the pass
    /// continues with the next surfaced session.
    LogAndContinue,
}

/// Run the composed restart-reconcile pass: reclaim every restart-orphan session
/// the configured authoritative [`StaleSessionSource`] surfaces.
///
/// This is the exact composition [`ProcessReclaimRuntime`](crate::process_ledger::ProcessReclaimRuntime)
/// runs at boot AND the periodic restart tick runs afterwards — MT-019 F6 folds
/// the previously duplicated inline staleness-task loop into this one function so
/// the two cannot drift, with the caller choosing the error policy and supplying
/// a cancellation hook.
///
/// A generic spawned-process START row (for example an Official-CLI bridge child)
/// whose owning runtime instance is provably dead is killed via the composed
/// [`SandboxKill`] and given a durable STOP.
///
/// Kill failure is NOT an error here: [`Reclaim::run_claimed`] releases the claim,
/// clears the fence, and writes no STOP, so the row stays truthfully open. Those
/// rows are counted in [`RestartOrphanBootReconcileReport::processes_kill_failed`]
/// for the caller to surface and retry.
///
/// When the source knows its own instance identity the reclaim binds an explicit
/// `owner_runtime_instance_id <> self` predicate (P-4c), so a restart pass cannot
/// claim the calling instance's own rows even if surfacing is ever wrong.
pub async fn reconcile_restart_orphans(
    reclaim: &Reclaim,
    stale_source: &dyn StaleSessionSource,
    policy: RestartOrphanReconcileErrorPolicy,
    cancelled: &(dyn Fn() -> bool + Send + Sync),
) -> Result<RestartOrphanBootReconcileReport, ProcessLedgerError> {
    let mut report = RestartOrphanBootReconcileReport::default();
    let surfaced = match stale_source.restart_session_process_sets().await {
        Ok(surfaced) => surfaced,
        Err(error) => match policy {
            RestartOrphanReconcileErrorPolicy::AbortOnFirstError => return Err(error),
            RestartOrphanReconcileErrorPolicy::LogAndContinue => {
                tracing::error!(error = %error, "process-ledger restart-session scan failed");
                report.session_errors.push(error.to_string());
                return Ok(report);
            }
        },
    };
    let excluded_owner = stale_source.self_runtime_instance_id().ok_or_else(|| {
        ProcessLedgerError::InvalidConfig(
            "RESTART_RECLAIM_SELF_RUNTIME_INSTANCE_REQUIRED".to_owned(),
        )
    })?;
    for candidate in surfaced {
        let session_id = &candidate.session_id;
        if cancelled() {
            report.cancelled = true;
            return Ok(report);
        }
        match reclaim
            .reconcile_in_progress_for_session(
                &candidate.resource_scope,
                session_id,
                excluded_owner,
                &candidate.authorized_process_uuids,
            )
            .await
        {
            Ok(sweep) => {
                if let Some(sweep_error) = sweep.reclaim_error {
                    tracing::warn!(
                        session_id,
                        error = %sweep_error,
                        "in-progress kill-operation sweep advanced state but its follow-up reclaim failed; the row remains open for a later pass"
                    );
                    report.sweep_reclaim_errors.push(sweep_error);
                }
            }
            Err(error) => match policy {
                RestartOrphanReconcileErrorPolicy::AbortOnFirstError => return Err(error),
                RestartOrphanReconcileErrorPolicy::LogAndContinue => {
                    tracing::error!(session_id, error = %error, "process-ledger restart kill reconciliation failed");
                    report.session_errors.push(error.to_string());
                    continue;
                }
            },
        }
        let reclaim_result = reclaim
            .run_restart_orphan_session(
                &candidate.resource_scope,
                session_id,
                excluded_owner,
                &candidate.authorized_process_uuids,
            )
            .await;
        match reclaim_result {
            Ok(reclaim_report) => report.record(reclaim_report),
            Err(error) => match policy {
                RestartOrphanReconcileErrorPolicy::AbortOnFirstError => return Err(error),
                RestartOrphanReconcileErrorPolicy::LogAndContinue => {
                    tracing::error!(session_id, error = %error, "process-ledger restart reclaim failed");
                    report.session_errors.push(error.to_string());
                }
            },
        }
    }
    Ok(report)
}

/// Boot entry point for [`reconcile_restart_orphans`]: fail closed on the first
/// error, no cancellation hook.
pub async fn reconcile_restart_orphans_at_boot(
    reclaim: &Reclaim,
    stale_source: &dyn StaleSessionSource,
) -> Result<RestartOrphanBootReconcileReport, ProcessLedgerError> {
    reconcile_restart_orphans(
        reclaim,
        stale_source,
        RestartOrphanReconcileErrorPolicy::AbortOnFirstError,
        &|| false,
    )
    .await
}

async fn reconcile_and_reclaim_stale_session(
    reclaim: &Reclaim,
    stale_source: &dyn StaleSessionSource,
    candidate: &StaleSessionProcessSet,
) -> Result<(), ProcessLedgerError> {
    let (owner_runtime_instance_id, owner_host_scope_id) =
        stale_source.require_runtime_owner_scope()?;
    reclaim
        .reconcile_in_progress_for_stale_owner(
            &candidate.resource_scope,
            &candidate.session_id,
            owner_runtime_instance_id,
            &owner_host_scope_id,
            &candidate.authorized_process_uuids,
        )
        .await?;
    reclaim
        .run_stale_owned_session(
            &candidate.resource_scope,
            &candidate.session_id,
            owner_runtime_instance_id,
            &owner_host_scope_id,
            &candidate.authorized_process_uuids,
        )
        .await?;
    Ok(())
}

pub fn spawn_staleness_reclaim_task(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> JoinHandle<()> {
    let config = config.normalized();
    tokio::spawn(async move {
        let mut interval = time::interval(config.scan_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let candidates = match stale_source.stale_session_process_sets(config.ttl).await {
                Ok(candidates) => candidates,
                Err(error) => {
                    tracing::error!(error = %error, "process-ledger stale-session scan failed");
                    continue;
                }
            };
            for candidate in candidates {
                if let Err(error) = reconcile_and_reclaim_stale_session(
                    reclaim.as_ref(),
                    stale_source.as_ref(),
                    &candidate,
                )
                .await
                {
                    tracing::error!(error = %error, "process-ledger stale-session reclaim failed");
                }
            }
        }
    })
}

#[derive(Clone)]
pub struct ManagedStalenessReclaimTask {
    inner: Arc<ManagedStalenessReclaimTaskInner>,
}

struct ManagedStalenessReclaimTaskInner {
    shutdown: watch::Sender<bool>,
    join: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl ManagedStalenessReclaimTask {
    pub async fn shutdown_and_join(&self, timeout: Duration) -> bool {
        let _ = self.inner.shutdown.send(true);
        let join = self
            .inner
            .join
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(mut join) = join else {
            return true;
        };
        match time::timeout(timeout, &mut join).await {
            Ok(Ok(())) => true,
            Ok(Err(error)) if error.is_cancelled() => true,
            Ok(Err(error)) => {
                tracing::error!(error = %error, "managed process-reclaim task failed to join");
                false
            }
            Err(_) => {
                join.abort();
                let _ = join.await;
                false
            }
        }
    }

    pub fn abort_and_join_blocking(&self, timeout: Duration) -> bool {
        let _ = self.inner.shutdown.send(true);
        let join = self
            .inner
            .join
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(join) = join else {
            return true;
        };
        join.abort();
        let (completed_tx, completed_rx) = std::sync::mpsc::sync_channel(1);
        let helper = std::thread::Builder::new()
            .name("handshake-reclaim-drop-join".to_string())
            .spawn(move || {
                let _ = futures::executor::block_on(join);
                let _ = completed_tx.send(());
            });
        let Ok(_helper) = helper else {
            return false;
        };
        completed_rx.recv_timeout(timeout).is_ok()
    }
}

pub fn spawn_managed_staleness_reclaim_task(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> ManagedStalenessReclaimTask {
    spawn_managed_staleness_reclaim_task_internal(reclaim, stale_source, config, true)
}

/// Post-boot variant: the caller has ALREADY run the boot restart pass inline, so
/// this task skips the immediate restart pass and relies on its periodic tick
/// (MT-019 F2) to re-surface anything the boot pass skipped or timed out on.
pub fn spawn_managed_staleness_reclaim_task_after_boot(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> ManagedStalenessReclaimTask {
    spawn_managed_staleness_reclaim_task_internal(reclaim, stale_source, config, false)
}

fn spawn_managed_staleness_reclaim_task_internal(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
    run_restart_pass: bool,
) -> ManagedStalenessReclaimTask {
    let config = config.normalized();
    let (shutdown, mut shutdown_rx) = watch::channel(false);
    let join = tokio::spawn(async move {
        // MT-019 F6: one shared implementation for the boot pass and every
        // periodic pass. The task can only tolerate errors, never abort, so it
        // always uses LogAndContinue plus its shutdown watch as the hook.
        let run_restart_reconcile = |shutdown_rx: &watch::Receiver<bool>| {
            let cancelled = shutdown_rx.clone();
            let reclaim = Arc::clone(&reclaim);
            let stale_source = Arc::clone(&stale_source);
            async move {
                let report = reconcile_restart_orphans(
                    reclaim.as_ref(),
                    stale_source.as_ref(),
                    RestartOrphanReconcileErrorPolicy::LogAndContinue,
                    &move || *cancelled.borrow(),
                )
                .await;
                match report {
                    Ok(report) if report.processes_kill_failed > 0 => tracing::warn!(
                        sessions_reconciled = report.sessions_reconciled,
                        processes_reclaimed = report.processes_reclaimed,
                        processes_kill_failed = report.processes_kill_failed,
                        "periodic restart-orphan reclaim left un-reapable processes open for a later pass"
                    ),
                    Ok(_) => {}
                    Err(error) => {
                        tracing::error!(error = %error, "periodic restart-orphan reclaim pass failed")
                    }
                }
            }
        };

        if run_restart_pass {
            run_restart_reconcile(&shutdown_rx).await;
        }

        let mut interval = time::interval(config.scan_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return;
                    }
                }
                _ = interval.tick() => {
                    // MT-019 F2: the boot restart pass runs exactly once inline in
                    // `production_with_lease`, so a SKIPPED or timed-out boot pass
                    // used to leave restart orphans unreaped until the next boot.
                    // The periodic tick now re-surfaces them. This is safe to run
                    // continuously only because of P-4: a live instance never
                    // releases its loopback lease before process exit, a prior
                    // owner must be observed free twice at least one scan interval
                    // apart, and the claim itself excludes this instance's rows.
                    run_restart_reconcile(&shutdown_rx).await;
                    if *shutdown_rx.borrow() {
                        return;
                    }
                    let candidates = match stale_source.stale_session_process_sets(config.ttl).await {
                        Ok(candidates) => candidates,
                        Err(error) => {
                            tracing::error!(error = %error, "process-ledger stale-session scan failed");
                            continue;
                        }
                    };
                    for candidate in candidates {
                        if *shutdown_rx.borrow() {
                            return;
                        }
                        if let Err(error) = reconcile_and_reclaim_stale_session(
                            reclaim.as_ref(),
                            stale_source.as_ref(),
                            &candidate,
                        )
                        .await
                        {
                            tracing::error!(error = %error, "process-ledger stale-session reclaim failed");
                        }
                    }
                }
            }
        }
    });
    ManagedStalenessReclaimTask {
        inner: Arc::new(ManagedStalenessReclaimTaskInner {
            shutdown,
            join: std::sync::Mutex::new(Some(join)),
        }),
    }
}

impl Drop for ManagedStalenessReclaimTaskInner {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        let join = self
            .join
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(join) = join else {
            return;
        };
        join.abort();
        let (completed_tx, completed_rx) = std::sync::mpsc::sync_channel(1);
        let helper = std::thread::Builder::new()
            .name("handshake-reclaim-final-drop-join".to_string())
            .spawn(move || {
                let _ = futures::executor::block_on(join);
                let _ = completed_tx.send(());
            });
        if helper.is_ok() && completed_rx.recv_timeout(Duration::from_secs(2)).is_err() {
            tracing::error!(
                "managed process-reclaim task did not terminate within the bounded final-drop deadline"
            );
        }
    }
}

#[cfg(test)]
mod exact_scope_static_tests {
    use super::*;

    #[test]
    fn production_kill_identity_is_bound_to_all_five_scope_fields() {
        for predicate in [
            "owner_account_id = $owner_account_id",
            "actor_principal_id = $actor_principal_id",
            "authenticated_session_id = $authenticated_session_id",
            "access_space_id = $access_space_id",
            "workspace_id = $workspace_id",
        ] {
            assert!(LOAD_PRODUCTION_KILL_IDENTITY.contains(predicate));
        }
    }

    #[test]
    fn one_field_scope_mismatch_is_not_exact_scope_identity() {
        let account_uuid = Uuid::now_v7();
        let actor_uuid = Uuid::now_v7();
        let session_uuid = Uuid::now_v7();
        let access_space_uuid = Uuid::now_v7();
        let expected = ReclaimResourceScope {
            account_uuid,
            actor_uuid,
            session_uuid,
            workspace_id: "workspace-a".to_owned(),
            access_space_uuid,
        };
        let mismatched = ReclaimResourceScope {
            actor_uuid: Uuid::now_v7(),
            ..expected.clone()
        };
        assert_ne!(expected, mismatched);
    }
}
