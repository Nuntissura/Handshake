//! MT-208 bounded embedded-SurrealDB negative-path fixtures.
//!
//! Each mutation is a fixed UserManual-local operation. The seam deliberately
//! exposes no raw query or database-client authority to integration tests.

use std::collections::{BTreeMap, BTreeSet};

use super::store::{sha256_hex, NewManualSection, NewUserManualPage, UserManualStore};
use super::USER_MANUAL_VERSION;
use crate::storage::{StorageError, StorageResult};

pub async fn receipt_exists(store: &UserManualStore, event_id: &str) -> StorageResult<bool> {
    store.fixture_receipt_exists(event_id).await
}

pub async fn receipt_count(store: &UserManualStore) -> StorageResult<usize> {
    store.fixture_receipt_count().await
}

pub async fn tamper_page_content_hash(
    store: &UserManualStore,
    slug: &str,
) -> StorageResult<String> {
    let previous = store
        .get_page_by_slug(slug)
        .await?
        .map(|(page, _, _)| page.content_hash)
        .ok_or(StorageError::Validation(
            "user manual page fixture target is missing",
        ))?;
    let tampered = sha256_hex(&format!("tampered:{previous}"));
    store.fixture_set_page_content_hash(slug, &tampered).await?;
    Ok(previous)
}

pub async fn restore_page_content_hash(
    store: &UserManualStore,
    slug: &str,
    content_hash: &str,
) -> StorageResult<()> {
    store
        .fixture_set_page_content_hash(slug, content_hash)
        .await
}

pub async fn delete_page(store: &UserManualStore, slug: &str) -> StorageResult<u64> {
    Ok(u64::from(store.fixture_delete_page(slug).await?))
}

pub async fn delete_page_sections(store: &UserManualStore, page_id: &str) -> StorageResult<()> {
    store.fixture_delete_page_sections(page_id).await
}

pub async fn tamper_section(
    store: &UserManualStore,
    section_id: &str,
    title: &str,
    body_md: &str,
) -> StorageResult<()> {
    store
        .fixture_tamper_section(section_id, title, body_md)
        .await
}

pub async fn delete_route_anchor(store: &UserManualStore, route: &str) -> StorageResult<usize> {
    store.fixture_delete_route_anchor(route).await
}

pub async fn break_first_page_link(
    store: &UserManualStore,
    slug: &str,
    missing_target: &str,
) -> StorageResult<String> {
    store
        .fixture_break_first_page_link(slug, missing_target)
        .await
}

pub async fn inject_page_receipt_without_mutation(
    store: &UserManualStore,
    page: &NewUserManualPage,
) -> StorageResult<String> {
    store
        .fixture_inject_page_receipt_without_mutation(page, USER_MANUAL_VERSION)
        .await
}

pub async fn insert_orphan_page(store: &UserManualStore) -> StorageResult<String> {
    let page = NewUserManualPage {
        slug: "fixture-orphan-page".to_owned(),
        title: "Fixture Orphan".to_owned(),
        page_kind: "surface_guide",
        audience: "model",
        spec_anchors: Vec::new(),
        sections: vec![NewManualSection {
            section_kind: "purpose",
            title: "Fixture purpose".to_owned(),
            body_md: "Negative reachability fixture.".to_owned(),
            body_json: None,
        }],
        anchors: Vec::new(),
    };
    store
        .upsert_page(&page, USER_MANUAL_VERSION, "current")
        .await?;
    Ok(page.slug)
}

pub async fn unreachable_pages(store: &UserManualStore) -> StorageResult<Vec<String>> {
    let pages = store.list_pages(None, None, super::store::LIST_CAP).await?;
    let page_slugs: BTreeMap<String, String> = pages
        .iter()
        .map(|page| (page.page_id.clone(), page.slug.clone()))
        .collect();
    let links: Vec<(String, String)> = store
        .anchors_by_kind("page_link")
        .await?
        .into_iter()
        .filter_map(|anchor| {
            page_slugs
                .get(&anchor.page_id)
                .cloned()
                .map(|from| (from, anchor.anchor_value))
        })
        .collect();

    let mut reachable = BTreeSet::new();
    let mut queue = vec!["manual-toc".to_owned()];
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
        .filter(|page| page.status == "current" && !reachable.contains(&page.slug))
        .map(|page| page.slug)
        .collect())
}
