//! WP-KERNEL-009 MT-209..216 ParallelSwarmStateRecovery integration proof.
//!
//! These tests run against real PostgreSQL through the existing knowledge
//! PostgreSQL harness. They prove backend behavior, not status text:
//! typed lane identity, worktree/workspace claims, role-mailbox handoff refs,
//! deterministic backend navigation commands, restartable checkpoints and
//! recovery receipts, local/cloud attribution without secret leakage, and a
//! lease queue that serializes parallel index writers per scope.

mod knowledge_pg_support;

use std::{collections::HashSet, sync::Arc};

use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::swarm_orchestration::state_recovery::{
    AgentCapability, AgentLaneIdentity, AgentLaneKind, AttributionMode, BackendNavigationCommand,
    ClaimScope, ClaimStatus, IndexLeaseStatus, IndexingLeaseRequest, LocalCloudAttribution,
    ModelProviderKind, NavigationCommandSet, ParallelSwarmStateRecoveryStore,
    RecoveryCheckpointRequest, RecoveryResumePointer, RoleMailboxHandoffRequest,
    StateRecoveryError, SwarmReceiptStatus, WorkClaimRequest,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Barrier;
use uuid::Uuid;

async fn recovery_store() -> Option<(sqlx::PgPool, ParallelSwarmStateRecoveryStore)> {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP parallel_swarm_state_recovery_tests: no PostgreSQL");
        return None;
    };
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect isolated parallel swarm schema");
    let event_db = PostgresDatabase::new(pool.clone()).into_arc();
    let store = ParallelSwarmStateRecoveryStore::new(pool.clone(), event_db);
    Some((pool, store))
}

fn local_lane(actor_suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-local-{actor_suffix}"),
        format!("coder-{actor_suffix}"),
        AgentLaneKind::Local,
        LocalCloudAttribution::local("llama_cpp", "qwen-coder-32b"),
    )
    .expect("valid local lane")
}

fn cloud_lane(actor_suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-cloud-{actor_suffix}"),
        format!("cloud-{actor_suffix}"),
        AgentLaneKind::Cloud,
        LocalCloudAttribution::cloud(
            ModelProviderKind::OpenAi,
            "gpt-5.4-codex",
            "vault://providers/openai/default",
            json!({
                "api_key": "sk-test-secret-must-not-persist",
                "organization": "org-visible",
                "endpoint": "https://api.openai.example/v1"
            }),
        ),
    )
    .expect("valid cloud lane with scrubbed metadata")
}

fn raw_cloud_lane(actor_suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-cloud-raw-{actor_suffix}"),
        format!("cloud-raw-{actor_suffix}"),
        AgentLaneKind::Cloud,
        LocalCloudAttribution {
            mode: AttributionMode::Cloud,
            provider: Some(ModelProviderKind::OpenAi),
            runtime: None,
            model_label: "gpt-5.4-codex".to_string(),
            credential_ref: Some("vault://providers/openai/raw".to_string()),
            provider_metadata: json!({
                "api_key": "sk-raw-secret-must-not-persist",
                "nested": {
                    "session_token": "raw-token-must-not-persist"
                },
                "organization": "org-visible"
            }),
        },
    )
    .expect("valid raw cloud lane")
}

fn lane_with_kind(actor_suffix: &str, lane_kind: AgentLaneKind) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-{actor_suffix}"),
        format!("actor-{actor_suffix}"),
        lane_kind,
        LocalCloudAttribution::local("test-runtime", format!("test-model-{actor_suffix}")),
    )
    .expect("valid typed lane")
}

fn assert_invalid_input_contains<T>(result: Result<T, StateRecoveryError>, expected: &str) {
    match result {
        Err(StateRecoveryError::InvalidInput(message)) => assert!(
            message.contains(expected),
            "expected invalid input to mention {expected}, got {message}"
        ),
        Err(error) => panic!("expected invalid input containing {expected}, got {error}"),
        Ok(_) => panic!("expected invalid input containing {expected}, got success"),
    }
}

async fn ledger_count_for_payload_value(
    pool: &sqlx::PgPool,
    aggregate_type: &str,
    payload_key: &str,
    payload_value: &str,
) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = $1
          AND payload ->> $2 = $3
        "#,
    )
    .bind(aggregate_type)
    .bind(payload_key)
    .bind(payload_value)
    .fetch_one(pool)
    .await
    .expect("count parallel swarm ledger events")
}

async fn ledger_event_type_and_status(pool: &sqlx::PgPool, event_id: &str) -> (String, String) {
    sqlx::query_as(
        r#"
        SELECT event_type, payload ->> 'status'
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("fetch ledger event type and status")
}

async fn install_parallel_swarm_event_delay(pool: &sqlx::PgPool, aggregate_type: &str) {
    let suffix = aggregate_type.replace('_', "");
    let function_name = format!("slow_psr_{suffix}_event");
    let trigger_name = format!("slow_psr_{suffix}_trigger");
    let function_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            PERFORM pg_sleep(0.25);
            RETURN NEW;
        END
        $$
        "#
    );
    sqlx::query(&function_sql)
        .execute(pool)
        .await
        .expect("install event delay function");
    let trigger_sql = format!(
        r#"
        CREATE TRIGGER {trigger_name}
        BEFORE INSERT ON kernel_event_ledger
        FOR EACH ROW
        WHEN (
            NEW.source_component = 'parallel_swarm_state_recovery'
            AND NEW.aggregate_type = '{aggregate_type}'
        )
        EXECUTE FUNCTION {function_name}()
        "#
    );
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .expect("install event delay trigger");
}

