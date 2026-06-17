//! WP-KERNEL-004 (ROI follow-up): typed `FR-EVT-AGENT-*` Flight-Recorder events
//! for structured agent activity captured from the official-CLI bridge's JSON
//! output stream.
//!
//! These mirror the `FR-EVT-LLM-INFER-*` events ([`super::events_llm_infer`]):
//! a stable `event_id`, a `schema_version`, a shared `request_id` correlation
//! (the SAME id the runtime mints for the INFER START/END of the request, so
//! agent-activity rows correlate to their inference), and an `ordered_index`
//! for stable within-request ordering.
//!
//! ## Session correlation
//!
//! The swarm composite `instance_id` (`<model_id>#<instance>`) is stamped onto
//! BOTH `session_span_id` (via [`FlightRecorderEvent::with_session_span`]) AND
//! `payload.instance_id`, so the session-transcript aggregator's raw seam
//! (`session_span_id == instance_id` OR `payload.instance_id == instance_id`)
//! picks the rows up with no aggregator change. On the production swarm path the
//! factory threads this composite into the runtime
//! (`CliBridgeCloudRuntimeBuilder::build_loaded` ->
//! `CliBridgeModelRuntime::with_session_correlation`), so real swarm sessions
//! carry it.
//!
//! When the instance id is unknown (`None` — an ad-hoc build with no swarm
//! session, never the production swarm path), the event still emits carrying
//! `model_id` so it is recorded and NEVER dropped, but it is NOT
//! transcript-retrievable: the session-transcript fetch
//! (`app/src-tauri/src/commands/session_transcript.rs::fetch_fr_events`) keys
//! ONLY on the composite session id via `model_session_id` /
//! `session_span_id` / `payload.instance_id` — there is no bare-`model_id` seam.
//! A `None` event is therefore durable-but-invisible to the per-session
//! timeline; it is not a partial-visibility degradation. Session correlation
//! must be threaded for the row to reach the transcript / SessionReplayPanel.
//!
//! ## Redaction
//!
//! [`agent_activity_event`] runs every captured string surface (tool input,
//! thinking text, text body, raw fallback) through the supplied
//! [`SecretRedactor`] BEFORE the payload is built, so the structured surface is
//! no leakier than the raw terminal capture (which redacts the byte stream via
//! `redact_chunk`).

use serde_json::{json, Value};
use uuid::Uuid;

use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::model_runtime::cloud::agent_activity::{AgentActivity, AgentActivityKind};
use crate::model_runtime::ModelId;
use crate::terminal::redaction::SecretRedactor;

pub const FR_EVT_AGENT_TOOLCALL: &str = "FR-EVT-AGENT-TOOLCALL";
pub const FR_EVT_AGENT_THINKING: &str = "FR-EVT-AGENT-THINKING";
pub const FR_EVT_AGENT_TEXT: &str = "FR-EVT-AGENT-TEXT";
pub const FR_EVT_AGENT_OTHER: &str = "FR-EVT-AGENT-OTHER";
pub const AGENT_ACTIVITY_SCHEMA: &str = "hsk.fr.agent_activity@0.1";

/// The stable `FR-EVT-AGENT-*` id for a given activity kind.
pub fn agent_event_id(kind: AgentActivityKind) -> &'static str {
    match kind {
        AgentActivityKind::ToolCall => FR_EVT_AGENT_TOOLCALL,
        AgentActivityKind::Thinking => FR_EVT_AGENT_THINKING,
        AgentActivityKind::Text => FR_EVT_AGENT_TEXT,
        AgentActivityKind::Other => FR_EVT_AGENT_OTHER,
    }
}

/// Redact a single string surface through the lane redactor (returns the
/// redacted form; a non-matching string round-trips unchanged).
fn redact_str(redactor: &dyn SecretRedactor, text: &str) -> String {
    redactor.redact_chunk(text.as_bytes()).redacted
}

