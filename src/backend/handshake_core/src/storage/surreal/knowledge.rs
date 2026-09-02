//! Embedded-SurrealDB implementation of the WP-009 [`KnowledgeStore`] surface
//! (WP-KERNEL-012 MT-136).
//!
//! Semantics are implemented against canonical embedded SurrealDB while
//! retaining the storage invariants recorded at reference commit 1af216a1.
//! Three deliberate rules apply:
//!
//! 1. Multi-write operations that form one atomic unit are one
//!    `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` query string here, so
//!    they stay crash-atomic. Conditional guards inside those transactions
//!    `THROW` module-scoped `HSK-…` codes which map to typed [`StorageError`]
//!    values.
//! 2. The embedded RocksDB store is single-process by construction (the
//!    engine's `LOCK` file enforces it), so a process-local async mutex
//!    ([`RICH_DOCUMENT_MUTATION_LOCK`]) serializes rich-document mutation
//!    paths.
//! 3. Affected-row detection uses `RETURN AFTER` row counts: a conditional
//!    `UPDATE` that matched nothing yields zero rows, which is the lost-race
//!    signal (mirrors `rows_affected`).

use std::collections::{BTreeSet, HashMap, HashSet};
#[cfg(any(test, feature = "surreal-test-support"))]
use std::{cell::Cell, future::Future};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue, Value as SurrealValueData};
use tokio::sync::Mutex;

use super::{schema::parse_named_array, SurrealDatabase, SurrealStorage, SurrealStorageError};
use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    derive_knowledge_relationship_id, is_sha256_hex, knowledge_canonical_json_sha256,
    knowledge_request_hash, new_knowledge_id, normalize_repo_relative_path,
    parse_rich_document_version_result_ref_id, rich_document_crdt_id_change_requested,
    rich_document_loom_projection, rich_document_version_result_ref_id,
    validate_knowledge_idempotency_key, KnowledgeClaim, KnowledgeClaimConflict,
    KnowledgeClaimRetirement, KnowledgeClaimRetirementReason, KnowledgeClaimState,
    KnowledgeCodeFile, KnowledgeCodeLanguage, KnowledgeCodeParseStatus, KnowledgeCompactionPolicy,
    KnowledgeContextBundle, KnowledgeContextBundleItem, KnowledgeDocumentBacklink,
    KnowledgeDocumentEmbed, KnowledgeEdge, KnowledgeEdgeLifecycle, KnowledgeEdgeType,
    KnowledgeEditorCodeNode, KnowledgeEntity, KnowledgeEntityKind, KnowledgeExtractionStatus,
    KnowledgeIdempotentWrite, KnowledgeIndexRun, KnowledgeIndexRunCounts, KnowledgeIndexRunOutcome,
    KnowledgeIndexingEligibility, KnowledgeMemoryPassage, KnowledgeNamespaceAudit,
    KnowledgeParserStatus, KnowledgePassageEvidenceRef, KnowledgeRebuildStatus,
    KnowledgeRetrievalTrace, KnowledgeRichDocument, KnowledgeRichDocumentDraft,
    KnowledgeRichDocumentVersion, KnowledgeRichDocumentVersionMeta, KnowledgeSchemaRegistryRow,
    KnowledgeSource, KnowledgeSourceKind, KnowledgeSourceRoot, KnowledgeSpan, KnowledgeSpanKind,
    KnowledgeStore, KnowledgeWikiProjection, NewKnowledgeClaim, NewKnowledgeContextBundle,
    NewKnowledgeEdge, NewKnowledgeEntity, NewKnowledgeIndexRun, NewKnowledgeMemoryPassage,
    NewKnowledgeRichDocument, NewKnowledgeSource, NewKnowledgeSourceRoot, NewKnowledgeSpan,
    NewKnowledgeWikiPage, NewKnowledgeWikiProjection, UpsertEditorCodeNode,
    UpsertKnowledgeDocumentBacklink, UpsertKnowledgeDocumentEmbed,
    UpsertKnowledgeRichDocumentDraft, WikiCodeFileInput, WikiCrossSourceEdge, WikiEntityWithSpan,
    WikiLoomBlockState, RICH_DOCUMENT_RESULT_REF_KIND, RICH_DOCUMENT_VERSION_RESULT_REF_KIND,
};
use crate::storage::{StorageError, StorageResult};

const WORKSPACES_TABLE: &str = "workspaces";
const DOCUMENTS_TABLE: &str = "documents";
const ASSETS_TABLE: &str = "assets";
const LOOM_BLOCKS_TABLE: &str = "loom_blocks";
const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";
const KNOWLEDGE_SOURCE_ROOTS_TABLE: &str = "knowledge_source_roots";
const KNOWLEDGE_SOURCES_TABLE: &str = "knowledge_sources";
const KNOWLEDGE_INDEX_RUNS_TABLE: &str = "knowledge_index_runs";
const KNOWLEDGE_SPANS_TABLE: &str = "knowledge_spans";
const KNOWLEDGE_ENTITIES_TABLE: &str = "knowledge_entities";
const KNOWLEDGE_CLAIMS_TABLE: &str = "knowledge_claims";
const KNOWLEDGE_RICH_DOCUMENTS_TABLE: &str = "knowledge_rich_documents";
const KNOWLEDGE_CONTEXT_BUNDLES_TABLE: &str = "knowledge_context_bundles";
const KNOWLEDGE_CODE_FILES_TABLE: &str = "knowledge_code_files";
const KNOWLEDGE_ENTITY_SPANS_TABLE: &str = "knowledge_entity_spans";
const KNOWLEDGE_EDGES_TABLE: &str = "knowledge_edges";
const KNOWLEDGE_WIKI_PROJECTIONS_TABLE: &str = "knowledge_wiki_projections";

/// Serializes rich-document read-decide-write mutation paths inside the
/// single-process embedded store. One process-local mutex provides the needed
/// serialization guarantee (coarser, and therefore strictly safe).
static RICH_DOCUMENT_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());
/// Serializes identity-based knowledge upserts whose unique key is discovered
/// before creating a generated record id. This closes the select/create race
/// between parallel in-process agents while retaining the existing ids and
/// update semantics.
static KNOWLEDGE_UPSERT_LOCK: Mutex<()> = Mutex::const_new(());

fn map_err(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

/// Maps a transaction aborted by one of this module's `THROW` guard codes to
/// the typed error the removed backend raised for the same condition.
fn map_guarded_err(
    error: SurrealStorageError,
    guards: &[(&str, fn() -> StorageError)],
) -> StorageError {
    let rendered = error.to_string();
    for (code, to_error) in guards {
        if rendered.contains(code) {
            return to_error();
        }
    }
    StorageError::Database(rendered)
}

fn thing(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key.to_owned())
}

fn opt_thing(table: &str, key: Option<&str>) -> Option<RecordId> {
    key.map(|key| thing(table, key))
}

fn record_key(record_id: RecordId) -> StorageResult<String> {
    match record_id.key {
        RecordIdKey::String(id) => Ok(id),
        _ => Err(StorageError::Serialization(
            "knowledge record link is not a string key".to_owned(),
        )),
    }
}

fn opt_record_key(record_id: Option<RecordId>) -> StorageResult<Option<String>> {
    record_id.map(record_key).transpose()
}

fn opt_time(datetime: Option<Datetime>) -> Option<DateTime<Utc>> {
    datetime.map(Datetime::into_inner)
}

fn int_i32(value: i64, field: &'static str) -> StorageResult<i32> {
    i32::try_from(value)
        .map_err(|_| StorageError::Serialization(format!("{field} exceeds the i32 range")))
}

fn opt_int_i32(value: Option<i64>, field: &'static str) -> StorageResult<Option<i32>> {
    value.map(|value| int_i32(value, field)).transpose()
}

// ---------------------------------------------------------------------------
// Row shapes (SurrealValue) and their domain conversions.
// ---------------------------------------------------------------------------

#[derive(SurrealValue)]
struct RegistryRecord {
    family_key: String,
    table_name: String,
    record_family: String,
    authority_class: String,
    schema_source: String,
    wp_id: String,
    mt_id: String,
    registered_at: Datetime,
}

