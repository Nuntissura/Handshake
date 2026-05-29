use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_FEMS_MT_HANDOFF_MEMORY_CONTEXT_STUB_ID: &str =
    "WP-1-FEMS-MT-Handoff-Memory-Context-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FemsMtHandoffReason {
    Escalation,
    Retry,
    RoleSwitch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FemsMtHandoffItemKind {
    MemoryPackItem,
    InsightCheckpoint,
    FailedAttempt,
    RecommendedProceduralItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMtHandoffMemoryItemV1 {
    pub item_id: String,
    pub kind: FemsMtHandoffItemKind,
    pub source_session_id: String,
    pub memory_ref: String,
    pub scope_refs: Vec<String>,
    pub provenance_ref: String,
    pub token_count: u32,
    pub base_score_x100: u8,
    #[serde(default)]
    pub pinned: bool,
    pub predecessor_recommended: bool,
    pub source_attempt_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMtFailedAttemptV1 {
    pub attempt_id: String,
    pub source_session_id: String,
    pub failure_summary: String,
    pub evidence_refs: Vec<String>,
    pub retryable: bool,
    pub score_penalty_x100: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMtHandoffMemoryContextV1 {
    pub schema_id: String,
    pub context_id: String,
    pub folded_stub_ids: Vec<String>,
    pub wp_id: String,
    pub mt_id: String,
    pub source_session_id: String,
    pub target_session_id: String,
    pub handoff_reason: FemsMtHandoffReason,
    pub carried_items: Vec<FemsMtHandoffMemoryItemV1>,
    pub failed_attempts: Vec<FemsMtFailedAttemptV1>,
    pub recommended_item_ids: Vec<String>,
    pub max_handoff_tokens: u32,
    pub fr_event_ref: String,
    pub locus_mt_iteration_ref: String,
    pub automatic_long_term_merge_allowed: bool,
    pub cross_wp_handoff_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsMtHandoffMemoryProjectionV1 {
    pub schema_id: String,
    pub context_id: String,
    pub selected_item_ids: Vec<String>,
    pub dropped_item_ids: Vec<String>,
    pub failed_attempt_ids: Vec<String>,
    pub recommended_item_ids: Vec<String>,
    pub boosted_item_ids: Vec<String>,
    pub deterministic_reduction_markers: Vec<String>,
    pub effective_handoff_tokens: u32,
    pub fr_event_ref: String,
    pub locus_mt_iteration_ref: String,
    pub mutates_long_term_memory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsMtHandoffMemoryContextValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_fems_mt_handoff_memory_context(
    context: &FemsMtHandoffMemoryContextV1,
) -> Result<(), Vec<FemsMtHandoffMemoryContextValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &context.schema_id);
    require_non_empty(&mut errors, "context_id", &context.context_id);
    require_non_empty(&mut errors, "wp_id", &context.wp_id);
    require_non_empty(&mut errors, "mt_id", &context.mt_id);
    require_non_empty(&mut errors, "source_session_id", &context.source_session_id);
    require_non_empty(&mut errors, "target_session_id", &context.target_session_id);
    require_vec(&mut errors, "folded_stub_ids", &context.folded_stub_ids);
    require_vec(&mut errors, "carried_items", &context.carried_items);
    require_vec(
        &mut errors,
        "recommended_item_ids",
        &context.recommended_item_ids,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &context.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &context.folded_source_refs,
    );

    if !contains_exact(
        &context.folded_stub_ids,
        FOLDED_FEMS_MT_HANDOFF_MEMORY_CONTEXT_STUB_ID,
    ) {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "folded_stub_ids",
            message: "handoff context must preserve the folded FEMS MT handoff memory stub id",
        });
    }
    if !contains_text(
        &context.folded_source_refs,
        FOLDED_FEMS_MT_HANDOFF_MEMORY_CONTEXT_STUB_ID,
    ) {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "folded_source_refs",
            message: "handoff context must preserve the folded source reference",
        });
    }
    if context.source_session_id == context.target_session_id {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "target_session_id",
            message: "handoff source and target sessions must be distinct",
        });
    }
    if context.cross_wp_handoff_allowed {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "cross_wp_handoff_allowed",
            message: "FEMS MT handoff context is intra-WP and intra-MT only",
        });
    }
    if context.automatic_long_term_merge_allowed {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "automatic_long_term_merge_allowed",
            message: "handoff context must not automatically merge into LongTermMemory",
        });
    }
    if context.max_handoff_tokens == 0 || context.max_handoff_tokens > 500 {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "max_handoff_tokens",
            message: "handoff context token budget must be configured and capped at <= 500",
        });
    }
    if !context.fr_event_ref.starts_with("FR-EVT-MEM-004") {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "fr_event_ref",
            message: "receiving session must record handoff provenance with FR-EVT-MEM-004",
        });
    }
    require_non_empty(
        &mut errors,
        "locus_mt_iteration_ref",
        &context.locus_mt_iteration_ref,
    );

    validate_authority_refs(&mut errors, context);
    validate_carried_items(&mut errors, context);
    validate_failed_attempts(&mut errors, context);
    validate_recommended_items(&mut errors, context);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_fems_mt_handoff_memory_context(
    context: &FemsMtHandoffMemoryContextV1,
) -> Result<FemsMtHandoffMemoryProjectionV1, Vec<FemsMtHandoffMemoryContextValidationError>> {
    validate_fems_mt_handoff_memory_context(context)?;

    let recommended_ids: HashSet<&str> = context
        .recommended_item_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut scored_items: Vec<(bool, u8, &FemsMtHandoffMemoryItemV1)> = context
        .carried_items
        .iter()
        .map(|item| {
            (
                item.pinned,
                adjusted_score_x100(item, &recommended_ids),
                item,
            )
        })
        .collect();
    scored_items.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.item_id.cmp(&right.2.item_id))
    });

    let mut selected_item_ids = Vec::new();
    let mut dropped_item_ids = Vec::new();
    let mut deterministic_reduction_markers = Vec::new();
    let mut effective_handoff_tokens = 0u32;

    for (_pinned, score, item) in scored_items {
        if effective_handoff_tokens.saturating_add(item.token_count) > context.max_handoff_tokens {
            dropped_item_ids.push(item.item_id.clone());
            deterministic_reduction_markers.push(format!(
                "TRUNCATED:{}:{}+{}>{}:score={}",
                item.item_id,
                effective_handoff_tokens,
                item.token_count,
                context.max_handoff_tokens,
                score
            ));
            continue;
        }

        selected_item_ids.push(item.item_id.clone());
        effective_handoff_tokens += item.token_count;
    }

    Ok(FemsMtHandoffMemoryProjectionV1 {
        schema_id: "hsk.kernel.fems_mt_handoff_memory_context_projection@1".to_string(),
        context_id: context.context_id.clone(),
        selected_item_ids,
        dropped_item_ids,
        failed_attempt_ids: context
            .failed_attempts
            .iter()
            .map(|attempt| attempt.attempt_id.clone())
            .collect(),
        recommended_item_ids: context.recommended_item_ids.clone(),
        boosted_item_ids: context
            .carried_items
            .iter()
            .filter(|item| {
                item.predecessor_recommended || recommended_ids.contains(item.item_id.as_str())
            })
            .map(|item| item.item_id.clone())
            .collect(),
        deterministic_reduction_markers,
        effective_handoff_tokens,
        fr_event_ref: context.fr_event_ref.clone(),
        locus_mt_iteration_ref: context.locus_mt_iteration_ref.clone(),
        mutates_long_term_memory: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<FemsMtHandoffMemoryContextValidationError>,
    context: &FemsMtHandoffMemoryContextV1,
) {
    for required_ref in [
        "ace.memory_pack",
        "flight_recorder.memory_handoff_context",
        "locus.mt_iteration",
        "kernel.fems_memory_poisoning_drift_guardrails",
    ] {
        if !contains_exact(&context.product_authority_refs, required_ref) {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "product_authority_refs",
                message: "handoff context must cite MemoryPack, Flight Recorder, Locus iteration, and memory guardrail authorities",
            });
        }
    }
}

