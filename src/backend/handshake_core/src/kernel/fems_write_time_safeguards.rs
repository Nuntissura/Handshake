use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_FEMS_WRITE_TIME_SAFEGUARDS_STUB_ID: &str = "WP-1-FEMS-Write-Time-Safeguards-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FemsResetStoragePrimitive {
    Postgres,
    EventLedger,
    CrdtSearchIndex,
    LegacyLocalStore,
    LegacyFts5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FemsMemoryItemStatus {
    Active,
    Superseded,
    Conflicted,
    FlaggedStale,
    Tombstoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMemoryWriteProposalV1 {
    pub proposal_id: String,
    pub memory_class: String,
    pub memory_type: String,
    pub scope_refs: Vec<String>,
    pub summary: String,
    pub summary_hash: String,
    pub content_hash: String,
    pub importance_x100: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsExistingMemoryItemV1 {
    pub memory_id: String,
    pub memory_class: String,
    pub memory_type: String,
    pub scope_refs: Vec<String>,
    pub summary_hash: String,
    pub status: FemsMemoryItemStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMemoryComparisonFactV1 {
    pub proposal_id: String,
    pub existing_memory_id: String,
    pub same_scope: bool,
    pub exact_key_match: bool,
    pub similarity_x100: u8,
    pub contradiction_detected: bool,
    pub generated_by_reset_approved_search: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsScopeResolutionV1 {
    pub scope_ref: String,
    pub resolves: bool,
    pub checked_with_primitive: FemsResetStoragePrimitive,
    pub state_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsWriteTimeSafeguardConfigV1 {
    pub mechanical_no_llm: bool,
    pub novelty_similarity_threshold_x100: u8,
    pub novelty_penalty_multiplier_x100: u8,
    pub max_latency_ms: u32,
    pub storage_primitives: Vec<FemsResetStoragePrimitive>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsAuditTrailConfigV1 {
    pub jsonl_enabled: bool,
    pub audit_trail_ref: String,
    pub flight_recorder_event_ref: String,
    pub event_ledger_ref: String,
    pub debug_bundle_exportable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsWriteTimeSafeguardsV1 {
    pub schema_id: String,
    pub safeguard_id: String,
    pub folded_stub_ids: Vec<String>,
    pub proposals: Vec<FemsMemoryWriteProposalV1>,
    pub existing_items: Vec<FemsExistingMemoryItemV1>,
    pub comparison_facts: Vec<FemsMemoryComparisonFactV1>,
    pub scope_resolutions: Vec<FemsScopeResolutionV1>,
    pub config: FemsWriteTimeSafeguardConfigV1,
    pub audit: FemsAuditTrailConfigV1,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsProposalSafeguardOutcomeV1 {
    pub proposal_id: String,
    pub skip_write: bool,
    pub novelty_penalty_multiplier_x100: Option<u8>,
    pub supersedes_memory_ids: Vec<String>,
    pub contradicted_memory_ids: Vec<String>,
    pub stale_scope_refs: Vec<String>,
    pub commit_report_warnings: Vec<String>,
    pub audit_event_ref: String,
    pub mechanical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsWriteTimeSafeguardReportV1 {
    pub schema_id: String,
    pub safeguard_id: String,
    pub proposal_outcomes: Vec<FemsProposalSafeguardOutcomeV1>,
    pub superseded_memory_ids: Vec<String>,
    pub conflicted_memory_ids: Vec<String>,
    pub dcc_conflict_queue_refs: Vec<String>,
    pub memory_pack_exclusion_ids: Vec<String>,
    pub audit_event_refs: Vec<String>,
    pub warnings: Vec<String>,
    pub uses_llm_calls: bool,
    pub uses_legacy_search_authority: bool,
    pub authoritative_storage_primitives: Vec<FemsResetStoragePrimitive>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsWriteTimeSafeguardValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_fems_write_time_safeguards(
    safeguards: &FemsWriteTimeSafeguardsV1,
) -> Result<(), Vec<FemsWriteTimeSafeguardValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &safeguards.schema_id);
    require_non_empty(&mut errors, "safeguard_id", &safeguards.safeguard_id);
    require_vec(&mut errors, "folded_stub_ids", &safeguards.folded_stub_ids);
    require_vec(&mut errors, "proposals", &safeguards.proposals);
    require_vec(
        &mut errors,
        "scope_resolutions",
        &safeguards.scope_resolutions,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &safeguards.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &safeguards.folded_source_refs,
    );

    if !contains_exact(
        &safeguards.folded_stub_ids,
        FOLDED_FEMS_WRITE_TIME_SAFEGUARDS_STUB_ID,
    ) {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "folded_stub_ids",
            message: "safeguards must preserve the folded FEMS write-time safeguards stub id",
        });
    }
    if !contains_text(
        &safeguards.folded_source_refs,
        FOLDED_FEMS_WRITE_TIME_SAFEGUARDS_STUB_ID,
    ) {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "folded_source_refs",
            message: "safeguards must preserve the folded source reference",
        });
    }

    validate_config(&mut errors, &safeguards.config);
    validate_audit(&mut errors, &safeguards.audit);
    validate_authority_refs(&mut errors, safeguards);
    validate_proposals(&mut errors, safeguards);
    validate_existing_items(&mut errors, &safeguards.existing_items);
    validate_scope_resolutions(&mut errors, safeguards);
    validate_comparison_facts(&mut errors, safeguards);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn evaluate_fems_write_time_safeguards(
    safeguards: &FemsWriteTimeSafeguardsV1,
) -> Result<FemsWriteTimeSafeguardReportV1, Vec<FemsWriteTimeSafeguardValidationError>> {
    validate_fems_write_time_safeguards(safeguards)?;

    let existing_by_id: HashMap<&str, &FemsExistingMemoryItemV1> = safeguards
        .existing_items
        .iter()
        .map(|item| (item.memory_id.as_str(), item))
        .collect();
    let comparisons_by_proposal = comparisons_by_proposal(&safeguards.comparison_facts);
    let scope_resolution_by_ref: HashMap<&str, &FemsScopeResolutionV1> = safeguards
        .scope_resolutions
        .iter()
        .map(|resolution| (resolution.scope_ref.as_str(), resolution))
        .collect();

    let mut outcomes = Vec::new();
    let mut superseded_memory_ids = Vec::new();
    let mut conflicted_memory_ids = Vec::new();
    let mut dcc_conflict_queue_refs = Vec::new();
    let mut memory_pack_exclusion_ids = Vec::new();
    let mut audit_event_refs = Vec::new();
    let mut warnings = Vec::new();

    for proposal in &safeguards.proposals {
        let mut outcome = FemsProposalSafeguardOutcomeV1 {
            proposal_id: proposal.proposal_id.clone(),
            skip_write: false,
            novelty_penalty_multiplier_x100: None,
            supersedes_memory_ids: Vec::new(),
            contradicted_memory_ids: Vec::new(),
            stale_scope_refs: Vec::new(),
            commit_report_warnings: Vec::new(),
            audit_event_ref: format!(
                "{}/{}.jsonl",
                safeguards.audit.audit_trail_ref.trim_end_matches('/'),
                proposal.proposal_id
            ),
            mechanical: true,
        };

        for scope_ref in &proposal.scope_refs {
            if let Some(resolution) = scope_resolution_by_ref.get(scope_ref.as_str()) {
                if !resolution.resolves {
                    outcome.stale_scope_refs.push(scope_ref.clone());
                    outcome.commit_report_warnings.push(format!(
                        "state validation flagged stale scope_ref {scope_ref}"
                    ));
                }
            }
        }

        let comparison_facts = comparisons_by_proposal
            .get(proposal.proposal_id.as_str())
            .cloned()
            .unwrap_or_default();

        for comparison in comparison_facts {
            let Some(existing) = existing_by_id.get(comparison.existing_memory_id.as_str()) else {
                continue;
            };

            if is_duplicate(proposal, existing, comparison) {
                outcome.skip_write = true;
                outcome.commit_report_warnings.push(format!(
                    "duplicate write skipped for existing memory {}",
                    existing.memory_id
                ));
                continue;
            }

            if comparison.same_scope && comparison.contradiction_detected {
                push_unique(&mut outcome.contradicted_memory_ids, &existing.memory_id);
                push_unique(&mut conflicted_memory_ids, &existing.memory_id);
                let queue_ref = format!(
                    "dcc-conflict://{}--{}",
                    proposal.proposal_id, existing.memory_id
                );
                push_unique(&mut dcc_conflict_queue_refs, &queue_ref);
                outcome.commit_report_warnings.push(format!(
                    "contradiction routed to DCC conflict queue for {}",
                    existing.memory_id
                ));
                continue;
            }

            if should_apply_novelty_penalty(proposal, comparison, &safeguards.config) {
                outcome.novelty_penalty_multiplier_x100 =
                    Some(safeguards.config.novelty_penalty_multiplier_x100);
                outcome.commit_report_warnings.push(format!(
                    "novelty penalty applied from similarity {}",
                    comparison.similarity_x100
                ));
            }

            if should_supersede(proposal, existing, comparison) {
                push_unique(&mut outcome.supersedes_memory_ids, &existing.memory_id);
                push_unique(&mut superseded_memory_ids, &existing.memory_id);
                push_unique(&mut memory_pack_exclusion_ids, &existing.memory_id);
                outcome.commit_report_warnings.push(format!(
                    "procedural item {} superseded and excluded from MemoryPack",
                    existing.memory_id
                ));
            }
        }

        if outcome.commit_report_warnings.is_empty() {
            outcome
                .commit_report_warnings
                .push("write accepted by mechanical safeguards".to_string());
        }

        audit_event_refs.push(outcome.audit_event_ref.clone());
        warnings.extend(outcome.commit_report_warnings.iter().cloned());
        outcomes.push(outcome);
    }

    Ok(FemsWriteTimeSafeguardReportV1 {
        schema_id: "hsk.kernel.fems_write_time_safeguard_report@1".to_string(),
        safeguard_id: safeguards.safeguard_id.clone(),
        proposal_outcomes: outcomes,
        superseded_memory_ids,
        conflicted_memory_ids,
        dcc_conflict_queue_refs,
        memory_pack_exclusion_ids,
        audit_event_refs,
        warnings,
        uses_llm_calls: false,
        uses_legacy_search_authority: false,
        authoritative_storage_primitives: safeguards.config.storage_primitives.clone(),
    })
}

fn validate_config(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    config: &FemsWriteTimeSafeguardConfigV1,
) {
    if !config.mechanical_no_llm {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "config.mechanical_no_llm",
            message: "write-time safeguards must execute without LLM calls",
        });
    }
    if config.max_latency_ms > 10 {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "config.max_latency_ms",
            message: "mechanical write-time safeguards must stay within the 10ms target",
        });
    }
    if config.novelty_similarity_threshold_x100 == 0 {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "config.novelty_similarity_threshold_x100",
            message: "novelty threshold must be configured",
        });
    }
    if config.novelty_penalty_multiplier_x100 != 30 {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "config.novelty_penalty_multiplier_x100",
            message: "near duplicates must receive the folded 0.3x importance penalty",
        });
    }
    if !config
        .storage_primitives
        .contains(&FemsResetStoragePrimitive::Postgres)
        || !config
            .storage_primitives
            .contains(&FemsResetStoragePrimitive::EventLedger)
        || !config
            .storage_primitives
            .contains(&FemsResetStoragePrimitive::CrdtSearchIndex)
        || config.storage_primitives.iter().any(|primitive| {
            matches!(
                primitive,
                FemsResetStoragePrimitive::LegacyLocalStore | FemsResetStoragePrimitive::LegacyFts5
            )
        })
    {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "config.storage_primitives",
            message:
                "write-time safeguards must use Postgres/EventLedger/CRDT search, not legacy local/FTS authority",
        });
    }
}

