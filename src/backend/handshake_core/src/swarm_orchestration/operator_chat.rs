//! WP-1 MT-012: Operator chat/launch work-surface backend engine.
//!
//! This module is the net-new binding the MT-012 contract calls for: it re-homes
//! the WP-KERNEL-004 CLI-wrapper capture ([`AgentActivity`], produced by
//! [`parse_agent_activity_line`]) onto WP-1 ModelLane authority. Every captured
//! *completed* activity block becomes ONE typed [`NewModelLaneMessage`] recorded
//! under a live [`ModelLaneRun`](super::model_lane) via
//! [`ModelLaneStore::record_message`] (EventLedger authority) AND one Flight
//! Recorder `FR-EVT-AGENT-*` business event. The operator's own turn is persisted
//! as a `HUMAN_OPERATOR` ModelLane message.
//!
//! Launch resolves ONLY through [`SwarmCoordinator::spawn_session`]
//! (fails closed without a `ModelLaneStore`). Selection of a model/worktree is
//! recorded as a distinct auditable decision through the MT-014
//! [`ModelCatalog::record_selection_decision`] primitive.
//!
//! Message-kind mapping (contract F1, spec 4.3.9.2.5):
//! * `AgentActivity::ToolCall`                 -> `ModelLaneMessageKind::ToolRequest`
//! * `AgentActivity::Text` rendered tool_result -> `ModelLaneMessageKind::ToolResult`
//! * operator prompt / model answer / thought  -> `ModelLaneMessageKind::Status`,
//!   discriminated by `diagnostic_payload.activity_kind`
//!   (`tool_call|thinking|text|other`, from [`AgentActivityKind::label`]).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::flight_recorder::events_agent_activity::agent_activity_event;
use crate::flight_recorder::{FlightRecorder, RecorderError};
use crate::model_runtime::catalog::ModelCatalog;
use crate::model_runtime::cloud::{
    enumerate_cloud_access, parse_agent_activity_line, AgentActivity, AgentActivityKind, CliKind,
    CliOutputFormat, ProviderAccessRegistry,
};
use crate::model_runtime::{ModelId, ProviderKind};
use crate::terminal::redaction::{PatternRedactor, SecretRedactor};

use super::error::SwarmError;
use super::ids::{ByokCloudProvider, ModelInstanceId, SpawnRequest};
use super::model_lane::{
    DexterityLaunchContract, ModelLaneAuthority, ModelLaneError, ModelLaneMessageKind,
    ModelLaneRecord, ModelLaneRoutingMetadata, ModelLaneRunRecord, ModelLaneStore,
    ModelLaneTarget, NewModelLaneMessage,
};
use super::SwarmCoordinator;

/// Stable surface id carried on every operator-chat capture message so
/// diagnostic/projection tooling can filter this work-surface without prose.
pub const OPERATOR_CHAT_SURFACE_ID: &str = "wp1_operator_chat_launch";

/// Adapter label stamped on the `FR-EVT-AGENT-*` events this surface emits.
pub const OPERATOR_CHAT_CLI_ADAPTER: &str = "operator_chat_cli_bridge";

/// Prefix that [`parse_agent_activity_line`] uses when it renders a CLI
/// `tool_result` content block as a [`AgentActivity::Text`]. Detecting it lets us
/// map a rendered tool_result to [`ModelLaneMessageKind::ToolResult`] per F1.
pub const RENDERED_TOOL_RESULT_PREFIX: &str = "[tool_result]";

