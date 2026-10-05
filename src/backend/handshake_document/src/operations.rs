//! Create/save sequencing. Authenticated query and durable receipt capabilities are supplied by the host.
use crate::diagnostics::observe_document_phase;
use crate::domain::*;
use crate::model::backlink::DocumentLinkReferences;
use crate::model::embed::{validate_block_embeds, ValidatedBlockEmbed};
use crate::model::{BlockTree, DocumentActorKind};
use handshake_storage_support::diagnostics::observe_result;
use handshake_storage_support::StorageError;
use serde::Deserialize;
use serde_json::{json, Value};
pub const SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD: &str = "minted_by_principal";
#[async_trait::async_trait]
pub trait DocumentStore: Send + Sync {
    async fn create_knowledge_rich_document(
        &self,
        value: NewKnowledgeRichDocument,
    ) -> Result<KnowledgeRichDocument, StorageError>;
    async fn create_knowledge_rich_document_with_profile_read(
        &self,
        value: NewKnowledgeRichDocument,
        _profile_can_read_fs: Option<bool>,
    ) -> Result<KnowledgeRichDocument, StorageError> {
        self.create_knowledge_rich_document(value).await
    }
    async fn create_knowledge_rich_document_if_title_absent(
        &self,
        value: NewKnowledgeRichDocument,
    ) -> Result<(KnowledgeRichDocument, bool), StorageError>;
    async fn create_knowledge_rich_document_if_title_absent_with_profile_read(
        &self,
        value: NewKnowledgeRichDocument,
        _profile_can_read_fs: Option<bool>,
    ) -> Result<(KnowledgeRichDocument, bool), StorageError> {
        self.create_knowledge_rich_document_if_title_absent(value)
            .await
    }
    async fn save_knowledge_rich_document_version(
        &self,
        id: &str,
        expected: i64,
        content: Value,
        crdt: Option<&str>,
        snapshot: Option<&str>,
        receipt: Option<&str>,
    ) -> Result<KnowledgeRichDocument, StorageError>;
    async fn replace_knowledge_document_backlinks(
        &self,
        id: &str,
        values: Vec<UpsertKnowledgeDocumentBacklink>,
    ) -> Result<Vec<KnowledgeDocumentBacklink>, StorageError>;
    async fn replace_knowledge_document_backlinks_with_profile_read(
        &self,
        id: &str,
        values: Vec<UpsertKnowledgeDocumentBacklink>,
        _profile_can_read_fs: Option<bool>,
    ) -> Result<Vec<KnowledgeDocumentBacklink>, StorageError> {
        self.replace_knowledge_document_backlinks(id, values).await
    }
    async fn replace_knowledge_document_embeds(
        &self,
        id: &str,
        values: Vec<UpsertKnowledgeDocumentEmbed>,
    ) -> Result<Vec<KnowledgeDocumentEmbed>, StorageError>;
    async fn get_knowledge_source_by_document_id(
        &self,
        workspace: &str,
        document: &str,
    ) -> Result<Option<KnowledgeSource>, StorageError>;
    async fn mark_knowledge_source_stale(&self, id: &str) -> Result<KnowledgeSource, StorageError>;
    async fn upsert_knowledge_source(
        &self,
        value: NewKnowledgeSource,
    ) -> Result<KnowledgeSource, StorageError>;
    async fn upsert_knowledge_entity(
        &self,
        value: NewKnowledgeEntity,
    ) -> Result<KnowledgeEntity, StorageError>;
}
#[async_trait::async_trait]
pub trait DocumentHost: Send + Sync {
    fn require_index(&self) -> Result<(), ()>;
    fn profile_can_read_fs(&self) -> Option<bool> {
        None
    }
    fn actor_kind(&self) -> DocumentActorKind;
    fn minted_by_principal(&self) -> Option<&str>;
    fn take_backlink_failure(&self, id: &str) -> bool;
    fn take_embed_failure(&self, id: &str) -> bool;
    async fn record_saved_receipt(
        &self,
        id: &str,
        payload: Value,
    ) -> (Option<String>, Option<String>);
}
#[derive(Debug, Deserialize)]
pub struct SaveDocumentBody {
    pub expected_version: i64,
    pub content_json: Value,
    #[serde(default)]
    pub crdt_document_id: Option<String>,
    #[serde(default)]
    pub crdt_snapshot_id: Option<String>,
    #[serde(default)]
    pub promotion_receipt_event_id: Option<String>,
}

