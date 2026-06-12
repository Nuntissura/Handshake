//! MT-194 UserManualStorageModel: PostgreSQL store for UserManual pages,
//! sections, anchors, tool entries, feature entries, version metadata, and
//! legacy aliases (migration 0310). EventLedger receipts use the
//! `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` family.
//!
//! Authority law (spec 2.3.13.11 / 10.15.8): these rows ARE the UserManual.
//! The compiled-in seed corpus (`super::seed`) is the deterministic input;
//! rendered markdown/HTML are projections. All list reads are bounded.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::postgres::PostgresDatabase;
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

// ---------------------------------------------------------------------------
// Store.
// ---------------------------------------------------------------------------

/// PostgreSQL-backed UserManual store. Thin: borrows the shared
/// [`PostgresDatabase`] (same pool the API layer holds), owns no state.
pub struct UserManualStore<'a> {
    db: &'a PostgresDatabase,
}

impl<'a> UserManualStore<'a> {
    pub fn new(db: &'a PostgresDatabase) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &'a PostgresDatabase {
        self.db
    }

    /// Append a `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` EventLedger receipt for
    /// a manual mutation. Returns the event id for the row's
    /// `ledger_event_id`.
    pub async fn append_manual_receipt(
        &self,
        action: &str,
        subject: &str,
        payload: Value,
    ) -> StorageResult<String> {
        let event = NewKernelEvent::builder(
            format!("UM-{}", Uuid::now_v7()),
            format!("UMS-{}", Uuid::now_v7()),
            KernelEventType::KnowledgeUserManualEntryRecorded,
            KernelActor::System("user_manual".to_string()),
        )
        .aggregate("user_manual_entry", subject)
        .idempotency_key(format!("UMR-{}", Uuid::now_v7()))
        .source_component("user_manual::store")
        .payload(json!({
            "action": action,
            "subject": subject,
            "detail": payload,
        }))
        .build()
        .map_err(|_| StorageError::Validation("user manual receipt event invalid"))?;
        let recorded = self.db.append_kernel_event(event).await?;
        Ok(recorded.event_id)
    }

    // -- pages ---------------------------------------------------------------

    /// Idempotent page upsert keyed on `slug`. Returns `(page_id, changed)`:
    /// `changed == false` means the stored row already matches the seed —
    /// content hash AND child-row counts (a tampered/partially-deleted
    /// section or anchor set is NOT current even when the page hash matches,
    /// so resync heals it). On change, sections and anchors are replaced
    /// transactionally and a receipt is appended. The page row write is a
    /// single `ON CONFLICT (slug)` upsert (stable `page_id`), so concurrent
    /// seeders (startup + test, parallel lanes) cannot race an INSERT.
    pub async fn upsert_page(
        &self,
        page: &NewUserManualPage,
        manual_version: &str,
        status: &str,
    ) -> StorageResult<(String, bool)> {
        let content_hash = page.content_hash();
        let existing: Option<(String, String)> = sqlx::query(
            "SELECT page_id, content_hash FROM user_manual_pages WHERE slug = $1",
        )
        .bind(&page.slug)
        .fetch_optional(self.db.pool())
        .await?
        .map(|row| (row.get("page_id"), row.get("content_hash")));

        if let Some((page_id, stored_hash)) = &existing {
            if stored_hash == &content_hash {
                let (section_count, anchor_count): (i64, i64) = {
                    let row = sqlx::query(
                        r#"
                        SELECT
                          (SELECT COUNT(*) FROM user_manual_sections WHERE page_id = $1) AS sections,
                          (SELECT COUNT(*) FROM user_manual_anchors WHERE page_id = $1) AS anchors
                        "#,
                    )
                    .bind(page_id)
                    .fetch_one(self.db.pool())
                    .await?;
                    (row.get("sections"), row.get("anchors"))
                };
                // Anchors dedupe on (kind, value, method); compare against the
                // deduped expectation.
                let expected_anchors = page
                    .anchors
                    .iter()
                    .map(|a| (a.anchor_kind, a.anchor_value.as_str(), a.http_method))
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
                if section_count as usize == page.sections.len()
                    && anchor_count as usize == expected_anchors
                {
                    return Ok((page_id.clone(), false));
                }
            }
        }

        let receipt_id = self
            .append_manual_receipt(
                if existing.is_some() { "page_updated" } else { "page_seeded" },
                &page.slug,
                json!({
                    "content_hash": content_hash,
                    "manual_version": manual_version,
                    "page_kind": page.page_kind,
                }),
            )
            .await?;

        let mut tx = self.db.pool().begin().await?;
        let page_id: String = sqlx::query(
            r#"
            INSERT INTO user_manual_pages (
                page_id, slug, title, page_kind, audience, body,
                content_hash, manual_version, source_kind, spec_anchors,
                status, ledger_event_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'builtin_seed', $9, $10, $11)
            ON CONFLICT (slug) DO UPDATE SET
                title = EXCLUDED.title,
                page_kind = EXCLUDED.page_kind,
                audience = EXCLUDED.audience,
                body = EXCLUDED.body,
                content_hash = EXCLUDED.content_hash,
                manual_version = EXCLUDED.manual_version,
                spec_anchors = EXCLUDED.spec_anchors,
                status = EXCLUDED.status,
                ledger_event_id = EXCLUDED.ledger_event_id,
                updated_at = NOW()
            RETURNING page_id
            "#,
        )
        .bind(format!("UMP-{}", Uuid::now_v7()))
        .bind(&page.slug)
        .bind(&page.title)
        .bind(page.page_kind)
        .bind(page.audience)
        .bind(page.body_json())
        .bind(&content_hash)
        .bind(manual_version)
        .bind(json!(page.spec_anchors))
        .bind(status)
        .bind(&receipt_id)
        .fetch_one(&mut *tx)
        .await?
        .get("page_id");
        sqlx::query("DELETE FROM user_manual_sections WHERE page_id = $1")
            .bind(&page_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM user_manual_anchors WHERE page_id = $1")
            .bind(&page_id)
            .execute(&mut *tx)
            .await?;

        for (position, section) in page.sections.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO user_manual_sections (
                    section_id, page_id, position, section_kind, title,
                    body_md, body_json
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(format!("UMS-{}", Uuid::now_v7()))
            .bind(&page_id)
            .bind(position as i32)
            .bind(section.section_kind)
            .bind(&section.title)
            .bind(&section.body_md)
            .bind(&section.body_json)
            .execute(&mut *tx)
            .await?;
        }
        for anchor in &page.anchors {
            sqlx::query(
                r#"
                INSERT INTO user_manual_anchors (
                    anchor_id, page_id, anchor_kind, anchor_value, http_method
                ) VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (page_id, anchor_kind, anchor_value, http_method)
                DO NOTHING
                "#,
            )
            .bind(format!("UMA-{}", Uuid::now_v7()))
            .bind(&page_id)
            .bind(anchor.anchor_kind)
            .bind(&anchor.anchor_value)
            .bind(anchor.http_method)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok((page_id, true))
    }

    pub async fn get_page_by_slug(
        &self,
        slug: &str,
    ) -> StorageResult<Option<(UserManualPage, Vec<UserManualSection>, Vec<UserManualAnchor>)>>
    {
        let Some(row) = sqlx::query(
            r#"
            SELECT page_id, slug, title, page_kind, audience, body, content_hash,
                   manual_version, source_kind, spec_anchors, status,
                   superseded_by_slug, ledger_event_id, created_at, updated_at
            FROM user_manual_pages WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(self.db.pool())
        .await?
        else {
            return Ok(None);
        };
        let page = page_from_row(&row)?;
        let sections = self.sections_for(&page.page_id).await?;
        let anchors = self.anchors_for(&page.page_id).await?;
        Ok(Some((page, sections, anchors)))
    }

    pub async fn sections_for(&self, page_id: &str) -> StorageResult<Vec<UserManualSection>> {
        let rows = sqlx::query(
            r#"
            SELECT section_id, page_id, position, section_kind, title, body_md, body_json
            FROM user_manual_sections WHERE page_id = $1
            ORDER BY position ASC LIMIT $2
            "#,
        )
        .bind(page_id)
        .bind(LIST_CAP)
        .fetch_all(self.db.pool())
        .await?;
        rows.iter()
            .map(|row| {
                Ok(UserManualSection {
                    section_id: row.get("section_id"),
                    page_id: row.get("page_id"),
                    position: row.get("position"),
                    section_kind: row.get("section_kind"),
                    title: row.get("title"),
                    body_md: row.get("body_md"),
                    body_json: row.get("body_json"),
                })
            })
            .collect()
    }

    pub async fn anchors_for(&self, page_id: &str) -> StorageResult<Vec<UserManualAnchor>> {
        let rows = sqlx::query(
            r#"
            SELECT anchor_id, page_id, anchor_kind, anchor_value, http_method
            FROM user_manual_anchors WHERE page_id = $1
            ORDER BY anchor_kind, anchor_value LIMIT $2
            "#,
        )
        .bind(page_id)
        .bind(LIST_CAP)
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.iter().map(anchor_from_row).collect())
    }

    pub async fn list_pages(
        &self,
        page_kind: Option<&str>,
        audience: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<UserManualPage>> {
        let limit = limit.clamp(1, LIST_CAP);
        let rows = sqlx::query(
            r#"
            SELECT page_id, slug, title, page_kind, audience, body, content_hash,
                   manual_version, source_kind, spec_anchors, status,
                   superseded_by_slug, ledger_event_id, created_at, updated_at
            FROM user_manual_pages
            WHERE ($1::text IS NULL OR page_kind = $1)
              AND ($2::text IS NULL OR audience = $2)
            ORDER BY slug ASC
            LIMIT $3
            "#,
        )
        .bind(page_kind)
        .bind(audience)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        rows.iter().map(page_from_row).collect()
    }

    /// All anchors of a kind across pages (the MT-195 coverage gate and the
    /// MT-204 freshness check run over these).
    pub async fn anchors_by_kind(&self, anchor_kind: &str) -> StorageResult<Vec<UserManualAnchor>> {
        let rows = sqlx::query(
            r#"
            SELECT anchor_id, page_id, anchor_kind, anchor_value, http_method
            FROM user_manual_anchors WHERE anchor_kind = $1
            ORDER BY anchor_value LIMIT $2
            "#,
        )
        .bind(anchor_kind)
        .bind(LIST_CAP)
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.iter().map(anchor_from_row).collect())
    }

    /// MT-201 linking: pages this page links to (`page_link` anchors out) and
    /// pages that link to this page (in), resolved through slugs.
    pub async fn page_links(
        &self,
        slug: &str,
    ) -> StorageResult<Option<(Vec<String>, Vec<String>)>> {
        let Some(row) = sqlx::query("SELECT page_id FROM user_manual_pages WHERE slug = $1")
            .bind(slug)
            .fetch_optional(self.db.pool())
            .await?
        else {
            return Ok(None);
        };
        let page_id: String = row.get("page_id");
        let outbound: Vec<String> = sqlx::query(
            r#"
            SELECT anchor_value FROM user_manual_anchors
            WHERE page_id = $1 AND anchor_kind = 'page_link'
            ORDER BY anchor_value LIMIT $2
            "#,
        )
        .bind(&page_id)
        .bind(LIST_CAP)
        .fetch_all(self.db.pool())
        .await?
        .iter()
        .map(|r| r.get("anchor_value"))
        .collect();
        let inbound: Vec<String> = sqlx::query(
            r#"
            SELECT p.slug FROM user_manual_anchors a
            JOIN user_manual_pages p ON p.page_id = a.page_id
            WHERE a.anchor_kind = 'page_link' AND a.anchor_value = $1
            ORDER BY p.slug LIMIT $2
            "#,
        )
        .bind(slug)
        .bind(LIST_CAP)
        .fetch_all(self.db.pool())
        .await?
        .iter()
        .map(|r| r.get("slug"))
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
        // Escape LIKE wildcards so a query is literal text, never a pattern.
        let escaped = trimmed
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let mut hits = Vec::new();

        let page_rows = sqlx::query(
            r#"
            SELECT slug, title FROM user_manual_pages
            WHERE title ILIKE $1 OR slug ILIKE $1
            ORDER BY slug LIMIT $2
            "#,
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        for row in &page_rows {
            let slug: String = row.get("slug");
            hits.push(ManualSearchHit {
                result_kind: "page".into(),
                result_ref: slug.clone(),
                page_slug: Some(slug),
                title: row.get("title"),
                excerpt: String::new(),
            });
        }

        let section_rows = sqlx::query(
            r#"
            SELECT p.slug AS page_slug, s.title, s.body_md
            FROM user_manual_sections s
            JOIN user_manual_pages p ON p.page_id = s.page_id
            WHERE s.title ILIKE $1 OR s.body_md ILIKE $1
            ORDER BY p.slug, s.position LIMIT $2
            "#,
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        for row in &section_rows {
            let body: String = row.get("body_md");
            hits.push(ManualSearchHit {
                result_kind: "section".into(),
                result_ref: row.get("page_slug"),
                page_slug: Some(row.get("page_slug")),
                title: row.get("title"),
                excerpt: excerpt_around(&body, trimmed),
            });
        }

        let tool_rows = sqlx::query(
            r#"
            SELECT tool_id, name, description FROM user_manual_tool_entries
            WHERE tool_id ILIKE $1 OR name ILIKE $1 OR description ILIKE $1
               OR http_route ILIKE $1
            ORDER BY tool_id LIMIT $2
            "#,
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        for row in &tool_rows {
            hits.push(ManualSearchHit {
                result_kind: "tool".into(),
                result_ref: row.get("tool_id"),
                page_slug: None,
                title: row.get("name"),
                excerpt: row.get("description"),
            });
        }

        hits.truncate(limit as usize);
        Ok(hits)
    }

    // -- tool entries ----------------------------------------------------------

    pub async fn upsert_tool_entry(&self, entry: &UserManualToolEntry) -> StorageResult<bool> {
        let stored: Option<String> = sqlx::query(
            "SELECT content_hash FROM user_manual_tool_entries WHERE tool_id = $1",
        )
        .bind(&entry.tool_id)
        .fetch_optional(self.db.pool())
        .await?
        .map(|row| row.get("content_hash"));
        if stored.as_deref() == Some(entry.content_hash.as_str()) {
            return Ok(false);
        }
        sqlx::query(
            r#"
            INSERT INTO user_manual_tool_entries (
                tool_id, page_id, name, status, ipc_channel, tauri_command,
                cli_flag, http_route, http_method, description, expected_input,
                expected_output, schema_fields, common_errors, recovery_steps,
                origin, content_hash, manual_version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
            ON CONFLICT (tool_id) DO UPDATE SET
                page_id = EXCLUDED.page_id,
                name = EXCLUDED.name,
                status = EXCLUDED.status,
                ipc_channel = EXCLUDED.ipc_channel,
                tauri_command = EXCLUDED.tauri_command,
                cli_flag = EXCLUDED.cli_flag,
                http_route = EXCLUDED.http_route,
                http_method = EXCLUDED.http_method,
                description = EXCLUDED.description,
                expected_input = EXCLUDED.expected_input,
                expected_output = EXCLUDED.expected_output,
                schema_fields = EXCLUDED.schema_fields,
                common_errors = EXCLUDED.common_errors,
                recovery_steps = EXCLUDED.recovery_steps,
                origin = EXCLUDED.origin,
                content_hash = EXCLUDED.content_hash,
                manual_version = EXCLUDED.manual_version,
                updated_at = NOW()
            "#,
        )
        .bind(&entry.tool_id)
        .bind(&entry.page_id)
        .bind(&entry.name)
        .bind(&entry.status)
        .bind(&entry.ipc_channel)
        .bind(&entry.tauri_command)
        .bind(&entry.cli_flag)
        .bind(&entry.http_route)
        .bind(&entry.http_method)
        .bind(&entry.description)
        .bind(&entry.expected_input)
        .bind(&entry.expected_output)
        .bind(json!(entry.schema_fields))
        .bind(json!(entry.common_errors))
        .bind(json!(entry.recovery_steps))
        .bind(&entry.origin)
        .bind(&entry.content_hash)
        .bind(&entry.manual_version)
        .execute(self.db.pool())
        .await?;
        Ok(true)
    }

    pub async fn get_tool_entry(&self, tool_id: &str) -> StorageResult<Option<UserManualToolEntry>> {
        let row = sqlx::query(
            r#"
            SELECT tool_id, page_id, name, status, ipc_channel, tauri_command,
                   cli_flag, http_route, http_method, description, expected_input,
                   expected_output, schema_fields, common_errors, recovery_steps,
                   origin, content_hash, manual_version
            FROM user_manual_tool_entries WHERE tool_id = $1
            "#,
        )
        .bind(tool_id)
        .fetch_optional(self.db.pool())
        .await?;
        row.as_ref().map(tool_from_row).transpose()
    }

    pub async fn list_tool_entries(
        &self,
        status: Option<&str>,
        origin: Option<&str>,
        limit: i64,
    ) -> StorageResult<Vec<UserManualToolEntry>> {
        let limit = limit.clamp(1, LIST_CAP);
        let rows = sqlx::query(
            r#"
            SELECT tool_id, page_id, name, status, ipc_channel, tauri_command,
                   cli_flag, http_route, http_method, description, expected_input,
                   expected_output, schema_fields, common_errors, recovery_steps,
                   origin, content_hash, manual_version
            FROM user_manual_tool_entries
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR origin = $2)
            ORDER BY tool_id LIMIT $3
            "#,
        )
        .bind(status)
        .bind(origin)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        rows.iter().map(tool_from_row).collect()
    }

    // -- feature entries --------------------------------------------------------

    pub async fn upsert_feature_entry(
        &self,
        entry: &UserManualFeatureEntry,
    ) -> StorageResult<bool> {
        let stored: Option<String> = sqlx::query(
            "SELECT content_hash FROM user_manual_feature_entries WHERE feature_id = $1",
        )
        .bind(&entry.feature_id)
        .fetch_optional(self.db.pool())
        .await?
        .map(|row| row.get("content_hash"));
        if stored.as_deref() == Some(entry.content_hash.as_str()) {
            return Ok(false);
        }
        sqlx::query(
            r#"
            INSERT INTO user_manual_feature_entries (
                feature_id, title, description, tool_ids, origin,
                content_hash, manual_version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (feature_id) DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                tool_ids = EXCLUDED.tool_ids,
                origin = EXCLUDED.origin,
                content_hash = EXCLUDED.content_hash,
                manual_version = EXCLUDED.manual_version,
                updated_at = NOW()
            "#,
        )
        .bind(&entry.feature_id)
        .bind(&entry.title)
        .bind(&entry.description)
        .bind(json!(entry.tool_ids))
        .bind(&entry.origin)
        .bind(&entry.content_hash)
        .bind(&entry.manual_version)
        .execute(self.db.pool())
        .await?;
        Ok(true)
    }

    pub async fn list_feature_entries(
        &self,
        limit: i64,
    ) -> StorageResult<Vec<UserManualFeatureEntry>> {
        let limit = limit.clamp(1, LIST_CAP);
        let rows = sqlx::query(
            r#"
            SELECT feature_id, title, description, tool_ids, origin,
                   content_hash, manual_version
            FROM user_manual_feature_entries ORDER BY feature_id LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;
        rows.iter()
            .map(|row| {
                Ok(UserManualFeatureEntry {
                    feature_id: row.get("feature_id"),
                    title: row.get("title"),
                    description: row.get("description"),
                    tool_ids: string_vec(row.get("tool_ids"))?,
                    origin: row.get("origin"),
                    content_hash: row.get("content_hash"),
                    manual_version: row.get("manual_version"),
                })
            })
            .collect()
    }

    // -- legacy aliases -----------------------------------------------------------

    pub async fn upsert_legacy_alias(&self, alias: &LegacyAliasRow) -> StorageResult<()> {
        sqlx::query(
            r#"
            INSERT INTO user_manual_legacy_aliases (
                alias, alias_kind, canonical_kind, canonical_ref,
                deprecation_note, manual_version
            ) VALUES ($1,$2,$3,$4,$5,$6)
            ON CONFLICT (alias) DO UPDATE SET
                alias_kind = EXCLUDED.alias_kind,
                canonical_kind = EXCLUDED.canonical_kind,
                canonical_ref = EXCLUDED.canonical_ref,
                deprecation_note = EXCLUDED.deprecation_note,
                manual_version = EXCLUDED.manual_version,
                updated_at = NOW()
            "#,
        )
        .bind(&alias.alias)
        .bind(&alias.alias_kind)
        .bind(&alias.canonical_kind)
        .bind(&alias.canonical_ref)
        .bind(&alias.deprecation_note)
        .bind(&alias.manual_version)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn get_legacy_alias(&self, alias: &str) -> StorageResult<Option<LegacyAliasRow>> {
        let row = sqlx::query(
            r#"
            SELECT alias, alias_kind, canonical_kind, canonical_ref,
                   deprecation_note, manual_version
            FROM user_manual_legacy_aliases WHERE alias = $1
            "#,
        )
        .bind(alias)
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(|row| LegacyAliasRow {
            alias: row.get("alias"),
            alias_kind: row.get("alias_kind"),
            canonical_kind: row.get("canonical_kind"),
            canonical_ref: row.get("canonical_ref"),
            deprecation_note: row.get("deprecation_note"),
            manual_version: row.get("manual_version"),
        }))
    }

    pub async fn list_legacy_aliases(&self) -> StorageResult<Vec<LegacyAliasRow>> {
        let rows = sqlx::query(
            r#"
            SELECT alias, alias_kind, canonical_kind, canonical_ref,
                   deprecation_note, manual_version
            FROM user_manual_legacy_aliases ORDER BY alias LIMIT $1
            "#,
        )
        .bind(LIST_CAP)
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|row| LegacyAliasRow {
                alias: row.get("alias"),
                alias_kind: row.get("alias_kind"),
                canonical_kind: row.get("canonical_kind"),
                canonical_ref: row.get("canonical_ref"),
                deprecation_note: row.get("deprecation_note"),
                manual_version: row.get("manual_version"),
            })
            .collect())
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
        sqlx::query(
            r#"
            INSERT INTO user_manual_versions (
                manual_version, seed_content_hash, page_count, tool_count,
                feature_count, ledger_event_id, note
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (manual_version) DO UPDATE SET
                seed_content_hash = EXCLUDED.seed_content_hash,
                page_count = EXCLUDED.page_count,
                tool_count = EXCLUDED.tool_count,
                feature_count = EXCLUDED.feature_count,
                ledger_event_id = EXCLUDED.ledger_event_id,
                note = EXCLUDED.note,
                seeded_at = NOW()
            "#,
        )
        .bind(manual_version)
        .bind(seed_content_hash)
        .bind(page_count)
        .bind(tool_count)
        .bind(feature_count)
        .bind(ledger_event_id)
        .bind(note)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn get_version(
        &self,
        manual_version: &str,
    ) -> StorageResult<Option<UserManualVersionRow>> {
        let row = sqlx::query(
            r#"
            SELECT manual_version, seeded_at, seed_content_hash, page_count,
                   tool_count, feature_count, ledger_event_id, note
            FROM user_manual_versions WHERE manual_version = $1
            "#,
        )
        .bind(manual_version)
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(|row| UserManualVersionRow {
            manual_version: row.get("manual_version"),
            seeded_at: row.get("seeded_at"),
            seed_content_hash: row.get("seed_content_hash"),
            page_count: row.get("page_count"),
            tool_count: row.get("tool_count"),
            feature_count: row.get("feature_count"),
            ledger_event_id: row.get("ledger_event_id"),
            note: row.get("note"),
        }))
    }
}

// ---------------------------------------------------------------------------
// Row mapping.
// ---------------------------------------------------------------------------

fn page_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<UserManualPage> {
    Ok(UserManualPage {
        page_id: row.get("page_id"),
        slug: row.get("slug"),
        title: row.get("title"),
        page_kind: row.get("page_kind"),
        audience: row.get("audience"),
        body: row.get("body"),
        content_hash: row.get("content_hash"),
        manual_version: row.get("manual_version"),
        source_kind: row.get("source_kind"),
        spec_anchors: string_vec(row.get("spec_anchors"))?,
        status: row.get("status"),
        superseded_by_slug: row.get("superseded_by_slug"),
        ledger_event_id: row.get("ledger_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn anchor_from_row(row: &sqlx::postgres::PgRow) -> UserManualAnchor {
    UserManualAnchor {
        anchor_id: row.get("anchor_id"),
        page_id: row.get("page_id"),
        anchor_kind: row.get("anchor_kind"),
        anchor_value: row.get("anchor_value"),
        http_method: row.get("http_method"),
    }
}

fn tool_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<UserManualToolEntry> {
    Ok(UserManualToolEntry {
        tool_id: row.get("tool_id"),
        page_id: row.get("page_id"),
        name: row.get("name"),
        status: row.get("status"),
        ipc_channel: row.get("ipc_channel"),
        tauri_command: row.get("tauri_command"),
        cli_flag: row.get("cli_flag"),
        http_route: row.get("http_route"),
        http_method: row.get("http_method"),
        description: row.get("description"),
        expected_input: row.get("expected_input"),
        expected_output: row.get("expected_output"),
        schema_fields: string_vec(row.get("schema_fields"))?,
        common_errors: string_vec(row.get("common_errors"))?,
        recovery_steps: string_vec(row.get("recovery_steps"))?,
        origin: row.get("origin"),
        content_hash: row.get("content_hash"),
        manual_version: row.get("manual_version"),
    })
}

fn string_vec(value: Value) -> StorageResult<Vec<String>> {
    serde_json::from_value(value)
        .map_err(|_| StorageError::Validation("user manual JSONB string array is malformed"))
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