fn validate_audit(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    audit: &FemsAuditTrailConfigV1,
) {
    if !audit.jsonl_enabled {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "audit.jsonl_enabled",
            message: "JSONL audit trail must be enabled",
        });
    }
    if !audit.debug_bundle_exportable {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field: "audit.debug_bundle_exportable",
            message: "audit trail must be exportable into debug bundles",
        });
    }
    require_non_empty(errors, "audit.audit_trail_ref", &audit.audit_trail_ref);
    require_non_empty(
        errors,
        "audit.flight_recorder_event_ref",
        &audit.flight_recorder_event_ref,
    );
    require_non_empty(errors, "audit.event_ledger_ref", &audit.event_ledger_ref);
}

fn validate_authority_refs(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    safeguards: &FemsWriteTimeSafeguardsV1,
) {
    for required_ref in [
        "kernel.reset_invariants",
        "flight_recorder.memory_item_status_changed",
        "kernel.fems_working_memory_checkpoint",
    ] {
        if !contains_exact(&safeguards.product_authority_refs, required_ref) {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "product_authority_refs",
                message: "FEMS safeguards must cite reset invariants, Flight Recorder memory status events, and working-memory checkpoints",
            });
        }
    }
}

fn validate_proposals(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    safeguards: &FemsWriteTimeSafeguardsV1,
) {
    let mut proposal_ids = HashSet::new();
    for proposal in &safeguards.proposals {
        if !proposal_ids.insert(proposal.proposal_id.as_str()) {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "proposals.proposal_id",
                message: "proposal ids must be unique",
            });
        }
        require_non_empty(errors, "proposals.proposal_id", &proposal.proposal_id);
        require_non_empty(errors, "proposals.memory_class", &proposal.memory_class);
        require_non_empty(errors, "proposals.memory_type", &proposal.memory_type);
        require_vec(errors, "proposals.scope_refs", &proposal.scope_refs);
        require_non_empty(errors, "proposals.summary", &proposal.summary);
        require_non_empty(errors, "proposals.summary_hash", &proposal.summary_hash);
        require_non_empty(errors, "proposals.content_hash", &proposal.content_hash);
        if proposal.importance_x100 == 0 || proposal.importance_x100 > 100 {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "proposals.importance_x100",
                message: "proposal importance must be 1..=100",
            });
        }
    }
}

