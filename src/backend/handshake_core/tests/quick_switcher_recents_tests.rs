//! WP-KERNEL-009 MT-256 QuickSwitcher durable recents proof.
//!
//! Recents are part of the ProjectKnowledgeIndex/Loom navigation workflow, so
//! they must persist in PostgreSQL and retain a Kernel EventLedger receipt. This
//! test intentionally drives the storage trait against a real isolated
//! PostgreSQL schema; no mock, browser storage, or in-memory fallback can pass.

mod knowledge_pg_support;

use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{
    Database, LoomSearchResultKind, LoomSearchSourceKind, QuickSwitcherRecentInput, StorageError,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;
use sqlx::Row;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                panic!("MT-256 quick switcher recents proof requires real PostgreSQL");
            }
        }
    }};
}

#[tokio::test]
async fn mt256_quick_switcher_recents_reject_empty_ref_or_title() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let missing_ref = pg
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: "   ".to_string(),
                title: "Recent Beta".to_string(),
                excerpt: String::new(),
                metadata: json!({}),
            },
        )
        .await
        .expect_err("empty ref_id must be rejected");
    assert!(matches!(
        missing_ref,
        StorageError::Validation("quick switcher recent ref_id is required")
    ));

    let missing_title = pg
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: "recent-beta".to_string(),
                title: "\t".to_string(),
                excerpt: String::new(),
                metadata: json!({}),
            },
        )
        .await
        .expect_err("empty title must be rejected");
    assert!(matches!(
        missing_title,
        StorageError::Validation("quick switcher recent title is required")
    ));
}

#[tokio::test]
async fn mt256_quick_switcher_recents_persist_with_eventledger() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    pg.db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: "recent-beta".to_string(),
                title: "Recent Beta".to_string(),
                excerpt: "Selected from the QuickSwitcher.".to_string(),
                metadata: json!({ "page_slug": "recent-beta" }),
            },
        )
        .await
        .expect("record beta recent");
    pg.db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: "recent-alpha".to_string(),
                title: "Recent Alpha".to_string(),
                excerpt: "Older selection.".to_string(),
                metadata: json!({ "page_slug": "recent-alpha" }),
            },
        )
        .await
        .expect("record alpha recent");
    let promoted = pg
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: "recent-beta".to_string(),
                title: "Recent Beta Updated".to_string(),
                excerpt: "Selected again and promoted.".to_string(),
                metadata: json!({ "page_slug": "recent-beta", "source": "repeat" }),
            },
        )
        .await
        .expect("promote beta recent");
    let wiki_recent = pg
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::WikiPage,
                source_kind: LoomSearchSourceKind::WikiPage,
                ref_id: "KWP-alpha".to_string(),
                title: "GraphSearchAlpha Wiki Page".to_string(),
                excerpt: "Compiled project wiki page selected from QuickSwitcher.".to_string(),
                metadata: json!({ "projection_id": "KWP-alpha", "page_type": "concept" }),
            },
        )
        .await
        .expect("record wiki page recent");

    assert_eq!(promoted.hit_key, "user_manual_page:recent-beta");
    assert_eq!(promoted.selected_count, 2);
    assert_eq!(promoted.title, "Recent Beta Updated");
    assert!(!promoted.event_ledger_event_id.trim().is_empty());
    assert_eq!(wiki_recent.hit_key, "wiki_page:KWP-alpha");
    assert!(!wiki_recent.event_ledger_event_id.trim().is_empty());

    let recents = pg
        .db
        .list_quick_switcher_recents(&ws, 20)
        .await
        .expect("list durable recents");
    let ordered_refs: Vec<_> = recents
        .iter()
        .map(|recent| recent.ref_id.as_str())
        .collect();
    assert_eq!(
        ordered_refs,
        vec!["KWP-alpha", "recent-beta", "recent-alpha"]
    );
    assert_eq!(
        recents[0].event_ledger_event_id,
        wiki_recent.event_ledger_event_id
    );

    let mut conn = pg.raw_connection().await;
    let event_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND event_type = $2
          AND aggregate_type = 'quick_switcher_recent'
          AND payload ->> 'workspace_id' = $3
          AND payload ->> 'hit_key' = 'user_manual_page:recent-beta'
        "#,
    )
    .bind(&promoted.event_ledger_event_id)
    .bind(KernelEventType::KnowledgeQuickSwitcherRecentRecorded.as_str())
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("query matching kernel event")
    .get("count");
    assert_eq!(
        event_count, 1,
        "recorded recent must reference exactly one typed Kernel EventLedger row"
    );

    let row_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM knowledge_quick_switcher_recents r
        JOIN kernel_event_ledger e
          ON e.event_id = r.event_ledger_event_id
        WHERE r.workspace_id = $1
          AND r.hit_key = 'user_manual_page:recent-beta'
          AND e.event_type = $2
        "#,
    )
    .bind(&ws)
    .bind(KernelEventType::KnowledgeQuickSwitcherRecentRecorded.as_str())
    .fetch_one(&mut conn)
    .await
    .expect("query recent row event FK")
    .get("count");
    assert_eq!(row_count, 1, "recent row must retain its EventLedger FK");
}

#[tokio::test]
async fn mt256_quick_switcher_recents_accept_breadth_source_kinds() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    for (source_kind, result_kind, ref_id, title) in [
        (
            LoomSearchSourceKind::File,
            LoomSearchResultKind::LoomBlock,
            "LB-file-alpha",
            "GraphSearchAlpha source file",
        ),
        (
            LoomSearchSourceKind::TagHub,
            LoomSearchResultKind::LoomBlock,
            "LB-tag-alpha",
            "GraphSearchAlpha tag hub",
        ),
        (
            LoomSearchSourceKind::Document,
            LoomSearchResultKind::KnowledgeEntity,
            "KRD-00000000000000000000000000000001",
            "GraphSearchAlpha standalone document",
        ),
    ] {
        let recent = pg
            .db
            .record_quick_switcher_recent(
                &ws,
                QuickSwitcherRecentInput {
                    result_kind,
                    source_kind,
                    ref_id: ref_id.to_string(),
                    title: title.to_string(),
                    excerpt: "Selected from QuickSwitcher breadth results.".to_string(),
                    metadata: json!({ "source_kind": source_kind.as_str() }),
                },
            )
            .await
            .expect("record breadth recent");
        assert_eq!(recent.hit_key, format!("{}:{ref_id}", source_kind.as_str()));
        assert!(!recent.event_ledger_event_id.trim().is_empty());
    }

    let recents = pg
        .db
        .list_quick_switcher_recents(&ws, 10)
        .await
        .expect("list breadth recents");
    let source_kinds: Vec<_> = recents
        .iter()
        .map(|recent| recent.source_kind.as_str())
        .collect();
    assert_eq!(source_kinds, vec!["document", "tag_hub", "file"]);
}