/// Redact a JSON value's string leaves recursively. Strings are redacted;
/// objects/arrays recurse; other scalars pass through. This keeps a structured
/// tool `input` object no leakier than the raw stream without forcing a
/// serialize/redact/reparse round-trip that could change shape.
fn redact_value(redactor: &dyn SecretRedactor, value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(redact_str(redactor, s)),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| redact_value(redactor, v)).collect())
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(k.clone(), redact_value(redactor, v));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Build a typed `FR-EVT-AGENT-*` event from one parsed [`AgentActivity`].
///
/// - `request_id`: the per-request correlation (reuse the INFER request id so
///   agent rows join their inference).
/// - `ordered_index`: a within-request monotonic index for stable ordering.
/// - `instance_id`: the swarm composite (`<model_id>#<instance>`) when known;
///   stamped onto `session_span_id` + `payload.instance_id` for the raw seam.
/// - `redactor`: every captured string surface is run through it first.
#[allow(clippy::too_many_arguments)]
pub fn agent_activity_event(
    model_id: ModelId,
    request_id: Uuid,
    ordered_index: u64,
    instance_id: Option<&str>,
    adapter: &str,
    activity: &AgentActivity,
    redactor: &dyn SecretRedactor,
) -> FlightRecorderEvent {
    let kind = activity.kind();
    let event_id = agent_event_id(kind);

    // Build the kind-specific payload fields, redacting every string surface.
    let (name, detail, text): (Option<String>, Option<Value>, Option<String>) = match activity {
        AgentActivity::ToolCall { name, input, .. } => (
            Some(redact_str(redactor, name)),
            Some(redact_value(redactor, input)),
            None,
        ),
        AgentActivity::Thinking { text } => (None, None, Some(redact_str(redactor, text))),
        AgentActivity::Text { text } => (None, None, Some(redact_str(redactor, text))),
        AgentActivity::Other { raw } => (None, None, Some(redact_str(redactor, raw))),
    };

    let mut payload = json!({
        "schema_version": AGENT_ACTIVITY_SCHEMA,
        "event_id": event_id,
        "type": "agent_activity",
        "activity_kind": kind.label(),
        "trace_id": request_id.to_string(),
        "request_id": request_id.to_string(),
        "model_call_correlation_id": request_id.to_string(),
        "model_id": model_id.to_string(),
        "adapter": adapter,
        "ordered_index": ordered_index,
    });
    if let Value::Object(map) = &mut payload {
        if let Some(name) = name {
            map.insert("name".to_string(), Value::String(name));
        }
        if let Some(detail) = detail {
            map.insert("detail".to_string(), detail);
        }
        if let Some(text) = text {
            map.insert("text".to_string(), Value::String(text));
        }
        if let Some(instance_id) = instance_id {
            map.insert(
                "instance_id".to_string(),
                Value::String(instance_id.to_string()),
            );
        }
    }

    // ToolCall reuses the existing ToolCall event_type; thinking/text/other use
    // System (the distinguishing event_id carries the kind — mirrors INFER which
    // classifies by event_id, not enum churn).
    let event_type = match kind {
        AgentActivityKind::ToolCall => FlightRecorderEventType::ToolCall,
        _ => FlightRecorderEventType::System,
    };

    let mut event =
        FlightRecorderEvent::new(event_type, FlightRecorderActor::Agent, request_id, payload)
            .with_model_id(model_id.to_string());
    if let Some(instance_id) = instance_id {
        event = event.with_session_span(instance_id.to_string());
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::redaction::PatternRedactor;
    use serde_json::json;

    fn req_id() -> Uuid {
        Uuid::now_v7()
    }

    #[test]
    fn toolcall_event_has_id_name_detail_and_session_span() {
        let activity = AgentActivity::ToolCall {
            name: "Bash".to_string(),
            input: json!({"command": "ls"}),
            call_id: Some("toolu_1".to_string()),
        };
        let ev = agent_activity_event(
            ModelId::new_v7(),
            req_id(),
            7,
            Some("mid#0"),
            "official_cli_bridge",
            &activity,
            &PatternRedactor,
        );
        assert_eq!(ev.event_type, FlightRecorderEventType::ToolCall);
        assert_eq!(ev.session_span_id.as_deref(), Some("mid#0"));
        assert_eq!(ev.payload.get("event_id").unwrap(), FR_EVT_AGENT_TOOLCALL);
        assert_eq!(ev.payload.get("activity_kind").unwrap(), "tool_call");
        assert_eq!(ev.payload.get("name").unwrap(), "Bash");
        assert_eq!(ev.payload.get("instance_id").unwrap(), "mid#0");
        assert_eq!(ev.payload.get("ordered_index").unwrap(), 7);
        assert_eq!(
            ev.payload
                .get("detail")
                .and_then(|d| d.get("command"))
                .unwrap(),
            "ls"
        );
    }

    #[test]
    fn thinking_event_uses_system_type_and_text() {
        let activity = AgentActivity::Thinking {
            text: "reasoning here".to_string(),
        };
        let ev = agent_activity_event(
            ModelId::new_v7(),
            req_id(),
            1,
            None,
            "official_cli_bridge",
            &activity,
            &PatternRedactor,
        );
        assert_eq!(ev.event_type, FlightRecorderEventType::System);
        assert_eq!(ev.payload.get("event_id").unwrap(), FR_EVT_AGENT_THINKING);
        assert_eq!(ev.payload.get("text").unwrap(), "reasoning here");
        // instance_id absent -> no session_span (honest degradation).
        assert!(ev.session_span_id.is_none());
        assert!(ev.payload.get("instance_id").is_none());
    }

    #[test]
    fn other_event_keeps_raw_text() {
        let activity = AgentActivity::Other {
            raw: "weird line".to_string(),
        };
        let ev = agent_activity_event(
            ModelId::new_v7(),
            req_id(),
            0,
            None,
            "official_cli_bridge",
            &activity,
            &PatternRedactor,
        );
        assert_eq!(ev.payload.get("event_id").unwrap(), FR_EVT_AGENT_OTHER);
        assert_eq!(ev.payload.get("text").unwrap(), "weird line");
    }

    #[test]
    fn secrets_in_toolcall_input_and_text_are_redacted() {
        let activity = AgentActivity::ToolCall {
            name: "Bash".to_string(),
            input: json!({"command": "export API_KEY=supersecretvalue123 && run"}),
            call_id: None,
        };
        let ev = agent_activity_event(
            ModelId::new_v7(),
            req_id(),
            0,
            None,
            "official_cli_bridge",
            &activity,
            &PatternRedactor,
        );
        let command = ev
            .payload
            .get("detail")
            .and_then(|d| d.get("command"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            command.contains("REDACTED"),
            "secret must be redacted: {command}"
        );
        assert!(!command.contains("supersecretvalue123"));

        let think = AgentActivity::Thinking {
            text: "the password=hunter2value is".to_string(),
        };
        let ev2 = agent_activity_event(
            ModelId::new_v7(),
            req_id(),
            0,
            None,
            "official_cli_bridge",
            &think,
            &PatternRedactor,
        );
        let text = ev2.payload.get("text").and_then(|v| v.as_str()).unwrap();
        assert!(
            text.contains("REDACTED"),
            "thinking secret must be redacted: {text}"
        );
        assert!(!text.contains("hunter2value"));
    }

    #[test]
    fn request_id_correlates_across_events() {
        let rid = req_id();
        let a = agent_activity_event(
            ModelId::new_v7(),
            rid,
            0,
            None,
            "official_cli_bridge",
            &AgentActivity::Text { text: "x".into() },
            &PatternRedactor,
        );
        assert_eq!(
            a.payload.get("request_id").and_then(|v| v.as_str()),
            Some(rid.to_string().as_str())
        );
    }
}
