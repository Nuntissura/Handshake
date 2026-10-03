//! RichDocument SurrealQL adapter; the host supplies a live authenticated query boundary.
use crate::domain::*;
use chrono::{DateTime, Utc};
use handshake_storage_support::diagnostics::observe_result;
use handshake_storage_support::{StorageError, StorageResult};
use serde_json::Value as JsonValue;
use std::collections::{BTreeSet, HashMap, HashSet};
use surrealdb_types::{Datetime, RecordId, RecordIdKey, SurrealValue, Value as SurrealValueData};
pub type Binds = Vec<(String, SurrealValueData)>;
pub type Guard = (&'static str, fn() -> StorageError);
#[async_trait::async_trait]
pub trait DocumentQuery: Send + Sync {
    async fn execute(&self, statement: String, binds: Binds) -> StorageResult<()>;
    fn session_id(&self) -> Option<String>;
    fn source_authority_sql(&self) -> &'static str;
    async fn prepare_source_resource(
        &self,
        source_id: &str,
        workspace_id: &str,
    ) -> StorageResult<SurrealValueData>;

    async fn rows_pair<A: SurrealValue + Send + 'static, B: SurrealValue + Send + 'static>(
        &self,
        statement: String,
        binds: Binds,
    ) -> StorageResult<(Vec<A>, Vec<B>)>;

    async fn rows<R: SurrealValue + Send + 'static>(
        &self,
        statement: String,
        binds: Binds,
        index: usize,
        guards: Option<&[Guard]>,
    ) -> StorageResult<Vec<R>>;
}
pub fn b(name: &str, value: impl SurrealValue) -> (String, SurrealValueData) {
    (name.to_owned(), value.into_value())
}
pub fn thing(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key.to_owned())
}
pub fn record_key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(id) => Ok(id),
        _ => Err(StorageError::Serialization(
            "knowledge record link is not a string key".to_owned(),
        )),
    }
}
pub fn opt_record_key(record: Option<RecordId>) -> StorageResult<Option<String>> {
    record.map(record_key).transpose()
}
pub fn opt_time(value: Option<Datetime>) -> Option<DateTime<Utc>> {
    value.map(Datetime::into_inner)
}
async fn query_rows<R: SurrealValue + Send + 'static>(
    storage: &impl DocumentQuery,
    statement: impl Into<String>,
    binds: Binds,
) -> StorageResult<Vec<R>> {
    storage.rows(statement.into(), binds, 0, None).await
}
async fn query_first_row<R: SurrealValue + Send + 'static>(
    storage: &impl DocumentQuery,
    statement: impl Into<String>,
    binds: Binds,
) -> StorageResult<Option<R>> {
    Ok(query_rows(storage, statement, binds)
        .await?
        .into_iter()
        .next())
}
const WORKSPACES_TABLE: &str = "workspaces";
const LOOM_BLOCKS_TABLE: &str = "loom_blocks";
const KNOWLEDGE_RICH_DOCUMENTS_TABLE: &str = "knowledge_rich_documents";
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct RichDocRecord {
    pub rich_document_id: String,
    pub workspace_id: RecordId,
    pub document_id: Option<RecordId>,
    pub title: String,
    pub schema_version: String,
    pub doc_version: i64,
    pub content_json: JsonValue,
    pub content_sha256: String,
    pub crdt_document_id: Option<String>,
    pub crdt_snapshot_id: Option<String>,
    pub promotion_receipt_event_id: Option<RecordId>,
    pub projection_refs: JsonValue,
    pub project_ref: Option<String>,
    pub folder_ref: Option<String>,
    pub authority_label: String,
    pub owner_actor_kind: Option<String>,
    pub owner_actor_id: Option<String>,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

pub fn rich_document_to_domain(record: RichDocRecord) -> StorageResult<KnowledgeRichDocument> {
    Ok(KnowledgeRichDocument {
        block_id: record.rich_document_id.clone(),
        rich_document_id: record.rich_document_id,
        workspace_id: record_key(record.workspace_id)?,
        document_id: opt_record_key(record.document_id)?,
        title: record.title,
        schema_version: record.schema_version,
        doc_version: record.doc_version,
        content_json: record.content_json,
        content_sha256: record.content_sha256,
        crdt_document_id: record.crdt_document_id,
        crdt_snapshot_id: record.crdt_snapshot_id,
        promotion_receipt_event_id: opt_record_key(record.promotion_receipt_event_id)?,
        projection_refs: record.projection_refs,
        project_ref: record.project_ref,
        folder_ref: record.folder_ref,
        authority_label: record.authority_label,
        owner_actor_kind: record.owner_actor_kind,
        owner_actor_id: record.owner_actor_id,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct BacklinkRecord {
    pub backlink_id: String,
    pub workspace_id: RecordId,
    pub relationship_id: String,
    pub source_document_id: RecordId,
    pub link_kind: String,
    pub target: String,
    pub block_id: String,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

pub fn backlink_to_domain(record: BacklinkRecord) -> StorageResult<KnowledgeDocumentBacklink> {
    Ok(KnowledgeDocumentBacklink {
        backlink_id: record.backlink_id,
        workspace_id: record_key(record.workspace_id)?,
        relationship_id: record.relationship_id,
        source_document_id: record_key(record.source_document_id)?,
        link_kind: record.link_kind,
        target: record.target,
        block_id: record.block_id,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct PriorBacklinkRecord {
    pub relationship_id: String,
    pub target: String,
}

#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct CandidateDocRecord {
    pub rich_document_id: String,
    pub title: String,
    pub is_live: bool,
}

#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct CandidateLoomRecord {
    pub block_id: String,
    pub workspace_id: RecordId,
}

pub struct ResolvedBacklink {
    pub backlink_id: String,
    pub relationship_id: String,
    pub link_kind: String,
    pub target: String,
    pub block_id: String,
    pub project_to_loom: bool,
}

/// The write half of the backlink rebuild, appended inside a transaction.
/// Five statements: owned-loom-edge delete, backlink delete, backlink create
/// loop, loom-edge create loop (fails closed on a foreign edge id), and the
/// affected-block count recomputation loop.
pub const BACKLINK_WRITE_STATEMENTS: &str = "DELETE loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $source_key) AND string::starts_with(edge_id, 'KDLNK-') AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND edit_event_id = '00000000-0000-0000-0000-000000000000' AND source_document_id = $source_key;\nDELETE knowledge_document_backlinks WHERE source_document_id = type::record('knowledge_rich_documents', $source_key);\nFOR $row IN $backlink_rows { CREATE type::record('knowledge_document_backlinks', $row.backlink_id) CONTENT { backlink_id: $row.backlink_id, workspace_id: $workspace, relationship_id: $row.relationship_id, source_document_id: type::record('knowledge_rich_documents', $source_key), link_kind: $row.link_kind, target: $row.target, block_id: $row.block_id } RETURN NONE; };\nFOR $row IN $loom_edge_rows { IF (SELECT VALUE id FROM loom_edges WHERE edge_id = $row.relationship_id LIMIT 1)[0] != NONE { THROW 'HSK-KDBL-LOOM-EDGE-OWNED'; }; CREATE type::record('loom_edges', $row.relationship_id) CONTENT { edge_id: $row.relationship_id, workspace_id: $workspace, source_block_id: type::record('loom_blocks', $source_key), target_block_id: type::record('loom_blocks', $row.target), edge_type: 'mention', created_by: 'user', last_actor_kind: 'SYSTEM', last_actor_id: 'knowledge_rich_document_backlink_projection', edit_event_id: '00000000-0000-0000-0000-000000000000', source_document_id: $source_key, source_text_block_id: $row.block_id } RETURN NONE; };\nFOR $affected IN $affected_blocks { UPDATE type::record('loom_blocks', $affected) SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $affected) AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $affected) AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = type::record('loom_blocks', $affected) AND edge_type IN ['mention', 'tag'])) WHERE workspace_id = $workspace AND block_id = $affected RETURN NONE; };";

/// Number of `;`-terminated statements in [`BACKLINK_WRITE_STATEMENTS`].
pub const BACKLINK_WRITE_STATEMENT_COUNT: usize = 5;

pub const BACKLINK_GUARDS: [(&str, fn() -> StorageError); 1] =
    [("HSK-KDBL-LOOM-EDGE-OWNED", || {
        StorageError::Conflict("knowledge backlink Loom edge identity is owned by another writer")
    })];

pub fn backlink_write_binds(
    workspace: RecordId,
    source_key: &str,
    resolved: &[ResolvedBacklink],
    affected_blocks: &BTreeSet<String>,
) -> Binds {
    let backlink_rows: Vec<JsonValue> = resolved
        .iter()
        .map(|row| {
            serde_json::json!({
                "backlink_id": row.backlink_id,
                "relationship_id": row.relationship_id,
                "link_kind": row.link_kind,
                "target": row.target,
                "block_id": row.block_id,
            })
        })
        .collect();
    let loom_edge_rows: Vec<JsonValue> = resolved
        .iter()
        .filter(|row| row.project_to_loom)
        .map(|row| {
            serde_json::json!({
                "relationship_id": row.relationship_id,
                "target": row.target,
                "block_id": row.block_id,
            })
        })
        .collect();
    vec![
        b("workspace", workspace),
        b("source_key", source_key.to_owned()),
        b("backlink_rows", JsonValue::Array(backlink_rows)),
        b("loom_edge_rows", JsonValue::Array(loom_edge_rows)),
        b(
            "affected_blocks",
            affected_blocks.iter().cloned().collect::<Vec<String>>(),
        ),
    ]
}

/// MT-170 AC-170-3: why a wikilink did not become a Loom mention edge. Emitted as a typed
/// `tracing` event (relationship id and reason only; never link text) instead of a silent skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklinkProjectionSkip {
    /// The target block lives in another workspace; the backlink row is dropped.
    ForeignWorkspace,
    /// A `KRD-` target is not a live rich document; the backlink row is dropped.
    MissingDocument,
    /// The title matches only deleted documents; the backlink row is dropped.
    DeletedTitle,
    /// No live Loom block readable by the saver matches the target; the row stays textual.
    NoReadableLoomTarget,
}

impl BacklinkProjectionSkip {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ForeignWorkspace => "foreign_workspace",
            Self::MissingDocument => "missing_document",
            Self::DeletedTitle => "deleted_title",
            Self::NoReadableLoomTarget => "no_readable_loom_target",
        }
    }
}

fn log_projection_skip(relationship_id: &str, skip: BacklinkProjectionSkip) {
    tracing::info!(
        target: "handshake_document::backlinks",
        relationship_id,
        reason = skip.as_str(),
        "knowledge backlink not projected to a Loom mention edge"
    );
}

/// Ports the removed backend's wikilink resolution verbatim: exact live
/// same-workspace Loom identity wins, cross-workspace ids are dropped, KRD ids
/// must be live, ambiguous titles keep the prior live target or stay textual,
/// deleted titles drop the row, and a live RichDocument without its same-id
/// LoomBlock projection fails closed.
pub async fn resolve_backlink_rows(
    storage: &impl DocumentQuery,
    workspace_key: &str,
    source_document_id: &str,
    upserts: Vec<UpsertKnowledgeDocumentBacklink>,
    prior_by_relationship: &HashMap<String, String>,
    prior_loom_targets: &[String],
) -> StorageResult<Vec<ResolvedBacklink>> {
    let mut candidate_titles: Vec<String> = upserts
        .iter()
        .filter(|upsert| upsert.link_kind == "wikilink" && !upsert.target.starts_with("KRD-"))
        .map(|upsert| upsert.target.clone())
        .collect();
    candidate_titles.sort();
    candidate_titles.dedup();
    let mut candidate_ids: Vec<String> = upserts
        .iter()
        .filter(|upsert| upsert.target.starts_with("KRD-"))
        .map(|upsert| upsert.target.clone())
        .chain(
            prior_by_relationship
                .values()
                .filter(|target| target.starts_with("KRD-"))
                .cloned(),
        )
        .collect();
    candidate_ids.sort();
    candidate_ids.dedup();

    let candidates_empty = candidate_titles.is_empty() && candidate_ids.is_empty();
    let candidate_targets: Vec<CandidateDocRecord> = if candidates_empty {
        Vec::new()
    } else if candidate_titles.is_empty() {
        let candidate_records: Vec<RecordId> = candidate_ids
            .iter()
            .map(|id| thing(KNOWLEDGE_RICH_DOCUMENTS_TABLE, id))
            .collect();
        query_rows(
            storage,
            "SELECT rich_document_id, title, (deleted_at = NONE) AS is_live \
             FROM $candidate_records \
             WHERE workspace_id = $workspace AND rich_document_id IN $candidate_ids \
             ORDER BY rich_document_id ASC;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_key)),
                b("candidate_ids", candidate_ids),
                b("candidate_records", candidate_records),
            ],
        )
        .await?
    } else {
        query_rows(
            storage,
            "SELECT rich_document_id, title, (deleted_at = NONE) AS is_live \
             FROM knowledge_rich_documents \
             WHERE workspace_id = $workspace \
               AND (rich_document_id IN $candidate_ids OR title IN $candidate_titles) \
             ORDER BY rich_document_id ASC;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_key)),
                b("candidate_ids", candidate_ids),
                b("candidate_titles", candidate_titles),
            ],
        )
        .await?
    };
    let live_ids: HashSet<String> = candidate_targets
        .iter()
        .filter(|row| row.is_live)
        .map(|row| row.rich_document_id.clone())
        .collect();
    let deleted_titles: HashSet<String> = candidate_targets
        .iter()
        .filter(|row| !row.is_live)
        .map(|row| row.title.clone())
        .collect();
    let mut ids_by_title: HashMap<String, Vec<String>> = HashMap::new();
    for row in candidate_targets {
        if row.is_live {
            ids_by_title
                .entry(row.title)
                .or_default()
                .push(row.rich_document_id);
        }
    }

    let mut candidate_loom_ids: Vec<String> = upserts
        .iter()
        .filter(|upsert| upsert.link_kind == "wikilink")
        .map(|upsert| upsert.target.clone())
        .chain(prior_by_relationship.values().cloned())
        .chain(prior_loom_targets.iter().cloned())
        .chain(live_ids.iter().cloned())
        .collect();
    candidate_loom_ids.sort();
    candidate_loom_ids.dedup();
    let candidate_input_count = upserts
        .iter()
        .filter(|upsert| upsert.link_kind == "wikilink")
        .count();
    let candidate_loom_targets: Vec<CandidateLoomRecord> = if candidate_loom_ids.is_empty() {
        Vec::new()
    } else {
        let candidate_loom_records: Vec<RecordId> = candidate_loom_ids
            .iter()
            .map(|id| thing(LOOM_BLOCKS_TABLE, id))
            .collect();
        #[cfg(feature = "surreal-test-support")]
        let query_started = std::time::Instant::now();
        let candidate_result = query_rows(
            storage,
            "SELECT block_id, workspace_id FROM $candidate_loom_records \
             WHERE block_id IN $candidate_loom_ids ORDER BY block_id ASC;",
            vec![
                b("candidate_loom_ids", candidate_loom_ids.clone()),
                b("candidate_loom_records", candidate_loom_records),
            ],
        )
        .await;
        #[cfg(feature = "surreal-test-support")]
        match &candidate_result {
            Ok(rows) => {
                let mut same_workspace_count = 0usize;
                let mut foreign_workspace_count = 0usize;
                for row in rows {
                    if record_key(row.workspace_id.clone())? == workspace_key {
                        same_workspace_count += 1;
                    } else {
                        foreign_workspace_count += 1;
                    }
                }
                tracing::info!(
                    target: "handshake_document::mt170_resolver_diagnostic",
                    owning_module = module_path!(),
                    case_label = "mt170_record_user_candidate_read",
                    candidate_input_count,
                    candidate_id_count = candidate_loom_ids.len(),
                    returned_row_count = rows.len(),
                    same_workspace_count,
                    foreign_workspace_count,
                    query_outcome = "ok",
                    error_class = "none",
                    elapsed_micros = query_started.elapsed().as_micros() as u64,
                    "MT-170 redacted resolver candidate observation"
                );
            }
            Err(error) => {
                let error_class = match error {
                    StorageError::NotFound(_) => "not_found",
                    StorageError::Conflict(_) => "conflict",
                    StorageError::ConflictDetails { .. } => "conflict_details",
                    StorageError::Validation(_) => "validation",
                    StorageError::Guard(_) => "guard",
                    StorageError::NotImplemented(_) => "not_implemented",
                    StorageError::Serialization(_) => "serialization",
                    StorageError::Database(_) => "database",
                    StorageError::Migration(_) => "migration",
                };
                tracing::info!(
                    target: "handshake_document::mt170_resolver_diagnostic",
                    owning_module = module_path!(),
                    case_label = "mt170_record_user_candidate_read",
                    candidate_input_count,
                    candidate_id_count = candidate_loom_ids.len(),
                    returned_row_count = 0usize,
                    same_workspace_count = 0usize,
                    foreign_workspace_count = 0usize,
                    query_outcome = "error",
                    error_class,
                    elapsed_micros = query_started.elapsed().as_micros() as u64,
                    "MT-170 redacted resolver candidate observation"
                );
            }
        }
        candidate_result?
    };
    let mut live_loom_ids: HashSet<String> = HashSet::new();
    let mut foreign_loom_ids: HashSet<String> = HashSet::new();
    for row in candidate_loom_targets {
        if record_key(row.workspace_id)? == workspace_key {
            live_loom_ids.insert(row.block_id);
        } else {
            foreign_loom_ids.insert(row.block_id);
        }
    }

    let mut resolved = Vec::with_capacity(upserts.len());
    for upsert in upserts {
        let prior_live_target = prior_by_relationship
            .get(&upsert.relationship_id)
            .filter(|target| live_loom_ids.contains(*target));
        let target = if upsert.link_kind == "wikilink" && live_loom_ids.contains(&upsert.target) {
            upsert.target.clone()
        } else if upsert.link_kind == "wikilink" && foreign_loom_ids.contains(&upsert.target) {
            log_projection_skip(
                &upsert.relationship_id,
                BacklinkProjectionSkip::ForeignWorkspace,
            );
            continue;
        } else if upsert.link_kind == "wikilink" && upsert.target.starts_with("KRD-") {
            if !live_ids.contains(&upsert.target) {
                log_projection_skip(
                    &upsert.relationship_id,
                    BacklinkProjectionSkip::MissingDocument,
                );
                continue;
            }
            upsert.target.clone()
        } else if upsert.link_kind == "wikilink" {
            match ids_by_title.get(&upsert.target) {
                Some(matches) if matches.len() == 1 => matches[0].clone(),
                Some(matches) => match prior_live_target {
                    Some(prior_target) if matches.contains(prior_target) => prior_target.clone(),
                    _ => upsert.target.clone(),
                },
                None if prior_live_target.is_some() => {
                    prior_live_target.expect("checked above").clone()
                }
                None if deleted_titles.contains(&upsert.target) => {
                    log_projection_skip(
                        &upsert.relationship_id,
                        BacklinkProjectionSkip::DeletedTitle,
                    );
                    continue;
                }
                None => upsert.target.clone(),
            }
        } else {
            upsert.target.clone()
        };
        if upsert.link_kind == "wikilink"
            && live_ids.contains(&target)
            && !live_loom_ids.contains(&target)
        {
            return Err(StorageError::Conflict(
                "knowledge backlink target is missing its LoomBlock projection",
            ));
        }
        let project_to_loom = upsert.link_kind == "wikilink" && live_loom_ids.contains(&target);
        if upsert.link_kind == "wikilink" && !project_to_loom {
            log_projection_skip(
                &upsert.relationship_id,
                BacklinkProjectionSkip::NoReadableLoomTarget,
            );
        }
        resolved.push(ResolvedBacklink {
            backlink_id: new_knowledge_id("KDBL"),
            relationship_id: upsert.relationship_id,
            link_kind: upsert.link_kind,
            target,
            block_id: upsert.block_id,
            project_to_loom,
        });
    }
    let _ = source_document_id;
    Ok(resolved)
}

pub async fn read_live_rich_document(
    storage: &impl DocumentQuery,
    rich_document_id: &str,
) -> StorageResult<Option<KnowledgeRichDocument>> {
    query_first_row::<RichDocRecord>(
        storage,
        "SELECT * FROM type::record('knowledge_rich_documents', $doc_id) \
         WHERE rich_document_id = $doc_id AND deleted_at = NONE;",
        vec![b("doc_id", rich_document_id.to_owned())],
    )
    .await?
    .map(rich_document_to_domain)
    .transpose()
}

pub async fn read_prior_backlink_state(
    storage: &impl DocumentQuery,
    workspace_key: &str,
    source_document_id: &str,
) -> StorageResult<(HashMap<String, String>, Vec<String>)> {
    let (prior_rows, prior_loom_targets): (Vec<PriorBacklinkRecord>, Vec<RecordId>) = storage
        .rows_pair(
            "SELECT relationship_id, target FROM knowledge_document_backlinks \
             WHERE source_document_id = type::record('knowledge_rich_documents', $source_key) \
             ORDER BY relationship_id ASC; \
             SELECT VALUE target_block_id FROM loom_edges \
             WHERE workspace_id = $workspace \
               AND source_block_id = type::record('loom_blocks', $source_key) \
               AND string::starts_with(edge_id, 'KDLNK-') \
               AND last_actor_kind = 'SYSTEM' \
               AND last_actor_id = 'knowledge_rich_document_backlink_projection' \
               AND edit_event_id = '00000000-0000-0000-0000-000000000000' \
               AND source_document_id = $source_key;"
                .to_owned(),
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_key)),
                b("source_key", source_document_id.to_owned()),
            ],
        )
        .await?;
    let prior_by_relationship: HashMap<String, String> = prior_rows
        .into_iter()
        .map(|row| (row.relationship_id, row.target))
        .collect();
    let prior_loom_targets = prior_loom_targets
        .into_iter()
        .map(record_key)
        .collect::<StorageResult<Vec<_>>>()?;
    Ok((prior_by_relationship, prior_loom_targets))
}

