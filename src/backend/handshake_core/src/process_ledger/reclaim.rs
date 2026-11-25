use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{RecordId, SurrealValue, Value as SurrealDataValue};
use thiserror::Error;
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use crate::storage::surreal::SurrealStorage;

use super::{
    LedgerEvent, LedgerEventKind, ProcessEngineKind, ProcessLedgerError, ProcessLedgerStore,
    ProcessRuntimeOwner, ProcessStop, SurrealProcessLedgerStore,
};

pub use handshake_runtime_instance::{
    acquire_embedded_runtime_instance_lease, resolve_embedded_runtime_host_scope,
    resolve_embedded_runtime_host_scope_with_override, EmbeddedRuntimeInstanceDescriptor,
    EmbeddedRuntimeInstanceLease, EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
    EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL, HANDSHAKE_HOST_SCOPE_ID_ENV,
};
pub(crate) use handshake_runtime_instance::runtime_owner_loopback_lease_is_free;

/// Sentinel written into `stop_reason` when the claim finds the column empty.
///
/// It is also the marker the follow-up STOP upsert overwrites, and â€” together
/// with the bound claim timestamp â€” the precise guard the compensating
/// un-claim uses.
const RECLAIM_CLAIM_STOP_REASON: &str = "reclaim_claimed";
const RECLAIM_KILLED_STOP_REASON: &str = "reclaim_killed";
const SURREAL_CLAIM_MAX_ATTEMPTS: usize = 10;
const SURREAL_CLAIM_BACKOFF_CAP_MS: u64 = 32;
/// Master Spec 5.7.5 exit code of a reclaim STOP. It is recorded only when the
/// owner proved that the process ended but no exit status could be read.
const RECLAIM_STOP_EXIT_CODE: i32 = -1;
static RECLAIM_BOOT_OWNER_ID: OnceLock<Uuid> = OnceLock::new();

fn reclaim_boot_owner_id() -> Uuid {
    *RECLAIM_BOOT_OWNER_ID.get_or_init(Uuid::now_v7)
}

fn is_surreal_retryable_transaction_conflict(
    error: &crate::storage::surreal::SurrealStorageError,
) -> bool {
    matches!(
        error,
        crate::storage::surreal::SurrealStorageError::Database(source)
            if source
                .to_string()
                .contains("Transaction conflict: Resource busy. This transaction can be retried")
    )
}

fn surreal_claim_retry_delay(seed: Uuid, failed_attempt: usize) -> Duration {
    let exponential_cap = 1_u64
        .checked_shl(failed_attempt.min(5) as u32)
        .unwrap_or(SURREAL_CLAIM_BACKOFF_CAP_MS)
        .min(SURREAL_CLAIM_BACKOFF_CAP_MS);
    let seed = seed.as_u128();
    let mut mixed = (seed as u64)
        ^ ((seed >> 64) as u64)
        ^ (failed_attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    Duration::from_millis(mixed % (exponential_cap + 1))
}

/// MT-008: atomically claim the active (un-stopped) rows for a session.
///
/// The original PostgreSQL form was `SELECT ... FOR UPDATE` executed in a
/// transaction that was committed immediately, releasing the row locks *before*
/// the reclaim decision (sandbox kill + stop-event write) ran. Two concurrent
/// reclaims could therefore both read the same active rows and both act on
/// them, double-killing a process and writing duplicate STOP rows (or, under
/// interleaving, missing a row).
///
/// The exclusion must cover the rows being reclaimed for the *whole* reclaim
/// decision, so the read-modify-write is collapsed into ONE statement. SurrealDB
/// runs a single statement in its own transaction, so this `UPDATE ... WHERE
/// stopped_at = NONE ... RETURN BEFORE` is the direct equivalent of the
/// `UPDATE ... WHERE stopped_at IS NULL ... RETURNING` it replaces: it claims
/// each matching row by stamping `stopped_at`, and a concurrent reclaim's
/// identical statement then matches zero of those rows because `stopped_at` is
/// no longer `NONE`. Claiming and excluding happen in the same commit, so two
/// racing reclaims cannot both observe a row as active.
///
/// Two details are load-bearing:
///
/// * The claim timestamp is BOUND from the caller rather than produced by
///   `time::now()` server-side. Knowing the exact stamp we wrote is what lets
///   the compensating un-claim below target only rows this claim actually took:
///   if a real STOP landed in between it overwrote `stopped_at`, the guard no
///   longer matches, and the row is correctly left alone.
/// * `RETURN BEFORE` yields the pre-claim row. Every field the caller needs is
///   an identity field the UPDATE does not touch, so BEFORE and AFTER are
///   identical for them â€” but BEFORE additionally carries the original
///   `stop_reason`, which the compensation restores verbatim.
/// * The same UPDATE persists the exact prior terminal pair under a reserved
///   metadata key before replacing it. If the returned row is too malformed to
///   identify, compensation re-reads these per-record receipts; it never resets
///   a whole session to an invented `NONE/NONE` state.
///
/// The claim is durable the instant the statement commits; the kill outcome /
/// exit_code is refined afterward by the normal STOP upsert, which overwrites
/// the sentinel `stopped_at` with the real stop time. The owner suffix in
/// `stop_reason` is a process-boot token, not a time lease: every store handle in
/// one live process shares it, so a slow kill can never be stolen merely because
/// time elapsed. A restarted process has a different suffix and can immediately
/// recover the prior boot's abandoned sentinel.
pub const SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     metadata.__handshake_internal_reclaim_durable_receipt_v1 = IF stop_reason = $killed_reason { \
         metadata.__handshake_internal_reclaim_durable_receipt_v1 \
     } ELSE { { \
         receipt_kind: 'handshake_reclaim_durable_receipt_v1', \
         previous_stopped_at: stopped_at, \
         previous_stop_reason: stop_reason \
     } }, \
     stopped_at = IF stop_reason = $killed_reason { stopped_at } ELSE { $claimed_at }, \
     stop_reason = IF stopped_at != NONE AND exit_code = NONE \
         AND string::starts_with(stop_reason, $killed_prefix) { $killed_reason } ELSE { $claim_reason } \
     WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND parent_session_id = $session_id AND ( \
         stopped_at = NONE OR ( \
             exit_code = NONE AND ( \
                 (string::starts_with(stop_reason, $claim_prefix) \
                  AND stop_reason != $claim_reason) \
                 OR string::starts_with(stop_reason, $killed_prefix) \
             ) \
         ) \
     ) RETURN BEFORE;";

/// Releases one row claimed by [`SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY`].
///
/// `stopped_at = $claimed_at` is the whole safety of this statement: it matches
/// only a row still carrying the exact sentinel this claim wrote. A row whose
/// STOP has since landed has a different `stopped_at` and is skipped.
const SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stopped_at = $previous_stopped_at, stop_reason = $stop_reason, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1 = NONE \
     WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND id = $record AND stopped_at = $claimed_at \
       AND (stop_reason = $claim_reason OR ( \
         stop_reason = $killed_reason AND exit_code = NONE \
         AND $previous_stopped_at != NONE \
         AND string::starts_with($stop_reason, 'reclaim_killed:') \
         AND $stop_reason != $killed_reason \
         AND metadata.__handshake_internal_reclaim_durable_receipt_v1 = { \
             receipt_kind: 'handshake_reclaim_durable_receipt_v1', \
             previous_stopped_at: $previous_stopped_at, previous_stop_reason: $stop_reason \
         } \
     )) RETURN AFTER;";

const SURREAL_ACTIVE_RECLAIM_OUTSTANDING_SESSION_QUERY: &str = "SELECT VALUE id FROM \
     kernel_process_lifecycle WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND parent_session_id = $session_id \
     AND stopped_at = $claimed_at AND (stop_reason = $claim_reason \
         OR (exit_code = NONE AND stop_reason = $killed_reason \
             AND metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stopped_at != NONE \
             AND string::starts_with(metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason, 'reclaim_killed:') \
             AND metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason != $killed_reason \
             AND metadata.__handshake_internal_reclaim_durable_receipt_v1 = { \
                 receipt_kind: 'handshake_reclaim_durable_receipt_v1', \
                 previous_stopped_at: metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stopped_at, \
                 previous_stop_reason: metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason \
             } \
         ));";

const SURREAL_ACTIVE_RECLAIM_DURABLE_RECEIPTS_QUERY: &str = "SELECT id, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1.receipt_kind AS receipt_kind, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stopped_at AS previous_stopped_at, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason AS previous_stop_reason \
     FROM kernel_process_lifecycle WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND parent_session_id = $session_id \
     AND stopped_at = $claimed_at AND (stop_reason = $claim_reason \
         OR (exit_code = NONE AND stop_reason = $killed_reason \
             AND metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stopped_at != NONE \
             AND string::starts_with(metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason, 'reclaim_killed:') \
             AND metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason != $killed_reason \
             AND metadata.__handshake_internal_reclaim_durable_receipt_v1 = { \
                 receipt_kind: 'handshake_reclaim_durable_receipt_v1', \
                 previous_stopped_at: metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stopped_at, \
                 previous_stop_reason: metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason \
             } \
         ));";

const SURREAL_ACTIVE_RECLAIM_SAME_OWNER_QUERY: &str = "SELECT VALUE id FROM \
     kernel_process_lifecycle WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND parent_session_id = $session_id \
     AND exit_code = NONE AND stop_reason = $claim_reason;";

const SURREAL_ACTIVE_RECLAIM_ABANDON_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stopped_at = $previous_stopped_at, stop_reason = $previous_stop_reason, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1 = NONE \
     WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND id = $record AND stopped_at = $claimed_at \
        AND exit_code = NONE AND (stop_reason = $claim_reason OR ( \
         stop_reason = $killed_reason AND exit_code = NONE \
         AND $previous_stopped_at != NONE \
         AND string::starts_with($previous_stop_reason, 'reclaim_killed:') \
         AND $previous_stop_reason != $killed_reason \
         AND metadata.__handshake_internal_reclaim_durable_receipt_v1 = { \
             receipt_kind: 'handshake_reclaim_durable_receipt_v1', \
             previous_stopped_at: $previous_stopped_at, previous_stop_reason: $previous_stop_reason \
         } \
     )) RETURN AFTER;";

const SURREAL_ACTIVE_RECLAIM_CANONICAL_READ_QUERY: &str = "SELECT id, process_uuid, stopped_at, stop_reason, \
     exit_code FROM ONLY $record WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE;";

const RECLAIM_RECEIPT_METADATA_KEY: &str = "__handshake_internal_reclaim_receipt_v1";
const RECLAIM_DURABLE_RECEIPT_METADATA_KEY: &str =
    "__handshake_internal_reclaim_durable_receipt_v1";
const RECLAIM_DURABLE_RECEIPT_KIND: &str = "handshake_reclaim_durable_receipt_v1";

const SURREAL_ACTIVE_RECLAIM_MARK_KILLED_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stop_reason = $killed_reason \
     WHERE owner_account_id = NONE AND actor_principal_id = NONE AND authenticated_session_id = NONE AND access_space_id = NONE AND workspace_id = NONE AND owner_runtime_instance_id = NONE AND owner_host_scope_id = NONE AND owner_lease_schema_id = NONE AND owner_lease_protocol = NONE AND owner_lease_address = NONE AND owner_lease_port = NONE AND reclaim_claimant_uuid = NONE AND reclaim_kill_operation_uuid = NONE AND reclaim_generation = NONE AND reclaim_claimed_at_unix_ms = NONE AND reclaim_lease_expires_at_unix_ms = NONE AND reclaim_state = NONE AND event_ledger_event_id = NONE AND metadata.owner_account_id = NONE AND metadata.actor_principal_id = NONE AND metadata.authenticated_session_id = NONE AND metadata.access_space_id = NONE AND metadata.workspace_id = NONE AND metadata.runtime_instance_id = NONE AND metadata.runtime_host_scope_id = NONE AND metadata.runtime_instance_schema_id = NONE AND metadata.runtime_lease_protocol = NONE AND metadata.runtime_lease_address = NONE AND metadata.runtime_lease_port = NONE AND metadata.reclaim_claim = NONE AND metadata.reclaim_pending_stop = NONE AND metadata.reclaim_last_kill_operation = NONE AND id = $record AND process_uuid = $process_uuid AND stopped_at = $claimed_at \
       AND exit_code = NONE AND stop_reason = $claim_reason RETURN VALUE process_uuid;";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimTrigger {
    Close,
    Failure,
    Stale,
    OperatorCancel,
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
            Self::Stale => "stale",
            Self::OperatorCancel => "operator_cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedReclaimableProcess {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopedReclaimTerminal {
    Exited { code: i32 },
    Killed { by_signal: crate::sandbox::Signal },
}

impl ScopedReclaimTerminal {
    /// `None` is the owner's proof that the process ended without a readable
    /// exit status; see [`ScopedSandboxKill::kill_observed`].
    fn from_observed(
        process: &ScopedReclaimableProcess,
        status: Option<crate::sandbox::ProcessStatus>,
    ) -> Result<Self, KillError> {
        match status {
            None => Ok(Self::Exited { code: RECLAIM_STOP_EXIT_CODE }),
            Some(crate::sandbox::ProcessStatus::Exited { code }) => Ok(Self::Exited { code }),
            Some(crate::sandbox::ProcessStatus::Killed { by_signal })
                if process.sandbox_adapter_id.as_deref()
                    == Some(crate::sandbox::cloud_hypervisor::CLOUD_HYPERVISOR_ADAPTER_ID) =>
                Ok(Self::Killed { by_signal }),
            // Other owners report a signalled end without a numeric code.
            Some(crate::sandbox::ProcessStatus::Killed { .. }) =>
                Ok(Self::Exited { code: RECLAIM_STOP_EXIT_CODE }),
            Some(status) => Err(KillError::new(format!(
                "owning adapter returned no terminal status: {status:?}",
            ))),
        }
    }

    pub(crate) fn from_stop(stop: &ProcessStop) -> Result<Self, ProcessLedgerError> {
        let invalid = || ProcessLedgerError::Store(
            "scoped STOP has no exact supported terminal outcome".into(),
        );
        let pending = stop.metadata_jsonb.get("reclaim_pending_stop")
            .and_then(Value::as_object).ok_or_else(invalid)?;
        match stop.exit_code {
            Some(code) if !pending.contains_key("by_signal")
                && pending.get("exit_code").and_then(Value::as_i64) == Some(i64::from(code)) =>
                Ok(Self::Exited { code }),
            None if stop.sandbox_adapter_id.as_deref()
                == Some(crate::sandbox::cloud_hypervisor::CLOUD_HYPERVISOR_ADAPTER_ID)
                && !pending.contains_key("exit_code") =>
            {
                let by_signal = match pending.get("by_signal").and_then(Value::as_str) {
                    Some("int") => crate::sandbox::Signal::Int,
                    Some("kill") => crate::sandbox::Signal::Kill,
                    Some("term") => crate::sandbox::Signal::Term,
                    _ => return Err(invalid()),
                };
                Ok(Self::Killed { by_signal })
            }
            _ => Err(invalid()),
        }
    }

    pub(crate) fn pending_metadata(self, claim: &ReclaimClaim) -> Value {
        let mut pending = serde_json::json!({
            "stop_reason": "reclaim",
            "claimant_uuid": claim.claimant_uuid,
            "kill_operation_uuid": claim.kill_operation_uuid,
            "generation": claim.generation,
        });
        match self {
            Self::Exited { code } => pending["exit_code"] = serde_json::json!(code),
            Self::Killed { by_signal } => pending["by_signal"] = serde_json::json!(by_signal),
        }
        pending
    }

    fn exit_code(self) -> Option<i32> {
        match self {
            Self::Exited { code } => Some(code),
            Self::Killed { .. } => None,
        }
    }
}

impl ScopedReclaimableProcess {
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

    pub(crate) fn original_pending_stop(&self) -> Result<ProcessStop, ProcessLedgerError> {
        let invalid = || ProcessLedgerError::Store(format!(
            "process {} has no exact immutable pending STOP for its original reclaim claim",
            self.process_uuid));
        let raw = self.metadata_jsonb.get("reclaim_stop_payload").ok_or_else(invalid)?;
        let mut stop: ProcessStop = serde_json::from_value(raw.clone()).map_err(|_| invalid())?;
        if serde_json::to_value(&stop).map_err(|_| invalid())? != *raw {
            return Err(invalid());
        }
        let terminal = ScopedReclaimTerminal::from_stop(&stop)?;
        let mut expected = self.reclaim_stop_terminal(terminal);
        expected.stopped_at = stop.stopped_at;
        expected.metadata_jsonb.as_object_mut().ok_or_else(invalid)?.remove("reclaim_stop_payload");
        if expected != stop { return Err(invalid()); }
        stop.metadata_jsonb.as_object_mut().ok_or_else(invalid)?
            .insert("reclaim_stop_payload".into(), raw.clone());
        Ok(stop)
    }

    pub fn reclaim_stop(&self, exit_code: i32) -> ProcessStop {
        self.reclaim_stop_terminal(ScopedReclaimTerminal::Exited { code: exit_code })
    }