async fn install_parallel_swarm_event_failure(pool: &sqlx::PgPool, aggregate_type: &str) {
    let suffix = aggregate_type.replace('_', "");
    let function_name = format!("fail_psr_{suffix}_event");
    let trigger_name = format!("fail_psr_{suffix}_trigger");
    let function_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'forced parallel swarm event failure';
        END
        $$
        "#
    );
    sqlx::query(&function_sql)
        .execute(pool)
        .await
        .expect("install event failure function");
    let trigger_sql = format!(
        r#"
        CREATE TRIGGER {trigger_name}
        BEFORE INSERT ON kernel_event_ledger
        FOR EACH ROW
        WHEN (
            NEW.source_component = 'parallel_swarm_state_recovery'
            AND NEW.aggregate_type = '{aggregate_type}'
        )
        EXECUTE FUNCTION {function_name}()
        "#
    );
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .expect("install event failure trigger");
}

async fn install_claim_receipt_authority_failure(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION fail_psr_claim_receipt_authority()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.reason = 'forced claim receipt authority failure after receipt'
               AND NEW.event_ledger_event_id IS NOT NULL THEN
                RAISE EXCEPTION 'forced claim receipt authority failure';
            END IF;
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(pool)
    .await
    .expect("install claim receipt authority failure function");
    sqlx::query(
        r#"
        CREATE TRIGGER fail_psr_claim_receipt_authority_trigger
        BEFORE INSERT OR UPDATE ON knowledge_agent_worktree_claims
        FOR EACH ROW
        EXECUTE FUNCTION fail_psr_claim_receipt_authority()
        "#,
    )
    .execute(pool)
    .await
    .expect("install claim receipt authority failure trigger");
}