fn validate_existing_items(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    existing_items: &[FemsExistingMemoryItemV1],
) {
    let mut memory_ids = HashSet::new();
    for item in existing_items {
        if !memory_ids.insert(item.memory_id.as_str()) {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "existing_items.memory_id",
                message: "existing memory ids must be unique",
            });
        }
        require_non_empty(errors, "existing_items.memory_id", &item.memory_id);
        require_non_empty(errors, "existing_items.memory_class", &item.memory_class);
        require_non_empty(errors, "existing_items.memory_type", &item.memory_type);
        require_vec(errors, "existing_items.scope_refs", &item.scope_refs);
        require_non_empty(errors, "existing_items.summary_hash", &item.summary_hash);
    }
}

fn validate_scope_resolutions(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    safeguards: &FemsWriteTimeSafeguardsV1,
) {
    let known_scope_refs: HashSet<&str> = safeguards
        .scope_resolutions
        .iter()
        .map(|resolution| resolution.scope_ref.as_str())
        .collect();
    for proposal in &safeguards.proposals {
        for scope_ref in &proposal.scope_refs {
            if !known_scope_refs.contains(scope_ref.as_str()) {
                errors.push(FemsWriteTimeSafeguardValidationError {
                    field: "scope_resolutions.scope_ref",
                    message: "every proposal scope_ref must have a state validation record",
                });
            }
        }
    }
    for resolution in &safeguards.scope_resolutions {
        require_non_empty(errors, "scope_resolutions.scope_ref", &resolution.scope_ref);
        require_non_empty(errors, "scope_resolutions.state_ref", &resolution.state_ref);
        if matches!(
            resolution.checked_with_primitive,
            FemsResetStoragePrimitive::LegacyLocalStore | FemsResetStoragePrimitive::LegacyFts5
        ) {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "scope_resolutions.checked_with_primitive",
                message: "state validation must use reset-approved primitives",
            });
        }
    }
}

