//! MT-208 UserManualFixtures: negative-path fixture builders that prove the
//! manual surface cannot false-PASS.
//!
//! Fixture families (each driven to its negative verdict by
//! `tests/user_manual_content_tests.rs`):
//! * stale manual detection — tamper a stored page's content hash and the
//!   freshness check MUST flip to `stale_content`;
//! * missing page — delete a seeded page and freshness MUST report
//!   `missing_page` (and the read route 404s);
//! * legacy name redirect — every legacy alias resolves; an unknown alias
//!   MUST NOT resolve;
//! * search — a seeded corpus term is findable; a nonsense term returns
//!   empty (bounded, no fuzzy false hits);
//! * context bundle citation — a manual citation carries slug + version +
//!   anchor + hash prefix;
//! * visual navigation — the TOC reaches every page; severing the link graph
//!   MUST be detected as an orphan.

use sqlx::Row;

use super::store::sha256_hex;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

/// Tamper a stored page's content hash (simulates seed/code drift or row
/// tampering). Returns the previous hash so the test can restore it.
pub async fn tamper_page_content_hash(
    db: &PostgresDatabase,
    slug: &str,
) -> StorageResult<String> {
    let row = sqlx::query("SELECT content_hash FROM user_manual_pages WHERE slug = $1")
        .bind(slug)
        .fetch_one(db.pool())
        .await?;
    let previous: String = row.get("content_hash");
    let tampered = sha256_hex(&format!("tampered:{previous}"));
    sqlx::query("UPDATE user_manual_pages SET content_hash = $2 WHERE slug = $1")
        .bind(slug)
        .bind(&tampered)
        .execute(db.pool())
        .await?;
    Ok(previous)
}

/// Restore a page hash after the stale fixture.
pub async fn restore_page_content_hash(
    db: &PostgresDatabase,
    slug: &str,
    content_hash: &str,
) -> StorageResult<()> {
    sqlx::query("UPDATE user_manual_pages SET content_hash = $2 WHERE slug = $1")
        .bind(slug)
        .bind(content_hash)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Delete a seeded page (missing-page fixture). Sections/anchors cascade.
pub async fn delete_page(db: &PostgresDatabase, slug: &str) -> StorageResult<u64> {
    let result = sqlx::query("DELETE FROM user_manual_pages WHERE slug = $1")
        .bind(slug)
        .execute(db.pool())
        .await?;
    Ok(result.rows_affected())
}

/// Insert an orphan page that nothing links to (visual-navigation fixture):
/// the navigation audit MUST flag it as unreachable from the TOC.
pub async fn insert_orphan_page(db: &PostgresDatabase) -> StorageResult<String> {
    let slug = "fixture-orphan-page";
    let body = serde_json::json!({"sections": [], "anchors": []});
    let hash = sha256_hex("fixture-orphan-page-body");
    sqlx::query(
        r#"
        INSERT INTO user_manual_pages (
            page_id, slug, title, page_kind, audience, body, content_hash,
            manual_version, source_kind, spec_anchors, status
        ) VALUES ($1, $2, 'Fixture Orphan', 'surface_guide', 'model', $3, $4,
                  'fixture', 'runtime_edit', '[]'::jsonb, 'current')
        ON CONFLICT (slug) DO NOTHING
        "#,
    )
    .bind(format!("UMP-fixture-{}", uuid::Uuid::now_v7()))
    .bind(slug)
    .bind(body)
    .bind(hash)
    .execute(db.pool())
    .await?;
    Ok(slug.to_string())
}

/// Audit TOC reachability over the STORED rows (not the seed): BFS from
/// `manual-toc` across `page_link` anchors; returns slugs of stored pages
/// that are not reachable. The healthy corpus returns an empty list; the
/// orphan fixture must appear here.
pub async fn unreachable_pages(db: &PostgresDatabase) -> StorageResult<Vec<String>> {
    let pages: Vec<(String, String)> =
        sqlx::query("SELECT page_id, slug FROM user_manual_pages WHERE status = 'current'")
            .fetch_all(db.pool())
            .await?
            .iter()
            .map(|row| (row.get("page_id"), row.get("slug")))
            .collect();
    let links: Vec<(String, String)> = sqlx::query(
        r#"
        SELECT p.slug AS from_slug, a.anchor_value AS to_slug
        FROM user_manual_anchors a
        JOIN user_manual_pages p ON p.page_id = a.page_id
        WHERE a.anchor_kind = 'page_link'
        "#,
    )
    .fetch_all(db.pool())
    .await?
    .iter()
    .map(|row| (row.get("from_slug"), row.get("to_slug")))
    .collect();

    let mut reachable = std::collections::BTreeSet::new();
    let mut queue = vec!["manual-toc".to_string()];
    while let Some(slug) = queue.pop() {
        if !reachable.insert(slug.clone()) {
            continue;
        }
        for (from, to) in &links {
            if *from == slug && !reachable.contains(to) {
                queue.push(to.clone());
            }
        }
    }
    Ok(pages
        .into_iter()
        .map(|(_, slug)| slug)
        .filter(|slug| !reachable.contains(slug))
        .collect())
}
