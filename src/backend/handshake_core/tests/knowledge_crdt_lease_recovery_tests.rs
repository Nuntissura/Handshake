//! WP-KERNEL-009 CRDTAndConcurrencyCore lease + recovery tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_076_agent_lease: MT-076 AgentLeaseExpiration (MT-041 seed)
//!   - mt_079_recovery_receipt: MT-079 CrdtRecoveryReceiptFormat
//!
//! Expiry is enforced on the embedded database clock; the expiry tests claim
//! 1-second leases and wait them out against the real store.

use handshake_core::kernel::crdt::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use handshake_core::kernel::crdt::agent_lease::{
    claim_lease, expire_due_leases, guard_lease_for_write, new_ulid, release_lease, renew_lease,
    takeover_lease, ulid_at, KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1, LeaseClaimRequestV1,
    LeaseTakeoverOutcomeV1, LeaseWriteDenialReasonV1, LeaseWriteGuardOutcomeV1,
};
use handshake_core::storage::knowledge_crdt::{get_lease, LeaseTakeoverFailure};
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig, TestFieldMutation,
    TestMutationValue,
};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::Database;

async fn embedded_backend_or_blocked() -> EmbeddedTestBackend {
    match embedded_test_backend().await {
        Ok(backend) => backend,
        Err(err) => panic!("failed to init embedded backend: {err:?}"),
    }
}

async fn reopen_embedded_store(
    backend: &EmbeddedTestBackend,
) -> (SurrealStorage, std::sync::Arc<dyn Database>) {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original embedded CRDT lease store");
    let reopened = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened embedded CRDT lease store"),
    )
    .await
    .expect("reopen embedded CRDT lease store");
    bootstrap_schema(&reopened)
        .await
        .expect("bootstrap reopened CRDT lease schema");
    let database: std::sync::Arc<dyn Database> =
        std::sync::Arc::new(SurrealDatabase::new(reopened.clone()));
    (reopened, database)
}

async fn close_reopened_and_remove(reopened: SurrealStorage, backend: EmbeddedTestBackend) {
    reopened
        .shutdown()
        .await
        .expect("close reopened CRDT lease store");
    drop(reopened);
    backend
        .close_and_remove()
        .await
        .expect("remove embedded CRDT lease store");
}

fn claim_request(
    actor: &KnowledgeActorIdV1,
    session: &str,
    scope_kind: KnowledgeLeaseScopeKind,
    scope_id: &str,
    ttl_seconds: i64,
) -> LeaseClaimRequestV1 {
    LeaseClaimRequestV1 {
        lane_id: format!("lane-{session}"),
        actor: actor.clone(),
        session_id: session.to_string(),
        correlation_id: format!("corr-{session}"),
        scope_kind,
        scope_id: scope_id.to_string(),
        ttl_seconds,
    }
}

