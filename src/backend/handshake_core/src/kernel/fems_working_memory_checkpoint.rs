use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_FEMS_WORKING_MEMORY_CHECKPOINT_STUB_ID: &str =
    "WP-1-FEMS-Working-Memory-Checkpoint-Schema-v1";
pub const MEMORY_EXTRACT_PROTOCOL_V0_1: &str = "memory_extract_v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkingMemoryCheckpointKind {
    SessionOpen,
    PreTask,
    Insight,
    TaskComplete,
    SessionClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkingMemoryCaptureSource {
    Explicit,
    ActionStream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingMemoryCheckpointV1 {
    pub checkpoint_id: String,
    pub session_id: String,
    pub kind: WorkingMemoryCheckpointKind,
    pub capture_source: WorkingMemoryCaptureSource,
    pub sequence: u32,
    pub content: String,
    pub decisions: Vec<String>,
    pub files_referenced: Vec<String>,
    pub scope_refs: Vec<String>,
    pub action_stream_refs: Vec<String>,
    pub insight_topic: Option<String>,
    pub contains_insight: bool,
    pub completed_session: bool,
    pub pinned: bool,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsWorkingMemoryCheckpointSchemaV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub folded_stub_ids: Vec<String>,
    pub checkpoints: Vec<WorkingMemoryCheckpointV1>,
    pub min_content_chars: usize,
    pub session_close_triggers_memory_extract: bool,
    pub memory_extract_protocol_id: String,
    pub repeated_insight_promotion_threshold: usize,
    pub gc_completed_sessions_without_insights: bool,
    pub action_stream_auto_populates_working_memory: bool,
    pub durable_memory_authority_write_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryExtractJobRequestV1 {
    pub schema_id: String,
    pub protocol_id: String,
    pub session_id: String,
    pub checkpoint_ids: Vec<String>,
    pub source_checkpoint_count: usize,
    pub memory_write_proposal_required: bool,
    pub mutates_durable_memory_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsightPromotionCandidateV1 {
    pub schema_id: String,
    pub insight_topic: String,
    pub session_ids: Vec<String>,
    pub checkpoint_ids: Vec<String>,
    pub occurrence_count: usize,
    pub feeds_hygiene_manager: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingMemoryGcCandidateV1 {
    pub schema_id: String,
    pub session_id: String,
    pub checkpoint_ids: Vec<String>,
    pub reason: String,
    pub within_configurable_window: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsWorkingMemoryCheckpointProjectionV1 {
    pub schema_id: String,
    pub checkpoint_count: usize,
    pub required_kind_count: usize,
    pub session_close_triggers_memory_extract: bool,
    pub memory_extract_protocol_id: String,
    pub promotion_candidate_count: usize,
    pub gc_candidate_count: usize,
    pub action_stream_capture_enabled: bool,
    pub mutates_durable_memory_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsWorkingMemoryCheckpointValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_fems_working_memory_checkpoints(
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
) -> Result<(), Vec<FemsWorkingMemoryCheckpointValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &schema.schema_id);
    require_non_empty(&mut errors, "schema_version", &schema.schema_version);
    require_vec(&mut errors, "folded_stub_ids", &schema.folded_stub_ids);
    require_vec(&mut errors, "checkpoints", &schema.checkpoints);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &schema.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &schema.folded_source_refs,
    );

    if !contains_exact(
        &schema.folded_stub_ids,
        FOLDED_FEMS_WORKING_MEMORY_CHECKPOINT_STUB_ID,
    ) {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "folded_stub_ids",
            message: "schema must preserve the folded FEMS working-memory checkpoint stub id",
        });
    }
    if !contains_text(
        &schema.folded_source_refs,
        FOLDED_FEMS_WORKING_MEMORY_CHECKPOINT_STUB_ID,
    ) {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "folded_source_refs",
            message: "schema must preserve the folded source reference",
        });
    }
    if schema.min_content_chars == 0 {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "min_content_chars",
            message: "minimum checkpoint content length must be configured",
        });
    }
    if !schema.session_close_triggers_memory_extract {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "session_close_triggers_memory_extract",
            message: "SESSION_CLOSE checkpoints must trigger memory extraction",
        });
    }
    if schema.memory_extract_protocol_id != MEMORY_EXTRACT_PROTOCOL_V0_1 {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "memory_extract_protocol_id",
            message: "SESSION_CLOSE bridge must use memory_extract_v0.1",
        });
    }
    if schema.repeated_insight_promotion_threshold < 3 {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "repeated_insight_promotion_threshold",
            message: "repeated insight promotion requires at least three sessions",
        });
    }
    if !schema.gc_completed_sessions_without_insights {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "gc_completed_sessions_without_insights",
            message: "completed sessions without insights must remain GC candidates",
        });
    }
    if !schema.action_stream_auto_populates_working_memory {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "action_stream_auto_populates_working_memory",
            message: "working memory must support action-stream capture",
        });
    }
    if schema.durable_memory_authority_write_allowed {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field: "durable_memory_authority_write_allowed",
            message: "checkpoint projection must not directly write durable memory authority",
        });
    }

    validate_authority_refs(&mut errors, schema);
    validate_required_kind_coverage(&mut errors, &schema.checkpoints);
    validate_checkpoints(&mut errors, schema);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn memory_extract_request_for_session_close(
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
    session_id: &str,
) -> Result<MemoryExtractJobRequestV1, Vec<FemsWorkingMemoryCheckpointValidationError>> {
    validate_fems_working_memory_checkpoints(schema)?;

    let session_checkpoints = session_checkpoints(schema, session_id);
    if session_checkpoints.is_empty() {
        return Err(vec![FemsWorkingMemoryCheckpointValidationError {
            field: "session_id",
            message: "requested session has no working-memory checkpoints",
        }]);
    }
    if !session_checkpoints
        .iter()
        .any(|checkpoint| checkpoint.kind == WorkingMemoryCheckpointKind::SessionClose)
    {
        return Err(vec![FemsWorkingMemoryCheckpointValidationError {
            field: "checkpoints.kind",
            message: "requested session has no SESSION_CLOSE checkpoint",
        }]);
    }

    Ok(MemoryExtractJobRequestV1 {
        schema_id: "hsk.kernel.fems_memory_extract_request@1".to_string(),
        protocol_id: schema.memory_extract_protocol_id.clone(),
        session_id: session_id.to_string(),
        checkpoint_ids: session_checkpoints
            .iter()
            .map(|checkpoint| checkpoint.checkpoint_id.clone())
            .collect(),
        source_checkpoint_count: session_checkpoints.len(),
        memory_write_proposal_required: true,
        mutates_durable_memory_authority: false,
    })
}