async fn install_recovery_receipt_authority_failure(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION fail_psr_recovery_receipt_authority()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.new_session_id = 'session-recovery-authority-fail' THEN
                RAISE EXCEPTION 'forced recovery receipt authority failure';
            END IF;
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(pool)
    .await
    .expect("install recovery receipt authority failure function");
    sqlx::query(
        r#"
        CREATE TRIGGER fail_psr_recovery_receipt_authority_trigger
        BEFORE INSERT ON knowledge_agent_recovery_receipts
        FOR EACH ROW
        EXECUTE FUNCTION fail_psr_recovery_receipt_authority()
        "#,
    )
    .execute(pool)
    .await
    .expect("install recovery receipt authority failure trigger");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_lanes_and_work_claims_are_typed_attributable_and_exclusive() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let local = local_lane("claim-a");
    let cloud = cloud_lane("claim-b");
    let other_local = local_lane("claim-c");
    assert!(local
        .capabilities()
        .contains(&AgentCapability::ClaimWorktree));
    assert!(local
        .capabilities()
        .contains(&AgentCapability::WriteLocalIndex));
    assert!(cloud
        .capabilities()
        .contains(&AgentCapability::NavigateBackend));
    assert!(
        !cloud
            .capabilities()
            .contains(&AgentCapability::WriteLocalIndex),
        "cloud lanes must not silently become local index writers"
    );
    assert!(
        !serde_json::to_string(&cloud)
            .expect("serialize cloud lane")
            .contains("sk-test-secret"),
        "provider secrets must be scrubbed from lane attribution"
    );

    let scope = ClaimScope::Worktree {
        worktree_id: "wtc-kernel-009".to_string(),
    };
    let first = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-alpha".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: scope.clone(),
            lane: local.clone(),
            session_id: "session-local-a".to_string(),
            ttl_seconds: 600,
            reason: "claim the product worktree for MT-210".to_string(),
        })
        .await
        .expect("first claim");
    assert_eq!(first.status, ClaimStatus::Active);
    assert!(first.claim_id.starts_with("PSR-CLAIM-"));

    let second = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-alpha".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: scope.clone(),
            lane: other_local.clone(),
            session_id: "session-local-c".to_string(),
            ttl_seconds: 600,
            reason: "parallel local worker should wait".to_string(),
        })
        .await
        .expect("second claim returns held, not corruption");
    assert_eq!(second.status, ClaimStatus::Held);
    assert_eq!(
        second
            .active_holder
            .as_ref()
            .expect("held claim names active holder")
            .actor_id,
        local.actor_id
    );

    let active = store
        .list_active_claims("workspace-alpha")
        .await
        .expect("active claims");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].claim_id, first.claim_id);

    assert!(
        store
            .release_claim(&first.claim_id, &local, "MT-210 complete")
            .await
            .expect("release claim"),
        "claim holder can release the claim"
    );
    let release_event_id: Option<String> = sqlx::query_scalar(
        "SELECT to_jsonb(c) ->> 'release_event_ledger_event_id' FROM knowledge_agent_worktree_claims c WHERE claim_id = $1",
    )
    .bind(&first.claim_id)
    .fetch_one(&pool)
    .await
    .expect("fetch release receipt event id");
    let release_event_id = release_event_id.expect("released claim carries release receipt");
    assert!(
        release_event_id.starts_with("KE-"),
        "released claim must carry an EventLedger release receipt"
    );

    let after_release = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-alpha".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope,
            lane: other_local,
            session_id: "session-local-c".to_string(),
            ttl_seconds: 600,
            reason: "claim after release".to_string(),
        })
        .await
        .expect("claim after release");
    assert_eq!(after_release.status, ClaimStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_authority_failure_after_receipt_rolls_back_eventledger_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    install_claim_receipt_authority_failure(&pool).await;
    let workspace_id = format!("workspace-claim-receipt-fail-{}", Uuid::now_v7());
    let result = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-claim-fail-{}", Uuid::now_v7()),
            },
            lane: local_lane("claim-receipt-fail"),
            session_id: "session-claim-receipt-fail".to_string(),
            ttl_seconds: 600,
            reason: "forced claim receipt authority failure after receipt".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "forced authority failure should reject claim receipt persistence"
    );

    let claim_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count failed claim rows");
    assert_eq!(claim_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim",
            "workspace_id",
            &workspace_id
        )
        .await,
        0,
        "failed claim authority write must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_claim_rolls_back_authority_state_if_receipt_insert_fails() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-release-receipt-fail-{}", Uuid::now_v7());
    let lane = local_lane("release-receipt-fail-holder");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-release-fail-{}", Uuid::now_v7()),
            },
            lane: lane.clone(),
            session_id: "session-release-receipt-fail-holder".to_string(),
            ttl_seconds: 600,
            reason: "claim that should remain active if release receipt fails".to_string(),
        })
        .await
        .expect("claim before forced release receipt failure");

    install_parallel_swarm_event_failure(&pool, "parallel_swarm_claim").await;
    let result = store
        .release_claim(
            &claim.claim_id,
            &lane,
            "forced release receipt insertion failure",
        )
        .await;
    assert!(
        result.is_err(),
        "forced EventLedger failure should reject release"
    );

    let (status, has_released_at, release_event_id): (String, bool, Option<String>) =
        sqlx::query_as(
            r#"
            SELECT status,
                   released_at_utc IS NOT NULL,
                   to_jsonb(c) ->> 'release_event_ledger_event_id'
            FROM knowledge_agent_worktree_claims c
            WHERE claim_id = $1
            "#,
        )
        .bind(&claim.claim_id)
        .fetch_one(&pool)
        .await
        .expect("fetch claim after failed release");
    assert_eq!(
        status, "active",
        "claim must not be left released when receipt insertion fails"
    );
    assert!(
        !has_released_at,
        "failed release must not stamp released_at_utc"
    );
    assert!(
        release_event_id.is_none(),
        "failed release must not attach a missing receipt id"
    );
    assert_eq!(
        ledger_count_for_payload_value(&pool, "parallel_swarm_claim", "claim_id", &claim.claim_id)
            .await,
        1,
        "failed release must not add a second false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_lane_is_denied_worktree_claim_and_local_index_write_lease() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-capability-deny-{}", Uuid::now_v7());
    let cloud = cloud_lane("capability-deny");
    let claim_result = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-209".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: "wtc-kernel-009".to_string(),
            },
            lane: cloud.clone(),
            session_id: "session-cloud-capability-deny".to_string(),
            ttl_seconds: 600,
            reason: "cloud lanes must not claim local worktrees".to_string(),
        })
        .await;
    assert_invalid_input_contains(claim_result, "ClaimWorktree");

    let persisted_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied worktree claims");
    assert_eq!(persisted_claims, 0);

    let claim_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_swarm_claim'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied claim events");
    assert_eq!(claim_events, 0);

    let lease_result = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-216".to_string(),
            scope: ClaimScope::IndexRun {
                workspace_id: workspace_id.clone(),
                source_root_id: "root-capability-deny".to_string(),
            },
            lane: cloud,
            session_id: "session-cloud-index-deny".to_string(),
            index_run_id: format!("index-run-{}", Uuid::now_v7()),
            priority: 10,
            ttl_seconds: 600,
        })
        .await;
    assert_invalid_input_contains(lease_result, "WriteLocalIndex");

    let persisted_leases: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied indexing leases");
    assert_eq!(persisted_leases, 0);

    let lease_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_indexing_lease'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied lease events");
    assert_eq!(lease_events, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editor_document_and_graph_claims_serialize_parallel_mutations() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-editor-safety-{}", Uuid::now_v7());
    let doc_scope = ClaimScope::RichDocument {
        workspace_id: workspace_id.clone(),
        document_id: "note-alpha".to_string(),
    };
    let graph_scope = ClaimScope::GraphMutation {
        workspace_id: workspace_id.clone(),
        graph_id: "graph-main".to_string(),
    };
    let first_editor = lane_with_kind("editor-doc-a", AgentLaneKind::Editor);
    let second_editor = lane_with_kind("editor-doc-b", AgentLaneKind::Editor);

    let first_doc_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: doc_scope.clone(),
            lane: first_editor.clone(),
            session_id: "session-editor-doc-a".to_string(),
            ttl_seconds: 600,
            reason: "claim rich document before editing".to_string(),
        })
        .await
        .expect("first rich-document claim");
    assert_eq!(first_doc_claim.status, ClaimStatus::Active);

    let second_doc_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: doc_scope.clone(),
            lane: second_editor.clone(),
            session_id: "session-editor-doc-b".to_string(),
            ttl_seconds: 600,
            reason: "parallel editor should wait for same document".to_string(),
        })
        .await
        .expect("second rich-document claim returns held");
    assert_eq!(second_doc_claim.status, ClaimStatus::Held);
    assert_eq!(
        second_doc_claim
            .active_holder
            .as_ref()
            .expect("held document claim names active holder")
            .actor_id,
        first_editor.actor_id
    );

    let other_doc_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "note-beta".to_string(),
            },
            lane: second_editor.clone(),
            session_id: "session-editor-doc-b".to_string(),
            ttl_seconds: 600,
            reason: "different document can proceed in parallel".to_string(),
        })
        .await
        .expect("different document claim");
    assert_eq!(other_doc_claim.status, ClaimStatus::Active);

    let first_graph_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: graph_scope.clone(),
            lane: first_editor.clone(),
            session_id: "session-editor-graph-a".to_string(),
            ttl_seconds: 600,
            reason: "claim graph before mutation".to_string(),
        })
        .await
        .expect("first graph mutation claim");
    assert_eq!(first_graph_claim.status, ClaimStatus::Active);

    let second_graph_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: graph_scope.clone(),
            lane: second_editor.clone(),
            session_id: "session-editor-graph-b".to_string(),
            ttl_seconds: 600,
            reason: "parallel editor should wait for same graph".to_string(),
        })
        .await
        .expect("second graph claim returns held");
    assert_eq!(second_graph_claim.status, ClaimStatus::Held);
    assert_eq!(
        second_graph_claim
            .active_holder
            .as_ref()
            .expect("held graph claim names active holder")
            .actor_id,
        first_editor.actor_id
    );

    let persisted_scopes: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT scope_kind, scope_id
        FROM knowledge_agent_worktree_claims
        WHERE workspace_id = $1
        ORDER BY scope_kind ASC, scope_id ASC
        "#,
    )
    .bind(&workspace_id)
    .fetch_all(&pool)
    .await
    .expect("fetch persisted editor safety claims");
    assert!(
        persisted_scopes.contains(&(
            "rich_document".to_string(),
            format!("{workspace_id}/note-alpha")
        )),
        "rich-document claims must persist a stable per-document scope id"
    );
    assert!(
        persisted_scopes.contains(&(
            "graph_mutation".to_string(),
            format!("{workspace_id}/graph-main")
        )),
        "graph mutation claims must persist a stable per-graph scope id"
    );

    assert!(
        store
            .release_claim(
                &first_doc_claim.claim_id,
                &first_editor,
                "rich document edit complete"
            )
            .await
            .expect("release rich document claim"),
        "document claim holder can release the claim"
    );
    let after_release = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: doc_scope,
            lane: second_editor,
            session_id: "session-editor-doc-b".to_string(),
            ttl_seconds: 600,
            reason: "same document can proceed after release".to_string(),
        })
        .await
        .expect("claim released document");
    assert_eq!(after_release.status, ClaimStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_editor_lanes_cannot_claim_editor_mutation_scopes() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-editor-deny-{}", Uuid::now_v7());
    for (suffix, lane_kind, scope, expected_capability) in [
        (
            "validator-doc",
            AgentLaneKind::Validator,
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "note-denied".to_string(),
            },
            "EditRichDocument",
        ),
        (
            "validator-graph",
            AgentLaneKind::Validator,
            ClaimScope::GraphMutation {
                workspace_id: workspace_id.clone(),
                graph_id: "graph-denied".to_string(),
            },
            "MutateGraph",
        ),
        (
            "cloud-doc",
            AgentLaneKind::Cloud,
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "cloud-note-denied".to_string(),
            },
            "EditRichDocument",
        ),
        (
            "indexer-graph",
            AgentLaneKind::Indexer,
            ClaimScope::GraphMutation {
                workspace_id: workspace_id.clone(),
                graph_id: "indexer-graph-denied".to_string(),
            },
            "MutateGraph",
        ),
    ] {
        let claim_result = store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-217".to_string()),
                scope,
                lane: lane_with_kind(suffix, lane_kind),
                session_id: format!("session-{suffix}"),
                ttl_seconds: 600,
                reason: "non-editor lanes must not mutate editor surfaces".to_string(),
            })
            .await;
        assert_invalid_input_contains(claim_result, expected_capability);
    }

    let persisted_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied editor safety claims");
    assert_eq!(persisted_claims, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim",
            "workspace_id",
            &workspace_id
        )
        .await,
        0,
        "denied mutation-scope claims must not emit false EventLedger receipts"
    );

    let allowed_workspace_id = format!("workspace-editor-allowed-{}", Uuid::now_v7());
    let editor_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: allowed_workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: ClaimScope::RichDocument {
                workspace_id: allowed_workspace_id,
                document_id: "note-allowed".to_string(),
            },
            lane: lane_with_kind("editor-allowed", AgentLaneKind::Editor),
            session_id: "session-editor-allowed".to_string(),
            ttl_seconds: 600,
            reason: "editor lane may claim rich document mutation scope".to_string(),
        })
        .await
        .expect("editor may claim document mutation scope");
    assert_eq!(editor_claim.status, ClaimStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_editor_mutation_scopes_do_not_persist_claims_or_receipts() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-editor-malformed-{}", Uuid::now_v7());
    let editor = lane_with_kind("editor-malformed-scope", AgentLaneKind::Editor);
    for (suffix, scope, expected) in [
        (
            "empty-doc-workspace",
            ClaimScope::RichDocument {
                workspace_id: String::new(),
                document_id: "note-empty-workspace".to_string(),
            },
            "rich_document.workspace_id",
        ),
        (
            "empty-document-id",
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: String::new(),
            },
            "rich_document.document_id",
        ),
        (
            "slash-document-id",
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "nested/note".to_string(),
            },
            "rich_document.document_id",
        ),
        (
            "mismatched-doc-workspace",
            ClaimScope::RichDocument {
                workspace_id: format!("other-{workspace_id}"),
                document_id: "note-mismatch".to_string(),
            },
            "workspace_id must match",
        ),
        (
            "empty-graph-id",
            ClaimScope::GraphMutation {
                workspace_id: workspace_id.clone(),
                graph_id: String::new(),
            },
            "graph_mutation.graph_id",
        ),
        (
            "mismatched-graph-workspace",
            ClaimScope::GraphMutation {
                workspace_id: format!("other-graph-{workspace_id}"),
                graph_id: "graph-mismatch".to_string(),
            },
            "workspace_id must match",
        ),
    ] {
        let claim_result = store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-217".to_string()),
                scope,
                lane: editor.clone(),
                session_id: format!("session-invalid-editor-scope-{suffix}"),
                ttl_seconds: 600,
                reason: "invalid editor mutation scope should not persist".to_string(),
            })
            .await;
        assert_invalid_input_contains(claim_result, expected);
    }

    let persisted_claims: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_worktree_claims
        WHERE session_id LIKE 'session-invalid-editor-scope-%'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count malformed editor scope claims");
    assert_eq!(
        persisted_claims, 0,
        "malformed editor mutation claims must be rejected before authority rows"
    );
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim",
            "workspace_id",
            &workspace_id
        )
        .await,
        0,
        "malformed editor mutation claims must not emit false EventLedger receipts"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_handoff_requires_write_mailbox_capability() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    for (lane_kind, suffix) in [
        (AgentLaneKind::Indexer, "indexer-deny-mailbox"),
        (AgentLaneKind::Editor, "editor-deny-mailbox"),
    ] {
        let thread_id = format!("thread-{suffix}-{}", Uuid::now_v7());
        let result = store
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: lane_with_kind(suffix, lane_kind),
                to_role: "WP_VALIDATOR".to_string(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-211".to_string(),
                claim_id: None,
                mailbox_thread_id: thread_id.clone(),
                mailbox_message_id: format!("message-{suffix}"),
                status: SwarmReceiptStatus::Blocked,
                summary: format!("{suffix} must not write role mailbox handoffs"),
                body_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_string(),
            })
            .await;
        assert_invalid_input_contains(result, "WriteMailbox");

        let handoff_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM knowledge_agent_role_mailbox_handoffs WHERE mailbox_thread_id = $1",
        )
        .bind(&thread_id)
        .fetch_one(&pool)
        .await
        .expect("count denied mailbox handoff rows");
        assert_eq!(handoff_rows, 0);
        assert_eq!(
            ledger_count_for_payload_value(
                &pool,
                "parallel_swarm_handoff",
                "mailbox_thread_id",
                &thread_id,
            )
            .await,
            0,
            "denied mailbox writers must not leave EventLedger handoff receipts"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_mailbox_handoff_claim_ref_does_not_emit_false_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let thread_id = format!("thread-invalid-claim-{}", Uuid::now_v7());
    let result = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: local_lane("invalid-handoff-ref"),
            to_role: "WP_VALIDATOR".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-211".to_string(),
            claim_id: Some(format!("PSR-CLAIM-missing-{}", Uuid::now_v7())),
            mailbox_thread_id: thread_id.clone(),
            mailbox_message_id: "message-invalid-claim".to_string(),
            status: SwarmReceiptStatus::Blocked,
            summary: "invalid claim FK must fail without a false receipt".to_string(),
            body_sha256: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                .to_string(),
        })
        .await;
    assert!(result.is_err(), "invalid claim FK should reject handoff");

    let handoff_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_role_mailbox_handoffs WHERE mailbox_thread_id = $1",
    )
    .bind(&thread_id)
    .fetch_one(&pool)
    .await
    .expect("count failed handoff rows");
    assert_eq!(handoff_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_handoff",
            "mailbox_thread_id",
            &thread_id,
        )
        .await,
        0,
        "failed handoff FK insert must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_checkpoint_refs_do_not_emit_false_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-invalid-checkpoint-{}", Uuid::now_v7());
    let result = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: local_lane("invalid-checkpoint-ref"),
            session_id: "session-invalid-checkpoint-ref".to_string(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-214".to_string(),
            claim_id: Some(format!("PSR-CLAIM-missing-{}", Uuid::now_v7())),
            mailbox_handoff_id: Some(format!("PSR-HANDOFF-missing-{}", Uuid::now_v7())),
            navigation_command_id: Some("symbols".to_string()),
            resume_pointer: RecoveryResumePointer::MicroTask {
                mt_id: "MT-214".to_string(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["cargo test --test parallel_swarm_state_recovery_tests".to_string()],
            hbr_rows: vec!["HBR-SWARM-004".to_string()],
            next_step_context: "invalid refs should fail before receipt".to_string(),
            payload: json!({"invalid_refs": true}),
            compaction_reason: "test_invalid_refs".to_string(),
            git_head: "deadbeef".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "invalid claim/mailbox FKs should reject checkpoint"
    );

    let checkpoint_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_state_recovery_checkpoints WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count failed checkpoint rows");
    assert_eq!(checkpoint_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_checkpoint",
            "workspace_id",
            &workspace_id,
        )
        .await,
        0,
        "failed checkpoint FK insert must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_scope_claim_records_one_durable_claim_event() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    install_parallel_swarm_event_delay(&pool, "parallel_swarm_claim").await;

    let workspace_id = format!("workspace-claim-race-{}", Uuid::now_v7());
    let scope = ClaimScope::Worktree {
        worktree_id: format!("wtc-kernel-009-race-{}", Uuid::now_v7()),
    };
    let barrier = Arc::new(Barrier::new(2));
    let left_store = store.clone();
    let left_barrier = barrier.clone();
    let left_workspace = workspace_id.clone();
    let left_scope = scope.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        left_store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: left_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-210".to_string()),
                scope: left_scope,
                lane: local_lane("claim-race-a"),
                session_id: "session-claim-race-a".to_string(),
                ttl_seconds: 600,
                reason: "left concurrent claim".to_string(),
            })
            .await
    });
    let right_store = store.clone();
    let right_barrier = barrier.clone();
    let right_workspace = workspace_id.clone();
    let right_scope = scope.clone();
    let right = tokio::spawn(async move {
        right_barrier.wait().await;
        right_store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: right_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-210".to_string()),
                scope: right_scope,
                lane: local_lane("claim-race-b"),
                session_id: "session-claim-race-b".to_string(),
                ttl_seconds: 600,
                reason: "right concurrent claim".to_string(),
            })
            .await
    });

    let left = left.await.expect("left claim task joins");
    let right = right.await.expect("right claim task joins");
    assert!(
        left.is_ok(),
        "left claim should resolve to an outcome: {left:?}"
    );
    assert!(
        right.is_ok(),
        "right claim should resolve to an outcome: {right:?}"
    );
    let outcomes = [left.expect("left outcome"), right.expect("right outcome")];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == ClaimStatus::Active)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == ClaimStatus::Held)
            .count(),
        1
    );

    let claim_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count claim race rows");
    assert_eq!(claim_rows, 1);

    let claim_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_swarm_claim'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count claim race events");
    assert_eq!(
        claim_events, claim_rows,
        "claim race losers must not leave false durable EventLedger claim events"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_navigation_checkpoint_and_recovery_are_restartable_from_postgres() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let lane = cloud_lane("recover");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-recovery".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-213".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: "workspace-recovery".to_string(),
            },
            lane: lane.clone(),
            session_id: "session-cloud-recover".to_string(),
            ttl_seconds: 600,
            reason: "checkpoint recovery proof".to_string(),
        })
        .await
        .expect("workspace claim");

    let handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: lane.clone(),
            to_role: "WP_VALIDATOR".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-213".to_string(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_thread_id: "thread-mt-213".to_string(),
            mailbox_message_id: "message-mt-213".to_string(),
            status: SwarmReceiptStatus::Blocked,
            summary: "validator should inspect checkpoint receipt shape".to_string(),
            body_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
        })
        .await
        .expect("mailbox handoff receipt");
    assert!(handoff.event_ledger_event_id.starts_with("KE-"));

    let nav = NavigationCommandSet::default();
    let ids: HashSet<_> = nav.commands().iter().map(|cmd| cmd.command_id).collect();
    assert_eq!(ids.len(), nav.commands().len(), "command ids are unique");
    for expected in [
        "sources",
        "symbols",
        "docs",
        "graph",
        "retrieval_traces",
        "user_manual_pages",
        "repair_queue",
        "validation_state",
    ] {
        assert!(
            ids.contains(expected),
            "missing navigation command {expected}"
        );
    }
    let resolved_once = nav
        .resolve(
            BackendNavigationCommand::Symbols,
            json!({"workspace_id": "workspace-recovery", "name": "AgentLaneIdentity"}),
        )
        .expect("symbols command resolves");
    let resolved_twice = nav
        .resolve(
            BackendNavigationCommand::Symbols,
            json!({"name": "AgentLaneIdentity", "workspace_id": "workspace-recovery"}),
        )
        .expect("symbols command resolves deterministically");
    assert_eq!(
        resolved_once.deterministic_cache_key, resolved_twice.deterministic_cache_key,
        "same command and params must produce a stable backend navigation key"
    );

    let checkpoint = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: lane.clone(),
            session_id: "session-cloud-recover".to_string(),
            workspace_id: "workspace-recovery".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-214".to_string(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_handoff_id: Some(handoff.handoff_id.clone()),
            navigation_command_id: Some(resolved_once.command_id.to_string()),
            resume_pointer: RecoveryResumePointer::MicroTask {
                mt_id: "MT-214".to_string(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["cargo test --test parallel_swarm_state_recovery_tests".to_string()],
            hbr_rows: vec!["HBR-SWARM-001".to_string(), "HBR-SWARM-004".to_string()],
            next_step_context: "resume at checkpoint recovery flow".to_string(),
            payload: json!({
                "pending": "implement minimal backend recovery flow",
                "chat_history_required": false
            }),
            compaction_reason: "session_compaction".to_string(),
            git_head: "abc1234".to_string(),
        })
        .await
        .expect("record checkpoint");
    assert!(checkpoint.event_ledger_event_id.starts_with("KE-"));

    let resumed_lane = local_lane("resume");
    let recovered = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            resumed_lane,
            "session-local-resume",
        )
        .await
        .expect("recover from checkpoint");
    assert_eq!(recovered.checkpoint.checkpoint_id, checkpoint.checkpoint_id);
    assert_eq!(
        recovered.resume_pointer,
        RecoveryResumePointer::MicroTask {
            mt_id: "MT-214".to_string()
        }
    );
    assert_eq!(recovered.checkpoint.touched_files, checkpoint.touched_files);
    assert_eq!(recovered.receipt.prior_session_id, "session-cloud-recover");
    assert_eq!(recovered.receipt.new_session_id, "session-local-resume");
    assert!(
        !serde_json::to_string(&recovered)
            .expect("serialize recovery")
            .contains("sk-test-secret"),
        "recovery evidence must not leak provider secrets"
    );

    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_recovery_receipts WHERE checkpoint_id = $1",
    )
    .bind(&checkpoint.checkpoint_id)
    .fetch_one(&pool)
    .await
    .expect("count recovery receipts");
    assert_eq!(receipt_count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovery_receipt_authority_failure_does_not_emit_false_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-recovery-authority-fail-{}", Uuid::now_v7());
    let checkpoint = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: local_lane("recovery-authority-source"),
            session_id: "session-recovery-authority-source".to_string(),
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-214".to_string(),
            claim_id: None,
            mailbox_handoff_id: None,
            navigation_command_id: Some("validation_state".to_string()),
            resume_pointer: RecoveryResumePointer::MicroTask {
                mt_id: "MT-214".to_string(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["cargo test --test parallel_swarm_state_recovery_tests".to_string()],
            hbr_rows: vec!["HBR-SWARM-004".to_string()],
            next_step_context: "valid checkpoint before forced recovery insert failure".to_string(),
            payload: json!({"checkpoint": "valid"}),
            compaction_reason: "test_recovery_authority_failure".to_string(),
            git_head: "feedface".to_string(),
        })
        .await
        .expect("valid checkpoint before forced recovery authority failure");

    install_recovery_receipt_authority_failure(&pool).await;
    let result = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("recovery-authority-fail"),
            "session-recovery-authority-fail",
        )
        .await;
    assert!(
        result.is_err(),
        "forced authority failure should reject recovery receipt"
    );

    let receipt_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_recovery_receipts WHERE checkpoint_id = $1",
    )
    .bind(&checkpoint.checkpoint_id)
    .fetch_one(&pool)
    .await
    .expect("count failed recovery receipt rows");
    assert_eq!(receipt_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_recovery",
            "new_session_id",
            "session-recovery-authority-fail",
        )
        .await,
        0,
        "failed recovery receipt insert must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_handoff_statuses_round_trip_from_postgres() {
    let Some((_pool, store)) = recovery_store().await else {
        return;
    };

    for status in [
        SwarmReceiptStatus::Started,
        SwarmReceiptStatus::Progress,
        SwarmReceiptStatus::Blocked,
        SwarmReceiptStatus::Pass,
        SwarmReceiptStatus::Fail,
    ] {
        let status_name = format!("{status:?}").to_ascii_lowercase();
        let handoff = store
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: cloud_lane(&format!("handoff-{status_name}")),
                to_role: "WP_VALIDATOR".to_string(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-211".to_string(),
                claim_id: None,
                mailbox_thread_id: format!("thread-{status_name}"),
                mailbox_message_id: format!("message-{status_name}"),
                status,
                summary: format!("round-trip {status_name} handoff status"),
                body_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
            })
            .await
            .expect("record handoff status");
        assert_eq!(
            handoff.status, status,
            "mailbox handoff status must decode from the database row"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_secret_like_provider_metadata_is_scrubbed_at_persist_time() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-raw-attribution-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-215".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: workspace_id.clone(),
            },
            lane: raw_cloud_lane("persist-time"),
            session_id: "session-raw-attribution".to_string(),
            ttl_seconds: 600,
            reason: "raw metadata should be scrubbed before persistence".to_string(),
        })
        .await
        .expect("workspace claim with raw cloud attribution");
    assert_eq!(claim.status, ClaimStatus::Active);

    let persisted_attribution: String = sqlx::query_scalar(
        "SELECT attribution_jsonb::text FROM knowledge_agent_worktree_claims WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .fetch_one(&pool)
    .await
    .expect("fetch persisted attribution");
    let persisted_event_payload: String =
        sqlx::query_scalar("SELECT payload::text FROM kernel_event_ledger WHERE event_id = $1")
            .bind(
                claim
                    .event_ledger_event_id
                    .as_deref()
                    .expect("claim has event receipt"),
            )
            .fetch_one(&pool)
            .await
            .expect("fetch persisted event payload");

    for persisted in [&persisted_attribution, &persisted_event_payload] {
        assert!(
            !persisted.contains("sk-raw-secret-must-not-persist"),
            "raw API key leaked into persisted attribution/event payload: {persisted}"
        );
        assert!(
            !persisted.contains("raw-token-must-not-persist"),
            "raw token leaked into persisted attribution/event payload: {persisted}"
        );
        assert!(
            persisted.contains("[REDACTED]"),
            "persisted metadata should retain redaction markers: {persisted}"
        );
        assert!(
            persisted.contains("org-visible"),
            "non-secret provider metadata should remain useful: {persisted}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_indexing_lease_queue_serializes_same_scope_writers_and_reclaims_orphans() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    let scope = ClaimScope::IndexRun {
        workspace_id: "workspace-index".to_string(),
        source_root_id: "root-a".to_string(),
    };
    let first = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: "workspace-index".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-216".to_string(),
            scope: scope.clone(),
            lane: local_lane("index-a"),
            session_id: "session-index-a".to_string(),
            index_run_id: format!("index-run-{}", Uuid::now_v7()),
            priority: 10,
            ttl_seconds: 600,
        })
        .await
        .expect("first index lease");
    assert_eq!(first.status, IndexLeaseStatus::Acquired);

    let second = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: "workspace-index".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-216".to_string(),
            scope: scope.clone(),
            lane: local_lane("index-b"),
            session_id: "session-index-b".to_string(),
            index_run_id: format!("index-run-{}", Uuid::now_v7()),
            priority: 20,
            ttl_seconds: 600,
        })
        .await
        .expect("second index lease queues");
    assert_eq!(second.status, IndexLeaseStatus::Queued);
    assert_eq!(
        second.blocked_by_lease_id.as_deref(),
        Some(first.lease_id.as_str())
    );
    let (queued_event_type, queued_status) =
        ledger_event_type_and_status(&pool, &second.event_ledger_event_id).await;
    assert_eq!(queued_event_type, "SESSION_QUEUED");
    assert_eq!(queued_status, "queued");

    let active = store
        .active_index_writer_for_scope(&scope)
        .await
        .expect("active index writer")
        .expect("writer exists");
    assert_eq!(active.lease_id, first.lease_id);

    store
        .complete_indexing_lease(&first.lease_id, &local_lane("index-a"))
        .await
        .expect("complete first lease");
    let promoted = store
        .acquire_next_indexing_lease(&scope)
        .await
        .expect("acquire next")
        .expect("queued lease promoted");
    assert_eq!(promoted.lease_id, second.lease_id);
    assert_eq!(promoted.status, IndexLeaseStatus::Acquired);
    assert_ne!(
        promoted.event_ledger_event_id, second.event_ledger_event_id,
        "queued lease promotion must attach a fresh acquired receipt"
    );
    let (promoted_event_type, promoted_status) =
        ledger_event_type_and_status(&pool, &promoted.event_ledger_event_id).await;
    assert_eq!(promoted_event_type, "KNOWLEDGE_INDEX_RUN_STARTED");
    assert_eq!(promoted_status, "acquired");

    sqlx::query(
        "UPDATE knowledge_parallel_indexing_lease_queue SET expires_at_utc = NOW() - INTERVAL '1 second' WHERE lease_id = $1",
    )
    .bind(&promoted.lease_id)
    .execute(&pool)
    .await
    .expect("force orphaned lease in isolated test schema");
    let reclaimed = store
        .reclaim_orphaned_indexing_leases()
        .await
        .expect("reclaim orphans");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].lease_id, promoted.lease_id);
    assert_eq!(reclaimed[0].status, IndexLeaseStatus::Reclaimed);
    assert_ne!(
        reclaimed[0].event_ledger_event_id, promoted.event_ledger_event_id,
        "orphan reclaim must attach a fresh reclaimed receipt"
    );
    let (reclaimed_event_type, reclaimed_status) =
        ledger_event_type_and_status(&pool, &reclaimed[0].event_ledger_event_id).await;
    assert_eq!(reclaimed_event_type, "KNOWLEDGE_INDEX_RUN_CANCELLED");
    assert_eq!(reclaimed_status, "reclaimed");
    assert!(
        store
            .active_index_writer_for_scope(&scope)
            .await
            .expect("active writer after reclaim")
            .is_none(),
        "expired writer must not remain active after reclaim"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_scope_indexing_lease_records_only_real_outcome_events() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    install_parallel_swarm_event_delay(&pool, "parallel_indexing_lease").await;

    let workspace_id = format!("workspace-lease-race-{}", Uuid::now_v7());
    let scope = ClaimScope::IndexRun {
        workspace_id: workspace_id.clone(),
        source_root_id: "root-lease-race".to_string(),
    };
    let barrier = Arc::new(Barrier::new(2));
    let left_store = store.clone();
    let left_barrier = barrier.clone();
    let left_workspace = workspace_id.clone();
    let left_scope = scope.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        left_store
            .enqueue_indexing_lease(IndexingLeaseRequest {
                workspace_id: left_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-216".to_string(),
                scope: left_scope,
                lane: local_lane("lease-race-a"),
                session_id: "session-lease-race-a".to_string(),
                index_run_id: format!("index-run-{}", Uuid::now_v7()),
                priority: 10,
                ttl_seconds: 600,
            })
            .await
    });
    let right_store = store.clone();
    let right_barrier = barrier.clone();
    let right_workspace = workspace_id.clone();
    let right_scope = scope.clone();
    let right = tokio::spawn(async move {
        right_barrier.wait().await;
        right_store
            .enqueue_indexing_lease(IndexingLeaseRequest {
                workspace_id: right_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-216".to_string(),
                scope: right_scope,
                lane: local_lane("lease-race-b"),
                session_id: "session-lease-race-b".to_string(),
                index_run_id: format!("index-run-{}", Uuid::now_v7()),
                priority: 20,
                ttl_seconds: 600,
            })
            .await
    });

    let left = left.await.expect("left lease task joins");
    let right = right.await.expect("right lease task joins");
    assert!(
        left.is_ok(),
        "left lease should resolve to an outcome: {left:?}"
    );
    assert!(
        right.is_ok(),
        "right lease should resolve to an outcome: {right:?}"
    );
    let outcomes = [left.expect("left lease"), right.expect("right lease")];
    assert_eq!(
        outcomes
            .iter()
            .filter(|lease| lease.status == IndexLeaseStatus::Acquired)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|lease| lease.status == IndexLeaseStatus::Queued)
            .count(),
        1
    );

    let lease_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count lease race rows");
    assert_eq!(lease_rows, 2);

    let acquired_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_indexing_lease'
          AND payload ->> 'workspace_id' = $1
          AND payload ->> 'status' = 'acquired'
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count acquired lease events");
    let queued_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_indexing_lease'
          AND payload ->> 'workspace_id' = $1
          AND payload ->> 'status' = 'queued'
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count queued lease events");
    assert_eq!(
        acquired_events, 1,
        "only the real acquired lease may emit an acquired event"
    );
    assert_eq!(
        queued_events, 1,
        "the race loser should persist as a queued lease with a queued event"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_expired_claim_reclaim_records_event_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-explicit-reclaim-{}", Uuid::now_v7());
    let lane = local_lane("explicit-reclaim-holder");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-reclaim-{}", Uuid::now_v7()),
            },
            lane: lane.clone(),
            session_id: "session-explicit-reclaim-holder".to_string(),
            ttl_seconds: 600,
            reason: "claim that will be made stale in the isolated test schema".to_string(),
        })
        .await
        .expect("claim to reclaim");
    sqlx::query(
        "UPDATE knowledge_agent_worktree_claims SET expires_at_utc = NOW() - INTERVAL '1 second' WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .execute(&pool)
    .await
    .expect("force expired claim in isolated test schema");

    let reclaimed = store
        .reclaim_expired_work_claims(
            &local_lane("explicit-reclaimer"),
            "session-explicit-reclaim",
            "explicit stale claim sweep",
        )
        .await
        .expect("explicit reclaim expired claims");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].claim_id, claim.claim_id);
    assert_eq!(reclaimed[0].status, ClaimStatus::Reclaimed);
    assert!(
        reclaimed[0]
            .reclaim_event_ledger_event_id
            .as_deref()
            .is_some_and(|event_id| event_id.starts_with("KE-")),
        "reclaimed claims must carry a reclaim event receipt"
    );

    let receipt_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_swarm_claim_reclaim'
          AND payload ->> 'claim_id' = $1
          AND payload ->> 'reason' = 'explicit stale claim sweep'
        "#,
    )
    .bind(&claim.claim_id)
    .fetch_one(&pool)
    .await
    .expect("count reclaim receipt events");
    assert_eq!(receipt_events, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_claim_reclaim_rolls_back_if_receipt_insert_fails() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-reclaim-failure-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-reclaim-fail-{}", Uuid::now_v7()),
            },
            lane: local_lane("reclaim-failure-holder"),
            session_id: "session-reclaim-failure-holder".to_string(),
            ttl_seconds: 600,
            reason: "claim that should remain active if reclaim receipt fails".to_string(),
        })
        .await
        .expect("claim before forced reclaim receipt failure");
    sqlx::query(
        "UPDATE knowledge_agent_worktree_claims SET expires_at_utc = NOW() - INTERVAL '1 second' WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .execute(&pool)
    .await
    .expect("force expired claim before failure injection");

    install_parallel_swarm_event_failure(&pool, "parallel_swarm_claim_reclaim").await;
    let result = store
        .reclaim_expired_work_claims(
            &local_lane("reclaim-failure-reclaimer"),
            "session-reclaim-failure",
            "forced reclaim receipt failure",
        )
        .await;
    assert!(
        result.is_err(),
        "forced EventLedger failure should reject reclaim"
    );

    let (status, reclaim_event_id): (String, Option<String>) = sqlx::query_as(
        "SELECT status, reclaim_event_ledger_event_id FROM knowledge_agent_worktree_claims WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .fetch_one(&pool)
    .await
    .expect("fetch claim after failed reclaim");
    assert_eq!(
        status, "active",
        "claim must not be left reclaimed when receipt insertion fails"
    );
    assert!(
        reclaim_event_id.is_none(),
        "failed reclaim must not attach a missing receipt id"
    );
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim_reclaim",
            "claim_id",
            &claim.claim_id,
        )
        .await,
        0,
        "failed reclaim must not leave a false EventLedger receipt"
    );
}
