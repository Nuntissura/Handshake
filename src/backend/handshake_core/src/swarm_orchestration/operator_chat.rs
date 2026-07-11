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
//! Launch resolves ONLY through sanctioned [`SwarmCoordinator`] authority:
//! process-backed lanes use `spawn_session`, and SUBAGENT uses the no-OS subagent
//! lane helper (both fail closed without a `ModelLaneStore`). Selection of a
//! model/worktree is recorded as a distinct auditable decision through the MT-014
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

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::flight_recorder::events_agent_activity::agent_activity_event;
use crate::flight_recorder::{FlightRecorder, RecorderError};
use crate::model_runtime::catalog::ModelCatalog;
use crate::model_runtime::cloud::{
    enumerate_cloud_access, parse_agent_activity_line, AgentActivity, AgentActivityKind, CliKind,
    CliOutputFormat, ProviderAccessRegistry, ProviderAccessStatus,
};
use crate::model_runtime::{
    CancellationToken, FinishReason, GenPrompt, GenerateRequest, ModelId, ProviderKind,
    SamplingParams,
};
use crate::process_ledger::RetainedLedgerBatcher;

use crate::terminal::redaction::{PatternRedactor, SecretRedactor};
use futures::StreamExt;

use super::error::SwarmError;
use super::ids::{ByokCloudProvider, ModelInstanceId, SpawnRequest};
use super::model_lane::{
    dexterity_spawn_model_session_id, DexterityLaunchAdapterKind, DexterityLaunchAdapterRequest,
    DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneDiagnosticTier,
    ModelLaneDiagnosticTierState, ModelLaneError, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRecord, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneRunRecord, ModelLaneStatus, ModelLaneStore, ModelLaneTarget,
    NewModelLane, NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus,
    NewModelLaneMessage, RuntimeBinding,
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
    /// Runtime-owned subagent participant lane with no OS process spawned.
    Subagent,
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
    /// For `Cli`: the exact official CLI provider row (`claude_code` | `codex`).
    #[serde(default)]
    pub cli_provider: Option<String>,
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
    /// Number of typed [`NewModelLaneMessage`] rows captured from the launched
    /// runtime's REAL stdout during this turn (F1/F2 live loop). `0` for a lane
    /// whose runtime produced no structured activity or when the coordinator has
    /// no `ModelLaneStore` to persist into.
    #[serde(default)]
    pub captured_message_count: usize,
}

/// Max tokens the operator-chat turn requests from the launched runtime. The CLI
/// bridge lane ignores this (the subprocess runs to completion and the whole
/// stdout is drained), but the local/candle lanes honor it; a generous cap keeps
/// a real turn from being truncated mid-answer.
const OPERATOR_CHAT_MAX_TOKENS: u32 = 4096;

/// Map the operator-selected model id to the CLI dialect its stdout speaks, so
/// [`ModelLaneCaptureRecorder::capture_cli_stream`] parses the launched runtime's
/// real output with the correct parser (claude vs codex vs gemini vs generic).
pub fn cli_kind_for_model(model_id: &str) -> CliKind {
    let m = model_id.to_ascii_lowercase();
    if m.contains("claude") {
        CliKind::ClaudeCode
    } else if m.contains("codex") || m.contains("gpt") {
        CliKind::CodexCli
    } else if m.contains("gemini") {
        CliKind::GeminiCli
    } else {
        CliKind::Other
    }
}

/// Prefer the explicit CLI provider id the picker sent; fall back to the model
/// name only for legacy callers.
pub fn cli_kind_for_selection(selection: &OperatorChatSelection) -> CliKind {
    match selection.cli_provider.as_deref() {
        Some("claude_code" | "claude-code" | "claude") => CliKind::ClaudeCode,
        Some("codex" | "codex_cli" | "codex-cli") => CliKind::CodexCli,
        Some("gemini" | "gemini_cli" | "gemini-cli") => CliKind::GeminiCli,
        Some(_) | None => cli_kind_for_model(&selection.model_id),
    }
}

/// One pane-friendly transcript row projected from a captured
/// [`ModelLaneMessageRecord`] (F8 transcript render).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatTranscriptRow {
    /// `operator | thinking | tool_call | tool_result | text | other`.
    pub role: String,
    /// The captured turn text (thought / answer / tool summary).
    pub text: String,
    /// The typed lane message kind (`status | tool_request | tool_result | ...`).
    pub kind: String,
    pub message_id: String,
    pub ordered_index: u64,
}

/// Non-secret model inventory for the picker: local models (MT-014) + cloud
/// access rows (MT-015). Serializable so the enumeration route returns it verbatim.
#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatModelInventory {
    pub local: Vec<OperatorChatModelRow>,
    pub cloud_byok: Vec<OperatorChatCloudRow>,
    pub cloud_cli_bridge: Vec<OperatorChatCloudRow>,
    pub subagents: Vec<OperatorChatSubagentRow>,
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
    pub model_id: String,
    pub label: String,
    /// `"configured"` | `"unavailable"` (fail-closed when a provider is absent).
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatSubagentRow {
    pub role: String,
    pub model_id: String,
    pub label: String,
    /// `"available"` | `"unavailable"` (fail-closed when the manager is absent).
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

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &Value) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    other => output.push(other),
                }
            }
            output.push('"');
        }
        Value::Array(values) => {
            output.push('[');
            for (idx, item) in values.iter().enumerate() {
                if idx > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (idx, key) in keys.into_iter().enumerate() {
                if idx > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &Value::String(key.clone()));
                output.push(':');
                write_canonical_json(output, &map[key]);
            }
            output.push('}');
        }
    }
}

fn build_message_payload_binding(
    message: &NewModelLaneMessage,
    payload_json: Value,
) -> Result<NewModelLaneContextBundleArtifactBinding, OperatorChatError> {
    let work_packet_id = message.work_packet_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "message {} is missing work_packet_id required for payload artifact binding",
            message.message_id
        ))
    })?;
    let micro_task_id = message.micro_task_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "message {} is missing micro_task_id required for payload artifact binding",
            message.message_id
        ))
    })?;
    let task_board_id = message.task_board_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "message {} is missing task_board_id required for payload artifact binding",
            message.message_id
        ))
    })?;
    Ok(NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", message.message_id),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".to_string(),
        artifact_manifest_ref: format!(
            "artifact-store://operator-chat/{}/{}.json",
            message.run_id, message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json,
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id,
        micro_task_id,
        task_board_id,
        owner_session: message.owner_session.clone(),
        idempotency_key: format!(
            "operator-chat-artifact::{}::{}",
            message.run_id, message.message_id
        ),
        created_at_utc: Utc::now().to_rfc3339(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for operator-chat payload",
            "surface": OPERATOR_CHAT_SURFACE_ID,
            "message_id": &message.message_id,
            "payload_ref": &message.payload_ref,
        }),
    })
}

