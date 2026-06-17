//! WP-KERNEL-009 MT-239 UserManualFreshnessFixture.
//!
//! Real PostgreSQL proof that UserManual freshness rejects false PASS states
//! when stored child rows drift without changing page row hashes or row counts.

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::seed::ensure_seeded;
use handshake_core::user_manual::store::UserManualStore;
use sqlx::Connection;

#[tokio::test]
async fn mt239_freshness_detects_same_count_page_child_tampering() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt239_same_count_child_tamper"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
    let clean = check_freshness(&kpg.db)
        .await
        .expect("freshness before tamper");
    assert!(clean.fresh, "seeded manual must start fresh: {:?}", clean);

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

    let mut conn = kpg.raw_connection().await;
    sqlx::query(
        r#"
        UPDATE user_manual_sections
        SET title = 'tampered same-count section title',
            body_md = 'tampered same-count section body'
        WHERE section_id = $1
        "#,
    )
    .bind(&section.section_id)
    .execute(&mut conn)
    .await
    .expect("tamper section in place");
    let stored_hash_after_tamper: String =
        sqlx::query_scalar("SELECT content_hash FROM user_manual_pages WHERE page_id = $1")
            .bind(&page.page_id)
            .fetch_one(&mut conn)
            .await
            .expect("page hash after tamper");
    let section_count_after_tamper: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_manual_sections WHERE page_id = $1")
            .bind(&page.page_id)
            .fetch_one(&mut conn)
            .await
            .expect("section count after tamper");
    conn.close().await.ok();

    assert_eq!(
        stored_hash_after_tamper, page.content_hash,
        "fixture must not update the page row hash"
    );
    assert_eq!(
        section_count_after_tamper as usize,
        sections.len(),
        "fixture must keep the child row count unchanged"
    );

    let stale = check_freshness(&kpg.db)
        .await
        .expect("freshness after same-count child tamper");
    assert!(
        !stale.fresh,
        "same-count child tampering must not report fresh: {:?}",
        stale
    );
    assert!(
        stale
            .verdicts
            .iter()
            .any(|v| { v.kind == FreshnessVerdictKind::StaleContent && v.subject == page.slug }),
        "same-count child tampering must yield stale_content for {}; got {:?}",
        page.slug,
        stale.verdicts
    );

    let healed = ensure_seeded(&kpg.db).await.expect("healing reseed");
    assert!(
        healed.pages_changed >= 1,
        "reseed must heal same-count child row tampering"
    );
    let (_, healed_sections, _) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("manual-toc after heal")
        .expect("manual-toc still seeded");
    assert_eq!(healed_sections.len(), sections.len());
    assert_eq!(healed_sections[0].title, original_title);
    assert_eq!(healed_sections[0].body_md, original_body);
}