/// Errors surfaced by the operator chat/launch engine.
#[derive(Debug, thiserror::Error)]
pub enum OperatorChatError {
    #[error("model lane store error: {0}")]
    ModelLane(#[from] ModelLaneError),
    #[error("flight recorder error: {0}")]
    Recorder(#[from] RecorderError),
    #[error("swarm error: {0}")]
    Swarm(#[from] SwarmError),
    #[error("operator chat selection invalid: {0}")]
    Invalid(String),
}

/// Which lane kind the operator picked in the chat/launch pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorChatLaneKind {
    /// Embedded local ModelRuntime lane (enumerated via [`ModelCatalog`]).
    Local,
    /// BYOK cloud lane (enumerated via the MT-015 cloud enumeration API).
    Cloud,
    /// Official CLI bridge lane (claude/codex) — the CLI-wrapper capture path.
    Cli,
}

/// One operator launch request assembled from the pane selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSelection {
    pub lane_kind: OperatorChatLaneKind,
    /// Concrete model id the operator selected (local model id or cloud model name).
    pub model_id: String,
    /// For `Cloud`: the BYOK provider flavor (`anthropic` | `openai`).
    #[serde(default)]
    pub cloud_provider: Option<String>,
    /// Operator-selected folder/worktree path. Becomes the REAL CLI subprocess
    /// cwd (plumbed `SpawnRequest.working_dir -> CliBridgeConfig.working_dir`).
    pub working_dir: String,
    /// Optional worktree id attribution.
    #[serde(default)]
    pub worktree_id: Option<String>,
    /// The operator's prompt / message for this turn.
    pub prompt: String,
    /// Owner session identity (governs the process ledger + operator-lane authority).
    pub owner_session: String,
    /// Parent session that requested the launch.
    pub parent_session_id: String,
    /// Locus work-packet attribution (defaults to the operator-chat workspace).
    #[serde(default)]
    pub work_packet_id: Option<String>,
    /// Locus micro-task attribution (defaults to the operator turn).
    #[serde(default)]
    pub micro_task_id: Option<String>,
}

impl OperatorChatSelection {
    fn work_packet(&self) -> &str {
        self.work_packet_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("operator-chat-workspace")
    }

    fn micro_task(&self) -> &str {
        self.micro_task_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("operator-chat-turn")
    }
}

/// Result of a successful operator launch: the persisted CLI ModelLane ids so the
/// pane can bind its transcript to the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatLaunched {
    pub instance_id: String,
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub lane_kind: OperatorChatLaneKind,
}

/// Non-secret model inventory for the picker: local models (MT-014) + cloud
/// access rows (MT-015). Serializable so the enumeration route returns it verbatim.
#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatModelInventory {
    pub local: Vec<OperatorChatModelRow>,
    pub cloud_byok: Vec<OperatorChatCloudRow>,
    pub cloud_cli_bridge: Vec<OperatorChatCloudRow>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatModelRow {
    pub model_id: String,
    pub display_name: String,
    pub runtime_binding: String,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatCloudRow {
    pub provider: String,
    pub label: String,
    /// `"configured"` | `"unavailable"` (fail-closed when a provider is absent).
    pub status: String,
}

/// The plan for turning ONE captured activity into a typed lane message.
#[derive(Debug, Clone)]
pub struct CapturedMessagePlan {
    pub kind: ModelLaneMessageKind,
    /// `tool_call | thinking | text | other`, from [`AgentActivityKind::label`].
    pub activity_kind: &'static str,
    pub summary: String,
    pub payload: Value,
    /// Tool messages require at least one `tool_gate_decision_ref` (validator law).
    pub is_tool: bool,
}

fn is_rendered_tool_result(text: &str) -> bool {
    text.trim_start().starts_with(RENDERED_TOOL_RESULT_PREFIX)
}

fn short_summary(text: &str, fallback: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    let mut out: String = trimmed.chars().take(120).collect();
    if trimmed.chars().count() > 120 {
        out.push('…');
    }
    out
}

/// Map one captured [`AgentActivity`] to its lane-message plan (contract F1).
///
/// Pure + deterministic so the mapping is unit-testable without a database.
pub fn plan_from_activity(activity: &AgentActivity) -> CapturedMessagePlan {
    let activity_kind = activity.kind().label();
    match activity {
        AgentActivity::ToolCall {
            name,
            input,
            call_id,
        } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::ToolRequest,
            activity_kind,
            summary: format!("tool_call:{name}"),
            payload: json!({
                "activity_kind": activity_kind,
                "name": name,
                "input": input,
                "call_id": call_id,
            }),
            is_tool: true,
        },
        AgentActivity::Text { text } if is_rendered_tool_result(text) => CapturedMessagePlan {
            kind: ModelLaneMessageKind::ToolResult,
            activity_kind,
            summary: short_summary(text, "tool_result"),
            payload: json!({
                "activity_kind": activity_kind,
                "rendered_tool_result": true,
                "text": text,
            }),
            is_tool: true,
        },
        AgentActivity::Text { text } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind,
            summary: short_summary(text, "model_text"),
            payload: json!({ "activity_kind": activity_kind, "text": text }),
            is_tool: false,
        },
        AgentActivity::Thinking { text } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind,
            summary: short_summary(text, "model_thought"),
            payload: json!({ "activity_kind": activity_kind, "text": text }),
            is_tool: false,
        },
        AgentActivity::Other { raw } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind,
            summary: short_summary(raw, "cli_other"),
            payload: json!({ "activity_kind": activity_kind, "raw": raw }),
            is_tool: false,
        },
    }
}

