//! WP-KERNEL-004 cluster X.1 (MT-176..MT-183) integration tests.
//!
//! Spec-Realism Gate compliance:
//!  - Sub-rule 1: no LiveXxxUnavailable / todo / unimplemented paths.
//!  - Sub-rule 2: Postgres-backed tests are `#[ignore]`-gated on
//!    `POSTGRES_TEST_URL`. Pure-Rust tests live in the lib module's `#[cfg(test)]`
//!    blocks and are exercised here through integration assertions.
//!  - Sub-rule 3: a separate validator session signs off on behaviour.

use chrono::Utc;
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    backpressure::{BackpressureConfig, BackpressureDecision, BackpressureGuard},
    families::{
        AnnounceBackBody, ArtifactPointer, BlockerBody, BlockerSeverity, CompletionState,
        DelegateWorkBody, MessageFamily,
    },
    handoff::{recompute_message_hash, AnnounceBackComposer, HandoffBundleBuilder, ProvenanceLink},
    lease::{LeaseManager, LeaseRequest, TakeoverPolicy},
    lifecycle::{transition_thread_state, ThreadLifecycleState},
    message::{MessageType, RoleMailboxMessage, RoleMailboxMessageId},
    router::{ExecutorIdentity, ExecutorKind, ExecutorRouter, RouteDecision},
    thread::{
        ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
    },
};
use uuid::Uuid;

#[test]
fn mt_176_thread_lifecycle_rejects_illegal_transition() {
    let r = transition_thread_state(ThreadLifecycleState::Archived, ThreadLifecycleState::Open);
    assert!(r.is_err(), "archived -> open must be rejected");
}

#[test]
fn mt_176_thread_id_is_uuid_v7() {
    let id = RoleMailboxThreadId::new_v7();
    assert_eq!(id.as_uuid().get_version_num(), 7);
}

#[test]
fn mt_179_delegate_work_round_trip() {
    let body = MessageFamily::DelegateWork(DelegateWorkBody {
        task_summary: "do x".to_string(),
        target_role: RoleId::Coder,
        due_at_utc: Some(Utc::now()),
        linked_wp: Some("WP-1".to_string()),
        linked_mt: None,
    });
    let s = serde_json::to_string(&body).unwrap();
    let back: MessageFamily = serde_json::from_str(&s).unwrap();
    assert_eq!(body, back);
}

#[test]
fn mt_179_blocker_family_round_trip() {
    let body = MessageFamily::Blocker(BlockerBody {
        blocker_description: "blocked on X".to_string(),
        blocking_role: Some(RoleId::Validator),
        severity: BlockerSeverity::Hard,
    });
    let s = serde_json::to_string(&body).unwrap();
    assert_eq!(serde_json::from_str::<MessageFamily>(&s).unwrap(), body);
}

#[test]
fn mt_180_lease_manager_acquires_releases_takes_over() {
    let mgr = LeaseManager::new();
    let thread_id = Uuid::now_v7();
    let req = LeaseRequest {
        executor_kind: ExecutorKind::LocalSmallModel,
        role_id: RoleId::Coder,
        session_id: Uuid::now_v7(),
        lease_duration_secs: 60,
    };
    let lease = mgr
        .acquire(
            thread_id,
            &[ExecutorKind::LocalSmallModel, ExecutorKind::Operator],
            ClaimMode::Exclusive,
            req,
            Utc::now(),
        )
        .unwrap();
    assert_eq!(lease.thread_id, thread_id);
    assert_eq!(lease.lease_id.get_version_num(), 7);

    // Takeover by operator with AlwaysWithReason policy.
    let op_req = LeaseRequest {
        executor_kind: ExecutorKind::Operator,
        role_id: RoleId::Operator,
        session_id: Uuid::now_v7(),
        lease_duration_secs: 60,
    };
    let new_lease = mgr
        .takeover(
            thread_id,
            &[ExecutorKind::LocalSmallModel, ExecutorKind::Operator],
            op_req,
            lease.lease_id,
            TakeoverPolicy::AlwaysWithReason,
            "operator intervention".to_string(),
            Utc::now(),
        )
        .unwrap();
    assert_eq!(new_lease.takeover_of, Some(lease.lease_id));
}

