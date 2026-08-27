//! MT-208 negative-path fixture builders for the embedded UserManual store.

use surrealdb::types::{RecordId, SurrealValue};

use super::store::{sha256_hex, UserManualStore};
use crate::storage::surreal::SurrealDatabase;
use crate::storage::{StorageError, StorageResult};

fn map_error(error: crate::storage::surreal::SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

pub async fn tamper_page_content_hash(db: &SurrealDatabase, slug: &str) -> StorageResult<String> {
    let store = UserManualStore::new(db);
    let previous = store
        .get_page_by_slug(slug)
        .await?
        .ok_or(StorageError::NotFound("user manual page"))?
        .0
        .content_hash;
    restore_page_content_hash(db, slug, &sha256_hex(&format!("tampered:{previous}"))).await?;
    Ok(previous)
}

pub async fn restore_page_content_hash(
    db: &SurrealDatabase,
    slug: &str,
    content_hash: &str,
) -> StorageResult<()> {
    #[derive(SurrealValue)]
    struct Bindings {
        slug: String,
        hash: String,
    }
    let rows: Vec<surrealdb::types::Value> = db.storage().with_data_operation(move |database| {
        let bindings = Bindings { slug: slug.to_owned(), hash: content_hash.to_owned() };
        Box::pin(async move { database.query_values("UPDATE user_manual_pages SET content_hash = $hash, updated_at = time::now() WHERE slug = $slug RETURN AFTER;", bindings).await })
    }).await.map_err(map_error)?;
    if rows.is_empty() {
        return Err(StorageError::NotFound("user manual page"));
    }
    Ok(())
}

pub async fn delete_page(db: &SurrealDatabase, slug: &str) -> StorageResult<u64> {
    #[derive(SurrealValue)]
    struct Bindings {
        slug: String,
    }
    let rows: Vec<surrealdb::types::Value> = db
        .storage()
        .with_data_operation(move |database| {
            let bindings = Bindings {
                slug: slug.to_owned(),
            };
            Box::pin(async move {
                database
                    .query_values(
                        "DELETE user_manual_pages WHERE slug = $slug RETURN BEFORE;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_error)?;
    Ok(rows.len() as u64)
}

pub async fn insert_orphan_page(db: &SurrealDatabase) -> StorageResult<String> {
    #[derive(SurrealValue)]
    struct Bindings {
        id: String,
        slug: String,
        body: serde_json::Value,
        hash: String,
    }
    let slug = "fixture-orphan-page".to_owned();
    let bindings = Bindings {
        id: format!("UMP-fixture-{}", uuid::Uuid::now_v7()),
        slug: slug.clone(),
        body: serde_json::json!({"sections": [], "anchors": []}),
        hash: sha256_hex("fixture-orphan-page-body"),
    };
    db.storage().with_data_operation(move |database| Box::pin(async move {
        database.query_values::<surrealdb::types::Value, _>("IF (SELECT VALUE id FROM user_manual_pages WHERE slug = $slug LIMIT 1)[0] = NONE { CREATE type::record('user_manual_pages', $id) CONTENT { page_id: $id, slug: $slug, title: 'Fixture Orphan', page_kind: 'surface_guide', audience: 'model', body: $body, content_hash: $hash, manual_version: 'fixture', source_kind: 'runtime_edit', spec_anchors: [], status: 'current' }; };", bindings).await
    })).await.map_err(map_error)?;
    Ok(slug)
}

pub async fn unreachable_pages(db: &SurrealDatabase) -> StorageResult<Vec<String>> {
    let store = UserManualStore::new(db);
    let pages = store.list_pages(None, None, super::store::LIST_CAP).await?;
    let mut links = Vec::new();
    for page in &pages {
        for anchor in store.anchors_for(&page.page_id).await? {
            if anchor.anchor_kind == "page_link" {
                links.push((page.slug.clone(), anchor.anchor_value));
            }
        }
    }
    let mut reachable = std::collections::BTreeSet::new();
    let mut queue = vec!["manual-toc".to_owned()];
    while let Some(slug) = queue.pop() {
        if !reachable.insert(slug.clone()) {
            continue;
        }
        for (from, to) in &links {
            if from == &slug && !reachable.contains(to) {
                queue.push(to.clone());
            }
        }
    }
    Ok(pages
        .into_iter()
        .filter(|page| page.status == "current" && !reachable.contains(&page.slug))
        .map(|page| page.slug)
        .collect())
}
