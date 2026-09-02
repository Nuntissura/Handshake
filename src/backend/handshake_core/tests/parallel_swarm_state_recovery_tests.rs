//! MT-141 adversarial retirement inventory for the former parallel-swarm
//! integration corpus.
//!
//! MT-142 is a future concurrency/load contract. Its proof targets do not
//! automatically supersede the policy, rollback, projection, redaction,
//! mailbox, quiet-work, checkpoint, and recovery assertions removed here.
//! Every former test is listed separately with an executable embedded
//! successor.

#[allow(dead_code)]
mod user_manual_support;

use std::sync::Arc;

use handshake_core::{
    api::{kernel as kernel_api, knowledge_code_nav as nav_api},
    kernel::KernelActor,
    knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine},
    storage::{
        knowledge::{KnowledgeStore, NewKnowledgeRichDocument},
        surreal::{
            bootstrap_schema, RowFilter, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
        },
        tests::{embedded_test_backend, EmbeddedTestBackend},
        Database, NewWorkspace, StorageError, WriteContext,
    },
    swarm_orchestration::state_recovery::{
        validate_cloud_assistance_receipt, validate_handoff_compression_template,
        validate_swarm_dashboard_projection, AgentCapability, AgentLaneIdentity, AgentLaneKind,
        AttributionMode, BackendNavigationCommand, ClaimScope, ClaimStatus,
        CloudAssistanceOutputKind, CloudAssistanceReceiptV1, CloudAssistanceRequest,
        CloudFallbackBasisRequest, CloudFallbackReason, HandoffCompressionRequest,
        IndexLeaseStatus, IndexingLeaseRequest, LocalCloudAttribution, ModelProviderKind,
        NavigationCommandSet, ParallelSwarmStateRecoveryStore, QuietBackgroundPolicy,
        QuietBackgroundWorkKind, QuietBackgroundWorkRequest, RecoveryCheckpointRequest,
        RecoveryResumePointer, RoleMailboxHandoffRequest, StateRecoveryError,
        StateRecoveryTestFailpoint, SwarmDashboardProjectionRequest,
        SwarmEvidenceInspectionRequest, SwarmReceiptStatus, WorkClaimRequest,
    },
};
use serde_json::json;
use tokio::sync::Barrier;
use user_manual_support::{app_state_for, start_server};
use uuid::Uuid;

struct RetiredTestDisposition {
    retired_test: &'static str,
    retired_behavior: &'static str,
    successor_status: &'static str,
}

async fn recovery_store() -> (EmbeddedTestBackend, ParallelSwarmStateRecoveryStore) {
    let backend = embedded_test_backend()
        .await
        .expect("open isolated embedded swarm store");
    let store = ParallelSwarmStateRecoveryStore::new(backend.storage.clone());
    (backend, store)
}

async fn finish_store(store: ParallelSwarmStateRecoveryStore, backend: EmbeddedTestBackend) {
    drop(store);
    backend
        .close_and_remove()
        .await
        .expect("remove embedded swarm store");
}

async fn reopen_store(
    backend: &EmbeddedTestBackend,
) -> (SurrealStorage, ParallelSwarmStateRecoveryStore) {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original swarm store");
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened swarm store"),
    )
    .await
    .expect("reopen swarm store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap reopened swarm schema");
    let store = ParallelSwarmStateRecoveryStore::new(storage.clone());
    (storage, store)
}

async fn close_reopened_store(
    storage: SurrealStorage,
    store: ParallelSwarmStateRecoveryStore,
    backend: EmbeddedTestBackend,
) {
    drop(store);
    storage
        .shutdown()
        .await
        .expect("close reopened swarm store");
    drop(storage);
    backend
        .close_and_remove()
        .await
        .expect("remove reopened swarm store");
}

fn local_lane(suffix: &str) -> AgentLaneIdentity {
    lane_with_kind(suffix, AgentLaneKind::Local)
}

fn lane_with_kind(suffix: &str, kind: AgentLaneKind) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-{suffix}"),
        format!("actor-{suffix}"),
        kind,
        LocalCloudAttribution::local("test-runtime", format!("test-model-{suffix}")),
    )
    .expect("valid lane")
}

fn cloud_lane(suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-cloud-{suffix}"),
        format!("actor-cloud-{suffix}"),
        AgentLaneKind::Cloud,
        LocalCloudAttribution::cloud(
            ModelProviderKind::OpenAi,
            "gpt-test",
            "vault://providers/openai/default",
            json!({"api_key":"must-not-persist","organization":"org-visible"}),
        ),
    )
    .expect("valid cloud lane")
}

fn raw_cloud_lane(suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-cloud-raw-{suffix}"),
        format!("actor-cloud-raw-{suffix}"),
        AgentLaneKind::Cloud,
        LocalCloudAttribution {
            mode: AttributionMode::Cloud,
            provider: Some(ModelProviderKind::OpenAi),
            runtime: None,
            model_label: "gpt-test".to_owned(),
            credential_ref: Some("vault://providers/openai/default".to_owned()),
            provider_metadata: json!({
                "api_key": "sk-raw-secret-must-not-persist",
                "nested": {"token": "raw-token-must-not-persist"},
                "organization": "org-visible"
            }),
        },
    )
    .expect("valid raw cloud lane")
}

fn assert_invalid_input_contains<T>(result: Result<T, StateRecoveryError>, expected: &str) {
    match result {
        Err(StateRecoveryError::InvalidInput(message)) => assert!(
            message.contains(expected),
            "expected invalid input containing {expected}, got {message}"
        ),
        Err(error) => panic!("expected InvalidInput containing {expected}, got {error}"),
        Ok(_) => panic!("expected InvalidInput containing {expected}, got success"),
    }
}

fn claim_request(
    workspace_id: &str,
    scope: ClaimScope,
    lane: AgentLaneIdentity,
    suffix: &str,
) -> WorkClaimRequest {
    WorkClaimRequest {
        workspace_id: workspace_id.to_owned(),
        wp_id: "WP-KERNEL-009".to_owned(),
        mt_id: Some("MT-210".to_owned()),
        scope,
        lane,
        session_id: format!("session-{suffix}"),
        ttl_seconds: 600,
        reason: format!("claim proof {suffix}"),
    }
}

fn checkpoint_request(
    lane: AgentLaneIdentity,
    workspace_id: &str,
    claim_id: Option<String>,
    handoff_id: Option<String>,
) -> RecoveryCheckpointRequest {
    RecoveryCheckpointRequest {
        lane,
        session_id: "session-checkpoint-source".to_owned(),
        workspace_id: workspace_id.to_owned(),
        wp_id: "WP-KERNEL-009".to_owned(),
        mt_id: "MT-214".to_owned(),
        claim_id,
        mailbox_handoff_id: handoff_id,
        navigation_command_id: Some("validation_state".to_owned()),
        resume_pointer: RecoveryResumePointer::MicroTask {
            mt_id: "MT-214".to_owned(),
        },
        touched_files: vec!["src/proof.rs".to_owned()],
        tests: vec!["parallel_swarm_state_recovery_tests".to_owned()],
        hbr_rows: vec!["HBR-SWARM-004".to_owned()],
        next_step_context: "resume exact durable checkpoint".to_owned(),
        payload: json!({"counter": 7}),
        compaction_reason: "mt141_successor".to_owned(),
        git_head: "mt141proof".to_owned(),
    }
}

fn handoff_checkpoint_request(suffix: &str) -> RecoveryCheckpointRequest {
    let mut request = checkpoint_request(
        local_lane(&format!("handoff-{suffix}")),
        &format!("workspace-handoff-{suffix}"),
        None,
        None,
    );
    request.session_id = format!("session-handoff-{suffix}");
    request.mt_id = "MT-222".to_owned();
    request.resume_pointer = RecoveryResumePointer::MicroTask {
        mt_id: "MT-222".to_owned(),
    };
    request.compaction_reason = "session_limit_recovery".to_owned();
    request
}

fn cloud_basis_request(workspace: &str, claim_id: &str, suffix: &str) -> CloudFallbackBasisRequest {
    CloudFallbackBasisRequest {
        lane: local_lane(&format!("basis-{suffix}")),
        workspace_id: workspace.to_owned(),
        wp_id: "WP-KERNEL-009".to_owned(),
        mt_id: "MT-221".to_owned(),
        claim_id: claim_id.to_owned(),
        parent_session_id: format!("session-parent-{suffix}"),
        prompt_sha256: "a".repeat(64),
        session_id: format!("session-basis-{suffix}"),
        fallback_reason: CloudFallbackReason::LocalFailed,
        local_attempt_ref: format!("local://basis/{suffix}"),
        evidence_sha256: "b".repeat(64),
        summary: "local failure authorizes one reviewable cloud output".to_owned(),
    }
}

fn cloud_assistance_request(
    lane: AgentLaneIdentity,
    workspace: &str,
    claim_id: &str,
    basis_event_id: &str,
    suffix: &str,
) -> CloudAssistanceRequest {
    CloudAssistanceRequest {
        from_lane: lane,
        workspace_id: workspace.to_owned(),
        wp_id: "WP-KERNEL-009".to_owned(),
        mt_id: "MT-221".to_owned(),
        claim_id: claim_id.to_owned(),
        session_id: format!("session-output-{suffix}"),
        to_role: "WP_VALIDATOR".to_owned(),
        mailbox_thread_id: format!("thread-{suffix}"),
        mailbox_message_id: format!("message-{suffix}"),
        fallback_basis_event_id: basis_event_id.to_owned(),
        parent_session_id: format!("session-parent-{suffix}"),
        prompt_sha256: "a".repeat(64),
        fallback_reason: CloudFallbackReason::LocalFailed,
        output_kind: CloudAssistanceOutputKind::PatchSuggestion,
        output_sha256: "c".repeat(64),
        body_sha256: "d".repeat(64),
        output_text: "cloud patch suggestion pending validator review".to_owned(),
        output_body_jsonb: json!({"text": "cloud patch suggestion pending validator review"}),
        summary: "reviewable non-authoritative cloud assistance".to_owned(),
        target_ref: "wp://WP-KERNEL-009/MT-221".to_owned(),
    }
}

macro_rules! retired {
    ($name:literal, $behavior:literal, $successor:literal) => {
        RetiredTestDisposition {
            retired_test: $name,
            retired_behavior: $behavior,
            successor_status: $successor,
        }
    };
}

