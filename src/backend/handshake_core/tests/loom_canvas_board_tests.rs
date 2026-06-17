//! WP-KERNEL-009 MT-261 CanvasBoard — REAL PostgreSQL + EventLedger authority
//! proof.
//!
//! Proves the Obsidian-canvas-class surface over LoomBlock authority
//! (Master Spec §7.1.4.3 / §10.12). All assertions run against the same
//! isolated schema the full migration chain ran in (`knowledge_pg`).
//!
//! Covered:
//!  * the canvas IS a `LoomBlock(content_type='canvas')` with a knowledge bridge;
//!  * board persists: placements/visual-edges/viewport round-trip + EventLedger
//!    receipt on the board row;
//!  * reference-not-copy: `remove_canvas_placement` keeps the source block;
//!    deleting the canvas block keeps placed blocks (CASCADE only hits the board
//!    + its placements/visual-edges, never the referenced loom_blocks);
//!  * editing the source block reflects through the placement (live reference);
//!  * a SEMANTIC edge appears in the local Loom graph; a VISUAL-ONLY edge does
//!    NOT (it is never a loom_edge);
//!  * a free-text card is a real note LoomBlock + RichDocument.

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomCanvasPlacementUpdate,
    LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock, NewLoomCanvasPlacement, NewLoomEdge,
    WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-261 loom canvas board proof: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

async fn make_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    title: &str,
    content_type: LoomBlockContentType,
) -> String {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create loom block");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge block");
    block.block_id
}

fn board_state(pan_x: f64, pan_y: f64, zoom: f64) -> serde_json::Value {
    json!({
        "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
        "pan_x": pan_x,
        "pan_y": pan_y,
        "zoom": zoom,
    })
}

async fn make_canvas(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    workspace_id: &str,
    title: &str,
) -> String {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create canvas block");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge canvas block");
    db.create_canvas_board(&ctx, workspace_id, &block.block_id, board_state(0.0, 0.0, 1.0))
        .await
        .expect("create canvas board");
    block.block_id
}

#[tokio::test]
async fn canvas_is_a_loom_block_with_knowledge_bridge() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let canvas_id = make_canvas(&pg.db, &ws, "Project map").await;

    // The canvas board's block IS a content_type='canvas' LoomBlock.
    let block = pg.db.get_loom_block(&ws, &canvas_id).await.expect("get block");
    assert!(matches!(block.content_type, LoomBlockContentType::Canvas));

    // It is authority-resolved through the ProjectKnowledgeIndex bridge.
    let bridge = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &canvas_id)
        .await
        .expect("read bridge")
        .expect("bridge exists for canvas block");
    let entity = pg
        .db
        .get_knowledge_entity(&bridge.entity_id)
        .await
        .expect("get entity")
        .expect("entity exists");
    assert!(matches!(entity.entity_kind, KnowledgeEntityKind::LoomBlock));

    // The board row carries an EventLedger receipt.
    let view = pg.db.get_canvas_board(&ws, &canvas_id).await.expect("get board");
    assert!(!view.board.event_ledger_event_id.is_empty());
    assert_eq!(
        view.board.board_state.get("schema_id").and_then(|v| v.as_str()),
        Some(LOOM_CANVAS_BOARD_SCHEMA_ID)
    );
}