    fn reclaim_stop_terminal(&self, terminal: ScopedReclaimTerminal) -> ProcessStop {
        let mut metadata_jsonb = self.metadata_jsonb.clone();
        if let Some(metadata) = metadata_jsonb.as_object_mut() {
            metadata.insert(
                "reclaim_claim".to_owned(),
                serde_json::json!(self.reclaim_claim),
            );
            metadata.insert(
                "reclaim_pending_stop".to_string(),
                terminal.pending_metadata(&self.reclaim_claim),
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
            exit_code: terminal.exit_code(),
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
            reclaim_claimed_at: None,
            reclaim_expected_reason: None,
            reclaim_expected_killed_reason: None,
        }
    }
}


fn pack_scoped_pending_stop(
    process: &ScopedReclaimableProcess, terminal: ScopedReclaimTerminal,
) -> Result<ProcessStop, ProcessLedgerError> {
    if process.metadata_jsonb.get("reclaim_stop_payload").is_some() {
        let original = process.original_pending_stop()?;
        if ScopedReclaimTerminal::from_stop(&original)? != terminal {
            return Err(ProcessLedgerError::Store("original terminal STOP outcome changed".into()));
        }
        return Ok(original);
    }
    let mut stop = process.reclaim_stop_terminal(terminal);
    let payload = serde_json::to_value(&stop)
        .map_err(|error| ProcessLedgerError::Store(error.to_string()))?;
    stop.metadata_jsonb.as_object_mut()
        .ok_or_else(|| ProcessLedgerError::Store("scoped STOP metadata is not an object".into()))?
        .insert("reclaim_stop_payload".into(), payload);
    let mut original = process.clone();
    original.metadata_jsonb = stop.metadata_jsonb.clone();
    original.original_pending_stop()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimableProcess {
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    pub parent_session_id: String,
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
    pub sandbox_capabilities_snapshot: serde_json::Value,
    pub metadata_jsonb: serde_json::Value,
    /// Exact timestamp written by the atomic reclaim claim. The terminal STOP
    /// must present this stamp before it may replace the durable sentinel.
    pub reclaim_claimed_at: DateTime<Utc>,
    /// Exact owner-qualified sentinel reason that must still be durable when
    /// the terminal reclaim STOP is applied.
    pub reclaim_expected_reason: String,
    /// Exact owner-qualified cleanup-completed sentinel that may replace the
    /// claim reason before terminalization.
    pub reclaim_expected_killed_reason: String,
    /// `true` when this claim re-acquired a durable post-cleanup marker from a
    /// prior attempt. The external cleanup must not be repeated; only the exact
    /// terminal STOP remains.
    pub reclaim_cleanup_completed: bool,
}

impl ReclaimableProcess {
    fn reclaim_receipt(&self) -> Result<ReclaimPriorState, ProcessLedgerError> {
        let receipt = self
            .metadata_jsonb
            .as_object()
            .and_then(|metadata| metadata.get(RECLAIM_RECEIPT_METADATA_KEY))
            .ok_or_else(|| {
                ProcessLedgerError::Store(format!(
                    "missing exact reclaim receipt for process {}",
                    self.process_uuid
                ))
            })?;
        serde_json::from_value(receipt.clone()).map_err(|error| {
            ProcessLedgerError::Store(format!(
                "invalid exact reclaim receipt for process {}: {error}",
                self.process_uuid
            ))
        })
    }

    fn metadata_without_reclaim_receipt(&self) -> serde_json::Value {
        let mut metadata = self.metadata_jsonb.clone();
        if let Some(object) = metadata.as_object_mut() {
            let is_our_receipt = object
                .get(RECLAIM_RECEIPT_METADATA_KEY)
                .and_then(|value| serde_json::from_value::<ReclaimPriorState>(value.clone()).ok())
                .is_some_and(|receipt| receipt.record_process_uuid == self.process_uuid);
            if is_our_receipt {
                object.remove(RECLAIM_RECEIPT_METADATA_KEY);
            }
        }
        metadata
    }

    pub fn reclaim_stop(&self, exit_code: i32) -> ProcessStop {
        ProcessStop {
            process_uuid: self.process_uuid,
            os_pid: self.os_pid,
            parent_session_id: Some(self.parent_session_id.clone()),
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
            runtime_owner: None,
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot.clone(),
            metadata_jsonb: self.metadata_without_reclaim_receipt(),
            reclaim_claimed_at: Some(self.reclaim_claimed_at),
            reclaim_expected_reason: Some(self.reclaim_expected_reason.clone()),
            reclaim_expected_killed_reason: Some(self.reclaim_expected_killed_reason.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReclaimPriorState {
    record: RecordId,
    record_process_uuid: Uuid,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillFailureKind {
    /// The kill returned without proof that the process ended.
    Uncertain,
    /// The owning provider refused before it touched the process.
    BeforeAdapterInvocation,
    /// The observation window closed while the original kill still runs.
    StillRunning,
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct KillError {
    message: String,
    kind: KillFailureKind,
    identity_lookup_cause: Option<Arc<std::sync::Mutex<Option<ProcessLedgerError>>>>,
}

impl PartialEq for KillError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message && self.kind == other.kind
    }
}
impl Eq for KillError {}

impl KillError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: KillFailureKind::Uncertain,
            identity_lookup_cause: None,
        }
    }

    fn identity_lookup(error: ProcessLedgerError) -> Self {
        Self {
            message: error.to_string(),
            kind: KillFailureKind::Uncertain,
            identity_lookup_cause: Some(Arc::new(std::sync::Mutex::new(Some(error)))),
        }
    }

    fn proved_before_adapter(mut self) -> Self {
        self.kind = KillFailureKind::BeforeAdapterInvocation;
        self
    }

    /// The owning provider proves it refused before invoking its adapter.
    pub fn before_adapter_invocation(message: impl Into<String>) -> Self {
        Self {
            message: message.into(), kind: KillFailureKind::BeforeAdapterInvocation,
            identity_lookup_cause: None,
        }
    }

    fn still_running(message: impl Into<String>) -> Self {
        Self {
            message: message.into(), kind: KillFailureKind::StillRunning,
            identity_lookup_cause: None,
        }
    }

    fn from_detached_reclaim(error: crate::sandbox::DetachedReclaimError) -> Self {
        match error {
            crate::sandbox::DetachedReclaimError::BeforeInvocation(error) =>
                Self::before_adapter_invocation(error.to_string()),
            crate::sandbox::DetachedReclaimError::AfterInvocation(error) =>
                Self::new(error.to_string()),
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
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedProcess {
    pub process_uuid: Uuid,
    pub engine_kind: ProcessEngineKind,
    pub sandbox_adapter_id: Option<String>,
    pub kill_result: KillOutcome,
    pub stop_event_kind: LedgerEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimReport {
    pub session_id: String,
    pub trigger: ReclaimTrigger,
    pub processes_reclaimed: Vec<ReclaimedProcess>,
    pub total_duration_ms: u128,
}

pub struct ScopedReclaimRenewal {
    pub(crate) deadline: time::Instant,
    pub(crate) original: std::pin::Pin<Box<dyn std::future::Future<
        Output = Result<ReclaimClaim, ProcessLedgerError>,
    > + Send + 'static>>,
}

#[async_trait]
pub trait ScopedReclaimProcessStore: Send + Sync + 'static {
    async fn active_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
    ) -> Result<Vec<ScopedReclaimableProcess>, ProcessLedgerError>;

    async fn active_process_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        process_uuid: Uuid,
    ) -> Result<Option<ScopedReclaimableProcess>, ProcessLedgerError>;

    async fn active_owned_process(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        owner_runtime_instance_id: Uuid,
    ) -> Result<Option<ScopedReclaimableProcess>, ProcessLedgerError>;

    async fn active_foreign_owner_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ScopedReclaimableProcess>, ProcessLedgerError>;

    async fn active_stale_owned_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ScopedReclaimableProcess>, ProcessLedgerError>;

    async fn renew_reclaim_claim(
        &self,
        process: &ScopedReclaimableProcess,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError>;

    fn retained_reclaim_renewal(
        &self, _process: &ScopedReclaimableProcess, _claim: &ReclaimClaim,
        _deadline: time::Instant,
    ) -> Option<ScopedReclaimRenewal> {
        None
    }

    async fn mark_reclaim_kill_started(
        &self,
        process: &ScopedReclaimableProcess,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn mark_reclaim_kill_succeeded(
        &self,
        process: &ScopedReclaimableProcess,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    /// Verify a submitted original STOP against its final lifecycle and full receipt.
    async fn verify_submitted_stop(
        &self, _process: &ScopedReclaimableProcess, _stop: &ProcessStop,
    ) -> Result<(), ProcessLedgerError> {
        Err(ProcessLedgerError::Store("exact submitted STOP readback is unsupported".into()))
    }

    /// Read the full pending payload through the canonical same-claim marker.
    /// A caller-supplied metadata object alone is not durable evidence.
    async fn verified_pending_stop(
        &self, _process: &ScopedReclaimableProcess,
    ) -> Result<ProcessStop, ProcessLedgerError> {
        Err(ProcessLedgerError::Store("canonical pending STOP readback is unsupported".into()))
    }

    /// Release only a canonically unstarted exact claim. Unsupported stores retain it.
    async fn release_unstarted_reclaim_claim(
        &self, _process: &ScopedReclaimableProcess, _claim: &ReclaimClaim,
    ) -> Result<bool, ProcessLedgerError> {
        Ok(false)
    }

    /// Release only with the caller's explicit proof of no adapter invocation.
    async fn release_proved_uninvoked_claim(
        &self, _process: &ScopedReclaimableProcess, _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Err(ProcessLedgerError::Store("proved uninvoked claim release is unsupported".into()))
    }

    async fn release_reclaim_claim(
        &self,
        process: &ScopedReclaimableProcess,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError>;

    async fn resolve_reclaim_kill_operation(
        &self,
        process: &ScopedReclaimableProcess,
        authorized_process_uuids: &[Uuid],
        excluded_owner_runtime_instance_id: Option<Uuid>,
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
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError>;

    async fn claimed_recovery_process_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<Option<ScopedReclaimableProcess>, ProcessLedgerError>;

    async fn claimed_recovery_process_for_stale_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<Option<ScopedReclaimableProcess>, ProcessLedgerError>;
    async fn resolve_recovery_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        let process = self
            .claimed_recovery_process_for_session(
                resource_scope,
                session_id,
                excluded_owner_runtime_instance_id,
                authorized_process_uuids,
                process_uuid,
                kill_operation_uuid,
            )
            .await?
            .ok_or_else(|| {
                ProcessLedgerError::Store("canonical scoped recovery claim unavailable".to_owned())
            })?;
        self.resolve_reclaim_kill_operation(
            &process,
            authorized_process_uuids,
            Some(excluded_owner_runtime_instance_id),
            status,
        )
        .await
    }

    async fn resolve_recovery_for_stale_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        let process = self
            .claimed_recovery_process_for_stale_owner(
                resource_scope,
                session_id,
                owner_runtime_instance_id,
                owner_host_scope_id,
                authorized_process_uuids,
                process_uuid,
                kill_operation_uuid,
            )
            .await?
            .ok_or_else(|| {
                ProcessLedgerError::Store(
                    "canonical stale-owner recovery claim unavailable".to_owned(),
                )
            })?;
        self.resolve_reclaim_kill_operation(&process, authorized_process_uuids, None, status)
            .await
    }
}

#[async_trait]
pub trait ReclaimProcessStore: Send + Sync + 'static {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError>;

    /// Durably records that external cleanup succeeded before terminal STOP.
    /// A later claim may then finish the ledger transition without repeating
    /// the external kill/cleanup side effect.
    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError>;

    /// Release claims whose external cleanup has not been durably recorded, so
    /// the same boot can retry instead of waiting for process restart.
    async fn abandon(&self, processes: &[ReclaimableProcess]) -> Result<(), ProcessLedgerError>;
}

pub trait SandboxKill: Send + Sync + 'static {
    fn kill(&self, process_uuid: Uuid) -> Result<(), KillError>;
}


#[repr(u8)]
enum ScopedKillDispatchState { Ready, EffectAdmitted, CancelledBeforeEffect }

#[derive(Clone)]
enum ProductionIdentityReadContext {
    Current,
    Captured(crate::storage::surreal::ReclaimStorageContext),
    Unavailable,
}

#[derive(Clone)]
pub struct ScopedKillDispatchObservation(
    Arc<std::sync::atomic::AtomicU8>,
    ProductionIdentityReadContext,
);

impl ScopedKillDispatchObservation {
    fn new() -> Self {
        Self(
            Arc::new(std::sync::atomic::AtomicU8::new(ScopedKillDispatchState::Ready as u8)),
            ProductionIdentityReadContext::Current,
        )
    }

    fn capture_identity_context(deadline: time::Instant) -> Self {
        let mut dispatch = Self::new();
        dispatch.1 = match crate::storage::surreal::capture_reclaim_storage_context(deadline.into_std()) {
            Some(context) => ProductionIdentityReadContext::Captured(context),
            None => ProductionIdentityReadContext::Unavailable,
        };
        dispatch
    }

    /// Atomically admit possible effect or return a pre-adapter refusal; never authority.
    pub fn mark_possible_effect(&self) -> Result<(), KillError> {
        match self.0.compare_exchange(
            ScopedKillDispatchState::Ready as u8, ScopedKillDispatchState::EffectAdmitted as u8,
            std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(state) if state == ScopedKillDispatchState::EffectAdmitted as u8 => Ok(()),
            Err(_) => Err(KillError::before_adapter_invocation(
                "original reclaim deadline cancelled dispatch before adapter effect",
            )),
        }
    }

    fn cancel_before_effect(&self) -> bool {
        self.0.compare_exchange(
            ScopedKillDispatchState::Ready as u8, ScopedKillDispatchState::CancelledBeforeEffect as u8,
            std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Acquire,
        ).is_ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopedKillRecoveryObservation {
    Succeeded { code: i32 },
    SucceededBySignal { by_signal: crate::sandbox::Signal },
    Failed,
    NotStarted,
    InProgress,
    Unknown,
}

#[async_trait]
pub trait ScopedSandboxKill: Send + Sync + 'static {
    /// Execute or coalesce one stable kill operation. Implementations must use
    /// `kill_operation_uuid` as their idempotency key across retries. `Ok`
    /// asserts that the process has ended; an unproven end is an error.
    async fn kill(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<(), KillError>;

    /// [`Self::kill`] with the terminal status the owner observed. `Ok(None)`
    /// asserts that the process has ended while no exit status can be read.
    async fn kill_observed(
        &self, resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid, kill_operation_uuid: Uuid,
    ) -> Result<Option<crate::sandbox::ProcessStatus>, KillError> {
        self.kill(resource_scope, process_uuid, kill_operation_uuid).await?;
        Ok(None)
    }

    async fn kill_observed_with_dispatch(
        &self, resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid, kill_operation_uuid: Uuid,
        dispatch: ScopedKillDispatchObservation,
    ) -> Result<Option<crate::sandbox::ProcessStatus>, KillError> {
        dispatch.mark_possible_effect()?;
        self.kill_observed(resource_scope, process_uuid, kill_operation_uuid).await
    }

    async fn recovery_observation(
        &self, resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid, kill_operation_uuid: Uuid,
    ) -> Result<ScopedKillRecoveryObservation, KillError> {
        Ok(match self.kill_operation_status(resource_scope, process_uuid, kill_operation_uuid).await? {
            // The owner's record proves the end of the process, not its exit code.
            ReclaimKillOperationStatus::Succeeded => ScopedKillRecoveryObservation::Succeeded {
                code: RECLAIM_STOP_EXIT_CODE,
            },
            ReclaimKillOperationStatus::Failed => ScopedKillRecoveryObservation::Failed,
            ReclaimKillOperationStatus::NotStarted => ScopedKillRecoveryObservation::NotStarted,
            ReclaimKillOperationStatus::InProgress => ScopedKillRecoveryObservation::InProgress,
            ReclaimKillOperationStatus::Unknown => ScopedKillRecoveryObservation::Unknown,
        })
    }

    /// Query the adapter's authoritative idempotency record for crash recovery.
    async fn kill_operation_status(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError>;
}

const RECLAIM_KILL_TIMEOUT: Duration = Duration::from_secs(30);

/// Production process killer for ledger rows whose owning adapter exposes an
/// OS process identity. The ledger's immutable executable hash and launch
/// process-generation identity are checked before termination, so a reused PID
/// cannot redirect stale reclaim to another process generation.
#[derive(Clone)]
pub struct ProductionSandboxKill {
    storage: SurrealStorage,
    sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    host_scope_id: Option<String>,
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
            host_scope_id: None,
        }
    }

    /// Name the host this killer runs on. Only a process recorded by the same
    /// host can be proven absent here; without a host scope no absence counts.
    pub(crate) fn with_host_scope(mut self, host_scope_id: impl Into<String>) -> Self {
        self.host_scope_id = Some(host_scope_id.into());
        self
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
            KillError::identity_lookup(ProcessLedgerError::Store(format!(
                "production reclaim identity lookup timed out for process {process_uuid}"
            )))
        })?
        .map_err(KillError::identity_lookup)
    }

    fn current_identity_context() -> Result<crate::storage::surreal::ReclaimStorageContext, KillError> {
        crate::storage::surreal::capture_reclaim_storage_context(
            (time::Instant::now() + RECLAIM_KILL_TIMEOUT).into_std(),
        ).ok_or_else(|| KillError::before_adapter_invocation(
            "production reclaim cannot transfer protected storage authority into native dispatch",
        ))
    }

    async fn identity_with_context(
        &self, resource_scope: &ReclaimResourceScope, process_uuid: Uuid,
        context: crate::storage::surreal::ReclaimStorageContext,
    ) -> Result<ProductionKillIdentity, KillError> {
        let deadline = time::Instant::from_std(context.deadline);
        context.scope(async {
            tokio::time::timeout_at(
                deadline, load_production_kill_identity(&self.storage, resource_scope, process_uuid),
            ).await.map_err(|_| ProcessLedgerError::Store(format!(
                "production reclaim identity lookup timed out for process {process_uuid}",
            )))?
        }).await.map_err(KillError::identity_lookup)
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
    owner_host_scope_id: Option<String>,
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
    stop_reason: Option<String>,
    reclaim_state: Option<String>,
    reclaim_kill_operation_uuid: Option<Uuid>,
    os_pid: Option<i64>,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    owner_host_scope_id: Option<String>,
    metadata: Value,
}

const LOAD_PRODUCTION_KILL_IDENTITY: &str = r#"
SELECT process_uuid, os_pid, started_at, stopped_at, sandbox_adapter_id,
    sandbox_internal_id, owner_host_scope_id, metadata, stop_reason, reclaim_state,
    reclaim_kill_operation_uuid
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
        record: RecordId::new(super::PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
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
    let last_operation_uuid = metadata
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
    let current_claim_operation = metadata.get("reclaim_claim")
        .and_then(|claim| claim.get("kill_operation_uuid"))
        .and_then(Value::as_str).and_then(|value| Uuid::parse_str(value).ok());
    let last_status = metadata.get("reclaim_last_kill_operation")
        .and_then(|operation| operation.get("status")).and_then(Value::as_str);
    let kill_operation_uuid = if row.stopped_at.is_none()
        && row.stop_reason.as_deref() == Some("reclaim_kill_in_progress")
        && row.reclaim_state.as_deref() == Some("kill_in_progress")
        && row.reclaim_kill_operation_uuid.is_some_and(|operation| !operation.is_nil())
        && current_claim_operation == row.reclaim_kill_operation_uuid
        && last_operation_uuid == row.reclaim_kill_operation_uuid
        && last_status == Some("in_progress")
    {
        row.reclaim_kill_operation_uuid
    } else { None };
    let os_pid = row.os_pid.map(os_pid_to_u32).transpose()?;
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
        owner_host_scope_id: row.owner_host_scope_id,
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
impl ScopedSandboxKill for ProductionSandboxKill {
    async fn kill_observed(
        &self, resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid, kill_operation_uuid: Uuid,
    ) -> Result<Option<crate::sandbox::ProcessStatus>, KillError> {
        self.kill_observed_with_dispatch(
            resource_scope, process_uuid, kill_operation_uuid, ScopedKillDispatchObservation::new(),
        ).await
    }

    async fn kill_observed_with_dispatch(
        &self, resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid, kill_operation_uuid: Uuid,
        dispatch: ScopedKillDispatchObservation,
    ) -> Result<Option<crate::sandbox::ProcessStatus>, KillError> {
        let context = match &dispatch.1 {
            ProductionIdentityReadContext::Current => Self::current_identity_context()?,
            ProductionIdentityReadContext::Captured(context) => context.clone(),
            ProductionIdentityReadContext::Unavailable => return Err(KillError::before_adapter_invocation(
                "production reclaim original storage authority could not be captured",
            )),
        };
        let deadline = context.deadline;
        let identity = self.identity_with_context(resource_scope, process_uuid, context).await
            .map_err(KillError::proved_before_adapter)?;
        if identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Err(KillError::before_adapter_invocation("detached result does not match the original reclaim operation"));
        }
        if identity.stopped {
            return Err(KillError::before_adapter_invocation(
                "process already has a durable STOP; nothing was signalled",
            ));
        }
        let adapter = self.owning_adapter(&identity)
            .map_err(|error| KillError::before_adapter_invocation(error.message().to_owned()))?;
        if std::time::Instant::now() >= deadline {
            return Err(KillError::before_adapter_invocation(
                "production reclaim original deadline expired before adapter effect",
            ));
        }
        dispatch.mark_possible_effect()?;
        match adapter.reclaim_detached_observed(
            &identity.detached, crate::sandbox::Signal::Kill,
        ).await.map_err(KillError::from_detached_reclaim)? {
            crate::sandbox::DetachedReclaimOutcome::Observed(status) => Ok(Some(status)),
            crate::sandbox::DetachedReclaimOutcome::Terminated => Ok(None),
            // The owner found no such process. That ends the recorded process
            // only when the row was recorded by this host.
            crate::sandbox::DetachedReclaimOutcome::AlreadyAbsent
                if self.host_scope_id.is_some()
                    && identity.owner_host_scope_id == self.host_scope_id => Ok(None),
            crate::sandbox::DetachedReclaimOutcome::AlreadyAbsent =>
                Err(KillError::before_adapter_invocation(format!(
                    "PROCESS_ABSENT_HOST_UNPROVEN: process {process_uuid} does not exist on this host, which is not proven to be its recorded host",
                ))),
        }
    }

    async fn recovery_observation(
        &self, resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid, kill_operation_uuid: Uuid,
    ) -> Result<ScopedKillRecoveryObservation, KillError> {
        let identity = self.identity(resource_scope, process_uuid).await?;
        if identity.stopped || identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Ok(ScopedKillRecoveryObservation::Unknown);
        }
        let adapter = self.owning_adapter(&identity)?;
        let observed = tokio::time::timeout(
            Duration::from_secs(10), adapter.detached_status(&identity.detached),
        ).await.map_err(|_| KillError::new("owning combined recovery observation timed out"))?;
        let status = observed.map_err(|error| KillError::new(error.to_string()))?;
        match status {
            crate::sandbox::ProcessStatus::Running => Ok(ScopedKillRecoveryObservation::InProgress),
            crate::sandbox::ProcessStatus::Exited { code }
                if identity.detached.handle.adapter_id ==
                    crate::sandbox::AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
                || identity.detached.handle.adapter_id == crate::sandbox::AdapterId::new(
                    crate::sandbox::palmistry_watcher::PALMISTRY_WATCHER_ADAPTER_ID) =>
                Ok(ScopedKillRecoveryObservation::Succeeded { code }),
            observed @ (crate::sandbox::ProcessStatus::Exited { .. }
                | crate::sandbox::ProcessStatus::Killed { .. })
                if identity.detached.handle.adapter_id == crate::sandbox::AdapterId::new(
                    crate::sandbox::cloud_hypervisor::CLOUD_HYPERVISOR_ADAPTER_ID) =>
            {
                match adapter.cached_settled_detached_terminal(&identity.detached)
                    .map_err(|error| KillError::new(error.to_string()))?
                {
                    Some(cached) if cached == observed => match cached {
                        crate::sandbox::ProcessStatus::Exited { code } =>
                            Ok(ScopedKillRecoveryObservation::Succeeded { code }),
                        crate::sandbox::ProcessStatus::Killed { by_signal } =>
                            Ok(ScopedKillRecoveryObservation::SucceededBySignal { by_signal }),
                        _ => Err(KillError::new("owning combined recovery has no supported settled terminal outcome")),
                    },
                    _ => Err(KillError::new("owning combined recovery has no supported settled terminal outcome")),
                }
            }
            crate::sandbox::ProcessStatus::Exited { .. } | crate::sandbox::ProcessStatus::Killed { .. } =>
                Err(KillError::new("owning combined recovery has no supported verified terminal numeric code")),
            crate::sandbox::ProcessStatus::Orphaned => Ok(ScopedKillRecoveryObservation::Unknown),
            crate::sandbox::ProcessStatus::FailedToStart { .. } => Ok(ScopedKillRecoveryObservation::Unknown),
        }
    }

    async fn kill(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<(), KillError> {
        self.kill_observed(resource_scope, process_uuid, kill_operation_uuid).await.map(|_| ())
    }

    async fn kill_operation_status(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        let identity = self.identity(resource_scope, process_uuid).await?;
        if identity.stopped || identity.kill_operation_uuid != Some(kill_operation_uuid) {
            return Ok(ReclaimKillOperationStatus::Unknown);
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
            | crate::sandbox::ProcessStatus::Killed { .. } => ReclaimKillOperationStatus::Succeeded,
            crate::sandbox::ProcessStatus::Orphaned => ReclaimKillOperationStatus::Unknown,
            crate::sandbox::ProcessStatus::FailedToStart { .. } => {
                ReclaimKillOperationStatus::Unknown
            }
        })
    }
}

#[async_trait]
pub trait ScopedReclaimStopReservation: Send + 'static {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError>;

    /// Internal owner retains this exact observation across caller deadlines.
    /// External implementations preserve their existing persistence contract.
    async fn persist_owned(
        self: Box<Self>, stop: ProcessStop, timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.persist(stop, timeout).await
    }

    async fn persist_confirmed(
        self: Box<Self>, stop: ProcessStop, timeout: Duration,
        _confirmation: ScopedStopConfirmation,
    ) -> Result<(), ProcessLedgerError> {
        self.persist_owned(stop, timeout).await
    }
}

pub trait ScopedReclaimStopWriter: Send + Sync + 'static {
    fn reserve_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ScopedReclaimStopReservation>, ProcessLedgerError>;

    /// Admit a fresh native reclaim before effect; custom sinks retain their contract.
    fn reserve_fresh_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ScopedReclaimStopReservation>, ProcessLedgerError> {
        self.reserve_reclaim_stop()
    }
}


pub struct ScopedStopConfirmation {
    owner: std::sync::Weak<ScopedStopSubmission>,
}

impl ScopedStopConfirmation {
    fn original_owner(&self, stop: &ProcessStop) -> Result<Arc<ScopedStopSubmission>, ProcessLedgerError> {
        let owner = self.owner.upgrade().ok_or_else(|| ProcessLedgerError::InvalidConfig(
            "original scoped STOP confirmation owner is unavailable".into(),
        ))?;
        if owner.stop != *stop || !owner.submitting.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ProcessLedgerError::InvalidConfig(
                "scoped STOP confirmation does not match its original submitted payload".into(),
            ));
        }
        Ok(owner)
    }

