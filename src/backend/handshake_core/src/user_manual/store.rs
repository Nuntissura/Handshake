//! MT-194 UserManualStorageModel: embedded SurrealDB store for UserManual pages,
//! sections, anchors, tool entries, feature entries, version metadata, and
//! legacy aliases (migration 0310). EventLedger receipts use the
//! `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` family.
//!
//! Authority law (spec 2.3.13.11 / 10.15.8): these rows ARE the UserManual.
//! The compiled-in seed corpus (`super::seed`) is the deterministic input;
//! rendered markdown/HTML are projections. All list reads are bounded.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::surreal::event_ledger::{prepare_event, LedgerWrite};
use crate::storage::surreal::{SurrealDatabase, SurrealStorageError};
use crate::storage::{Database, StorageError, StorageResult};

/// Bound for list/search reads (matches the knowledge API convention).
pub const LIST_CAP: i64 = 500;

// ---------------------------------------------------------------------------
// Row types.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UserManualSection {
    pub section_id: String,
    pub page_id: String,
    pub position: i32,
    pub section_kind: String,
    pub title: String,
    pub body_md: String,
    pub body_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UserManualAnchor {
    pub anchor_id: String,
    pub page_id: String,
    pub anchor_kind: String,
    pub anchor_value: String,
    /// Empty string when not an HTTP route anchor.
    pub http_method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UserManualFeatureEntry {
    pub feature_id: String,
    pub title: String,
    pub description: String,
    pub tool_ids: Vec<String>,
    pub origin: String,
    pub content_hash: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LegacyAliasRow {
    pub alias: String,
    pub alias_kind: String,
    pub canonical_kind: String,
    pub canonical_ref: String,
    pub deprecation_note: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

    /// The denormalized `body` JSONB mirror persisted on the page row.
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

/// Embedded-SurrealDB UserManual store. Thin: borrows the shared database.
pub struct UserManualStore<'a> {
    db: &'a SurrealDatabase,
}

// ---------------------------------------------------------------------------
// SurrealDB implementation.
// ---------------------------------------------------------------------------

static USER_MANUAL_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

fn manual_thing(table: &str, id: &str) -> RecordId {
    RecordId::new(table, id.to_owned())
}

fn manual_opt_thing(table: &str, id: Option<&str>) -> Option<RecordId> {
    id.map(|id| manual_thing(table, id))
}

fn manual_key(id: RecordId) -> StorageResult<String> {
    match id.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Serialization(
            "UserManual record link does not have a string key".to_owned(),
        )),
    }
}

fn manual_opt_key(id: Option<RecordId>) -> StorageResult<Option<String>> {
    id.map(manual_key).transpose()
}

fn manual_surreal_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