#[derive(Debug, Clone)]
struct CloudProjectionArtifactBindingPlan {
    run_id: String,
    trace_id: String,
    lane_id: String,
    event_ledger_stream_id: String,
    work_packet_id: String,
    micro_task_id: String,
    task_board_id: String,
    owner_session: String,
    cloud_input_artifact_ref: String,
    cloud_payload_artifact_ref: String,
    cloud_input_payload: Value,
    projected_payload: Value,
}

fn cloud_input_artifact_ref(run_id: &str) -> String {
    format!("artifact-store://operator-chat/{run_id}/cloud-input.json")
}

fn cloud_projection_payload_artifact_ref(run_id: &str) -> String {
    format!("artifact-store://operator-chat/{run_id}/cloud-projection-payload.json")
}

fn cloud_provider_kind_for_request(
    request: &SpawnRequest,
) -> Result<&'static str, OperatorChatError> {
    match request.byok_cloud_provider {
        Some(ByokCloudProvider::Anthropic) => Ok("anthropic"),
        Some(ByokCloudProvider::OpenAi) => Ok("openai"),
        None => Err(OperatorChatError::Invalid(
            "operator-chat cloud launch requires byok_cloud_provider".into(),
        )),
    }
}

fn cloud_requested_model_id(request: &SpawnRequest, contract: &DexterityLaunchContract) -> String {
    contract
        .candidate_model_ids
        .first()
        .cloned()
        .unwrap_or_else(|| {
            request
                .cloud_model_name
                .as_deref()
                .map(|model| format!("model://dexterity/byok_cloud/{model}"))
                .unwrap_or_else(|| request.instance_id.model_id.to_string())
        })
}

fn build_cloud_projected_payload(
    selection: &OperatorChatSelection,
    contract: &DexterityLaunchContract,
    provider_kind: &str,
    requested_model_id: &str,
    model_session_id: &str,
) -> Value {
    json!({
        "schema_id": "hsk.operator_chat_cloud_projection@1",
        "surface": OPERATOR_CHAT_SURFACE_ID,
        "run_id": &contract.run_id,
        "lane_id": &contract.lane_id,
        "trace_id": &contract.trace_id,
        "model_session_id": model_session_id,
        "provider_kind": provider_kind,
        "requested_model_id": requested_model_id,
        "prompt_sha256": sha256_hex(selection.prompt.as_bytes()),
        "working_dir": &selection.working_dir,
        "worktree_id": &selection.worktree_id,
        "context_bundle_id": &contract.context_bundle_id,
    })
}

fn build_cloud_input_payload(
    selection: &OperatorChatSelection,
    contract: &DexterityLaunchContract,
    provider_kind: &str,
    requested_model_id: &str,
    model_session_id: &str,
) -> Value {
    json!({
        "schema_id": "hsk.operator_chat_cloud_input@1",
        "surface": OPERATOR_CHAT_SURFACE_ID,
        "run_id": &contract.run_id,
        "lane_id": &contract.lane_id,
        "trace_id": &contract.trace_id,
        "model_session_id": model_session_id,
        "provider_kind": provider_kind,
        "requested_model_id": requested_model_id,
        "prompt_sha256": sha256_hex(selection.prompt.as_bytes()),
        "working_dir": &selection.working_dir,
        "worktree_id": &selection.worktree_id,
        "context_bundle_id": &contract.context_bundle_id,
        "projection_plan_ref": &contract.projection_plan_ref,
        "consent_receipt_ref": &contract.consent_receipt_ref,
        "payload_artifact_ref": cloud_projection_payload_artifact_ref(&contract.run_id),
    })
}

fn build_cloud_projection_artifact_binding_plan(
    selection: &OperatorChatSelection,
    request: &SpawnRequest,
) -> Result<Option<CloudProjectionArtifactBindingPlan>, OperatorChatError> {
    if request.provider != Some(ProviderKind::ByokCloud) {
        return Ok(None);
    }
    let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
        OperatorChatError::Invalid(
            "operator-chat cloud launch requires a Dexterity launch contract".into(),
        )
    })?;
    let provider_kind = cloud_provider_kind_for_request(request)?;
    let requested_model_id = cloud_requested_model_id(request, contract);
    let model_session_id = dexterity_spawn_model_session_id(request);
    Ok(Some(CloudProjectionArtifactBindingPlan {
        run_id: contract.run_id.clone(),
        trace_id: contract.trace_id.clone(),
        lane_id: contract.lane_id.clone(),
        event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
        work_packet_id: selection.work_packet().to_string(),
        micro_task_id: selection.micro_task().to_string(),
        task_board_id: contract.task_board_id.clone(),
        owner_session: selection.owner_session.clone(),
        cloud_input_artifact_ref: cloud_input_artifact_ref(&contract.run_id),
        cloud_payload_artifact_ref: cloud_projection_payload_artifact_ref(&contract.run_id),
        cloud_input_payload: build_cloud_input_payload(
            selection,
            contract,
            provider_kind,
            &requested_model_id,
            &model_session_id,
        ),
        projected_payload: build_cloud_projected_payload(
            selection,
            contract,
            provider_kind,
            &requested_model_id,
            &model_session_id,
        ),
    }))
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

    let payload_sha256 = sha256_hex(&canonical_json_bytes(&plan.payload));

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

    /// Record ONE captured activity. Persist the authoritative typed
    /// ModelLaneMessage/payload binding first, then emit the Flight Recorder
    /// `FR-EVT-AGENT-*` business event. A terminal lane denial therefore cannot
    /// leave a misleading Flight Recorder activity with no durable capture.
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
        // Build both representations before either write, but persist ModelLane
        // authority first so cancellation/terminal guards fail closed without a
        // phantom activity event.
        let event = agent_activity_event(
            model_id,
            request_id,
            ordered_index,
            Some(lane.model_session_id.as_str()),
            OPERATOR_CHAT_CLI_ADAPTER,
            activity,
            self.redactor.as_ref(),
        );
        let plan = plan_from_activity(activity);
        let message = build_captured_message(run, lane, &plan, ordered_index, "model")?;
        let binding = build_message_payload_binding(&message, plan.payload)?;
        let record = self
            .store
            .record_message_with_payload_binding(message, binding)
            .await?;
        self.recorder.record_event(event).await?;
        Ok(record)
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
    ) -> Result<
        Vec<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord>,
        OperatorChatError,
    >
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
        let activity = AgentActivity::Text {
            text: prompt.to_string(),
        };
        let event = agent_activity_event(
            model_id_for_operator_prompt_event(operator_run, operator_lane),
            uuid_or_new(operator_run.trace_id.as_str()),
            0,
            Some(operator_lane.model_session_id.as_str()),
            OPERATOR_CHAT_CLI_ADAPTER,
            &activity,
            self.redactor.as_ref(),
        );
        let message = build_captured_message(operator_run, operator_lane, &plan, 0, "operator")?;
        let binding = build_message_payload_binding(&message, plan.payload)?;
        let record = self
            .store
            .record_message_with_payload_binding(message, binding)
            .await?;
        self.recorder.record_event(event).await?;
        Ok(record)
    }
}

