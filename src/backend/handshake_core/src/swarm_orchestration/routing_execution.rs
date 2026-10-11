//! Embedded SurrealDB/EventLedger execution state for canonical mixed-model routing graphs.
//!
//! The execution row is a replay projection. EventLedger is the append authority;
//! stage attempts and the transactional outbox make claims attributable, leased,
//! idempotent, and recoverable after coordinator failure.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use super::ids::SpawnRequest;
use super::model_lane::{
    ModelLanePromotionDecisionRecord, ModelLanePromotionOutcome, ModelLaneRunRecord,
};
use super::routing::{
    ModelLaneRoutingAuthority, ModelLaneRoutingDispatchTarget, ModelLaneRoutingGraph,
    ModelLaneRoutingStageLaunchPlan, ModelLaneRoutingStageOutcome,
};
use crate::kernel::{
    context_bundle::canonical_json_bytes, KernelActor, KernelEventType, NewKernelEvent,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
    model_lane_store: super::model_lane::ModelLaneStore,
    lease_owner: String,
    lease_ms: u64,
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
    pub(crate) fn new(store: super::model_lane::ModelLaneStore) -> Self {
        Self::with_lease(
            store,
            format!("routing-executor:{}", uuid::Uuid::now_v7()),
            DEFAULT_LEASE_MS,
        )
    }
    pub(crate) fn with_lease(
        store: super::model_lane::ModelLaneStore,
        owner: impl Into<String>,
        ms: u64,
    ) -> Self {
        Self {
            model_lane_store: store,
            lease_owner: owner.into(),
            lease_ms: ms.max(1),
        }
    }
    pub async fn snapshot(
        &self,
        id: &str,
    ) -> Result<Option<ModelLaneRoutingExecutionState>, String> {
        self.model_lane_store
            .routing_execution_snapshot(id)
            .await
            .map_err(|e| e.to_string())
    }
    pub(crate) async fn diagnostics_for_run(
        &self,
        id: &str,
    ) -> Result<Vec<ModelLaneRoutingExecutionDiagnostics>, String> {
        self.model_lane_store
            .routing_execution_diagnostics_for_run(id)
            .await
            .map_err(|e| e.to_string())
    }
    pub(crate) async fn active_instance_ids_for_cancellation(
        &self,
        id: &str,
    ) -> Result<Option<Vec<String>>, String> {
        let Some(x) = self.snapshot(id).await? else {
            return Ok(None);
        };
        Ok(Some(
            x.stages
                .values()
                .filter(|v| {
                    matches!(
                        v.state,
                        ModelLaneRoutingStageStateKind::Claimed
                            | ModelLaneRoutingStageStateKind::InFlight
                            | ModelLaneRoutingStageStateKind::AwaitingAuthority
                    )
                })
                .filter_map(|v| v.instance_id.clone())
                .collect(),
        ))
    }
    pub(crate) async fn begin_execution(
        &self,
        id: &str,
        did: &str,
        authority: &ModelLaneRoutingAuthority,
        c: ModelLaneRoutingExecutionContext,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        if let Some(x) = self.snapshot(id).await? {
            if x.selecting_decision_id == did
                && x.authority == *authority
                && x.run_id == c.run_id
                && x.trace_id == c.trace_id
                && x.run_span_id == c.run_span_id
                && x.coordinator_session_id == c.coordinator_session_id
                && x.locus_ref == c.locus_ref
                && x.work_packet_id == c.work_packet_id
                && x.micro_task_id == c.micro_task_id
                && x.task_board_id == c.task_board_id
                && x.owner_session == c.owner_session
                && x.initial_input_ref.as_deref() == Some(c.initial_input_ref.as_str())
                && x.initial_input_sha256.as_deref() == Some(c.initial_input_sha256.as_str())
            {
                return Ok(x);
            }
            return Err(format!("routing execution {id} immutable context conflict"));
        }
        let d = self
            .model_lane_store
            .replay_promotion_decisions(&c.run_id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|v| v.decision_id == did)
            .ok_or_else(|| format!("unknown selecting promotion decision {did}"))?;
        if d.outcome != ModelLanePromotionOutcome::Approved
            || d.run_id != c.run_id
            || d.trace_id != c.trace_id
            || d.coordinator_session_id != c.coordinator_session_id
            || d.work_packet_id.as_deref() != Some(c.work_packet_id.as_str())
            || d.task_board_id.as_deref() != Some(c.task_board_id.as_str())
            || d.owner_session != c.owner_session
        {
            return Err(format!(
                "selecting promotion decision {did} lacks execution authority"
            ));
        }
        let r = self
            .model_lane_store
            .replay_run(&c.run_id)
            .await
            .map_err(|e| e.to_string())?;
        check_run(&r.run, &c)?;
        self.check_input(&c).await?;
        if !d.selected_input_refs.contains(&c.initial_input_ref) {
            return Err("initial input is not selected".into());
        }
        let g = ModelLaneRoutingGraph::for_policy(d.routing_policy);
        g.validate().map_err(|e| e.to_string())?;
        let gj = serde_json::to_value(&g).map_err(|e| e.to_string())?;
        if d.diagnostic_payload.get("routing_graph") != Some(&gj) {
            return Err("selecting decision graph mismatch".into());
        }
        validate_launch_plan(&g, &d.routing_launch_plan)?;
        let ph = canonical_sha256(
            &serde_json::to_value(&d.routing_launch_plan).map_err(|e| e.to_string())?,
        )?;
        let derived = ModelLaneRoutingAuthority {
            cloud_consent_receipt_ref: d
                .diagnostic_payload
                .get("cloud_consent_receipt_ref")
                .and_then(Value::as_str)
                .map(str::to_owned),
            validator_authority_ref: d.validator_authority_ref.clone(),
            operator_authority_ref: d.operator_authority_ref.clone(),
        };
        g.require_authority_contract(&derived)
            .map_err(|e| e.to_string())?;
        if authority != &derived || c.micro_task_id.is_none() {
            return Err("routing authority mismatch".into());
        }
        let gh = canonical_sha256(&gj)?;
        let mut x = initial_execution(
            id,
            gj,
            gh,
            authority,
            c,
            did,
            &d.event_ledger_event_id,
            d.event_ledger_seq,
            d.routing_launch_plan.clone(),
            ph,
        );
        let root = stage_views(&x.canonical_graph)?
            .into_iter()
            .find(|v| v.dependencies.is_empty())
            .ok_or("routing graph has no root")?;
        let changed = stage(&x, &root, 1)?;
        x.stages.insert(root.stage_id, changed.clone());
        x.revision = 1;
        let e = events(&x, &changed, "pending", "begin")?;
        self.model_lane_store
            .commit_routing_execution_atomic(0, None, x, changed, "pending", e)
            .await
            .map_err(|e| e.to_string())
    }
    async fn check_input(&self, c: &ModelLaneRoutingExecutionContext) -> Result<(), String> {
        let id = c
            .initial_input_ref
            .strip_prefix("model-lane-message://")
            .ok_or("initial input is not a message")?;
        let p = self
            .model_lane_store
            .navigation_by_message(id)
            .await
            .map_err(|e| e.to_string())?;
        let m = p
            .messages
            .iter()
            .find(|v| v.message_id == id)
            .ok_or("initial message missing")?;
        let a = p
            .artifacts
            .iter()
            .find(|v| v.artifact_ref == m.payload_ref)
            .ok_or("initial artifact missing")?;
        if m.run_id != c.run_id
            || m.payload_sha256 != c.initial_input_sha256
            || a.artifact_sha256 != c.initial_input_sha256
            || canonical_sha256(&a.payload_json)? != c.initial_input_sha256
        {
            return Err("initial input authority mismatch".into());
        }
        Ok(())
    }
    pub(crate) async fn claim_ready(
        &self,
        id: &str,
        launches: &[ModelLaneRoutingStageLaunch],
    ) -> Result<Vec<ModelLaneRoutingStageClaim>, String> {
        let mut x = self
            .snapshot(id)
            .await?
            .ok_or("unknown routing execution")?;
        check_launches(&x, launches)?;
        let mut cs = Vec::new();
        for v in stage_views(&x.canonical_graph)? {
            if !is_ready(&v, &x.stages) {
                continue;
            }
            let old = x.stages.get(&v.stage_id).cloned();
            if old
                .as_ref()
                .is_some_and(|s| s.state != ModelLaneRoutingStageStateKind::Scheduled)
            {
                continue;
            }
            let mut a = old.unwrap_or(stage(&x, &v, 1)?);
            let now = now_ms();
            let c = ModelLaneRoutingStageClaim {
                execution_id: x.execution_id.clone(),
                stage_id: a.stage_id.clone(),
                attempt: a.attempt,
                fencing_token: uuid::Uuid::now_v7().to_string(),
                lease_owner: self.lease_owner.clone(),
                lease_expires_at_unix_ms: now + self.lease_ms,
                dispatch_target: a.dispatch_target,
                expected_run_id: a.expected_run_id.clone(),
                expected_lane_id: a.expected_lane_id.clone(),
                expected_model_id: a.expected_model_id.clone(),
                expected_provider: a.expected_provider,
            };
            a.state = ModelLaneRoutingStageStateKind::Claimed;
            a.lease_owner = Some(c.lease_owner.clone());
            a.fencing_token = Some(c.fencing_token.clone());
            a.lease_expires_at_unix_ms = Some(c.lease_expires_at_unix_ms);
            a.updated_at_unix_ms = now;
            let rev = x.revision;
            x.stages.insert(v.stage_id, a.clone());
            x.revision = rev + 1;
            let e = events(&x, &a, "claimed", "claim")?;
            x = self
                .model_lane_store
                .commit_routing_execution_atomic(rev, None, x, a, "claimed", e)
                .await
                .map_err(|e| e.to_string())?;
            cs.push(c)
        }
        Ok(cs)
    }
    pub(crate) async fn record_transition(
        &self,
        c: &ModelLaneRoutingStageClaim,
        s: ModelLaneRoutingStageStateKind,
        i: Option<String>,
        d: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        self.record_stage_result(c, s, i, None, None, vec![], None, None, None, None, d)
            .await
    }
    pub(crate) async fn heartbeat_claim(
        &self,
        c: &ModelLaneRoutingStageClaim,
        s: ModelLaneRoutingStageStateKind,
        i: Option<String>,
        l: Option<String>,
        r: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        if !matches!(
            s,
            ModelLaneRoutingStageStateKind::InFlight
                | ModelLaneRoutingStageStateKind::AwaitingAuthority
        ) {
            return Err("heartbeat state invalid".into());
        }
        self.record_stage_result(
            c,
            s,
            i,
            l,
            r,
            vec![],
            None,
            None,
            None,
            None,
            Some("lease heartbeat".into()),
        )
        .await
    }
    pub(crate) async fn validate_active_claim(
        &self,
        c: &ModelLaneRoutingStageClaim,
    ) -> Result<(), String> {
        claim(
            &self
                .snapshot(&c.execution_id)
                .await?
                .ok_or("unknown execution")?,
            c,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn record_stage_result(
        &self,
        c: &ModelLaneRoutingStageClaim,
        s: ModelLaneRoutingStageStateKind,
        i: Option<String>,
        l: Option<String>,
        r: Option<String>,
        ins: Vec<String>,
        o: Option<String>,
        m: Option<String>,
        h: Option<String>,
        p: Option<Value>,
        d: Option<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let x = self
            .snapshot(&c.execution_id)
            .await?
            .ok_or("unknown execution")?;
        let (n, a) = self.transition(&x, c, s, i, l, r, ins, o, m, h, p, d)?;
        let status = obox(s);
        let e = events(&n, &a, status, "transition")?;
        self.model_lane_store
            .commit_routing_execution_atomic(x.revision, Some(c), n, a, status, e)
            .await
            .map_err(|e| e.to_string())
    }
    #[allow(clippy::too_many_arguments)]
    fn transition(
        &self,
        x: &ModelLaneRoutingExecutionState,
        c: &ModelLaneRoutingStageClaim,
        s: ModelLaneRoutingStageStateKind,
        i: Option<String>,
        l: Option<String>,
        r: Option<String>,
        ins: Vec<String>,
        o: Option<String>,
        m: Option<String>,
        h: Option<String>,
        p: Option<Value>,
        d: Option<String>,
    ) -> Result<(ModelLaneRoutingExecutionState, ModelLaneRoutingStageState), String> {
        claim(x, c)?;
        let old = x.stages[&c.stage_id].clone();
        if old.state != s && !valid_transition(old.state, s) {
            return Err("invalid routing transition".into());
        }
        if s.is_success()
            && (o.is_none()
                || (old.dispatch_target != ModelLaneRoutingDispatchTarget::CoordinatorJoin
                    && m.is_none()))
        {
            return Err("successful routing stage lacks output authority".into());
        }
        if let Some(v) = p.as_ref() {
            if v.get("artifact_sha256").and_then(Value::as_str) != h.as_deref() {
                return Err("output hash mismatch".into());
            }
        }
        let now = now_ms();
        let active = matches!(
            s,
            ModelLaneRoutingStageStateKind::InFlight
                | ModelLaneRoutingStageStateKind::AwaitingAuthority
        );
        let mut a = ModelLaneRoutingStageState {
            stage_id: old.stage_id.clone(),
            state: s,
            instance_id: i,
            lane_id: l,
            authority_request_message_ref: r.or_else(|| old.authority_request_message_ref.clone()),
            input_refs: if ins.is_empty() {
                old.input_refs.clone()
            } else {
                ins
            },
            output_ref: o,
            output_message_ref: m,
            output_sha256: h,
            output_payload: p,
            lease_owner: active.then(|| c.lease_owner.clone()),
            fencing_token: active.then(|| c.fencing_token.clone()),
            lease_expires_at_unix_ms: active.then(|| now + self.lease_ms),
            detail: d,
            updated_at_unix_ms: now,
            ..old
        };
        a.event_ledger_event_id.clear();
        a.event_ledger_seq = 0;
        let mut n = x.clone();
        n.stages.insert(c.stage_id.clone(), a.clone());
        n.revision = x.revision + 1;
        refresh_execution_status(&mut n)?;
        Ok((n, a))
    }
    pub(crate) async fn record_authority_request(
        &self,
        c: &ModelLaneRoutingStageClaim,
        l: String,
        m: super::model_lane::NewModelLaneMessage,
    ) -> Result<
        (
            ModelLaneRoutingExecutionState,
            super::model_lane::ModelLaneMessageRecord,
        ),
        String,
    > {
        let x = self
            .snapshot(&c.execution_id)
            .await?
            .ok_or("unknown execution")?;
        let (n, a) = self.transition(
            &x,
            c,
            ModelLaneRoutingStageStateKind::AwaitingAuthority,
            None,
            Some(l),
            Some(m.message_id.clone()),
            vec![],
            None,
            None,
            None,
            None,
            Some("awaiting authority".into()),
        )?;
        let e = events(&n, &a, "claimed", "authority")?;
        self.model_lane_store
            .commit_routing_authority_request_atomic(x.revision, c, n, a, "claimed", m, e)
            .await
            .map_err(|e| e.to_string())
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
        let model_lane_store = &self.model_lane_store;
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
        let model_lane_store = &self.model_lane_store;
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
        let output_message_id = (stage.dispatch_target
            != ModelLaneRoutingDispatchTarget::CoordinatorJoin)
            .then_some(message_id);
        let pointer = json!({"schema_id":"hsk.model_lane_routing_output_pointer@1","artifact_ref":output_ref,"message_ref":output_message_id,"artifact_sha256":output_sha256,"typed_output":typed_output});
        let (next, changed) = self.transition(
            &snapshot,
            claim,
            state,
            instance_id,
            lane_id,
            None,
            stage.input_refs.clone(),
            Some(output_ref.clone()),
            output_message_id.clone(),
            Some(output_sha256),
            Some(pointer),
            detail,
        )?;
        let events = events(&next, &changed, "acked", "output")?;
        let (stored, m, b) = self
            .model_lane_store
            .commit_routing_generated_output_atomic(
                snapshot.revision,
                claim,
                next,
                changed,
                "acked",
                (stage.dispatch_target != ModelLaneRoutingDispatchTarget::CoordinatorJoin)
                    .then_some(message),
                binding,
                events,
            )
            .await
            .map_err(|e| e.to_string())?;
        if b.inner.artifact_ref != output_ref
            || m.as_ref().map(|v| v.message_id.as_str()) != output_message_id.as_deref()
        {
            return Err("atomic routing output identity mismatch".into());
        }
        Ok(stored)
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

    pub(crate) async fn recover_expired_claims(&self, id: &str) -> Result<Vec<String>, String> {
        let mut x = self.snapshot(id).await?.ok_or("unknown execution")?;
        let mut done = vec![];
        for sid in x.stages.keys().cloned().collect::<Vec<_>>() {
            let old = x.stages[&sid].clone();
            let expired = matches!(
                old.state,
                ModelLaneRoutingStageStateKind::Claimed
                    | ModelLaneRoutingStageStateKind::InFlight
                    | ModelLaneRoutingStageStateKind::AwaitingAuthority
            ) && old.lease_expires_at_unix_ms.is_some_and(|v| v <= now_ms());
            let interrupted = old.state == ModelLaneRoutingStageStateKind::Compensated
                && old.detail.as_deref() == Some("expired lease compensated; requeue pending");
            if !expired && !interrupted {
                continue;
            }
            if expired {
                let rev = x.revision;
                let mut compensated = old.clone();
                compensated.state = ModelLaneRoutingStageStateKind::Compensated;
                compensated.lease_owner = None;
                compensated.fencing_token = None;
                compensated.lease_expires_at_unix_ms = None;
                compensated.detail = Some("expired lease compensated; requeue pending".into());
                compensated.updated_at_unix_ms = now_ms();
                x.stages.insert(sid.clone(), compensated.clone());
                x.revision = rev + 1;
                let e = events(&x, &compensated, "compensated", "compensate")?;
                x = self
                    .model_lane_store
                    .commit_routing_execution_atomic(rev, None, x, compensated, "compensated", e)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            let rev = x.revision;
            let mut a = x.stages[&sid].clone();
            a.state = if a.attempt >= MAX_STAGE_ATTEMPTS {
                ModelLaneRoutingStageStateKind::Failed
            } else {
                ModelLaneRoutingStageStateKind::Scheduled
            };
            if a.state == ModelLaneRoutingStageStateKind::Scheduled {
                a.attempt += 1;
                done.push(sid.clone())
            }
            a.lease_owner = None;
            a.fencing_token = None;
            a.lease_expires_at_unix_ms = None;
            a.updated_at_unix_ms = now_ms();
            x.stages.insert(sid, a.clone());
            x.revision = rev + 1;
            let status = if a.state == ModelLaneRoutingStageStateKind::Scheduled {
                "pending"
            } else {
                "acked"
            };
            let e = events(&x, &a, status, "recover")?;
            x = self
                .model_lane_store
                .commit_routing_execution_atomic(rev, None, x, a, status, e)
                .await
                .map_err(|e| e.to_string())?
        }
        Ok(done)
    }
    pub(crate) async fn expired_stage_attempts(
        &self,
        id: &str,
    ) -> Result<Vec<(String, u32, String)>, String> {
        let x = self.snapshot(id).await?.ok_or("unknown execution")?;
        Ok(x.stages
            .values()
            .filter(|v| v.lease_expires_at_unix_ms.is_some_and(|e| e <= now_ms()))
            .filter_map(|v| {
                v.instance_id
                    .clone()
                    .map(|i| (v.stage_id.clone(), v.attempt, i))
            })
            .collect())
    }
    pub(crate) async fn cancel_execution(
        &self,
        id: &str,
        reason: impl Into<String>,
    ) -> Result<ModelLaneRoutingExecutionState, String> {
        let reason = reason.into();
        let mut x = self.snapshot(id).await?.ok_or("unknown execution")?;
        if x.status == ModelLaneRoutingExecutionStatus::Cancelled
            && x.stages.values().all(|stage| stage.state.is_terminal())
        {
            return Ok(x);
        }
        for sid in x
            .stages
            .values()
            .filter(|v| !v.state.is_terminal())
            .map(|v| v.stage_id.clone())
            .collect::<Vec<_>>()
        {
            let rev = x.revision;
            let mut a = x.stages[&sid].clone();
            a.state = ModelLaneRoutingStageStateKind::Cancelled;
            a.detail = Some(reason.clone());
            a.lease_owner = None;
            a.fencing_token = None;
            a.lease_expires_at_unix_ms = None;
            a.updated_at_unix_ms = now_ms();
            x.stages.insert(sid, a.clone());
            x.status = ModelLaneRoutingExecutionStatus::Cancelled;
            x.cancel_reason = Some(reason.clone());
            x.revision = rev + 1;
            let e = events(&x, &a, "cancelled", "cancel")?;
            x = self
                .model_lane_store
                .commit_routing_execution_atomic(rev, None, x, a, "cancelled", e)
                .await
                .map_err(|e| e.to_string())?
        }
        Ok(x)
    }
}
fn check_run(r: &ModelLaneRunRecord, c: &ModelLaneRoutingExecutionContext) -> Result<(), String> {
    if r.run_id != c.run_id
        || r.trace_id != c.trace_id
        || r.run_span_id != c.run_span_id
        || r.coordinator_session_id != c.coordinator_session_id
        || r.work_packet_id.as_deref() != Some(&c.work_packet_id)
        || r.micro_task_id != c.micro_task_id
        || r.task_board_id.as_deref() != Some(&c.task_board_id)
        || r.owner_session != c.owner_session
    {
        return Err("run context mismatch".into());
    }
    Ok(())
}
fn check_launches(
    x: &ModelLaneRoutingExecutionState,
    ls: &[ModelLaneRoutingStageLaunch],
) -> Result<(), String> {
    let vs = stage_views(&x.canonical_graph)?;
    let mut seen = BTreeSet::new();
    for l in ls {
        if !seen.insert(&l.stage_id) {
            return Err("duplicate launch".into());
        }
        let v = vs
            .iter()
            .find(|v| v.stage_id == l.stage_id)
            .ok_or("unknown launch stage")?;
        let p = x
            .canonical_launch_plan
            .iter()
            .find(|p| p.stage_id == l.stage_id)
            .ok_or("missing launch plan")?;
        if l.expected_run_id != x.run_id || p.dispatch_target != v.dispatch_target {
            return Err("launch authority mismatch".into());
        }
        if matches!(
            v.dispatch_target,
            ModelLaneRoutingDispatchTarget::LocalModel | ModelLaneRoutingDispatchTarget::CloudModel
        ) {
            let r = l.request.as_ref().ok_or("missing spawn request")?;
            let d = r
                .dexterity_launch
                .as_ref()
                .ok_or("missing launch contract")?;
            if l.generate_request.is_none()
                || d.run_id != l.expected_run_id
                || d.lane_id != l.expected_lane_id
                || r.instance_id.model_id.to_string() != l.expected_model_id
                || r.provider != l.expected_provider
                || p.lane_id.as_deref() != Some(&l.expected_lane_id)
                || p.model_id.as_deref() != Some(&l.expected_model_id)
                || p.provider != l.expected_provider
            {
                return Err("model launch mismatch".into());
            }
        }
    }
    Ok(())
}
fn stage(
    x: &ModelLaneRoutingExecutionState,
    v: &StageView,
    n: u32,
) -> Result<ModelLaneRoutingStageState, String> {
    let p = x
        .canonical_launch_plan
        .iter()
        .find(|p| p.stage_id == v.stage_id)
        .ok_or("missing stage plan")?;
    let mut ins = predecessor_output_refs(v, &x.stages);
    if let Some(i) = x.initial_input_ref.clone() {
        ins.push(i)
    }
    Ok(ModelLaneRoutingStageState {
        stage_id: v.stage_id.clone(),
        state: ModelLaneRoutingStageStateKind::Scheduled,
        attempt: n,
        dispatch_target: v.dispatch_target,
        expected_run_id: x.run_id.clone(),
        expected_lane_id: p.lane_id.clone().unwrap_or_default(),
        expected_model_id: p.model_id.clone().unwrap_or_default(),
        expected_provider: p.provider,
        instance_id: None,
        lane_id: None,
        input_refs: ins,
        output_ref: None,
        output_message_ref: None,
        authority_request_message_ref: None,
        output_sha256: None,
        output_payload: None,
        authority_ref: authority_for_stage(v, &x.authority)?,
        lease_owner: None,
        fencing_token: None,
        lease_expires_at_unix_ms: None,
        detail: None,
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        updated_at_unix_ms: now_ms(),
    })
}
fn claim(x: &ModelLaneRoutingExecutionState, c: &ModelLaneRoutingStageClaim) -> Result<(), String> {
    let a = x.stages.get(&c.stage_id).ok_or("unknown stage")?;
    if c.execution_id != x.execution_id
        || a.attempt != c.attempt
        || a.lease_owner.as_deref() != Some(&c.lease_owner)
        || a.fencing_token.as_deref() != Some(&c.fencing_token)
        || a.lease_expires_at_unix_ms.is_none_or(|v| v < now_ms())
    {
        return Err("stale routing claim".into());
    }
    Ok(())
}
fn obox(s: ModelLaneRoutingStageStateKind) -> &'static str {
    if matches!(
        s,
        ModelLaneRoutingStageStateKind::Claimed
            | ModelLaneRoutingStageStateKind::InFlight
            | ModelLaneRoutingStageStateKind::AwaitingAuthority
    ) {
        "claimed"
    } else {
        "acked"
    }
}
fn events(
    x: &ModelLaneRoutingExecutionState,
    a: &ModelLaneRoutingStageState,
    status: &str,
    act: &str,
) -> Result<Vec<NewKernelEvent>, String> {
    let k = if a.state.is_success() {
        KernelEventType::ModelResponseRecorded
    } else if matches!(
        a.state,
        ModelLaneRoutingStageStateKind::Failed
            | ModelLaneRoutingStageStateKind::Cancelled
            | ModelLaneRoutingStageStateKind::Compensated
    ) {
        KernelEventType::SessionFailed
    } else {
        KernelEventType::ModelAdapterInvoked
    };
    let aid = format!("{}:{}:{}", x.execution_id, a.stage_id, a.attempt);
    let oid = format!(
        "routing-command:{}:{}:{}",
        x.execution_id, a.stage_id, a.attempt
    );
    let e = |t: &str, id: &str, key: String, p: Value| {
        NewKernelEvent::builder(
            x.run_id.clone(),
            x.execution_id.clone(),
            k.clone(),
            KernelActor::ModelAdapter("DexterityRoutingExecutor".into()),
        )
        .aggregate(t, id)
        .idempotency_key(key)
        .correlation_id(format!("dexterity-routing:{}", x.execution_id))
        .source_component(SOURCE_COMPONENT)
        .payload(p)
        .build()
        .map_err(|e| e.to_string())
    };
    Ok(vec![
        e(
            "model_lane_routing_execution",
            &x.execution_id,
            format!("routing:{act}:execution:{}:{}", x.execution_id, x.revision),
            json!({"schema_id":ROUTING_EXECUTION_SCHEMA_ID,"record":x}),
        )?,
        e(
            "model_lane_routing_stage_attempt",
            &aid,
            format!("routing:{act}:attempt:{aid}:{}", x.revision),
            json!({"schema_id":ROUTING_STAGE_ATTEMPT_SCHEMA_ID,"record":a}),
        )?,
        e(
            "model_lane_routing_outbox",
            &oid,
            format!("routing:{act}:outbox:{oid}:{}", x.revision),
            json!({"schema_id":ROUTING_OUTBOX_SCHEMA_ID,"status":status}),
        )?,
    ])
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
