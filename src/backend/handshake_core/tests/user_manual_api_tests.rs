//! WP-KERNEL-009 UserManual route-level proof against the embedded SurrealDB over a
//! loopback listener (quiet; no foreground window):
//! * MT-201 UserManualBackendApi — list / read / search / link routes.
//! * MT-199 UserManualModelQuickstartBundles — per-area bundles.
//! * MT-200 UserManualInAppAccess — access points resolve against live rows.
//! * MT-203 LegacyManualShimTests — bridge route + compatibility receipt.
//! * MT-204 UserManualFreshnessCheck — current / tampered verdicts.
//! * MT-205 UserManualVisualDebugProof — HTML projection selectors +
//!   navigation reachability.
//! * Resync permission gate (cloud_model / unauthenticated write-deny).
//! * THE doc-vs-runtime keystone: every registry surface is probed against
//!   the REAL full product router (`api::routes`) — a documented route the
//!   router does not serve fails the suite.

#[allow(dead_code)]
mod user_manual_support;

// WP-KERNEL-012 MT-109 / LM-RLS-002: the route proofs run as an authenticated record user
// (persisted account session + live native-MCP channel binding).
#[path = "account_session_support/mod.rs"]
mod account_session_support;

use account_session_support::AccountFixture;
use handshake_core::api;
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::Database;
use handshake_core::user_manual::fixtures::{
    restore_page_content_hash, tamper_page_content_hash, unreachable_pages,
};
use handshake_core::user_manual::registry::{probe_path, wp009_surface_registry, SurfaceGroup};
use handshake_core::user_manual::seed::{ensure_seeded, QUICKSTART_AREAS};
use handshake_core::user_manual::store::UserManualStore;
use handshake_core::user_manual::USER_MANUAL_VERSION;
use serde_json::Value;
use std::collections::BTreeSet;
use surrealdb::types::{RecordId, SurrealValue};
use user_manual_support::{app_state_for, manual_test_backend, start_server, ManualTestBackend};

struct ApiFixture {
    backend: ManualTestBackend,
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
    _account: AccountFixture,
}

async fn fixture() -> ApiFixture {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed corpus");
    let account = AccountFixture::install(backend.db.storage()).await;
    let state = app_state_for(&backend.db).await;
    let (base, server) = start_server(api::user_manual::routes(state)).await;
    ApiFixture {
        backend,
        base,
        _server: server,
        http: account.client(),
        _account: account,
    }
}

async fn receipt_exists(db: &SurrealDatabase, subject: &str, event_id: &str) -> bool {
    db.list_kernel_events_for_aggregate("user_manual_entry", subject)
        .await
        .expect("receipt lookup")
        .iter()
        .any(|event| {
            event.event_id == event_id
                && event.event_type == KernelEventType::KnowledgeUserManualEntryRecorded
        })
}

async fn tamper_quickstart_page_link(db: &SurrealDatabase) -> String {
    let store = UserManualStore::new(db);
    let (_, _, anchors) = store
        .get_page_by_slug("quickstart-index")
        .await
        .expect("read quickstart anchors")
        .expect("seeded quickstart page");
    let anchor = anchors
        .into_iter()
        .filter(|anchor| anchor.anchor_kind == "page_link")
        .min_by(|left, right| left.anchor_value.cmp(&right.anchor_value))
        .expect("seeded quickstart page_link anchor");
    let anchor_id = anchor.anchor_id.clone();

    #[derive(SurrealValue)]
    struct Bindings {
        anchor: RecordId,
        replacement: String,
    }

    let record_id = anchor_id.clone();
    let changed: Vec<Value> = db
        .storage()
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE $anchor SET anchor_value = $replacement RETURN AFTER;",
                        Bindings {
                            anchor: RecordId::new("user_manual_anchors", record_id),
                            replacement: "missing-linked-page-for-mt199".to_owned(),
                        },
                    )
                    .await
            })
        })
        .await
        .expect("tamper quickstart page_link");
    assert_eq!(
        changed.len(),
        1,
        "tamper must update one quickstart page_link"
    );
    anchor_id
}

fn assert_internal_diagnostics_and_palmistry_wired(body: &str) {
    assert!(
        body.contains("internal_diagnostics is WIRED through the native producer and Problems projection")
            || body.contains("internal_diagnostics` is WIRED through the native producer and Problems projection"),
        "UserManual must report internal_diagnostics as WIRED through the native producer and Problems projection"
    );
    assert!(
        body.contains("Palmistry is WIRED through the authenticated watcher and survivor recovery importer"),
        "UserManual must report Palmistry as WIRED through the authenticated watcher and survivor recovery importer"
    );
    assert!(
        !body.contains("internal_diagnostics is DEFERRED-with-reason")
            && !body.contains("internal_diagnostics` is DEFERRED-with-reason")
            && !body.contains("until the separate watcher worktree is merged")
            && !body.contains("from follow-up worktrees"),
        "UserManual must not regress to stale diagnostics worktree deferrals"
    );
}