    pub(crate) fn validate(&self, stop: &ProcessStop) -> Result<(), ProcessLedgerError> {
        self.original_owner(stop).map(|_| ())
    }

    pub(crate) fn publish(self, stop: &ProcessStop) {
        if let Ok(owner) = self.original_owner(stop) {
            owner.publish_durable();
        }
    }
}

type ScopedStopFuture = std::pin::Pin<Box<dyn std::future::Future<Output = (bool, Result<(), ProcessLedgerError>)> + Send>>;
struct ScopedStopSubmission {
    process: ScopedReclaimableProcess,
    store: Arc<dyn ScopedReclaimProcessStore>,
    writer: Arc<dyn ScopedReclaimStopWriter>,
    stop: ProcessStop,
    kill_fence: Option<std::sync::Weak<ProcessKillFence>>,
    submitting: Arc<std::sync::atomic::AtomicBool>,
    durable_positive: std::sync::atomic::AtomicBool,
    durable_ready: tokio::sync::Notify,
    state: tokio::sync::Mutex<ScopedStopSubmissionState>,
}
impl ScopedStopSubmission {
    fn publish_durable(self: &Arc<Self>) {
        self.durable_positive.store(true, std::sync::atomic::Ordering::Release);
        self.durable_ready.notify_waiters();
        {
            let mut owners = SCOPED_STOP_SUBMISSIONS.get_or_init(Default::default)
                .lock().unwrap_or_else(|error| error.into_inner());
            owners.retain(|other| !Arc::ptr_eq(other, self));
        }
        if let Some(fence) = self.kill_fence.as_ref().and_then(std::sync::Weak::upgrade) {
            clear_process_kill_fence(self.process.process_uuid, &fence);
        }
    }

    async fn verify_and_publish(self: &Arc<Self>) -> Result<ScopedReclaimableProcess, ProcessLedgerError> {
        let mut original = self.process.clone();
        original.metadata_jsonb = self.stop.metadata_jsonb.clone();
        if original.original_pending_stop()? != self.stop {
            return Err(ProcessLedgerError::Store("original submitted scoped STOP claim changed".into()));
        }
        self.store.verify_submitted_stop(&original, &self.stop).await?;
        self.publish_durable();
        Ok(original)
    }

    fn matches(&self, process: &ScopedReclaimableProcess, store: &Arc<dyn ScopedReclaimProcessStore>,
        writer: &Arc<dyn ScopedReclaimStopWriter>) -> bool {
        Arc::ptr_eq(&self.store, store) && Arc::ptr_eq(&self.writer, writer)
            && self.process.reclaim_claim == process.reclaim_claim
            && self.process.resource_scope == process.resource_scope
            && self.process.process_uuid == process.process_uuid
            && self.process.runtime_owner == process.runtime_owner
            && self.process.started_at == process.started_at
            && self.process.os_pid == process.os_pid
            && self.process.sandbox_adapter_id == process.sandbox_adapter_id
            && self.process.sandbox_internal_id == process.sandbox_internal_id
            && self.process.parent_session_id == process.parent_session_id
    }
}
struct ScopedStopSubmissionState {
    original: Option<ScopedStopFuture>,
    submitted: bool,
    outcome: Option<Result<(), Arc<ProcessLedgerError>>>,
}
static SCOPED_STOP_SUBMISSIONS: OnceLock<std::sync::Mutex<Vec<Arc<ScopedStopSubmission>>>> = OnceLock::new();

fn scoped_stop_submission(
    process: &ScopedReclaimableProcess, store: &Arc<dyn ScopedReclaimProcessStore>,
    writer: &Arc<dyn ScopedReclaimStopWriter>,
) -> Result<Option<Arc<ScopedStopSubmission>>, ProcessLedgerError> {
    let owners = SCOPED_STOP_SUBMISSIONS.get_or_init(Default::default)
        .lock().unwrap_or_else(|error| error.into_inner());
    let Some(owner) = owners.iter().find(|owner|
        owner.process.process_uuid == process.process_uuid
        && owner.process.resource_scope == process.resource_scope) else { return Ok(None); };
    if !owner.matches(process, store, writer) {
        return Err(ProcessLedgerError::Store("retained scoped STOP submission owner mismatch".into()));
    }
    Ok(Some(Arc::clone(owner)))
}

async fn persist_scoped_stop(
    process: &ScopedReclaimableProcess, store: &Arc<dyn ScopedReclaimProcessStore>,
    writer: &Arc<dyn ScopedReclaimStopWriter>,
    reservation: Option<Box<dyn ScopedReclaimStopReservation>>,
    kill_fence: Option<Arc<ProcessKillFence>>,
    stop: ProcessStop, timeout: Duration,
) -> Result<(), ProcessLedgerError> {
    let existing = scoped_stop_submission(process, store, writer)?;
    let owner = if let Some(owner) = existing {
        drop(reservation);
        if owner.stop != stop {
            return Err(ProcessLedgerError::Store("retained scoped STOP payload changed".into()));
        }
        owner
    } else {
        match kill_fence.as_ref() {
            Some(fence) if !fence.matches(process, store, &fence.killer, writer) =>
                return Err(ProcessLedgerError::Store("original reclaim STOP permit owner mismatch".into())),
            None if reservation.is_none() => return Err(ProcessLedgerError::Store(
                "original scoped STOP reservation unavailable".into())),
            _ => {}
        }
        let mut pending = process.clone();
        pending.kill_succeeded_pending_stop = true;
        pending.metadata_jsonb = stop.metadata_jsonb.clone();
        let verification_store = Arc::clone(store);
        let expected_stop = stop.clone();
        let submitting = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let submission_marker = Arc::clone(&submitting);
        let owner = Arc::new_cyclic(|owner| {
            let confirmation = ScopedStopConfirmation { owner: owner.clone() };
            ScopedStopSubmission {
            process: process.clone(), store: Arc::clone(store), writer: Arc::clone(writer),
            stop: stop.clone(), kill_fence: kill_fence.as_ref().map(Arc::downgrade), submitting,
            durable_positive: std::sync::atomic::AtomicBool::new(false),
            durable_ready: tokio::sync::Notify::new(),
            state: tokio::sync::Mutex::new(ScopedStopSubmissionState {
                original: Some(Box::pin(async move {
                    let verified = match verification_store.verified_pending_stop(&pending).await {
                        Ok(verified) => verified,
                        Err(error) => return (false, Err(error)),
                    };
                    if verified != expected_stop {
                        return (false, Err(ProcessLedgerError::Store("canonical original scoped STOP payload changed".into())));
                    }
                    let retained = kill_fence.and_then(|fence| fence.stop_permit.lock()
                        .unwrap_or_else(|error| error.into_inner()).take());
                    let Some(reservation) = retained.or(reservation) else {
                        return (false, Err(ProcessLedgerError::Store(
                            "original reclaim STOP permit remains unavailable".into())));
                    };
                    submission_marker.store(true, std::sync::atomic::Ordering::Release);
                    (true, reservation.persist_confirmed(stop, timeout, confirmation).await)
                })), submitted: false, outcome: None,
            }),
            }
        });
        let mut owners = SCOPED_STOP_SUBMISSIONS.get_or_init(Default::default)
            .lock().unwrap_or_else(|error| error.into_inner());
        if let Some(other) = owners.iter().find(|other| other.process.process_uuid == process.process_uuid
            && other.process.resource_scope == process.resource_scope) {
            if !other.matches(process, store, writer) || other.stop != owner.stop {
                return Err(ProcessLedgerError::Store("scoped STOP submission registration owner mismatch".into()));
            }
            Arc::clone(other)
        } else {
            owners.push(Arc::clone(&owner));
            owner
        }
    };
    let deadline = tokio::time::Instant::now() + timeout;
    let observation = async {
        loop {
            let ready = owner.durable_ready.notified();
            tokio::pin!(ready);
            ready.as_mut().enable();
            if owner.durable_positive.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(());
            }
            tokio::select! {
                _ = &mut ready => {}
                result = async {
        let mut state = owner.state.lock().await;
        if state.outcome.is_none() {
            let original = state.original.as_mut().ok_or_else(||
                ProcessLedgerError::Store("original scoped STOP observation unavailable".into()))?;
            let (submitted, result) = original.await;
            state.submitted = submitted;
            state.outcome = Some(result.map_err(Arc::new));
            state.original.take();
        }
        if !state.submitted && state.outcome.as_ref().is_some_and(Result::is_err) {
            let mut owners = SCOPED_STOP_SUBMISSIONS.get_or_init(Default::default)
                .lock().unwrap_or_else(|error| error.into_inner());
            owners.retain(|other| !Arc::ptr_eq(other, &owner));
        }
        match state.outcome.as_ref().expect("retained scoped STOP verdict") {
            Ok(()) => {
                let mut owners = SCOPED_STOP_SUBMISSIONS.get_or_init(Default::default)
                    .lock().unwrap_or_else(|error| error.into_inner());
                owners.retain(|other| !Arc::ptr_eq(other, &owner));
                Ok(())
            },
            Err(error) => match error.as_ref() {
                ProcessLedgerError::DurabilityAckLost { event_kind, process_uuid } =>
                    Err(ProcessLedgerError::DurabilityAckLost { event_kind: event_kind.clone(), process_uuid: *process_uuid }),
                ProcessLedgerError::DurabilityRejected { event_kind, process_uuid, reason } =>
                    Err(ProcessLedgerError::DurabilityRejected { event_kind: event_kind.clone(), process_uuid: *process_uuid, reason: reason.clone() }),
                _ => Err(ProcessLedgerError::Store(format!("original scoped STOP submission remains failed: {error}"))),
            },
        }
                } => {
                    return if owner.durable_positive.load(std::sync::atomic::Ordering::Acquire) {
                        Ok(())
                    } else { result };
                }
            }
        }
    };
    let observed = tokio::time::timeout_at(deadline, observation).await;
    if owner.durable_positive.load(std::sync::atomic::Ordering::Acquire) {
        return Ok(());
    }
    match observed {
        Ok(Err(error @ ProcessLedgerError::DurabilityAckLost { .. })) => {
            let matching_lost_ack = matches!(&error,
                ProcessLedgerError::DurabilityAckLost { event_kind, process_uuid }
                    if event_kind.as_str() == LedgerEventKind::Stop.as_str()
                        && *process_uuid == owner.process.process_uuid);
            if matching_lost_ack
                && owner.submitting.load(std::sync::atomic::Ordering::Acquire)
                && tokio::time::Instant::now() < deadline
            {
                if matches!(tokio::time::timeout_at(
                    deadline, owner.verify_and_publish(),
                ).await, Ok(Ok(_))) {
                    return Ok(());
                }
            }
            if owner.durable_positive.load(std::sync::atomic::Ordering::Acquire) {
                Ok(())
            } else {
                Err(error)
            }
        }
        Ok(result) => result,
        Err(_) => Err(ProcessLedgerError::DurabilityAckTimeout {
            event_kind: LedgerEventKind::Stop.as_str().to_string(),
            process_uuid: process.process_uuid, timeout_ms: timeout.as_millis(),
        }),
    }
}

struct WriterScopedReclaimStopReservation {
    reserved: super::ReservedProcessStop,
}

#[async_trait]
impl ScopedReclaimStopReservation for WriterScopedReclaimStopReservation {
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

    async fn persist_owned(
        self: Box<Self>, stop: ProcessStop, _timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.reserved.commit_with_durable_ack(stop)?.wait_unbounded().await
    }

    async fn persist_confirmed(
        self: Box<Self>, stop: ProcessStop, _timeout: Duration,
        confirmation: ScopedStopConfirmation,
    ) -> Result<(), ProcessLedgerError> {
        self.reserved.commit_with_scoped_confirmation(stop, confirmation)?.wait_unbounded().await
    }
}

impl ScopedReclaimStopWriter for super::ProcessLedgerWriter {
    fn reserve_fresh_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ScopedReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterScopedReclaimStopReservation {
            reserved: self.try_reserve_fresh_reclaim_stop()?,
        }))
    }

    fn reserve_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ScopedReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterScopedReclaimStopReservation {
            reserved: self.try_reserve_reclaim_stop()?,
        }))
    }
}

impl ScopedReclaimStopWriter for super::LedgerBatcher {
    fn reserve_fresh_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ScopedReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterScopedReclaimStopReservation {
            reserved: self.try_reserve_fresh_reclaim_stop()?,
        }))
    }

    fn reserve_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ScopedReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(WriterScopedReclaimStopReservation {
            reserved: self.try_reserve_reclaim_stop()?,
        }))
    }
}

#[async_trait]
pub trait ReclaimStopWriter: Send + Sync + 'static {
    /// Persist the exact reclaim STOP before returning success. Queue acceptance
    /// is not sufficient because reclaim reports are durability receipts.
    async fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError>;
}

pub const RECLAIM_IN_PROGRESS_RECOVERY_LIMIT: usize = 64;
const RECLAIM_CLAIM_TTL: Duration = Duration::from_secs(30);
const RECLAIM_CLAIM_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const RECLAIM_STOP_ACK_TIMEOUT: Duration = Duration::from_secs(5);


#[derive(Clone)]
enum RetainedKillOutcome {
    NotInvoked(KillError),
    Completed(Result<Option<crate::sandbox::ProcessStatus>, KillError>),
    JoinedWithoutResult(KillError),
}

type JoinedKill = Result<Result<Option<crate::sandbox::ProcessStatus>, KillError>, tokio::task::JoinError>;

