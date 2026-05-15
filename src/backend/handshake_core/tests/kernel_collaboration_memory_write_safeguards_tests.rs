use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::{ApprovalPosture, AuthorityEffect},
    fems_write_time_safeguards::{
        evaluate_fems_write_time_safeguards, validate_fems_write_time_safeguards,
        FemsAuditTrailConfigV1, FemsExistingMemoryItemV1, FemsMemoryComparisonFactV1,
        FemsMemoryItemStatus, FemsMemoryWriteProposalV1, FemsResetStoragePrimitive,
        FemsScopeResolutionV1, FemsWriteTimeSafeguardConfigV1, FemsWriteTimeSafeguardsV1,
    },
};

#[test]
fn kernel_collaboration_memory_write_safeguards_run_mechanical_guard_report() {
    let safeguards = sample_safeguards();

    validate_fems_write_time_safeguards(&safeguards).expect("safeguards validate");
    let report = evaluate_fems_write_time_safeguards(&safeguards).expect("guard report");

    assert!(!report.uses_llm_calls);
    assert!(!report.uses_legacy_search_authority);
    assert_eq!(report.proposal_outcomes.len(), 5);

    let duplicate = outcome(&report, "proposal-duplicate");
    assert!(duplicate.skip_write);
    assert!(duplicate
        .commit_report_warnings
        .iter()
        .any(|warning| warning.contains("duplicate")));

    let near = outcome(&report, "proposal-near-duplicate");
    assert_eq!(near.novelty_penalty_multiplier_x100, Some(30));
    assert!(near
        .commit_report_warnings
        .iter()
        .any(|warning| warning.contains("novelty")));

    let superseding = outcome(&report, "proposal-superseding-procedure");
    assert!(superseding
        .supersedes_memory_ids
        .contains(&"mem-old-procedural".to_string()));
    assert!(report
        .memory_pack_exclusion_ids
        .contains(&"mem-old-procedural".to_string()));

    let conflict = outcome(&report, "proposal-conflict");
    assert!(conflict
        .contradicted_memory_ids
        .contains(&"mem-conflicting".to_string()));
    assert!(report
        .dcc_conflict_queue_refs
        .contains(&"dcc-conflict://proposal-conflict--mem-conflicting".to_string()));

    let stale = outcome(&report, "proposal-stale-scope");
    assert!(stale
        .stale_scope_refs
        .contains(&"scope://deleted-feature".to_string()));

    assert_eq!(
        report.audit_event_refs.len(),
        report.proposal_outcomes.len()
    );
    assert!(report
        .authoritative_storage_primitives
        .contains(&FemsResetStoragePrimitive::Postgres));
    assert!(report
        .authoritative_storage_primitives
        .contains(&FemsResetStoragePrimitive::EventLedger));
    assert!(report
        .authoritative_storage_primitives
        .contains(&FemsResetStoragePrimitive::CrdtSearchIndex));
}

#[test]
fn kernel_collaboration_memory_write_safeguards_reject_legacy_storage_and_llm_guards() {
    let mut safeguards = sample_safeguards();
    safeguards.config.mechanical_no_llm = false;
    safeguards.config.max_latency_ms = 25;
    safeguards.config.novelty_penalty_multiplier_x100 = 50;
    safeguards.config.storage_primitives = vec![
        FemsResetStoragePrimitive::LegacyLocalStore,
        FemsResetStoragePrimitive::LegacyFts5,
    ];
    safeguards.audit.jsonl_enabled = false;
    safeguards.audit.flight_recorder_event_ref.clear();
    safeguards.comparison_facts[0].generated_by_reset_approved_search = false;

    let errors =
        validate_fems_write_time_safeguards(&safeguards).expect_err("legacy guards must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "config.mechanical_no_llm"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.max_latency_ms"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.novelty_penalty_multiplier_x100"));
    assert!(errors
        .iter()
        .any(|error| error.field == "config.storage_primitives"));
    assert!(errors
        .iter()
        .any(|error| error.field == "audit.jsonl_enabled"));
    assert!(errors
        .iter()
        .any(|error| error.field == "comparison_facts.generated_by_reset_approved_search"));
}

#[test]
fn kernel_collaboration_memory_write_safeguards_catalogs_evaluation_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.fems_write_time_safeguards.evaluate")
        .expect("FEMS write-time safeguards action must be cataloged");

    assert_eq!(
        action.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        action.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "fems_write_time_mechanical_guards"));
}