fn validate_comparison_facts(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    safeguards: &FemsWriteTimeSafeguardsV1,
) {
    let proposal_ids: HashSet<&str> = safeguards
        .proposals
        .iter()
        .map(|proposal| proposal.proposal_id.as_str())
        .collect();
    let memory_ids: HashSet<&str> = safeguards
        .existing_items
        .iter()
        .map(|item| item.memory_id.as_str())
        .collect();

    for comparison in &safeguards.comparison_facts {
        require_non_empty(
            errors,
            "comparison_facts.proposal_id",
            &comparison.proposal_id,
        );
        require_non_empty(
            errors,
            "comparison_facts.existing_memory_id",
            &comparison.existing_memory_id,
        );
        if !proposal_ids.contains(comparison.proposal_id.as_str()) {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "comparison_facts.proposal_id",
                message: "comparison fact references an unknown proposal",
            });
        }
        if !memory_ids.contains(comparison.existing_memory_id.as_str()) {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "comparison_facts.existing_memory_id",
                message: "comparison fact references an unknown memory item",
            });
        }
        if !comparison.generated_by_reset_approved_search {
            errors.push(FemsWriteTimeSafeguardValidationError {
                field: "comparison_facts.generated_by_reset_approved_search",
                message: "comparison facts must come from reset-approved storage/search primitives",
            });
        }
    }
}

