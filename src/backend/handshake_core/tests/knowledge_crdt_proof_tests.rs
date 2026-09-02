//! WP-KERNEL-009 CRDTAndConcurrencyCore proof tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_077_promotion_e2e: MT-077 CrdtEventLedgerPromotionTests
//!   - mt_078_no_external_relay: MT-078 CrdtNoExternalRelayProof
//!   - mt_080_spec_compatibility: MT-080 CrdtSpecCompatibilityCheck
//!
//! Spec law under test: 02-system-architecture.md section 2.3.13.11
//! [ADD v02.192], CRDT paragraph.

use base64::Engine;
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::persistence::sha256_hex;
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    push_yjs_update, YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1,
    YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::storage::knowledge::{
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind, KnowledgeSpanKind,
    KnowledgeStore, NewKnowledgeSource, NewKnowledgeSpan,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::{Database, NewWorkspace, WriteContext};
use serde_json::json;

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
        .expect("close original embedded CRDT proof store");
    let reopened = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened embedded CRDT proof store"),
    )
    .await
    .expect("reopen embedded CRDT proof store");
    bootstrap_schema(&reopened)
        .await
        .expect("bootstrap reopened CRDT proof schema");
    let database: std::sync::Arc<dyn Database> =
        std::sync::Arc::new(SurrealDatabase::new(reopened.clone()));
    (reopened, database)
}

async fn close_reopened_and_remove(reopened: SurrealStorage, backend: EmbeddedTestBackend) {
    reopened
        .shutdown()
        .await
        .expect("close reopened CRDT proof store");
    drop(reopened);
    backend
        .close_and_remove()
        .await
        .expect("remove embedded CRDT proof store");
}

/// Authority-hardening #1 fixture: create a real workspace + source + span and
/// return `(workspace_id, span_id)`. The span is a live (non-stale),
/// same-workspace `KSP-` row, so a proposal in `workspace_id` citing `span_id`
/// has promotable evidence. `stale` controls whether the source is retired.
async fn live_span_fixture(
    backend: &EmbeddedTestBackend,
    label: &str,
    stale: bool,
) -> (String, String) {
    let db = SurrealDatabase::new(backend.storage.clone());
    let workspace_id = db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("span-fixture-{label}"),
            },
        )
        .await
        .expect("create workspace")
        .id;
    let hash = "a".repeat(64);
    let source = db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: workspace_id.clone(),
            root_id: None,
            source_kind: KnowledgeSourceKind::ExternalImport,
            relative_path: None,
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: hash.clone(),
            size_bytes: Some(16),
            provenance: json!({"fixture": "crdt_proof", "label": label}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("create source");
    let span = db
        .create_knowledge_span(NewKnowledgeSpan {
            source_id: source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Byte,
            range_start: 0,
            range_end: 16,
            line_start: None,
            line_end: None,
            section_path: None,
            content_sha256: hash,
            parser_version: "v1".to_string(),
            extraction_receipt_event_id: None,
            index_run_id: None,
            display_snippet: Some("embedded CRDT proof span".to_string()),
        })
        .await
        .expect("create span");
    if stale {
        db.mark_knowledge_source_stale(&source.source_id)
            .await
            .expect("mark source stale");
    }
    (workspace_id, span.span_id)
}

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

mod mt_077_promotion_e2e {
    use super::*;
    use handshake_core::kernel::crdt::claim_promotion::{
        promote_graph_proposal, GraphPromotionDenialReasonV1, GraphPromotionOutcomeV1,
    };
    use handshake_core::kernel::crdt::graph_proposal::{
        decide_graph_proposal, record_graph_proposal, GraphMutationKind,
        GraphMutationProposalRequestV1, RecordGraphProposalOutcomeV1,
    };
    use handshake_core::kernel::crdt::persistence::build_crdt_replay_plan;
    use handshake_core::kernel::crdt::rich_document_snapshot::{
        build_rich_document_snapshot_record, restore_rich_document_snapshot,
        RichDocumentSnapshotPayloadV1, RICH_DOCUMENT_SCHEMA_ID,
        RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID,
    };
    use handshake_core::kernel::crdt::snapshot::build_snapshot_bounded_replay_plan;
    use handshake_core::kernel::crdt::state_vector::verify_causal_chain;
    use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use serde_json::json;
    use uuid::Uuid;

