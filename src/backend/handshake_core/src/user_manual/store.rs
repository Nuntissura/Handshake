//! MT-194 UserManualStorageModel: embedded SurrealDB store for UserManual pages,
//! sections, anchors, tool entries, feature entries, version metadata, and
//! legacy aliases. EventLedger receipts use the
//! `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` family.
//!
//! Authority law (spec 2.3.13.11 / 10.15.8): these rows ARE the UserManual.
//! The compiled-in seed corpus (`super::seed`) is the deterministic input;
//! rendered markdown/HTML are projections. All list reads are bounded.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

use super::freshness::{FreshnessReport, FreshnessVerdict, FreshnessVerdictKind};
use super::registry::wp009_surface_registry;
use super::seed::{corpus_hash, seed_corpus, SeedReport};
use super::USER_MANUAL_VERSION;
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::surreal::SurrealStorage;
use crate::storage::{StorageError, StorageResult};

/// Bound for list/search reads (matches the knowledge API convention).
pub const LIST_CAP: i64 = 500;

// ---------------------------------------------------------------------------
// Row types.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct UserManualPage {
    pub page_id: String,
    pub slug: String,
    pub title: String,
    pub page_kind: String,
    pub audience: String,
    pub body: Value,
    pub content_hash: String,
    pub manual_version: String,
    pub source_kind: String,
    pub spec_anchors: Vec<String>,
    pub status: String,
    pub superseded_by_slug: Option<String>,
    pub ledger_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct UserManualSection {
    pub section_id: String,
    pub page_id: String,
    pub position: i32,
    pub section_kind: String,
    pub title: String,
    pub body_md: String,
    pub body_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct UserManualAnchor {
    pub anchor_id: String,
    pub page_id: String,
    pub anchor_kind: String,
    pub anchor_value: String,
    /// Empty string when not an HTTP route anchor.
    pub http_method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct UserManualToolEntry {
    pub tool_id: String,
    pub page_id: Option<String>,
    pub name: String,
    pub status: String,
    pub ipc_channel: Option<String>,
    pub tauri_command: Option<String>,
    pub cli_flag: Option<String>,
    pub http_route: Option<String>,
    pub http_method: String,
    pub description: String,
    pub expected_input: String,
    pub expected_output: String,
    pub schema_fields: Vec<String>,
    pub common_errors: Vec<String>,
    pub recovery_steps: Vec<String>,
    pub origin: String,
    pub content_hash: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct UserManualFeatureEntry {
    pub feature_id: String,
    pub title: String,
    pub description: String,
    pub tool_ids: Vec<String>,
    pub origin: String,
    pub content_hash: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct UserManualVersionRow {
    pub manual_version: String,
    pub seeded_at: DateTime<Utc>,
    pub seed_content_hash: String,
    pub page_count: i32,
    pub tool_count: i32,
    pub feature_count: i32,
    pub ledger_event_id: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct LegacyAliasRow {
    pub alias: String,
    pub alias_kind: String,
    pub canonical_kind: String,
    pub canonical_ref: String,
    pub deprecation_note: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
pub struct ManualSearchHit {
    /// `page` | `section` | `tool`.
    pub result_kind: String,
    /// Page slug or tool id.
    pub result_ref: String,
    /// The owning page slug (for sections); equals `result_ref` for pages.
    pub page_slug: Option<String>,
    pub title: String,
    pub excerpt: String,
}

// ---------------------------------------------------------------------------
// Seed input types (`super::seed` builds these; the store persists them).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NewManualSection {
    pub section_kind: &'static str,
    pub title: String,
    pub body_md: String,
    pub body_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NewManualAnchor {
    pub anchor_kind: &'static str,
    pub anchor_value: String,
    /// Empty when not an HTTP route anchor.
    pub http_method: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NewUserManualPage {
    pub slug: String,
    pub title: String,
    pub page_kind: &'static str,
    pub audience: &'static str,
    pub spec_anchors: Vec<String>,
    pub sections: Vec<NewManualSection>,
    pub anchors: Vec<NewManualAnchor>,
}

impl NewUserManualPage {
    /// Canonical content hash over everything a reader can observe. The
    /// MT-204 freshness check compares this compiled-in hash against the
    /// stored row: a drifted DB row (or a changed seed without resync) is
    /// `stale_content`, never silent.
    pub fn content_hash(&self) -> String {
        sha256_hex(
            &serde_json::to_string(&json!({
                "slug": self.slug,
                "title": self.title,
                "page_kind": self.page_kind,
                "audience": self.audience,
                "spec_anchors": self.spec_anchors,
                "sections": self.sections,
                "anchors": self.anchors,
            }))
            .expect("manual page serializes"),
        )
    }

    /// The denormalized `body` object persisted on the page record.
    pub fn body_json(&self) -> Value {
        json!({
            "sections": self.sections,
            "anchors": self.anchors,
        })
    }
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn section_rows_match_seed(stored: &[UserManualSection], expected: &[NewManualSection]) -> bool {
    stored.len() == expected.len()
        && stored
            .iter()
            .zip(expected)
            .enumerate()
            .all(|(position, (stored, expected))| {
                stored.position == position as i32
                    && stored.section_kind.as_str() == expected.section_kind
                    && stored.title.as_str() == expected.title.as_str()
                    && stored.body_md.as_str() == expected.body_md.as_str()
                    && stored.body_json.as_ref() == expected.body_json.as_ref()
            })
}

fn anchor_rows_match_seed(stored: &[UserManualAnchor], expected: &[NewManualAnchor]) -> bool {
    let stored_keys: BTreeSet<_> = stored
        .iter()
        .map(|anchor| {
            (
                anchor.anchor_kind.as_str(),
                anchor.anchor_value.as_str(),
                anchor.http_method.as_str(),
            )
        })
        .collect();
    let expected_keys: BTreeSet<_> = expected
        .iter()
        .map(|anchor| {
            (
                anchor.anchor_kind,
                anchor.anchor_value.as_str(),
                anchor.http_method,
            )
        })
        .collect();
    stored_keys == expected_keys
}

// ---------------------------------------------------------------------------
// Store.
// ---------------------------------------------------------------------------

#[derive(Debug, SurrealValue)]
struct RecordLookup {
    record: RecordId,
}

#[derive(Debug, SurrealValue)]
struct StringLookup {
    value: String,
}

#[derive(Debug, SurrealValue)]
struct FilteredListBindings {
    first: Option<String>,
    second: Option<String>,
    limit: i64,
}

#[derive(Debug, SurrealValue)]
struct PageRecordBindings {
    page: RecordId,
    limit: i64,
}

#[derive(Debug, SurrealValue)]
struct ReceiptEventWrite {
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: Value,
}

#[derive(Debug, SurrealValue)]
struct ReceiptWriteBindings {
    event: ReceiptEventWrite,
}

#[derive(Debug, SurrealValue)]
struct ReceiptResult {
    event_id: String,
}

#[derive(Debug, SurrealValue)]
struct ManualPageContent {
    page_id: String,
    slug: String,
    title: String,
    page_kind: String,
    audience: String,
    body: Value,
    content_hash: String,
    manual_version: String,
    source_kind: String,
    spec_anchors: Vec<String>,
    status: String,
    superseded_by_slug: Option<String>,
    ledger_event_id: RecordId,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct ManualSectionContent {
    section_id: String,
    page_id: RecordId,
    position: i64,
    section_kind: String,
    title: String,
    body_md: String,
    body_json: Option<Value>,
}

#[derive(Debug, SurrealValue)]
struct ManualAnchorContent {
    anchor_id: String,
    page_id: RecordId,
    anchor_kind: String,
    anchor_value: String,
    http_method: String,
}

#[derive(Debug, SurrealValue)]
struct SectionWrite {
    record: RecordId,
    content: ManualSectionContent,
}

#[derive(Debug, SurrealValue)]
struct AnchorWrite {
    record: RecordId,
    content: ManualAnchorContent,
}

#[derive(Debug, SurrealValue)]
struct PageWriteBindings {
    page_record: RecordId,
    page: ManualPageContent,
    sections: Vec<SectionWrite>,
    anchors: Vec<AnchorWrite>,
    event: ReceiptEventWrite,
}

#[derive(Debug, SurrealValue)]
struct PageWriteResult {
    page_id: String,
}

#[derive(Debug, SurrealValue)]
struct PageSearchRow {
    slug: String,
    title: String,
}

#[derive(Debug, SurrealValue)]
struct SectionSearchRow {
    page_slug: String,
    title: String,
    body_md: String,
}

#[derive(Debug, SurrealValue)]
struct ToolSearchRow {
    tool_id: String,
    name: String,
    description: String,
}

#[derive(Debug, SurrealValue)]
struct SearchBindings {
    needle: String,
    limit: i64,
}

#[derive(Debug, SurrealValue)]
struct StringRow {
    value: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct RecordStringBindings {
    record: RecordId,
    value: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct RecordTwoStringsBindings {
    record: RecordId,
    first: String,
    second: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct TwoStringsBindings {
    first: String,
    second: String,
}

#[derive(Debug, SurrealValue)]
struct LimitBindings {
    limit: i64,
}

#[derive(Debug, SurrealValue)]
struct ManualToolContent {
    tool_id: String,
    page_id: Option<RecordId>,
    name: String,
    status: String,
    ipc_channel: Option<String>,
    tauri_command: Option<String>,
    cli_flag: Option<String>,
    http_route: Option<String>,
    http_method: String,
    description: String,
    expected_input: String,
    expected_output: String,
    schema_fields: Vec<String>,
    common_errors: Vec<String>,
    recovery_steps: Vec<String>,
    origin: String,
    content_hash: String,
    manual_version: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct ManualFeatureContent {
    feature_id: String,
    title: String,
    description: String,
    tool_ids: Vec<String>,
    origin: String,
    content_hash: String,
    manual_version: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct ManualAliasContent {
    alias: String,
    alias_kind: String,
    canonical_kind: String,
    canonical_ref: String,
    deprecation_note: String,
    manual_version: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct ManualVersionContent {
    manual_version: String,
    seeded_at: DateTime<Utc>,
    seed_content_hash: String,
    page_count: i64,
    tool_count: i64,
    feature_count: i64,
    ledger_event_id: Option<RecordId>,
    note: String,
}

#[derive(Debug, SurrealValue)]
struct VersionReceiptBindings {
    version_record: RecordId,
    version: ManualVersionContent,
    event: ReceiptEventWrite,
}

const APPEND_RECEIPT_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT event_id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key LIMIT 2);
IF array::len($existing) > 1 {
    THROW 'user manual receipt identity is ambiguous';
} ELSE IF array::len($existing) = 1 {
    RETURN $existing;
} ELSE {
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT {
        event_id: $event.event_id,
        event_version: $event.event_version,
        kernel_task_run_id: $event.kernel_task_run_id,
        session_run_id: $event.session_run_id,
        aggregate_type: $event.aggregate_type,
        aggregate_id: $event.aggregate_id,
        idempotency_key: $event.idempotency_key,
        event_type: $event.event_type,
        actor_kind: $event.actor_kind,
        actor_id: $event.actor_id,
        causation_id: $event.causation_id,
        correlation_id: $event.correlation_id,
        payload_hash: $event.payload_hash,
        source_component: $event.source_component,
        payload: $event.payload
    };
    RETURN [{ event_id: $event.event_id }];
};
COMMIT TRANSACTION;
"#;

const UPSERT_PAGE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key LIMIT 2);
IF array::len($event_existing) != 0 {
    THROW 'user manual page receipt already exists without this mutation';
};
CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT {
    event_id: $event.event_id,
    event_version: $event.event_version,
    kernel_task_run_id: $event.kernel_task_run_id,
    session_run_id: $event.session_run_id,
    aggregate_type: $event.aggregate_type,
    aggregate_id: $event.aggregate_id,
    idempotency_key: $event.idempotency_key,
    event_type: $event.event_type,
    actor_kind: $event.actor_kind,
    actor_id: $event.actor_id,
    causation_id: $event.causation_id,
    correlation_id: $event.correlation_id,
    payload_hash: $event.payload_hash,
    source_component: $event.source_component,
    payload: $event.payload
};
UPSERT $page_record MERGE $page;
DELETE user_manual_sections WHERE page_id = $page_record;
DELETE user_manual_anchors WHERE page_id = $page_record;
FOR $section IN $sections {
    UPSERT $section.record CONTENT $section.content;
};
FOR $anchor IN $anchors {
    UPSERT $anchor.record CONTENT $anchor.content;
};
RETURN [{ page_id: $page.page_id }];
COMMIT TRANSACTION;
"#;

const UPSERT_VERSION_WITH_RECEIPT_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $event_existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key LIMIT 2);
IF array::len($event_existing) != 0 {
    THROW 'user manual version receipt already exists without this mutation';
};
CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT {
    event_id: $event.event_id,
    event_version: $event.event_version,
    kernel_task_run_id: $event.kernel_task_run_id,
    session_run_id: $event.session_run_id,
    aggregate_type: $event.aggregate_type,
    aggregate_id: $event.aggregate_id,
    idempotency_key: $event.idempotency_key,
    event_type: $event.event_type,
    actor_kind: $event.actor_kind,
    actor_id: $event.actor_id,
    causation_id: $event.causation_id,
    correlation_id: $event.correlation_id,
    payload_hash: $event.payload_hash,
    source_component: $event.source_component,
    payload: $event.payload
};
UPSERT $version_record CONTENT $version;
RETURN [{ event_id: $event.event_id }];
COMMIT TRANSACTION;
"#;

/// Product-global UserManual store over the shared embedded database. The
/// cloned handle retains only the lifecycle-managed SurrealStorage authority.
#[derive(Clone)]
pub struct UserManualStore {
    storage: SurrealStorage,
}

impl UserManualStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    async fn query_values<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> StorageResult<Vec<R>>
    where
        R: SurrealValue + Send,
        B: SurrealValue + Send,
    {
        Ok(self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await?)
    }

    async fn query_values_at<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
        result_index: usize,
    ) -> StorageResult<Vec<R>>
    where
        R: SurrealValue + Send,
        B: SurrealValue + Send,
    {
        Ok(self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at(statement, bindings, result_index)
                        .await
                })
            })
            .await?)
    }

    async fn query_first<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> StorageResult<Option<R>>
    where
        R: SurrealValue + Send,
        B: SurrealValue + Send,
    {
        Ok(self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move { database.query_first(statement, bindings).await })
            })
            .await?)
    }

    /// Append a `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` EventLedger receipt for
    /// a manual mutation. Returns the event id for the row's
    /// `ledger_event_id`.
    fn manual_receipt_event(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
    ) -> StorageResult<NewKernelEvent> {
        self.manual_receipt_event_with_key(
            action,
            subject,
            payload,
            format!("UMR-{}", Uuid::now_v7()),
        )
    }

    fn manual_mutation_receipt_event(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
        predecessor: &str,
    ) -> StorageResult<NewKernelEvent> {
        let target = serde_json::to_string(&payload)
            .map_err(|_| StorageError::Validation("user manual mutation payload invalid"))?;
        let idempotency_key = format!(
            "UMR-MUT-{}",
            sha256_hex(&format!("{action}\0{subject}\0{predecessor}\0{target}"))
        );
        self.manual_receipt_event_with_key(
            action,
            subject,
            json!({
                "predecessor_fingerprint": predecessor,
                "target": payload,
            }),
            idempotency_key,
        )
    }

    fn manual_receipt_event_with_key(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
        idempotency_key: String,
    ) -> StorageResult<NewKernelEvent> {
        NewKernelEvent::builder(
            format!("UM-{}", Uuid::now_v7()),
            format!("UMS-{}", Uuid::now_v7()),
            KernelEventType::KnowledgeUserManualEntryRecorded,
            KernelActor::System("user_manual".to_string()),
        )
        .aggregate("user_manual_entry", subject)
        .idempotency_key(idempotency_key)
        .source_component("user_manual::store")
        .payload(json!({
            "action": action,
            "subject": subject,
            "detail": payload,
        }))
        .build()
        .map_err(|_| StorageError::Validation("user manual receipt event invalid"))
    }

    pub async fn append_manual_receipt(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
    ) -> StorageResult<String> {
        let event = receipt_write(self.manual_receipt_event(action, subject, payload)?);
        let mut rows = self
            .query_values_at::<ReceiptResult, _>(
                APPEND_RECEIPT_QUERY,
                ReceiptWriteBindings { event },
                2,
            )
            .await?;
        rows.pop()
            .map(|row| row.event_id)
            .ok_or(StorageError::Validation(
                "user manual receipt transaction returned no event",
            ))
    }

    // -- pages ---------------------------------------------------------------

    /// Idempotent page upsert keyed on `slug`. Returns `(page_id, changed)`:
    /// `changed == false` means the stored record already matches the target
    /// content hash, version, status, and child-record content (a tampered or
    /// partially deleted section/anchor set is NOT current even when the page
    /// hash matches, so resync heals it). On change, sections and anchors are replaced
    /// transactionally and a receipt is appended. The deterministic Surreal
    /// record identity `user_manual_pages:<slug>` keeps `page_id` stable; the
    /// receipt, page record, deterministic section/anchor records, and child
    /// replacement commit in one embedded transaction.
    pub async fn upsert_page(
        &self,
        page: &NewUserManualPage,
        manual_version: &str,
        status: &str,
    ) -> StorageResult<(String, bool)> {
        let content_hash = page.content_hash();
        let existing = self.get_page_by_slug(&page.slug).await?;
        if let Some((stored, sections, anchors)) = &existing {
            if stored.content_hash == content_hash
                && stored.manual_version == manual_version
                && stored.status == status
                && section_rows_match_seed(sections, &page.sections)
                && anchor_rows_match_seed(anchors, &page.anchors)
            {
                return Ok((stored.page_id.clone(), false));
            }
        }

        let predecessor = existing
            .as_ref()
            .map(|stored| {
                serde_json::to_string(stored)
                    .map(|value| sha256_hex(&value))
                    .map_err(|_| StorageError::Validation("user manual page predecessor invalid"))
            })
            .transpose()?
            .unwrap_or_else(|| "absent".to_owned());
        let receipt_event = receipt_write(self.manual_mutation_receipt_event(
            if existing.is_some() {
                "page_updated"
            } else {
                "page_seeded"
            },
            &page.slug,
            json!({
                "content_hash": content_hash,
                "manual_version": manual_version,
                "page_kind": page.page_kind,
                "status": status,
            }),
            &predecessor,
        )?);
        let page_id = page.slug.clone();
        let page_record = RecordId::new("user_manual_pages", page_id.clone());
        let receipt_record = RecordId::new("kernel_event_ledger", receipt_event.event_id.clone());
        let sections = page
            .sections
            .iter()
            .enumerate()
            .map(|(position, section)| {
                let section_id = sha256_hex(&format!("{page_id}:section:{position}"));
                SectionWrite {
                    record: RecordId::new("user_manual_sections", section_id.clone()),
                    content: ManualSectionContent {
                        section_id,
                        page_id: page_record.clone(),
                        position: position as i64,
                        section_kind: section.section_kind.to_owned(),
                        title: section.title.clone(),
                        body_md: section.body_md.clone(),
                        body_json: section.body_json.clone(),
                    },
                }
            })
            .collect();
        let anchors = page
            .anchors
            .iter()
            .map(|anchor| {
                let anchor_id = sha256_hex(&format!(
                    "{page_id}:anchor:{}:{}:{}",
                    anchor.anchor_kind, anchor.anchor_value, anchor.http_method
                ));
                AnchorWrite {
                    record: RecordId::new("user_manual_anchors", anchor_id.clone()),
                    content: ManualAnchorContent {
                        anchor_id,
                        page_id: page_record.clone(),
                        anchor_kind: anchor.anchor_kind.to_owned(),
                        anchor_value: anchor.anchor_value.clone(),
                        http_method: anchor.http_method.to_owned(),
                    },
                }
            })
            .collect();
        let bindings = PageWriteBindings {
            page_record,
            page: ManualPageContent {
                page_id: page_id.clone(),
                slug: page.slug.clone(),
                title: page.title.clone(),
                page_kind: page.page_kind.to_owned(),
                audience: page.audience.to_owned(),
                body: page.body_json(),
                content_hash,
                manual_version: manual_version.to_owned(),
                source_kind: "builtin_seed".to_owned(),
                spec_anchors: page.spec_anchors.clone(),
                status: status.to_owned(),
                superseded_by_slug: None,
                ledger_event_id: receipt_record,
                updated_at: Utc::now(),
            },
            sections,
            anchors,
            event: receipt_event,
        };
        let mut rows = self
            .query_values_at::<PageWriteResult, _>(UPSERT_PAGE_QUERY, bindings, 9)
            .await?;
        let stored = rows.pop().ok_or(StorageError::Validation(
            "user manual page transaction returned no page",
        ))?;
        Ok((stored.page_id, true))
    }

    pub(crate) async fn page_child_rows_match_seed(
        &self,
        page_id: &str,
        page: &NewUserManualPage,
    ) -> StorageResult<bool> {
        let sections = self.sections_for(page_id).await?;
        if !section_rows_match_seed(&sections, &page.sections) {
            return Ok(false);
        }
        let anchors = self.anchors_for(page_id).await?;
        Ok(anchor_rows_match_seed(&anchors, &page.anchors))
    }

    pub async fn get_page_by_slug(
        &self,
        slug: &str,
    ) -> StorageResult<
        Option<(
            UserManualPage,
            Vec<UserManualSection>,
            Vec<UserManualAnchor>,
        )>,
    > {
        let rows = self
            .query_values::<UserManualPage, _>(
                r#"
            SELECT page_id, slug, title, page_kind, audience, body, content_hash,
                   manual_version, source_kind, spec_anchors, status,
                   superseded_by_slug, record::id(ledger_event_id) AS ledger_event_id,
                   created_at, updated_at
            FROM user_manual_pages WHERE slug = $value LIMIT 2
            "#,
                StringLookup {
                    value: slug.to_owned(),
                },
            )
            .await?;
        if rows.len() > 1 {
            return Err(StorageError::Validation(
                "user manual page slug identity is ambiguous",
            ));
        }
        let Some(page) = rows.into_iter().next() else {
            return Ok(None);
        };
        let sections = self.sections_for(&page.page_id).await?;
        let anchors = self.anchors_for(&page.page_id).await?;
        Ok(Some((page, sections, anchors)))
    }

    pub async fn sections_for(&self, page_id: &str) -> StorageResult<Vec<UserManualSection>> {
        self.query_values(
            r#"
            SELECT section_id, record::id(page_id) AS page_id, position,
                   section_kind, title, body_md, body_json
            FROM user_manual_sections WHERE page_id = $page
            ORDER BY position ASC LIMIT $limit
            "#,
            PageRecordBindings {
                page: RecordId::new("user_manual_pages", page_id.to_owned()),
                limit: LIST_CAP,
            },
        )
        .await
    }

    pub async fn anchors_for(&self, page_id: &str) -> StorageResult<Vec<UserManualAnchor>> {
        self.query_values(
            r#"
            SELECT anchor_id, record::id(page_id) AS page_id, anchor_kind,
                   anchor_value, http_method
            FROM user_manual_anchors WHERE page_id = $page
            ORDER BY anchor_kind, anchor_value LIMIT $limit
            "#,
            PageRecordBindings {
                page: RecordId::new("user_manual_pages", page_id.to_owned()),
                limit: LIST_CAP,
            },
        )
        .await
    }

    pub async fn list_pages(
        &self,
        page_kind: Option<&str>,
        audience: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<UserManualPage>> {
        let limit = limit.clamp(1, LIST_CAP);
        self.query_values(
            r#"
            SELECT page_id, slug, title, page_kind, audience, body, content_hash,
                   manual_version, source_kind, spec_anchors, status,
                   superseded_by_slug, record::id(ledger_event_id) AS ledger_event_id,
                   created_at, updated_at
            FROM user_manual_pages
            WHERE ($first = NONE OR page_kind = $first)
              AND ($second = NONE OR audience = $second)
            ORDER BY slug ASC
            LIMIT $limit
            "#,
            FilteredListBindings {
                first: page_kind.map(str::to_owned),
                second: audience.map(str::to_owned),
                limit,
            },
        )
        .await
    }

    /// All anchors of a kind across pages (the MT-195 coverage gate and the
    /// MT-204 freshness check run over these).
    pub async fn anchors_by_kind(&self, anchor_kind: &str) -> StorageResult<Vec<UserManualAnchor>> {
        self.query_values(
            r#"
            SELECT anchor_id, record::id(page_id) AS page_id, anchor_kind,
                   anchor_value, http_method
            FROM user_manual_anchors WHERE anchor_kind = $value
            ORDER BY anchor_value LIMIT 500
            "#,
            StringLookup {
                value: anchor_kind.to_owned(),
            },
        )
        .await
    }

    /// MT-201 linking: pages this page links to (`page_link` anchors out) and
    /// pages that link to this page (in), resolved through slugs.
    pub async fn page_links(
        &self,
        slug: &str,
    ) -> StorageResult<Option<(Vec<String>, Vec<String>)>> {
        let Some((page, _, _)) = self.get_page_by_slug(slug).await? else {
            return Ok(None);
        };
        let outbound = self
            .query_values::<StringRow, _>(
                r#"
                SELECT anchor_value AS value FROM user_manual_anchors
                WHERE page_id = $page AND anchor_kind = 'page_link'
                ORDER BY anchor_value LIMIT $limit
                "#,
                PageRecordBindings {
                    page: RecordId::new("user_manual_pages", page.page_id),
                    limit: LIST_CAP,
                },
            )
            .await?
            .into_iter()
            .map(|row| row.value)
            .collect();
        let inbound = self
            .query_values::<StringRow, _>(
                r#"
                SELECT page_id.slug AS value FROM user_manual_anchors
                WHERE anchor_kind = 'page_link' AND anchor_value = $value
                ORDER BY value LIMIT 500
                "#,
                StringLookup {
                    value: slug.to_owned(),
                },
            )
            .await?
            .into_iter()
            .map(|row| row.value)
            .collect();
        Ok(Some((outbound, inbound)))
    }

    /// Bounded case-insensitive search across pages, sections, and tools.
    pub async fn search(&self, query: &str, limit: i64) -> StorageResult<Vec<ManualSearchHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.clamp(1, LIST_CAP);
        let needle = trimmed.to_lowercase();
        let mut hits = Vec::new();

        let page_rows = self
            .query_values::<PageSearchRow, _>(
                r#"
            SELECT slug, title FROM user_manual_pages
            WHERE string::lowercase(title) CONTAINS $needle
               OR string::lowercase(slug) CONTAINS $needle
            ORDER BY slug LIMIT $limit
            "#,
                SearchBindings {
                    needle: needle.clone(),
                    limit,
                },
            )
            .await?;
        for row in page_rows {
            let slug = row.slug;
            hits.push(ManualSearchHit {
                result_kind: "page".into(),
                result_ref: slug.clone(),
                page_slug: Some(slug),
                title: row.title,
                excerpt: String::new(),
            });
        }

        let section_rows = self
            .query_values::<SectionSearchRow, _>(
                r#"
            SELECT page_id.slug AS page_slug, title, body_md
            FROM user_manual_sections
            WHERE string::lowercase(title) CONTAINS $needle
               OR string::lowercase(body_md) CONTAINS $needle
            ORDER BY page_slug, position LIMIT $limit
            "#,
                SearchBindings {
                    needle: needle.clone(),
                    limit,
                },
            )
            .await?;
        for row in section_rows {
            hits.push(ManualSearchHit {
                result_kind: "section".into(),
                result_ref: row.page_slug.clone(),
                page_slug: Some(row.page_slug),
                title: row.title,
                excerpt: excerpt_around(&row.body_md, trimmed),
            });
        }

        let tool_rows = self
            .query_values::<ToolSearchRow, _>(
                r#"
            SELECT tool_id, name, description FROM user_manual_tool_entries
            WHERE string::lowercase(tool_id) CONTAINS $needle
               OR string::lowercase(name) CONTAINS $needle
               OR string::lowercase(description) CONTAINS $needle
               OR (http_route != NONE AND string::lowercase(http_route) CONTAINS $needle)
            ORDER BY tool_id LIMIT $limit
            "#,
                SearchBindings { needle, limit },
            )
            .await?;
        for row in tool_rows {
            hits.push(ManualSearchHit {
                result_kind: "tool".into(),
                result_ref: row.tool_id,
                page_slug: None,
                title: row.name,
                excerpt: row.description,
            });
        }

        hits.truncate(limit as usize);
        Ok(hits)
    }

    // -- tool entries ----------------------------------------------------------

    pub async fn upsert_tool_entry(&self, entry: &UserManualToolEntry) -> StorageResult<bool> {
        let stored = self.get_tool_entry(&entry.tool_id).await?;
        if stored.as_ref() == Some(entry) {
            return Ok(false);
        }
        let tool_id = entry.tool_id.clone();
        let content = ManualToolContent {
            tool_id: tool_id.clone(),
            page_id: entry
                .page_id
                .as_ref()
                .map(|id| RecordId::new("user_manual_pages", id.clone())),
            name: entry.name.clone(),
            status: entry.status.clone(),
            ipc_channel: entry.ipc_channel.clone(),
            tauri_command: entry.tauri_command.clone(),
            cli_flag: entry.cli_flag.clone(),
            http_route: entry.http_route.clone(),
            http_method: entry.http_method.clone(),
            description: entry.description.clone(),
            expected_input: entry.expected_input.clone(),
            expected_output: entry.expected_output.clone(),
            schema_fields: entry.schema_fields.clone(),
            common_errors: entry.common_errors.clone(),
            recovery_steps: entry.recovery_steps.clone(),
            origin: entry.origin.clone(),
            content_hash: entry.content_hash.clone(),
            manual_version: entry.manual_version.clone(),
            updated_at: Utc::now(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>("user_manual_tool_entries", &tool_id, content)
                        .await
                })
            })
            .await?;
        Ok(true)
    }

    pub async fn get_tool_entry(
        &self,
        tool_id: &str,
    ) -> StorageResult<Option<UserManualToolEntry>> {
        self.query_first(
            r#"
            SELECT tool_id, record::id(page_id) AS page_id, name, status,
                   ipc_channel, tauri_command,
                   cli_flag, http_route, http_method, description, expected_input,
                   expected_output, schema_fields, common_errors, recovery_steps,
                   origin, content_hash, manual_version
            FROM $record
            "#,
            RecordLookup {
                record: RecordId::new("user_manual_tool_entries", tool_id.to_owned()),
            },
        )
        .await
    }

    pub async fn list_tool_entries(
        &self,
        status: Option<&str>,
        origin: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<UserManualToolEntry>> {
        let limit = limit.clamp(1, LIST_CAP);
        self.query_values(
            r#"
            SELECT tool_id, record::id(page_id) AS page_id, name, status,
                   ipc_channel, tauri_command,
                   cli_flag, http_route, http_method, description, expected_input,
                   expected_output, schema_fields, common_errors, recovery_steps,
                   origin, content_hash, manual_version
            FROM user_manual_tool_entries
            WHERE ($first = NONE OR status = $first)
              AND ($second = NONE OR origin = $second)
            ORDER BY tool_id LIMIT $limit
            "#,
            FilteredListBindings {
                first: status.map(str::to_owned),
                second: origin.map(str::to_owned),
                limit,
            },
        )
        .await
    }

    // -- feature entries --------------------------------------------------------

    pub async fn upsert_feature_entry(
        &self,
        entry: &UserManualFeatureEntry,
    ) -> StorageResult<bool> {
        let stored = self.get_feature_entry(&entry.feature_id).await?;
        if stored.as_ref() == Some(entry) {
            return Ok(false);
        }
        let feature_id = entry.feature_id.clone();
        let content = ManualFeatureContent {
            feature_id: feature_id.clone(),
            title: entry.title.clone(),
            description: entry.description.clone(),
            tool_ids: entry.tool_ids.clone(),
            origin: entry.origin.clone(),
            content_hash: entry.content_hash.clone(),
            manual_version: entry.manual_version.clone(),
            updated_at: Utc::now(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>("user_manual_feature_entries", &feature_id, content)
                        .await
                })
            })
            .await?;
        Ok(true)
    }

    pub async fn get_feature_entry(
        &self,
        feature_id: &str,
    ) -> StorageResult<Option<UserManualFeatureEntry>> {
        self.query_first(
            r#"
            SELECT feature_id, title, description, tool_ids, origin,
                   content_hash, manual_version
            FROM $record
            "#,
            RecordLookup {
                record: RecordId::new("user_manual_feature_entries", feature_id.to_owned()),
            },
        )
        .await
    }

    pub async fn list_feature_entries(
        &self,
        limit: i64,
    ) -> StorageResult<Vec<UserManualFeatureEntry>> {
        let limit = limit.clamp(1, LIST_CAP);
        self.query_values(
            r#"
            SELECT feature_id, title, description, tool_ids, origin,
                   content_hash, manual_version
            FROM user_manual_feature_entries ORDER BY feature_id LIMIT $limit
            "#,
            LimitBindings { limit },
        )
        .await
    }

    // -- legacy aliases -----------------------------------------------------------

    pub async fn upsert_legacy_alias(&self, alias: &LegacyAliasRow) -> StorageResult<bool> {
        let stored = self.get_legacy_alias(&alias.alias).await?;
        if stored.as_ref() == Some(alias) {
            return Ok(false);
        }
        let alias_id = alias.alias.clone();
        let content = ManualAliasContent {
            alias: alias.alias.clone(),
            alias_kind: alias.alias_kind.clone(),
            canonical_kind: alias.canonical_kind.clone(),
            canonical_ref: alias.canonical_ref.clone(),
            deprecation_note: alias.deprecation_note.clone(),
            manual_version: alias.manual_version.clone(),
            updated_at: Utc::now(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>("user_manual_legacy_aliases", &alias_id, content)
                        .await
                })
            })
            .await?;
        Ok(true)
    }

    pub async fn get_legacy_alias(&self, alias: &str) -> StorageResult<Option<LegacyAliasRow>> {
        self.query_first(
            r#"
            SELECT alias, alias_kind, canonical_kind, canonical_ref,
                   deprecation_note, manual_version
            FROM $record
            "#,
            RecordLookup {
                record: RecordId::new("user_manual_legacy_aliases", alias.to_owned()),
            },
        )
        .await
    }

    pub async fn list_legacy_aliases(&self) -> StorageResult<Vec<LegacyAliasRow>> {
        self.query_values(
            r#"
            SELECT alias, alias_kind, canonical_kind, canonical_ref,
                   deprecation_note, manual_version
            FROM user_manual_legacy_aliases ORDER BY alias LIMIT $limit
            "#,
            LimitBindings { limit: LIST_CAP },
        )
        .await
    }

    // -- version metadata ----------------------------------------------------------

    pub async fn record_version(
        &self,
        manual_version: &str,
        seed_content_hash: &str,
        page_count: i32,
        tool_count: i32,
        feature_count: i32,
        ledger_event_id: Option<&str>,
        note: &str,
    ) -> StorageResult<()> {
        let version_id = manual_version.to_owned();
        let content = ManualVersionContent {
            manual_version: version_id.clone(),
            seeded_at: Utc::now(),
            seed_content_hash: seed_content_hash.to_owned(),
            page_count: i64::from(page_count),
            tool_count: i64::from(tool_count),
            feature_count: i64::from(feature_count),
            ledger_event_id: ledger_event_id
                .map(|id| RecordId::new("kernel_event_ledger", id.to_owned())),
            note: note.to_owned(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>("user_manual_versions", &version_id, content)
                        .await
                })
            })
            .await?;
        Ok(())
    }

    pub async fn record_version_with_receipt(
        &self,
        manual_version: &str,
        seed_content_hash: &str,
        page_count: i32,
        tool_count: i32,
        feature_count: i32,
        receipt_payload: Value,
        note: &str,
    ) -> StorageResult<String> {
        let event = receipt_write(self.manual_receipt_event(
            "corpus_seeded",
            manual_version,
            receipt_payload,
        )?);
        let receipt_id = event.event_id.clone();
        let bindings = VersionReceiptBindings {
            version_record: RecordId::new("user_manual_versions", manual_version.to_owned()),
            version: ManualVersionContent {
                manual_version: manual_version.to_owned(),
                seeded_at: Utc::now(),
                seed_content_hash: seed_content_hash.to_owned(),
                page_count: i64::from(page_count),
                tool_count: i64::from(tool_count),
                feature_count: i64::from(feature_count),
                ledger_event_id: Some(RecordId::new("kernel_event_ledger", receipt_id.clone())),
                note: note.to_owned(),
            },
            event,
        };
        let rows = self
            .query_values_at::<ReceiptResult, _>(UPSERT_VERSION_WITH_RECEIPT_QUERY, bindings, 5)
            .await?;
        if rows.len() != 1 || rows[0].event_id != receipt_id {
            return Err(StorageError::Validation(
                "user manual version transaction returned an invalid receipt",
            ));
        }
        Ok(receipt_id)
    }

    pub async fn get_version(
        &self,
        manual_version: &str,
    ) -> StorageResult<Option<UserManualVersionRow>> {
        self.query_first(
            r#"
            SELECT manual_version, seeded_at, seed_content_hash, page_count,
                   tool_count, feature_count,
                   record::id(ledger_event_id) AS ledger_event_id, note
            FROM $record
            "#,
            RecordLookup {
                record: RecordId::new("user_manual_versions", manual_version.to_owned()),
            },
        )
        .await
    }

    /// Idempotently materialize the one canonical compiled corpus through this
    /// store so changed pages and the version row retain EventLedger linkage.
    pub async fn ensure_seeded(&self) -> StorageResult<SeedReport> {
        let corpus = seed_corpus();
        let seed_content_hash = corpus_hash(&corpus);
        let page_count = i32::try_from(corpus.pages.len()).map_err(|_| {
            StorageError::Validation("user manual page count exceeds storage range")
        })?;
        let tool_count = i32::try_from(corpus.tools.len()).map_err(|_| {
            StorageError::Validation("user manual tool count exceeds storage range")
        })?;
        let feature_count = i32::try_from(corpus.features.len()).map_err(|_| {
            StorageError::Validation("user manual feature count exceeds storage range")
        })?;

        let mut pages_changed = 0usize;
        for page in &corpus.pages {
            if self
                .upsert_page(page, USER_MANUAL_VERSION, "current")
                .await?
                .1
            {
                pages_changed += 1;
            }
        }
        let mut tools_changed = 0usize;
        for tool in &corpus.tools {
            if self.upsert_tool_entry(tool).await? {
                tools_changed += 1;
            }
        }
        let mut features_changed = 0usize;
        for feature in &corpus.features {
            if self.upsert_feature_entry(feature).await? {
                features_changed += 1;
            }
        }
        let mut aliases_changed = 0usize;
        for alias in &corpus.aliases {
            if self.upsert_legacy_alias(alias).await? {
                aliases_changed += 1;
            }
        }

        let version_changed =
            self.get_version(USER_MANUAL_VERSION)
                .await?
                .map_or(true, |version| {
                    version.seed_content_hash != seed_content_hash
                        || version.page_count != page_count
                        || version.tool_count != tool_count
                        || version.feature_count != feature_count
                });
        let corpus_changed = pages_changed + tools_changed + features_changed + aliases_changed > 0;
        let version_receipt_event_id = if corpus_changed || version_changed {
            Some(
                self.record_version_with_receipt(
                    USER_MANUAL_VERSION,
                    &seed_content_hash,
                    page_count,
                    tool_count,
                    feature_count,
                    json!({
                        "pages_total": corpus.pages.len(),
                        "pages_changed": pages_changed,
                        "tools_total": corpus.tools.len(),
                        "tools_changed": tools_changed,
                        "features_total": corpus.features.len(),
                        "features_changed": features_changed,
                        "aliases_total": corpus.aliases.len(),
                        "aliases_changed": aliases_changed,
                    }),
                    "WP-1 MT-022 canonical built-in UserManual seed corpus",
                )
                .await?,
            )
        } else {
            None
        };

        Ok(SeedReport {
            manual_version: USER_MANUAL_VERSION.to_owned(),
            seed_content_hash,
            pages_total: corpus.pages.len(),
            pages_changed,
            tools_total: corpus.tools.len(),
            tools_changed,
            features_total: corpus.features.len(),
            features_changed,
            aliases_total: corpus.aliases.len(),
            aliases_changed,
            version_receipt_event_id,
        })
    }

    /// Compare the product-global embedded projection with the canonical
    /// corpus and mounted-surface registry.
    pub async fn check_freshness(&self) -> StorageResult<FreshnessReport> {
        let corpus = seed_corpus();
        let seed_hash = corpus_hash(&corpus);
        let mut verdicts = Vec::new();

        match self.get_version(USER_MANUAL_VERSION).await? {
            Some(row)
                if row.seed_content_hash == seed_hash
                    && row.page_count as usize == corpus.pages.len()
                    && row.tool_count as usize == corpus.tools.len()
                    && row.feature_count as usize == corpus.features.len() => {}
            Some(row) => verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::UnseededVersion,
                subject: USER_MANUAL_VERSION.to_owned(),
                detail: format!(
                    "stored version metadata differs from the compiled-in corpus: hash {}/{}, pages {}/{}, tools {}/{}, features {}/{} — run POST /usermanual/resync",
                    row.seed_content_hash,
                    seed_hash,
                    row.page_count,
                    corpus.pages.len(),
                    row.tool_count,
                    corpus.tools.len(),
                    row.feature_count,
                    corpus.features.len(),
                ),
            }),
            None => verdicts.push(FreshnessVerdict {
                kind: FreshnessVerdictKind::UnseededVersion,
                subject: USER_MANUAL_VERSION.to_owned(),
                detail:
                    "no user_manual_versions row for this binary's corpus — run POST /usermanual/resync"
                        .to_owned(),
            }),
        }

        let stored_pages = self.list_pages(None, None, LIST_CAP).await?;
        let stored_by_slug: BTreeMap<&str, &UserManualPage> = stored_pages
            .iter()
            .map(|page| (page.slug.as_str(), page))
            .collect();
        let seed_slugs: BTreeSet<&str> =
            corpus.pages.iter().map(|page| page.slug.as_str()).collect();
        for page in &corpus.pages {
            match stored_by_slug.get(page.slug.as_str()) {
                None => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::MissingPage,
                    subject: page.slug.clone(),
                    detail: "seed expects this page; the database does not hold it".to_owned(),
                }),
                Some(stored) if stored.content_hash != page.content_hash() => {
                    verdicts.push(FreshnessVerdict {
                        kind: FreshnessVerdictKind::StaleContent,
                        subject: page.slug.clone(),
                        detail: format!(
                            "stored hash {} != seed hash {}",
                            stored.content_hash,
                            page.content_hash()
                        ),
                    });
                }
                Some(stored) => {
                    let (kind, detail) = if self
                        .page_child_rows_match_seed(&stored.page_id, page)
                        .await?
                    {
                        (FreshnessVerdictKind::Current, String::new())
                    } else {
                        (
                            FreshnessVerdictKind::StaleContent,
                            "stored page child rows differ from the seed corpus despite matching page hash"
                                .to_owned(),
                        )
                    };
                    verdicts.push(FreshnessVerdict {
                        kind,
                        subject: page.slug.clone(),
                        detail,
                    });
                }
            }
        }

        let stored_tools = self.list_tool_entries(None, None, LIST_CAP).await?;
        let stored_tools_by_id: BTreeMap<&str, &UserManualToolEntry> = stored_tools
            .iter()
            .map(|tool| (tool.tool_id.as_str(), tool))
            .collect();
        let seed_tool_ids: BTreeSet<&str> = corpus
            .tools
            .iter()
            .map(|tool| tool.tool_id.as_str())
            .collect();
        for tool in &corpus.tools {
            match stored_tools_by_id.get(tool.tool_id.as_str()) {
                None => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::MissingToolEntry,
                    subject: tool.tool_id.clone(),
                    detail: "seed expects this tool entry; the database does not hold it"
                        .to_owned(),
                }),
                Some(stored) if *stored != tool => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleToolEntry,
                    subject: tool.tool_id.clone(),
                    detail: "stored tool entry row differs from the seed corpus".to_owned(),
                }),
                Some(_) => {}
            }
        }
        for stored in &stored_tools {
            if !seed_tool_ids.contains(stored.tool_id.as_str()) {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleToolEntry,
                    subject: stored.tool_id.clone(),
                    detail: "database holds a tool entry the seed corpus does not declare"
                        .to_owned(),
                });
            }
        }

        let stored_features = self.list_feature_entries(LIST_CAP).await?;
        let stored_features_by_id: BTreeMap<&str, &UserManualFeatureEntry> = stored_features
            .iter()
            .map(|feature| (feature.feature_id.as_str(), feature))
            .collect();
        let seed_feature_ids: BTreeSet<&str> = corpus
            .features
            .iter()
            .map(|feature| feature.feature_id.as_str())
            .collect();
        for feature in &corpus.features {
            match stored_features_by_id.get(feature.feature_id.as_str()) {
                None => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::MissingFeatureEntry,
                    subject: feature.feature_id.clone(),
                    detail: "seed expects this feature entry; the database does not hold it"
                        .to_owned(),
                }),
                Some(stored) if *stored != feature => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleFeatureEntry,
                    subject: feature.feature_id.clone(),
                    detail: "stored feature entry row differs from the seed corpus".to_owned(),
                }),
                Some(_) => {}
            }
        }
        for stored in &stored_features {
            if !seed_feature_ids.contains(stored.feature_id.as_str()) {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleFeatureEntry,
                    subject: stored.feature_id.clone(),
                    detail: "database holds a feature entry the seed corpus does not declare"
                        .to_owned(),
                });
            }
        }

        let stored_aliases = self.list_legacy_aliases().await?;
        let stored_aliases_by_id: BTreeMap<&str, &LegacyAliasRow> = stored_aliases
            .iter()
            .map(|alias| (alias.alias.as_str(), alias))
            .collect();
        let seed_alias_ids: BTreeSet<&str> = corpus
            .aliases
            .iter()
            .map(|alias| alias.alias.as_str())
            .collect();
        for alias in &corpus.aliases {
            match stored_aliases_by_id.get(alias.alias.as_str()) {
                None => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::MissingLegacyAlias,
                    subject: alias.alias.clone(),
                    detail: "seed expects this legacy alias; the database does not hold it"
                        .to_owned(),
                }),
                Some(stored) if *stored != alias => verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleLegacyAlias,
                    subject: alias.alias.clone(),
                    detail: "stored legacy alias row differs from the seed corpus".to_owned(),
                }),
                Some(_) => {}
            }
        }
        for stored in &stored_aliases {
            if !seed_alias_ids.contains(stored.alias.as_str()) {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleLegacyAlias,
                    subject: stored.alias.clone(),
                    detail: "database holds a legacy alias the seed corpus does not declare"
                        .to_owned(),
                });
            }
        }

        for (subject, stored_count, seed_count) in [
            ("user_manual_pages", stored_pages.len(), corpus.pages.len()),
            (
                "user_manual_tool_entries",
                stored_tools.len(),
                corpus.tools.len(),
            ),
            (
                "user_manual_feature_entries",
                stored_features.len(),
                corpus.features.len(),
            ),
            (
                "user_manual_legacy_aliases",
                stored_aliases.len(),
                corpus.aliases.len(),
            ),
        ] {
            if stored_count != seed_count {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::StaleContent,
                    subject: subject.to_owned(),
                    detail: format!(
                        "stored row count {stored_count} differs from seed corpus count {seed_count}"
                    ),
                });
            }
        }

        let route_anchors = self.anchors_by_kind("http_route").await?;
        let covered: BTreeSet<(String, String)> = route_anchors
            .iter()
            .map(|anchor| (anchor.http_method.clone(), anchor.anchor_value.clone()))
            .collect();
        for surface in wp009_surface_registry() {
            if !covered.contains(&(surface.method.to_owned(), surface.route.to_owned())) {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::UncoveredSurface,
                    subject: format!("{} {}", surface.method, surface.route),
                    detail: format!(
                        "registry surface {} has no http_route anchor on any UserManual page",
                        surface.surface_id
                    ),
                });
            }
        }

        let declared: BTreeSet<(String, String)> = wp009_surface_registry()
            .iter()
            .map(|surface| (surface.method.to_owned(), surface.route.to_owned()))
            .collect();
        for anchor in &route_anchors {
            if !declared.contains(&(anchor.http_method.clone(), anchor.anchor_value.clone())) {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::DanglingAnchor,
                    subject: format!("{} {}", anchor.http_method, anchor.anchor_value),
                    detail:
                        "http_route anchor documents a surface the WP-009 registry does not declare"
                            .to_owned(),
                });
            }
        }

        let stored_slugs: BTreeSet<&str> =
            stored_pages.iter().map(|page| page.slug.as_str()).collect();
        for anchor in self.anchors_by_kind("page_link").await? {
            if !stored_slugs.contains(anchor.anchor_value.as_str())
                && !seed_slugs.contains(anchor.anchor_value.as_str())
            {
                verdicts.push(FreshnessVerdict {
                    kind: FreshnessVerdictKind::DanglingAnchor,
                    subject: anchor.anchor_value,
                    detail: "page_link anchor targets a page that exists neither in the database nor in the seed"
                        .to_owned(),
                });
            }
        }

        let current_count = verdicts
            .iter()
            .filter(|verdict| verdict.kind == FreshnessVerdictKind::Current)
            .count();
        let problem_count = verdicts
            .iter()
            .filter(|verdict| verdict.kind.is_problem())
            .count();
        Ok(FreshnessReport {
            manual_version: USER_MANUAL_VERSION.to_owned(),
            seed_content_hash: seed_hash,
            fresh: problem_count == 0,
            current_count,
            problem_count,
            verdicts,
        })
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_receipt_exists(&self, event_id: &str) -> StorageResult<bool> {
        let rows = self
            .query_values::<ReceiptResult, _>(
                r#"
                SELECT event_id FROM $record
                WHERE event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED'
                "#,
                RecordLookup {
                    record: RecordId::new("kernel_event_ledger", event_id.to_owned()),
                },
            )
            .await?;
        if rows.len() > 1 {
            return Err(StorageError::Validation(
                "user manual receipt record identity is ambiguous",
            ));
        }
        Ok(rows.len() == 1)
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_receipt_count(&self) -> StorageResult<usize> {
        Ok(self
            .query_values::<ReceiptResult, _>(
                r#"
                SELECT event_id FROM kernel_event_ledger
                WHERE event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED'
                LIMIT $limit
                "#,
                LimitBindings { limit: 1000 },
            )
            .await?
            .len())
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_set_page_content_hash(
        &self,
        slug: &str,
        content_hash: &str,
    ) -> StorageResult<()> {
        let Some((page, _, _)) = self.get_page_by_slug(slug).await? else {
            return Err(StorageError::Validation(
                "user manual page fixture target is missing",
            ));
        };
        self.query_values::<Value, _>(
            "UPDATE $record SET content_hash = $value",
            RecordStringBindings {
                record: RecordId::new("user_manual_pages", page.page_id),
                value: content_hash.to_owned(),
            },
        )
        .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_delete_page(&self, slug: &str) -> StorageResult<bool> {
        let Some((page, _, _)) = self.get_page_by_slug(slug).await? else {
            return Ok(false);
        };
        self.query_values::<Value, _>(
            "DELETE $record",
            RecordLookup {
                record: RecordId::new("user_manual_pages", page.page_id),
            },
        )
        .await?;
        Ok(self.get_page_by_slug(slug).await?.is_none())
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_delete_page_sections(&self, page_id: &str) -> StorageResult<()> {
        self.query_values::<Value, _>(
            "DELETE user_manual_sections WHERE page_id = $record",
            RecordLookup {
                record: RecordId::new("user_manual_pages", page_id.to_owned()),
            },
        )
        .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_tamper_section(
        &self,
        section_id: &str,
        title: &str,
        body_md: &str,
    ) -> StorageResult<()> {
        self.query_values::<Value, _>(
            "UPDATE $record SET title = $first, body_md = $second",
            RecordTwoStringsBindings {
                record: RecordId::new("user_manual_sections", section_id.to_owned()),
                first: title.to_owned(),
                second: body_md.to_owned(),
            },
        )
        .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_delete_route_anchor(&self, route: &str) -> StorageResult<usize> {
        let before = self
            .anchors_by_kind("http_route")
            .await?
            .into_iter()
            .filter(|anchor| anchor.anchor_value == route)
            .count();
        self.query_values::<Value, _>(
            "DELETE user_manual_anchors WHERE anchor_kind = $first AND anchor_value = $second",
            TwoStringsBindings {
                first: "http_route".to_owned(),
                second: route.to_owned(),
            },
        )
        .await?;
        Ok(before)
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_break_first_page_link(
        &self,
        slug: &str,
        missing_target: &str,
    ) -> StorageResult<String> {
        let Some((_, _, anchors)) = self.get_page_by_slug(slug).await? else {
            return Err(StorageError::Validation(
                "user manual page-link fixture target is missing",
            ));
        };
        let anchor = anchors
            .into_iter()
            .filter(|anchor| anchor.anchor_kind == "page_link")
            .min_by(|left, right| left.anchor_value.cmp(&right.anchor_value))
            .ok_or(StorageError::Validation(
                "user manual page-link fixture has no page link",
            ))?;
        self.query_values::<Value, _>(
            "UPDATE $record SET anchor_value = $value",
            RecordStringBindings {
                record: RecordId::new("user_manual_anchors", anchor.anchor_id.clone()),
                value: missing_target.to_owned(),
            },
        )
        .await?;
        Ok(anchor.anchor_id)
    }

    #[cfg(feature = "test-utils")]
    pub(super) async fn fixture_inject_page_receipt_without_mutation(
        &self,
        page: &NewUserManualPage,
        manual_version: &str,
    ) -> StorageResult<String> {
        if self.get_page_by_slug(&page.slug).await?.is_some() {
            return Err(StorageError::Validation(
                "orphan receipt fixture requires an absent page",
            ));
        }
        let content_hash = page.content_hash();
        let event = receipt_write(self.manual_mutation_receipt_event(
            "page_seeded",
            &page.slug,
            json!({
                "content_hash": content_hash,
                "manual_version": manual_version,
                "page_kind": page.page_kind,
                "status": "current",
            }),
            "absent",
        )?);
        let expected_event_id = event.event_id.clone();
        let rows = self
            .query_values_at::<ReceiptResult, _>(
                APPEND_RECEIPT_QUERY,
                ReceiptWriteBindings { event },
                2,
            )
            .await?;
        if rows.len() != 1 || rows[0].event_id != expected_event_id {
            return Err(StorageError::Validation(
                "orphan receipt fixture returned unstable evidence",
            ));
        }
        Ok(expected_event_id)
    }
}

fn receipt_write(event: NewKernelEvent) -> ReceiptEventWrite {
    let event = KernelEvent::from_new(event);
    let actor_kind = event.actor.actor_kind().to_owned();
    let actor_id = event.actor.actor_id().to_owned();
    ReceiptEventWrite {
        event_id: event.event_id,
        event_version: event.event_version,
        kernel_task_run_id: event.kernel_task_run_id,
        session_run_id: event.session_run_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        idempotency_key: event.idempotency_key,
        event_type: event.event_type.to_string(),
        actor_kind,
        actor_id,
        causation_id: event.causation_id,
        correlation_id: event.correlation_id,
        payload_hash: event.payload_hash,
        source_component: event.source_component,
        payload: event.payload,
    }
}

/// Bounded excerpt centred on the first case-insensitive match.
fn excerpt_around(body: &str, needle: &str) -> String {
    const WINDOW: usize = 160;
    let lower_body = body.to_lowercase();
    let lower_needle = needle.to_lowercase();
    let start = lower_body.find(&lower_needle).unwrap_or(0);
    let from = start.saturating_sub(WINDOW / 2);
    // Snap to char boundaries.
    let mut begin = from;
    while begin > 0 && !body.is_char_boundary(begin) {
        begin -= 1;
    }
    let mut end = (begin + WINDOW).min(body.len());
    while end < body.len() && !body.is_char_boundary(end) {
        end += 1;
    }
    body[begin..end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_stable_and_sensitive() {
        let mut page = NewUserManualPage {
            slug: "test-page".into(),
            title: "Test".into(),
            page_kind: "purpose",
            audience: "model_and_operator",
            spec_anchors: vec!["10.15.8".into()],
            sections: vec![NewManualSection {
                section_kind: "purpose",
                title: "Purpose".into(),
                body_md: "Body".into(),
                body_json: None,
            }],
            anchors: vec![NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/usermanual/pages".into(),
                http_method: "GET",
            }],
        };
        let h1 = page.content_hash();
        assert_eq!(h1, page.content_hash(), "hash is deterministic");
        assert_eq!(h1.len(), 64);
        page.sections[0].body_md = "Body changed".into();
        assert_ne!(h1, page.content_hash(), "hash tracks content");
    }

    #[test]
    fn excerpt_is_bounded_and_contains_match() {
        let body = "x".repeat(50) + " needle " + &"y".repeat(500);
        let excerpt = excerpt_around(&body, "NEEDLE");
        assert!(excerpt.len() <= 170);
        assert!(excerpt.to_lowercase().contains("needle"));
    }
}