fn message_kind_label(kind: &ModelLaneMessageKind) -> &'static str {
    match kind {
        ModelLaneMessageKind::Proposal => "proposal",
        ModelLaneMessageKind::Critique => "critique",
        ModelLaneMessageKind::ToolRequest => "tool_request",
        ModelLaneMessageKind::ToolResult => "tool_result",
        ModelLaneMessageKind::Status => "status",
        ModelLaneMessageKind::PromotionRequest => "promotion_request",
        ModelLaneMessageKind::Recovery => "recovery",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Build a validation-passing [`NewModelLaneMessage`] under `run`/`lane` for a
/// captured activity. The message reuses the lane's locus binding so
/// `validate_locus_common` holds; `linked_span_contexts`, `routing`, and
/// `tool_gate_decision_refs` satisfy the ModelLaneStore message validators.
pub fn build_captured_message(
    run: &ModelLaneRunRecord,
    lane: &ModelLaneRecord,
    plan: &CapturedMessagePlan,
    ordered_index: u64,
    turn_role: &str,
) -> Result<NewModelLaneMessage, OperatorChatError> {
    let locus = lane.locus_binding.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "lane {} is missing a locus_binding required for capture",
            lane.lane_id
        ))
    })?;

    let payload_bytes = serde_json::to_vec(&plan.payload)
        .map_err(|e| OperatorChatError::Invalid(format!("payload serialize failed: {e}")))?;
    let payload_sha256 = sha256_hex(&payload_bytes);

    let message_id = format!("mlm-{}-{turn_role}-{ordered_index}", lane.lane_id);
    let message_span_id = format!("span-msg-{}-{turn_role}-{ordered_index}", lane.lane_id);
    // linked_span_contexts must be non-empty and MUST NOT contain message_span_id.
    let linked_span_contexts = vec![run.run_span_id.clone()];

    let tool_gate_decision_refs = if plan.is_tool {
        vec![format!(
            "toolgate://operator-chat/{}/observed/{ordered_index}",
            lane.lane_id
        )]
    } else {
        Vec::new()
    };

    let diagnostic_payload = json!({
        "activity_kind": plan.activity_kind,
        "message_kind": message_kind_label(&plan.kind),
        "surface": OPERATOR_CHAT_SURFACE_ID,
        "turn_role": turn_role,
        "ordered_index": ordered_index,
        "capture": plan.payload,
    });

    Ok(NewModelLaneMessage {
        message_id: message_id.clone(),
        run_id: run.run_id.clone(),
        trace_id: run.trace_id.clone(),
        message_span_id,
        parent_span_id: Some(lane.lane_span_id.clone()),
        linked_span_contexts,
        from_lane_id: lane.lane_id.clone(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "operator_chat_coordinator".to_string(),
            target_session: run.coordinator_session_id.clone(),
            correlation_id: run.run_id.clone(),
            requires_ack: false,
            ack_for: None,
        }),
        kind: plan.kind.clone(),
        payload_ref: format!("artifact://operator-chat/{}/{message_id}", run.run_id),
        payload_sha256,
        event_ledger_stream_id: run.event_ledger_stream_id.clone(),
        summary: plan.summary.clone(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs,
        coordinator_session_id: locus.coordinator_session_id.clone(),
        work_packet_id: Some(locus.work_packet_id.clone()),
        micro_task_id: Some(locus.micro_task_id.clone()),
        task_board_id: locus.task_board_id.clone(),
        owner_session: locus.owner_session.clone(),
        locus_binding: Some(locus),
        idempotency_key: format!(
            "operator-chat::{}::{}::{turn_role}::{ordered_index}",
            run.run_id, lane.lane_id
        ),
        replay_order_key: format!("operator-chat-{ordered_index:020}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: None,
        created_at_utc: Utc::now().to_rfc3339(),
        diagnostic_payload,
    })
}