fn validate_carried_items(
    errors: &mut Vec<FemsMtHandoffMemoryContextValidationError>,
    context: &FemsMtHandoffMemoryContextV1,
) {
    let mut item_ids = HashSet::new();
    let mut pinned_token_count = 0u32;
    for item in &context.carried_items {
        if item.pinned {
            pinned_token_count = pinned_token_count.saturating_add(item.token_count);
        }

        if !item_ids.insert(item.item_id.as_str()) {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "carried_items.item_id",
                message: "handoff carried item ids must be unique",
            });
        }

        require_non_empty(errors, "carried_items.item_id", &item.item_id);
        require_non_empty(
            errors,
            "carried_items.source_session_id",
            &item.source_session_id,
        );
        require_non_empty(errors, "carried_items.memory_ref", &item.memory_ref);
        require_vec(errors, "carried_items.scope_refs", &item.scope_refs);
        require_non_empty(errors, "carried_items.provenance_ref", &item.provenance_ref);

        if item.source_session_id != context.source_session_id {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "carried_items.source_session_id",
                message: "carried handoff items must preserve source_session_id provenance",
            });
        }
        if !item.scope_refs.iter().any(|scope| scope == &context.wp_id)
            || !item.scope_refs.iter().any(|scope| scope == &context.mt_id)
        {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "carried_items.scope_refs",
                message: "carried items must be scoped to the same WP and MT",
            });
        }
        if item.token_count == 0 {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "carried_items.token_count",
                message: "carried item token count must be greater than zero",
            });
        }
        if item.base_score_x100 == 0 || item.base_score_x100 > 100 {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "carried_items.base_score_x100",
                message: "carried item base score must be 1..=100",
            });
        }
        if item.kind == FemsMtHandoffItemKind::FailedAttempt && !item.source_attempt_failed {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "carried_items.source_attempt_failed",
                message: "failed-attempt handoff items must be marked as failed-source evidence",
            });
        }
    }

    if pinned_token_count > context.max_handoff_tokens {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "carried_items.pinned",
            message: "pinned handoff items must fit before scored handoff reduction",
        });
    }
}