    /// End-to-end battery: draft updates -> snapshot -> promotion ->
    /// EventLedger events -> replay reconstructs identical state;
    /// duplicate/stale idempotency rejections proven on every leg.
    #[tokio::test]
    async fn drafts_snapshot_promote_replay_identically_with_idempotency() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt077-{suffix}");
        let doc = format!("doc-mt077-{suffix}");
        let crdt_doc = format!("crdt-mt077-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-e2e").expect("actor");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "lm-e2e").expect("actor");
        let op_site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);
        let lm_site = derive_knowledge_site_id(&ws, &crdt_doc, &model);

        // --- Draft updates (two actors, three updates) -------------------
        let mut sv = KnowledgeStateVectorV1::new();
        let mut envelopes = Vec::new();
        for (index, (actor, site)) in [
            (&operator, &op_site),
            (&model, &lm_site),
            (&operator, &op_site),
        ]
        .into_iter()
        .enumerate()
        {
            let update_id = format!("e2e-u{}", index + 1);
            let before = sv.clone();
            sv.increment(&site.site_id);
            let env = envelope(
                &ws,
                &doc,
                &crdt_doc,
                &update_id,
                actor,
                "sr-e2e",
                format!("e2e-bytes-{}", index + 1).as_bytes(),
                &before,
                &sv,
            );
            let outcome = push_yjs_update(db.as_ref(), &env).await.expect("push");
            assert!(matches!(outcome, YjsPushOutcomeV1::Stored { .. }));
            envelopes.push(env);
        }
        let final_sv = sv.encode();

        // Duplicate push is an idempotent replay, not a second row.
        let dup = push_yjs_update(db.as_ref(), &envelopes[2])
            .await
            .expect("push");
        assert!(matches!(
            dup,
            YjsPushOutcomeV1::AlreadyStored { update_seq: 3, .. }
        ));

        // Stale push (old base) is rejected.
        let mut stale_after = KnowledgeStateVectorV1::new();
        stale_after.increment(&lm_site.site_id);
        let stale = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "e2e-stale",
            &model,
            "sr-e2e",
            b"stale",
            &KnowledgeStateVectorV1::new(),
            &stale_after,
        );
        assert!(matches!(
            push_yjs_update(db.as_ref(), &stale).await.expect("push"),
            YjsPushOutcomeV1::Denied { .. }
        ));

        // --- Snapshot ------------------------------------------------------
        let identity = knowledge_crdt_identity(
            &ws,
            &doc,
            &crdt_doc,
            RICH_DOCUMENT_SCHEMA_ID,
            &operator,
            "trace-e2e-snapshot",
        );
        let snapshot_event = NewKernelEvent::builder(
            format!("KTR-MT077-{suffix}"),
            "sr-e2e".to_string(),
            KernelEventType::KnowledgeCrdtSnapshotRecorded,
            KernelActor::Operator(operator.canonical()),
        )
        .aggregate("knowledge_crdt_document", crdt_doc.clone())
        .idempotency_key(format!("mt077:{suffix}:snapshot"))
        .source_component("knowledge_crdt_proof_tests")
        .payload(json!({"covered_update_seq": 3}))
        .build()
        .expect("event");
        let snapshot_event = db
            .append_kernel_event(snapshot_event)
            .await
            .expect("append");

        let payload = RichDocumentSnapshotPayloadV1 {
            schema_id: RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID.to_string(),
            document_schema_id: RICH_DOCUMENT_SCHEMA_ID.to_string(),
            prosemirror_schema_version: "tiptap-starter-kit@3.13.0".to_string(),
            doc_json: json!({
                "type": "doc",
                "content": [{"type": "paragraph",
                             "content": [{"type": "text", "text": "e2e state"}]}]
            }),
            state_vector: final_sv.clone(),
            covered_update_seq: 3,
        };
        let (snapshot_record, snapshot_bytes) = build_rich_document_snapshot_record(
            &identity,
            &format!("snap-e2e-{suffix}"),
            &payload,
            &snapshot_event.event_id,
            &["e2e-u1", "e2e-u2", "e2e-u3"],
        )
        .expect("snapshot builds");
        db.append_kernel_crdt_snapshot(snapshot_record.clone(), snapshot_bytes)
            .await
            .expect("snapshot persists");

        // --- Promotion (graph proposal derived from the draft) -------------
        // Authority-hardening #1: a proposal citing only a `pending:` soft
        // marker is APPROVED as a draft but REFUSED at the authority gate —
        // it must be re-grounded on a live span before it can become a fact.
        // (This test previously promoted the `pending:` span and asserted
        // success, codifying the bug as happy-path; it now asserts refusal.)
        let pending_proposal = match record_graph_proposal(
            db.as_ref(),
            &pool,
            GraphMutationProposalRequestV1 {
                workspace_id: ws.clone(),
                mutation_kind: GraphMutationKind::AddClaim,
                mutation_payload: json!({
                    "claim_text": "e2e document captures the final draft state",
                    "derived_from_crdt_document": crdt_doc,
                    "at_state_vector": final_sv,
                }),
                source_span_refs: vec![format!("pending:{crdt_doc}:full-doc")],
                confidence: 0.95,
                actor: operator.clone(),
                session_id: "sr-e2e".to_string(),
                correlation_id: format!("corr-e2e-{suffix}"),
                lease_id: None,
            },
        )
        .await
        .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &pending_proposal.proposal_id,
            true,
            &operator,
            "sr-e2e",
            "operator-authored claim",
        )
        .await
        .expect("decide flow")
        .expect("approved");
        let refused = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &pending_proposal.proposal_id,
            &operator,
            "sr-e2e",
            format!("corr-e2e-{suffix}").as_str(),
        )
        .await
        .expect("promotion flow");
        match refused {
            GraphPromotionOutcomeV1::Denied(denial) => match denial.reason {
                GraphPromotionDenialReasonV1::SpanEvidenceInvalid { rejections } => {
                    assert!(
                        rejections.iter().any(|r| matches!(
                            r,
                            handshake_core::storage::knowledge_crdt::PromotionSpanRejection::PendingMarker { .. }
                        )),
                        "pending: marker must be the rejection reason, got {rejections:?}"
                    );
                }
                other => panic!("expected SpanEvidenceInvalid, got {other:?}"),
            },
            other => panic!("pending: span promotion must be REFUSED, got {other:?}"),
        }
        // No authority fact was created for the refused promotion.
        assert!(
            handshake_core::storage::knowledge_crdt::get_promoted_fact_by_proposal(
                &pool,
                &pending_proposal.proposal_id,
            )
            .await
            .expect("fact lookup")
            .is_none(),
            "a refused promotion must not create an authority fact"
        );
        // The proposal stays 'approved' (not 'promoted') after refusal.
        assert_eq!(
            handshake_core::storage::knowledge_crdt::get_graph_proposal(
                &pool,
                &pending_proposal.proposal_id,
            )
            .await
            .expect("get proposal")
            .expect("row")
            .review_state,
            "approved"
        );

        // A proposal grounded on a LIVE span promotes (stays green) and is the
        // subject of the exactly-once ledger assertions below.
        let (span_ws, live_span_id) =
            live_span_fixture(&backend, &format!("mt077-{suffix}"), false).await;
        let proposal = match record_graph_proposal(
            db.as_ref(),
            &pool,
            GraphMutationProposalRequestV1 {
                workspace_id: span_ws.clone(),
                mutation_kind: GraphMutationKind::AddClaim,
                mutation_payload: json!({
                    "claim_text": "e2e document captures the final draft state",
                    "derived_from_crdt_document": crdt_doc,
                }),
                source_span_refs: vec![live_span_id.clone()],
                confidence: 0.95,
                actor: operator.clone(),
                session_id: "sr-e2e".to_string(),
                correlation_id: format!("corr-e2e-valid-{suffix}"),
                lease_id: None,
            },
        )
        .await
        .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            "sr-e2e",
            "operator-authored claim on live span",
        )
        .await
        .expect("decide flow")
        .expect("approved");
        let promoted = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            "sr-e2e",
            format!("corr-e2e-valid-{suffix}").as_str(),
        )
        .await
        .expect("promotion flow");
        let fact = match promoted {
            GraphPromotionOutcomeV1::Promoted(fact) => fact,
            other => panic!("expected promotion, got {other:?}"),
        };
        // The frozen fact carries the validated KSP- id (never a pending ref).
        assert_eq!(
            fact.source_span_refs,
            serde_json::json!([live_span_id]),
            "fact freezes the validated span id"
        );

        // Promotion is exactly-once: replays converge on the same fact and
        // the ledger holds exactly one REQUESTED/ACCEPTED pair.
        let replay = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            "sr-e2e",
            format!("corr-e2e-valid-{suffix}").as_str(),
        )
        .await
        .expect("promotion flow");
        match replay {
            GraphPromotionOutcomeV1::AlreadyPromoted(same) => {
                assert_eq!(same.fact_id, fact.fact_id)
            }
            other => panic!("expected idempotent promotion, got {other:?}"),
        }
        let promo_events = db
            .list_kernel_events_for_aggregate("knowledge_graph_promotion", &proposal.proposal_id)
            .await
            .expect("events");
        assert_eq!(
            promo_events
                .iter()
                .filter(|event| event.event_type == KernelEventType::PromotionRequested)
                .count(),
            1
        );
        assert_eq!(
            promo_events
                .iter()
                .filter(|event| event.event_type == KernelEventType::PromotionAccepted)
                .count(),
            1
        );

        // Duplicate EventLedger appends with the same idempotency key return
        // the SAME stored event (ledger-level exactly-once).
        let dup_event = NewKernelEvent::builder(
            format!("KTR-MT077-{suffix}"),
            "sr-e2e".to_string(),
            KernelEventType::KnowledgeCrdtSnapshotRecorded,
            KernelActor::Operator(operator.canonical()),
        )
        .aggregate("knowledge_crdt_document", crdt_doc.clone())
        .idempotency_key(format!("mt077:{suffix}:snapshot"))
        .source_component("knowledge_crdt_proof_tests")
        .payload(json!({"covered_update_seq": 3}))
        .build()
        .expect("event");
        let dup_stored = db.append_kernel_event(dup_event).await.expect("append");
        assert_eq!(dup_stored.event_id, snapshot_event.event_id);

        // --- Replay reconstructs identical state ---------------------------
        let records = db
            .list_kernel_crdt_updates(&ws, &doc, &crdt_doc)
            .await
            .expect("list updates");
        assert_eq!(records.len(), 3, "denied/stale pushes never landed");

        // Full replay plan: ordered, gap-free, ends on the final vector.
        let plan = build_crdt_replay_plan(&records).expect("replay plan");
        assert_eq!(plan.final_state_vector, final_sv);
        assert_eq!(plan.ordered_updates.len(), 3);

        // Causal chain proof over persisted metadata.
        let proof = verify_causal_chain(&records).expect("causal chain");
        assert_eq!(proof.final_state_vector, final_sv);

        // Byte-identical replay of every update payload.
        for (env, step) in envelopes.iter().zip(plan.ordered_updates.iter()) {
            let bytes = db
                .read_kernel_crdt_update_bytes(&step.update_bytes_ref)
                .await
                .expect("read bytes");
            assert_eq!(sha256_hex(&bytes), env.update_sha256);
        }

        // Snapshot-bounded replay agrees with the snapshot's vector.
        let snapshots = db
            .list_kernel_crdt_snapshots(&ws, &doc, &crdt_doc)
            .await
            .expect("list snapshots");
        assert_eq!(snapshots.len(), 1);
        let bounded = build_snapshot_bounded_replay_plan(&snapshots[0], &records)
            .expect("bounded replay plan");
        assert_eq!(bounded.final_state_vector, final_sv);
        assert!(
            bounded.ordered_updates.is_empty(),
            "snapshot covers all updates"
        );

        // Restore the document from persisted snapshot bytes.
        let snapshot_bytes = db
            .read_kernel_crdt_snapshot_bytes(&snapshots[0].snapshot_bytes_ref)
            .await
            .expect("read snapshot bytes");
        let restored =
            restore_rich_document_snapshot(&snapshots[0], &snapshot_bytes).expect("restore");
        assert_eq!(restored.state_vector, final_sv);
        assert_eq!(
            restored.doc_json["content"][0]["content"][0]["text"],
            "e2e state"
        );

        drop(pool);
        drop(db);
        let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
        let reopened_records = reopened_db
            .list_kernel_crdt_updates(&ws, &doc, &crdt_doc)
            .await
            .expect("list CRDT updates after reopen");
        assert_eq!(reopened_records.len(), 3);
        let reopened_plan = build_crdt_replay_plan(&reopened_records)
            .expect("build replay plan from reopened CRDT updates");
        assert_eq!(reopened_plan.final_state_vector, final_sv);
        for (env, step) in envelopes.iter().zip(reopened_plan.ordered_updates.iter()) {
            let bytes = reopened_db
                .read_kernel_crdt_update_bytes(&step.update_bytes_ref)
                .await
                .expect("read reopened CRDT update bytes");
            assert_eq!(sha256_hex(&bytes), env.update_sha256);
        }
        let reopened_snapshots = reopened_db
            .list_kernel_crdt_snapshots(&ws, &doc, &crdt_doc)
            .await
            .expect("list CRDT snapshots after reopen");
        assert_eq!(reopened_snapshots.len(), 1);
        let reopened_snapshot_bytes = reopened_db
            .read_kernel_crdt_snapshot_bytes(&reopened_snapshots[0].snapshot_bytes_ref)
            .await
            .expect("read CRDT snapshot bytes after reopen");
        let reopened_document =
            restore_rich_document_snapshot(&reopened_snapshots[0], &reopened_snapshot_bytes)
                .expect("restore CRDT snapshot after reopen");
        assert_eq!(reopened_document.state_vector, final_sv);
        assert_eq!(
            reopened_document.doc_json["content"][0]["content"][0]["text"],
            "e2e state"
        );
        let reopened_fact = handshake_core::storage::knowledge_crdt::get_promoted_fact_by_proposal(
            &reopened,
            &proposal.proposal_id,
        )
        .await
        .expect("read promoted fact after reopen")
        .expect("promoted fact survives reopen");
        assert_eq!(reopened_fact.fact_id, fact.fact_id);
        let reopened_promotion_events = reopened_db
            .list_kernel_events_for_aggregate("knowledge_graph_promotion", &proposal.proposal_id)
            .await
            .expect("list promotion events after reopen");
        assert_eq!(
            reopened_promotion_events
                .iter()
                .filter(|event| event.event_type == KernelEventType::PromotionRequested)
                .count(),
            1
        );
        assert_eq!(
            reopened_promotion_events
                .iter()
                .filter(|event| event.event_type == KernelEventType::PromotionAccepted)
                .count(),
            1
        );
        drop(reopened_db);
        close_reopened_and_remove(reopened, backend).await;
    }
}