/// Records CLI-wrapper capture as typed ModelLane messages + Flight Recorder
/// business events. This is the core net-new binding of MT-012.
pub struct ModelLaneCaptureRecorder {
    store: ModelLaneStore,
    recorder: Arc<dyn FlightRecorder>,
    redactor: Arc<dyn SecretRedactor>,
}

impl ModelLaneCaptureRecorder {
    pub fn new(store: ModelLaneStore, recorder: Arc<dyn FlightRecorder>) -> Self {
        Self {
            store,
            recorder,
            redactor: Arc::new(PatternRedactor),
        }
    }

    pub fn with_redactor(mut self, redactor: Arc<dyn SecretRedactor>) -> Self {
        self.redactor = redactor;
        self
    }

    /// Record ONE captured activity: emit the Flight Recorder `FR-EVT-AGENT-*`
    /// business event, then persist the typed ModelLaneMessage under the run/lane.
    pub async fn record_activity(
        &self,
        run: &ModelLaneRunRecord,
        lane: &ModelLaneRecord,
        model_id: ModelId,
        request_id: Uuid,
        ordered_index: u64,
        activity: &AgentActivity,
    ) -> Result<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord, OperatorChatError>
    {
        // Tier-1 Flight Recorder business event for the raw activity.
        let event = agent_activity_event(
            model_id,
            request_id,
            ordered_index,
            Some(lane.model_session_id.as_str()),
            OPERATOR_CHAT_CLI_ADAPTER,
            activity,
            self.redactor.as_ref(),
        );
        self.recorder.record_event(event).await?;

        // EventLedger-authority ModelLaneMessage.
        let plan = plan_from_activity(activity);
        let message = build_captured_message(run, lane, &plan, ordered_index, "model")?;
        Ok(self.store.record_message(message).await?)
    }

    /// Capture a whole CLI stdout stream: for every line, run the real
    /// [`parse_agent_activity_line`] and persist ONE ModelLaneMessage per
    /// *completed* activity block. Streaming delta / `item.updated` lines yield
    /// no activities, so there is exactly one message per completed block (F5).
    ///
    /// `start_index` is the first `ordered_index` to use (so successive capture
    /// calls on the same lane keep distinct, monotonic idempotency keys). Returns
    /// the persisted messages; the next free index is `start_index + records.len()`.
    pub async fn capture_cli_stream<I, S>(
        &self,
        run: &ModelLaneRunRecord,
        lane: &ModelLaneRecord,
        model_id: ModelId,
        request_id: Uuid,
        cli_kind: CliKind,
        start_index: u64,
        stdout_lines: I,
    ) -> Result<Vec<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord>, OperatorChatError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut records = Vec::new();
        let mut ordered_index: u64 = start_index;
        for line in stdout_lines {
            for activity in parse_agent_activity_line(cli_kind, line.as_ref()) {
                let record = self
                    .record_activity(run, lane, model_id, request_id, ordered_index, &activity)
                    .await?;
                records.push(record);
                ordered_index += 1;
            }
        }
        Ok(records)
    }

    /// Persist the operator's own prompt as a typed message under a
    /// `HUMAN_OPERATOR` ModelLane (contract F1 / spec 1619). The prompt is a
    /// `Status` message discriminated by `diagnostic_payload.activity_kind="text"`.
    pub async fn record_operator_prompt(
        &self,
        operator_run: &ModelLaneRunRecord,
        operator_lane: &ModelLaneRecord,
        prompt: &str,
    ) -> Result<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord, OperatorChatError>
    {
        let plan = CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind: AgentActivityKind::Text.label(),
            summary: short_summary(prompt, "operator_prompt"),
            payload: json!({
                "activity_kind": AgentActivityKind::Text.label(),
                "turn_role": "operator",
                "text": prompt,
            }),
            is_tool: false,
        };
        let message = build_captured_message(operator_run, operator_lane, &plan, 0, "operator")?;
        Ok(self.store.record_message(message).await?)
    }
}

