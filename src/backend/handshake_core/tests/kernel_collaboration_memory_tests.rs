use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    fems_working_memory_checkpoint::{
        memory_extract_request_for_session_close, project_fems_working_memory_checkpoints,
        repeated_insight_promotions, validate_fems_working_memory_checkpoints,
        working_memory_gc_candidates, FemsWorkingMemoryCheckpointSchemaV1,
        WorkingMemoryCaptureSource, WorkingMemoryCheckpointKind, WorkingMemoryCheckpointV1,
    },
};

#[test]
fn kernel_collaboration_memory_validates_checkpoint_types_and_quality_gates() {
    let schema = sample_schema();

    validate_fems_working_memory_checkpoints(&schema).expect("checkpoint schema validates");

    let kinds: Vec<_> = schema
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.kind)
        .collect();
    assert!(kinds.contains(&WorkingMemoryCheckpointKind::SessionOpen));
    assert!(kinds.contains(&WorkingMemoryCheckpointKind::PreTask));
    assert!(kinds.contains(&WorkingMemoryCheckpointKind::Insight));
    assert!(kinds.contains(&WorkingMemoryCheckpointKind::TaskComplete));
    assert!(kinds.contains(&WorkingMemoryCheckpointKind::SessionClose));
    assert!(schema.action_stream_auto_populates_working_memory);
}

#[test]
fn kernel_collaboration_memory_session_close_builds_memory_extract_request() {
    let schema = sample_schema();
    let request = memory_extract_request_for_session_close(&schema, "session-a")
        .expect("SESSION_CLOSE should trigger memory extract");

    assert_eq!(request.protocol_id, "memory_extract_v0.1");
    assert_eq!(request.session_id, "session-a");
    assert_eq!(request.checkpoint_ids.len(), 5);
    assert!(request.memory_write_proposal_required);
    assert!(!request.mutates_durable_memory_authority);
}

#[test]
fn kernel_collaboration_memory_promotes_repeated_insights_and_gc_candidates() {
    let schema = sample_schema();

    let promotions = repeated_insight_promotions(&schema).expect("promotion candidates");
    assert_eq!(promotions.len(), 1);
    assert_eq!(promotions[0].insight_topic, "mailbox-claims-need-leases");
    assert_eq!(promotions[0].session_ids.len(), 3);
    assert!(promotions[0].feeds_hygiene_manager);

    let gc = working_memory_gc_candidates(&schema).expect("gc candidates");
    assert_eq!(gc.len(), 1);
    assert_eq!(gc[0].session_id, "session-d");
    assert_eq!(gc[0].reason, "completed_session_without_insights");
}

#[test]
fn kernel_collaboration_memory_projection_remains_read_only() {
    let schema = sample_schema();
    let projection =
        project_fems_working_memory_checkpoints(&schema).expect("projection should build");

    assert_eq!(projection.checkpoint_count, schema.checkpoints.len());
    assert_eq!(projection.required_kind_count, 5);
    assert_eq!(projection.promotion_candidate_count, 1);
    assert_eq!(projection.gc_candidate_count, 1);
    assert!(projection.session_close_triggers_memory_extract);
    assert!(!projection.mutates_durable_memory_authority);
}

#[test]
fn kernel_collaboration_memory_rejects_weak_quality_and_authority_mutation() {
    let mut schema = sample_schema();
    schema.durable_memory_authority_write_allowed = true;
    schema.repeated_insight_promotion_threshold = 2;
    schema.checkpoints[1].files_referenced.clear();
    schema.checkpoints[1].scope_refs.clear();
    schema.checkpoints[1].content = "short".to_string();
    schema.checkpoints[2].action_stream_refs.clear();

    let errors = validate_fems_working_memory_checkpoints(&schema)
        .expect_err("unsafe checkpoint schema must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "durable_memory_authority_write_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "repeated_insight_promotion_threshold"));
    assert!(errors
        .iter()
        .any(|error| error.field == "checkpoints.content"));
    assert!(errors
        .iter()
        .any(|error| error.field == "checkpoints.files_referenced"));
    assert!(errors
        .iter()
        .any(|error| error.field == "checkpoints.scope_refs"));
    assert!(errors
        .iter()
        .any(|error| error.field == "checkpoints.action_stream_refs"));
}

#[test]
fn kernel_collaboration_memory_action_catalog_exposes_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.fems_working_memory_checkpoint.project")
        .expect("FEMS working-memory checkpoint projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "fems_checkpoint_quality_gates"));
}