impl RetainedKillOutcome {
    fn from_join(joined: JoinedKill) -> Self {
        match joined {
            Ok(result) => Self::Completed(result),
            Err(error) => Self::JoinedWithoutResult(KillError::new(format!(
                "original reclaim task failed to join: {error}",
            ))),
        }
    }

    fn into_kill_result(self) -> Result<Option<crate::sandbox::ProcessStatus>, KillError> {
        match self {
            Self::NotInvoked(error) =>
                Err(KillError::before_adapter_invocation(error.message().to_owned())),
            Self::Completed(result) => result,
            Self::JoinedWithoutResult(error) => Err(error),
        }
    }
}

struct RetainedClaimRenewal {
    deadline: time::Instant,
    result: Arc<std::sync::Mutex<Option<Result<ReclaimClaim, ProcessLedgerError>>>>,
}

struct ReclaimClaimProgress {
    acknowledged_claim: ReclaimClaim,
    /// The last renewal failed or never answered, so the durable lease may be
    /// longer than the acknowledged one until a retry confirms it.
    renewal_unconfirmed: bool,
    retained_renewal: Option<RetainedClaimRenewal>,
    pending_stop: Option<ProcessStop>,
}

impl ReclaimClaimProgress {
    fn new(claim: ReclaimClaim) -> Self {
        Self {
            acknowledged_claim: claim, renewal_unconfirmed: false,
            retained_renewal: None, pending_stop: None,
        }
    }

    /// Adopt a renewed lease of the same claim. Every other answer leaves the
    /// acknowledged claim in place for a retry.
    fn record_renewal(
        &mut self, outcome: Result<ReclaimClaim, ProcessLedgerError>,
    ) -> Result<(), ProcessLedgerError> {
        self.renewal_unconfirmed = true;
        let renewed = outcome?;
        let mut expected = self.acknowledged_claim.clone();
        expected.lease_expires_at_unix_ms = renewed.lease_expires_at_unix_ms;
        if renewed != expected
            || renewed.lease_expires_at_unix_ms < self.acknowledged_claim.lease_expires_at_unix_ms
        {
            return Err(ProcessLedgerError::Store(
                "claim renewal returned a different identity or shortened lease".into(),
            ));
        }
        self.acknowledged_claim = renewed;
        self.renewal_unconfirmed = false;
        Ok(())
    }
}

struct ProcessKillFence {
    original: ScopedReclaimableProcess,
    store: Arc<dyn ScopedReclaimProcessStore>,
    killer: Arc<dyn ScopedSandboxKill>,
    stop_writer: Arc<dyn ScopedReclaimStopWriter>,
    stop_permit: std::sync::Mutex<Option<Box<dyn ScopedReclaimStopReservation>>>,
    task: tokio::sync::Mutex<Option<JoinHandle<Result<Option<crate::sandbox::ProcessStatus>, KillError>>>>,
    supervisor: std::sync::Mutex<Option<JoinHandle<()>>>,
    result: std::sync::Mutex<Option<RetainedKillOutcome>>,
    claim_progress: std::sync::Mutex<ReclaimClaimProgress>,
    completed: tokio::sync::Notify,
}

impl ProcessKillFence {
    /// Collect a finished background renewal, or give up on one that passed its
    /// deadline. Returns the deadline of a renewal that is still in flight.
    fn collect_claim_renewal(&self) -> Option<time::Instant> {
        let mut progress = self.claim_progress.lock().unwrap_or_else(|error| error.into_inner());
        let renewal = progress.retained_renewal.as_ref()?;
        let deadline = renewal.deadline;
        let outcome = renewal.result.lock().unwrap_or_else(|error| error.into_inner()).take();
        if outcome.is_none() && time::Instant::now() < deadline { return Some(deadline); }
        progress.retained_renewal = None;
        let outcome = outcome.unwrap_or_else(|| Err(ProcessLedgerError::Store(
            "claim renewal has no result within its deadline".into(),
        )));
        if let Err(error) = progress.record_renewal(outcome) {
            tracing::warn!(
                process_uuid = %self.original.process_uuid, error = %error,
                "reclaim claim renewal failed and will be retried"
            );
        }
        None
    }

    fn is_same_process(&self, process: &ScopedReclaimableProcess) -> bool {
        self.original.resource_scope == process.resource_scope
            && self.original.process_uuid == process.process_uuid
            && self.original.runtime_owner == process.runtime_owner
            && self.original.os_pid == process.os_pid
            && self.original.started_at == process.started_at
            && self.original.parent_session_id == process.parent_session_id
            && self.original.sandbox_adapter_id == process.sandbox_adapter_id
            && self.original.sandbox_internal_id == process.sandbox_internal_id
    }

    fn matches(
        &self, process: &ScopedReclaimableProcess,
        store: &Arc<dyn ScopedReclaimProcessStore>, killer: &Arc<dyn ScopedSandboxKill>,
        stop_writer: &Arc<dyn ScopedReclaimStopWriter>,
    ) -> bool {
        self.is_same_process(process)
            && process.resource_scope == process.reclaim_claim.resource_scope
            && self.original.reclaim_claim.kill_operation_uuid == process.reclaim_claim.kill_operation_uuid
            && self.original.reclaim_claim.claimant_uuid == process.reclaim_claim.claimant_uuid
            && self.original.reclaim_claim.generation == process.reclaim_claim.generation
            && self.original.reclaim_claim.claimed_at_unix_ms == process.reclaim_claim.claimed_at_unix_ms
            && Arc::ptr_eq(&self.store, store) && Arc::ptr_eq(&self.killer, killer)
            && Arc::ptr_eq(&self.stop_writer, stop_writer)
    }

    /// The outcome of this kill, or `None` while the kill may still run.
    fn finished(&self) -> Option<RetainedKillOutcome> {
        let published = self.result.lock().unwrap_or_else(|error| error.into_inner()).clone();
        if published.is_some() { return published; }
        // A supervisor that was dropped with its runtime leaves the task here.
        let mut task = self.task.try_lock().ok()?;
        let joined = task.as_mut()?.now_or_never()?;
        task.take();
        drop(task);
        self.publish(RetainedKillOutcome::from_join(joined));
        self.result.lock().unwrap_or_else(|error| error.into_inner()).clone()
    }

    fn publish(&self, outcome: RetainedKillOutcome) {
        self.result.lock().unwrap_or_else(|error| error.into_inner()).get_or_insert(outcome);
        self.completed.notify_waiters();
    }
}

/// Marks a kill as not invoked when its creator stops before the dispatch.
struct ReclaimPredispatchGuard {
    owner: Arc<ProcessKillFence>,
    dispatched: bool,
}

impl ReclaimPredispatchGuard {
    fn publish_not_invoked(&self) {
        if !self.dispatched {
            self.owner.publish(RetainedKillOutcome::NotInvoked(
                KillError::new("original reclaim creator exited before native task handoff")));
        }
    }
}

impl Drop for ReclaimPredispatchGuard {
    fn drop(&mut self) { self.publish_not_invoked(); }
}

/// Returns an unused STOP permit on every exit, including cancellation. A kill
/// that outlives its observation must not keep the writer from draining.
struct UnusedStopPermitGuard(Arc<ProcessKillFence>);

impl Drop for UnusedStopPermitGuard {
    fn drop(&mut self) {
        self.0.stop_permit.lock().unwrap_or_else(|error| error.into_inner()).take();
    }
}

static PROCESS_KILL_FENCES: OnceLock<
    std::sync::Mutex<std::collections::HashMap<Uuid, Arc<ProcessKillFence>>>,
> = OnceLock::new();

/// The fence of exactly this claim's kill in this instance.
fn lookup_process_kill_fence(
    process: &ScopedReclaimableProcess,
    store: &Arc<dyn ScopedReclaimProcessStore>, killer: &Arc<dyn ScopedSandboxKill>,
    stop_writer: &Arc<dyn ScopedReclaimStopWriter>,
) -> Option<Arc<ProcessKillFence>> {
    PROCESS_KILL_FENCES.get_or_init(Default::default).lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&process.process_uuid)
        .filter(|owner| owner.matches(process, store, killer, stop_writer))
        .cloned()
}

/// Processes whose kill was started by this instance and may still run. Their
/// expired claims must not be taken over here.
pub(crate) fn running_kill_process_uuids() -> Vec<Uuid> {
    PROCESS_KILL_FENCES.get_or_init(Default::default).lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .filter(|(_, fence)| fence.finished().is_none())
        .map(|(process_uuid, _)| *process_uuid)
        .collect()
}

/// Register the kill of a newly claimed process. Returns `None` while a kill
/// from an earlier claim may still run: no second kill is started beside it. A
/// finished earlier kill that proved the end of the process is adopted instead
/// of repeated.
fn acquire_process_kill_fence(
    process: &ScopedReclaimableProcess,
    store: &Arc<dyn ScopedReclaimProcessStore>, killer: &Arc<dyn ScopedSandboxKill>,
    stop_writer: &Arc<dyn ScopedReclaimStopWriter>,
) -> Option<Arc<ProcessKillFence>> {
    let mut fences = PROCESS_KILL_FENCES.get_or_init(Default::default).lock()
        .unwrap_or_else(|error| error.into_inner());
    let adopted = match fences.get(&process.process_uuid) {
        None => None,
        Some(earlier) => Some(earlier.finished()?).filter(|outcome|
            matches!(outcome, RetainedKillOutcome::Completed(Ok(_))) && earlier.is_same_process(process)),
    };
    let fence = Arc::new(ProcessKillFence {
        original: process.clone(), store: Arc::clone(store), killer: Arc::clone(killer),
        stop_writer: Arc::clone(stop_writer), stop_permit: std::sync::Mutex::new(None),
        task: tokio::sync::Mutex::new(None), supervisor: std::sync::Mutex::new(None),
        result: std::sync::Mutex::new(adopted),
        claim_progress: std::sync::Mutex::new(ReclaimClaimProgress::new(process.reclaim_claim.clone())),
        completed: tokio::sync::Notify::new(),
    });
    fences.insert(process.process_uuid, Arc::clone(&fence));
    Some(fence)
}