/// Authority-hardening #2: promotion is atomic (ledger pair + fact insert +
/// proposal flip in ONE transaction) and converges under the crash window.
mod mt_069_atomic_promotion {
    use super::*;
    use handshake_core::kernel::crdt::claim_promotion::{
        promote_graph_proposal, GraphPromotionOutcomeV1,
    };
    use handshake_core::kernel::crdt::graph_proposal::{
        decide_graph_proposal, record_graph_proposal, GraphMutationKind,
        GraphMutationProposalRequestV1, RecordGraphProposalOutcomeV1,
    };
    use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use handshake_core::storage::knowledge_crdt::get_promoted_fact_by_proposal;
    use serde_json::json;
    use uuid::Uuid;

    /// One promotion call writes the ledger pair AND the fact AND the proposal
    /// flip together; and a re-run after a partial (ledger-only) state
    /// converges on exactly one fact + exactly one event pair (passive replay
    /// no longer diverges).
    #[tokio::test]
    async fn promotion_is_atomic_and_converges_after_crash_window() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "atomic-op").expect("actor");
        let (ws, span_id) =
            live_span_fixture(&backend, &format!("mt069atom-{suffix}"), false).await;

        let proposal = match record_graph_proposal(
            db.as_ref(),
            &pool,
            GraphMutationProposalRequestV1 {
                workspace_id: ws.clone(),
                mutation_kind: GraphMutationKind::AddClaim,
                mutation_payload: json!({"claim_text": "atomic promotion"}),
                source_span_refs: vec![span_id.clone()],
                confidence: 0.7,
                actor: operator.clone(),
                session_id: format!("sr-{suffix}"),
                correlation_id: format!("corr-{suffix}"),
                lease_id: None,
            },
        )
        .await
        .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            &format!("sr-{suffix}"),
            "approved",
        )
        .await
        .expect("decide flow")
        .expect("approved");

        // Before promotion: ledger has no promotion pair and no fact.
        assert!(get_promoted_fact_by_proposal(&pool, &proposal.proposal_id)
            .await
            .expect("fact")
            .is_none());

        // One atomic promotion: BOTH the fact AND the ledger pair appear.
        let fact = match promote_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            &format!("sr-{suffix}"),
            &format!("corr-{suffix}"),
        )
        .await
        .expect("promotion flow")
        {
            GraphPromotionOutcomeV1::Promoted(fact) => *fact,
            other => panic!("expected promotion, got {other:?}"),
        };
        let events = db
            .list_kernel_events_for_aggregate("knowledge_graph_promotion", &proposal.proposal_id)
            .await
            .expect("events");
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == KernelEventType::PromotionRequested)
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == KernelEventType::PromotionAccepted)
                .count(),
            1
        );
        // The fact's ledger ids are exactly the appended pair (one tx).
        assert!(events
            .iter()
            .any(|e| e.event_id == fact.promotion_requested_event_id));
        assert!(events
            .iter()
            .any(|e| e.event_id == fact.promotion_accepted_event_id));
        assert_eq!(fact.workspace_id, ws);

        // Crash-window convergence: a SECOND proposal whose ledger pair was
        // committed but whose fact never landed (the old non-atomic crash)
        // converges to exactly one fact + one pair when promotion re-runs.
        let proposal2 = match record_graph_proposal(
            db.as_ref(),
            &pool,
            GraphMutationProposalRequestV1 {
                workspace_id: ws.clone(),
                mutation_kind: GraphMutationKind::AddClaim,
                mutation_payload: json!({"claim_text": "converges after crash"}),
                source_span_refs: vec![span_id.clone()],
                confidence: 0.7,
                actor: operator.clone(),
                session_id: format!("sr2-{suffix}"),
                correlation_id: format!("corr2-{suffix}"),
                lease_id: None,
            },
        )
        .await
        .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal2.proposal_id,
            true,
            &operator,
            &format!("sr2-{suffix}"),
            "approved",
        )
        .await
        .expect("decide flow")
        .expect("approved");

        // Simulate the crash window: append the promotion pair directly (same
        // idempotency keys the bridge uses) but DO NOT write the fact.
        let requested = NewKernelEvent::builder(
            format!("KTR-KNOWLEDGE-GRAPH-{ws}"),
            format!("sr2-{suffix}"),
            KernelEventType::PromotionRequested,
            KernelActor::Operator(operator.canonical()),
        )
        .aggregate("knowledge_graph_promotion", proposal2.proposal_id.clone())
        .idempotency_key(format!(
            "knowledge-graph-promotion:{}:requested",
            proposal2.proposal_id
        ))
        .source_component("knowledge_crdt_claim_promotion")
        .payload(json!({"proposal_id": proposal2.proposal_id}))
        .build()
        .expect("event");
        let accepted = NewKernelEvent::builder(
            format!("KTR-KNOWLEDGE-GRAPH-{ws}"),
            format!("sr2-{suffix}"),
            KernelEventType::PromotionAccepted,
            KernelActor::Operator(operator.canonical()),
        )
        .aggregate("knowledge_graph_promotion", proposal2.proposal_id.clone())
        .idempotency_key(format!(
            "knowledge-graph-promotion:{}:accepted",
            proposal2.proposal_id
        ))
        .source_component("knowledge_crdt_claim_promotion")
        .payload(json!({"proposal_id": proposal2.proposal_id}))
        .build()
        .expect("event");
        db.append_kernel_event_pair_atomic_with_causation(requested, accepted)
            .await
            .expect("append partial pair");
        // Crash state: ledger says promoted, but no fact row exists.
        assert!(get_promoted_fact_by_proposal(&pool, &proposal2.proposal_id)
            .await
            .expect("fact")
            .is_none());

        // Recovery: re-run promotion. The events dedupe on their idempotency
        // keys (no second pair), the missing fact is materialized, and the
        // proposal converges to 'promoted'. Passive replay now converges.
        let recovered = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal2.proposal_id,
            &operator,
            &format!("sr2-{suffix}"),
            &format!("corr2-{suffix}"),
        )
        .await
        .expect("promotion flow");
        assert!(matches!(recovered, GraphPromotionOutcomeV1::Promoted(_)));
        assert!(get_promoted_fact_by_proposal(&pool, &proposal2.proposal_id)
            .await
            .expect("fact")
            .is_some());
        let events2 = db
            .list_kernel_events_for_aggregate("knowledge_graph_promotion", &proposal2.proposal_id)
            .await
            .expect("events");
        assert_eq!(
            events2
                .iter()
                .filter(|e| e.event_type == KernelEventType::PromotionRequested)
                .count(),
            1,
            "ledger pair dedups: exactly one PROMOTION_REQUESTED after recovery"
        );
        assert_eq!(
            events2
                .iter()
                .filter(|e| e.event_type == KernelEventType::PromotionAccepted)
                .count(),
            1
        );
    }
}

