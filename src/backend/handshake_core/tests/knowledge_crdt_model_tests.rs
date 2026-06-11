//! WP-KERNEL-009 CRDTAndConcurrencyCore model tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_065_actor_site: MT-065 ActorSiteIdModel
//!   - mt_072_state_vector: MT-072 VectorClockOrEquivalentMetadata
//!   - mt_066_snapshot_model: MT-066 RichDocumentCrdtSnapshotModel
//!
//! Every PostgreSQL test runs against the Handshake-managed/real PostgreSQL
//! named by POSTGRES_TEST_URL through `postgres_backend_from_env` (isolated
//! schema, full migration chain). No SQLite, no mocks, no fixtures-as-proof.

use handshake_core::storage::{tests::postgres_backend_from_env, StorageError};

async fn postgres_or_environment_blocked() -> std::sync::Arc<dyn handshake_core::storage::Database>
{
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!("ENVIRONMENT_BLOCKED: WP-009 CRDT model tests require POSTGRES_TEST_URL; {msg}");
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

mod mt_066_snapshot_model {
    use super::postgres_or_environment_blocked;
    use handshake_core::kernel::crdt::actor_site::{
        knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
    };
    use handshake_core::kernel::crdt::rich_document_snapshot::{
        build_rich_document_snapshot_record, restore_rich_document_snapshot,
        validate_rich_document_snapshot_payload, RichDocumentRestoreError,
        RichDocumentSnapshotPayloadV1, RICH_DOCUMENT_SCHEMA_ID,
        RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID,
    };
    use handshake_core::kernel::{KernelEventType, NewKernelEvent};
    use serde_json::json;
    use uuid::Uuid;

    fn sample_payload() -> RichDocumentSnapshotPayloadV1 {
        RichDocumentSnapshotPayloadV1 {
            schema_id: RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID.to_string(),
            document_schema_id: RICH_DOCUMENT_SCHEMA_ID.to_string(),
            prosemirror_schema_version: "tiptap-starter-kit@3.13.0".to_string(),
            doc_json: json!({
                "type": "doc",
                "content": [
                    {"type": "heading", "attrs": {"level": 1},
                     "content": [{"type": "text", "text": "MT-066"}]},
                    {"type": "paragraph",
                     "content": [{"type": "text", "text": "snapshot model"}]}
                ]
            }),
            state_vector: "hsk-sv1:site-aaaa=4;site-bbbb=2".to_string(),
            covered_update_seq: 6,
        }
    }

    #[test]
    fn payload_validation_rejects_structural_defects() {
        assert!(validate_rich_document_snapshot_payload(&sample_payload()).is_ok());

        let mut wrong_root = sample_payload();
        wrong_root.doc_json = json!({"type": "paragraph"});
        let errors = validate_rich_document_snapshot_payload(&wrong_root)
            .expect_err("non-doc root must fail");
        assert!(errors.iter().any(|error| error.field == "doc_json.type"));

        let mut not_object = sample_payload();
        not_object.doc_json = json!(["not", "a", "doc"]);
        assert!(validate_rich_document_snapshot_payload(&not_object)
            .expect_err("array root must fail")
            .iter()
            .any(|error| error.field == "doc_json"));

        let mut unstamped = sample_payload();
        unstamped.prosemirror_schema_version = "  ".to_string();
        assert!(validate_rich_document_snapshot_payload(&unstamped)
            .expect_err("blank schema version must fail")
            .iter()
            .any(|error| error.field == "prosemirror_schema_version"));

        let mut bad_vector = sample_payload();
        bad_vector.state_vector = "yjs-binary-sv".to_string();
        assert!(validate_rich_document_snapshot_payload(&bad_vector)
            .expect_err("untyped state vector must fail")
            .iter()
            .any(|error| error.field == "state_vector"));

        let mut wrong_schema = sample_payload();
        wrong_schema.schema_id = "hsk.other@1".to_string();
        assert!(validate_rich_document_snapshot_payload(&wrong_schema)
            .expect_err("wrong payload schema id must fail")
            .iter()
            .any(|error| error.field == "schema_id"));
    }

    #[test]
    fn build_and_restore_round_trip_without_storage() {
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt066").expect("valid actor");
        let identity = knowledge_crdt_identity(
            "ws-mt066",
            "doc-mt066",
            "crdt-mt066",
            RICH_DOCUMENT_SCHEMA_ID,
            &actor,
            "trace-mt066",
        );
        let payload = sample_payload();
        let (record, bytes) = build_rich_document_snapshot_record(
            &identity,
            "snap-mt066-1",
            &payload,
            "evt-mt066-1",
            &["mt066-u5", "mt066-u6"],
        )
        .expect("snapshot record builds");
        assert_eq!(record.covered_update_seq, payload.covered_update_seq);
        assert_eq!(record.state_vector, payload.state_vector);
        assert_eq!(
            record.promotion_evidence_update_ids,
            vec!["mt066-u5".to_string(), "mt066-u6".to_string()]
        );

        let restored =
            restore_rich_document_snapshot(&record, &bytes).expect("restore round-trips");
        assert_eq!(restored.doc_json, payload.doc_json);
        assert_eq!(
            restored.prosemirror_schema_version,
            "tiptap-starter-kit@3.13.0"
        );
        assert_eq!(restored.covered_update_seq, 6);

        // Tampered bytes are refused by the hash check.
        let mut tampered = bytes.clone();
        tampered[0] ^= 0xFF;
        assert!(matches!(
            restore_rich_document_snapshot(&record, &tampered),
            Err(RichDocumentRestoreError::ByteHashMismatch { .. })
        ));

        // Envelope/payload drift is refused even when hashes are recomputed.
        let mut drifted_payload = payload.clone();
        drifted_payload.covered_update_seq = 7;
        let (drift_record, _) = build_rich_document_snapshot_record(
            &identity,
            "snap-mt066-2",
            &drifted_payload,
            "evt-mt066-2",
            &[],
        )
        .expect("drift record builds");
        // Restore the *other* payload bytes against this envelope.
        assert!(matches!(
            restore_rich_document_snapshot(&drift_record, &bytes),
            Err(RichDocumentRestoreError::ByteHashMismatch { .. })
        ));

        // Identity/payload schema mismatch fails the build path itself.
        let mut alien_identity = identity.clone();
        alien_identity.document_schema_id = "hsk.doc.other@1".to_string();
        assert!(build_rich_document_snapshot_record(
            &alien_identity,
            "snap-mt066-3",
            &payload,
            "evt-mt066-3",
            &[],
        )
        .is_err());
    }

    /// PostgreSQL proof: the rich-document snapshot persists through the
    /// kernel CRDT snapshot store and restores to the identical document
    /// JSON after a fresh read of envelope + bytes.
    #[tokio::test]
    async fn rich_document_snapshot_persists_and_restores_from_postgres() {
        let db = postgres_or_environment_blocked().await;
        let suffix = Uuid::now_v7().simple().to_string();
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt066").expect("valid actor");
        let identity = knowledge_crdt_identity(
            &format!("ws-mt066-{suffix}"),
            &format!("doc-mt066-{suffix}"),
            &format!("crdt-mt066-{suffix}"),
            RICH_DOCUMENT_SCHEMA_ID,
            &actor,
            &format!("trace-mt066-{suffix}"),
        );

        let event = NewKernelEvent::builder(
            format!("KTR-MT066-{suffix}"),
            format!("SR-MT066-{suffix}"),
            KernelEventType::KnowledgeCrdtSnapshotRecorded,
            actor.to_kernel_actor(),
        )
        .aggregate("knowledge_crdt_document", identity.crdt_document_id.clone())
        .idempotency_key(format!("mt066:{suffix}:snapshot-1"))
        .source_component("knowledge_crdt_model_tests")
        .payload(json!({"snapshot_id": format!("snap-{suffix}")}))
        .build()
        .expect("valid event");
        let stored_event = db.append_kernel_event(event).await.expect("append event");

        let payload = sample_payload();
        let (record, bytes) = build_rich_document_snapshot_record(
            &identity,
            &format!("snap-{suffix}"),
            &payload,
            &stored_event.event_id,
            &[],
        )
        .expect("snapshot record builds");

        db.append_kernel_crdt_snapshot(record.clone(), bytes)
            .await
            .expect("append snapshot to Postgres");

        let snapshots = db
            .list_kernel_crdt_snapshots(
                &identity.workspace_id,
                &identity.document_id,
                &identity.crdt_document_id,
            )
            .await
            .expect("list snapshots");
        assert_eq!(snapshots.len(), 1);
        let persisted = &snapshots[0];
        let persisted_bytes = db
            .read_kernel_crdt_snapshot_bytes(&persisted.snapshot_bytes_ref)
            .await
            .expect("read snapshot bytes from Postgres");

        let restored = restore_rich_document_snapshot(persisted, &persisted_bytes)
            .expect("restore from persisted snapshot");
        assert_eq!(restored.doc_json, payload.doc_json);
        assert_eq!(restored.state_vector, payload.state_vector);
        assert_eq!(restored.covered_update_seq, payload.covered_update_seq);
        assert_eq!(restored.event_ledger_event_id, stored_event.event_id);
        assert_eq!(
            restored.prosemirror_schema_version,
            "tiptap-starter-kit@3.13.0"
        );
    }
}

mod mt_072_state_vector {
    use super::postgres_or_environment_blocked;
    use handshake_core::kernel::crdt::actor_site::{
        derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
    };
    use handshake_core::kernel::crdt::persistence::{
        new_crdt_update_record, CrdtReplayMetadataV1, CrdtUpdateRecordInputV1,
    };
    use handshake_core::kernel::crdt::state_vector::{
        verify_causal_chain, KnowledgeCausalChainError, KnowledgeStateVectorOrdering,
        KnowledgeStateVectorParseError, KnowledgeStateVectorV1,
    };
    use handshake_core::kernel::{KernelEventType, NewKernelEvent};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn state_vector_encoding_is_canonical_and_round_trips() {
        let empty = KnowledgeStateVectorV1::new();
        assert_eq!(empty.encode(), "hsk-sv1:");
        assert_eq!(
            KnowledgeStateVectorV1::parse("hsk-sv1:").expect("empty parses"),
            empty
        );

        let mut sv = KnowledgeStateVectorV1::new();
        sv.observe("site-bb", 2);
        sv.observe("site-aa", 7);
        // Lexicographic site order regardless of observe order.
        assert_eq!(sv.encode(), "hsk-sv1:site-aa=7;site-bb=2");
        let reparsed = KnowledgeStateVectorV1::parse(&sv.encode()).expect("round-trip");
        assert_eq!(reparsed, sv);
        assert_eq!(reparsed.clock("site-aa"), 7);
        assert_eq!(reparsed.clock("site-absent"), 0);
        assert_eq!(reparsed.lamport_max(), 7);

        // serde round-trip uses the canonical string.
        let json_form = serde_json::to_string(&sv).expect("serialize");
        assert_eq!(json_form, "\"hsk-sv1:site-aa=7;site-bb=2\"");
        let back: KnowledgeStateVectorV1 = serde_json::from_str(&json_form).expect("deserialize");
        assert_eq!(back, sv);
    }

    #[test]
    fn state_vector_parse_rejects_malformed_inputs() {
        for (input, expectation) in [
            ("sv-without-prefix", "MissingPrefix"),
            ("hsk-sv1:;", "EmptyEntry"),
            ("hsk-sv1:site-a", "MissingClockSeparator"),
            ("hsk-sv1:=3", "EmptySite"),
            ("hsk-sv1:site a=3", "BadSiteChar"),
            ("hsk-sv1:site-a=x", "BadClock"),
            ("hsk-sv1:site-a=0", "ZeroClock"),
            ("hsk-sv1:site-a=1;site-a=2", "DuplicateSite"),
        ] {
            let err = KnowledgeStateVectorV1::parse(input)
                .expect_err(&format!("'{input}' must fail to parse"));
            let matches_expectation = matches!(
                (&err, expectation),
                (
                    KnowledgeStateVectorParseError::MissingPrefix { .. },
                    "MissingPrefix"
                ) | (KnowledgeStateVectorParseError::EmptyEntry, "EmptyEntry")
                    | (
                        KnowledgeStateVectorParseError::MissingClockSeparator { .. },
                        "MissingClockSeparator"
                    )
                    | (
                        KnowledgeStateVectorParseError::EmptySite { .. },
                        "EmptySite"
                    )
                    | (
                        KnowledgeStateVectorParseError::BadSiteChar { .. },
                        "BadSiteChar"
                    )
                    | (KnowledgeStateVectorParseError::BadClock { .. }, "BadClock")
                    | (
                        KnowledgeStateVectorParseError::ZeroClock { .. },
                        "ZeroClock"
                    )
                    | (
                        KnowledgeStateVectorParseError::DuplicateSite { .. },
                        "DuplicateSite"
                    )
            );
            assert!(
                matches_expectation,
                "'{input}' produced unexpected error {err:?}"
            );
        }
    }

    #[test]
    fn causality_comparison_covers_all_orderings() {
        let parse = |s: &str| KnowledgeStateVectorV1::parse(s).expect("valid sv");
        let base = parse("hsk-sv1:a=2;b=1");
        assert_eq!(
            base.compare(&parse("hsk-sv1:a=2;b=1")),
            KnowledgeStateVectorOrdering::Equal
        );
        assert_eq!(
            base.compare(&parse("hsk-sv1:a=1;b=1")),
            KnowledgeStateVectorOrdering::Dominates
        );
        assert_eq!(
            base.compare(&parse("hsk-sv1:a=2;b=1;c=4")),
            KnowledgeStateVectorOrdering::DominatedBy
        );
        assert_eq!(
            base.compare(&parse("hsk-sv1:a=1;b=1;c=4")),
            KnowledgeStateVectorOrdering::Concurrent
        );
        // Empty vector is dominated by anything non-empty.
        assert_eq!(
            KnowledgeStateVectorV1::new().compare(&base),
            KnowledgeStateVectorOrdering::DominatedBy
        );
        assert!(base.dominates_or_equal(&parse("hsk-sv1:a=1")));
        assert!(!base.dominates_or_equal(&parse("hsk-sv1:c=1")));
    }

    #[test]
    fn merge_takes_pointwise_maximum_and_increment_advances_locally() {
        let parse = |s: &str| KnowledgeStateVectorV1::parse(s).expect("valid sv");
        let ours = parse("hsk-sv1:a=3;b=1");
        let theirs = parse("hsk-sv1:a=1;c=5");
        let merged = ours.merge(&theirs);
        assert_eq!(merged.encode(), "hsk-sv1:a=3;b=1;c=5");
        assert!(merged.dominates_or_equal(&ours));
        assert!(merged.dominates_or_equal(&theirs));

        let mut local = merged.clone();
        assert_eq!(local.increment("b"), 2);
        assert_eq!(local.increment("new-site"), 1);
        assert_eq!(
            local.compare(&merged),
            KnowledgeStateVectorOrdering::Dominates
        );
    }

    #[test]
    fn causal_chain_proof_rejects_broken_chains() {
        // Build two records whose before/after vectors do not chain.
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "mt072").expect("valid actor");
        let identity = knowledge_crdt_identity(
            "ws-mt072-neg",
            "doc-mt072-neg",
            "crdt-mt072-neg",
            "hsk.doc.rich_document@1",
            &actor,
            "trace-mt072-neg",
        );
        let site = derive_knowledge_site_id("ws-mt072-neg", "crdt-mt072-neg", &actor);
        let make = |seq: u64, before: String, after: String| {
            new_crdt_update_record(CrdtUpdateRecordInputV1 {
                identity: &identity,
                update_id: &format!("u{seq}"),
                update_seq: seq,
                update_bytes: b"x",
                update_bytes_ref: &format!(
                    "postgres://kernel_crdt_updates/{}/u{seq}/update_bytes",
                    identity.crdt_document_id
                ),
                session_id: "sr-mt072",
                trace_id: "trace-mt072",
                state_vector_before: &before,
                state_vector_after: &after,
                replay_metadata: CrdtReplayMetadataV1 {
                    replay_order_key: format!("k/{seq:020}"),
                    dependency_update_ids: Vec::new(),
                    encoding: "yjs-update-v1".to_string(),
                    schema_version: "kernel-crdt-update-v1".to_string(),
                },
                event_ledger_event_id: &format!("evt-mt072-{seq}"),
            })
        };
        let sv = |clock: u64| format!("hsk-sv1:{}={clock}", site.site_id);

        // after == before (no domination) is rejected.
        let stuck = make(1, sv(1), sv(1));
        assert!(matches!(
            verify_causal_chain(&[stuck]),
            Err(KnowledgeCausalChainError::AfterDoesNotDominateBefore { .. })
        ));

        // before of step 2 skips the after of step 1.
        let first = make(1, "hsk-sv1:".to_string(), sv(1));
        let skipping = make(2, sv(2), sv(3));
        assert!(matches!(
            verify_causal_chain(&[first.clone(), skipping]),
            Err(KnowledgeCausalChainError::BeforeBreaksChain { .. })
        ));

        // sequence gap is rejected before vector checks.
        let gapped = make(3, sv(1), sv(2));
        assert!(matches!(
            verify_causal_chain(&[first, gapped]),
            Err(KnowledgeCausalChainError::NonContiguousSequence { .. })
        ));

        assert!(matches!(
            verify_causal_chain(&[]),
            Err(KnowledgeCausalChainError::EmptyChain)
        ));
    }

    /// PostgreSQL proof: typed causal metadata persists on kernel_crdt_updates
    /// rows and the replay-ordering proof verifies over the listed records,
    /// out-of-order input included (Postgres replay order is by update_seq).
    #[tokio::test]
    async fn persisted_causal_metadata_yields_replay_ordering_proof() {
        let db = postgres_or_environment_blocked().await;
        let suffix = Uuid::now_v7().simple().to_string();
        let human =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt072").expect("valid actor");
        let model = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "lm-mt072")
            .expect("valid actor");
        let workspace_id = format!("ws-mt072-{suffix}");
        let document_id = format!("doc-mt072-{suffix}");
        let crdt_document_id = format!("crdt-mt072-{suffix}");

        let human_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &human);
        let model_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &model);

        // Interleaved two-actor causal chain: op, model, op.
        let mut sv = KnowledgeStateVectorV1::new();
        let mut chain = Vec::new();
        for (seq, (actor, site)) in [
            (&human, &human_site),
            (&model, &model_site),
            (&human, &human_site),
        ]
        .into_iter()
        .enumerate()
        {
            let seq = seq as u64 + 1;
            let before = sv.encode();
            sv.increment(&site.site_id);
            let after = sv.encode();
            let identity = knowledge_crdt_identity(
                &workspace_id,
                &document_id,
                &crdt_document_id,
                "hsk.doc.rich_document@1",
                actor,
                &format!("trace-mt072-{suffix}"),
            );
            let event = NewKernelEvent::builder(
                format!("KTR-MT072-{suffix}"),
                format!("SR-MT072-{suffix}"),
                KernelEventType::KnowledgeCrdtUpdateRecorded,
                actor.to_kernel_actor(),
            )
            .aggregate("knowledge_crdt_document", crdt_document_id.clone())
            .idempotency_key(format!("mt072:{suffix}:u{seq}"))
            .source_component("knowledge_crdt_model_tests")
            .payload(json!({"seq": seq, "after": after}))
            .build()
            .expect("valid event");
            let stored_event = db.append_kernel_event(event).await.expect("append event");

            let bytes = format!("mt072-update-{seq}").into_bytes();
            let record = new_crdt_update_record(CrdtUpdateRecordInputV1 {
                identity: &identity,
                update_id: &format!("mt072-u{seq}"),
                update_seq: seq,
                update_bytes: &bytes,
                update_bytes_ref: &format!(
                    "postgres://kernel_crdt_updates/{crdt_document_id}/mt072-u{seq}/update_bytes"
                ),
                session_id: &format!("SR-MT072-{suffix}"),
                trace_id: &format!("trace-mt072-{suffix}"),
                state_vector_before: &before,
                state_vector_after: &after,
                replay_metadata: CrdtReplayMetadataV1 {
                    replay_order_key: format!("{workspace_id}/{document_id}/{seq:020}"),
                    dependency_update_ids: Vec::new(),
                    encoding: "yjs-update-v1".to_string(),
                    schema_version: "kernel-crdt-update-v1".to_string(),
                },
                event_ledger_event_id: &stored_event.event_id,
            });
            db.append_kernel_crdt_update(record.clone(), bytes)
                .await
                .expect("append update");
            chain.push(record);
        }

        let mut replayed = db
            .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
            .await
            .expect("list persisted updates");
        assert_eq!(replayed.len(), 3);
        // Feed the proof out of order on purpose; it must sort by update_seq.
        replayed.reverse();
        let proof = verify_causal_chain(&replayed).expect("persisted chain proves replay order");
        assert_eq!(proof.steps.len(), 3);
        assert_eq!(proof.final_lamport, 2, "operator advanced twice");
        assert_eq!(
            proof.final_state_vector,
            sv.encode(),
            "proof reconstructs the exact final typed state vector"
        );
        assert_eq!(
            proof.steps[1].advanced_sites,
            vec![model_site.site_id.clone()]
        );
        assert!(proof.steps[0].advanced_sites.contains(&human_site.site_id));
    }
}