const RETIRED_TESTS: &[RetiredTestDisposition] = &[
    retired!("agent_lanes_and_work_claims_are_typed_attributable_and_exclusive", "typed lane attribution, capability derivation, exclusive claims, and durable claim receipts", "agent_lanes_and_work_claims_are_typed_attributable_and_exclusive"),
    retired!("claim_authority_failure_after_receipt_rolls_back_eventledger_receipt", "claim-authority failure atomically removes the paired EventLedger receipt", "claim_authority_failure_after_receipt_rolls_back_eventledger_receipt"),
    retired!("release_claim_rolls_back_authority_state_if_receipt_insert_fails", "receipt failure rolls claim release back to active authority state", "release_claim_rolls_back_authority_state_if_receipt_insert_fails"),
    retired!("cloud_lane_is_denied_worktree_claim_and_local_index_write_lease", "cloud lanes cannot claim worktrees or local index-write leases", "cloud_lane_is_denied_worktree_claim_and_local_index_write_lease"),
    retired!("cloud_assistance_output_is_reviewable_attributed_and_non_authoritative", "cloud assistance remains attributed, reviewable, and non-authoritative", "cloud_assistance_output_is_reviewable_attributed_and_non_authoritative"),
    retired!("cloud_assistance_requires_cloud_owned_workspace_claim_and_valid_output_hash", "cloud output requires a cloud-owned workspace claim and valid content hash", "cloud_assistance_requires_cloud_owned_workspace_claim_and_valid_output_hash"),
    retired!("cloud_assistance_rejects_loose_or_replayed_fallback_basis", "fallback basis must be tightly bound and non-replayable", "cloud_assistance_rejects_loose_or_replayed_fallback_basis"),
    retired!("editor_document_and_graph_claims_serialize_parallel_mutations", "document and graph mutation scopes serialize same-scope writers", "editor_document_and_graph_claims_serialize_parallel_mutations"),
    retired!("non_editor_lanes_cannot_claim_editor_mutation_scopes", "non-editor lanes are denied editor mutation claims", "non_editor_lanes_cannot_claim_editor_mutation_scopes"),
    retired!("malformed_editor_mutation_scopes_do_not_persist_claims_or_receipts", "malformed editor scopes fail before claims or receipts persist", "malformed_editor_mutation_scopes_do_not_persist_claims_or_receipts"),
    retired!("validator_lanes_inspect_swarm_evidence_without_mutating_state", "validator inspection is read-only and cannot mutate swarm authority", "validator_lanes_inspect_swarm_evidence_without_mutating_state"),
    retired!("swarm_dashboard_projection_derives_from_postgres_eventledger_and_is_projection_only", "dashboard rows derive from EventLedger authority and remain projection-only", "swarm_dashboard_projection_derives_from_embedded_eventledger_and_is_projection_only"),
    retired!("swarm_dashboard_projection_api_exposes_postgres_eventledger_read_model", "HTTP projection API exposes the authoritative swarm read model", "swarm_dashboard_projection_api_exposes_embedded_eventledger_read_model"),
    retired!("swarm_dashboard_projection_totals_remain_authoritative_when_rows_are_limited", "authoritative totals remain complete when detail rows are limited", "swarm_dashboard_projection_totals_remain_authoritative_when_rows_are_limited"),
    retired!("quiet_background_work_receipts_reject_foreground_or_focus_stealing_work", "quiet-work receipts reject foreground, focus-stealing, and keyboard-capture declarations", "quiet_background_work_receipts_reject_foreground_or_focus_stealing_work"),
    retired!("indexing_leases_and_backend_navigation_are_quiet_by_contract", "indexing leases and backend navigation carry explicit quiet policy", "indexing_leases_and_backend_navigation_are_quiet_by_contract"),
    retired!("real_product_entrypoints_emit_quiet_background_work_receipts", "real indexing/navigation entrypoints emit quiet-work receipts", "real_product_entrypoints_emit_quiet_background_work_receipts"),
    retired!("quiet_entrypoint_denials_happen_before_product_side_effects", "quiet-policy denial occurs before product side effects", "quiet_entrypoint_denials_happen_before_product_side_effects"),
    retired!("mailbox_handoff_requires_write_mailbox_capability", "mailbox handoff requires write-mailbox capability", "mailbox_handoff_requires_write_mailbox_capability"),
    retired!("invalid_mailbox_handoff_claim_ref_does_not_emit_false_receipt", "invalid handoff claim references emit no false receipt", "invalid_mailbox_handoff_claim_ref_does_not_emit_false_receipt"),
    retired!("invalid_checkpoint_refs_do_not_emit_false_receipt", "invalid checkpoint references emit no checkpoint receipt", "invalid_checkpoint_refs_do_not_emit_false_receipt"),
    retired!("concurrent_same_scope_claim_records_one_durable_claim_event", "same-scope contention yields one durable winning claim event", "concurrent_same_scope_claim_records_one_durable_claim_event"),
    retired!("mailbox_navigation_checkpoint_and_recovery_are_restartable_from_postgres", "mailbox handoff, navigation, checkpoint, and recovery survive a real restart", "mailbox_checkpoint_and_recovery_are_restartable_from_surrealdb"),
    retired!("compressed_handoff_template_is_bounded_restartable_and_secret_safe", "compressed handoff is bounded, restartable, and secret-safe", "compressed_handoff_template_is_bounded_restartable_and_secret_safe"),
    retired!("compressed_handoff_redacts_all_dynamic_sections_and_redacted_labels_validate", "all dynamic handoff sections redact secrets while redacted labels remain valid", "compressed_handoff_redacts_all_dynamic_sections_and_redacted_labels_validate"),
    retired!("compressed_handoff_omits_raw_transcript_markers_from_next_step_context", "next-step context excludes raw transcript markers", "compressed_handoff_omits_raw_transcript_markers_from_next_step_context"),
    retired!("compressed_handoff_rejects_secret_like_mandatory_metadata", "secret-like mandatory handoff metadata fails closed", "compressed_handoff_rejects_secret_like_mandatory_metadata"),
    retired!("compressed_handoff_accepts_redacted_url_credentials", "properly redacted URL credentials remain accepted", "compressed_handoff_accepts_redacted_url_credentials"),
    retired!("mt223_missing_checkpoint_recovery_does_not_emit_receipt", "missing checkpoint recovery emits no recovery receipt", "mt223_missing_checkpoint_recovery_does_not_emit_receipt"),
    retired!("mt223_corrupt_checkpoint_payload_hash_does_not_emit_recovery_receipt", "corrupt checkpoint hash emits no recovery receipt", "mt223_corrupt_checkpoint_payload_hash_does_not_emit_recovery_receipt"),
    retired!("mt223_interrupted_indexing_start_failure_leaves_no_swarm_or_kir_receipts", "index-run start failure leaves neither swarm nor indexing-run receipts", "mt223_interrupted_indexing_start_failure_leaves_no_swarm_or_kir_receipts"),
    retired!("mt223_quiet_receipt_failure_rolls_back_index_run_and_lease", "quiet-receipt failure rolls back index run and lease", "mt223_quiet_receipt_failure_rolls_back_index_run_and_lease"),
    retired!("mt223_stale_indexing_lease_reclaim_then_queued_writer_is_promotable", "stale lease reclaim promotes the queued writer", "mt223_stale_indexing_lease_reclaim_then_queued_writer_is_promotable"),
    retired!("mt223_stale_indexing_lease_enqueue_does_not_leapfrog_queued_writer", "new enqueue cannot leapfrog an existing queued writer after stale lease", "mt223_stale_indexing_lease_enqueue_does_not_leapfrog_queued_writer"),
    retired!("mt223_interrupted_editor_save_reclaim_unblocks_rich_document_claim", "interrupted editor-save reclaim unblocks the document claim", "mt223_interrupted_editor_save_reclaim_unblocks_rich_document_claim"),
    retired!("mt223_partial_validation_progress_handoff_is_not_reported_as_pass", "partial validation handoff cannot be reported as pass", "mt223_partial_validation_progress_handoff_is_not_reported_as_pass"),
    retired!("mt223_restart_after_crash_reconstructs_swarm_state_from_postgres", "crash restart reconstructs claims, handoffs, checkpoints, leases, and receipts", "mt223_restart_after_close_reopen_reconstructs_swarm_state_from_surrealdb"),
    retired!("recovery_receipt_authority_failure_does_not_emit_false_receipt", "recovery authority failure emits no false receipt", "recovery_receipt_authority_failure_does_not_emit_false_receipt"),
    retired!("mailbox_handoff_statuses_round_trip_from_postgres", "all mailbox handoff statuses round-trip through durable storage", "mailbox_handoff_statuses_round_trip_from_surrealdb"),
    retired!("raw_secret_like_provider_metadata_is_scrubbed_at_persist_time", "secret-like provider metadata is scrubbed before persistence", "raw_secret_like_provider_metadata_is_scrubbed_at_persist_time"),
    retired!("parallel_indexing_lease_queue_serializes_same_scope_writers_and_reclaims_orphans", "parallel lease queue serializes same-scope writers and reclaims orphans", "parallel_indexing_lease_queue_serializes_same_scope_writers plus mt223_stale_indexing_lease_reclaim_then_queued_writer_is_promotable"),
    retired!("concurrent_same_scope_indexing_lease_records_only_real_outcome_events", "contention records only actual lease outcomes without false events", "concurrent_same_scope_indexing_lease_records_only_real_outcome_events"),
    retired!("explicit_expired_claim_reclaim_records_event_receipt", "explicit expired-claim reclaim records its durable event receipt", "explicit_expired_claim_reclaim_records_event_receipt"),
    retired!("expired_claim_reclaim_rolls_back_if_receipt_insert_fails", "reclaim rolls back when its receipt cannot persist", "expired_claim_reclaim_rolls_back_if_receipt_insert_fails"),
];

#[test]
fn mt141_parallel_swarm_retirement_inventory_is_complete_and_names_exact_successors_or_gaps() {
    assert_eq!(RETIRED_TESTS.len(), 44);
    let names = RETIRED_TESTS
        .iter()
        .map(|entry| entry.retired_test)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(), RETIRED_TESTS.len());
    for entry in RETIRED_TESTS {
        assert!(!entry.retired_behavior.is_empty());
        assert!(!entry.successor_status.is_empty());
        assert!(!entry.successor_status.contains("UNPROVEN"));
        assert!(!entry.successor_status.contains("PARTIAL"));
    }
}

async fn inspect_workspace(
    store: &ParallelSwarmStateRecoveryStore,
    workspace_id: &str,
) -> handshake_core::swarm_orchestration::state_recovery::SwarmEvidenceInspectionSnapshot {
    store
        .inspect_swarm_evidence(SwarmEvidenceInspectionRequest {
            lane: lane_with_kind("evidence-reader", AgentLaneKind::Validator),
            workspace_id: workspace_id.to_owned(),
            limit: 500,
        })
        .await
        .expect("inspect embedded swarm evidence")
}