pub async fn index_document_into_knowledge_index(
    db: &impl DocumentStore,
    document: &KnowledgeRichDocument,
) -> Result<(), StorageError> {
    let source = match observe_result(
        "index_source_lookup",
        db.get_knowledge_source_by_document_id(&document.workspace_id, &document.rich_document_id),
    )
    .await?
    {
        Some(existing) => {
            if existing.content_hash != document.content_sha256 && !existing.stale {
                // The document changed since the source was indexed: stale is
                // the truthful index state until the pipeline re-indexes it.
                observe_result(
                    "index_source_stale",
                    db.mark_knowledge_source_stale(&existing.source_id),
                )
                .await?
            } else {
                existing
            }
        }
        None => {
            observe_result(
                "index_source_upsert",
                db.upsert_knowledge_source(NewKnowledgeSource {
                    workspace_id: document.workspace_id.clone(),
                    root_id: None,
                    source_kind: KnowledgeSourceKind::RichDocument,
                    relative_path: None,
                    asset_id: None,
                    loom_block_id: None,
                    // The schema's document_id column FKs the LEGACY documents
                    // table; the KRD linkage is provenance-keyed (see
                    // get_knowledge_source_by_document_id).
                    document_id: None,
                    content_hash: document.content_sha256.clone(),
                    size_bytes: Some(document.content_json.to_string().len() as i64),
                    provenance: json!({
                        "discovered_by": "knowledge_documents_api",
                        "rich_document_id": document.rich_document_id,
                        "schema_version": document.schema_version,
                    }),
                    permission_scope: KnowledgePermissionScope::Workspace,
                    redaction_state: KnowledgeRedactionState::None,
                    source_modified_at: None,
                }),
            )
            .await?
        }
    };

    observe_result(
        "index_entity_upsert",
        db.upsert_knowledge_entity(NewKnowledgeEntity {
            workspace_id: document.workspace_id.clone(),
            entity_kind: KnowledgeEntityKind::RichDocument,
            entity_key: document.rich_document_id.clone(),
            display_name: document.title.clone(),
            detection_provenance: json!({
                "extractor": "knowledge_documents_api",
                "content_sha256": document.content_sha256,
                "doc_version": document.doc_version,
            }),
            primary_source_id: Some(source.source_id),
            detected_in_run: None,
            evidence_span_ids: vec![],
        }),
    )
    .await?;
    Ok(())
}

/// Run the MT-154 index step post-commit and RECORD a failure instead of
/// erroring a committed write (MT-149 law). Returns (indexed, error).
pub async fn index_document_non_fatal(
    db: &impl DocumentStore,
    document: &KnowledgeRichDocument,
) -> (bool, Option<String>) {
    match index_document_into_knowledge_index(db, document).await {
        Ok(()) => (true, None),
        Err(err) => {
            tracing::error!(
                target: "handshake_core::knowledge_documents_api",
                rich_document_id = %document.rich_document_id,
                error = %err,
                "rich_document_knowledge_index_failed_post_commit"
            );
            (false, Some(err.to_string()))
        }
    }
}

pub fn embed_upserts(
    rich_document_id: &str,
    validated: &[ValidatedBlockEmbed],
) -> Vec<UpsertKnowledgeDocumentEmbed> {
    validated
        .iter()
        .map(|embed| UpsertKnowledgeDocumentEmbed {
            rich_document_id: rich_document_id.to_string(),
            block_id: embed.block_id.clone(),
            ref_kind: embed.target.kind.as_str().to_string(),
            ref_value: embed.target.value.clone(),
            caption: embed.caption.clone(),
        })
        .collect()
}