fn model_id_for_operator_prompt_event(
    operator_run: &ModelLaneRunRecord,
    operator_lane: &ModelLaneRecord,
) -> ModelId {
    operator_lane
        .model_id
        .as_deref()
        .or(operator_run.selected_model_id.as_deref())
        .and_then(|value| Uuid::parse_str(value).ok())
        .map(ModelId::from)
        .unwrap_or_else(ModelId::new_v7)
}

fn uuid_or_new(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::now_v7())
}

async fn record_hbr_int_009_tiers(
    store: &ModelLaneStore,
    run: &ModelLaneRunRecord,
) -> Result<(), OperatorChatError> {
    let work_packet_id = run.work_packet_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing work_packet_id required for HBR-INT-009 tier status",
            run.run_id
        ))
    })?;
    let micro_task_id = run.micro_task_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing micro_task_id required for HBR-INT-009 tier status",
            run.run_id
        ))
    })?;
    let task_board_id = run.task_board_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing task_board_id required for HBR-INT-009 tier status",
            run.run_id
        ))
    })?;

    let rows = [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "operator-chat launch/capture emits Flight Recorder and EventLedger rows",
            None,
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "internal diagnostics inspection is deferred until the dedicated native diagnostics surface is wired",
            Some("WP-KERNEL-012"),
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "Palmistry watcher integration is deferred until the watcher work packet is active",
            Some("WP-KERNEL-016"),
        ),
    ];

    for (tier, state, reason, follow_up_ref) in rows {
        let tier_label = tier.as_str();
        store
            .record_diagnostic_tier_status(NewModelLaneDiagnosticTierStatus {
                diagnostic_status_id: format!("diag-{}-{}", run.run_id, tier_label),
                behavior_id: "HBR-INT-009".to_string(),
                run_id: run.run_id.clone(),
                tier,
                state,
                reason: reason.to_string(),
                evidence_ref: format!("kernel-event-ledger://{}", run.event_ledger_event_id),
                follow_up_ref: follow_up_ref.map(str::to_string),
                event_ledger_stream_id: run.event_ledger_stream_id.clone(),
                work_packet_id: work_packet_id.clone(),
                micro_task_id: micro_task_id.clone(),
                task_board_id: task_board_id.clone(),
                owner_session: run.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-hbr-int-009::{}::{}",
                    run.run_id, tier_label
                ),
                diagnostic_payload: json!({
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "behavior_id": "HBR-INT-009",
                    "tier": tier_label,
                    "state": state.as_str(),
                }),
            })
            .await?;
    }
    Ok(())
}

fn operator_lane_for_run(
    run: &ModelLaneRunRecord,
    selection: &OperatorChatSelection,
) -> Result<NewModelLane, OperatorChatError> {
    let work_packet_id = run.work_packet_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing work_packet_id required for operator prompt lane",
            run.run_id
        ))
    })?;
    let micro_task_id = run.micro_task_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing micro_task_id required for operator prompt lane",
            run.run_id
        ))
    })?;
    let task_board_id = run.task_board_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing task_board_id required for operator prompt lane",
            run.run_id
        ))
    })?;
    let lane_id = format!("dexterity-lane-human-operator-{}", run.run_id);
    let session_id = format!("operator-session-{}", run.run_id);
    let model_session_id = format!("operator-model-session-{}", run.run_id);
    let locus = ModelLaneLocusBinding {
        work_packet_id: work_packet_id.clone(),
        micro_task_id: micro_task_id.clone(),
        task_board_id: Some(task_board_id.clone()),
        coordinator_session_id: run.coordinator_session_id.clone(),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        owner_session: selection.owner_session.clone(),
        locus_binding_ref: format!("locus://operator-chat/{}/{lane_id}", run.run_id),
    };
    Ok(NewModelLane {
        lane_id: lane_id.clone(),
        run_id: run.run_id.clone(),
        trace_id: run.trace_id.clone(),
        lane_span_id: format!("span-{lane_id}-lane"),
        event_ledger_stream_id: run.event_ledger_stream_id.clone(),
        kind: ModelLaneKind::HumanOperator,
        role: "operator".to_string(),
        backend: "human".to_string(),
        model_id: None,
        session_id,
        model_session_id,
        adapter_id: "operator_chat_human".to_string(),
        runtime_binding: RuntimeBinding::Human,
        launch_authority: LaunchAuthority::Operator,
        provider_kind: ModelLaneProviderKind::Human,
        capability_token_ids: vec!["capability://operator-chat/human-prompt".to_string()],
        effective_capability_snapshot_ref: Some(format!(
            "capability-snapshot://operator-chat/{lane_id}/human"
        )),
        capability_negotiation_ref: Some(format!(
            "capability-negotiation://operator-chat/{}/human",
            run.run_id
        )),
        provider_feature_profile_ref: Some("provider-feature-profile://human-operator".to_string()),
        requested_execution_policy_ref: Some("execution-policy://operator-chat/human".to_string()),
        effective_execution_policy_ref: Some("execution-policy://operator-chat/human".to_string()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!(
            "toolgate://operator-chat/{}/human-prompt",
            run.run_id
        )],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some(Utc::now().to_rfc3339()),
        lease_expires_at_utc: None,
        reclaim_after_utc: None,
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://operator-chat/{}/human", run.run_id)),
        reclaim_policy_ref: Some("reclaim-policy://operator-chat/human".to_string()),
        terminal_status_mapping_ref: Some("terminal-status://operator-chat/human".to_string()),
        process_ownership_ref: None,
        no_os_process_reason_ref: Some(format!("no-os://operator-chat/{}/human", run.run_id)),
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://operator-chat/{}/human", run.run_id)),
        last_runtime_status_ref: Some(format!(
            "runtime-status://operator-chat/{}/human-ready",
            run.run_id
        )),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://operator-chat-launch#human-operator".to_string()),
        work_packet_id: Some(work_packet_id),
        micro_task_id: Some(micro_task_id),
        task_board_id: Some(task_board_id),
        owner_session: selection.owner_session.clone(),
        locus_binding: Some(locus),
    })
}