async fn inspector_row_count(storage: &SurrealStorage, table_name: &str, filter: RowFilter) -> u64 {
    let inspector = storage.test_inspector();
    let table = inspector
        .table_selector(table_name)
        .await
        .expect("select inspector table");
    inspector
        .row_count(&table, filter)
        .await
        .expect("count inspector rows")
}

async fn create_product_workspace(db: &SurrealDatabase, suffix: &str) -> String {
    db.create_workspace(
        &WriteContext::human(None),
        NewWorkspace {
            name: format!("swarm-product-{suffix}-{}", Uuid::now_v7()),
        },
    )
    .await
    .expect("create product workspace")
    .id
}

fn code_index_context(suffix: &str) -> CodeIndexContext {
    CodeIndexContext {
        actor: KernelActor::System(format!("swarm-index-{suffix}")),
        kernel_task_run_id: format!("KTR-SWARM-{suffix}"),
        session_run_id: format!("SR-SWARM-{suffix}"),
        correlation_id: Some(format!("CORR-SWARM-{suffix}")),
    }
}

fn nav_headers(client: reqwest::RequestBuilder, suffix: &str) -> reqwest::RequestBuilder {
    client
        .header("x-hsk-actor-kind", "model_adapter")
        .header("x-hsk-actor-id", format!("swarm-nav-{suffix}"))
        .header(
            "x-hsk-kernel-task-run-id",
            format!("KTR-SWARM-NAV-{suffix}"),
        )
        .header("x-hsk-session-run-id", format!("SR-SWARM-NAV-{suffix}"))
        .header("x-hsk-correlation-id", format!("CORR-SWARM-NAV-{suffix}"))
}

#[tokio::test]
async fn agent_lanes_and_work_claims_are_typed_attributable_and_exclusive() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-claim-{}", Uuid::now_v7());
    let scope = ClaimScope::Worktree {
        worktree_id: "wtc-kernel-009".to_owned(),
    };
    let first_lane = local_lane("claim-a");
    let second_lane = local_lane("claim-b");
    let cloud = cloud_lane("claim-cloud");
    assert!(first_lane
        .capabilities()
        .contains(&AgentCapability::ClaimWorktree));
    assert!(!cloud
        .capabilities()
        .contains(&AgentCapability::WriteLocalIndex));
    assert!(!serde_json::to_string(&cloud)
        .expect("serialize cloud lane")
        .contains("must-not-persist"));

    let first = store
        .claim_work_surface(claim_request(
            &workspace,
            scope.clone(),
            first_lane.clone(),
            "claim-a",
        ))
        .await
        .expect("first claim");
    assert_eq!(first.status, ClaimStatus::Active);
    let held = store
        .claim_work_surface(claim_request(
            &workspace,
            scope.clone(),
            second_lane.clone(),
            "claim-b",
        ))
        .await
        .expect("contending claim");
    assert_eq!(held.status, ClaimStatus::Held);
    assert_eq!(
        held.active_holder.expect("active holder").actor_id,
        first_lane.actor_id
    );
    assert!(store
        .release_claim(&first.claim_id, &first_lane, "handoff complete")
        .await
        .expect("release first claim"));
    let second = store
        .claim_work_surface(claim_request(
            &workspace,
            scope,
            second_lane,
            "claim-b-after",
        ))
        .await
        .expect("claim after release");
    assert_eq!(second.status, ClaimStatus::Active);

    drop(store);
    let (storage, reopened) = reopen_store(&backend).await;
    let evidence = inspect_workspace(&reopened, &workspace).await;
    let released = evidence
        .claims
        .iter()
        .find(|claim| claim.claim_id == first.claim_id)
        .expect("released claim survives reopen");
    assert_eq!(released.status, ClaimStatus::Released);
    assert!(released
        .release_event_ledger_event_id
        .as_deref()
        .is_some_and(|id| id.starts_with("KE-")));
    assert!(evidence
        .claims
        .iter()
        .any(|claim| claim.claim_id == second.claim_id && claim.status == ClaimStatus::Active));
    close_reopened_store(storage, reopened, backend).await;
}

#[tokio::test]
async fn cloud_lane_is_denied_worktree_claim_and_local_index_write_lease() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-cloud-deny-{}", Uuid::now_v7());
    let cloud = cloud_lane("deny");
    assert_invalid_input_contains(
        store
            .claim_work_surface(claim_request(
                &workspace,
                ClaimScope::Worktree {
                    worktree_id: "wt-cloud-denied".to_owned(),
                },
                cloud.clone(),
                "cloud-denied",
            ))
            .await,
        "ClaimWorktree",
    );
    assert_invalid_input_contains(
        store
            .enqueue_indexing_lease(IndexingLeaseRequest {
                workspace_id: workspace.clone(),
                wp_id: "WP-KERNEL-009".to_owned(),
                mt_id: "MT-216".to_owned(),
                scope: ClaimScope::IndexRun {
                    workspace_id: workspace.clone(),
                    source_root_id: "root-cloud-denied".to_owned(),
                },
                lane: cloud,
                session_id: "session-cloud-denied".to_owned(),
                index_run_id: "index-cloud-denied".to_owned(),
                priority: 1,
                ttl_seconds: 600,
                quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
            })
            .await,
        "WriteLocalIndex",
    );
    let evidence = inspect_workspace(&store, &workspace).await;
    assert!(evidence.claims.is_empty());
    assert!(evidence.indexing_leases.is_empty());
    finish_store(store, backend).await;
}

#[tokio::test]
async fn cloud_assistance_output_is_reviewable_attributed_and_non_authoritative() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-cloud-review-{}", Uuid::now_v7());
    let cloud = cloud_lane("review");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            cloud.clone(),
            "cloud-review",
        ))
        .await
        .expect("cloud workspace claim");
    let basis = store
        .record_cloud_fallback_basis(cloud_basis_request(&workspace, &claim.claim_id, "review"))
        .await
        .expect("local fallback basis");
    let receipt = store
        .record_cloud_assistance_output(cloud_assistance_request(
            cloud,
            &workspace,
            &claim.claim_id,
            &basis.fallback_basis_event_id,
            "review",
        ))
        .await
        .expect("reviewable cloud output");
    validate_cloud_assistance_receipt(&receipt).expect("receipt contract validates");
    assert_eq!(receipt.provider, Some(ModelProviderKind::OpenAi));
    assert_eq!(receipt.review_state, "pending_review");
    assert!(receipt.non_authoritative);
    assert!(receipt.requires_promotion);
    assert!(!receipt.authority_mutation_allowed);
    assert!(receipt.promotion_event_id.is_none());
    let evidence = inspect_workspace(&store, &workspace).await;
    assert_eq!(evidence.mailbox_handoffs.len(), 1);
    assert_eq!(evidence.mailbox_handoffs[0].handoff_id, receipt.handoff_id);
    assert_eq!(
        evidence.mailbox_handoffs[0].event_ledger_event_id,
        receipt.handoff_event_ledger_event_id
    );

    let mut forged: CloudAssistanceReceiptV1 = receipt;
    forged.non_authoritative = false;
    forged.authority_mutation_allowed = true;
    let errors = validate_cloud_assistance_receipt(&forged)
        .expect_err("authoritative cloud output must fail validation");
    assert!(errors
        .iter()
        .any(|error| error.contains("non_authoritative=true")));
    assert!(errors
        .iter()
        .any(|error| error.contains("must not allow authority mutation")));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn cloud_assistance_requires_cloud_owned_workspace_claim_and_valid_output_hash() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-cloud-deny-{}", Uuid::now_v7());
    let cloud = cloud_lane("deny");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            cloud.clone(),
            "cloud-deny",
        ))
        .await
        .expect("cloud workspace claim");
    let events_before =
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await;
    let base = cloud_assistance_request(
        cloud,
        &workspace,
        &claim.claim_id,
        "KE-missing-fallback-basis",
        "deny",
    );

    let mut local = base.clone();
    local.from_lane = local_lane("cloud-deny-local");
    assert_invalid_input_contains(
        store.record_cloud_assistance_output(local).await,
        "cloud lane with cloud attribution",
    );
    let mut bad_hash = base.clone();
    bad_hash.output_sha256 = "not-a-sha".to_owned();
    assert_invalid_input_contains(
        store.record_cloud_assistance_output(bad_hash).await,
        "sha256",
    );
    let mut wrong_claim = base.clone();
    wrong_claim.claim_id = format!("PSR-CLAIM-missing-{}", Uuid::now_v7());
    assert_invalid_input_contains(
        store.record_cloud_assistance_output(wrong_claim).await,
        "active cloud-owned workspace claim",
    );
    assert_invalid_input_contains(
        store.record_cloud_assistance_output(base).await,
        "fallback-basis EventLedger",
    );
    assert!(inspect_workspace(&store, &workspace)
        .await
        .mailbox_handoffs
        .is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        events_before
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn cloud_assistance_rejects_loose_or_replayed_fallback_basis() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-cloud-basis-{}", Uuid::now_v7());
    let cloud = cloud_lane("basis");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            cloud.clone(),
            "cloud-basis",
        ))
        .await
        .expect("cloud workspace claim");
    let basis = store
        .record_cloud_fallback_basis(cloud_basis_request(&workspace, &claim.claim_id, "basis"))
        .await
        .expect("tight fallback basis");
    let request = cloud_assistance_request(
        cloud,
        &workspace,
        &claim.claim_id,
        &basis.fallback_basis_event_id,
        "basis",
    );
    let mut mismatch = request.clone();
    mismatch.prompt_sha256 = "0".repeat(64);
    assert_invalid_input_contains(
        store.record_cloud_assistance_output(mismatch).await,
        "fallback-basis EventLedger",
    );
    store
        .record_cloud_assistance_output(request.clone())
        .await
        .expect("first basis use");
    let mut replay = request;
    replay.mailbox_message_id = "message-basis-replay".to_owned();
    replay.output_sha256 = "e".repeat(64);
    replay.body_sha256 = "f".repeat(64);
    assert!(store.record_cloud_assistance_output(replay).await.is_err());
    finish_store(store, backend).await;
}

