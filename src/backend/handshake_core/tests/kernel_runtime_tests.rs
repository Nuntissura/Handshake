use handshake_core::kernel::{
    ContextBundle, DummyEchoModelAdapter, KernelActor, KernelEventType, KernelTaskRun,
    ModelAdapter, ModelAdapterRequest, SessionBroker, SessionRun, SessionRunState,
    StructuredSummaryModelAdapter,
};
#[cfg(feature = "runtime-full")]
use handshake_core::kernel::{ToolDecisionKind, ToolDecisionRecord};
#[cfg(feature = "runtime-full")]
use handshake_core::mcp::gate::{evaluate_kernel_tool_gate_decision, KernelMcpToolGateRequest};
use serde_json::json;

#[test]
fn kernel_run_identifiers_are_stable_and_prefixed() {
    let task = KernelTaskRun::new("operator-import", json!({"intent": "prove kernel"}));
    let session = SessionRun::queued(&task.kernel_task_run_id, "dummy-echo");

    assert!(task.kernel_task_run_id.starts_with("KTR-"));
    assert!(session.session_run_id.starts_with("SR-"));
    assert_eq!(session.kernel_task_run_id, task.kernel_task_run_id);
    assert_eq!(session.state, SessionRunState::Queued);
}

#[test]
fn session_broker_state_machine_allows_only_legal_kernel_transitions() {
    assert!(SessionBroker::can_transition(
        SessionRunState::Queued,
        SessionRunState::Claimed
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::Claimed,
        SessionRunState::Running
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::Running,
        SessionRunState::Completed
    ));
    assert!(!SessionBroker::can_transition(
        SessionRunState::Completed,
        SessionRunState::Running
    ));

    let event_type =
        SessionBroker::transition_event_type(SessionRunState::Running, SessionRunState::Completed)
            .expect("running to completed is a legal transition");
    assert_eq!(event_type, KernelEventType::SessionCompleted);
}

#[test]
fn broker_cancellation_backpressure_deadletter_states() {
    assert!(SessionBroker::can_transition(
        SessionRunState::Queued,
        SessionRunState::Cancelled
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::Queued,
        SessionRunState::BackpressureDelayed
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::BackpressureDelayed,
        SessionRunState::Queued
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::Failed,
        SessionRunState::RetryScheduled
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::RetryScheduled,
        SessionRunState::Claimed
    ));
    assert!(SessionBroker::can_transition(
        SessionRunState::Failed,
        SessionRunState::DeadLettered
    ));
    assert_eq!(
        SessionBroker::transition_event_type(
            SessionRunState::Queued,
            SessionRunState::BackpressureDelayed
        )
        .expect("backpressure event"),
        KernelEventType::SessionBackpressureDelayed
    );
    assert_eq!(
        SessionBroker::transition_event_type(
            SessionRunState::RetryScheduled,
            SessionRunState::Claimed
        )
        .expect("retry claim event"),
        KernelEventType::SessionClaimed
    );
}

#[test]
fn context_bundle_contract_records_exact_allowed_context_with_stable_hash() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({
            "visible_messages": [
                {"role": "system", "content_artifact_id": "artifact-system"},
                {"role": "user", "content_artifact_id": "artifact-user"}
            ],
            "tool_grants": ["read_trace"],
            "redactions": []
        }),
    )
    .expect("valid context bundle");

    let same_bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({
            "redactions": [],
            "tool_grants": ["read_trace"],
            "visible_messages": [
                {"content_artifact_id": "artifact-system", "role": "system"},
                {"content_artifact_id": "artifact-user", "role": "user"}
            ]
        }),
    )
    .expect("same context bundle with different object order");

    assert!(bundle.context_bundle_id.starts_with("CTX-"));
    assert_eq!(bundle.context_hash, same_bundle.context_hash);
    assert_eq!(bundle.allowed_context["tool_grants"][0], "read_trace");
}