#[tokio::test]
async fn board_placements_viewport_and_visual_edges_round_trip() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Round trip").await;
    let a = make_block(&pg.db, &ws, "Alpha note", LoomBlockContentType::Note).await;
    let b = make_block(&pg.db, &ws, "Beta note", LoomBlockContentType::Note).await;

    let pa = pg
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 10.0,
                y: 20.0,
                w: 200.0,
                h: 120.0,
                z_index: 0,
                group_id: None,
            },
        )
        .await
        .expect("place a");
    let pb = pg
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: b.clone(),
                x: 300.0,
                y: 40.0,
                w: 200.0,
                h: 120.0,
                z_index: 1,
                group_id: Some("g1".to_string()),
            },
        )
        .await
        .expect("place b");

    // Move + resize + group a placement.
    pg.db
        .update_canvas_placement(
            &ctx,
            &ws,
            &pa.placement_id,
            LoomCanvasPlacementUpdate {
                x: Some(15.0),
                y: Some(25.0),
                w: Some(220.0),
                h: None,
                z_index: Some(5),
                group_id: Some(Some("g1".to_string())),
            },
        )
        .await
        .expect("move a");

    // A board-local visual-only edge between the two placements.
    let ve = pg
        .db
        .add_canvas_visual_edge(
            &ctx,
            &ws,
            &canvas_id,
            &pa.placement_id,
            &pb.placement_id,
            Some("see also".to_string()),
        )
        .await
        .expect("add visual edge");

    // Persist a new viewport.
    pg.db
        .update_canvas_board_state(&ctx, &ws, &canvas_id, board_state(120.5, -40.0, 1.75))
        .await
        .expect("update viewport");

    // Reload everything from PostgreSQL.
    let view = pg.db.get_canvas_board(&ws, &canvas_id).await.expect("reload");
    assert_eq!(view.placements.len(), 2);
    let reloaded_a = view
        .placements
        .iter()
        .find(|p| p.placement_id == pa.placement_id)
        .expect("a reloaded");
    assert_eq!(reloaded_a.x, 15.0);
    assert_eq!(reloaded_a.w, 220.0);
    assert_eq!(reloaded_a.h, 120.0);
    assert_eq!(reloaded_a.z_index, 5);
    assert_eq!(reloaded_a.group_id.as_deref(), Some("g1"));

    assert_eq!(view.visual_edges.len(), 1);
    assert_eq!(view.visual_edges[0].visual_edge_id, ve.visual_edge_id);
    assert_eq!(view.visual_edges[0].label.as_deref(), Some("see also"));

    assert_eq!(view.board.board_state.get("zoom").and_then(|v| v.as_f64()), Some(1.75));
    assert_eq!(view.board.board_state.get("pan_x").and_then(|v| v.as_f64()), Some(120.5));
}

#[tokio::test]
async fn remove_placement_keeps_source_block() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Negative proof").await;
    let a = make_block(&pg.db, &ws, "Survivor", LoomBlockContentType::Note).await;

    let pa = pg
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
            },
        )
        .await
        .expect("place");

    pg.db
        .remove_canvas_placement(&ctx, &ws, &pa.placement_id)
        .await
        .expect("remove placement");

    // The placement is gone, but the SOURCE block survives (reference-not-copy).
    let view = pg.db.get_canvas_board(&ws, &canvas_id).await.expect("reload");
    assert!(view.placements.is_empty());
    let survivor = pg.db.get_loom_block(&ws, &a).await.expect("source block survives");
    assert_eq!(survivor.title.as_deref(), Some("Survivor"));
}

#[tokio::test]
async fn deleting_canvas_keeps_placed_blocks() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Doomed canvas").await;
    let a = make_block(&pg.db, &ws, "Independent A", LoomBlockContentType::Note).await;
    let b = make_block(&pg.db, &ws, "Independent B", LoomBlockContentType::Note).await;

    for (blk, x) in [(&a, 0.0), (&b, 200.0)] {
        pg.db
            .place_block_on_canvas(
                &ctx,
                NewLoomCanvasPlacement {
                    canvas_block_id: canvas_id.clone(),
                    workspace_id: ws.clone(),
                    placed_block_id: blk.clone(),
                    x,
                    y: 0.0,
                    w: 100.0,
                    h: 100.0,
                    z_index: 0,
                    group_id: None,
                },
            )
            .await
            .expect("place");
    }

    // Delete the canvas LoomBlock. The board + placements CASCADE; the placed
    // blocks are untouched (placements FK placed_block_id ON DELETE RESTRICT,
    // and the cascade only deletes placement rows, never loom_blocks).
    pg.db
        .delete_loom_block(&ctx, &ws, &canvas_id)
        .await
        .expect("delete canvas block");

    assert!(pg.db.get_loom_block(&ws, &canvas_id).await.is_err());
    assert!(pg.db.get_canvas_board(&ws, &canvas_id).await.is_err());
    // The placed blocks live on.
    assert_eq!(
        pg.db.get_loom_block(&ws, &a).await.expect("a survives").title.as_deref(),
        Some("Independent A")
    );
    assert_eq!(
        pg.db.get_loom_block(&ws, &b).await.expect("b survives").title.as_deref(),
        Some("Independent B")
    );
}

