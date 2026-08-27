//! MT-136 exhaustive `KnowledgeStore` proof against the real embedded RocksDB engine.
//!
//! This module deliberately invokes every method with `KnowledgeStore`-qualified syntax so
//! coverage can be checked mechanically against the trait declaration. The lifecycle is ordered
//! around real foreign-key/reference dependencies, then the same store is closed and reopened to
//! prove that each durable record family survived an engine restart.

use super::{
    knowledge::measure_knowledge_store_queries, mt136_proof_harness::embedded_proof_backend,
    SurrealDatabase, SurrealStorage,
};
use crate::kernel::context_bundle::ContextBundle;
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::*;
use crate::storage::{
    Database, NewDocument, NewWorkspace, StorageError, StorageResult, WriteContext,
};
use serde_json::{json, Value};
use surrealdb::types::RecordId;

fn rich_content(text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": text }]
        }]
    })
}

fn receipt_event() -> NewKernelEvent {
    NewKernelEvent::builder(
        "KTR-mt136-knowledge-proof",
        "session-mt136-knowledge-proof",
        KernelEventType::ValidationRecorded,
        KernelActor::System("mt136-knowledge-proof".to_owned()),
    )
    .aggregate("knowledge_store", "mt136-full-surface")
    .idempotency_key("mt136-knowledge-store-receipt")
    .correlation_id("mt136-knowledge-proof")
    .source_component("mt136_knowledge_surface_proof")
    .payload(json!({ "proof": "real embedded RocksDB" }))
    .build()
    .expect("valid MT-136 knowledge receipt")
}

fn conflict_receipt_event(conflict_id: &str) -> NewKernelEvent {
    NewKernelEvent::builder(
        "KTR-mt136-knowledge-conflict-proof",
        "session-mt136-knowledge-proof",
        KernelEventType::ValidationRecorded,
        KernelActor::System("mt136-knowledge-proof".to_owned()),
    )
    .aggregate("knowledge_claim_conflict", conflict_id)
    .idempotency_key(format!("mt136-knowledge-conflict-receipt-{conflict_id}"))
    .correlation_id("mt136-knowledge-proof")
    .source_component("mt136_knowledge_surface_proof")
    .payload(json!({ "proof": "knowledge claim conflict resolution" }))
    .build()
    .expect("valid MT-136 knowledge conflict receipt")
}

fn passage(
    workspace_id: &str,
    text: &str,
    source_id: &str,
    claim_id: &str,
    span_id: &str,
    index_run_id: &str,
) -> NewKnowledgeMemoryPassage {
    NewKnowledgeMemoryPassage {
        workspace_id: workspace_id.to_owned(),
        passage_text: text.to_owned(),
        token_count: Some(12),
        ocr_transcript_metadata: None,
        extraction_confidence: 0.91,
        ranking_features: json!({ "semantic": 0.8, "lexical": 0.7 }),
        retrieval_mode: KnowledgeRetrievalMode::HybridRag,
        compaction_policy: KnowledgeCompactionPolicy::Keep,
        failure_receipt_event_id: None,
        derived_in_run: Some(index_run_id.to_owned()),
        evidence: vec![
            KnowledgePassageEvidenceRef::Source {
                source_id: source_id.to_owned(),
            },
            KnowledgePassageEvidenceRef::Claim {
                claim_id: claim_id.to_owned(),
            },
            KnowledgePassageEvidenceRef::Span {
                span_id: span_id.to_owned(),
            },
        ],
    }
}

fn assert_conflict<T>(result: StorageResult<T>) {
    assert!(matches!(result, Err(StorageError::Conflict(_))));
}

fn assert_validation<T>(result: StorageResult<T>) {
    assert!(matches!(result, Err(StorageError::Validation(_))));
}

async fn force_rich_document_version(
    storage: &SurrealStorage,
    rich_document_id: &str,
    version: i64,
) -> StorageResult<()> {
    let document = RecordId::new("knowledge_rich_documents", rich_document_id.to_owned());
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .client
                    .query("UPDATE $document SET doc_version = $version;")
                    .bind(("document", document))
                    .bind(("version", version))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await
        .map_err(|error| StorageError::Database(error.to_string()))
}

async fn seed_knowledge_code_file(
    storage: &SurrealStorage,
    code_file_id: &str,
    workspace_id: &str,
    source_id: &str,
    file_entity_id: &str,
    index_run_id: &str,
    receipt_event_id: &str,
    language: &str,
    symbols_indexed: i64,
) -> StorageResult<()> {
    let record = RecordId::new("knowledge_code_files", code_file_id.to_owned());
    let workspace = RecordId::new("workspaces", workspace_id.to_owned());
    let source = RecordId::new("knowledge_sources", source_id.to_owned());
    let file_entity = RecordId::new("knowledge_entities", file_entity_id.to_owned());
    let index_run = RecordId::new("knowledge_index_runs", index_run_id.to_owned());
    let receipt = RecordId::new("kernel_event_ledger", receipt_event_id.to_owned());
    let code_file_id = code_file_id.to_owned();
    let language = language.to_owned();
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .client
                    .query(
                        "CREATE $record CONTENT { code_file_id: $code_file_id, workspace_id: $workspace, source_id: $source, file_entity_id: $file_entity, language: $language, indexed_content_hash: $content_hash, parser_version: 'mt136-tree-sitter-v1', parse_status: 'parsed', stale: false, symbols_indexed: $symbols_indexed, edges_indexed: 1, failure_detail: NONE, last_indexed_in_run: $index_run, last_index_receipt_event_id: $receipt };",
                    )
                    .bind(("record", record))
                    .bind(("code_file_id", code_file_id))
                    .bind(("workspace", workspace))
                    .bind(("source", source))
                    .bind(("file_entity", file_entity))
                    .bind(("language", language))
                    .bind(("content_hash", "c".repeat(64)))
                    .bind(("symbols_indexed", symbols_indexed))
                    .bind(("index_run", index_run))
                    .bind(("receipt", receipt))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await
        .map_err(|error| StorageError::Database(error.to_string()))
}

