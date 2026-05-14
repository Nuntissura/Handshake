use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_FEMS_MEMORY_POISONING_DRIFT_GUARDRAILS_STUB_ID: &str =
    "WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FemsGuardrailTrustLevel {
    Trusted,
    Reviewed,
    Untrusted,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsGuardrailMemoryCandidateV1 {
    pub memory_id: String,
    pub memory_class: String,
    pub trust_level: FemsGuardrailTrustLevel,
    pub source_ref: String,
    pub scope_refs: Vec<String>,
    pub token_count: u32,
    pub content_hash: String,
    pub source_fresh: bool,
    pub provenance_preserved: bool,
    pub proposal_event_ref: String,
    pub approval_event_ref: Option<String>,
    pub denial_event_ref: Option<String>,
    pub long_lived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMemoryPoisoningDriftGuardrailConfigV1 {
    pub max_pack_tokens: u32,
    pub deterministic_reduction_enabled: bool,
    pub procedural_trust_gate_enabled: bool,
    pub untrusted_long_lived_memory_denied: bool,
    pub proposal_approval_denial_events_required: bool,
    pub effective_pack_hash_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsGuardrailAuditConfigV1 {
    pub flight_recorder_event_family: String,
    pub event_ledger_ref: String,
    pub replay_log_ref: String,
    pub debug_bundle_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FemsMemoryPoisoningDriftGuardrailsV1 {
    pub schema_id: String,
    pub guardrail_id: String,
    pub folded_stub_ids: Vec<String>,
    pub pack_id: String,
    pub candidates: Vec<FemsGuardrailMemoryCandidateV1>,
    pub config: FemsMemoryPoisoningDriftGuardrailConfigV1,
    pub audit: FemsGuardrailAuditConfigV1,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsMemoryPoisoningDriftGuardrailReportV1 {
    pub schema_id: String,
    pub guardrail_id: String,
    pub pack_id: String,
    pub selected_memory_ids: Vec<String>,
    pub denied_memory_ids: Vec<String>,
    pub drift_denied_memory_ids: Vec<String>,
    pub deterministic_reduction_markers: Vec<String>,
    pub effective_pack_tokens: u32,
    pub effective_pack_hash: String,
    pub proposal_event_refs: Vec<String>,
    pub approval_event_refs: Vec<String>,
    pub denial_event_refs: Vec<String>,
    pub effective_pack_event_ref: String,
    pub replay_log_ref: String,
    pub can_invoke_model_until_guarded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FemsMemoryPoisoningDriftGuardrailValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_fems_memory_poisoning_drift_guardrails(
    guardrails: &FemsMemoryPoisoningDriftGuardrailsV1,
) -> Result<(), Vec<FemsMemoryPoisoningDriftGuardrailValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &guardrails.schema_id);
    require_non_empty(&mut errors, "guardrail_id", &guardrails.guardrail_id);
    require_non_empty(&mut errors, "pack_id", &guardrails.pack_id);
    require_vec(&mut errors, "folded_stub_ids", &guardrails.folded_stub_ids);
    require_vec(&mut errors, "candidates", &guardrails.candidates);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &guardrails.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &guardrails.folded_source_refs,
    );

    if !contains_exact(
        &guardrails.folded_stub_ids,
        FOLDED_FEMS_MEMORY_POISONING_DRIFT_GUARDRAILS_STUB_ID,
    ) {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "folded_stub_ids",
            message: "guardrails must preserve the folded FEMS memory poisoning/drift stub id",
        });
    }
    if !contains_text(
        &guardrails.folded_source_refs,
        FOLDED_FEMS_MEMORY_POISONING_DRIFT_GUARDRAILS_STUB_ID,
    ) {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "folded_source_refs",
            message: "guardrails must preserve the folded source reference",
        });
    }

    validate_config(&mut errors, &guardrails.config);
    validate_audit(&mut errors, &guardrails.audit);
    validate_authority_refs(&mut errors, guardrails);
    validate_candidates(&mut errors, guardrails);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn evaluate_fems_memory_poisoning_drift_guardrails(
    guardrails: &FemsMemoryPoisoningDriftGuardrailsV1,
) -> Result<
    FemsMemoryPoisoningDriftGuardrailReportV1,
    Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