fn sample_safeguards() -> FemsWriteTimeSafeguardsV1 {
    FemsWriteTimeSafeguardsV1 {
        schema_id: "hsk.kernel.fems_write_time_safeguards@1".to_string(),
        safeguard_id: "kernel002-fems-write-time-mt034".to_string(),
        folded_stub_ids: vec!["WP-1-FEMS-Write-Time-Safeguards-v1".to_string()],
        proposals: vec![
            proposal(
                "proposal-duplicate",
                "semantic",
                "fact",
                &["scope://feature-a"],
                "hash-duplicate",
            ),
            proposal(
                "proposal-near-duplicate",
                "semantic",
                "fact",
                &["scope://feature-near"],
                "hash-near-new",
            ),
            proposal(
                "proposal-superseding-procedure",
                "procedural",
                "runbook",
                &["scope://operator-runbook"],
                "hash-runbook-new",
            ),
            proposal(
                "proposal-conflict",
                "semantic",
                "fact",
                &["scope://feature-conflict"],
                "hash-conflict-new",
            ),
            proposal(
                "proposal-stale-scope",
                "semantic",
                "fact",
                &["scope://deleted-feature"],
                "hash-stale",
            ),
        ],
        existing_items: vec![
            existing(
                "mem-duplicate",
                "semantic",
                "fact",
                &["scope://feature-a"],
                "hash-duplicate",
            ),
            existing(
                "mem-near",
                "semantic",
                "fact",
                &["scope://feature-near"],
                "hash-near-old",
            ),
            existing(
                "mem-old-procedural",
                "procedural",
                "runbook",
                &["scope://operator-runbook"],
                "hash-runbook-old",
            ),
            existing(
                "mem-conflicting",
                "semantic",
                "fact",
                &["scope://feature-conflict"],
                "hash-conflict-old",
            ),
        ],
        comparison_facts: vec![
            comparison(
                "proposal-duplicate",
                "mem-duplicate",
                true,
                true,
                100,
                false,
            ),
            comparison(
                "proposal-near-duplicate",
                "mem-near",
                true,
                false,
                92,
                false,
            ),
            comparison(
                "proposal-superseding-procedure",
                "mem-old-procedural",
                true,
                false,
                40,
                false,
            ),
            comparison(
                "proposal-conflict",
                "mem-conflicting",
                true,
                false,
                25,
                true,
            ),
        ],
        scope_resolutions: vec![
            resolution("scope://feature-a", true),
            resolution("scope://feature-near", true),
            resolution("scope://operator-runbook", true),
            resolution("scope://feature-conflict", true),
            resolution("scope://deleted-feature", false),
        ],
        config: FemsWriteTimeSafeguardConfigV1 {
            mechanical_no_llm: true,
            novelty_similarity_threshold_x100: 85,
            novelty_penalty_multiplier_x100: 30,
            max_latency_ms: 10,
            storage_primitives: vec![
                FemsResetStoragePrimitive::Postgres,
                FemsResetStoragePrimitive::EventLedger,
                FemsResetStoragePrimitive::CrdtSearchIndex,
            ],
        },
        audit: FemsAuditTrailConfigV1 {
            jsonl_enabled: true,
            audit_trail_ref: "jsonl://fems-write-time-safeguards/mt034".to_string(),
            flight_recorder_event_ref: "FR-EVT-MEM-005".to_string(),
            event_ledger_ref: "event-ledger://fems-write-time".to_string(),
            debug_bundle_exportable: true,
        },
        product_authority_refs: vec![
            "kernel.reset_invariants".to_string(),
            "flight_recorder.memory_item_status_changed".to_string(),
            "kernel.fems_working_memory_checkpoint".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-FEMS-Write-Time-Safeguards-v1.contract.json".to_string(),
        ],
    }
}

fn proposal(
    proposal_id: &str,
    memory_class: &str,
    memory_type: &str,
    scope_refs: &[&str],
    summary_hash: &str,
) -> FemsMemoryWriteProposalV1 {
    FemsMemoryWriteProposalV1 {
        proposal_id: proposal_id.to_string(),
        memory_class: memory_class.to_string(),
        memory_type: memory_type.to_string(),
        scope_refs: scope_refs
            .iter()
            .map(|scope| (*scope).to_string())
            .collect(),
        summary: format!("Summary for {proposal_id}"),
        summary_hash: summary_hash.to_string(),
        content_hash: format!("content-{summary_hash}"),
        importance_x100: 80,
    }
}

fn existing(
    memory_id: &str,
    memory_class: &str,
    memory_type: &str,
    scope_refs: &[&str],
    summary_hash: &str,
) -> FemsExistingMemoryItemV1 {
    FemsExistingMemoryItemV1 {
        memory_id: memory_id.to_string(),
        memory_class: memory_class.to_string(),
        memory_type: memory_type.to_string(),
        scope_refs: scope_refs
            .iter()
            .map(|scope| (*scope).to_string())
            .collect(),
        summary_hash: summary_hash.to_string(),
        status: FemsMemoryItemStatus::Active,
    }
}

fn comparison(
    proposal_id: &str,
    existing_memory_id: &str,
    same_scope: bool,
    exact_key_match: bool,
    similarity_x100: u8,
    contradiction_detected: bool,
) -> FemsMemoryComparisonFactV1 {
    FemsMemoryComparisonFactV1 {
        proposal_id: proposal_id.to_string(),
        existing_memory_id: existing_memory_id.to_string(),
        same_scope,
        exact_key_match,
        similarity_x100,
        contradiction_detected,
        generated_by_reset_approved_search: true,
    }
}

fn resolution(scope_ref: &str, resolves: bool) -> FemsScopeResolutionV1 {
    FemsScopeResolutionV1 {
        scope_ref: scope_ref.to_string(),
        resolves,
        checked_with_primitive: FemsResetStoragePrimitive::CrdtSearchIndex,
        state_ref: format!("state://{scope_ref}"),
    }
}

fn outcome<'a>(
    report: &'a handshake_core::kernel::fems_write_time_safeguards::FemsWriteTimeSafeguardReportV1,
    proposal_id: &str,
) -> &'a handshake_core::kernel::fems_write_time_safeguards::FemsProposalSafeguardOutcomeV1 {
    report
        .proposal_outcomes
        .iter()
        .find(|outcome| outcome.proposal_id == proposal_id)
        .expect("outcome exists")
}
