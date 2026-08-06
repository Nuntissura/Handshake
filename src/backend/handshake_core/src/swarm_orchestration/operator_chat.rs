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
    parse_agent_activity_line, AgentActivity, AgentActivityKind, CliKind, CliOutputFormat,
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
    dexterity_spawn_model_session_id, CloudExportDelegation, DexterityLaunchAdapterKind,
    DexterityLaunchAdapterRequest, DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptRecord, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudConsentTargetBinding, ModelLaneCloudExportPosture,
    ModelLaneCloudProjectionPlanRecord, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneError, ModelLaneKind, ModelLaneLocusBinding, ModelLaneMessageKind,
    ModelLaneProviderKind, ModelLaneRecord, ModelLaneRecoveryState, ModelLaneRoutingMetadata,
    ModelLaneRunRecord, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus,
    NewModelLaneMessage, RuntimeBinding,
};
use super::resource_scope::AccountBoundAuthority;
use super::routing::ModelLaneRoutingAuthority;
use super::routing_execution::{
    ModelLaneRoutingDispatchBatch, ModelLaneRoutingExecutionContext,
    ModelLaneRoutingExecutionState, ModelLaneRoutingStageLaunch,
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

/// How long an auto-issued operator-chat cloud ConsentReceipt stays valid.
///
/// # Why this is 12 hours and not 365 days
///
/// This receipt is minted by the launch path itself, for exactly ONE lane in ONE
/// run, and it authorizes sending operator context to a third-party provider.
/// The previous value was `now + 365 days`, which is longer than the resource it
/// authorizes can possibly live: a lane cannot outlive its run, a run cannot
/// outlive the coordinator process, and neither survives a machine restart. A
/// capability that outlives its subject by three orders of magnitude is a
/// dangling capability — after the lane is gone the receipt is still an
/// approved, unrevoked, replayable authorization for that provider/scope pair,
/// and every idempotent re-launch on the same run/lane ids re-uses it instead of
/// asking again.
///
/// The genuinely correct binding is lifetime-scoped consent, and that primitive
/// exists (`ModelLaneCloudConsentScope::SingleRun`, migration
/// `0353_model_lane_run_scoped_consent`), but it is a *scope* mechanism, not an
/// *expiry* mechanism: it says which lanes one grant covers, not when the grant
/// dies. There is no run-end timestamp available at mint time to bind
/// `valid_until_utc` to, so the honest construction is a bounded window that is
/// long enough to cover a real working session (a cloud lane can legitimately
/// run for hours) and short enough that an abandoned receipt expires on its own
/// rather than waiting for someone to notice it. 12 hours is that window: longer
/// than any single operator-chat turn or session, shorter than a day, and it
/// cannot silently survive to the next working day.
///
/// Revocation stays the immediate kill switch; expiry is the backstop for the
/// case where nobody revokes because nobody remembers the receipt exists.
pub const OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS: i64 = 12;

/// Clock-skew grace applied to `valid_from_utc` so a receipt minted on one clock
/// is not rejected as "not yet valid" by a check on a slightly earlier clock.
pub const OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES: i64 = 5;

/// Recorded as `approved_by_ref` when the operator-chat launch path has no
/// authenticated account to bind the approval to.
///
/// It replaces `format!("operator://{}/cloud-selection", owner_session)`, which
/// claimed an operator approved the export when all it actually recorded was the
/// governance role label of the thing requesting the export. This value claims
/// nothing: it states that no authenticated account existed, and the typed
/// `approver` alongside it is [`AccountBoundAuthority::Unattributed`], which no
/// account-scoped gate will accept.
pub const OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF: &str =
    "unattributed://operator-chat/no-authenticated-account";

/// Stable reason stamped on an unattributed operator-chat cloud approval so
/// every such row is enumerable by an auditor.
pub const OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REASON: &str =
    "OPERATOR_CHAT_CLOUD_LAUNCH_WITHOUT_AUTHENTICATED_ACCOUNT";

/// Operator Chat request for one immutable run-scoped cloud-consent grant.
/// The receipt is derived from the stored plan so its hash, policies, and
/// enumerated targets cannot diverge at this product boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudConsentGrant {
    pub projection_plan: NewModelLaneCloudProjectionPlan,
    pub consent_receipt_id: String,
    pub approved_by_ref: String,
    pub approved_at_utc: String,
    pub valid_from_utc: String,
    pub valid_until_utc: String,
    pub consent_idempotency_key: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudLaunchRequest {
    pub grant: OperatorChatSingleRunCloudConsentGrant,
    pub selections: Vec<OperatorChatSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudLaunched {
    pub projection_plan_id: String,
    pub consent_receipt_id: String,
    pub instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudRevokeRequest {
    pub consent_receipt_id: String,
    pub revoked_by_ref: String,
    pub reason: String,
}

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

/// External operator-chat launch contract. The client supplies only the governed
/// owner id; trusted owner/parent lineage is resolved by the API from SessionRegistry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatLaunchRequest {
    pub lane_kind: OperatorChatLaneKind,
    pub model_id: String,
    #[serde(default)]
    pub cloud_provider: Option<String>,
    #[serde(default)]
    pub cli_provider: Option<String>,
    pub working_dir: String,
    #[serde(default)]
    pub worktree_id: Option<String>,
    pub prompt: String,
    pub owner_session_id: String,
    #[serde(default)]
    pub work_packet_id: Option<String>,
    #[serde(default)]
    pub micro_task_id: Option<String>,
}

impl OperatorChatLaunchRequest {
    pub fn into_governed_selection(
        self,
        owner_session: String,
        parent_session_id: String,
    ) -> OperatorChatSelection {
        OperatorChatSelection {
            lane_kind: self.lane_kind,
            model_id: self.model_id,
            cloud_provider: self.cloud_provider,
            cli_provider: self.cli_provider,
            working_dir: self.working_dir,
            worktree_id: self.worktree_id,
            prompt: self.prompt,
            owner_session,
            parent_session_id,
            work_packet_id: self.work_packet_id,
            micro_task_id: self.micro_task_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingStageRequest {
    pub stage_id: String,
    #[serde(default)]
    pub lane_id: Option<String>,
    #[serde(default)]
    pub selection: Option<OperatorChatSelection>,
    #[serde(default)]
    pub authority_lane_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingLifecycleRequest {
    pub execution_id: String,
    pub selecting_decision_id: String,
    pub authority: ModelLaneRoutingAuthority,
    pub context: ModelLaneRoutingExecutionContext,
    pub stages: Vec<OperatorChatRoutingStageRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingCancelRequest {
    pub execution_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingAuthorityRequest {
    pub execution_id: String,
    pub stage_id: String,
    pub message_id: String,
    pub routing_request: OperatorChatRoutingLifecycleRequest,
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
    pub inventory_source: &'static str,
    pub sessions: Vec<OperatorChatSessionRow>,
    pub local: Vec<OperatorChatModelRow>,
    pub cloud_byok: Vec<OperatorChatCloudRow>,
    pub cloud_cli_bridge: Vec<OperatorChatCloudRow>,
    pub subagents: Vec<OperatorChatSubagentRow>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatSessionRow {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub label: String,
    /// `available` only when owner and registered parent are Active and governed.
    pub status: String,
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

    // Write the COMPLETE run-level HBR-INT-009 triplet (one envelope per
    // ModelLaneRun, per the MT-011 run-level design). The launch service can
    // PROVE only its own EventLedger/Flight Recorder production, so it records
    // FlightRecorder=Wired. It cannot prove the native internal_diagnostics
    // producer or the authenticated Palmistry watcher from inside the launch
    // context: those tiers are produced and read back later through the
    // authenticated diagnostic observation/readback boundary
    // (`api/palmistry.rs`), which records them as Wired against the same run
    // with a later EventLedger seq. `diagnostic_tier_posture` selects the
    // latest state per tier, so those deferred_with_reason rows are cleanly
    // superseded (deferred -> wired) once the observation lands. Recording them
    // as deferred_with_reason (not Wired) is the honest launch-context state and
    // satisfies `validate_diagnostic_tier_posture`, which requires all three
    // tiers present, none Missing, and a non-null follow_up_ref on every
    // deferred tier.
    let rows: [(
        ModelLaneDiagnosticTier,
        ModelLaneDiagnosticTierState,
        &str,
        Option<String>,
    ); 3] = [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "operator-chat launch/capture emitted this run's EventLedger row",
            None,
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "native internal_diagnostics producer records this tier later through the authenticated diagnostic observation/readback boundary; deferred in the launch-service context which cannot prove the native producer",
            Some(format!(
                "internal-diagnostics://observation-readback/run/{}",
                run.run_id
            )),
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "authenticated Palmistry watcher records this tier later through the diagnostic observation/readback boundary; deferred in the launch-service context which cannot prove the out-of-process watcher",
            Some(format!(
                "palmistry://observation-readback/run/{}",
                run.run_id
            )),
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
                evidence_ref: format!("eventledger://kernel/{}", run.event_ledger_event_id),
                follow_up_ref,
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
    /// The live coordinator this service launches through.
    ///
    /// WP-1 MT-021 AC-3: the operator-facing swarm concurrency control must move
    /// the REAL model-session admission cap, and the coordinator singleton is
    /// only reachable from the API through this service. Exposed read-only so a
    /// route can call `set_max_concurrent`/`max_concurrent` without gaining any
    /// other coordinator authority.
    pub fn coordinator(&self) -> &Arc<SwarmCoordinator> {
        &self.coordinator
    }

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

    /// Persist a real Operator Chat `SingleRun` ProjectionPlan and its derived
    /// ConsentReceipt through ModelLane/EventLedger authority.
    pub async fn grant_single_run_cloud_consent(
        &self,
        request: OperatorChatSingleRunCloudConsentGrant,
    ) -> Result<
        (
            ModelLaneCloudProjectionPlanRecord,
            ModelLaneCloudConsentReceiptRecord,
        ),
        OperatorChatError,
    > {
        if request.projection_plan.consent_scope != ModelLaneCloudConsentScope::SingleRun {
            return Err(OperatorChatError::Invalid(
                "operator-chat SingleRun grant requires consent_scope=single_run".into(),
            ));
        }
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat SingleRun grant requires a ModelLaneStore".into(),
            )
        })?;

        // HBR-PRIV-005: identity is server-derived, never client-supplied. The
        // caller may propose a plan and a provenance label; it may not tell the
        // backend whose data is being exported or who approved it. Overriding
        // rather than validating is deliberate — there is no legitimate case
        // where a client knows a source scope the store does not.
        let approver = AccountBoundAuthority::from_access(store.access());
        let mut projection_plan = request.projection_plan;
        projection_plan.export_delegation.source_scope = approver.clone();
        projection_plan.export_delegation.authorization_receipt_ref =
            Some(request.consent_receipt_id.clone());
        let stored_plan = store.record_cloud_projection_plan(projection_plan).await?;
        let receipt = store
            .record_cloud_consent_receipt(NewModelLaneCloudConsentReceipt {
                consent_receipt_id: request.consent_receipt_id,
                projection_plan_id: stored_plan.projection_plan_id.clone(),
                projection_plan_hash: stored_plan.projection_plan_hash.clone(),
                run_id: stored_plan.run_id.clone(),
                trace_id: stored_plan.trace_id.clone(),
                lane_id: None,
                model_session_id: None,
                provider_kind: None,
                requested_model_id: None,
                scope_hash: stored_plan.scope_hash.clone(),
                consent_scope: ModelLaneCloudConsentScope::SingleRun,
                target_bindings: stored_plan.target_bindings.clone(),
                retention_policy: stored_plan.retention_policy.clone(),
                export_posture: stored_plan.export_posture.clone(),
                fan_out_targets: stored_plan.fan_out_targets.clone(),
                approved: true,
                approver,
                approved_by_ref: request.approved_by_ref,
                approved_at_utc: request.approved_at_utc,
                valid_from_utc: request.valid_from_utc,
                valid_until_utc: request.valid_until_utc,
                revoked_at_utc: None,
                revocation_ref: None,
                revocation_input_hash: None,
                status: ModelLaneCloudConsentReceiptStatus::Approved,
                event_ledger_stream_id: stored_plan.event_ledger_stream_id.clone(),
                work_packet_id: stored_plan.work_packet_id.clone(),
                micro_task_id: stored_plan.micro_task_id.clone(),
                task_board_id: stored_plan.task_board_id.clone(),
                owner_session: stored_plan.owner_session.clone(),
                idempotency_key: request.consent_idempotency_key,
                created_at_utc: stored_plan.created_at_utc.clone(),
                user_manual_behavior_ref: stored_plan.user_manual_behavior_ref.clone(),
                diagnostic_payload: request.diagnostic_payload,
            })
            .await?;
        Ok((stored_plan, receipt))
    }

    /// Revoke an Operator Chat cloud grant through the same atomic ModelLane
    /// cancellation path used by coordinator-managed lanes.
    pub async fn revoke_single_run_cloud_consent(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> Result<Vec<ModelLaneRecord>, OperatorChatError> {
        Ok(self
            .coordinator
            .revoke_cloud_consent_receipt(consent_receipt_id, revoked_by_ref, reason)
            .await?)
    }

    /// Grant and consume SingleRun cloud authority through the production
    /// coordinator batch boundary. All targets preflight before any factory call.
    pub async fn launch_single_run_cloud_consent(
        &self,
        mut request: OperatorChatSingleRunCloudLaunchRequest,
    ) -> Result<OperatorChatSingleRunCloudLaunched, OperatorChatError> {
        if request.selections.len() < 2 {
            return Err(OperatorChatError::Invalid(
                "operator-chat SingleRun launch requires at least two selections".into(),
            ));
        }
        let mut spawn_requests = Vec::with_capacity(request.selections.len());
        let mut target_bindings = Vec::with_capacity(request.selections.len());
        for selection in &request.selections {
            if selection.lane_kind != OperatorChatLaneKind::Cloud {
                return Err(OperatorChatError::Invalid(
                    "operator-chat SingleRun launch accepts cloud selections only".into(),
                ));
            }
            let mut spawn = self.build_spawn_request(selection)?;
            let provider_kind = cloud_provider_kind_for_request(&spawn)?;
            let model_session_id = dexterity_spawn_model_session_id(&spawn);
            let contract = spawn.dexterity_launch.as_mut().ok_or_else(|| {
                OperatorChatError::Invalid(
                    "operator-chat SingleRun launch requires Dexterity contracts".into(),
                )
            })?;
            contract.run_id = request.grant.projection_plan.run_id.clone();
            contract.trace_id = request.grant.projection_plan.trace_id.clone();
            contract.event_ledger_stream_id =
                request.grant.projection_plan.event_ledger_stream_id.clone();
            let requested_model_id = contract
                .candidate_model_ids
                .first()
                .cloned()
                .unwrap_or_else(|| selection.model_id.clone());
            target_bindings.push(ModelLaneCloudConsentTargetBinding {
                lane_id: contract.lane_id.clone(),
                model_session_id,
                provider_kind: provider_kind.to_string(),
                requested_model_id,
                capability_snapshot_ref: contract.effective_capability_snapshot_ref.clone(),
                provider_endpoint_ref: contract.adapter_id.clone(),
            });
            spawn_requests.push(spawn);
        }
        request.grant.projection_plan.consent_scope = ModelLaneCloudConsentScope::SingleRun;
        request.grant.projection_plan.lane_id = None;
        request.grant.projection_plan.model_session_id = None;
        request.grant.projection_plan.provider_kind = None;
        request.grant.projection_plan.requested_model_id = None;
        request.grant.projection_plan.target_bindings = target_bindings;
        request.grant.projection_plan.fan_out_targets = request
            .grant
            .projection_plan
            .target_bindings
            .iter()
            .map(|target| format!("provider-endpoint://{}", target.provider_endpoint_ref))
            .collect();
        // HBR-PRIV-007: the SingleRun audience is exactly the enumerated launch
        // targets. Deriving it here (rather than trusting the caller's list)
        // means a broadcast grant cannot name an endpoint that is not one of the
        // lanes actually being launched under it.
        request.grant.projection_plan.export_delegation.audience_refs =
            request.grant.projection_plan.fan_out_targets.clone();
        let (plan, receipt) = self.grant_single_run_cloud_consent(request.grant).await?;
        for spawn in &mut spawn_requests {
            let contract = spawn.dexterity_launch.as_mut().ok_or_else(|| {
                OperatorChatError::Invalid(
                    "operator-chat SingleRun launch requires Dexterity contracts".into(),
                )
            })?;
            contract.projection_plan_ref = Some(plan.projection_plan_id.clone());
            contract.consent_receipt_ref = Some(receipt.consent_receipt_id.clone());
        }
        let instances = match self
            .coordinator
            .spawn_cloud_consent_batch(spawn_requests)
            .await
        {
            Ok(instances) => instances,
            Err(error) => {
                let _ = self
                    .coordinator
                    .revoke_cloud_consent_receipt(
                        &receipt.consent_receipt_id,
                        "operator-chat://single-run/failed-launch",
                        "SingleRun batch launch failed closed",
                    )
                    .await;
                return Err(error.into());
            }
        };
        Ok(OperatorChatSingleRunCloudLaunched {
            projection_plan_id: plan.projection_plan_id.clone(),
            consent_receipt_id: receipt.consent_receipt_id.clone(),
            instance_ids: instances
                .into_iter()
                .map(|instance| instance.to_string())
                .collect(),
        })
    }

    async fn routing_launches(
        &self,
        request: &OperatorChatRoutingLifecycleRequest,
    ) -> Result<Vec<ModelLaneRoutingStageLaunch>, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("operator-chat routing requires a ModelLaneStore".into())
        })?;
        let replay = store.replay_run(&request.context.run_id).await?;
        let run = replay.run;
        if run.trace_id != request.context.trace_id
            || run.run_span_id != request.context.run_span_id
            || run.coordinator_session_id != request.context.coordinator_session_id
            || run.work_packet_id.as_deref() != Some(request.context.work_packet_id.as_str())
            || run.micro_task_id.as_deref() != request.context.micro_task_id.as_deref()
            || run.task_board_id.as_deref() != Some(request.context.task_board_id.as_str())
            || run.owner_session != request.context.owner_session
            || run
                .locus_binding
                .as_ref()
                .map(|binding| binding.locus_binding_ref.as_str())
                != Some(request.context.locus_ref.as_str())
        {
            return Err(OperatorChatError::Invalid(
                "operator-chat routing context differs from canonical ModelLaneRun".into(),
            ));
        }
        let mut launches = Vec::with_capacity(request.stages.len());
        for stage in &request.stages {
            let spawn = if let Some(selection) = stage.selection.as_ref() {
                let lane_id = stage
                    .lane_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        OperatorChatError::Invalid(format!(
                            "routing model stage {} requires lane_id",
                            stage.stage_id
                        ))
                    })?;
                let mut spawn = self.build_spawn_request(selection)?;
                spawn.parent_session_id = request.context.coordinator_session_id.clone();
                spawn.owner_role = request.context.owner_session.clone();
                spawn.owner_wp = Some(request.context.work_packet_id.clone());
                spawn.wp_id = Some(request.context.work_packet_id.clone());
                spawn.mt_id = request.context.micro_task_id.clone();
                let contract = spawn.dexterity_launch.as_mut().ok_or_else(|| {
                    OperatorChatError::Invalid(format!(
                        "routing stage {} requires Dexterity launch contract",
                        stage.stage_id
                    ))
                })?;
                contract.lane_id = lane_id.to_string();
                contract.lane_span_id = format!("span-{lane_id}-lane");
                contract.run_id = run.run_id.clone();
                contract.trace_id = run.trace_id.clone();
                contract.run_span_id = run.run_span_id.clone();
                contract.routing_policy = run.routing_policy.clone();
                contract.context_bundle_id = run.context_bundle_id.clone();
                contract.event_ledger_stream_id = run.event_ledger_stream_id.clone();
                contract.artifact_namespace = run.artifact_namespace.clone();
                contract.task_board_id = run.task_board_id.clone().ok_or_else(|| {
                    OperatorChatError::Invalid(
                        "canonical ModelLaneRun requires task_board_id".into(),
                    )
                })?;
                contract.locus_binding_ref = request.context.locus_ref.clone();
                contract.projection_plan_ref = run.projection_plan_ref.clone();
                contract.consent_receipt_ref = request.authority.cloud_consent_receipt_ref.clone();
                contract.memory_pack_ref = run.memory_pack_ref.clone();
                contract.memory_pack_hash = run.memory_pack_hash.clone();
                contract.determinism_mode = run.determinism_mode.clone();
                contract.budget_summary_ref = run.budget_summary_ref.clone();
                contract.candidate_model_ids = run.candidate_model_ids.clone();
                let selected_model_id = spawn.instance_id.model_id.to_string();
                contract
                    .candidate_model_ids
                    .retain(|value| value != &selected_model_id);
                contract.candidate_model_ids.insert(0, selected_model_id);
                contract.effective_capability_snapshot_ref =
                    format!("capability-snapshot://operator-chat-routing/{lane_id}");
                contract.tool_gate_decision_refs = vec![format!(
                    "toolgate://operator-chat-routing/{lane_id}/generate"
                )];
                contract.procedural_review_status = run.procedural_review_status.clone();
                contract.truncation_warning_ref = run.truncation_warning_ref.clone();
                contract.rejection_reason_refs = run.rejection_reason_refs.clone();
                contract.run_recovery_hint_ref = run.recovery_hint_ref.clone();
                Some(spawn)
            } else {
                None
            };
            let generate_request = spawn.as_ref().map(|spawn| GenerateRequest {
                id: spawn.instance_id.model_id,
                prompt: GenPrompt::new(
                    stage
                        .selection
                        .as_ref()
                        .map(|selection| selection.prompt.clone())
                        .unwrap_or_default(),
                ),
                sampling: SamplingParams::default(),
                lora_overrides: vec![],
                steering_overrides: vec![],
                kv_prefix_handle: None,
                cancel: CancellationToken::new(),
                max_tokens: OPERATOR_CHAT_MAX_TOKENS,
                stop_sequences: vec![],
                speculative_mode: None,
                structured_decoding: None,
            });
            launches.push(ModelLaneRoutingStageLaunch {
                stage_id: stage.stage_id.clone(),
                expected_run_id: request.context.run_id.clone(),
                expected_lane_id: stage.lane_id.clone().unwrap_or_default(),
                expected_model_id: spawn
                    .as_ref()
                    .map(|spawn| spawn.instance_id.model_id.to_string())
                    .unwrap_or_default(),
                expected_provider: spawn.as_ref().and_then(|spawn| spawn.provider),
                request: spawn,
                generate_request,
                authority_lane_id: stage.authority_lane_id.clone(),
            });
        }
        Ok(launches)
    }

    pub async fn execute_routing_lifecycle(
        &self,
        request: OperatorChatRoutingLifecycleRequest,
    ) -> Result<ModelLaneRoutingDispatchBatch, OperatorChatError> {
        let launches = self.routing_launches(&request).await?;
        super::production_factory::execute_production_routing_lifecycle(
            &self.coordinator,
            &request.execution_id,
            &request.selecting_decision_id,
            &request.authority,
            request.context,
            launches,
        )
        .await
        .map_err(OperatorChatError::from)
    }

    pub async fn recover_routing_lifecycle(
        &self,
        request: OperatorChatRoutingLifecycleRequest,
    ) -> Result<ModelLaneRoutingDispatchBatch, OperatorChatError> {
        let launches = self.routing_launches(&request).await?;
        self.coordinator
            .recover_routing_execution(&request.execution_id, launches)
            .await
            .map_err(OperatorChatError::from)
    }

    pub async fn complete_routing_authority(
        &self,
        request: OperatorChatRoutingAuthorityRequest,
    ) -> Result<ModelLaneRoutingDispatchBatch, OperatorChatError> {
        if request.execution_id != request.routing_request.execution_id {
            return Err(OperatorChatError::Invalid(
                "authority execution_id differs from routing request; no mutation performed".into(),
            ));
        }
        let launches = self.routing_launches(&request.routing_request).await?;
        self.coordinator
            .complete_authority_and_resume_routing_lifecycle(
                &request.execution_id,
                &request.stage_id,
                &request.message_id,
                launches,
            )
            .await
            .map_err(OperatorChatError::from)
    }

    pub async fn cancel_routing_lifecycle(
        &self,
        request: OperatorChatRoutingCancelRequest,
    ) -> Result<ModelLaneRoutingExecutionState, OperatorChatError> {
        self.coordinator
            .cancel_routing_execution(&request.execution_id, request.reason)
            .await
            .map_err(OperatorChatError::from)
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

        // HBR-PRIV-005/007. The approver is derived from the store's account
        // context — the request seam (`api/account_scope.rs` today, an
        // authenticated session after WP-KERNEL-006) — and NEVER from
        // `selection.owner_session`, which is a governance role label shared by
        // every operator on every machine.
        //
        // When there is no account context the launch is still allowed to
        // proceed, because this is the documented pre-WP-KERNEL-006 posture for
        // every WP-1 table and refusing here would take away the only cloud
        // launch path on a build that has no authentication at all. What it may
        // NOT do is lie about who approved: the receipt is recorded as explicitly
        // unattributed, the row is stamped with a NULL owning account (so no
        // account-scoped reader can see or reuse it), and
        // `ensure_cloud_launch_authority_tx` will refuse it the moment a launch
        // carries an account context.
        let approver = AccountBoundAuthority::from_access(store.access());
        let approved_by_ref = match &approver {
            AccountBoundAuthority::Account {
                owner_account_id, ..
            } => format!("account://{owner_account_id}/cloud-selection"),
            AccountBoundAuthority::Unattributed { .. } => {
                OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF.to_string()
            }
        };
        let approver = match approver {
            account @ AccountBoundAuthority::Account { .. } => account,
            AccountBoundAuthority::Unattributed { .. } => {
                AccountBoundAuthority::unattributed(OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REASON)
            }
        };
        let stored_plan = store
            .record_cloud_projection_plan(NewModelLaneCloudProjectionPlan {
                projection_plan_id: projection_plan_id.clone(),
                run_id: contract.run_id.clone(),
                trace_id: contract.trace_id.clone(),
                lane_id: Some(contract.lane_id.clone()),
                model_session_id: Some(model_session_id.clone()),
                provider_kind: Some(provider_kind.to_string()),
                requested_model_id: Some(requested_model_id.clone()),
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
                // HBR-PRIV-007: the audience is exactly the disclosed fan-out —
                // this projection delegates to the BYOK provider endpoint and
                // nothing else — and the local visibility it is derived from is
                // the same account that approves it, so a remote export cannot
                // be broader than what that account can already see locally.
                export_delegation: CloudExportDelegation {
                    audience_refs: fan_out_targets.clone(),
                    source_scope: approver.clone(),
                    authorization_receipt_ref: Some(consent_receipt_id.clone()),
                },
                consent_scope: ModelLaneCloudConsentScope::SingleLane,
                target_bindings: vec![],
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
                lane_id: Some(contract.lane_id.clone()),
                model_session_id: Some(model_session_id),
                provider_kind: Some(provider_kind.to_string()),
                requested_model_id: Some(requested_model_id),
                scope_hash,
                consent_scope: ModelLaneCloudConsentScope::SingleLane,
                target_bindings: vec![],
                retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
                export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
                fan_out_targets,
                approved: true,
                approver: approver.clone(),
                approved_by_ref,
                approved_at_utc: now.to_rfc3339(),
                valid_from_utc: (now
                    - Duration::minutes(OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES))
                .to_rfc3339(),
                valid_until_utc: (now
                    + Duration::hours(OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS))
                .to_rfc3339(),
                revoked_at_utc: None,
                revocation_ref: None,
                revocation_input_hash: None,
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
                    // Scoped diagnostic evidence (HBR-PRIV-005/008): the KIND of
                    // approval is visible without disclosing the account id, so
                    // an operator or a diagnostics surface can see at a glance
                    // that a receipt is unattributed rather than having to infer
                    // it from a string that used to look like an approval.
                    "approver_kind": if approver.is_account_bound() { "account_bound" } else { "unattributed" },
                    "consent_validity_hours": OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS,
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
        let (run, _lane, _manager) = self
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
        let mut stream = self
            .coordinator
            .generate_session_managed(instance_id, request)?;
        // The coordinator-owned token this session is actually cancelled through.
        //
        // The in-band `FinishReason::Cancelled` marker is NOT a sufficient fence
        // on its own: output already buffered in the chunk channel can be
        // dequeued ahead of it, so whether a post-cancellation activity block
        // becomes durable would depend on which the executor polls first. Every
        // durable write below is therefore fenced on this token, which is the
        // same signal the runtime observes, so "only pre-cancel output is
        // durable" holds deterministically rather than by luck.
        let session_cancel = self.coordinator.session_cancel_token(instance_id);
        let cancellation_requested = || {
            session_cancel
                .as_ref()
                .is_some_and(|token| token.is_cancelled())
        };
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
                    // Fence on the coordinator token, not just the in-band
                    // marker. An activity block that was already buffered when
                    // cancellation was requested arrives here AFTER the terminal
                    // boundary; persisting it would contradict the lane's
                    // Cancelled status and fabricate durable model output the
                    // operator never accepted.
                    if cancellation_requested() {
                        stream_cancelled = true;
                        stdout_buffer.clear();
                        continue;
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
        if !stream_cancelled && !cancellation_requested() && !stdout_buffer.trim().is_empty() {
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
        if !entry.default_selectable
            || entry.runtime_role != crate::model_runtime::registry::ModelRuntimeRole::Completion
        {
            return Err(OperatorChatError::Invalid(format!(
                "local model_id '{}' has runtime role '{}' and is not eligible as the default completion model",
                selection.model_id,
                entry.runtime_role.as_str()
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
                .with_sandbox_posture(
                    crate::sandbox::TrustClass::Trusted,
                    crate::sandbox::IsolationTier::Tier1Container,
                    std::collections::BTreeSet::from([
                        crate::sandbox::RequiredCapability::HighStdioThroughput,
                    ]),
                    crate::sandbox::NetPolicy::HostInherited,
                    crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
                )
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
    fn an_auto_issued_cloud_consent_cannot_outlive_a_working_day() {
        // Regression guard for the 365-day self-issued approval. If someone
        // widens this again, this test names the reason it was narrowed instead
        // of leaving the number to look arbitrary.
        assert!(
            OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS > 0,
            "an auto-issued consent must have a positive validity window"
        );
        assert!(
            OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS <= 24,
            "an auto-issued cloud-export approval must not survive into the next day: {OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS}h"
        );
        assert!(
            OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES > 0
                && OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES < 60,
            "the clock-skew grace must be a grace, not a second validity window"
        );
    }

    #[test]
    fn the_unattributed_approval_ref_does_not_claim_an_operator_approved() {
        // The replaced value was `operator://<role_label>/cloud-selection`, which
        // read as an operator approval. Its replacement must not.
        assert!(
            !OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF.starts_with("operator://"),
            "an unattributed approval must not present itself as an operator approval"
        );
        assert!(OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF.starts_with("unattributed://"));
    }

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
        assert_eq!(
            request.requested_trust_class,
            Some(crate::sandbox::TrustClass::Trusted)
        );
        assert_eq!(
            request.isolation_tier,
            Some(crate::sandbox::IsolationTier::Tier1Container)
        );
        assert_eq!(
            request.requested_net_policy,
            Some(crate::sandbox::NetPolicy::HostInherited)
        );
        // Assert against the CONSTANT, not a hand-written literal.
        //
        // This assertion previously demanded "execution-policy://operator-chat/official-cli",
        // a value that exists nowhere in src/ except this test. It was not merely stale: the
        // official CLI bridge REJECTS any requested ref that is not
        // CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF (official_cli_bridge.rs
        // `if requested_ref != crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF`),
        // so a spawn request carrying the old expected value would be refused by the product.
        // Satisfying the test as written would have BROKEN the operator-chat CLI lane.
        //
        // Verified pre-existing, not introduced by the 2026-08-04 lanes: the constant is
        // byte-identical at HEAD 50517aec and in the working tree, and neither this literal
        // nor the policy assignment appears in any uncommitted diff.
        //
        // Binding to the constant means a future rename moves both sides together instead of
        // silently re-breaking this test.
        assert_eq!(
            request.requested_execution_policy_ref.as_deref(),
            Some(crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF)
        );
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
    fn build_spawn_request_local_rejects_ready_embedding_role_catalog_bypass() {
        let model_id = ModelId::new_v7();
        let mut registry = crate::model_runtime::ModelRegistry::default();
        registry
            .register(crate::model_runtime::ModelRegistration {
                model_id,
                artifact_path: "D:/models/dedicated-embedding.gguf".into(),
                sha256: [10u8; 32],
                runtime_binding: crate::model_runtime::registry::RuntimeBinding::Candle,
                declared_capabilities: crate::model_runtime::ModelCapabilities {
                    supports_embedding: true,
                    embedding_dimension: Some(768),
                    ..crate::model_runtime::ModelCapabilities::default()
                },
                base_model_tag: crate::model_runtime::BaseModelTag::new(
                    "Dedicated Embedding Model",
                ),
                registered_at_utc: Utc::now(),
                registered_by: crate::model_runtime::OperatorId::new("operator-test"),
                provider: ProviderKind::Local,
            })
            .expect("register dedicated embedding model");
        registry
            .mark_loaded(model_id)
            .expect("mark embedding model READY");
        let catalog = ModelCatalog::from_registry_with_roles(
            Arc::new(registry),
            std::collections::HashMap::from([(
                model_id,
                crate::model_runtime::registry::ModelRuntimeRole::Embedding,
            )]),
        );
        let entry = catalog
            .entry(&model_id.to_string())
            .expect("READY embedding catalog entry");
        assert!(entry.ready);
        assert!(!entry.default_selectable);
        let selection = OperatorChatSelection {
            lane_kind: OperatorChatLaneKind::Local,
            model_id: model_id.to_string(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: Some("wt-embedding-bypass".into()),
            prompt: "this direct launch must fail closed".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };

        let error = build_spawn_request_with_catalog(&selection, 3, Some(catalog.as_ref()))
            .expect_err("direct launch must reject a READY embedding-role catalog row");
        assert!(
            error
                .to_string()
                .contains("not eligible as the default completion model"),
            "unexpected role-boundary error: {error}"
        );
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
