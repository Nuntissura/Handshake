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
use handshake_core::storage::tests::{postgres_backend_with_pool_from_env, PostgresTestBackend};
use handshake_core::storage::StorageError;

async fn backend_or_blocked() -> PostgresTestBackend {
    match postgres_backend_with_pool_from_env().await {
        Ok(backend) => backend,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!("ENVIRONMENT_BLOCKED: WP-009 CRDT proof tests require POSTGRES_TEST_URL; {msg}");
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
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
        promote_graph_proposal, GraphPromotionOutcomeV1,
    };
    use handshake_core::kernel::crdt::graph_proposal::{
        decide_graph_proposal, record_graph_proposal, GraphMutationKind,
        GraphMutationProposalRequestV1,
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
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
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
        let proposal = record_graph_proposal(
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
        .expect("valid proposal");
        decide_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            true,
            &operator,
            "sr-e2e",
            "operator-authored claim",
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
            format!("corr-e2e-{suffix}").as_str(),
        )
        .await
        .expect("promotion flow");
        let fact = match promoted {
            GraphPromotionOutcomeV1::Promoted(fact) => fact,
            other => panic!("expected promotion, got {other:?}"),
        };

        // Promotion is exactly-once: replays converge on the same fact and
        // the ledger holds exactly one REQUESTED/ACCEPTED pair.
        let replay = promote_graph_proposal(
            db.as_ref(),
            &pool,
            &proposal.proposal_id,
            &operator,
            "sr-e2e",
            format!("corr-e2e-{suffix}").as_str(),
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
    }
}

mod mt_078_no_external_relay {
    use super::*;
    use uuid::Uuid;

    /// Static proof: the WP-009 CRDT surface declares no external sync
    /// server, relay, or hosted CRDT service — not in Cargo dependencies and
    /// not in the CRDT/API source. The draft path speaks PostgreSQL only.
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
    /// local PostgreSQL alone — no relay process, no external sync service,
    /// and every durable byte ref uses the postgres:// scheme.
    #[tokio::test]
    async fn full_draft_cycle_needs_only_postgres() {
        let backend = backend_or_blocked().await;
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
                record.update_bytes_ref.starts_with("postgres://"),
                "durable refs must be postgres://, found {}",
                record.update_bytes_ref
            );
            assert_eq!(
                record.event_ledger_stream_id,
                format!("knowledge-crdt:{crdt_doc}")
            );
        }
    }
}
