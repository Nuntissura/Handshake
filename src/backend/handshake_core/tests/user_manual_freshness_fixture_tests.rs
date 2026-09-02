//! WP-KERNEL-009 MT-239 UserManualFreshnessFixture.
//!
//! Embedded SurrealDB proof that UserManual freshness rejects false PASS
//! states when stored child records drift without changing page hashes or
//! child counts.

mod surreal_test_store_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::user_manual::fixtures::tamper_section;
use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::seed::ensure_seeded;
use user_manual_support::UserManualTestScope;

#[tokio::test]
async fn model_lane_user_manual_freshness_detects_stale_code_truth() {
    let scope = UserManualTestScope::create().await;
    let storage = scope.storage();
    let store = scope.store();
    ensure_seeded(&storage).await.expect("seed");
    let clean = check_freshness(&storage)
        .await
        .expect("freshness before model-lane tamper");
    assert!(clean.fresh, "seeded manual must start fresh: {:?}", clean);

    let (page, sections, _) = store
        .get_page_by_slug("model-lane-validation-harness")
        .await
        .expect("model-lane validation harness query")
        .expect("model-lane validation harness seeded");
    let section = sections
        .iter()
        .find(|section| section.title == "Behavior coverage matrix")
        .expect("model-lane validation harness has behavior matrix section");

    tamper_section(
        &store,
        &section.section_id,
        &section.title,
        "tampered MT-011 self-consistency proof text",
    )
    .await
    .expect("tamper model-lane behavior matrix section");

    let stale = check_freshness(&storage)
        .await
        .expect("freshness after model-lane behavior matrix tamper");
    assert!(
        !stale.fresh,
        "model-lane manual tampering must not report fresh: {:?}",
        stale
    );
    assert!(
        stale
            .verdicts
            .iter()
            .any(|v| { v.kind == FreshnessVerdictKind::StaleContent && v.subject == page.slug }),
        "model-lane manual tampering must yield stale_content for {}; got {:?}",
        page.slug,
        stale.verdicts
    );

    let healed = ensure_seeded(&storage).await.expect("healing reseed");
    assert!(
        healed.pages_changed >= 1,
        "reseed must heal model-lane UserManual drift"
    );
    drop(store);
    drop(storage);
    scope.cleanup().await;
}

#[tokio::test]
async fn mt239_freshness_detects_same_count_page_child_tampering() {
    let scope = UserManualTestScope::create().await;
    let storage = scope.storage();
    let store = scope.store();
    ensure_seeded(&storage).await.expect("seed");
    let clean = check_freshness(&storage)
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

    tamper_section(
        &store,
        &section.section_id,
        "tampered same-count section title",
        "tampered same-count section body",
    )
    .await
    .expect("tamper section in place");
    let (stored_after_tamper, sections_after_tamper, _) = store
        .get_page_by_slug(&page.slug)
        .await
        .expect("page after tamper")
        .expect("page remains present after child tamper");

    assert_eq!(
        stored_after_tamper.content_hash, page.content_hash,
        "fixture must not update the page row hash"
    );
    assert_eq!(
        sections_after_tamper.len(),
        sections.len(),
        "fixture must keep the child row count unchanged"
    );

    let stale = check_freshness(&storage)
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

    let healed = ensure_seeded(&storage).await.expect("healing reseed");
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
    drop(store);
    drop(storage);
    scope.cleanup().await;
}