#[tokio::test]
async fn editor_document_and_graph_claims_serialize_parallel_mutations() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-editor-{}", Uuid::now_v7());
    let first = lane_with_kind("editor-a", AgentLaneKind::Editor);
    let second = lane_with_kind("editor-b", AgentLaneKind::Editor);
    for scope in [
        ClaimScope::RichDocument {
            workspace_id: workspace.clone(),
            document_id: "note-alpha".to_owned(),
        },
        ClaimScope::GraphMutation {
            workspace_id: workspace.clone(),
            graph_id: "graph-main".to_owned(),
        },
    ] {
        let winner = store
            .claim_work_surface(claim_request(
                &workspace,
                scope.clone(),
                first.clone(),
                "editor-a",
            ))
            .await
            .expect("first editor claim");
        let loser = store
            .claim_work_surface(claim_request(&workspace, scope, second.clone(), "editor-b"))
            .await
            .expect("second editor claim");
        assert_eq!(winner.status, ClaimStatus::Active);
        assert_eq!(loser.status, ClaimStatus::Held);
    }
    let parallel = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::RichDocument {
                workspace_id: workspace.clone(),
                document_id: "note-beta".to_owned(),
            },
            second,
            "editor-other-doc",
        ))
        .await
        .expect("different document claim");
    assert_eq!(parallel.status, ClaimStatus::Active);
    finish_store(store, backend).await;
}

#[tokio::test]
async fn non_editor_lanes_cannot_claim_editor_mutation_scopes() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-non-editor-{}", Uuid::now_v7());
    for (kind, scope, capability) in [
        (
            AgentLaneKind::Validator,
            ClaimScope::RichDocument {
                workspace_id: workspace.clone(),
                document_id: "note".to_owned(),
            },
            "EditRichDocument",
        ),
        (
            AgentLaneKind::Indexer,
            ClaimScope::GraphMutation {
                workspace_id: workspace.clone(),
                graph_id: "graph".to_owned(),
            },
            "MutateGraph",
        ),
    ] {
        assert_invalid_input_contains(
            store
                .claim_work_surface(claim_request(
                    &workspace,
                    scope,
                    lane_with_kind(capability, kind),
                    capability,
                ))
                .await,
            capability,
        );
    }
    assert!(inspect_workspace(&store, &workspace)
        .await
        .claims
        .is_empty());
    finish_store(store, backend).await;
}

#[tokio::test]
async fn malformed_editor_mutation_scopes_do_not_persist_claims_or_receipts() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-malformed-{}", Uuid::now_v7());
    let editor = lane_with_kind("editor-malformed", AgentLaneKind::Editor);
    for (scope, expected) in [
        (
            ClaimScope::RichDocument {
                workspace_id: workspace.clone(),
                document_id: String::new(),
            },
            "rich_document.document_id",
        ),
        (
            ClaimScope::GraphMutation {
                workspace_id: format!("other-{workspace}"),
                graph_id: "graph".to_owned(),
            },
            "workspace_id must match",
        ),
    ] {
        assert_invalid_input_contains(
            store
                .claim_work_surface(claim_request(&workspace, scope, editor.clone(), expected))
                .await,
            expected,
        );
    }
    assert!(inspect_workspace(&store, &workspace)
        .await
        .claims
        .is_empty());
    finish_store(store, backend).await;
}

#[tokio::test]
async fn validator_lanes_inspect_swarm_evidence_without_mutating_state() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-validator-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            local_lane("validator-seed"),
            "validator-seed",
        ))
        .await
        .expect("seed validator evidence");
    let validator = lane_with_kind("validator", AgentLaneKind::Validator);
    assert!(validator
        .capabilities()
        .contains(&AgentCapability::InspectEvidence));
    let before = store
        .inspect_swarm_evidence(SwarmEvidenceInspectionRequest {
            lane: validator.clone(),
            workspace_id: workspace.clone(),
            limit: 50,
        })
        .await
        .expect("validator inspection");
    assert_eq!(before.claims[0].claim_id, claim.claim_id);
    assert_invalid_input_contains(
        store
            .claim_work_surface(claim_request(
                &workspace,
                ClaimScope::Workspace {
                    workspace_id: workspace.clone(),
                },
                validator,
                "validator-denied",
            ))
            .await,
        "ClaimWorkspace",
    );
    let after = inspect_workspace(&store, &workspace).await;
    assert_eq!(after.claims, before.claims);
    finish_store(store, backend).await;
}

#[tokio::test]
async fn swarm_dashboard_projection_derives_from_embedded_eventledger_and_is_projection_only() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-dashboard-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            local_lane("dashboard"),
            "dashboard",
        ))
        .await
        .expect("dashboard source claim");
    let projection = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane: lane_with_kind("dashboard-validator", AgentLaneKind::Validator),
            workspace_id: workspace.clone(),
            wp_id: Some("WP-KERNEL-009".to_owned()),
            mt_id: Some("MT-210".to_owned()),
            limit: 100,
        })
        .await
        .expect("embedded dashboard projection");
    validate_swarm_dashboard_projection(&projection).expect("projection contract validates");
    assert!(projection.projection_contract.projection_only);
    assert!(!projection.projection_contract.authority_mutation_allowed);
    assert!(!projection.projection_contract.ui_state_authoritative);
    assert_eq!(projection.totals.claims, 1);
    assert_eq!(projection.claims.len(), 1);
    assert_eq!(projection.claims[0].claim_id, claim.claim_id);
    assert_eq!(projection.source_watermark.event_count, 1);
    assert!(projection.source_watermark.missing_event_refs.is_empty());
    assert_eq!(
        projection.claims[0].source_refs[0]
            .event_ledger_event_id
            .as_deref(),
        claim.event_ledger_event_id.as_deref()
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn swarm_dashboard_projection_totals_remain_authoritative_when_rows_are_limited() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-dashboard-limit-{}", Uuid::now_v7());
    for suffix in ["limit-a", "limit-b"] {
        store
            .claim_work_surface(claim_request(
                &workspace,
                ClaimScope::Worktree {
                    worktree_id: format!("worktree-{suffix}"),
                },
                local_lane(suffix),
                suffix,
            ))
            .await
            .expect("dashboard limited source claim");
    }
    let projection = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane: lane_with_kind("dashboard-limit-validator", AgentLaneKind::Validator),
            workspace_id: workspace,
            wp_id: None,
            mt_id: None,
            limit: 1,
        })
        .await
        .expect("limited dashboard projection");
    validate_swarm_dashboard_projection(&projection).expect("limited projection validates");
    assert_eq!(projection.totals.claims, 2);
    assert_eq!(projection.claims.len(), 1);
    assert_eq!(projection.source_watermark.event_count, 1);
    assert_eq!(projection.warnings.len(), 1);
    assert_eq!(projection.warnings[0].code, "claims_truncated");
    finish_store(store, backend).await;
}

#[tokio::test]
async fn indexing_leases_and_backend_navigation_are_quiet_by_contract() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-quiet-contract-{}", Uuid::now_v7());
    for command in NavigationCommandSet.commands() {
        assert_eq!(
            command.quiet_policy().work_kind,
            QuietBackgroundWorkKind::BackendNavigation
        );
        assert!(command.quiet_policy().all_quiet());
    }
    let navigation = store
        .resolve_backend_navigation_quiet(
            local_lane("quiet-contract-nav"),
            "session-quiet-contract-nav".to_owned(),
            "WP-KERNEL-009".to_owned(),
            "MT-219".to_owned(),
            BackendNavigationCommand::ValidationState,
            json!({"workspace_id": workspace.clone()}),
        )
        .await
        .expect("durable quiet navigation");
    assert!(navigation.resolved.quiet_policy.all_quiet());
    assert!(navigation.quiet_receipt.policy.all_quiet());

    let scope = ClaimScope::IndexRun {
        workspace_id: workspace.clone(),
        source_root_id: "quiet-root".to_owned(),
    };
    let lease = store
        .enqueue_indexing_lease(lease_request(
            &workspace,
            scope.clone(),
            "quiet-contract-index",
            600,
            10,
        ))
        .await
        .expect("quiet indexing lease");
    assert!(lease.quiet_policy.all_quiet());
    let mut loud = lease_request(&workspace, scope, "loud-index", 600, 20);
    loud.quiet_policy.no_os_shell_window = false;
    assert_invalid_input_contains(
        store.enqueue_indexing_lease(loud).await,
        "no_os_shell_window",
    );

    let evidence = inspect_workspace(&store, &workspace).await;
    assert_eq!(evidence.quiet_background_work.len(), 1);
    assert_eq!(evidence.indexing_leases.len(), 1);
    finish_store(store, backend).await;
}

#[tokio::test]
async fn quiet_background_work_receipts_reject_foreground_or_focus_stealing_work() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-quiet-{}", Uuid::now_v7());
    let lane = local_lane("quiet");
    let mut loud = QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::VisualCapture);
    loud.no_focus_steal = false;
    assert_invalid_input_contains(
        store
            .record_quiet_background_work(QuietBackgroundWorkRequest {
                lane: lane.clone(),
                workspace_id: workspace.clone(),
                wp_id: "WP-KERNEL-009".to_owned(),
                mt_id: "MT-219".to_owned(),
                work_kind: QuietBackgroundWorkKind::VisualCapture,
                subject_id: "loud".to_owned(),
                session_id: "session-loud".to_owned(),
                policy: loud,
                evidence_ref: "capture://loud".to_owned(),
            })
            .await,
        "no_focus_steal",
    );
    let quiet = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane,
            workspace_id: workspace.clone(),
            wp_id: "WP-KERNEL-009".to_owned(),
            mt_id: "MT-219".to_owned(),
            work_kind: QuietBackgroundWorkKind::VisualCapture,
            subject_id: "quiet".to_owned(),
            session_id: "session-quiet".to_owned(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::VisualCapture),
            evidence_ref: "capture://headless".to_owned(),
        })
        .await
        .expect("quiet receipt");
    assert!(quiet.policy.all_quiet());
    let evidence = inspect_workspace(&store, &workspace).await;
    assert_eq!(evidence.quiet_background_work.len(), 1);
    assert_eq!(
        evidence.quiet_background_work[0].receipt_id,
        quiet.receipt_id
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mailbox_handoff_requires_write_mailbox_capability() {
    let (backend, store) = recovery_store().await;
    for kind in [AgentLaneKind::Indexer, AgentLaneKind::Editor] {
        assert_invalid_input_contains(
            store
                .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                    from_lane: lane_with_kind("mailbox-denied", kind),
                    to_role: "WP_VALIDATOR".to_owned(),
                    wp_id: "WP-KERNEL-009".to_owned(),
                    mt_id: "MT-211".to_owned(),
                    claim_id: None,
                    mailbox_thread_id: format!("thread-{kind:?}"),
                    mailbox_message_id: "message-denied".to_owned(),
                    status: SwarmReceiptStatus::Blocked,
                    summary: "denied mailbox writer".to_owned(),
                    body_sha256: "c".repeat(64),
                })
                .await,
            "WriteMailbox",
        );
    }
    finish_store(store, backend).await;
}