async fn all_knowledge_store_methods_use_real_rocksdb_and_survive_reopen() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let setup_database = backend.database.clone();
    let storage = backend.storage.clone();
    let ctx = WriteContext::human(Some("mt136-knowledge-proof".to_owned()));
    let workspace = setup_database
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: "MT-136 KnowledgeStore RocksDB proof".to_owned(),
            },
        )
        .await?;
    let legacy_document = setup_database
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace.id.clone(),
                title: "MT-136 legacy document anchor".to_owned(),
            },
        )
        .await?;
    let database = SurrealDatabase::new(storage.clone());
    let receipt = database.append_kernel_event(receipt_event()).await?;

    // MT-049: schema registry and namespace audit.
    let registry = KnowledgeStore::list_knowledge_schema_registry(&database).await?;
    assert_eq!(registry.len(), 62);
    assert!(registry.iter().any(|row| {
        row.family_key == "schema_registry"
            && row.table_name == "knowledge_schema_registry"
            && row.schema_source == "storage/surreal/schema.surql"
            && row.mt_id == "MT-049"
    }));
    assert!(registry.iter().any(|row| {
        row.family_key == "rich_document_loom_projection_0343_state"
            && row.table_name == "knowledge_rich_document_loom_projection_0343_state"
            && row.record_family == "Support"
            && row.authority_class == KnowledgeAuthorityClass::Support
            && row.schema_source == "storage/surreal/schema.surql"
            && row.wp_id == "WP-KERNEL-012"
            && row.mt_id == "MT-032"
    }));
    let audit = KnowledgeStore::audit_knowledge_namespace(&database).await?;
    assert!(audit.is_sound(), "knowledge namespace drift: {audit:?}");

    // MT-050/051: root and source lifecycle.
    let root = KnowledgeStore::create_knowledge_source_root(
        &database,
        NewKnowledgeSourceRoot {
            workspace_id: workspace.id.clone(),
            display_name: "src root".to_owned(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: "src".to_owned(),
            allowlist_policy: json!({ "include": ["**/*.rs"] }),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        },
    )
    .await?;
    assert_eq!(
        KnowledgeStore::get_knowledge_source_root(&database, &root.root_id)
            .await?
            .map(|row| row.root_id),
        Some(root.root_id.clone())
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_source_roots(&database, &workspace.id)
            .await?
            .len(),
        1
    );
    let paused_root = KnowledgeStore::set_knowledge_root_eligibility(
        &database,
        &root.root_id,
        KnowledgeIndexingEligibility::Paused,
    )
    .await?;
    assert_eq!(
        paused_root.indexing_eligibility,
        KnowledgeIndexingEligibility::Paused
    );

    let source = KnowledgeStore::upsert_knowledge_source(
        &database,
        NewKnowledgeSource {
            workspace_id: workspace.id.clone(),
            root_id: Some(root.root_id.clone()),
            source_kind: KnowledgeSourceKind::File,
            relative_path: Some("storage/knowledge-proof.rs".to_owned()),
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: "1".repeat(64),
            size_bytes: Some(128),
            provenance: json!({ "proof": "MT-136" }),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        },
    )
    .await?;
    assert_eq!(
        KnowledgeStore::get_knowledge_source(&database, &source.source_id)
            .await?
            .map(|row| row.source_id),
        Some(source.source_id.clone())
    );
    assert!(KnowledgeStore::get_knowledge_source_by_document_id(
        &database,
        &workspace.id,
        "missing-rich-document"
    )
    .await?
    .is_none());
    assert_eq!(
        KnowledgeStore::list_knowledge_sources_for_root(&database, &root.root_id)
            .await?
            .len(),
        1
    );
    assert!(
        KnowledgeStore::mark_knowledge_source_stale(&database, &source.source_id)
            .await?
            .stale
    );
    let source = KnowledgeStore::record_knowledge_source_index_receipt(
        &database,
        &source.source_id,
        KnowledgeParserStatus::Parsed,
        KnowledgeExtractionStatus::Extracted,
        &receipt.event_id,
    )
    .await?;
    assert!(!source.stale);
    let cross_source = KnowledgeStore::upsert_knowledge_source(
        &database,
        NewKnowledgeSource {
            workspace_id: workspace.id.clone(),
            root_id: Some(root.root_id.clone()),
            source_kind: KnowledgeSourceKind::File,
            relative_path: Some("storage/cross-source-proof.ts".to_owned()),
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: "4".repeat(64),
            size_bytes: Some(96),
            provenance: json!({ "proof": "MT-136 cross-source" }),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        },
    )
    .await?;

    // MT-052: guarded index-run lifecycle.
    let index_run = KnowledgeStore::start_knowledge_index_run(
        &database,
        NewKnowledgeIndexRun {
            workspace_id: workspace.id.clone(),
            root_id: Some(root.root_id.clone()),
            scope: json!({ "source_id": source.source_id }),
            actor_kind: "system".to_owned(),
            actor_id: "mt136-proof".to_owned(),
            worktree_id: Some("wtc-native-editors-v1".to_owned()),
            start_receipt_event_id: None,
        },
    )
    .await?;
    assert!(
        KnowledgeStore::get_knowledge_index_run(&database, &index_run.index_run_id)
            .await?
            .is_some()
    );
    KnowledgeStore::checkpoint_knowledge_index_run(
        &database,
        &index_run.index_run_id,
        json!({ "cursor": 1 }),
    )
    .await?;
    let counts = KnowledgeIndexRunCounts {
        sources_seen: 1,
        sources_indexed: 1,
        spans_extracted: 2,
        entities_detected: 3,
        edges_written: 1,
        claims_written: 3,
    };
    KnowledgeStore::finish_knowledge_index_run(
        &database,
        &index_run.index_run_id,
        KnowledgeIndexRunOutcome::Completed { counts },
        None,
    )
    .await?;
    assert_conflict(
        KnowledgeStore::finish_knowledge_index_run(
            &database,
            &index_run.index_run_id,
            KnowledgeIndexRunOutcome::Cancelled { counts },
            None,
        )
        .await,
    );

    // MT-055/053: citeable spans and stable entity identities.
    let span_a = KnowledgeStore::create_knowledge_span(
        &database,
        NewKnowledgeSpan {
            source_id: source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Ast,
            range_start: 0,
            range_end: 32,
            line_start: Some(1),
            line_end: Some(2),
            section_path: Some("proof::first".to_owned()),
            content_sha256: "2".repeat(64),
            parser_version: "tree-sitter-proof-v1".to_owned(),
            extraction_receipt_event_id: None,
            index_run_id: Some(index_run.index_run_id.clone()),
            display_snippet: Some("fn first()".to_owned()),
        },
    )
    .await?;
    let span_b = KnowledgeStore::create_knowledge_span(
        &database,
        NewKnowledgeSpan {
            source_id: source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Ast,
            range_start: 33,
            range_end: 64,
            line_start: Some(3),
            line_end: Some(4),
            section_path: Some("proof::second".to_owned()),
            content_sha256: "3".repeat(64),
            parser_version: "tree-sitter-proof-v1".to_owned(),
            extraction_receipt_event_id: None,
            index_run_id: Some(index_run.index_run_id.clone()),
            display_snippet: Some("fn second()".to_owned()),
        },
    )
    .await?;
    let cross_span = KnowledgeStore::create_knowledge_span(
        &database,
        NewKnowledgeSpan {
            source_id: cross_source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Ast,
            range_start: 0,
            range_end: 48,
            line_start: Some(1),
            line_end: Some(3),
            section_path: Some("proof::cross_source".to_owned()),
            content_sha256: "5".repeat(64),
            parser_version: "tree-sitter-proof-v1".to_owned(),
            extraction_receipt_event_id: None,
            index_run_id: Some(index_run.index_run_id.clone()),
            display_snippet: Some("export function crossSource()".to_owned()),
        },
    )
    .await?;
    assert!(
        KnowledgeStore::get_knowledge_span(&database, &span_a.span_id)
            .await?
            .is_some()
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_spans_for_source(&database, &source.source_id)
            .await?
            .len(),
        2
    );

    let entity_a = KnowledgeStore::upsert_knowledge_entity(
        &database,
        NewKnowledgeEntity {
            workspace_id: workspace.id.clone(),
            entity_kind: KnowledgeEntityKind::Symbol,
            entity_key: "proof::first".to_owned(),
            display_name: "first".to_owned(),
            detection_provenance: json!({ "extractor": "mt136" }),
            primary_source_id: Some(source.source_id.clone()),
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![span_a.span_id.clone()],
        },
    )
    .await?;
    let entity_b = KnowledgeStore::upsert_knowledge_entity(
        &database,
        NewKnowledgeEntity {
            workspace_id: workspace.id.clone(),
            entity_kind: KnowledgeEntityKind::Symbol,
            entity_key: "proof::second".to_owned(),
            display_name: "second".to_owned(),
            detection_provenance: json!({ "extractor": "mt136" }),
            primary_source_id: Some(cross_source.source_id.clone()),
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![cross_span.span_id.clone()],
        },
    )
    .await?;
    let retire_candidate = KnowledgeStore::upsert_knowledge_entity(
        &database,
        NewKnowledgeEntity {
            workspace_id: workspace.id.clone(),
            entity_kind: KnowledgeEntityKind::Concept,
            entity_key: "retire-candidate".to_owned(),
            display_name: "retire candidate".to_owned(),
            detection_provenance: json!({ "extractor": "mt136" }),
            primary_source_id: Some(source.source_id.clone()),
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![span_a.span_id.clone()],
        },
    )
    .await?;
    assert!(
        KnowledgeStore::get_knowledge_entity(&database, &entity_a.entity_id)
            .await?
            .is_some()
    );
    assert_eq!(
        KnowledgeStore::get_knowledge_entity_by_identity(
            &database,
            &workspace.id,
            KnowledgeEntityKind::Symbol,
            "proof::first",
        )
        .await?
        .map(|row| row.entity_id),
        Some(entity_a.entity_id.clone())
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_entities_by_kind(
            &database,
            &workspace.id,
            KnowledgeEntityKind::Symbol,
        )
        .await?
        .len(),
        2
    );

    let primary_code_file_id = "KCF-mt136-primary".to_owned();
    let cross_code_file_id = "KCF-mt136-cross".to_owned();
    seed_knowledge_code_file(
        &storage,
        &primary_code_file_id,
        &workspace.id,
        &source.source_id,
        &entity_a.entity_id,
        &index_run.index_run_id,
        &receipt.event_id,
        "rust",
        7,
    )
    .await?;
    seed_knowledge_code_file(
        &storage,
        &cross_code_file_id,
        &workspace.id,
        &cross_source.source_id,
        &entity_b.entity_id,
        &index_run.index_run_id,
        &receipt.event_id,
        "typescript",
        3,
    )
    .await?;
    let primary_code_file =
        KnowledgeStore::get_knowledge_code_file_by_source(&database, &source.source_id)
            .await?
            .ok_or(StorageError::NotFound("MT-136 primary knowledge code file"))?;
    assert_eq!(primary_code_file.code_file_id, primary_code_file_id);
    assert_eq!(
        primary_code_file.file_entity_id,
        Some(entity_a.entity_id.clone())
    );
    assert_eq!(primary_code_file.language, KnowledgeCodeLanguage::Rust);
    assert_eq!(
        primary_code_file.parse_status,
        KnowledgeCodeParseStatus::Parsed
    );
    assert_eq!(primary_code_file.symbols_indexed, 7);
    assert_eq!(
        database
            .list_knowledge_code_files(&workspace.id)
            .await?
            .len(),
        2
    );
    let wiki_code_inputs = database.list_wiki_code_file_inputs(&workspace.id).await?;
    assert_eq!(wiki_code_inputs.len(), 2);
    assert_eq!(
        wiki_code_inputs[0].relative_path,
        "storage/cross-source-proof.ts"
    );
    assert_eq!(wiki_code_inputs[0].source_id, cross_source.source_id);
    assert_eq!(
        wiki_code_inputs[1].relative_path,
        "storage/knowledge-proof.rs"
    );
    assert_eq!(wiki_code_inputs[1].source_id, source.source_id);
    let filtered_wiki_inputs = database
        .list_wiki_code_file_inputs_by_sources(
            &workspace.id,
            std::slice::from_ref(&cross_source.source_id),
        )
        .await?;
    assert_eq!(filtered_wiki_inputs.len(), 1);
    assert_eq!(filtered_wiki_inputs[0].code_file_id, cross_code_file_id);
    assert!(database
        .list_wiki_code_file_inputs_by_sources(&workspace.id, &[])
        .await?
        .is_empty());
    let primary_code_file = database
        .mark_knowledge_code_file_stale(&primary_code_file_id)
        .await?;
    assert!(primary_code_file.stale);

    // MT-136 code-navigation hardening: prove that name/path/prefix matching is
    // executed by the bounded embedded query without approximate over-fetch.
    let nav_find = KnowledgeStore::upsert_knowledge_entity(
        &database,
        NewKnowledgeEntity {
            workspace_id: workspace.id.clone(),
            entity_kind: KnowledgeEntityKind::Symbol,
            entity_key: "rust:src/lib.rs#Alpha::find~as:Trait".to_owned(),
            display_name: "find".to_owned(),
            detection_provenance: json!({ "extractor": "mt136-nav-query" }),
            primary_source_id: Some(source.source_id.clone()),
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![span_a.span_id.clone()],
        },
    )
    .await?;
    KnowledgeStore::upsert_knowledge_entity(
        &database,
        NewKnowledgeEntity {
            workspace_id: workspace.id.clone(),
            entity_kind: KnowledgeEntityKind::Symbol,
            entity_key: "typescript:src/lib.rs#findings".to_owned(),
            display_name: "findings".to_owned(),
            detection_provenance: json!({ "extractor": "mt136-nav-query" }),
            primary_source_id: Some(source.source_id.clone()),
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![span_a.span_id.clone()],
        },
    )
    .await?;
    KnowledgeStore::upsert_knowledge_entity(
        &database,
        NewKnowledgeEntity {
            workspace_id: workspace.id.clone(),
            entity_kind: KnowledgeEntityKind::Symbol,
            entity_key: "rust:src/lib.rs.bak#find".to_owned(),
            display_name: "find".to_owned(),
            detection_provenance: json!({ "extractor": "mt136-nav-query" }),
            primary_source_id: Some(source.source_id.clone()),
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![span_a.span_id.clone()],
        },
    )
    .await?;
    let exact_nav = database
        .lookup_knowledge_code_symbols(&workspace.id, Some("find"), None, Some("src/lib.rs"), 500)
        .await?;
    assert_eq!(exact_nav.len(), 1);
    assert_eq!(exact_nav[0].entity_id, nav_find.entity_id);
    let prefix_nav = database
        .lookup_knowledge_code_symbols(&workspace.id, None, Some("FiN"), Some("src/lib.rs"), 500)
        .await?;
    assert_eq!(prefix_nav.len(), 2);
    let bounded_nav = database
        .lookup_knowledge_code_symbols(&workspace.id, None, Some("find"), Some("src/lib.rs"), 1)
        .await?;
    assert_eq!(bounded_nav.len(), 1);

    let (small_wiki_entities, small_wiki_entity_query_count) =
        measure_knowledge_store_queries(|| {
            database.list_wiki_source_entities_with_spans(
                &workspace.id,
                &source.source_id,
                KnowledgeEntityKind::Symbol,
            )
        })
        .await;
    assert!(!small_wiki_entities?.is_empty());
    assert_eq!(small_wiki_entity_query_count, 3);

    // Fixed-query wiki joins: a fixture large enough to make a per-entity or
    // per-span query loop material, while all entities share a bounded span
    // set. The implementation must stitch the three bulk result sets without
    // changing latest-span or entity-key ordering semantics.
    let mut batch_entity_ids = Vec::new();
    for ordinal in 0..32 {
        let evidence_span_id = if ordinal % 2 == 0 {
            span_a.span_id.clone()
        } else {
            span_b.span_id.clone()
        };
        let entity = KnowledgeStore::upsert_knowledge_entity(
            &database,
            NewKnowledgeEntity {
                workspace_id: workspace.id.clone(),
                entity_kind: KnowledgeEntityKind::Symbol,
                entity_key: format!("proof::batch::{ordinal:02}"),
                display_name: format!("batch_{ordinal:02}"),
                detection_provenance: json!({ "extractor": "mt136-wiki-bulk-proof" }),
                primary_source_id: Some(source.source_id.clone()),
                detected_in_run: Some(index_run.index_run_id.clone()),
                evidence_span_ids: vec![evidence_span_id],
            },
        )
        .await?;
        batch_entity_ids.push(entity.entity_id);
    }
    let (batch_with_spans, large_wiki_entity_query_count) = measure_knowledge_store_queries(|| {
        database.list_wiki_source_entities_with_spans(
            &workspace.id,
            &source.source_id,
            KnowledgeEntityKind::Symbol,
        )
    })
    .await;
    assert_eq!(large_wiki_entity_query_count, small_wiki_entity_query_count);
    let batch_with_spans = batch_with_spans?
        .into_iter()
        .filter(|row| row.entity.entity_key.starts_with("proof::batch::"))
        .collect::<Vec<_>>();
    assert_eq!(batch_with_spans.len(), batch_entity_ids.len());
    assert!(batch_with_spans
        .windows(2)
        .all(|pair| pair[0].entity.entity_key < pair[1].entity.entity_key));
    let (small_states, small_state_query_count) =
        measure_knowledge_store_queries(|| database.get_wiki_entity_states(&batch_entity_ids[..1]))
            .await;
    assert_eq!(small_states?.len(), 1);
    assert_eq!(small_state_query_count, 2);
    let (batch_states, large_state_query_count) =
        measure_knowledge_store_queries(|| database.get_wiki_entity_states(&batch_entity_ids))
            .await;
    assert_eq!(large_state_query_count, small_state_query_count);
    let batch_states = batch_states?;
    assert_eq!(batch_states.len(), batch_entity_ids.len());
    assert!(batch_states
        .iter()
        .all(|(_, hash)| hash.as_deref() == Some(source.content_hash.as_str())));
    assert!(batch_states
        .windows(2)
        .all(|pair| pair[0].0.entity_id < pair[1].0.entity_id));

    assert_eq!(
        KnowledgeStore::list_knowledge_entity_span_ids(&database, &entity_a.entity_id).await?,
        vec![span_a.span_id.clone()]
    );
    KnowledgeStore::replace_knowledge_entity_spans_for_source_kind(
        &database,
        &entity_a.entity_id,
        &source.source_id,
        KnowledgeSpanKind::Ast,
        &[span_b.span_id.clone()],
        Some(&index_run.index_run_id),
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_entity_span_ids(&database, &entity_a.entity_id).await?,
        vec![span_b.span_id.clone()]
    );
    assert_eq!(
        KnowledgeStore::retire_knowledge_entity(&database, &retire_candidate.entity_id)
            .await?
            .lifecycle_state,
        KnowledgeEntityLifecycle::Retired
    );

    // MT-054: evidence-required stable graph edge and lifecycle guard.
    assert_validation(
        KnowledgeStore::upsert_knowledge_edge(
            &database,
            NewKnowledgeEdge {
                workspace_id: workspace.id.clone(),
                edge_type: KnowledgeEdgeType::References,
                source_entity_id: entity_a.entity_id.clone(),
                target_entity_id: entity_b.entity_id.clone(),
                extractor_version: "mt136-v1".to_owned(),
                confidence: 0.9,
                detected_in_run: Some(index_run.index_run_id.clone()),
                evidence_span_ids: Vec::new(),
            },
        )
        .await,
    );
    let edge = KnowledgeStore::upsert_knowledge_edge(
        &database,
        NewKnowledgeEdge {
            workspace_id: workspace.id.clone(),
            edge_type: KnowledgeEdgeType::References,
            source_entity_id: entity_a.entity_id.clone(),
            target_entity_id: entity_b.entity_id.clone(),
            extractor_version: "mt136-v1".to_owned(),
            confidence: 0.9,
            detected_in_run: Some(index_run.index_run_id.clone()),
            evidence_span_ids: vec![span_a.span_id.clone()],
        },
    )
    .await?;
    let cross_source_edges = database
        .list_wiki_cross_source_code_edges(&workspace.id, 100)
        .await?;
    assert_eq!(cross_source_edges.len(), 1);
    assert_eq!(
        cross_source_edges[0].edge_type,
        KnowledgeEdgeType::References
    );
    assert_eq!(
        cross_source_edges[0].from_source_id.as_str(),
        source.source_id.as_str()
    );
    assert_eq!(
        cross_source_edges[0].to_source_id.as_str(),
        cross_source.source_id.as_str()
    );
    assert!(KnowledgeStore::get_knowledge_edge(&database, &edge.edge_id)
        .await?
        .is_some());
    assert!(KnowledgeStore::get_knowledge_edge_by_relationship_id(
        &database,
        &workspace.id,
        &edge.relationship_id,
    )
    .await?
    .is_some());
    assert_eq!(
        KnowledgeStore::list_knowledge_edges_for_entity(&database, &entity_a.entity_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_edge_span_ids(&database, &edge.edge_id).await?,
        vec![span_a.span_id.clone()]
    );
    KnowledgeStore::set_knowledge_edge_lifecycle(
        &database,
        &edge.edge_id,
        KnowledgeEdgeLifecycle::Conflicted,
        Some(json!({ "reason": "proof" })),
    )
    .await?;
    let edge = KnowledgeStore::set_knowledge_edge_lifecycle(
        &database,
        &edge.edge_id,
        KnowledgeEdgeLifecycle::Active,
        None,
    )
    .await?;

    // MT-056: evidence-required claims, guarded transitions, conflicts and resolution.
    let new_claim = |text: &str, subject_entity_id: &str, span_id: &str| NewKnowledgeClaim {
        workspace_id: workspace.id.clone(),
        claim_kind: KnowledgeClaimKind::ProductBehavior,
        claim_text: text.to_owned(),
        subject_entity_id: Some(subject_entity_id.to_owned()),
        temporal_qualifier: None,
        granularity_qualifier: Some("method".to_owned()),
        confidence: 0.85,
        proposed_in_run: Some(index_run.index_run_id.clone()),
        evidence_span_ids: vec![span_id.to_owned()],
    };
    assert_validation(
        KnowledgeStore::create_knowledge_claim(
            &database,
            NewKnowledgeClaim {
                evidence_span_ids: Vec::new(),
                ..new_claim(
                    "invalid evidence-free claim",
                    &entity_a.entity_id,
                    &span_a.span_id,
                )
            },
        )
        .await,
    );
    let claim_a = KnowledgeStore::create_knowledge_claim(
        &database,
        new_claim("first behavior", &entity_a.entity_id, &span_a.span_id),
    )
    .await?;
    let claim_b = KnowledgeStore::create_knowledge_claim(
        &database,
        new_claim("conflicting behavior", &entity_b.entity_id, &span_b.span_id),
    )
    .await?;
    assert!(
        KnowledgeStore::get_knowledge_claim(&database, &claim_a.claim_id)
            .await?
            .is_some()
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_claim_span_ids(&database, &claim_a.claim_id).await?,
        vec![span_a.span_id.clone()]
    );
    assert_conflict(
        KnowledgeStore::transition_knowledge_claim(
            &database,
            &claim_a.claim_id,
            KnowledgeClaimState::Proposed,
            None,
            None,
        )
        .await,
    );
    let conflict = KnowledgeStore::record_knowledge_claim_conflict(
        &database,
        &claim_a.claim_id,
        &claim_b.claim_id,
        "mutually exclusive proof claims",
        Some(&index_run.index_run_id),
    )
    .await?;
    let conflict_receipt = database
        .append_kernel_event(conflict_receipt_event(&conflict.conflict_id))
        .await?;
    let conflict = KnowledgeStore::resolve_knowledge_claim_conflict(
        &database,
        &conflict.conflict_id,
        &conflict_receipt.event_id,
    )
    .await?;
    assert!(conflict.resolved_at.is_some());
    assert_eq!(
        KnowledgeStore::list_knowledge_claim_conflicts(&database, &claim_a.claim_id)
            .await?
            .len(),
        1
    );
    let claim_a = KnowledgeStore::transition_knowledge_claim(
        &database,
        &claim_a.claim_id,
        KnowledgeClaimState::Accepted,
        None,
        Some(&conflict_receipt.event_id),
    )
    .await?;

    // MT-057/062: passage lineage, compaction, replay and divergent-key guards.
    let durable_passage = KnowledgeStore::create_knowledge_memory_passage(
        &database,
        passage(
            &workspace.id,
            "durable non-idempotent passage",
            &source.source_id,
            &claim_a.claim_id,
            &span_a.span_id,
            &index_run.index_run_id,
        ),
    )
    .await?;
    assert!(
        KnowledgeStore::get_knowledge_memory_passage(&database, &durable_passage.passage_id)
            .await?
            .is_some()
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_passage_evidence(&database, &durable_passage.passage_id)
            .await?
            .len(),
        3
    );
    let durable_passage = KnowledgeStore::set_knowledge_passage_compaction(
        &database,
        &durable_passage.passage_id,
        KnowledgeCompactionPolicy::Compactable,
        true,
    )
    .await?;

    let idempotent_passage_payload = passage(
        &workspace.id,
        "idempotent passage",
        &source.source_id,
        &claim_a.claim_id,
        &span_b.span_id,
        &index_run.index_run_id,
    );
    let (first_passage_write, second_passage_write) = tokio::join!(
        KnowledgeStore::create_knowledge_memory_passage_idempotent(
            &database,
            "mt136-passage-idempotency",
            idempotent_passage_payload.clone(),
        ),
        KnowledgeStore::create_knowledge_memory_passage_idempotent(
            &database,
            "mt136-passage-idempotency",
            idempotent_passage_payload,
        ),
    );
    let mut passage_writes = vec![first_passage_write?, second_passage_write?];
    passage_writes.sort_by_key(|write| write.replayed);
    let first_idempotent_passage = &passage_writes[0];
    let replayed_passage = &passage_writes[1];
    assert!(!first_idempotent_passage.replayed);
    assert!(replayed_passage.replayed);
    assert_eq!(
        first_idempotent_passage.value.passage_id,
        replayed_passage.value.passage_id
    );
    assert_conflict(
        KnowledgeStore::create_knowledge_memory_passage_idempotent(
            &database,
            "mt136-passage-idempotency",
            passage(
                &workspace.id,
                "divergent passage",
                &source.source_id,
                &claim_a.claim_id,
                &span_b.span_id,
                &index_run.index_run_id,
            ),
        )
        .await,
    );

    // MT-058: projection rebuild lifecycle plus regenerable deletion.
    let projection = KnowledgeStore::upsert_knowledge_wiki_projection(
        &database,
        NewKnowledgeWikiProjection {
            workspace_id: workspace.id.clone(),
            projection_kind: KnowledgeProjectionKind::WikiPage,
            title: "MT-136 disposable projection".to_owned(),
            source_records: json!([{ "source_id": source.source_id }]),
            rendered_content: "stale".to_owned(),
            staleness_hash: "4".repeat(64),
        },
    )
    .await?;
    assert!(
        KnowledgeStore::get_knowledge_wiki_projection(&database, &projection.projection_id)
            .await?
            .is_some()
    );
    KnowledgeStore::set_knowledge_projection_rebuild_status(
        &database,
        &projection.projection_id,
        KnowledgeRebuildStatus::Rebuilding,
    )
    .await?;
    KnowledgeStore::mark_knowledge_projection_rebuilt(
        &database,
        &projection.projection_id,
        &"5".repeat(64),
        "fresh",
        None,
    )
    .await?;
    let durable_projection = KnowledgeStore::upsert_knowledge_wiki_projection(
        &database,
        NewKnowledgeWikiProjection {
            workspace_id: workspace.id.clone(),
            projection_kind: KnowledgeProjectionKind::OperatorSummary,
            title: "MT-136 durable projection".to_owned(),
            source_records: json!([{ "claim_id": claim_a.claim_id }]),
            rendered_content: "durable".to_owned(),
            staleness_hash: "6".repeat(64),
        },
    )
    .await?;
    KnowledgeStore::delete_knowledge_wiki_projection(&database, &projection.projection_id).await?;
    assert!(
        KnowledgeStore::get_knowledge_wiki_projection(&database, &projection.projection_id)
            .await?
            .is_none()
    );

    // MT-059/145/152/153/155/156/157/255: rich-document authority and projections.
    let rich_document = KnowledgeStore::create_knowledge_rich_document(
        &database,
        NewKnowledgeRichDocument {
            workspace_id: workspace.id.clone(),
            document_id: Some(legacy_document.id.clone()),
            title: "MT-136 anchored rich document".to_owned(),
            schema_version: "hsk_richdoc_v1".to_owned(),
            content_json: rich_content("version-one"),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
            project_ref: Some("project-proof".to_owned()),
            folder_ref: Some("folder-proof".to_owned()),
            authority_label: Some("promoted".to_owned()),
            owner_actor_kind: Some("operator".to_owned()),
            owner_actor_id: Some("mt136".to_owned()),
        },
    )
    .await?;
    KnowledgeStore::upsert_knowledge_source(
        &database,
        NewKnowledgeSource {
            workspace_id: workspace.id.clone(),
            root_id: None,
            source_kind: KnowledgeSourceKind::RichDocument,
            relative_path: None,
            asset_id: None,
            loom_block_id: None,
            document_id: Some(legacy_document.id.clone()),
            content_hash: rich_document.content_sha256.clone(),
            size_bytes: None,
            provenance: json!({ "rich_document_id": rich_document.rich_document_id }),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        },
    )
    .await?;
    let absent_payload = NewKnowledgeRichDocument {
        workspace_id: workspace.id.clone(),
        title: "MT-136 create-if-absent".to_owned(),
        schema_version: "hsk_richdoc_v1".to_owned(),
        content_json: rich_content("create-if-absent"),
        ..Default::default()
    };
    let (created_if_absent, created) =
        KnowledgeStore::create_knowledge_rich_document_if_title_absent(
            &database,
            absent_payload.clone(),
        )
        .await?;
    assert!(created);
    let (same_if_absent, created) =
        KnowledgeStore::create_knowledge_rich_document_if_title_absent(&database, absent_payload)
            .await?;
    assert!(!created);
    assert_eq!(
        created_if_absent.rich_document_id,
        same_if_absent.rich_document_id
    );
    let overflow_document = KnowledgeStore::create_knowledge_rich_document(
        &database,
        NewKnowledgeRichDocument {
            workspace_id: workspace.id.clone(),
            title: "MT-136 version overflow guard".to_owned(),
            schema_version: "hsk_richdoc_v1".to_owned(),
            content_json: rich_content("must-not-change"),
            ..Default::default()
        },
    )
    .await?;
    force_rich_document_version(&storage, &overflow_document.rich_document_id, i64::MAX).await?;
    assert_validation(
        KnowledgeStore::save_knowledge_rich_document_version(
            &database,
            &overflow_document.rich_document_id,
            i64::MAX,
            rich_content("overflowed"),
            None,
            None,
            None,
        )
        .await,
    );
    assert_validation(
        KnowledgeStore::save_knowledge_rich_document_version_idempotent(
            &database,
            "mt136-rich-save-overflow",
            &overflow_document.rich_document_id,
            i64::MAX,
            rich_content("overflowed-idempotent"),
            None,
            None,
            None,
        )
        .await,
    );
    assert_eq!(
        KnowledgeStore::get_knowledge_rich_document(
            &database,
            &overflow_document.rich_document_id,
        )
        .await?
        .expect("overflow proof document remains readable")
        .doc_version,
        i64::MAX
    );
    assert_eq!(
        KnowledgeStore::count_knowledge_rich_document_versions(
            &database,
            &overflow_document.rich_document_id,
        )
        .await?,
        1
    );
    assert!(KnowledgeStore::get_knowledge_rich_document(
        &database,
        &rich_document.rich_document_id
    )
    .await?
    .is_some());
    assert_eq!(
        KnowledgeStore::get_knowledge_rich_document_by_document_id(
            &database,
            &workspace.id,
            &legacy_document.id,
        )
        .await?
        .map(|row| row.rich_document_id),
        Some(rich_document.rich_document_id.clone())
    );
    assert!(KnowledgeStore::get_knowledge_source_by_document_id(
        &database,
        &workspace.id,
        &rich_document.rich_document_id,
    )
    .await?
    .is_some());

    assert!(KnowledgeStore::get_knowledge_rich_document_draft(
        &database,
        &rich_document.rich_document_id
    )
    .await?
    .is_none());
    let draft = KnowledgeStore::upsert_knowledge_rich_document_draft(
        &database,
        UpsertKnowledgeRichDocumentDraft {
            rich_document_id: rich_document.rich_document_id.clone(),
            base_doc_version: rich_document.doc_version,
            base_content_sha256: rich_document.content_sha256.clone(),
            content_json: rich_content("draft"),
            actor_kind: "operator".to_owned(),
            actor_id: "mt136".to_owned(),
            kernel_task_run_id: "kernel-task-mt136".to_owned(),
            session_run_id: "session-mt136".to_owned(),
        },
    )
    .await?;
    assert_eq!(draft.base_doc_version, 1);
    assert!(KnowledgeStore::get_knowledge_rich_document_draft(
        &database,
        &rich_document.rich_document_id
    )
    .await?
    .is_some());
    assert!(
        KnowledgeStore::clear_knowledge_rich_document_draft(
            &database,
            &rich_document.rich_document_id
        )
        .await?
    );
    assert!(
        !KnowledgeStore::clear_knowledge_rich_document_draft(
            &database,
            &rich_document.rich_document_id
        )
        .await?
    );

    let rich_document = KnowledgeStore::save_knowledge_rich_document_version(
        &database,
        &rich_document.rich_document_id,
        1,
        rich_content("version-two"),
        None,
        Some("snapshot-v2"),
        None,
    )
    .await?;
    assert_conflict(
        KnowledgeStore::save_knowledge_rich_document_version(
            &database,
            &rich_document.rich_document_id,
            1,
            rich_content("stale-writer"),
            None,
            None,
            None,
        )
        .await,
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_rich_document_versions(
            &database,
            &rich_document.rich_document_id,
        )
        .await?
        .len(),
        2
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_rich_document_version_metas(
            &database,
            &rich_document.rich_document_id,
            10,
            0,
        )
        .await?
        .len(),
        2
    );
    assert_eq!(
        KnowledgeStore::count_knowledge_rich_document_versions(
            &database,
            &rich_document.rich_document_id,
        )
        .await?,
        2
    );
    assert!(KnowledgeStore::get_knowledge_rich_document_version(
        &database,
        &rich_document.rich_document_id,
        1,
    )
    .await?
    .is_some());
    let rich_document = KnowledgeStore::rename_knowledge_rich_document(
        &database,
        &rich_document.rich_document_id,
        "MT-136 renamed rich document",
        None,
    )
    .await?;
    let rich_document = KnowledgeStore::move_knowledge_rich_document(
        &database,
        &rich_document.rich_document_id,
        Some("project-rehomed"),
        Some("folder-rehomed"),
    )
    .await?;
    let rich_document = KnowledgeStore::set_knowledge_rich_document_authority_label(
        &database,
        &rich_document.rich_document_id,
        "archived",
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_rich_documents(
            &database,
            &workspace.id,
            Some("project-rehomed"),
            Some("folder-rehomed"),
        )
        .await?
        .len(),
        1
    );

    let code_node = KnowledgeStore::upsert_knowledge_editor_code_node(
        &database,
        UpsertEditorCodeNode {
            rich_document_id: rich_document.rich_document_id.clone(),
            node_path: "body.0.code".to_owned(),
            language_id: "rust".to_owned(),
            code_text: "fn proven() {}".to_owned(),
            worker_requirements: json!({ "worker": "rust", "bundled": false }),
            source_mapping: Some(json!({ "source_id": source.source_id })),
            lint_diagnostics: json!([]),
        },
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_editor_code_nodes(
            &database,
            &rich_document.rich_document_id,
        )
        .await?
        .len(),
        1
    );

    let embed = KnowledgeStore::upsert_knowledge_document_embed(
        &database,
        UpsertKnowledgeDocumentEmbed {
            rich_document_id: rich_document.rich_document_id.clone(),
            block_id: "embed-a".to_owned(),
            ref_kind: "source".to_owned(),
            ref_value: source.source_id.clone(),
            caption: Some("source evidence".to_owned()),
        },
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_document_embeds(&database, &rich_document.rich_document_id,)
            .await?
            .len(),
        1
    );
    KnowledgeStore::set_knowledge_document_embed_repair_state(
        &database,
        &embed.embed_id,
        Some("temporary proof failure"),
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_document_broken_embeds(
            &database,
            &rich_document.rich_document_id,
        )
        .await?
        .len(),
        1
    );
    let embeds = KnowledgeStore::replace_knowledge_document_embeds(
        &database,
        &rich_document.rich_document_id,
        vec![
            UpsertKnowledgeDocumentEmbed {
                rich_document_id: rich_document.rich_document_id.clone(),
                block_id: "embed-b".to_owned(),
                ref_kind: "url".to_owned(),
                ref_value: "https://example.invalid/proof".to_owned(),
                caption: None,
            },
            UpsertKnowledgeDocumentEmbed {
                rich_document_id: rich_document.rich_document_id.clone(),
                block_id: "embed-c".to_owned(),
                ref_kind: "source".to_owned(),
                ref_value: source.source_id.clone(),
                caption: Some("durable source".to_owned()),
            },
        ],
    )
    .await?;
    assert_eq!(embeds.len(), 2);

    let backlink = UpsertKnowledgeDocumentBacklink {
        workspace_id: workspace.id.clone(),
        relationship_id: format!("KDLNK-{}", "a".repeat(64)),
        source_document_id: rich_document.rich_document_id.clone(),
        link_kind: "tag".to_owned(),
        target: "surrealdb-proof".to_owned(),
        block_id: "paragraph-version-two".to_owned(),
    };
    KnowledgeStore::upsert_knowledge_document_backlink(&database, backlink.clone()).await?;
    KnowledgeStore::replace_knowledge_document_backlinks(
        &database,
        &rich_document.rich_document_id,
        vec![backlink],
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_document_backlinks_from(
            &database,
            &rich_document.rich_document_id,
        )
        .await?
        .len(),
        1
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_document_backlinks_to(
            &database,
            &workspace.id,
            "tag",
            "surrealdb-proof",
        )
        .await?
        .len(),
        1
    );

    // MT-060: durable bundle decisions and replayable retrieval trace.
    let kernel_bundle = ContextBundle::new(
        "kernel-task-mt136",
        "session-mt136",
        json!({ "workspace_id": workspace.id, "source_id": source.source_id }),
    )
    .expect("valid MT-136 context bundle");
    let bundle = KnowledgeStore::record_knowledge_context_bundle(
        &database,
        NewKnowledgeContextBundle {
            workspace_id: workspace.id.clone(),
            bundle: kernel_bundle,
            query_text: Some("prove SurrealDB durability".to_owned()),
            token_budget: Some(1000),
            tokens_used: Some(12),
            build_receipt_event_id: None,
            items: vec![NewKnowledgeContextBundleItem {
                ref_kind: KnowledgeBundleItemRefKind::Passage,
                ref_id: durable_passage.passage_id.clone(),
                retrieval_decision: KnowledgeBundleItemDecision::Included,
                relevance_score: Some(0.95),
                token_count: Some(12),
                citation: Some(format!("passage:{}", durable_passage.passage_id)),
                supported: true,
                unsupported_reason: None,
            }],
        },
    )
    .await?;
    let (_, bundle_items) =
        KnowledgeStore::get_knowledge_context_bundle(&database, &bundle.bundle_id)
            .await?
            .expect("newly recorded context bundle");
    assert_eq!(bundle_items.len(), 1);
    let trace = KnowledgeStore::record_knowledge_retrieval_trace(
        &database,
        NewKnowledgeRetrievalTrace {
            workspace_id: workspace.id.clone(),
            retrieval_mode: KnowledgeRetrievalMode::HybridRag,
            mode_reason: "graph context required for the proof".to_owned(),
            query_text: Some("prove SurrealDB durability".to_owned()),
            bundle_id: Some(bundle.bundle_id.clone()),
            decisions: json!([{ "step": "bundle", "action": "include" }]),
            trace_receipt_event_id: None,
        },
    )
    .await?;
    assert_eq!(
        KnowledgeStore::list_knowledge_retrieval_traces_for_bundle(&database, &bundle.bundle_id,)
            .await?
            .len(),
        1
    );

    let (first_rich_save, second_rich_save) = tokio::join!(
        KnowledgeStore::save_knowledge_rich_document_version_idempotent(
            &database,
            "mt136-rich-save-idempotency",
            &rich_document.rich_document_id,
            2,
            rich_content("version-three-idempotent"),
            None,
            Some("snapshot-v3"),
            None,
        ),
        KnowledgeStore::save_knowledge_rich_document_version_idempotent(
            &database,
            "mt136-rich-save-idempotency",
            &rich_document.rich_document_id,
            2,
            rich_content("version-three-idempotent"),
            None,
            Some("snapshot-v3"),
            None,
        ),
    );
    let mut rich_saves = vec![first_rich_save?, second_rich_save?];
    rich_saves.sort_by_key(|write| write.replayed);
    let saved = &rich_saves[0];
    let replayed = &rich_saves[1];
    assert!(!saved.replayed);
    assert!(replayed.replayed);
    assert_eq!(saved.value.doc_version, replayed.value.doc_version);
    assert_conflict(
        KnowledgeStore::save_knowledge_rich_document_version_idempotent(
            &database,
            "mt136-rich-save-idempotency",
            &rich_document.rich_document_id,
            2,
            rich_content("divergent-version-three"),
            None,
            Some("snapshot-v3"),
            None,
        )
        .await,
    );

    let durable_code_file =
        KnowledgeStore::get_knowledge_code_file_by_source(&database, &source.source_id)
            .await?
            .ok_or(StorageError::NotFound("MT-136 durable knowledge code file"))?;
    assert_eq!(durable_code_file.code_file_id, primary_code_file_id);
    assert!(durable_code_file.stale);
    assert_eq!(
        database
            .list_knowledge_code_files(&workspace.id)
            .await?
            .len(),
        2
    );

    // Close every live handle and reopen the exact same RocksDB directory.
    drop(database);
    drop(setup_database);
    drop(storage);
    let backend = backend.reopen().await?;
    let reopened = SurrealDatabase::new(backend.storage.clone());

    assert_eq!(
        KnowledgeStore::list_knowledge_schema_registry(&reopened).await?,
        registry
    );
    assert!(KnowledgeStore::audit_knowledge_namespace(&reopened)
        .await?
        .is_sound());
    assert!(
        KnowledgeStore::get_knowledge_source_root(&reopened, &root.root_id)
            .await?
            .is_some()
    );
    assert!(
        KnowledgeStore::get_knowledge_source(&reopened, &source.source_id)
            .await?
            .is_some()
    );
    assert!(
        KnowledgeStore::get_knowledge_index_run(&reopened, &index_run.index_run_id)
            .await?
            .is_some()
    );
    assert!(
        KnowledgeStore::get_knowledge_span(&reopened, &span_a.span_id)
            .await?
            .is_some()
    );
    assert!(
        KnowledgeStore::get_knowledge_entity(&reopened, &entity_a.entity_id)
            .await?
            .is_some()
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_entity_span_ids(&reopened, &entity_a.entity_id).await?,
        vec![span_b.span_id.clone()]
    );
    assert!(KnowledgeStore::get_knowledge_edge(&reopened, &edge.edge_id)
        .await?
        .is_some());
    assert!(
        KnowledgeStore::get_knowledge_claim(&reopened, &claim_a.claim_id)
            .await?
            .is_some()
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_claim_conflicts(&reopened, &claim_a.claim_id)
            .await?
            .len(),
        1
    );
    assert!(
        KnowledgeStore::get_knowledge_memory_passage(&reopened, &durable_passage.passage_id)
            .await?
            .is_some()
    );
    assert!(KnowledgeStore::get_knowledge_memory_passage(
        &reopened,
        &first_idempotent_passage.value.passage_id
    )
    .await?
    .is_some());
    assert!(KnowledgeStore::get_knowledge_wiki_projection(
        &reopened,
        &durable_projection.projection_id
    )
    .await?
    .is_some());
    assert_eq!(
        KnowledgeStore::get_knowledge_rich_document(&reopened, &rich_document.rich_document_id)
            .await?
            .expect("reopened rich document")
            .doc_version,
        3
    );
    assert_eq!(
        KnowledgeStore::count_knowledge_rich_document_versions(
            &reopened,
            &rich_document.rich_document_id,
        )
        .await?,
        3
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_editor_code_nodes(
            &reopened,
            &rich_document.rich_document_id,
        )
        .await?
        .first()
        .map(|row| row.code_node_id.as_str()),
        Some(code_node.code_node_id.as_str())
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_document_embeds(&reopened, &rich_document.rich_document_id)
            .await?
            .len(),
        2
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_document_backlinks_from(
            &reopened,
            &rich_document.rich_document_id,
        )
        .await?
        .len(),
        1
    );
    assert_eq!(
        KnowledgeStore::get_knowledge_context_bundle(&reopened, &bundle.bundle_id)
            .await?
            .expect("reopened context bundle")
            .1
            .len(),
        1
    );
    assert_eq!(
        KnowledgeStore::list_knowledge_retrieval_traces_for_bundle(&reopened, &bundle.bundle_id,)
            .await?
            .first()
            .map(|row| row.trace_id.as_str()),
        Some(trace.trace_id.as_str())
    );
    let reopened_code_file =
        KnowledgeStore::get_knowledge_code_file_by_source(&reopened, &source.source_id)
            .await?
            .ok_or(StorageError::NotFound(
                "MT-136 reopened knowledge code file",
            ))?;
    assert_eq!(reopened_code_file.code_file_id, primary_code_file_id);
    assert_eq!(
        reopened_code_file.file_entity_id,
        Some(entity_a.entity_id.clone())
    );
    assert!(reopened_code_file.stale);
    assert_eq!(
        reopened
            .list_knowledge_code_files(&workspace.id)
            .await?
            .len(),
        2
    );
    let reopened_wiki_inputs = reopened.list_wiki_code_file_inputs(&workspace.id).await?;
    assert_eq!(reopened_wiki_inputs.len(), 2);
    assert_eq!(reopened_wiki_inputs[0].code_file_id, cross_code_file_id);
    let reopened_cross_edges = reopened
        .list_wiki_cross_source_code_edges(&workspace.id, 100)
        .await?;
    assert_eq!(reopened_cross_edges.len(), 1);
    assert_eq!(
        reopened_cross_edges[0].from_source_id.as_str(),
        source.source_id.as_str()
    );
    assert_eq!(
        reopened_cross_edges[0].to_source_id.as_str(),
        cross_source.source_id.as_str()
    );

    drop(reopened);
    backend.close_and_remove().await?;
    Ok(())
}

pub(super) async fn run_all() -> StorageResult<()> {
    all_knowledge_store_methods_use_real_rocksdb_and_survive_reopen().await
}

#[cfg(test)]
#[tokio::test]
async fn mt136_knowledge_surface_proof() -> StorageResult<()> {
    run_all().await
}