/// Wait (bounded) until the database clock reports the lease as expired.
async fn wait_for_db_expiry(
    pool: &handshake_core::storage::surreal::SurrealStorage,
    lease_id: &str,
) {
    for _ in 0..40 {
        let lease = get_lease(pool, lease_id)
            .await
            .expect("get lease")
            .expect("lease exists");
        if lease.is_expired {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("lease {lease_id} did not expire on the database clock within 10s");
}

mod mt_076_agent_lease {
    use super::*;
    use handshake_core::kernel::KernelEventType;
    use uuid::Uuid;

    #[test]
    fn ulids_are_canonical_and_time_ordered() {
        let ulid = new_ulid();
        assert_eq!(ulid.len(), 26);
        assert!(ulid
            .chars()
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)));

        // Lexicographic order follows the timestamp.
        let earlier = ulid_at(1_000_000, 42);
        let later = ulid_at(2_000_000, 7);
        assert!(earlier < later);
        // Same millisecond: random tail decides, but length/alphabet hold.
        let twin_a = ulid_at(1_000_000, 1);
        let twin_b = ulid_at(1_000_000, 2);
        assert_eq!(&twin_a[..10], &twin_b[..10], "time prefix is shared");
        assert!(twin_a < twin_b);
    }

    /// Full transition battery on embedded storage: claim -> renew -> release with
    /// EventLedger receipts and idempotency keys; foreign renewals refused;
    /// expiry enforced server-side; takeover with lineage; every denial
    /// leaves a durable receipt.
    #[tokio::test]
    async fn lease_transitions_expiry_and_takeover_with_event_receipts() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt076-{suffix}");
        let holder =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "holder-lm").expect("actor");
        let intruder =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "intruder-cm").expect("actor");

        // Claim.
        let claimed = claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &holder,
                &format!("sr-hold-{suffix}"),
                KnowledgeLeaseScopeKind::Workspace,
                &ws,
                600,
            ),
        )
        .await
        .expect("claim flow");
        let lease = match claimed {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("expected claim, got {other:?}"),
        };
        assert_eq!(lease.renewal_count, 0);
        assert!(lease.is_active());

        // Renew extends and counts; renewal event carries a per-renewal key.
        let renewed = renew_lease(db.as_ref(), &pool, &lease.lease_id, &holder, 900)
            .await
            .expect("renew flow")
            .expect("own active lease renews");
        assert_eq!(renewed.renewal_count, 1);
        assert!(renewed.expires_at_utc > lease.expires_at_utc);

        // Foreign renewal is refused server-side (no row matched).
        let foreign_renew = renew_lease(db.as_ref(), &pool, &lease.lease_id, &intruder, 900)
            .await
            .expect("renew flow");
        assert!(foreign_renew.is_none());

        // Foreign WRITE under the lease leaves a durable typed denial.
        let denial = guard_lease_for_write(
            db.as_ref(),
            &pool,
            &lease.lease_id,
            &intruder,
            "sr-intrude",
            "corr-intrude",
            &ws,
            KnowledgeLeaseScopeKind::Workspace,
            &ws,
        )
        .await
        .expect("guard flow");
        match denial {
            LeaseWriteGuardOutcomeV1::Denied(denial) => {
                assert!(matches!(
                    denial.reason,
                    LeaseWriteDenialReasonV1::ForeignLease { .. }
                ));
                assert!(!denial.denial_receipt_id.is_empty());
                assert!(!denial.event_ledger_event_id.is_empty());
            }
            other => panic!("foreign write must be denied, got {other:?}"),
        }

        // The holder's write passes the guard.
        let allowed = guard_lease_for_write(
            db.as_ref(),
            &pool,
            &lease.lease_id,
            &holder,
            &format!("sr-hold-{suffix}"),
            "corr-hold",
            &ws,
            KnowledgeLeaseScopeKind::Workspace,
            &ws,
        )
        .await
        .expect("guard flow");
        assert!(matches!(allowed, LeaseWriteGuardOutcomeV1::Allowed(_)));

        // Release; transition event sequence is in the ledger.
        release_lease(db.as_ref(), &pool, &lease.lease_id, &holder)
            .await
            .expect("release flow")
            .expect("own lease releases");
        let events = db
            .list_kernel_events_for_aggregate("knowledge_agent_lease", &lease.lease_id)
            .await
            .expect("events");
        for expected in [
            KernelEventType::KnowledgeCrdtLeaseClaimed,
            KernelEventType::KnowledgeCrdtLeaseRenewed,
            KernelEventType::KnowledgeCrdtLeaseWriteDenied,
            KernelEventType::KnowledgeCrdtLeaseReleased,
        ] {
            assert!(
                events.iter().any(|event| event.event_type == expected),
                "missing lease event {expected:?}"
            );
        }
        // Idempotency keys are present and unique per transition.
        let mut keys: Vec<&str> = events
            .iter()
            .map(|event| event.idempotency_key.as_str())
            .collect();
        keys.sort_unstable();
        let before_dedup = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before_dedup, "idempotency keys must be unique");

        // Expiry: 1-second lease on a fresh scope, enforced by the DB clock.
        let scope = format!("{ws}-expiry");
        let short = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &holder,
                &format!("sr-short-{suffix}"),
                KnowledgeLeaseScopeKind::SourceRoot,
                &scope,
                1,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("expected claim, got {other:?}"),
        };
        wait_for_db_expiry(&pool, &short.lease_id).await;

        // Writes under the expired lease are denied durably.
        let expired_write = guard_lease_for_write(
            db.as_ref(),
            &pool,
            &short.lease_id,
            &holder,
            &format!("sr-short-{suffix}"),
            "corr-short",
            &ws,
            KnowledgeLeaseScopeKind::SourceRoot,
            &scope,
        )
        .await
        .expect("guard flow");
        assert!(matches!(
            expired_write,
            LeaseWriteGuardOutcomeV1::Denied(denial)
                if matches!(denial.reason, LeaseWriteDenialReasonV1::LeaseExpired { .. })
        ));

        // Renewal after expiry is refused server-side.
        assert!(
            renew_lease(db.as_ref(), &pool, &short.lease_id, &holder, 600)
                .await
                .expect("renew flow")
                .is_none()
        );

        // Takeover of an ACTIVE lease is refused...
        let active_scope = format!("{ws}-active");
        let active = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &holder,
                &format!("sr-active-{suffix}"),
                KnowledgeLeaseScopeKind::SourceRoot,
                &active_scope,
                600,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("expected claim, got {other:?}"),
        };
        let premature_takeover = takeover_lease(
            db.as_ref(),
            &pool,
            &active.lease_id,
            claim_request(
                &intruder,
                &format!("sr-take-{suffix}"),
                KnowledgeLeaseScopeKind::SourceRoot,
                &active_scope,
                600,
            ),
        )
        .await
        .expect("takeover flow");
        assert!(matches!(
            premature_takeover,
            LeaseTakeoverOutcomeV1::Refused(LeaseTakeoverFailure::PriorLeaseNotExpired { .. })
        ));

        // ...but the EXPIRED lease's scope can be taken over with lineage.
        let takeover = takeover_lease(
            db.as_ref(),
            &pool,
            &short.lease_id,
            claim_request(
                &intruder,
                &format!("sr-take-{suffix}"),
                KnowledgeLeaseScopeKind::SourceRoot,
                &scope,
                600,
            ),
        )
        .await
        .expect("takeover flow");
        let new_lease = match takeover {
            LeaseTakeoverOutcomeV1::TakenOver(lease) => lease,
            other => panic!("expected takeover, got {other:?}"),
        };
        assert_eq!(
            new_lease.takeover_of.as_deref(),
            Some(short.lease_id.as_str())
        );
        let takeover_events = db
            .list_kernel_events_for_aggregate("knowledge_agent_lease", &new_lease.lease_id)
            .await
            .expect("events");
        assert!(takeover_events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtLeaseTakenOver));

        // The expiry sweep stamps remaining overdue leases + emits events.
        let sweep_scope = format!("{ws}-sweep");
        let sweep_lease = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &holder,
                &format!("sr-sweep-{suffix}"),
                KnowledgeLeaseScopeKind::IndexRun,
                &sweep_scope,
                1,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("expected claim, got {other:?}"),
        };
        wait_for_db_expiry(&pool, &sweep_lease.lease_id).await;
        let swept = expire_due_leases(db.as_ref(), &pool).await.expect("sweep");
        assert!(swept
            .iter()
            .any(|lease| lease.lease_id == sweep_lease.lease_id));
        let sweep_events = db
            .list_kernel_events_for_aggregate("knowledge_agent_lease", &sweep_lease.lease_id)
            .await
            .expect("events");
        assert!(sweep_events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtLeaseExpired));

        let takeover_lease_id = new_lease.lease_id.clone();
        let sweep_lease_id = sweep_lease.lease_id.clone();
        drop(pool);
        drop(db);
        let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
        let reopened_takeover = get_lease(&reopened, &takeover_lease_id)
            .await
            .expect("get takeover lease after reopen")
            .expect("takeover lease survives reopen");
        assert_eq!(
            reopened_takeover.takeover_of.as_deref(),
            Some(short.lease_id.as_str())
        );
        let reopened_sweep = get_lease(&reopened, &sweep_lease_id)
            .await
            .expect("get swept lease after reopen")
            .expect("swept lease survives reopen");
        assert!(reopened_sweep.is_expired);
        let reopened_takeover_events = reopened_db
            .list_kernel_events_for_aggregate("knowledge_agent_lease", &takeover_lease_id)
            .await
            .expect("list takeover events after reopen");
        assert!(reopened_takeover_events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtLeaseTakenOver));
        let reopened_sweep_events = reopened_db
            .list_kernel_events_for_aggregate("knowledge_agent_lease", &sweep_lease_id)
            .await
            .expect("list expiry events after reopen");
        assert!(reopened_sweep_events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtLeaseExpired));
        drop(reopened_db);
        close_reopened_and_remove(reopened, backend).await;
    }
}