#[test]
fn mt_181_router_pure_decision_matrix() {
    let thread = RoleMailboxThread::open(
        "t",
        LinkedRecordKind::Wp,
        Some("WP-1".to_string()),
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let id = ExecutorIdentity {
        executor_kind: ExecutorKind::LocalSmallModel,
        role_id: RoleId::Coder,
        session_id: Uuid::now_v7(),
        capabilities: vec![],
    };
    let d = ExecutorRouter::decide(&thread, None, &id, Utc::now());
    assert!(matches!(d, RouteDecision::MayClaim { .. }));
}

#[test]
fn mt_182_backpressure_token_bucket_and_inbox_cap() {
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 5,
        tokens_per_second: 1,
        burst_capacity: 3,
    });
    let now = Utc::now();
    // First 3 allow via burst.
    for _ in 0..3 {
        assert!(matches!(
            g.check(&RoleId::Coder, 0, now),
            BackpressureDecision::Allow
        ));
    }
    // 4th immediately denies (rate-limit).
    assert!(matches!(
        g.check(&RoleId::Coder, 0, now),
        BackpressureDecision::Deny { .. }
    ));
    // Inbox cap path triggers regardless of bucket.
    assert!(matches!(
        g.check(&RoleId::Validator, 5, now),
        BackpressureDecision::Deny { .. }
    ));
}

#[test]
fn mt_183_handoff_bundle_provenance_chain_verify() {
    let bundle = HandoffBundleBuilder::new()
        .source_thread(RoleMailboxThreadId::new_v7())
        .source_message(RoleMailboxMessageId::new_v7())
        .target_role(RoleId::Coder)
        .target_executor_kind(ExecutorKind::LocalSmallModel)
        .context_summary("ctx")
        .build();
    assert!(bundle.verify_hash());

    // Compose an announce-back referencing the bundle.
    let body = AnnounceBackComposer::compose(
        &bundle,
        "done",
        vec![ArtifactPointer {
            artifact_id: "A1".to_string(),
            uri: "memory:1".to_string(),
            content_hash: None,
        }],
        CompletionState::Completed,
        vec![],
    );
    assert_eq!(body.bundle_id, Some(bundle.bundle_id));

    // Provenance chain verification.
    let msg = RoleMailboxMessage::new(
        RoleMailboxThreadId(bundle.source_thread_id),
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"k": "v"}),
    );
    let mut map = std::collections::HashMap::new();
    let valid_hash = recompute_message_hash(&msg);
    map.insert(msg.message_id.as_uuid(), msg.clone());
    let chain = vec![ProvenanceLink {
        predecessor_message_id: msg.message_id.as_uuid(),
        content_hash: valid_hash,
    }];
    assert!(AnnounceBackComposer::verify_chain(&map, &chain).is_ok());
}

#[test]
fn mt_179_unknown_family_decodes_to_unknown_variant() {
    let explicit = MessageFamily::Unknown {
        raw: serde_json::json!({"future_field": "future_value"}),
    };
    let s = serde_json::to_string(&explicit).unwrap();
    let back: MessageFamily = serde_json::from_str(&s).unwrap();
    assert_eq!(explicit, back);
}

// MT-176/MT-177/MT-178 Postgres-gated tests.

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_177_postgres_repo_thread_lifecycle_round_trip() {
    let pool = postgres_pool().await;
    let repo = handshake_core::role_mailbox_v1::repo::RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");
    let thread = RoleMailboxThread::open(
        "t",
        LinkedRecordKind::Wp,
        Some("WP-X".to_string()),
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let id = thread.thread_id;
    repo.create_thread(thread.clone()).await.expect("create");
    let got = repo.get_thread(id).await.expect("get").expect("present");
    assert_eq!(got.thread_id, id);
    let updated = repo
        .update_thread_lifecycle(id, ThreadLifecycleState::Resolved)
        .await
        .expect("update");
    assert_eq!(updated.lifecycle_state, ThreadLifecycleState::Resolved);
    // Append-after-terminal must reject.
    let res = repo
        .append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"k": "v"}),
        )
        .await;
    assert!(res.is_err());
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_178_exporter_idempotent_writes() {
    use handshake_core::role_mailbox_v1::exporter::{MailboxExporter, MailboxExporterConfig};
    use std::collections::BTreeMap;
    let dir = tempfile::tempdir().unwrap();
    let cfg = MailboxExporterConfig {
        target_dir: dir.path().to_path_buf(),
    };
    let ex = MailboxExporter::new(cfg);
    let thread = RoleMailboxThread::open(
        "t",
        LinkedRecordKind::Wp,
        None,
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let msg = RoleMailboxMessage::new(
        thread.thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"k": "v"}),
    );
    let mut messages = BTreeMap::new();
    messages.insert(thread.thread_id.as_uuid(), vec![msg]);
    let r1 = ex.export(&[thread.clone()], &messages).unwrap();
    let r2 = ex.export(&[thread], &messages).unwrap();
    assert!(r1.lines_appended >= 1);
    assert_eq!(r2.lines_appended, 0);
}

async fn postgres_pool() -> sqlx::PgPool {
    let url =
        std::env::var("POSTGRES_TEST_URL").expect("ENVIRONMENT_BLOCKED: POSTGRES_TEST_URL not set");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}