fn start_original_fenced_kill(
    owner: &Arc<ProcessKillFence>, timeout: Duration,
    predispatch: &mut ReclaimPredispatchGuard,
    owning_runtime: Option<&tokio::runtime::Handle>,
) -> Result<(), ProcessLedgerError> {
    if !Arc::ptr_eq(owner, &predispatch.owner) || predispatch.dispatched {
        return Err(ProcessLedgerError::Store("original reclaim dispatch guard mismatch".into()));
    }
    let mut original = owner.task.try_lock().map_err(|_| ProcessLedgerError::Store(
        "original reclaim task registration remains busy; claim retained".into(),
    ))?;
    if original.is_some()
        || owner.supervisor.lock().unwrap_or_else(|error| error.into_inner()).is_some()
        || owner.result.lock().unwrap_or_else(|error| error.into_inner()).is_some() {
        return Err(ProcessLedgerError::Store("original reclaim task was already registered".into()));
    }
    if owner.stop_permit.lock().unwrap_or_else(|error| error.into_inner()).is_none() {
        return Err(ProcessLedgerError::Store("original reclaim STOP permit unavailable before dispatch".into()));
    }
    let killer = Arc::clone(&owner.killer);
    let scope = owner.original.resource_scope.clone();
    let process_uuid = owner.original.process_uuid;
    let operation = owner.original.reclaim_claim.kill_operation_uuid;
    let deadline_at = time::Instant::now() + timeout;
    let dispatch = ScopedKillDispatchObservation::capture_identity_context(deadline_at);
    let original_dispatch = dispatch.clone();
    let owning_runtime = owning_runtime.cloned().unwrap_or_else(tokio::runtime::Handle::current);
    predispatch.dispatched = true;
    *original = Some(owning_runtime.spawn(async move {
        killer.kill_observed_with_dispatch(&scope, process_uuid, operation, original_dispatch).await
    }));
    let retained = Arc::clone(owner);
    let supervisor = owning_runtime.spawn(async move {
        let mut task = retained.task.lock().await;
        let Some(original) = task.as_mut() else { return; };
        let observed = time::timeout_at(deadline_at, &mut *original).await;
        let window_closed = observed.is_err();
        let outcome = match observed {
            Ok(joined) => RetainedKillOutcome::from_join(joined),
            Err(_) if dispatch.cancel_before_effect() => {
                original.abort();
                match (&mut *original).await {
                    Err(error) if error.is_cancelled() =>
                        RetainedKillOutcome::Completed(Err(KillError::before_adapter_invocation(format!(
                            "reclaim kill operation for process {process_uuid} exceeded {}ms before adapter effect",
                            timeout.as_millis(),
                        )))),
                    joined => RetainedKillOutcome::from_join(joined),
                }
            }
            // The kill was admitted and may still end the process. It gets one
            // more window to report; until then no second kill is started.
            Err(_) => match time::timeout(timeout, &mut *original).await {
                Ok(joined) => RetainedKillOutcome::from_join(joined),
                Err(_) => {
                    original.abort();
                    RetainedKillOutcome::JoinedWithoutResult(KillError::new(format!(
                        "reclaim kill operation for process {process_uuid} returned nothing within {}ms and was abandoned",
                        (timeout * 2).as_millis(),
                    )))
                }
            },
        };
        task.take();
        drop(task);
        // Nobody observes a failure any more once the window closed.
        let abandoned = window_closed && !matches!(outcome, RetainedKillOutcome::Completed(Ok(_)));
        retained.publish(outcome);
        if abandoned { clear_process_kill_fence(process_uuid, &retained); }
    });
    *owner.supervisor.lock().unwrap_or_else(|error| error.into_inner()) = Some(supervisor);
    Ok(())
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


/// Drop a finished kill whose durable row no longer needs it.
fn clear_finished_process_kill_fence(process_uuid: Uuid) {
    let mut fences = PROCESS_KILL_FENCES.get_or_init(Default::default).lock()
        .unwrap_or_else(|error| error.into_inner());
    if fences.get(&process_uuid).is_some_and(|fence| fence.finished().is_some()) {
        fences.remove(&process_uuid);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedReclaimTrigger {
    Close,
    Failure,
    Restart,
    Stale,
    OperatorCancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ScopedKillOutcome {
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
pub struct ScopedReclaimedProcess {
    pub process_uuid: Uuid,
    pub engine_kind: ProcessEngineKind,
    pub sandbox_adapter_id: Option<String>,
    pub kill_result: ScopedKillOutcome,
    pub stop_event_kind: Option<LedgerEventKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedReclaimReport {
    pub session_id: String,
    pub trigger: ScopedReclaimTrigger,
    pub processes_reclaimed: Vec<ScopedReclaimedProcess>,
    pub total_duration_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedReclaimKillOperationSweep {
    pub operations: Vec<ReclaimKillOperationSweepEntry>,
    pub reclaim_report: Option<ScopedReclaimReport>,
    pub reclaim_error: Option<String>,
}

pub struct ScopedReclaim {
    owning_runtime: Option<tokio::runtime::Handle>,
    store: Arc<dyn ScopedReclaimProcessStore>,
    sandbox_kill: Arc<dyn ScopedSandboxKill>,
    stop_writer: Arc<dyn ScopedReclaimStopWriter>,
    claim_renew_interval: Duration,
    kill_timeout: Duration,
    stop_ack_timeout: Duration,
}

#[derive(Clone, Copy)]
enum ScopedRecoveryContext<'a> {
    Restart {
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &'a [Uuid],
    },
    StaleOwner {
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &'a str,
        authorized_process_uuids: &'a [Uuid],
    },
}
impl<'a> ScopedRecoveryContext<'a> {
    fn authorized_process_uuids(self) -> &'a [Uuid] {
        match self {
            Self::Restart {
                authorized_process_uuids,
                ..
            }
            | Self::StaleOwner {
                authorized_process_uuids,
                ..
            } => authorized_process_uuids,
        }
    }
    fn excluded_owner_runtime_instance_id(self) -> Option<Uuid> {
        match self {
            Self::Restart {
                excluded_owner_runtime_instance_id,
                ..
            } => Some(excluded_owner_runtime_instance_id),
            Self::StaleOwner { .. } => None,
        }
    }
}

impl ScopedReclaim {
    pub fn new<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Self
    where
        S: ScopedReclaimProcessStore,
        K: ScopedSandboxKill,
        W: ScopedReclaimStopWriter,
    {
        Self {
            owning_runtime: None,
            store,
            sandbox_kill,
            stop_writer,
            claim_renew_interval: RECLAIM_CLAIM_RENEW_INTERVAL,
            kill_timeout: RECLAIM_KILL_TIMEOUT,
            stop_ack_timeout: RECLAIM_STOP_ACK_TIMEOUT,
        }
    }

    pub(crate) fn owning_runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.owning_runtime.clone()
    }

    pub(crate) fn with_owning_runtime(mut self, runtime: tokio::runtime::Handle) -> Self {
        self.owning_runtime = Some(runtime);
        self
    }

    /// Bound each kill observation inside the caller's own budget, so that one
    /// process that does not end is a reported failure and not a boot timeout.
    pub(crate) fn with_kill_observation_budget(mut self, budget: Duration) -> Self {
        self.kill_timeout = self.kill_timeout.min(budget / 2);
        self
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
        trigger: ScopedReclaimTrigger,
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
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
        trigger: ScopedReclaimTrigger,
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let claimed = self.store
            .active_process_for_session(resource_scope, session_id, process_uuid).await?;
        if claimed.is_none() {
            if let Some(report) = self.retained_stop_readback(
                resource_scope, process_uuid, Some(session_id), None, trigger, started,
            ).await? { return Ok(report); }
            self.require_no_retained_operation(resource_scope, process_uuid, Some(session_id), None)?;
        }
        self.run_claimed(session_id, trigger, started, claimed.into_iter().collect()).await
    }


    fn require_no_retained_operation(
        &self, resource_scope: &ReclaimResourceScope, process_uuid: Uuid,
        session_id: Option<&str>, runtime_owner: Option<Uuid>,
    ) -> Result<(), ProcessLedgerError> {
        let Some(fences) = PROCESS_KILL_FENCES.get() else { return Ok(()); };
        let owner = fences.lock().unwrap_or_else(|error| error.into_inner())
            .get(&process_uuid).cloned();
        let Some(owner) = owner else { return Ok(()); };
        if &owner.original.resource_scope != resource_scope { return Ok(()); }
        if owner.original.process_uuid != process_uuid
            || !owner.matches(&owner.original, &self.store, &self.sandbox_kill, &self.stop_writer)
            || session_id.is_some_and(|session| owner.original.parent_session_id.as_deref() != Some(session))
            || runtime_owner.is_some_and(|runtime| owner.original.runtime_owner.as_ref()
                .map(|owner| owner.runtime_instance_id) != Some(runtime))
        {
            return Err(ProcessLedgerError::Store("retained reclaim operation owner or scope mismatch".into()));
        }
        Err(ProcessLedgerError::Store(
            "original reclaim operation remains pending without a canonical claim; custody retained".into(),
        ))
    }

    async fn retained_stop_readback(
        &self, resource_scope: &ReclaimResourceScope, process_uuid: Uuid,
        session_id: Option<&str>, runtime_owner: Option<Uuid>,
        trigger: ScopedReclaimTrigger, started: std::time::Instant,
    ) -> Result<Option<ScopedReclaimReport>, ProcessLedgerError> {
        let owner = {
            let owners = SCOPED_STOP_SUBMISSIONS.get_or_init(Default::default)
                .lock().unwrap_or_else(|error| error.into_inner());
            owners.iter().find(|owner| owner.process.process_uuid == process_uuid
                && &owner.process.resource_scope == resource_scope).cloned()
        };
        let Some(owner) = owner else { return Ok(None); };
        if !Arc::ptr_eq(&owner.store, &self.store) || !Arc::ptr_eq(&owner.writer, &self.stop_writer)
            || session_id.is_some_and(|session| owner.process.parent_session_id.as_deref() != Some(session))
            || runtime_owner.is_some_and(|runtime| owner.process.runtime_owner.as_ref()
                .map(|owner| owner.runtime_instance_id) != Some(runtime))
            || !owner.submitting.load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(ProcessLedgerError::Store("original submitted scoped STOP readback owner unavailable".into()));
        }
        let original = owner.verify_and_publish().await?;
        clear_finished_process_kill_fence(process_uuid);
        let original_session = original.parent_session_id.clone()
            .unwrap_or_else(|| format!("process-ledger://{process_uuid}"));
        Ok(Some(ScopedReclaimReport {
            session_id: session_id.unwrap_or(&original_session).to_owned(), trigger,
            processes_reclaimed: vec![ScopedReclaimedProcess {
                process_uuid, engine_kind: original.engine_kind,
                sandbox_adapter_id: original.sandbox_adapter_id,
                kill_result: ScopedKillOutcome::Killed,
                stop_event_kind: Some(LedgerEventKind::Stop),
            }],
            total_duration_ms: started.elapsed().as_millis(),
        }))
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
        trigger: ScopedReclaimTrigger,
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let claimed = self
            .store
            .active_owned_process(resource_scope, process_uuid, owner_runtime_instance_id)
            .await?;
        if claimed.is_none() {
            if let Some(report) = self.retained_stop_readback(
                resource_scope, process_uuid, None, Some(owner_runtime_instance_id), trigger, started,
            ).await? { return Ok(report); }
            self.require_no_retained_operation(
                resource_scope, process_uuid, None, Some(owner_runtime_instance_id),
            )?;
        }
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
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
        self.run_restart_orphan_session_with_policy(
            resource_scope, session_id, excluded_owner_runtime_instance_id,
            authorized_process_uuids, RestartOrphanReconcileErrorPolicy::LogAndContinue,
        ).await
    }

    async fn run_restart_orphan_session_with_policy(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        policy: RestartOrphanReconcileErrorPolicy,
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
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
        self.run_claimed_with_policy(
            session_id, ScopedReclaimTrigger::Restart, started, active, policy,
        ).await
    }

    /// ScopedReclaim a stale session without widening the source's runtime+host
    /// ownership decision to foreign rows that happen to share the session id.
    pub async fn run_stale_owned_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
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
        self.run_claimed(session_id, ScopedReclaimTrigger::Stale, started, active)
            .await
    }

    async fn run_claimed(
        &self,
        session_id: &str,
        trigger: ScopedReclaimTrigger,
        started: std::time::Instant,
        active: Vec<ScopedReclaimableProcess>,
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
        self.run_claimed_with_policy(
            session_id, trigger, started, active, RestartOrphanReconcileErrorPolicy::LogAndContinue,
        ).await
    }

    async fn run_claimed_with_policy(
        &self,
        session_id: &str,
        trigger: ScopedReclaimTrigger,
        started: std::time::Instant,
        active: Vec<ScopedReclaimableProcess>,
        policy: RestartOrphanReconcileErrorPolicy,
    ) -> Result<ScopedReclaimReport, ProcessLedgerError> {
        let mut reclaimed = Vec::with_capacity(active.len());
        let mut active = active.into_iter();

        while let Some(mut process) = active.next() {
            let row_result: Result<ScopedReclaimedProcess, ProcessLedgerError> = async {
            if process.resource_scope != process.reclaim_claim.resource_scope {
                self.release_unprocessed_claims_after_abort(
                    std::iter::once(&process),
                    "claim ResourceScope mismatch",
                )
                .await;
                return Err(ProcessLedgerError::Store(format!(
                    "reclaim claim for process {} escaped its exact ResourceScope",
                    process.process_uuid
                )));
            }
            let retained_submission = scoped_stop_submission(&process, &self.store, &self.stop_writer)?;
            if retained_submission.is_some() && !process.kill_succeeded_pending_stop {
                return Err(ProcessLedgerError::Store("original scoped STOP submission awaits canonical pending recovery".into()));
            }
            let (kill_result, stop_event_kind) = if process.kill_succeeded_pending_stop {
                let persisted = async {
                    let stop = process.original_pending_stop()?;
                    let fence = lookup_process_kill_fence(
                        &process, &self.store, &self.sandbox_kill, &self.stop_writer,
                    );
                    let reservation = if retained_submission.is_some() {
                        None
                    } else {
                        let verified = self.store.verified_pending_stop(&process).await?;
                        if verified != stop {
                            return Err(ProcessLedgerError::Store("canonical pending STOP payload changed".into()));
                        }
                        Some(self.stop_writer.reserve_reclaim_stop()?)
                    };
                    persist_scoped_stop(
                        &process, &self.store, &self.stop_writer, reservation, fence,
                        stop, self.stop_ack_timeout,
                    ).await
                }.await;
                match persisted {
                    Ok(()) => {
                        clear_finished_process_kill_fence(process.process_uuid);
                        (ScopedKillOutcome::Killed, Some(LedgerEventKind::Stop))
                    }
                    Err(error) if policy == RestartOrphanReconcileErrorPolicy::AbortOnFirstError => {
                        return Err(error);
                    }
                    Err(error) => (
                        ScopedKillOutcome::KilledPendingStop {
                            error: error.to_string(),
                        },
                        None,
                    ),
                }
            } else if let Some(kill_fence) = acquire_process_kill_fence(
                &process, &self.store, &self.sandbox_kill, &self.stop_writer,
            ) {
                let _unused_permit = UnusedStopPermitGuard(Arc::clone(&kill_fence));
                let reservation = match self.stop_writer.reserve_fresh_reclaim_stop() {
                    Ok(reservation) => reservation,
                    Err(error) => {
                        clear_process_kill_fence(process.process_uuid, &kill_fence);
                        self.release_unprocessed_claims_after_abort(
                            std::iter::once(&process), "STOP reservation rejection",
                        ).await;
                        return Err(error);
                    }
                };
                let mut predispatch = ReclaimPredispatchGuard {
                    owner: Arc::clone(&kill_fence), dispatched: false,
                };
                if let Err(error) = self
                    .store
                    .mark_reclaim_kill_started(&process, &process.reclaim_claim)
                    .await
                {
                    drop(reservation);
                    predispatch.publish_not_invoked();
                    let mut released = matches!(self.store.release_unstarted_reclaim_claim(
                        &process, &process.reclaim_claim,
                    ).await, Ok(true));
                    if !released {
                        released = self.store.release_proved_uninvoked_claim(
                            &process, &process.reclaim_claim,
                        ).await.is_ok();
                    }
                    if released {
                        clear_process_kill_fence(process.process_uuid, &kill_fence);
                    }
                    return Err(error);
                }
                *kill_fence.stop_permit.lock().unwrap_or_else(|error| error.into_inner()) =
                    Some(reservation);
                // An adopted earlier kill already proved the end of the process.
                if kill_fence.finished().is_none() {
                    start_original_fenced_kill(
                        &kill_fence, self.effective_kill_timeout(), &mut predispatch,
                        self.owning_runtime.as_ref(),
                    )?;
                }
                let kill = self.observe_fenced_kill(&process, &kill_fence).await
                    .and_then(|status| ScopedReclaimTerminal::from_observed(&process, status));
                let settled_claim = self.settle_fenced_claim(&process, &kill_fence).await;
                match kill {
                    Ok(terminal) => {
                        let persisted = async {
                            process.reclaim_claim = settled_claim?;
                            process.sync_reclaim_claim_metadata()?;
                            let stop = {
                                let mut progress = kill_fence.claim_progress.lock()
                                    .unwrap_or_else(|error| error.into_inner());
                                if progress.pending_stop.is_none() {
                                    progress.pending_stop = Some(pack_scoped_pending_stop(&process, terminal)?);
                                }
                                progress.pending_stop.as_ref().expect("retained original STOP").clone()
                            };
                            let mut original = process.clone();
                            original.metadata_jsonb = stop.metadata_jsonb.clone();
                            let stop = original.original_pending_stop()?;
                            if ScopedReclaimTerminal::from_stop(&stop)? != terminal {
                                return Err(ProcessLedgerError::Store("retained terminal outcome changed".into()));
                            }
                            self.store.mark_reclaim_kill_succeeded(
                                &process, &stop, &process.reclaim_claim,
                            ).await?;
                            persist_scoped_stop(
                                &process, &self.store, &self.stop_writer, None,
                                Some(Arc::clone(&kill_fence)), stop, self.stop_ack_timeout,
                            ).await
                        }.await;
                        match persisted {
                            Ok(()) => {
                                clear_process_kill_fence(process.process_uuid, &kill_fence);
                                (ScopedKillOutcome::Killed, Some(LedgerEventKind::Stop))
                            }
                            Err(error) if policy == RestartOrphanReconcileErrorPolicy::AbortOnFirstError => {
                                return Err(error);
                            }
                            Err(error) => (ScopedKillOutcome::KilledPendingStop {
                                error: error.to_string(),
                            }, None),
                        }
                    }
                    Err(error) => {
                        // MT-019 F3: a kill failure is reported and counted, it never
                        // aborts the pass, not even at boot. The row stays truthfully
                        // open and unclaimed, so a later pass retries it. Only a kill
                        // that still runs keeps its fence against a second kill.
                        if error.kind != KillFailureKind::StillRunning {
                            clear_process_kill_fence(process.process_uuid, &kill_fence);
                        }
                        process.reclaim_claim = settled_claim?;
                        process.sync_reclaim_claim_metadata()?;
                        if error.kind == KillFailureKind::BeforeAdapterInvocation {
                            self.store.release_proved_uninvoked_claim(&process, &process.reclaim_claim).await?;
                            if policy == RestartOrphanReconcileErrorPolicy::AbortOnFirstError {
                                // A failed identity read is a store error, not a kill failure.
                                if let Some(cause) = &error.identity_lookup_cause {
                                    let original = cause.lock().unwrap_or_else(|error| error.into_inner())
                                        .take().ok_or_else(|| ProcessLedgerError::Store(
                                            "original identity lookup cause was already handed back".into(),
                                        ))?;
                                    return Err(original);
                                }
                            }
                        } else {
                            self.store.release_reclaim_claim(&process, &process.reclaim_claim).await?;
                        }
                        (ScopedKillOutcome::Failed { error: error.message().to_owned() }, None)
                    }
                }
            } else {
                self.store.release_unstarted_reclaim_claim(&process, &process.reclaim_claim).await?;
                (ScopedKillOutcome::Failed { error: format!(
                    "KILL_STILL_RUNNING: an earlier kill of process {} has not returned; no second kill was started",
                    process.process_uuid,
                ) }, None)
            };
            Ok(ScopedReclaimedProcess {
                process_uuid: process.process_uuid,
                engine_kind: process.engine_kind,
                sandbox_adapter_id: process.sandbox_adapter_id,
                kill_result,
                stop_event_kind,
            })
            }.await;
            match row_result {
                Ok(process) => reclaimed.push(process),
                Err(error) => {
                    self.release_unprocessed_claims_after_abort(
                        active.as_slice().iter(), "claimed batch row failure",
                    ).await;
                    return Err(error);
                }
            }
        }

        Ok(ScopedReclaimReport {
            session_id: session_id.to_string(),
            trigger,
            processes_reclaimed: reclaimed,
            total_duration_ms: started.elapsed().as_millis(),
        })
    }

    /// Reconcile one original restart recovery operation without widening its
    /// excluded runtime owner or complete authorized process population.
    pub async fn reconcile_kill_operation_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, ProcessLedgerError> {
        self.reconcile_original_kill_operation(
            resource_scope,
            session_id,
            process_uuid,
            kill_operation_uuid,
            ScopedRecoveryContext::Restart {
                excluded_owner_runtime_instance_id,
                authorized_process_uuids,
            },
        )
        .await
    }

    /// Reconcile one original stale-owner operation under the same instance,
    /// host and complete authorized process population.
    pub async fn reconcile_kill_operation_for_stale_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, ProcessLedgerError> {
        self.reconcile_original_kill_operation(
            resource_scope,
            session_id,
            process_uuid,
            kill_operation_uuid,
            ScopedRecoveryContext::StaleOwner {
                owner_runtime_instance_id,
                owner_host_scope_id,
                authorized_process_uuids,
            },
        )
        .await
    }

    async fn reconcile_original_kill_operation(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        context: ScopedRecoveryContext<'_>,
    ) -> Result<ReclaimKillOperationStatus, ProcessLedgerError> {
        let mut process = match context {
            ScopedRecoveryContext::Restart {
                excluded_owner_runtime_instance_id,
                authorized_process_uuids,
            } => {
                self.store
                    .claimed_recovery_process_for_session(
                        resource_scope,
                        session_id,
                        excluded_owner_runtime_instance_id,
                        authorized_process_uuids,
                        process_uuid,
                        kill_operation_uuid,
                    )
                    .await?
            }
            ScopedRecoveryContext::StaleOwner {
                owner_runtime_instance_id,
                owner_host_scope_id,
                authorized_process_uuids,
            } => {
                self.store
                    .claimed_recovery_process_for_stale_owner(
                        resource_scope,
                        session_id,
                        owner_runtime_instance_id,
                        owner_host_scope_id,
                        authorized_process_uuids,
                        process_uuid,
                        kill_operation_uuid,
                    )
                    .await?
            }
        }
        .ok_or_else(|| {
            ProcessLedgerError::Store("canonical scoped recovery claim unavailable".to_owned())
        })?;
        let status = tokio::time::timeout(Duration::from_secs(10),
            self.retained_recovery_status(&mut process))
            .await.map_err(|_| ProcessLedgerError::Store("owning adapter recovery status deadline exceeded".to_owned()))?
            .map_err(|error| ProcessLedgerError::Store(format!("kill-operation status query failed for process {process_uuid} operation {kill_operation_uuid}: {error}")))?;
        if matches!(
            status,
            ReclaimKillOperationStatus::Succeeded
                | ReclaimKillOperationStatus::Failed
                | ReclaimKillOperationStatus::NotStarted
        ) {
            self.store
                .resolve_reclaim_kill_operation(
                    &process,
                    context.authorized_process_uuids(),
                    context.excluded_owner_runtime_instance_id(),
                    status,
                )
                .await?;
            if matches!(status, ReclaimKillOperationStatus::Failed | ReclaimKillOperationStatus::NotStarted) {
                clear_finished_process_kill_fence(process.process_uuid);
            }
        }
        Ok(status)
    }

    pub async fn reconcile_in_progress_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ScopedReclaimKillOperationSweep, ProcessLedgerError> {
        self.reconcile_in_progress_for_session_with_policy(
            resource_scope, session_id, excluded_owner_runtime_instance_id,
            authorized_process_uuids, RestartOrphanReconcileErrorPolicy::LogAndContinue,
            &|| false,
        ).await
    }

    async fn reconcile_in_progress_for_session_with_policy(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        policy: RestartOrphanReconcileErrorPolicy,
        cancelled: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<ScopedReclaimKillOperationSweep, ProcessLedgerError> {
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
        self.reconcile_in_progress_candidates(
            resource_scope,
            session_id,
            recoverable,
            ScopedRecoveryContext::Restart {
                excluded_owner_runtime_instance_id,
                authorized_process_uuids,
            },
            policy,
            cancelled,
        )
        .await
    }

    pub async fn reconcile_in_progress_for_stale_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<ScopedReclaimKillOperationSweep, ProcessLedgerError> {
        self.reconcile_in_progress_for_stale_owner_with_cancel(
            resource_scope, session_id, owner_runtime_instance_id, owner_host_scope_id,
            authorized_process_uuids, &|| false,
        ).await
    }

    async fn reconcile_in_progress_for_stale_owner_with_cancel(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
        cancelled: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<ScopedReclaimKillOperationSweep, ProcessLedgerError> {
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
            ScopedRecoveryContext::StaleOwner {
                owner_runtime_instance_id,
                owner_host_scope_id,
                authorized_process_uuids,
            },
            RestartOrphanReconcileErrorPolicy::LogAndContinue,
            cancelled,
        )
        .await
    }

    async fn reconcile_in_progress_candidates(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        recoverable: Vec<ReclaimKillOperationCandidate>,
        context: ScopedRecoveryContext<'_>,
        policy: RestartOrphanReconcileErrorPolicy,
        cancelled: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<ScopedReclaimKillOperationSweep, ProcessLedgerError> {
        let mut operations = Vec::with_capacity(recoverable.len());
        let mut state_advanced = false;
        for candidate in recoverable {
            if cancelled() { break; }
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
            let acquired = match context {
                ScopedRecoveryContext::Restart {
                    excluded_owner_runtime_instance_id,
                    authorized_process_uuids,
                } => {
                    self.store
                        .claimed_recovery_process_for_session(
                            resource_scope,
                            session_id,
                            excluded_owner_runtime_instance_id,
                            authorized_process_uuids,
                            operation.process_uuid,
                            operation.kill_operation_uuid,
                        )
                        .await
                }
                ScopedRecoveryContext::StaleOwner {
                    owner_runtime_instance_id,
                    owner_host_scope_id,
                    authorized_process_uuids,
                } => {
                    self.store
                        .claimed_recovery_process_for_stale_owner(
                            resource_scope,
                            session_id,
                            owner_runtime_instance_id,
                            owner_host_scope_id,
                            authorized_process_uuids,
                            operation.process_uuid,
                            operation.kill_operation_uuid,
                        )
                        .await
                }
            };
            let mut process = match acquired {
                Ok(Some(process)) => process,
                Err(error) if policy == RestartOrphanReconcileErrorPolicy::AbortOnFirstError => {
                    return Err(error);
                }
                missing_or_error => {
                    let error = match missing_or_error {
                        Ok(None) => "canonical scoped recovery claim unavailable".to_owned(),
                        Err(error) => error.to_string(),
                        Ok(Some(_)) => unreachable!(),
                    };
                    operations.push(ReclaimKillOperationSweepEntry {
                        candidate: ReclaimKillOperationCandidate::Operation { operation },
                        outcome: ReclaimKillOperationSweepOutcome::MalformedRecoveryRow { error },
                    });
                    continue;
                }
            };
            let status_result = tokio::time::timeout(
                Duration::from_secs(10),
                self.retained_recovery_status(&mut process),
            )
            .await;
            let status = match status_result.unwrap_or_else(|_| {
                Err(KillError::new(
                    "owning adapter recovery status deadline exceeded",
                ))
            }) {
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
                        &process,
                        context.authorized_process_uuids(),
                        context.excluded_owner_runtime_instance_id(),
                        status,
                    )
                    .await
                {
                    Ok(()) => {
                        if matches!(status, ReclaimKillOperationStatus::Failed | ReclaimKillOperationStatus::NotStarted) {
                            clear_finished_process_kill_fence(process.process_uuid);
                        }
                        state_advanced = true;
                        operations.push(ReclaimKillOperationSweepEntry {
                            candidate: ReclaimKillOperationCandidate::Operation { operation },
                            outcome: ReclaimKillOperationSweepOutcome::StateAdvanced { status },
                        });
                    }
                    Err(error) if policy == RestartOrphanReconcileErrorPolicy::AbortOnFirstError => {
                        return Err(error);
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
        let (reclaim_report, reclaim_error) = if state_advanced && !cancelled() {
            let reclaim_result = match context {
                ScopedRecoveryContext::Restart {
                    excluded_owner_runtime_instance_id,
                    authorized_process_uuids,
                } => {
                    self.run_restart_orphan_session_with_policy(
                        resource_scope,
                        session_id,
                        excluded_owner_runtime_instance_id,
                        authorized_process_uuids,
                        policy,
                    )
                    .await
                }
                ScopedRecoveryContext::StaleOwner {
                    owner_runtime_instance_id,
                    owner_host_scope_id,
                    authorized_process_uuids,
                } => {
                    self.run_stale_owned_session(
                        resource_scope,
                        session_id,
                        owner_runtime_instance_id,
                        owner_host_scope_id,
                        authorized_process_uuids,
                    )
                    .await
                }
            };
            match reclaim_result {
                Ok(report) => (Some(report), None),
                Err(error) if policy == RestartOrphanReconcileErrorPolicy::AbortOnFirstError => {
                    return Err(error);
                }
                Err(error) => (None, Some(error.to_string())),
            }
        } else {
            (None, None)
        };
        Ok(ScopedReclaimKillOperationSweep {
            operations,
            reclaim_report,
            reclaim_error,
        })
    }

    async fn release_unprocessed_claims_after_abort<'a>(
        &self,
        processes: impl Iterator<Item = &'a ScopedReclaimableProcess>,
        context: &'static str,
    ) {
        for process in processes {
            if process.kill_succeeded_pending_stop {
                continue;
            }
            if lookup_process_kill_fence(process, &self.store, &self.sandbox_kill, &self.stop_writer).is_some()
                || !matches!(scoped_stop_submission(process, &self.store, &self.stop_writer), Ok(None))
            {
                continue;
            }
            if let Err(error) = self
                .store
                .release_unstarted_reclaim_claim(process, &process.reclaim_claim)
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

    fn effective_kill_timeout(&self) -> Duration {
        self.kill_timeout.max(Duration::from_millis(1))
    }

    /// Wait for a fenced kill while keeping its claim alive. The wait ends with
    /// the kill window; a kill that still runs then is reported, not awaited.
    async fn observe_fenced_kill(
        &self, process: &ScopedReclaimableProcess, kill_fence: &Arc<ProcessKillFence>,
    ) -> Result<Option<crate::sandbox::ProcessStatus>, KillError> {
        let renew_every = self.claim_renew_interval
            .clamp(Duration::from_millis(1), RECLAIM_CLAIM_TTL / 2);
        let mut renewal = time::interval(renew_every);
        renewal.set_missed_tick_behavior(MissedTickBehavior::Delay);
        renewal.tick().await;
        let kill_timeout = self.effective_kill_timeout();
        let deadline_at = time::Instant::now() + kill_timeout;
        loop {
            let completed = kill_fence.completed.notified();
            tokio::pin!(completed);
            completed.as_mut().enable();
            kill_fence.collect_claim_renewal();
            if let Some(outcome) = kill_fence.finished() {
                return outcome.into_kill_result();
            }
            if time::Instant::now() >= deadline_at {
                return Err(KillError::still_running(format!(
                    "reclaim kill of process {} has not returned within {}ms",
                    process.process_uuid, kill_timeout.as_millis(),
                )));
            }
            tokio::select! {
                biased;
                _ = &mut completed => {}
                _ = time::sleep_until(deadline_at) => {}
                _ = renewal.tick() => self.renew_fenced_claim(process, kill_fence, deadline_at).await,
            }
        }
    }

    /// Extend the claim lease once. A renewal that fails changes nothing: the
    /// next tick or the final settlement retries it from the acknowledged claim.
    async fn renew_fenced_claim(
        &self, process: &ScopedReclaimableProcess, kill_fence: &Arc<ProcessKillFence>,
        deadline: time::Instant,
    ) {
        let (renewal_process, claim) = {
            let mut progress = kill_fence.claim_progress.lock()
                .unwrap_or_else(|error| error.into_inner());
            if progress.retained_renewal.is_some() || progress.pending_stop.is_some() { return; }
            let claim = progress.acknowledged_claim.clone();
            let mut renewal_process = process.clone();
            renewal_process.reclaim_claim = claim.clone();
            if renewal_process.sync_reclaim_claim_metadata().is_err() { return; }
            if let (Some(runtime), Some(invocation)) = (
                self.owning_runtime.as_ref(),
                self.owning_runtime.as_ref().and_then(|_|
                    self.store.retained_reclaim_renewal(&renewal_process, &claim, deadline)),
            ) {
                let result = Arc::new(std::sync::Mutex::new(None));
                progress.retained_renewal = Some(RetainedClaimRenewal {
                    deadline: invocation.deadline, result: Arc::clone(&result),
                });
                let owner = Arc::clone(kill_fence);
                runtime.spawn(async move {
                    let outcome = invocation.original.await;
                    *result.lock().unwrap_or_else(|error| error.into_inner()) = Some(outcome);
                    owner.completed.notify_waiters();
                });
                return;
            }
            (renewal_process, claim)
        };
        let outcome = time::timeout_at(
            deadline, self.store.renew_reclaim_claim(&renewal_process, &claim),
        ).await.unwrap_or_else(|_| Err(ProcessLedgerError::Store(
            "claim renewal exceeded the kill window".into(),
        )));
        let recorded = kill_fence.claim_progress.lock()
            .unwrap_or_else(|error| error.into_inner()).record_renewal(outcome);
        if let Err(error) = recorded {
            tracing::warn!(
                process_uuid = %process.process_uuid, error = %error,
                "reclaim claim renewal failed and will be retried"
            );
        }
    }

    /// The durable claim after the kill window. A renewal in flight is awaited
    /// until its own deadline and a failed one is retried once, so that a lost
    /// renewal answer cannot block the STOP or the release for good.
    async fn settle_fenced_claim(
        &self, process: &ScopedReclaimableProcess, kill_fence: &Arc<ProcessKillFence>,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        loop {
            let completed = kill_fence.completed.notified();
            tokio::pin!(completed);
            completed.as_mut().enable();
            let Some(deadline) = kill_fence.collect_claim_renewal() else { break; };
            let _ = time::timeout_at(deadline, completed).await;
        }
        let mut settled = process.clone();
        {
            let progress = kill_fence.claim_progress.lock().unwrap_or_else(|error| error.into_inner());
            settled.reclaim_claim = progress.acknowledged_claim.clone();
            if !progress.renewal_unconfirmed { return Ok(settled.reclaim_claim); }
        }
        settled.sync_reclaim_claim_metadata()?;
        let outcome = time::timeout(
            self.stop_ack_timeout, self.store.renew_reclaim_claim(&settled, &settled.reclaim_claim),
        ).await.unwrap_or_else(|_| Err(ProcessLedgerError::Store(
            "claim renewal retry exceeded the STOP acknowledgement window".into(),
        )));
        let mut progress = kill_fence.claim_progress.lock().unwrap_or_else(|error| error.into_inner());
        progress.record_renewal(outcome)?;
        Ok(progress.acknowledged_claim.clone())
    }

    async fn retained_recovery_status(
        &self, process: &mut ScopedReclaimableProcess,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        if process.kill_succeeded_pending_stop {
            process.original_pending_stop().map_err(|error| KillError::new(error.to_string()))?;
            return Ok(ReclaimKillOperationStatus::Succeeded);
        }
        let owner = lookup_process_kill_fence(process, &self.store, &self.sandbox_kill, &self.stop_writer);
        let Some(owner) = owner else {
            // No local retained operation exists. The caller already holds the
            // exact canonical recovery claim and its original bounded callback.
            let terminal = match self.sandbox_kill.recovery_observation(
                &process.resource_scope, process.process_uuid,
                process.reclaim_claim.kill_operation_uuid,
            ).await? {
                ScopedKillRecoveryObservation::Succeeded { code } => ScopedReclaimTerminal::Exited { code },
                ScopedKillRecoveryObservation::SucceededBySignal { by_signal } =>
                    ScopedReclaimTerminal::from_observed(
                        process, Some(crate::sandbox::ProcessStatus::Killed { by_signal }))?,
                ScopedKillRecoveryObservation::Failed => return Ok(ReclaimKillOperationStatus::Failed),
                ScopedKillRecoveryObservation::NotStarted => return Ok(ReclaimKillOperationStatus::NotStarted),
                ScopedKillRecoveryObservation::InProgress => return Ok(ReclaimKillOperationStatus::InProgress),
                ScopedKillRecoveryObservation::Unknown => return Ok(ReclaimKillOperationStatus::Unknown),
            };
            let original = pack_scoped_pending_stop(process, terminal)
                .map_err(|error| KillError::new(error.to_string()))?;
            process.metadata_jsonb = original.metadata_jsonb;
            return Ok(ReclaimKillOperationStatus::Succeeded);
        };
        if owner.collect_claim_renewal().is_some() {
            return Ok(ReclaimKillOperationStatus::Unknown);
        }
        {
            // The caller just read this claim, so its lease is the durable one.
            let mut progress = owner.claim_progress.lock().unwrap_or_else(|error| error.into_inner());
            if progress.acknowledged_claim != process.reclaim_claim
                && (progress.pending_stop.is_some()
                    || progress.record_renewal(Ok(process.reclaim_claim.clone())).is_err())
            {
                return Ok(ReclaimKillOperationStatus::Unknown);
            }
        }
        let terminal = match owner.finished() {
            None => return Ok(ReclaimKillOperationStatus::InProgress),
            Some(RetainedKillOutcome::NotInvoked(_)) => return Ok(ReclaimKillOperationStatus::NotStarted),
            Some(RetainedKillOutcome::Completed(Err(error)))
                if error.kind == KillFailureKind::BeforeAdapterInvocation =>
                    return Ok(ReclaimKillOperationStatus::NotStarted),
            Some(RetainedKillOutcome::Completed(Ok(status))) =>
                ScopedReclaimTerminal::from_observed(process, status)?,
            Some(RetainedKillOutcome::Completed(Err(_)))
            | Some(RetainedKillOutcome::JoinedWithoutResult(_)) => {
                let retained_terminal = {
                    let progress = owner.claim_progress.lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if progress.acknowledged_claim != process.reclaim_claim {
                        return Ok(ReclaimKillOperationStatus::Unknown);
                    }
                    progress.pending_stop.as_ref().map(ScopedReclaimTerminal::from_stop).transpose()
                        .map_err(|error| KillError::new(error.to_string()))?
                };
                match retained_terminal {
                    Some(terminal) => terminal,
                    None => match owner.killer.recovery_observation(
                        &process.resource_scope, process.process_uuid,
                        process.reclaim_claim.kill_operation_uuid,
                    ).await? {
                        ScopedKillRecoveryObservation::Succeeded { code } => ScopedReclaimTerminal::Exited { code },
                        ScopedKillRecoveryObservation::SucceededBySignal { by_signal } =>
                            ScopedReclaimTerminal::from_observed(
                                process, Some(crate::sandbox::ProcessStatus::Killed { by_signal }))?,
                        _ => return Ok(ReclaimKillOperationStatus::Unknown),
                    },
                }
            }
        };
        let original = {
            let mut progress = owner.claim_progress.lock()
                .unwrap_or_else(|error| error.into_inner());
            if progress.acknowledged_claim != process.reclaim_claim {
                return Ok(ReclaimKillOperationStatus::Unknown);
            }
            if progress.pending_stop.is_none() {
                progress.pending_stop = Some(pack_scoped_pending_stop(process, terminal)
                    .map_err(|error| KillError::new(error.to_string()))?);
            }
            progress.pending_stop.as_ref().expect("retained original STOP").clone()
        };
        let mut scoped = process.clone();
        scoped.metadata_jsonb = original.metadata_jsonb.clone();
        let verified = scoped.original_pending_stop()
            .map_err(|error| KillError::new(error.to_string()))?;
        if verified != original
            || ScopedReclaimTerminal::from_stop(&original).map_err(|error| KillError::new(error.to_string()))? != terminal
            || original.runtime_owner != process.runtime_owner
        {
            return Err(KillError::new("original retained STOP differs from canonical recovery claim"));
        }
        // The existing fenced resolver alone advances the pending marker.
        // Preserve the first immutable STOP across cancellation and retry.
        *process = scoped;
        Ok(ReclaimKillOperationStatus::Succeeded)
    }

}

pub struct Reclaim {
    store: Arc<dyn ReclaimProcessStore>,
    sandbox_kill: Arc<dyn SandboxKill>,
    stop_writer: Arc<dyn ReclaimStopWriter>,
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
        }
    }

    async fn release_after_failure(
        &self,
        claimed: &[ReclaimableProcess],
        cause: String,
    ) -> ProcessLedgerError {
        match self.store.abandon(claimed).await {
            Ok(()) => ProcessLedgerError::Store(cause),
            Err(release_error) => ProcessLedgerError::Store(format!(
                "{cause}; exact reclaim-claim release also failed: {release_error}"
            )),
        }
    }

    pub async fn run(
        &self,
        session_id: &str,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self.store.active_processes_for_session(session_id).await?;
        let mut reclaimed = Vec::with_capacity(active.len());

        for (index, process) in active.iter().enumerate() {
            let kill_result = if process.reclaim_cleanup_completed {
                KillOutcome::Killed
            } else {
                match self.sandbox_kill.kill(process.process_uuid) {
                    Ok(()) => {
                        if let Err(error) = self.store.mark_cleanup_completed(process).await {
                            return Err(self
                                .release_after_failure(
                                    &active[index..],
                                    format!(
                                        "durable post-cleanup marker failed for process {}: {error}",
                                        process.process_uuid
                                    ),
                                )
                                .await);
                        }
                        KillOutcome::Killed
                    }
                    Err(error) => {
                        return Err(self
                            .release_after_failure(
                                &active[index..],
                                format!(
                                    "external reclaim cleanup failed for process {}: {}",
                                    process.process_uuid,
                                    error.message()
                                ),
                            )
                            .await);
                    }
                }
            };
            if let Err(error) = self
                .stop_writer
                .append_reclaim_stop(process.reclaim_stop(-1))
                .await
            {
                let release_from = if process.reclaim_cleanup_completed
                    || matches!(&kill_result, KillOutcome::Killed)
                {
                    index.saturating_add(1)
                } else {
                    index
                };
                return Err(self
                    .release_after_failure(
                        &active[release_from..],
                        format!(
                            "durable reclaim STOP failed for process {}: {error}",
                            process.process_uuid
                        ),
                    )
                    .await);
            }
            reclaimed.push(ReclaimedProcess {
                process_uuid: process.process_uuid,
                engine_kind: process.engine_kind,
                sandbox_adapter_id: process.sandbox_adapter_id.clone(),
                kill_result,
                stop_event_kind: LedgerEventKind::Stop,
            });
        }

        Ok(ReclaimReport {
            session_id: session_id.to_string(),
            trigger,
            processes_reclaimed: reclaimed,
            total_duration_ms: started.elapsed().as_millis(),
        })
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

#[async_trait]
impl ReclaimStopWriter for SurrealProcessLedgerStore {
    async fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.write_batch(vec![LedgerEvent::Stop(stop)]).await
    }
}

/// The projection the claim statement returns.
///
/// Field types mirror the `SCHEMAFULL` `kernel_process_lifecycle` definition, so
/// the conversions the PostgreSQL version had to perform by hand no longer
/// exist: `process_uuid`/`parent_process_id` arrive as native `uuid`, and the
/// two JSON columns arrive as `object FLEXIBLE` rather than text that had to be
/// re-parsed. Only `engine_kind` and `os_pid` still need a checked conversion,
/// and both are constrained by the schema (`engine_kind` is a literal union
/// matching [`ProcessEngineKind`] exactly; `os_pid` asserts `>= 0`).
#[derive(Debug, Clone, SurrealValue)]
struct ClaimedRow {
    process_uuid: Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    engine_kind: String,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    exit_code: Option<i64>,
    model_artifact_sha256: Option<String>,
    work_profile_id: Option<String>,
    owner_role: String,
    owner_wp: Option<String>,
    role_id: Option<String>,
    wp_id: Option<String>,
    mt_id: Option<String>,
    sandbox_capabilities_snapshot: Value,
    metadata: Value,
}

#[derive(Debug, Clone)]
struct ClaimReceipt {
    record: RecordId,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
    raw_row: SurrealDataValue,
}

impl ClaimReceipt {
    fn from_raw(raw_row: SurrealDataValue) -> Result<Self, ProcessLedgerError> {
        let SurrealDataValue::Object(object) = &raw_row else {
            return Err(ProcessLedgerError::Store(
                "reclaim claim returned a non-object row".to_string(),
            ));
        };
        let record = object
            .get("id")
            .cloned()
            .ok_or_else(|| {
                ProcessLedgerError::Store(
                    "reclaim claim returned a row without its record id".to_string(),
                )
            })
            .and_then(|value| {
                RecordId::from_value(value).map_err(|error| {
                    ProcessLedgerError::Store(format!(
                        "reclaim claim returned an invalid record id: {error}"
                    ))
                })
            })?;
        let previous_stopped_at = Option::<DateTime<Utc>>::from_value(
            object
                .get("stopped_at")
                .cloned()
                .unwrap_or(SurrealDataValue::None),
        )
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "reclaim claim returned an invalid prior stopped_at for {record:?}: {error}"
            ))
        })?;
        let previous_stop_reason = Option::<String>::from_value(
            object
                .get("stop_reason")
                .cloned()
                .unwrap_or(SurrealDataValue::None),
        )
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "reclaim claim returned an invalid prior stop_reason for {record:?}: {error}"
            ))
        })?;
        Ok(Self {
            record,
            previous_stopped_at,
            previous_stop_reason,
            raw_row,
        })
    }

    fn decode(&self) -> Result<ClaimedRow, ProcessLedgerError> {
        ClaimedRow::from_value(self.raw_row.clone()).map_err(|error| {
            ProcessLedgerError::Store(format!(
                "reclaim claimed row {:?} failed typed decode: {error}",
                self.record
            ))
        })
    }
}

