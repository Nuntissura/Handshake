use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::kernel::{KernelEvent, KernelEventType};

use super::trait_def::{EventLedgerRow, SessionId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceProjection {
    pub session_id: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub what_task: TraceTask,
    pub what_context: TraceContext,
    pub what_returns: Vec<TraceReturn>,
    pub what_tool_calls: Vec<TraceToolCall>,
    pub what_artifacts: Vec<TraceArtifact>,
    pub what_validation: Vec<TraceValidation>,
    pub what_promotion: Option<TracePromotion>,
}

pub type InspectorTraceProjection = TraceProjection;
pub type TraceTaskProjection = TraceTask;
pub type TraceContextProjection = TraceContext;
pub type TraceReturnProjection = TraceReturn;
pub type TraceToolCallProjection = TraceToolCall;
pub type TraceArtifactProjection = TraceArtifact;
pub type TraceValidationProjection = TraceValidation;
pub type TracePromotionProjection = TracePromotion;
pub type TraceEventLedgerRow = EventLedgerRow;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceTask {
    pub wp_id: String,
    pub mt_id: String,
    pub task_summary: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub context_bundle_id: String,
    pub context_summary: String,
    pub hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceReturn {
    pub event_id: String,
    pub content_summary: String,
    pub hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceToolCall {
    pub tool_name: String,
    pub args_hash: String,
    pub result_summary: String,
    pub gate_pass: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceArtifact {
    pub artifact_id: String,
    pub kind: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceValidation {
    pub check_id: String,
    pub verdict: String,
    pub evidence_pointer: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracePromotion {
    pub promotion_event_id: String,
    pub gate_id: String,
    pub verdict: String,
}

#[derive(Clone, Debug)]
struct ProjectionEvent {
    event_id: String,
    event_sequence: i64,
    event_type: String,
    session_run_id: String,
    aggregate_id: String,
    actor_id: String,
    payload_hash: String,
    payload: Value,
    created_at: String,
}

#[derive(Clone, Debug)]
struct ToolCallDraft {
    call_id: String,
    call: TraceToolCall,
}

impl TraceProjection {
    pub fn from_event_ledger(session_id: SessionId, events: &[KernelEvent]) -> Option<Self> {
        let projection_events = events
            .iter()
            .map(ProjectionEvent::from_kernel_event)
            .collect::<Vec<_>>();
        Self::from_projection_events(session_id, projection_events)
    }

    pub fn from_event_rows(session_id: SessionId, rows: &[EventLedgerRow]) -> Option<Self> {
        let projection_events = rows
            .iter()
            .map(ProjectionEvent::from_event_ledger_row)
            .collect::<Vec<_>>();
        Self::from_projection_events(session_id, projection_events)
    }

    fn from_projection_events(session_id: SessionId, events: Vec<ProjectionEvent>) -> Option<Self> {
        let mut events = events
            .into_iter()
            .filter(|event| event.matches_session(&session_id.0))
            .collect::<Vec<_>>();
        if events.is_empty() {
            return None;
        }

        events.sort_by(|left, right| {
            left.event_sequence
                .cmp(&right.event_sequence)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });

        let opened_at = events
            .iter()
            .find(|event| event.event_type == KernelEventType::SessionStarted.as_str())
            .or_else(|| events.first())
            .map(|event| event.created_at.clone())
            .unwrap_or_default();
        let closed_at = events
            .iter()
            .find(|event| is_terminal_session_event(&event.event_type))
            .map(|event| event.created_at.clone());

        let mut projection = TraceProjection {
            session_id: session_id.0,
            opened_at,
            closed_at,
            what_task: TraceTask::default(),
            what_context: TraceContext::default(),
            what_returns: Vec::new(),
            what_tool_calls: Vec::new(),
            what_artifacts: Vec::new(),
            what_validation: Vec::new(),
            what_promotion: None,
        };
        let mut tool_calls = Vec::new();

        for event in &events {
            match event.event_type.as_str() {
                "TASK_INTENT_RECORDED" | "TASK_OPEN" => {
                    projection.what_task = TraceTask {
                        wp_id: text_field(
                            &event.payload,
                            &["wp_id", "wp", "work_packet_id", "work_packet"],
                        )
                        .unwrap_or_default(),
                        mt_id: text_field(
                            &event.payload,
                            &["mt_id", "microtask_id", "task_id", "microtask"],
                        )
                        .unwrap_or_default(),
                        task_summary: text_field(
                            &event.payload,
                            &["task_summary", "summary", "intent", "title"],
                        )
                        .unwrap_or_else(|| summarize_value(&event.payload)),
                    };
                }
                "CONTEXT_BUNDLE_RECORDED" | "CONTEXT_BUNDLE_DISPATCH" => {
                    let hash = content_value(
                        &event.payload,
                        &["context_full", "context", "allowed_context"],
                    )
                    .map(|value| hash_value(value))
                    .or_else(|| text_field(&event.payload, &["context_hash", "hash"]))
                    .unwrap_or_else(|| event.payload_hash.clone());
                    projection.what_context = TraceContext {
                        context_bundle_id: text_field(
                            &event.payload,
                            &["context_bundle_id", "bundle_id", "context_id"],
                        )
                        .unwrap_or_default(),
                        context_summary: text_field(
                            &event.payload,
                            &["context_summary", "summary", "description"],
                        )
                        .unwrap_or_else(|| summarize_value(&event.payload)),
                        hash,
                    };
                }
                "MODEL_RESPONSE_RECORDED" | "MODEL_RESPONSE" => {
                    let content = content_value(
                        &event.payload,
                        &[
                            "response_text",
                            "return_text",
                            "content",
                            "text",
                            "response",
                        ],
                    );
                    let content_text = content
                        .and_then(|value| value_to_text(value))
                        .unwrap_or_else(|| summarize_value(&event.payload));
                    let hash = content
                        .map(|value| hash_value(value))
                        .unwrap_or_else(|| hash_value(&event.payload));
                    projection.what_returns.push(TraceReturn {
                        event_id: event.event_id.clone(),
                        content_summary: first_chars(&content_text, 200),
                        hash,
                    });
                }
                "TOOL_REQUEST_RECORDED" | "TOOL_CALL_REQUEST" => {
                    upsert_tool_call(&mut tool_calls, event, false);
                }
                "TOOL_DECISION_RECORDED" | "TOOL_RESULT_RECORDED" => {
                    upsert_tool_call(&mut tool_calls, event, true);
                }
                "ARTIFACT_STORED" | "ARTIFACT_WRITTEN" => {
                    projection.what_artifacts.push(TraceArtifact {
                        artifact_id: text_field(&event.payload, &["artifact_id", "id", "artifact"])
                            .unwrap_or_else(|| event.event_id.clone()),
                        kind: text_field(
                            &event.payload,
                            &["artifact_kind", "kind", "artifact_layer", "artifact_type"],
                        )
                        .unwrap_or_default(),
                        path: text_field(
                            &event.payload,
                            &[
                                "artifact_payload_ref",
                                "artifact_manifest_ref",
                                "path",
                                "file_path",
                                "ref",
                            ],
                        )
                        .unwrap_or_default(),
                        sha256: text_field(&event.payload, &["content_hash", "sha256", "hash"])
                            .or_else(|| {
                                content_value(&event.payload, &["content", "artifact_content"])
                                    .map(|value| hash_value(value))
                            })
                            .unwrap_or_else(|| event.payload_hash.clone()),
                    });
                }
                "VALIDATION_RECORDED" | "VALIDATION_RESULT" => {
                    projection.what_validation.push(TraceValidation {
                        check_id: text_field(
                            &event.payload,
                            &["check_id", "validation_id", "id", "name"],
                        )
                        .unwrap_or_else(|| event.event_id.clone()),
                        verdict: text_field(&event.payload, &["verdict", "outcome", "status"])
                            .unwrap_or_default(),
                        evidence_pointer: text_field(
                            &event.payload,
                            &["evidence_pointer", "evidence_ref", "artifact_manifest_ref"],
                        )
                        .or_else(|| first_array_text(&event.payload, "evidence_refs"))
                        .unwrap_or_default(),
                    });
                }
                "PROMOTION_DECIDED"
                | "PROMOTION_EVENT"
                | "PROMOTION_ACCEPTED"
                | "PROMOTION_REJECTED"
                | "PROMOTION_REQUESTED" => {
                    projection.what_promotion = Some(TracePromotion {
                        promotion_event_id: event.event_id.clone(),
                        gate_id: text_field(&event.payload, &["gate_id", "promotion_gate_id"])
                            .unwrap_or_else(|| event.actor_id.clone()),
                        verdict: text_field(&event.payload, &["verdict", "decision", "outcome"])
                            .unwrap_or_else(|| event.event_type.to_ascii_lowercase()),
                    });
                }
                _ => {}
            }
        }

        projection.what_tool_calls = tool_calls
            .into_iter()
            .map(|draft| draft.call)
            .collect::<Vec<_>>();
        Some(projection)
    }
}

impl ProjectionEvent {
    fn from_kernel_event(event: &KernelEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            event_sequence: event.event_sequence,
            event_type: event.event_type.as_str().to_string(),
            session_run_id: event.session_run_id.clone(),
            aggregate_id: event.aggregate_id.clone(),
            actor_id: event.actor.actor_id().to_string(),
            payload_hash: event.payload_hash.clone(),
            payload: event.payload.clone(),
            created_at: event.created_at.to_rfc3339(),
        }
    }

    fn from_event_ledger_row(row: &EventLedgerRow) -> Self {
        Self {
            event_id: row.event_id.clone(),
            event_sequence: row.event_sequence,
            event_type: normalize_event_type(&row.event_type),
            session_run_id: row.session_run_id.clone(),
            aggregate_id: row.aggregate_id.clone(),
            actor_id: row.actor_id.clone(),
            payload_hash: row.payload_hash.clone(),
            payload: row.payload.clone(),
            created_at: row.created_at_utc.clone(),
        }
    }

    fn matches_session(&self, session_id: &str) -> bool {
        self.session_run_id == session_id
            || self.aggregate_id == session_id
            || text_field(
                &self.payload,
                &["session_id", "session_run_id", "parent_session_id"],
            )
            .as_deref()
                == Some(session_id)
    }
}

fn upsert_tool_call(
    tool_calls: &mut Vec<ToolCallDraft>,
    event: &ProjectionEvent,
    is_decision: bool,
) {
    let call_id = text_field(&event.payload, &["tool_call_id", "request_id", "id"])
        .unwrap_or_else(|| event.event_id.clone());
    let tool_name = text_field(
        &event.payload,
        &["tool_name", "canonical_tool_id", "tool_id", "name"],
    )
    .unwrap_or_else(|| event.actor_id.clone());
    let args_hash = text_field(&event.payload, &["args_hash", "arguments_hash"])
        .or_else(|| {
            content_value(&event.payload, &["args", "arguments"]).map(|value| hash_value(value))
        })
        .unwrap_or_default();
    let result_summary = text_field(
        &event.payload,
        &["result_summary", "summary", "result_ref", "result"],
    )
    .unwrap_or_default();
    let gate_pass = bool_field(&event.payload, &["gate_pass", "allowed", "passed"])
        .unwrap_or_else(|| decision_is_pass(&event.payload, is_decision));

    if let Some(existing) = tool_calls
        .iter_mut()
        .find(|draft| draft.call_id == call_id || draft.call.tool_name == tool_name)
    {
        if !tool_name.is_empty() {
            existing.call.tool_name = tool_name;
        }
        if !args_hash.is_empty() {
            existing.call.args_hash = args_hash;
        }
        if !result_summary.is_empty() {
            existing.call.result_summary = result_summary;
        }
        existing.call.gate_pass = existing.call.gate_pass || gate_pass;
        return;
    }

    tool_calls.push(ToolCallDraft {
        call_id,
        call: TraceToolCall {
            tool_name,
            args_hash,
            result_summary,
            gate_pass,
        },
    });
}

fn text_field(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| payload.get(*key))
        .filter_map(|value| value_to_text(value))
        .find(|value| !value.trim().is_empty())
}

fn bool_field(payload: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .filter_map(|key| payload.get(*key))
        .find_map(Value::as_bool)
}

fn content_value<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .filter_map(|key| payload.get(*key))
        .find(|value| !value.is_null())
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).ok(),
        Value::Null => None,
    }
}

fn summarize_value(value: &Value) -> String {
    first_chars(
        &serde_json::to_string(value).unwrap_or_else(|_| String::new()),
        200,
    )
}

fn first_array_text(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|value| value_to_text(value))
}

fn first_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn hash_value(value: &Value) -> String {
    match value {
        Value::String(text) => sha256_hex(text.as_bytes()),
        _ => serde_json::to_vec(value)
            .map(|bytes| sha256_hex(&bytes))
            .unwrap_or_else(|_| sha256_hex(&[])),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn trace_content_sha256(content: &str) -> String {
    sha256_hex(content.as_bytes())
}

fn decision_is_pass(payload: &Value, is_decision: bool) -> bool {
    if !is_decision {
        return false;
    }
    text_field(payload, &["decision", "verdict", "outcome", "status"])
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "allow" | "allowed" | "approve" | "approved" | "pass" | "passed" | "ok"
            )
        })
        .unwrap_or(false)
}

fn is_terminal_session_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "SESSION_COMPLETED" | "SESSION_FAILED" | "SESSION_CANCELLED" | "SESSION_DEAD_LETTERED"
    )
}

fn normalize_event_type(event_type: &str) -> String {
    event_type
        .trim()
        .replace(['-', ' '], "_")
        .to_ascii_uppercase()
}