> {
    validate_fems_memory_poisoning_drift_guardrails(guardrails)?;

    let mut selected_memory_ids = Vec::new();
    let mut denied_memory_ids = Vec::new();
    let mut drift_denied_memory_ids = Vec::new();
    let mut deterministic_reduction_markers = Vec::new();
    let mut effective_pack_tokens = 0u32;
    let mut proposal_event_refs = Vec::new();
    let mut approval_event_refs = Vec::new();
    let mut denial_event_refs = Vec::new();
    let mut hash_parts = Vec::new();

    for candidate in &guardrails.candidates {
        push_unique(&mut proposal_event_refs, &candidate.proposal_event_ref);

        if let Some(approval_event_ref) = candidate.approval_event_ref.as_deref() {
            push_unique(&mut approval_event_refs, approval_event_ref);
        }
        if let Some(denial_event_ref) = candidate.denial_event_ref.as_deref() {
            push_unique(&mut denial_event_refs, denial_event_ref);
        }

        if denies_untrusted_long_lived_procedural(candidate, &guardrails.config) {
            push_unique(&mut denied_memory_ids, &candidate.memory_id);
            if let Some(denial_event_ref) = candidate.denial_event_ref.as_deref() {
                hash_parts.push(format!("denied:{}:{denial_event_ref}", candidate.memory_id));
            }
            continue;
        }
        if denies_source_drift(candidate) {
            push_unique(&mut denied_memory_ids, &candidate.memory_id);
            push_unique(&mut drift_denied_memory_ids, &candidate.memory_id);
            if let Some(denial_event_ref) = candidate.denial_event_ref.as_deref() {
                hash_parts.push(format!(
                    "drift-denied:{}:{denial_event_ref}",
                    candidate.memory_id
                ));
            }
            continue;
        }

        if effective_pack_tokens.saturating_add(candidate.token_count)
            > guardrails.config.max_pack_tokens
        {
            let marker = format!(
                "TRUNCATED:{}:{}+{}>{}",
                candidate.memory_id,
                effective_pack_tokens,
                candidate.token_count,
                guardrails.config.max_pack_tokens
            );
            deterministic_reduction_markers.push(marker.clone());
            hash_parts.push(marker);
            continue;
        }

        selected_memory_ids.push(candidate.memory_id.clone());
        effective_pack_tokens += candidate.token_count;
        hash_parts.push(format!(
            "selected:{}:{}:{}:{}",
            candidate.memory_id,
            candidate.content_hash,
            candidate.token_count,
            candidate.source_ref
        ));
    }

    let effective_pack_event_ref = format!("FR-EVT-MEM-PACK-HASH-{}", guardrails.pack_id);
    hash_parts.push(format!("pack:{}", guardrails.pack_id));
    hash_parts.push(format!("tokens:{effective_pack_tokens}"));
    hash_parts.push(format!("event:{effective_pack_event_ref}"));

    Ok(FemsMemoryPoisoningDriftGuardrailReportV1 {
        schema_id: "hsk.kernel.fems_memory_poisoning_drift_guardrail_report@1".to_string(),
        guardrail_id: guardrails.guardrail_id.clone(),
        pack_id: guardrails.pack_id.clone(),
        selected_memory_ids,
        denied_memory_ids,
        drift_denied_memory_ids,
        deterministic_reduction_markers,
        effective_pack_tokens,
        effective_pack_hash: stable_hex_hash_64(&hash_parts),
        proposal_event_refs,
        approval_event_refs,
        denial_event_refs,
        effective_pack_event_ref,
        replay_log_ref: guardrails.audit.replay_log_ref.clone(),
        can_invoke_model_until_guarded: false,
    })
}

fn validate_config(
    errors: &mut Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
    config: &FemsMemoryPoisoningDriftGuardrailConfigV1,
) {
    if config.max_pack_tokens == 0 || config.max_pack_tokens > 500 {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "config.max_pack_tokens",
            message: "FEMS MemoryPack budget must be hard-capped at <= 500 tokens",
        });
    }
    if !config.deterministic_reduction_enabled {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "config.deterministic_reduction_enabled",
            message: "oversized packs must be reduced deterministically",
        });
    }
    if !config.procedural_trust_gate_enabled {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "config.procedural_trust_gate_enabled",
            message: "procedural memory writes must pass trust gates",
        });
    }
    if !config.untrusted_long_lived_memory_denied {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "config.untrusted_long_lived_memory_denied",
            message: "untrusted long-lived memory must not auto-promote",
        });
    }
    if !config.proposal_approval_denial_events_required {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "config.proposal_approval_denial_events_required",
            message: "proposal, approval, and denial decisions need replay-grade evidence",
        });
    }
    if !config.effective_pack_hash_required {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "config.effective_pack_hash_required",
            message: "effective pack hashes are required to prevent replay drift",
        });
    }
}