pub fn repeated_insight_promotions(
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
) -> Result<Vec<InsightPromotionCandidateV1>, Vec<FemsWorkingMemoryCheckpointValidationError>> {
    validate_fems_working_memory_checkpoints(schema)?;

    let mut grouped: BTreeMap<String, Vec<&WorkingMemoryCheckpointV1>> = BTreeMap::new();
    for checkpoint in schema
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.kind == WorkingMemoryCheckpointKind::Insight)
    {
        if let Some(topic) = checkpoint.insight_topic.as_deref() {
            grouped
                .entry(normalize_topic(topic))
                .or_default()
                .push(checkpoint);
        }
    }

    let mut candidates = Vec::new();
    for (topic, checkpoints) in grouped {
        let session_ids = unique_sorted(
            checkpoints
                .iter()
                .map(|checkpoint| checkpoint.session_id.as_str()),
        );
        if session_ids.len() >= schema.repeated_insight_promotion_threshold {
            candidates.push(InsightPromotionCandidateV1 {
                schema_id: "hsk.kernel.fems_repeated_insight_candidate@1".to_string(),
                insight_topic: topic,
                session_ids,
                checkpoint_ids: checkpoints
                    .iter()
                    .map(|checkpoint| checkpoint.checkpoint_id.clone())
                    .collect(),
                occurrence_count: checkpoints.len(),
                feeds_hygiene_manager: true,
            });
        }
    }

    Ok(candidates)
}

