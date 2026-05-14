use handshake_core::kernel::{
    ArtifactRecord, ContextBundle, DummyEchoModelAdapter, KernelActor, KernelEvent,
    KernelEventType, ModelAdapter, ModelAdapterRequest, NewKernelEvent, OperatorPromotionApproval,
    PromotionDecisionKind, PromotionGate, TraceProjection, ValidationOutcome, ValidationRunner,
};
use serde_json::json;
use tempfile::tempdir;

fn kernel_event(event_type: KernelEventType, payload: serde_json::Value) -> KernelEvent {
    KernelEvent::from_new(
        NewKernelEvent::builder(
            "KTR-EXAMPLE",
            "SR-EXAMPLE",
            event_type,
            KernelActor::System("proof".to_string()),
        )
        .correlation_id("corr-proof")
        .payload(payload)
        .build()
        .expect("event"),
    )
}

fn chained_kernel_events() -> Vec<KernelEvent> {
    let event_types = [
        KernelEventType::TaskIntentRecorded,
        KernelEventType::SessionQueued,
        KernelEventType::SessionClaimed,
        KernelEventType::SessionStarted,
        KernelEventType::ContextBundleRecorded,
        KernelEventType::ModelAdapterInvoked,
        KernelEventType::ModelResponseRecorded,
        KernelEventType::ToolRequestRecorded,
        KernelEventType::ToolDecisionRecorded,
        KernelEventType::ArtifactProposed,
        KernelEventType::ArtifactStored,
        KernelEventType::ValidationRecorded,
        KernelEventType::PromotionDecided,
        KernelEventType::SessionCompleted,
    ];
    let mut prior_event_id: Option<String> = None;
    event_types
        .into_iter()
        .enumerate()
        .map(|(index, event_type)| {
            let payload = match event_type {
                KernelEventType::ToolDecisionRecorded => json!({
                    "decision": "allow",
                    "gate_receipt_kind": "mcp_gate_wrapped_decision",
                    "args_ref": "kernel-tool-args://example",
                    "args_hash": "0".repeat(64),
                    "result_ref": "kernel-tool-result://example",
                    "result_hash": "1".repeat(64)
                }),
                KernelEventType::ArtifactStored => json!({
                    "artifact_id": "ART-example",
                    "artifact_manifest_ref": ".handshake/artifacts/L2/example/artifact.json",
                    "content_hash": "2".repeat(64)
                }),
                KernelEventType::ValidationRecorded => json!({
                    "validation_id": "VAL-example",
                    "artifact_id": "ART-example",
                    "content_hash": "2".repeat(64),
                    "artifact_content_hash_validated": true,
                    "evidence_refs": [".handshake/artifacts/L2/example/artifact.json"]
                }),
                KernelEventType::PromotionDecided => json!({
                    "promotion_decision_id": "PROM-example",
                    "artifact_id": "ART-example",
                    "validation_id": "VAL-example",
                    "decision": "APPROVED",
                    "operator_id": "operator-ilja",
                    "operator_review": {
                        "review_required": true,
                        "approval_source": "operator_review_receipt",
                        "review_receipt_id": "OPERATOR-REVIEW-example"
                    }
                }),
                _ => json!({"authority": "event_ledger"}),
            };
            let mut builder = NewKernelEvent::builder(
                "KTR-EXAMPLE",
                "SR-EXAMPLE",
                event_type,
                KernelActor::System("proof".to_string()),
            )
            .idempotency_key(format!("idem-trace-{index}"))
            .correlation_id("corr-proof")
            .payload(payload);
            if let Some(causation_id) = prior_event_id.as_ref() {
                builder = builder.causation_id(causation_id.clone());
            }
            let mut event = KernelEvent::from_new(builder.build().expect("event"));
            event.event_sequence = index as i64 + 1;
            prior_event_id = Some(event.event_id.clone());
            event
        })
        .collect()
}