fn validate_failed_attempts(
    errors: &mut Vec<FemsMtHandoffMemoryContextValidationError>,
    context: &FemsMtHandoffMemoryContextV1,
) {
    if matches!(
        context.handoff_reason,
        FemsMtHandoffReason::Escalation | FemsMtHandoffReason::Retry
    ) && context.failed_attempts.is_empty()
    {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field: "failed_attempts",
            message: "escalation and retry handoffs must carry failed-attempt context",
        });
    }

    let mut attempt_ids = HashSet::new();
    for attempt in &context.failed_attempts {
        if !attempt_ids.insert(attempt.attempt_id.as_str()) {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "failed_attempts.attempt_id",
                message: "failed attempt ids must be unique",
            });
        }

        require_non_empty(errors, "failed_attempts.attempt_id", &attempt.attempt_id);
        require_non_empty(
            errors,
            "failed_attempts.source_session_id",
            &attempt.source_session_id,
        );
        require_non_empty(
            errors,
            "failed_attempts.failure_summary",
            &attempt.failure_summary,
        );
        require_vec(
            errors,
            "failed_attempts.evidence_refs",
            &attempt.evidence_refs,
        );

        if attempt.source_session_id != context.source_session_id {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "failed_attempts.source_session_id",
                message: "failed-attempt context must preserve source_session_id provenance",
            });
        }
        if attempt.score_penalty_x100 > 100 {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "failed_attempts.score_penalty_x100",
                message: "failed-attempt penalty must be bounded to 0..=100",
            });
        }
    }
}

fn validate_recommended_items(
    errors: &mut Vec<FemsMtHandoffMemoryContextValidationError>,
    context: &FemsMtHandoffMemoryContextV1,
) {
    let items_by_id: HashMap<&str, &FemsMtHandoffMemoryItemV1> = context
        .carried_items
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect();
    let mut recommended_ids = HashSet::new();

    for item_id in &context.recommended_item_ids {
        if !recommended_ids.insert(item_id.as_str()) {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "recommended_item_ids",
                message: "recommended item ids must be unique",
            });
        }

        let Some(item) = items_by_id.get(item_id.as_str()) else {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "recommended_item_ids",
                message: "recommended item must reference a carried handoff item",
            });
            continue;
        };

        if item.kind != FemsMtHandoffItemKind::RecommendedProceduralItem
            || !item.predecessor_recommended
        {
            errors.push(FemsMtHandoffMemoryContextValidationError {
                field: "recommended_item_ids",
                message: "recommended items must be predecessor-recommended procedural items",
            });
        }
    }
}

fn adjusted_score_x100(item: &FemsMtHandoffMemoryItemV1, recommended_ids: &HashSet<&str>) -> u8 {
    let mut score = i16::from(item.base_score_x100);
    if item.predecessor_recommended || recommended_ids.contains(item.item_id.as_str()) {
        score += 20;
    }
    if item.source_attempt_failed {
        score -= 30;
    }
    score.clamp(0, 100) as u8
}

fn require_non_empty(
    errors: &mut Vec<FemsMtHandoffMemoryContextValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(FemsMtHandoffMemoryContextValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<FemsMtHandoffMemoryContextValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(FemsMtHandoffMemoryContextValidationError {
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