fn subagent_launch_request(selection: &OperatorChatSelection) -> DexterityLaunchAdapterRequest {
    let run_uuid = Uuid::now_v7();
    let lane_uuid = Uuid::now_v7();
    let run_id = format!("dexterity-run-subagent-{run_uuid}");
    let lane_id = format!("dexterity-lane-subagent-{lane_uuid}");
    let wp = selection.work_packet().to_string();
    let mt = selection.micro_task().to_string();
    let task_board_id = format!("task-board://operator-chat/{wp}");
    let session_id = format!("subagent-session-{lane_uuid}");
    let model_session_id = format!("subagent-model-session-{lane_uuid}");
    let candidate = selection.model_id.clone();
    DexterityLaunchAdapterRequest {
        adapter_kind: DexterityLaunchAdapterKind::Subagent,
        run_id: run_id.clone(),
        lane_id: lane_id.clone(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}-run"),
        lane_span_id: format!("span-{lane_id}-lane"),
        coordinator_session_id: selection.parent_session_id.clone(),
        routing_policy: "dexterity_subagent".into(),
        context_bundle_id: format!(
            "context-bundle://operator-chat/{}",
            selection.parent_session_id
        ),
        event_ledger_stream_id: format!("event-ledger://operator-chat/{run_id}"),
        artifact_namespace: format!("artifact://operator-chat/{run_id}"),
        work_packet_id: Some(wp.clone()),
        micro_task_id: Some(mt.clone()),
        task_board_id: Some(task_board_id.clone()),
        owner_session: selection.owner_session.clone(),
        locus_binding_ref: format!("locus://operator-chat/{wp}/{mt}/{lane_id}"),
        role: "subagent_coder".into(),
        backend: None,
        adapter_id: None,
        model_id: Some(candidate.clone()),
        session_id,
        model_session_id,
        extra_capability_token_ids: vec![],
        requested_tool_capability_tokens: vec!["tool-capability://read-context".into()],
        effective_capability_snapshot_ref: None,
        capability_negotiation_ref: None,
        provider_feature_profile_ref: None,
        requested_execution_policy_ref: None,
        effective_execution_policy_ref: None,
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!("toolgate://operator-chat/{lane_id}/read-context")],
        status: Some(ModelLaneStatus::Ready),
        heartbeat_at_utc: Some(Utc::now().to_rfc3339()),
        lease_expires_at_utc: None,
        reclaim_after_utc: None,
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://operator-chat/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://operator-chat/subagent".into()),
        terminal_status_mapping_ref: Some("terminal-status://operator-chat/subagent".into()),
        process_ownership_ref: None,
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://operator-chat/{lane_id}")),
        last_runtime_status_ref: Some(format!("runtime-status://operator-chat/{lane_id}/ready")),
        last_recovery_event_ref: None,
        startup_failure_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        run_recovery_hint_ref: Some("usermanual://operator-chat-launch#subagent".into()),
        lane_recovery_hint_ref: Some("usermanual://operator-chat-launch#subagent".into()),
        memory_pack_ref: format!("memory-pack://operator-chat/{run_id}"),
        memory_pack_hash: sha256_hex(
            format!(
                "{}:{}:{}:{}",
                selection.owner_session,
                selection.parent_session_id,
                candidate,
                selection.working_dir
            )
            .as_bytes(),
        ),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: format!("budget://operator-chat/{run_id}"),
        selected_model_id: Some(candidate.clone()),
        candidate_model_ids: vec![candidate],
        procedural_review_status: "operator_chat_subagent_registry_normalized".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn default_byok_model_id(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "claude-sonnet-4",
        "openai" | "open_ai" => "gpt-4o",
        _ => "cloud-model",
    }
}