#[tokio::test]
async fn validation_runner_contract_and_promotion_gate_contract_require_evidence_before_approval() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({"visible_messages": [{"role": "user", "content": "proof"}]}),
    )
    .expect("valid bundle");
    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let output = adapter
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("adapter output");
    let artifact = ArtifactRecord::from_adapter_output("KTR-EXAMPLE", "SR-EXAMPLE", &output)
        .expect("artifact record");

    let approval = OperatorPromotionApproval::new("operator-ilja", "approve proof");
    let err = PromotionGate::decide(
        &artifact,
        None,
        PromotionDecisionKind::Approved,
        approval.clone(),
    )
    .expect_err("approval without validation must fail closed");
    assert!(err.to_string().contains("validation"));

    let validation = ValidationRunner::record(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        &artifact,
        ValidationOutcome::Passed,
        json!({
            "command": "cargo test end_to_end_kernel_proof",
            "exit_code": 0,
            "artifact_id": artifact.artifact_id.clone(),
            "content_hash": artifact.content_hash.clone(),
            "artifact_content_hash_validated": true,
            "evidence_refs": ["memory://test-evidence"]
        }),
    )
    .expect("validation record");
    let decision = PromotionGate::decide(
        &artifact,
        Some(&validation),
        PromotionDecisionKind::Approved,
        approval,
    )
    .expect("validated approval");

    assert_eq!(validation.event_type, KernelEventType::ValidationRecorded);
    assert_eq!(decision.event_type, KernelEventType::PromotionDecided);
    assert_eq!(decision.decision, PromotionDecisionKind::Approved);
    assert_eq!(
        decision.validation_id.as_deref(),
        Some(validation.validation_id.as_str())
    );
}

#[tokio::test]
async fn validation_runner_rejects_pass_without_artifact_hash_evidence() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({"visible_messages": [{"role": "user", "content": "proof"}]}),
    )
    .expect("valid bundle");
    let output = DummyEchoModelAdapter::new("dummy-echo")
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("adapter output");
    let artifact = ArtifactRecord::from_adapter_output("KTR-EXAMPLE", "SR-EXAMPLE", &output)
        .expect("artifact record");

    let err = ValidationRunner::record(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        &artifact,
        ValidationOutcome::Passed,
        json!({}),
    )
    .expect_err("passed validation must require artifact hash evidence");

    assert!(
        err.to_string().contains("artifact hash evidence"),
        "unexpected validation evidence error: {err}"
    );
}

#[tokio::test]
async fn promotion_gate_rejects_fixture_operator_review_for_approval() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({"visible_messages": [{"role": "user", "content": "proof"}]}),
    )
    .expect("valid bundle");
    let output = DummyEchoModelAdapter::new("dummy-echo")
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("adapter output");
    let artifact = ArtifactRecord::from_adapter_output("KTR-EXAMPLE", "SR-EXAMPLE", &output)
        .expect("artifact record");
    let validation = ValidationRunner::record(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        &artifact,
        ValidationOutcome::Passed,
        json!({
            "artifact_id": artifact.artifact_id.clone(),
            "content_hash": artifact.content_hash.clone(),
            "artifact_content_hash_validated": true,
            "evidence_refs": ["memory://test-evidence"]
        }),
    )
    .expect("validation record");

    let err = PromotionGate::decide(
        &artifact,
        Some(&validation),
        PromotionDecisionKind::Approved,
        OperatorPromotionApproval::new(
            "kernel-proof-operator-review",
            "deterministic first-slice proof operator-review fixture",
        ),
    )
    .expect_err("fixture operator approval must not become product authority");

    assert!(
        err.to_string().contains("operator-reviewable"),
        "unexpected fixture rejection error: {err}"
    );
}

