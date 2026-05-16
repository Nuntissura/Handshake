use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    session_spawn_conversation_distillation::{
        project_session_spawn_conversation_distillation,
        validate_session_spawn_conversation_distillation, DistillationArtifactV1,
        SessionSpawnConversationDistillationV1, SessionSpawnConversationPairV1,
        SpawnConversationDistillationMetadataV1,
    },
    session_spawn_tree_dcc::SessionSpawnMode,
};

#[test]
fn kernel_spawn_conversation_distillation_projects_pairs_and_artifacts() {
    let distillation = sample_distillation();
    validate_session_spawn_conversation_distillation(&distillation)
        .expect("distillation validates");

    let projection =
        project_session_spawn_conversation_distillation(&distillation).expect("projection builds");

    assert_eq!(projection.distillation_id, "spawn-distillation-mt044");
    assert_eq!(projection.pair_ids.len(), 2);
    assert_eq!(projection.artifact_ids.len(), 2);
    assert!(projection
        .metadata_tags
        .contains(&"depth=1;child_role=CODER;task_type=implementation".to_string()));
    assert!(!projection.conversation_text_authority);
    assert!(!projection.mutates_training_corpus);
}

#[test]
fn kernel_spawn_conversation_distillation_preserves_refs_without_raw_text_authority() {
    let projection = project_session_spawn_conversation_distillation(&sample_distillation())
        .expect("projection");

    assert!(projection
        .request_refs
        .iter()
        .all(|request_ref| request_ref.starts_with("conversation-ref://parent-request/")));
    assert!(projection
        .summary_refs
        .iter()
        .all(|summary_ref| summary_ref.starts_with("summary-ref://child-session/")));
    assert!(projection
        .spawn_record_refs
        .iter()
        .all(|spawn_ref| spawn_ref.starts_with("runtime://session-spawn/")));
}

#[test]
fn kernel_spawn_conversation_distillation_rejects_authority_and_pair_drift() {
    let mut distillation = sample_distillation();
    distillation.pairs[0].child_session_id = distillation.pairs[0].parent_session_id.clone();
    distillation.pairs[0].parent_request_ref.clear();
    distillation.artifacts[0].conversation_text_authority = true;
    distillation.artifacts[1].pair_id = "pair.missing".to_string();
    distillation.artifacts[1].metadata.flight_recorder_ref = "FR-EVT-OTHER".to_string();

    let errors = validate_session_spawn_conversation_distillation(&distillation)
        .expect_err("unsafe distillation must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "pairs.child_session_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "pairs.parent_request_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "artifacts.conversation_text_authority"));
    assert!(errors
        .iter()
        .any(|error| error.field == "artifacts.pair_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "artifacts.metadata.flight_recorder_ref"));
}

#[test]
fn kernel_spawn_conversation_distillation_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.session_spawn_conversation_distillation.project")
        .expect("spawn conversation distillation projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "spawn_distillation_no_conversation_text_authority"));
}

fn sample_distillation() -> SessionSpawnConversationDistillationV1 {
    SessionSpawnConversationDistillationV1 {
        schema_id: "hsk.kernel.session_spawn_conversation_distillation@1".to_string(),
        distillation_id: "spawn-distillation-mt044".to_string(),
        folded_stub_ids: vec!["WP-1-Session-Spawn-Conversation-Distillation-v1".to_string()],
        pairs: vec![
            pair("pair.child.a", "session.parent", "session.child.a"),
            pair("pair.child.b", "session.parent", "session.child.b"),
        ],
        artifacts: vec![
            artifact("artifact.child.a", "pair.child.a", 1, "implementation"),
            artifact("artifact.child.b", "pair.child.b", 1, "validation"),
        ],
        product_authority_refs: vec![
            "kernel.session_spawn_tree_dcc".to_string(),
            "kernel.fems_mt_handoff_memory_context".to_string(),
            "kernel.action_catalog".to_string(),
            "distillation.pipeline".to_string(),
            "flight_recorder.session_spawn".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Session-Spawn-Conversation-Distillation-v1.contract.json"
                .to_string(),
        ],
    }
}

fn pair(
    pair_id: &str,
    parent_session_id: &str,
    child_session_id: &str,
) -> SessionSpawnConversationPairV1 {
    SessionSpawnConversationPairV1 {
        pair_id: pair_id.to_string(),
        parent_session_id: parent_session_id.to_string(),
        child_session_id: child_session_id.to_string(),
        parent_request_ref: format!("conversation-ref://parent-request/{pair_id}"),
        child_summary_ref: format!("summary-ref://child-session/{pair_id}"),
        raw_conversation_text_ref: format!("conversation-ref://raw/{pair_id}"),
        raw_conversation_text_authoritative: false,
    }
}

fn artifact(
    artifact_id: &str,
    pair_id: &str,
    depth: u32,
    task_type: &str,
) -> DistillationArtifactV1 {
    DistillationArtifactV1 {
        artifact_id: artifact_id.to_string(),
        pair_id: pair_id.to_string(),
        training_example_ref: format!("distillation-example://{artifact_id}"),
        teacher_signal_ref: format!("teacher-signal://{pair_id}"),
        student_solution_ref: format!("student-solution://{pair_id}"),
        write_box_schema_id: "hsk.write_box.distillation_artifact@1".to_string(),
        conversation_text_authority: false,
        metadata: SpawnConversationDistillationMetadataV1 {
            depth,
            parent_role: "ORCHESTRATOR".to_string(),
            child_role: "CODER".to_string(),
            task_type: task_type.to_string(),
            spawn_mode: SessionSpawnMode::SessionPersistent,
            spawn_record_ref: format!("runtime://session-spawn/{pair_id}"),
            flight_recorder_ref: format!("FR-EVT-SESSION-SPAWN-{}", pair_id.replace('.', "-")),
        },
    }
}