/// Append a document EventLedger receipt (save/promotion/nav) and return its id.
pub async fn create_document(
    db: &impl DocumentStore,
    host: &impl DocumentHost,
    new_document: NewKnowledgeRichDocument,
    create_if_title_absent: bool,
) -> Result<Value, StorageError> {
    let created_result = observe_document_phase(
        "create_transaction",
        async {
            if create_if_title_absent {
                db.create_knowledge_rich_document_if_title_absent_with_profile_read(
                    new_document,
                    host.profile_can_read_fs(),
                )
                .await
            } else {
                db.create_knowledge_rich_document_with_profile_read(
                    new_document,
                    host.profile_can_read_fs(),
                )
                .await
                .map(|created| (created, true))
            }
        },
        Result::is_err,
    )
    .await;
    let (created, document_created) = match created_result {
        Ok(created) => created,
        Err(error) => {
            #[cfg(test)]
            eprintln!("knowledge-document-create failed: {error}");
            return Err(error);
        }
    };

    if !document_created {
        return Ok(json!({
            "document": created,
            "created": false,
            "save_receipt_event_id": Value::Null,
            "receipt_error": Value::Null,
            "embeds_persisted": 0,
            "embeds_error": Value::Null,
            "knowledge_indexed": false,
            "knowledge_index_error": Value::Null,
        }));
    }

    // ---- post-commit (MT-149): the create above is committed; the steps
    // below are best-effort and RECORDED, never an error for a committed write.
    let (receipt, receipt_error) = observe_document_phase(
        "create_receipt",
        host.record_saved_receipt(
            &created.rich_document_id,
            json!({"event": "created", "doc_version": created.doc_version}),
        ),
        |result| result.1.is_some(),
    )
    .await;

    // MT-152: sync the typed embed side table from the validated content.
    // Re-validate against the REAL document id so derived block ids match.
    let mut embeds_persisted = 0usize;
    let mut embeds_error: Option<String> = None;
    let created_tree = BlockTree::from_document_json(
        &created.rich_document_id,
        &created.schema_version,
        &created.content_json,
    )
    .ok();
    if let Some(created_tree) = created_tree {
        if let Ok(validated) = validate_block_embeds(&created_tree) {
            match observe_document_phase(
                "create_embeds",
                db.replace_knowledge_document_embeds(
                    &created.rich_document_id,
                    embed_upserts(&created.rich_document_id, &validated),
                ),
                Result::is_err,
            )
            .await
            {
                Ok(persisted) => embeds_persisted = persisted.len(),
                Err(err) => {
                    tracing::error!(
                        target: "handshake_core::knowledge_documents_api",
                        rich_document_id = %created.rich_document_id,
                        error = %err,
                        "rich_document_embed_sync_failed_post_commit"
                    );
                    embeds_error = Some(err.to_string());
                }
            }
        }
    }

    // MT-154: index the created document (source + title entity) when the
    // actor may index; denial just skips (read-only actors cannot create).
    let mut knowledge_indexed = false;
    let mut knowledge_index_error: Option<String> = None;
    if host.require_index().is_ok() {
        (knowledge_indexed, knowledge_index_error) = observe_document_phase(
            "create_index",
            index_document_non_fatal(db, &created),
            |result| result.1.is_some(),
        )
        .await;
    }

    Ok(json!({
        "document": created,
        "created": true,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
        "embeds_persisted": embeds_persisted,
        "embeds_error": embeds_error,
        "knowledge_indexed": knowledge_indexed,
        "knowledge_index_error": knowledge_index_error,
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn save_document(
    db: &impl DocumentStore,
    host: &impl DocumentHost,
    document_id: &str,
    body: SaveDocumentBody,
    crdt_document_id: Option<String>,
    validated_embeds: Vec<ValidatedBlockEmbed>,
    document_link_references: DocumentLinkReferences,
    receipt_reference_targets: Vec<String>,
) -> Result<Value, StorageError> {
    let saved = observe_document_phase(
        "save_transaction",
        db.save_knowledge_rich_document_version(
            &document_id,
            body.expected_version,
            body.content_json.clone(),
            crdt_document_id.as_deref(),
            body.crdt_snapshot_id.as_deref(),
            body.promotion_receipt_event_id.as_deref(),
        ),
        Result::is_err,
    )
    .await?;

    // ---- post-commit (MT-149): nothing below may error a committed save. ----
    // WP-KERNEL-012 MT-120: when (and only when) the caller authenticated a live native-MCP session,
    // stamp the SERVER-DERIVED principal into the receipt payload. This is the anchor the Flight
    // Recorder's `document_saved` receipt-ownership clause compares against. The ledger `actor_id`
    // column deliberately stays the CLIENT-declared per-agent id so two swarm agents saving the same
    // document remain individually attributable; ownership and attribution are different questions
    // and now have different fields. `ctx.actor`, the run ids and the correlation id are untouched —
    // the same clause compares those against the client-supplied Flight Recorder payload.
    let mut receipt_payload = json!({
        "event": "saved",
        "doc_version": saved.doc_version,
        "workspace_id": saved.workspace_id.clone(),
        "content_hash": saved.content_sha256.clone(),
        "reference_targets": receipt_reference_targets,
    });
    if let Some(principal) = host.minted_by_principal() {
        if let Some(map) = receipt_payload.as_object_mut() {
            map.insert(
                SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD.to_owned(),
                Value::String(principal.to_owned()),
            );
        }
    }
    let (receipt, receipt_error) = observe_document_phase(
        "save_receipt",
        host.record_saved_receipt(&saved.rich_document_id, receipt_payload),
        |result| result.1.is_some(),
    )
    .await;

    // MT-155 backlinks + MT-152 embeds: re-extract + persist from the new
    // content (the document content is the source of truth; both rebuilds are
    // idempotent). Index permission is checked, but a denial is non-fatal to
    // the save — it just skips the index step and reports it. A storage
    // failure in either step is RECORDED, never an error for the saved write.
    let mut backlinks_persisted = 0usize;
    let mut backlinks_error: Option<String> = None;
    let mut backlinks_skipped_reason: Option<String> = None;
    let mut embeds_persisted = 0usize;
    let mut embeds_error: Option<String> = None;
    let mut knowledge_indexed = false;
    let mut knowledge_index_error: Option<String> = None;
    match host.require_index() {
        Ok(()) => {
            let upserts: Vec<UpsertKnowledgeDocumentBacklink> = document_link_references
                .references
                .iter()
                .map(|r| UpsertKnowledgeDocumentBacklink {
                    workspace_id: saved.workspace_id.clone(),
                    relationship_id: r.relationship_id.clone(),
                    source_document_id: saved.rich_document_id.clone(),
                    link_kind: r.kind.as_str().to_string(),
                    target: r.target.clone(),
                    block_id: r.block_id.clone(),
                })
                .collect();
            let inject_backlink_failure = host.take_backlink_failure(&saved.rich_document_id);
            if inject_backlink_failure {
                backlinks_error = Some("MT-141 injected post-commit backlink failure".to_owned());
            } else {
                match observe_document_phase(
                    "save_backlinks",
                    db.replace_knowledge_document_backlinks_with_profile_read(
                        &saved.rich_document_id,
                        upserts,
                        host.profile_can_read_fs(),
                    ),
                    Result::is_err,
                )
                .await
                {
                    Ok(persisted) => backlinks_persisted = persisted.len(),
                    Err(err) => {
                        tracing::error!(
                            target: "handshake_core::knowledge_documents_api",
                            rich_document_id = %saved.rich_document_id,
                            error = %err,
                            "rich_document_backlink_index_failed_post_commit"
                        );
                        backlinks_error = Some(err.to_string());
                    }
                }
            }
            let inject_embed_failure = host.take_embed_failure(&saved.rich_document_id);
            if inject_embed_failure {
                embeds_error = Some("MT-141 injected post-commit embed failure".to_owned());
            } else {
                match observe_document_phase(
                    "save_embeds",
                    db.replace_knowledge_document_embeds(
                        &saved.rich_document_id,
                        embed_upserts(&saved.rich_document_id, &validated_embeds),
                    ),
                    Result::is_err,
                )
                .await
                {
                    Ok(persisted) => embeds_persisted = persisted.len(),
                    Err(err) => {
                        tracing::error!(
                            target: "handshake_core::knowledge_documents_api",
                            rich_document_id = %saved.rich_document_id,
                            error = %err,
                            "rich_document_embed_sync_failed_post_commit"
                        );
                        embeds_error = Some(err.to_string());
                    }
                }
            }
            // MT-154: the document is indexed into the Project Knowledge
            // Index (source row + title entity; staleness on content change).
            (knowledge_indexed, knowledge_index_error) = observe_document_phase(
                "save_index",
                index_document_non_fatal(db, &saved),
                |result| result.1.is_some(),
            )
            .await;
        }
        Err(_) => {
            backlinks_skipped_reason = Some(format!("{}_index_denied", host.actor_kind().as_str()));
        }
    }

    Ok(json!({
        "document": saved,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
        "backlinks_persisted": backlinks_persisted,
        "backlinks_error": backlinks_error,
        "backlinks_skipped_reason": backlinks_skipped_reason,
        "embeds_persisted": embeds_persisted,
        "embeds_error": embeds_error,
        "knowledge_indexed": knowledge_indexed,
        "knowledge_index_error": knowledge_index_error,
    }))
}
