//! WP-KERNEL-009 MT-239 UserManualFreshnessFixture.
//!
//! Embedded-SurrealDB proof that freshness rejects false PASS states when a
//! stored child row drifts without changing its parent hash or row count.

#[allow(dead_code)]
mod user_manual_support;

use handshake_core::storage::surreal::{TestFieldMutation, TestMutationValue};
use handshake_core::storage::Database;
use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::seed::ensure_seeded;
use handshake_core::user_manual::store::UserManualStore;
use user_manual_support::manual_test_backend;

#[tokio::test]
async fn mt239_freshness_detects_same_count_page_child_tampering() {
    let backend = manual_test_backend()
        .await
        .expect("open embedded UserManual backend");
    ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);
    let clean = check_freshness(&backend.db)
        .await
        .expect("freshness before tamper");
    assert!(clean.fresh, "seeded manual must start fresh: {clean:?}");

    let (page, sections, _) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("manual-toc query")
        .expect("manual-toc seeded");
    let section = sections
        .first()
        .expect("manual-toc must have at least one section");
    let original_title = section.title.clone();
    let original_body = section.body_md.clone();

    let inspector = backend.db.storage().test_inspector();
    let section_table = inspector
        .table_selector("user_manual_sections")
        .await
        .expect("select UserManual section table");
    let section_title = section_table.field("title").expect("select title field");
    let section_body = section_table.field("body_md").expect("select body field");
    backend
        .db
        .storage()
        .test_mutator()
        .update_row(
            &section_table,
            &section.section_id,
            &[
                TestFieldMutation::new(
                    section_title,
                    TestMutationValue::string("tampered same-count section title"),
                ),
                TestFieldMutation::new(
                    section_body,
                    TestMutationValue::string("tampered same-count section body"),
                ),
            ],
        )
        .await
        .expect("tamper one child row in place");

    let (stored_page, stored_sections, _) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("manual-toc after tamper")
        .expect("manual-toc remains stored");
    assert_eq!(
        stored_page.content_hash, page.content_hash,
        "fixture must not update the parent page hash"
    );
    assert_eq!(
        stored_sections.len(),
        sections.len(),
        "fixture must keep the child row count unchanged"
    );

    let stale = check_freshness(&backend.db)
        .await
        .expect("freshness after same-count child tamper");
    assert!(
        !stale.fresh,
        "same-count child tamper reported fresh: {stale:?}"
    );
    assert!(
        stale
            .verdicts
            .iter()
            .any(|verdict| verdict.kind == FreshnessVerdictKind::StaleContent
                && verdict.subject == page.slug),
        "same-count child tamper must yield stale_content for {}: {:?}",
        page.slug,
        stale.verdicts
    );

    let healed = ensure_seeded(&backend.db).await.expect("healing reseed");
    assert!(healed.pages_changed >= 1, "reseed must heal child drift");
    let (_, healed_sections, _) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("manual-toc after heal")
        .expect("manual-toc still seeded");
    assert_eq!(healed_sections.len(), sections.len());
    assert_eq!(healed_sections[0].title, original_title);
    assert_eq!(healed_sections[0].body_md, original_body);

    drop(store);
    backend
        .close_and_remove()
        .await
        .expect("close embedded UserManual backend");
}
