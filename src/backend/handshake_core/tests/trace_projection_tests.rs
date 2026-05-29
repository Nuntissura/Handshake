#![cfg(feature = "inspector")]

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use handshake_core::{
    inspector_read::{
        EventLedgerRow, InspectorReadSnapshot, InspectorServer, InspectorTraceProjection, SessionId,
    },
    kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent},
};

const KERNEL_TASK_RUN_ID: &str = "ktr-alpha";
const SESSION_ID: &str = "session-alpha";

#[test]
fn trace_projection_tests_reconstructs_all_sections_from_event_ledger() {
    let events = synthetic_trace_events();

    let projection =
        InspectorTraceProjection::from_event_ledger(SessionId::new(SESSION_ID), &events)
            .expect("trace projection for session events");

    assert_eq!(projection.session_id, SESSION_ID);
    assert_eq!(projection.opened_at, "2026-05-18T09:00:02+00:00");
    assert_eq!(
        projection.closed_at.as_deref(),
        Some("2026-05-18T09:00:09+00:00")
    );
    assert_eq!(projection.what_task.wp_id, "WP-KERNEL-004");
    assert_eq!(projection.what_task.mt_id, "MT-031");
    assert_eq!(
        projection.what_task.task_summary,
        "Build TraceProjection inspector route"
    );

    assert_eq!(projection.what_context.context_bundle_id, "ctx-alpha");
    assert_eq!(
        projection.what_context.context_summary,
        "Repo authority, MT-031 contract, and source files"
    );
    assert_eq!(
        projection.what_context.hash,
        sha256_hex("full context body used by the local model")
    );

    assert_eq!(projection.what_returns.len(), 1);
    assert_eq!(projection.what_returns[0].event_id, "evt-004");
    assert_eq!(
        projection.what_returns[0].content_summary,
        "projection implementation answer"
    );
    assert_eq!(
        projection.what_returns[0].hash,
        sha256_hex("projection implementation answer")
    );

    assert_eq!(projection.what_tool_calls.len(), 1);
    assert_eq!(projection.what_tool_calls[0].tool_name, "cargo test");
    assert_eq!(
        projection.what_tool_calls[0].args_hash,
        sha256_hex("cargo test -p handshake_core --features inspector trace_projection_tests")
    );
    assert_eq!(
        projection.what_tool_calls[0].result_summary,
        "trace projection tests passed"
    );
    assert!(projection.what_tool_calls[0].gate_pass);

    assert_eq!(projection.what_artifacts.len(), 1);
    assert_eq!(
        projection.what_artifacts[0].artifact_id,
        "artifact-trace-projection"
    );
    assert_eq!(projection.what_artifacts[0].kind, "source_patch");
    assert_eq!(
        projection.what_artifacts[0].path,
        "src/backend/handshake_core/src/inspector_read/trace_projection.rs"
    );
    assert_eq!(
        projection.what_artifacts[0].sha256,
        sha256_hex("artifact full content")
    );

    assert_eq!(projection.what_validation.len(), 1);
    assert_eq!(
        projection.what_validation[0].check_id,
        "trace_projection_tests"
    );
    assert_eq!(projection.what_validation[0].verdict, "passed");
    assert_eq!(
        projection.what_validation[0].evidence_pointer,
        "cargo://trace_projection_tests"
    );

    let promotion = projection.what_promotion.expect("promotion decision");
    assert_eq!(promotion.promotion_event_id, "evt-008");
    assert_eq!(promotion.gate_id, "promotion-gate-1");
    assert_eq!(promotion.verdict, "approved");
}

#[test]
fn trace_projection_tests_empty_session_returns_none_not_empty_projection() {
    let events = synthetic_trace_events();

    assert!(
        InspectorTraceProjection::from_event_ledger(SessionId::new("missing-session"), &events)
            .is_none(),
        "a session with no durable events must map to HTTP 404, not an empty trace"
    );
}

#[test]
fn trace_projection_tests_hashes_full_return_content_not_summary() {
    let shared_prefix = "a".repeat(200);
    let first_content = format!("{shared_prefix}first-tail");
    let second_content = format!("{shared_prefix}second-tail");
    let events = vec![
        event(
            1,
            KernelEventType::SessionStarted,
            KernelActor::SessionBroker("broker".to_string()),
            json!({}),
        ),
        event(
            2,
            KernelEventType::ModelResponseRecorded,
            KernelActor::ModelAdapter("local-model".to_string()),
            json!({ "response_text": first_content }),
        ),
        event(
            3,
            KernelEventType::ModelResponseRecorded,
            KernelActor::ModelAdapter("local-model".to_string()),
            json!({ "response_text": second_content }),
        ),
    ];

    let projection =
        InspectorTraceProjection::from_event_ledger(SessionId::new(SESSION_ID), &events)
            .expect("trace projection");

    assert_eq!(projection.what_returns[0].content_summary, shared_prefix);
    assert_eq!(projection.what_returns[1].content_summary, shared_prefix);
    assert_ne!(
        projection.what_returns[0].hash,
        projection.what_returns[1].hash
    );
    assert_eq!(projection.what_returns[0].hash, sha256_hex(&first_content));
    assert_eq!(projection.what_returns[1].hash, sha256_hex(&second_content));
}