pub fn working_memory_gc_candidates(
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
) -> Result<Vec<WorkingMemoryGcCandidateV1>, Vec<FemsWorkingMemoryCheckpointValidationError>> {
    validate_fems_working_memory_checkpoints(schema)?;

    let mut by_session: BTreeMap<&str, Vec<&WorkingMemoryCheckpointV1>> = BTreeMap::new();
    for checkpoint in &schema.checkpoints {
        by_session
            .entry(checkpoint.session_id.as_str())
            .or_default()
            .push(checkpoint);
    }

    let mut candidates = Vec::new();
    for (session_id, checkpoints) in by_session {
        let session_completed = checkpoints
            .iter()
            .all(|checkpoint| checkpoint.completed_session);
        let has_insight = checkpoints.iter().any(|checkpoint| {
            checkpoint.kind == WorkingMemoryCheckpointKind::Insight || checkpoint.contains_insight
        });
        let pinned = checkpoints.iter().any(|checkpoint| checkpoint.pinned);

        if session_completed && !has_insight && !pinned {
            candidates.push(WorkingMemoryGcCandidateV1 {
                schema_id: "hsk.kernel.fems_working_memory_gc_candidate@1".to_string(),
                session_id: session_id.to_string(),
                checkpoint_ids: checkpoints
                    .iter()
                    .map(|checkpoint| checkpoint.checkpoint_id.clone())
                    .collect(),
                reason: "completed_session_without_insights".to_string(),
                within_configurable_window: true,
            });
        }
    }

    Ok(candidates)
}

pub fn project_fems_working_memory_checkpoints(
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
) -> Result<FemsWorkingMemoryCheckpointProjectionV1, Vec<FemsWorkingMemoryCheckpointValidationError>>
{
    validate_fems_working_memory_checkpoints(schema)?;
    let promotions = repeated_insight_promotions(schema)?;
    let gc_candidates = working_memory_gc_candidates(schema)?;

    Ok(FemsWorkingMemoryCheckpointProjectionV1 {
        schema_id: "hsk.kernel.fems_working_memory_checkpoint_projection@1".to_string(),
        checkpoint_count: schema.checkpoints.len(),
        required_kind_count: required_checkpoint_kinds()
            .iter()
            .filter(|kind| {
                schema
                    .checkpoints
                    .iter()
                    .any(|checkpoint| checkpoint.kind == **kind)
            })
            .count(),
        session_close_triggers_memory_extract: schema.session_close_triggers_memory_extract,
        memory_extract_protocol_id: schema.memory_extract_protocol_id.clone(),
        promotion_candidate_count: promotions.len(),
        gc_candidate_count: gc_candidates.len(),
        action_stream_capture_enabled: schema.action_stream_auto_populates_working_memory
            && schema.checkpoints.iter().any(|checkpoint| {
                checkpoint.capture_source == WorkingMemoryCaptureSource::ActionStream
            }),
        mutates_durable_memory_authority: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<FemsWorkingMemoryCheckpointValidationError>,
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
) {
    for required_ref in [
        "kernel.software_delivery_runtime_truth",
        "kernel.role_mailbox_loop_control",
        "flight_recorder.memory_write_proposed",
    ] {
        if !contains_exact(&schema.product_authority_refs, required_ref) {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "product_authority_refs",
                message: "FEMS checkpoint schema must cite runtime truth, loop checkpoints, and Flight Recorder memory proposal refs",
            });
        }
    }
}

fn validate_required_kind_coverage(
    errors: &mut Vec<FemsWorkingMemoryCheckpointValidationError>,
    checkpoints: &[WorkingMemoryCheckpointV1],
) {
    for required_kind in required_checkpoint_kinds() {
        if !checkpoints
            .iter()
            .any(|checkpoint| checkpoint.kind == required_kind)
        {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.kind",
                message: "schema must include all five FEMS working-memory checkpoint kinds",
            });
        }
    }
}