impl ClaimedRow {
    fn into_reclaimable(
        self,
        session_id: &str,
        reclaim_claimed_at: DateTime<Utc>,
        reclaim_expected_reason: String,
        reclaim_expected_killed_reason: String,
        record: RecordId,
    ) -> Result<ReclaimableProcess, ProcessLedgerError> {
        let reclaim_cleanup_completed = self.stopped_at.is_some() && self.exit_code.is_none()
            && self.stop_reason.as_deref().is_some_and(|reason| {
            reason
                .strip_prefix(RECLAIM_KILLED_STOP_REASON)
                .is_some_and(|suffix| suffix.starts_with(':'))
        });
        let receipt = ReclaimPriorState {
            record,
            record_process_uuid: self.process_uuid,
            previous_stopped_at: self.stopped_at,
            previous_stop_reason: self.stop_reason.clone(),
        };
        let mut metadata = self.metadata;
        let metadata_object = metadata.as_object_mut().ok_or_else(|| {
            ProcessLedgerError::Store(format!(
                "reclaim metadata for process {} is not an object",
                self.process_uuid
            ))
        })?;
        if metadata_object.contains_key(RECLAIM_RECEIPT_METADATA_KEY) {
            return Err(ProcessLedgerError::Store(format!(
                "reclaim metadata for process {} uses reserved key {RECLAIM_RECEIPT_METADATA_KEY}",
                self.process_uuid
            )));
        }
        if let Some(durable_receipt) = metadata_object
            .get(RECLAIM_DURABLE_RECEIPT_METADATA_KEY)
            .cloned()
        {
            let receipt_kind = durable_receipt
                .as_object()
                .and_then(|receipt| receipt.get("receipt_kind"))
                .and_then(Value::as_str);
            if receipt_kind != Some(RECLAIM_DURABLE_RECEIPT_KIND) {
                return Err(ProcessLedgerError::Store(format!(
                    "reclaim metadata for process {} uses reserved key {RECLAIM_DURABLE_RECEIPT_METADATA_KEY}",
                    self.process_uuid
                )));
            }
            metadata_object.remove(RECLAIM_DURABLE_RECEIPT_METADATA_KEY);
        }
        metadata_object.insert(
            RECLAIM_RECEIPT_METADATA_KEY.to_owned(),
            serde_json::to_value(receipt).map_err(|error| {
                ProcessLedgerError::Store(format!(
                    "failed to retain exact reclaim receipt for process {}: {error}",
                    self.process_uuid
                ))
            })?,
        );
        Ok(ReclaimableProcess {
            process_uuid: self.process_uuid,
            os_pid: self.os_pid.map(os_pid_to_u32).transpose()?,
            // The claim's `WHERE parent_session_id = $session_id` makes this
            // provably equal to `session_id`; the fallback only covers the
            // schema's `option<string>` typing, it is not a guess.
            parent_session_id: self
                .parent_session_id
                .unwrap_or_else(|| session_id.to_owned()),
            parent_process_id: self.parent_process_id,
            sandbox_adapter_id: self.sandbox_adapter_id,
            sandbox_internal_id: self.sandbox_internal_id,
            engine_kind: ProcessEngineKind::try_from(self.engine_kind.as_str())
                .map_err(ProcessLedgerError::Store)?,
            started_at: self.started_at,
            model_artifact_sha256: self.model_artifact_sha256,
            work_profile_id: self.work_profile_id,
            owner_role: self.owner_role,
            owner_wp: self.owner_wp,
            role_id: self.role_id,
            wp_id: self.wp_id,
            mt_id: self.mt_id,
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot,
            metadata_jsonb: metadata,
            reclaim_claimed_at,
            reclaim_expected_reason,
            reclaim_expected_killed_reason,
            reclaim_cleanup_completed,
        })
    }
}