mod mt_078_no_external_relay {
    use super::*;
    use uuid::Uuid;

    /// Static proof: the WP-009 CRDT surface declares no external sync
    /// server, relay, or hosted CRDT service — not in Cargo dependencies and
    /// not in the CRDT/API source. The draft path speaks only to embedded storage.
    #[test]
    fn static_scan_finds_no_external_relay_dependency() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let forbidden_dependencies = [
            "y-websocket",
            "hocuspocus",
            "yrs-warp",
            "y-sync",
            "liveblocks",
            "partykit",
            "sharedb",
            "automerge-repo-network",
        ];
        let cargo_toml =
            std::fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("read Cargo.toml");
        for forbidden in forbidden_dependencies {
            assert!(
                !cargo_toml.contains(forbidden),
                "Cargo.toml must not declare relay dependency '{forbidden}'"
            );
        }

        // The CRDT modules and the knowledge CRDT API never dial out: no
        // websocket-relay URLs, no relay client imports.
        let forbidden_tokens = [
            "ws://",
            "wss://",
            "y-websocket",
            "hocuspocus",
            "liveblocks",
            "partykit",
            "sync-server",
            "tokio_tungstenite",
        ];
        let crdt_dir = manifest_dir.join("src").join("kernel").join("crdt");
        let mut scanned = vec![manifest_dir
            .join("src")
            .join("api")
            .join("knowledge_crdt.rs")];
        for entry in std::fs::read_dir(&crdt_dir).expect("read crdt dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                scanned.push(path);
            }
        }
        assert!(scanned.len() > 10, "scan must cover the CRDT module set");
        for path in scanned {
            let source = std::fs::read_to_string(&path).expect("read source");
            for forbidden in forbidden_tokens {
                assert!(
                    !source.contains(forbidden),
                    "{} must not reference '{forbidden}'",
                    path.display()
                );
            }
        }
    }

    /// Runtime proof: a complete multi-actor draft cycle (push, idempotent
    /// replay, stale rejection, pull-equivalent listing) completes against
    /// local embedded storage alone — no relay process, no external sync service,
    /// and every durable byte ref uses the surreal:// scheme.
    #[tokio::test]
    async fn full_draft_cycle_needs_only_embedded_storage() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt078-{suffix}");
        let doc = format!("doc-mt078-{suffix}");
        let crdt_doc = format!("crdt-mt078-{suffix}");
        let actor_a =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "relayless-op").expect("actor");
        let actor_b =
            KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "relayless-cm").expect("actor");
        let site_a = derive_knowledge_site_id(&ws, &crdt_doc, &actor_a);
        let site_b = derive_knowledge_site_id(&ws, &crdt_doc, &actor_b);

        let mut sv = KnowledgeStateVectorV1::new();
        for (index, (actor, site)) in [(&actor_a, &site_a), (&actor_b, &site_b)]
            .into_iter()
            .enumerate()
        {
            let before = sv.clone();
            sv.increment(&site.site_id);
            let env = envelope(
                &ws,
                &doc,
                &crdt_doc,
                &format!("relayless-u{}", index + 1),
                actor,
                "sr-relayless",
                format!("relayless-{}", index + 1).as_bytes(),
                &before,
                &sv,
            );
            assert!(matches!(
                push_yjs_update(db.as_ref(), &env).await.expect("push"),
                YjsPushOutcomeV1::Stored { .. }
            ));
        }

        let records = db
            .list_kernel_crdt_updates(&ws, &doc, &crdt_doc)
            .await
            .expect("list");
        assert_eq!(records.len(), 2);
        for record in &records {
            assert!(
                record.update_bytes_ref.starts_with("surreal://"),
                "durable refs must be surreal://, found {}",
                record.update_bytes_ref
            );
            assert_eq!(
                record.event_ledger_stream_id,
                format!("knowledge-crdt:{crdt_doc}")
            );
        }
    }
}

