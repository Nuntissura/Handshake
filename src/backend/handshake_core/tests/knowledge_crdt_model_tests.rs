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