fn validate_checkpoints(
    errors: &mut Vec<FemsWorkingMemoryCheckpointValidationError>,
    schema: &FemsWorkingMemoryCheckpointSchemaV1,
) {
    let mut checkpoint_ids = HashSet::new();
    let mut sequences_by_session: HashMap<&str, HashSet<u32>> = HashMap::new();

    for checkpoint in &schema.checkpoints {
        if !checkpoint_ids.insert(checkpoint.checkpoint_id.as_str()) {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.checkpoint_id",
                message: "checkpoint ids must be unique",
            });
        }
        if !sequences_by_session
            .entry(checkpoint.session_id.as_str())
            .or_default()
            .insert(checkpoint.sequence)
        {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.sequence",
                message: "checkpoint sequence must be unique per session",
            });
        }

        require_non_empty(
            errors,
            "checkpoints.checkpoint_id",
            &checkpoint.checkpoint_id,
        );
        require_non_empty(errors, "checkpoints.session_id", &checkpoint.session_id);
        require_non_empty(errors, "checkpoints.content", &checkpoint.content);
        require_non_empty(
            errors,
            "checkpoints.created_at_utc",
            &checkpoint.created_at_utc,
        );
        require_vec(errors, "checkpoints.decisions", &checkpoint.decisions);
        require_vec(
            errors,
            "checkpoints.files_referenced",
            &checkpoint.files_referenced,
        );
        require_vec(errors, "checkpoints.scope_refs", &checkpoint.scope_refs);

        if checkpoint.sequence == 0 {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.sequence",
                message: "checkpoint sequence must be greater than zero",
            });
        }
        if checkpoint.content.chars().count() < schema.min_content_chars {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.content",
                message: "checkpoint content is below the configured quality gate",
            });
        }
        if checkpoint.capture_source == WorkingMemoryCaptureSource::ActionStream
            && checkpoint.action_stream_refs.is_empty()
        {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.action_stream_refs",
                message: "action-stream checkpoints must cite captured actions",
            });
        }
        if checkpoint.kind == WorkingMemoryCheckpointKind::Insight
            && checkpoint
                .insight_topic
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.insight_topic",
                message: "INSIGHT checkpoints must declare a promotion topic",
            });
        }
        if checkpoint.kind != WorkingMemoryCheckpointKind::Insight && checkpoint.contains_insight {
            errors.push(FemsWorkingMemoryCheckpointValidationError {
                field: "checkpoints.contains_insight",
                message: "only INSIGHT checkpoints may be marked as insight-bearing",
            });
        }
    }
}

fn session_checkpoints<'a>(
    schema: &'a FemsWorkingMemoryCheckpointSchemaV1,
    session_id: &str,
) -> Vec<&'a WorkingMemoryCheckpointV1> {
    let mut checkpoints: Vec<_> = schema
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.session_id == session_id)
        .collect();
    checkpoints.sort_by_key(|checkpoint| checkpoint.sequence);
    checkpoints
}

fn required_checkpoint_kinds() -> [WorkingMemoryCheckpointKind; 5] {
    [
        WorkingMemoryCheckpointKind::SessionOpen,
        WorkingMemoryCheckpointKind::PreTask,
        WorkingMemoryCheckpointKind::Insight,
        WorkingMemoryCheckpointKind::TaskComplete,
        WorkingMemoryCheckpointKind::SessionClose,
    ]
}

fn normalize_topic(topic: &str) -> String {
    topic.trim().to_ascii_lowercase()
}

fn unique_sorted<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut set = HashSet::new();
    let mut unique = Vec::new();
    for value in values {
        if set.insert(value) {
            unique.push(value.to_string());
        }
    }
    unique.sort();
    unique
}

fn require_non_empty(
    errors: &mut Vec<FemsWorkingMemoryCheckpointValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<FemsWorkingMemoryCheckpointValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(FemsWorkingMemoryCheckpointValidationError {
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