pub async fn replace_backlinks_attempt(
    storage: &impl DocumentQuery,
    source_document_id: &str,
    upserts: &[UpsertKnowledgeDocumentBacklink],
) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
    let source = observe_result(
        "backlink_source_read",
        read_live_rich_document(storage, source_document_id),
    )
    .await?
    .ok_or(StorageError::NotFound("knowledge rich document"))?;
    if upserts.iter().any(|upsert| {
        upsert.source_document_id != source_document_id
            || upsert.workspace_id != source.workspace_id
    }) {
        return Err(StorageError::Validation(
            "knowledge backlink rebuild source/workspace mismatch",
        ));
    }
    let (prior_by_relationship, prior_loom_targets) = observe_result(
        "backlink_prior_read",
        read_prior_backlink_state(storage, &source.workspace_id, source_document_id),
    )
    .await?;
    let resolved = observe_result(
        "backlink_target_reads",
        resolve_backlink_rows(
            storage,
            &source.workspace_id,
            source_document_id,
            upserts.to_vec(),
            &prior_by_relationship,
            &prior_loom_targets,
        ),
    )
    .await?;
    let insertion_order: HashMap<String, usize> = resolved
        .iter()
        .enumerate()
        .map(|(index, row)| (row.relationship_id.clone(), index))
        .collect();
    let affected_blocks: BTreeSet<String> = prior_loom_targets
        .iter()
        .cloned()
        .chain(
            resolved
                .iter()
                .filter(|row| row.project_to_loom)
                .map(|row| row.target.clone()),
        )
        .chain(std::iter::once(source_document_id.to_owned()))
        .collect();
    // Statements: BEGIN(0) backlink-writes(1..5) final-select(6) COMMIT.
    let statement = format!(
        "BEGIN TRANSACTION;\n\
         {BACKLINK_WRITE_STATEMENTS}\n\
         SELECT * FROM knowledge_document_backlinks WHERE source_document_id = type::record('knowledge_rich_documents', $source_key);\n\
         COMMIT TRANSACTION;"
    );
    let binds = backlink_write_binds(
        thing(WORKSPACES_TABLE, &source.workspace_id),
        source_document_id,
        &resolved,
        &affected_blocks,
    );
    let rows: Vec<BacklinkRecord> = observe_result(
        "backlink_write",
        storage.rows(
            statement,
            binds,
            1 + BACKLINK_WRITE_STATEMENT_COUNT,
            Some(&BACKLINK_GUARDS),
        ),
    )
    .await?;
    let mut out = rows
        .into_iter()
        .map(backlink_to_domain)
        .collect::<StorageResult<Vec<_>>>()?;
    out.sort_by_key(|backlink| {
        insertion_order
            .get(&backlink.relationship_id)
            .copied()
            .unwrap_or(usize::MAX)
    });
    Ok(out)
}

