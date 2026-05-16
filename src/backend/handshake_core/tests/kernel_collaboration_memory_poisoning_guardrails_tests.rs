use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    fems_memory_poisoning_drift_guardrails::{
        evaluate_fems_memory_poisoning_drift_guardrails,
        validate_fems_memory_poisoning_drift_guardrails, FemsGuardrailAuditConfigV1,
        FemsGuardrailMemoryCandidateV1, FemsGuardrailTrustLevel,
        FemsMemoryPoisoningDriftGuardrailConfigV1, FemsMemoryPoisoningDriftGuardrailsV1,
    },
};

#[test]
fn kernel_collaboration_memory_guardrails_gate_untrusted_procedural_memory() {
    let guardrails = sample_guardrails();
    validate_fems_memory_poisoning_drift_guardrails(&guardrails).expect("guardrails validate");

    let report =
        evaluate_fems_memory_poisoning_drift_guardrails(&guardrails).expect("report builds");

    assert!(report
        .denied_memory_ids
        .contains(&"mem-untrusted-procedural".to_string()));
    assert!(report
        .drift_denied_memory_ids
        .contains(&"mem-drifted-source".to_string()));
    assert!(!report
        .selected_memory_ids
        .contains(&"mem-untrusted-procedural".to_string()));
    assert!(!report
        .selected_memory_ids
        .contains(&"mem-drifted-source".to_string()));
    assert!(report
        .proposal_event_refs
        .contains(&"FR-EVT-MEM-PROPOSAL-untrusted-procedural".to_string()));
    assert!(report
        .denial_event_refs
        .contains(&"FR-EVT-MEM-DENIAL-untrusted-procedural".to_string()));
    assert!(report
        .approval_event_refs
        .contains(&"FR-EVT-MEM-APPROVAL-trusted-procedural".to_string()));
}

#[test]
fn kernel_collaboration_memory_guardrails_enforce_budget_and_effective_hash() {
    let guardrails = sample_guardrails();
    let report =
        evaluate_fems_memory_poisoning_drift_guardrails(&guardrails).expect("report builds");
    let report_again =
        evaluate_fems_memory_poisoning_drift_guardrails(&guardrails).expect("deterministic report");

    assert!(report.effective_pack_tokens <= 500);
    assert!(!report.deterministic_reduction_markers.is_empty());
    assert!(report
        .deterministic_reduction_markers
        .iter()
        .any(|marker| marker.contains("mem-over-budget")));
    assert_eq!(report.effective_pack_hash, report_again.effective_pack_hash);
    assert_eq!(report.effective_pack_hash.len(), 64);
    assert!(!report.can_invoke_model_until_guarded);
}

#[test]
fn kernel_collaboration_memory_guardrails_reject_drift_prone_configuration() {
    let mut guardrails = sample_guardrails();
    guardrails.config.max_pack_tokens = 650;
    guardrails.config.deterministic_reduction_enabled = false;
    guardrails.config.procedural_trust_gate_enabled = false;
    guardrails.config.proposal_approval_denial_events_required = false;
    guardrails.config.effective_pack_hash_required = false;
    guardrails.audit.flight_recorder_event_family = "FR-EVT-OTHER".to_string();
    guardrails.candidates[0].proposal_event_ref.clear();

    let errors = validate_fems_memory_poisoning_drift_guardrails(&guardrails)
        .expect_err("drift-prone configuration must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "config.max_pack_tokens"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.deterministic_reduction_enabled"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.procedural_trust_gate_enabled"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.proposal_approval_denial_events_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.effective_pack_hash_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "audit.flight_recorder_event_family"));
    assert!(errors
        .iter()
        .any(|error| error.field == "candidates.proposal_event_ref"));
}

#[test]
fn kernel_collaboration_memory_guardrails_catalogs_evaluation_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.fems_memory_poisoning_drift_guardrails.evaluate")
        .expect("FEMS memory poisoning/drift guardrail action must be cataloged");

    assert_eq!(
        action.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "fems_procedural_trust_gate"));
}