#[tokio::test]
async fn invalid_mailbox_handoff_claim_ref_does_not_emit_false_receipt() {
    let (backend, store) = recovery_store().await;
    let events_before =
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await;
    let result = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: local_lane("invalid-handoff"),
            to_role: "WP_VALIDATOR".to_owned(),
            wp_id: "WP-KERNEL-009".to_owned(),
            mt_id: "MT-211".to_owned(),
            claim_id: Some(format!("PSR-CLAIM-missing-{}", Uuid::now_v7())),
            mailbox_thread_id: "thread-invalid".to_owned(),
            mailbox_message_id: "message-invalid".to_owned(),
            status: SwarmReceiptStatus::Blocked,
            summary: "invalid claim ref".to_owned(),
            body_sha256: "d".repeat(64),
        })
        .await;
    assert!(result.is_err());
    let snapshot = inspect_workspace(&store, "workspace-not-present").await;
    assert!(snapshot.mailbox_handoffs.is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        events_before
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn invalid_checkpoint_refs_do_not_emit_false_receipt() {
    let (backend, store) = recovery_store().await;
    let events_before =
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await;
    let workspace = format!("workspace-invalid-checkpoint-{}", Uuid::now_v7());
    let result = store
        .record_checkpoint(checkpoint_request(
            local_lane("invalid-checkpoint"),
            &workspace,
            Some(format!("PSR-CLAIM-missing-{}", Uuid::now_v7())),
            Some(format!("PSR-HANDOFF-missing-{}", Uuid::now_v7())),
        ))
        .await;
    assert!(result.is_err());
    assert!(inspect_workspace(&store, &workspace)
        .await
        .checkpoints
        .is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        events_before
    );
    finish_store(store, backend).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_scope_claim_records_one_durable_claim_event() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-claim-race-{}", Uuid::now_v7());
    let scope = ClaimScope::Worktree {
        worktree_id: format!("wt-race-{}", Uuid::now_v7()),
    };
    let barrier = Arc::new(Barrier::new(2));
    let left = {
        let store = store.clone();
        let barrier = Arc::clone(&barrier);
        let workspace = workspace.clone();
        let scope = scope.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            store
                .claim_work_surface(claim_request(
                    &workspace,
                    scope,
                    local_lane("race-a"),
                    "race-a",
                ))
                .await
        })
    };
    let right = {
        let store = store.clone();
        let barrier = Arc::clone(&barrier);
        let workspace = workspace.clone();
        let scope = scope.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            store
                .claim_work_surface(claim_request(
                    &workspace,
                    scope,
                    local_lane("race-b"),
                    "race-b",
                ))
                .await
        })
    };
    let outcomes = [
        left.await.expect("left joins").expect("left outcome"),
        right.await.expect("right joins").expect("right outcome"),
    ];
    assert_eq!(
        outcomes
            .iter()
            .filter(|row| row.status == ClaimStatus::Active)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|row| row.status == ClaimStatus::Held)
            .count(),
        1
    );
    let evidence = inspect_workspace(&store, &workspace).await;
    assert_eq!(evidence.claims.len(), 1);
    assert!(evidence.claims[0]
        .event_ledger_event_id
        .as_deref()
        .is_some_and(|id| id.starts_with("KE-")));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mailbox_checkpoint_and_recovery_are_restartable_from_surrealdb() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-restart-{}", Uuid::now_v7());
    let lane = cloud_lane("restart-source");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            lane.clone(),
            "restart-claim",
        ))
        .await
        .expect("workspace claim");
    let handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: lane.clone(),
            to_role: "WP_VALIDATOR".to_owned(),
            wp_id: "WP-KERNEL-009".to_owned(),
            mt_id: "MT-213".to_owned(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_thread_id: "thread-restart".to_owned(),
            mailbox_message_id: "message-restart".to_owned(),
            status: SwarmReceiptStatus::Blocked,
            summary: "restart checkpoint handoff".to_owned(),
            body_sha256: "a".repeat(64),
        })
        .await
        .expect("mailbox handoff");
    let navigation = store
        .resolve_backend_navigation_quiet(
            lane.clone(),
            "session-navigation".to_owned(),
            "WP-KERNEL-009".to_owned(),
            "MT-213".to_owned(),
            BackendNavigationCommand::ValidationState,
            json!({"workspace_id": workspace.clone()}),
        )
        .await
        .expect("quiet navigation");
    let mut request = checkpoint_request(
        lane,
        &workspace,
        Some(claim.claim_id),
        Some(handoff.handoff_id.clone()),
    );
    request.navigation_command_id = Some(navigation.resolved.command_id.to_owned());
    let checkpoint = store.record_checkpoint(request).await.expect("checkpoint");
    drop(store);

    let (storage, reopened) = reopen_store(&backend).await;
    let recovered = reopened
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("restart-target"),
            "session-restart-target",
        )
        .await
        .expect("recover reopened checkpoint");
    assert_eq!(recovered.checkpoint.payload, json!({"counter": 7}));
    assert!(recovered.receipt.event_ledger_event_id.starts_with("KE-"));
    let evidence = inspect_workspace(&reopened, &workspace).await;
    assert!(evidence
        .mailbox_handoffs
        .iter()
        .any(|row| row.handoff_id == handoff.handoff_id));
    assert!(evidence
        .checkpoints
        .iter()
        .any(|row| row.checkpoint_id == checkpoint.checkpoint_id));
    assert!(evidence
        .recovery_receipts
        .iter()
        .any(|row| row.receipt_id == recovered.receipt.receipt_id));
    assert!(evidence
        .quiet_background_work
        .iter()
        .any(|row| row.receipt_id == navigation.quiet_receipt.receipt_id));
    close_reopened_store(storage, reopened, backend).await;
}

