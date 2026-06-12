//! WP-KERNEL-009 MT-178 BacklinkComputation — REAL PostgreSQL authority proof.
//!
//! Proves the Heaper/Obsidian backlink payoff (Master Spec §10.12
//! [LM-BACK-001..003] / Pattern H-5) over the loom_edges + loom_blocks Postgres
//! store (no parallel index):
//!   * linked backlinks: every incoming MENTION/TAG edge surfaces the source
//!     block + a surrounding-text context snippet;
//!   * unlinked mentions: a block whose text contains the viewed block's title
//!     on a word boundary but with NO formal edge is surfaced for one-click
//!     conversion, and a block that IS linked is excluded;
//!   * fail-closed on a missing viewed block.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock,
    NewLoomEdge, WriteContext,
};
use knowledge_pg_support::knowledge_pg;
use uuid::Uuid;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-178 loom backlink proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn make_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    title: &str,
    full_text: Option<&str>,
) -> String {
    let ctx = WriteContext::human(None);
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = full_text.map(|t| t.to_string());
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.to_string(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived,
            },
        )
        .await
        .expect("create block");
    block.block_id
}

async fn link(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    source: &str,
    target: &str,
    edge_type: LoomEdgeType,
) {
    let ctx = WriteContext::human(None);
    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: ws.to_string(),
            source_block_id: source.to_string(),
            target_block_id: target.to_string(),
            edge_type,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await
    .expect("create edge");
}

#[tokio::test]
async fn linked_backlinks_carry_source_block_and_context_snippet() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let target = make_block(&pg.db, &ws, "Roadmap", None).await;
    let source = make_block(
        &pg.db,
        &ws,
        "Quarterly Planning",
        Some("We must align the Roadmap with the budget before the review meeting."),
    )
    .await;
    link(&pg.db, &ws, &source, &target, LoomEdgeType::Mention).await;

    let backlinks = pg
        .db
        .get_backlinks_with_context(&ws, &target)
        .await
        .expect("backlinks");
    assert_eq!(backlinks.len(), 1, "one incoming mention edge");
    let bl = &backlinks[0];
    assert_eq!(bl.source_block.block_id, source);
    assert_eq!(bl.edge.edge_type, LoomEdgeType::Mention);
    let snippet = bl
        .context_snippet
        .as_deref()
        .expect("a context snippet for a titled source");
    assert!(
        snippet.contains("Roadmap"),
        "snippet shows the reference in context: {snippet}"
    );
}

#[tokio::test]
async fn unlinked_mentions_surface_only_unlinked_word_boundary_matches() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let target = make_block(&pg.db, &ws, "Roadmap", None).await;

    // (a) An UNLINKED block whose text mentions "Roadmap" -> should surface.
    let unlinked = make_block(
        &pg.db,
        &ws,
        "Notes",
        Some("The Roadmap needs a second pass after planning."),
    )
    .await;

    // (b) A block that mentions "Roadmaps" only as part of a larger word ->
    //     must NOT surface (word-boundary rule).
    let _substring_only = make_block(
        &pg.db,
        &ws,
        "Glossary",
        Some("Roadmapping is a discipline."),
    )
    .await;

    // (c) A LINKED block that also mentions "Roadmap" -> must NOT surface as
    //     unlinked (it already has a formal edge).
    let linked = make_block(
        &pg.db,
        &ws,
        "Strategy",
        Some("Our Roadmap is set for the year."),
    )
    .await;
    link(&pg.db, &ws, &linked, &target, LoomEdgeType::Mention).await;

    let mentions = pg
        .db
        .scan_unlinked_mentions(&ws, &target, &[], 100)
        .await
        .expect("scan unlinked");

    let ids: Vec<&str> = mentions.iter().map(|m| m.source_block.block_id.as_str()).collect();
    assert!(ids.contains(&unlinked.as_str()), "unlinked mention surfaces");
    assert!(
        !ids.contains(&linked.as_str()),
        "already-linked block is not an unlinked mention"
    );
    assert!(
        !ids.iter().any(|id| *id == _substring_only),
        "substring-only (Roadmapping) does not match on a word boundary"
    );

    // The surfaced mention carries the matched term + a snippet + offset.
    let m = mentions
        .iter()
        .find(|m| m.source_block.block_id == unlinked)
        .unwrap();
    assert_eq!(m.matched_term, "Roadmap");
    assert!(m.snippet.contains("Roadmap"));
    assert!(m.match_offset >= 0);
}

#[tokio::test]
async fn unlinked_mentions_scan_aliases_too() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let target = make_block(&pg.db, &ws, "Roadmap", None).await;
    let by_alias = make_block(
        &pg.db,
        &ws,
        "Plan",
        Some("The Plan-of-Record covers Q3; see the OKRs."),
    )
    .await;

    // No mention of "Roadmap", but mentions the alias "Plan-of-Record".
    let mentions = pg
        .db
        .scan_unlinked_mentions(&ws, &target, &["Plan-of-Record".to_string()], 100)
        .await
        .expect("scan with alias");
    assert!(
        mentions
            .iter()
            .any(|m| m.source_block.block_id == by_alias && m.matched_term == "Plan-of-Record"),
        "alias term is scanned"
    );
}

#[tokio::test]
async fn backlinks_fail_closed_on_missing_block() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let missing = format!("loom-missing-{}", Uuid::now_v7());
    let err = pg
        .db
        .get_backlinks_with_context(&ws, &missing)
        .await
        .expect_err("missing block fails closed");
    let msg = format!("{err}");
    assert!(msg.contains("not found") || msg.contains("not_found"), "{msg}");
}