fn sample_guardrails() -> FemsMemoryPoisoningDriftGuardrailsV1 {
    FemsMemoryPoisoningDriftGuardrailsV1 {
        schema_id: "hsk.kernel.fems_memory_poisoning_drift_guardrails@1".to_string(),
        guardrail_id: "kernel002-fems-poisoning-drift-mt035".to_string(),
        folded_stub_ids: vec!["WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1".to_string()],
        pack_id: "memory-pack-mt035".to_string(),
        candidates: vec![
            candidate(
                "mem-trusted-procedural",
                "procedural",
                FemsGuardrailTrustLevel::Trusted,
                220,
                true,
                Some("FR-EVT-MEM-APPROVAL-trusted-procedural"),
                None,
                true,
                true,
            ),
            candidate(
                "mem-reviewed-semantic",
                "semantic",
                FemsGuardrailTrustLevel::Reviewed,
                210,
                false,
                None,
                None,
                true,
                true,
            ),
            candidate(
                "mem-untrusted-procedural",
                "procedural",
                FemsGuardrailTrustLevel::Untrusted,
                120,
                true,
                None,
                Some("FR-EVT-MEM-DENIAL-untrusted-procedural"),
                true,
                true,
            ),
            candidate(
                "mem-over-budget",
                "semantic",
                FemsGuardrailTrustLevel::Trusted,
                190,
                true,
                Some("FR-EVT-MEM-APPROVAL-over-budget"),
                None,
                true,
                true,
            ),
            candidate(
                "mem-drifted-source",
                "semantic",
                FemsGuardrailTrustLevel::Trusted,
                40,
                true,
                Some("FR-EVT-MEM-APPROVAL-drifted-source"),
                Some("FR-EVT-MEM-DENIAL-drifted-source"),
                false,
                true,
            ),
        ],
        config: FemsMemoryPoisoningDriftGuardrailConfigV1 {
            max_pack_tokens: 500,
            deterministic_reduction_enabled: true,
            procedural_trust_gate_enabled: true,
            untrusted_long_lived_memory_denied: true,
            proposal_approval_denial_events_required: true,
            effective_pack_hash_required: true,
        },
        audit: FemsGuardrailAuditConfigV1 {
            flight_recorder_event_family: "FR-EVT-MEM".to_string(),
            event_ledger_ref: "event-ledger://fems-memory-guardrails".to_string(),
            replay_log_ref: "jsonl://fems-memory-guardrails/mt035".to_string(),
            debug_bundle_ref: "debug-bundle://fems-memory-guardrails/mt035".to_string(),
        },
        product_authority_refs: vec![
            "ace.memory_pack".to_string(),
            "flight_recorder.memory_pack_built".to_string(),
            "kernel.fems_write_time_safeguards".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1.contract.json"
                .to_string(),
        ],
    }
}

fn candidate(
    memory_id: &str,
    memory_class: &str,
    trust_level: FemsGuardrailTrustLevel,
    token_count: u32,
    long_lived: bool,
    approval_event_ref: Option<&str>,
    denial_event_ref: Option<&str>,
    source_fresh: bool,
    provenance_preserved: bool,
) -> FemsGuardrailMemoryCandidateV1 {
    FemsGuardrailMemoryCandidateV1 {
        memory_id: memory_id.to_string(),
        memory_class: memory_class.to_string(),
        trust_level,
        source_ref: format!("source://{memory_id}"),
        scope_refs: vec![format!("scope://{memory_id}")],
        token_count,
        content_hash: format!("content-hash-{memory_id}"),
        source_fresh,
        provenance_preserved,
        proposal_event_ref: format!("FR-EVT-MEM-PROPOSAL-{memory_id}").replace("mem-", ""),
        approval_event_ref: approval_event_ref.map(str::to_string),
        denial_event_ref: denial_event_ref.map(str::to_string),
        long_lived,
    }
}