#[tokio::test]
async fn compressed_handoff_template_is_bounded_restartable_and_secret_safe() {
    let (backend, store) = recovery_store().await;
    let mut request = handoff_checkpoint_request("bounded");
    request.touched_files = vec![
        "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_owned(),
        "src/backend/handshake_core/tests/parallel_swarm_state_recovery_tests.rs".to_owned(),
    ];
    request.next_step_context =
        "resume MT-222; secret_token=sk-test-secret-1234567890 must not cross the handoff"
            .to_owned();
    request.payload = json!({
        "provider_chat_transcript": "cloud said secret_token=sk-test-secret-1234567890",
        "large_context": "x".repeat(4096)
    });
    let checkpoint = store
        .record_checkpoint(request)
        .await
        .expect("bounded handoff checkpoint");
    let template = store
        .build_handoff_compression_template(HandoffCompressionRequest {
            requested_by_lane: local_lane("handoff-bounded-reader"),
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            max_chars: 1200,
        })
        .await
        .expect("bounded handoff template");
    validate_handoff_compression_template(&template).expect("bounded template validates");
    assert_eq!(template.checkpoint_id, checkpoint.checkpoint_id);
    assert_eq!(template.payload_sha256, checkpoint.payload_sha256);
    assert!(template.body.len() <= 1200);
    assert!(template.body.contains("MT-222"));
    assert!(template.body.contains("payload_sha256"));
    assert!(template
        .omitted_inputs
        .contains(&"provider_chat_transcript".to_owned()));
    assert!(template
        .omitted_inputs
        .contains(&"raw_checkpoint_payload".to_owned()));
    let serialized = serde_json::to_string(&template).expect("serialize bounded template");
    assert!(!serialized.contains("sk-test-secret"));
    assert!(!serialized.contains(&"x".repeat(256)));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn compressed_handoff_redacts_all_dynamic_sections_and_redacted_labels_validate() {
    let (backend, store) = recovery_store().await;
    let entropy = "aBcD1234EfGh5678IjKl9012MnOp3456";
    let mut request = handoff_checkpoint_request("redaction");
    request.touched_files = vec![format!("src/fixtures/{entropy}.md")];
    request.tests = vec![format!("run-fixture --token {entropy}")];
    request.hbr_rows = vec!["HBR-SWARM-004 api_key=sk-secret-not-real".to_owned()];
    request.next_step_context =
        "resume after secret_token=sk-context-secret-222 was redacted".to_owned();
    let checkpoint = store
        .record_checkpoint(request)
        .await
        .expect("redaction checkpoint");
    let template = store
        .build_handoff_compression_template(HandoffCompressionRequest {
            requested_by_lane: local_lane("handoff-redaction-reader"),
            checkpoint_id: checkpoint.checkpoint_id,
            max_chars: 20_000,
        })
        .await
        .expect("redacted handoff template");
    validate_handoff_compression_template(&template).expect("redacted labels validate");
    let serialized = serde_json::to_string(&template).expect("serialize redacted template");
    assert!(!serialized.contains(entropy));
    assert!(!serialized.contains("sk-secret-not-real"));
    assert!(!serialized.contains("sk-context-secret-222"));
    assert!(serialized.contains("[REDACTED:"));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn compressed_handoff_omits_raw_transcript_markers_from_next_step_context() {
    let (backend, store) = recovery_store().await;
    let mut request = handoff_checkpoint_request("transcript");
    request.next_step_context = "provider_chat_transcript: cloud model said to copy this raw paragraph; full_conversation_history: operator turn follows".to_owned();
    let checkpoint = store
        .record_checkpoint(request)
        .await
        .expect("transcript-marked checkpoint");
    let template = store
        .build_handoff_compression_template(HandoffCompressionRequest {
            requested_by_lane: local_lane("handoff-transcript-reader"),
            checkpoint_id: checkpoint.checkpoint_id,
            max_chars: 20_000,
        })
        .await
        .expect("transcript-safe handoff template");
    assert!(!template.body.contains("cloud model said"));
    assert!(!template.body.contains("provider_chat_transcript:"));
    assert!(!template.body.contains("full_conversation_history:"));
    assert!(template
        .warnings
        .contains(&"next_step_context_omitted_raw_input_marker".to_owned()));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn compressed_handoff_rejects_secret_like_mandatory_metadata() {
    let (backend, store) = recovery_store().await;
    let entropy = "zYxW9876VuTs5432RqPo1098NmLk7654";
    let mut request = handoff_checkpoint_request("mandatory-secret");
    request.session_id = format!("session-{entropy}");
    request.workspace_id = format!("workspace-{entropy}");
    request.git_head = format!("git-{entropy}");
    let checkpoint = store
        .record_checkpoint(request)
        .await
        .expect("mandatory-secret checkpoint");
    let result = store
        .build_handoff_compression_template(HandoffCompressionRequest {
            requested_by_lane: local_lane("handoff-mandatory-reader"),
            checkpoint_id: checkpoint.checkpoint_id,
            max_chars: 20_000,
        })
        .await;
    assert!(matches!(
        result,
        Err(StateRecoveryError::InvalidInput(message))
            if message.contains("mandatory checkpoint metadata")
    ));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn compressed_handoff_accepts_redacted_url_credentials() {
    let (backend, store) = recovery_store().await;
    let mut request = handoff_checkpoint_request("url-redaction");
    request.tests =
        vec!["git clone https://user:hunter2@example.invalid/repo.git before retry".to_owned()];
    let checkpoint = store
        .record_checkpoint(request)
        .await
        .expect("URL credential checkpoint");
    let template = store
        .build_handoff_compression_template(HandoffCompressionRequest {
            requested_by_lane: local_lane("handoff-url-reader"),
            checkpoint_id: checkpoint.checkpoint_id,
            max_chars: 20_000,
        })
        .await
        .expect("URL-redacted handoff template");
    validate_handoff_compression_template(&template).expect("URL-redacted template validates");
    assert!(template
        .body
        .contains("https://[REDACTED:URL_CRED]@example.invalid"));
    assert!(!template.body.contains("hunter2"));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_missing_checkpoint_recovery_does_not_emit_receipt() {
    let (backend, store) = recovery_store().await;
    let events_before =
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await;
    let missing = format!("PSR-CHKPT-missing-{}", Uuid::now_v7());
    let result = store
        .recover_from_checkpoint(&missing, local_lane("missing-recovery"), "session-missing")
        .await;
    assert!(matches!(result, Err(StateRecoveryError::CheckpointNotFound(id)) if id == missing));
    assert!(inspect_workspace(&store, "workspace-missing")
        .await
        .recovery_receipts
        .is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        events_before
    );
    finish_store(store, backend).await;
}

fn lease_request(
    workspace: &str,
    scope: ClaimScope,
    suffix: &str,
    ttl_seconds: i64,
    priority: i32,
) -> IndexingLeaseRequest {
    IndexingLeaseRequest {
        workspace_id: workspace.to_owned(),
        wp_id: "WP-KERNEL-009".to_owned(),
        mt_id: "MT-216".to_owned(),
        scope,
        lane: local_lane(suffix),
        session_id: format!("session-{suffix}"),
        index_run_id: format!("index-{suffix}-{}", Uuid::now_v7()),
        priority,
        ttl_seconds,
        quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
    }
}

#[tokio::test]
async fn parallel_indexing_lease_queue_serializes_same_scope_writers() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-index-{}", Uuid::now_v7());
    let scope = ClaimScope::IndexRun {
        workspace_id: workspace.clone(),
        source_root_id: "root-a".to_owned(),
    };
    let first = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "index-a", 600, 10))
        .await
        .expect("first lease");
    let second = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "index-b", 600, 20))
        .await
        .expect("second lease");
    assert_eq!(first.status, IndexLeaseStatus::Acquired);
    assert_eq!(second.status, IndexLeaseStatus::Queued);
    assert_eq!(
        second.blocked_by_lease_id.as_deref(),
        Some(first.lease_id.as_str())
    );
    store
        .complete_indexing_lease(&first.lease_id, &local_lane("index-a"))
        .await
        .expect("complete first");
    let promoted = store
        .acquire_next_indexing_lease(&scope)
        .await
        .expect("promote next")
        .expect("queued lease");
    assert_eq!(promoted.lease_id, second.lease_id);
    assert_eq!(promoted.status, IndexLeaseStatus::Acquired);
    finish_store(store, backend).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_scope_indexing_lease_records_only_real_outcome_events() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-lease-race-{}", Uuid::now_v7());
    let scope = ClaimScope::IndexRun {
        workspace_id: workspace.clone(),
        source_root_id: "root-race".to_owned(),
    };
    let barrier = Arc::new(Barrier::new(2));
    let left = {
        let s = store.clone();
        let b = Arc::clone(&barrier);
        let w = workspace.clone();
        let c = scope.clone();
        tokio::spawn(async move {
            b.wait().await;
            s.enqueue_indexing_lease(lease_request(&w, c, "lease-a", 600, 10))
                .await
        })
    };
    let right = {
        let s = store.clone();
        let b = Arc::clone(&barrier);
        let w = workspace.clone();
        let c = scope.clone();
        tokio::spawn(async move {
            b.wait().await;
            s.enqueue_indexing_lease(lease_request(&w, c, "lease-b", 600, 20))
                .await
        })
    };
    let rows = [
        left.await.expect("left joins").expect("left lease"),
        right.await.expect("right joins").expect("right lease"),
    ];
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == IndexLeaseStatus::Acquired)
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == IndexLeaseStatus::Queued)
            .count(),
        1
    );
    assert!(rows
        .iter()
        .all(|row| row.event_ledger_event_id.starts_with("KE-")));
    assert_eq!(
        inspect_workspace(&store, &workspace)
            .await
            .indexing_leases
            .len(),
        2
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_stale_indexing_lease_reclaim_then_queued_writer_is_promotable() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-stale-lease-{}", Uuid::now_v7());
    let scope = ClaimScope::IndexRun {
        workspace_id: workspace.clone(),
        source_root_id: "root-stale".to_owned(),
    };
    let stale = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "stale", 1, 10))
        .await
        .expect("stale lease seed");
    let queued = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "queued", 600, 20))
        .await
        .expect("queued lease seed");
    assert_eq!(stale.status, IndexLeaseStatus::Acquired);
    assert_eq!(queued.status, IndexLeaseStatus::Queued);
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let reclaimed = store
        .reclaim_orphaned_indexing_leases()
        .await
        .expect("reclaim expired lease");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].lease_id, stale.lease_id);
    assert_eq!(reclaimed[0].status, IndexLeaseStatus::Reclaimed);
    let promoted = store
        .acquire_next_indexing_lease(&scope)
        .await
        .expect("promote queued lease")
        .expect("queued writer available");
    assert_eq!(promoted.lease_id, queued.lease_id);
    assert_eq!(promoted.status, IndexLeaseStatus::Acquired);
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_stale_indexing_lease_enqueue_does_not_leapfrog_queued_writer() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-fair-{}", Uuid::now_v7());
    let scope = ClaimScope::IndexRun {
        workspace_id: workspace.clone(),
        source_root_id: "root-fair".to_owned(),
    };
    let stale = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "fair-a", 1, 10))
        .await
        .expect("stale acquired writer");
    let queued = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "fair-b", 600, 20))
        .await
        .expect("original queued writer");
    assert_eq!(stale.status, IndexLeaseStatus::Acquired);
    assert_eq!(queued.status, IndexLeaseStatus::Queued);
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let newcomer = store
        .enqueue_indexing_lease(lease_request(&workspace, scope.clone(), "fair-c", 600, 10))
        .await
        .expect("newcomer after stale reclaim");
    assert_eq!(newcomer.status, IndexLeaseStatus::Queued);
    assert_eq!(
        newcomer.blocked_by_lease_id.as_deref(),
        Some(queued.lease_id.as_str())
    );
    let promoted = store
        .acquire_next_indexing_lease(&scope)
        .await
        .expect("promote original queue head")
        .expect("queued writer available");
    assert_eq!(promoted.lease_id, queued.lease_id);
    let evidence = inspect_workspace(&store, &workspace).await;
    assert!(evidence
        .indexing_leases
        .iter()
        .any(|row| row.lease_id == newcomer.lease_id && row.status == IndexLeaseStatus::Queued));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_interrupted_editor_save_reclaim_unblocks_rich_document_claim() {
    let (backend, store) = recovery_store().await;
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace = db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("editor-interrupted-{}", Uuid::now_v7()),
            },
        )
        .await
        .expect("editor workspace");
    let document = db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: workspace.id.clone(),
            document_id: None,
            title: "Interrupted editor save".to_owned(),
            schema_version: "hsk_richdoc_v1".to_owned(),
            content_json: json!({"type": "doc", "content": []}),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
            project_ref: None,
            folder_ref: None,
            authority_label: Some("draft".to_owned()),
            owner_actor_kind: Some("operator".to_owned()),
            owner_actor_id: Some("mt223-editor".to_owned()),
        })
        .await
        .expect("editor document");
    let scope = ClaimScope::RichDocument {
        workspace_id: workspace.id.clone(),
        document_id: document.rich_document_id.clone(),
    };
    let first_lane = lane_with_kind("editor-interrupted-a", AgentLaneKind::Editor);
    let second_lane = lane_with_kind("editor-interrupted-b", AgentLaneKind::Editor);
    let first = store
        .claim_work_surface({
            let mut request = claim_request(&workspace.id, scope.clone(), first_lane, "editor-a");
            request.ttl_seconds = 1;
            request
        })
        .await
        .expect("first editor claim");
    let held = store
        .claim_work_surface(claim_request(
            &workspace.id,
            scope.clone(),
            second_lane.clone(),
            "editor-b-held",
        ))
        .await
        .expect("second editor held");
    assert_eq!(held.status, ClaimStatus::Held);
    let saved_content = json!({"type": "doc", "content": [{"type": "paragraph"}]});
    let saved = db
        .save_knowledge_rich_document_version(
            &document.rich_document_id,
            1,
            saved_content.clone(),
            None,
            None,
            None,
        )
        .await
        .expect("committed save before interruption");
    assert_eq!(saved.doc_version, 2);
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let reclaimed = store
        .reclaim_expired_work_claims(
            &local_lane("editor-reclaimer"),
            "session-editor-reclaimer",
            "lost editor lane after committed save",
        )
        .await
        .expect("reclaim interrupted editor");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].claim_id, first.claim_id);
    assert!(reclaimed[0]
        .reclaim_event_ledger_event_id
        .as_deref()
        .is_some_and(|id| id.starts_with("KE-")));
    let resumed = store
        .claim_work_surface(claim_request(
            &workspace.id,
            scope,
            second_lane,
            "editor-b-resumed",
        ))
        .await
        .expect("second editor resumes");
    assert_eq!(resumed.status, ClaimStatus::Active);
    let retry = db
        .save_knowledge_rich_document_version(
            &document.rich_document_id,
            1,
            saved_content.clone(),
            None,
            None,
            None,
        )
        .await;
    assert!(
        matches!(retry, Err(StorageError::Conflict(message)) if message.contains("expected_version is stale"))
    );
    assert_eq!(
        db.list_knowledge_rich_document_versions(&document.rich_document_id)
            .await
            .expect("document versions")
            .len(),
        2
    );
    drop(db);
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_partial_validation_progress_handoff_is_not_reported_as_pass() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-partial-validation-{}", Uuid::now_v7());
    let lane = local_lane("partial-validation");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            lane.clone(),
            "partial-validation",
        ))
        .await
        .expect("partial validation claim");
    let handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: lane,
            to_role: "WP_VALIDATOR".to_owned(),
            wp_id: "WP-KERNEL-009".to_owned(),
            mt_id: "MT-223".to_owned(),
            claim_id: Some(claim.claim_id),
            mailbox_thread_id: "thread-partial-validation".to_owned(),
            mailbox_message_id: "message-partial-validation".to_owned(),
            status: SwarmReceiptStatus::Progress,
            summary: "checkpoint passed; editor recovery still running".to_owned(),
            body_sha256: "c".repeat(64),
        })
        .await
        .expect("partial validation handoff");
    let projection = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane: lane_with_kind("partial-dashboard", AgentLaneKind::Validator),
            workspace_id: workspace,
            wp_id: Some("WP-KERNEL-009".to_owned()),
            mt_id: Some("MT-223".to_owned()),
            limit: 25,
        })
        .await
        .expect("partial validation projection");
    assert_eq!(
        projection.totals.handoffs_by_status.get("progress"),
        Some(&1)
    );
    assert_eq!(
        projection
            .totals
            .handoffs_by_status
            .get("pass")
            .copied()
            .unwrap_or(0),
        0
    );
    assert!(projection
        .mailbox_handoffs
        .iter()
        .any(|row| row.handoff_id == handoff.handoff_id && row.status == "progress"));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_restart_after_close_reopen_reconstructs_swarm_state_from_surrealdb() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-full-restart-{}", Uuid::now_v7());
    let lane = local_lane("full-restart");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            lane.clone(),
            "full-restart",
        ))
        .await
        .expect("restart claim");
    let handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: lane.clone(),
            to_role: "WP_VALIDATOR".to_owned(),
            wp_id: "WP-KERNEL-009".to_owned(),
            mt_id: "MT-223".to_owned(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_thread_id: "thread-full-restart".to_owned(),
            mailbox_message_id: "message-full-restart".to_owned(),
            status: SwarmReceiptStatus::Progress,
            summary: "restart seed".to_owned(),
            body_sha256: "d".repeat(64),
        })
        .await
        .expect("restart handoff");
    let checkpoint = store
        .record_checkpoint(checkpoint_request(
            lane.clone(),
            &workspace,
            Some(claim.claim_id.clone()),
            Some(handoff.handoff_id.clone()),
        ))
        .await
        .expect("restart checkpoint");
    let lease = store
        .enqueue_indexing_lease(lease_request(
            &workspace,
            ClaimScope::IndexRun {
                workspace_id: workspace.clone(),
                source_root_id: "restart-root".to_owned(),
            },
            "full-restart-lease",
            600,
            10,
        ))
        .await
        .expect("restart lease");
    let quiet = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane,
            workspace_id: workspace.clone(),
            wp_id: "WP-KERNEL-009".to_owned(),
            mt_id: "MT-223".to_owned(),
            work_kind: QuietBackgroundWorkKind::BackendNavigation,
            subject_id: "restart-navigation".to_owned(),
            session_id: "session-full-restart-quiet".to_owned(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::BackendNavigation),
            evidence_ref: "backend-nav://validation_state#restart".to_owned(),
        })
        .await
        .expect("restart quiet receipt");
    drop(store);

    let (storage, reopened) = reopen_store(&backend).await;
    let recovered = reopened
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("full-restart-target"),
            "session-full-restart-target",
        )
        .await
        .expect("recover after reopen");
    let evidence = inspect_workspace(&reopened, &workspace).await;
    assert!(evidence
        .claims
        .iter()
        .any(|row| row.claim_id == claim.claim_id));
    assert!(evidence
        .mailbox_handoffs
        .iter()
        .any(|row| row.handoff_id == handoff.handoff_id));
    assert!(evidence
        .checkpoints
        .iter()
        .any(|row| row.checkpoint_id == checkpoint.checkpoint_id));
    assert!(evidence
        .recovery_receipts
        .iter()
        .any(|row| row.receipt_id == recovered.receipt.receipt_id));
    assert!(evidence
        .indexing_leases
        .iter()
        .any(|row| row.lease_id == lease.lease_id));
    assert!(evidence
        .quiet_background_work
        .iter()
        .any(|row| row.receipt_id == quiet.receipt_id));
    close_reopened_store(storage, reopened, backend).await;
}

