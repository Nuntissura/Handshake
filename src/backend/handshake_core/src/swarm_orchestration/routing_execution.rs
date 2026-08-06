//! PostgreSQL/EventLedger execution state for canonical mixed-model routing graphs.
//!
//! The execution row is a replay projection. EventLedger is the append authority;
//! stage attempts and the transactional outbox make claims attributable, leased,
//! idempotent, and recoverable after coordinator failure.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::kernel::{
    context_bundle::canonical_json_bytes, KernelActor, KernelEventType, NewKernelEvent,
};
use crate::storage::postgres::append_kernel_event_with_executor;

use super::ids::SpawnRequest;
use super::model_lane::{
    ModelLanePromotionDecisionRecord, ModelLanePromotionOutcome, ModelLaneRunRecord,
};
use super::routing::{
    ModelLaneRoutingAuthority, ModelLaneRoutingDispatchTarget, ModelLaneRoutingGraph,
    ModelLaneRoutingStageLaunchPlan, ModelLaneRoutingStageOutcome,
};

const ROUTING_EXECUTION_SCHEMA_ID: &str = "hsk.model_lane_routing_execution@5";
const ROUTING_STAGE_ATTEMPT_SCHEMA_ID: &str = "hsk.model_lane_routing_stage_attempt@4";
const ROUTING_OUTBOX_SCHEMA_ID: &str = "hsk.model_lane_routing_outbox@4";
const SOURCE_COMPONENT: &str = "model_lane_routing_executor";
const DEFAULT_LEASE_MS: u64 = 30_000;
const MAX_STAGE_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CloudReviewVerdict {
    Accept,
    Reject,
    PromotionRecommended,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct CloudReviewOutput {
    verdict: CloudReviewVerdict,
    review: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingExecutionStatus {
    Running,
    AwaitingAuthority,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingStageStateKind {
    Scheduled,
    Claimed,
    InFlight,
    AwaitingAuthority,
    Succeeded,
    Failed,
    Joined,
    Cancelled,
    Compensated,
}

impl ModelLaneRoutingStageStateKind {
    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Joined | Self::Cancelled | Self::Compensated
        )
    }

    fn is_success(self) -> bool {
        matches!(self, Self::Succeeded | Self::Joined)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingStageState {
    pub stage_id: String,
    pub state: ModelLaneRoutingStageStateKind,
    pub attempt: u32,
    pub dispatch_target: ModelLaneRoutingDispatchTarget,
    pub expected_run_id: String,
    pub expected_lane_id: String,
    pub expected_model_id: String,
    pub expected_provider: Option<crate::model_runtime::ProviderKind>,
    pub instance_id: Option<String>,
    pub lane_id: Option<String>,
    pub input_refs: Vec<String>,
    pub output_ref: Option<String>,
    pub output_message_ref: Option<String>,
    pub authority_request_message_ref: Option<String>,
    pub output_sha256: Option<String>,
    pub output_payload: Option<Value>,
    pub authority_ref: Option<String>,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<u64>,
    pub detail: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRoutingExecutionState {
    pub schema_id: String,
    pub execution_id: String,
    pub run_id: String,
    pub selecting_decision_id: String,
    pub selecting_decision_event_id: String,
    pub selecting_decision_event_seq: i64,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub locus_ref: String,
    pub work_packet_id: String,
    pub micro_task_id: Option<String>,
    pub task_board_id: String,
    pub owner_session: String,
    pub canonical_graph: Value,
    pub canonical_graph_sha256: String,
    #[serde(default)]
    pub canonical_launch_plan: Vec<ModelLaneRoutingStageLaunchPlan>,
    #[serde(default)]
    pub canonical_launch_plan_sha256: String,
    pub authority: ModelLaneRoutingAuthority,
    pub initial_input_ref: Option<String>,
    pub initial_input_sha256: Option<String>,
    pub status: ModelLaneRoutingExecutionStatus,
    pub failure_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub revision: u64,
    pub stages: BTreeMap<String, ModelLaneRoutingStageState>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingExecutionContext {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub locus_ref: String,
    pub work_packet_id: String,
    pub micro_task_id: Option<String>,
    pub task_board_id: String,
    pub owner_session: String,
    pub initial_input_ref: String,
    pub initial_input_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelLaneRoutingStageClaim {
    pub execution_id: String,
    pub stage_id: String,
    pub attempt: u32,
    pub fencing_token: String,
    pub lease_owner: String,
    pub lease_expires_at_unix_ms: u64,
    pub dispatch_target: ModelLaneRoutingDispatchTarget,
    pub expected_run_id: String,
    pub expected_lane_id: String,
    pub expected_model_id: String,
    pub expected_provider: Option<crate::model_runtime::ProviderKind>,
}

#[derive(Clone)]
pub struct ModelLaneRoutingStageLaunch {
    pub stage_id: String,
    pub request: Option<SpawnRequest>,
    pub generate_request: Option<crate::model_runtime::GenerateRequest>,
    pub authority_lane_id: Option<String>,
    pub expected_run_id: String,
    pub expected_lane_id: String,
    pub expected_model_id: String,
    pub expected_provider: Option<crate::model_runtime::ProviderKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingStageDispatch {
    pub stage_id: String,
    pub dispatch_target: ModelLaneRoutingDispatchTarget,
    pub state: ModelLaneRoutingStageStateKind,
    pub instance_id: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRoutingDispatchBatch {
    pub execution: ModelLaneRoutingExecutionState,
    pub dispatched: Vec<ModelLaneRoutingStageDispatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingOutboxDiagnostics {
    pub command_id: String,
    pub status: String,
    pub fencing_token: Option<String>,
    pub lease_owner: Option<String>,
    pub lease_expires_at_unix_ms: Option<u64>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingStageDiagnostics {
    pub execution_id: String,
    pub stage_id: String,
    pub state: String,
    pub attempt: u32,
    pub dispatch_target: String,
    pub dependency_stage_ids: Vec<String>,
    pub expected_run_id: String,
    pub expected_lane_id: String,
    pub expected_model_id: String,
    pub expected_provider: Option<String>,
    pub instance_id: Option<String>,
    pub lane_id: Option<String>,
    pub input_refs: Vec<String>,
    pub output_ref: Option<String>,
    pub output_message_ref: Option<String>,
    pub authority_request_message_ref: Option<String>,
    pub output_sha256: Option<String>,
    pub authority_ref: Option<String>,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<u64>,
    pub lease_expired: bool,
    pub detail: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub updated_at_unix_ms: u64,
    pub outbox: ModelLaneRoutingOutboxDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingExecutionDiagnostics {
    pub execution_id: String,
    pub run_id: String,
    pub selecting_decision_id: String,
    pub selecting_decision_event_id: String,
    pub selecting_decision_event_seq: i64,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub locus_ref: String,
    pub work_packet_id: String,
    pub micro_task_id: Option<String>,
    pub task_board_id: String,
    pub owner_session: String,
    pub canonical_graph_sha256: String,
    pub canonical_launch_plan_sha256: String,
    pub cloud_consent_receipt_ref: Option<String>,
    pub validator_authority_ref: Option<String>,
    pub operator_authority_ref: Option<String>,
    pub initial_input_ref: Option<String>,
    pub initial_input_sha256: Option<String>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub revision: u64,
    pub stages: Vec<ModelLaneRoutingStageDiagnostics>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Debug, Clone)]
pub struct ModelLaneRoutingExecutionStore {
    pool: PgPool,
    lease_owner: String,
    lease_ms: u64,
    /// Carried so the routing executor stamps the same account scope onto the
    /// ModelLane message/artifact rows it writes as the store that owns the run
    /// (HBR-PRIV-001). A routing-produced row must not be less attributable than
    /// a directly recorded one.
    access: crate::swarm_orchestration::resource_scope::ResourceAccessContext,
}

#[derive(Debug)]
struct StageView {
    stage_id: String,
    dispatch_target: ModelLaneRoutingDispatchTarget,
    dependencies: Vec<String>,
    activation: String,
    gate: String,
}

impl ModelLaneRoutingExecutionStore {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self::new_with_access(
            pool,
            crate::swarm_orchestration::resource_scope::ResourceAccessContext::system(
                crate::swarm_orchestration::resource_scope::SystemScopeAuthority::legacy_unscoped_call_site(),
            ),
        )
    }

    pub(crate) fn new_with_access(
        pool: PgPool,
        access: crate::swarm_orchestration::resource_scope::ResourceAccessContext,
    ) -> Self {
        Self {
            pool,
            lease_owner: format!("routing-executor:{}", uuid::Uuid::now_v7()),
            lease_ms: DEFAULT_LEASE_MS,
            access,
        }
    }

    pub(crate) fn with_lease(pool: PgPool, lease_owner: impl Into<String>, lease_ms: u64) -> Self {
        Self {
            pool,
            lease_owner: lease_owner.into(),
            lease_ms: lease_ms.max(1),
            access: crate::swarm_orchestration::resource_scope::ResourceAccessContext::system(
                crate::swarm_orchestration::resource_scope::SystemScopeAuthority::legacy_unscoped_call_site(),
            ),
        }
    }

    pub(crate) fn postgres_pool(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn snapshot(
        &self,
        execution_id: &str,
    ) -> Result<Option<ModelLaneRoutingExecutionState>, String> {
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        lock_execution(&mut tx, execution_id).await?;
        let state = load_execution_tx(&mut tx, execution_id).await?;
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(state)
    }

    /// Read the execution WITHOUT taking the execution-keyed advisory lock.
    ///
    /// This exists for exactly one caller: cancellation. `lock_execution` is an
    /// xact lock, so a stage that is mid-generation holds it until its
    /// transaction ends - and that transaction only ends once the stage is
    /// cancelled. `cancel_routing_execution` used the locking `snapshot` as its
    /// FIRST step, which made cancellation deadlock against the work it was
    /// trying to cancel:
    ///
    ///   worker holds the advisory lock, blocked in generation, waiting to be
    ///   cancelled -> cancel blocks on `snapshot` before it can fire any session
    ///   cancel token -> generation never observes cancellation -> the lock is
    ///   never released.
    ///
    /// Proven from live pg_stat_activity during the hang: the lock holder sat in
    /// Client/ClientRead (idle in transaction) with two cancellation-side
    /// sessions queued behind it on Lock/advisory.
    ///
    /// Reading unlocked is SAFE for this caller because the result is only used
    /// to decide WHICH live instances to signal. A concurrently-changing stage
    /// can only mean a signal that is redundant (already terminal) or one more
    /// that the authoritative, still-locked `cancel_execution` will terminalize
    /// anyway. Cancellation must never queue behind the work it is cancelling.
    /// Returns `None` when the execution does not exist.
    ///
    /// Deliberately does NOT go through `load_execution_tx`. That path verifies
    /// projection/EventLedger integrity, and the advisory lock is what makes
    /// that verification sound - see the "fractured projection/EventLedger view"
    /// note on the locked read. Running it unlocked produces spurious
    /// "projection/EventLedger integrity failure" errors when a stage is
    /// mid-write, which is EXACTLY the moment cancellation is most likely to be
    /// issued. Cancellation does not need that guarantee: it only needs to know
    /// which instances to signal, and an instance whose state changed under it
    /// is either already terminal (the signal is a no-op) or will be
    /// terminalized by the authoritative, still-locked `cancel_execution`.
    pub(crate) async fn active_instance_ids_for_cancellation(
        &self,
        execution_id: &str,
    ) -> Result<Option<Vec<String>>, String> {
        let record: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT record_json FROM model_lane_routing_executions WHERE execution_id = $1",
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| err.to_string())?;
        let Some(record) = record else {
            return Ok(None);
        };
        let mut instance_ids = Vec::new();
        if let Some(stages) = record.get("stages").and_then(serde_json::Value::as_object) {
            for stage in stages.values() {
                let is_active = matches!(
                    stage.get("state").and_then(serde_json::Value::as_str),
                    Some("claimed") | Some("in_flight") | Some("awaiting_authority")
                );
                if is_active {
                    if let Some(instance_id) =
                        stage.get("instance_id").and_then(serde_json::Value::as_str)
                    {
                        instance_ids.push(instance_id.to_string());
                    }
                }
            }
        }
        Ok(Some(instance_ids))
    }

    /// Read native diagnostics through the production execution/stage/outbox
    /// integrity gate and retain deterministic run/stage ordering.
    pub(crate) async fn diagnostics_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ModelLaneRoutingExecutionDiagnostics>, String> {
        let execution_ids = sqlx::query_scalar::<_, String>(
            r#"SELECT execution_id FROM model_lane_routing_executions
               WHERE run_id = $1 ORDER BY event_ledger_seq ASC, execution_id ASC"#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|err| err.to_string())?;
        let observed_at_unix_ms = now_ms();
        let mut diagnostics = Vec::with_capacity(execution_ids.len());
        for execution_id in execution_ids {
            let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
            lock_execution(&mut tx, &execution_id).await?;
            let state = load_execution_tx(&mut tx, &execution_id)
                .await?
                .ok_or_else(|| format!("routing execution {execution_id} disappeared"))?;
            if state.run_id != run_id {
                return Err(format!(
                    "routing execution {execution_id} escaped requested run {run_id}"
                ));
            }
            let graph: ModelLaneRoutingGraph =
                serde_json::from_value(state.canonical_graph.clone()).map_err(|err| {
                    format!("routing execution {execution_id} graph decode failed: {err}")
                })?;
            graph
                .validate()
                .map_err(|err| format!("routing execution {execution_id} graph invalid: {err}"))?;
            let dependencies = graph
                .stages
                .iter()
                .map(|stage| (stage.stage_id.clone(), stage.depends_on.clone()))
                .collect::<BTreeMap<_, _>>();
            let mut stages = Vec::with_capacity(state.stages.len());
            for stage in state.stages.values() {
                let dependency_stage_ids = dependencies
                    .get(&stage.stage_id)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "routing execution {execution_id} current stage {} is absent from canonical graph",
                            stage.stage_id
                        )
                    })?;
                let command_id = format!(
                    "routing-command:{execution_id}:{}:{}",
                    stage.stage_id, stage.attempt
                );
                let row = sqlx::query(
                    r#"SELECT status, fencing_token, lease_owner,
                              lease_expires_at_unix_ms, event_ledger_event_id,
                              event_ledger_seq, created_at_unix_ms, updated_at_unix_ms
                       FROM model_lane_routing_outbox
                       WHERE command_id = $1 FOR UPDATE"#,
                )
                .bind(&command_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("routing outbox projection missing for {command_id}"))?;
                let lease_expires_at_unix_ms = row
                    .get::<Option<i64>, _>("lease_expires_at_unix_ms")
                    .map(|value| value as u64);
                let outbox = ModelLaneRoutingOutboxDiagnostics {
                    command_id,
                    status: row.get("status"),
                    fencing_token: row.get("fencing_token"),
                    lease_owner: row.get("lease_owner"),
                    lease_expires_at_unix_ms,
                    event_ledger_event_id: row.get("event_ledger_event_id"),
                    event_ledger_seq: row.get("event_ledger_seq"),
                    created_at_unix_ms: row.get::<i64, _>("created_at_unix_ms") as u64,
                    updated_at_unix_ms: row.get::<i64, _>("updated_at_unix_ms") as u64,
                };
                stages.push(ModelLaneRoutingStageDiagnostics {
                    execution_id: execution_id.clone(),
                    stage_id: stage.stage_id.clone(),
                    state: stage_kind_name(stage.state).to_owned(),
                    attempt: stage.attempt,
                    dispatch_target: dispatch_target_name(&stage.dispatch_target).to_owned(),
                    dependency_stage_ids,
                    expected_run_id: stage.expected_run_id.clone(),
                    expected_lane_id: stage.expected_lane_id.clone(),
                    expected_model_id: stage.expected_model_id.clone(),
                    expected_provider: stage
                        .expected_provider
                        .map(|provider| format!("{provider:?}").to_ascii_lowercase()),
                    instance_id: stage.instance_id.clone(),
                    lane_id: stage.lane_id.clone(),
                    input_refs: stage.input_refs.clone(),
                    output_ref: stage.output_ref.clone(),
                    output_message_ref: stage.output_message_ref.clone(),
                    authority_request_message_ref: stage.authority_request_message_ref.clone(),
                    output_sha256: stage.output_sha256.clone(),
                    authority_ref: stage.authority_ref.clone(),
                    lease_owner: stage.lease_owner.clone(),
                    fencing_token: stage.fencing_token.clone(),
                    lease_expires_at_unix_ms: stage.lease_expires_at_unix_ms,
                    lease_expired: stage
                        .lease_expires_at_unix_ms
                        .is_some_and(|expires| expires <= observed_at_unix_ms),
                    detail: stage.detail.clone(),
                    event_ledger_event_id: stage.event_ledger_event_id.clone(),
                    event_ledger_seq: stage.event_ledger_seq,
                    updated_at_unix_ms: stage.updated_at_unix_ms,
                    outbox,
                });
            }
            tx.commit().await.map_err(|err| err.to_string())?;
            diagnostics.push(ModelLaneRoutingExecutionDiagnostics {
                execution_id: state.execution_id,
                run_id: state.run_id,
                selecting_decision_id: state.selecting_decision_id,
                selecting_decision_event_id: state.selecting_decision_event_id,
                selecting_decision_event_seq: state.selecting_decision_event_seq,
                trace_id: state.trace_id,
                run_span_id: state.run_span_id,
                coordinator_session_id: state.coordinator_session_id,
                locus_ref: state.locus_ref,
                work_packet_id: state.work_packet_id,
                micro_task_id: state.micro_task_id,
                task_board_id: state.task_board_id,
                owner_session: state.owner_session,
                canonical_graph_sha256: state.canonical_graph_sha256,
                canonical_launch_plan_sha256: state.canonical_launch_plan_sha256,
                cloud_consent_receipt_ref: state.authority.cloud_consent_receipt_ref,
                validator_authority_ref: state.authority.validator_authority_ref,
                operator_authority_ref: state.authority.operator_authority_ref,
                initial_input_ref: state.initial_input_ref,
                initial_input_sha256: state.initial_input_sha256,
                status: execution_status_name(state.status).to_owned(),
                failure_reason: state.failure_reason,
                cancel_reason: state.cancel_reason,
                revision: state.revision,
                stages,
                event_ledger_event_id: state.event_ledger_event_id,
                event_ledger_seq: state.event_ledger_seq,
            });
        }
        Ok(diagnostics)
    }

    pub(crate) async fn begin_execution(
        &self,
        execution_id: &str,
        selecting_decision_id: &str,
        authority: &ModelLaneRoutingAuthority,
        context: ModelLaneRoutingExecutionContext,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        lock_execution(&mut tx, execution_id).await?;
        let decision_row = sqlx::query(
            r#"SELECT decision.record_json, decision.event_ledger_event_id,
                      decision.event_ledger_seq, ledger.aggregate_type,
                      ledger.aggregate_id, ledger.payload
               FROM model_lane_promotion_decisions decision
               LEFT JOIN kernel_event_ledger ledger
                 ON ledger.event_id = decision.event_ledger_event_id
                AND ledger.event_sequence = decision.event_ledger_seq
               WHERE decision.decision_id = $1
               FOR UPDATE OF decision"#,
        )
        .bind(selecting_decision_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("unknown selecting promotion decision {selecting_decision_id}"))?;
        let decision_json: Value = decision_row.get("record_json");
        let decision: ModelLanePromotionDecisionRecord =
            serde_json::from_value(decision_json.clone()).map_err(|err| err.to_string())?;
        if decision.outcome != ModelLanePromotionOutcome::Approved {
            return Err(format!(
                "selecting promotion decision {selecting_decision_id} is not approved"
            ));
        }
        let aggregate_type: Option<String> = decision_row.get("aggregate_type");
        let aggregate_id: Option<String> = decision_row.get("aggregate_id");
        let ledger_payload: Option<Value> = decision_row.get("payload");
        if aggregate_type.as_deref() != Some("model_lane_promotion_decision")
            || aggregate_id.as_deref() != Some(selecting_decision_id)
            || ledger_payload
                .as_ref()
                .and_then(|value| value.pointer("/record"))
                .cloned()
                .map(record_without_generated_event_fields)
                != Some(record_without_generated_event_fields(decision_json.clone()))
        {
            return Err(format!("selecting promotion decision {selecting_decision_id} projection/EventLedger mismatch"));
        }
        if decision.run_id != context.run_id
            || decision.trace_id != context.trace_id
            || decision.coordinator_session_id != context.coordinator_session_id
            || decision.work_packet_id.as_deref() != Some(context.work_packet_id.as_str())
            || decision.task_board_id.as_deref() != Some(context.task_board_id.as_str())
            || decision.owner_session != context.owner_session
        {
            return Err(format!("selecting promotion decision {selecting_decision_id} does not bind execution context"));
        }
        let run_row = sqlx::query(
            r#"SELECT run.record_json, ledger.aggregate_type, ledger.aggregate_id, ledger.payload
               FROM model_lane_runs run
               LEFT JOIN kernel_event_ledger ledger
                 ON ledger.event_id = run.event_ledger_event_id
                AND ledger.event_sequence = run.event_ledger_seq
               WHERE run.run_id = $1
               FOR UPDATE OF run"#,
        )
        .bind(&context.run_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!(
                "selecting decision references missing ModelLaneRun {}",
                context.run_id
            )
        })?;
        let run_json: Value = run_row.get("record_json");
        let run: ModelLaneRunRecord =
            serde_json::from_value(run_json.clone()).map_err(|err| err.to_string())?;
        let run_payload: Option<Value> = run_row.get("payload");
        let run_aggregate_type: Option<String> = run_row.get("aggregate_type");
        let run_aggregate_id: Option<String> = run_row.get("aggregate_id");
        if run_aggregate_type.as_deref() != Some("model_lane_run")
            || run_aggregate_id.as_deref() != Some(context.run_id.as_str())
            || run_payload
                .as_ref()
                .and_then(|value| value.pointer("/record"))
                .cloned()
                .map(record_without_generated_event_fields)
                != Some(record_without_generated_event_fields(run_json))
            || run.trace_id != context.trace_id
            || run.run_span_id != context.run_span_id
            || run.coordinator_session_id != context.coordinator_session_id
            || run.work_packet_id.as_deref() != Some(context.work_packet_id.as_str())
            || run.micro_task_id.as_deref() != context.micro_task_id.as_deref()
            || run.task_board_id.as_deref() != Some(context.task_board_id.as_str())
            || run.owner_session != context.owner_session
            || run
                .locus_binding
                .as_ref()
                .map(|binding| binding.locus_binding_ref.as_str())
                != Some(context.locus_ref.as_str())
        {
            return Err(format!(
                "ModelLaneRun {} projection/EventLedger/context mismatch",
                context.run_id
            ));
        }
        if !decision
            .selected_input_refs
            .iter()
            .any(|value| value == &context.initial_input_ref)
        {
            return Err(format!(
                "initial input is not selected by promotion decision {selecting_decision_id}"
            ));
        }
        validate_initial_input_tx(
            &mut tx,
            &context.run_id,
            &context.initial_input_ref,
            &context.initial_input_sha256,
        )
        .await?;
        let graph = ModelLaneRoutingGraph::for_policy(decision.routing_policy);
        graph.validate().map_err(|err| err.to_string())?;
        if decision.diagnostic_payload.get("routing_graph")
            != Some(&serde_json::to_value(&graph).map_err(|err| err.to_string())?)
        {
            return Err(format!("selecting promotion decision {selecting_decision_id} does not persist the exact canonical graph"));
        }
        validate_launch_plan(&graph, &decision.routing_launch_plan)?;
        let launch_plan_hash = canonical_sha256(
            &serde_json::to_value(&decision.routing_launch_plan).map_err(|err| err.to_string())?,
        )?;
        let derived_authority = ModelLaneRoutingAuthority {
            cloud_consent_receipt_ref: decision
                .diagnostic_payload
                .get("cloud_consent_receipt_ref")
                .and_then(Value::as_str)
                .map(str::to_owned),
            validator_authority_ref: decision.validator_authority_ref.clone(),
            operator_authority_ref: decision.operator_authority_ref.clone(),
        };
        let authority_failure = graph
            .require_authority_contract(&derived_authority)
            .err()
            .map(|error| error.to_string())
            .or_else(|| {
                (authority != &derived_authority).then(|| {
                    format!("routing authority differs from selecting promotion decision {selecting_decision_id}")
                })
            })
            .or_else(|| context.micro_task_id.is_none().then(|| {
                "routing execution requires micro_task_id for durable ModelLane output authority".to_string()
            }));
        let canonical_graph = serde_json::to_value(&graph).map_err(|err| err.to_string())?;
        let graph_hash = canonical_sha256(&canonical_graph)?;
        if let Some(existing) = load_execution_tx(&mut tx, execution_id).await? {
            if existing.canonical_graph_sha256 == graph_hash
                && existing.selecting_decision_id == selecting_decision_id
                && existing.authority == *authority
                && existing.run_id == context.run_id
                && existing.trace_id == context.trace_id
                && existing.locus_ref == context.locus_ref
            {
                tx.commit().await.map_err(|err| err.to_string())?;
                return Ok(existing);
            }
            return Err(format!(
                "routing execution {execution_id} immutable context conflict"
            ));
        }
        let mut state = initial_execution(
            execution_id,
            canonical_graph,
            graph_hash,
            authority,
            context,
            selecting_decision_id,
            &decision.event_ledger_event_id,
            decision.event_ledger_seq,
            decision.routing_launch_plan.clone(),
            launch_plan_hash,
        );
        if let Some(reason) = authority_failure {
            state.status = ModelLaneRoutingExecutionStatus::Failed;
            state.failure_reason = Some(format!("begin-time authority contract failure: {reason}"));
        }
        let stored = append_event(
            &mut tx,
            if state.status == ModelLaneRoutingExecutionStatus::Failed {
                KernelEventType::SessionFailed
            } else {
                KernelEventType::SessionStarted
            },
            "model_lane_routing_execution",
            execution_id,
            &format!("routing-execution-start:{execution_id}"),
            &state.run_id,
            execution_id,
            json!({"schema_id": ROUTING_EXECUTION_SCHEMA_ID, "record": state}),
        )
        .await?;
        state.event_ledger_event_id = stored.0;
        state.event_ledger_seq = stored.1;
        save_execution_tx(&mut tx, &state).await?;
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(state)
    }

    pub(crate) async fn claim_ready(
        &self,
        execution_id: &str,
        launches: &[ModelLaneRoutingStageLaunch],
    ) -> Result<Vec<ModelLaneRoutingStageClaim>, String> {
        let now = now_ms();
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        lock_execution(&mut tx, execution_id).await?;

        let mut execution = match load_execution_tx(&mut tx, execution_id).await? {
            Some(existing) => existing,
            None => return Err(format!(
                "routing execution {execution_id} has no explicit run/trace/Locus context; call begin_execution first"
            )),
        };
        let graph: ModelLaneRoutingGraph =
            serde_json::from_value(execution.canonical_graph.clone())
                .map_err(|err| err.to_string())?;
        graph.validate().map_err(|err| err.to_string())?;
        let views = stage_views(&execution.canonical_graph)?;
        let mut launch_stage_ids = BTreeSet::new();
        for launch in launches {
            if !launch_stage_ids.insert(launch.stage_id.as_str()) {
                return Err(format!(
                    "duplicate routing launch for stage {}",
                    launch.stage_id
                ));
            }
            let view = views
                .iter()
                .find(|view| view.stage_id == launch.stage_id)
                .ok_or_else(|| {
                    format!(
                        "routing launch references unknown stage {}",
                        launch.stage_id
                    )
                })?;
            let plan = execution
                .canonical_launch_plan
                .iter()
                .find(|plan| plan.stage_id == launch.stage_id)
                .ok_or_else(|| {
                    format!(
                        "routing launch {} has no selecting-decision plan",
                        launch.stage_id
                    )
                })?;
            if launch.expected_run_id != execution.run_id {
                return Err(format!(
                    "routing launch {} changes canonical run",
                    launch.stage_id
                ));
            }
            if matches!(
                view.dispatch_target,
                ModelLaneRoutingDispatchTarget::LocalModel
                    | ModelLaneRoutingDispatchTarget::CloudModel
            ) {
                let request = launch.request.as_ref().ok_or_else(|| {
                    format!(
                        "model routing stage {} has no SpawnRequest",
                        launch.stage_id
                    )
                })?;
                let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
                    format!(
                        "model routing stage {} has no Dexterity launch contract",
                        launch.stage_id
                    )
                })?;
                if launch.generate_request.is_none()
                    || launch.expected_lane_id.is_empty()
                    || contract.run_id != launch.expected_run_id
                    || contract.lane_id != launch.expected_lane_id
                    || request.instance_id.model_id.to_string() != launch.expected_model_id
                    || request.provider != launch.expected_provider
                    || plan.dispatch_target != view.dispatch_target
                    || plan.lane_id.as_deref() != Some(launch.expected_lane_id.as_str())
                    || plan.model_id.as_deref() != Some(launch.expected_model_id.as_str())
                    || plan.provider != launch.expected_provider
                {
                    return Err(format!(
                        "routing launch {} differs from its selecting-decision run/lane/model/provider plan or generation authority",
                        launch.stage_id
                    ));
                }
            } else if launch.request.is_some() || launch.generate_request.is_some() {
                return Err(format!(
                    "non-model routing stage {} cannot carry model launch requests",
                    launch.stage_id
                ));
            } else if plan.dispatch_target != view.dispatch_target
                || plan.lane_id.as_deref() != launch.authority_lane_id.as_deref()
                || plan.model_id.is_some()
                || plan.provider.is_some()
            {
                return Err(format!(
                    "routing launch {} changes selecting-decision non-model plan",
                    launch.stage_id
                ));
            }
            if let Some(existing) = execution.stages.get(&launch.stage_id) {
                let persisted_contract_matches = if matches!(
                    view.dispatch_target,
                    ModelLaneRoutingDispatchTarget::LocalModel
                        | ModelLaneRoutingDispatchTarget::CloudModel
                ) {
                    existing.expected_run_id == launch.expected_run_id
                        && existing.expected_lane_id == launch.expected_lane_id
                        && existing.expected_model_id == launch.expected_model_id
                        && existing.expected_provider == launch.expected_provider
                } else {
                    existing.expected_run_id == launch.expected_run_id
                        && existing.expected_lane_id
                            == launch.authority_lane_id.as_deref().unwrap_or_default()
                        && existing.expected_model_id.is_empty()
                        && existing.expected_provider.is_none()
                };
                if !persisted_contract_matches {
                    return Err(format!(
                        "routing launch {} changes persisted provider/model/run contract",
                        launch.stage_id
                    ));
                }
            }
        }

        if matches!(
            execution.status,
            ModelLaneRoutingExecutionStatus::Succeeded
                | ModelLaneRoutingExecutionStatus::Failed
                | ModelLaneRoutingExecutionStatus::Cancelled
        ) {
            tx.commit().await.map_err(|err| err.to_string())?;
            return Ok(Vec::new());
        }

        let mut scheduled: Vec<String> = execution
            .stages
            .values()
            .filter(|stage| {
                stage.state == ModelLaneRoutingStageStateKind::Scheduled
                    && stage.lease_owner.is_none()
                    && stage.lease_expires_at_unix_ms.is_none()
            })
            .map(|stage| stage.stage_id.clone())
            .collect();
        for view in &views {
            if execution.stages.contains_key(&view.stage_id) || !is_ready(view, &execution.stages) {
                continue;
            }
            let mut input_refs = predecessor_output_refs(view, &execution.stages);
            if let Some(initial_input_ref) = execution.initial_input_ref.clone() {
                input_refs.push(initial_input_ref);
            }
            input_refs.sort();
            input_refs.dedup();
            let authority_ref = authority_for_stage(view, &execution.authority)?;
            let launch = launches
                .iter()
                .find(|launch| launch.stage_id == view.stage_id);
            let plan = execution
                .canonical_launch_plan
                .iter()
                .find(|plan| plan.stage_id == view.stage_id)
                .ok_or_else(|| format!("stage {} has no selecting-decision plan", view.stage_id))?;
            if matches!(
                view.dispatch_target,
                ModelLaneRoutingDispatchTarget::LocalModel
                    | ModelLaneRoutingDispatchTarget::CloudModel
            ) && launch.is_none()
            {
                return Err(format!(
                    "model routing stage {} has no canonical launch contract",
                    view.stage_id
                ));
            }
            let attempt = 1;
            let mut state = ModelLaneRoutingStageState {
                stage_id: view.stage_id.clone(),
                state: ModelLaneRoutingStageStateKind::Scheduled,
                attempt,
                dispatch_target: view.dispatch_target,
                expected_run_id: launch
                    .map(|value| value.expected_run_id.clone())
                    .unwrap_or_else(|| execution.run_id.clone()),
                expected_lane_id: plan.lane_id.clone().unwrap_or_default(),
                expected_model_id: plan.model_id.clone().unwrap_or_default(),
                expected_provider: plan.provider,
                instance_id: None,
                lane_id: None,
                input_refs: input_refs.clone(),
                output_ref: None,
                output_message_ref: None,
                authority_request_message_ref: None,
                output_sha256: None,
                output_payload: None,
                authority_ref: authority_ref.clone(),
                lease_owner: None,
                fencing_token: None,
                lease_expires_at_unix_ms: None,
                detail: None,
                event_ledger_event_id: String::new(),
                event_ledger_seq: 0,
                updated_at_unix_ms: now,
            };
            let idempotency = format!(
                "routing-stage-scheduled:{execution_id}:{}:{attempt}",
                view.stage_id
            );
            let stored = append_event(
                &mut tx,
                KernelEventType::ModelAdapterInvoked,
                "model_lane_routing_stage_attempt",
                &format!("{execution_id}:{}:{attempt}", view.stage_id),
                &idempotency,
                &execution.run_id,
                execution_id,
                json!({
                    "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                    "execution_id": execution_id,
                    "run_id": execution.run_id,
                    "trace_id": execution.trace_id,
                    "locus_ref": execution.locus_ref,
                    "stage_id": view.stage_id,
                    "attempt": attempt,
                    "dispatch_target": view.dispatch_target,
                    "expected_run_id": execution.run_id,
                    "expected_lane_id": plan.lane_id,
                    "expected_model_id": plan.model_id,
                    "expected_provider": plan.provider,
                    "state": "scheduled",
                    "authority_ref": authority_ref,
                    "input_refs": &input_refs,
                    "record": attempt_record_without_self_pointer(
                        serde_json::to_value(&state).map_err(|err| err.to_string())?
                    ),
                }),
            )
            .await?;
            for (index, input_ref) in input_refs.iter().enumerate() {
                let input_sha256 =
                    if execution.initial_input_ref.as_deref() == Some(input_ref.as_str()) {
                        execution.initial_input_sha256.clone()
                    } else {
                        execution.stages.values().find_map(|stage| {
                            (stage.output_ref.as_deref() == Some(input_ref.as_str()))
                                .then(|| stage.output_sha256.clone())
                                .flatten()
                        })
                    };
                append_event(
                    &mut tx,
                    KernelEventType::ContextBundleRecorded,
                    "model_lane_context_bundle_handoff",
                    &format!("{execution_id}:{}:{attempt}:{index}", view.stage_id),
                    &format!(
                        "routing-input-handoff:{execution_id}:{}:{attempt}:{index}",
                        view.stage_id
                    ),
                    &execution.run_id,
                    execution_id,
                    json!({
                        "schema_id": "hsk.model_lane_context_bundle_handoff@1",
                        "execution_id": execution_id,
                        "run_id": execution.run_id,
                        "trace_id": execution.trace_id,
                        "locus_ref": execution.locus_ref,
                        "stage_id": view.stage_id,
                        "attempt": attempt,
                        "input_ref": input_ref,
                        "input_sha256": input_sha256,
                        "authority_ref": &authority_ref,
                    }),
                )
                .await?;
            }
            state.event_ledger_event_id = stored.0;
            state.event_ledger_seq = stored.1;
            insert_attempt_and_outbox_tx(&mut tx, execution_id, &state, &execution).await?;
            execution.stages.insert(view.stage_id.clone(), state);
            scheduled.push(view.stage_id.clone());
        }

        let mut claims = Vec::new();
        for stage_id in scheduled {
            let Some(stage) = execution.stages.get(&stage_id) else {
                continue;
            };
            let attempt = stage.attempt;
            let dispatch_target = stage.dispatch_target.clone();
            let expected_run_id = stage.expected_run_id.clone();
            let expected_lane_id = stage.expected_lane_id.clone();
            let expected_model_id = stage.expected_model_id.clone();
            let expected_provider = stage.expected_provider;
            let lease_expires = now.saturating_add(self.lease_ms);
            let fencing_token = uuid::Uuid::now_v7().to_string();
            let claimed = sqlx::query(
                r#"UPDATE model_lane_routing_outbox outbox
                   SET status = 'claimed', lease_owner = $4, lease_expires_at_unix_ms = $5,
                       fencing_token = $6, updated_at_unix_ms = $7
                   WHERE outbox.command_id = (
                       SELECT pending.command_id
                       FROM model_lane_routing_outbox pending
                       WHERE pending.execution_id = $1
                         AND pending.stage_id = $2
                         AND pending.attempt = $3
                         AND pending.status = 'pending'
                       FOR UPDATE SKIP LOCKED
                       LIMIT 1
                   )
                   RETURNING outbox.command_id"#,
            )
            .bind(execution_id)
            .bind(&stage_id)
            .bind(i64::from(attempt))
            .bind(&self.lease_owner)
            .bind(lease_expires as i64)
            .bind(&fencing_token)
            .bind(now as i64)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| err.to_string())?;
            if claimed.is_none() {
                continue;
            }
            let mut claimed_stage = stage.clone();
            claimed_stage.state = ModelLaneRoutingStageStateKind::Claimed;
            claimed_stage.lease_owner = Some(self.lease_owner.clone());
            claimed_stage.fencing_token = Some(fencing_token.clone());
            claimed_stage.lease_expires_at_unix_ms = Some(lease_expires);
            claimed_stage.updated_at_unix_ms = now;
            let stored = append_event(
                &mut tx,
                KernelEventType::ModelAdapterInvoked,
                "model_lane_routing_stage_attempt",
                &format!("{execution_id}:{stage_id}:{attempt}"),
                &format!("routing-stage-claimed:{execution_id}:{stage_id}:{attempt}"),
                &execution.run_id,
                execution_id,
                json!({
                    "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                    "execution_id": execution_id,
                    "run_id": execution.run_id,
                    "trace_id": execution.trace_id,
                    "locus_ref": execution.locus_ref,
                    "stage_id": stage_id,
                    "attempt": attempt,
                    "state": "claimed",
                    "lease_owner": self.lease_owner,
                    "fencing_token": fencing_token,
                    "lease_expires_at_unix_ms": lease_expires,
                    "dispatch_target": dispatch_target,
                    "expected_run_id": expected_run_id,
                    "expected_lane_id": expected_lane_id,
                    "expected_model_id": expected_model_id,
                    "expected_provider": expected_provider,
                    "input_refs": stage.input_refs,
                    "authority_ref": stage.authority_ref,
                    "record": attempt_record_without_self_pointer(
                        serde_json::to_value(&claimed_stage).map_err(|err| err.to_string())?
                    ),
                }),
            )
            .await?;
            claimed_stage.event_ledger_event_id = stored.0;
            claimed_stage.event_ledger_seq = stored.1;
            execution
                .stages
                .insert(stage_id.clone(), claimed_stage.clone());
            persist_outbox_state_tx(&mut tx, &execution, &claimed_stage, "claimed").await?;
            claims.push(ModelLaneRoutingStageClaim {
                execution_id: execution_id.to_string(),
                stage_id: stage_id.clone(),
                attempt,
                fencing_token,
                lease_owner: self.lease_owner.clone(),
                lease_expires_at_unix_ms: lease_expires,
                dispatch_target,
                expected_run_id,
                expected_lane_id,
                expected_model_id,
                expected_provider,
            });
        }

        if !claims.is_empty() {
            execution.revision = execution.revision.saturating_add(1);
            execution.status = ModelLaneRoutingExecutionStatus::Running;
            let stored = append_event(
                &mut tx,
                KernelEventType::ModelAdapterInvoked,
                "model_lane_routing_execution",
                execution_id,
                &format!("routing-claim:{execution_id}:{}", execution.revision),
                &execution.run_id,
                execution_id,
                json!({"schema_id": ROUTING_EXECUTION_SCHEMA_ID, "record": execution}),
            )
            .await?;
            execution.event_ledger_event_id = stored.0;
            execution.event_ledger_seq = stored.1;
            save_execution_tx(&mut tx, &execution).await?;
            save_attempt_projections_tx(&mut tx, execution_id, &execution.stages).await?;
        }
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(claims)
    }

    pub(crate) async fn record_transition(
        &self,
        claim: &ModelLaneRoutingStageClaim,
        state: ModelLaneRoutingStageStateKind,
        instance_id: Option<String>,
        detail: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        self.record_stage_result(
            claim,
            state,
            instance_id,
            None,
            None,
            Vec::new(),
            None,
            None,
            None,
            None,
            detail,
        )
        .await
    }

    pub(crate) async fn heartbeat_claim(
        &self,
        claim: &ModelLaneRoutingStageClaim,
        state: ModelLaneRoutingStageStateKind,
        instance_id: Option<String>,
        lane_id: Option<String>,
        authority_request_message_ref: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        if !matches!(
            state,
            ModelLaneRoutingStageStateKind::InFlight
                | ModelLaneRoutingStageStateKind::AwaitingAuthority
        ) {
            return Err("routing heartbeat requires an active stage state".into());
        }
        self.record_stage_result(
            claim,
            state,
            instance_id,
            lane_id,
            authority_request_message_ref,
            Vec::new(),
            None,
            None,
            None,
            None,
            Some("routing claim lease heartbeat".into()),
        )
        .await
    }

    pub(crate) async fn validate_active_claim(
        &self,
        claim: &ModelLaneRoutingStageClaim,
    ) -> Result<(), String> {
        let snapshot = self
            .snapshot(&claim.execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {}", claim.execution_id))?;
        let stage = snapshot
            .stages
            .get(&claim.stage_id)
            .ok_or_else(|| format!("unknown routing stage {}", claim.stage_id))?;
        if stage.attempt != claim.attempt
            || stage.lease_owner.as_deref() != Some(claim.lease_owner.as_str())
            || stage.fencing_token.as_deref() != Some(claim.fencing_token.as_str())
            || stage.lease_expires_at_unix_ms.unwrap_or_default() < now_ms()
            || !matches!(
                stage.state,
                ModelLaneRoutingStageStateKind::InFlight
                    | ModelLaneRoutingStageStateKind::AwaitingAuthority
            )
        {
            return Err(format!(
                "stale routing claim rejected for {}/{}/attempt-{} before output side effects",
                claim.execution_id, claim.stage_id, claim.attempt
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn record_stage_result(
        &self,
        claim: &ModelLaneRoutingStageClaim,
        state: ModelLaneRoutingStageStateKind,
        instance_id: Option<String>,
        lane_id: Option<String>,
        authority_request_message_ref: Option<String>,
        input_refs: Vec<String>,
        output_ref: Option<String>,
        output_message_ref: Option<String>,
        output_sha256: Option<String>,
        output_payload: Option<Value>,
        detail: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        let execution = self
            .record_stage_result_tx(
                &mut tx,
                claim,
                state,
                instance_id,
                lane_id,
                authority_request_message_ref,
                input_refs,
                output_ref,
                output_message_ref,
                output_sha256,
                output_payload,
                detail,
            )
            .await?;
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(execution)
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_stage_result_tx(
        &self,
        mut tx: &mut Transaction<'_, Postgres>,
        claim: &ModelLaneRoutingStageClaim,
        state: ModelLaneRoutingStageStateKind,
        instance_id: Option<String>,
        lane_id: Option<String>,
        authority_request_message_ref: Option<String>,
        input_refs: Vec<String>,
        output_ref: Option<String>,
        output_message_ref: Option<String>,
        output_sha256: Option<String>,
        output_payload: Option<Value>,
        detail: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let execution_id = claim.execution_id.as_str();
        let stage_id = claim.stage_id.as_str();
        lock_execution(&mut tx, execution_id).await?;
        let mut execution = load_execution_tx(&mut tx, execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        let current = execution
            .stages
            .get(stage_id)
            .cloned()
            .ok_or_else(|| format!("stage {stage_id} was not durably scheduled"))?;
        if current.attempt != claim.attempt
            || current.lease_owner.as_deref() != Some(claim.lease_owner.as_str())
            || current.fencing_token.as_deref() != Some(claim.fencing_token.as_str())
            || current.lease_expires_at_unix_ms.unwrap_or_default() < now_ms()
        {
            return Err(format!(
                "stale routing claim rejected for {execution_id}/{stage_id}/attempt-{}",
                claim.attempt
            ));
        }
        if !matches!(
            state,
            ModelLaneRoutingStageStateKind::InFlight
                | ModelLaneRoutingStageStateKind::AwaitingAuthority
        ) && current.state == state
            && current.instance_id == instance_id
            && current.lane_id == lane_id
            && current.authority_request_message_ref == authority_request_message_ref
            && current.output_ref == output_ref
            && current.output_message_ref == output_message_ref
        {
            return Ok(execution);
        }
        if current.state != state && !valid_transition(current.state, state) {
            return Err(format!(
                "invalid routing stage transition {stage_id}: {:?} -> {:?}",
                current.state, state
            ));
        }
        if state.is_success() && output_ref.is_none() {
            return Err(format!(
                "successful stage {stage_id} requires a durable output ref"
            ));
        }
        if state.is_success()
            && current.dispatch_target != ModelLaneRoutingDispatchTarget::CoordinatorJoin
            && output_message_ref.is_none()
        {
            return Err(format!(
                "successful lane-backed stage {stage_id} requires a durable ModelLaneMessage ref"
            ));
        }
        if let Some(payload) = output_payload.as_ref() {
            let expected_hash = output_sha256
                .as_deref()
                .ok_or_else(|| format!("successful stage {stage_id} requires output_sha256"))?;
            if payload.get("artifact_sha256").and_then(Value::as_str) != Some(expected_hash) {
                return Err(format!(
                    "successful stage {stage_id} output pointer hash mismatch"
                ));
            }
        }
        let now = now_ms();
        let persisted_input_refs = if input_refs.is_empty() {
            current.input_refs.clone()
        } else {
            input_refs.clone()
        };
        let persisted_authority_request_message_ref = authority_request_message_ref
            .clone()
            .or_else(|| current.authority_request_message_ref.clone());
        let remains_active = matches!(
            state,
            ModelLaneRoutingStageStateKind::InFlight
                | ModelLaneRoutingStageStateKind::AwaitingAuthority
        );
        let mut next = ModelLaneRoutingStageState {
            stage_id: stage_id.to_string(),
            state,
            instance_id: instance_id.clone(),
            lane_id: lane_id.clone(),
            authority_request_message_ref: persisted_authority_request_message_ref.clone(),
            input_refs: persisted_input_refs.clone(),
            output_ref: output_ref.clone(),
            output_message_ref: output_message_ref.clone(),
            output_sha256: output_sha256.clone(),
            output_payload: output_payload.clone(),
            lease_owner: remains_active.then(|| self.lease_owner.clone()),
            fencing_token: remains_active.then(|| claim.fencing_token.clone()),
            lease_expires_at_unix_ms: remains_active.then(|| now.saturating_add(self.lease_ms)),
            detail: detail.clone(),
            event_ledger_event_id: current.event_ledger_event_id.clone(),
            event_ledger_seq: current.event_ledger_seq,
            updated_at_unix_ms: now,
            ..current.clone()
        };
        let idempotency = format!(
            "routing-stage-transition:{execution_id}:{stage_id}:{}:{state:?}:{}",
            current.attempt,
            execution.revision.saturating_add(1)
        );
        let stored = append_event(
            &mut tx,
            match state {
                ModelLaneRoutingStageStateKind::Succeeded
                | ModelLaneRoutingStageStateKind::Joined => KernelEventType::ModelResponseRecorded,
                ModelLaneRoutingStageStateKind::Failed
                | ModelLaneRoutingStageStateKind::Cancelled
                | ModelLaneRoutingStageStateKind::Compensated => KernelEventType::SessionFailed,
                ModelLaneRoutingStageStateKind::Scheduled
                | ModelLaneRoutingStageStateKind::Claimed
                | ModelLaneRoutingStageStateKind::InFlight
                | ModelLaneRoutingStageStateKind::AwaitingAuthority => {
                    KernelEventType::ModelAdapterInvoked
                }
            },
            "model_lane_routing_stage_attempt",
            &format!("{execution_id}:{stage_id}:{}", current.attempt),
            &idempotency,
            &execution.run_id,
            execution_id,
            json!({
                "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                "execution_id": execution_id,
                "run_id": execution.run_id,
                "trace_id": execution.trace_id,
                "locus_ref": execution.locus_ref,
                "stage_id": stage_id,
                "attempt": current.attempt,
                "fencing_token": claim.fencing_token,
                "state": state,
                "instance_id": instance_id,
                "lane_id": lane_id,
                "authority_request_message_ref": persisted_authority_request_message_ref,
                "input_refs": persisted_input_refs,
                "output_ref": output_ref,
                "output_message_ref": output_message_ref,
                "output_sha256": output_sha256,
                "output_payload": output_payload,
                "authority_ref": current.authority_ref,
                "dispatch_target": current.dispatch_target,
                "expected_run_id": current.expected_run_id,
                "expected_lane_id": current.expected_lane_id,
                "expected_model_id": current.expected_model_id,
                "expected_provider": current.expected_provider,
                "detail": detail,
                "record": attempt_record_without_self_pointer(
                    serde_json::to_value(&next).map_err(|err| err.to_string())?
                ),
            }),
        )
        .await?;
        next.event_ledger_event_id = stored.0;
        next.event_ledger_seq = stored.1;
        execution.stages.insert(stage_id.to_string(), next);
        execution.revision = execution.revision.saturating_add(1);
        refresh_execution_status(&mut execution)?;
        let stored_execution = append_event(
            &mut tx,
            match execution.status {
                ModelLaneRoutingExecutionStatus::Succeeded => KernelEventType::SessionCompleted,
                ModelLaneRoutingExecutionStatus::Failed
                | ModelLaneRoutingExecutionStatus::Cancelled => KernelEventType::SessionFailed,
                _ => KernelEventType::ModelResponseRecorded,
            },
            "model_lane_routing_execution",
            execution_id,
            &format!(
                "routing-execution-revision:{execution_id}:{}",
                execution.revision
            ),
            &execution.run_id,
            execution_id,
            json!({"schema_id": ROUTING_EXECUTION_SCHEMA_ID, "record": execution}),
        )
        .await?;
        execution.event_ledger_event_id = stored_execution.0;
        execution.event_ledger_seq = stored_execution.1;
        save_execution_tx(&mut tx, &execution).await?;
        save_attempt_projections_tx(&mut tx, execution_id, &execution.stages).await?;
        persist_outbox_state_tx(
            &mut tx,
            &execution,
            &execution.stages[stage_id],
            if remains_active {
                "claimed"
            } else if matches!(
                state,
                ModelLaneRoutingStageStateKind::Cancelled
                    | ModelLaneRoutingStageStateKind::Compensated
            ) {
                stage_kind_name(state)
            } else {
                "acked"
            },
        )
        .await?;
        Ok(execution)
    }

    pub(crate) async fn stage_input_envelope(
        &self,
        execution_id: &str,
        stage_id: &str,
    ) -> Result<String, String> {
        let execution = self
            .snapshot(execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        let view = stage_views(&execution.canonical_graph)?
            .into_iter()
            .find(|view| view.stage_id == stage_id)
            .ok_or_else(|| format!("unknown routing stage {stage_id}"))?;
        let model_lane_store = super::model_lane::ModelLaneStore::new_with_access(self.pool.clone(), self.access.clone());
        let mut predecessors = Vec::new();
        let mut predecessor_states = Vec::new();
        for dependency in &view.dependencies {
            let Some(stage) = execution.stages.get(dependency) else {
                continue;
            };
            predecessor_states.push(json!({
                "stage_id": stage.stage_id,
                "dispatch_target": stage.dispatch_target,
                "state": stage.state,
                "attempt": stage.attempt,
                "detail": stage.detail,
            }));
            let Some(output_ref) = stage.output_ref.as_deref() else {
                if matches!(
                    stage.state,
                    ModelLaneRoutingStageStateKind::Failed
                        | ModelLaneRoutingStageStateKind::Cancelled
                        | ModelLaneRoutingStageStateKind::Compensated
                ) {
                    continue;
                }
                return Err(format!(
                    "predecessor {dependency} has no artifact reference"
                ));
            };
            let projection = model_lane_store
                .navigation_by_artifact_or_context(Some(output_ref), None, Some(&execution.run_id))
                .await
                .map_err(|err| err.to_string())?;
            let artifact = projection
                .artifacts
                .iter()
                .find(|artifact| artifact.artifact_ref == output_ref)
                .ok_or_else(|| format!("predecessor artifact {output_ref} is missing"))?;
            if stage.output_sha256.as_deref() != Some(artifact.artifact_sha256.as_str())
                || canonical_sha256(&artifact.payload_json)? != artifact.artifact_sha256
            {
                return Err(format!(
                    "predecessor artifact {output_ref} hash binding mismatch"
                ));
            }
            predecessors.push(json!({
                "stage_id": stage.stage_id,
                "dispatch_target": stage.dispatch_target,
                "output_ref": output_ref,
                "output_message_ref": stage.output_message_ref,
                "output_sha256": artifact.artifact_sha256,
                "payload": artifact.payload_json,
            }));
        }
        let initial_message_id = execution
            .initial_input_ref
            .as_deref()
            .and_then(|value| value.strip_prefix("model-lane-message://"))
            .ok_or_else(|| {
                "execution initial input is not a ModelLaneMessage reference".to_string()
            })?;
        let initial_projection = model_lane_store
            .navigation_by_message(initial_message_id)
            .await
            .map_err(|err| err.to_string())?;
        let initial_message = initial_projection
            .messages
            .iter()
            .find(|message| message.message_id == initial_message_id)
            .ok_or_else(|| format!("initial message {initial_message_id} is missing"))?;
        let initial_artifact = initial_projection
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_ref == initial_message.payload_ref)
            .ok_or_else(|| {
                format!("initial message {initial_message_id} has no payload artifact")
            })?;
        if execution.initial_input_sha256.as_deref()
            != Some(initial_message.payload_sha256.as_str())
            || initial_artifact.artifact_sha256 != initial_message.payload_sha256
            || canonical_sha256(&initial_artifact.payload_json)? != initial_message.payload_sha256
        {
            return Err("initial input payload differs from selecting-decision binding".into());
        }
        serde_json::to_string(&json!({
            "schema_id": "hsk.model_lane_routing_stage_input@1",
            "execution_id": execution.execution_id,
            "run_id": execution.run_id,
            "trace_id": execution.trace_id,
            "locus_ref": execution.locus_ref,
            "stage_id": stage_id,
            "initial_input_ref": execution.initial_input_ref,
            "initial_input_sha256": execution.initial_input_sha256,
            "initial_input_payload": initial_artifact.payload_json,
            "predecessor_states": predecessor_states,
            "predecessor_outputs": predecessors,
        }))
        .map_err(|err| err.to_string())
    }

    pub(crate) async fn record_generated_output(
        &self,
        claim: &ModelLaneRoutingStageClaim,
        state: ModelLaneRoutingStageStateKind,
        instance_id: Option<String>,
        lane_id: Option<String>,
        output: String,
        detail: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let execution_id = claim.execution_id.as_str();
        let stage_id = claim.stage_id.as_str();
        if !matches!(
            state,
            ModelLaneRoutingStageStateKind::Succeeded | ModelLaneRoutingStageStateKind::Joined
        ) {
            return Err("generated output may only complete a successful or joined stage".into());
        }
        let snapshot = self
            .snapshot(execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        let stage = snapshot
            .stages
            .get(stage_id)
            .ok_or_else(|| format!("unknown routing stage {stage_id}"))?;
        self.validate_active_claim(claim).await?;
        let cloud_review = if stage_id == "cloud-review" {
            let parsed: CloudReviewOutput = serde_json::from_str(output.trim()).map_err(|err| {
                format!("cloud-review output must be a typed JSON verdict: {err}")
            })?;
            if parsed.review.trim().is_empty() {
                return Err("cloud-review output requires a nonblank review".into());
            }
            Some(parsed)
        } else {
            None
        };
        let predecessor_output_refs: Vec<String> = stage
            .input_refs
            .iter()
            .filter(|input_ref| snapshot.initial_input_ref.as_ref() != Some(*input_ref))
            .cloned()
            .collect();
        let typed_output = match stage_id {
            "cloud-review" => json!({
                "schema_id": "hsk.model_lane_cloud_review_verdict@1",
                "verdict": cloud_review.as_ref().map(|value| &value.verdict),
                "review": cloud_review.as_ref().map(|value| value.review.trim()),
            }),
            "debate-join" => deterministic_debate_adjudication(&output, &predecessor_output_refs)?,
            _ => json!({
                "schema_id": "hsk.model_lane_routing_proposal@1",
                "proposal": output,
            }),
        };
        let output_payload = json!({
            "schema_id": "hsk.model_lane_routing_output@1",
            "execution_id": execution_id,
            "run_id": snapshot.run_id,
            "trace_id": snapshot.trace_id,
            "locus_ref": snapshot.locus_ref,
            "stage_id": stage_id,
            "dispatch_target": stage.dispatch_target,
            "authority_ref": stage.authority_ref,
            "input_refs": stage.input_refs,
            "typed_output": typed_output,
        });
        let output_sha256 = canonical_sha256(&output_payload)?;
        let output_ref = format!(
            "artifact://model-lane-routing/{execution_id}/{stage_id}/{}/{output_sha256}",
            stage.attempt
        );
        let source_lane_id = lane_id
            .clone()
            .or_else(|| {
                snapshot
                    .stages
                    .values()
                    .find(|candidate| {
                        candidate.lane_id.is_some()
                            && candidate.output_ref.as_ref().is_some_and(|output_ref| {
                                stage
                                    .input_refs
                                    .iter()
                                    .any(|input_ref| input_ref == output_ref)
                            })
                    })
                    .and_then(|candidate| candidate.lane_id.clone())
            })
            .ok_or_else(|| {
                format!("routing output {stage_id} has no canonical source ModelLane")
            })?;
        let model_lane_store = super::model_lane::ModelLaneStore::new_with_access(self.pool.clone(), self.access.clone());
        let source_projection = model_lane_store
            .navigation_by_lane(&source_lane_id)
            .await
            .map_err(|err| err.to_string())?;
        let source_lane = source_projection
            .lanes
            .first()
            .ok_or_else(|| format!("routing output source lane {source_lane_id} missing"))?;
        let replay = model_lane_store
            .replay_run(&snapshot.run_id)
            .await
            .map_err(|err| err.to_string())?;
        let mut linked_span_contexts = Vec::new();
        for input_ref in &stage.input_refs {
            let message = if let Some(message_id) = input_ref.strip_prefix("model-lane-message://")
            {
                replay
                    .messages
                    .iter()
                    .find(|candidate| candidate.message_id == message_id)
            } else {
                replay
                    .messages
                    .iter()
                    .find(|candidate| candidate.payload_ref == *input_ref)
            }
            .ok_or_else(|| {
                format!("routing output {stage_id} input {input_ref} has no causal message")
            })?;
            linked_span_contexts.push(message.message_span_id.clone());
        }
        linked_span_contexts.sort();
        linked_span_contexts.dedup();
        if linked_span_contexts.is_empty() {
            return Err(format!(
                "routing output {stage_id} has no causal input span"
            ));
        }
        let message_id = format!("routing-output:{execution_id}:{stage_id}:{}", stage.attempt);
        let created_at_utc =
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(stage.updated_at_unix_ms as i64)
                .ok_or_else(|| format!("routing stage {stage_id} has invalid durable timestamp"))?
                .to_rfc3339();
        // Routing-stage output is an advisory proposal over a durable artifact,
        // not automatically a CRDT mutation. CRDT authority is carried only by
        // an explicit complete CRDT posture whose refs dereference to persisted
        // bytes; ordinary advisory proposals therefore keep every CRDT field
        // null instead of synthesizing `crdt-*://` identifiers.
        let message_kind = if stage_id == "cloud-review" {
            super::model_lane::ModelLaneMessageKind::Critique
        } else if state == ModelLaneRoutingStageStateKind::Joined {
            super::model_lane::ModelLaneMessageKind::Status
        } else {
            super::model_lane::ModelLaneMessageKind::Proposal
        };
        let message = super::model_lane::NewModelLaneMessage {
            message_id: message_id.clone(),
            run_id: snapshot.run_id.clone(),
            trace_id: snapshot.trace_id.clone(),
            message_span_id: format!(
                "routing-output-span:{execution_id}:{stage_id}:{}",
                stage.attempt
            ),
            parent_span_id: Some(snapshot.run_span_id.clone()),
            linked_span_contexts,
            from_lane_id: source_lane_id,
            to_lane: super::model_lane::ModelLaneTarget::Coordinator,
            routing: Some(super::model_lane::ModelLaneRoutingMetadata {
                target_role: "coordinator".into(),
                target_session: snapshot.coordinator_session_id.clone(),
                correlation_id: format!("routing:{execution_id}:{stage_id}"),
                requires_ack: false,
                ack_for: None,
            }),
            kind: message_kind,
            payload_ref: output_ref.clone(),
            payload_sha256: output_sha256.clone(),
            event_ledger_stream_id: source_lane.event_ledger_stream_id.clone(),
            summary: format!("canonical routing output for {stage_id}"),
            authority: super::model_lane::ModelLaneAuthority::Advisory,
            promotion_decision_id: None,
            promotion_gate_ref: None,
            promotion_receipt_ref: None,
            validator_verdict_ref: None,
            operator_decision_ref: None,
            promoted_artifact_ref: None,
            promoted_artifact_sha256: None,
            promoted_artifact_version: None,
            tool_gate_decision_refs: Vec::new(),
            coordinator_session_id: snapshot.coordinator_session_id.clone(),
            work_packet_id: Some(snapshot.work_packet_id.clone()),
            micro_task_id: snapshot.micro_task_id.clone(),
            task_board_id: Some(snapshot.task_board_id.clone()),
            owner_session: snapshot.owner_session.clone(),
            locus_binding: Some(super::model_lane::ModelLaneLocusBinding {
                work_packet_id: snapshot.work_packet_id.clone(),
                micro_task_id: snapshot
                    .micro_task_id
                    .clone()
                    .ok_or_else(|| format!("routing output {stage_id} requires micro_task_id"))?,
                task_board_id: Some(snapshot.task_board_id.clone()),
                coordinator_session_id: snapshot.coordinator_session_id.clone(),
                session_id: source_lane.session_id.clone(),
                model_session_id: source_lane.model_session_id.clone(),
                owner_session: snapshot.owner_session.clone(),
                locus_binding_ref: snapshot.locus_ref.clone(),
            }),
            idempotency_key: format!("routing-output:{execution_id}:{stage_id}:{}", stage.attempt),
            replay_order_key: format!(
                "routing/{execution_id}/{stage_id}/output/{:04}",
                stage.attempt
            ),
            replay_after_event_ledger_seq: Some(stage.event_ledger_seq),
            proposal_ref: None,
            crdt_update_ref: None,
            crdt_base_snapshot_ref: None,
            crdt_state_vector: None,
            crdt_proposal_ref: None,
            crdt_stale_base_ref: None,
            failstate_code: None,
            reason_ref: None,
            recovery_hint_ref: None,
            created_at_utc: created_at_utc.clone(),
            diagnostic_payload: output_payload.clone(),
        };
        let binding = super::model_lane::NewModelLaneContextBundleArtifactBinding {
            artifact_binding_id: format!(
                "routing-output-binding:{execution_id}:{stage_id}:{}",
                stage.attempt
            ),
            run_id: snapshot.run_id.clone(),
            trace_id: snapshot.trace_id.clone(),
            artifact_ref: output_ref.clone(),
            artifact_sha256: output_sha256.clone(),
            content_hash: output_sha256.clone(),
            artifact_kind: "model_lane_routing_output".into(),
            artifact_manifest_ref: format!(
                "manifest://model-lane-routing/{execution_id}/{stage_id}/{}",
                stage.attempt
            ),
            artifact_payload_ref: output_ref.clone(),
            payload_json: output_payload.clone(),
            event_ledger_stream_id: source_lane.event_ledger_stream_id.clone(),
            work_packet_id: snapshot.work_packet_id.clone(),
            micro_task_id: snapshot
                .micro_task_id
                .clone()
                .ok_or_else(|| format!("routing output {stage_id} requires micro_task_id"))?,
            task_board_id: snapshot.task_board_id.clone(),
            owner_session: snapshot.owner_session.clone(),
            idempotency_key: format!(
                "routing-output-binding:{execution_id}:{stage_id}:{}",
                stage.attempt
            ),
            created_at_utc,
            diagnostic_payload: json!({
                "execution_id": execution_id,
                "stage_id": stage_id,
                "attempt": stage.attempt,
                "fencing_token": claim.fencing_token,
            }),
        };
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        lock_execution(&mut tx, execution_id).await?;
        let locked_execution = load_execution_tx(&mut tx, execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        let locked_stage = locked_execution
            .stages
            .get(stage_id)
            .ok_or_else(|| format!("stage {stage_id} was not durably scheduled"))?;
        if locked_stage.attempt != claim.attempt
            || locked_stage.lease_owner.as_deref() != Some(claim.lease_owner.as_str())
            || locked_stage.fencing_token.as_deref() != Some(claim.fencing_token.as_str())
            || locked_stage.lease_expires_at_unix_ms.unwrap_or_default() < now_ms()
        {
            return Err(format!(
                "stale routing output claim rejected before artifact commit for {execution_id}/{stage_id}/attempt-{}",
                claim.attempt
            ));
        }
        // A per-attempt transaction-scoped fence makes the exact point between
        // final claim validation and projection writes externally observable
        // for deterministic race tests while remaining uncontended in normal
        // operation. The execution row lock is already held, so recovery or
        // reassignment cannot cross this barrier and create stale output rows.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!(
                "routing-output:{execution_id}:{stage_id}:{}",
                claim.attempt
            ))
            .execute(&mut *tx)
            .await
            .map_err(|err| err.to_string())?;
        let output_message_id = if stage.dispatch_target
            == ModelLaneRoutingDispatchTarget::CoordinatorJoin
        {
            super::model_lane::ModelLaneStore::record_context_bundle_artifact_binding_with_validation_tx(
                &mut tx, binding, self.access.insert_columns(),
            )
            .await
            .map_err(|err| err.to_string())?;
            None
        } else {
            Some(
                super::model_lane::ModelLaneStore::record_message_with_payload_binding_tx(
                    &mut tx,
                    message,
                    binding,
                    self.access.insert_columns(),
                )
                .await
                .map_err(|err| err.to_string())?
                .message_id
                .clone(),
            )
        };
        let bounded_typed_metadata = match stage_id {
            "cloud-review" => json!({
                "schema_id": "hsk.model_lane_cloud_review_verdict@1",
                "verdict": typed_output.get("verdict"),
                "review_present": true,
            }),
            "debate-join" => json!({
                "schema_id": "hsk.model_lane_parallel_debate_adjudication@1",
                "decision": typed_output.get("decision"),
                "rationale": typed_output.get("rationale"),
                "selected_output_ref": typed_output.get("selected_output_ref"),
            }),
            _ => json!({
                "schema_id": "hsk.model_lane_routing_proposal_metadata@1",
                "content_bytes": output.as_bytes().len(),
            }),
        };
        let bounded_projection = json!({
            "schema_id": "hsk.model_lane_routing_output_pointer@1",
            "artifact_ref": output_ref,
            "message_ref": output_message_id,
            "artifact_sha256": output_sha256,
            "typed_output": bounded_typed_metadata,
        });
        let execution = self
            .record_stage_result_tx(
                &mut tx,
                claim,
                state,
                instance_id,
                lane_id,
                None,
                stage.input_refs.clone(),
                Some(output_ref),
                output_message_id,
                Some(output_sha256),
                Some(bounded_projection),
                detail,
            )
            .await?;
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(execution)
    }

    pub(crate) async fn complete_authority_stage(
        &self,
        claim: &ModelLaneRoutingStageClaim,
        authority_ref: &str,
        output_ref: String,
        output_message_ref: String,
        output_sha256: String,
        output_payload: Value,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let execution_id = claim.execution_id.as_str();
        let stage_id = claim.stage_id.as_str();
        let snapshot = self
            .snapshot(execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        let stage = snapshot
            .stages
            .get(stage_id)
            .ok_or_else(|| format!("unknown routing stage {stage_id}"))?;
        if stage.state != ModelLaneRoutingStageStateKind::AwaitingAuthority {
            return Err(format!("stage {stage_id} is not awaiting authority"));
        }
        if stage.authority_ref.as_deref() != Some(authority_ref) {
            return Err(format!("authority ref does not match stage {stage_id}"));
        }
        if canonical_sha256(&output_payload)? != output_sha256 {
            return Err(format!(
                "authority output hash mismatch for stage {stage_id}"
            ));
        }
        self.record_stage_result(
            claim,
            ModelLaneRoutingStageStateKind::Succeeded,
            stage.instance_id.clone(),
            stage.lane_id.clone(),
            stage.authority_request_message_ref.clone(),
            stage.input_refs.clone(),
            Some(output_ref.clone()),
            Some(output_message_ref.clone()),
            Some(output_sha256.clone()),
            Some(json!({
                "schema_id": "hsk.model_lane_routing_output_pointer@1",
                "artifact_ref": output_ref,
                "message_ref": output_message_ref,
                "artifact_sha256": output_sha256,
                "typed_output": {
                    "schema_id": "hsk.model_lane_routing_authority_response_metadata@1",
                    "verdict": output_payload.pointer("/diagnostic_payload/verdict")
                        .or_else(|| output_payload.get("verdict")),
                },
            })),
            None,
        )
        .await
    }

    pub(crate) async fn recover_expired_claims(
        &self,
        execution_id: &str,
    ) -> Result<Vec<String>, String> {
        let now = now_ms();
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        lock_execution(&mut tx, execution_id).await?;
        let mut execution = load_execution_tx(&mut tx, execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        let stage_ids: Vec<String> = execution.stages.keys().cloned().collect();
        let mut recovered = Vec::new();
        let mut changed = false;
        for stage_id in stage_ids {
            let current = execution.stages[&stage_id].clone();
            if !matches!(
                current.state,
                ModelLaneRoutingStageStateKind::Claimed
                    | ModelLaneRoutingStageStateKind::InFlight
                    | ModelLaneRoutingStageStateKind::AwaitingAuthority
            ) || current.lease_expires_at_unix_ms.unwrap_or(u64::MAX) > now
            {
                continue;
            }
            let mut compensated = current.clone();
            compensated.state = ModelLaneRoutingStageStateKind::Compensated;
            compensated.lease_owner = None;
            compensated.fencing_token = None;
            compensated.lease_expires_at_unix_ms = None;
            compensated.detail = Some("compensated after expired lease".into());
            compensated.updated_at_unix_ms = now;
            let compensation = append_event(
                &mut tx,
                KernelEventType::SessionFailed,
                "model_lane_routing_stage_attempt",
                &format!("{execution_id}:{stage_id}:{}", current.attempt),
                &format!(
                    "routing-stage-compensated:{execution_id}:{stage_id}:{}",
                    current.attempt
                ),
                &execution.run_id,
                execution_id,
                json!({
                    "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                    "execution_id": execution_id,
                    "stage_id": stage_id,
                    "attempt": current.attempt,
                    "state": "compensated",
                    "compensation": "expired_lease",
                    "superseded_fencing_token": current.fencing_token,
                    "dispatch_target": current.dispatch_target,
                    "expected_run_id": current.expected_run_id,
                    "expected_lane_id": current.expected_lane_id,
                    "expected_model_id": current.expected_model_id,
                    "expected_provider": current.expected_provider,
                    "input_refs": current.input_refs,
                    "authority_ref": current.authority_ref,
                    "record": attempt_record_without_self_pointer(
                        serde_json::to_value(&compensated).map_err(|err| err.to_string())?
                    ),
                }),
            )
            .await?;
            compensated.event_ledger_event_id = compensation.0;
            compensated.event_ledger_seq = compensation.1;
            sqlx::query(
                "UPDATE model_lane_routing_outbox SET status='compensated', lease_owner=NULL, fencing_token=NULL, lease_expires_at_unix_ms=NULL, updated_at_unix_ms=$4 WHERE execution_id=$1 AND stage_id=$2 AND attempt=$3",
            )
            .bind(execution_id)
            .bind(&stage_id)
            .bind(i64::from(current.attempt))
            .bind(now as i64)
            .execute(&mut *tx)
            .await
            .map_err(|err| err.to_string())?;
            persist_outbox_state_tx(&mut tx, &execution, &compensated, "compensated").await?;
            sqlx::query(
                "UPDATE model_lane_routing_stage_attempts SET status='compensated', lease_owner=NULL, fencing_token=NULL, lease_expires_at_unix_ms=NULL, event_ledger_event_id=$4, event_ledger_seq=$5, record_json=$6, updated_at_unix_ms=$7 WHERE execution_id=$1 AND stage_id=$2 AND attempt=$3",
            )
            .bind(execution_id)
            .bind(&stage_id)
            .bind(i64::from(current.attempt))
            .bind(&compensated.event_ledger_event_id)
            .bind(compensated.event_ledger_seq)
            .bind(serde_json::to_value(&compensated).map_err(|err| err.to_string())?)
            .bind(now as i64)
            .execute(&mut *tx)
            .await
            .map_err(|err| err.to_string())?;

            if current.attempt >= MAX_STAGE_ATTEMPTS {
                let mut failed = current.clone();
                failed.state = ModelLaneRoutingStageStateKind::Failed;
                failed.detail = Some("routing stage exhausted bounded recovery attempts".into());
                failed.lease_owner = None;
                failed.fencing_token = None;
                failed.lease_expires_at_unix_ms = None;
                failed.updated_at_unix_ms = now;
                let stored = append_event(
                    &mut tx,
                    KernelEventType::SessionFailed,
                    "model_lane_routing_stage_attempt",
                    &format!("{execution_id}:{stage_id}:{}", current.attempt),
                    &format!(
                        "routing-stage-exhausted:{execution_id}:{stage_id}:{}",
                        current.attempt
                    ),
                    &execution.run_id,
                    execution_id,
                    json!({
                        "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                        "execution_id": execution_id,
                        "stage_id": stage_id,
                        "attempt": current.attempt,
                        "state": "failed",
                        "compensation": "expired_lease",
                        "reason": "bounded_recovery_exhausted",
                        "dispatch_target": current.dispatch_target,
                        "expected_run_id": current.expected_run_id,
                        "expected_lane_id": current.expected_lane_id,
                        "expected_model_id": current.expected_model_id,
                        "expected_provider": current.expected_provider,
                        "input_refs": current.input_refs,
                        "authority_ref": current.authority_ref,
                        "record": attempt_record_without_self_pointer(
                            serde_json::to_value(&failed).map_err(|err| err.to_string())?
                        ),
                    }),
                )
                .await?;
                failed.event_ledger_event_id = stored.0;
                failed.event_ledger_seq = stored.1;
                persist_outbox_state_tx(&mut tx, &execution, &failed, "acked").await?;
                execution.stages.insert(stage_id, failed);
                changed = true;
                continue;
            }

            let next_attempt = current.attempt + 1;
            let mut next = current.clone();
            next.attempt = next_attempt;
            next.state = ModelLaneRoutingStageStateKind::Scheduled;
            next.instance_id = None;
            next.lane_id = None;
            next.output_ref = None;
            next.output_message_ref = None;
            next.authority_request_message_ref = None;
            next.output_sha256 = None;
            next.output_payload = None;
            next.lease_owner = None;
            next.fencing_token = None;
            next.lease_expires_at_unix_ms = None;
            next.detail = Some("reclaimed after expired lease and compensation".into());
            next.updated_at_unix_ms = now;
            let stored = append_event(
                &mut tx,
                KernelEventType::ModelAdapterInvoked,
                "model_lane_routing_stage_attempt",
                &format!("{execution_id}:{stage_id}:{next_attempt}"),
                &format!("routing-stage-recovered:{execution_id}:{stage_id}:{next_attempt}"),
                &execution.run_id,
                execution_id,
                json!({
                    "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                    "execution_id": execution_id,
                    "stage_id": stage_id,
                    "attempt": next_attempt,
                    "state": "scheduled",
                    "compensated_attempt": current.attempt,
                    "requeue_reason": "expired_lease",
                    "dispatch_target": current.dispatch_target,
                    "expected_run_id": current.expected_run_id,
                    "expected_lane_id": current.expected_lane_id,
                    "expected_model_id": current.expected_model_id,
                    "expected_provider": current.expected_provider,
                    "input_refs": current.input_refs,
                    "authority_ref": current.authority_ref,
                    "record": attempt_record_without_self_pointer(
                        serde_json::to_value(&next).map_err(|err| err.to_string())?
                    ),
                }),
            )
            .await?;
            next.event_ledger_event_id = stored.0;
            next.event_ledger_seq = stored.1;
            insert_attempt_and_outbox_tx(&mut tx, execution_id, &next, &execution).await?;
            recovered.push(stage_id.clone());
            execution.stages.insert(stage_id, next);
            changed = true;
        }
        if changed {
            execution.revision = execution.revision.saturating_add(1);
            refresh_execution_status(&mut execution)?;
            let stored = append_event(
                &mut tx,
                KernelEventType::ModelResponseRecorded,
                "model_lane_routing_execution",
                execution_id,
                &format!("routing-recovery:{execution_id}:{}", execution.revision),
                &execution.run_id,
                execution_id,
                json!({"schema_id": ROUTING_EXECUTION_SCHEMA_ID, "record": execution}),
            )
            .await?;
            execution.event_ledger_event_id = stored.0;
            execution.event_ledger_seq = stored.1;
            save_execution_tx(&mut tx, &execution).await?;
            save_attempt_projections_tx(&mut tx, execution_id, &execution.stages).await?;
        }
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(recovered)
    }

    pub(crate) async fn expired_stage_instance_ids(
        &self,
        execution_id: &str,
    ) -> Result<Vec<String>, String> {
        let now = now_ms();
        let execution = self
            .snapshot(execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        Ok(execution
            .stages
            .values()
            .filter(|stage| {
                matches!(
                    stage.state,
                    ModelLaneRoutingStageStateKind::Claimed
                        | ModelLaneRoutingStageStateKind::InFlight
                        | ModelLaneRoutingStageStateKind::AwaitingAuthority
                ) && stage
                    .lease_expires_at_unix_ms
                    .is_some_and(|expiry| expiry <= now)
            })
            .filter_map(|stage| stage.instance_id.clone())
            .collect())
    }
    pub(crate) async fn cancel_execution(
        &self,
        execution_id: &str,
        reason: impl Into<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let reason = reason.into();
        let mut tx = self.pool.begin().await.map_err(|err| err.to_string())?;
        lock_execution(&mut tx, execution_id).await?;
        let mut execution = load_execution_tx(&mut tx, execution_id)
            .await?
            .ok_or_else(|| format!("unknown routing execution {execution_id}"))?;
        if matches!(
            execution.status,
            ModelLaneRoutingExecutionStatus::Succeeded | ModelLaneRoutingExecutionStatus::Failed
        ) {
            return Err("terminal routing execution cannot be cancelled".into());
        }
        let now = now_ms();
        let cancellable: Vec<String> = execution
            .stages
            .values()
            .filter(|stage| !stage.state.is_terminal())
            .map(|stage| stage.stage_id.clone())
            .collect();
        for stage_id in cancellable {
            let current = execution.stages[&stage_id].clone();
            let mut cancelled_stage = current.clone();
            cancelled_stage.state = ModelLaneRoutingStageStateKind::Cancelled;
            cancelled_stage.detail = Some(reason.clone());
            cancelled_stage.lease_owner = None;
            cancelled_stage.fencing_token = None;
            cancelled_stage.lease_expires_at_unix_ms = None;
            cancelled_stage.updated_at_unix_ms = now;
            let stored_stage = append_event(
                &mut tx,
                KernelEventType::SessionFailed,
                "model_lane_routing_stage_attempt",
                &format!("{execution_id}:{stage_id}:{}", current.attempt),
                &format!(
                    "routing-stage-cancel:{execution_id}:{stage_id}:{}",
                    current.attempt
                ),
                &execution.run_id,
                execution_id,
                json!({
                    "schema_id": ROUTING_STAGE_ATTEMPT_SCHEMA_ID,
                    "execution_id": execution_id,
                    "stage_id": stage_id,
                    "attempt": current.attempt,
                    "fencing_token": current.fencing_token,
                    "state": "cancelled",
                    "reason": reason,
                    "dispatch_target": current.dispatch_target,
                    "expected_run_id": current.expected_run_id,
                    "expected_lane_id": current.expected_lane_id,
                    "expected_model_id": current.expected_model_id,
                    "expected_provider": current.expected_provider,
                    "input_refs": current.input_refs,
                    "authority_ref": current.authority_ref,
                    "record": attempt_record_without_self_pointer(
                        serde_json::to_value(&cancelled_stage).map_err(|err| err.to_string())?
                    ),
                }),
            )
            .await?;
            cancelled_stage.event_ledger_event_id = stored_stage.0;
            cancelled_stage.event_ledger_seq = stored_stage.1;
            persist_outbox_state_tx(&mut tx, &execution, &cancelled_stage, "cancelled").await?;
            execution.stages.insert(stage_id, cancelled_stage);
        }
        execution.status = ModelLaneRoutingExecutionStatus::Cancelled;
        execution.cancel_reason = Some(reason);
        execution.revision = execution.revision.saturating_add(1);
        let stored = append_event(
            &mut tx,
            KernelEventType::SessionFailed,
            "model_lane_routing_execution",
            execution_id,
            &format!("routing-cancel:{execution_id}"),
            &execution.run_id,
            execution_id,
            json!({"schema_id": ROUTING_EXECUTION_SCHEMA_ID, "record": execution}),
        )
        .await?;
        execution.event_ledger_event_id = stored.0;
        execution.event_ledger_seq = stored.1;
        save_execution_tx(&mut tx, &execution).await?;
        save_attempt_projections_tx(&mut tx, execution_id, &execution.stages).await?;
        tx.commit().await.map_err(|err| err.to_string())?;
        Ok(execution)
    }
}

fn validate_launch_plan(
    graph: &ModelLaneRoutingGraph,
    plan: &[ModelLaneRoutingStageLaunchPlan],
) -> Result<(), String> {
    if plan.len() != graph.stages.len() {
        return Err(
            "selecting decision launch plan must cover every canonical stage exactly once".into(),
        );
    }
    let mut seen = std::collections::BTreeSet::new();
    for stage in &graph.stages {
        let entry = plan
            .iter()
            .find(|entry| entry.stage_id == stage.stage_id)
            .ok_or_else(|| format!("selecting decision launch plan omits {}", stage.stage_id))?;
        if !seen.insert(entry.stage_id.as_str()) || entry.dispatch_target != stage.target {
            return Err(format!(
                "selecting decision launch plan conflicts with {}",
                stage.stage_id
            ));
        }
        match stage.target {
            ModelLaneRoutingDispatchTarget::LocalModel
            | ModelLaneRoutingDispatchTarget::CloudModel => {
                if entry.lane_id.as_deref().map_or(true, str::is_empty)
                    || entry.model_id.as_deref().map_or(true, str::is_empty)
                {
                    return Err(format!(
                        "model stage {} requires planned lane and model",
                        stage.stage_id
                    ));
                }
                if stage.target == ModelLaneRoutingDispatchTarget::CloudModel
                    && !matches!(
                        entry.provider,
                        Some(
                            crate::model_runtime::ProviderKind::ByokCloud
                                | crate::model_runtime::ProviderKind::OfficialCli
                        )
                    )
                {
                    return Err(format!(
                        "cloud stage {} requires a cloud provider plan",
                        stage.stage_id
                    ));
                }
            }
            ModelLaneRoutingDispatchTarget::Validator
            | ModelLaneRoutingDispatchTarget::Operator => {
                if entry.lane_id.as_deref().map_or(true, str::is_empty)
                    || entry.model_id.is_some()
                    || entry.provider.is_some()
                {
                    return Err(format!(
                        "authority stage {} requires only its authority lane",
                        stage.stage_id
                    ));
                }
            }
            ModelLaneRoutingDispatchTarget::CoordinatorJoin => {
                if entry.lane_id.is_some() || entry.model_id.is_some() || entry.provider.is_some() {
                    return Err(format!(
                        "coordinator join {} cannot carry a launch",
                        stage.stage_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn deterministic_debate_adjudication(
    envelope: &str,
    predecessor_refs: &[String],
) -> Result<Value, String> {
    let parsed: Value = serde_json::from_str(envelope)
        .map_err(|err| format!("debate join requires canonical input envelope: {err}"))?;
    let predecessors = parsed
        .get("predecessor_outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "debate join input has no predecessor outputs".to_string())?;
    if predecessors.len() != 2 {
        return Err("debate adjudication requires exactly two predecessor outputs".into());
    }
    let mut candidates = Vec::new();
    for predecessor in predecessors {
        let reference = predecessor
            .get("output_ref")
            .and_then(Value::as_str)
            .ok_or_else(|| "debate predecessor has no output_ref".to_string())?;
        let proposal = predecessor
            .pointer("/payload/typed_output/proposal")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("debate predecessor {reference} has no typed proposal"))?;
        if proposal.trim().is_empty() {
            return Err(format!(
                "debate predecessor {reference} has a blank proposal"
            ));
        }
        candidates.push((reference.to_string(), proposal.trim().to_string()));
    }
    let same = candidates[0].1.eq_ignore_ascii_case(&candidates[1].1);
    candidates.sort_by(|left, right| {
        canonical_sha256(&Value::String(left.1.clone()))
            .unwrap_or_default()
            .cmp(&canonical_sha256(&Value::String(right.1.clone())).unwrap_or_default())
            .then_with(|| left.0.cmp(&right.0))
    });
    let selected = &candidates[0];
    Ok(json!({
        "schema_id": "hsk.model_lane_parallel_debate_adjudication@1",
        "decision": if same { "consensus" } else { "selected_canonical_candidate" },
        "rationale": if same {
            "both proposals are textually equivalent after trimming and case folding"
        } else {
            "proposals conflict; selected the lowest canonical SHA-256 with artifact-ref tie-break"
        },
        "selected_output_ref": selected.0,
        "selected_output": selected.1,
        "predecessor_refs": predecessor_refs,
    }))
}

async fn validate_initial_input_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    input_ref: &str,
    expected_sha256: &str,
) -> Result<(), String> {
    let message_id = input_ref
        .strip_prefix("model-lane-message://")
        .ok_or_else(|| "initial input must be a model-lane-message:// reference".to_string())?;
    let row = sqlx::query(
        r#"SELECT message.record_json AS message_json, message_ledger.payload AS message_payload,
                  artifact.record_json AS artifact_json, artifact_ledger.payload AS artifact_payload
           FROM model_lane_messages message
           JOIN kernel_event_ledger message_ledger
             ON message_ledger.event_id=message.event_ledger_event_id
            AND message_ledger.event_sequence=message.event_ledger_seq
            AND message_ledger.aggregate_type='model_lane_message'
            AND message_ledger.aggregate_id=message.message_id
           JOIN model_lane_context_bundle_artifacts artifact
             ON artifact.run_id=message.run_id
            AND artifact.artifact_ref=message.record_json->>'payload_ref'
           JOIN kernel_event_ledger artifact_ledger
             ON artifact_ledger.event_id=artifact.event_ledger_event_id
            AND artifact_ledger.event_sequence=artifact.event_ledger_seq
            AND artifact_ledger.aggregate_type='model_lane_context_bundle_artifact'
            AND artifact_ledger.aggregate_id=artifact.artifact_binding_id
           WHERE message.message_id=$1 AND message.run_id=$2"#,
    )
    .bind(message_id)
    .bind(run_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| err.to_string())?
    .ok_or_else(|| {
        format!("selected initial input {input_ref} has no canonical message/artifact lineage")
    })?;
    let message_json: Value = row.get("message_json");
    let message_payload: Value = row.get("message_payload");
    let artifact_json: Value = row.get("artifact_json");
    let artifact_payload: Value = row.get("artifact_payload");
    let bound_hash = message_json.get("payload_sha256").and_then(Value::as_str);
    // The MODEL_RESPONSE_RECORDED EventLedger payload stores `crdt_authority_binding`
    // as a SIBLING of `/record` (see `record_message_tx`), while the mutable
    // `record_json` projection nests it inside the record object. Compare the record
    // body without that field, then verify the binding against the ledger sibling
    // separately, mirroring the diagnostics-projection drift check. Without this a
    // CRDT-bearing initial input (`crdt_authority_binding` = Some) is falsely rejected
    // as a hash/EventLedger binding mismatch; the crdt binding is still fully asserted.
    let mut canonical_message_record = record_without_generated_event_fields(message_json.clone());
    let row_crdt_binding = canonical_message_record
        .as_object_mut()
        .and_then(|record| record.remove("crdt_authority_binding"))
        .unwrap_or(Value::Null);
    let ledger_crdt_binding = message_payload
        .get("crdt_authority_binding")
        .cloned()
        .unwrap_or(Value::Null);
    if message_payload.pointer("/record") != Some(&canonical_message_record)
        || row_crdt_binding != ledger_crdt_binding
        || artifact_payload.pointer("/record") != Some(&artifact_json)
        || bound_hash != Some(expected_sha256)
        || artifact_json.get("artifact_sha256").and_then(Value::as_str) != Some(expected_sha256)
        || canonical_sha256(
            artifact_json
                .get("payload_json")
                .ok_or_else(|| "initial input artifact has no payload_json".to_string())?,
        )? != expected_sha256
    {
        return Err(format!(
            "selected initial input {input_ref} hash/EventLedger binding mismatch"
        ));
    }
    Ok(())
}

fn initial_execution(
    execution_id: &str,
    canonical_graph: Value,
    graph_hash: String,
    authority: &ModelLaneRoutingAuthority,
    context: ModelLaneRoutingExecutionContext,
    selecting_decision_id: &str,
    selecting_decision_event_id: &str,
    selecting_decision_event_seq: i64,
    canonical_launch_plan: Vec<ModelLaneRoutingStageLaunchPlan>,
    canonical_launch_plan_sha256: String,
) -> ModelLaneRoutingExecutionState {
    ModelLaneRoutingExecutionState {
        schema_id: ROUTING_EXECUTION_SCHEMA_ID.into(),
        execution_id: execution_id.into(),
        run_id: context.run_id,
        selecting_decision_id: selecting_decision_id.into(),
        selecting_decision_event_id: selecting_decision_event_id.into(),
        selecting_decision_event_seq,
        trace_id: context.trace_id,
        run_span_id: context.run_span_id,
        coordinator_session_id: context.coordinator_session_id,
        locus_ref: context.locus_ref,
        work_packet_id: context.work_packet_id,
        micro_task_id: context.micro_task_id,
        task_board_id: context.task_board_id,
        owner_session: context.owner_session,
        canonical_graph,
        canonical_graph_sha256: graph_hash,
        canonical_launch_plan,
        canonical_launch_plan_sha256,
        authority: authority.clone(),
        initial_input_ref: Some(context.initial_input_ref),
        initial_input_sha256: Some(context.initial_input_sha256),
        status: ModelLaneRoutingExecutionStatus::Running,
        failure_reason: None,
        cancel_reason: None,
        revision: 0,
        stages: BTreeMap::new(),
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
    }
}

/// How long a routing-execution advisory acquisition may wait before failing
/// loudly. Long enough that ordinary contention (a concurrent stage transition
/// on the same execution) still succeeds on a loaded host, short enough that an
/// operator cancel reports instead of hanging. Advisory locks are invisible to
/// PostgreSQL's deadlock detector, so this bound is the ONLY thing that breaks a
/// cycle on this path.
const ROUTING_EXECUTION_LOCK_TIMEOUT: &str = "15s";

async fn lock_execution(
    tx: &mut Transaction<'_, Postgres>,
    execution_id: &str,
) -> Result<(), String> {
    // Transaction-local call-site marker; see the matching note in
    // model_lane.rs::record_or_extend_run_tx. Both sites take the SAME salt-0
    // keyspace, so naming the holder is what distinguishes a cross-site
    // collision from a re-entrant self-block in pg_stat_activity.
    //
    // lock_execution has EIGHT callers, so the bare marker proves a self-
    // collision on the execution key without naming WHICH two callers collide.
    // Setting HANDSHAKE_LOCK_TRACE=1 appends the nearest caller frame. The
    // backtrace is captured ONLY under that env var, so production pays
    // nothing: this is a diagnostic seam, not an always-on cost.
    // NOTE: application_name is capped at NAMEDATALEN-1 (63 bytes). A marker of
    // the form hsk:lock_execution:<caller>:<execution_id> is silently truncated
    // mid-execution_id, which hides the very field being added. The execution
    // id is already known from the test under inspection, so the marker carries
    // the CALLER only and stays well inside the cap.
    // The TASK id is the discriminator that matters here, and it is far more
    // reliable than a symbol name: on Windows/MSVC every async frame symbolises
    // as `async_fn$0`, so a backtrace cannot name the calling function at all.
    // If the holder and a waiter share a task id, one logical operation holds
    // this key on one pooled connection while requesting it on another - a true
    // re-entrant self-deadlock. Distinct task ids mean cross-task lock-ordering
    // contention instead. Those two faults have different fixes.
    let task = tokio::task::try_id()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "none".to_string());
    sqlx::query("SELECT set_config('application_name', $1, true)")
        .bind(format!("hsk:le:task{task}"))
        .execute(&mut **tx)
        .await
        .map_err(|err| err.to_string())?;
    // BOUND the acquisition. An unbounded pg_advisory_xact_lock on the routing
    // terminal path means an operator cancel can block forever: PostgreSQL does
    // NOT run deadlock detection over advisory locks, so a cycle here is never
    // broken by the database. Re-entrant acquisition (the same task holding this
    // key on one pooled connection while requesting it on another) was proven
    // for this keyspace via task-id markers, and it hung indefinitely.
    //
    // Bounding turns that from an unrecoverable hang into a typed, reportable
    // failure that names the execution and the holder. `SET LOCAL` semantics
    // (`true`) confine the timeout to this transaction. Precedent in-repo:
    // model_runtime/registry_persistence.rs bounds its advisory acquisition the
    // same way.
    sqlx::query("SELECT set_config('lock_timeout', $1, true)")
        .bind(ROUTING_EXECUTION_LOCK_TIMEOUT)
        .execute(&mut **tx)
        .await
        .map_err(|err| err.to_string())?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(execution_id)
        .execute(&mut **tx)
        .await
        .map_err(|err| {
            format!(
                "routing execution advisory lock for {execution_id} was not granted within {ROUTING_EXECUTION_LOCK_TIMEOUT} \
                 (holder identifies itself in pg_stat_activity.application_name as hsk:le:task<id>; the SAME task id on \
                 both holder and waiter means re-entrant acquisition, a different id means cross-task lock ordering): {err}"
            )
        })?;
    Ok(())
}

async fn load_execution_tx(
    tx: &mut Transaction<'_, Postgres>,
    execution_id: &str,
) -> Result<Option<ModelLaneRoutingExecutionState>, String> {
    let row = sqlx::query(
        r#"SELECT routing.record_json, routing.graph_sha256,
                  ledger.aggregate_type, ledger.aggregate_id, ledger.payload
           FROM model_lane_routing_executions routing
           LEFT JOIN kernel_event_ledger ledger
             ON ledger.event_id = routing.event_ledger_event_id
            AND ledger.event_sequence = routing.event_ledger_seq
           WHERE routing.execution_id = $1
           FOR UPDATE OF routing"#,
    )
    .bind(execution_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    let Some(row) = row else { return Ok(None) };
    let record_json: Value = row.get("record_json");
    let state: ModelLaneRoutingExecutionState =
        serde_json::from_value(record_json.clone()).map_err(|err| err.to_string())?;
    let graph_hash: String = row.get("graph_sha256");
    let aggregate_type: Option<String> = row.get("aggregate_type");
    let aggregate_id: Option<String> = row.get("aggregate_id");
    let payload: Option<Value> = row.get("payload");
    if aggregate_type.as_deref() != Some("model_lane_routing_execution")
        || aggregate_id.as_deref() != Some(execution_id)
        || payload
            .as_ref()
            .and_then(|value| value.pointer("/record"))
            .cloned()
            .map(execution_record_without_self_pointer)
            != Some(execution_record_without_self_pointer(record_json))
        || graph_hash != canonical_sha256(&state.canonical_graph)?
        || state.canonical_graph_sha256 != graph_hash
        || state.canonical_launch_plan_sha256
            != canonical_sha256(
                &serde_json::to_value(&state.canonical_launch_plan)
                    .map_err(|err| err.to_string())?,
            )?
    {
        return Err(format!(
            "routing execution {execution_id} projection/EventLedger integrity failure"
        ));
    }
    let decision_row = sqlx::query(
        r#"SELECT decision.record_json, decision.event_ledger_event_id,
                  decision.event_ledger_seq, ledger.aggregate_type,
                  ledger.aggregate_id, ledger.payload
           FROM model_lane_promotion_decisions decision
           LEFT JOIN kernel_event_ledger ledger
             ON ledger.event_id = decision.event_ledger_event_id
            AND ledger.event_sequence = decision.event_ledger_seq
           WHERE decision.decision_id = $1
           FOR UPDATE OF decision"#,
    )
    .bind(&state.selecting_decision_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| err.to_string())?
    .ok_or_else(|| {
        format!("routing execution {execution_id} selecting decision projection missing")
    })?;
    let decision_json: Value = decision_row.get("record_json");
    let decision_event_id: String = decision_row.get("event_ledger_event_id");
    let decision_event_seq: i64 = decision_row.get("event_ledger_seq");
    let decision_aggregate_type: Option<String> = decision_row.get("aggregate_type");
    let decision_aggregate_id: Option<String> = decision_row.get("aggregate_id");
    let decision_payload: Option<Value> = decision_row.get("payload");
    let decision: ModelLanePromotionDecisionRecord =
        serde_json::from_value(decision_json.clone()).map_err(|err| err.to_string())?;
    if decision_event_id != state.selecting_decision_event_id
        || decision_event_seq != state.selecting_decision_event_seq
        || decision_aggregate_type.as_deref() != Some("model_lane_promotion_decision")
        || decision_aggregate_id.as_deref() != Some(state.selecting_decision_id.as_str())
        || decision_payload
            .as_ref()
            .and_then(|value| value.pointer("/record"))
            .cloned()
            .map(record_without_generated_event_fields)
            != Some(record_without_generated_event_fields(decision_json))
        || decision.routing_launch_plan != state.canonical_launch_plan
    {
        return Err(format!(
            "routing execution {execution_id} selecting decision projection/EventLedger integrity failure"
        ));
    }
    // Lock the mutable run projection before reading its immutable ledger
    // event. A joined SELECT ... FOR UPDATE can wait behind a concurrent run
    // extension and then return the new run version with joined ledger columns
    // evaluated from the statement's older READ COMMITTED snapshot. Splitting
    // the reads prevents that fractured projection/EventLedger view.
    let run_row = sqlx::query(
        r#"SELECT record_json, event_ledger_event_id, event_ledger_seq
           FROM model_lane_runs
           WHERE run_id = $1
           FOR UPDATE"#,
    )
    .bind(&state.run_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| err.to_string())?
    .ok_or_else(|| format!("routing execution {execution_id} ModelLaneRun projection missing"))?;
    let run_json: Value = run_row.get("record_json");
    let run_event_id: String = run_row.get("event_ledger_event_id");
    let run_event_seq: i64 = run_row.get("event_ledger_seq");
    let run: ModelLaneRunRecord =
        serde_json::from_value(run_json.clone()).map_err(|err| err.to_string())?;
    let run_event_row = sqlx::query(
        r#"SELECT aggregate_type, aggregate_id, payload
           FROM kernel_event_ledger
           WHERE event_id = $1 AND event_sequence = $2"#,
    )
    .bind(&run_event_id)
    .bind(run_event_seq)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    let run_payload: Option<Value> = run_event_row
        .as_ref()
        .map(|row| row.get::<Value, _>("payload"));
    let run_aggregate_type: Option<String> = run_event_row
        .as_ref()
        .map(|row| row.get::<String, _>("aggregate_type"));
    let run_aggregate_id: Option<String> = run_event_row
        .as_ref()
        .map(|row| row.get::<String, _>("aggregate_id"));
    let run_record_matches_event = run_payload
        .as_ref()
        .and_then(|value| value.pointer("/record"))
        .cloned()
        .map(record_without_generated_event_fields)
        == Some(record_without_generated_event_fields(run_json));
    let run_locus_ref = run
        .locus_binding
        .as_ref()
        .map(|binding| binding.locus_binding_ref.as_str());
    let mut run_integrity_mismatches = Vec::new();
    if run_aggregate_type.as_deref() != Some("model_lane_run") {
        run_integrity_mismatches.push("aggregate_type");
    }
    if run_aggregate_id.as_deref() != Some(state.run_id.as_str()) {
        run_integrity_mismatches.push("aggregate_id");
    }
    if !run_record_matches_event {
        run_integrity_mismatches.push("event_record");
    }
    if run.trace_id != state.trace_id {
        run_integrity_mismatches.push("trace_id");
    }
    if run.run_span_id != state.run_span_id {
        run_integrity_mismatches.push("run_span_id");
    }
    if run.coordinator_session_id != state.coordinator_session_id {
        run_integrity_mismatches.push("coordinator_session_id");
    }
    if run.work_packet_id.as_deref() != Some(state.work_packet_id.as_str()) {
        run_integrity_mismatches.push("work_packet_id");
    }
    if run.micro_task_id.as_deref() != state.micro_task_id.as_deref() {
        run_integrity_mismatches.push("micro_task_id");
    }
    if run.task_board_id.as_deref() != Some(state.task_board_id.as_str()) {
        run_integrity_mismatches.push("task_board_id");
    }
    if run.owner_session != state.owner_session {
        run_integrity_mismatches.push("owner_session");
    }
    if run_locus_ref != Some(state.locus_ref.as_str()) {
        run_integrity_mismatches.push("locus_ref");
    }
    if !run_integrity_mismatches.is_empty() {
        return Err(format!(
            "routing execution {execution_id} ModelLaneRun projection/EventLedger/context integrity failure: {}",
            run_integrity_mismatches.join(",")
        ));
    }
    for stage in state.stages.values() {
        let attempt_row = sqlx::query(
            r#"SELECT attempt.record_json, attempt.event_ledger_event_id,
                      attempt.event_ledger_seq, ledger.aggregate_type,
                      ledger.aggregate_id, ledger.payload
               FROM model_lane_routing_stage_attempts attempt
               LEFT JOIN kernel_event_ledger ledger
                 ON ledger.event_id = attempt.event_ledger_event_id
                AND ledger.event_sequence = attempt.event_ledger_seq
               WHERE attempt.execution_id = $1
                 AND attempt.stage_id = $2
                 AND attempt.attempt = $3
               FOR UPDATE OF attempt"#,
        )
        .bind(execution_id)
        .bind(&stage.stage_id)
        .bind(i64::from(stage.attempt))
        .fetch_optional(&mut **tx)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!(
                "routing attempt projection missing for {execution_id}/{}/{}",
                stage.stage_id, stage.attempt
            )
        })?;
        let attempt_json: Value = attempt_row.get("record_json");
        let attempt_event_id: String = attempt_row.get("event_ledger_event_id");
        let attempt_event_seq: i64 = attempt_row.get("event_ledger_seq");
        let attempt_payload: Option<Value> = attempt_row.get("payload");
        let attempt_aggregate_type: Option<String> = attempt_row.get("aggregate_type");
        let attempt_aggregate_id: Option<String> = attempt_row.get("aggregate_id");
        let expected_attempt_aggregate_id =
            format!("{execution_id}:{}:{}", stage.stage_id, stage.attempt);
        if attempt_json != serde_json::to_value(stage).map_err(|err| err.to_string())?
            || attempt_event_id != stage.event_ledger_event_id
            || attempt_event_seq != stage.event_ledger_seq
            || attempt_aggregate_type.as_deref() != Some("model_lane_routing_stage_attempt")
            || attempt_aggregate_id.as_deref() != Some(expected_attempt_aggregate_id.as_str())
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("execution_id"))
                .and_then(Value::as_str)
                != Some(execution_id)
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("stage_id"))
                .and_then(Value::as_str)
                != Some(stage.stage_id.as_str())
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("attempt"))
                .and_then(Value::as_u64)
                != Some(u64::from(stage.attempt))
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("state"))
                .and_then(enum_name)
                .as_deref()
                != Some(stage_kind_name(stage.state))
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("dispatch_target"))
                != Some(
                    &serde_json::to_value(stage.dispatch_target).map_err(|err| err.to_string())?,
                )
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("expected_run_id"))
                .and_then(Value::as_str)
                != Some(stage.expected_run_id.as_str())
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("expected_lane_id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                != stage.expected_lane_id
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("expected_model_id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                != stage.expected_model_id
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("expected_provider"))
                != Some(
                    &serde_json::to_value(stage.expected_provider)
                        .map_err(|err| err.to_string())?,
                )
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("input_refs"))
                != Some(&serde_json::to_value(&stage.input_refs).map_err(|err| err.to_string())?)
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("authority_ref"))
                != Some(&serde_json::to_value(&stage.authority_ref).map_err(|err| err.to_string())?)
            || attempt_payload
                .as_ref()
                .and_then(|value| value.get("record"))
                .cloned()
                .map(attempt_record_without_self_pointer)
                != Some(attempt_record_without_self_pointer(attempt_json.clone()))
        {
            return Err(format!(
                "routing attempt {execution_id}/{}/{} projection/EventLedger integrity failure",
                stage.stage_id, stage.attempt
            ));
        }
        let command_id = format!(
            "routing-command:{execution_id}:{}:{}",
            stage.stage_id, stage.attempt
        );
        let outbox = sqlx::query(
            r#"SELECT outbox.status, outbox.command_json, outbox.fencing_token,
                      outbox.lease_owner, outbox.lease_expires_at_unix_ms,
                      outbox.event_ledger_event_id, outbox.event_ledger_seq,
                      ledger.aggregate_type, ledger.aggregate_id, ledger.payload
               FROM model_lane_routing_outbox outbox
               LEFT JOIN kernel_event_ledger ledger
                 ON ledger.event_id=outbox.event_ledger_event_id
                AND ledger.event_sequence=outbox.event_ledger_seq
               WHERE outbox.command_id=$1 FOR UPDATE OF outbox"#,
        )
        .bind(&command_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("routing outbox projection missing for {command_id}"))?;
        let outbox_status: String = outbox.get("status");
        let command_json: Value = outbox.get("command_json");
        let outbox_fence: Option<String> = outbox.get("fencing_token");
        let outbox_owner: Option<String> = outbox.get("lease_owner");
        let outbox_expiry: Option<i64> = outbox.get("lease_expires_at_unix_ms");
        let outbox_aggregate_type: Option<String> = outbox.get("aggregate_type");
        let outbox_aggregate_id: Option<String> = outbox.get("aggregate_id");
        let outbox_payload: Option<Value> = outbox.get("payload");
        let expected_outbox_status = match stage.state {
            ModelLaneRoutingStageStateKind::Scheduled => "pending",
            ModelLaneRoutingStageStateKind::Claimed
            | ModelLaneRoutingStageStateKind::InFlight
            | ModelLaneRoutingStageStateKind::AwaitingAuthority => "claimed",
            ModelLaneRoutingStageStateKind::Cancelled => "cancelled",
            ModelLaneRoutingStageStateKind::Compensated => "compensated",
            _ => "acked",
        };
        let expected_command = json!({
            "schema_id": ROUTING_OUTBOX_SCHEMA_ID,
            "execution_id": execution_id,
            "stage_id": stage.stage_id,
            "attempt": stage.attempt,
            "dispatch_target": stage.dispatch_target,
            "expected_run_id": stage.expected_run_id,
            "expected_lane_id": stage.expected_lane_id,
            "expected_model_id": stage.expected_model_id,
            "expected_provider": stage.expected_provider,
            "input_refs": stage.input_refs,
            "authority_ref": stage.authority_ref,
        });
        if outbox_status != expected_outbox_status
            || command_json != expected_command
            || outbox_aggregate_type.as_deref() != Some("model_lane_routing_outbox")
            || outbox_aggregate_id.as_deref() != Some(command_id.as_str())
            || outbox_payload
                .as_ref()
                .and_then(|payload| payload.get("command"))
                != Some(&expected_command)
            || outbox_payload
                .as_ref()
                .and_then(|payload| payload.get("status"))
                .and_then(Value::as_str)
                != Some(expected_outbox_status)
            || outbox_payload
                .as_ref()
                .and_then(|payload| payload.get("fencing_token"))
                .and_then(Value::as_str)
                != stage.fencing_token.as_deref()
            || outbox_payload
                .as_ref()
                .and_then(|payload| payload.get("lease_owner"))
                .and_then(Value::as_str)
                != stage.lease_owner.as_deref()
            || outbox_payload
                .as_ref()
                .and_then(|payload| payload.get("lease_expires_at_unix_ms"))
                .and_then(Value::as_u64)
                != stage.lease_expires_at_unix_ms
            || outbox_fence.as_deref() != stage.fencing_token.as_deref()
            || outbox_owner.as_deref() != stage.lease_owner.as_deref()
            || outbox_expiry.map(|value| value as u64) != stage.lease_expires_at_unix_ms
        {
            return Err(format!(
                "routing outbox {command_id} projection/EventLedger integrity failure"
            ));
        }
    }
    Ok(Some(state))
}

async fn save_execution_tx(
    tx: &mut Transaction<'_, Postgres>,
    execution: &ModelLaneRoutingExecutionState,
) -> Result<(), String> {
    let record_json = serde_json::to_value(execution).map_err(|err| err.to_string())?;
    sqlx::query(
        r#"INSERT INTO model_lane_routing_executions
           (execution_id, run_id, selecting_decision_id, selecting_decision_event_id,
            selecting_decision_event_seq, trace_id, run_span_id, coordinator_session_id,
            locus_ref, work_packet_id, micro_task_id, task_board_id, owner_session,
            initial_input_ref, initial_input_sha256, graph_sha256, status, revision,
            event_ledger_event_id, event_ledger_seq, record_json, updated_at_unix_ms)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)
           ON CONFLICT (execution_id) DO UPDATE SET
             status=EXCLUDED.status, revision=EXCLUDED.revision,
             event_ledger_event_id=EXCLUDED.event_ledger_event_id,
             event_ledger_seq=EXCLUDED.event_ledger_seq,
             record_json=EXCLUDED.record_json, updated_at_unix_ms=EXCLUDED.updated_at_unix_ms"#,
    )
    .bind(&execution.execution_id)
    .bind(&execution.run_id)
    .bind(&execution.selecting_decision_id)
    .bind(&execution.selecting_decision_event_id)
    .bind(execution.selecting_decision_event_seq)
    .bind(&execution.trace_id)
    .bind(&execution.run_span_id)
    .bind(&execution.coordinator_session_id)
    .bind(&execution.locus_ref)
    .bind(&execution.work_packet_id)
    .bind(&execution.micro_task_id)
    .bind(&execution.task_board_id)
    .bind(&execution.owner_session)
    .bind(execution.initial_input_ref.as_deref().unwrap_or_default())
    .bind(
        execution
            .initial_input_sha256
            .as_deref()
            .unwrap_or_default(),
    )
    .bind(&execution.canonical_graph_sha256)
    .bind(execution_status_name(execution.status))
    .bind(execution.revision as i64)
    .bind(&execution.event_ledger_event_id)
    .bind(execution.event_ledger_seq)
    .bind(record_json)
    .bind(now_ms() as i64)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

async fn insert_attempt_and_outbox_tx(
    tx: &mut Transaction<'_, Postgres>,
    execution_id: &str,
    stage: &ModelLaneRoutingStageState,
    execution: &ModelLaneRoutingExecutionState,
) -> Result<(), String> {
    let attempt_json = serde_json::to_value(stage).map_err(|err| err.to_string())?;
    sqlx::query(
        r#"INSERT INTO model_lane_routing_stage_attempts
           (execution_id, stage_id, attempt, dispatch_target, expected_run_id,
            expected_lane_id, expected_model_id, expected_provider, status, run_id, trace_id,
            locus_ref, authority_ref, input_refs, output_ref, output_message_ref,
            authority_request_message_ref, lease_owner, fencing_token,
            lease_expires_at_unix_ms, event_ledger_event_id,
            event_ledger_seq, record_json, updated_at_unix_ms)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)
           ON CONFLICT (execution_id, stage_id, attempt) DO NOTHING"#,
    )
    .bind(execution_id)
    .bind(&stage.stage_id)
    .bind(i64::from(stage.attempt))
    .bind(dispatch_target_name(&stage.dispatch_target))
    .bind(&stage.expected_run_id)
    .bind(&stage.expected_lane_id)
    .bind(&stage.expected_model_id)
    .bind(stage.expected_provider.map(|provider| format!("{provider:?}").to_ascii_lowercase()))
    .bind(stage_kind_name(stage.state))
    .bind(&execution.run_id)
    .bind(&execution.trace_id)
    .bind(&execution.locus_ref)
    .bind(&stage.authority_ref)
    .bind(json!(stage.input_refs))
    .bind(&stage.output_ref)
    .bind(&stage.output_message_ref)
    .bind(&stage.authority_request_message_ref)
    .bind(&stage.lease_owner)
    .bind(&stage.fencing_token)
    .bind(stage.lease_expires_at_unix_ms.map(|value| value as i64))
    .bind(&stage.event_ledger_event_id)
    .bind(stage.event_ledger_seq)
    .bind(attempt_json)
    .bind(stage.updated_at_unix_ms as i64)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    let command_id = format!(
        "routing-command:{execution_id}:{}:{}",
        stage.stage_id, stage.attempt
    );
    let command_json = json!({
        "schema_id": ROUTING_OUTBOX_SCHEMA_ID,
        "execution_id": execution_id,
        "stage_id": stage.stage_id,
        "attempt": stage.attempt,
        "dispatch_target": stage.dispatch_target,
        "expected_run_id": stage.expected_run_id,
        "expected_lane_id": stage.expected_lane_id,
        "expected_model_id": stage.expected_model_id,
        "expected_provider": stage.expected_provider,
        "input_refs": stage.input_refs,
        "authority_ref": stage.authority_ref,
    });
    let outbox_event = append_event(
        tx,
        KernelEventType::ModelAdapterInvoked,
        "model_lane_routing_outbox",
        &command_id,
        &format!(
            "routing-outbox-pending:{execution_id}:{}:{}",
            stage.stage_id, stage.attempt
        ),
        &execution.run_id,
        execution_id,
        json!({
            "schema_id": ROUTING_OUTBOX_SCHEMA_ID,
            "command_id": command_id,
            "status": "pending",
            "command": command_json,
            "lease_owner": Value::Null,
            "fencing_token": Value::Null,
            "lease_expires_at_unix_ms": Value::Null,
        }),
    )
    .await?;
    sqlx::query(
        r#"INSERT INTO model_lane_routing_outbox
           (command_id, idempotency_key, execution_id, stage_id, attempt,
            dispatch_target, status, command_json, event_ledger_event_id,
            event_ledger_seq, created_at_unix_ms, updated_at_unix_ms)
           VALUES ($1,$2,$3,$4,$5,$6,'pending',$7,$8,$9,$10,$10)
           ON CONFLICT (idempotency_key) DO NOTHING"#,
    )
    .bind(&command_id)
    .bind(format!(
        "routing-dispatch:{execution_id}:{}:{}",
        stage.stage_id, stage.attempt
    ))
    .bind(execution_id)
    .bind(&stage.stage_id)
    .bind(i64::from(stage.attempt))
    .bind(dispatch_target_name(&stage.dispatch_target))
    .bind(command_json)
    .bind(outbox_event.0)
    .bind(outbox_event.1)
    .bind(stage.updated_at_unix_ms as i64)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

async fn persist_outbox_state_tx(
    tx: &mut Transaction<'_, Postgres>,
    execution: &ModelLaneRoutingExecutionState,
    stage: &ModelLaneRoutingStageState,
    status: &str,
) -> Result<(), String> {
    let command_id = format!(
        "routing-command:{}:{}:{}",
        execution.execution_id, stage.stage_id, stage.attempt
    );
    let command_json: Value = sqlx::query_scalar(
        "SELECT command_json FROM model_lane_routing_outbox WHERE command_id=$1 FOR UPDATE",
    )
    .bind(&command_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    let active = status == "claimed";
    let event = append_event(
        tx,
        KernelEventType::ModelAdapterInvoked,
        "model_lane_routing_outbox",
        &command_id,
        &format!(
            "routing-outbox-{status}:{}:{}:{}:{}",
            execution.execution_id, stage.stage_id, stage.attempt, stage.event_ledger_seq
        ),
        &execution.run_id,
        &execution.execution_id,
        json!({
            "schema_id": ROUTING_OUTBOX_SCHEMA_ID,
            "command_id": command_id,
            "status": status,
            "command": command_json,
            "lease_owner": active.then(|| stage.lease_owner.clone()).flatten(),
            "fencing_token": active.then(|| stage.fencing_token.clone()).flatten(),
            "lease_expires_at_unix_ms": active.then(|| stage.lease_expires_at_unix_ms).flatten(),
        }),
    )
    .await?;
    sqlx::query(
        r#"UPDATE model_lane_routing_outbox
           SET status=$2, lease_owner=$3, fencing_token=$4,
               lease_expires_at_unix_ms=$5, event_ledger_event_id=$6,
               event_ledger_seq=$7, updated_at_unix_ms=$8
           WHERE command_id=$1"#,
    )
    .bind(command_id)
    .bind(status)
    .bind(active.then(|| stage.lease_owner.clone()).flatten())
    .bind(active.then(|| stage.fencing_token.clone()).flatten())
    .bind(
        active
            .then(|| stage.lease_expires_at_unix_ms)
            .flatten()
            .map(|value| value as i64),
    )
    .bind(event.0)
    .bind(event.1)
    .bind(stage.updated_at_unix_ms as i64)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

async fn save_attempt_projections_tx(
    tx: &mut Transaction<'_, Postgres>,
    execution_id: &str,
    stages: &BTreeMap<String, ModelLaneRoutingStageState>,
) -> Result<(), String> {
    for stage in stages.values() {
        sqlx::query(
            r#"UPDATE model_lane_routing_stage_attempts SET status=$4, input_refs=$5,
               output_ref=$6, output_message_ref=$7, authority_request_message_ref=$8,
               lease_owner=$9, fencing_token=$10,
               lease_expires_at_unix_ms=$11, event_ledger_event_id=$12,
               event_ledger_seq=$13, record_json=$14, updated_at_unix_ms=$15
               WHERE execution_id=$1 AND stage_id=$2 AND attempt=$3"#,
        )
        .bind(execution_id)
        .bind(&stage.stage_id)
        .bind(i64::from(stage.attempt))
        .bind(stage_kind_name(stage.state))
        .bind(json!(stage.input_refs))
        .bind(&stage.output_ref)
        .bind(&stage.output_message_ref)
        .bind(&stage.authority_request_message_ref)
        .bind(&stage.lease_owner)
        .bind(&stage.fencing_token)
        .bind(stage.lease_expires_at_unix_ms.map(|value| value as i64))
        .bind(&stage.event_ledger_event_id)
        .bind(stage.event_ledger_seq)
        .bind(serde_json::to_value(stage).map_err(|err| err.to_string())?)
        .bind(stage.updated_at_unix_ms as i64)
        .execute(&mut **tx)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn append_event(
    tx: &mut Transaction<'_, Postgres>,
    event_type: KernelEventType,
    aggregate_type: &str,
    aggregate_id: &str,
    idempotency_key: &str,
    kernel_task_run_id: &str,
    session_run_id: &str,
    payload: Value,
) -> Result<(String, i64), String> {
    let event = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        KernelActor::ModelAdapter("DexterityRoutingExecutor".into()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(idempotency_key)
    .correlation_id(format!(
        "dexterity-routing:{kernel_task_run_id}:{session_run_id}"
    ))
    .source_component(SOURCE_COMPONENT)
    .payload(payload)
    .build()
    .map_err(|err| err.to_string())?;
    let stored = append_kernel_event_with_executor(&mut **tx, event)
        .await
        .map_err(|err| err.to_string())?;
    Ok((stored.event_id, stored.event_sequence))
}

pub(crate) fn canonical_sha256(value: &Value) -> Result<String, String> {
    Ok(format!("{:x}", Sha256::digest(canonical_json_bytes(value))))
}

fn execution_record_without_self_pointer(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("event_ledger_event_id");
        object.remove("event_ledger_seq");
    }
    value
}

fn attempt_record_without_self_pointer(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("event_ledger_event_id");
        object.remove("event_ledger_seq");
    }
    value
}

fn record_without_generated_event_fields(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("event_ledger_event_id");
        object.remove("event_ledger_seq");
        object.remove("event_stream_version");
        object.remove("transaction_seq");
    }
    value
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn stage_views(graph: &Value) -> Result<Vec<StageView>, String> {
    let stages = graph
        .get("stages")
        .and_then(Value::as_array)
        .ok_or_else(|| "canonical routing graph has no stages array".to_string())?;
    stages
        .iter()
        .map(|stage| {
            let object = stage
                .as_object()
                .ok_or_else(|| "canonical routing stage is not an object".to_string())?;
            let stage_id = string_field(object, &["stage_id", "id"])?;
            let target_value = value_field(object, &["dispatch_target", "target"])?;
            let dispatch_target = serde_json::from_value(target_value.clone())
                .map_err(|err| format!("invalid dispatch target for {stage_id}: {err}"))?;
            let dependencies = object
                .get("depends_on")
                .or_else(|| object.get("dependencies"))
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let activation = object
                .get("activation")
                .and_then(enum_name)
                .unwrap_or_else(|| "always".into());
            let gate = object
                .get("authority_gate")
                .or_else(|| object.get("gate"))
                .and_then(enum_name)
                .unwrap_or_else(|| "none".into());
            Ok(StageView {
                stage_id,
                dispatch_target,
                dependencies,
                activation,
                gate,
            })
        })
        .collect()
}

fn authority_for_stage(
    stage: &StageView,
    authority: &ModelLaneRoutingAuthority,
) -> Result<Option<String>, String> {
    let gate = stage.gate.replace('_', "");
    match (&stage.dispatch_target, gate.as_str()) {
        (ModelLaneRoutingDispatchTarget::CloudModel, _) | (_, "cloudconsent") => authority
            .cloud_consent_receipt_ref
            .clone()
            .map(Some)
            .ok_or_else(|| format!("stage {} requires a cloud consent receipt", stage.stage_id)),
        (ModelLaneRoutingDispatchTarget::Validator, _) | (_, "validatorauthority") => authority
            .validator_authority_ref
            .clone()
            .map(Some)
            .ok_or_else(|| format!("stage {} requires validator authority", stage.stage_id)),
        (ModelLaneRoutingDispatchTarget::Operator, _) | (_, "operatorauthority") => authority
            .operator_authority_ref
            .clone()
            .map(Some)
            .ok_or_else(|| format!("stage {} requires operator authority", stage.stage_id)),
        _ => Ok(None),
    }
}

fn predecessor_output_refs(
    stage: &StageView,
    states: &BTreeMap<String, ModelLaneRoutingStageState>,
) -> Vec<String> {
    stage
        .dependencies
        .iter()
        .filter_map(|dependency| states.get(dependency))
        .filter_map(|state| state.output_ref.clone())
        .collect()
}

fn is_ready(stage: &StageView, states: &BTreeMap<String, ModelLaneRoutingStageState>) -> bool {
    if stage.dependencies.is_empty() {
        return true;
    }
    let dependencies: Option<Vec<&ModelLaneRoutingStageState>> = stage
        .dependencies
        .iter()
        .map(|dependency| states.get(dependency))
        .collect();
    let Some(dependencies) = dependencies else {
        return false;
    };
    if !dependencies.iter().all(|state| state.state.is_terminal()) {
        return false;
    }
    match stage.activation.replace('_', "").as_str() {
        "afterfailure" => dependencies
            .iter()
            .any(|state| state.state == ModelLaneRoutingStageStateKind::Failed),
        "aftersuccess" => dependencies.iter().all(|state| state.state.is_success()),
        _ => dependencies.iter().all(|state| state.state.is_success()),
    }
}

fn refresh_execution_status(execution: &mut ModelLaneRoutingExecutionState) -> Result<(), String> {
    if execution
        .stages
        .values()
        .any(|stage| stage.state == ModelLaneRoutingStageStateKind::AwaitingAuthority)
    {
        execution.status = ModelLaneRoutingExecutionStatus::AwaitingAuthority;
        return Ok(());
    }
    let policy = execution
        .canonical_graph
        .get("policy")
        .and_then(enum_name)
        .unwrap_or_default()
        .replace('_', "");
    let succeeded = |id: &str| {
        execution
            .stages
            .get(id)
            .is_some_and(|stage| stage.state.is_success())
    };
    let failed = |id: &str| {
        execution
            .stages
            .get(id)
            .is_some_and(|stage| stage.state == ModelLaneRoutingStageStateKind::Failed)
    };
    let terminal_success = match policy.as_str() {
        "localfirst" => succeeded("local-attempt") || succeeded("cloud-escalation"),
        "cloudreview" => succeeded("cloud-review"),
        "cloudplanlocalexecute" => succeeded("local-execute"),
        "paralleldebate" => succeeded("debate-join"),
        "validatorlane" => succeeded("validator-verdict"),
        "operatorlane" => succeeded("operator-decision"),
        _ => false,
    };
    if terminal_success {
        execution.status = ModelLaneRoutingExecutionStatus::Succeeded;
        return Ok(());
    }
    let terminal_failure = match policy.as_str() {
        "localfirst" => failed("cloud-escalation"),
        "cloudreview" => failed("local-candidate") || failed("cloud-review"),
        "cloudplanlocalexecute" => failed("cloud-plan") || failed("local-execute"),
        "paralleldebate" => {
            failed("debate-local") || failed("debate-cloud") || failed("debate-join")
        }
        "validatorlane" => failed("validation-candidate") || failed("validator-verdict"),
        "operatorlane" => failed("operator-candidate") || failed("operator-decision"),
        _ => false,
    };
    if terminal_failure {
        execution.status = ModelLaneRoutingExecutionStatus::Failed;
        execution.failure_reason = Some("canonical routing terminal stage failed".into());
    } else {
        execution.status = ModelLaneRoutingExecutionStatus::Running;
    }
    Ok(())
}

fn valid_transition(
    from: ModelLaneRoutingStageStateKind,
    to: ModelLaneRoutingStageStateKind,
) -> bool {
    matches!(
        (from, to),
        (
            ModelLaneRoutingStageStateKind::Scheduled,
            ModelLaneRoutingStageStateKind::Claimed
        ) | (
            ModelLaneRoutingStageStateKind::Scheduled,
            ModelLaneRoutingStageStateKind::InFlight
        ) | (
            ModelLaneRoutingStageStateKind::Scheduled,
            ModelLaneRoutingStageStateKind::Failed
        ) | (
            ModelLaneRoutingStageStateKind::Scheduled,
            ModelLaneRoutingStageStateKind::Joined
        ) | (
            ModelLaneRoutingStageStateKind::Claimed,
            ModelLaneRoutingStageStateKind::InFlight
        ) | (
            ModelLaneRoutingStageStateKind::Claimed,
            ModelLaneRoutingStageStateKind::AwaitingAuthority
        ) | (
            ModelLaneRoutingStageStateKind::Claimed,
            ModelLaneRoutingStageStateKind::Failed
        ) | (
            ModelLaneRoutingStageStateKind::Claimed,
            ModelLaneRoutingStageStateKind::Joined
        ) | (
            ModelLaneRoutingStageStateKind::Claimed,
            ModelLaneRoutingStageStateKind::Cancelled
        ) | (
            ModelLaneRoutingStageStateKind::Claimed,
            ModelLaneRoutingStageStateKind::Compensated
        ) | (
            ModelLaneRoutingStageStateKind::InFlight,
            ModelLaneRoutingStageStateKind::Succeeded
        ) | (
            ModelLaneRoutingStageStateKind::InFlight,
            ModelLaneRoutingStageStateKind::Joined
        ) | (
            ModelLaneRoutingStageStateKind::InFlight,
            ModelLaneRoutingStageStateKind::Failed
        ) | (
            ModelLaneRoutingStageStateKind::InFlight,
            ModelLaneRoutingStageStateKind::Cancelled
        ) | (
            ModelLaneRoutingStageStateKind::InFlight,
            ModelLaneRoutingStageStateKind::Compensated
        ) | (
            ModelLaneRoutingStageStateKind::AwaitingAuthority,
            ModelLaneRoutingStageStateKind::Succeeded
        ) | (
            ModelLaneRoutingStageStateKind::AwaitingAuthority,
            ModelLaneRoutingStageStateKind::Failed
        ) | (
            ModelLaneRoutingStageStateKind::AwaitingAuthority,
            ModelLaneRoutingStageStateKind::Cancelled
        )
    )
}

fn execution_status_name(status: ModelLaneRoutingExecutionStatus) -> &'static str {
    match status {
        ModelLaneRoutingExecutionStatus::Running => "running",
        ModelLaneRoutingExecutionStatus::AwaitingAuthority => "awaiting_authority",
        ModelLaneRoutingExecutionStatus::Succeeded => "succeeded",
        ModelLaneRoutingExecutionStatus::Failed => "failed",
        ModelLaneRoutingExecutionStatus::Cancelled => "cancelled",
    }
}

fn stage_kind_name(state: ModelLaneRoutingStageStateKind) -> &'static str {
    match state {
        ModelLaneRoutingStageStateKind::Scheduled => "scheduled",
        ModelLaneRoutingStageStateKind::Claimed => "claimed",
        ModelLaneRoutingStageStateKind::InFlight => "in_flight",
        ModelLaneRoutingStageStateKind::AwaitingAuthority => "awaiting_authority",
        ModelLaneRoutingStageStateKind::Succeeded => "succeeded",
        ModelLaneRoutingStageStateKind::Failed => "failed",
        ModelLaneRoutingStageStateKind::Joined => "joined",
        ModelLaneRoutingStageStateKind::Cancelled => "cancelled",
        ModelLaneRoutingStageStateKind::Compensated => "compensated",
    }
}

fn dispatch_target_name(target: &ModelLaneRoutingDispatchTarget) -> &'static str {
    match target {
        ModelLaneRoutingDispatchTarget::LocalModel => "local_model",
        ModelLaneRoutingDispatchTarget::CloudModel => "cloud_model",
        ModelLaneRoutingDispatchTarget::Validator => "validator",
        ModelLaneRoutingDispatchTarget::Operator => "operator",
        ModelLaneRoutingDispatchTarget::CoordinatorJoin => "coordinator_join",
    }
}

fn string_field(object: &serde_json::Map<String, Value>, names: &[&str]) -> Result<String, String> {
    names
        .iter()
        .find_map(|name| object.get(*name).and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| format!("missing routing stage field {}", names.join("/")))
}

fn value_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    names: &[&str],
) -> Result<&'a Value, String> {
    names
        .iter()
        .find_map(|name| object.get(*name))
        .ok_or_else(|| format!("missing routing stage field {}", names.join("/")))
}

fn enum_name(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(|value| value.to_ascii_lowercase())
        .or_else(|| {
            value
                .as_object()
                .and_then(|object| object.keys().next())
                .map(|value| value.to_ascii_lowercase())
        })
}

pub fn terminal_state_for_outcome(
    outcome: ModelLaneRoutingStageOutcome,
) -> ModelLaneRoutingStageStateKind {
    match outcome {
        ModelLaneRoutingStageOutcome::Succeeded => ModelLaneRoutingStageStateKind::Succeeded,
        ModelLaneRoutingStageOutcome::Failed => ModelLaneRoutingStageStateKind::Failed,
    }
}