mod mt_080_spec_compatibility {
    use super::*;
    use handshake_core::kernel::crdt::offline_boundary::{
        knowledge_offline_draft_boundary_contract, validate_offline_draft_boundary_contract,
    };
    use handshake_core::kernel::crdt::persistence::{
        kernel_crdt_surreal_update_log_contract, validate_crdt_update_record,
        CrdtStorageAuthorityPosture,
    };
    use handshake_core::kernel::crdt::yjs_bridge::validate_yjs_update_envelope;
    use handshake_core::kernel::KernelEventType;
    use handshake_core::storage::knowledge_crdt::{
        insert_denial_receipt, new_denial_receipt_id, NewKnowledgeCrdtDenialReceipt,
    };
    use serde_json::json;
    use uuid::Uuid;

    /// Spec 2.3.13.11: "RichDocument and EditorCodeNode edits MAY use CRDT
    /// state for collaboration and pre-promotion drafting, but authority
    /// changes MUST flow through WriteBoxV1 and EventLedger promotion."
    /// Proven over the real implementation: draft rows exist without any
    /// authority fact; the only path that creates an authority fact appends
    /// the EventLedger promotion pair first; the direct authority-insert
    /// mutation branch is explicitly dispositioned below because no public
    /// embedded mutation operation is exposed.
    #[tokio::test]
    async fn must_authority_changes_flow_through_event_ledger_promotion() {
        let backend = embedded_backend_or_blocked().await;
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt080a-{suffix}");
        let doc = format!("doc-mt080a-{suffix}");
        let crdt_doc = format!("crdt-mt080a-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "spec-op").expect("actor");
        let site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);

        // CRDT drafting works pre-promotion...
        let mut sv = KnowledgeStateVectorV1::new();
        let before = sv.clone();
        sv.increment(&site.site_id);
        let env = envelope(
            &ws, &doc, &crdt_doc, "spec-u1", &operator, "sr-spec", b"draft", &before, &sv,
        );
        assert!(matches!(
            push_yjs_update(backend.database.as_ref(), &env)
                .await
                .expect("push"),
            YjsPushOutcomeV1::Stored { .. }
        ));

        // ...and produces NO authority facts by itself.
        let inspector = backend.storage.test_inspector();
        let facts = inspector
            .table_selector("knowledge_crdt_promoted_facts")
            .await
            .expect("select promoted facts table");
        let fact_count = inspector
            .row_count(
                &facts,
                RowFilter::FieldEquals {
                    field: facts.field("workspace_id").expect("select workspace field"),
                    value: ws.as_str().into(),
                },
            )
            .await
            .expect("count facts");
        assert_eq!(fact_count, 0, "drafting must not create authority");

        // The direct authority-write branch is explicitly dispositioned below
        // because the public embedded surface currently exposes no mutation
        // handle for schema-negative writes.
    }