fn default_cli_model_id(provider: &str) -> &'static str {
    match provider {
        "claude_code" | "claude-code" | "anthropic" => "claude-sonnet-4",
        "codex_cli" | "codex-cli" | "openai" => "gpt-5-codex",
        "gemini_cli" | "gemini-cli" | "google" => "gemini-cli",
        _ => "official-cli-model",
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
    _process_ledger_runtime: Option<RetainedLedgerBatcher>,
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
            _process_ledger_runtime: None,
        }
    }

    pub fn new_with_process_ledger_runtime(
        coordinator: Arc<SwarmCoordinator>,
        catalog: Arc<ModelCatalog>,
        recorder: Arc<dyn FlightRecorder>,
        process_ledger_runtime: RetainedLedgerBatcher,
    ) -> Self {
        Self {
            coordinator,
            catalog,
            recorder,
            instance_counter: Arc::new(AtomicU32::new(1)),
            _process_ledger_runtime: Some(process_ledger_runtime),
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
                model_id: default_byok_model_id(row.provider).to_string(),
                label: row.label.to_string(),
                status: provider_access_status_label(row.status),
            })
            .collect();
        let cloud_cli_bridge = cloud
            .cli_bridge
            .into_iter()
            .map(|row| OperatorChatCloudRow {
                provider: row.provider.to_string(),
                model_id: default_cli_model_id(row.provider).to_string(),
                label: row.label.to_string(),
                status: provider_access_status_label(ProviderAccessStatus::Unavailable),
            })
            .collect();
        let subagents = vec![OperatorChatSubagentRow {
            role: "subagent_coder".to_string(),
            model_id: "subagent://operator-chat/coder".to_string(),
            label: "Subagent Manager / Coder".to_string(),
            status: "available".to_string(),
        }];
        OperatorChatModelInventory {
            local,
            cloud_byok,
            cloud_cli_bridge,
            subagents,
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
        build_spawn_request_with_catalog(
            selection,
            self.next_instance(),
            Some(self.catalog.as_ref()),
        )
    }

    fn next_instance(&self) -> u32 {
        self.instance_counter.fetch_add(1, Ordering::SeqCst)
    }

    async fn attach_cloud_launch_authority(
        &self,
        selection: &OperatorChatSelection,
        request: &mut SpawnRequest,
    ) -> Result<(), OperatorChatError> {
        if request.provider != Some(ProviderKind::ByokCloud) {
            return Ok(());
        }
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat cloud launch requires a ModelLaneStore for ProjectionPlan/ConsentReceipt authority"
                    .into(),
            )
        })?;
        let mut contract = request.dexterity_launch.clone().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat cloud launch requires a Dexterity launch contract".into(),
            )
        })?;
        let provider_kind = cloud_provider_kind_for_request(request)?;
        let requested_model_id = cloud_requested_model_id(request, &contract);
        let model_session_id = dexterity_spawn_model_session_id(request);
        let now = Utc::now();
        let projection_plan_id = format!(
            "cloud-projection-plan://{}/{}",
            contract.run_id, contract.lane_id
        );
        let consent_receipt_id = format!(
            "cloud-consent-receipt://{}/{}",
            contract.run_id, contract.lane_id
        );
        let cloud_input_artifact_ref = cloud_input_artifact_ref(&contract.run_id);
        let cloud_payload_artifact_ref = cloud_projection_payload_artifact_ref(&contract.run_id);
        let projected_payload = build_cloud_projected_payload(
            selection,
            &contract,
            provider_kind,
            &requested_model_id,
            &model_session_id,
        );
        let payload_sha256 = sha256_hex(&canonical_json_bytes(&projected_payload));
        let scope_basis = json!({
            "run_id": &contract.run_id,
            "lane_id": &contract.lane_id,
            "model_session_id": &model_session_id,
            "provider_kind": provider_kind,
            "requested_model_id": &requested_model_id,
            "projection_plan_id": &projection_plan_id,
        });
        let scope_hash = sha256_hex(&canonical_json_bytes(&scope_basis));
        let fan_out_targets = vec![format!("provider://{provider_kind}/byok")];
        let stored_plan = store
            .record_cloud_projection_plan(NewModelLaneCloudProjectionPlan {
                projection_plan_id: projection_plan_id.clone(),
                run_id: contract.run_id.clone(),
                trace_id: contract.trace_id.clone(),
                lane_id: contract.lane_id.clone(),
                model_session_id: model_session_id.clone(),
                provider_kind: provider_kind.to_string(),
                requested_model_id: requested_model_id.clone(),
                scope_hash: scope_hash.clone(),
                source_artifact_refs: vec![
                    contract.context_bundle_id.clone(),
                    cloud_input_artifact_ref,
                ],
                payload_artifact_ref: cloud_payload_artifact_ref,
                payload_sha256,
                redaction_policy_ref: "redaction-policy://operator-chat/cloud-byok".to_string(),
                redaction_summary:
                    "operator-chat BYOK launch exports the selected prompt and redacted context only"
                        .to_string(),
                retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
                export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
                provider_profile_ref: format!("provider-profile://operator-chat/{provider_kind}"),
                fan_out_targets: fan_out_targets.clone(),
                consent_scope: ModelLaneCloudConsentScope::SingleLane,
                status: ModelLaneCloudProjectionPlanStatus::Active,
                event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
                work_packet_id: selection.work_packet().to_string(),
                micro_task_id: selection.micro_task().to_string(),
                task_board_id: contract.task_board_id.clone(),
                owner_session: selection.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-cloud-projection::{}::{}",
                    contract.run_id, contract.lane_id
                ),
                created_at_utc: now.to_rfc3339(),
                user_manual_behavior_ref:
                    "usermanual://model-lane-cloud-projection-consent#launch".to_string(),
                diagnostic_payload: json!({
                    "flight_recorder": "EventLedger",
                    "internal_diagnostics": "deferred: operator-chat cloud authority is visible through ModelLane diagnostics",
                    "palmistry": "deferred: external watcher links by run_id/lane_id when available",
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "payload": projected_payload,
                }),
            })
            .await?;
        let stored_receipt = store
            .record_cloud_consent_receipt(NewModelLaneCloudConsentReceipt {
                consent_receipt_id: consent_receipt_id.clone(),
                projection_plan_id: stored_plan.projection_plan_id.clone(),
                projection_plan_hash: stored_plan.projection_plan_hash.clone(),
                run_id: contract.run_id.clone(),
                trace_id: contract.trace_id.clone(),
                lane_id: contract.lane_id.clone(),
                model_session_id,
                provider_kind: provider_kind.to_string(),
                requested_model_id,
                scope_hash,
                consent_scope: ModelLaneCloudConsentScope::SingleLane,
                retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
                export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
                fan_out_targets,
                approved: true,
                approved_by_ref: format!("operator://{}/cloud-selection", selection.owner_session),
                approved_at_utc: now.to_rfc3339(),
                valid_from_utc: (now - Duration::minutes(5)).to_rfc3339(),
                valid_until_utc: (now + Duration::days(365)).to_rfc3339(),
                revoked_at_utc: None,
                revocation_ref: None,
                status: ModelLaneCloudConsentReceiptStatus::Approved,
                event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
                work_packet_id: selection.work_packet().to_string(),
                micro_task_id: selection.micro_task().to_string(),
                task_board_id: contract.task_board_id.clone(),
                owner_session: selection.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-cloud-consent::{}::{}",
                    contract.run_id, contract.lane_id
                ),
                created_at_utc: now.to_rfc3339(),
                user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                    .to_string(),
                diagnostic_payload: json!({
                    "flight_recorder": "EventLedger",
                    "provider_call_attempted": false,
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "selection_ref": format!("operator-chat-selection://{}", contract.run_id),
                }),
            })
            .await?;
        contract.projection_plan_ref = Some(stored_plan.projection_plan_id.clone());
        contract.consent_receipt_ref = Some(stored_receipt.consent_receipt_id.clone());
        request.dexterity_launch = Some(contract);
        Ok(())
    }

    async fn record_cloud_projection_artifact_bindings(
        &self,
        plan: CloudProjectionArtifactBindingPlan,
    ) -> Result<(), OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat cloud launch requires a ModelLaneStore for ProjectionPlan artifact bindings"
                    .into(),
            )
        })?;
        let now = Utc::now().to_rfc3339();
        let input_sha256 = sha256_hex(&canonical_json_bytes(&plan.cloud_input_payload));
        let payload_sha256 = sha256_hex(&canonical_json_bytes(&plan.projected_payload));

        store
            .record_context_bundle_artifact_binding(NewModelLaneContextBundleArtifactBinding {
                artifact_binding_id: format!("artifact-binding-{}-cloud-input", plan.lane_id),
                run_id: plan.run_id.clone(),
                trace_id: plan.trace_id.clone(),
                artifact_ref: plan.cloud_input_artifact_ref.clone(),
                artifact_sha256: input_sha256.clone(),
                content_hash: input_sha256,
                artifact_kind: "operator_chat_cloud_input".to_string(),
                artifact_manifest_ref: format!(
                    "artifact-store://operator-chat/{}/cloud-input.manifest.json",
                    plan.run_id
                ),
                artifact_payload_ref: plan.cloud_input_artifact_ref.clone(),
                payload_json: plan.cloud_input_payload,
                event_ledger_stream_id: plan.event_ledger_stream_id.clone(),
                work_packet_id: plan.work_packet_id.clone(),
                micro_task_id: plan.micro_task_id.clone(),
                task_board_id: plan.task_board_id.clone(),
                owner_session: plan.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-cloud-artifact::{}::{}::input",
                    plan.run_id, plan.lane_id
                ),
                created_at_utc: now.clone(),
                diagnostic_payload: json!({
                    "flight_recorder": "ArtifactStore/EventLedger binding for operator-chat cloud input",
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "lane_id": &plan.lane_id,
                    "artifact_ref": &plan.cloud_input_artifact_ref,
                }),
            })
            .await?;

        store
            .record_context_bundle_artifact_binding(NewModelLaneContextBundleArtifactBinding {
                artifact_binding_id: format!(
                    "artifact-binding-{}-cloud-projection-payload",
                    plan.lane_id
                ),
                run_id: plan.run_id.clone(),
                trace_id: plan.trace_id.clone(),
                artifact_ref: plan.cloud_payload_artifact_ref.clone(),
                artifact_sha256: payload_sha256.clone(),
                content_hash: payload_sha256,
                artifact_kind: "operator_chat_cloud_projection_payload".to_string(),
                artifact_manifest_ref: format!(
                    "artifact-store://operator-chat/{}/cloud-projection-payload.manifest.json",
                    plan.run_id
                ),
                artifact_payload_ref: plan.cloud_payload_artifact_ref.clone(),
                payload_json: plan.projected_payload,
                event_ledger_stream_id: plan.event_ledger_stream_id,
                work_packet_id: plan.work_packet_id,
                micro_task_id: plan.micro_task_id,
                task_board_id: plan.task_board_id,
                owner_session: plan.owner_session,
                idempotency_key: format!(
                    "operator-chat-cloud-artifact::{}::{}::payload",
                    plan.run_id, plan.lane_id
                ),
                created_at_utc: now,
                diagnostic_payload: json!({
                    "flight_recorder": "ArtifactStore/EventLedger binding for operator-chat cloud projected payload",
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "lane_id": &plan.lane_id,
                    "artifact_ref": &plan.cloud_payload_artifact_ref,
                }),
            })
            .await?;

        Ok(())
    }

    /// Launch a CLI/local/cloud lane for the operator selection through the
    /// sanctioned [`SwarmCoordinator::spawn_session`] authority, THEN drive the
    /// launched runtime with the operator's prompt and persist its REAL stdout as
    /// typed ModelLaneMessage rows (the F1/F2 live launch->capture loop). Fails
    /// closed if the coordinator has no `ModelLaneStore`.
    ///
    /// Assembled loop (no scaffolding): `spawn_session` persists the lane ->
    /// [`SwarmCoordinator::session_runtime`] hands back the exact runtime the
    /// coordinator just registered -> its `generate()` streams the launched CLI
    /// subprocess's live stdout -> that stdout is re-homed through
    /// [`ModelLaneCaptureRecorder::capture_cli_stream`] into ModelLaneMessage +
    /// Flight Recorder authority. The captured messages therefore ORIGINATE from
    /// the launched session's output, not a separately-authored vec.
    pub async fn launch(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        if selection.lane_kind == OperatorChatLaneKind::Subagent {
            return self.launch_subagent(selection).await;
        }
        let mut request = self.build_spawn_request(selection)?;
        self.attach_cloud_launch_authority(selection, &mut request)
            .await?;
        let cloud_artifact_binding_plan =
            build_cloud_projection_artifact_binding_plan(selection, &request)?;
        let contract = request.dexterity_launch.clone().ok_or_else(|| {
            OperatorChatError::Invalid("missing dexterity launch contract".into())
        })?;
        let instance_id = self.coordinator.spawn_session(request).await?;

        if let Some(plan) = cloud_artifact_binding_plan {
            if let Err(err) = self.record_cloud_projection_artifact_bindings(plan).await {
                let err_text = err.to_string();
                if let Err(cleanup_err) = self
                    .coordinator
                    .cancel_session(
                        instance_id,
                        format!("operator-chat cloud artifact binding failed: {err_text}"),
                    )
                    .await
                {
                    return Err(OperatorChatError::Invalid(format!(
                        "operator-chat cloud artifact binding failed ({err_text}); session cleanup failed: {cleanup_err}"
                    )));
                }
                return Err(err);
            }
        }

        // Drive the launched runtime and capture its real stdout as lane messages.
        let captured_result = self
            .drive_and_capture(selection, &contract.run_id, instance_id)
            .await;

        // Per-turn fresh-invocation model (contract F6): the turn is complete once
        // its stdout is captured, so complete the session to free its concurrency
        // permit + write the ledger STOP. Terminal ledger failure is part of the
        // runtime contract, so it is returned instead of hidden as a partial success.
        let captured_message_count = match captured_result {
            Ok(count) => {
                self.coordinator.complete_session(instance_id).await?;
                count
            }
            Err(err) => {
                let err_text = err.to_string();
                match self
                    .coordinator
                    .cancel_session(
                        instance_id,
                        format!("operator-chat capture failed: {err_text}"),
                    )
                    .await
                {
                    Ok(()) | Err(SwarmError::UnknownInstance(_)) => {}
                    Err(cleanup_err) => {
                        return Err(OperatorChatError::Invalid(format!(
                            "operator-chat capture failed ({err_text}); session cleanup failed: {cleanup_err}"
                        )));
                    }
                }
                return Err(err);
            }
        };

        Ok(OperatorChatLaunched {
            instance_id: instance_id.to_string(),
            run_id: contract.run_id,
            lane_id: contract.lane_id,
            trace_id: contract.trace_id,
            lane_kind: selection.lane_kind,
            captured_message_count,
        })
    }

    async fn launch_subagent(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        if selection.model_id.trim().is_empty() {
            return Err(OperatorChatError::Invalid(
                "subagent lane requires a selected subagent role".into(),
            ));
        }
        if selection.working_dir.trim().is_empty() {
            return Err(OperatorChatError::Invalid(
                "operator must select a working directory / worktree before launch".into(),
            ));
        }
        if selection.prompt.trim().is_empty() {
            return Err(OperatorChatError::Invalid(
                "operator prompt is empty".into(),
            ));
        }
        let request = subagent_launch_request(selection);
        let (run, _lane) = self
            .coordinator
            .launch_operator_subagent_model_lane(request)
            .await?;
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat subagent launch requires a ModelLaneStore".into(),
            )
        })?;
        let operator_lane = store
            .record_lane(operator_lane_for_run(&run, selection)?)
            .await?;
        record_hbr_int_009_tiers(&store, &run).await?;
        let capture = ModelLaneCaptureRecorder::new(store, self.recorder.clone());
        capture
            .record_operator_prompt(&run, &operator_lane, &selection.prompt)
            .await?;
        let run_id = run.run_id.clone();
        let lane_id = run
            .lane_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "dexterity-lane-subagent-missing".to_string());
        let trace_id = run.trace_id.clone();
        Ok(OperatorChatLaunched {
            instance_id: format!("no-os:{run_id}"),
            run_id,
            lane_id,
            trace_id,
            lane_kind: selection.lane_kind,
            captured_message_count: 0,
        })
    }

    /// Drive the just-spawned runtime with the operator prompt and persist its
    /// real stdout as ModelLaneMessage rows under the launched run. Returns the
    /// number of captured model messages. A coordinator without a `ModelLaneStore`,
    /// or a session that is not runnable, fails closed instead of returning a
    /// partial launch without capture authority.
    async fn drive_and_capture(
        &self,
        selection: &OperatorChatSelection,
        run_id: &str,
        instance_id: ModelInstanceId,
    ) -> Result<usize, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("operator-chat launch requires a ModelLaneStore".into())
        })?;
        // Prepare the durable capture surfaces before driving the runtime. This
        // makes a completed activity block durable before the next stream poll,
        // so coordinator cancellation preserves that prefix and blocks only
        // genuinely late output.
        let replay = store
            .replay_run(run_id)
            .await
            .map_err(OperatorChatError::from)?;
        let run = replay.run;
        let Some(lane) = replay.lanes.into_iter().next() else {
            return Err(OperatorChatError::Invalid(format!(
                "operator-chat run {run_id} has no persisted model lane"
            )));
        };
        let operator_lane = store
            .record_lane(operator_lane_for_run(&run, selection)?)
            .await?;
        record_hbr_int_009_tiers(&store, &run).await?;
        let capture = ModelLaneCaptureRecorder::new(store, self.recorder.clone());
        capture
            .record_operator_prompt(&run, &operator_lane, &selection.prompt)
            .await?;

        let model_id = self
            .coordinator
            .session_model_id(instance_id)
            .ok_or_else(|| {
                OperatorChatError::Invalid(format!(
                    "operator-chat launch {instance_id} has no runnable session model"
                ))
            })?;
        let capture_request_id = Uuid::new_v4();
        // `generate_session` replaces this placeholder token with the
        // coordinator-owned session token before the runtime receives it.
        // Callers therefore cannot bypass the coordinator's terminal ledger
        // transition by supplying their own cancellation token.
        let request = GenerateRequest {
            id: model_id,
            prompt: GenPrompt::new(selection.prompt.clone()),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: OPERATOR_CHAT_MAX_TOKENS,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        };
        let mut stream = self.coordinator.generate_session(instance_id, request)?;
        let mut stdout_buffer = String::new();
        let mut next_capture_index = 1;
        let mut captured_message_count = 0;
        let mut stream_error: Option<String> = None;
        let mut stream_cancelled = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(token) => {
                    if token.finish_reason == Some(FinishReason::Cancelled) {
                        stream_cancelled = true;
                    }
                    if token.finish_reason.is_none() && !token.text.is_empty() {
                        stdout_buffer.push_str(&token.text);
                        while let Some(newline_index) = stdout_buffer.find('\n') {
                            let line = stdout_buffer[..newline_index]
                                .trim_end_matches('\r')
                                .to_string();
                            stdout_buffer.drain(..=newline_index);
                            let records = capture
                                .capture_cli_stream(
                                    &run,
                                    &lane,
                                    model_id,
                                    capture_request_id,
                                    cli_kind_for_selection(selection),
                                    next_capture_index,
                                    std::iter::once(line.as_str()),
                                )
                                .await?;
                            next_capture_index += records.len() as u64;
                            captured_message_count += records.len();
                        }
                    }
                }
                Err(err) => {
                    stream_error = Some(format!(
                        "operator-chat runtime stream failed for {}: {err}",
                        instance_id
                    ));
                    break;
                }
            }
        }

        // A final non-newline-terminated activity is valid stdout and must be
        // captured on normal/error completion. On a cooperative cancellation it
        // is intentionally not promoted: only newline-complete activity blocks
        // observed before the terminal boundary are durable.
        if !stream_cancelled && !stdout_buffer.trim().is_empty() {
            let records = capture
                .capture_cli_stream(
                    &run,
                    &lane,
                    model_id,
                    capture_request_id,
                    cli_kind_for_selection(selection),
                    next_capture_index,
                    std::iter::once(stdout_buffer.as_str()),
                )
                .await?;
            captured_message_count += records.len();
        }
        if let Some(err) = stream_error {
            return Err(OperatorChatError::Invalid(err));
        }
        if stream_cancelled {
            return Err(OperatorChatError::Invalid(format!(
                "operator-chat runtime stream cancelled for {instance_id}"
            )));
        }
        Ok(captured_message_count)
    }

    /// Project the captured ModelLaneMessage rows for a run into pane-friendly
    /// transcript rows (F8 transcript render). Reads EventLedger authority through
    /// the coordinator's `ModelLaneStore`; fails closed when no store is wired.
    pub async fn fetch_transcript(
        &self,
        run_id: &str,
    ) -> Result<Vec<OperatorChatTranscriptRow>, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("operator-chat transcript requires a ModelLaneStore".into())
        })?;
        let replay = store
            .replay_run(run_id)
            .await
            .map_err(OperatorChatError::from)?;
        Ok(replay
            .messages
            .into_iter()
            .map(transcript_row_from_message)
            .collect())
    }
}