#[tokio::test]
async fn model_lane_schema_user_manual_entry_is_current() {
    let fx = fixture().await;
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/model-lane-schema", fx.base))
        .send()
        .await
        .expect("read ModelLane schema manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);
    assert_eq!(
        page["page"]["manual_version"], USER_MANUAL_VERSION,
        "page detail must surface the canonical manual_version on its nested page row"
    );

    let legacy: Value = fx
        .http
        .get(format!("{}/usermanual/legacy/model-manual", fx.base))
        .send()
        .await
        .expect("read legacy ModelManual bridge")
        .json()
        .await
        .expect("legacy bridge json");
    let legacy_navigation = legacy["canonical"]["pages"]
        .as_array()
        .expect("legacy pages")
        .iter()
        .find(|row| row["slug"] == "model-lane-navigation")
        .expect("legacy bridge exposes model-lane-navigation");
    let canonical_navigation_response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-navigation",
            fx.base
        ))
        .send()
        .await
        .expect("read canonical ModelLane navigation page");
    assert_eq!(canonical_navigation_response.status(), 200);
    let canonical_navigation: Value = canonical_navigation_response
        .json()
        .await
        .expect("canonical ModelLane navigation json");
    let canonical_navigation_page = &canonical_navigation["page"];
    assert_eq!(canonical_navigation_page["slug"], "model-lane-navigation");
    assert_eq!(
        canonical_navigation_page["manual_version"],
        USER_MANUAL_VERSION
    );
    assert!(
        canonical_navigation_page["content_hash"]
            .as_str()
            .is_some_and(|hash| !hash.trim().is_empty()),
        "canonical same-slug detail must expose a nonempty content_hash"
    );
    assert_eq!(legacy_navigation["slug"], "model-lane-navigation");
    assert!(
        legacy_navigation["manual_version"]
            .as_str()
            .is_some_and(|version| !version.trim().is_empty()),
        "legacy same-slug projection must expose a nonempty manual_version"
    );
    assert!(
        legacy_navigation["content_hash"]
            .as_str()
            .is_some_and(|hash| !hash.trim().is_empty()),
        "legacy same-slug projection must expose a nonempty content_hash"
    );
    assert_eq!(
        legacy_navigation["manual_version"], canonical_navigation_page["manual_version"],
        "legacy ModelManual bridge must carry the same-slug canonical page manual_version"
    );
    assert_eq!(
        legacy_navigation["content_hash"], canonical_navigation_page["content_hash"],
        "legacy ModelManual bridge must carry the same-slug canonical page content_hash"
    );

    for required in [
        "Dexterity",
        "ModelLaneRun",
        "ModelLane",
        "ModelLaneMessage",
        "embedded SurrealDB",
        "EventLedger",
        "ArtifactStore",
        "CRDT",
        "Flight Recorder",
        "internal_diagnostics",
        "Palmistry",
        "locus_binding_ref",
        "event_ledger_seq",
        "payload_sha256",
        "replay_order_key",
        "recovery_state",
        "promotion_receipt_ref",
        "memory_pack_ref",
        "determinism_mode",
        "budget_summary_ref",
        "SpawnRequest::with_dexterity_launch",
        "SwarmCoordinator::spawn_session",
        "internal_diagnostics is WIRED through the native producer and Problems projection",
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer",
        "dexterity_launch_records_real_swarm_spawn_session_runtime_path",
        "model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict",
        "model_lane_schema_persists_and_replays_eventledger_rows",
        "model_lane_schema_serializes_competing_terminal_updates",
    ] {
        assert!(
            body.contains(required),
            "model-lane UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let schema_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_launch_tests")
        .expect("manual tool catalog must expose the MT-002 exact proof target");
    assert_eq!(schema_tool["status"], "wired");
    for required in [
        "EventLedger-backed ModelLane rows",
        "runtime spawn_session launch rows",
        "idempotency behavior",
        "replay ordered by event_ledger_seq",
    ] {
        assert!(
            schema_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "schema tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = schema_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneRun",
        "ModelLane",
        "ModelLaneMessage",
        "event_ledger_seq",
        "payload_sha256",
        "replay_order_key",
        "ArtifactStore",
        "CRDT",
        "Flight Recorder",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "schema tool schema_fields must include {required}"
        );
    }
    let common_errors = schema_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("missing locus_binding_ref"));
    assert!(common_errors.contains("idempotency conflict"));
    assert!(common_errors.contains("payload_sha256 is not lowercase sha256 hex"));
    let recovery_steps = schema_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("Run migrations"));
    assert!(recovery_steps.contains("Replay by event_ledger_seq"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

#[tokio::test]
async fn model_lane_promotion_user_manual_entry_is_current() {
    let fx = fixture().await;
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/model-lane-promotion", fx.base))
        .send()
        .await
        .expect("read ModelLane promotion manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity",
        "ModelLanePromotionDecision",
        "NewModelLanePromotionDecision",
        "hsk.model_lane_promotion_decision@1",
        "model_lane_promotion_decision",
        "local_first",
        "cloud_review",
        "cloud_plan_local_execute",
        "parallel_debate",
        "validator_lane",
        "operator_lane",
        "expired",
        "skipped",
        "unsupported",
        "advisory -> promotion_requested -> pending_policy -> pending_approval -> approved -> executing -> executed",
        "advisory -> promotion_requested -> pending_policy -> denied",
        "target_role",
        "target_session",
        "correlation_id",
        "requires_ack",
        "ack_for",
        "promotion_decision_id",
        "promotion_gate_ref",
        "promotion_receipt_ref",
        "promoted_artifact_ref",
        "promoted_artifact_sha256",
        "promoted_artifact_version",
        "final_state",
        "expected_event_ledger_version",
        "current_event_ledger_version",
        "base_snapshot_ref",
        "current_base_snapshot_ref",
        "state_vector",
        "current_state_vector",
        "schema_id",
        "deterministic tie-break rule",
        "canonical_hash_basis",
        "canonical_decision_hash",
        "AggregateVersionMismatch",
        "SchemaMismatch",
        "StaleBase",
        "StaleStateVector",
        "InputRefMismatch",
        "DirectAuthorityMutation",
        "MissingPromotionAuthority",
        "MissingPromotedArtifactBinding",
        "ModelLaneAuthority::Promoted",
        "PromotionGate resolution",
        "ModelLaneStore::new_scoped",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
        "caller payloads cannot select or widen them",
        "foreign account, Principal, session, AccessSpace, or workspace",
        "before any promoted-message or EventLedger authority mutation",
        "mixed resources from incompatible scopes",
        "PromotionGate never widens a derived artifact",
        "Proposal",
        "Critique",
        "Recovery",
        "ContextBundle",
        "recovery_hint_ref",
        "Flight Recorder/EventLedger",
        "internal_diagnostics",
        "Palmistry",
        "DEFERRED-with-reason",
        "model_lane_promotion_appends_eventledger_and_replays_decision",
        "model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation",
        "model_lane_promotion_reordered_inputs_keep_same_decision_hash",
        "model_lane_promotion_preserves_exact_scope_and_denies_foreign_or_mixed_sources",
        "model_lane_promotion_user_manual_entry_is_current",
        "app/src-tauri",
        "TypeScript",
    ] {
        assert!(
            body.contains(required),
            "model-lane promotion UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("worktree_model_lane_live_surreal_tests"));

    let promotion_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "worktree_model_lane_live_surreal_tests")
        .expect("promotion tool entry");
    assert_eq!(promotion_tool["status"], "wired");
    assert!(promotion_tool["description"]
        .as_str()
        .unwrap()
        .contains("advisory-to-authority promotion decisions"));
    for required in [
        "EventLedger-backed ModelLanePromotionDecision rows",
        "typed approved/denied state_history",
        "final_state",
        "DB-derived CRDT base/state-vector denials",
        "schema and aggregate-version denials",
        "phantom input-ref denial",
        "exact promotion_decision_id and promoted artifact binding",
        "direct authority mutation rejection",
        "duplicate idempotency conflict",
        "typed message routing",
        "canonical decision hash stable across reordered input refs",
    ] {
        assert!(
            promotion_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "promotion tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = promotion_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneRoutingPolicy",
        "ModelLaneRoutingMetadata",
        "ModelLanePromotionDecision",
        "NewModelLanePromotionDecision",
        "hsk.model_lane_promotion_decision@1",
        "target_role",
        "target_session",
        "correlation_id",
        "requires_ack",
        "ack_for",
        "canonical_hash_basis",
        "canonical_decision_hash",
        "final_state",
        "expected_event_ledger_version",
        "current_event_ledger_version",
        "base_snapshot_ref",
        "current_base_snapshot_ref",
        "state_vector",
        "current_state_vector",
        "schema_id",
        "promotion_decision_id",
        "promotion_gate_ref",
        "promotion_receipt_ref",
        "promoted_artifact_ref",
        "promoted_artifact_sha256",
        "promoted_artifact_version",
        "event_ledger_seq",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "promotion tool schema_fields must include {required}"
        );
    }
    let common_errors = promotion_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("AggregateVersionMismatch"));
    assert!(common_errors.contains("SchemaMismatch"));
    assert!(common_errors.contains("StaleBase"));
    assert!(common_errors.contains("StaleStateVector"));
    assert!(common_errors.contains("InputRefMismatch"));
    assert!(common_errors.contains("DirectAuthorityMutation"));
    assert!(common_errors.contains("MissingPromotedArtifactBinding"));
    assert!(common_errors.contains("PromotionGate resolution"));
    assert!(common_errors.contains("idempotency conflict"));
    let recovery_steps = promotion_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::replay_promotion_decisions"));
    assert!(recovery_steps.contains("current_base_snapshot_ref/current_state_vector"));
    assert!(recovery_steps.contains("model_lane_schema_registry"));
    assert!(recovery_steps.contains("kernel_event_ledger aggregate version"));
    assert!(recovery_steps.contains("model-lane-message:// ref exists"));
    assert!(recovery_steps.contains("Never write ModelLaneAuthority::Promoted directly"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

#[tokio::test]
async fn model_lane_context_bundle_user_manual_entry_is_current() {
    let fx = fixture().await;
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-context-bundle-handoff",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane ContextBundle manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity",
        "ModelLaneContextBundleHandoff",
        "NewModelLaneContextBundleArtifactBinding",
        "ModelLaneContextBundleArtifactBindingRecord",
        "NewModelLaneContextBundleHandoff",
        "ModelLaneContextBundleHandoffRecord",
        "ModelLaneDownstreamContextBundle",
        "ModelLaneCrdtHandoffMetadata",
        "ModelLaneLoomHandoffRef",
        "ModelLaneMemoryPackHandoffRef",
        "ModelLaneStore::record_context_bundle_artifact_binding",
        "ModelLaneStore::record_context_bundle_handoff",
        "ModelLaneStore::replay_context_bundle_handoffs",
        "ModelLaneStore::consume_context_bundle_for_downstream",
        "SwarmCoordinator::context_bundle_for_downstream_lane",
        "SwarmCoordinator::invoke_downstream_context_bundle",
        "ModelAdapterRequest",
        "model_lane_context_bundle_id_for_handoff",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "model_lane_context_bundle_artifacts",
        "model_lane_context_bundle_handoff",
        "ARTIFACT_STORED",
        "CONTEXT_BUNDLE_RECORDED",
        "context_bundle_id",
        "context_bundle_hash",
        "artifact_binding_hash",
        "artifact_manifest_ref",
        "artifact_payload_ref",
        "payload_json",
        "downstream_lane_id",
        "work_packet_id",
        "micro_task_id",
        "task_board_id",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "ModelLaneStore::new_scoped",
        "server-derived scope is authority",
        "default-denied at SQL and post-decode boundaries",
        "session or AccessSpace switch cannot retarget",
        "Foreign account, Principal, session, AccessSpace, or workspace contexts are default-denied",
        "Mixed-scope source derivation is rejected before a store or consume can be constructed",
        "downstream consumption never unions records across source scopes",
        "to_kernel_context_bundle",
        "CTX-<hash>",
        "selected",
        "rejected",
        "unresolved",
        "superseded",
        "source_message_id",
        "artifact_ref",
        "artifact_sha256",
        "content_hash",
        "hsk.model_lane_crdt_payload@1",
        "update_bytes_ref",
        "update_sha256",
        "state_vector",
        "base_snapshot_ref",
        "materialized_projection_hash",
        "knowledge_crdt_agent_lane_leases",
        "ModelLaneCrdtAuthorityBinding",
        "lease_id",
        "lease_correlation_id",
        "lease_scope_kind",
        "lease_scope_id",
        "lease_claimed_at_utc",
        "lease_expires_at_utc",
        "lease_admitted_at_utc",
        "released_at_utc IS NULL",
        "claimed_at_utc <= lease_admitted_at_utc < expires_at_utc",
        "MODEL_RESPONSE_RECORDED",
        "multiple covering workspace/document matches",
        "later release or natural expiry",
        "historical replay",
        "stored-authority validators",
        "Old rows with a missing binding are rejected",
        "CONTEXT_BUNDLE_RECORDED",
        "recompute its canonical",
        "transaction advisory-lock domain",
        "workspace:<workspace_id>",
        "crdt_document:<crdt_document_id>",
        "second covering claim",
        "mutable projection/EventLedger disagreement",
        "replay_metadata",
        "replay_order_key",
        "dependency_update_ids",
        "schema_version",
        "forged values fail closed before any handoff row is written",
        "Yjs-compatible",
        "yjs_update_v1",
        "promotion-gate://model-lane-message/<source_message_id>",
        "eventledger://<update_event_id>",
        "authority_effect = advisory_only",
        "event_ledger_evidence_ref",
        "flight_recorder_evidence_ref",
        "memory_pack_ref",
        "memory_pack_hash",
        "scope_tag",
        "review_status",
        "cloud_safe",
        "classification",
        "projection_ref",
        "local_only_context",
        "hidden provider memory",
        "not replayable",
        "cloud_safe = false",
        "ArtifactStore/EventLedger authority",
        "review_status must be reviewed",
        "projection_ref",
        "memory_pack_refs exceeds bounded FEMS limit",
        "loom_refs exceeds bounded limit",
        "Flight Recorder/EventLedger",
        "internal_diagnostics",
        "Palmistry",
        "DEFERRED-with-reason",
        "model_lane_context_bundle_persists_selection_state_and_replays",
        "model_lane_context_bundle_missing_artifact_ref_fails_closed",
        "model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable",
        "model_lane_context_bundle_consume_preserves_exact_scope_and_denies_foreign_or_mixed_sources",
        "model_lane_context_bundle_user_manual_entry_is_current",
        "prompt-only",
        "hidden-memory",
    ] {
        assert!(
            body.contains(required),
            "model-lane ContextBundle UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("worktree_model_lane_live_surreal_tests"));
    assert!(tool_ids.contains("worktree_model_lane_live_surreal_tests"));

    let handoff_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "worktree_model_lane_live_surreal_tests")
        .expect("ContextBundle tool entry");
    assert_eq!(handoff_tool["status"], "wired");
    assert!(handoff_tool["description"]
        .as_str()
        .unwrap()
        .contains("model-to-model ContextBundle handoff persistence"));
    for required in [
        "EventLedger-backed ModelLaneContextBundleArtifactBindingRecord",
        "ARTIFACT_STORED payload stamping",
        "model_lane_context_bundle_artifacts authority rows",
        "artifact_manifest_ref/artifact_payload_ref/payload_json/artifact_binding_hash",
        "EventLedger-backed ModelLaneContextBundleArtifactBindingRecord and ModelLaneContextBundleHandoff rows",
        "downstream-only ModelLaneDownstreamContextBundle consumption",
        "ModelLaneStore::consume_context_bundle_for_downstream",
        "SwarmCoordinator::context_bundle_for_downstream_lane",
        "SwarmCoordinator::invoke_downstream_context_bundle adapter invocation",
        "to_kernel_context_bundle conversion",
        "ContextBundle V1 CTX-<hash> identity",
        "selected/rejected/unresolved/superseded selection states",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "fail-closed missing source",
        "artifact_ref/artifact_sha256/content_hash mismatch",
        "ArtifactStore/EventLedger authority",
        "cloud-safe FEMS MemoryPack enforcement",
        "local_only_context cloud rejection",
        "review_status reviewed, operator_reviewed, or validator_reviewed",
        "hidden provider/session memory rejection including projection_ref and normalized hidden-memory URI checks",
        "memory_pack_refs exceeds bounded FEMS limit",
        "CRDT state_vector/base_snapshot_ref/update_bytes_ref validation",
        "Yjs-compatible format yjs_update_v1 only",
        "Loom event_ledger_evidence_ref and flight_recorder_evidence_ref replay",
        "loom_refs exceeds bounded limit",
        "duplicate idempotency returning the original context_bundle_hash",
        "exact five-field resource scope retained on ContextBundle and derived artifact refs",
        "SQL plus post-decode default-deny across owner/Principal/authenticated session/AccessSpace/workspace switches",
    ] {
        assert!(
            handoff_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "ContextBundle tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = handoff_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneContextBundleArtifactBindingRecord",
        "NewModelLaneContextBundleArtifactBinding",
        "ModelLaneContextBundleHandoffRecord",
        "NewModelLaneContextBundleHandoff",
        "ModelLaneDownstreamContextBundle",
        "ModelLaneCrdtHandoffMetadata",
        "ModelLaneLoomHandoffRef",
        "ModelLaneMemoryPackHandoffRef",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "model_lane_context_bundle_artifacts",
        "context_bundle_id",
        "context_bundle_hash",
        "artifact_binding_hash",
        "artifact_manifest_ref",
        "artifact_payload_ref",
        "payload_json",
        "downstream_lane_id",
        "SwarmCoordinator::invoke_downstream_context_bundle",
        "ModelAdapterRequest",
        "source_message_id",
        "artifact_ref",
        "artifact_sha256",
        "content_hash",
        "work_packet_id",
        "micro_task_id",
        "task_board_id",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "to_kernel_context_bundle",
        "CTX-<hash>",
        "selected",
        "rejected",
        "unresolved",
        "superseded",
        "update_bytes_ref",
        "update_sha256",
        "state_vector",
        "base_snapshot_ref",
        "materialized_projection_hash",
        "replay_metadata",
        "replay_order_key",
        "dependency_update_ids",
        "schema_version",
        "yjs_update_v1",
        "validation_runner_ref",
        "authority_effect",
        "event_ledger_evidence_ref",
        "flight_recorder_evidence_ref",
        "memory_pack_ref",
        "memory_pack_hash",
        "scope_tag",
        "review_status",
        "cloud_safe",
        "classification",
        "projection_ref",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "ContextBundle tool schema_fields must include {required}"
        );
    }
    let common_errors = handoff_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("artifact binding must exist"));
    assert!(common_errors.contains("artifact_payload_ref must match artifact_ref"));
    assert!(common_errors.contains("payload_json sha256 must match content_hash"));
    assert!(common_errors.contains("source_message_id is not replayable"));
    assert!(common_errors.contains("handoff.artifact_ref must match source.payload_ref"));
    assert!(common_errors.contains("handoff.artifact_sha256 must match source.payload_sha256"));
    assert!(common_errors.contains("ArtifactStore/EventLedger authority"));
    assert!(common_errors.contains("downstream_lane_id is required"));
    assert!(common_errors.contains("work_packet_id is required"));
    assert!(common_errors.contains("micro_task_id is required"));
    assert!(common_errors.contains("task_board_id is required"));
    assert!(common_errors.contains("cloud_safe"));
    assert!(common_errors.contains("local_only_context"));
    assert!(common_errors.contains("hidden provider/session memory"));
    assert!(common_errors.contains("projection_ref"));
    assert!(common_errors.contains("review_status"));
    assert!(common_errors.contains("memory_pack_refs exceeds bounded FEMS limit"));
    assert!(common_errors.contains("crdt_payload.update_bytes_ref"));
    assert!(common_errors.contains("Yjs-compatible format yjs_update_v1"));
    assert!(common_errors.contains("advisory_only"));
    assert!(common_errors.contains("loom_refs exceeds bounded limit"));
    assert!(common_errors.contains("missing or mismatched exact resource scope is default-denied"));
    let recovery_steps = handoff_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::record_context_bundle_artifact_binding"));
    assert!(recovery_steps.contains("model_lane_context_bundle_artifacts.payload_json"));
    assert!(recovery_steps.contains("ModelLaneStore::replay_context_bundle_handoffs"));
    assert!(recovery_steps.contains("ModelLaneStore::consume_context_bundle_for_downstream"));
    assert!(recovery_steps.contains("SwarmCoordinator::context_bundle_for_downstream_lane"));
    assert!(recovery_steps.contains("SwarmCoordinator::invoke_downstream_context_bundle"));
    assert!(recovery_steps.contains("ModelAdapterRequest.context_bundle"));
    assert!(recovery_steps.contains("ModelLaneDownstreamContextBundle"));
    assert!(recovery_steps.contains("to_kernel_context_bundle"));
    assert!(recovery_steps.contains("CTX-<hash>"));
    assert!(recovery_steps.contains("kernel_event_ledger"));
    assert!(recovery_steps.contains("source ModelLaneMessage first"));
    assert!(recovery_steps.contains(
        "original server-derived owner/Principal/authenticated session/AccessSpace/workspace context"
    ));
    assert!(recovery_steps.contains("artifact_ref/artifact_sha256/content_hash"));
    assert!(recovery_steps.contains("cloud_safe = true"));
    assert!(recovery_steps.contains("classification other than local_only_context"));
    assert!(recovery_steps.contains("update_bytes_ref, state_vector, and base_snapshot_ref"));
    assert!(recovery_steps.contains("EventLedger and Flight Recorder evidence refs"));
    assert!(recovery_steps.contains("direct Flight Recorder event emission"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

#[tokio::test]
async fn model_lane_navigation_user_manual_entries_are_current() {
    let fx = fixture().await;
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-navigation",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane navigation manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity ModelLane Backend Navigation",
        "hsk.model_lane_navigation@1",
        "ModelLaneNavigationProjection",
        "ModelLaneStore::navigation_by_run",
        "ModelLaneStore::navigation_by_lane",
        "ModelLaneStore::navigation_by_message",
        "ModelLaneStore::navigation_by_artifact_or_context",
        "ModelLaneStore::navigation_by_trace",
        "ModelLaneStore::navigation_by_diagnostics",
        "ModelLaneStore::navigation_by_recovery",
        "ModelLaneStore::navigation_by_lookup",
        "GET /swarm/model-lanes/navigation/runs/{run_id}",
        "GET /swarm/model-lanes/navigation/lanes/{lane_id}",
        "GET /swarm/model-lanes/navigation/messages/{message_id}",
        "GET /swarm/model-lanes/navigation/artifacts",
        "GET /swarm/model-lanes/navigation/traces/{trace_id}",
        "GET /swarm/model-lanes/navigation/diagnostics/{run_id}",
        "GET /swarm/model-lanes/navigation/recovery/{run_id}",
        "GET /swarm/model-lanes/navigation/lookup",
        "model_session_id",
        "session_id",
        "wp_id",
        "mt_id",
        "task_board_id",
        "artifact_ref",
        "context_bundle_id",
        "Locus",
        "Loom",
        "FEMS",
        "MemoryPack",
        "EventLedger event IDs/sequences",
        "trace_id",
        "span_id",
        "error-code selectors",
        "embedded SurrealDB",
        "kernel_event_ledger",
        "Flight Recorder",
        "Palmistry",
        "model_lane_navigation_routes_return_run_lane_message_artifact_trace_and_recovery",
        "model_lane_navigation_user_manual_registry_rows_match_runtime_routes",
        "model_lane_navigation_user_manual_entries_are_current",
    ] {
        assert!(
            body.contains(required),
            "model-lane navigation UserManual page must mention {required}"
        );
    }

    let registry_ids: std::collections::BTreeSet<_> = wp009_surface_registry()
        .iter()
        .filter(|surface| surface.group == SurfaceGroup::ModelLaneNavigation)
        .map(|surface| surface.surface_id)
        .collect();
    for required in [
        "model_lane.navigation.run",
        "model_lane.navigation.lane",
        "model_lane.navigation.message",
        "model_lane.navigation.artifact_context",
        "model_lane.navigation.trace_span",
        "model_lane.navigation.diagnostic_tier",
        "model_lane.navigation.recovery",
        "model_lane.navigation.lookup",
    ] {
        assert!(
            registry_ids.contains(required),
            "navigation registry must include {required}"
        );
    }

    let wp009_tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp009_surface&limit=500",
            fx.base
        ))
        .send()
        .await
        .expect("list wp009 tools")
        .json()
        .await
        .expect("tools json");
    let wp009_tool_ids: std::collections::BTreeSet<_> = wp009_tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    for required in registry_ids {
        assert!(
            wp009_tool_ids.contains(required),
            "wp009 tool entries must include navigation surface {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let navigation_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_navigation_api_tests")
        .expect("model lane navigation tool entry");
    assert_eq!(navigation_tool["status"], "wired");
    for required in [
        "ModelLaneNavigationProjection",
        "ModelLaneNavigationLookup",
        "ModelLaneStore::navigation_by_lookup",
        "GET /swarm/model-lanes/navigation/lookup",
        "model_lane.navigation.lookup",
        "event_ledger_event_id",
        "event_ledger_seq",
        "trace_id",
        "span_id",
        "error_code",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
        "Locus",
        "Loom",
        "FEMS",
        "ContextBundle",
        "MemoryPack",
    ] {
        assert!(
            navigation_tool.to_string().contains(required),
            "navigation tool entry must mention {required}"
        );
    }
}

fn loom_router_surfaces_from_source() -> BTreeSet<(String, String)> {
    let source = include_str!("../src/api/loom.rs");
    let mut surfaces = BTreeSet::new();
    let mut remaining = source;

    while let Some(route_start) = remaining.find(".route(") {
        remaining = &remaining[route_start + ".route(".len()..];
        let Some(path_start) = remaining.find('"') else {
            continue;
        };
        let after_path_start = &remaining[path_start + 1..];
        let Some(path_end) = after_path_start.find('"') else {
            continue;
        };
        let path = &after_path_start[..path_end];
        let after_path = &after_path_start[path_end + 1..];
        let Some(argument_start) = after_path.find(',') else {
            continue;
        };
        let handler_and_tail = &after_path[argument_start + 1..];
        let mut route_depth = 1_i32;
        let mut route_end = handler_and_tail.len();
        for (offset, character) in handler_and_tail.char_indices() {
            match character {
                '(' => route_depth += 1,
                ')' => {
                    route_depth -= 1;
                    if route_depth == 0 {
                        route_end = offset;
                        break;
                    }
                }
                _ => {}
            }
        }
        let route_block = handler_and_tail[..route_end].trim_start();

        for (needle, method) in [
            ("get(", "GET"),
            (".get(", "GET"),
            ("post(", "POST"),
            (".post(", "POST"),
            ("put(", "PUT"),
            (".put(", "PUT"),
            ("patch(", "PATCH"),
            (".patch(", "PATCH"),
            ("delete(", "DELETE"),
            (".delete(", "DELETE"),
        ] {
            let mounted = if needle.starts_with('.') {
                route_block.contains(needle)
            } else {
                route_block.starts_with(needle)
            };
            if mounted {
                surfaces.insert((method.to_string(), path.to_string()));
            }
        }

        remaining = after_path;
    }

    surfaces
}

/// MT-195 negative guard: a Loom route mounted in `api::loom` must be present
/// in the UserManual registry. This catches the opposite direction from the
/// doc-vs-runtime probe: runtime route -> manual inventory.
#[test]
fn mtdoc_every_loom_router_route_is_in_surface_registry() {
    let registry: BTreeSet<(String, String)> = wp009_surface_registry()
        .iter()
        .map(|s| (s.method.to_string(), s.route.to_string()))
        .collect();
    let missing: Vec<_> = loom_router_surfaces_from_source()
        .into_iter()
        .filter(|surface| !registry.contains(surface))
        .collect();
    assert!(
        missing.is_empty(),
        "mounted Loom routes missing from wp009_surface_registry: {missing:?}"
    );
}

#[test]
fn mt027_results_manual_matches_required_json_extractor_contract() {
    let surface = wp009_surface_registry()
        .iter()
        .find(|surface| surface.surface_id == "loom.views.definitions.results")
        .expect("MT027 results surface in canonical registry");
    assert!(
        surface.expected_input.contains("required JSON body")
            && surface.expected_input.contains("use {} for defaults")
            && !surface.expected_input.contains("optional JSON"),
        "Axum Json<BlockViewResultsRequest> rejects a missing body; manual inputs were {:?}",
        surface.expected_input
    );
}

/// MT-201: pages list + read; an anonymous read works (bootstrap surface) and
/// RETURNS a real, persisted bootstrap receipt.
#[tokio::test]
async fn mt201_pages_list_and_read_with_bootstrap_receipt() {
    let fx = fixture().await;
    let list: Value = fx
        .http
        .get(format!("{}/usermanual/pages", fx.base))
        .send()
        .await
        .expect("list pages")
        .json()
        .await
        .expect("list json");
    assert_eq!(list["manual_version"], USER_MANUAL_VERSION);
    assert!(
        list["count"].as_u64().unwrap() >= 24,
        "expected the full seeded corpus, got {}",
        list["count"]
    );

    let response = fx
        .http
        .get(format!("{}/usermanual/pages/manual-toc", fx.base))
        .send()
        .await
        .expect("read page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("page json");
    assert_eq!(page["page"]["slug"], "manual-toc");
    assert!(!page["sections"].as_array().unwrap().is_empty());
    assert_eq!(page["bootstrap_identity_used"], true);
    let receipt = page["bootstrap_receipt_event_id"]
        .as_str()
        .expect("receipt id");
    assert!(
        receipt_exists(&fx.backend.db, "manual-toc", receipt).await,
        "bootstrap receipt {receipt} must be persisted in the EventLedger"
    );

    // Unknown slug: typed 404, not a router fallback.
    let missing = fx
        .http
        .get(format!("{}/usermanual/pages/zzz-no-such-page", fx.base))
        .send()
        .await
        .expect("missing page");
    assert_eq!(missing.status(), 404);
    let body: Value = missing.json().await.expect("404 json");
    assert_eq!(body["error"], "not_found");
}

#[tokio::test]
async fn mt027_notes_manual_seeds_block_collection_workflow_in_embedded_surrealdb() {
    let fx = fixture().await;
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/notes-loom-surface", fx.base))
        .send()
        .await
        .expect("read notes manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("notes manual json");
    let sections = page["sections"]
        .as_array()
        .expect("notes manual sections array");
    let section = sections
        .iter()
        .find(|section| {
            section["title"]
                .as_str()
                .is_some_and(|title| title.contains("Block Collection Views"))
        })
        .expect("dedicated Block Collection Views section");
    assert_eq!(section["section_kind"], "workflows");
    let body = section["body_md"]
        .as_str()
        .expect("Block Collection Views body");
    for needle in [
        "menu.view.open-block-collections",
        "bcv.new-view",
        "bcv.table.sort.*",
        "bcv.kanban.card.*",
        "bcv.calendar.apply-range",
        "No blocks match this view.",
        "bcv.retry",
        "transactional embedded SurrealDB outbox",
        "tests/run_mt027_argus_proof.ps1",
    ] {
        assert!(
            body.contains(needle),
            "canonical notes manual must document MT-027 fact {needle:?}"
        );
    }
}

/// MT-201: search hits pages/sections/tools; an empty query is a typed 400.
#[tokio::test]
async fn mt201_search_finds_pages_and_tools() {
    let fx = fixture().await;
    let found: Value = fx
        .http
        .get(format!("{}/usermanual/search?q=backlinks", fx.base))
        .send()
        .await
        .expect("search")
        .json()
        .await
        .expect("search json");
    assert!(
        found["count"].as_u64().unwrap() > 0,
        "seeded corpus documents backlinks"
    );

    let empty = fx
        .http
        .get(format!("{}/usermanual/search", fx.base))
        .send()
        .await
        .expect("empty search");
    assert_eq!(empty.status(), 400);
    let body: Value = empty.json().await.expect("400 json");
    assert_eq!(body["error"], "bad_request");
}

/// MT-201: the tool catalog resolves by id with failure modes + recovery
/// steps; this is also the MT-112 closure proof — the /knowledge/code/*
/// routes are manual-registered and readable.
#[tokio::test]
async fn mt201_tools_list_and_read_resolve() {
    let fx = fixture().await;
    let tools: Value = fx
        .http
        .get(format!("{}/usermanual/tools?origin=wp009_surface", fx.base))
        .send()
        .await
        .expect("list tools")
        .json()
        .await
        .expect("tools json");
    let count = tools["count"].as_u64().unwrap();
    assert_eq!(
        count as usize,
        wp009_surface_registry().len(),
        "one wp009 tool entry per registry surface"
    );

    // MT-112 closure: every /knowledge/code/* nav route is a readable entry.
    for surface in wp009_surface_registry()
        .iter()
        .filter(|s| s.route.starts_with("/knowledge/code/"))
    {
        let tool: Value = fx
            .http
            .get(format!(
                "{}/usermanual/tools/{}",
                fx.base, surface.surface_id
            ))
            .send()
            .await
            .expect("read code-nav tool")
            .json()
            .await
            .expect("tool json");
        assert_eq!(tool["tool"]["http_route"], surface.route);
        assert_eq!(tool["tool"]["http_method"], surface.method);
        assert!(!tool["tool"]["common_errors"].as_array().unwrap().is_empty());
        assert!(!tool["tool"]["recovery_steps"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    let missing = fx
        .http
        .get(format!("{}/usermanual/tools/zzz.no.such.tool", fx.base))
        .send()
        .await
        .expect("missing tool");
    assert_eq!(missing.status(), 404);

    // Legacy entries imported (deterministic 10.15.8 mapping).
    let legacy: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=legacy_model_manual&limit=500",
            fx.base
        ))
        .send()
        .await
        .expect("legacy tools")
        .json()
        .await
        .expect("legacy json");
    assert!(
        legacy["count"].as_u64().unwrap() > 50,
        "legacy manifest imported"
    );
}

/// MT-201: page linking — outbound page links and inbound backlinks resolve.
#[tokio::test]
async fn mt201_page_links_resolve() {
    let fx = fixture().await;
    let links: Value = fx
        .http
        .get(format!("{}/usermanual/pages/manual-toc/links", fx.base))
        .send()
        .await
        .expect("toc links")
        .json()
        .await
        .expect("links json");
    let outbound: Vec<&str> = links["outbound"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(outbound.contains(&"quickstart-index"));
    assert!(outbound.contains(&"state-recovery-guide"));

    let recovery: Value = fx
        .http
        .get(format!(
            "{}/usermanual/pages/state-recovery-guide/links",
            fx.base
        ))
        .send()
        .await
        .expect("recovery links")
        .json()
        .await
        .expect("recovery json");
    let inbound: Vec<&str> = recovery["inbound"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        inbound.contains(&"manual-toc"),
        "TOC links every page; inbound must show it (got {inbound:?})"
    );
}

/// MT-199: every contract area returns a bundled quickstart with linked
/// pages inlined; an unknown area is a typed 404.
#[tokio::test]
async fn mt199_quickstart_bundles_resolve_all_areas() {
    let fx = fixture().await;
    for area in QUICKSTART_AREAS {
        let bundle: Value = fx
            .http
            .get(format!("{}/usermanual/quickstarts/{area}", fx.base))
            .send()
            .await
            .expect("quickstart")
            .json()
            .await
            .expect("quickstart json");
        assert_eq!(bundle["area"], *area);
        assert!(
            !bundle["quickstart"]["sections"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{area} quickstart has sections"
        );
        assert!(
            !bundle["linked_pages"].as_array().unwrap().is_empty(),
            "{area} quickstart inlines its linked pages"
        );
        let receipt = bundle["bootstrap_receipt_event_id"].as_str().unwrap();
        let subject = format!("quickstart-{area}");
        assert!(receipt_exists(&fx.backend.db, &subject, receipt).await);
    }
    let missing = fx
        .http
        .get(format!("{}/usermanual/quickstarts/zzz", fx.base))
        .send()
        .await
        .expect("unknown area");
    assert_eq!(missing.status(), 404);
}

/// MT-199 negative guard: quickstart bundles must fail closed when a seeded
/// `page_link` target is missing. Returning 200 with a silently omitted linked
/// page would hand a no-context model an incomplete bootstrap bundle.
#[tokio::test]
async fn mt199_quickstart_fails_when_linked_page_is_missing() {
    let fx = fixture().await;
    let changed_anchor = tamper_quickstart_page_link(&fx.backend.db).await;

    let response = fx
        .http
        .get(format!("{}/usermanual/quickstarts/index", fx.base))
        .send()
        .await
        .expect("quickstart with missing link");
    assert_eq!(
        response.status(),
        404,
        "quickstart with tampered anchor {changed_anchor} must fail closed"
    );
    let body: Value = response.json().await.expect("missing link error json");
    assert_eq!(body["error"], "not_found");
    assert!(
        body["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("missing-linked-page-for-mt199"),
        "missing link error should name the unresolved page_link: {body:?}"
    );
}

/// MT-200: access points cover the five contract host surfaces and every
/// target slug resolves against the LIVE database.
#[tokio::test]
async fn mt200_access_points_resolve() {
    let fx = fixture().await;
    let payload: Value = fx
        .http
        .get(format!("{}/usermanual/access-points", fx.base))
        .send()
        .await
        .expect("access points")
        .json()
        .await
        .expect("access json");
    let rows = payload["access_points"].as_array().unwrap();
    assert!(rows.len() >= 8);
    let mut hosts = std::collections::BTreeSet::new();
    for row in rows {
        assert_eq!(
            row["target_resolves"], true,
            "access point {} targets a missing page {}",
            row["access_point_id"], row["target_page_slug"]
        );
        assert!(row["stable_element_id"]
            .as_str()
            .unwrap()
            .starts_with("hs-usermanual-"));
        hosts.insert(row["host_surface"].as_str().unwrap().to_string());
    }
    for host in [
        "editor",
        "notes_loom",
        "retrieval_debug",
        "diagnostics",
        "command_palette",
    ] {
        assert!(hosts.contains(host), "missing host surface {host}");
    }
}

/// MT-203: the legacy bridge answers with the canonical mapping AND a
/// persisted compatibility receipt (spec 10.15.8 bridge law).
#[tokio::test]
async fn mt203_legacy_bridge_route_maps_and_emits_compat_receipt() {
    let fx = fixture().await;
    let bridge: Value = fx
        .http
        .get(format!("{}/usermanual/legacy/model-manual", fx.base))
        .send()
        .await
        .expect("legacy bridge")
        .json()
        .await
        .expect("bridge json");
    assert_eq!(bridge["deprecated"], true);
    assert!(!bridge["canonical"]["pages"].as_array().unwrap().is_empty());
    assert_eq!(bridge["canonical"]["route_namespace"], "/usermanual");
    let receipt = bridge["compatibility_receipt_event_id"].as_str().unwrap();
    assert!(
        receipt_exists(&fx.backend.db, "model_manual", receipt).await,
        "compatibility receipt must be persisted (spec 10.15.8)"
    );

    let aliases: Value = fx
        .http
        .get(format!("{}/usermanual/legacy/aliases", fx.base))
        .send()
        .await
        .expect("aliases")
        .json()
        .await
        .expect("aliases json");
    assert!(aliases["count"].as_u64().unwrap() >= 6);

    let plan: Value = fx
        .http
        .get(format!("{}/usermanual/migration-plan", fx.base))
        .send()
        .await
        .expect("plan")
        .json()
        .await
        .expect("plan json");
    assert_eq!(plan["canonical_term"], "UserManual");
    assert!(plan["rows"].as_array().unwrap().len() >= 13);
}

/// MT-204: freshness is `current` after seed; a tampered page flips to
/// stale_content; restoring heals. The check itself is receipted.
#[tokio::test]
async fn mt204_freshness_current_then_stale_fixture() {
    let fx = fixture().await;
    let fresh: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("freshness")
        .json()
        .await
        .expect("freshness json");
    assert_eq!(
        fresh["report"]["fresh"],
        true,
        "seeded manual must be fresh: {:?}",
        fresh["report"]["verdicts"].as_array().map(|v| v
            .iter()
            .filter(|x| x["kind"] != "current")
            .collect::<Vec<_>>())
    );
    let receipt = fresh["receipt_event_id"].as_str().unwrap();
    assert!(receipt_exists(&fx.backend.db, USER_MANUAL_VERSION, receipt).await);

    // Stale fixture (MT-208 family): tamper one stored page hash.
    let previous = tamper_page_content_hash(&fx.backend.db, "core-workflows")
        .await
        .expect("tamper");
    let stale: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("stale freshness")
        .json()
        .await
        .expect("stale json");
    assert_eq!(stale["report"]["fresh"], false);
    assert!(
        stale["report"]["verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| { v["kind"] == "stale_content" && v["subject"] == "core-workflows" }),
        "tampered page must yield stale_content"
    );

    restore_page_content_hash(&fx.backend.db, "core-workflows", &previous)
        .await
        .expect("restore");
    let healed: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("healed freshness")
        .json()
        .await
        .expect("healed json");
    assert_eq!(healed["report"]["fresh"], true);
}

/// MT-205: the HTML projection carries stable selectors, ordered sections,
/// escaped content, and resolvable navigation; every stored page is
/// reachable from the TOC.
#[tokio::test]
async fn mt205_projection_renders_readable_navigable_html() {
    let fx = fixture().await;
    let projection: Value = fx
        .http
        .get(format!(
            "{}/usermanual/pages/manual-toc/projection?format=html",
            fx.base
        ))
        .send()
        .await
        .expect("projection")
        .json()
        .await
        .expect("projection json");
    let html = projection["rendered"].as_str().unwrap();
    assert!(html.contains("data-hs-manual-page=\"manual-toc\""));
    assert!(html.contains("data-hs-manual-section="));
    assert!(html.contains("data-hs-manual-link=\"quickstart-index\""));
    assert!(html.contains("data-hs-href=\"/usermanual/pages/quickstart-index\""));
    assert!(
        !html.contains("<script>"),
        "projection must never emit live script"
    );

    let markdown: Value = fx
        .http
        .get(format!(
            "{}/usermanual/pages/manual-toc/projection?format=markdown",
            fx.base
        ))
        .send()
        .await
        .expect("md projection")
        .json()
        .await
        .expect("md json");
    assert!(markdown["rendered"]
        .as_str()
        .unwrap()
        .contains("<topic id=\"manual-toc-0\""));

    let bad = fx
        .http
        .get(format!(
            "{}/usermanual/pages/manual-toc/projection?format=pdf",
            fx.base
        ))
        .send()
        .await
        .expect("bad format");
    assert_eq!(bad.status(), 400);

    // Visual navigation law: no stored page is orphaned from the TOC.
    let orphans = unreachable_pages(&fx.backend.db).await.expect("nav audit");
    assert!(orphans.is_empty(), "orphan manual pages: {orphans:?}");
}

/// Resync permission gate: unauthenticated/cloud_model/validator are DENIED
/// with stable reasons; unknown tokens are 400; local_model succeeds.
#[tokio::test]
async fn mt201_resync_permission_gate_fails_closed() {
    let fx = fixture().await;

    let anonymous = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .send()
        .await
        .expect("anonymous resync");
    assert_eq!(anonymous.status(), 403);
    let body: Value = anonymous.json().await.expect("403 json");
    assert_eq!(body["reason"], "unauthenticated_resync_denied");

    let cloud = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "cloud_model")
        .send()
        .await
        .expect("cloud resync");
    assert_eq!(cloud.status(), 403);
    let body: Value = cloud.json().await.expect("403 json");
    assert_eq!(body["reason"], "cloud_model_resync_denied");

    let unknown = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "root")
        .send()
        .await
        .expect("unknown kind");
    assert_eq!(
        unknown.status(),
        400,
        "unknown tokens are rejected, never coerced"
    );

    let allowed = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "local_model")
        .send()
        .await
        .expect("local_model resync");
    assert_eq!(allowed.status(), 200);
    let report: Value = allowed.json().await.expect("resync json");
    assert_eq!(
        report["resync"]["pages_changed"], 0,
        "already-seeded resync is a no-op"
    );
}

/// THE doc-vs-runtime keystone: every surface the manual declares is probed
/// against the REAL full product router. A documented route the router does
/// not mount (router-level 404: empty body) or a wrong documented method
/// (405) fails the suite — the manual cannot describe surfaces the product
/// does not serve (spec 10.15.8: stale docs are a build defect).
#[tokio::test]
async fn mtdoc_every_registry_surface_exists_on_the_real_router() {
    let backend = manual_test_backend()
        .await
        .expect("open embedded backend for router probe");
    ensure_seeded(&backend.db).await.expect("seed");
    let account = AccountFixture::install(backend.db.storage()).await;
    let state = app_state_for(&backend.db).await;
    let (base, _server) = start_server(api::routes(state)).await;
    let http = account.client();

    for surface in wp009_surface_registry() {
        let path = probe_path(surface.route);
        let url = format!("{base}{path}");
        let request = match surface.method {
            "GET" => http.get(&url),
            "POST" => http.post(&url),
            "PUT" => http.put(&url),
            "DELETE" => http.delete(&url),
            "PATCH" => http.patch(&url),
            other => panic!("unsupported method {other}"),
        };
        let response = request.send().await.unwrap_or_else(|err| {
            panic!(
                "probe {} {} failed to send: {err}",
                surface.method, surface.route
            )
        });
        let status = response.status();
        assert_ne!(
            status,
            reqwest::StatusCode::METHOD_NOT_ALLOWED,
            "manual documents {} {} but the router answers 405 — wrong method documented",
            surface.method,
            surface.route
        );
        if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            assert!(
                !body.trim().is_empty(),
                "manual documents {} {} but the router has NO such route (bare 404)",
                surface.method,
                surface.route
            );
        }
    }
}