const KNOWLEDGE_SOURCES_TABLE: &str = "knowledge_sources";
pub fn opt_thing(table: &str, key: Option<&str>) -> Option<RecordId> {
    key.map(|key| thing(table, key))
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct SourceRecord {
    pub source_id: String,
    pub workspace_id: RecordId,
    pub root_id: Option<RecordId>,
    pub source_kind: String,
    pub relative_path: Option<String>,
    pub asset_id: Option<RecordId>,
    pub loom_block_id: Option<RecordId>,
    pub document_id: Option<RecordId>,
    pub content_hash: String,
    pub size_bytes: Option<i64>,
    pub provenance: JsonValue,
    pub permission_scope: String,
    pub redaction_state: String,
    pub parser_status: String,
    pub extraction_status: String,
    pub stale: bool,
    pub last_index_receipt_event_id: Option<RecordId>,
    pub source_modified_at: Option<Datetime>,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

pub fn source_to_domain(record: SourceRecord) -> StorageResult<KnowledgeSource> {
    Ok(KnowledgeSource {
        source_id: record.source_id,
        workspace_id: record_key(record.workspace_id)?,
        root_id: opt_record_key(record.root_id)?,
        source_kind: record.source_kind.parse()?,
        relative_path: record.relative_path,
        asset_id: opt_record_key(record.asset_id)?,
        loom_block_id: opt_record_key(record.loom_block_id)?,
        document_id: opt_record_key(record.document_id)?,
        content_hash: record.content_hash,
        size_bytes: record.size_bytes,
        provenance: record.provenance,
        permission_scope: record.permission_scope.parse()?,
        redaction_state: record.redaction_state.parse()?,
        parser_status: record.parser_status.parse()?,
        extraction_status: record.extraction_status.parse()?,
        stale: record.stale,
        last_index_receipt_event_id: opt_record_key(record.last_index_receipt_event_id)?,
        source_modified_at: opt_time(record.source_modified_at),
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
#[surreal(crate = "surrealdb_types")]
pub struct EntityRecord {
    pub entity_id: String,
    pub workspace_id: RecordId,
    pub entity_kind: String,
    pub entity_key: String,
    pub display_name: String,
    pub detection_provenance: JsonValue,
    pub lifecycle_state: String,
    pub primary_source_id: Option<RecordId>,
    pub first_detected_in_run: Option<RecordId>,
    pub last_detected_in_run: Option<RecordId>,
    pub created_at: Datetime,
    pub updated_at: Datetime,
}

pub fn entity_to_domain(record: EntityRecord) -> StorageResult<KnowledgeEntity> {
    Ok(KnowledgeEntity {
        entity_id: record.entity_id,
        workspace_id: record_key(record.workspace_id)?,
        entity_kind: record.entity_kind.parse()?,
        entity_key: record.entity_key,
        display_name: record.display_name,
        detection_provenance: record.detection_provenance,
        lifecycle_state: record.lifecycle_state.parse()?,
        primary_source_id: opt_record_key(record.primary_source_id)?,
        first_detected_in_run: opt_record_key(record.first_detected_in_run)?,
        last_detected_in_run: opt_record_key(record.last_detected_in_run)?,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

pub async fn owned_source_upsert_rows(
    storage: &impl DocumentQuery,
    statement: &str,
    mut binds: Binds,
    source_id: String,
    workspace_id: String,
    has_relative_path: bool,
) -> StorageResult<Vec<SourceRecord>> {
    let Some(session_id) = storage.session_id() else {
        return query_rows(storage, statement, binds).await;
    };
    let existing: Vec<SourceRecord> = if has_relative_path {
        query_rows(
            storage,
            "SELECT * FROM knowledge_sources WHERE $relative_path != NONE AND workspace_id = $workspace AND root_id = $root_id AND relative_path = $relative_path;",
            binds.clone(),
        )
        .await?
    } else {
        Vec::new()
    };
    if existing.len() > 1 {
        return Err(StorageError::Validation("HSK-403-PROTECTED-RESOURCE"));
    }
    let result_id = existing
        .first()
        .map(|row| row.source_id.clone())
        .unwrap_or_else(|| source_id.clone());
    let owned_resources = if existing.is_empty() {
        vec![
            observe_result(
                "index_source_prepare",
                storage.prepare_source_resource(&source_id, &workspace_id),
            )
            .await?,
        ]
    } else {
        Vec::new()
    };
    binds.extend([
        b("creator", thing("authenticated_sessions", &session_id)),
        b("owned_resources", owned_resources),
        b("result_id", result_id),
    ]);
    let mutation = statement
        .replace("RETURN UPDATE", "UPDATE")
        .replace("RETURN CREATE", "CREATE")
        .replace("RETURN AFTER", "RETURN NONE")
        .replace(
            "WHERE root_id =",
            "WHERE workspace_id = $workspace AND root_id =",
        )
        .replace(
            "CONTENT { source_id:",
            "CONTENT { created_in_session_id: $creator, source_id:",
        );
    let authority = storage.source_authority_sql();
    let sql = format!("BEGIN TRANSACTION; {mutation} {authority} IF array::len((UPDATE knowledge_sources SET content_hash = $content_hash WHERE source_id = $result_id AND workspace_id = $workspace RETURN VALUE id)) != 1 {{ THROW 'HSK-403-PROTECTED-RESOURCE'; }}; SELECT * FROM type::record('knowledge_sources', $result_id) WHERE source_id = $result_id AND workspace_id = $workspace; COMMIT TRANSACTION;");
    // BEGIN, mutation IF, authority FOR, count fence IF, SELECT, COMMIT.
    // The host checks every result, including COMMIT, before decoding slot 4.
    observe_result(
        "index_source_transaction",
        storage.rows(sql, binds, 4, None),
    )
    .await
}

pub async fn get_knowledge_source_by_document_id(
    storage: &impl DocumentQuery,
    workspace_id: &str,
    document_id: &str,
) -> StorageResult<Option<KnowledgeSource>> {
    // Provenance-keyed rich-document linkage (MT-154): the `document_id`
    // column links the legacy `documents` table, so a RichDocument source
    // carries its id in `provenance.rich_document_id`.
    query_first_row::<SourceRecord>(
            storage,
            "SELECT * FROM knowledge_sources WHERE workspace_id = $workspace AND source_kind = 'rich_document' AND provenance.rich_document_id = $document_id ORDER BY created_at ASC LIMIT 1;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("document_id", document_id.to_owned()),
            ],
        )
        .await?
        .map(source_to_domain)
        .transpose()
}

pub async fn mark_knowledge_source_stale(
    storage: &impl DocumentQuery,
    source_id: &str,
) -> StorageResult<KnowledgeSource> {
    let rows: Vec<SourceRecord> = query_rows(
            storage,
            "UPDATE knowledge_sources SET stale = true, updated_at = time::now() WHERE source_id = $source_id RETURN AFTER;",
            vec![b("source_id", source_id.to_owned())],
        )
        .await?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::NotFound("knowledge source"))
        .and_then(source_to_domain)
}

pub async fn upsert_entity_attempt(
    storage: &impl DocumentQuery,
    new_entity: &NewKnowledgeEntity,
    entity_id: String,
) -> StorageResult<Vec<EntityRecord>> {
    storage.rows(

                        "BEGIN TRANSACTION;\n\
                         IF (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key LIMIT 1)[0] != NONE { UPDATE knowledge_entities SET display_name = $display_name, detection_provenance = $detection_provenance, primary_source_id = $primary_source_id ?? primary_source_id, last_detected_in_run = $detected_in_run ?? last_detected_in_run, lifecycle_state = 'active', updated_at = time::now() WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key RETURN NONE; } ELSE { CREATE type::record('knowledge_entities', $entity_id) CONTENT { entity_id: $entity_id, workspace_id: $workspace, entity_kind: $entity_kind, entity_key: $entity_key, display_name: $display_name, detection_provenance: $detection_provenance, primary_source_id: $primary_source_id, first_detected_in_run: $detected_in_run, last_detected_in_run: $detected_in_run } RETURN NONE; };\n\
                         FOR $span_id IN $evidence_span_ids { LET $entity = (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key LIMIT 1)[0]; IF (SELECT VALUE id FROM knowledge_entity_spans WHERE entity_id = $entity AND span_id = type::record('knowledge_spans', $span_id) LIMIT 1)[0] = NONE { CREATE knowledge_entity_spans CONTENT { entity_id: $entity, span_id: type::record('knowledge_spans', $span_id), detected_in_run: $detected_in_run } RETURN NONE; }; };\n\
                         SELECT * FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key LIMIT 1;\n\
                         COMMIT TRANSACTION;".to_owned(),
                        vec![
                            b("entity_id", entity_id),
                            b("workspace", thing(WORKSPACES_TABLE, &new_entity.workspace_id)),
                            b("entity_kind", new_entity.entity_kind.as_str().to_owned()),
                            b("entity_key", new_entity.entity_key.clone()),
                            b("display_name", new_entity.display_name.clone()),
                            b(
                                "detection_provenance",
                                new_entity.detection_provenance.clone(),
                            ),
                            b(
                                "primary_source_id",
                                opt_thing(
                                    KNOWLEDGE_SOURCES_TABLE,
                                    new_entity.primary_source_id.as_deref(),
                                ),
                            ),
                            b(
                                "detected_in_run",
                                opt_thing(
                                    KNOWLEDGE_INDEX_RUNS_TABLE,
                                    new_entity.detected_in_run.as_deref(),
                                ),
                            ),
                            b("evidence_span_ids", new_entity.evidence_span_ids.clone()),
                        ],
                        3,
                        None,
                    )
                    .await
}

pub const SOURCE_UPSERT_STATEMENT: &str = "IF $relative_path != NONE AND (SELECT VALUE id FROM knowledge_sources WHERE root_id = $root_id AND relative_path = $relative_path LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_sources SET content_hash = $content_hash, size_bytes = $size_bytes, provenance = $provenance, permission_scope = $permission_scope, redaction_state = $redaction_state, source_modified_at = $source_modified_at, parser_status = 'pending', extraction_status = 'pending', stale = false, updated_at = time::now() WHERE root_id = $root_id AND relative_path = $relative_path RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_sources', $source_id) CONTENT { source_id: $source_id, workspace_id: $workspace, root_id: $root_id, source_kind: $source_kind, relative_path: $relative_path, asset_id: $asset_id, loom_block_id: $loom_block_id, document_id: $document_id, content_hash: $content_hash, size_bytes: $size_bytes, provenance: $provenance, permission_scope: $permission_scope, redaction_state: $redaction_state, source_modified_at: $source_modified_at } RETURN AFTER; };";

const KNOWLEDGE_INDEX_RUNS_TABLE: &str = "knowledge_index_runs";