#[tokio::test]
async fn mailbox_handoff_statuses_round_trip_from_surrealdb() {
    let (backend, store) = recovery_store().await;
    for status in [
        SwarmReceiptStatus::Started,
        SwarmReceiptStatus::Progress,
        SwarmReceiptStatus::Blocked,
        SwarmReceiptStatus::Pass,
        SwarmReceiptStatus::Fail,
    ] {
        let suffix = format!("{:?}", status).to_ascii_lowercase();
        let handoff = store
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: cloud_lane(&format!("status-{suffix}")),
                to_role: "WP_VALIDATOR".to_owned(),
                wp_id: "WP-KERNEL-009".to_owned(),
                mt_id: "MT-211".to_owned(),
                claim_id: None,
                mailbox_thread_id: format!("thread-{suffix}"),
                mailbox_message_id: format!("message-{suffix}"),
                status,
                summary: format!("round-trip {suffix}"),
                body_sha256: "b".repeat(64),
            })
            .await
            .expect("status handoff");
        assert_eq!(handoff.status, status);
    }
    finish_store(store, backend).await;
}

#[tokio::test]
async fn raw_secret_like_provider_metadata_is_scrubbed_at_persist_time() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-raw-attribution-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            raw_cloud_lane("persist-time"),
            "raw-attribution",
        ))
        .await
        .expect("raw cloud claim");
    drop(store);
    let (storage, reopened) = reopen_store(&backend).await;
    let evidence = inspect_workspace(&reopened, &workspace).await;
    let persisted = evidence
        .claims
        .iter()
        .find(|row| row.claim_id == claim.claim_id)
        .expect("reopened raw-attribution claim");
    let attribution = serde_json::to_string(&persisted.lane.attribution)
        .expect("serialize persisted attribution");
    assert!(!attribution.contains("sk-raw-secret-must-not-persist"));
    assert!(!attribution.contains("raw-token-must-not-persist"));
    assert!(attribution.contains("[REDACTED]"));
    assert!(attribution.contains("org-visible"));
    let inspector = storage.test_inspector();
    let events = inspector
        .table_selector("kernel_event_ledger")
        .await
        .expect("select EventLedger table");
    let payload = events.field("payload").expect("select EventLedger payload");
    let event_id = claim
        .event_ledger_event_id
        .as_deref()
        .expect("claim EventLedger receipt");
    let projected = inspector
        .project(
            &events,
            &[payload],
            RowFilter::IdEquals(event_id.to_owned()),
        )
        .await
        .expect("project claim EventLedger payload");
    assert_eq!(projected.len(), 1);
    let event_payload = projected[0].values["payload"].to_string();
    assert!(!event_payload.contains("sk-raw-secret-must-not-persist"));
    assert!(!event_payload.contains("raw-token-must-not-persist"));
    assert!(event_payload.contains("[REDACTED]"));
    assert!(event_payload.contains("org-visible"));
    close_reopened_store(storage, reopened, backend).await;
}

#[tokio::test]
async fn explicit_expired_claim_reclaim_records_event_receipt() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-explicit-reclaim-{}", Uuid::now_v7());
    let mut request = claim_request(
        &workspace,
        ClaimScope::Worktree {
            worktree_id: format!("worktree-reclaim-{}", Uuid::now_v7()),
        },
        local_lane("reclaim-holder"),
        "reclaim-holder",
    );
    request.ttl_seconds = 1;
    let claim = store
        .claim_work_surface(request)
        .await
        .expect("claim to expire");
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    let reclaimed = store
        .reclaim_expired_work_claims(
            &local_lane("explicit-reclaimer"),
            "session-explicit-reclaimer",
            "explicit stale claim sweep",
        )
        .await
        .expect("explicit reclaim");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].claim_id, claim.claim_id);
    assert_eq!(reclaimed[0].status, ClaimStatus::Reclaimed);
    assert!(reclaimed[0]
        .reclaim_event_ledger_event_id
        .as_deref()
        .is_some_and(|id| id.starts_with("KE-")));
    let evidence = inspect_workspace(&store, &workspace).await;
    assert!(evidence.claims.iter().any(|row| {
        row.claim_id == claim.claim_id
            && row.status == ClaimStatus::Reclaimed
            && row.reclaim_event_ledger_event_id == reclaimed[0].reclaim_event_ledger_event_id
    }));
    finish_store(store, backend).await;
}

#[tokio::test]
async fn claim_authority_failure_after_receipt_rolls_back_eventledger_receipt() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-claim-rollback-{}", Uuid::now_v7());
    store.arm_test_failpoint(StateRecoveryTestFailpoint::ClaimAfterEventBeforeAuthority);
    let result = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            local_lane("claim-rollback"),
            "claim-rollback",
        ))
        .await;
    assert!(
        result.is_err(),
        "the deterministic claim failpoint must fire"
    );
    assert!(inspect_workspace(&store, &workspace)
        .await
        .claims
        .is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        0
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn release_claim_rolls_back_authority_state_if_receipt_insert_fails() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-release-rollback-{}", Uuid::now_v7());
    let lane = local_lane("release-rollback");
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            lane.clone(),
            "release-rollback",
        ))
        .await
        .expect("seed active claim");
    store.arm_test_failpoint(StateRecoveryTestFailpoint::ReleaseAfterAuthorityBeforeEvent);
    let result = store
        .release_claim(&claim.claim_id, &lane, "injected receipt failure")
        .await;
    assert!(
        result.is_err(),
        "the deterministic release failpoint must fire"
    );
    let evidence = inspect_workspace(&store, &workspace).await;
    let persisted = evidence
        .claims
        .iter()
        .find(|row| row.claim_id == claim.claim_id)
        .expect("claim remains after rolled-back release");
    assert_eq!(persisted.status, ClaimStatus::Active);
    assert!(persisted.released_at_utc.is_none());
    assert!(persisted.release_event_ledger_event_id.is_none());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        1
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn swarm_dashboard_projection_api_exposes_embedded_eventledger_read_model() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-dashboard-api-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(claim_request(
            &workspace,
            ClaimScope::Workspace {
                workspace_id: workspace.clone(),
            },
            local_lane("dashboard-api"),
            "dashboard-api",
        ))
        .await
        .expect("seed API projection claim");
    let db = SurrealDatabase::new(backend.storage.clone());
    let state = app_state_for(&db).await;
    let (base, server) = start_server(kernel_api::routes(state)).await;
    let response = reqwest::Client::new()
        .get(format!("{base}/kernel/parallel_swarm/dashboard_projection"))
        .query(&[("workspace_id", workspace.as_str()), ("limit", "100")])
        .send()
        .await
        .expect("request dashboard projection");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = response.json().await.expect("dashboard projection JSON");
    assert_eq!(body["projection_contract"]["projection_only"], true);
    assert_eq!(body["totals"]["claims"].as_u64(), Some(1));
    assert_eq!(body["claims"][0]["claim_id"], claim.claim_id);
    assert_eq!(
        inspect_workspace(&store, &workspace).await.claims,
        vec![claim]
    );
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        1,
        "the projection API must not append authority events"
    );
    server.abort();
    finish_store(store, backend).await;
}