fn comparisons_by_proposal<'a>(
    comparisons: &'a [FemsMemoryComparisonFactV1],
) -> BTreeMap<&'a str, Vec<&'a FemsMemoryComparisonFactV1>> {
    let mut grouped: BTreeMap<&str, Vec<&FemsMemoryComparisonFactV1>> = BTreeMap::new();
    for comparison in comparisons {
        grouped
            .entry(comparison.proposal_id.as_str())
            .or_default()
            .push(comparison);
    }
    grouped
}

fn is_duplicate(
    proposal: &FemsMemoryWriteProposalV1,
    existing: &FemsExistingMemoryItemV1,
    comparison: &FemsMemoryComparisonFactV1,
) -> bool {
    comparison.same_scope
        && comparison.exact_key_match
        && proposal.memory_class == existing.memory_class
        && proposal.memory_type == existing.memory_type
        && proposal.summary_hash == existing.summary_hash
        && scope_refs_match(&proposal.scope_refs, &existing.scope_refs)
}

fn should_apply_novelty_penalty(
    proposal: &FemsMemoryWriteProposalV1,
    comparison: &FemsMemoryComparisonFactV1,
    config: &FemsWriteTimeSafeguardConfigV1,
) -> bool {
    comparison.same_scope
        && !comparison.exact_key_match
        && !comparison.contradiction_detected
        && comparison.similarity_x100 >= config.novelty_similarity_threshold_x100
        && proposal.memory_class != "procedural"
}

fn should_supersede(
    proposal: &FemsMemoryWriteProposalV1,
    existing: &FemsExistingMemoryItemV1,
    comparison: &FemsMemoryComparisonFactV1,
) -> bool {
    comparison.same_scope
        && !comparison.exact_key_match
        && !comparison.contradiction_detected
        && proposal.memory_class == "procedural"
        && existing.memory_class == "procedural"
        && existing.status == FemsMemoryItemStatus::Active
        && scope_refs_match(&proposal.scope_refs, &existing.scope_refs)
}

fn scope_refs_match(left: &[String], right: &[String]) -> bool {
    let mut left_sorted = left.to_vec();
    let mut right_sorted = right.to_vec();
    left_sorted.sort();
    right_sorted.sort();
    left_sorted == right_sorted
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn require_non_empty(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(FemsWriteTimeSafeguardValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<FemsWriteTimeSafeguardValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(FemsWriteTimeSafeguardValidationError {
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