#[tokio::test]
async fn tool_request_event_contract_and_artifact_proposal_contract_are_adapter_output_shapes() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({"visible_messages": [{"role": "user", "content": "proof"}]}),
    )
    .expect("valid bundle");
    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let output = adapter
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("adapter output");

    assert_eq!(
        output.tool_request.event_type,
        KernelEventType::ToolRequestRecorded
    );
    assert!(output.tool_request.tool_request_id.starts_with("TOOLREQ-"));
    assert_eq!(output.tool_request.tool_id, "read_trace");
    assert_eq!(
        output.artifact_proposal.event_type,
        KernelEventType::ArtifactProposed
    );
    assert!(output
        .artifact_proposal
        .artifact_proposal_id
        .starts_with("AP-"));
}

#[tokio::test]
async fn artifact_store_ledger_link_uses_adapter_output_provenance() {
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({"visible_messages": [{"role": "user", "content": "proof"}]}),
    )
    .expect("valid bundle");
    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let output = adapter
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("adapter output");
    let artifact = ArtifactRecord::from_adapter_output("KTR-EXAMPLE", "SR-EXAMPLE", &output)
        .expect("artifact record");

    assert_eq!(artifact.event_type, KernelEventType::ArtifactStored);
    assert_eq!(
        artifact.artifact_proposal_id,
        output.artifact_proposal.artifact_proposal_id
    );
    assert_eq!(artifact.content_hash, output.output_hash);
}

#[tokio::test]
async fn artifact_store_writes_manifest_and_trace_links_manifest_ref() {
    let workspace = tempdir().expect("artifact workspace tempdir");
    let bundle = ContextBundle::new(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        json!({"visible_messages": [{"role": "user", "content": "proof"}]}),
    )
    .expect("valid bundle");
    let output = DummyEchoModelAdapter::new("dummy-echo")
        .invoke(ModelAdapterRequest::new(
            bundle,
            KernelActor::ModelAdapter("dummy-echo".to_string()),
        ))
        .await
        .expect("adapter output");

    let artifact = ArtifactRecord::store_adapter_output(
        workspace.path(),
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        &output,
    )
    .expect("stored artifact");

    assert!(artifact.artifact_manifest_ref.ends_with("artifact.json"));
    handshake_core::storage::artifacts::validate_artifact_content_hash(
        workspace.path(),
        artifact.artifact_layer,
        artifact.artifact_uuid,
    )
    .expect("artifact manifest validates payload hash");
}

#[test]
fn trace_projection_replays_kernel_run_from_events_not_diagnostics() {
    let events = chained_kernel_events();

    let projection = TraceProjection::from_events("KTR-EXAMPLE", "SR-EXAMPLE", events)
        .expect("trace projection");

    assert_eq!(projection.kernel_task_run_id, "KTR-EXAMPLE");
    assert_eq!(projection.session_run_id, "SR-EXAMPLE");
    assert!(projection.contains_event_type(KernelEventType::PromotionDecided));
    assert_eq!(
        projection.authority_source, "postgres_event_ledger",
        "Flight Recorder and diagnostics must remain projections"
    );
}

#[test]
fn trace_projection_rejects_events_without_causation_chain_and_recomputed_payload_hash() {
    let mut events = chained_kernel_events();
    events[3].causation_id = None;
    events[5].payload_hash = "f".repeat(64);

    let err = TraceProjection::from_events("KTR-EXAMPLE", "SR-EXAMPLE", events)
        .expect_err("trace without causation/hash integrity must fail");

    assert!(
        err.to_string().contains("causation") || err.to_string().contains("payload_hash"),
        "expected causation or payload hash error, got {err}"
    );
}

#[test]
fn trace_projection_rejects_single_event_incomplete_kernel_trace() {
    let events = vec![kernel_event(
        KernelEventType::TaskIntentRecorded,
        json!({"intent": "partial run"}),
    )];

    let err = TraceProjection::from_events("KTR-EXAMPLE", "SR-EXAMPLE", events)
        .expect_err("single-event trace must be rejected");

    assert!(
        err.to_string().contains("incomplete trace"),
        "expected incomplete trace error, got {err}"
    );
}