/// Force the CLI bridge into a JSON-stream output format so captured activities
/// are TYPED (`tool_call`/`thinking`/`text`) instead of a wall of `Other{raw}`
/// (contract F8). Idempotently ensures a `--output-format stream-json` flag pair
/// is present in the arg template and sets [`CliOutputFormat::JsonStream`].
pub fn force_json_stream_output(
    mut config: crate::model_runtime::cloud::CliBridgeConfig,
) -> crate::model_runtime::cloud::CliBridgeConfig {
    config.output_format = CliOutputFormat::JsonStream;
    let already = config
        .args_template
        .iter()
        .any(|arg| arg == "stream-json" || arg == "--json" || arg == "json-stream");
    if !already {
        config.args_template.push("--output-format".to_string());
        config.args_template.push("stream-json".to_string());
    }
    config
}

/// Launch service for the operator chat/launch surface. Owns the sanctioned
/// launch authority ([`SwarmCoordinator::spawn_session`]), the local model
/// catalog (MT-014), and the Flight Recorder (for the selection-decision audit).
#[derive(Clone)]
pub struct OperatorChatLaunchService {
    coordinator: Arc<SwarmCoordinator>,
    catalog: Arc<ModelCatalog>,
    recorder: Arc<dyn FlightRecorder>,
    instance_counter: Arc<AtomicU32>,
}

