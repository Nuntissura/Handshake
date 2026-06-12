//! WP-KERNEL-009 MT-184 WikiPageProjectionCompiler / MT-185 WikiPageEditable
//! Overlay / MT-187 ObsidianImportBoundary — REAL PostgreSQL authority proof.
//!
//! Proves the projection-NEVER-authority law (§10.12 §9.1.1):
//!   * MT-184 a wiki page is compiled from LoomBlock authority with citations
//!     (source block ids) + a staleness hash; editing a source block marks the
//!     projection stale; regenerate refreshes it.
//!   * MT-187 a markdown-like note imports into LoomBlock + RichDocument
//!     authority rows; the markdown SOURCE is never authority.
//!   * NEGATIVE TEST (operator hard requirement): deleting a wiki projection
//!     leaves the LoomBlock authority BYTE-IDENTICAL.
//!   * MT-185 an operator overlay is its own authority row; deleting/adding an
//!     overlay never mutates the projection, and deleting the projection never
//!     touches the source blocks.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, NewLoomBlock, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-184/185/187 loom wiki boundary proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn note(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    title: &str,
    body: &str,
) -> String {
    let ctx = WriteContext::human(None);
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = Some(body.to_string());
    db.create_loom_block(
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
    .expect("note")
    .block_id
}

/// Snapshot the authority-relevant fields of a LoomBlock for byte-identity
/// comparison across a projection delete.
fn block_fingerprint(b: &handshake_core::storage::LoomBlock) -> String {
    serde_json::to_string(b).expect("serialize block")
}

#[tokio::test]
async fn wiki_projection_compiles_with_citations_and_staleness() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let a = note(&pg.db, &ws, "Alpha", "alpha body").await;
    let b = note(&pg.db, &ws, "Beta", "beta body").await;

    let proj = pg
        .db
        .compile_loom_wiki_projection(&ws, "Topic Page", &[a.clone(), b.clone()])
        .await
        .expect("compile");
    // Citations: both source blocks are recorded.
    assert_eq!(proj.source_block_ids, vec![a.clone(), b.clone()]);
    // Rendered content cites the blocks and is deterministic.
    assert!(proj.rendered_content.contains("loom_block:") );
    assert!(proj.rendered_content.contains("Alpha") && proj.rendered_content.contains("Beta"));
    assert!(!proj.staleness_hash.is_empty());

    // Fresh right after compile.
    assert!(
        !pg.db.loom_wiki_projection_is_stale(&ws, &proj.projection_id).await.expect("stale?"),
        "freshly compiled projection is not stale"
    );

    // Edit a source block -> projection becomes stale.
    pg.db
        .update_loom_block(
            &WriteContext::human(None),
            &ws,
            &a,
            LoomBlockUpdate {
                title: Some("Alpha Renamed".into()),
                ..Default::default()
            },
        )
        .await
        .expect("rename a");
    assert!(
        pg.db.loom_wiki_projection_is_stale(&ws, &proj.projection_id).await.expect("stale2"),
        "editing a source block marks the projection stale"
    );

    // Regenerate -> fresh again, content reflects the rename.
    let regen = pg
        .db
        .regenerate_loom_wiki_projection(&ws, &proj.projection_id)
        .await
        .expect("regenerate");
    assert!(regen.rendered_content.contains("Alpha Renamed"));
    assert!(
        !pg.db.loom_wiki_projection_is_stale(&ws, &proj.projection_id).await.expect("stale3"),
        "regenerated projection is fresh"
    );
}

#[tokio::test]
async fn deleting_projection_leaves_block_authority_byte_identical() {
    // THE OPERATOR HARD REQUIREMENT: projection delete must not touch authority.
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let a = note(&pg.db, &ws, "Authority Block", "the canonical content").await;
    // Bridge the block to the ProjectKnowledgeIndex (the storage-layer create
    // does not auto-bridge; the API layer does — here we bridge explicitly so
    // we can prove the bridge is also untouched by a projection delete).
    pg.db
        .bridge_loom_block_to_knowledge(&WriteContext::human(None), &ws, &a)
        .await
        .expect("bridge block");

    // Snapshot block authority BEFORE the projection exists.
    let before = block_fingerprint(&pg.db.get_loom_block(&ws, &a).await.expect("before"));
    let bridge_before = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &a)
        .await
        .expect("bridge before")
        .expect("block is bridged");

    let proj = pg
        .db
        .compile_loom_wiki_projection(&ws, "Doomed Page", &[a.clone()])
        .await
        .expect("compile");

    // Delete the projection.
    pg.db
        .delete_loom_wiki_projection(&ws, &proj.projection_id)
        .await
        .expect("delete projection");
    assert!(
        pg.db.get_loom_wiki_projection(&ws, &proj.projection_id).await.is_err(),
        "projection is gone"
    );

    // The block authority is BYTE-IDENTICAL, and its ProjectKnowledgeIndex
    // bridge (entity + receipt) is unchanged.
    let after = block_fingerprint(&pg.db.get_loom_block(&ws, &a).await.expect("after"));
    assert_eq!(before, after, "block authority is byte-identical after projection delete");
    let bridge_after = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &a)
        .await
        .expect("bridge after")
        .expect("still bridged");
    assert_eq!(bridge_before.entity_id, bridge_after.entity_id);
    assert_eq!(bridge_before.index_event_id, bridge_after.index_event_id);
}