#[derive(Clone, SurrealValue)]
struct ClaimBindings {
    session_id: String,
    claimed_at: DateTime<Utc>,
    claim_prefix: String,
    killed_prefix: String,
    claim_reason: String,
    killed_reason: String,
}

#[derive(Clone, SurrealValue)]
struct ReleaseBindings {
    record: RecordId,
    claimed_at: DateTime<Utc>,
    previous_stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    claim_reason: String,
    killed_reason: String,
}

#[derive(Clone, SurrealValue)]
struct ReleaseSessionBindings {
    session_id: String,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
    killed_reason: String,
}

#[derive(Debug, Clone, SurrealValue)]
struct DurableClaimReceiptRow {
    id: RecordId,
    receipt_kind: String,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct AbandonBindings {
    record: RecordId,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
    killed_reason: String,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct CanonicalReadBindings {
    record: RecordId,
}

#[derive(Debug, SurrealValue)]
struct CanonicalReclaimState {
    id: RecordId,
    process_uuid: Uuid,
    stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    exit_code: Option<i64>,
}

#[derive(SurrealValue)]
struct MarkKilledBindings {
    record: RecordId,
    process_uuid: Uuid,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
    killed_reason: String,
}

#[async_trait]
impl ReclaimProcessStore for SurrealProcessLedgerStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.active_processes_for_session_at_with_owner(
            session_id,
            Utc::now(),
            reclaim_boot_owner_id(),
        )
        .await
    }

    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        self.mark_reclaim_cleanup_completed(process).await
    }

    async fn abandon(&self, processes: &[ReclaimableProcess]) -> Result<(), ProcessLedgerError> {
        self.abandon_reclaim_claims(processes).await
    }
}

impl SurrealProcessLedgerStore {
    /// Atomically claims active rows and re-acquires only a sentinel owned by a
    /// different process boot. Explicit time and owner inputs make close/reopen
    /// and concurrent-owner proofs deterministic without sleeping.
    pub(crate) async fn active_processes_for_session_at_with_owner(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.active_processes_for_session_at_with_owner_inner(
            session_id, claimed_at, owner_id, None,
        )
        .await
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn active_processes_for_session_with_raw_receipt_failure(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
        corrupt_process_uuid: Uuid,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.active_processes_for_session_at_with_owner_inner(
            session_id,
            claimed_at,
            owner_id,
            Some(corrupt_process_uuid),
        )
        .await
    }

    async fn active_processes_for_session_at_with_owner_inner(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
        corrupt_raw_receipt_for: Option<Uuid>,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        // MT-008: the claim (exclusion + stopped_at mutation) is atomic inside
        // the single UPDATE ... RETURN BEFORE statement.
        let bindings = ClaimBindings {
            session_id: session_id.to_owned(),
            claimed_at,
            claim_prefix: format!("{RECLAIM_CLAIM_STOP_REASON}:"),
            killed_prefix: format!("{RECLAIM_KILLED_STOP_REASON}:"),
            claim_reason: format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}"),
            killed_reason: format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}"),
        };
        let retry_seed = Uuid::now_v7();
        let mut failed_attempt = 0;
        let mut raw_rows: Vec<SurrealDataValue> = loop {
            let attempt_bindings = bindings.clone();
            let outcome = self
                .storage()
                .with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .query_values(SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY, attempt_bindings)
                            .await
                    })
                })
                .await;
            match outcome {
                Ok(rows) => break rows,
                Err(error)
                    if failed_attempt + 1 < SURREAL_CLAIM_MAX_ATTEMPTS
                        && is_surreal_retryable_transaction_conflict(&error) =>
                {
                    tokio::time::sleep(surreal_claim_retry_delay(retry_seed, failed_attempt)).await;
                    failed_attempt += 1;
                }
                Err(error) => return Err(ProcessLedgerError::from(error)),
            }
        };

        #[cfg(feature = "surreal-test-support")]
        if let Some(process_uuid) = corrupt_raw_receipt_for {
            let raw_row = raw_rows
                .iter_mut()
                .find(|raw_row| {
                    let SurrealDataValue::Object(object) = raw_row else {
                        return false;
                    };
                    object
                        .get("process_uuid")
                        .cloned()
                        .and_then(|value| Uuid::from_value(value).ok())
                        == Some(process_uuid)
                })
                .ok_or_else(|| {
                    ProcessLedgerError::Store(format!(
                        "raw receipt failure injection target {process_uuid} was not claimed"
                    ))
                })?;
            *raw_row = SurrealDataValue::None;
        }
        #[cfg(not(feature = "surreal-test-support"))]
        let _ = corrupt_raw_receipt_for;

        // The PostgreSQL version mapped rows inside the claiming transaction so
        // that a mapping failure rolled the claim back and left the rows
        // reclaimable. The seam commits the claim before Rust sees the rows, so
        // that intent is preserved by an explicit compensating release rather
        // than by a rollback. A crash after the claim commit is recovered by
        // the different-process-boot branch in the same atomic claim query.
        // The release is precise (see its exact current-claim guard), and for a
        // re-acquired row it restores the prior sentinel rather than falsely
        // making that row active again.
        let receipts = match raw_rows
            .into_iter()
            .map(ClaimReceipt::from_raw)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(receipts) => receipts,
            Err(error) => {
                let release = self
                    .release_session_claim_from_durable_receipts(session_id, claimed_at, owner_id)
                    .await;
                return match release {
                    Ok(()) => Err(error),
                    Err(release_error) => Err(ProcessLedgerError::Store(format!(
                        "{error}; exact durable-receipt compensation also failed: {release_error}"
                    ))),
                };
            }
        };
        let same_owner: Vec<RecordId> = match self
            .storage()
            .with_data_operation({
                let bindings = bindings.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .query_values(SURREAL_ACTIVE_RECLAIM_SAME_OWNER_QUERY, bindings)
                            .await
                    })
                }
            })
            .await
        {
            Ok(rows) => rows,
            Err(read_error) => {
                let release = self.release_claim(&receipts, claimed_at, owner_id).await;
                return match release {
                    Ok(()) => Err(ProcessLedgerError::Store(format!(
                        "same-owner reclaim convergence read failed: {read_error}"
                    ))),
                    Err(release_error) => Err(ProcessLedgerError::Store(format!(
                        "same-owner reclaim convergence read failed: {read_error}; claim compensation failed: {release_error}"
                    ))),
                };
            }
        };
        let stale_same_owner = same_owner
            .into_iter()
            .filter(|record| !receipts.iter().any(|receipt| receipt.record == *record))
            .collect::<Vec<_>>();
        if !stale_same_owner.is_empty() {
            let error = ProcessLedgerError::Store(format!(
                "same-boot reclaim claims have not converged: {stale_same_owner:?}"
            ));
            return match self.release_claim(&receipts, claimed_at, owner_id).await {
                Ok(()) => Err(error),
                Err(release_error) => Err(ProcessLedgerError::Store(format!(
                    "{error}; current claim compensation failed: {release_error}"
                ))),
            };
        }
        let decoded = receipts
            .iter()
            .map(|receipt| {
                let effective_claimed_at = if receipt.previous_stop_reason.as_deref()
                    == Some(bindings.killed_reason.as_str())
                {
                    receipt.previous_stopped_at.ok_or_else(|| {
                        ProcessLedgerError::Store(format!(
                            "same-boot cleanup marker {:?} has no stopped_at guard",
                            receipt.record
                        ))
                    })?
                } else {
                    claimed_at
                };
                receipt.decode().and_then(|row| {
                    row.into_reclaimable(
                        session_id,
                        effective_claimed_at,
                        bindings.claim_reason.clone(),
                        bindings.killed_reason.clone(),
                        receipt.record.clone(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>();
        match decoded {
            Ok(reclaimable) => Ok(reclaimable),
            Err(error) => {
                // The atomic statement claimed every returned row before any
                // Rust decoding began. A failure in any row must therefore
                // compensate the complete statement result, including rows
                // decoded before the malformed row.
                match self.release_claim(&receipts, claimed_at, owner_id).await {
                    Ok(()) => Err(error),
                    Err(release_error) => Err(ProcessLedgerError::Store(format!(
                        "{error}; claim compensation failed: {release_error}"
                    ))),
                }
            }
        }
    }

    async fn canonical_reclaim_state(
        &self,
        record: RecordId,
    ) -> Result<Option<CanonicalReclaimState>, ProcessLedgerError> {
        let bindings = CanonicalReadBindings {
            record: record.clone(),
        };
        let rows: Vec<CanonicalReclaimState> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_CANONICAL_READ_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(rows.into_iter().next()),
            count => Err(ProcessLedgerError::Store(format!(
                "canonical reclaim read returned {count} rows for {record:?}"
            ))),
        }
    }

    async fn verify_reclaim_release_converged(
        &self,
        record: RecordId,
        previous_stopped_at: Option<DateTime<Utc>>,
        previous_stop_reason: Option<&str>,
        current_claim_reason: &str,
        current_killed_reason: &str,
        claimed_at: DateTime<Utc>,
    ) -> Result<(), ProcessLedgerError> {
        let Some(state) = self.canonical_reclaim_state(record.clone()).await? else {
            return Err(ProcessLedgerError::Store(format!(
                "canonical reclaim row {record:?} is missing"
            )));
        };
        if state.stopped_at == previous_stopped_at
            && state.stop_reason.as_deref() == previous_stop_reason
        {
            return Ok(());
        }
        if state.stopped_at == Some(claimed_at) && state.exit_code.is_none()
            && state.stop_reason.as_deref() == Some(current_killed_reason)
        {
            return Err(ProcessLedgerError::Store(format!(
                "{record:?} still carries current cleanup marker"
            )));
        }
        // A current-boot sentinel is never convergence. In particular, it must
        // not make the next same-boot recovery's excluded claim look like a
        // successful zero-row recovery.
        if state.stop_reason.as_deref() == Some(current_claim_reason) {
            return Err(ProcessLedgerError::Store(format!(
                "{record:?} still carries current reclaim sentinel"
            )));
        }
        // A different durable stopped state proves that another writer has
        // advanced the row. Compensation must not overwrite that terminal or
        // post-cleanup state.
        if state.stopped_at.is_some() && (state.exit_code.is_some() || state.stop_reason.is_some())
        {
            return Ok(());
        }
        Err(ProcessLedgerError::Store(format!(
            "{record:?} did not converge to its pre-claim or a durable advanced state"
        )))
    }

    /// Puts rows this claim took back into the reclaimable pool.
    ///
    /// Transaction conflicts are retried with the same bounded jitter policy as
    /// the claim. Any exhausted/non-retryable failure is returned with the exact
    /// record id, so a failed compensation can never be mistaken for success.
    async fn release_claim(
        &self,
        receipts: &[ClaimReceipt],
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        let mut failures = Vec::new();
        for receipt in receipts {
            let claim_reason = format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}");
            let killed_reason = format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}");
            let bindings = ReleaseBindings {
                record: receipt.record.clone(),
                claimed_at,
                previous_stopped_at: receipt.previous_stopped_at,
                stop_reason: receipt.previous_stop_reason.clone(),
                claim_reason: claim_reason.clone(),
                killed_reason: killed_reason.clone(),
            };
            let retry_seed = Uuid::now_v7();
            let mut failed_attempt = 0;
            loop {
                let attempt_bindings = bindings.clone();
                let outcome = self
                    .storage()
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .execute_returning(
                                    SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY,
                                    attempt_bindings,
                                )
                                .await
                        })
                    })
                    .await;
                match outcome {
                    Ok(1) => break,
                    Ok(0) => {
                        let convergence = self
                            .verify_reclaim_release_converged(
                                receipt.record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                                &killed_reason,
                                claimed_at,
                            )
                            .await;
                        match convergence {
                            Ok(()) => {}
                            Err(error) => failures.push(format!(
                                "{:?}: zero-row release did not converge: {error}",
                                receipt.record
                            )),
                        }
                        break;
                    }
                    Ok(count) => {
                        failures.push(format!(
                            "{:?} returned impossible affected-row count {count}",
                            receipt.record
                        ));
                        break;
                    }
                    Err(error)
                        if failed_attempt + 1 < SURREAL_CLAIM_MAX_ATTEMPTS
                            && is_surreal_retryable_transaction_conflict(&error) =>
                    {
                        tokio::time::sleep(surreal_claim_retry_delay(retry_seed, failed_attempt))
                            .await;
                        failed_attempt += 1;
                    }
                    Err(error) => {
                        let convergence = self
                            .verify_reclaim_release_converged(
                                receipt.record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                                &killed_reason,
                                claimed_at,
                            )
                            .await;
                        if let Err(convergence_error) = convergence {
                            failures.push(format!(
                                "{:?}: {error}; canonical convergence failed: {convergence_error}",
                                receipt.record
                            ));
                        }
                        break;
                    }
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "unreleased reclaim records: {}",
                failures.join(", ")
            )))
        }
    }

    /// Restores every row claimed by this invocation from the pre-claim receipt
    /// persisted atomically in that same row. If the receipt query or any
    /// receipt validation fails, the claims remain fail-closed; this path never
    /// substitutes `NONE/NONE` for an unknown prior state.
    async fn release_session_claim_from_durable_receipts(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        let bindings = ReleaseSessionBindings {
            session_id: session_id.to_owned(),
            claimed_at,
            claim_reason: format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}"),
            killed_reason: format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}"),
        };
        let durable_rows: Vec<DurableClaimReceiptRow> = self
            .storage()
            .with_data_operation({
                let bindings = bindings.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .query_values(SURREAL_ACTIVE_RECLAIM_DURABLE_RECEIPTS_QUERY, bindings)
                            .await
                    })
                }
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        let mut receipts = Vec::with_capacity(durable_rows.len());
        for durable in durable_rows {
            if durable.receipt_kind != RECLAIM_DURABLE_RECEIPT_KIND {
                return Err(ProcessLedgerError::Store(format!(
                    "durable reclaim receipt for {:?} has unexpected kind {:?}",
                    durable.id, durable.receipt_kind
                )));
            }
            receipts.push(ClaimReceipt {
                record: durable.id,
                previous_stopped_at: durable.previous_stopped_at,
                previous_stop_reason: durable.previous_stop_reason,
                raw_row: SurrealDataValue::None,
            });
        }
        self.release_claim(&receipts, claimed_at, owner_id).await?;

        let outstanding: Vec<RecordId> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_OUTSTANDING_SESSION_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        if outstanding.is_empty() {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "exact durable-receipt compensation left outstanding reclaim records: {outstanding:?}"
            )))
        }
    }

    /// Releases exact claims that could not be completed by an external kill or
    /// durable STOP. Exact timestamp + boot-owner guards prevent this recovery
    /// path from undoing a concurrent terminal event or a later boot's claim.
    pub(crate) async fn abandon_reclaim_claims(
        &self,
        processes: &[ReclaimableProcess],
    ) -> Result<(), ProcessLedgerError> {
        let owner_id = reclaim_boot_owner_id();
        let claim_reason = format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}");
        let killed_reason = format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}");
        let mut failures = Vec::new();
        for process in processes {
            let receipt = match process.reclaim_receipt() {
                Ok(receipt) if receipt.record_process_uuid == process.process_uuid => receipt,
                Ok(receipt) => {
                    failures.push(format!(
                        "kernel_process_lifecycle:{}: receipt identifies {}",
                        process.process_uuid, receipt.record_process_uuid
                    ));
                    continue;
                }
                Err(error) => {
                    failures.push(format!(
                        "kernel_process_lifecycle:{}: {error}",
                        process.process_uuid
                    ));
                    continue;
                }
            };
            let record = receipt.record.clone();
            let bindings = AbandonBindings {
                record: record.clone(),
                claimed_at: process.reclaim_claimed_at,
                claim_reason: claim_reason.clone(),
                killed_reason: killed_reason.clone(),
                previous_stopped_at: receipt.previous_stopped_at,
                previous_stop_reason: receipt.previous_stop_reason.clone(),
            };
            let retry_seed = Uuid::now_v7();
            let mut failed_attempt = 0;
            loop {
                let attempt_bindings = bindings.clone();
                let outcome = self
                    .storage()
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .execute_returning(
                                    SURREAL_ACTIVE_RECLAIM_ABANDON_QUERY,
                                    attempt_bindings,
                                )
                                .await
                        })
                    })
                    .await;
                match outcome {
                    Ok(1) => break,
                    Ok(0) => {
                        if let Err(error) = self
                            .verify_reclaim_release_converged(
                                record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                                &killed_reason,
                                process.reclaim_claimed_at,
                            )
                            .await
                        {
                            failures.push(format!(
                                "kernel_process_lifecycle:{}: zero-row abandon did not converge: {error}",
                                process.process_uuid
                            ));
                        }
                        break;
                    }
                    Ok(count) => {
                        failures.push(format!(
                            "kernel_process_lifecycle:{}: impossible affected-row count {count}",
                            process.process_uuid
                        ));
                        break;
                    }
                    Err(error)
                        if failed_attempt + 1 < SURREAL_CLAIM_MAX_ATTEMPTS
                            && is_surreal_retryable_transaction_conflict(&error) =>
                    {
                        tokio::time::sleep(surreal_claim_retry_delay(retry_seed, failed_attempt))
                            .await;
                        failed_attempt += 1;
                    }
                    Err(error) => {
                        if let Err(convergence_error) = self
                            .verify_reclaim_release_converged(
                                record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                                &killed_reason,
                                process.reclaim_claimed_at,
                            )
                            .await
                        {
                            failures.push(format!(
                                "kernel_process_lifecycle:{}: {error}; canonical convergence failed: {convergence_error}",
                                process.process_uuid
                            ));
                        }
                        break;
                    }
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "unabandoned reclaim records: {}",
                failures.join(", ")
            )))
        }
    }

    /// Persist that external cleanup has succeeded before attempting the final
    /// STOP. A later boot can then finish the ledger transition without killing
    /// the same external identity again.
    pub(crate) async fn mark_reclaim_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        let owner_id = reclaim_boot_owner_id();
        let claim_reason = format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}");
        let killed_reason = format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}");
        let receipt = process.reclaim_receipt()?;
        if receipt.record.table.as_str() != "kernel_process_lifecycle"
            || receipt.record_process_uuid != process.process_uuid
            || process.reclaim_expected_reason != claim_reason
            || process.reclaim_expected_killed_reason != killed_reason
        {
            return Err(ProcessLedgerError::Store(format!(
                "invalid original reclaim cleanup marker identity for {}",
                process.process_uuid
            )));
        }
        let bindings = MarkKilledBindings {
            record: receipt.record.clone(),
            process_uuid: process.process_uuid,
            claimed_at: process.reclaim_claimed_at,
            claim_reason,
            killed_reason: killed_reason.clone(),
        };
        let updated: Result<Vec<Uuid>, ProcessLedgerError> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_MARK_KILLED_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from);
        let missing_marker = || ProcessLedgerError::Store(format!(
            "exact reclaim cleanup marker was not persisted for {}",
            process.process_uuid
        ));
        let original_error = match updated {
            Ok(updated) if updated == [process.process_uuid] => return Ok(()),
            Ok(updated) if !updated.is_empty() => return Err(missing_marker()),
            Ok(_) => missing_marker(),
            Err(error) => error,
        };
        match self.canonical_reclaim_state(receipt.record.clone()).await {
            Ok(Some(state))
                if state.id == receipt.record
                    && state.process_uuid == process.process_uuid
                    && state.stopped_at == Some(process.reclaim_claimed_at)
                    && state.stop_reason.as_deref() == Some(killed_reason.as_str())
                    && state.exit_code.is_none() =>
                Ok(()),
            _ => Err(original_error),
        }
    }
}

