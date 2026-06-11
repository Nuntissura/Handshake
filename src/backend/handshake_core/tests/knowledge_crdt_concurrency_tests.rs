//! WP-KERNEL-009 CRDTAndConcurrencyCore concurrency tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_070_save_semantics: MT-070 ConcurrentEditorSaveSemantics
//!   - mt_071_index_run_guard: MT-071 ConcurrentIndexRunSemantics
//!   - mt_073_offline_boundary: MT-073 OfflineDraftStateBoundary
//!
//! All durable assertions run against real PostgreSQL (POSTGRES_TEST_URL,
//! isolated schema, full migration chain incl. 0150/0151).

use base64::Engine;
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::persistence::sha256_hex;
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1, YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::storage::tests::{postgres_backend_with_pool_from_env, PostgresTestBackend};
use handshake_core::storage::StorageError;

async fn backend_or_blocked() -> PostgresTestBackend {
    match postgres_backend_with_pool_from_env().await {
        Ok(backend) => backend,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: WP-009 CRDT concurrency tests require POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

/// Build a structurally valid Yjs update envelope for tests. Bytes are real
/// opaque payloads (the backend treats Yjs updates as opaque by contract).
#[allow(clippy::too_many_arguments)]
fn envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    bytes: &[u8],
    before: &KnowledgeStateVectorV1,
    after: &KnowledgeStateVectorV1,
) -> YjsUpdateEnvelopeV1 {
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, actor);
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        update_id: update_id.to_string(),
        actor_id: actor.canonical(),
        site_id: site.site_id,
        session_id: session_id.to_string(),
        trace_id: format!("trace-{update_id}"),
        document_schema_id: "hsk.doc.rich_document@1".to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
        update_sha256: sha256_hex(bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
}

mod mt_070_save_semantics {
    use super::*;
    use handshake_core::kernel::crdt::save_semantics::{
        decide_concurrent_save, save_rich_document_draft, KnowledgeDraftSaveOutcomeV1,
        KnowledgeSaveDecisionV1,
    };
    use handshake_core::kernel::crdt::yjs_bridge::read_draft_head;
    use handshake_core::kernel::KernelEventType;
    use handshake_core::storage::knowledge_crdt::list_denial_receipts_for_document;
    use uuid::Uuid;

    #[test]
    fn decision_matrix_is_deterministic() {
        let sv = |s: &str| KnowledgeStateVectorV1::parse(s).expect("valid sv");
        // base == head
        assert_eq!(
            decide_concurrent_save(&sv("hsk-sv1:a=1"), &sv("hsk-sv1:a=1")),
            KnowledgeSaveDecisionV1::FastForward
        );
        // head dominates base -> stale write
        assert!(matches!(
            decide_concurrent_save(&sv("hsk-sv1:a=2"), &sv("hsk-sv1:a=1")),
            KnowledgeSaveDecisionV1::StaleWrite { .. }
        ));
        // base dominates head -> ahead of head
        assert!(matches!(
            decide_concurrent_save(&sv("hsk-sv1:a=1"), &sv("hsk-sv1:a=2")),
            KnowledgeSaveDecisionV1::AheadOfHead { .. }
        ));
        // concurrent fork
        assert!(matches!(
            decide_concurrent_save(&sv("hsk-sv1:a=2;b=1"), &sv("hsk-sv1:a=1;b=2")),
            KnowledgeSaveDecisionV1::ConcurrentFork { .. }
        ));
    }

    /// Two simulated actors (operator + local model) saving concurrently:
    /// the stale writer gets a typed conflict with a durable receipt and a
    /// KNOWLEDGE_CRDT_CONFLICT_DETECTED event; the draft log is never
    /// silently overwritten; the rebased resubmission is accepted.
    #[tokio::test]
    async fn concurrent_saves_yield_typed_conflicts_and_durable_receipts() {
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt070-{suffix}");
        let doc = format!("doc-mt070-{suffix}");
        let crdt_doc = format!("crdt-mt070-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-a").expect("actor");
        let model = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "lm-b").expect("actor");
        let op_site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);
        let lm_site = derive_knowledge_site_id(&ws, &crdt_doc, &model);

        // Operator lands the first save.
        let empty = KnowledgeStateVectorV1::new();
        let mut after_a = empty.clone();
        after_a.increment(&op_site.site_id);
        let save_a = save_rich_document_draft(
            db.as_ref(),
            &pool,
            &envelope(
                &ws,
                &doc,
                &crdt_doc,
                "u-a1",
                &operator,
                "sr-op",
                b"op-edit-1",
                &empty,
                &after_a,
            ),
        )
        .await
        .expect("save flow runs");
        assert!(matches!(
            save_a,
            KnowledgeDraftSaveOutcomeV1::Accepted { update_seq: 1, .. }
        ));

        // Model saves concurrently from the SAME empty base -> stale write.
        let mut after_b = empty.clone();
        after_b.increment(&lm_site.site_id);
        let stale_envelope = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "u-b1",
            &model,
            "sr-lm",
            b"lm-edit-1",
            &empty,
            &after_b,
        );
        let save_b = save_rich_document_draft(db.as_ref(), &pool, &stale_envelope)
            .await
            .expect("save flow runs");
        let (decision, receipt_id, conflict_event_id) = match save_b {
            KnowledgeDraftSaveOutcomeV1::Conflict {
                decision,
                denial_receipt_id,
                conflict_event_id,
                head_update_seq,
                ..
            } => {
                assert_eq!(head_update_seq, 1);
                (decision, denial_receipt_id, conflict_event_id)
            }
            other => panic!("stale save must conflict, got {other:?}"),
        };
        assert!(matches!(
            decision,
            KnowledgeSaveDecisionV1::StaleWrite { .. }
        ));

        // Durable receipt exists and references the EventLedger event.
        let receipts = list_denial_receipts_for_document(&pool, &crdt_doc)
            .await
            .expect("list receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].receipt_id, receipt_id);
        assert_eq!(receipts[0].receipt_kind, "stale_draft_save");
        assert_eq!(receipts[0].event_ledger_event_id, conflict_event_id);
        assert_eq!(receipts[0].actor_id, model.canonical());

        // The denied save did NOT mutate the draft log (no silent overwrite).
        let head = read_draft_head(db.as_ref(), &ws, &doc, &crdt_doc)
            .await
            .expect("head");
        assert_eq!(head.head_update_seq, 1);
        assert_eq!(head.head_state_vector, after_a.encode());

        // Conflict event is in the ledger.
        let events = db
            .list_kernel_events_for_aggregate("knowledge_crdt_document", &crdt_doc)
            .await
            .expect("events");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtConflictDetected));

        // Model pulls/merges (client-side) and resubmits rebased -> accepted.
        let merged_after = after_a.merge(&after_b);
        let rebased = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "u-b1-rebased",
            &model,
            "sr-lm",
            b"lm-edit-1-rebased",
            &after_a,
            &merged_after,
        );
        let save_b2 = save_rich_document_draft(db.as_ref(), &pool, &rebased)
            .await
            .expect("save flow runs");
        assert!(matches!(
            save_b2,
            KnowledgeDraftSaveOutcomeV1::Accepted { update_seq: 2, .. }
        ));

        // Concurrent fork: operator advances head while model bases on a
        // privately-advanced vector -> ConcurrentFork receipt.
        let head_now = merged_after.clone();
        let mut op_next = head_now.clone();
        op_next.increment(&op_site.site_id);
        let save_a2 = save_rich_document_draft(
            db.as_ref(),
            &pool,
            &envelope(
                &ws,
                &doc,
                &crdt_doc,
                "u-a2",
                &operator,
                "sr-op",
                b"op-edit-2",
                &head_now,
                &op_next,
            ),
        )
        .await
        .expect("save flow runs");
        assert!(matches!(
            save_a2,
            KnowledgeDraftSaveOutcomeV1::Accepted { .. }
        ));

        let mut forked_base = head_now.clone();
        forked_base.increment(&lm_site.site_id); // model-side private progress
        let mut forked_after = forked_base.clone();
        forked_after.increment(&lm_site.site_id);
        let fork_save = save_rich_document_draft(
            db.as_ref(),
            &pool,
            &envelope(
                &ws,
                &doc,
                &crdt_doc,
                "u-b2",
                &model,
                "sr-lm",
                b"lm-edit-2",
                &forked_base,
                &forked_after,
            ),
        )
        .await
        .expect("save flow runs");
        match fork_save {
            KnowledgeDraftSaveOutcomeV1::Conflict { decision, .. } => {
                assert!(matches!(
                    decision,
                    KnowledgeSaveDecisionV1::ConcurrentFork { .. }
                ));
            }
            other => panic!("forked save must conflict, got {other:?}"),
        }
        let receipts = list_denial_receipts_for_document(&pool, &crdt_doc)
            .await
            .expect("list receipts");
        assert_eq!(receipts.len(), 2);
        assert!(receipts
            .iter()
            .any(|receipt| receipt.receipt_kind == "concurrent_draft_fork"));

        // Identical resubmission of an accepted update is idempotent.
        let replay = save_rich_document_draft(db.as_ref(), &pool, &rebased)
            .await
            .expect("save flow runs");
        assert!(matches!(
            replay,
            KnowledgeDraftSaveOutcomeV1::AlreadyApplied { update_seq: 2, .. }
        ));
    }
}