mod mt_065_actor_site {
    use super::postgres_or_environment_blocked;
    use handshake_core::kernel::crdt::actor_site::{
        derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdError,
        KnowledgeActorIdV1, KnowledgeActorKind, KNOWLEDGE_ACTOR_IDENT_MAX_LEN,
    };
    use handshake_core::kernel::crdt::identity::validate_crdt_workspace_identity;
    use handshake_core::kernel::crdt::persistence::{
        new_crdt_update_record, sha256_hex, validate_crdt_update_record, CrdtReplayMetadataV1,
        CrdtUpdateRecordInputV1,
    };
    use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn actor_id_round_trips_for_every_actor_kind() {
        for (kind, token) in [
            (KnowledgeActorKind::Operator, "operator"),
            (KnowledgeActorKind::LocalModel, "local_model"),
            (KnowledgeActorKind::CloudModel, "cloud_model"),
            (KnowledgeActorKind::Validator, "validator"),
            (KnowledgeActorKind::System, "system"),
        ] {
            let actor = KnowledgeActorIdV1::new(kind, "lane-7.alpha_B-2").expect("valid actor");
            let canonical = actor.canonical();
            assert_eq!(canonical, format!("{token}:lane-7.alpha_B-2"));
            let parsed = KnowledgeActorIdV1::parse(&canonical).expect("canonical form parses");
            assert_eq!(parsed, actor);
            assert_eq!(parsed.kind(), kind);
            assert_eq!(parsed.kind().as_str(), token);

            // serde round-trip uses the canonical string representation.
            let serialized = serde_json::to_string(&actor).expect("serialize");
            assert_eq!(serialized, format!("\"{canonical}\""));
            let deserialized: KnowledgeActorIdV1 =
                serde_json::from_str(&serialized).expect("deserialize");
            assert_eq!(deserialized, actor);
        }
    }