#[derive(SurrealValue)]
struct ManualPageRecord {
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
    ledger_event_id: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn manual_page(row: ManualPageRecord) -> StorageResult<UserManualPage> {
    Ok(UserManualPage {
        page_id: row.page_id,
        slug: row.slug,
        title: row.title,
        page_kind: row.page_kind,
        audience: row.audience,
        body: row.body,
        content_hash: row.content_hash,
        manual_version: row.manual_version,
        source_kind: row.source_kind,
        spec_anchors: row.spec_anchors,
        status: row.status,
        superseded_by_slug: row.superseded_by_slug,
        ledger_event_id: manual_opt_key(row.ledger_event_id)?,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct ManualSectionRecord {
    section_id: String,
    page_id: RecordId,
    position: i64,
    section_kind: String,
    title: String,
    body_md: String,
    body_json: Option<Value>,
}

fn manual_section(row: ManualSectionRecord) -> StorageResult<UserManualSection> {
    Ok(UserManualSection {
        section_id: row.section_id,
        page_id: manual_key(row.page_id)?,
        position: i32::try_from(row.position)
            .map_err(|_| StorageError::Serialization("manual position exceeds i32".to_owned()))?,
        section_kind: row.section_kind,
        title: row.title,
        body_md: row.body_md,
        body_json: row.body_json,
    })
}

#[derive(SurrealValue)]
struct ManualAnchorRecord {
    anchor_id: String,
    page_id: RecordId,
    anchor_kind: String,
    anchor_value: String,
    http_method: String,
}

fn manual_anchor(row: ManualAnchorRecord) -> StorageResult<UserManualAnchor> {
    Ok(UserManualAnchor {
        anchor_id: row.anchor_id,
        page_id: manual_key(row.page_id)?,
        anchor_kind: row.anchor_kind,
        anchor_value: row.anchor_value,
        http_method: row.http_method,
    })
}

#[derive(SurrealValue)]
struct ManualToolRecord {
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
}

fn manual_tool(row: ManualToolRecord) -> StorageResult<UserManualToolEntry> {
    Ok(UserManualToolEntry {
        tool_id: row.tool_id,
        page_id: manual_opt_key(row.page_id)?,
        name: row.name,
        status: row.status,
        ipc_channel: row.ipc_channel,
        tauri_command: row.tauri_command,
        cli_flag: row.cli_flag,
        http_route: row.http_route,
        http_method: row.http_method,
        description: row.description,
        expected_input: row.expected_input,
        expected_output: row.expected_output,
        schema_fields: row.schema_fields,
        common_errors: row.common_errors,
        recovery_steps: row.recovery_steps,
        origin: row.origin,
        content_hash: row.content_hash,
        manual_version: row.manual_version,
    })
}

#[derive(SurrealValue)]
struct ManualFeatureRecord {
    feature_id: String,
    title: String,
    description: String,
    tool_ids: Vec<String>,
    origin: String,
    content_hash: String,
    manual_version: String,
}

fn manual_feature(row: ManualFeatureRecord) -> UserManualFeatureEntry {
    UserManualFeatureEntry {
        feature_id: row.feature_id,
        title: row.title,
        description: row.description,
        tool_ids: row.tool_ids,
        origin: row.origin,
        content_hash: row.content_hash,
        manual_version: row.manual_version,
    }
}

#[derive(SurrealValue)]
struct ManualAliasRecord {
    alias: String,
    alias_kind: String,
    canonical_kind: String,
    canonical_ref: String,
    deprecation_note: String,
    manual_version: String,
}

fn manual_alias(row: ManualAliasRecord) -> LegacyAliasRow {
    LegacyAliasRow {
        alias: row.alias,
        alias_kind: row.alias_kind,
        canonical_kind: row.canonical_kind,
        canonical_ref: row.canonical_ref,
        deprecation_note: row.deprecation_note,
        manual_version: row.manual_version,
    }
}

#[derive(SurrealValue)]
struct ManualVersionRecord {
    manual_version: String,
    seeded_at: Datetime,
    seed_content_hash: String,
    page_count: i64,
    tool_count: i64,
    feature_count: i64,
    ledger_event_id: Option<RecordId>,
    note: String,
}

fn manual_version(row: ManualVersionRecord) -> StorageResult<UserManualVersionRow> {
    Ok(UserManualVersionRow {
        manual_version: row.manual_version,
        seeded_at: row.seeded_at.into_inner(),
        seed_content_hash: row.seed_content_hash,
        page_count: i32::try_from(row.page_count)
            .map_err(|_| StorageError::Serialization("manual page_count exceeds i32".to_owned()))?,
        tool_count: i32::try_from(row.tool_count)
            .map_err(|_| StorageError::Serialization("manual tool_count exceeds i32".to_owned()))?,
        feature_count: i32::try_from(row.feature_count).map_err(|_| {
            StorageError::Serialization("manual feature_count exceeds i32".to_owned())
        })?,
        ledger_event_id: manual_opt_key(row.ledger_event_id)?,
        note: row.note,
    })
}

impl<'a> UserManualStore<'a> {
    pub fn new(db: &'a SurrealDatabase) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &'a SurrealDatabase {
        self.db
    }

    async fn rows<R, B>(&self, statement: &'static str, bindings: B) -> StorageResult<Vec<R>>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
            .map_err(manual_surreal_error)
    }

    async fn rows_at<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
        index: usize,
    ) -> StorageResult<Vec<R>>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.db
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values_at(statement, bindings, index).await })
            })
            .await
            .map_err(manual_surreal_error)
    }

    fn manual_receipt_event(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
    ) -> StorageResult<NewKernelEvent> {
        NewKernelEvent::builder(
            format!("UM-{}", Uuid::now_v7()),
            format!("UMS-{}", Uuid::now_v7()),
            KernelEventType::KnowledgeUserManualEntryRecorded,
            KernelActor::System("user_manual".to_owned()),
        )
        .aggregate("user_manual_entry", subject)
        .idempotency_key(format!("UMR-{}", Uuid::now_v7()))
        .source_component("user_manual::store")
        .payload(json!({"action": action, "subject": subject, "detail": payload}))
        .build()
        .map_err(|_| StorageError::Validation("user manual receipt event invalid"))
    }

    pub async fn append_manual_receipt(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
    ) -> StorageResult<String> {
        Ok(self
            .db
            .append_kernel_event(self.manual_receipt_event(action, subject, payload)?)
            .await?
            .event_id)
    }

    pub async fn upsert_page(
        &self,
        page: &NewUserManualPage,
        manual_version: &str,
        status: &str,
    ) -> StorageResult<(String, bool)> {
        let _guard = USER_MANUAL_MUTATION_LOCK.lock().await;
        let content_hash = page.content_hash();
        let existing = self.get_page_by_slug(&page.slug).await?;
        if let Some((stored, sections, anchors)) = &existing {
            if stored.content_hash == content_hash
                && section_rows_match_seed(sections, &page.sections)
                && anchor_rows_match_seed(anchors, &page.anchors)
            {
                return Ok((stored.page_id.clone(), false));
            }
        }
        let page_id = existing
            .as_ref()
            .map(|(page, _, _)| page.page_id.clone())
            .unwrap_or_else(|| format!("UMP-{}", Uuid::now_v7()));
        let (_receipt, event) = prepare_event(self.manual_receipt_event(
            if existing.is_some() {
                "page_updated"
            } else {
                "page_seeded"
            },
            &page.slug,
            json!({"content_hash":content_hash,"manual_version":manual_version,"page_kind":page.page_kind}),
        )?)?;
        #[derive(SurrealValue)]
        struct SectionInput {
            section_id: String,
            page_id: RecordId,
            position: i64,
            section_kind: String,
            title: String,
            body_md: String,
            body_json: Option<Value>,
        }
        #[derive(SurrealValue)]
        struct AnchorInput {
            anchor_id: String,
            page_id: RecordId,
            anchor_kind: String,
            anchor_value: String,
            http_method: String,
        }
        #[derive(SurrealValue)]
        struct Bindings {
            page_id: String,
            page_record: RecordId,
            slug: String,
            title: String,
            page_kind: String,
            audience: String,
            body: Value,
            content_hash: String,
            manual_version: String,
            spec_anchors: Vec<String>,
            status: String,
            receipt: RecordId,
            event: LedgerWrite,
            sections: Vec<SectionInput>,
            anchors: Vec<AnchorInput>,
        }
        let page_record = manual_thing("user_manual_pages", &page_id);
        let sections = page
            .sections
            .iter()
            .enumerate()
            .map(|(position, section)| SectionInput {
                section_id: format!("UMS-{}", Uuid::now_v7()),
                page_id: page_record.clone(),
                position: position as i64,
                section_kind: section.section_kind.to_owned(),
                title: section.title.clone(),
                body_md: section.body_md.clone(),
                body_json: section.body_json.clone(),
            })
            .collect();
        let anchors = page
            .anchors
            .iter()
            .map(|anchor| AnchorInput {
                anchor_id: format!("UMA-{}", Uuid::now_v7()),
                page_id: page_record.clone(),
                anchor_kind: anchor.anchor_kind.to_owned(),
                anchor_value: anchor.anchor_value.clone(),
                http_method: anchor.http_method.to_owned(),
            })
            .collect();
        self.rows_at::<surrealdb::types::Value, _>(
            "BEGIN TRANSACTION; CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; UPSERT $page_record CONTENT { page_id: $page_id, slug: $slug, title: $title, page_kind: $page_kind, audience: $audience, body: $body, content_hash: $content_hash, manual_version: $manual_version, source_kind: 'builtin_seed', spec_anchors: $spec_anchors, status: $status, ledger_event_id: $receipt, created_at: IF created_at = NONE { time::now() } ELSE { created_at }, updated_at: time::now() }; DELETE user_manual_sections WHERE page_id = $page_record; DELETE user_manual_anchors WHERE page_id = $page_record; FOR $section IN $sections { CREATE type::record('user_manual_sections', $section.section_id) CONTENT $section; }; FOR $anchor IN $anchors { CREATE type::record('user_manual_anchors', $anchor.anchor_id) CONTENT $anchor; }; COMMIT TRANSACTION;",
            Bindings { page_id: page_id.clone(), page_record, slug: page.slug.clone(), title: page.title.clone(), page_kind: page.page_kind.to_owned(), audience: page.audience.to_owned(), body: page.body_json(), content_hash, manual_version: manual_version.to_owned(), spec_anchors: page.spec_anchors.clone(), status: status.to_owned(), receipt: event.record.clone(), event, sections, anchors },
            2,
        ).await?;
        Ok((page_id, true))
    }

    pub(crate) async fn page_child_rows_match_seed(
        &self,
        page_id: &str,
        page: &NewUserManualPage,
    ) -> StorageResult<bool> {
        Ok(
            section_rows_match_seed(&self.sections_for(page_id).await?, &page.sections)
                && anchor_rows_match_seed(&self.anchors_for(page_id).await?, &page.anchors),
        )
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
        #[derive(SurrealValue)]
        struct Bindings {
            slug: String,
        }
        let Some(row) = self
            .rows::<ManualPageRecord, _>(
                "SELECT * FROM user_manual_pages WHERE slug = $slug LIMIT 1;",
                Bindings {
                    slug: slug.to_owned(),
                },
            )
            .await?
            .into_iter()
            .next()
        else {
            return Ok(None);
        };
        let page = manual_page(row)?;
        let sections = self.sections_for(&page.page_id).await?;
        let anchors = self.anchors_for(&page.page_id).await?;
        Ok(Some((page, sections, anchors)))
    }

    pub async fn sections_for(&self, page_id: &str) -> StorageResult<Vec<UserManualSection>> {
        #[derive(SurrealValue)]
        struct Bindings {
            page: RecordId,
            limit: i64,
        }
        self.rows::<ManualSectionRecord, _>("SELECT * FROM user_manual_sections WHERE page_id = $page ORDER BY position ASC LIMIT $limit;", Bindings { page: manual_thing("user_manual_pages", page_id), limit: LIST_CAP }).await?.into_iter().map(manual_section).collect()
    }

    pub async fn anchors_for(&self, page_id: &str) -> StorageResult<Vec<UserManualAnchor>> {
        #[derive(SurrealValue)]
        struct Bindings {
            page: RecordId,
            limit: i64,
        }
        self.rows::<ManualAnchorRecord, _>("SELECT * FROM user_manual_anchors WHERE page_id = $page ORDER BY anchor_kind, anchor_value LIMIT $limit;", Bindings { page: manual_thing("user_manual_pages", page_id), limit: LIST_CAP }).await?.into_iter().map(manual_anchor).collect()
    }

    pub async fn list_pages(
        &self,
        page_kind: Option<&str>,
        audience: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<UserManualPage>> {
        #[derive(SurrealValue)]
        struct Bindings {
            page_kind: Option<String>,
            audience: Option<String>,
            limit: i64,
        }
        self.rows::<ManualPageRecord, _>("SELECT * FROM user_manual_pages WHERE ($page_kind = NONE OR page_kind = $page_kind) AND ($audience = NONE OR audience = $audience) ORDER BY slug ASC LIMIT $limit;", Bindings { page_kind: page_kind.map(str::to_owned), audience: audience.map(str::to_owned), limit: limit.clamp(1, LIST_CAP) }).await?.into_iter().map(manual_page).collect()
    }

    pub async fn anchors_by_kind(&self, anchor_kind: &str) -> StorageResult<Vec<UserManualAnchor>> {
        #[derive(SurrealValue)]
        struct Bindings {
            kind: String,
            limit: i64,
        }
        self.rows::<ManualAnchorRecord, _>("SELECT * FROM user_manual_anchors WHERE anchor_kind = $kind ORDER BY anchor_value LIMIT $limit;", Bindings { kind: anchor_kind.to_owned(), limit: LIST_CAP }).await?.into_iter().map(manual_anchor).collect()
    }

    pub async fn page_links(
        &self,
        slug: &str,
    ) -> StorageResult<Option<(Vec<String>, Vec<String>)>> {
        let Some((page, _, _)) = self.get_page_by_slug(slug).await? else {
            return Ok(None);
        };
        let outbound = self
            .anchors_for(&page.page_id)
            .await?
            .into_iter()
            .filter(|anchor| anchor.anchor_kind == "page_link")
            .map(|anchor| anchor.anchor_value)
            .collect();
        let pages = self.list_pages(None, None, LIST_CAP).await?;
        let mut inbound = Vec::new();
        for candidate in pages {
            if self
                .anchors_for(&candidate.page_id)
                .await?
                .iter()
                .any(|anchor| anchor.anchor_kind == "page_link" && anchor.anchor_value == slug)
            {
                inbound.push(candidate.slug);
            }
        }
        inbound.sort();
        Ok(Some((outbound, inbound)))
    }

    pub async fn search(&self, query: &str, limit: i64) -> StorageResult<Vec<ManualSearchHit>> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.clamp(1, LIST_CAP) as usize;
        let mut hits = Vec::new();
        for page in self.list_pages(None, None, LIST_CAP).await? {
            if page.slug.to_lowercase().contains(&needle)
                || page.title.to_lowercase().contains(&needle)
            {
                hits.push(ManualSearchHit {
                    result_kind: "page".to_owned(),
                    result_ref: page.slug.clone(),
                    page_slug: Some(page.slug.clone()),
                    title: page.title,
                    excerpt: String::new(),
                });
            }
            for section in self.sections_for(&page.page_id).await? {
                if section.title.to_lowercase().contains(&needle)
                    || section.body_md.to_lowercase().contains(&needle)
                {
                    hits.push(ManualSearchHit {
                        result_kind: "section".to_owned(),
                        result_ref: page.slug.clone(),
                        page_slug: Some(page.slug.clone()),
                        title: section.title,
                        excerpt: excerpt_around(&section.body_md, query),
                    });
                }
            }
        }
        for tool in self.list_tool_entries(None, None, LIST_CAP).await? {
            if tool.tool_id.to_lowercase().contains(&needle)
                || tool.name.to_lowercase().contains(&needle)
                || tool.description.to_lowercase().contains(&needle)
                || tool
                    .http_route
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle)
            {
                hits.push(ManualSearchHit {
                    result_kind: "tool".to_owned(),
                    result_ref: tool.tool_id,
                    page_slug: None,
                    title: tool.name,
                    excerpt: tool.description,
                });
            }
        }
        hits.truncate(limit);
        Ok(hits)
    }

    pub async fn upsert_tool_entry(&self, entry: &UserManualToolEntry) -> StorageResult<bool> {
        if self.get_tool_entry(&entry.tool_id).await?.as_ref() == Some(entry) {
            return Ok(false);
        }
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            page: Option<RecordId>,
            name: String,
            status: String,
            ipc: Option<String>,
            tauri: Option<String>,
            cli: Option<String>,
            route: Option<String>,
            method: String,
            description: String,
            input: String,
            output: String,
            fields: Vec<String>,
            errors: Vec<String>,
            recovery: Vec<String>,
            origin: String,
            hash: String,
            version: String,
        }
        self.rows::<surrealdb::types::Value, _>("UPSERT type::record('user_manual_tool_entries', $id) CONTENT { tool_id: $id, page_id: $page, name: $name, status: $status, ipc_channel: $ipc, tauri_command: $tauri, cli_flag: $cli, http_route: $route, http_method: $method, description: $description, expected_input: $input, expected_output: $output, schema_fields: $fields, common_errors: $errors, recovery_steps: $recovery, origin: $origin, content_hash: $hash, manual_version: $version, updated_at: time::now() };", Bindings { id: entry.tool_id.clone(), page: manual_opt_thing("user_manual_pages", entry.page_id.as_deref()), name: entry.name.clone(), status: entry.status.clone(), ipc: entry.ipc_channel.clone(), tauri: entry.tauri_command.clone(), cli: entry.cli_flag.clone(), route: entry.http_route.clone(), method: entry.http_method.clone(), description: entry.description.clone(), input: entry.expected_input.clone(), output: entry.expected_output.clone(), fields: entry.schema_fields.clone(), errors: entry.common_errors.clone(), recovery: entry.recovery_steps.clone(), origin: entry.origin.clone(), hash: entry.content_hash.clone(), version: entry.manual_version.clone() }).await?;
        Ok(true)
    }

    pub async fn get_tool_entry(
        &self,
        tool_id: &str,
    ) -> StorageResult<Option<UserManualToolEntry>> {
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
        }
        self.rows::<ManualToolRecord, _>(
            "SELECT * FROM user_manual_tool_entries WHERE tool_id = $id LIMIT 1;",
            Bindings {
                id: tool_id.to_owned(),
            },
        )
        .await?
        .into_iter()
        .next()
        .map(manual_tool)
        .transpose()
    }
    pub async fn list_tool_entries(
        &self,
        status: Option<&str>,
        origin: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<UserManualToolEntry>> {
        #[derive(SurrealValue)]
        struct Bindings {
            status: Option<String>,
            origin: Option<String>,
            limit: i64,
        }
        self.rows::<ManualToolRecord, _>("SELECT * FROM user_manual_tool_entries WHERE ($status = NONE OR status = $status) AND ($origin = NONE OR origin = $origin) ORDER BY tool_id LIMIT $limit;", Bindings { status: status.map(str::to_owned), origin: origin.map(str::to_owned), limit: limit.clamp(1, LIST_CAP) }).await?.into_iter().map(manual_tool).collect()
    }

    pub async fn upsert_feature_entry(
        &self,
        entry: &UserManualFeatureEntry,
    ) -> StorageResult<bool> {
        if self.get_feature_entry(&entry.feature_id).await?.as_ref() == Some(entry) {
            return Ok(false);
        }
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
            title: String,
            description: String,
            tools: Vec<String>,
            origin: String,
            hash: String,
            version: String,
        }
        self.rows::<surrealdb::types::Value, _>("UPSERT type::record('user_manual_feature_entries', $id) CONTENT { feature_id: $id, title: $title, description: $description, tool_ids: $tools, origin: $origin, content_hash: $hash, manual_version: $version, updated_at: time::now() };", Bindings { id: entry.feature_id.clone(), title: entry.title.clone(), description: entry.description.clone(), tools: entry.tool_ids.clone(), origin: entry.origin.clone(), hash: entry.content_hash.clone(), version: entry.manual_version.clone() }).await?;
        Ok(true)
    }
    pub async fn get_feature_entry(
        &self,
        feature_id: &str,
    ) -> StorageResult<Option<UserManualFeatureEntry>> {
        #[derive(SurrealValue)]
        struct Bindings {
            id: String,
        }
        Ok(self
            .rows::<ManualFeatureRecord, _>(
                "SELECT * FROM user_manual_feature_entries WHERE feature_id = $id LIMIT 1;",
                Bindings {
                    id: feature_id.to_owned(),
                },
            )
            .await?
            .into_iter()
            .next()
            .map(manual_feature))
    }
    pub async fn list_feature_entries(
        &self,
        limit: i64,
    ) -> StorageResult<Vec<UserManualFeatureEntry>> {
        #[derive(SurrealValue)]
        struct Bindings {
            limit: i64,
        }
        Ok(self
            .rows::<ManualFeatureRecord, _>(
                "SELECT * FROM user_manual_feature_entries ORDER BY feature_id LIMIT $limit;",
                Bindings {
                    limit: limit.clamp(1, LIST_CAP),
                },
            )
            .await?
            .into_iter()
            .map(manual_feature)
            .collect())
    }

    pub async fn upsert_legacy_alias(&self, alias: &LegacyAliasRow) -> StorageResult<bool> {
        if self.get_legacy_alias(&alias.alias).await?.as_ref() == Some(alias) {
            return Ok(false);
        }
        #[derive(SurrealValue)]
        struct Bindings {
            alias: String,
            alias_kind: String,
            canonical_kind: String,
            canonical_ref: String,
            note: String,
            version: String,
        }
        self.rows::<surrealdb::types::Value, _>("UPSERT type::record('user_manual_legacy_aliases', $alias) CONTENT { alias: $alias, alias_kind: $alias_kind, canonical_kind: $canonical_kind, canonical_ref: $canonical_ref, deprecation_note: $note, manual_version: $version, updated_at: time::now() };", Bindings { alias: alias.alias.clone(), alias_kind: alias.alias_kind.clone(), canonical_kind: alias.canonical_kind.clone(), canonical_ref: alias.canonical_ref.clone(), note: alias.deprecation_note.clone(), version: alias.manual_version.clone() }).await?;
        Ok(true)
    }
    pub async fn get_legacy_alias(&self, alias: &str) -> StorageResult<Option<LegacyAliasRow>> {
        #[derive(SurrealValue)]
        struct Bindings {
            alias: String,
        }
        Ok(self
            .rows::<ManualAliasRecord, _>(
                "SELECT * FROM user_manual_legacy_aliases WHERE alias = $alias LIMIT 1;",
                Bindings {
                    alias: alias.to_owned(),
                },
            )
            .await?
            .into_iter()
            .next()
            .map(manual_alias))
    }
    pub async fn list_legacy_aliases(&self) -> StorageResult<Vec<LegacyAliasRow>> {
        #[derive(SurrealValue)]
        struct Bindings {
            limit: i64,
        }
        Ok(self
            .rows::<ManualAliasRecord, _>(
                "SELECT * FROM user_manual_legacy_aliases ORDER BY alias LIMIT $limit;",
                Bindings { limit: LIST_CAP },
            )
            .await?
            .into_iter()
            .map(manual_alias)
            .collect())
    }

    pub async fn record_version(
        &self,
        manual_version_value: &str,
        seed_content_hash: &str,
        page_count: i32,
        tool_count: i32,
        feature_count: i32,
        ledger_event_id: Option<&str>,
        note: &str,
    ) -> StorageResult<()> {
        #[derive(SurrealValue)]
        struct Bindings {
            version: String,
            hash: String,
            pages: i32,
            tools: i32,
            features: i32,
            event: Option<RecordId>,
            note: String,
        }
        self.rows::<surrealdb::types::Value, _>("UPSERT type::record('user_manual_versions', $version) CONTENT { manual_version: $version, seeded_at: time::now(), seed_content_hash: $hash, page_count: $pages, tool_count: $tools, feature_count: $features, ledger_event_id: $event, note: $note };", Bindings { version: manual_version_value.to_owned(), hash: seed_content_hash.to_owned(), pages: page_count, tools: tool_count, features: feature_count, event: manual_opt_thing("kernel_event_ledger", ledger_event_id), note: note.to_owned() }).await?;
        Ok(())
    }
    pub async fn record_version_with_receipt(
        &self,
        manual_version_value: &str,
        seed_content_hash: &str,
        page_count: i32,
        tool_count: i32,
        feature_count: i32,
        receipt_payload: Value,
        note: &str,
    ) -> StorageResult<String> {
        let (receipt, event) = prepare_event(self.manual_receipt_event(
            "corpus_seeded",
            manual_version_value,
            receipt_payload,
        )?)?;
        #[derive(SurrealValue)]
        struct Bindings {
            version: String,
            hash: String,
            pages: i32,
            tools: i32,
            features: i32,
            receipt: RecordId,
            note: String,
            event: LedgerWrite,
        }
        self.rows_at::<surrealdb::types::Value, _>(
            "BEGIN TRANSACTION; CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; UPSERT type::record('user_manual_versions', $version) CONTENT { manual_version: $version, seeded_at: time::now(), seed_content_hash: $hash, page_count: $pages, tool_count: $tools, feature_count: $features, ledger_event_id: $receipt, note: $note }; COMMIT TRANSACTION;",
            Bindings { version: manual_version_value.to_owned(), hash: seed_content_hash.to_owned(), pages: page_count, tools: tool_count, features: feature_count, receipt: event.record.clone(), note: note.to_owned(), event },
            2,
        ).await?;
        Ok(receipt.event_id)
    }
    pub async fn get_version(
        &self,
        manual_version_value: &str,
    ) -> StorageResult<Option<UserManualVersionRow>> {
        #[derive(SurrealValue)]
        struct Bindings {
            version: String,
        }
        self.rows::<ManualVersionRecord, _>(
            "SELECT * FROM user_manual_versions WHERE manual_version = $version LIMIT 1;",
            Bindings {
                version: manual_version_value.to_owned(),
            },
        )
        .await?
        .into_iter()
        .next()
        .map(manual_version)
        .transpose()
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