#[tokio::test]
async fn trace_projection_tests_route_returns_projection_and_missing_trace_404() {
    let mut snapshot = InspectorReadSnapshot::default();
    snapshot.event_ledger_tail = event_rows(synthetic_trace_events());

    let handle = InspectorServer::start(Arc::new(snapshot))
        .await
        .expect("server starts");
    let base = format!("http://{}", handle.addr());
    let client = reqwest::Client::new();

    let projection: InspectorTraceProjection = client
        .get(format!("{base}/inspector/v1/trace/{SESSION_ID}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(projection.what_task.mt_id, "MT-031");

    let missing = client
        .get(format!("{base}/inspector/v1/trace/missing-session"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

    let post_status = client
        .post(format!("{base}/inspector/v1/trace/{SESSION_ID}"))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(post_status, reqwest::StatusCode::METHOD_NOT_ALLOWED);
}

fn synthetic_trace_events() -> Vec<KernelEvent> {
    vec![
        event(
            1,
            KernelEventType::TaskIntentRecorded,
            KernelActor::Operator("operator".to_string()),
            json!({
                "wp_id": "WP-KERNEL-004",
                "mt_id": "MT-031",
                "task_summary": "Build TraceProjection inspector route"
            }),
        ),
        event(
            2,
            KernelEventType::SessionStarted,
            KernelActor::SessionBroker("broker".to_string()),
            json!({}),
        ),
        event(
            3,
            KernelEventType::ContextBundleRecorded,
            KernelActor::System("context-bundler".to_string()),
            json!({
                "context_bundle_id": "ctx-alpha",
                "context_summary": "Repo authority, MT-031 contract, and source files",
                "context_full": "full context body used by the local model"
            }),
        ),
        event(
            4,
            KernelEventType::ModelResponseRecorded,
            KernelActor::ModelAdapter("local-model".to_string()),
            json!({
                "response_text": "projection implementation answer"
            }),
        ),
        event(
            5,
            KernelEventType::ToolDecisionRecorded,
            KernelActor::ToolGate("tool-gate".to_string()),
            json!({
                "tool_call_id": "tool-call-1",
                "canonical_tool_id": "cargo test",
                "args_hash": sha256_hex("cargo test -p handshake_core --features inspector trace_projection_tests"),
                "result_summary": "trace projection tests passed",
                "decision": "allow"
            }),
        ),
        event(
            6,
            KernelEventType::ArtifactStored,
            KernelActor::System("artifact-ledger".to_string()),
            json!({
                "artifact_id": "artifact-trace-projection",
                "artifact_kind": "source_patch",
                "artifact_payload_ref": "src/backend/handshake_core/src/inspector_read/trace_projection.rs",
                "content_hash": sha256_hex("artifact full content")
            }),
        ),
        event(
            7,
            KernelEventType::ValidationRecorded,
            KernelActor::ValidationRunner("cargo".to_string()),
            json!({
                "check_id": "trace_projection_tests",
                "verdict": "passed",
                "evidence_pointer": "cargo://trace_projection_tests"
            }),
        ),
        event(
            8,
            KernelEventType::PromotionDecided,
            KernelActor::PromotionGate("promotion-gate-1".to_string()),
            json!({
                "gate_id": "promotion-gate-1",
                "verdict": "approved"
            }),
        ),
        event(
            9,
            KernelEventType::SessionCompleted,
            KernelActor::SessionBroker("broker".to_string()),
            json!({}),
        ),
    ]
}

fn event(seq: i64, event_type: KernelEventType, actor: KernelActor, payload: Value) -> KernelEvent {
    let new = NewKernelEvent::builder(KERNEL_TASK_RUN_ID, SESSION_ID, event_type, actor)
        .aggregate("session_run", SESSION_ID)
        .idempotency_key(format!("trace-projection-tests-{seq}"))
        .source_component("trace_projection_tests")
        .payload(payload)
        .build()
        .expect("valid kernel event");
    let mut event = KernelEvent::from_new(new);
    event.event_id = format!("evt-{seq:03}");
    event.event_sequence = seq;
    event.created_at = DateTime::parse_from_rfc3339(&format!("2026-05-18T09:00:{seq:02}Z"))
        .unwrap()
        .with_timezone(&Utc);
    event
}

fn event_rows(events: Vec<KernelEvent>) -> Vec<EventLedgerRow> {
    events
        .into_iter()
        .map(|event| EventLedgerRow {
            event_id: event.event_id,
            event_type: event.event_type.as_str().to_string(),
            event_sequence: event.event_sequence,
            created_at_utc: event.created_at.to_rfc3339(),
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            actor_kind: event.actor.actor_kind().to_string(),
            actor_id: event.actor.actor_id().to_string(),
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            payload_hash: event.payload_hash,
            source_component: event.source_component,
            payload: event.payload,
        })
        .collect()
}

fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