fn sample_schema() -> FemsWorkingMemoryCheckpointSchemaV1 {
    FemsWorkingMemoryCheckpointSchemaV1 {
        schema_id: "hsk.kernel.fems_working_memory_checkpoint@1".to_string(),
        schema_version: "1".to_string(),
        folded_stub_ids: vec!["WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1".to_string()],
        checkpoints: vec![
            checkpoint(
                "wm-open-a",
                "session-a",
                WorkingMemoryCheckpointKind::SessionOpen,
                WorkingMemoryCaptureSource::Explicit,
                "Session opens with intent to fold the FEMS checkpoint schema into the kernel.",
                None,
                1,
                false,
            ),
            checkpoint(
                "wm-pre-task-a",
                "session-a",
                WorkingMemoryCheckpointKind::PreTask,
                WorkingMemoryCaptureSource::Explicit,
                "Before implementation, assumptions and relevant files are captured for review.",
                None,
                2,
                false,
            ),
            checkpoint(
                "wm-insight-a",
                "session-a",
                WorkingMemoryCheckpointKind::Insight,
                WorkingMemoryCaptureSource::ActionStream,
                "Role Mailbox claim loops need explicit leases to prevent parallel worker overlap.",
                Some("mailbox-claims-need-leases"),
                3,
                true,
            ),
            checkpoint(
                "wm-task-complete-a",
                "session-a",
                WorkingMemoryCheckpointKind::TaskComplete,
                WorkingMemoryCaptureSource::Explicit,
                "Task completed with memory extract ready and durable authority still gated.",
                None,
                4,
                false,
            ),
            checkpoint(
                "wm-close-a",
                "session-a",
                WorkingMemoryCheckpointKind::SessionClose,
                WorkingMemoryCaptureSource::Explicit,
                "Session closes and queues memory_extract_v0.1 over the session checkpoint set.",
                None,
                5,
                false,
            ),
            checkpoint(
                "wm-insight-b",
                "session-b",
                WorkingMemoryCheckpointKind::Insight,
                WorkingMemoryCaptureSource::Explicit,
                "A separate session repeats that mailbox claim leases are required for safe handoff.",
                Some("mailbox-claims-need-leases"),
                1,
                true,
            ),
            checkpoint(
                "wm-insight-c",
                "session-c",
                WorkingMemoryCheckpointKind::Insight,
                WorkingMemoryCaptureSource::Explicit,
                "A third session confirms mailbox claim leases as a reusable procedural lesson.",
                Some("mailbox-claims-need-leases"),
                1,
                true,
            ),
            checkpoint(
                "wm-open-d",
                "session-d",
                WorkingMemoryCheckpointKind::SessionOpen,
                WorkingMemoryCaptureSource::Explicit,
                "Completed low-signal session opens with ordinary routing context only.",
                None,
                1,
                false,
            ),
            checkpoint(
                "wm-close-d",
                "session-d",
                WorkingMemoryCheckpointKind::SessionClose,
                WorkingMemoryCaptureSource::Explicit,
                "Completed low-signal session closes without insights and can be garbage-collected.",
                None,
                2,
                false,
            ),
        ],
        min_content_chars: 24,
        session_close_triggers_memory_extract: true,
        memory_extract_protocol_id: "memory_extract_v0.1".to_string(),
        repeated_insight_promotion_threshold: 3,
        gc_completed_sessions_without_insights: true,
        action_stream_auto_populates_working_memory: true,
        durable_memory_authority_write_allowed: false,
        product_authority_refs: vec![
            "kernel.software_delivery_runtime_truth".to_string(),
            "kernel.role_mailbox_loop_control".to_string(),
            "flight_recorder.memory_write_proposed".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1.contract.json"
                .to_string(),
        ],
    }
}

fn checkpoint(
    checkpoint_id: &str,
    session_id: &str,
    kind: WorkingMemoryCheckpointKind,
    capture_source: WorkingMemoryCaptureSource,
    content: &str,
    insight_topic: Option<&str>,
    sequence: u32,
    contains_insight: bool,
) -> WorkingMemoryCheckpointV1 {
    WorkingMemoryCheckpointV1 {
        checkpoint_id: checkpoint_id.to_string(),
        session_id: session_id.to_string(),
        kind,
        capture_source,
        sequence,
        content: content.to_string(),
        decisions: vec![
            "Keep durable memory writes behind MemoryWriteProposal review.".to_string(),
        ],
        files_referenced: vec![
            "src/backend/handshake_core/src/kernel/mod.rs".to_string(),
            "src/backend/handshake_core/src/kernel/action_catalog.rs".to_string(),
        ],
        scope_refs: vec![
            "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            "MT-033".to_string(),
        ],
        action_stream_refs: match capture_source {
            WorkingMemoryCaptureSource::ActionStream => vec![
                "tool://rg/fems-working-memory".to_string(),
                "tool://cargo/test-kernel-collaboration-memory".to_string(),
            ],
            WorkingMemoryCaptureSource::Explicit => Vec::new(),
        },
        insight_topic: insight_topic.map(str::to_string),
        contains_insight,
        completed_session: session_id == "session-d"
            || kind == WorkingMemoryCheckpointKind::SessionClose,
        pinned: false,
        created_at_utc: "2026-05-14T17:00:00Z".to_string(),
    }
}