fn registry_to_domain(record: RegistryRecord) -> StorageResult<KnowledgeSchemaRegistryRow> {
    Ok(KnowledgeSchemaRegistryRow {
        family_key: record.family_key,
        table_name: record.table_name,
        record_family: record.record_family,
        authority_class: record.authority_class.parse()?,
        schema_source: record.schema_source,
        wp_id: record.wp_id,
        mt_id: record.mt_id,
        registered_at: record.registered_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct RootRecord {
    root_id: String,
    workspace_id: RecordId,
    display_name: String,
    root_kind: String,
    repo_relative_path: String,
    path_normalization: String,
    allowlist_policy: JsonValue,
    indexing_eligibility: String,
    created_at: Datetime,
    updated_at: Datetime,
}

fn root_to_domain(record: RootRecord) -> StorageResult<KnowledgeSourceRoot> {
    Ok(KnowledgeSourceRoot {
        root_id: record.root_id,
        workspace_id: record_key(record.workspace_id)?,
        display_name: record.display_name,
        root_kind: record.root_kind.parse()?,
        repo_relative_path: record.repo_relative_path,
        path_normalization: record.path_normalization,
        allowlist_policy: record.allowlist_policy,
        indexing_eligibility: record.indexing_eligibility.parse()?,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct SourceRecord {
    source_id: String,
    workspace_id: RecordId,
    root_id: Option<RecordId>,
    source_kind: String,
    relative_path: Option<String>,
    asset_id: Option<RecordId>,
    loom_block_id: Option<RecordId>,
    document_id: Option<RecordId>,
    content_hash: String,
    size_bytes: Option<i64>,
    provenance: JsonValue,
    permission_scope: String,
    redaction_state: String,
    parser_status: String,
    extraction_status: String,
    stale: bool,
    last_index_receipt_event_id: Option<RecordId>,
    source_modified_at: Option<Datetime>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn source_to_domain(record: SourceRecord) -> StorageResult<KnowledgeSource> {
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
struct RunRecord {
    index_run_id: String,
    workspace_id: RecordId,
    root_id: Option<RecordId>,
    run_state: String,
    scope: JsonValue,
    actor_kind: String,
    actor_id: String,
    worktree_id: Option<String>,
    restart_checkpoint: Option<JsonValue>,
    sources_seen: i64,
    sources_indexed: i64,
    spans_extracted: i64,
    entities_detected: i64,
    edges_written: i64,
    claims_written: i64,
    error_capture: Option<JsonValue>,
    start_receipt_event_id: Option<RecordId>,
    finish_receipt_event_id: Option<RecordId>,
    started_at: Datetime,
    finished_at: Option<Datetime>,
}

fn run_to_domain(record: RunRecord) -> StorageResult<KnowledgeIndexRun> {
    Ok(KnowledgeIndexRun {
        index_run_id: record.index_run_id,
        workspace_id: record_key(record.workspace_id)?,
        root_id: opt_record_key(record.root_id)?,
        run_state: record.run_state.parse()?,
        scope: record.scope,
        actor_kind: record.actor_kind,
        actor_id: record.actor_id,
        worktree_id: record.worktree_id,
        restart_checkpoint: record.restart_checkpoint,
        counts: KnowledgeIndexRunCounts {
            sources_seen: int_i32(record.sources_seen, "sources_seen")?,
            sources_indexed: int_i32(record.sources_indexed, "sources_indexed")?,
            spans_extracted: int_i32(record.spans_extracted, "spans_extracted")?,
            entities_detected: int_i32(record.entities_detected, "entities_detected")?,
            edges_written: int_i32(record.edges_written, "edges_written")?,
            claims_written: int_i32(record.claims_written, "claims_written")?,
        },
        error_capture: record.error_capture,
        start_receipt_event_id: opt_record_key(record.start_receipt_event_id)?,
        finish_receipt_event_id: opt_record_key(record.finish_receipt_event_id)?,
        started_at: record.started_at.into_inner(),
        finished_at: opt_time(record.finished_at),
    })
}

#[derive(SurrealValue)]
struct SpanRecord {
    span_id: String,
    source_id: RecordId,
    span_kind: String,
    range_start: i64,
    range_end: i64,
    line_start: Option<i64>,
    line_end: Option<i64>,
    section_path: Option<String>,
    content_sha256: String,
    parser_version: String,
    extraction_receipt_event_id: Option<RecordId>,
    index_run_id: Option<RecordId>,
    display_snippet: Option<String>,
    created_at: Datetime,
}

fn span_to_domain(record: SpanRecord) -> StorageResult<KnowledgeSpan> {
    Ok(KnowledgeSpan {
        span_id: record.span_id,
        source_id: record_key(record.source_id)?,
        span_kind: record.span_kind.parse()?,
        range_start: record.range_start,
        range_end: record.range_end,
        line_start: opt_int_i32(record.line_start, "line_start")?,
        line_end: opt_int_i32(record.line_end, "line_end")?,
        section_path: record.section_path,
        content_sha256: record.content_sha256,
        parser_version: record.parser_version,
        extraction_receipt_event_id: opt_record_key(record.extraction_receipt_event_id)?,
        index_run_id: opt_record_key(record.index_run_id)?,
        display_snippet: record.display_snippet,
        created_at: record.created_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct EntityRecord {
    entity_id: String,
    workspace_id: RecordId,
    entity_kind: String,
    entity_key: String,
    display_name: String,
    detection_provenance: JsonValue,
    lifecycle_state: String,
    primary_source_id: Option<RecordId>,
    first_detected_in_run: Option<RecordId>,
    last_detected_in_run: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn entity_to_domain(record: EntityRecord) -> StorageResult<KnowledgeEntity> {
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

#[derive(SurrealValue)]
struct EdgeRecord {
    edge_id: String,
    workspace_id: RecordId,
    relationship_id: String,
    edge_type: String,
    source_entity_id: RecordId,
    target_entity_id: RecordId,
    extractor_version: String,
    lifecycle_state: String,
    confidence: f64,
    conflict_marker: Option<JsonValue>,
    created_in_run: Option<RecordId>,
    last_seen_in_run: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn edge_to_domain(record: EdgeRecord) -> StorageResult<KnowledgeEdge> {
    Ok(KnowledgeEdge {
        edge_id: record.edge_id,
        workspace_id: record_key(record.workspace_id)?,
        relationship_id: record.relationship_id,
        edge_type: record.edge_type.parse()?,
        source_entity_id: record_key(record.source_entity_id)?,
        target_entity_id: record_key(record.target_entity_id)?,
        extractor_version: record.extractor_version,
        lifecycle_state: record.lifecycle_state.parse()?,
        confidence: record.confidence,
        conflict_marker: record.conflict_marker,
        created_in_run: opt_record_key(record.created_in_run)?,
        last_seen_in_run: opt_record_key(record.last_seen_in_run)?,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct ClaimRecord {
    claim_id: String,
    workspace_id: RecordId,
    claim_kind: String,
    claim_text: String,
    subject_entity_id: Option<RecordId>,
    lifecycle_state: String,
    temporal_qualifier: Option<JsonValue>,
    granularity_qualifier: Option<String>,
    confidence: f64,
    retirement_reason: Option<String>,
    superseded_by_claim_id: Option<RecordId>,
    proposed_in_run: Option<RecordId>,
    resolution_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn claim_to_domain(record: ClaimRecord) -> StorageResult<KnowledgeClaim> {
    Ok(KnowledgeClaim {
        claim_id: record.claim_id,
        workspace_id: record_key(record.workspace_id)?,
        claim_kind: record.claim_kind.parse()?,
        claim_text: record.claim_text,
        subject_entity_id: opt_record_key(record.subject_entity_id)?,
        lifecycle_state: record.lifecycle_state.parse()?,
        temporal_qualifier: record.temporal_qualifier,
        granularity_qualifier: record.granularity_qualifier,
        confidence: record.confidence,
        retirement_reason: record
            .retirement_reason
            .map(|reason| reason.parse::<KnowledgeClaimRetirementReason>())
            .transpose()?,
        superseded_by_claim_id: opt_record_key(record.superseded_by_claim_id)?,
        proposed_in_run: opt_record_key(record.proposed_in_run)?,
        resolution_receipt_event_id: opt_record_key(record.resolution_receipt_event_id)?,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct ConflictRecord {
    conflict_id: String,
    claim_id: RecordId,
    conflicting_claim_id: RecordId,
    detected_in_run: Option<RecordId>,
    conflict_reason: String,
    resolution_receipt_event_id: Option<RecordId>,
    detected_at: Datetime,
    resolved_at: Option<Datetime>,
}

fn conflict_to_domain(record: ConflictRecord) -> StorageResult<KnowledgeClaimConflict> {
    Ok(KnowledgeClaimConflict {
        conflict_id: record.conflict_id,
        claim_id: record_key(record.claim_id)?,
        conflicting_claim_id: record_key(record.conflicting_claim_id)?,
        detected_in_run: opt_record_key(record.detected_in_run)?,
        conflict_reason: record.conflict_reason,
        resolution_receipt_event_id: opt_record_key(record.resolution_receipt_event_id)?,
        detected_at: record.detected_at.into_inner(),
        resolved_at: opt_time(record.resolved_at),
    })
}

#[derive(SurrealValue)]
struct PassageRecord {
    passage_id: String,
    workspace_id: RecordId,
    passage_text: String,
    token_count: Option<i64>,
    ocr_transcript_metadata: Option<JsonValue>,
    extraction_confidence: f64,
    ranking_features: JsonValue,
    retrieval_mode: String,
    freshness_at: Datetime,
    compaction_policy: String,
    failure_receipt_event_id: Option<RecordId>,
    derived_in_run: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn passage_to_domain(record: PassageRecord) -> StorageResult<KnowledgeMemoryPassage> {
    Ok(KnowledgeMemoryPassage {
        passage_id: record.passage_id,
        workspace_id: record_key(record.workspace_id)?,
        passage_text: record.passage_text,
        token_count: opt_int_i32(record.token_count, "token_count")?,
        ocr_transcript_metadata: record.ocr_transcript_metadata,
        extraction_confidence: record.extraction_confidence,
        ranking_features: record.ranking_features,
        retrieval_mode: record.retrieval_mode.parse()?,
        freshness_at: record.freshness_at.into_inner(),
        compaction_policy: record.compaction_policy.parse()?,
        failure_receipt_event_id: opt_record_key(record.failure_receipt_event_id)?,
        derived_in_run: opt_record_key(record.derived_in_run)?,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct EvidenceRecord {
    ref_kind: String,
    source_id: Option<RecordId>,
    claim_id: Option<RecordId>,
    span_id: Option<RecordId>,
}

fn evidence_to_domain(record: EvidenceRecord) -> StorageResult<KnowledgePassageEvidenceRef> {
    match record.ref_kind.as_str() {
        "source" => Ok(KnowledgePassageEvidenceRef::Source {
            source_id: opt_record_key(record.source_id)?.ok_or(StorageError::Validation(
                "passage evidence row missing source_id",
            ))?,
        }),
        "claim" => Ok(KnowledgePassageEvidenceRef::Claim {
            claim_id: opt_record_key(record.claim_id)?.ok_or(StorageError::Validation(
                "passage evidence row missing claim_id",
            ))?,
        }),
        "span" => Ok(KnowledgePassageEvidenceRef::Span {
            span_id: opt_record_key(record.span_id)?.ok_or(StorageError::Validation(
                "passage evidence row missing span_id",
            ))?,
        }),
        _ => Err(StorageError::Validation(
            "invalid knowledge passage evidence ref_kind",
        )),
    }
}

#[derive(SurrealValue)]
struct ProjectionRecord {
    projection_id: String,
    workspace_id: RecordId,
    projection_kind: String,
    title: String,
    source_records: JsonValue,
    rendered_content: String,
    rebuild_status: String,
    staleness_hash: String,
    rebuild_receipt_event_id: Option<RecordId>,
    last_rebuilt_at: Option<Datetime>,
    page_type: Option<String>,
    compile_stamp: Option<JsonValue>,
    compile_recipe: Option<JsonValue>,
    page_links: JsonValue,
    created_at: Datetime,
    updated_at: Datetime,
}

fn projection_to_domain(record: ProjectionRecord) -> StorageResult<KnowledgeWikiProjection> {
    Ok(KnowledgeWikiProjection {
        projection_id: record.projection_id,
        workspace_id: record_key(record.workspace_id)?,
        projection_kind: record.projection_kind.parse()?,
        title: record.title,
        source_records: record.source_records,
        rendered_content: record.rendered_content,
        rebuild_status: record.rebuild_status.parse()?,
        staleness_hash: record.staleness_hash,
        rebuild_receipt_event_id: opt_record_key(record.rebuild_receipt_event_id)?,
        last_rebuilt_at: opt_time(record.last_rebuilt_at),
        page_type: record.page_type,
        compile_stamp: record.compile_stamp,
        compile_recipe: record.compile_recipe,
        page_links: record.page_links,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct WikiCodeFileRecord {
    code_file_id: String,
    source_id: RecordId,
    language: String,
    parse_status: String,
    stale: bool,
    symbols_indexed: i64,
}

#[derive(SurrealValue)]
struct KnowledgeCodeFileRecord {
    code_file_id: String,
    workspace_id: RecordId,
    source_id: RecordId,
    file_entity_id: Option<RecordId>,
    language: String,
    indexed_content_hash: String,
    parser_version: String,
    parse_status: String,
    stale: bool,
    symbols_indexed: i64,
    edges_indexed: i64,
    failure_detail: Option<JsonValue>,
    last_indexed_in_run: Option<RecordId>,
    last_index_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn knowledge_code_file_to_domain(
    record: KnowledgeCodeFileRecord,
) -> StorageResult<KnowledgeCodeFile> {
    Ok(KnowledgeCodeFile {
        code_file_id: record.code_file_id,
        workspace_id: record_key(record.workspace_id)?,
        source_id: record_key(record.source_id)?,
        file_entity_id: opt_record_key(record.file_entity_id)?,
        language: record.language.parse()?,
        indexed_content_hash: record.indexed_content_hash,
        parser_version: record.parser_version,
        parse_status: record.parse_status.parse()?,
        stale: record.stale,
        symbols_indexed: int_i32(record.symbols_indexed, "symbols_indexed")?,
        edges_indexed: int_i32(record.edges_indexed, "edges_indexed")?,
        failure_detail: record.failure_detail,
        last_indexed_in_run: opt_record_key(record.last_indexed_in_run)?,
        last_index_receipt_event_id: opt_record_key(record.last_index_receipt_event_id)?,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct EntitySpanLinkRecord {
    span_id: RecordId,
}

#[derive(SurrealValue)]
struct WikiEntitySpanLinkRecord {
    entity_id: RecordId,
    span_id: RecordId,
}

#[derive(Clone, SurrealValue)]
struct WikiSpanRecord {
    span_id: String,
    content_sha256: String,
    line_start: Option<i64>,
    line_end: Option<i64>,
    section_path: Option<String>,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct WikiCrossSourceEdgeRecord {
    edge_type: String,
    from_source_id: RecordId,
    to_source_id: RecordId,
}

#[derive(SurrealValue)]
struct WikiLoomBlockRecord {
    block_id: String,
    title: Option<String>,
    content_type: String,
    derived_json: JsonValue,
    document_id: Option<RecordId>,
    asset_id: Option<RecordId>,
    content_hash: Option<String>,
}

#[derive(SurrealValue)]
struct WikiSourceHashRecord {
    source_id: String,
    content_hash: String,
}

#[derive(SurrealValue)]
struct WikiRichDocumentHashRecord {
    rich_document_id: String,
    content_sha256: String,
}

#[derive(SurrealValue)]
struct WikiLedgerVersionRecord {
    event_sequence: i64,
}

#[derive(SurrealValue)]
struct RichDocRecord {
    rich_document_id: String,
    workspace_id: RecordId,
    document_id: Option<RecordId>,
    title: String,
    schema_version: String,
    doc_version: i64,
    content_json: JsonValue,
    content_sha256: String,
    crdt_document_id: Option<String>,
    crdt_snapshot_id: Option<String>,
    promotion_receipt_event_id: Option<RecordId>,
    projection_refs: JsonValue,
    project_ref: Option<String>,
    folder_ref: Option<String>,
    authority_label: String,
    owner_actor_kind: Option<String>,
    owner_actor_id: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct RichDocumentDeleteEventRow {
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
}

#[derive(SurrealValue)]
struct RichDocumentDeleteTombstoneRow {
    deleted_at: Option<Datetime>,
    deleted_receipt_event_id: Option<RecordId>,
    projection_refs: JsonValue,
}

/// Durable effects returned by the atomic rich-document deletion boundary.
///
/// This is intentionally an inherent embedded-store surface instead of a
/// `KnowledgeStore` method: deletion owns EventLedger, RichDocument, source,
/// backlink, Loom, and Canvas records in one transaction and must not be
/// decomposed by a provider-neutral caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KnowledgeRichDocumentDeleteOutcome {
    pub(crate) receipt_event_id: String,
    pub(crate) source_marked_stale: bool,
    pub(crate) backlinks_deleted: usize,
    pub(crate) loom_block_deleted: bool,
}

fn rich_document_to_domain(record: RichDocRecord) -> StorageResult<KnowledgeRichDocument> {
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
struct VersionRecord {
    rich_document_id: RecordId,
    doc_version: i64,
    schema_version: String,
    content_json: JsonValue,
    content_sha256: String,
    crdt_snapshot_id: Option<String>,
    promotion_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
}

fn version_to_domain(record: VersionRecord) -> StorageResult<KnowledgeRichDocumentVersion> {
    Ok(KnowledgeRichDocumentVersion {
        rich_document_id: record_key(record.rich_document_id)?,
        doc_version: record.doc_version,
        schema_version: record.schema_version,
        content_json: record.content_json,
        content_sha256: record.content_sha256,
        crdt_snapshot_id: record.crdt_snapshot_id,
        promotion_receipt_event_id: opt_record_key(record.promotion_receipt_event_id)?,
        created_at: record.created_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct VersionMetaRecord {
    rich_document_id: RecordId,
    doc_version: i64,
    schema_version: String,
    content_sha256: String,
    crdt_snapshot_id: Option<String>,
    promotion_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
}

fn version_meta_to_domain(
    record: VersionMetaRecord,
) -> StorageResult<KnowledgeRichDocumentVersionMeta> {
    Ok(KnowledgeRichDocumentVersionMeta {
        rich_document_id: record_key(record.rich_document_id)?,
        doc_version: record.doc_version,
        schema_version: record.schema_version,
        content_sha256: record.content_sha256,
        crdt_snapshot_id: record.crdt_snapshot_id,
        promotion_receipt_event_id: opt_record_key(record.promotion_receipt_event_id)?,
        created_at: record.created_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct DraftRecord {
    rich_document_id: RecordId,
    workspace_id: RecordId,
    base_doc_version: i64,
    base_content_sha256: String,
    draft_content_json: JsonValue,
    draft_content_sha256: String,
    actor_kind: String,
    actor_id: String,
    kernel_task_run_id: String,
    session_run_id: String,
    created_at: Datetime,
    updated_at: Datetime,
}

fn draft_to_domain(record: DraftRecord) -> StorageResult<KnowledgeRichDocumentDraft> {
    Ok(KnowledgeRichDocumentDraft {
        rich_document_id: record_key(record.rich_document_id)?,
        workspace_id: record_key(record.workspace_id)?,
        base_doc_version: record.base_doc_version,
        base_content_sha256: record.base_content_sha256,
        draft_content_json: record.draft_content_json,
        draft_content_sha256: record.draft_content_sha256,
        actor_kind: record.actor_kind,
        actor_id: record.actor_id,
        kernel_task_run_id: record.kernel_task_run_id,
        session_run_id: record.session_run_id,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct CodeNodeRecord {
    code_node_id: String,
    rich_document_id: RecordId,
    node_path: String,
    language_id: String,
    code_text: String,
    round_trip_sha256: String,
    worker_requirements: JsonValue,
    source_mapping: Option<JsonValue>,
    lint_diagnostics: JsonValue,
    created_at: Datetime,
    updated_at: Datetime,
}

fn code_node_to_domain(record: CodeNodeRecord) -> StorageResult<KnowledgeEditorCodeNode> {
    Ok(KnowledgeEditorCodeNode {
        code_node_id: record.code_node_id,
        rich_document_id: record_key(record.rich_document_id)?,
        node_path: record.node_path,
        language_id: record.language_id,
        code_text: record.code_text,
        round_trip_sha256: record.round_trip_sha256,
        worker_requirements: record.worker_requirements,
        source_mapping: record.source_mapping,
        lint_diagnostics: record.lint_diagnostics,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct EmbedRecord {
    embed_id: String,
    rich_document_id: RecordId,
    block_id: String,
    ref_kind: String,
    ref_value: String,
    caption: Option<String>,
    repair_state: String,
    repair_reason: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn embed_to_domain(record: EmbedRecord) -> StorageResult<KnowledgeDocumentEmbed> {
    Ok(KnowledgeDocumentEmbed {
        embed_id: record.embed_id,
        rich_document_id: record_key(record.rich_document_id)?,
        block_id: record.block_id,
        ref_kind: record.ref_kind,
        ref_value: record.ref_value,
        caption: record.caption,
        repair_state: record.repair_state,
        repair_reason: record.repair_reason,
        created_at: record.created_at.into_inner(),
        updated_at: record.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct BacklinkRecord {
    backlink_id: String,
    workspace_id: RecordId,
    relationship_id: String,
    source_document_id: RecordId,
    link_kind: String,
    target: String,
    block_id: String,
    created_at: Datetime,
    updated_at: Datetime,
}

fn backlink_to_domain(record: BacklinkRecord) -> StorageResult<KnowledgeDocumentBacklink> {
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
struct BundleRecord {
    bundle_id: String,
    workspace_id: RecordId,
    kernel_task_run_id: String,
    session_run_id: String,
    allowed_context: JsonValue,
    context_hash: String,
    query_text: Option<String>,
    token_budget: Option<i64>,
    tokens_used: Option<i64>,
    build_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
}

fn bundle_to_domain(record: BundleRecord) -> StorageResult<KnowledgeContextBundle> {
    Ok(KnowledgeContextBundle {
        bundle_id: record.bundle_id,
        workspace_id: record_key(record.workspace_id)?,
        kernel_task_run_id: record.kernel_task_run_id,
        session_run_id: record.session_run_id,
        allowed_context: record.allowed_context,
        context_hash: record.context_hash,
        query_text: record.query_text,
        token_budget: opt_int_i32(record.token_budget, "token_budget")?,
        tokens_used: opt_int_i32(record.tokens_used, "tokens_used")?,
        build_receipt_event_id: opt_record_key(record.build_receipt_event_id)?,
        created_at: record.created_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct BundleItemRecord {
    bundle_id: RecordId,
    item_ordinal: i64,
    ref_kind: String,
    ref_id: String,
    retrieval_decision: String,
    relevance_score: Option<f64>,
    token_count: Option<i64>,
    citation: Option<String>,
    supported: bool,
    unsupported_reason: Option<String>,
}

fn bundle_item_to_domain(record: BundleItemRecord) -> StorageResult<KnowledgeContextBundleItem> {
    Ok(KnowledgeContextBundleItem {
        bundle_id: record_key(record.bundle_id)?,
        item_ordinal: int_i32(record.item_ordinal, "item_ordinal")?,
        ref_kind: record.ref_kind.parse()?,
        ref_id: record.ref_id,
        retrieval_decision: record.retrieval_decision.parse()?,
        relevance_score: record.relevance_score,
        token_count: opt_int_i32(record.token_count, "token_count")?,
        citation: record.citation,
        supported: record.supported,
        unsupported_reason: record.unsupported_reason,
    })
}

#[derive(SurrealValue)]
struct TraceRecord {
    trace_id: String,
    workspace_id: RecordId,
    retrieval_mode: String,
    mode_reason: String,
    query_text: Option<String>,
    bundle_id: Option<RecordId>,
    decisions: JsonValue,
    trace_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
}

fn trace_to_domain(record: TraceRecord) -> StorageResult<KnowledgeRetrievalTrace> {
    Ok(KnowledgeRetrievalTrace {
        trace_id: record.trace_id,
        workspace_id: record_key(record.workspace_id)?,
        retrieval_mode: record.retrieval_mode.parse()?,
        mode_reason: record.mode_reason,
        query_text: record.query_text,
        bundle_id: opt_record_key(record.bundle_id)?,
        decisions: record.decisions,
        trace_receipt_event_id: opt_record_key(record.trace_receipt_event_id)?,
        created_at: record.created_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct IdemKeyRecord {
    request_hash: String,
    result_ref_kind: String,
    result_ref_id: String,
}

#[derive(SurrealValue)]
struct LedgerAggregateRecord {
    aggregate_type: String,
    aggregate_id: String,
}

// ---------------------------------------------------------------------------
// Query plumbing: one bound query through the lifecycle lease, one result set.
// ---------------------------------------------------------------------------

type Binds = Vec<(String, SurrealValueData)>;

#[cfg(any(test, feature = "surreal-test-support"))]
tokio::task_local! {
    static KNOWLEDGE_QUERY_COUNT: Cell<usize>;
}

/// Measures actual queries crossing this module's embedded-store boundary for
/// one proof operation. Task-local scope prevents unrelated concurrent tests
/// or runtime work from contaminating the count.
#[cfg(any(test, feature = "surreal-test-support"))]
pub(super) async fn measure_knowledge_store_queries<F, Fut, T>(operation: F) -> (T, usize)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    KNOWLEDGE_QUERY_COUNT
        .scope(Cell::new(0), async move {
            let output = operation().await;
            let count = KNOWLEDGE_QUERY_COUNT.with(Cell::get);
            (output, count)
        })
        .await
}

fn b(name: &str, value: impl SurrealValue) -> (String, SurrealValueData) {
    (name.to_owned(), value.into_value())
}

/// Runs one (possibly multi-statement) query and returns the rows of the
/// result set at `index`, without translating the error. Statement indexes
/// count every `;`-terminated statement including `BEGIN TRANSACTION`
/// (matching `storage/surreal/blocks.rs`); top-level `LET` is never used in
/// this module, so indexes stay mechanical.
async fn raw_rows_at<R>(
    storage: &SurrealStorage,
    statement: impl Into<String>,
    binds: Binds,
    index: usize,
) -> Result<Vec<R>, SurrealStorageError>
where
    R: SurrealValue + Send + 'static,
{
    let statement = statement.into();
    #[cfg(any(test, feature = "surreal-test-support"))]
    let _ = KNOWLEDGE_QUERY_COUNT.try_with(|count| count.set(count.get() + 1));
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                let mut query = database.client.query(statement);
                for (name, value) in binds {
                    query = query.bind((name, value));
                }
                let mut response = query.await?.check()?;
                Ok(response.take(index)?)
            })
        })
        .await
}

async fn query_rows<R>(
    storage: &SurrealStorage,
    statement: impl Into<String>,
    binds: Binds,
) -> StorageResult<Vec<R>>
where
    R: SurrealValue + Send + 'static,
{
    raw_rows_at(storage, statement, binds, 0)
        .await
        .map_err(map_err)
}

async fn query_first_row<R>(
    storage: &SurrealStorage,
    statement: impl Into<String>,
    binds: Binds,
) -> StorageResult<Option<R>>
where
    R: SurrealValue + Send + 'static,
{
    Ok(query_rows::<R>(storage, statement, binds)
        .await?
        .into_iter()
        .next())
}

// ---------------------------------------------------------------------------
// Rich-document projection fragments (MT-032). One statement each; used inside
// the create/save/rename transactions. The nested IF + THROW retains fail-closed
// identity behavior when an existing block does not match the expected owner.
// ---------------------------------------------------------------------------

const LOOM_PROJECTION_STATEMENT: &str = "IF (SELECT VALUE id FROM loom_blocks WHERE block_id = $doc_block_id LIMIT 1)[0] != NONE { IF (SELECT VALUE id FROM loom_blocks WHERE block_id = $doc_block_id AND workspace_id = $workspace AND content_type = 'note' LIMIT 1)[0] = NONE { THROW 'HSK-KRD-LOOM-IDENTITY'; }; UPDATE loom_blocks SET title = $doc_title, content_hash = $doc_content_sha256, derived_json = $doc_derived_json, last_actor_kind = $loom_actor_kind, last_actor_id = $loom_actor_id, edit_event_id = $loom_edit_event_id, updated_at = time::now() WHERE block_id = $doc_block_id RETURN NONE; } ELSE { CREATE type::record('loom_blocks', $doc_block_id) CONTENT { block_id: $doc_block_id, workspace_id: $workspace, content_type: 'note', title: $doc_title, content_hash: $doc_content_sha256, derived_json: $doc_derived_json, last_actor_kind: $loom_actor_kind, last_actor_id: $loom_actor_id, edit_event_id: $loom_edit_event_id } RETURN NONE; };";

const SEARCH_PROJECTION_STATEMENT: &str = "IF (SELECT VALUE id FROM loom_block_search_index WHERE block_id = type::record('loom_blocks', $doc_block_id) LIMIT 1)[0] != NONE { IF (SELECT VALUE id FROM loom_block_search_index WHERE block_id = type::record('loom_blocks', $doc_block_id) AND workspace_id = $workspace AND content_type = 'note' LIMIT 1)[0] = NONE { THROW 'HSK-KRD-SEARCH-IDENTITY'; }; UPDATE loom_block_search_index SET search_text = $doc_search_text, indexed_at = time::now() WHERE block_id = type::record('loom_blocks', $doc_block_id) RETURN NONE; } ELSE { CREATE type::record('loom_block_search_index', type::record('loom_blocks', $doc_block_id)) CONTENT { block_id: type::record('loom_blocks', $doc_block_id), workspace_id: $workspace, content_type: 'note', search_text: $doc_search_text } RETURN NONE; };";

const LOOM_IDENTITY_GUARDS: [(&str, fn() -> StorageError); 2] = [
    ("HSK-KRD-LOOM-IDENTITY", || {
        StorageError::Conflict("rich document LoomBlock projection identity mismatch")
    }),
    ("HSK-KRD-SEARCH-IDENTITY", || {
        StorageError::Conflict("rich document LoomBlock search projection identity mismatch")
    }),
];

/// Binds shared by every transaction embedding the two projection statements.
#[allow(clippy::too_many_arguments)]
fn projection_binds(
    document_id: &str,
    workspace: RecordId,
    title: &str,
    content_sha256: &str,
    derived_json: JsonValue,
    search_text: &str,
    loom_actor_kind: &str,
    loom_actor_id: Option<String>,
) -> Binds {
    vec![
        b("doc_block_id", document_id.to_owned()),
        b("workspace", workspace),
        b("doc_title", title.to_owned()),
        b("doc_content_sha256", content_sha256.to_owned()),
        b("doc_derived_json", derived_json),
        b("doc_search_text", search_text.to_owned()),
        b("loom_actor_kind", loom_actor_kind.to_owned()),
        b("loom_actor_id", loom_actor_id),
        b("loom_edit_event_id", uuid::Uuid::now_v7().to_string()),
    ]
}

fn loom_projection_inputs(
    title: &str,
    content_json: &JsonValue,
) -> StorageResult<(JsonValue, String)> {
    let (derived_json, search_text) = rich_document_loom_projection(title, content_json)?;
    let derived_json: JsonValue = serde_json::from_str(&derived_json)?;
    Ok((derived_json, search_text))
}

// ---------------------------------------------------------------------------
// MT-032 backlink rebuild: read + resolve under RICH_DOCUMENT_MUTATION_LOCK,
// then apply every write in ONE transaction so the rebuild stays crash-atomic.
// ---------------------------------------------------------------------------

#[derive(SurrealValue)]
struct PriorBacklinkRecord {
    relationship_id: String,
    target: String,
}

#[derive(SurrealValue)]
struct CandidateDocRecord {
    rich_document_id: String,
    title: String,
    is_live: bool,
}

#[derive(SurrealValue)]
struct CandidateLoomRecord {
    block_id: String,
    workspace_id: RecordId,
}

struct ResolvedBacklink {
    backlink_id: String,
    relationship_id: String,
    link_kind: String,
    target: String,
    block_id: String,
    project_to_loom: bool,
}

/// The write half of the backlink rebuild, appended inside a transaction.
/// Five statements: owned-loom-edge delete, backlink delete, backlink create
/// loop, loom-edge create loop (fails closed on a foreign edge id), and the
/// affected-block count recomputation loop.
const BACKLINK_WRITE_STATEMENTS: &str = "DELETE loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $source_key) AND string::starts_with(edge_id, 'KDLNK-') AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND edit_event_id = '00000000-0000-0000-0000-000000000000' AND source_document_id = $source_key;\nDELETE knowledge_document_backlinks WHERE source_document_id = type::record('knowledge_rich_documents', $source_key);\nFOR $row IN $backlink_rows { CREATE type::record('knowledge_document_backlinks', $row.backlink_id) CONTENT { backlink_id: $row.backlink_id, workspace_id: $workspace, relationship_id: $row.relationship_id, source_document_id: type::record('knowledge_rich_documents', $source_key), link_kind: $row.link_kind, target: $row.target, block_id: $row.block_id } RETURN NONE; };\nFOR $row IN $loom_edge_rows { IF (SELECT VALUE id FROM loom_edges WHERE edge_id = $row.relationship_id LIMIT 1)[0] != NONE { THROW 'HSK-KDBL-LOOM-EDGE-OWNED'; }; CREATE loom_edges CONTENT { edge_id: $row.relationship_id, workspace_id: $workspace, source_block_id: type::record('loom_blocks', $source_key), target_block_id: type::record('loom_blocks', $row.target), edge_type: 'mention', created_by: 'user', last_actor_kind: 'SYSTEM', last_actor_id: 'knowledge_rich_document_backlink_projection', edit_event_id: '00000000-0000-0000-0000-000000000000', source_document_id: $source_key, source_text_block_id: $row.block_id } RETURN NONE; };\nFOR $affected IN $affected_blocks { UPDATE loom_blocks SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $affected) AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $affected) AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = type::record('loom_blocks', $affected) AND edge_type IN ['mention', 'tag'])) WHERE workspace_id = $workspace AND block_id = $affected RETURN NONE; };";

/// Number of `;`-terminated statements in [`BACKLINK_WRITE_STATEMENTS`].
const BACKLINK_WRITE_STATEMENT_COUNT: usize = 5;

const BACKLINK_GUARDS: [(&str, fn() -> StorageError); 1] = [("HSK-KDBL-LOOM-EDGE-OWNED", || {
    StorageError::Conflict("knowledge backlink Loom edge identity is owned by another writer")
})];

fn backlink_write_binds(
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

/// Ports the removed backend's wikilink resolution verbatim: exact live
/// same-workspace Loom identity wins, cross-workspace ids are dropped, KRD ids
/// must be live, ambiguous titles keep the prior live target or stay textual,
/// deleted titles drop the row, and a live RichDocument without its same-id
/// LoomBlock projection fails closed.
async fn resolve_backlink_rows(
    storage: &SurrealStorage,
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

    let candidate_targets: Vec<CandidateDocRecord> = query_rows(
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
    .await?;
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
    let candidate_loom_targets: Vec<CandidateLoomRecord> = query_rows(
        storage,
        "SELECT block_id, workspace_id FROM loom_blocks \
         WHERE block_id IN $candidate_loom_ids ORDER BY block_id ASC;",
        vec![b("candidate_loom_ids", candidate_loom_ids)],
    )
    .await?;
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
            continue;
        } else if upsert.link_kind == "wikilink" && upsert.target.starts_with("KRD-") {
            if !live_ids.contains(&upsert.target) {
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
                None if deleted_titles.contains(&upsert.target) => continue,
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

async fn read_live_rich_document(
    storage: &SurrealStorage,
    rich_document_id: &str,
) -> StorageResult<Option<KnowledgeRichDocument>> {
    query_first_row::<RichDocRecord>(
        storage,
        "SELECT * FROM knowledge_rich_documents \
         WHERE rich_document_id = $doc_id AND deleted_at = NONE;",
        vec![b("doc_id", rich_document_id.to_owned())],
    )
    .await?
    .map(rich_document_to_domain)
    .transpose()
}

fn delete_event_matches(
    stored: &RichDocumentDeleteEventRow,
    candidate: &super::event_ledger::LedgerWrite,
) -> bool {
    stored.event_version == candidate.event_version
        && stored.kernel_task_run_id == candidate.kernel_task_run_id
        && stored.session_run_id == candidate.session_run_id
        && stored.aggregate_type == candidate.aggregate_type
        && stored.aggregate_id == candidate.aggregate_id
        && stored.event_type == candidate.event_type
        && stored.actor_kind == candidate.actor_kind
        && stored.actor_id == candidate.actor_id
        && stored.causation_id == candidate.causation_id
        && stored.correlation_id == candidate.correlation_id
        && stored.payload_hash == candidate.payload_hash
        && stored.source_component == candidate.source_component
}

fn delete_outcome_from_tombstone(
    tombstone: RichDocumentDeleteTombstoneRow,
    receipt_event_id: &str,
) -> StorageResult<KnowledgeRichDocumentDeleteOutcome> {
    if tombstone.deleted_at.is_none()
        || tombstone
            .deleted_receipt_event_id
            .map(record_key)
            .transpose()?
            .as_deref()
            != Some(receipt_event_id)
    {
        return Err(StorageError::Database(
            "rich document delete receipt is not linked to its tombstone".to_owned(),
        ));
    }
    let stored = tombstone
        .projection_refs
        .as_array()
        .and_then(|refs| {
            refs.iter().find(|value| {
                value.get("kind").and_then(JsonValue::as_str)
                    == Some("rich_document_delete_outcome_v1")
                    && value.get("receipt_event_id").and_then(JsonValue::as_str)
                        == Some(receipt_event_id)
            })
        })
        .ok_or_else(|| {
            StorageError::Database(
                "rich document tombstone is missing its delete outcome".to_owned(),
            )
        })?;
    let backlinks_deleted = stored
        .get("backlinks_deleted")
        .and_then(JsonValue::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| {
            StorageError::Database(
                "rich document tombstone has an invalid backlink delete count".to_owned(),
            )
        })?;
    let source_marked_stale = stored
        .get("source_marked_stale")
        .and_then(JsonValue::as_bool)
        .ok_or_else(|| {
            StorageError::Database(
                "rich document tombstone has an invalid source outcome".to_owned(),
            )
        })?;
    let loom_block_deleted = stored
        .get("loom_block_deleted")
        .and_then(JsonValue::as_bool)
        .ok_or_else(|| {
            StorageError::Database("rich document tombstone has an invalid Loom outcome".to_owned())
        })?;
    Ok(KnowledgeRichDocumentDeleteOutcome {
        receipt_event_id: receipt_event_id.to_owned(),
        source_marked_stale,
        backlinks_deleted,
        loom_block_deleted,
    })
}

async fn read_rich_document_delete_replay(
    storage: &SurrealStorage,
    rich_document_id: &str,
    candidate: &super::event_ledger::LedgerWrite,
) -> StorageResult<Option<KnowledgeRichDocumentDeleteOutcome>> {
    let stored = query_first_row::<RichDocumentDeleteEventRow>(
        storage,
        "SELECT event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, \
         aggregate_id, event_type, actor_kind, actor_id, causation_id, correlation_id, \
         payload_hash, source_component FROM kernel_event_ledger \
         WHERE idempotency_key = $idempotency_key LIMIT 1;",
        vec![b("idempotency_key", candidate.idempotency_key.clone())],
    )
    .await?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    if !delete_event_matches(&stored, candidate) {
        return Err(StorageError::Conflict(
            "kernel event idempotency key was reused with different event content",
        ));
    }
    let tombstone = query_first_row::<RichDocumentDeleteTombstoneRow>(
        storage,
        "SELECT deleted_at, deleted_receipt_event_id, projection_refs \
         FROM knowledge_rich_documents WHERE rich_document_id = $doc_id LIMIT 1;",
        vec![b("doc_id", rich_document_id.to_owned())],
    )
    .await?
    .ok_or_else(|| {
        StorageError::Database(
            "rich document delete receipt exists without its tombstone".to_owned(),
        )
    })?;
    delete_outcome_from_tombstone(tombstone, &stored.event_id).map(Some)
}

impl SurrealDatabase {
    /// Bounded code-symbol lookup for the navigation API.
    ///
    /// The workspace/kind equality prefix is forced through the existing
    /// `idx_knowledge_entities_workspace_kind` index. Name, prefix, and path
    /// matching are then evaluated by SurrealDB before the ordered `LIMIT`, so
    /// callers never materialize every symbol in a workspace and false-positive
    /// candidates cannot consume the result bound.
    pub(crate) async fn lookup_knowledge_code_symbols(
        &self,
        workspace_id: &str,
        name: Option<&str>,
        prefix: Option<&str>,
        path: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<KnowledgeEntity>> {
        const MAX_LIMIT: i64 = 500;
        let limit = limit.clamp(1, MAX_LIMIT);
        let name = name.unwrap_or_default();
        let prefix = prefix.unwrap_or_default().to_ascii_lowercase();
        let path = path.unwrap_or_default();

        // This expression exactly mirrors `symbol_simple_name` in the API:
        // take the last `#` segment, remove the `~` discriminator, then take
        // the last Rust `::` and JavaScript `.` path segments.
        let rows: Vec<EntityRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_entities WITH INDEX idx_knowledge_entities_workspace_kind \
             WHERE workspace_id = $workspace AND entity_kind = 'symbol' \
               AND ($name_enabled = false OR display_name = $name OR \
                    array::last(string::split(array::last(string::split(array::first(string::split(array::last(string::split(entity_key, '#')), '~')), '::')), '.')) = $name) \
               AND ($prefix_enabled = false OR string::starts_with(string::lowercase(display_name), $prefix) OR \
                    string::starts_with(string::lowercase(array::last(string::split(array::last(string::split(array::first(string::split(array::last(string::split(entity_key, '#')), '~')), '::')), '.'))), $prefix)) \
               AND ($path_enabled = false OR string::starts_with(entity_key, $rust_path) \
                    OR string::starts_with(entity_key, $typescript_path) \
                    OR string::starts_with(entity_key, $tsx_path) \
                    OR string::starts_with(entity_key, $javascript_path)) \
             ORDER BY entity_key ASC LIMIT $limit;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("name_enabled", !name.is_empty()),
                b("name", name.to_owned()),
                b("prefix_enabled", !prefix.is_empty()),
                b("prefix", prefix),
                b("path_enabled", !path.is_empty()),
                b("rust_path", format!("rust:{path}#")),
                b("typescript_path", format!("typescript:{path}#")),
                b("tsx_path", format!("tsx:{path}#")),
                b("javascript_path", format!("javascript:{path}#")),
                b("limit", limit),
            ],
        )
        .await?;
        rows.into_iter().map(entity_to_domain).collect()
    }

    /// Public code-symbol lookup retained for fixture and integration consumers.
    /// The argument order matches the former storage API while execution stays
    /// entirely inside the embedded SurrealDB query above.
    pub async fn lookup_code_symbols(
        &self,
        workspace_id: &str,
        name: Option<&str>,
        path: Option<&str>,
        prefix: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<KnowledgeEntity>> {
        self.lookup_knowledge_code_symbols(workspace_id, name, prefix, path, limit)
            .await
    }

    /// Tombstone one live RichDocument and remove every addressable projection
    /// in the same embedded-Surreal transaction as its EventLedger receipt.
    ///
    /// `expected` is the API's preflight read. The mutation lock and the
    /// transaction guards revalidate every receipt-bearing identity dimension,
    /// so a concurrent save can never make the delete receipt describe stale
    /// document state. Exact EventLedger idempotency replays reuse the stored
    /// receipt; divergent reuse aborts before any tombstone or cleanup commits.
    pub(crate) async fn delete_knowledge_rich_document_atomic(
        &self,
        expected: &KnowledgeRichDocument,
        event: NewKernelEvent,
    ) -> StorageResult<KnowledgeRichDocumentDeleteOutcome> {
        if event.event_type != KernelEventType::KnowledgeRichDocumentDeleted
            || event.aggregate_type != "knowledge_rich_document"
            || event.aggregate_id != expected.rich_document_id
        {
            return Err(StorageError::Validation(
                "rich document delete receipt identity is invalid",
            ));
        }

        let (_, event) = super::event_ledger::prepare_event(event)?;
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        if let Some(outcome) =
            read_rich_document_delete_replay(self.storage(), &expected.rich_document_id, &event)
                .await?
        {
            return Ok(outcome);
        }
        let current = read_live_rich_document(self.storage(), &expected.rich_document_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        if current.workspace_id != expected.workspace_id
            || current.block_id != expected.block_id
            || current.title != expected.title
            || current.doc_version != expected.doc_version
            || current.content_sha256 != expected.content_sha256
        {
            return Err(StorageError::Conflict(
                "knowledge rich document changed before delete",
            ));
        }

        // The transaction stores its externally returned counts on the
        // tombstone before deleting the contributing rows. That durable
        // outcome, linked to the EventLedger receipt, makes an exact replay
        // return byte-for-byte-equivalent semantics after the first commit.
        let statement = "BEGIN TRANSACTION; \
            IF (SELECT VALUE id FROM kernel_event_ledger \
                WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0] != NONE { \
                LET $stored = (SELECT event_version, kernel_task_run_id, session_run_id, \
                    aggregate_type, aggregate_id, event_type, actor_kind, actor_id, causation_id, \
                    correlation_id, payload_hash, source_component FROM kernel_event_ledger \
                    WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0]; \
                IF $stored.event_version != $event.event_version \
                    OR $stored.kernel_task_run_id != $event.kernel_task_run_id \
                    OR $stored.session_run_id != $event.session_run_id \
                    OR $stored.aggregate_type != $event.aggregate_type \
                    OR $stored.aggregate_id != $event.aggregate_id \
                    OR $stored.event_type != $event.event_type \
                    OR $stored.actor_kind != $event.actor_kind \
                    OR $stored.actor_id != $event.actor_id \
                    OR $stored.causation_id != $event.causation_id \
                    OR $stored.correlation_id != $event.correlation_id \
                    OR $stored.payload_hash != $event.payload_hash \
                    OR $stored.source_component != $event.source_component { \
                    THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; \
                }; \
                THROW 'HSK-KRD-DELETE-REPLAY'; \
            }; \
            IF array::len((SELECT id FROM $document WHERE deleted_at = NONE)) != 1 { \
                THROW 'HSK-KRD-DELETE-NOT-FOUND'; \
            }; \
            IF array::len((SELECT id FROM $document WHERE deleted_at = NONE \
                AND workspace_id = $workspace AND rich_document_id = $doc_id \
                AND title = $expected_title AND doc_version = $expected_doc_version \
                AND content_sha256 = $expected_content_sha256)) != 1 { \
                THROW 'HSK-KRD-DELETE-STALE'; \
            }; \
            IF array::len((SELECT id FROM $block WHERE workspace_id = $workspace \
                AND block_id = $doc_id AND content_type = 'note')) != 1 { \
                THROW 'HSK-KRD-LOOM-IDENTITY'; \
            }; \
            IF array::len((SELECT id FROM $search)) > 0 \
                AND array::len((SELECT id FROM $search WHERE workspace_id = $workspace \
                    AND block_id = $block AND content_type = 'note')) != 1 { \
                THROW 'HSK-KRD-SEARCH-IDENTITY'; \
            }; \
            LET $affected_blocks = array::distinct(array::union( \
                (SELECT VALUE target_block_id FROM loom_edges WHERE workspace_id = $workspace \
                    AND source_block_id = $block AND target_block_id != $block), \
                (SELECT VALUE source_block_id FROM loom_edges WHERE workspace_id = $workspace \
                    AND target_block_id = $block AND source_block_id != $block) \
            )); \
            LET $unique_title = array::len((SELECT id FROM knowledge_rich_documents \
                WHERE workspace_id = $workspace AND title = $expected_title \
                    AND deleted_at = NONE)) = 1; \
            LET $source_count = array::len((SELECT id FROM knowledge_sources \
                WHERE workspace_id = $workspace AND source_kind = 'rich_document' \
                    AND provenance.rich_document_id = $doc_id)); \
            LET $backlink_count = array::len((SELECT id FROM knowledge_document_backlinks \
                WHERE workspace_id = $workspace AND (source_document_id = $document \
                    OR target = $doc_id OR ($unique_title AND target = $expected_title)))); \
            CREATE $event.record CONTENT { event_id: $event.event_id, \
                event_version: $event.event_version, \
                kernel_task_run_id: $event.kernel_task_run_id, \
                session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                event_type: $event.event_type, actor_kind: $event.actor_kind, \
                actor_id: $event.actor_id, causation_id: $event.causation_id, \
                correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                source_component: $event.source_component, payload: $event.payload, \
                created_at: $event.created_at } RETURN NONE; \
            IF array::len((UPDATE $document SET deleted_at = time::now(), \
                deleted_receipt_event_id = (SELECT VALUE id FROM kernel_event_ledger \
                    WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0], \
                projection_refs = array::append(projection_refs, { \
                    kind: 'rich_document_delete_outcome_v1', \
                    receipt_event_id: $event.event_id, \
                    source_marked_stale: $source_count > 0, \
                    backlinks_deleted: $backlink_count, \
                    loom_block_deleted: true \
                }), \
                updated_at = time::now() WHERE deleted_at = NONE RETURN AFTER)) != 1 { \
                THROW 'HSK-KRD-DELETE-NOT-FOUND'; \
            }; \
            UPDATE knowledge_sources SET stale = true, updated_at = time::now() \
                WHERE workspace_id = $workspace AND source_kind = 'rich_document' \
                    AND provenance.rich_document_id = $doc_id RETURN AFTER; \
            DELETE knowledge_document_backlinks WHERE workspace_id = $workspace \
                AND (source_document_id = $document OR target = $doc_id \
                    OR ($unique_title AND target = $expected_title)) RETURN BEFORE; \
            DELETE knowledge_rich_document_drafts WHERE rich_document_id = $document; \
            DELETE loom_canvas_placements WHERE workspace_id = $workspace \
                AND placed_block_id = $block; \
            IF array::len((DELETE $block WHERE workspace_id = $workspace \
                AND block_id = $doc_id AND content_type = 'note' RETURN BEFORE)) != 1 { \
                THROW 'HSK-KRD-LOOM-IDENTITY'; \
            }; \
            FOR $affected IN $affected_blocks { \
                UPDATE loom_blocks SET \
                    mention_count = array::len((SELECT VALUE id FROM loom_edges \
                        WHERE workspace_id = $workspace AND source_block_id = $affected \
                            AND edge_type = 'mention')), \
                    tag_count = array::len((SELECT VALUE id FROM loom_edges \
                        WHERE workspace_id = $workspace AND source_block_id = $affected \
                            AND edge_type = 'tag')), \
                    backlink_count = array::len((SELECT VALUE id FROM loom_edges \
                        WHERE workspace_id = $workspace AND target_block_id = $affected \
                            AND edge_type IN ['mention', 'tag'])) \
                    WHERE workspace_id = $workspace AND id = $affected RETURN NONE; \
            }; \
            COMMIT TRANSACTION;";
        let binds = vec![
            b(
                "document",
                thing(KNOWLEDGE_RICH_DOCUMENTS_TABLE, &current.rich_document_id),
            ),
            b("workspace", thing(WORKSPACES_TABLE, &current.workspace_id)),
            b("block", thing(LOOM_BLOCKS_TABLE, &current.block_id)),
            b(
                "search",
                thing("loom_block_search_index", &current.block_id),
            ),
            b("doc_id", current.rich_document_id),
            b("expected_title", current.title),
            b("expected_doc_version", current.doc_version),
            b("expected_content_sha256", current.content_sha256),
            b("event", event.clone()),
        ];

        let replay_event = event;
        let result: Result<(), SurrealStorageError> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    let mut query = database.client.query(statement);
                    for (name, value) in binds {
                        query = query.bind((name, value));
                    }
                    query.await?.check()?;
                    Ok(())
                })
            })
            .await;
        if let Err(error) = result {
            // A second process can commit the exact operation between this
            // process's replay preflight and BEGIN. Reconcile every failure
            // against the canonical receipt+tombstone pair before classifying
            // the transaction error.
            if let Some(outcome) = read_rich_document_delete_replay(
                self.storage(),
                &expected.rich_document_id,
                &replay_event,
            )
            .await?
            {
                return Ok(outcome);
            }
            return Err(map_guarded_err(
                error,
                &[
                    ("HSK-KRD-DELETE-NOT-FOUND", || {
                        StorageError::NotFound("knowledge rich document")
                    }),
                    ("HSK-KRD-DELETE-STALE", || {
                        StorageError::Conflict("knowledge rich document changed before delete")
                    }),
                    LOOM_IDENTITY_GUARDS[0],
                    LOOM_IDENTITY_GUARDS[1],
                    ("HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT", || {
                        StorageError::Conflict(
                            "kernel event idempotency key was reused with different event content",
                        )
                    }),
                ],
            ));
        }
        read_rich_document_delete_replay(self.storage(), &expected.rich_document_id, &replay_event)
            .await?
            .ok_or_else(|| {
                StorageError::Database(
                    "rich document delete committed without a replayable receipt".to_owned(),
                )
            })
    }
}

async fn read_prior_backlink_state(
    storage: &SurrealStorage,
    workspace_key: &str,
    source_document_id: &str,
) -> StorageResult<(HashMap<String, String>, Vec<String>)> {
    let prior_rows: Vec<PriorBacklinkRecord> = query_rows(
        storage,
        "SELECT relationship_id, target FROM knowledge_document_backlinks \
         WHERE source_document_id = type::record('knowledge_rich_documents', $source_key) \
         ORDER BY relationship_id ASC;",
        vec![b("source_key", source_document_id.to_owned())],
    )
    .await?;
    let prior_by_relationship: HashMap<String, String> = prior_rows
        .into_iter()
        .map(|row| (row.relationship_id, row.target))
        .collect();
    let prior_loom_targets: Vec<RecordId> = query_rows(
        storage,
        "SELECT VALUE target_block_id FROM loom_edges \
         WHERE workspace_id = $workspace \
           AND source_block_id = type::record('loom_blocks', $source_key) \
           AND string::starts_with(edge_id, 'KDLNK-') \
           AND last_actor_kind = 'SYSTEM' \
           AND last_actor_id = 'knowledge_rich_document_backlink_projection' \
           AND edit_event_id = '00000000-0000-0000-0000-000000000000' \
           AND source_document_id = $source_key;",
        vec![
            b("workspace", thing(WORKSPACES_TABLE, workspace_key)),
            b("source_key", source_document_id.to_owned()),
        ],
    )
    .await?;
    let prior_loom_targets = prior_loom_targets
        .into_iter()
        .map(record_key)
        .collect::<StorageResult<Vec<_>>>()?;
    Ok((prior_by_relationship, prior_loom_targets))
}

/// Extracts the initial backlink upserts from a document's content, exactly
/// as the removed create-transaction did.
fn extract_backlink_upserts(
    document: &KnowledgeRichDocument,
) -> StorageResult<Vec<UpsertKnowledgeDocumentBacklink>> {
    let tree = crate::knowledge_document::block_tree::BlockTree::from_document_json(
        &document.rich_document_id,
        &document.schema_version,
        &document.content_json,
    )
    .map_err(|_| StorageError::Validation("knowledge rich document block tree is malformed"))?;
    let references = crate::knowledge_document::backlink::DocumentLinkReferences::extract(&tree);
    Ok(references
        .references
        .into_iter()
        .map(|reference| UpsertKnowledgeDocumentBacklink {
            workspace_id: document.workspace_id.clone(),
            relationship_id: reference.relationship_id,
            source_document_id: document.rich_document_id.clone(),
            link_kind: reference.kind.as_str().to_string(),
            target: reference.target,
            block_id: reference.block_id,
        })
        .collect())
}

/// MT-032 create closure: authority row, version 1, same-id Loom projection,
/// search projection, and initial backlinks become durable in one
/// transaction. Callers must hold [`RICH_DOCUMENT_MUTATION_LOCK`].
async fn create_rich_document_locked(
    storage: &SurrealStorage,
    new_document: &NewKnowledgeRichDocument,
) -> StorageResult<KnowledgeRichDocument> {
    if new_document.title.trim() != new_document.title || new_document.title.is_empty() {
        return Err(StorageError::Validation(
            "knowledge rich document title must be non-empty and trimmed",
        ));
    }
    if new_document.schema_version.trim() != new_document.schema_version
        || new_document.schema_version.is_empty()
    {
        return Err(StorageError::Validation(
            "knowledge rich document schema_version must be non-empty and trimmed",
        ));
    }
    let authority_label = new_document
        .authority_label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("promoted");
    if !matches!(authority_label, "draft" | "promoted" | "archived") {
        return Err(StorageError::Validation(
            "knowledge rich document authority_label must be draft|promoted|archived",
        ));
    }
    if new_document.owner_actor_kind.is_some() != new_document.owner_actor_id.is_some() {
        return Err(StorageError::Validation(
            "knowledge rich document owner_actor_kind and owner_actor_id must be set together",
        ));
    }
    let rich_document_id = new_knowledge_id("KRD");
    let content_sha256 = knowledge_canonical_json_sha256(&new_document.content_json);
    let (derived_json, search_text) =
        loom_projection_inputs(&new_document.title, &new_document.content_json)?;
    let loom_actor_kind = match new_document.owner_actor_kind.as_deref() {
        Some("system" | "local_model" | "cloud_model") => "SYSTEM",
        _ => "HUMAN",
    };

    // The document does not exist yet, so the prior backlink state is empty;
    // the initial reference set comes from the new content.
    let pending = KnowledgeRichDocument {
        rich_document_id: rich_document_id.clone(),
        block_id: rich_document_id.clone(),
        workspace_id: new_document.workspace_id.clone(),
        document_id: new_document.document_id.clone(),
        title: new_document.title.clone(),
        schema_version: new_document.schema_version.clone(),
        doc_version: 1,
        content_json: new_document.content_json.clone(),
        content_sha256: content_sha256.clone(),
        crdt_document_id: new_document.crdt_document_id.clone(),
        crdt_snapshot_id: new_document.crdt_snapshot_id.clone(),
        promotion_receipt_event_id: new_document.promotion_receipt_event_id.clone(),
        projection_refs: JsonValue::Array(Vec::new()),
        project_ref: new_document.project_ref.clone(),
        folder_ref: new_document.folder_ref.clone(),
        authority_label: authority_label.to_owned(),
        owner_actor_kind: new_document.owner_actor_kind.clone(),
        owner_actor_id: new_document.owner_actor_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let upserts = extract_backlink_upserts(&pending)?;
    let resolved = resolve_backlink_rows(
        storage,
        &new_document.workspace_id,
        &rich_document_id,
        upserts,
        &HashMap::new(),
        &[],
    )
    .await?;
    let affected_blocks: BTreeSet<String> = resolved
        .iter()
        .filter(|row| row.project_to_loom)
        .map(|row| row.target.clone())
        .chain(std::iter::once(rich_document_id.clone()))
        .collect();

    // Statements: BEGIN(0) CREATE doc(1) loom(2) search(3) version(4)
    // backlink writes(5..9) COMMIT.
    let statement = format!(
        "BEGIN TRANSACTION;\n\
         CREATE type::record('knowledge_rich_documents', $doc_id) CONTENT {{ rich_document_id: $doc_id, workspace_id: $workspace, document_id: $legacy_document_id, title: $doc_title, schema_version: $schema_version, content_json: $content_json, content_sha256: $doc_content_sha256, crdt_document_id: $crdt_document_id, crdt_snapshot_id: $crdt_snapshot_id, promotion_receipt_event_id: $promotion_receipt, project_ref: $project_ref, folder_ref: $folder_ref, authority_label: $authority_label, owner_actor_kind: $owner_actor_kind, owner_actor_id: $owner_actor_id }} RETURN AFTER;\n\
         {LOOM_PROJECTION_STATEMENT}\n\
         {SEARCH_PROJECTION_STATEMENT}\n\
         CREATE knowledge_rich_document_versions CONTENT {{ rich_document_id: type::record('knowledge_rich_documents', $doc_id), doc_version: 1, schema_version: $schema_version, content_json: $content_json, content_sha256: $doc_content_sha256, crdt_snapshot_id: $crdt_snapshot_id, promotion_receipt_event_id: $promotion_receipt }} RETURN NONE;\n\
         {BACKLINK_WRITE_STATEMENTS}\n\
         COMMIT TRANSACTION;"
    );
    let mut binds = vec![
        b("doc_id", rich_document_id.clone()),
        b(
            "legacy_document_id",
            opt_thing(DOCUMENTS_TABLE, new_document.document_id.as_deref()),
        ),
        b("schema_version", new_document.schema_version.clone()),
        b("content_json", new_document.content_json.clone()),
        b("crdt_document_id", new_document.crdt_document_id.clone()),
        b("crdt_snapshot_id", new_document.crdt_snapshot_id.clone()),
        b(
            "promotion_receipt",
            opt_thing(
                KERNEL_EVENT_LEDGER_TABLE,
                new_document.promotion_receipt_event_id.as_deref(),
            ),
        ),
        b("project_ref", new_document.project_ref.clone()),
        b("folder_ref", new_document.folder_ref.clone()),
        b("authority_label", authority_label.to_owned()),
        b("owner_actor_kind", new_document.owner_actor_kind.clone()),
        b("owner_actor_id", new_document.owner_actor_id.clone()),
    ];
    binds.extend(projection_binds(
        &rich_document_id,
        thing(WORKSPACES_TABLE, &new_document.workspace_id),
        &new_document.title,
        &content_sha256,
        derived_json,
        &search_text,
        loom_actor_kind,
        new_document.owner_actor_id.clone(),
    ));
    binds.extend(backlink_write_binds(
        thing(WORKSPACES_TABLE, &new_document.workspace_id),
        &rich_document_id,
        &resolved,
        &affected_blocks,
    ));
    // `projection_binds` and `backlink_write_binds` both carry `workspace`;
    // duplicate bind names would overwrite with the identical value, which is
    // harmless, but deduplicate for determinism.
    binds = dedup_binds(binds);

    let rows: Vec<RichDocRecord> =
        raw_rows_at(storage, statement, binds, 1)
            .await
            .map_err(|error| {
                map_guarded_err(
                    error,
                    &[
                        LOOM_IDENTITY_GUARDS[0],
                        LOOM_IDENTITY_GUARDS[1],
                        BACKLINK_GUARDS[0],
                    ],
                )
            })?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "knowledge rich document CREATE returned no record".to_owned(),
        ))
        .and_then(rich_document_to_domain)
}

fn dedup_binds(binds: Binds) -> Binds {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(binds.len());
    // Keep the LAST occurrence of every name so later bind groups win, then
    // restore the original relative order.
    for (name, value) in binds.into_iter().rev() {
        if seen.insert(name.clone()) {
            out.push((name, value));
        }
    }
    out.reverse();
    out
}

/// Optional idempotency-key claim appended inside the save transaction
/// (MT-062): the write and the key row commit or roll back together.
struct IdempotentSaveClaim {
    idempotency_key: String,
    workspace_key: String,
    request_hash: String,
    result_ref_id: String,
}

const IDEMPOTENCY_CLAIM_STATEMENT: &str = "IF (SELECT VALUE id FROM $idem_key_record)[0] = NONE { CREATE $idem_key_record CONTENT { idempotency_key: $idempotency_key, workspace_id: $idem_workspace, operation_kind: $idem_operation_kind, request_hash: $idem_request_hash, result_ref_kind: $idem_result_ref_kind, result_ref_id: $idem_result_ref_id } RETURN NONE; } ELSE { THROW 'HSK-KIDEM-RACE'; };";

fn checked_next_rich_document_version(expected_version: i64) -> StorageResult<i64> {
    expected_version
        .checked_add(1)
        .ok_or(StorageError::Validation(
            "knowledge rich document version cannot exceed i64::MAX",
        ))
}

fn idempotency_claim_binds(
    idempotency_key: &str,
    workspace_key: &str,
    operation_kind: &str,
    request_hash: &str,
    result_ref_kind: &str,
    result_ref_id: &str,
) -> Binds {
    vec![
        b(
            "idem_key_record",
            thing("knowledge_idempotency_keys", idempotency_key),
        ),
        b("idempotency_key", idempotency_key.to_owned()),
        b("idem_workspace", thing(WORKSPACES_TABLE, workspace_key)),
        b("idem_operation_kind", operation_kind.to_owned()),
        b("idem_request_hash", request_hash.to_owned()),
        b("idem_result_ref_kind", result_ref_kind.to_owned()),
        b("idem_result_ref_id", result_ref_id.to_owned()),
    ]
}

/// Optimistic-concurrency save shared by the plain and idempotent paths.
/// Returns `Ok(None)` when the appended idempotency-key claim lost its race:
/// the transaction aborted, so nothing was written (the caller re-reads the
/// winner's committed result). Callers hold [`RICH_DOCUMENT_MUTATION_LOCK`].
#[allow(clippy::too_many_arguments)]
async fn save_rich_document_version_locked(
    storage: &SurrealStorage,
    rich_document_id: &str,
    expected_version: i64,
    next_version: i64,
    content_json: &JsonValue,
    crdt_document_id: Option<&str>,
    crdt_snapshot_id: Option<&str>,
    promotion_receipt_event_id: Option<&str>,
    claim: Option<IdempotentSaveClaim>,
) -> StorageResult<Option<KnowledgeRichDocument>> {
    let current = read_live_rich_document(storage, rich_document_id)
        .await?
        .ok_or(StorageError::NotFound("knowledge rich document"))?;
    if current.doc_version == expected_version
        && rich_document_crdt_id_change_requested(
            current.crdt_document_id.as_deref(),
            crdt_document_id,
        )
    {
        return Err(StorageError::Validation(
            "knowledge rich document crdt_document_id cannot change once set",
        ));
    }
    if current.doc_version != expected_version {
        return Err(StorageError::Conflict(
            "knowledge rich document version conflict: expected_version is stale",
        ));
    }

    let content_sha256 = knowledge_canonical_json_sha256(content_json);
    let (derived_json, search_text) = loom_projection_inputs(&current.title, content_json)?;

    // Statements: BEGIN(0) guarded-update(1) loom(2) search(3) version(4)
    // draft-delete(5) [claim(6)] final-select(6|7) COMMIT.
    let claim_statement = if claim.is_some() {
        IDEMPOTENCY_CLAIM_STATEMENT
    } else {
        ""
    };
    let final_select_index = if claim.is_some() { 7 } else { 6 };
    let statement = format!(
        "BEGIN TRANSACTION;\n\
         IF array::len((UPDATE knowledge_rich_documents SET doc_version = $next_version, content_json = $content_json, content_sha256 = $doc_content_sha256, crdt_document_id = $crdt_document_id ?? crdt_document_id, crdt_snapshot_id = $crdt_snapshot_id, promotion_receipt_event_id = $promotion_receipt, updated_at = time::now() WHERE rich_document_id = $doc_id AND doc_version = $expected_version AND deleted_at = NONE AND ($crdt_document_id = NONE OR crdt_document_id = NONE OR crdt_document_id = $crdt_document_id) RETURN AFTER)) != 1 {{ THROW 'HSK-KRD-SAVE-STALE'; }};\n\
         {LOOM_PROJECTION_STATEMENT}\n\
         {SEARCH_PROJECTION_STATEMENT}\n\
         CREATE knowledge_rich_document_versions CONTENT {{ rich_document_id: type::record('knowledge_rich_documents', $doc_id), doc_version: $next_version, schema_version: $schema_version, content_json: $content_json, content_sha256: $doc_content_sha256, crdt_snapshot_id: $crdt_snapshot_id, promotion_receipt_event_id: $promotion_receipt }} RETURN NONE;\n\
         DELETE knowledge_rich_document_drafts WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id);\n\
         {claim_statement}\n\
         SELECT * FROM knowledge_rich_documents WHERE rich_document_id = $doc_id;\n\
         COMMIT TRANSACTION;"
    );
    let mut binds = vec![
        b("doc_id", rich_document_id.to_owned()),
        b("expected_version", expected_version),
        b("next_version", next_version),
        b("content_json", content_json.clone()),
        b("schema_version", current.schema_version.clone()),
        b("crdt_document_id", crdt_document_id.map(str::to_owned)),
        b("crdt_snapshot_id", crdt_snapshot_id.map(str::to_owned)),
        b(
            "promotion_receipt",
            opt_thing(KERNEL_EVENT_LEDGER_TABLE, promotion_receipt_event_id),
        ),
    ];
    binds.extend(projection_binds(
        rich_document_id,
        thing(WORKSPACES_TABLE, &current.workspace_id),
        &current.title,
        &content_sha256,
        derived_json,
        &search_text,
        "SYSTEM",
        Some(
            if claim.is_some() {
                "knowledge_rich_document_idempotent_save"
            } else {
                "knowledge_rich_document_save"
            }
            .to_owned(),
        ),
    ));
    if let Some(claim) = &claim {
        binds.extend(idempotency_claim_binds(
            &claim.idempotency_key,
            &claim.workspace_key,
            "rich_document_save",
            &claim.request_hash,
            RICH_DOCUMENT_VERSION_RESULT_REF_KIND,
            &claim.result_ref_id,
        ));
    }
    binds = dedup_binds(binds);

    let result: Result<Vec<RichDocRecord>, SurrealStorageError> =
        raw_rows_at(storage, statement, binds, final_select_index).await;
    match result {
        Ok(rows) => rows
            .into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge rich document"))
            .and_then(rich_document_to_domain)
            .map(Some),
        Err(error) => {
            let rendered = error.to_string();
            if rendered.contains("HSK-KIDEM-RACE") {
                return Ok(None);
            }
            if rendered.contains("HSK-KRD-SAVE-STALE") {
                // Serialized by the mutation lock, so a surprise here means the
                // row changed through a non-knowledge path; classify exactly
                // like the removed backend did after its failed UPDATE.
                return Err(
                    match read_live_rich_document(storage, rich_document_id).await? {
                        Some(current)
                            if current.doc_version == expected_version
                                && rich_document_crdt_id_change_requested(
                                    current.crdt_document_id.as_deref(),
                                    crdt_document_id,
                                ) =>
                        {
                            StorageError::Validation(
                                "knowledge rich document crdt_document_id cannot change once set",
                            )
                        }
                        Some(_) => StorageError::Conflict(
                            "knowledge rich document version conflict: expected_version is stale",
                        ),
                        None => StorageError::NotFound("knowledge rich document"),
                    },
                );
            }
            Err(map_guarded_err(
                error,
                &[LOOM_IDENTITY_GUARDS[0], LOOM_IDENTITY_GUARDS[1]],
            ))
        }
    }
}

/// Reads a committed idempotency key. Same hash: prior result ref. Different
/// hash: typed Conflict (divergent duplicate).
async fn find_idempotency_result(
    storage: &SurrealStorage,
    idempotency_key: &str,
    request_hash: &str,
) -> StorageResult<Option<(String, String)>> {
    let record: Option<IdemKeyRecord> = query_first_row(
        storage,
        "SELECT request_hash, result_ref_kind, result_ref_id FROM $idem_key_record;",
        vec![b(
            "idem_key_record",
            thing("knowledge_idempotency_keys", idempotency_key),
        )],
    )
    .await?;
    let Some(record) = record else {
        return Ok(None);
    };
    if record.request_hash != request_hash {
        return Err(StorageError::Conflict(
            "knowledge idempotency key replayed with a different request payload",
        ));
    }
    Ok(Some((record.result_ref_kind, record.result_ref_id)))
}

/// The retired backend derived this key inside its database layer so Rust and the
/// database could never disagree; with the embedded store both the key and
/// the candidate comparison happen in this same function, so consistency
/// holds by construction.
fn normalize_rich_document_title(title: &str) -> String {
    title
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(SurrealValue)]
struct DocTitleRecord {
    rich_document_id: String,
    title: String,
    updated_at: Datetime,
}

fn validate_new_passage(new_passage: &NewKnowledgeMemoryPassage) -> StorageResult<()> {
    if new_passage.evidence.is_empty() {
        return Err(StorageError::Validation(
            "knowledge memory passages are derived from sources and claims; evidence is required (spec 2.3.13.11)",
        ));
    }
    if new_passage.passage_text.is_empty() {
        return Err(StorageError::Validation(
            "knowledge passage_text is required",
        ));
    }
    if !(0.0..=1.0).contains(&new_passage.extraction_confidence) {
        return Err(StorageError::Validation(
            "knowledge passage extraction_confidence must be within [0.0, 1.0]",
        ));
    }
    Ok(())
}

/// Two statements: the passage CREATE (result-bearing) and the lineage loop.
const PASSAGE_INSERT_STATEMENTS: &str = "CREATE type::record('knowledge_memory_passages', $passage_id) CONTENT { passage_id: $passage_id, workspace_id: $workspace, passage_text: $passage_text, token_count: $token_count, ocr_transcript_metadata: $ocr_transcript_metadata, extraction_confidence: $extraction_confidence, ranking_features: $ranking_features, retrieval_mode: $retrieval_mode, compaction_policy: $compaction_policy, failure_receipt_event_id: $failure_receipt, derived_in_run: $derived_in_run } RETURN AFTER;\nFOR $item IN $evidence_rows { CREATE knowledge_passage_evidence CONTENT { passage_id: type::record('knowledge_memory_passages', $passage_id), ref_kind: $item.ref_kind, source_id: IF $item.source_id != NONE { type::record('knowledge_sources', $item.source_id) } ELSE { NONE }, claim_id: IF $item.claim_id != NONE { type::record('knowledge_claims', $item.claim_id) } ELSE { NONE }, span_id: IF $item.span_id != NONE { type::record('knowledge_spans', $item.span_id) } ELSE { NONE }, ordinal: $item.ordinal } RETURN NONE; };";

fn passage_insert_binds(passage_id: &str, new_passage: &NewKnowledgeMemoryPassage) -> Binds {
    let evidence_rows: Vec<JsonValue> = new_passage
        .evidence
        .iter()
        .enumerate()
        .map(|(index, evidence)| {
            let mut row = serde_json::Map::new();
            row.insert("ordinal".into(), JsonValue::from(index as i64));
            match evidence {
                KnowledgePassageEvidenceRef::Source { source_id } => {
                    row.insert("ref_kind".into(), JsonValue::from("source"));
                    row.insert("source_id".into(), JsonValue::from(source_id.clone()));
                }
                KnowledgePassageEvidenceRef::Claim { claim_id } => {
                    row.insert("ref_kind".into(), JsonValue::from("claim"));
                    row.insert("claim_id".into(), JsonValue::from(claim_id.clone()));
                }
                KnowledgePassageEvidenceRef::Span { span_id } => {
                    row.insert("ref_kind".into(), JsonValue::from("span"));
                    row.insert("span_id".into(), JsonValue::from(span_id.clone()));
                }
            }
            JsonValue::Object(row)
        })
        .collect();
    vec![
        b("passage_id", passage_id.to_owned()),
        b(
            "workspace",
            thing(WORKSPACES_TABLE, &new_passage.workspace_id),
        ),
        b("passage_text", new_passage.passage_text.clone()),
        b("token_count", new_passage.token_count.map(i64::from)),
        b(
            "ocr_transcript_metadata",
            new_passage.ocr_transcript_metadata.clone(),
        ),
        b("extraction_confidence", new_passage.extraction_confidence),
        b("ranking_features", new_passage.ranking_features.clone()),
        b(
            "retrieval_mode",
            new_passage.retrieval_mode.as_str().to_owned(),
        ),
        b(
            "compaction_policy",
            new_passage.compaction_policy.as_str().to_owned(),
        ),
        b(
            "failure_receipt",
            opt_thing(
                KERNEL_EVENT_LEDGER_TABLE,
                new_passage.failure_receipt_event_id.as_deref(),
            ),
        ),
        b(
            "derived_in_run",
            opt_thing(
                KNOWLEDGE_INDEX_RUNS_TABLE,
                new_passage.derived_in_run.as_deref(),
            ),
        ),
        b("evidence_rows", JsonValue::Array(evidence_rows)),
    ]
}

#[async_trait]
impl KnowledgeStore for SurrealDatabase {
    async fn list_knowledge_schema_registry(
        &self,
    ) -> StorageResult<Vec<KnowledgeSchemaRegistryRow>> {
        let rows: Vec<RegistryRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_schema_registry ORDER BY family_key ASC;",
            Vec::new(),
        )
        .await?;
        rows.into_iter().map(registry_to_domain).collect()
    }

    async fn audit_knowledge_namespace(&self) -> StorageResult<KnowledgeNamespaceAudit> {
        let registered = self.list_knowledge_schema_registry().await?;
        let info: SurrealValueData = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    let mut response = database
                        .client
                        .query("INFO FOR DB STRUCTURE;")
                        .await?
                        .check()?;
                    Ok(response.take(0)?)
                })
            })
            .await
            .map_err(map_err)?;
        let mut present =
            parse_named_array(&info, "tables").map_err(StorageError::Serialization)?;
        present.retain(|table| table.starts_with("knowledge_"));
        present.sort();

        let missing_tables = registered
            .iter()
            .filter(|row| !present.contains(&row.table_name))
            .map(|row| row.table_name.clone())
            .collect();
        let unregistered_tables = present
            .iter()
            .filter(|table| !registered.iter().any(|row| &row.table_name == *table))
            .cloned()
            .collect();
        Ok(KnowledgeNamespaceAudit {
            registered,
            missing_tables,
            unregistered_tables,
        })
    }

    async fn create_knowledge_source_root(
        &self,
        new_root: NewKnowledgeSourceRoot,
    ) -> StorageResult<KnowledgeSourceRoot> {
        if new_root.display_name.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge source root display_name is required",
            ));
        }
        let repo_relative_path = normalize_repo_relative_path(&new_root.repo_relative_path)?;
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let root_id = new_knowledge_id("KSR");
        let rows: Vec<RootRecord> = query_rows(
            self.storage(),
            "IF (SELECT VALUE id FROM knowledge_source_roots WHERE workspace_id = $workspace AND repo_relative_path = $path LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_source_roots SET display_name = $display_name, root_kind = $root_kind, allowlist_policy = $allowlist_policy, indexing_eligibility = $indexing_eligibility, updated_at = time::now() WHERE workspace_id = $workspace AND repo_relative_path = $path RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_source_roots', $root_id) CONTENT { root_id: $root_id, workspace_id: $workspace, display_name: $display_name, root_kind: $root_kind, repo_relative_path: $path, allowlist_policy: $allowlist_policy, indexing_eligibility: $indexing_eligibility } RETURN AFTER; };",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, &new_root.workspace_id)),
                b("path", repo_relative_path),
                b("root_id", root_id),
                b("display_name", new_root.display_name.clone()),
                b("root_kind", new_root.root_kind.as_str().to_owned()),
                b("allowlist_policy", new_root.allowlist_policy.clone()),
                b(
                    "indexing_eligibility",
                    new_root.indexing_eligibility.as_str().to_owned(),
                ),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge source root upsert returned no record".to_owned(),
            ))
            .and_then(root_to_domain)
    }

    async fn get_knowledge_source_root(
        &self,
        root_id: &str,
    ) -> StorageResult<Option<KnowledgeSourceRoot>> {
        query_first_row::<RootRecord>(
            self.storage(),
            "SELECT * FROM knowledge_source_roots WHERE root_id = $root_id;",
            vec![b("root_id", root_id.to_owned())],
        )
        .await?
        .map(root_to_domain)
        .transpose()
    }

    async fn list_knowledge_source_roots(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeSourceRoot>> {
        let rows: Vec<RootRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_source_roots WHERE workspace_id = $workspace ORDER BY repo_relative_path ASC;",
            vec![b("workspace", thing(WORKSPACES_TABLE, workspace_id))],
        )
        .await?;
        rows.into_iter().map(root_to_domain).collect()
    }

    async fn set_knowledge_root_eligibility(
        &self,
        root_id: &str,
        eligibility: KnowledgeIndexingEligibility,
    ) -> StorageResult<KnowledgeSourceRoot> {
        let rows: Vec<RootRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_source_roots SET indexing_eligibility = $indexing_eligibility, updated_at = time::now() WHERE root_id = $root_id RETURN AFTER;",
            vec![
                b("root_id", root_id.to_owned()),
                b("indexing_eligibility", eligibility.as_str().to_owned()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge source root"))
            .and_then(root_to_domain)
    }

    async fn upsert_knowledge_source(
        &self,
        new_source: NewKnowledgeSource,
    ) -> StorageResult<KnowledgeSource> {
        if !is_sha256_hex(&new_source.content_hash) {
            return Err(StorageError::Validation(
                "knowledge source content_hash must be a lowercase sha256 hex digest",
            ));
        }
        let relative_path = new_source
            .relative_path
            .as_deref()
            .map(normalize_repo_relative_path)
            .transpose()?;
        if matches!(new_source.source_kind, KnowledgeSourceKind::File)
            && (new_source.root_id.is_none() || relative_path.is_none())
        {
            return Err(StorageError::Validation(
                "file-kind knowledge sources require root_id and relative_path",
            ));
        }
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let source_id = new_knowledge_id("KSRC");
        let rows: Vec<SourceRecord> = query_rows(
            self.storage(),
            "IF $relative_path != NONE AND (SELECT VALUE id FROM knowledge_sources WHERE root_id = $root_id AND relative_path = $relative_path LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_sources SET content_hash = $content_hash, size_bytes = $size_bytes, provenance = $provenance, permission_scope = $permission_scope, redaction_state = $redaction_state, source_modified_at = $source_modified_at, parser_status = 'pending', extraction_status = 'pending', stale = false, updated_at = time::now() WHERE root_id = $root_id AND relative_path = $relative_path RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_sources', $source_id) CONTENT { source_id: $source_id, workspace_id: $workspace, root_id: $root_id, source_kind: $source_kind, relative_path: $relative_path, asset_id: $asset_id, loom_block_id: $loom_block_id, document_id: $document_id, content_hash: $content_hash, size_bytes: $size_bytes, provenance: $provenance, permission_scope: $permission_scope, redaction_state: $redaction_state, source_modified_at: $source_modified_at } RETURN AFTER; };",
            vec![
                b("source_id", source_id),
                b("workspace", thing(WORKSPACES_TABLE, &new_source.workspace_id)),
                b(
                    "root_id",
                    opt_thing(KNOWLEDGE_SOURCE_ROOTS_TABLE, new_source.root_id.as_deref()),
                ),
                b("source_kind", new_source.source_kind.as_str().to_owned()),
                b("relative_path", relative_path),
                b("asset_id", opt_thing(ASSETS_TABLE, new_source.asset_id.as_deref())),
                b(
                    "loom_block_id",
                    opt_thing(LOOM_BLOCKS_TABLE, new_source.loom_block_id.as_deref()),
                ),
                b(
                    "document_id",
                    opt_thing(DOCUMENTS_TABLE, new_source.document_id.as_deref()),
                ),
                b("content_hash", new_source.content_hash.clone()),
                b("size_bytes", new_source.size_bytes),
                b("provenance", new_source.provenance.clone()),
                b(
                    "permission_scope",
                    new_source.permission_scope.as_str().to_owned(),
                ),
                b(
                    "redaction_state",
                    new_source.redaction_state.as_str().to_owned(),
                ),
                b(
                    "source_modified_at",
                    new_source.source_modified_at.map(Datetime::from),
                ),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge source upsert returned no record".to_owned(),
            ))
            .and_then(source_to_domain)
    }

    async fn get_knowledge_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Option<KnowledgeSource>> {
        query_first_row::<SourceRecord>(
            self.storage(),
            "SELECT * FROM knowledge_sources WHERE source_id = $source_id;",
            vec![b("source_id", source_id.to_owned())],
        )
        .await?
        .map(source_to_domain)
        .transpose()
    }

    async fn get_knowledge_source_by_document_id(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> StorageResult<Option<KnowledgeSource>> {
        // Provenance-keyed rich-document linkage (MT-154): the `document_id`
        // column links the legacy `documents` table, so a RichDocument source
        // carries its id in `provenance.rich_document_id`.
        query_first_row::<SourceRecord>(
            self.storage(),
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

    async fn list_knowledge_sources_for_root(
        &self,
        root_id: &str,
    ) -> StorageResult<Vec<KnowledgeSource>> {
        let rows: Vec<SourceRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_sources WHERE root_id = type::record('knowledge_source_roots', $root_id) ORDER BY relative_path ASC;",
            vec![b("root_id", root_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(source_to_domain).collect()
    }

    async fn mark_knowledge_source_stale(&self, source_id: &str) -> StorageResult<KnowledgeSource> {
        let rows: Vec<SourceRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_sources SET stale = true, updated_at = time::now() WHERE source_id = $source_id RETURN AFTER;",
            vec![b("source_id", source_id.to_owned())],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge source"))
            .and_then(source_to_domain)
    }

    async fn record_knowledge_source_index_receipt(
        &self,
        source_id: &str,
        parser_status: KnowledgeParserStatus,
        extraction_status: KnowledgeExtractionStatus,
        receipt_event_id: &str,
    ) -> StorageResult<KnowledgeSource> {
        let rows: Vec<SourceRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_sources SET parser_status = $parser_status, extraction_status = $extraction_status, last_index_receipt_event_id = $receipt, stale = false, updated_at = time::now() WHERE source_id = $source_id RETURN AFTER;",
            vec![
                b("source_id", source_id.to_owned()),
                b("parser_status", parser_status.as_str().to_owned()),
                b("extraction_status", extraction_status.as_str().to_owned()),
                b("receipt", thing(KERNEL_EVENT_LEDGER_TABLE, receipt_event_id)),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge source"))
            .and_then(source_to_domain)
    }

    async fn start_knowledge_index_run(
        &self,
        new_run: NewKnowledgeIndexRun,
    ) -> StorageResult<KnowledgeIndexRun> {
        if new_run.actor_kind.trim().is_empty() || new_run.actor_id.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge index run requires actor_kind and actor_id",
            ));
        }
        let index_run_id = new_knowledge_id("KIR");
        let rows: Vec<RunRecord> = query_rows(
            self.storage(),
            "CREATE type::record('knowledge_index_runs', $index_run_id) CONTENT { index_run_id: $index_run_id, workspace_id: $workspace, root_id: $root_id, scope: $scope, actor_kind: $actor_kind, actor_id: $actor_id, worktree_id: $worktree_id, start_receipt_event_id: $start_receipt } RETURN AFTER;",
            vec![
                b("index_run_id", index_run_id),
                b("workspace", thing(WORKSPACES_TABLE, &new_run.workspace_id)),
                b(
                    "root_id",
                    opt_thing(KNOWLEDGE_SOURCE_ROOTS_TABLE, new_run.root_id.as_deref()),
                ),
                b("scope", new_run.scope.clone()),
                b("actor_kind", new_run.actor_kind.clone()),
                b("actor_id", new_run.actor_id.clone()),
                b("worktree_id", new_run.worktree_id.clone()),
                b(
                    "start_receipt",
                    opt_thing(
                        KERNEL_EVENT_LEDGER_TABLE,
                        new_run.start_receipt_event_id.as_deref(),
                    ),
                ),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge index run CREATE returned no record".to_owned(),
            ))
            .and_then(run_to_domain)
    }

    async fn get_knowledge_index_run(
        &self,
        index_run_id: &str,
    ) -> StorageResult<Option<KnowledgeIndexRun>> {
        query_first_row::<RunRecord>(
            self.storage(),
            "SELECT * FROM knowledge_index_runs WHERE index_run_id = $index_run_id;",
            vec![b("index_run_id", index_run_id.to_owned())],
        )
        .await?
        .map(run_to_domain)
        .transpose()
    }

    async fn checkpoint_knowledge_index_run(
        &self,
        index_run_id: &str,
        restart_checkpoint: JsonValue,
    ) -> StorageResult<KnowledgeIndexRun> {
        let rows: Vec<RunRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_index_runs SET restart_checkpoint = $restart_checkpoint WHERE index_run_id = $index_run_id AND run_state = 'started' RETURN AFTER;",
            vec![
                b("index_run_id", index_run_id.to_owned()),
                b("restart_checkpoint", restart_checkpoint),
            ],
        )
        .await?;
        match rows.into_iter().next() {
            Some(row) => run_to_domain(row),
            None => {
                if self.get_knowledge_index_run(index_run_id).await?.is_some() {
                    Err(StorageError::Conflict(
                        "knowledge index run is terminal; checkpoints only apply to started runs",
                    ))
                } else {
                    Err(StorageError::NotFound("knowledge index run"))
                }
            }
        }
    }

    async fn finish_knowledge_index_run(
        &self,
        index_run_id: &str,
        outcome: KnowledgeIndexRunOutcome,
        finish_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIndexRun> {
        let state = outcome.state();
        let counts = outcome.counts();
        let rows: Vec<RunRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_index_runs SET run_state = $run_state, sources_seen = $sources_seen, sources_indexed = $sources_indexed, spans_extracted = $spans_extracted, entities_detected = $entities_detected, edges_written = $edges_written, claims_written = $claims_written, error_capture = $error_capture, finish_receipt_event_id = $finish_receipt, restart_checkpoint = NONE, finished_at = time::now() WHERE index_run_id = $index_run_id AND run_state = 'started' RETURN AFTER;",
            vec![
                b("index_run_id", index_run_id.to_owned()),
                b("run_state", state.as_str().to_owned()),
                b("sources_seen", i64::from(counts.sources_seen)),
                b("sources_indexed", i64::from(counts.sources_indexed)),
                b("spans_extracted", i64::from(counts.spans_extracted)),
                b("entities_detected", i64::from(counts.entities_detected)),
                b("edges_written", i64::from(counts.edges_written)),
                b("claims_written", i64::from(counts.claims_written)),
                b("error_capture", outcome.error_capture().cloned()),
                b(
                    "finish_receipt",
                    opt_thing(KERNEL_EVENT_LEDGER_TABLE, finish_receipt_event_id),
                ),
            ],
        )
        .await?;
        match rows.into_iter().next() {
            Some(row) => run_to_domain(row),
            None => {
                if self.get_knowledge_index_run(index_run_id).await?.is_some() {
                    Err(StorageError::Conflict(
                        "knowledge index run lifecycle violation: run is already terminal",
                    ))
                } else {
                    Err(StorageError::NotFound("knowledge index run"))
                }
            }
        }
    }

    async fn create_knowledge_span(
        &self,
        new_span: NewKnowledgeSpan,
    ) -> StorageResult<KnowledgeSpan> {
        if !is_sha256_hex(&new_span.content_sha256) {
            return Err(StorageError::Validation(
                "knowledge span content_sha256 must be a lowercase sha256 hex digest",
            ));
        }
        if new_span.parser_version.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge span parser_version is required",
            ));
        }
        if new_span.range_end < new_span.range_start || new_span.range_start < 0 {
            return Err(StorageError::Validation(
                "knowledge span range must satisfy 0 <= range_start <= range_end",
            ));
        }
        let span_id = new_knowledge_id("KSP");
        let rows: Vec<SpanRecord> = query_rows(
            self.storage(),
            "CREATE type::record('knowledge_spans', $span_id) CONTENT { span_id: $span_id, source_id: type::record('knowledge_sources', $source_id), span_kind: $span_kind, range_start: $range_start, range_end: $range_end, line_start: $line_start, line_end: $line_end, section_path: $section_path, content_sha256: $content_sha256, parser_version: $parser_version, extraction_receipt_event_id: $extraction_receipt, index_run_id: $index_run_id, display_snippet: $display_snippet } RETURN AFTER;",
            vec![
                b("span_id", span_id),
                b("source_id", new_span.source_id.clone()),
                b("span_kind", new_span.span_kind.as_str().to_owned()),
                b("range_start", new_span.range_start),
                b("range_end", new_span.range_end),
                b("line_start", new_span.line_start.map(i64::from)),
                b("line_end", new_span.line_end.map(i64::from)),
                b("section_path", new_span.section_path.clone()),
                b("content_sha256", new_span.content_sha256.clone()),
                b("parser_version", new_span.parser_version.clone()),
                b(
                    "extraction_receipt",
                    opt_thing(
                        KERNEL_EVENT_LEDGER_TABLE,
                        new_span.extraction_receipt_event_id.as_deref(),
                    ),
                ),
                b(
                    "index_run_id",
                    opt_thing(KNOWLEDGE_INDEX_RUNS_TABLE, new_span.index_run_id.as_deref()),
                ),
                b("display_snippet", new_span.display_snippet.clone()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge span CREATE returned no record".to_owned(),
            ))
            .and_then(span_to_domain)
    }

    async fn get_knowledge_span(&self, span_id: &str) -> StorageResult<Option<KnowledgeSpan>> {
        query_first_row::<SpanRecord>(
            self.storage(),
            "SELECT * FROM knowledge_spans WHERE span_id = $span_id;",
            vec![b("span_id", span_id.to_owned())],
        )
        .await?
        .map(span_to_domain)
        .transpose()
    }

    async fn list_knowledge_spans_for_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Vec<KnowledgeSpan>> {
        let rows: Vec<SpanRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_spans WHERE source_id = type::record('knowledge_sources', $source_id) ORDER BY range_start ASC, range_end ASC;",
            vec![b("source_id", source_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(span_to_domain).collect()
    }

    async fn upsert_knowledge_entity(
        &self,
        new_entity: NewKnowledgeEntity,
    ) -> StorageResult<KnowledgeEntity> {
        if new_entity.entity_key.trim().is_empty()
            || new_entity.entity_key.trim() != new_entity.entity_key
        {
            return Err(StorageError::Validation(
                "knowledge entity_key must be non-empty without surrounding whitespace",
            ));
        }
        if new_entity.display_name.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge entity display_name is required",
            ));
        }
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let entity_id = new_knowledge_id("KEN");
        // Statements: BEGIN(0) upsert(1) evidence-loop(2) select(3) COMMIT.
        let rows: Vec<EntityRecord> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             IF (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key LIMIT 1)[0] != NONE { UPDATE knowledge_entities SET display_name = $display_name, detection_provenance = $detection_provenance, primary_source_id = $primary_source_id ?? primary_source_id, last_detected_in_run = $detected_in_run ?? last_detected_in_run, lifecycle_state = 'active', updated_at = time::now() WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key RETURN NONE; } ELSE { CREATE type::record('knowledge_entities', $entity_id) CONTENT { entity_id: $entity_id, workspace_id: $workspace, entity_kind: $entity_kind, entity_key: $entity_key, display_name: $display_name, detection_provenance: $detection_provenance, primary_source_id: $primary_source_id, first_detected_in_run: $detected_in_run, last_detected_in_run: $detected_in_run } RETURN NONE; };\n\
             FOR $span_id IN $evidence_span_ids { LET $entity = (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key LIMIT 1)[0]; IF (SELECT VALUE id FROM knowledge_entity_spans WHERE entity_id = $entity AND span_id = type::record('knowledge_spans', $span_id) LIMIT 1)[0] = NONE { CREATE knowledge_entity_spans CONTENT { entity_id: $entity, span_id: type::record('knowledge_spans', $span_id), detected_in_run: $detected_in_run } RETURN NONE; }; };\n\
             SELECT * FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key LIMIT 1;\n\
             COMMIT TRANSACTION;",
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
        )
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge entity upsert returned no record".to_owned(),
            ))
            .and_then(entity_to_domain)
    }

    async fn get_knowledge_entity(
        &self,
        entity_id: &str,
    ) -> StorageResult<Option<KnowledgeEntity>> {
        query_first_row::<EntityRecord>(
            self.storage(),
            "SELECT * FROM knowledge_entities WHERE entity_id = $entity_id;",
            vec![b("entity_id", entity_id.to_owned())],
        )
        .await?
        .map(entity_to_domain)
        .transpose()
    }

    async fn get_knowledge_entity_by_identity(
        &self,
        workspace_id: &str,
        entity_kind: KnowledgeEntityKind,
        entity_key: &str,
    ) -> StorageResult<Option<KnowledgeEntity>> {
        query_first_row::<EntityRecord>(
            self.storage(),
            "SELECT * FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind AND entity_key = $entity_key;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("entity_kind", entity_kind.as_str().to_owned()),
                b("entity_key", entity_key.to_owned()),
            ],
        )
        .await?
        .map(entity_to_domain)
        .transpose()
    }

    async fn list_knowledge_entities_by_kind(
        &self,
        workspace_id: &str,
        entity_kind: KnowledgeEntityKind,
    ) -> StorageResult<Vec<KnowledgeEntity>> {
        let rows: Vec<EntityRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_entities WHERE workspace_id = $workspace AND entity_kind = $entity_kind ORDER BY entity_key ASC;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("entity_kind", entity_kind.as_str().to_owned()),
            ],
        )
        .await?;
        rows.into_iter().map(entity_to_domain).collect()
    }

    async fn list_knowledge_entity_span_ids(&self, entity_id: &str) -> StorageResult<Vec<String>> {
        let ids: Vec<RecordId> = query_rows(
            self.storage(),
            "SELECT VALUE span_id FROM knowledge_entity_spans WHERE entity_id = type::record('knowledge_entities', $entity_id);",
            vec![b("entity_id", entity_id.to_owned())],
        )
        .await?;
        let mut keys = ids
            .into_iter()
            .map(record_key)
            .collect::<StorageResult<Vec<_>>>()?;
        keys.sort();
        Ok(keys)
    }

    async fn replace_knowledge_entity_spans_for_source_kind(
        &self,
        entity_id: &str,
        source_id: &str,
        span_kind: KnowledgeSpanKind,
        evidence_span_ids: &[String],
        detected_in_run: Option<&str>,
    ) -> StorageResult<()> {
        let mut replacement_ids = evidence_span_ids.to_vec();
        replacement_ids.sort();
        replacement_ids.dedup();
        if replacement_ids.is_empty() {
            return Err(StorageError::Validation(
                "replacement entity evidence spans must not be empty",
            ));
        }
        let matching: Vec<RecordId> = query_rows(
            self.storage(),
            "SELECT VALUE id FROM knowledge_spans WHERE source_id = type::record('knowledge_sources', $source_id) AND span_kind = $span_kind AND span_id IN $replacement_ids;",
            vec![
                b("source_id", source_id.to_owned()),
                b("span_kind", span_kind.as_str().to_owned()),
                b("replacement_ids", replacement_ids.clone()),
            ],
        )
        .await?;
        if matching.len() != replacement_ids.len() {
            return Err(StorageError::Validation(
                "replacement entity evidence spans must all exist on the requested source/kind",
            ));
        }
        // Statements: BEGIN(0) delete(1) upsert-loop(2) COMMIT.
        raw_rows_at::<SurrealValueData>(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             DELETE knowledge_entity_spans WHERE entity_id = type::record('knowledge_entities', $entity_id) AND span_id.source_id = type::record('knowledge_sources', $source_id) AND span_id.span_kind = $span_kind AND span_id.span_id NOT IN $replacement_ids;\n\
             FOR $span_id IN $replacement_ids { LET $existing = (SELECT VALUE id FROM knowledge_entity_spans WHERE entity_id = type::record('knowledge_entities', $entity_id) AND span_id = type::record('knowledge_spans', $span_id) LIMIT 1)[0]; IF $existing != NONE { UPDATE $existing SET detected_in_run = $detected_in_run RETURN NONE; } ELSE { CREATE knowledge_entity_spans CONTENT { entity_id: type::record('knowledge_entities', $entity_id), span_id: type::record('knowledge_spans', $span_id), detected_in_run: $detected_in_run } RETURN NONE; }; };\n\
             COMMIT TRANSACTION;",
            vec![
                b("entity_id", entity_id.to_owned()),
                b("source_id", source_id.to_owned()),
                b("span_kind", span_kind.as_str().to_owned()),
                b("replacement_ids", replacement_ids),
                b(
                    "detected_in_run",
                    opt_thing(KNOWLEDGE_INDEX_RUNS_TABLE, detected_in_run),
                ),
            ],
            0,
        )
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn retire_knowledge_entity(&self, entity_id: &str) -> StorageResult<KnowledgeEntity> {
        let rows: Vec<EntityRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_entities SET lifecycle_state = 'retired', updated_at = time::now() WHERE entity_id = $entity_id RETURN AFTER;",
            vec![b("entity_id", entity_id.to_owned())],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge entity"))
            .and_then(entity_to_domain)
    }

    async fn upsert_knowledge_edge(
        &self,
        new_edge: NewKnowledgeEdge,
    ) -> StorageResult<KnowledgeEdge> {
        if new_edge.evidence_span_ids.is_empty() {
            return Err(StorageError::Validation(
                "knowledge edge MUST carry at least one source span ref (spec 2.3.13.11)",
            ));
        }
        if !(0.0..=1.0).contains(&new_edge.confidence) {
            return Err(StorageError::Validation(
                "knowledge edge confidence must be within [0.0, 1.0]",
            ));
        }
        if new_edge.extractor_version.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge edge extractor_version is required",
            ));
        }
        let source = self
            .get_knowledge_entity(&new_edge.source_entity_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge edge source entity"))?;
        let target = self
            .get_knowledge_entity(&new_edge.target_entity_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge edge target entity"))?;
        if source.workspace_id != new_edge.workspace_id
            || target.workspace_id != new_edge.workspace_id
        {
            return Err(StorageError::Validation(
                "knowledge edge entities must belong to the edge workspace",
            ));
        }
        let relationship_id = derive_knowledge_relationship_id(
            new_edge.edge_type,
            source.entity_kind,
            &source.entity_key,
            target.entity_kind,
            &target.entity_key,
        );
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let edge_id = new_knowledge_id("KED");
        // Statements: BEGIN(0) upsert(1) evidence-loop(2) select(3) COMMIT.
        let rows: Vec<EdgeRecord> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             IF (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace AND relationship_id = $relationship_id LIMIT 1)[0] != NONE { UPDATE knowledge_edges SET confidence = $confidence, extractor_version = $extractor_version, last_seen_in_run = $detected_in_run ?? last_seen_in_run, updated_at = time::now() WHERE workspace_id = $workspace AND relationship_id = $relationship_id RETURN NONE; } ELSE { CREATE type::record('knowledge_edges', $edge_id) CONTENT { edge_id: $edge_id, workspace_id: $workspace, relationship_id: $relationship_id, edge_type: $edge_type, source_entity_id: type::record('knowledge_entities', $source_entity_id), target_entity_id: type::record('knowledge_entities', $target_entity_id), extractor_version: $extractor_version, confidence: $confidence, created_in_run: $detected_in_run, last_seen_in_run: $detected_in_run } RETURN NONE; };\n\
             FOR $span_id IN $evidence_span_ids { LET $edge = (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace AND relationship_id = $relationship_id LIMIT 1)[0]; IF (SELECT VALUE id FROM knowledge_edge_spans WHERE edge_id = $edge AND span_id = type::record('knowledge_spans', $span_id) LIMIT 1)[0] = NONE { CREATE knowledge_edge_spans CONTENT { edge_id: $edge, span_id: type::record('knowledge_spans', $span_id), recorded_in_run: $detected_in_run } RETURN NONE; }; };\n\
             SELECT * FROM knowledge_edges WHERE workspace_id = $workspace AND relationship_id = $relationship_id LIMIT 1;\n\
             COMMIT TRANSACTION;",
            vec![
                b("edge_id", edge_id),
                b("workspace", thing(WORKSPACES_TABLE, &new_edge.workspace_id)),
                b("relationship_id", relationship_id),
                b("edge_type", new_edge.edge_type.as_str().to_owned()),
                b("source_entity_id", new_edge.source_entity_id.clone()),
                b("target_entity_id", new_edge.target_entity_id.clone()),
                b("extractor_version", new_edge.extractor_version.clone()),
                b("confidence", new_edge.confidence),
                b(
                    "detected_in_run",
                    opt_thing(
                        KNOWLEDGE_INDEX_RUNS_TABLE,
                        new_edge.detected_in_run.as_deref(),
                    ),
                ),
                b("evidence_span_ids", new_edge.evidence_span_ids.clone()),
            ],
            3,
        )
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge edge upsert returned no record".to_owned(),
            ))
            .and_then(edge_to_domain)
    }

    async fn get_knowledge_edge(&self, edge_id: &str) -> StorageResult<Option<KnowledgeEdge>> {
        query_first_row::<EdgeRecord>(
            self.storage(),
            "SELECT * FROM knowledge_edges WHERE edge_id = $edge_id;",
            vec![b("edge_id", edge_id.to_owned())],
        )
        .await?
        .map(edge_to_domain)
        .transpose()
    }

    async fn get_knowledge_edge_by_relationship_id(
        &self,
        workspace_id: &str,
        relationship_id: &str,
    ) -> StorageResult<Option<KnowledgeEdge>> {
        query_first_row::<EdgeRecord>(
            self.storage(),
            "SELECT * FROM knowledge_edges WHERE workspace_id = $workspace AND relationship_id = $relationship_id;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("relationship_id", relationship_id.to_owned()),
            ],
        )
        .await?
        .map(edge_to_domain)
        .transpose()
    }

    async fn list_knowledge_edges_for_entity(
        &self,
        entity_id: &str,
    ) -> StorageResult<Vec<KnowledgeEdge>> {
        let rows: Vec<EdgeRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_edges WHERE source_entity_id = type::record('knowledge_entities', $entity_id) OR target_entity_id = type::record('knowledge_entities', $entity_id) ORDER BY relationship_id ASC;",
            vec![b("entity_id", entity_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(edge_to_domain).collect()
    }

    async fn list_knowledge_edge_span_ids(&self, edge_id: &str) -> StorageResult<Vec<String>> {
        let ids: Vec<RecordId> = query_rows(
            self.storage(),
            "SELECT VALUE span_id FROM knowledge_edge_spans WHERE edge_id = type::record('knowledge_edges', $edge_id);",
            vec![b("edge_id", edge_id.to_owned())],
        )
        .await?;
        let mut keys = ids
            .into_iter()
            .map(record_key)
            .collect::<StorageResult<Vec<_>>>()?;
        keys.sort();
        Ok(keys)
    }

    async fn set_knowledge_edge_lifecycle(
        &self,
        edge_id: &str,
        lifecycle: KnowledgeEdgeLifecycle,
        conflict_marker: Option<JsonValue>,
    ) -> StorageResult<KnowledgeEdge> {
        if matches!(lifecycle, KnowledgeEdgeLifecycle::Conflicted) && conflict_marker.is_none() {
            return Err(StorageError::Validation(
                "conflicted knowledge edges must carry a conflict marker",
            ));
        }
        let rows: Vec<EdgeRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_edges SET lifecycle_state = $lifecycle_state, conflict_marker = $conflict_marker, updated_at = time::now() WHERE edge_id = $edge_id RETURN AFTER;",
            vec![
                b("edge_id", edge_id.to_owned()),
                b("lifecycle_state", lifecycle.as_str().to_owned()),
                b("conflict_marker", conflict_marker),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge edge"))
            .and_then(edge_to_domain)
    }

    async fn create_knowledge_claim(
        &self,
        new_claim: NewKnowledgeClaim,
    ) -> StorageResult<KnowledgeClaim> {
        if new_claim.evidence_span_ids.is_empty() {
            return Err(StorageError::Validation(
                "knowledge claim MUST carry evidence spans (spec 2.3.13.11)",
            ));
        }
        if new_claim.claim_text.trim().is_empty() {
            return Err(StorageError::Validation("knowledge claim_text is required"));
        }
        if !(0.0..=1.0).contains(&new_claim.confidence) {
            return Err(StorageError::Validation(
                "knowledge claim confidence must be within [0.0, 1.0]",
            ));
        }
        let claim_id = new_knowledge_id("KCL");
        // Statements: BEGIN(0) create(1) evidence-loop(2) COMMIT.
        let rows: Vec<ClaimRecord> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             CREATE type::record('knowledge_claims', $claim_id) CONTENT { claim_id: $claim_id, workspace_id: $workspace, claim_kind: $claim_kind, claim_text: $claim_text, subject_entity_id: $subject_entity_id, temporal_qualifier: $temporal_qualifier, granularity_qualifier: $granularity_qualifier, confidence: $confidence, proposed_in_run: $proposed_in_run } RETURN AFTER;\n\
             FOR $span_id IN $evidence_span_ids { IF (SELECT VALUE id FROM knowledge_claim_spans WHERE claim_id = type::record('knowledge_claims', $claim_id) AND span_id = type::record('knowledge_spans', $span_id) LIMIT 1)[0] = NONE { CREATE knowledge_claim_spans CONTENT { claim_id: type::record('knowledge_claims', $claim_id), span_id: type::record('knowledge_spans', $span_id) } RETURN NONE; }; };\n\
             COMMIT TRANSACTION;",
            vec![
                b("claim_id", claim_id),
                b("workspace", thing(WORKSPACES_TABLE, &new_claim.workspace_id)),
                b("claim_kind", new_claim.claim_kind.as_str().to_owned()),
                b("claim_text", new_claim.claim_text.clone()),
                b(
                    "subject_entity_id",
                    opt_thing(
                        KNOWLEDGE_ENTITIES_TABLE,
                        new_claim.subject_entity_id.as_deref(),
                    ),
                ),
                b("temporal_qualifier", new_claim.temporal_qualifier.clone()),
                b(
                    "granularity_qualifier",
                    new_claim.granularity_qualifier.clone(),
                ),
                b("confidence", new_claim.confidence),
                b(
                    "proposed_in_run",
                    opt_thing(
                        KNOWLEDGE_INDEX_RUNS_TABLE,
                        new_claim.proposed_in_run.as_deref(),
                    ),
                ),
                b("evidence_span_ids", new_claim.evidence_span_ids.clone()),
            ],
            1,
        )
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge claim CREATE returned no record".to_owned(),
            ))
            .and_then(claim_to_domain)
    }

    async fn get_knowledge_claim(&self, claim_id: &str) -> StorageResult<Option<KnowledgeClaim>> {
        query_first_row::<ClaimRecord>(
            self.storage(),
            "SELECT * FROM knowledge_claims WHERE claim_id = $claim_id;",
            vec![b("claim_id", claim_id.to_owned())],
        )
        .await?
        .map(claim_to_domain)
        .transpose()
    }

    async fn list_knowledge_claim_span_ids(&self, claim_id: &str) -> StorageResult<Vec<String>> {
        let ids: Vec<RecordId> = query_rows(
            self.storage(),
            "SELECT VALUE span_id FROM knowledge_claim_spans WHERE claim_id = type::record('knowledge_claims', $claim_id);",
            vec![b("claim_id", claim_id.to_owned())],
        )
        .await?;
        let mut keys = ids
            .into_iter()
            .map(record_key)
            .collect::<StorageResult<Vec<_>>>()?;
        keys.sort();
        Ok(keys)
    }

    async fn transition_knowledge_claim(
        &self,
        claim_id: &str,
        to_state: KnowledgeClaimState,
        retirement: Option<KnowledgeClaimRetirement>,
        resolution_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeClaim> {
        let current = self
            .get_knowledge_claim(claim_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge claim"))?;
        if !current.lifecycle_state.can_transition_to(to_state) {
            return Err(StorageError::Conflict(
                "knowledge claim lifecycle violation: transition not allowed",
            ));
        }
        if matches!(
            (current.lifecycle_state, to_state),
            (
                KnowledgeClaimState::Conflicted,
                KnowledgeClaimState::Accepted
            ) | (
                KnowledgeClaimState::Conflicted,
                KnowledgeClaimState::Retired
            )
        ) {
            let unresolved: Vec<RecordId> = query_rows(
                self.storage(),
                "SELECT VALUE id FROM knowledge_claim_conflicts WHERE resolved_at = NONE AND (claim_id = type::record('knowledge_claims', $claim_id) OR conflicting_claim_id = type::record('knowledge_claims', $claim_id));",
                vec![b("claim_id", claim_id.to_owned())],
            )
            .await?;
            if !unresolved.is_empty() {
                return Err(StorageError::Conflict(
                    "knowledge claim lifecycle violation: unresolved conflicts must be receipt-resolved before exiting conflicted",
                ));
            }
            let Some(resolution_receipt_event_id) = resolution_receipt_event_id else {
                return Err(StorageError::Conflict(
                    "knowledge claim lifecycle violation: exiting conflicted requires a conflict-resolution receipt",
                ));
            };
            let matching_resolved: Vec<RecordId> = query_rows(
                self.storage(),
                "SELECT VALUE id FROM knowledge_claim_conflicts WHERE resolved_at != NONE AND resolution_receipt_event_id = $receipt AND (claim_id = type::record('knowledge_claims', $claim_id) OR conflicting_claim_id = type::record('knowledge_claims', $claim_id));",
                vec![
                    b("claim_id", claim_id.to_owned()),
                    b(
                        "receipt",
                        thing(KERNEL_EVENT_LEDGER_TABLE, resolution_receipt_event_id),
                    ),
                ],
            )
            .await?;
            if matching_resolved.is_empty() {
                return Err(StorageError::Conflict(
                    "knowledge claim lifecycle violation: exiting conflicted receipt must match a resolved conflict for the claim",
                ));
            }
            let matching_authority: Vec<RecordId> = query_rows(
                self.storage(),
                "SELECT VALUE id FROM knowledge_claim_conflicts WHERE resolved_at != NONE AND resolution_receipt_event_id = $receipt AND (claim_id = type::record('knowledge_claims', $claim_id) OR conflicting_claim_id = type::record('knowledge_claims', $claim_id)) AND resolution_receipt_event_id.aggregate_type = 'knowledge_claim_conflict' AND resolution_receipt_event_id.aggregate_id = conflict_id;",
                vec![
                    b("claim_id", claim_id.to_owned()),
                    b(
                        "receipt",
                        thing(KERNEL_EVENT_LEDGER_TABLE, resolution_receipt_event_id),
                    ),
                ],
            )
            .await?;
            if matching_authority.is_empty() {
                return Err(StorageError::Conflict(
                    "knowledge claim lifecycle violation: conflict resolution receipt aggregate mismatch",
                ));
            }
        }
        let (retirement_reason, superseded_by) = match (to_state, retirement) {
            (KnowledgeClaimState::Retired, Some(retirement)) => {
                if matches!(
                    retirement.reason,
                    KnowledgeClaimRetirementReason::Superseded
                ) && retirement.superseded_by_claim_id.is_none()
                {
                    return Err(StorageError::Validation(
                        "superseded claims must name superseded_by_claim_id",
                    ));
                }
                if !matches!(
                    retirement.reason,
                    KnowledgeClaimRetirementReason::Superseded
                ) && retirement.superseded_by_claim_id.is_some()
                {
                    return Err(StorageError::Validation(
                        "superseded_by_claim_id requires retirement reason 'superseded'",
                    ));
                }
                (
                    Some(retirement.reason.as_str().to_owned()),
                    retirement.superseded_by_claim_id,
                )
            }
            (KnowledgeClaimState::Retired, None) => {
                return Err(StorageError::Validation(
                    "retiring a knowledge claim requires a retirement reason",
                ));
            }
            (_, Some(_)) => {
                return Err(StorageError::Validation(
                    "retirement payload only applies when entering 'retired'",
                ));
            }
            (_, None) => (None, None),
        };

        // Optimistic transition: the WHERE pins the observed source state so a
        // concurrent transition cannot be silently overwritten.
        let rows: Vec<ClaimRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_claims SET lifecycle_state = $to_state, retirement_reason = $retirement_reason, superseded_by_claim_id = $superseded_by, resolution_receipt_event_id = $receipt ?? resolution_receipt_event_id, updated_at = time::now() WHERE claim_id = $claim_id AND lifecycle_state = $from_state RETURN AFTER;",
            vec![
                b("claim_id", claim_id.to_owned()),
                b("from_state", current.lifecycle_state.as_str().to_owned()),
                b("to_state", to_state.as_str().to_owned()),
                b("retirement_reason", retirement_reason),
                b(
                    "superseded_by",
                    opt_thing(KNOWLEDGE_CLAIMS_TABLE, superseded_by.as_deref()),
                ),
                b(
                    "receipt",
                    opt_thing(KERNEL_EVENT_LEDGER_TABLE, resolution_receipt_event_id),
                ),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Conflict(
                "knowledge claim lifecycle violation: state changed concurrently",
            ))
            .and_then(claim_to_domain)
    }

    async fn record_knowledge_claim_conflict(
        &self,
        claim_id: &str,
        conflicting_claim_id: &str,
        conflict_reason: &str,
        detected_in_run: Option<&str>,
    ) -> StorageResult<KnowledgeClaimConflict> {
        if claim_id == conflicting_claim_id {
            return Err(StorageError::Validation(
                "a knowledge claim cannot conflict with itself",
            ));
        }
        if conflict_reason.trim().is_empty() {
            return Err(StorageError::Validation(
                "knowledge claim conflict_reason is required",
            ));
        }
        let conflict_id = new_knowledge_id("KCC");
        // Statements: BEGIN(0) reverse-guard(1) create(2) claims-update(3) COMMIT.
        let result: Result<Vec<ConflictRecord>, SurrealStorageError> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             IF (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id = type::record('knowledge_claims', $conflicting_claim_id) AND conflicting_claim_id = type::record('knowledge_claims', $claim_id) LIMIT 1)[0] != NONE { THROW 'HSK-KCC-REVERSE-EXISTS'; };\n\
             CREATE type::record('knowledge_claim_conflicts', $conflict_id) CONTENT { conflict_id: $conflict_id, claim_id: type::record('knowledge_claims', $claim_id), conflicting_claim_id: type::record('knowledge_claims', $conflicting_claim_id), conflict_reason: $conflict_reason, detected_in_run: $detected_in_run } RETURN AFTER;\n\
             UPDATE knowledge_claims SET lifecycle_state = 'conflicted', updated_at = time::now() WHERE claim_id IN [$claim_id, $conflicting_claim_id] AND lifecycle_state IN ['proposed', 'accepted'] RETURN NONE;\n\
             COMMIT TRANSACTION;",
            vec![
                b("conflict_id", conflict_id),
                b("claim_id", claim_id.to_owned()),
                b("conflicting_claim_id", conflicting_claim_id.to_owned()),
                b("conflict_reason", conflict_reason.to_owned()),
                b(
                    "detected_in_run",
                    opt_thing(KNOWLEDGE_INDEX_RUNS_TABLE, detected_in_run),
                ),
            ],
            2,
        )
        .await;
        let rows = result.map_err(|error| {
            map_guarded_err(
                error,
                &[("HSK-KCC-REVERSE-EXISTS", || {
                    StorageError::Conflict(
                        "knowledge claim conflict pair already exists in reverse order",
                    )
                })],
            )
        })?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge claim conflict CREATE returned no record".to_owned(),
            ))
            .and_then(conflict_to_domain)
    }

    async fn resolve_knowledge_claim_conflict(
        &self,
        conflict_id: &str,
        resolution_receipt_event_id: &str,
    ) -> StorageResult<KnowledgeClaimConflict> {
        let receipt_aggregate: Option<LedgerAggregateRecord> = query_first_row(
            self.storage(),
            "SELECT aggregate_type, aggregate_id FROM $receipt;",
            vec![b(
                "receipt",
                thing(KERNEL_EVENT_LEDGER_TABLE, resolution_receipt_event_id),
            )],
        )
        .await?;
        if let Some(aggregate) = receipt_aggregate {
            if aggregate.aggregate_type != "knowledge_claim_conflict"
                || aggregate.aggregate_id != conflict_id
            {
                return Err(StorageError::Conflict(
                    "knowledge claim conflict resolution receipt aggregate mismatch",
                ));
            }
        }
        let rows: Vec<ConflictRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_claim_conflicts SET resolution_receipt_event_id = $receipt, resolved_at = time::now() WHERE conflict_id = $conflict_id AND resolved_at = NONE RETURN AFTER;",
            vec![
                b("conflict_id", conflict_id.to_owned()),
                b(
                    "receipt",
                    thing(KERNEL_EVENT_LEDGER_TABLE, resolution_receipt_event_id),
                ),
            ],
        )
        .await?;
        match rows.into_iter().next() {
            Some(row) => conflict_to_domain(row),
            None => {
                let exists: Vec<RecordId> = query_rows(
                    self.storage(),
                    "SELECT VALUE id FROM knowledge_claim_conflicts WHERE conflict_id = $conflict_id;",
                    vec![b("conflict_id", conflict_id.to_owned())],
                )
                .await?;
                if exists.is_empty() {
                    Err(StorageError::NotFound("knowledge claim conflict"))
                } else {
                    Err(StorageError::Conflict(
                        "knowledge claim conflict is already resolved",
                    ))
                }
            }
        }
    }

    async fn list_knowledge_claim_conflicts(
        &self,
        claim_id: &str,
    ) -> StorageResult<Vec<KnowledgeClaimConflict>> {
        let rows: Vec<ConflictRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_claim_conflicts WHERE claim_id = type::record('knowledge_claims', $claim_id) OR conflicting_claim_id = type::record('knowledge_claims', $claim_id) ORDER BY detected_at ASC;",
            vec![b("claim_id", claim_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(conflict_to_domain).collect()
    }

    async fn create_knowledge_memory_passage(
        &self,
        new_passage: NewKnowledgeMemoryPassage,
    ) -> StorageResult<KnowledgeMemoryPassage> {
        validate_new_passage(&new_passage)?;
        let passage_id = new_knowledge_id("KMP");
        // Statements: BEGIN(0) create(1) lineage-loop(2) COMMIT.
        let statement =
            format!("BEGIN TRANSACTION;\n{PASSAGE_INSERT_STATEMENTS}\nCOMMIT TRANSACTION;");
        let rows: Vec<PassageRecord> = raw_rows_at(
            self.storage(),
            statement,
            passage_insert_binds(&passage_id, &new_passage),
            1,
        )
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge memory passage CREATE returned no record".to_owned(),
            ))
            .and_then(passage_to_domain)
    }

    async fn get_knowledge_memory_passage(
        &self,
        passage_id: &str,
    ) -> StorageResult<Option<KnowledgeMemoryPassage>> {
        query_first_row::<PassageRecord>(
            self.storage(),
            "SELECT * FROM knowledge_memory_passages WHERE passage_id = $passage_id;",
            vec![b("passage_id", passage_id.to_owned())],
        )
        .await?
        .map(passage_to_domain)
        .transpose()
    }

    async fn list_knowledge_passage_evidence(
        &self,
        passage_id: &str,
    ) -> StorageResult<Vec<KnowledgePassageEvidenceRef>> {
        let rows: Vec<EvidenceRecord> = query_rows(
            self.storage(),
            "SELECT ref_kind, source_id, claim_id, span_id, ordinal FROM knowledge_passage_evidence WHERE passage_id = type::record('knowledge_memory_passages', $passage_id) ORDER BY ordinal ASC;",
            vec![b("passage_id", passage_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(evidence_to_domain).collect()
    }

    async fn set_knowledge_passage_compaction(
        &self,
        passage_id: &str,
        compaction_policy: KnowledgeCompactionPolicy,
        refresh_freshness: bool,
    ) -> StorageResult<KnowledgeMemoryPassage> {
        let rows: Vec<PassageRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_memory_passages SET compaction_policy = $compaction_policy, freshness_at = IF $refresh_freshness { time::now() } ELSE { freshness_at }, updated_at = time::now() WHERE passage_id = $passage_id RETURN AFTER;",
            vec![
                b("passage_id", passage_id.to_owned()),
                b(
                    "compaction_policy",
                    compaction_policy.as_str().to_owned(),
                ),
                b("refresh_freshness", refresh_freshness),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge memory passage"))
            .and_then(passage_to_domain)
    }

    async fn upsert_knowledge_wiki_projection(
        &self,
        new_projection: NewKnowledgeWikiProjection,
    ) -> StorageResult<KnowledgeWikiProjection> {
        if new_projection.title.trim() != new_projection.title || new_projection.title.is_empty() {
            return Err(StorageError::Validation(
                "knowledge projection title must be non-empty and trimmed",
            ));
        }
        if !is_sha256_hex(&new_projection.staleness_hash) {
            return Err(StorageError::Validation(
                "knowledge projection staleness_hash must be lowercase sha256 hex",
            ));
        }
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let projection_id = new_knowledge_id("KWP");
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            "IF (SELECT VALUE id FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = $projection_kind AND title = $title LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_wiki_projections SET source_records = $source_records, rendered_content = $rendered_content, rebuild_status = 'stale', staleness_hash = $staleness_hash, updated_at = time::now() WHERE workspace_id = $workspace AND projection_kind = $projection_kind AND title = $title RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_wiki_projections', $projection_id) CONTENT { projection_id: $projection_id, workspace_id: $workspace, projection_kind: $projection_kind, title: $title, source_records: $source_records, rendered_content: $rendered_content, rebuild_status: 'stale', staleness_hash: $staleness_hash } RETURN AFTER; };",
            vec![
                b("projection_id", projection_id),
                b(
                    "workspace",
                    thing(WORKSPACES_TABLE, &new_projection.workspace_id),
                ),
                b(
                    "projection_kind",
                    new_projection.projection_kind.as_str().to_owned(),
                ),
                b("title", new_projection.title.clone()),
                b("source_records", new_projection.source_records.clone()),
                b("rendered_content", new_projection.rendered_content.clone()),
                b("staleness_hash", new_projection.staleness_hash.clone()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge wiki projection upsert returned no record".to_owned(),
            ))
            .and_then(projection_to_domain)
    }

    async fn get_knowledge_wiki_projection(
        &self,
        projection_id: &str,
    ) -> StorageResult<Option<KnowledgeWikiProjection>> {
        query_first_row::<ProjectionRecord>(
            self.storage(),
            "SELECT * FROM knowledge_wiki_projections WHERE projection_id = $projection_id;",
            vec![b("projection_id", projection_id.to_owned())],
        )
        .await?
        .map(projection_to_domain)
        .transpose()
    }

    async fn mark_knowledge_projection_rebuilt(
        &self,
        projection_id: &str,
        staleness_hash: &str,
        rendered_content: &str,
        rebuild_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeWikiProjection> {
        if !is_sha256_hex(staleness_hash) {
            return Err(StorageError::Validation(
                "knowledge projection staleness_hash must be lowercase sha256 hex",
            ));
        }
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_wiki_projections SET rebuild_status = 'fresh', staleness_hash = $staleness_hash, rendered_content = $rendered_content, rebuild_receipt_event_id = $rebuild_receipt, last_rebuilt_at = time::now(), updated_at = time::now() WHERE projection_id = $projection_id RETURN AFTER;",
            vec![
                b("projection_id", projection_id.to_owned()),
                b("staleness_hash", staleness_hash.to_owned()),
                b("rendered_content", rendered_content.to_owned()),
                b(
                    "rebuild_receipt",
                    opt_thing(KERNEL_EVENT_LEDGER_TABLE, rebuild_receipt_event_id),
                ),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge wiki projection"))
            .and_then(projection_to_domain)
    }

    async fn set_knowledge_projection_rebuild_status(
        &self,
        projection_id: &str,
        rebuild_status: KnowledgeRebuildStatus,
    ) -> StorageResult<KnowledgeWikiProjection> {
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_wiki_projections SET rebuild_status = $rebuild_status, updated_at = time::now() WHERE projection_id = $projection_id RETURN AFTER;",
            vec![
                b("projection_id", projection_id.to_owned()),
                b("rebuild_status", rebuild_status.as_str().to_owned()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge wiki projection"))
            .and_then(projection_to_domain)
    }

    async fn delete_knowledge_wiki_projection(&self, projection_id: &str) -> StorageResult<()> {
        let deleted: Vec<SurrealValueData> = query_rows(
            self.storage(),
            "DELETE knowledge_wiki_projections WHERE projection_id = $projection_id RETURN BEFORE;",
            vec![b("projection_id", projection_id.to_owned())],
        )
        .await?;
        if deleted.is_empty() {
            return Err(StorageError::NotFound("knowledge wiki projection"));
        }
        Ok(())
    }

    async fn create_knowledge_rich_document(
        &self,
        new_document: NewKnowledgeRichDocument,
    ) -> StorageResult<KnowledgeRichDocument> {
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        create_rich_document_locked(self.storage(), &new_document).await
    }

    async fn create_knowledge_rich_document_if_title_absent(
        &self,
        new_document: NewKnowledgeRichDocument,
    ) -> StorageResult<(KnowledgeRichDocument, bool)> {
        if new_document.title.trim() != new_document.title || new_document.title.is_empty() {
            return Err(StorageError::Validation(
                "knowledge rich document title must be non-empty and trimmed",
            ));
        }
        // The mutation lock serializes independent creators of the same title
        // exactly as the removed backend's per-title advisory lock did (the
        // embedded store is single-process, so process-local is sufficient).
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        let normalized_title = normalize_rich_document_title(&new_document.title);
        let candidates: Vec<DocTitleRecord> = query_rows(
            self.storage(),
            "SELECT rich_document_id, title, updated_at FROM knowledge_rich_documents WHERE workspace_id = $workspace AND deleted_at = NONE;",
            vec![b(
                "workspace",
                thing(WORKSPACES_TABLE, &new_document.workspace_id),
            )],
        )
        .await?;
        let mut matches: Vec<DocTitleRecord> = candidates
            .into_iter()
            .filter(|row| normalize_rich_document_title(&row.title) == normalized_title)
            .collect();
        matches.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.rich_document_id.cmp(&right.rich_document_id))
        });
        match matches.len() {
            0 => {
                let document = create_rich_document_locked(self.storage(), &new_document).await?;
                Ok((document, true))
            }
            1 => {
                let document =
                    read_live_rich_document(self.storage(), &matches[0].rich_document_id)
                        .await?
                        .ok_or(StorageError::NotFound("knowledge rich document"))?;
                Ok((document, false))
            }
            _ => Err(StorageError::Conflict(
                "knowledge_rich_document_title_ambiguous",
            )),
        }
    }

    async fn get_knowledge_rich_document(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>> {
        read_live_rich_document(self.storage(), rich_document_id).await
    }

    async fn get_knowledge_rich_document_by_document_id(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocument>> {
        query_first_row::<RichDocRecord>(
            self.storage(),
            "SELECT * FROM knowledge_rich_documents WHERE workspace_id = $workspace AND document_id = type::record('documents', $document_id) AND deleted_at = NONE ORDER BY updated_at DESC, rich_document_id DESC LIMIT 1;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("document_id", document_id.to_owned()),
            ],
        )
        .await?
        .map(rich_document_to_domain)
        .transpose()
    }

    async fn get_knowledge_rich_document_draft(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Option<KnowledgeRichDocumentDraft>> {
        query_first_row::<DraftRecord>(
            self.storage(),
            "SELECT * FROM knowledge_rich_document_drafts WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id);",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?
        .map(draft_to_domain)
        .transpose()
    }

    async fn upsert_knowledge_rich_document_draft(
        &self,
        upsert: UpsertKnowledgeRichDocumentDraft,
    ) -> StorageResult<KnowledgeRichDocumentDraft> {
        if upsert.base_doc_version < 1 {
            return Err(StorageError::Validation(
                "knowledge rich document draft base_doc_version must be >= 1",
            ));
        }
        if !is_sha256_hex(&upsert.base_content_sha256) {
            return Err(StorageError::Validation(
                "knowledge rich document draft base_content_sha256 must be a sha256 hex digest",
            ));
        }
        for value in [
            upsert.actor_kind.as_str(),
            upsert.actor_id.as_str(),
            upsert.kernel_task_run_id.as_str(),
            upsert.session_run_id.as_str(),
        ] {
            if value.trim() != value || value.is_empty() {
                return Err(StorageError::Validation(
                    "knowledge rich document draft identity fields must be non-empty and trimmed",
                ));
            }
        }
        if !matches!(
            upsert.actor_kind.as_str(),
            "operator" | "local_model" | "cloud_model" | "validator" | "system" | "unauthenticated"
        ) {
            return Err(StorageError::Validation(
                "knowledge rich document draft actor_kind must be a document actor kind",
            ));
        }
        let draft_content_sha256 = knowledge_canonical_json_sha256(&upsert.content_json);

        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        let document = read_live_rich_document(self.storage(), &upsert.rich_document_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        // Statements: BEGIN(0) live-guard(1) upsert(2) COMMIT.
        let result: Result<Vec<DraftRecord>, SurrealStorageError> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             IF (SELECT VALUE id FROM knowledge_rich_documents WHERE rich_document_id = $doc_id AND deleted_at = NONE LIMIT 1)[0] = NONE { THROW 'HSK-KRD-NOT-FOUND'; };\n\
             UPSERT type::record('knowledge_rich_document_drafts', type::record('knowledge_rich_documents', $doc_id)) SET rich_document_id = type::record('knowledge_rich_documents', $doc_id), workspace_id = $workspace, base_doc_version = $base_doc_version, base_content_sha256 = $base_content_sha256, draft_content_json = $draft_content_json, draft_content_sha256 = $draft_content_sha256, actor_kind = $actor_kind, actor_id = $actor_id, kernel_task_run_id = $kernel_task_run_id, session_run_id = $session_run_id, updated_at = time::now() RETURN AFTER;\n\
             COMMIT TRANSACTION;",
            vec![
                b("doc_id", upsert.rich_document_id.clone()),
                b("workspace", thing(WORKSPACES_TABLE, &document.workspace_id)),
                b("base_doc_version", upsert.base_doc_version),
                b("base_content_sha256", upsert.base_content_sha256.clone()),
                b("draft_content_json", upsert.content_json.clone()),
                b("draft_content_sha256", draft_content_sha256),
                b("actor_kind", upsert.actor_kind.clone()),
                b("actor_id", upsert.actor_id.clone()),
                b("kernel_task_run_id", upsert.kernel_task_run_id.clone()),
                b("session_run_id", upsert.session_run_id.clone()),
            ],
            2,
        )
        .await;
        let rows = result.map_err(|error| {
            map_guarded_err(
                error,
                &[("HSK-KRD-NOT-FOUND", || {
                    StorageError::NotFound("knowledge rich document")
                })],
            )
        })?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge rich document"))
            .and_then(draft_to_domain)
    }

    async fn clear_knowledge_rich_document_draft(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<bool> {
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        read_live_rich_document(self.storage(), rich_document_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        let deleted: Vec<SurrealValueData> = query_rows(
            self.storage(),
            "DELETE knowledge_rich_document_drafts WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) RETURN BEFORE;",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?;
        Ok(!deleted.is_empty())
    }

    async fn save_knowledge_rich_document_version(
        &self,
        rich_document_id: &str,
        expected_version: i64,
        content_json: JsonValue,
        crdt_document_id: Option<&str>,
        crdt_snapshot_id: Option<&str>,
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeRichDocument> {
        let next_version = checked_next_rich_document_version(expected_version)?;
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        save_rich_document_version_locked(
            self.storage(),
            rich_document_id,
            expected_version,
            next_version,
            &content_json,
            crdt_document_id,
            crdt_snapshot_id,
            promotion_receipt_event_id,
            None,
        )
        .await?
        .ok_or(StorageError::Database(
            "knowledge rich document save without an idempotency claim cannot lose a key race"
                .to_owned(),
        ))
    }

    async fn list_knowledge_rich_document_versions(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeRichDocumentVersion>> {
        let rows: Vec<VersionRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_rich_document_versions WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) ORDER BY doc_version ASC;",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(version_to_domain).collect()
    }

    async fn list_knowledge_rich_document_version_metas(
        &self,
        rich_document_id: &str,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<KnowledgeRichDocumentVersionMeta>> {
        // MT-156: metadata only - content_json is deliberately NOT selected so
        // a long history can never balloon the response.
        let rows: Vec<VersionMetaRecord> = query_rows(
            self.storage(),
            "SELECT rich_document_id, doc_version, schema_version, content_sha256, crdt_snapshot_id, promotion_receipt_event_id, created_at FROM knowledge_rich_document_versions WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) ORDER BY doc_version ASC LIMIT $limit START $offset;",
            vec![
                b("doc_id", rich_document_id.to_owned()),
                b("limit", limit.max(0)),
                b("offset", offset.max(0)),
            ],
        )
        .await?;
        rows.into_iter().map(version_meta_to_domain).collect()
    }

    async fn count_knowledge_rich_document_versions(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<i64> {
        let counts: Vec<i64> = query_rows(
            self.storage(),
            "RETURN array::len((SELECT VALUE id FROM knowledge_rich_document_versions WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id)));",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?;
        Ok(counts.into_iter().next().unwrap_or(0))
    }

    async fn get_knowledge_rich_document_version(
        &self,
        rich_document_id: &str,
        doc_version: i64,
    ) -> StorageResult<Option<KnowledgeRichDocumentVersion>> {
        query_first_row::<VersionRecord>(
            self.storage(),
            "SELECT * FROM knowledge_rich_document_versions WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) AND doc_version = $doc_version;",
            vec![
                b("doc_id", rich_document_id.to_owned()),
                b("doc_version", doc_version),
            ],
        )
        .await?
        .map(version_to_domain)
        .transpose()
    }

    async fn rename_knowledge_rich_document(
        &self,
        rich_document_id: &str,
        title: &str,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> StorageResult<KnowledgeRichDocument> {
        if title.trim() != title || title.is_empty() {
            return Err(StorageError::Validation(
                "knowledge rich document title must be non-empty and trimmed",
            ));
        }
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        let current = read_live_rich_document(self.storage(), rich_document_id)
            .await?
            .ok_or(StorageError::NotFound("knowledge rich document"))?;
        if let Some(expected) = expected_updated_at {
            if current.updated_at != expected {
                return Err(StorageError::Conflict(
                    "knowledge_rich_document_stale_updated_at",
                ));
            }
        }
        let (_, search_text) = loom_projection_inputs(title, &current.content_json)?;
        // Statements: BEGIN(0) guarded-rename(1) loom-title(2) search(3)
        // final-select(4) COMMIT.
        let statement = format!(
            "BEGIN TRANSACTION;\n\
             IF array::len((UPDATE knowledge_rich_documents SET title = $doc_title, updated_at = time::now() WHERE rich_document_id = $doc_id AND deleted_at = NONE AND ($expected_updated_at = NONE OR updated_at = $expected_updated_at) RETURN AFTER)) != 1 {{ THROW 'HSK-KRD-RENAME-STALE'; }};\n\
             IF array::len((UPDATE loom_blocks SET title = $doc_title, updated_at = time::now() WHERE block_id = $doc_id AND workspace_id = $workspace AND content_type = 'note' RETURN AFTER)) != 1 {{ THROW 'HSK-KRD-RENAME-LOOM-MISSING'; }};\n\
             {SEARCH_PROJECTION_STATEMENT}\n\
             SELECT * FROM knowledge_rich_documents WHERE rich_document_id = $doc_id;\n\
             COMMIT TRANSACTION;"
        );
        let result: Result<Vec<RichDocRecord>, SurrealStorageError> = raw_rows_at(
            self.storage(),
            statement,
            vec![
                b("doc_id", rich_document_id.to_owned()),
                b("doc_title", title.to_owned()),
                b(
                    "expected_updated_at",
                    expected_updated_at.map(Datetime::from),
                ),
                b("workspace", thing(WORKSPACES_TABLE, &current.workspace_id)),
                b("doc_block_id", rich_document_id.to_owned()),
                b("doc_search_text", search_text),
            ],
            4,
        )
        .await;
        let rows = result.map_err(|error| {
            map_guarded_err(
                error,
                &[
                    ("HSK-KRD-RENAME-STALE", || {
                        StorageError::Conflict("knowledge_rich_document_stale_updated_at")
                    }),
                    ("HSK-KRD-RENAME-LOOM-MISSING", || {
                        StorageError::Conflict(
                            "rich document LoomBlock projection missing during rename",
                        )
                    }),
                    LOOM_IDENTITY_GUARDS[1],
                ],
            )
        })?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge rich document"))
            .and_then(rich_document_to_domain)
    }

    async fn move_knowledge_rich_document(
        &self,
        rich_document_id: &str,
        project_ref: Option<&str>,
        folder_ref: Option<&str>,
    ) -> StorageResult<KnowledgeRichDocument> {
        let rows: Vec<RichDocRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_rich_documents SET project_ref = $project_ref, folder_ref = $folder_ref, updated_at = time::now() WHERE rich_document_id = $doc_id AND deleted_at = NONE RETURN AFTER;",
            vec![
                b("doc_id", rich_document_id.to_owned()),
                b("project_ref", project_ref.map(str::to_owned)),
                b("folder_ref", folder_ref.map(str::to_owned)),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge rich document"))
            .and_then(rich_document_to_domain)
    }

    async fn set_knowledge_rich_document_authority_label(
        &self,
        rich_document_id: &str,
        authority_label: &str,
    ) -> StorageResult<KnowledgeRichDocument> {
        if !matches!(authority_label, "draft" | "promoted" | "archived") {
            return Err(StorageError::Validation(
                "knowledge rich document authority_label must be draft|promoted|archived",
            ));
        }
        let rows: Vec<RichDocRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_rich_documents SET authority_label = $authority_label, updated_at = time::now() WHERE rich_document_id = $doc_id AND deleted_at = NONE RETURN AFTER;",
            vec![
                b("doc_id", rich_document_id.to_owned()),
                b("authority_label", authority_label.to_owned()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge rich document"))
            .and_then(rich_document_to_domain)
    }

    async fn list_knowledge_rich_documents(
        &self,
        workspace_id: &str,
        project_ref: Option<&str>,
        folder_ref: Option<&str>,
    ) -> StorageResult<Vec<KnowledgeRichDocument>> {
        let rows: Vec<RichDocRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_rich_documents WHERE workspace_id = $workspace AND deleted_at = NONE AND ($project_ref = NONE OR project_ref = $project_ref) AND ($folder_ref = NONE OR folder_ref = $folder_ref) ORDER BY updated_at DESC, rich_document_id ASC;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("project_ref", project_ref.map(str::to_owned)),
                b("folder_ref", folder_ref.map(str::to_owned)),
            ],
        )
        .await?;
        rows.into_iter().map(rich_document_to_domain).collect()
    }

    async fn upsert_knowledge_editor_code_node(
        &self,
        upsert: UpsertEditorCodeNode,
    ) -> StorageResult<KnowledgeEditorCodeNode> {
        if upsert.node_path.trim() != upsert.node_path || upsert.node_path.is_empty() {
            return Err(StorageError::Validation(
                "knowledge editor code node node_path must be non-empty and trimmed",
            ));
        }
        if upsert.language_id.trim() != upsert.language_id || upsert.language_id.is_empty() {
            return Err(StorageError::Validation(
                "knowledge editor code node language_id must be non-empty and trimmed",
            ));
        }
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let code_node_id = new_knowledge_id("KCN");
        let round_trip_sha256 =
            crate::kernel::context_bundle::sha256_hex(upsert.code_text.as_bytes());
        let rows: Vec<CodeNodeRecord> = query_rows(
            self.storage(),
            "IF (SELECT VALUE id FROM knowledge_editor_code_nodes WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) AND node_path = $node_path LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_editor_code_nodes SET language_id = $language_id, code_text = $code_text, round_trip_sha256 = $round_trip_sha256, worker_requirements = $worker_requirements, source_mapping = $source_mapping, lint_diagnostics = $lint_diagnostics, updated_at = time::now() WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) AND node_path = $node_path RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_editor_code_nodes', $code_node_id) CONTENT { code_node_id: $code_node_id, rich_document_id: type::record('knowledge_rich_documents', $doc_id), node_path: $node_path, language_id: $language_id, code_text: $code_text, round_trip_sha256: $round_trip_sha256, worker_requirements: $worker_requirements, source_mapping: $source_mapping, lint_diagnostics: $lint_diagnostics } RETURN AFTER; };",
            vec![
                b("code_node_id", code_node_id),
                b("doc_id", upsert.rich_document_id.clone()),
                b("node_path", upsert.node_path.clone()),
                b("language_id", upsert.language_id.clone()),
                b("code_text", upsert.code_text.clone()),
                b("round_trip_sha256", round_trip_sha256),
                b("worker_requirements", upsert.worker_requirements.clone()),
                b("source_mapping", upsert.source_mapping.clone()),
                b("lint_diagnostics", upsert.lint_diagnostics.clone()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge editor code node upsert returned no record".to_owned(),
            ))
            .and_then(code_node_to_domain)
    }

    async fn list_knowledge_editor_code_nodes(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeEditorCodeNode>> {
        let rows: Vec<CodeNodeRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_editor_code_nodes WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) ORDER BY node_path ASC;",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(code_node_to_domain).collect()
    }

    async fn upsert_knowledge_document_embed(
        &self,
        upsert: UpsertKnowledgeDocumentEmbed,
    ) -> StorageResult<KnowledgeDocumentEmbed> {
        if upsert.block_id.trim() != upsert.block_id || upsert.block_id.is_empty() {
            return Err(StorageError::Validation(
                "knowledge document embed block_id must be non-empty and trimmed",
            ));
        }
        if !matches!(
            upsert.ref_kind.as_str(),
            "artifact" | "media" | "source" | "url"
        ) {
            return Err(StorageError::Validation(
                "knowledge document embed ref_kind must be artifact|media|source|url",
            ));
        }
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let embed_id = new_knowledge_id("KEMB");
        // An upsert re-points the embed; resolution is fresh, so the update
        // branch resets the repair state to ok (MT-153 repair through relink).
        let rows: Vec<EmbedRecord> = query_rows(
            self.storage(),
            "IF (SELECT VALUE id FROM knowledge_document_embeds WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) AND block_id = $block_id LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_document_embeds SET ref_kind = $ref_kind, ref_value = $ref_value, caption = $caption, repair_state = 'ok', repair_reason = NONE, updated_at = time::now() WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) AND block_id = $block_id RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_document_embeds', $embed_id) CONTENT { embed_id: $embed_id, rich_document_id: type::record('knowledge_rich_documents', $doc_id), block_id: $block_id, ref_kind: $ref_kind, ref_value: $ref_value, caption: $caption } RETURN AFTER; };",
            vec![
                b("embed_id", embed_id),
                b("doc_id", upsert.rich_document_id.clone()),
                b("block_id", upsert.block_id.clone()),
                b("ref_kind", upsert.ref_kind.clone()),
                b("ref_value", upsert.ref_value.clone()),
                b("caption", upsert.caption.clone()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge document embed upsert returned no record".to_owned(),
            ))
            .and_then(embed_to_domain)
    }

    async fn list_knowledge_document_embeds(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>> {
        let rows: Vec<EmbedRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_document_embeds WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) ORDER BY block_id ASC;",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(embed_to_domain).collect()
    }

    async fn set_knowledge_document_embed_repair_state(
        &self,
        embed_id: &str,
        broken_reason: Option<&str>,
    ) -> StorageResult<KnowledgeDocumentEmbed> {
        let (state, reason) = match broken_reason {
            Some(reason) => ("broken", Some(reason.to_owned())),
            None => ("ok", None),
        };
        let rows: Vec<EmbedRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_document_embeds SET repair_state = $repair_state, repair_reason = $repair_reason, updated_at = time::now() WHERE embed_id = $embed_id RETURN AFTER;",
            vec![
                b("embed_id", embed_id.to_owned()),
                b("repair_state", state.to_owned()),
                b("repair_reason", reason),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge document embed"))
            .and_then(embed_to_domain)
    }

    async fn list_knowledge_document_broken_embeds(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>> {
        let rows: Vec<EmbedRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_document_embeds WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id) AND repair_state = 'broken' ORDER BY block_id ASC;",
            vec![b("doc_id", rich_document_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(embed_to_domain).collect()
    }

    async fn replace_knowledge_document_embeds(
        &self,
        rich_document_id: &str,
        upserts: Vec<UpsertKnowledgeDocumentEmbed>,
    ) -> StorageResult<Vec<KnowledgeDocumentEmbed>> {
        // Sync semantics (MT-152): the document content is the source of
        // truth, so a re-save is delete-all-for-document + insert in one
        // transaction (mirrors replace_knowledge_document_backlinks).
        for upsert in &upserts {
            if upsert.block_id.trim() != upsert.block_id || upsert.block_id.is_empty() {
                return Err(StorageError::Validation(
                    "knowledge document embed block_id must be non-empty and trimmed",
                ));
            }
            if !matches!(
                upsert.ref_kind.as_str(),
                "artifact" | "media" | "source" | "url"
            ) {
                return Err(StorageError::Validation(
                    "knowledge document embed ref_kind must be artifact|media|source|url",
                ));
            }
        }
        let embed_rows: Vec<JsonValue> = upserts
            .iter()
            .map(|upsert| {
                let mut row = serde_json::Map::new();
                row.insert("embed_id".into(), JsonValue::from(new_knowledge_id("KEMB")));
                row.insert("block_id".into(), JsonValue::from(upsert.block_id.clone()));
                row.insert("ref_kind".into(), JsonValue::from(upsert.ref_kind.clone()));
                row.insert(
                    "ref_value".into(),
                    JsonValue::from(upsert.ref_value.clone()),
                );
                if let Some(caption) = &upsert.caption {
                    row.insert("caption".into(), JsonValue::from(caption.clone()));
                }
                JsonValue::Object(row)
            })
            .collect();
        let block_order: HashMap<String, usize> = upserts
            .iter()
            .enumerate()
            .map(|(index, upsert)| (upsert.block_id.clone(), index))
            .collect();
        // Statements: BEGIN(0) delete(1) insert-loop(2) final-select(3) COMMIT.
        let rows: Vec<EmbedRecord> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             DELETE knowledge_document_embeds WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id);\n\
             FOR $row IN $embed_rows { CREATE type::record('knowledge_document_embeds', $row.embed_id) CONTENT { embed_id: $row.embed_id, rich_document_id: type::record('knowledge_rich_documents', $doc_id), block_id: $row.block_id, ref_kind: $row.ref_kind, ref_value: $row.ref_value, caption: $row.caption } RETURN NONE; };\n\
             SELECT * FROM knowledge_document_embeds WHERE rich_document_id = type::record('knowledge_rich_documents', $doc_id);\n\
             COMMIT TRANSACTION;",
            vec![
                b("doc_id", rich_document_id.to_owned()),
                b("embed_rows", JsonValue::Array(embed_rows)),
            ],
            3,
        )
        .await
        .map_err(map_err)?;
        let mut out = rows
            .into_iter()
            .map(embed_to_domain)
            .collect::<StorageResult<Vec<_>>>()?;
        // Return rows in upsert order, as the removed backend did.
        out.sort_by_key(|embed| {
            block_order
                .get(&embed.block_id)
                .copied()
                .unwrap_or(usize::MAX)
        });
        Ok(out)
    }

    async fn upsert_knowledge_document_backlink(
        &self,
        upsert: UpsertKnowledgeDocumentBacklink,
    ) -> StorageResult<KnowledgeDocumentBacklink> {
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let backlink_id = new_knowledge_id("KDBL");
        let rows: Vec<BacklinkRecord> = query_rows(
            self.storage(),
            "IF (SELECT VALUE id FROM knowledge_document_backlinks WHERE workspace_id = $workspace AND relationship_id = $relationship_id LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_document_backlinks SET source_document_id = type::record('knowledge_rich_documents', $source_key), link_kind = $link_kind, target = $target, block_id = $block_id, updated_at = time::now() WHERE workspace_id = $workspace AND relationship_id = $relationship_id RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_document_backlinks', $backlink_id) CONTENT { backlink_id: $backlink_id, workspace_id: $workspace, relationship_id: $relationship_id, source_document_id: type::record('knowledge_rich_documents', $source_key), link_kind: $link_kind, target: $target, block_id: $block_id } RETURN AFTER; };",
            vec![
                b("backlink_id", backlink_id),
                b("workspace", thing(WORKSPACES_TABLE, &upsert.workspace_id)),
                b("relationship_id", upsert.relationship_id.clone()),
                b("source_key", upsert.source_document_id.clone()),
                b("link_kind", upsert.link_kind.clone()),
                b("target", upsert.target.clone()),
                b("block_id", upsert.block_id.clone()),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge document backlink upsert returned no record".to_owned(),
            ))
            .and_then(backlink_to_domain)
    }

    async fn replace_knowledge_document_backlinks(
        &self,
        source_document_id: &str,
        upserts: Vec<UpsertKnowledgeDocumentBacklink>,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
        let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
        let source = read_live_rich_document(self.storage(), source_document_id)
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
        let (prior_by_relationship, prior_loom_targets) =
            read_prior_backlink_state(self.storage(), &source.workspace_id, source_document_id)
                .await?;
        let resolved = resolve_backlink_rows(
            self.storage(),
            &source.workspace_id,
            source_document_id,
            upserts,
            &prior_by_relationship,
            &prior_loom_targets,
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
        let rows: Vec<BacklinkRecord> = raw_rows_at(
            self.storage(),
            statement,
            binds,
            1 + BACKLINK_WRITE_STATEMENT_COUNT,
        )
        .await
        .map_err(|error| map_guarded_err(error, &[BACKLINK_GUARDS[0]]))?;
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

    async fn list_knowledge_document_backlinks_from(
        &self,
        source_document_id: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
        let rows: Vec<BacklinkRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_document_backlinks WHERE source_document_id = type::record('knowledge_rich_documents', $source_key) ORDER BY link_kind ASC, target ASC, block_id ASC;",
            vec![b("source_key", source_document_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(backlink_to_domain).collect()
    }

    async fn list_knowledge_document_backlinks_to(
        &self,
        workspace_id: &str,
        link_kind: &str,
        target: &str,
    ) -> StorageResult<Vec<KnowledgeDocumentBacklink>> {
        let rows: Vec<BacklinkRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_document_backlinks WHERE workspace_id = $workspace AND link_kind = $link_kind AND target = $target ORDER BY source_document_id ASC, block_id ASC;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("link_kind", link_kind.to_owned()),
                b("target", target.to_owned()),
            ],
        )
        .await?;
        rows.into_iter().map(backlink_to_domain).collect()
    }

    async fn record_knowledge_context_bundle(
        &self,
        new_bundle: NewKnowledgeContextBundle,
    ) -> StorageResult<KnowledgeContextBundle> {
        for item in &new_bundle.items {
            if item.ref_id.trim() != item.ref_id || item.ref_id.is_empty() {
                return Err(StorageError::Validation(
                    "knowledge bundle item ref_id must be non-empty and trimmed",
                ));
            }
            let citation_marks_unsupported = item
                .citation
                .as_deref()
                .is_some_and(|citation| citation.ends_with("@UNSUPPORTED"));
            if item.supported && (item.unsupported_reason.is_some() || citation_marks_unsupported) {
                return Err(StorageError::Validation(
                    "supported knowledge bundle items must not carry unsupported markers",
                ));
            }
            if !item.supported {
                let reason = item.unsupported_reason.as_deref().unwrap_or("");
                if reason.trim() != reason || reason.is_empty() {
                    return Err(StorageError::Validation(
                        "unsupported knowledge bundle items must carry a trimmed unsupported_reason",
                    ));
                }
                if item
                    .citation
                    .as_deref()
                    .is_some_and(|citation| !citation.ends_with("@UNSUPPORTED"))
                {
                    return Err(StorageError::Validation(
                        "unsupported knowledge bundle item citations must carry @UNSUPPORTED",
                    ));
                }
            }
        }
        let bundle = &new_bundle.bundle;
        let item_rows: Vec<JsonValue> = new_bundle
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut row = serde_json::Map::new();
                row.insert("item_ordinal".into(), JsonValue::from(index as i64));
                row.insert(
                    "ref_kind".into(),
                    JsonValue::from(item.ref_kind.as_str().to_owned()),
                );
                row.insert("ref_id".into(), JsonValue::from(item.ref_id.clone()));
                row.insert(
                    "retrieval_decision".into(),
                    JsonValue::from(item.retrieval_decision.as_str().to_owned()),
                );
                if let Some(score) = item.relevance_score {
                    row.insert("relevance_score".into(), JsonValue::from(score));
                }
                if let Some(count) = item.token_count {
                    row.insert("token_count".into(), JsonValue::from(i64::from(count)));
                }
                if let Some(citation) = &item.citation {
                    row.insert("citation".into(), JsonValue::from(citation.clone()));
                }
                row.insert("supported".into(), JsonValue::from(item.supported));
                if let Some(reason) = &item.unsupported_reason {
                    row.insert("unsupported_reason".into(), JsonValue::from(reason.clone()));
                }
                JsonValue::Object(row)
            })
            .collect();
        // Statements: BEGIN(0) create-bundle(1) item-loop(2) COMMIT.
        let rows: Vec<BundleRecord> = raw_rows_at(
            self.storage(),
            "BEGIN TRANSACTION;\n\
             CREATE type::record('knowledge_context_bundles', $bundle_id) CONTENT { bundle_id: $bundle_id, workspace_id: $workspace, kernel_task_run_id: $kernel_task_run_id, session_run_id: $session_run_id, allowed_context: $allowed_context, context_hash: $context_hash, query_text: $query_text, token_budget: $token_budget, tokens_used: $tokens_used, build_receipt_event_id: $build_receipt } RETURN AFTER;\n\
             FOR $item IN $item_rows { CREATE knowledge_context_bundle_items CONTENT { bundle_id: type::record('knowledge_context_bundles', $bundle_id), item_ordinal: $item.item_ordinal, ref_kind: $item.ref_kind, ref_id: $item.ref_id, retrieval_decision: $item.retrieval_decision, relevance_score: $item.relevance_score, token_count: $item.token_count, citation: $item.citation, supported: $item.supported, unsupported_reason: $item.unsupported_reason } RETURN NONE; };\n\
             COMMIT TRANSACTION;",
            vec![
                b("bundle_id", bundle.context_bundle_id.clone()),
                b("workspace", thing(WORKSPACES_TABLE, &new_bundle.workspace_id)),
                b("kernel_task_run_id", bundle.kernel_task_run_id.clone()),
                b("session_run_id", bundle.session_run_id.clone()),
                b("allowed_context", bundle.allowed_context.clone()),
                b("context_hash", bundle.context_hash.clone()),
                b("query_text", new_bundle.query_text.clone()),
                b("token_budget", new_bundle.token_budget.map(i64::from)),
                b("tokens_used", new_bundle.tokens_used.map(i64::from)),
                b(
                    "build_receipt",
                    opt_thing(
                        KERNEL_EVENT_LEDGER_TABLE,
                        new_bundle.build_receipt_event_id.as_deref(),
                    ),
                ),
                b("item_rows", JsonValue::Array(item_rows)),
            ],
            1,
        )
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge context bundle CREATE returned no record".to_owned(),
            ))
            .and_then(bundle_to_domain)
    }

    async fn get_knowledge_context_bundle(
        &self,
        bundle_id: &str,
    ) -> StorageResult<Option<(KnowledgeContextBundle, Vec<KnowledgeContextBundleItem>)>> {
        let bundle: Option<BundleRecord> = query_first_row(
            self.storage(),
            "SELECT * FROM knowledge_context_bundles WHERE bundle_id = $bundle_id;",
            vec![b("bundle_id", bundle_id.to_owned())],
        )
        .await?;
        let Some(bundle) = bundle else {
            return Ok(None);
        };
        let bundle = bundle_to_domain(bundle)?;
        let item_rows: Vec<BundleItemRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_context_bundle_items WHERE bundle_id = type::record('knowledge_context_bundles', $bundle_id) ORDER BY item_ordinal ASC;",
            vec![b("bundle_id", bundle_id.to_owned())],
        )
        .await?;
        let items = item_rows
            .into_iter()
            .map(bundle_item_to_domain)
            .collect::<StorageResult<Vec<_>>>()?;
        Ok(Some((bundle, items)))
    }

    async fn record_knowledge_retrieval_trace(
        &self,
        new_trace: crate::storage::knowledge::NewKnowledgeRetrievalTrace,
    ) -> StorageResult<KnowledgeRetrievalTrace> {
        if new_trace.mode_reason.trim() != new_trace.mode_reason || new_trace.mode_reason.is_empty()
        {
            return Err(StorageError::Validation(
                "knowledge retrieval trace mode_reason is a spec MUST: record why broader retrieval was used or skipped",
            ));
        }
        let trace_id = new_knowledge_id("KRT");
        let rows: Vec<TraceRecord> = query_rows(
            self.storage(),
            "CREATE type::record('knowledge_retrieval_traces', $trace_id) CONTENT { trace_id: $trace_id, workspace_id: $workspace, retrieval_mode: $retrieval_mode, mode_reason: $mode_reason, query_text: $query_text, bundle_id: $bundle_id, decisions: $decisions, trace_receipt_event_id: $trace_receipt } RETURN AFTER;",
            vec![
                b("trace_id", trace_id),
                b("workspace", thing(WORKSPACES_TABLE, &new_trace.workspace_id)),
                b(
                    "retrieval_mode",
                    new_trace.retrieval_mode.as_str().to_owned(),
                ),
                b("mode_reason", new_trace.mode_reason.clone()),
                b("query_text", new_trace.query_text.clone()),
                b(
                    "bundle_id",
                    opt_thing(
                        KNOWLEDGE_CONTEXT_BUNDLES_TABLE,
                        new_trace.bundle_id.as_deref(),
                    ),
                ),
                b("decisions", new_trace.decisions.clone()),
                b(
                    "trace_receipt",
                    opt_thing(
                        KERNEL_EVENT_LEDGER_TABLE,
                        new_trace.trace_receipt_event_id.as_deref(),
                    ),
                ),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge retrieval trace CREATE returned no record".to_owned(),
            ))
            .and_then(trace_to_domain)
    }

    async fn list_knowledge_retrieval_traces_for_bundle(
        &self,
        bundle_id: &str,
    ) -> StorageResult<Vec<KnowledgeRetrievalTrace>> {
        let rows: Vec<TraceRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_retrieval_traces WHERE bundle_id = type::record('knowledge_context_bundles', $bundle_id) ORDER BY created_at ASC;",
            vec![b("bundle_id", bundle_id.to_owned())],
        )
        .await?;
        rows.into_iter().map(trace_to_domain).collect()
    }

    async fn create_knowledge_memory_passage_idempotent(
        &self,
        idempotency_key: &str,
        new_passage: NewKnowledgeMemoryPassage,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeMemoryPassage>> {
        validate_knowledge_idempotency_key(idempotency_key)?;
        validate_new_passage(&new_passage)?;
        let request_hash = knowledge_request_hash("passage_write", &new_passage)?;

        // Committed replay: return the prior result without writing.
        if let Some((_, passage_id)) =
            find_idempotency_result(self.storage(), idempotency_key, &request_hash).await?
        {
            let passage = self
                .get_knowledge_memory_passage(&passage_id)
                .await?
                .ok_or(StorageError::NotFound(
                    "knowledge idempotency result passage",
                ))?;
            return Ok(KnowledgeIdempotentWrite {
                value: passage,
                replayed: true,
            });
        }

        // Write + key claim in ONE transaction; a lost key race aborts the
        // whole transaction (no double-write).
        // Statements: BEGIN(0) create(1) lineage-loop(2) key-claim(3) COMMIT.
        let passage_id = new_knowledge_id("KMP");
        let statement = format!(
            "BEGIN TRANSACTION;\n{PASSAGE_INSERT_STATEMENTS}\n{IDEMPOTENCY_CLAIM_STATEMENT}\nCOMMIT TRANSACTION;"
        );
        let mut binds = passage_insert_binds(&passage_id, &new_passage);
        binds.extend(idempotency_claim_binds(
            idempotency_key,
            &new_passage.workspace_id,
            "passage_write",
            &request_hash,
            "memory_passage",
            &passage_id,
        ));
        let binds = dedup_binds(binds);
        let result: Result<Vec<PassageRecord>, SurrealStorageError> =
            raw_rows_at(self.storage(), statement, binds, 1).await;
        match result {
            Ok(rows) => rows
                .into_iter()
                .next()
                .ok_or(StorageError::Database(
                    "knowledge memory passage CREATE returned no record".to_owned(),
                ))
                .and_then(passage_to_domain)
                .map(|passage| KnowledgeIdempotentWrite {
                    value: passage,
                    replayed: false,
                }),
            Err(error) => {
                // SurrealDB can collapse a THROW inside a failed transaction
                // into a generic transaction error. Re-read the authoritative
                // idempotency row instead of depending on rendered error text.
                let Some((_, passage_id)) =
                    find_idempotency_result(self.storage(), idempotency_key, &request_hash).await?
                else {
                    return Err(map_err(error));
                };
                let passage = self
                    .get_knowledge_memory_passage(&passage_id)
                    .await?
                    .ok_or(StorageError::NotFound(
                        "knowledge idempotency result passage",
                    ))?;
                Ok(KnowledgeIdempotentWrite {
                    value: passage,
                    replayed: true,
                })
            }
        }
    }

    async fn save_knowledge_rich_document_version_idempotent(
        &self,
        idempotency_key: &str,
        rich_document_id: &str,
        expected_version: i64,
        content_json: JsonValue,
        crdt_document_id: Option<&str>,
        crdt_snapshot_id: Option<&str>,
        promotion_receipt_event_id: Option<&str>,
    ) -> StorageResult<KnowledgeIdempotentWrite<KnowledgeRichDocument>> {
        validate_knowledge_idempotency_key(idempotency_key)?;
        let next_version = checked_next_rich_document_version(expected_version)?;
        let request_hash = knowledge_request_hash(
            "rich_document_save",
            &serde_json::json!({
                "rich_document_id": rich_document_id,
                "expected_version": expected_version,
                "content_json": content_json,
                "crdt_document_id": crdt_document_id,
                "crdt_snapshot_id": crdt_snapshot_id,
                "promotion_receipt_event_id": promotion_receipt_event_id,
            }),
        )?;

        // Committed replay: return the prior result without writing.
        if let Some((result_ref_kind, result_ref_id)) =
            find_idempotency_result(self.storage(), idempotency_key, &request_hash).await?
        {
            let replayed = self
                .replay_rich_document_save(&result_ref_kind, &result_ref_id)
                .await?;
            return Ok(KnowledgeIdempotentWrite {
                value: replayed,
                replayed: true,
            });
        }

        let claim = IdempotentSaveClaim {
            idempotency_key: idempotency_key.to_owned(),
            workspace_key: String::new(),
            request_hash: request_hash.clone(),
            result_ref_id: rich_document_version_result_ref_id(rich_document_id, next_version),
        };
        let save_result = {
            let _serialize = RICH_DOCUMENT_MUTATION_LOCK.lock().await;
            // The claim row needs the document's workspace; read it under the
            // lock so the id cannot change between the read and the write.
            let current = read_live_rich_document(self.storage(), rich_document_id)
                .await?
                .ok_or(StorageError::NotFound("knowledge rich document"))?;
            let claim = IdempotentSaveClaim {
                workspace_key: current.workspace_id.clone(),
                ..claim
            };
            save_rich_document_version_locked(
                self.storage(),
                rich_document_id,
                expected_version,
                next_version,
                &content_json,
                crdt_document_id,
                crdt_snapshot_id,
                promotion_receipt_event_id,
                Some(claim),
            )
            .await
        };
        match save_result {
            Ok(Some(document)) => Ok(KnowledgeIdempotentWrite {
                value: document,
                replayed: false,
            }),
            // Race lost on the key claim: the write rolled back; re-read the
            // winner's committed result.
            Ok(None) => {
                let (result_ref_kind, result_ref_id) =
                    find_idempotency_result(self.storage(), idempotency_key, &request_hash)
                        .await?
                        .ok_or(StorageError::Conflict(
                            "knowledge idempotency race lost without a committed winner row",
                        ))?;
                let replayed = self
                    .replay_rich_document_save(&result_ref_kind, &result_ref_id)
                    .await?;
                Ok(KnowledgeIdempotentWrite {
                    value: replayed,
                    replayed: true,
                })
            }
            // A replayed save can lose either the key claim or optimistic
            // version race. SurrealDB may surface a transaction THROW only as
            // a generic failure, so the committed key is the authoritative
            // replay signal for every error class.
            Err(error) => {
                if let Some((result_ref_kind, result_ref_id)) =
                    find_idempotency_result(self.storage(), idempotency_key, &request_hash).await?
                {
                    let replayed = self
                        .replay_rich_document_save(&result_ref_kind, &result_ref_id)
                        .await?;
                    return Ok(KnowledgeIdempotentWrite {
                        value: replayed,
                        replayed: true,
                    });
                }
                Err(error)
            }
        }
    }

    async fn get_knowledge_code_file_by_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Option<KnowledgeCodeFile>> {
        SurrealDatabase::get_knowledge_code_file_by_source(self, source_id).await
    }
}

const WIKI_PAGE_TYPES: [&str; 6] = ["module", "concept", "flow", "entity", "decision", "index"];

fn validate_wiki_page(page: &NewKnowledgeWikiPage) -> StorageResult<()> {
    if page.title.trim() != page.title || page.title.is_empty() {
        return Err(StorageError::Validation(
            "wiki page title must be non-empty and trimmed",
        ));
    }
    if !is_sha256_hex(&page.staleness_hash) {
        return Err(StorageError::Validation(
            "wiki page staleness_hash must be lowercase sha256 hex",
        ));
    }
    if page
        .page_type
        .as_deref()
        .is_some_and(|page_type| !WIKI_PAGE_TYPES.contains(&page_type))
    {
        return Err(StorageError::Validation("invalid wiki page_type"));
    }
    let stamp_ok = page
        .compile_stamp
        .get("ledger_version")
        .is_some_and(|value| value.is_i64() || value.is_u64())
        && page
            .compile_stamp
            .get("cited_sources")
            .is_some_and(JsonValue::is_array);
    if !stamp_ok {
        return Err(StorageError::Validation(
            "wiki page compile_stamp must carry ledger_version + cited_sources (LM-PWIKI-006)",
        ));
    }
    Ok(())
}

impl SurrealDatabase {
    /// Upsert a stamped compiled page on the stable workspace/kind/title
    /// identity. The page becomes visible with its stamp, links and receipt in
    /// the same statement.
    pub async fn upsert_knowledge_wiki_page(
        &self,
        page: NewKnowledgeWikiPage,
    ) -> StorageResult<KnowledgeWikiProjection> {
        validate_wiki_page(&page)?;
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let projection_id = new_knowledge_id("KWP");
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            "IF (SELECT VALUE id FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND title = $title LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_wiki_projections SET source_records = $source_records, rendered_content = $rendered_content, rebuild_status = 'fresh', staleness_hash = $staleness_hash, rebuild_receipt_event_id = $receipt, last_rebuilt_at = time::now(), page_type = $page_type, compile_stamp = $compile_stamp, compile_recipe = $compile_recipe, page_links = $page_links, updated_at = time::now() WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND title = $title RETURN AFTER; } ELSE { RETURN CREATE type::record('knowledge_wiki_projections', $projection_id) CONTENT { projection_id: $projection_id, workspace_id: $workspace, projection_kind: 'wiki_page', title: $title, source_records: $source_records, rendered_content: $rendered_content, rebuild_status: 'fresh', staleness_hash: $staleness_hash, rebuild_receipt_event_id: $receipt, last_rebuilt_at: time::now(), page_type: $page_type, compile_stamp: $compile_stamp, compile_recipe: $compile_recipe, page_links: $page_links } RETURN AFTER; };",
            vec![
                b("projection_id", projection_id),
                b("workspace", thing(WORKSPACES_TABLE, &page.workspace_id)),
                b("title", page.title),
                b("source_records", page.source_records),
                b("rendered_content", page.rendered_content),
                b("staleness_hash", page.staleness_hash),
                b(
                    "receipt",
                    opt_thing(
                        KERNEL_EVENT_LEDGER_TABLE,
                        page.rebuild_receipt_event_id.as_deref(),
                    ),
                ),
                b("page_type", page.page_type),
                b("compile_stamp", page.compile_stamp),
                b("compile_recipe", page.compile_recipe),
                b("page_links", page.page_links),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::Database(
                "knowledge wiki page upsert returned no record".to_owned(),
            ))
            .and_then(projection_to_domain)
    }

    /// Replace one page by stable projection id, retaining fan-out identity
    /// even when authority changes the rendered title.
    pub async fn replace_knowledge_wiki_page_by_projection_id(
        &self,
        projection_id: &str,
        page: NewKnowledgeWikiPage,
    ) -> StorageResult<KnowledgeWikiProjection> {
        validate_wiki_page(&page)?;
        let _serialize = KNOWLEDGE_UPSERT_LOCK.lock().await;
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            "UPDATE knowledge_wiki_projections SET title = $title, source_records = $source_records, rendered_content = $rendered_content, rebuild_status = 'fresh', staleness_hash = $staleness_hash, rebuild_receipt_event_id = $receipt, last_rebuilt_at = time::now(), page_type = $page_type, compile_stamp = $compile_stamp, compile_recipe = $compile_recipe, page_links = $page_links, updated_at = time::now() WHERE projection_id = $projection_id AND workspace_id = $workspace AND projection_kind = 'wiki_page' RETURN AFTER;",
            vec![
                b("projection_id", projection_id.to_owned()),
                b("workspace", thing(WORKSPACES_TABLE, &page.workspace_id)),
                b("title", page.title),
                b("source_records", page.source_records),
                b("rendered_content", page.rendered_content),
                b("staleness_hash", page.staleness_hash),
                b(
                    "receipt",
                    opt_thing(
                        KERNEL_EVENT_LEDGER_TABLE,
                        page.rebuild_receipt_event_id.as_deref(),
                    ),
                ),
                b("page_type", page.page_type),
                b("compile_stamp", page.compile_stamp),
                b("compile_recipe", page.compile_recipe),
                b("page_links", page.page_links),
            ],
        )
        .await?;
        rows.into_iter()
            .next()
            .ok_or(StorageError::NotFound("knowledge wiki projection"))
            .and_then(projection_to_domain)
    }

    pub async fn list_knowledge_wiki_pages(
        &self,
        workspace_id: &str,
        page_type: Option<&str>,
        typed_only: bool,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<KnowledgeWikiProjection>> {
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND ($page_type = NONE OR page_type = $page_type) AND (!$typed_only OR page_type != NONE);",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("page_type", page_type.map(str::to_owned)),
                b("typed_only", typed_only),
            ],
        )
        .await?;
        let mut pages = rows
            .into_iter()
            .map(projection_to_domain)
            .collect::<StorageResult<Vec<_>>>()?;
        pages.sort_by(|left, right| {
            left.page_type
                .as_deref()
                .unwrap_or("zz")
                .cmp(right.page_type.as_deref().unwrap_or("zz"))
                .then_with(|| left.title.cmp(&right.title))
        });
        let offset = usize::try_from(offset.max(0)).unwrap_or(usize::MAX);
        Ok(pages
            .into_iter()
            .skip(offset)
            .take(limit.clamp(1, 2_000) as usize)
            .collect())
    }

    pub async fn list_knowledge_wiki_pages_citing(
        &self,
        workspace_id: &str,
        cited_kind: &str,
        cited_id: &str,
    ) -> StorageResult<Vec<KnowledgeWikiProjection>> {
        self.list_wiki_pages_matching_citation(workspace_id, cited_kind, "id", cited_id)
            .await
    }

    pub async fn list_knowledge_wiki_pages_citing_entity_source(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<Vec<KnowledgeWikiProjection>> {
        self.list_wiki_pages_matching_citation(workspace_id, "entity", "source_id", source_id)
            .await
    }

    async fn list_wiki_pages_matching_citation(
        &self,
        workspace_id: &str,
        cited_kind: &str,
        id_field: &'static str,
        cited_id: &str,
    ) -> StorageResult<Vec<KnowledgeWikiProjection>> {
        let statement = if id_field == "source_id" {
            "SELECT * FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND compile_stamp != NONE AND array::len(compile_stamp.cited_sources[WHERE $this.kind = $kind AND $this.source_id = $cited_id]) > 0 ORDER BY title ASC;"
        } else {
            "SELECT * FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND compile_stamp != NONE AND array::len(compile_stamp.cited_sources[WHERE $this.kind = $kind AND $this.id = $cited_id]) > 0 ORDER BY title ASC;"
        };
        let rows: Vec<ProjectionRecord> = query_rows(
            self.storage(),
            statement,
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("kind", cited_kind.to_owned()),
                b("cited_id", cited_id.to_owned()),
            ],
        )
        .await?;
        rows.into_iter().map(projection_to_domain).collect()
    }

    pub async fn update_knowledge_wiki_page_links(
        &self,
        projection_id: &str,
        page_links: &JsonValue,
    ) -> StorageResult<()> {
        let rows: Vec<SurrealValueData> = query_rows(
            self.storage(),
            "UPDATE knowledge_wiki_projections SET page_links = $page_links, updated_at = time::now() WHERE projection_id = $projection_id RETURN AFTER;",
            vec![
                b("projection_id", projection_id.to_owned()),
                b("page_links", page_links.clone()),
            ],
        )
        .await?;
        if rows.is_empty() {
            return Err(StorageError::NotFound("knowledge wiki projection"));
        }
        Ok(())
    }

    pub async fn current_event_ledger_version(&self) -> StorageResult<i64> {
        Ok(query_first_row::<WikiLedgerVersionRecord>(
            self.storage(),
            "SELECT event_sequence FROM kernel_event_ledger ORDER BY event_sequence DESC LIMIT 1;",
            Vec::new(),
        )
        .await?
        .map_or(0, |row| row.event_sequence))
    }

    /// Return the complete code-index state for one source, if it has been
    /// indexed. Source identity is unique in the Surreal schema, matching the
    /// retired relational seam this canonical SurrealDB method replaces.
    pub async fn get_knowledge_code_file_by_source(
        &self,
        source_id: &str,
    ) -> StorageResult<Option<KnowledgeCodeFile>> {
        query_first_row::<KnowledgeCodeFileRecord>(
            self.storage(),
            "SELECT * FROM knowledge_code_files WHERE source_id = $source LIMIT 1;",
            vec![b("source", thing(KNOWLEDGE_SOURCES_TABLE, source_id))],
        )
        .await?
        .map(knowledge_code_file_to_domain)
        .transpose()
    }

    pub async fn list_knowledge_code_files(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<KnowledgeCodeFile>> {
        let rows: Vec<KnowledgeCodeFileRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_code_files WHERE workspace_id = $workspace ORDER BY source_id ASC;",
            vec![b("workspace", thing(WORKSPACES_TABLE, workspace_id))],
        )
        .await?;
        rows.into_iter()
            .map(knowledge_code_file_to_domain)
            .collect()
    }

    /// Mark one indexed code file stale. The update is idempotent and returns
    /// the canonical row after the SurrealDB mutation.
    pub async fn mark_knowledge_code_file_stale(
        &self,
        code_file_id: &str,
    ) -> StorageResult<KnowledgeCodeFile> {
        query_first_row::<KnowledgeCodeFileRecord>(
            self.storage(),
            "UPDATE knowledge_code_files SET stale = true, updated_at = time::now() WHERE code_file_id = $code_file_id RETURN AFTER;",
            vec![b("code_file_id", code_file_id.to_owned())],
        )
        .await?
        .ok_or(StorageError::NotFound("knowledge code file"))
        .and_then(knowledge_code_file_to_domain)
    }

    pub async fn list_wiki_code_file_inputs(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<WikiCodeFileInput>> {
        self.list_wiki_code_file_inputs_inner(workspace_id, None)
            .await
    }

    pub async fn list_wiki_code_file_inputs_by_sources(
        &self,
        workspace_id: &str,
        source_ids: &[String],
    ) -> StorageResult<Vec<WikiCodeFileInput>> {
        if source_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.list_wiki_code_file_inputs_inner(workspace_id, Some(source_ids))
            .await
    }

    async fn list_wiki_code_file_inputs_inner(
        &self,
        workspace_id: &str,
        source_ids: Option<&[String]>,
    ) -> StorageResult<Vec<WikiCodeFileInput>> {
        let rows: Vec<WikiCodeFileRecord> = query_rows(
            self.storage(),
            "SELECT code_file_id, source_id, language, parse_status, stale, symbols_indexed FROM knowledge_code_files WHERE workspace_id = $workspace AND ($source_ids = NONE OR source_id IN $source_ids);",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b(
                    "source_ids",
                    source_ids.map(|ids| {
                        ids.iter()
                            .map(|id| thing(KNOWLEDGE_SOURCES_TABLE, id))
                            .collect::<Vec<_>>()
                    }),
                ),
            ],
        )
        .await?;
        let mut inputs = Vec::with_capacity(rows.len());
        for row in rows {
            let source_id = record_key(row.source_id)?;
            let source = query_first_row::<SourceRecord>(
                self.storage(),
                "SELECT * FROM knowledge_sources WHERE source_id = $source_id LIMIT 1;",
                vec![b("source_id", source_id.clone())],
            )
            .await?
            .ok_or(StorageError::NotFound("knowledge code-file source"))?;
            let Some(relative_path) = source.relative_path else {
                continue;
            };
            inputs.push(WikiCodeFileInput {
                code_file_id: row.code_file_id,
                source_id,
                relative_path,
                content_hash: source.content_hash,
                language: row.language.parse::<KnowledgeCodeLanguage>()?,
                parse_status: row.parse_status.parse::<KnowledgeCodeParseStatus>()?,
                stale: row.stale,
                symbols_indexed: int_i32(row.symbols_indexed, "symbols_indexed")?,
            });
        }
        inputs.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(inputs)
    }

    pub async fn list_wiki_source_entities_with_spans(
        &self,
        workspace_id: &str,
        source_id: &str,
        entity_kind: KnowledgeEntityKind,
    ) -> StorageResult<Vec<WikiEntityWithSpan>> {
        let rows: Vec<EntityRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_entities WHERE workspace_id = $workspace AND primary_source_id = $source AND entity_kind = $entity_kind AND lifecycle_state = 'active';",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("source", thing(KNOWLEDGE_SOURCES_TABLE, source_id)),
                b("entity_kind", entity_kind.as_str().to_owned()),
            ],
        )
        .await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let entity_ids = rows
            .iter()
            .map(|row| thing(KNOWLEDGE_ENTITIES_TABLE, &row.entity_id))
            .collect::<Vec<_>>();
        let links: Vec<WikiEntitySpanLinkRecord> = query_rows(
            self.storage(),
            "SELECT entity_id, span_id FROM knowledge_entity_spans \
             WHERE entity_id IN $entity_ids;",
            vec![b("entity_ids", entity_ids)],
        )
        .await?;
        let mut span_ids_by_entity: HashMap<String, Vec<String>> = HashMap::new();
        let mut span_ids = Vec::with_capacity(links.len());
        for link in links {
            let entity_id = record_key(link.entity_id)?;
            let span_id = record_key(link.span_id)?;
            span_ids_by_entity
                .entry(entity_id)
                .or_default()
                .push(span_id.clone());
            span_ids.push(span_id);
        }
        let spans: Vec<WikiSpanRecord> = if span_ids.is_empty() {
            Vec::new()
        } else {
            query_rows(
                self.storage(),
                "SELECT span_id, content_sha256, line_start, line_end, section_path, created_at \
                 FROM knowledge_spans WHERE span_id IN $span_ids AND source_id = $source;",
                vec![
                    b("span_ids", span_ids),
                    b("source", thing(KNOWLEDGE_SOURCES_TABLE, source_id)),
                ],
            )
            .await?
        };
        let spans_by_id = spans
            .into_iter()
            .map(|span| (span.span_id.clone(), span))
            .collect::<HashMap<_, _>>();

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let entity_id = row.entity_id.clone();
            let latest = span_ids_by_entity.get(&entity_id).and_then(|span_ids| {
                span_ids
                    .iter()
                    .filter_map(|span_id| spans_by_id.get(span_id))
                    .max_by(|left, right| {
                        left.created_at
                            .cmp(&right.created_at)
                            .then_with(|| left.span_id.cmp(&right.span_id))
                    })
            });
            if let Some(span) = latest {
                out.push(WikiEntityWithSpan {
                    entity: entity_to_domain(row)?,
                    span_id: span.span_id.clone(),
                    span_content_sha256: span.content_sha256.clone(),
                    line_start: opt_int_i32(span.line_start, "line_start")?,
                    line_end: opt_int_i32(span.line_end, "line_end")?,
                    section_path: span.section_path.clone(),
                });
            }
        }
        out.sort_by(|left, right| left.entity.entity_key.cmp(&right.entity.entity_key));
        Ok(out)
    }

    pub async fn list_wiki_cross_source_code_edges(
        &self,
        workspace_id: &str,
        limit: i64,
    ) -> StorageResult<Vec<WikiCrossSourceEdge>> {
        let rows: Vec<WikiCrossSourceEdgeRecord> = query_rows(
            self.storage(),
            "SELECT edge_type, source_entity_id.primary_source_id AS from_source_id, target_entity_id.primary_source_id AS to_source_id FROM knowledge_edges WHERE workspace_id = $workspace AND edge_type IN ['references', 'depends_on', 'implements'] AND lifecycle_state = 'active' AND source_entity_id.primary_source_id != NONE AND target_entity_id.primary_source_id != NONE AND source_entity_id.primary_source_id != target_entity_id.primary_source_id ORDER BY source_entity_id.primary_source_id ASC, target_entity_id.primary_source_id ASC LIMIT $limit;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("limit", limit.clamp(1, 100_000)),
            ],
        )
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(WikiCrossSourceEdge {
                    edge_type: row.edge_type.parse::<KnowledgeEdgeType>()?,
                    from_source_id: record_key(row.from_source_id)?,
                    to_source_id: record_key(row.to_source_id)?,
                })
            })
            .collect()
    }

    pub async fn get_wiki_source_hashes(
        &self,
        source_ids: &[String],
    ) -> StorageResult<HashMap<String, String>> {
        if source_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<WikiSourceHashRecord> = query_rows(
            self.storage(),
            "SELECT source_id, content_hash FROM knowledge_sources WHERE source_id IN $source_ids;",
            vec![b("source_ids", source_ids.to_vec())],
        )
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.source_id, row.content_hash))
            .collect())
    }

    pub async fn get_wiki_entity_states(
        &self,
        entity_ids: &[String],
    ) -> StorageResult<Vec<(KnowledgeEntity, Option<String>)>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<EntityRecord> = query_rows(
            self.storage(),
            "SELECT * FROM knowledge_entities WHERE entity_id IN $entity_ids ORDER BY entity_id ASC;",
            vec![b("entity_ids", entity_ids.to_vec())],
        )
        .await?;
        let source_ids = rows
            .iter()
            .filter_map(|row| row.primary_source_id.clone())
            .map(record_key)
            .collect::<StorageResult<BTreeSet<_>>>()?;
        let source_hashes = if source_ids.is_empty() {
            HashMap::new()
        } else {
            query_rows::<WikiSourceHashRecord>(
                self.storage(),
                "SELECT source_id, content_hash FROM knowledge_sources \
                 WHERE source_id IN $source_ids;",
                vec![b("source_ids", source_ids.into_iter().collect::<Vec<_>>())],
            )
            .await?
            .into_iter()
            .map(|source| (source.source_id, source.content_hash))
            .collect::<HashMap<_, _>>()
        };
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let source_id = opt_record_key(row.primary_source_id.clone())?;
            let source_hash =
                source_id.and_then(|source_id| source_hashes.get(&source_id).cloned());
            out.push((entity_to_domain(row)?, source_hash));
        }
        Ok(out)
    }

    pub async fn get_wiki_loom_block_states(
        &self,
        workspace_id: &str,
        block_ids: &[String],
    ) -> StorageResult<Vec<WikiLoomBlockState>> {
        if block_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<WikiLoomBlockRecord> = query_rows(
            self.storage(),
            "SELECT block_id, title, content_type, derived_json, document_id, asset_id, content_hash FROM loom_blocks WHERE workspace_id = $workspace AND block_id IN $block_ids;",
            vec![
                b("workspace", thing(WORKSPACES_TABLE, workspace_id)),
                b("block_ids", block_ids.to_vec()),
            ],
        )
        .await?;
        rows.into_iter()
            .map(|row| {
                let full_text_index = row
                    .derived_json
                    .get("full_text_index")
                    .and_then(JsonValue::as_str)
                    .map(str::to_owned);
                Ok(WikiLoomBlockState {
                    block_id: row.block_id,
                    title: row.title,
                    content_type: row.content_type,
                    full_text_index,
                    document_id: opt_record_key(row.document_id)?,
                    asset_id: opt_record_key(row.asset_id)?,
                    content_hash: row.content_hash,
                })
            })
            .collect()
    }

    pub async fn get_wiki_rich_document_hashes(
        &self,
        rich_document_ids: &[String],
    ) -> StorageResult<HashMap<String, String>> {
        if rich_document_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<WikiRichDocumentHashRecord> = query_rows(
            self.storage(),
            "SELECT rich_document_id, content_sha256 FROM knowledge_rich_documents WHERE rich_document_id IN $document_ids;",
            vec![b("document_ids", rich_document_ids.to_vec())],
        )
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.rich_document_id, row.content_sha256))
            .collect())
    }

    /// Rebuilds the replayed rich-document view from an idempotency result
    /// ref, mirroring the removed backend's replay closure.
    async fn replay_rich_document_save(
        &self,
        result_ref_kind: &str,
        result_ref_id: &str,
    ) -> StorageResult<KnowledgeRichDocument> {
        match result_ref_kind {
            RICH_DOCUMENT_VERSION_RESULT_REF_KIND => {
                let (document_id, doc_version) =
                    parse_rich_document_version_result_ref_id(result_ref_id)?;
                let current = self
                    .get_knowledge_rich_document(&document_id)
                    .await?
                    .ok_or(StorageError::NotFound(
                        "knowledge idempotency result rich document",
                    ))?;
                let version = self
                    .get_knowledge_rich_document_version(&document_id, doc_version)
                    .await?
                    .ok_or(StorageError::NotFound(
                        "knowledge idempotency result rich document version",
                    ))?;
                Ok(KnowledgeRichDocument {
                    schema_version: version.schema_version,
                    doc_version: version.doc_version,
                    content_json: version.content_json,
                    content_sha256: version.content_sha256,
                    crdt_snapshot_id: version.crdt_snapshot_id,
                    promotion_receipt_event_id: version.promotion_receipt_event_id,
                    updated_at: version.created_at,
                    ..current
                })
            }
            RICH_DOCUMENT_RESULT_REF_KIND => self
                .get_knowledge_rich_document(result_ref_id)
                .await?
                .ok_or(StorageError::NotFound(
                    "knowledge idempotency result rich document",
                )),
            _ => Err(StorageError::Validation(
                "knowledge idempotency result ref kind is not valid for rich document save",
            )),
        }
    }
}

#[cfg(test)]
mod wiki_store_tests {
    use super::*;
    use crate::storage::{tests::embedded_test_backend, Database, NewWorkspace, WriteContext};
    use serde_json::json;

    fn wiki_page(workspace_id: &str, title: &str, rendered_content: &str) -> NewKnowledgeWikiPage {
        NewKnowledgeWikiPage {
            workspace_id: workspace_id.to_owned(),
            title: title.to_owned(),
            page_type: Some("module".to_owned()),
            source_records: json!([]),
            rendered_content: rendered_content.to_owned(),
            staleness_hash: "a".repeat(64),
            compile_stamp: json!({
                "ledger_version": 0,
                "cited_sources": [
                    {"kind": "source", "id": "SRC-1"},
                    {"kind": "entity", "id": "ENT-1", "source_id": "SRC-1"}
                ]
            }),
            compile_recipe: Some(json!({"kind": "module"})),
            page_links: json!([]),
            rebuild_receipt_event_id: None,
        }
    }

    #[tokio::test]
    async fn embedded_wiki_page_seam_preserves_identity_citations_and_links() -> StorageResult<()> {
        let backend = embedded_test_backend().await?;
        let workspace = backend
            .database
            .create_workspace(
                &WriteContext::human(None),
                NewWorkspace {
                    name: "Surreal wiki seam".to_owned(),
                },
            )
            .await?;
        let database = SurrealDatabase::new(backend.storage.clone());

        assert_eq!(database.current_event_ledger_version().await?, 0);
        assert!(database
            .get_knowledge_code_file_by_source("missing-source")
            .await?
            .is_none());

        let created = database
            .upsert_knowledge_wiki_page(wiki_page(&workspace.id, "Module A", "first"))
            .await?;
        let updated = database
            .upsert_knowledge_wiki_page(wiki_page(&workspace.id, "Module A", "second"))
            .await?;
        assert_eq!(created.projection_id, updated.projection_id);
        assert_eq!(updated.rendered_content, "second");

        let source_citations = database
            .list_knowledge_wiki_pages_citing(&workspace.id, "source", "SRC-1")
            .await?;
        assert_eq!(source_citations.len(), 1);
        let entity_source_citations = database
            .list_knowledge_wiki_pages_citing_entity_source(&workspace.id, "SRC-1")
            .await?;
        assert_eq!(entity_source_citations.len(), 1);

        let links = json!([{"title": "Module B", "projection_id": "KWP-B"}]);
        database
            .update_knowledge_wiki_page_links(&updated.projection_id, &links)
            .await?;
        let listed = database
            .list_knowledge_wiki_pages(&workspace.id, Some("module"), true, 100, 0)
            .await?;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].page_links, links);

        let replacement = database
            .replace_knowledge_wiki_page_by_projection_id(
                &updated.projection_id,
                wiki_page(&workspace.id, "Module Renamed", "replacement"),
            )
            .await?;
        assert_eq!(replacement.projection_id, updated.projection_id);
        assert_eq!(replacement.title, "Module Renamed");
        assert_eq!(
            database
                .list_knowledge_wiki_pages(&workspace.id, None, false, 100, 0)
                .await?
                .len(),
            1
        );

        drop(database);
        backend.close_and_remove().await?;
        Ok(())
    }
}