    #[test]
    fn actor_id_parse_rejects_malformed_inputs() {
        assert!(matches!(
            KnowledgeActorIdV1::parse("no-separator"),
            Err(KnowledgeActorIdError::MissingSeparator { .. })
        ));
        assert!(matches!(
            KnowledgeActorIdV1::parse("martian:probe-1"),
            Err(KnowledgeActorIdError::UnknownActorKind { .. })
        ));
        assert!(matches!(
            KnowledgeActorIdV1::parse("operator:"),
            Err(KnowledgeActorIdError::EmptyIdent)
        ));
        assert!(matches!(
            KnowledgeActorIdV1::parse("operator:has space"),
            Err(KnowledgeActorIdError::IdentBadChar { found: ' ' })
        ));
        assert!(matches!(
            KnowledgeActorIdV1::parse("operator:nested:colon"),
            Err(KnowledgeActorIdError::IdentBadChar { found: ':' })
        ));
        let oversized = format!("system:{}", "x".repeat(KNOWLEDGE_ACTOR_IDENT_MAX_LEN + 1));
        assert!(matches!(
            KnowledgeActorIdV1::parse(&oversized),
            Err(KnowledgeActorIdError::IdentTooLong { .. })
        ));
        // serde path fails closed too.
        assert!(serde_json::from_str::<KnowledgeActorIdV1>("\"martian:probe-1\"").is_err());
    }