#[tokio::test]
async fn editing_source_block_reflects_through_placement() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Live ref").await;
    let a = make_block(&pg.db, &ws, "Original title", LoomBlockContentType::Note).await;

    pg.db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
            },
        )
        .await
        .expect("place");

    // Edit the SOURCE block (not the placement).
    pg.db
        .update_loom_block(
            &ctx,
            &ws,
            &a,
            LoomBlockUpdate {
                title: Some("Edited title".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("edit source");

    // The placement references the same block id; resolving it reads the LIVE
    // (edited) content — proof there is no content copy on the placement.
    let view = pg.db.get_canvas_board(&ws, &canvas_id).await.expect("reload");
    assert_eq!(view.placements.len(), 1);
    let placed_id = &view.placements[0].placed_block_id;
    let live = pg.db.get_loom_block(&ws, placed_id).await.expect("live block");
    assert_eq!(live.title.as_deref(), Some("Edited title"));
}

#[tokio::test]
async fn semantic_edge_in_graph_but_visual_only_edge_is_not() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Edge kinds").await;
    let a = make_block(&pg.db, &ws, "Node A", LoomBlockContentType::Note).await;
    let b = make_block(&pg.db, &ws, "Node B", LoomBlockContentType::Note).await;

    let pa = pg
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
            },
        )
        .await
        .expect("place a");
    let pb = pg
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: b.clone(),
                x: 200.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
            },
        )
        .await
        .expect("place b");

    // A SEMANTIC connection is a real loom_edge (the FE delegates to the
    // existing create_loom_edge path).
    pg.db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: a.clone(),
                target_block_id: b.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("create semantic edge");

    // A VISUAL-ONLY edge is board-local decoration — never a loom_edge.
    pg.db
        .add_canvas_visual_edge(&ctx, &ws, &canvas_id, &pa.placement_id, &pb.placement_id, None)
        .await
        .expect("add visual edge");

    // The semantic edge shows up in the local Loom graph from A.
    let graph = pg
        .db
        .local_graph(&ws, &a, 3, &[], 200)
        .await
        .expect("local graph");
    let semantic_present = graph
        .edges
        .iter()
        .any(|e| e.edge.source_block_id == a && e.edge.target_block_id == b);
    assert!(semantic_present, "semantic mention edge must appear in the graph");

    // The visual-only edge must NOT appear as any loom_edge in the graph.
    let edge_count_a_b = graph
        .edges
        .iter()
        .filter(|e| {
            (e.edge.source_block_id == a && e.edge.target_block_id == b)
                || (e.edge.source_block_id == b && e.edge.target_block_id == a)
        })
        .count();
    assert_eq!(
        edge_count_a_b, 1,
        "only the semantic edge should be in the graph; the visual-only edge is not graph authority"
    );

    // And the visual edge is still present on the BOARD projection.
    let view = pg.db.get_canvas_board(&ws, &canvas_id).await.expect("reload");
    assert_eq!(view.visual_edges.len(), 1);
}

#[tokio::test]
async fn free_text_card_is_a_real_note_block() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let _canvas_id = make_canvas(&pg.db, &ws, "Card host").await;

    // import_markdown_to_loom is the storage path the /cards endpoint uses: it
    // creates a real RichDocument + note LoomBlock + knowledge bridge.
    let imported = pg
        .db
        .import_markdown_to_loom(&ctx, &ws, "Idea card", "A free-text **idea**.")
        .await
        .expect("create card");
    assert!(matches!(imported.block.content_type, LoomBlockContentType::Note));
    assert!(!imported.rich_document_id.is_empty());

    // The card block is real authority: it round-trips from PostgreSQL.
    let block = pg
        .db
        .get_loom_block(&ws, &imported.block.block_id)
        .await
        .expect("card block exists");
    assert_eq!(block.title.as_deref(), Some("Idea card"));

    // And the backing RichDocument is real authority too.
    let doc = pg
        .db
        .get_knowledge_rich_document(&imported.rich_document_id)
        .await
        .expect("get rich doc")
        .expect("rich doc exists");
    assert_eq!(doc.title, "Idea card");
}