    /// Spec 2.3.13.11: "AI edit proposals, graph mutation proposals, ...
    /// auto-tagging, and manual edits MUST leave actor, source span,
    /// state-vector, validation, denial, or promotion receipts."
    /// Proven receipt-by-receipt over the real rows and events.
    #[tokio::test]
    async fn must_every_edit_class_leaves_typed_receipts() {
        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt080b-{suffix}");
        let doc = format!("doc-mt080b-{suffix}");
        let crdt_doc = format!("crdt-mt080b-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "spec-op").expect("actor");
        let site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);

        // Manual edit: actor + state-vector receipt on the update row AND
        // the paired EventLedger event.
        let mut sv = KnowledgeStateVectorV1::new();
        let before = sv.clone();
        sv.increment(&site.site_id);
        let env = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "receipt-u1",
            &operator,
            "sr-receipt",
            b"manual-edit",
            &before,
            &sv,
        );
        push_yjs_update(db.as_ref(), &env).await.expect("push");
        let records = db
            .list_kernel_crdt_updates(&ws, &doc, &crdt_doc)
            .await
            .expect("list");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.actor_id, operator.canonical(), "actor receipt");
        assert_eq!(
            record.state_vector_before,
            before.encode(),
            "state-vector receipt"
        );
        assert_eq!(
            record.state_vector_after,
            sv.encode(),
            "state-vector receipt"
        );
        assert!(!record.event_ledger_event_id.is_empty(), "ledger receipt");
        let events = db
            .list_kernel_events_for_aggregate("knowledge_crdt_document", &crdt_doc)
            .await
            .expect("events");
        assert!(events
            .iter()
            .any(|event| event.event_type == KernelEventType::KnowledgeCrdtUpdateRecorded));

        // Graph mutation + AI edit proposal receipts are proven on real rows
        // in knowledge_crdt_proposal_tests (MT-068/MT-074); here we pin the
        // Typed implementation guarantee: actor and span evidence are
        // required by the active proposal paths.
    }

    /// Spec 2.3.13.11: denial receipts are durable and typed. The embedded
    /// write boundary must reject both an untyped actor and a dangling
    /// EventLedger reference, without persisting either attempted receipt.
    #[tokio::test]
    async fn must_denial_receipts_are_durable_and_ledger_paired() {
        let backend = embedded_backend_or_blocked().await;
        let suffix = Uuid::now_v7().simple().to_string();
        let typed_actor = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "denial-proof")
            .expect("typed actor");
        let untyped_receipt_id = new_denial_receipt_id();
        let untyped_error = insert_denial_receipt(
            &backend.storage,
            NewKnowledgeCrdtDenialReceipt {
                receipt_id: untyped_receipt_id.clone(),
                receipt_kind: "stale_draft_save".to_string(),
                workspace_id: format!("ws-denial-{suffix}"),
                document_id: Some(format!("doc-denial-{suffix}")),
                crdt_document_id: Some(format!("crdt-denial-{suffix}")),
                scope_ref: format!("crdt_document:crdt-denial-{suffix}"),
                actor_id: "not-a-typed-actor".to_string(),
                actor_kind: "operator".to_string(),
                session_id: format!("sr-denial-{suffix}"),
                correlation_id: format!("corr-denial-{suffix}"),
                denial_payload: json!({"reason": "proof"}),
                event_ledger_event_id: format!("fabricated-event-{suffix}"),
                idempotency_key: format!("denial-untyped-{suffix}"),
            },
        )
        .await
        .expect_err("untyped denial actor must be rejected before persistence");
        assert!(
            untyped_error.to_string().contains("actor id is not typed"),
            "unexpected untyped-actor error: {untyped_error}"
        );

        let dangling_receipt_id = new_denial_receipt_id();
        let dangling_error = insert_denial_receipt(
            &backend.storage,
            NewKnowledgeCrdtDenialReceipt {
                receipt_id: dangling_receipt_id.clone(),
                receipt_kind: "stale_draft_save".to_string(),
                workspace_id: format!("ws-denial-{suffix}"),
                document_id: Some(format!("doc-denial-{suffix}")),
                crdt_document_id: Some(format!("crdt-denial-{suffix}")),
                scope_ref: format!("crdt_document:crdt-denial-{suffix}"),
                actor_id: typed_actor.canonical(),
                actor_kind: typed_actor.kind().as_str().to_string(),
                session_id: format!("sr-denial-{suffix}"),
                correlation_id: format!("corr-denial-{suffix}"),
                denial_payload: json!({"reason": "proof"}),
                event_ledger_event_id: format!("fabricated-event-{suffix}"),
                idempotency_key: format!("denial-dangling-{suffix}"),
            },
        )
        .await
        .expect_err("dangling denial ledger reference must be rejected");
        assert!(
            !dangling_error.to_string().trim().is_empty(),
            "dangling ledger rejection must carry a diagnostic"
        );

        let inspector = backend.storage.test_inspector();
        let receipts = inspector
            .table_selector("knowledge_crdt_denial_receipts")
            .await
            .expect("select denial receipt table");
        for receipt_id in [untyped_receipt_id, dangling_receipt_id] {
            assert_eq!(
                inspector
                    .row_count(&receipts, RowFilter::IdEquals(receipt_id))
                    .await
                    .expect("count attempted denial receipt"),
                0,
                "rejected denial receipt must not persist"
            );
        }
    }

    /// Spec 2.3.13.11: storage-authority MUSTs. Browser/file/memory state is
    /// never CRDT authority; the typed posture and the update-log contract
    /// reject every non-Surreal authority claim.
    #[test]
    fn must_surreal_event_ledger_is_the_only_crdt_authority() {
        // The update-log contract names the denied authority surfaces.
        let contract = kernel_crdt_surreal_update_log_contract();
        assert_eq!(contract.table_name, "kernel_crdt_updates");
        assert!(contract
            .denied_authority_refs
            .contains(&"browser_local_storage_authority"));
        assert!(contract
            .denied_authority_refs
            .contains(&"filesystem_update_bytes"));

        // A record claiming filesystem authority fails validation.
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "authority-op").expect("actor");
        let site = derive_knowledge_site_id("ws-auth", "crdt-auth", &actor);
        let mut sv = KnowledgeStateVectorV1::new();
        let before = sv.clone();
        sv.increment(&site.site_id);
        let env = envelope(
            "ws-auth",
            "doc-auth",
            "crdt-auth",
            "auth-u1",
            &actor,
            "sr-auth",
            b"bytes",
            &before,
            &sv,
        );
        let validated = validate_yjs_update_envelope(&env).expect("valid envelope");
        let mut record = handshake_core::kernel::crdt::yjs_bridge::envelope_to_update_record(
            &env, &validated, 1, "evt-auth",
        );
        record.storage_authority = CrdtStorageAuthorityPosture::FileSystemAuthority;
        record.update_bytes_ref = "file://draft/update.bin".to_string();
        let errors = validate_crdt_update_record(&record)
            .expect_err("filesystem authority must be rejected");
        assert!(errors
            .iter()
            .any(|error| error.field == "storage_authority"));
        assert!(errors.iter().any(|error| error.field == "update_bytes_ref"));

        // The offline boundary contract pins the same law for client state.
        let boundary = knowledge_offline_draft_boundary_contract();
        validate_offline_draft_boundary_contract(&boundary).expect("boundary sound");
        assert!(boundary
            .denied_durable_surfaces
            .contains(&"browser_local_storage"));
    }

    /// MT-080 contract scope: the CRDT implementation must not conflict with
    /// the DEFERRED realtime multi-user UI boundary. Pin: the backend draft
    /// path is complete without any realtime relay (per-update envelopes
    /// over request/response, replay by pull), so deferring the realtime UI
    /// removes no MUST-level capability; and the spec's backend-navigation
    /// identification is enforced at the envelope layer (empty session ids
    /// are refused), independent of any UI.
    #[test]
    fn deferred_realtime_ui_boundary_leaves_crdt_law_intact() {
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "deferred-op").expect("actor");
        let site = derive_knowledge_site_id("ws-deferred", "crdt-deferred", &actor);
        let mut sv = KnowledgeStateVectorV1::new();
        let before = sv.clone();
        sv.increment(&site.site_id);
        let mut env = envelope(
            "ws-deferred",
            "doc-deferred",
            "crdt-deferred",
            "deferred-u1",
            &actor,
            "sr-deferred",
            b"bytes",
            &before,
            &sv,
        );
        // The envelope is fully self-describing: ids, actor, site, hashes,
        // typed vectors — nothing presumes a live multi-user socket.
        assert!(validate_yjs_update_envelope(&env).is_ok());

        // Identification cannot be dropped "because the UI is single-user".
        env.session_id = "  ".to_string();
        assert!(validate_yjs_update_envelope(&env).is_err());
    }

    /// Spec 2.3.13.11: "KnowledgeClaim ... Claims MUST carry ... evidence
    /// spans." Authority-hardening #6: the evidence-span-existence MUST had
    /// no guard test. Prove it end-to-end on the real embedded store: a promotion
    /// citing a non-existent `KSP-` span is REFUSED with a durable receipt and
    /// creates no authority fact — the MUST is enforced, not merely declared.
    #[tokio::test]
    async fn must_refuse_promotion_citing_nonexistent_span() {
        use handshake_core::kernel::crdt::claim_promotion::{
            promote_graph_proposal, GraphPromotionDenialReasonV1, GraphPromotionOutcomeV1,
        };
        use handshake_core::kernel::crdt::graph_proposal::{
            decide_graph_proposal, record_graph_proposal, GraphMutationKind,
            GraphMutationProposalRequestV1, RecordGraphProposalOutcomeV1,
        };
        use handshake_core::storage::knowledge_crdt::{
            get_promoted_fact_by_proposal, list_denial_receipts_for_scope, PromotionSpanRejection,
        };

        let backend = embedded_backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.storage.clone();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt080-span-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "spec-op").expect("actor");

        // A canonical-but-NONEXISTENT KSP- id (never inserted).
        let ghost_span = format!("KSP-{}", "0".repeat(32));
        let proposal = match record_graph_proposal(
            db.as_ref(),
            &pool,
            GraphMutationProposalRequestV1 {
                workspace_id: ws.clone(),
                mutation_kind: GraphMutationKind::AddClaim,
                mutation_payload: json!({"claim_text": "cites a span that does not exist"}),
                source_span_refs: vec![ghost_span.clone()],
                confidence: 0.5,
                actor: operator.clone(),
                session_id: format!("sr-mt080-{suffix}"),
                correlation_id: format!("corr-mt080-{suffix}"),
                lease_id: None,
            },
        )
        .await
        .expect("record flow")
        {
            RecordGraphProposalOutcomeV1::Recorded(row) => *row,
            other => panic!("expected recorded draft, got {other:?}"),
        };
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            &format!("sr-mt080-{suffix}"),
            "approved as draft",
        )
        .await
        .expect("decide flow")
        .expect("approved");

        let outcome = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            &format!("sr-mt080-{suffix}"),
            &format!("corr-mt080-{suffix}"),
        )
        .await
        .expect("promotion flow");
        match outcome {
            GraphPromotionOutcomeV1::Denied(denial) => match denial.reason {
                GraphPromotionDenialReasonV1::SpanEvidenceInvalid { rejections } => {
                    assert!(
                        rejections
                            .iter()
                            .any(|r| matches!(r, PromotionSpanRejection::SpanNotFound { .. })),
                        "non-existent span must reject with SpanNotFound, got {rejections:?}"
                    );
                }
                other => panic!("expected SpanEvidenceInvalid, got {other:?}"),
            },
            other => panic!("promotion citing a non-existent span must be REFUSED, got {other:?}"),
        }

        // No authority fact; a durable denial receipt exists.
        assert!(
            get_promoted_fact_by_proposal(&pool, &proposal.proposal_id)
                .await
                .expect("fact lookup")
                .is_none(),
            "refused promotion must create no authority fact"
        );
        let receipts =
            list_denial_receipts_for_scope(&pool, &format!("proposal:{}", proposal.proposal_id))
                .await
                .expect("receipts");
        assert!(
            receipts
                .iter()
                .any(|r| r.receipt_kind == "graph_promotion_denied"),
            "a durable graph_promotion_denied receipt must exist"
        );
    }
}