    #[test]
    fn site_id_derivation_is_deterministic_and_actor_scoped() {
        let local = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "qwen3-a3")
            .expect("valid actor");
        let cloud = KnowledgeActorIdV1::new(KnowledgeActorKind::CloudModel, "qwen3-a3")
            .expect("valid actor");

        let a = derive_knowledge_site_id("ws-1", "crdt-doc-1", &local);
        let b = derive_knowledge_site_id("ws-1", "crdt-doc-1", &local);
        assert_eq!(a, b, "same inputs must derive the same site identity");
        assert!(a.site_id.starts_with("site-"));
        assert_eq!(a.site_id.len(), "site-".len() + 16);

        // Different actor kind, document, or workspace forks the site id.
        let c = derive_knowledge_site_id("ws-1", "crdt-doc-1", &cloud);
        let d = derive_knowledge_site_id("ws-1", "crdt-doc-2", &local);
        let e = derive_knowledge_site_id("ws-2", "crdt-doc-1", &local);
        for other in [&c, &d, &e] {
            assert_ne!(a.site_id, other.site_id);
            assert_ne!(a.yjs_client_id, other.yjs_client_id);
        }
    }

    #[test]
    fn kernel_actor_mapping_preserves_canonical_id() {
        let validator = KnowledgeActorIdV1::new(KnowledgeActorKind::Validator, "wp-val-1")
            .expect("valid actor");
        match validator.to_kernel_actor() {
            KernelActor::ValidationRunner(id) => assert_eq!(id, "validator:wp-val-1"),
            other => panic!("validator must map to ValidationRunner, got {other:?}"),
        }
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "ilja").expect("valid actor");
        assert!(
            matches!(operator.to_kernel_actor(), KernelActor::Operator(id) if id == "operator:ilja")
        );
        let local = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "m1").expect("actor");
        assert!(matches!(
            local.to_kernel_actor(),
            KernelActor::ModelAdapter(_)
        ));
        assert!(local.kind().is_model());
        assert!(!operator.kind().is_model());
    }

    #[test]
    fn knowledge_crdt_identity_passes_kernel_identity_validation() {
        let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "qwen3-coder")
            .expect("valid actor");
        let identity = knowledge_crdt_identity(
            "ws-knowledge",
            "doc-knowledge",
            "crdt-doc-knowledge",
            "hsk.doc.rich_document@1",
            &actor,
            "trace-mt065",
        );
        validate_crdt_workspace_identity(&identity)
            .expect("derived identity must satisfy kernel CRDT identity validation");
        assert_eq!(identity.actor_id, "local_model:qwen3-coder");
        assert_eq!(identity.actor_kind, "local_model");
        assert!(identity.crdt_site_id.starts_with("site-"));
        let derived = derive_knowledge_site_id("ws-knowledge", "crdt-doc-knowledge", &actor);
        assert_eq!(identity.crdt_site_id, derived.site_id);
        assert_eq!(identity.crdt_client_id, derived.yjs_client_id.to_string());
        assert_eq!(
            identity.authority_links.event_ledger_stream_id,
            "knowledge-crdt:crdt-doc-knowledge"
        );
    }

    /// PostgreSQL proof: a CRDT update attributed through the typed actor /
    /// site model persists to kernel_crdt_updates with its EventLedger
    /// receipt and survives replay listing (actor id and kind intact).
    #[tokio::test]
    async fn typed_actor_update_persists_to_postgres_with_event_receipt() {
        let db = postgres_or_environment_blocked().await;
        let suffix = Uuid::now_v7().simple().to_string();
        let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "mt065-model")
            .expect("valid actor");
        let identity = knowledge_crdt_identity(
            &format!("ws-mt065-{suffix}"),
            &format!("doc-mt065-{suffix}"),
            &format!("crdt-mt065-{suffix}"),
            "hsk.doc.rich_document@1",
            &actor,
            &format!("trace-mt065-{suffix}"),
        );

        let event = NewKernelEvent::builder(
            format!("KTR-MT065-{suffix}"),
            format!("SR-MT065-{suffix}"),
            KernelEventType::KnowledgeCrdtUpdateRecorded,
            actor.to_kernel_actor(),
        )
        .aggregate("knowledge_crdt_document", identity.crdt_document_id.clone())
        .idempotency_key(format!("mt065:{suffix}:update-1"))
        .source_component("knowledge_crdt_model_tests")
        .payload(json!({
            "actor_id": actor.canonical(),
            "site_id": identity.crdt_site_id,
        }))
        .build()
        .expect("valid kernel event");
        let stored_event = db.append_kernel_event(event).await.expect("append event");
        assert_eq!(
            stored_event.event_type,
            KernelEventType::KnowledgeCrdtUpdateRecorded,
            "new event family must round-trip through Postgres storage"
        );

        let update_bytes = b"mt065-update".to_vec();
        let record = new_crdt_update_record(CrdtUpdateRecordInputV1 {
            identity: &identity,
            update_id: "mt065-update-1",
            update_seq: 1,
            update_bytes: &update_bytes,
            update_bytes_ref: &format!(
                "postgres://kernel_crdt_updates/{}/mt065-update-1/update_bytes",
                identity.crdt_document_id
            ),
            session_id: &format!("SR-MT065-{suffix}"),
            trace_id: &format!("trace-mt065-{suffix}"),
            state_vector_before: "hsk-sv1:",
            state_vector_after: &format!("hsk-sv1:{}=1", identity.crdt_site_id),
            replay_metadata: CrdtReplayMetadataV1 {
                replay_order_key: format!(
                    "{}/{}/{:020}",
                    identity.workspace_id, identity.document_id, 1
                ),
                dependency_update_ids: Vec::new(),
                encoding: "yjs-update-v1".to_string(),
                schema_version: "kernel-crdt-update-v1".to_string(),
            },
            event_ledger_event_id: &stored_event.event_id,
        });
        validate_crdt_update_record(&record).expect("typed-actor record validates");
        assert_eq!(record.update_sha256, sha256_hex(&update_bytes));

        db.append_kernel_crdt_update(record.clone(), update_bytes)
            .await
            .expect("append typed-actor CRDT update");

        let replayed = db
            .list_kernel_crdt_updates(
                &identity.workspace_id,
                &identity.document_id,
                &identity.crdt_document_id,
            )
            .await
            .expect("list persisted updates");
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].actor_id, "local_model:mt065-model");
        assert_eq!(replayed[0].actor_kind, "local_model");
        let restored = KnowledgeActorIdV1::parse(&replayed[0].actor_id)
            .expect("persisted actor id parses back into the typed model");
        assert_eq!(restored, actor);
    }
}