#[tokio::test]
async fn overlay_is_own_authority_independent_of_projection() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let a = note(&pg.db, &ws, "Src", "body").await;
    let proj = pg
        .db
        .compile_loom_wiki_projection(&ws, "Overlaid", &[a.clone()])
        .await
        .expect("compile");

    // Add an operator annotation overlay.
    let overlay = pg
        .db
        .add_loom_wiki_overlay(&ws, &proj.projection_id, "operator note: verify Q3", Some(&a))
        .await
        .expect("add overlay");
    assert_eq!(overlay.annotation, "operator note: verify Q3");

    // The projection's rendered content is UNCHANGED by the overlay (the
    // overlay is separate authority, never folded into the projection).
    let proj_after = pg.db.get_loom_wiki_projection(&ws, &proj.projection_id).await.expect("get");
    assert_eq!(proj_after.rendered_content, proj.rendered_content);
    assert!(!proj_after.rendered_content.contains("operator note"));

    // The overlay is listed.
    let overlays = pg.db.list_loom_wiki_overlays(&ws, &proj.projection_id).await.expect("list");
    assert_eq!(overlays.len(), 1);

    // Regenerating the projection does not remove the overlay (separate row).
    pg.db.regenerate_loom_wiki_projection(&ws, &proj.projection_id).await.expect("regen");
    let overlays2 = pg.db.list_loom_wiki_overlays(&ws, &proj.projection_id).await.expect("list2");
    assert_eq!(overlays2.len(), 1, "overlay survives projection regenerate");

    // Deleting the overlay does not touch the projection.
    pg.db.delete_loom_wiki_overlay(&ws, &overlay.overlay_id).await.expect("del overlay");
    assert!(
        pg.db.get_loom_wiki_projection(&ws, &proj.projection_id).await.is_ok(),
        "projection survives overlay delete"
    );
}

#[tokio::test]
async fn markdown_import_creates_authority_never_vault() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let markdown = "# My Imported Note\n\nThis is **bold** and a list:\n\n- one\n- two\n";
    let imported = pg
        .db
        .import_markdown_to_loom(&ctx, &ws, "My Imported Note", markdown)
        .await
        .expect("import markdown");

    // A real authority LoomBlock backed by a real RichDocument exists in PG.
    // (loom_blocks.document_id FKs the legacy `documents` table, so the
    // RichDocument link is logical via rich_document_id, not that column.)
    assert_eq!(imported.block.content_type, LoomBlockContentType::Note);
    assert!(
        imported.rich_document_id.starts_with("KRD-"),
        "imported note is backed by a real authority RichDocument: {}",
        imported.rich_document_id
    );

    // The imported block resolves to the ProjectKnowledgeIndex (never a
    // vault-only row): it is bridged.
    let bridge = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &imported.block.block_id)
        .await
        .expect("bridge")
        .expect("imported note is bridged to authority");
    assert!(bridge.entity_id.starts_with("KEN-"));

    // The block is queryable as authority (round-trips from PG).
    let reloaded = pg.db.get_loom_block(&ws, &imported.block.block_id).await.expect("reload");
    assert_eq!(reloaded.title.as_deref(), Some("My Imported Note"));
}

#[tokio::test]
async fn wiki_apis_fail_closed() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    // Compile citing a non-existent block fails closed (no dangling citation).
    let err = pg
        .db
        .compile_loom_wiki_projection(&ws, "Bad", &["loom-missing".into()])
        .await
        .expect_err("missing source block");
    assert!(format!("{err}").contains("loom_block") || format!("{err}").contains("not"), "{err}");

    // Get a non-existent projection.
    assert!(pg.db.get_loom_wiki_projection(&ws, "LWPmissing").await.is_err());
}