#[tokio::test]
async fn real_product_entrypoints_emit_quiet_background_work_receipts() {
    let (backend, store) = recovery_store().await;
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace = create_product_workspace(&db, "quiet-entrypoints").await;
    let engine = CodeIndexEngine::new(Arc::new(db.clone()));
    let quiet_run = engine
        .start_quiet_run(
            &code_index_context("quiet-entrypoints"),
            &store,
            local_lane("quiet-entrypoints"),
            "WP-KERNEL-009",
            "MT-219",
            &workspace,
            None,
            10,
            600,
        )
        .await
        .expect("real CodeIndexEngine quiet entrypoint");
    assert_eq!(
        quiet_run.quiet_receipt.work_kind,
        QuietBackgroundWorkKind::Indexing
    );

    let state = app_state_for(&db).await;
    let (base, server) = start_server(nav_api::routes(state)).await;
    let response = nav_headers(
        reqwest::Client::new()
            .get(format!("{base}/knowledge/code/symbols"))
            .query(&[("workspace_id", workspace.as_str()), ("name", "missing")]),
        "quiet-entrypoints",
    )
    .send()
    .await
    .expect("real navigation entrypoint");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = response.json().await.expect("navigation response JSON");
    let nav_quiet_receipt_id = body["quiet_background_work_receipt_id"]
        .as_str()
        .expect("navigation quiet receipt id");
    let evidence = inspect_workspace(&store, &workspace).await;
    assert!(evidence.quiet_background_work.iter().any(|row| {
        row.receipt_id == quiet_run.quiet_receipt.receipt_id
            && row.work_kind == QuietBackgroundWorkKind::Indexing
    }));
    assert!(evidence.quiet_background_work.iter().any(|row| {
        row.receipt_id == nav_quiet_receipt_id
            && row.work_kind == QuietBackgroundWorkKind::BackendNavigation
    }));
    server.abort();
    finish_store(store, backend).await;
}

#[tokio::test]
async fn quiet_entrypoint_denials_happen_before_product_side_effects() {
    let (backend, store) = recovery_store().await;
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace = create_product_workspace(&db, "quiet-denial").await;
    let engine = CodeIndexEngine::new(Arc::new(db));
    let result = engine
        .start_quiet_run(
            &code_index_context("quiet-denial"),
            &store,
            lane_with_kind("quiet-denial", AgentLaneKind::Validator),
            "WP-KERNEL-009",
            "MT-219",
            &workspace,
            None,
            10,
            600,
        )
        .await;
    assert!(
        result.is_err(),
        "validator lane must be denied before index start"
    );
    for table in [
        "knowledge_index_runs",
        "knowledge_parallel_indexing_lease_queue",
        "knowledge_agent_quiet_background_work",
        "kernel_event_ledger",
    ] {
        assert_eq!(
            inspector_row_count(&backend.storage, table, RowFilter::All).await,
            0,
            "quiet denial must not write {table}"
        );
    }

    let first = engine
        .start_quiet_run(
            &code_index_context("quiet-contention-a"),
            &store,
            local_lane("quiet-contention-a"),
            "WP-KERNEL-009",
            "MT-219",
            &workspace,
            None,
            10,
            600,
        )
        .await
        .expect("first same-scope quiet run acquires its lease");
    let contended = engine
        .start_quiet_run(
            &code_index_context("quiet-contention-b"),
            &store,
            local_lane("quiet-contention-b"),
            "WP-KERNEL-009",
            "MT-219",
            &workspace,
            None,
            20,
            600,
        )
        .await;
    assert!(
        contended
            .as_ref()
            .is_err_and(|error| error.to_string().contains("did not acquire index lease")),
        "same-scope contention must deny the second product run: {contended:?}"
    );
    let evidence = inspect_workspace(&store, &workspace).await;
    assert_eq!(evidence.indexing_leases.len(), 1);
    assert_eq!(
        evidence.indexing_leases[0].lease_id,
        first.indexing_lease.lease_id
    );
    assert_eq!(evidence.quiet_background_work.len(), 1);
    assert_eq!(
        inspector_row_count(&backend.storage, "knowledge_index_runs", RowFilter::All).await,
        1,
        "contended product run must remove its unleased KIR"
    );
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        3,
        "only the acquired run's KIR, lease, and quiet receipts may remain"
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_corrupt_checkpoint_payload_hash_does_not_emit_recovery_receipt() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-corrupt-checkpoint-{}", Uuid::now_v7());
    let checkpoint = store
        .record_checkpoint(checkpoint_request(
            local_lane("corrupt-checkpoint"),
            &workspace,
            None,
            None,
        ))
        .await
        .expect("seed valid checkpoint");
    store
        .corrupt_checkpoint_payload_for_test(&checkpoint.checkpoint_id, json!({"counter": 8}))
        .await
        .expect("corrupt checkpoint payload without updating its hash");
    let result = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("corrupt-recovery"),
            "session-corrupt-recovery",
        )
        .await;
    assert!(matches!(
        result,
        Err(StateRecoveryError::PayloadHashMismatch { checkpoint_id, .. })
            if checkpoint_id == checkpoint.checkpoint_id
    ));
    assert!(inspect_workspace(&store, &workspace)
        .await
        .recovery_receipts
        .is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        1
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_interrupted_indexing_start_failure_leaves_no_swarm_or_kir_receipts() {
    let (backend, store) = recovery_store().await;
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace = create_product_workspace(&db, "interrupted-index-start").await;
    let engine = CodeIndexEngine::new(Arc::new(db));
    engine.arm_start_run_after_event_failpoint();
    let result = engine
        .start_quiet_run(
            &code_index_context("interrupted-index-start"),
            &store,
            local_lane("interrupted-index-start"),
            "WP-KERNEL-009",
            "MT-223",
            &workspace,
            None,
            10,
            600,
        )
        .await;
    assert!(result.is_err(), "atomic index-start failpoint must fire");
    for table in [
        "knowledge_index_runs",
        "knowledge_parallel_indexing_lease_queue",
        "knowledge_agent_quiet_background_work",
        "kernel_event_ledger",
    ] {
        assert_eq!(
            inspector_row_count(&backend.storage, table, RowFilter::All).await,
            0,
            "interrupted index start must not write {table}"
        );
    }
    finish_store(store, backend).await;
}

#[tokio::test]
async fn mt223_quiet_receipt_failure_rolls_back_index_run_and_lease() {
    let (backend, store) = recovery_store().await;
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace = create_product_workspace(&db, "quiet-receipt-rollback").await;
    let engine = CodeIndexEngine::new(Arc::new(db));
    store.arm_test_failpoint(StateRecoveryTestFailpoint::QuietAfterEventBeforeAuthority);
    let result = engine
        .start_quiet_run(
            &code_index_context("quiet-receipt-rollback"),
            &store,
            local_lane("quiet-receipt-rollback"),
            "WP-KERNEL-009",
            "MT-223",
            &workspace,
            None,
            10,
            600,
        )
        .await;
    assert!(result.is_err(), "quiet receipt failpoint must fire");
    for table in [
        "knowledge_index_runs",
        "knowledge_parallel_indexing_lease_queue",
        "knowledge_agent_quiet_background_work",
        "kernel_event_ledger",
    ] {
        assert_eq!(
            inspector_row_count(&backend.storage, table, RowFilter::All).await,
            0,
            "quiet receipt rollback must remove {table} artifacts"
        );
    }
    finish_store(store, backend).await;
}

#[tokio::test]
async fn recovery_receipt_authority_failure_does_not_emit_false_receipt() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-recovery-rollback-{}", Uuid::now_v7());
    let checkpoint = store
        .record_checkpoint(checkpoint_request(
            local_lane("recovery-rollback-source"),
            &workspace,
            None,
            None,
        ))
        .await
        .expect("seed recovery checkpoint");
    store.arm_test_failpoint(StateRecoveryTestFailpoint::RecoveryAfterEventBeforeAuthority);
    let result = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("recovery-rollback-target"),
            "session-recovery-rollback-target",
        )
        .await;
    assert!(result.is_err(), "recovery receipt failpoint must fire");
    assert!(inspect_workspace(&store, &workspace)
        .await
        .recovery_receipts
        .is_empty());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        1
    );
    finish_store(store, backend).await;
}

#[tokio::test]
async fn expired_claim_reclaim_rolls_back_if_receipt_insert_fails() {
    let (backend, store) = recovery_store().await;
    let workspace = format!("workspace-reclaim-rollback-{}", Uuid::now_v7());
    let mut request = claim_request(
        &workspace,
        ClaimScope::Worktree {
            worktree_id: format!("worktree-reclaim-rollback-{}", Uuid::now_v7()),
        },
        local_lane("reclaim-rollback-holder"),
        "reclaim-rollback-holder",
    );
    request.ttl_seconds = 1;
    let claim = store
        .claim_work_surface(request)
        .await
        .expect("seed expiring claim");
    tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
    store.arm_test_failpoint(StateRecoveryTestFailpoint::ReclaimAfterAuthorityBeforeEvent);
    let result = store
        .reclaim_expired_work_claims(
            &local_lane("reclaim-rollback-runner"),
            "session-reclaim-rollback-runner",
            "injected reclaim receipt failure",
        )
        .await;
    assert!(result.is_err(), "reclaim receipt failpoint must fire");
    let evidence = inspect_workspace(&store, &workspace).await;
    let persisted = evidence
        .claims
        .iter()
        .find(|row| row.claim_id == claim.claim_id)
        .expect("claim remains after rolled-back reclaim");
    assert_eq!(persisted.status, ClaimStatus::Active);
    assert!(persisted.released_at_utc.is_none());
    assert!(persisted.reclaim_event_ledger_event_id.is_none());
    assert_eq!(
        inspector_row_count(&backend.storage, "kernel_event_ledger", RowFilter::All).await,
        1
    );
    finish_store(store, backend).await;
}