fn os_pid_to_u32(value: i64) -> Result<u32, ProcessLedgerError> {
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
    /// Process rows this pass actually reclaimed: [`ScopedKillOutcome::Killed`] or
    /// [`ScopedKillOutcome::KilledPendingStop`] only.
    ///
    /// MT-019 F5: this previously counted every [`ScopedReclaimedProcess`], so a
    /// [`ScopedKillOutcome::Failed`] row — a process that is still running and whose
    /// START row is still OPEN — was reported as reclaimed.
    pub processes_reclaimed: usize,
    /// Process rows whose kill did NOT succeed. Their claim was released, their
    /// fence cleared, and no STOP was written, so they remain truthfully open and
    /// idempotently retryable by a later pass. Non-zero here means boot completed
    /// with known-unreaped processes (see the F3 resilient-boot contract).
    pub processes_kill_failed: usize,
    /// The per-session Restart reclaim reports, in surfaced order.
    pub reclaim_reports: Vec<ScopedReclaimReport>,
    /// MT-019 F6: reclaim errors observed INSIDE the in-progress kill-operation
    /// sweep ([`ReclaimKillOperationSweep::reclaim_error`]). The sweep returns
    /// `Ok` while carrying this field, so the boot call site used to drop it
    /// silently. Periodic passes retain this diagnostic; boot propagates the
    /// original typed follow-up error before conversion into the public sweep.
    pub sweep_reclaim_errors: Vec<String>,
    /// Non-fatal per-session errors tolerated under
    /// [`RestartOrphanReconcileErrorPolicy::LogAndContinue`].
    pub session_errors: Vec<String>,
    /// The pass stopped early because the caller's cancellation hook fired.
    pub cancelled: bool,
}

impl RestartOrphanBootReconcileReport {
    fn record(&mut self, reclaim_report: ScopedReclaimReport) {
        self.sessions_reconciled += 1;
        for reclaimed in &reclaim_report.processes_reclaimed {
            match reclaimed.kill_result {
                ScopedKillOutcome::Killed | ScopedKillOutcome::KilledPendingStop { .. } => {
                    self.processes_reclaimed += 1
                }
                ScopedKillOutcome::Failed { .. } => self.processes_kill_failed += 1,
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
/// [`ScopedSandboxKill`] and given a durable STOP.
///
/// Kill failure is NOT an error here: [`ScopedReclaim::run_claimed`] releases the claim,
/// clears the fence, and writes no STOP, so the row stays truthfully open. Those
/// rows are counted in [`RestartOrphanBootReconcileReport::processes_kill_failed`]
/// for the caller to surface and retry.
///
/// When the source knows its own instance identity the reclaim binds an explicit
/// `owner_runtime_instance_id <> self` predicate (P-4c), so a restart pass cannot
/// claim the calling instance's own rows even if surfacing is ever wrong.
pub async fn reconcile_restart_orphans(
    reclaim: &ScopedReclaim,
    stale_source: &dyn StaleSessionSource,
    policy: RestartOrphanReconcileErrorPolicy,
    cancelled: &(dyn Fn() -> bool + Send + Sync),
) -> Result<RestartOrphanBootReconcileReport, ProcessLedgerError> {
    let mut report = RestartOrphanBootReconcileReport::default();
    let cancellation_seen = std::sync::atomic::AtomicBool::new(false);
    let cancellation = || {
        if cancelled() {
            cancellation_seen.store(true, std::sync::atomic::Ordering::Release);
        }
        cancellation_seen.load(std::sync::atomic::Ordering::Acquire)
    };
    if cancellation() {
        report.cancelled = true;
        return Ok(report);
    }
    let surfaced = match stale_source.restart_session_process_sets().await {
        Ok(surfaced) => surfaced,
        Err(error) => match policy {
            RestartOrphanReconcileErrorPolicy::AbortOnFirstError => return Err(error),
            RestartOrphanReconcileErrorPolicy::LogAndContinue => {
                tracing::error!(error = %error, "process-ledger restart-session scan failed");
                report.session_errors.push(error.to_string());
                report.cancelled = cancellation();
                return Ok(report);
            }
        },
    };
    if cancellation() {
        report.cancelled = true;
        return Ok(report);
    }
    let excluded_owner = stale_source.self_runtime_instance_id().ok_or_else(|| {
        ProcessLedgerError::InvalidConfig(
            "RESTART_RECLAIM_SELF_RUNTIME_INSTANCE_REQUIRED".to_owned(),
        )
    })?;
    for candidate in surfaced {
        let session_id = &candidate.session_id;
        if cancellation() {
            report.cancelled = true;
            return Ok(report);
        }
        match reclaim
            .reconcile_in_progress_for_session_with_policy(
                &candidate.resource_scope,
                session_id,
                excluded_owner,
                &candidate.authorized_process_uuids,
                policy,
                &cancellation,
            )
            .await
        {
            Ok(sweep) => {
                for entry in &sweep.operations {
                    let (category, error) = match &entry.outcome {
                        ReclaimKillOperationSweepOutcome::StatusQueryFailed { error } =>
                            ("status_query_failed", error),
                        ReclaimKillOperationSweepOutcome::StateTransitionFailed { error, .. } =>
                            ("state_transition_failed", error),
                        ReclaimKillOperationSweepOutcome::MalformedRecoveryRow { error } =>
                            ("malformed_recovery_row", error),
                        _ => continue,
                    };
                    let (process_id, operation_id) = match &entry.candidate {
                        ReclaimKillOperationCandidate::Operation { operation } =>
                            (operation.process_uuid.to_string(), Some(operation.kill_operation_uuid.to_string())),
                        ReclaimKillOperationCandidate::Malformed {
                            process_identity, kill_operation_identity, ..
                        } => (process_identity.clone(), kill_operation_identity.clone()),
                    };
                    tracing::warn!(
                        session_id, process_id = %process_id, operation_id = ?operation_id,
                        category, error = %error,
                        "restart kill-operation sweep retained an operation diagnostic"
                    );
                    report.sweep_reclaim_errors.push(format!(
                        "session={session_id} process={process_id} operation={operation_id:?} {category}: {error}",
                    ));
                }
                let followup_ran = sweep.reclaim_report.is_some() || sweep.reclaim_error.is_some();
                if let Some(reclaim_report) = sweep.reclaim_report {
                    report.record(reclaim_report);
                }
                if let Some(sweep_error) = sweep.reclaim_error {
                    tracing::warn!(
                        session_id,
                        error = %sweep_error,
                        "in-progress kill-operation sweep advanced state but its follow-up reclaim failed; the row remains open for a later pass"
                    );
                    report.sweep_reclaim_errors.push(sweep_error);
                }
                if cancellation() {
                    report.cancelled = true;
                    return Ok(report);
                }
                if followup_ran { continue; }
            }
            Err(error) => match policy {
                RestartOrphanReconcileErrorPolicy::AbortOnFirstError => return Err(error),
                RestartOrphanReconcileErrorPolicy::LogAndContinue => {
                    tracing::error!(session_id, error = %error, "process-ledger restart kill reconciliation failed");
                    report.session_errors.push(error.to_string());
                    if cancellation() {
                        report.cancelled = true;
                        return Ok(report);
                    }
                    continue;
                }
            },
        }
        let reclaim_result = reclaim
            .run_restart_orphan_session_with_policy(
                &candidate.resource_scope,
                session_id,
                excluded_owner,
                &candidate.authorized_process_uuids,
                policy,
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
        if cancellation() {
            report.cancelled = true;
            return Ok(report);
        }
    }
    Ok(report)
}

/// Boot entry point for [`reconcile_restart_orphans`]: fail closed on the first
/// error, no cancellation hook.
pub async fn reconcile_restart_orphans_at_boot(
    reclaim: &ScopedReclaim,
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
    reclaim: &ScopedReclaim,
    stale_source: &dyn StaleSessionSource,
    candidate: &StaleSessionProcessSet,
    cancelled: &(dyn Fn() -> bool + Send + Sync),
) -> Result<(), ProcessLedgerError> {
    if cancelled() { return Ok(()); }
    let (owner_runtime_instance_id, owner_host_scope_id) =
        stale_source.require_runtime_owner_scope()?;
    let sweep = reclaim
        .reconcile_in_progress_for_stale_owner_with_cancel(
            &candidate.resource_scope,
            &candidate.session_id,
            owner_runtime_instance_id,
            &owner_host_scope_id,
            &candidate.authorized_process_uuids,
            cancelled,
        )
        .await?;
    if let Some(error) = sweep.reclaim_error {
        return Err(ProcessLedgerError::Store(error));
    }
    if sweep.reclaim_report.is_some() { return Ok(()); }
    if cancelled() { return Ok(()); }
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

#[derive(Clone)]
pub struct ManagedStalenessReclaimTask {
    inner: Arc<ManagedStalenessReclaimTaskInner>,
}

struct ManagedStalenessReclaimTaskInner {
    shutdown: watch::Sender<bool>,
    join: Arc<tokio::sync::Mutex<RetainedReclaimJoin>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedReclaimJoinObservation { Pending, Completed, Cancelled, Failed, TimedOut }

impl ManagedReclaimJoinObservation {
    fn successful(self) -> bool { matches!(self, Self::Completed | Self::Cancelled) }
}

struct RetainedReclaimJoin {
    handle: Option<JoinHandle<()>>,
    terminal: Option<ManagedReclaimJoinObservation>,
    timed_out: bool,
    abort_requested: bool,
    deadline: Option<time::Instant>,
    handoff_failed: Arc<std::sync::atomic::AtomicBool>,
    changed: Arc<tokio::sync::Notify>,
    handoff: Option<Arc<std::sync::Mutex<Option<JoinHandle<()>>>>>,
}

impl RetainedReclaimJoin {
    fn new(handle: JoinHandle<()>) -> Self {
        Self { handle: Some(handle), terminal: None, timed_out: false, abort_requested: false, deadline: None, handoff: None,
            handoff_failed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            changed: Arc::new(tokio::sync::Notify::new()),
        }
    }
    fn recover_failed_handoff(&mut self) {
        if self.handoff_failed.swap(false, std::sync::atomic::Ordering::AcqRel) {
            if let Some(packet) = self.handoff.take() {
                self.handle = packet.lock().unwrap_or_else(|error| error.into_inner()).take();
            }
        }
    }

}

fn reclaim_join_observation(result: Result<(), tokio::task::JoinError>) -> ManagedReclaimJoinObservation {
    match result {
        Ok(()) => ManagedReclaimJoinObservation::Completed,
        Err(error) if error.is_cancelled() => ManagedReclaimJoinObservation::Cancelled,
        Err(_) => ManagedReclaimJoinObservation::Failed,
    }
}

impl ManagedStalenessReclaimTask {
    pub fn join_observation(&self) -> ManagedReclaimJoinObservation {
        self.inner.join.try_lock().ok().and_then(|state| state.terminal)
            .unwrap_or(ManagedReclaimJoinObservation::Pending)
    }

    pub async fn shutdown_and_join(&self, timeout: Duration) -> bool {
        let caller_deadline = time::Instant::now() + timeout;
        let _ = self.inner.shutdown.send(true);
        loop {
            let Ok(mut state) = time::timeout_at(caller_deadline, self.inner.join.lock()).await else { return false; };
            state.recover_failed_handoff();
            if let Some(outcome) = state.terminal {
                return matches!(outcome, ManagedReclaimJoinObservation::Completed);
            }
            let grace_deadline = if state.abort_requested {
                state.deadline.unwrap_or(caller_deadline)
            } else {
                *state.deadline.get_or_insert(caller_deadline)
            };
            if state.handle.is_none() {
                let changed = Arc::clone(&state.changed);
                let notified = changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                // Helper failure is published outside this mutex. Register
                // first, then recover/recheck so the wake cannot be lost.
                state.recover_failed_handoff();
                if state.handle.is_some() || state.terminal.is_some() { continue; }
                drop(state);
                if time::timeout_at(caller_deadline, notified).await.is_err() { return false; }
                continue;
            }
            let observation_deadline = if state.abort_requested {
                caller_deadline
            } else {
                caller_deadline.min(grace_deadline)
            };
            let join = state.handle.as_mut().expect("original retained reclaim join");
            let result = time::timeout_at(observation_deadline, join).await;
            let result = match result {
                Ok(result) => result,
                Err(_) => {
                    if !state.abort_requested {
                        if time::Instant::now() < grace_deadline {
                            // Only this shorter observer expired. The original
                            // graceful phase and task remain unchanged.
                            return false;
                        }
                        state.timed_out = true;
                        state.handle.as_ref().expect("retained reclaim handle").abort();
                        state.abort_requested = true;
                    }
                    let join = state.handle.as_mut().expect("retained reclaim handle");
                    match time::timeout_at(caller_deadline, join).await {
                        Ok(result) => result,
                        Err(_) => return false,
                    }
                }
            };
            let outcome = if state.timed_out {
                ManagedReclaimJoinObservation::TimedOut
            } else {
                reclaim_join_observation(result)
            };
            state.handle.take();
            state.terminal = Some(outcome);
            state.changed.notify_waiters();
            return matches!(outcome, ManagedReclaimJoinObservation::Completed);
        }
    }

    pub fn abort_and_join_blocking(&self, timeout: Duration) -> bool {
        let _ = self.inner.shutdown.send(true);
        abort_retained_reclaim(Arc::clone(&self.inner.join), timeout)
    }
}

fn abort_retained_reclaim(owner: Arc<tokio::sync::Mutex<RetainedReclaimJoin>>, timeout: Duration) -> bool {
    let Ok(mut state) = owner.try_lock() else { return false; };
    state.recover_failed_handoff();
    if let Some(outcome) = state.terminal { return outcome.successful(); }
    let Some(handle) = state.handle.take() else { return false; };
    if !state.abort_requested {
        handle.abort();
        state.abort_requested = true;
    }
    let packet = Arc::new(std::sync::Mutex::new(Some(handle)));
    state.handoff = Some(Arc::clone(&packet));
    let timed_out = state.timed_out;
    let failed = Arc::clone(&state.handoff_failed);
    let changed = Arc::clone(&state.changed);
    drop(state);
    let worker_owner = Arc::clone(&owner);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let worker = move || {
        let handle = packet.lock().unwrap_or_else(|error| error.into_inner()).take();
        let Some(handle) = handle else { return; };
        let result = futures::executor::block_on(handle);
        let outcome = if timed_out { ManagedReclaimJoinObservation::TimedOut } else { reclaim_join_observation(result) };
        let mut state = worker_owner.blocking_lock();
        state.terminal = Some(outcome);
        state.handoff = None;
        state.changed.notify_waiters();
        drop(state);
        let _ = tx.send(outcome.successful());
    };
    let helper = std::thread::Builder::new().name("handshake-reclaim-retained-join".into()).spawn(worker);
    if helper.is_err() {
        failed.store(true, std::sync::atomic::Ordering::Release);
        changed.notify_waiters();
        return false;
    }
    rx.recv_timeout(timeout).unwrap_or(false)
}

pub fn spawn_managed_staleness_reclaim_task(
    reclaim: Arc<ScopedReclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> ManagedStalenessReclaimTask {
    spawn_managed_staleness_reclaim_task_internal(reclaim, stale_source, config, true)
}

/// Post-boot variant: the caller has ALREADY run the boot restart pass inline, so
/// this task skips the immediate restart pass and relies on its periodic tick
/// (MT-019 F2) to re-surface anything the boot pass skipped or timed out on.
pub fn spawn_managed_staleness_reclaim_task_after_boot(
    reclaim: Arc<ScopedReclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> ManagedStalenessReclaimTask {
    spawn_managed_staleness_reclaim_task_internal(reclaim, stale_source, config, false)
}

fn spawn_managed_staleness_reclaim_task_internal(
    reclaim: Arc<ScopedReclaim>,
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
                            &|| *shutdown_rx.borrow(),
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
            join: Arc::new(tokio::sync::Mutex::new(RetainedReclaimJoin::new(join))),
        }),
    }
}

impl Drop for ManagedStalenessReclaimTaskInner {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if !abort_retained_reclaim(Arc::clone(&self.join), Duration::from_secs(2)) {
            tracing::error!("managed process-reclaim shutdown remains pending or failed");
            if self.join.try_lock().map_or(true, |state| state.handle.is_some() || state.handoff_failed.load(std::sync::atomic::Ordering::Acquire)) {
                // No successful observer handoff means unresolved custody must survive Drop.
                std::mem::forget(Arc::clone(&self.join));
            }
        }
    }
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
            if let Ok(session_ids) = stale_source.stale_sessions(config.ttl).await {
                for session_id in session_ids {
                    let _ = reclaim.run(&session_id, ReclaimTrigger::Stale).await;
                }
            }
        }
    })
}

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
