use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::session_spawn_tree_dcc::SessionSpawnMode;

pub const FOLDED_SESSION_SPAWN_CONVERSATION_DISTILLATION_STUB_ID: &str =
    "WP-1-Session-Spawn-Conversation-Distillation-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSpawnConversationPairV1 {
    pub pair_id: String,
    pub parent_session_id: String,
    pub child_session_id: String,
    pub parent_request_ref: String,
    pub child_summary_ref: String,
    pub raw_conversation_text_ref: String,
    pub raw_conversation_text_authoritative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnConversationDistillationMetadataV1 {
    pub depth: u32,
    pub parent_role: String,
    pub child_role: String,
    pub task_type: String,
    pub spawn_mode: SessionSpawnMode,
    pub spawn_record_ref: String,
    pub flight_recorder_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationArtifactV1 {
    pub artifact_id: String,
    pub pair_id: String,
    pub training_example_ref: String,
    pub teacher_signal_ref: String,
    pub student_solution_ref: String,
    pub write_box_schema_id: String,
    pub conversation_text_authority: bool,
    pub metadata: SpawnConversationDistillationMetadataV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSpawnConversationDistillationV1 {
    pub schema_id: String,
    pub distillation_id: String,
    pub folded_stub_ids: Vec<String>,
    pub pairs: Vec<SessionSpawnConversationPairV1>,
    pub artifacts: Vec<DistillationArtifactV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpawnConversationDistillationProjectionV1 {
    pub schema_id: String,
    pub distillation_id: String,
    pub pair_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
    pub request_refs: Vec<String>,
    pub summary_refs: Vec<String>,
    pub spawn_record_refs: Vec<String>,
    pub metadata_tags: Vec<String>,
    pub conversation_text_authority: bool,
    pub mutates_training_corpus: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpawnConversationDistillationValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_session_spawn_conversation_distillation(
    distillation: &SessionSpawnConversationDistillationV1,
) -> Result<(), Vec<SessionSpawnConversationDistillationValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &distillation.schema_id);
    require_non_empty(
        &mut errors,
        "distillation_id",
        &distillation.distillation_id,
    );
    require_vec(
        &mut errors,
        "folded_stub_ids",
        &distillation.folded_stub_ids,
    );
    require_vec(&mut errors, "pairs", &distillation.pairs);
    require_vec(&mut errors, "artifacts", &distillation.artifacts);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &distillation.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &distillation.folded_source_refs,
    );

    if !contains_exact(
        &distillation.folded_stub_ids,
        FOLDED_SESSION_SPAWN_CONVERSATION_DISTILLATION_STUB_ID,
    ) {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field: "folded_stub_ids",
            message: "spawn conversation distillation must preserve the folded stub id",
        });
    }
    if !contains_text(
        &distillation.folded_source_refs,
        FOLDED_SESSION_SPAWN_CONVERSATION_DISTILLATION_STUB_ID,
    ) {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field: "folded_source_refs",
            message: "spawn conversation distillation must preserve the folded source reference",
        });
    }

    validate_authority_refs(&mut errors, distillation);
    validate_pairs(&mut errors, &distillation.pairs);
    validate_artifacts(&mut errors, distillation);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_session_spawn_conversation_distillation(
    distillation: &SessionSpawnConversationDistillationV1,
) -> Result<
    SessionSpawnConversationDistillationProjectionV1,
    Vec<SessionSpawnConversationDistillationValidationError>,
> {
    validate_session_spawn_conversation_distillation(distillation)?;

    Ok(SessionSpawnConversationDistillationProjectionV1 {
        schema_id: "hsk.kernel.session_spawn_conversation_distillation_projection@1".to_string(),
        distillation_id: distillation.distillation_id.clone(),
        pair_ids: distillation
            .pairs
            .iter()
            .map(|pair| pair.pair_id.clone())
            .collect(),
        artifact_ids: distillation
            .artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.clone())
            .collect(),
        request_refs: distillation
            .pairs
            .iter()
            .map(|pair| pair.parent_request_ref.clone())
            .collect(),
        summary_refs: distillation
            .pairs
            .iter()
            .map(|pair| pair.child_summary_ref.clone())
            .collect(),
        spawn_record_refs: distillation
            .artifacts
            .iter()
            .map(|artifact| artifact.metadata.spawn_record_ref.clone())
            .collect(),
        metadata_tags: distillation
            .artifacts
            .iter()
            .map(|artifact| {
                format!(
                    "depth={};child_role={};task_type={}",
                    artifact.metadata.depth,
                    artifact.metadata.child_role,
                    artifact.metadata.task_type
                )
            })
            .collect(),
        conversation_text_authority: false,
        mutates_training_corpus: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<SessionSpawnConversationDistillationValidationError>,
    distillation: &SessionSpawnConversationDistillationV1,
) {
    for required_ref in [
        "kernel.session_spawn_tree_dcc",
        "kernel.fems_mt_handoff_memory_context",
        "kernel.action_catalog",
        "distillation.pipeline",
        "flight_recorder.session_spawn",
    ] {
        if !contains_exact(&distillation.product_authority_refs, required_ref) {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "product_authority_refs",
                message: "spawn conversation distillation must cite spawn tree, handoff memory, catalog, distillation, and session Flight Recorder authorities",
            });
        }
    }
}