/// Project one captured message record into a transcript row: the `role` is the
/// `activity_kind` discriminator (`operator|thinking|tool_call|tool_result|text|
/// other`) and the `text` is the captured turn text (falling back to the summary).
fn transcript_row_from_message(
    msg: crate::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> OperatorChatTranscriptRow {
    let diag = &msg.diagnostic_payload;
    let role = diag
        .get("turn_role")
        .and_then(|v| v.as_str())
        .filter(|s| *s == "operator")
        .map(|s| s.to_string())
        .or_else(|| {
            diag.get("activity_kind")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "text".to_string());
    let capture = diag.get("capture");
    let text = capture
        .and_then(|c| c.get("text").or_else(|| c.get("raw")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| msg.summary.clone());
    let ordered_index = diag
        .get("ordered_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    OperatorChatTranscriptRow {
        role,
        text,
        kind: message_kind_label(&msg.kind).to_string(),
        message_id: msg.message_id.clone(),
        ordered_index,
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
    build_spawn_request_with_catalog(selection, instance, None)
}

fn build_spawn_request_with_catalog(
    selection: &OperatorChatSelection,
    instance: u32,
    catalog: Option<&ModelCatalog>,
) -> Result<SpawnRequest, OperatorChatError> {
    use crate::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;

    if selection.working_dir.trim().is_empty() {
        return Err(OperatorChatError::Invalid(
            "operator must select a working directory / worktree before launch".into(),
        ));
    }
    if selection.prompt.trim().is_empty() {
        return Err(OperatorChatError::Invalid(
            "operator prompt is empty".into(),
        ));
    }

    let local_entry = if selection.lane_kind == OperatorChatLaneKind::Local {
        let catalog = catalog.ok_or_else(|| {
            OperatorChatError::Invalid(
                "local operator-chat launch requires a ModelCatalog".to_string(),
            )
        })?;
        let entry = catalog.entry(&selection.model_id).ok_or_else(|| {
            OperatorChatError::Invalid(format!(
                "local model_id '{}' is not registered in the live ModelCatalog",
                selection.model_id
            ))
        })?;
        if !entry.ready {
            return Err(OperatorChatError::Invalid(format!(
                "local model_id '{}' is registered but not ready",
                selection.model_id
            )));
        }
        Some(entry)
    } else {
        None
    };
    let runtime_binding = match local_entry
        .as_ref()
        .map(|entry| entry.runtime_binding.as_str())
    {
        Some("candle") => RuntimeAdapterBinding::Candle,
        Some("llama_cpp") => RuntimeAdapterBinding::LlamaCpp,
        Some(other) => {
            return Err(OperatorChatError::Invalid(format!(
                "local model_id '{}' has unsupported runtime_binding '{}'",
                selection.model_id, other
            )));
        }
        None => RuntimeAdapterBinding::Candle,
    };

    let instance_id = ModelInstanceId::new(ModelId::new_v7(), instance);
    let mut request = SpawnRequest::new(
        instance_id,
        runtime_binding,
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
        OperatorChatLaneKind::Local => {
            let entry = local_entry.expect("local entry resolved above");
            request.with_local_artifact(entry.artifact_path, entry.artifact_sha256)
        }
        OperatorChatLaneKind::Cli => {
            let provider = selection
                .cli_provider
                .clone()
                .unwrap_or_else(|| cli_provider_for_model(&selection.model_id).to_string());
            request
                .with_cloud_provider(ProviderKind::OfficialCli, selection.model_id.clone())
                .with_official_cli_provider(provider)
        }
        OperatorChatLaneKind::Cloud => {
            let provider = match selection.cloud_provider.as_deref() {
                Some("anthropic") => ByokCloudProvider::Anthropic,
                Some("openai") => ByokCloudProvider::OpenAi,
                other => {
                    return Err(OperatorChatError::Invalid(format!(
                        "cloud lane requires cloud_provider anthropic|openai, got {other:?}"
                    )));
                }
            };
            request
                .with_cloud_provider(ProviderKind::ByokCloud, selection.model_id.clone())
                .with_byok_cloud_provider(provider)
        }
        OperatorChatLaneKind::Subagent => {
            return Err(OperatorChatError::Invalid(
                "subagent operator-chat launch uses the no-OS Dexterity path".into(),
            ));
        }
    };

    let contract =
        DexterityLaunchContract::from_spawn_request(&request).map_err(OperatorChatError::from)?;
    Ok(request.with_dexterity_launch(contract))
}

fn cli_provider_for_model(model_id: &str) -> &'static str {
    match cli_kind_for_model(model_id) {
        CliKind::ClaudeCode => "claude_code",
        CliKind::CodexCli => "codex",
        CliKind::GeminiCli | CliKind::Other => "codex",
    }
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
            again
                .args_template
                .iter()
                .filter(|a| *a == "stream-json")
                .count(),
            1
        );
    }

    #[test]
    fn build_spawn_request_cli_sets_provider_working_dir_and_contract() {
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Cli,
            model_id: "claude-sonnet-4".into(),
            cloud_provider: None,
            cli_provider: Some("claude_code".into()),
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
        assert_eq!(
            request.official_cli_provider.as_deref(),
            Some("claude_code")
        );
        assert!(request.dexterity_launch.is_some());
        assert_eq!(request.wp_id.as_deref(), Some("operator-chat-workspace"));
    }

    #[test]
    fn build_spawn_request_local_requires_catalog_resolution() {
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Local,
            model_id: Uuid::now_v7().to_string(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: None,
            prompt: "hi".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };
        let err = build_spawn_request(&selection, 1)
            .expect_err("local build without catalog must fail closed");
        assert!(
            err.to_string().contains("requires a ModelCatalog"),
            "local model selection must not silently launch a generic local runtime: {err}"
        );
    }

    #[test]
    fn build_spawn_request_local_resolves_selected_artifact_identity() {
        let model_id = ModelId::new_v7();
        let mut registry = crate::model_runtime::ModelRegistry::default();
        registry
            .register(crate::model_runtime::ModelRegistration {
                model_id,
                artifact_path: "D:/models/local-model.gguf".into(),
                sha256: [9u8; 32],
                runtime_binding: crate::model_runtime::registry::RuntimeBinding::LlamaCpp,
                declared_capabilities: crate::model_runtime::ModelCapabilities::default(),
                base_model_tag: crate::model_runtime::BaseModelTag::new("Local Model"),
                registered_at_utc: Utc::now(),
                registered_by: crate::model_runtime::OperatorId::new("operator-test"),
                provider: ProviderKind::Local,
            })
            .expect("register local model");
        registry.mark_loaded(model_id).expect("mark model loaded");
        let catalog = ModelCatalog::from_registry(Arc::new(registry));
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Local,
            model_id: model_id.to_string(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: Some("wt-local".into()),
            prompt: "hi".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };

        let request = build_spawn_request_with_catalog(&selection, 2, Some(catalog.as_ref()))
            .expect("local selection resolves through catalog");
        assert_eq!(request.provider, None);
        assert_eq!(
            request.runtime_binding,
            crate::model_runtime::registry::RuntimeBinding::LlamaCpp
        );
        assert_eq!(
            request.model_artifact_path.as_deref(),
            Some("D:/models/local-model.gguf")
        );
        assert_eq!(request.model_artifact_sha256, Some(hex::encode([9u8; 32])));
        assert_eq!(request.worktree_id.as_deref(), Some("wt-local"));
        assert!(request.dexterity_launch.is_some());
    }

    #[test]
    fn build_spawn_request_rejects_empty_working_dir() {
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Local,
            model_id: "m".into(),
            cloud_provider: None,
            cli_provider: None,
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