impl OperatorChatLaunchService {
    pub fn new(
        coordinator: Arc<SwarmCoordinator>,
        catalog: Arc<ModelCatalog>,
        recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            coordinator,
            catalog,
            recorder,
            instance_counter: Arc::new(AtomicU32::new(1)),
        }
    }

    /// Enumerate the picker inventory: local models (MT-014 [`ModelCatalog::list`])
    /// plus cloud access rows (MT-015 [`enumerate_cloud_access`]). A cloud provider
    /// that is not configured degrades to `unavailable` rather than erroring.
    pub fn enumerate_models(
        &self,
        cloud_registry: &dyn ProviderAccessRegistry,
    ) -> OperatorChatModelInventory {
        let local = self
            .catalog
            .list()
            .into_iter()
            .map(|entry| OperatorChatModelRow {
                model_id: entry.model_id,
                display_name: entry.display_name,
                runtime_binding: entry.runtime_binding,
                ready: entry.ready,
            })
            .collect();
        let cloud = enumerate_cloud_access(cloud_registry);
        let cloud_byok = cloud
            .byok
            .into_iter()
            .map(|row| OperatorChatCloudRow {
                provider: row.provider.to_string(),
                label: row.label.to_string(),
                status: provider_access_status_label(row.status),
            })
            .collect();
        let cloud_cli_bridge = cloud
            .cli_bridge
            .into_iter()
            .map(|row| OperatorChatCloudRow {
                provider: row.provider.to_string(),
                label: row.label.to_string(),
                status: "offered".to_string(),
            })
            .collect();
        OperatorChatModelInventory {
            local,
            cloud_byok,
            cloud_cli_bridge,
            excluded: cloud.excluded.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Record the operator's model/worktree selection as a distinct auditable
    /// decision (spec 4.3.9.4.4). Wires the MT-014
    /// [`ModelCatalog::record_selection_decision`] primitive — this closes
    /// MT-014 MED-1. Distinct from launch.
    pub async fn record_selection(
        &self,
        selected_model_id: &str,
        actor: &str,
        reason: &str,
    ) -> Result<(), OperatorChatError> {
        self.catalog
            .record_selection_decision(self.recorder.as_ref(), selected_model_id, actor, reason)
            .await
            .map_err(OperatorChatError::from)
    }

    /// Build the [`SpawnRequest`] for a selection (pure; unit-testable). Attaches a
    /// coordinator-generated Dexterity launch contract and plumbs the operator
    /// working_dir so a CLI lane truly runs there.
    pub fn build_spawn_request(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<SpawnRequest, OperatorChatError> {
        build_spawn_request(selection, self.next_instance())
    }

    fn next_instance(&self) -> u32 {
        self.instance_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Launch a CLI/local/cloud lane for the operator selection through the
    /// sanctioned [`SwarmCoordinator::spawn_session`] authority. Fails closed if
    /// the coordinator has no `ModelLaneStore`.
    pub async fn launch(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        let request = self.build_spawn_request(selection)?;
        let contract = request
            .dexterity_launch
            .clone()
            .ok_or_else(|| OperatorChatError::Invalid("missing dexterity launch contract".into()))?;
        let instance_id = self.coordinator.spawn_session(request).await?;
        Ok(OperatorChatLaunched {
            instance_id: instance_id.to_string(),
            run_id: contract.run_id,
            lane_id: contract.lane_id,
            trace_id: contract.trace_id,
            lane_kind: selection.lane_kind,
        })
    }
}

fn provider_access_status_label(
    status: crate::model_runtime::cloud::ProviderAccessStatus,
) -> String {
    use crate::model_runtime::cloud::ProviderAccessStatus;
    match status {
        ProviderAccessStatus::Configured => "configured".to_string(),
        ProviderAccessStatus::Unavailable => "unavailable".to_string(),
    }
}

/// Build a [`SpawnRequest`] from an operator selection (pure). The launch resolves
/// provider/runtime from the lane kind, plumbs `working_dir`, and attaches the
/// Dexterity launch contract required by [`SwarmCoordinator::spawn_session`].
pub fn build_spawn_request(
    selection: &OperatorChatSelection,
    instance: u32,
) -> Result<SpawnRequest, OperatorChatError> {
    use crate::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;

    if selection.working_dir.trim().is_empty() {
        return Err(OperatorChatError::Invalid(
            "operator must select a working directory / worktree before launch".into(),
        ));
    }
    if selection.prompt.trim().is_empty() {
        return Err(OperatorChatError::Invalid("operator prompt is empty".into()));
    }

    let instance_id = ModelInstanceId::new(ModelId::new_v7(), instance);
    let mut request = SpawnRequest::new(
        instance_id,
        RuntimeAdapterBinding::Candle,
        selection.owner_session.clone(),
        selection.parent_session_id.clone(),
    )
    .with_working_dir(selection.working_dir.clone())
    .with_wp(selection.work_packet().to_string())
    .with_mt(selection.micro_task().to_string());

    if let Some(worktree_id) = selection.worktree_id.clone() {
        request = request.with_worktree(worktree_id);
    }

    request = match selection.lane_kind {
        OperatorChatLaneKind::Local => request,
        OperatorChatLaneKind::Cli => {
            request.with_cloud_provider(ProviderKind::OfficialCli, selection.model_id.clone())
        }
        OperatorChatLaneKind::Cloud => {
            let provider = match selection.cloud_provider.as_deref() {
                Some("anthropic") => ByokCloudProvider::Anthropic,
                Some("openai") => ByokCloudProvider::OpenAi,
                other => {
                    return Err(OperatorChatError::Invalid(format!(
                        "cloud lane requires cloud_provider anthropic|openai, got {other:?}"
                    )))
                }
            };
            request
                .with_cloud_provider(ProviderKind::ByokCloud, selection.model_id.clone())
                .with_byok_cloud_provider(provider)
        }
    };

    let contract = DexterityLaunchContract::from_spawn_request(&request)
        .map_err(OperatorChatError::from)?;
    Ok(request.with_dexterity_launch(contract))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_maps_to_tool_request_with_activity_kind() {
        let activity = AgentActivity::ToolCall {
            name: "Read".into(),
            input: json!({"path": "a.rs"}),
            call_id: Some("call-1".into()),
        };
        let plan = plan_from_activity(&activity);
        assert_eq!(plan.kind, ModelLaneMessageKind::ToolRequest);
        assert_eq!(plan.activity_kind, "tool_call");
        assert!(plan.is_tool);
    }

    #[test]
    fn rendered_tool_result_maps_to_tool_result() {
        let activity = AgentActivity::Text {
            text: "[tool_result] ok".into(),
        };
        let plan = plan_from_activity(&activity);
        assert_eq!(plan.kind, ModelLaneMessageKind::ToolResult);
        assert!(plan.is_tool);
    }

    #[test]
    fn thought_is_status_labelled_thinking_not_unlabelled() {
        let activity = AgentActivity::Thinking {
            text: "let me think".into(),
        };
        let plan = plan_from_activity(&activity);
        assert_eq!(plan.kind, ModelLaneMessageKind::Status);
        assert_eq!(plan.activity_kind, "thinking");
        assert!(!plan.is_tool);
    }

    #[test]
    fn model_text_is_status_labelled_text() {
        let plan = plan_from_activity(&AgentActivity::Text {
            text: "the answer is 42".into(),
        });
        assert_eq!(plan.kind, ModelLaneMessageKind::Status);
        assert_eq!(plan.activity_kind, "text");
    }

    #[test]
    fn other_is_status_labelled_other() {
        let plan = plan_from_activity(&AgentActivity::Other {
            raw: "weird".into(),
        });
        assert_eq!(plan.kind, ModelLaneMessageKind::Status);
        assert_eq!(plan.activity_kind, "other");
    }

    #[test]
    fn force_json_stream_sets_format_and_flag() {
        use crate::model_runtime::cloud::{CliBridgeConfig, CliKind, CliOutputFormat};
        use std::collections::HashMap;
        let config = CliBridgeConfig {
            cli_kind: CliKind::ClaudeCode,
            executable_path: "claude".into(),
            args_template: vec!["-p".into(), "{prompt}".into()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 60,
        };
        let forced = force_json_stream_output(config);
        assert_eq!(forced.output_format, CliOutputFormat::JsonStream);
        assert!(forced.args_template.iter().any(|a| a == "stream-json"));
        // Idempotent: re-forcing does not duplicate the flag.
        let again = force_json_stream_output(forced);
        assert_eq!(
            again.args_template.iter().filter(|a| *a == "stream-json").count(),
            1
        );
    }

    #[test]
    fn build_spawn_request_cli_sets_provider_working_dir_and_contract() {
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Cli,
            model_id: "claude-sonnet-4".into(),
            cloud_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: Some("wt-1".into()),
            prompt: "hello".into(),
            owner_session: "operator-1".into(),
            parent_session_id: "parent-1".into(),
            work_packet_id: None,
            micro_task_id: None,
        };
        let request = build_spawn_request(&selection, 7).expect("cli spawn request builds");
        assert_eq!(request.provider, Some(ProviderKind::OfficialCli));
        assert_eq!(request.working_dir.as_deref(), Some("D:/work/repo"));
        assert_eq!(request.cloud_model_name.as_deref(), Some("claude-sonnet-4"));
        assert!(request.dexterity_launch.is_some());
        assert_eq!(request.wp_id.as_deref(), Some("operator-chat-workspace"));
    }

    #[test]
    fn build_spawn_request_rejects_empty_working_dir() {
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Local,
            model_id: "m".into(),
            cloud_provider: None,
            working_dir: "  ".into(),
            worktree_id: None,
            prompt: "hi".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };
        assert!(matches!(
            build_spawn_request(&selection, 1),
            Err(OperatorChatError::Invalid(_))
        ));
    }
}