mod mt_079_recovery_receipt {
    use super::*;
    use handshake_core::kernel::crdt::recovery_receipt::{
        recover_from_checkpoint, write_swarm_checkpoint, RecoveryFailureV1,
        SwarmCheckpointRequestV1, SwarmResumePointerV1,
    };
    use handshake_core::kernel::KernelEventType;
    use handshake_core::storage::knowledge_crdt::list_recovery_receipts_for_checkpoint;
    use serde_json::json;
    use uuid::Uuid;

    /// Seed scenario: a session checkpoints its work, dies (lease expires),
    /// a new session takes over the lease and recovers from the checkpoint.
    /// The recovery receipt links new session -> checkpoint -> lease lineage
    /// and everything reconstructs from embedded storage alone.
    #[tokio::test]
    async fn session_loss_recovery_emits_linked_receipt_reconstructable_from_embedded_store() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let scope_id = format!("crdt-mt079-{suffix}");
        let lane_id = format!("lane-mt079-{suffix}");
        let dying =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "dying-lm").expect("actor");
        let recovering = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "recovering-lm")
            .expect("actor");

        // Session 1 claims a short lease and checkpoints mid-work.
        let lease_one = match claim_lease(
            db.as_ref(),
            &pool,
            LeaseClaimRequestV1 {
                lane_id: lane_id.clone(),
                actor: dying.clone(),
                session_id: format!("sr-dying-{suffix}"),
                correlation_id: format!("corr-dying-{suffix}"),
                scope_kind: KnowledgeLeaseScopeKind::Document,
                scope_id: scope_id.clone(),
                ttl_seconds: 1,
            },
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("expected claim, got {other:?}"),
        };

        let checkpoint = write_swarm_checkpoint(
            db.as_ref(),
            &pool,
            SwarmCheckpointRequestV1 {
                session_id: format!("sr-dying-{suffix}"),
                actor: dying.clone(),
                lane_id: lane_id.clone(),
                lease_id: lease_one.lease_id.clone(),
                scope_ref: format!("document:{scope_id}"),
                resume_pointer: SwarmResumePointerV1::DocumentRevision {
                    crdt_document_id: scope_id.clone(),
                    update_seq: 7,
                    state_vector: "hsk-sv1:site-aaaa=7".to_string(),
                },
                checkpoint_payload: json!({
                    "pending_block_ids": ["blk-1", "blk-2"],
                    "draft_summary": "halfway through reference rewrite"
                }),
            },
        )
        .await
        .expect("checkpoint written");
        assert_eq!(checkpoint.payload_sha256.len(), 64);

        // Session 1 dies; the lease expires on the database clock.
        wait_for_db_expiry(&pool, &lease_one.lease_id).await;

        // Session 2 takes over the scope.
        let lease_two = match takeover_lease(
            db.as_ref(),
            &pool,
            &lease_one.lease_id,
            LeaseClaimRequestV1 {
                lane_id: lane_id.clone(),
                actor: recovering.clone(),
                session_id: format!("sr-recover-{suffix}"),
                correlation_id: format!("corr-recover-{suffix}"),
                scope_kind: KnowledgeLeaseScopeKind::Document,
                scope_id: scope_id.clone(),
                ttl_seconds: 600,
            },
        )
        .await
        .expect("takeover flow")
        {
            LeaseTakeoverOutcomeV1::TakenOver(lease) => lease,
            other => panic!("expected takeover, got {other:?}"),
        };

        // Recovery: payload verified, lineage walked, receipt + event.
        let recovery = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            &checkpoint.checkpoint_id,
            &format!("sr-recover-{suffix}"),
            &recovering,
            &lease_two.lease_id,
        )
        .await
        .expect("recovery flow")
        .expect("recovery succeeds");
        assert_eq!(
            recovery.lease_lineage_ids,
            vec![lease_two.lease_id.clone(), lease_one.lease_id.clone()],
            "lineage runs newest -> original claim"
        );
        assert_eq!(
            recovery.receipt.prior_session_id,
            format!("sr-dying-{suffix}")
        );
        assert_eq!(
            recovery.receipt.new_session_id,
            format!("sr-recover-{suffix}")
        );
        match &recovery.resume_pointer {
            SwarmResumePointerV1::DocumentRevision {
                crdt_document_id,
                update_seq,
                state_vector,
            } => {
                assert_eq!(crdt_document_id, &scope_id);
                assert_eq!(*update_seq, 7);
                assert_eq!(state_vector, "hsk-sv1:site-aaaa=7");
            }
            other => panic!("resume pointer round-trip failed: {other:?}"),
        }

        // Receipt + checkpoint events are in the ledger.
        let events = db
            .list_kernel_events_for_aggregate(
                "knowledge_swarm_checkpoint",
                &checkpoint.checkpoint_id,
            )
            .await
            .expect("events");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtCheckpointRecorded));
        assert!(events.iter().any(|event| {
            event.event_type == KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded
        }));

        // No-chat-history reconstruction: a fresh reader sees the same
        // receipt purely from embedded-store rows.
        let receipts = list_recovery_receipts_for_checkpoint(&pool, &checkpoint.checkpoint_id)
            .await
            .expect("receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0], recovery.receipt);

        // Failure path: unknown checkpoint. The tampered-payload branch is
        // covered by the active corruption-seam proof below.
        let unknown = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "sr-x",
            &recovering,
            &lease_two.lease_id,
        )
        .await
        .expect("recovery flow");
        assert!(matches!(
            unknown,
            Err(RecoveryFailureV1::CheckpointNotFound { .. })
        ));

        drop(pool);
        drop(db);
        let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
        let reopened_receipts =
            list_recovery_receipts_for_checkpoint(&reopened, &checkpoint.checkpoint_id)
                .await
                .expect("read recovery receipts after reopen");
        assert_eq!(reopened_receipts, vec![recovery.receipt.clone()]);
        let reopened_events = reopened_db
            .list_kernel_events_for_aggregate(
                "knowledge_swarm_checkpoint",
                &checkpoint.checkpoint_id,
            )
            .await
            .expect("read checkpoint events after reopen");
        assert!(reopened_events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtCheckpointRecorded));
        assert!(reopened_events.iter().any(|event| {
            event.event_type == KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded
        }));
        let reopened_lease = get_lease(&reopened, &lease_two.lease_id)
            .await
            .expect("read recovery lease after reopen")
            .expect("recovery lease survives reopen");
        assert_eq!(reopened_lease.takeover_of, Some(lease_one.lease_id));
        drop(reopened_db);
        close_reopened_and_remove(reopened, backend).await;

        // The active tamper proof below uses the catalog-validated embedded
        // mutation seam and remains separate from the successful recovery path.
    }

    #[tokio::test]
    async fn tampered_checkpoint_payload_is_rejected_after_reopen() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let scope_id = format!("crdt-mt079-tamper-{suffix}");
        let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "tamper-owner-lm")
            .expect("actor");
        let session_id = format!("sr-tamper-{suffix}");
        let lease = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &actor,
                &session_id,
                KnowledgeLeaseScopeKind::Document,
                &scope_id,
                600,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => lease,
            other => panic!("expected claim, got {other:?}"),
        };
        let checkpoint = write_swarm_checkpoint(
            db.as_ref(),
            &pool,
            SwarmCheckpointRequestV1 {
                session_id: session_id.clone(),
                actor: actor.clone(),
                lane_id: format!("lane-{session_id}"),
                lease_id: lease.lease_id.clone(),
                scope_ref: format!("document:{scope_id}"),
                resume_pointer: SwarmResumePointerV1::MicroTask {
                    mt_id: "MT-079".to_owned(),
                },
                checkpoint_payload: json!({
                    "pending_block_ids": ["blk-original"],
                    "draft_summary": "authentic checkpoint payload"
                }),
            },
        )
        .await
        .expect("checkpoint written");
        assert_eq!(checkpoint.payload_sha256.len(), 64);

        let checkpoints = pool
            .test_inspector()
            .table_selector("knowledge_crdt_swarm_checkpoints")
            .await
            .expect("select checkpoint table through the test catalog");
        let payload_field = checkpoints
            .field("checkpoint_payload")
            .expect("select checkpoint payload field through the test catalog");
        pool.test_mutator()
            .corrupt_row_field(
                &checkpoints,
                checkpoint.checkpoint_id.as_str(),
                &TestFieldMutation::new(
                    payload_field,
                    TestMutationValue::json(json!({
                        "pending_block_ids": ["blk-tampered"],
                        "draft_summary": "tampered checkpoint payload"
                    })),
                ),
            )
            .await
            .expect("tamper only the persisted checkpoint payload through the mutation seam");

        let before = list_recovery_receipts_for_checkpoint(&pool, &checkpoint.checkpoint_id)
            .await
            .expect("read recovery receipts before rejection");
        assert!(before.is_empty());
        let result = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            &checkpoint.checkpoint_id,
            "sr-tamper-recovery",
            &actor,
            &lease.lease_id,
        )
        .await
        .expect("recovery flow");
        assert!(matches!(
            result,
            Err(RecoveryFailureV1::PayloadHashMismatch { checkpoint_id, .. })
                if checkpoint_id == checkpoint.checkpoint_id
        ));
        let after = list_recovery_receipts_for_checkpoint(&pool, &checkpoint.checkpoint_id)
            .await
            .expect("read recovery receipts after rejection");
        assert!(
            after.is_empty(),
            "tampered payload must not emit a recovery receipt"
        );
        let events = db
            .list_kernel_events_for_aggregate(
                "knowledge_swarm_checkpoint",
                &checkpoint.checkpoint_id,
            )
            .await
            .expect("read checkpoint EventLedger rows");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtCheckpointRecorded));
        assert!(!events.iter().any(|event| {
            event.event_type == KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded
        }));

        drop(pool);
        drop(db);
        let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
        let reopened_result = recover_from_checkpoint(
            reopened_db.as_ref(),
            &reopened,
            &checkpoint.checkpoint_id,
            "sr-tamper-recovery-after-reopen",
            &actor,
            &lease.lease_id,
        )
        .await
        .expect("recovery flow after reopen");
        assert!(matches!(
            reopened_result,
            Err(RecoveryFailureV1::PayloadHashMismatch { checkpoint_id, .. })
                if checkpoint_id == checkpoint.checkpoint_id
        ));
        let reopened_receipts =
            list_recovery_receipts_for_checkpoint(&reopened, &checkpoint.checkpoint_id)
                .await
                .expect("read reopened recovery receipts");
        assert!(reopened_receipts.is_empty());
        drop(reopened_db);
        close_reopened_and_remove(reopened, backend).await;
    }

    /// Authority-hardening #3: recovery is only authorized under a lease that
    /// is unreleased, unexpired (DB clock), owned by the recovering actor, and
    /// scope-covering. Each bad lease yields a TYPED failure and writes NO
    /// recovery receipt. Previously recovery checked lease existence alone.
    #[tokio::test]
    async fn recovery_refuses_released_expired_foreign_or_wrong_scope_lease() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let scope_id = format!("crdt-mt079neg-{suffix}");
        let lane_id = format!("lane-mt079neg-{suffix}");
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "owner-lm").expect("actor");
        let stranger =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "stranger-lm").expect("actor");

        // A checkpoint scoped to `document:<scope_id>`.
        let lease_for_ckpt = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &actor,
                &format!("sr-ckpt-{suffix}"),
                KnowledgeLeaseScopeKind::Document,
                &scope_id,
                600,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => *lease,
            other => panic!("expected claim, got {other:?}"),
        };
        let checkpoint = write_swarm_checkpoint(
            db.as_ref(),
            &pool,
            SwarmCheckpointRequestV1 {
                session_id: format!("sr-ckpt-{suffix}"),
                actor: actor.clone(),
                lane_id: lane_id.clone(),
                lease_id: lease_for_ckpt.lease_id.clone(),
                scope_ref: format!("document:{scope_id}"),
                resume_pointer: SwarmResumePointerV1::MicroTask {
                    mt_id: "MT-079".to_string(),
                },
                checkpoint_payload: json!({"k": "v"}),
            },
        )
        .await
        .expect("checkpoint written");

        async fn receipt_count(
            pool: &handshake_core::storage::surreal::SurrealStorage,
            checkpoint_id: &str,
        ) -> usize {
            list_recovery_receipts_for_checkpoint(pool, checkpoint_id)
                .await
                .expect("receipts")
                .len()
        }

        // (a) RELEASED lease.
        let released = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &actor,
                &format!("sr-rel-{suffix}"),
                KnowledgeLeaseScopeKind::Document,
                &scope_id,
                600,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => *lease,
            // scope already held by lease_for_ckpt -> claim a distinct scope.
            LeaseClaimOutcomeV1::ScopeHeld { .. } => {
                match claim_lease(
                    db.as_ref(),
                    &pool,
                    claim_request(
                        &actor,
                        &format!("sr-rel-{suffix}"),
                        KnowledgeLeaseScopeKind::Document,
                        &format!("{scope_id}-rel"),
                        600,
                    ),
                )
                .await
                .expect("claim flow")
                {
                    LeaseClaimOutcomeV1::Claimed(lease) => *lease,
                    other => panic!("expected claim, got {other:?}"),
                }
            }
        };
        release_lease(db.as_ref(), &pool, &released.lease_id, &actor)
            .await
            .expect("release");
        let out = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            &checkpoint.checkpoint_id,
            &format!("sr-rec-{suffix}"),
            &actor,
            &released.lease_id,
        )
        .await
        .expect("recovery flow");
        assert!(
            matches!(out, Err(RecoveryFailureV1::LeaseReleased { .. })),
            "released lease must refuse recovery, got {out:?}"
        );
        assert_eq!(receipt_count(&pool, &checkpoint.checkpoint_id).await, 0);

        // (b) EXPIRED lease (1s ttl, waited out on the DB clock).
        let expiring = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &actor,
                &format!("sr-exp-{suffix}"),
                KnowledgeLeaseScopeKind::Document,
                &format!("{scope_id}-exp"),
                1,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => *lease,
            other => panic!("expected claim, got {other:?}"),
        };
        wait_for_db_expiry(&pool, &expiring.lease_id).await;
        let out = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            &checkpoint.checkpoint_id,
            &format!("sr-rec-{suffix}"),
            &actor,
            &expiring.lease_id,
        )
        .await
        .expect("recovery flow");
        assert!(
            matches!(out, Err(RecoveryFailureV1::LeaseExpired { .. })),
            "expired lease must refuse recovery, got {out:?}"
        );
        assert_eq!(receipt_count(&pool, &checkpoint.checkpoint_id).await, 0);

        // (c) FOREIGN actor: a live lease owned by `actor`, but recovery
        // claims `stranger`.
        let owned = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &actor,
                &format!("sr-own-{suffix}"),
                KnowledgeLeaseScopeKind::Document,
                &scope_id,
                600,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => *lease,
            // The original checkpoint lease already holds `document:scope_id`;
            // reuse it (it is live + owned by `actor`).
            LeaseClaimOutcomeV1::ScopeHeld { .. } => lease_for_ckpt.clone(),
        };
        let out = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            &checkpoint.checkpoint_id,
            &format!("sr-rec-{suffix}"),
            &stranger,
            &owned.lease_id,
        )
        .await
        .expect("recovery flow");
        assert!(
            matches!(out, Err(RecoveryFailureV1::LeaseForeignActor { .. })),
            "foreign-actor lease must refuse recovery, got {out:?}"
        );
        assert_eq!(receipt_count(&pool, &checkpoint.checkpoint_id).await, 0);

        // (d) WRONG scope: a live, owned lease on an unrelated document scope.
        let wrong_scope = match claim_lease(
            db.as_ref(),
            &pool,
            claim_request(
                &actor,
                &format!("sr-ws-{suffix}"),
                KnowledgeLeaseScopeKind::Document,
                &format!("unrelated-{suffix}"),
                600,
            ),
        )
        .await
        .expect("claim flow")
        {
            LeaseClaimOutcomeV1::Claimed(lease) => *lease,
            other => panic!("expected claim, got {other:?}"),
        };
        let out = recover_from_checkpoint(
            db.as_ref(),
            &pool,
            &checkpoint.checkpoint_id,
            &format!("sr-rec-{suffix}"),
            &actor,
            &wrong_scope.lease_id,
        )
        .await
        .expect("recovery flow");
        assert!(
            matches!(out, Err(RecoveryFailureV1::LeaseScopeMismatch { .. })),
            "wrong-scope lease must refuse recovery, got {out:?}"
        );
        assert_eq!(receipt_count(&pool, &checkpoint.checkpoint_id).await, 0);
    }
}