fn validate_pairs(
    errors: &mut Vec<SessionSpawnConversationDistillationValidationError>,
    pairs: &[SessionSpawnConversationPairV1],
) {
    let mut pair_ids = HashSet::new();
    for pair in pairs {
        if !pair_ids.insert(pair.pair_id.as_str()) {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "pairs.pair_id",
                message: "spawn conversation pair ids must be unique",
            });
        }
        require_non_empty(errors, "pairs.pair_id", &pair.pair_id);
        require_non_empty(errors, "pairs.parent_session_id", &pair.parent_session_id);
        require_non_empty(errors, "pairs.child_session_id", &pair.child_session_id);
        require_non_empty(errors, "pairs.parent_request_ref", &pair.parent_request_ref);
        require_non_empty(errors, "pairs.child_summary_ref", &pair.child_summary_ref);
        require_non_empty(
            errors,
            "pairs.raw_conversation_text_ref",
            &pair.raw_conversation_text_ref,
        );

        if pair.parent_session_id == pair.child_session_id {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "pairs.child_session_id",
                message: "child session must differ from parent session",
            });
        }
        if !pair
            .parent_request_ref
            .starts_with("conversation-ref://parent-request/")
        {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "pairs.parent_request_ref",
                message: "parent requests must be referenced by typed conversation refs",
            });
        }
        if !pair
            .child_summary_ref
            .starts_with("summary-ref://child-session/")
        {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "pairs.child_summary_ref",
                message: "child summaries must be referenced by typed summary refs",
            });
        }
        if pair.raw_conversation_text_authoritative {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "pairs.raw_conversation_text_authoritative",
                message: "raw conversation text must never be authority",
            });
        }
    }
}

fn validate_artifacts(
    errors: &mut Vec<SessionSpawnConversationDistillationValidationError>,
    distillation: &SessionSpawnConversationDistillationV1,
) {
    let pair_ids: HashSet<&str> = distillation
        .pairs
        .iter()
        .map(|pair| pair.pair_id.as_str())
        .collect();
    let mut artifact_ids = HashSet::new();

    for artifact in &distillation.artifacts {
        if !artifact_ids.insert(artifact.artifact_id.as_str()) {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "artifacts.artifact_id",
                message: "distillation artifact ids must be unique",
            });
        }
        require_non_empty(errors, "artifacts.artifact_id", &artifact.artifact_id);
        require_non_empty(errors, "artifacts.pair_id", &artifact.pair_id);
        require_non_empty(
            errors,
            "artifacts.training_example_ref",
            &artifact.training_example_ref,
        );
        require_non_empty(
            errors,
            "artifacts.teacher_signal_ref",
            &artifact.teacher_signal_ref,
        );
        require_non_empty(
            errors,
            "artifacts.student_solution_ref",
            &artifact.student_solution_ref,
        );
        require_non_empty(
            errors,
            "artifacts.write_box_schema_id",
            &artifact.write_box_schema_id,
        );

        if !pair_ids.contains(artifact.pair_id.as_str()) {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "artifacts.pair_id",
                message: "distillation artifact must reference an existing pair",
            });
        }
        if !artifact
            .training_example_ref
            .starts_with("distillation-example://")
        {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "artifacts.training_example_ref",
                message: "training examples must be referenced, not inline text",
            });
        }
        if !artifact.write_box_schema_id.starts_with("hsk.write_box.") {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "artifacts.write_box_schema_id",
                message: "distillation artifacts must bind a Handshake write-box schema",
            });
        }
        if artifact.conversation_text_authority {
            errors.push(SessionSpawnConversationDistillationValidationError {
                field: "artifacts.conversation_text_authority",
                message: "conversation text must not become artifact authority",
            });
        }

        validate_metadata(errors, &artifact.metadata);
    }
}

fn validate_metadata(
    errors: &mut Vec<SessionSpawnConversationDistillationValidationError>,
    metadata: &SpawnConversationDistillationMetadataV1,
) {
    if metadata.depth == 0 {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field: "artifacts.metadata.depth",
            message: "distillation metadata must record non-root spawn depth",
        });
    }
    require_non_empty(
        errors,
        "artifacts.metadata.parent_role",
        &metadata.parent_role,
    );
    require_non_empty(
        errors,
        "artifacts.metadata.child_role",
        &metadata.child_role,
    );
    require_non_empty(errors, "artifacts.metadata.task_type", &metadata.task_type);
    require_non_empty(
        errors,
        "artifacts.metadata.spawn_record_ref",
        &metadata.spawn_record_ref,
    );
    require_non_empty(
        errors,
        "artifacts.metadata.flight_recorder_ref",
        &metadata.flight_recorder_ref,
    );
    if !metadata
        .spawn_record_ref
        .starts_with("runtime://session-spawn/")
    {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field: "artifacts.metadata.spawn_record_ref",
            message: "distillation metadata must cite runtime spawn records",
        });
    }
    if !metadata
        .flight_recorder_ref
        .starts_with("FR-EVT-SESSION-SPAWN-")
    {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field: "artifacts.metadata.flight_recorder_ref",
            message: "distillation metadata must cite session-spawn Flight Recorder events",
        });
    }
}

fn require_non_empty(
    errors: &mut Vec<SessionSpawnConversationDistillationValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<SessionSpawnConversationDistillationValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(SessionSpawnConversationDistillationValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