#[test]
#[cfg(feature = "runtime-full")]
fn mcp_toolgate_bridge_records_allow_and_deny_decisions_from_explicit_grants() {
    let allowed = ToolDecisionRecord::from_mcp_gate_decision(
        evaluate_kernel_tool_gate_decision(
            "kernel-toolgate-test",
            ["read_trace".to_string()],
            KernelMcpToolGateRequest {
                tool_request_id: "TOOLREQ-ALLOW".to_string(),
                tool_id: "read_trace".to_string(),
                reason: "test allow".to_string(),
            },
        ),
        KernelEventType::ToolDecisionRecorded,
    );
    let denied = ToolDecisionRecord::from_mcp_gate_decision(
        evaluate_kernel_tool_gate_decision(
            "kernel-toolgate-test",
            ["read_trace".to_string()],
            KernelMcpToolGateRequest {
                tool_request_id: "TOOLREQ-DENY".to_string(),
                tool_id: "write_repo".to_string(),
                reason: "test deny".to_string(),
            },
        ),
        KernelEventType::ToolDecisionRecorded,
    );

    assert_eq!(allowed.gate_id, "kernel-toolgate-test");
    assert_eq!(allowed.event_type, KernelEventType::ToolDecisionRecorded);
    assert_eq!(allowed.decision, ToolDecisionKind::Allow);
    assert_eq!(allowed.tool_id, "read_trace");
    assert_eq!(
        allowed.policy_source,
        "mcp/gate.rs::kernel_tool_gate_bridge"
    );
    assert!(allowed
        .canonical_tool_id
        .starts_with("mcp.kernel_toolgate_test."));
    assert_eq!(denied.decision, ToolDecisionKind::Deny);
    assert_eq!(denied.tool_id, "write_repo");
}

#[tokio::test]
async fn local_model_adapter_implementations_share_the_kernel_event_contract() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({
            "visible_messages": [{"role": "user", "content": "build proof"}],
            "tool_grants": ["read_trace"]
        }),
    )
    .expect("valid context bundle");
    let echo = DummyEchoModelAdapter::new("dummy-echo");
    let structured = StructuredSummaryModelAdapter::new("structured-summary");

    let echo_output = echo
        .invoke(ModelAdapterRequest::new(
            bundle.clone(),
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("echo adapter output");
    let structured_output = structured
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("structured-summary".to_string()),
        ))
        .await
        .expect("structured adapter output");

    let echo_contract = (
        echo_output.response_event_type,
        echo_output.tool_request.event_type,
        echo_output.artifact_proposal.event_type,
        echo_output.tool_request.tool_id,
    );
    let structured_contract = (
        structured_output.response_event_type,
        structured_output.tool_request.event_type,
        structured_output.artifact_proposal.event_type,
        structured_output.tool_request.tool_id,
    );

    assert_eq!(echo_contract, structured_contract);
}

#[tokio::test]
async fn dummy_echo_model_adapter_is_deterministic_and_ledger_ready() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({
            "visible_messages": [{"role": "user", "content": "build proof"}],
            "tool_grants": ["read_trace"]
        }),
    )
    .expect("valid context bundle");
    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let request = ModelAdapterRequest::new(
        bundle.clone(),
        KernelActor::ModelAdapter("dummy-echo".to_string()),
    );

    let first = adapter
        .invoke(request.clone())
        .await
        .expect("first adapter output");
    let second = adapter
        .invoke(request)
        .await
        .expect("second adapter output");

    assert_eq!(first.output_hash, second.output_hash);
    assert_eq!(first.context_bundle_id, bundle.context_bundle_id);
    assert_eq!(
        first.response_event_type,
        KernelEventType::ModelResponseRecorded
    );
    assert_eq!(
        first.tool_request.event_type,
        KernelEventType::ToolRequestRecorded
    );
    assert_eq!(
        first.artifact_proposal.event_type,
        KernelEventType::ArtifactProposed
    );
}