fn validate_audit(
    errors: &mut Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
    audit: &FemsGuardrailAuditConfigV1,
) {
    if audit.flight_recorder_event_family != "FR-EVT-MEM" {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field: "audit.flight_recorder_event_family",
            message: "FEMS guardrails must use FR-EVT-MEM replay evidence",
        });
    }
    require_non_empty(
        errors,
        "audit.flight_recorder_event_family",
        &audit.flight_recorder_event_family,
    );
    require_non_empty(errors, "audit.event_ledger_ref", &audit.event_ledger_ref);
    require_non_empty(errors, "audit.replay_log_ref", &audit.replay_log_ref);
    require_non_empty(errors, "audit.debug_bundle_ref", &audit.debug_bundle_ref);
}

fn validate_authority_refs(
    errors: &mut Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
    guardrails: &FemsMemoryPoisoningDriftGuardrailsV1,
) {
    for required_ref in [
        "ace.memory_pack",
        "flight_recorder.memory_pack_built",
        "kernel.fems_write_time_safeguards",
    ] {
        if !contains_exact(&guardrails.product_authority_refs, required_ref) {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "product_authority_refs",
                message: "guardrails must cite ACE memory pack, Flight Recorder memory-pack evidence, and write-time safeguards",
            });
        }
    }
}

fn validate_candidates(
    errors: &mut Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
    guardrails: &FemsMemoryPoisoningDriftGuardrailsV1,
) {
    let mut memory_ids = HashSet::new();

    for candidate in &guardrails.candidates {
        if !memory_ids.insert(candidate.memory_id.as_str()) {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "candidates.memory_id",
                message: "candidate memory ids must be unique",
            });
        }
        require_non_empty(errors, "candidates.memory_id", &candidate.memory_id);
        require_non_empty(errors, "candidates.memory_class", &candidate.memory_class);
        require_non_empty(errors, "candidates.source_ref", &candidate.source_ref);
        require_vec(errors, "candidates.scope_refs", &candidate.scope_refs);
        require_non_empty(errors, "candidates.content_hash", &candidate.content_hash);
        require_non_empty(
            errors,
            "candidates.proposal_event_ref",
            &candidate.proposal_event_ref,
        );
        if !candidate.proposal_event_ref.starts_with("FR-EVT-MEM-") {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "candidates.proposal_event_ref",
                message: "proposal evidence must be an FR-EVT-MEM ref",
            });
        }
        if candidate.token_count == 0 {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "candidates.token_count",
                message: "candidate token count must be greater than zero",
            });
        }
        if denies_source_drift(candidate)
            && candidate
                .denial_event_ref
                .as_deref()
                .unwrap_or_default()
                .is_empty()
        {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "candidates.denial_event_ref",
                message: "drifted memory requires denial evidence before pack selection",
            });
        }

        let is_procedural = candidate.memory_class == "procedural";
        if guardrails.config.proposal_approval_denial_events_required
            && is_procedural
            && !denies_untrusted_long_lived_procedural(candidate, &guardrails.config)
            && candidate
                .approval_event_ref
                .as_deref()
                .unwrap_or_default()
                .is_empty()
        {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "candidates.approval_event_ref",
                message: "trusted procedural memory requires approval evidence",
            });
        }
        if guardrails.config.proposal_approval_denial_events_required
            && denies_untrusted_long_lived_procedural(candidate, &guardrails.config)
            && candidate
                .denial_event_ref
                .as_deref()
                .unwrap_or_default()
                .is_empty()
        {
            errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
                field: "candidates.denial_event_ref",
                message: "denied untrusted procedural memory requires denial evidence",
            });
        }
    }
}

fn denies_untrusted_long_lived_procedural(
    candidate: &FemsGuardrailMemoryCandidateV1,
    config: &FemsMemoryPoisoningDriftGuardrailConfigV1,
) -> bool {
    config.procedural_trust_gate_enabled
        && config.untrusted_long_lived_memory_denied
        && candidate.long_lived
        && candidate.memory_class == "procedural"
        && matches!(
            candidate.trust_level,
            FemsGuardrailTrustLevel::Untrusted | FemsGuardrailTrustLevel::External
        )
}

fn denies_source_drift(candidate: &FemsGuardrailMemoryCandidateV1) -> bool {
    !candidate.source_fresh || !candidate.provenance_preserved
}

fn stable_hex_hash_64(parts: &[String]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let chunk = format!("{hash:016x}");
    format!("{chunk}{chunk}{chunk}{chunk}")
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn require_non_empty(
    errors: &mut Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<FemsMemoryPoisoningDriftGuardrailValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(FemsMemoryPoisoningDriftGuardrailValidationError {
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
