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
    CompensateLoomCanvasStageCard, Database, LoomBlockContentType, LoomBlockDerived,
    LoomBlockUpdate, LoomCanvasPlacementUpdate, LoomCanvasStageProvenance, LoomEdgeCreatedBy,
    LoomEdgeType, LoomSearchResultKind, LoomSearchSourceKind, NewLoomBlock,
    NewLoomCanvasPlacement, NewLoomCanvasStageCard, NewLoomEdge, QuickSwitcherRecentInput,
    WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID, LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
};
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Connection;
use std::time::Duration;

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

async fn stage_provenance(
    pg: &KnowledgePg,
    workspace_id: &str,
    suffix: &str,
) -> LoomCanvasStageProvenance {
    let artifact_digest = format!("{:x}", Sha256::digest(suffix.as_bytes()));
    let artifact_id = format!("STGA-{}", &artifact_digest[..32]);
    let content_bytes = suffix.as_bytes();
    let sha256 = format!("{:x}", Sha256::digest(content_bytes));
    let manifest_ref = format!("manifest://{artifact_id}");
    let causal_action_id = format!("stage-causal-{suffix}");
    let provenance = LoomCanvasStageProvenance {
        schema_id: LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA.to_owned(),
        artifact_id: artifact_id.clone(),
        sha256: sha256.clone(),
        manifest_ref: manifest_ref.clone(),
        causal_action_id: causal_action_id.clone(),
    };
    let mut conn = pg.raw_connection().await;
    sqlx::query(
        r#"
        INSERT INTO stage_capture_artifacts (
            artifact_id, workspace_id, content_kind, label, content_type,
            content_json, content_bytes, size_bytes, content_sha256, manifest,
            manifest_ref, source_ref, idempotency_key, request_hash,
            actor_kind, actor_id, correlation_id, approval_id
        ) VALUES (
            $1, $2, 'canvas_node', $3, 'text/plain',
            jsonb_build_object('text', $3::text), $4, $5, $6,
            jsonb_build_object(
                'schema', 'hsk.stage.capture_manifest@1',
                'sha256', $6::text,
                'manifest_ref', $7::text,
                'content_type', 'text/plain',
                'size_bytes', $5::bigint
            ),
            $7, NULL, $8, $6, 'operator', 'loom-canvas-stage-test', $9, 'test-approval'
        )
        "#,
    )
    .bind(&artifact_id)
    .bind(workspace_id)
    .bind(suffix)
    .bind(content_bytes)
    .bind(content_bytes.len() as i64)
    .bind(&sha256)
    .bind(&manifest_ref)
    .bind(format!("loom-canvas-stage-{suffix}"))
    .bind(&causal_action_id)
    .execute(&mut conn)
    .await
    .expect("seed authoritative Stage capture artifact");
    provenance
}

fn stage_provenance_key(provenance: &LoomCanvasStageProvenance) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(provenance).expect("serialize Stage provenance"))
    )
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
                is_text_card: false,
                stage_provenance_key: None,
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
                is_text_card: false,
                stage_provenance_key: None,
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
                is_text_card: false,
                stage_provenance_key: None,
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
                    is_text_card: false,
                    stage_provenance_key: None,
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
                is_text_card: false,
                stage_provenance_key: None,
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
                is_text_card: false,
                stage_provenance_key: None,
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
                is_text_card: false,
                stage_provenance_key: None,
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

#[tokio::test]
async fn stage_canvas_compensation_is_atomic_and_idempotent() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Stage compensation").await;
    let provenance = stage_provenance(&pg, &ws, "atomic").await;
    let key = stage_provenance_key(&provenance);
    let markdown = format!(
        "{{\n  \"causal_action_id\": \"{}\", \"manifest_ref\": \"{}\",\n  \"sha256\": \"{}\", \"artifact_id\": \"{}\",\n  \"schema_id\": \"{}\"\n}}",
        provenance.causal_action_id,
        provenance.manifest_ref,
        provenance.sha256,
        provenance.artifact_id,
        provenance.schema_id,
    );
    let created = pg
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown,
                stage_provenance_key: key.clone(),
                stage_provenance: provenance.clone(),
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .expect("create Stage Canvas card");
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        placement_id: created.placement.placement_id.clone(),
        placed_block_id: created.block.block_id.clone(),
        stage_provenance_key: key,
        stage_provenance: provenance,
    };

    let first = pg
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect("first compensation commits");
    assert!(first.removed_by_request);
    let second = pg
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect("lost-response retry reconciles complete absence");
    assert!(!second.removed_by_request);

    let mut conn = pg.raw_connection().await;
    for (table, column, identity) in [
        ("loom_canvas_placements", "placement_id", receipt.placement_id.as_str()),
        ("knowledge_rich_documents", "rich_document_id", receipt.placed_block_id.as_str()),
        ("loom_blocks", "block_id", receipt.placed_block_id.as_str()),
        ("loom_block_knowledge_bridge", "block_id", receipt.placed_block_id.as_str()),
        ("loom_block_search_index", "block_id", receipt.placed_block_id.as_str()),
    ] {
        let statement = format!("SELECT COUNT(*) FROM {table} WHERE {column} = $1");
        let count: i64 = sqlx::query_scalar(&statement)
        .bind(identity)
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert_eq!(count, 0, "{table} compensation residue");
    }
    let entity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_entities WHERE workspace_id = $1 AND entity_kind = 'loom_block' AND entity_key = $2",
    )
    .bind(&ws)
    .bind(&receipt.placed_block_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(entity_count, 0, "knowledge entity projection residue");

    let compensation_events: Vec<(String, String, Value)> = sqlx::query_as(
        r#"
        SELECT event_type, correlation_id, payload
        FROM kernel_event_ledger
        WHERE source_component = 'loom_canvas_stage_compensation'
          AND aggregate_type = 'knowledge_rich_document'
          AND aggregate_id = $1
        "#,
    )
    .bind(&receipt.placed_block_id)
    .fetch_all(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        compensation_events.len(),
        1,
        "successful compensation and its replay append exactly one audit event"
    );
    let (event_type, correlation_id, payload) = &compensation_events[0];
    assert_eq!(event_type, "KNOWLEDGE_RICH_DOCUMENT_DELETED");
    assert_eq!(correlation_id, &receipt.stage_provenance.causal_action_id);
    assert_eq!(payload["workspace_id"], ws);
    assert_eq!(payload["canvas_block_id"], canvas_id);
    assert_eq!(payload["placement_id"], receipt.placement_id);
    assert_eq!(payload["block_id"], receipt.placed_block_id);
    assert_eq!(
        payload["title"],
        format!("Stage capture {}", receipt.stage_provenance.artifact_id)
    );
    assert_eq!(payload["artifact_id"], receipt.stage_provenance.artifact_id);
    assert_eq!(payload["stage_provenance_key"], receipt.stage_provenance_key);
}

#[tokio::test]
async fn stage_canvas_create_requires_exact_stage_authority_before_create_and_replay() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Stage authority gate").await;

    let fabricated = LoomCanvasStageProvenance {
        schema_id: LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA.to_owned(),
        artifact_id: format!("STGA-{}", "1".repeat(32)),
        sha256: "2".repeat(64),
        manifest_ref: "manifest://fabricated-stage-artifact".to_owned(),
        causal_action_id: "fabricated-stage-causal-action".to_owned(),
    };
    let fabricated_key = stage_provenance_key(&fabricated);
    assert!(
        pg.db
            .create_stage_canvas_card(
                &ctx,
                NewLoomCanvasStageCard {
                    canvas_block_id: canvas_id.clone(),
                    workspace_id: ws.clone(),
                    title: format!("Stage capture {}", fabricated.artifact_id),
                    markdown: serde_json::to_string(&fabricated).unwrap(),
                    stage_provenance_key: fabricated_key,
                    stage_provenance: fabricated,
                    x: 0.0,
                    y: 0.0,
                    w: 300.0,
                    h: 180.0,
                    z_index: 0,
                },
            )
            .await
            .is_err(),
        "structurally valid but unauthoritative Stage provenance must fail closed"
    );

    let authoritative = stage_provenance(&pg, &ws, "authority-gate").await;
    let mut mismatches = Vec::new();
    let mut sha_mismatch = authoritative.clone();
    sha_mismatch.sha256 = "f".repeat(64);
    mismatches.push(sha_mismatch);
    let mut manifest_mismatch = authoritative.clone();
    manifest_mismatch.manifest_ref = "manifest://wrong-authority".to_owned();
    mismatches.push(manifest_mismatch);
    let mut correlation_mismatch = authoritative.clone();
    correlation_mismatch.causal_action_id = "wrong-stage-causal-action".to_owned();
    mismatches.push(correlation_mismatch);

    for mismatch in mismatches {
        let mismatch_key = stage_provenance_key(&mismatch);
        assert!(
            pg.db
                .create_stage_canvas_card(
                    &ctx,
                    NewLoomCanvasStageCard {
                        canvas_block_id: canvas_id.clone(),
                        workspace_id: ws.clone(),
                        title: format!("Stage capture {}", mismatch.artifact_id),
                        markdown: serde_json::to_string(&mismatch).unwrap(),
                        stage_provenance_key: mismatch_key,
                        stage_provenance: mismatch,
                        x: 0.0,
                        y: 0.0,
                        w: 300.0,
                        h: 180.0,
                        z_index: 0,
                    },
                )
                .await
                .is_err(),
            "every Stage authority tuple field must match exactly"
        );
    }

    let mut conn = pg.raw_connection().await;
    let residue_before_valid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_canvas_placements WHERE workspace_id = $1 AND canvas_block_id = $2",
    )
    .bind(&ws)
    .bind(&canvas_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(residue_before_valid, 0, "failed authority checks leave no placement residue");
    let document_residue: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_rich_documents WHERE workspace_id = $1 AND title LIKE 'Stage capture %'",
    )
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(document_residue, 0, "failed authority checks leave no document residue");

    let key = stage_provenance_key(&authoritative);
    let reordered_markdown = format!(
        "{{ \"manifest_ref\": \"{}\", \"causal_action_id\": \"{}\", \"artifact_id\": \"{}\", \"schema_id\": \"{}\", \"sha256\": \"{}\" }}",
        authoritative.manifest_ref,
        authoritative.causal_action_id,
        authoritative.artifact_id,
        authoritative.schema_id,
        authoritative.sha256,
    );
    let card = NewLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        title: format!("Stage capture {}", authoritative.artifact_id),
        markdown: reordered_markdown,
        stage_provenance_key: key.clone(),
        stage_provenance: authoritative.clone(),
        x: 0.0,
        y: 0.0,
        w: 300.0,
        h: 180.0,
        z_index: 0,
    };
    let created = pg
        .db
        .create_stage_canvas_card(&ctx, card.clone())
        .await
        .expect("exact authoritative tuple creates despite caller JSON ordering");

    sqlx::query(
        "UPDATE stage_capture_artifacts SET correlation_id = 'authority-changed-after-create' WHERE workspace_id = $1 AND artifact_id = $2",
    )
    .bind(&ws)
    .bind(&authoritative.artifact_id)
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db.create_stage_canvas_card(&ctx, card).await.is_err(),
        "replay must re-prove current Stage authority before returning the existing card"
    );
    let placement_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_canvas_placements WHERE workspace_id = $1 AND canvas_block_id = $2 AND stage_provenance_key = $3",
    )
    .bind(&ws)
    .bind(&canvas_id)
    .bind(&key)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(placement_count, 1, "failed replay authority check never duplicates authority");

    sqlx::query(
        "UPDATE stage_capture_artifacts SET correlation_id = $1 WHERE workspace_id = $2 AND artifact_id = $3",
    )
    .bind(&authoritative.causal_action_id)
    .bind(&ws)
    .bind(&authoritative.artifact_id)
    .execute(&mut conn)
    .await
    .unwrap();
    pg.db
        .compensate_stage_canvas_card(
            &ctx,
            CompensateLoomCanvasStageCard {
                canvas_block_id: canvas_id,
                workspace_id: ws,
                placement_id: created.placement.placement_id,
                placed_block_id: created.block.block_id,
                stage_provenance_key: key,
                stage_provenance: authoritative,
            },
        )
        .await
        .expect("canonical typed provenance remains compensatable");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_canvas_compensation_serializes_with_concurrent_logical_reference_writer() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Stage logical-reference race").await;
    let provenance = stage_provenance(&pg, &ws, "logical-reference-race").await;
    let key = stage_provenance_key(&provenance);
    let created = pg
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown: serde_json::to_string(&provenance).unwrap(),
                stage_provenance_key: key.clone(),
                stage_provenance: provenance.clone(),
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .unwrap();
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        placement_id: created.placement.placement_id.clone(),
        placed_block_id: created.block.block_id.clone(),
        stage_provenance_key: key,
        stage_provenance: provenance,
    };

    let mut writer = pg.raw_connection().await;
    let mut writer_tx = writer.begin().await.unwrap();
    let proposal_id = format!("FMP-{}", "1".repeat(32));
    sqlx::query(
        "INSERT INTO fems_memory_proposals (proposal_id, request_id, workspace_id, document_id, selection_start, selection_end, content_hash, memory_class, proposal) VALUES ($1, $2, $3, $4, 0, 0, $5, 'fact', '{}'::jsonb)",
    )
    .bind(&proposal_id)
    .bind(format!("concurrent-stage-reference-{}", uuid::Uuid::now_v7()))
    .bind(&ws)
    .bind(&receipt.placed_block_id)
    .bind("1".repeat(64))
    .execute(&mut *writer_tx)
    .await
    .unwrap();

    let compensation_db =
        handshake_core::storage::postgres::PostgresDatabase::connect(&pg.schema_url, 2)
            .await
            .unwrap();
    let compensation_ctx = ctx.clone();
    let compensation_receipt = receipt.clone();
    let mut compensation = tokio::spawn(async move {
        compensation_db
            .compensate_stage_canvas_card(&compensation_ctx, compensation_receipt)
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut compensation)
            .await
            .is_err(),
        "compensation must wait for the in-flight logical-reference writer"
    );
    writer_tx.commit().await.unwrap();
    assert!(
        compensation.await.unwrap().is_err(),
        "after the writer commits, compensation must observe and preserve its reference"
    );

    assert_eq!(
        pg.db
            .get_canvas_board(&ws, &canvas_id)
            .await
            .unwrap()
            .placements
            .len(),
        1,
        "the concurrent reference and the Stage card both remain"
    );
    let mut conn = pg.raw_connection().await;
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE source_component = 'loom_canvas_stage_compensation' AND aggregate_id = $1",
    )
    .bind(&receipt.placed_block_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(audit_count, 0, "refused concurrent compensation appends no audit event");
    sqlx::query("DELETE FROM fems_memory_proposals WHERE proposal_id = $1")
        .bind(&proposal_id)
        .execute(&mut conn)
        .await
        .unwrap();
    pg.db
        .compensate_stage_canvas_card(&ctx, receipt)
        .await
        .expect("cleanup compensation succeeds after logical reference removal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_canvas_compensation_first_makes_waiting_and_later_logical_writers_fail() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Stage compensation-first race").await;
    let provenance = stage_provenance(&pg, &ws, "compensation-first-race").await;
    let key = stage_provenance_key(&provenance);
    let created = pg
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown: serde_json::to_string(&provenance).unwrap(),
                stage_provenance_key: key.clone(),
                stage_provenance: provenance.clone(),
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .unwrap();
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        placement_id: created.placement.placement_id.clone(),
        placed_block_id: created.block.block_id.clone(),
        stage_provenance_key: key,
        stage_provenance: provenance,
    };
    let bridge = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &receipt.placed_block_id)
        .await
        .unwrap()
        .unwrap();
    let backlink_source = pg
        .db
        .import_markdown_to_loom(
            &ctx,
            &ws,
            "Compensation-first backlink source",
            "source",
        )
        .await
        .unwrap();
    let stage_title = format!("Stage capture {}", receipt.stage_provenance.artifact_id);
    let edge_source = make_block(
        &pg.db,
        &ws,
        "Post-compensation edge source",
        LoomBlockContentType::Note,
    )
    .await;
    let edge_target = make_block(
        &pg.db,
        &ws,
        "Post-compensation edge target",
        LoomBlockContentType::Note,
    )
    .await;
    let context_hash = "5".repeat(64);
    let bundle_id = format!("CTX-{}", &context_hash[..16]);
    let mut setup = pg.raw_connection().await;
    sqlx::query(
        "INSERT INTO knowledge_context_bundles (bundle_id, workspace_id, kernel_task_run_id, session_run_id, allowed_context, context_hash) VALUES ($1, $2, 'KTR-stage-compensation-first', 'SR-stage-compensation-first', '[]'::jsonb, $3)",
    )
    .bind(&bundle_id)
    .bind(&ws)
    .bind(&context_hash)
    .execute(&mut setup)
    .await
    .unwrap();

    // The placement row is later in compensation's lock order than the
    // logical-reference identities. Holding it pauses compensation after it
    // owns the exclusive block/entity locks but before tombstone + deletion.
    let mut blocker = pg.raw_connection().await;
    let mut blocker_tx = blocker.begin().await.unwrap();
    sqlx::query("SELECT placement_id FROM loom_canvas_placements WHERE placement_id = $1 FOR UPDATE")
        .bind(&receipt.placement_id)
        .fetch_one(&mut *blocker_tx)
        .await
        .unwrap();

    let compensation_db =
        handshake_core::storage::postgres::PostgresDatabase::connect(&pg.schema_url, 2)
            .await
            .unwrap();
    let compensation_ctx = ctx.clone();
    let compensation_receipt = receipt.clone();
    let mut compensation = tokio::spawn(async move {
        compensation_db
            .compensate_stage_canvas_card(&compensation_ctx, compensation_receipt)
            .await
    });

    let logical_lock_keys = [
        format!(
            "stage-logical-ref\u{1f}{}\u{1f}block:{}",
            ws, receipt.placed_block_id
        ),
        format!("stage-logical-ref\u{1f}{ws}\u{1f}title:{stage_title}"),
    ];
    let mut probe = pg.raw_connection().await;
    for logical_lock_key in &logical_lock_keys {
        let mut compensation_holds_lock = false;
        for _ in 0..500 {
            let shared_lock_acquired: bool = sqlx::query_scalar(
                "SELECT pg_try_advisory_xact_lock_shared(hashtextextended($1, 32066::bigint))",
            )
            .bind(logical_lock_key)
            .fetch_one(&mut probe)
            .await
            .unwrap();
            if !shared_lock_acquired {
                compensation_holds_lock = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            compensation_holds_lock,
            "compensation must own logical-reference lock {logical_lock_key:?} before writers start"
        );
    }

    let writer_workspace = ws.clone();
    let writer_document = receipt.placed_block_id.clone();
    let proposal_id = format!("FMP-{}", "6".repeat(32));
    let writer_proposal_id = proposal_id.clone();
    let mut writer = sqlx::PgConnection::connect(&pg.schema_url).await.unwrap();
    let (writer_started_tx, writer_started_rx) = tokio::sync::oneshot::channel();
    let mut waiting_writer = tokio::spawn(async move {
        let _ = writer_started_tx.send(());
        sqlx::query(
            "INSERT INTO fems_memory_proposals (proposal_id, request_id, workspace_id, document_id, selection_start, selection_end, content_hash, memory_class, proposal) VALUES ($1, $2, $3, $4, 0, 0, $5, 'fact', '{}'::jsonb)",
        )
        .bind(&writer_proposal_id)
        .bind(format!("compensation-first-writer-{}", uuid::Uuid::now_v7()))
        .bind(&writer_workspace)
        .bind(&writer_document)
        .bind("6".repeat(64))
        .execute(&mut writer)
        .await
    });
    writer_started_rx.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut waiting_writer)
            .await
            .is_err(),
        "a compensation-first logical-reference writer must wait on the shared lock"
    );

    let backlink_id_target = format!("KDBL-{}", "a".repeat(32));
    let backlink_id_title = format!("KDBL-{}", "b".repeat(32));
    let mut backlink_id_writer = sqlx::PgConnection::connect(&pg.schema_url).await.unwrap();
    let backlink_workspace = ws.clone();
    let backlink_source_id = backlink_source.rich_document_id.clone();
    let backlink_target_id = receipt.placed_block_id.clone();
    let backlink_id_target_task = backlink_id_target.clone();
    let (id_writer_started_tx, id_writer_started_rx) = tokio::sync::oneshot::channel();
    let mut waiting_id_backlink = tokio::spawn(async move {
        let _ = id_writer_started_tx.send(());
        sqlx::query(
            "INSERT INTO knowledge_document_backlinks (backlink_id, workspace_id, relationship_id, source_document_id, link_kind, target, block_id) VALUES ($1, $2, $3, $4, 'wikilink', $5, 'body.0')",
        )
        .bind(&backlink_id_target_task)
        .bind(&backlink_workspace)
        .bind(format!("KDLNK-{}", "a".repeat(64)))
        .bind(&backlink_source_id)
        .bind(&backlink_target_id)
        .execute(&mut backlink_id_writer)
        .await
    });
    id_writer_started_rx.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut waiting_id_backlink)
            .await
            .is_err(),
        "direct SQL backlink target=id must wait behind compensation"
    );

    let mut backlink_title_writer = sqlx::PgConnection::connect(&pg.schema_url).await.unwrap();
    let backlink_workspace = ws.clone();
    let backlink_source_id = backlink_source.rich_document_id.clone();
    let backlink_target_title = stage_title.clone();
    let backlink_id_title_task = backlink_id_title.clone();
    let (title_writer_started_tx, title_writer_started_rx) = tokio::sync::oneshot::channel();
    let mut waiting_title_backlink = tokio::spawn(async move {
        let _ = title_writer_started_tx.send(());
        sqlx::query(
            "INSERT INTO knowledge_document_backlinks (backlink_id, workspace_id, relationship_id, source_document_id, link_kind, target, block_id) VALUES ($1, $2, $3, $4, 'wikilink', $5, 'body.1')",
        )
        .bind(&backlink_id_title_task)
        .bind(&backlink_workspace)
        .bind(format!("KDLNK-{}", "b".repeat(64)))
        .bind(&backlink_source_id)
        .bind(&backlink_target_title)
        .execute(&mut backlink_title_writer)
        .await
    });
    title_writer_started_rx.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut waiting_title_backlink)
            .await
            .is_err(),
        "direct SQL backlink target=title must wait behind compensation"
    );

    let quick_recent_hit_key = format!("loom_block:{}", receipt.placed_block_id);
    let mut quick_recent_writer = sqlx::PgConnection::connect(&pg.schema_url).await.unwrap();
    let quick_recent_workspace = ws.clone();
    let quick_recent_ref_id = receipt.placed_block_id.clone();
    let quick_recent_event_id = bridge.index_event_id.clone();
    let (recent_writer_started_tx, recent_writer_started_rx) = tokio::sync::oneshot::channel();
    let mut waiting_quick_recent = tokio::spawn(async move {
        let _ = recent_writer_started_tx.send(());
        sqlx::query(
            "INSERT INTO knowledge_quick_switcher_recents (workspace_id, hit_key, source_kind, ref_id, result_kind, title, event_ledger_event_id) VALUES ($1, $2, 'loom_block', $3, 'loom_block', 'Queued Stage recent', $4)",
        )
        .bind(&quick_recent_workspace)
        .bind(format!("loom_block:{quick_recent_ref_id}"))
        .bind(&quick_recent_ref_id)
        .bind(&quick_recent_event_id)
        .execute(&mut quick_recent_writer)
        .await
    });
    recent_writer_started_rx.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut waiting_quick_recent)
            .await
            .is_err(),
        "direct SQL quick-switcher LoomBlock recent must wait behind compensation"
    );

    blocker_tx.commit().await.unwrap();
    let compensated = compensation.await.unwrap().unwrap();
    assert!(compensated.removed_by_request);
    let writer_error = waiting_writer
        .await
        .unwrap()
        .expect_err("writer must revalidate and fail after compensation commits");
    assert_eq!(
        writer_error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23503")
    );
    for (label, writer_result) in [
        (
            "target=id",
            waiting_id_backlink
                .await
                .unwrap()
                .expect_err("target=id backlink must fail after compensation"),
        ),
        (
            "target=title",
            waiting_title_backlink
                .await
                .unwrap()
                .expect_err("target=title backlink must fail after compensation"),
        ),
    ] {
        assert_eq!(
            writer_result
                .as_database_error()
                .and_then(|database_error| database_error.code())
                .as_deref(),
            Some("23503"),
            "queued direct SQL backlink {label} must revalidate the tombstone"
        );
    }
    let quick_recent_error = waiting_quick_recent
        .await
        .unwrap()
        .expect_err("queued quick-switcher recent must fail after compensation");
    assert_eq!(
        quick_recent_error
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref(),
        Some("23503")
    );

    let mut conn = pg.raw_connection().await;
    let proposal_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fems_memory_proposals WHERE proposal_id = $1")
            .bind(&proposal_id)
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(proposal_count, 0, "the queued writer leaves no dangling row");
    let backlink_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_document_backlinks WHERE backlink_id IN ($1, $2)",
    )
    .bind(&backlink_id_target)
    .bind(&backlink_id_title)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(backlink_count, 0, "queued backlink writers leave no dangling rows");
    let queued_recent_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_quick_switcher_recents WHERE workspace_id = $1 AND hit_key = $2",
    )
    .bind(&ws)
    .bind(&quick_recent_hit_key)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        queued_recent_count, 0,
        "queued quick-switcher writer leaves no dangling recent"
    );

    let runtime_recent_error = pg
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::LoomBlock,
                source_kind: LoomSearchSourceKind::LoomBlock,
                ref_id: receipt.placed_block_id.clone(),
                title: "Post-compensation Stage recent".to_owned(),
                excerpt: String::new(),
                metadata: json!({"proof": "post_compensation_runtime"}),
            },
        )
        .await
        .expect_err("runtime writer must reject compensated LoomBlock identity");
    assert!(
        matches!(runtime_recent_error, handshake_core::storage::StorageError::Database(_)),
        "database trigger must reject the normal runtime writer"
    );

    let entity_recent_error = sqlx::query(
        "INSERT INTO knowledge_quick_switcher_recents (workspace_id, hit_key, source_kind, ref_id, result_kind, title, event_ledger_event_id) VALUES ($1, $2, 'symbol', $3, 'knowledge_entity', 'Compensated Stage entity recent', $4)",
    )
    .bind(&ws)
    .bind(format!("symbol:{}", bridge.entity_id))
    .bind(&bridge.entity_id)
    .bind(&bridge.index_event_id)
    .execute(&mut conn)
    .await
    .expect_err("direct knowledge-entity recent must reject compensated entity identity");
    assert_eq!(
        entity_recent_error
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref(),
        Some("23503")
    );

    for (label, result) in [
        (
            "rich-document KnowledgeSource",
            sqlx::query(
                "INSERT INTO knowledge_sources (source_id, workspace_id, source_kind, content_hash, provenance) VALUES ($1, $2, 'rich_document', $3, jsonb_build_object('rich_document_id', $4::text))",
            )
            .bind(format!("KSRC-{}", "7".repeat(32)))
            .bind(&ws)
            .bind("7".repeat(64))
            .bind(&receipt.placed_block_id)
            .execute(&mut conn)
            .await,
        ),
        (
            "entity context item",
            sqlx::query(
                "INSERT INTO knowledge_context_bundle_items (bundle_id, item_ordinal, ref_kind, ref_id, retrieval_decision) VALUES ($1, 0, 'entity', $2, 'included')",
            )
            .bind(&bundle_id)
            .bind(&bridge.entity_id)
            .execute(&mut conn)
            .await,
        ),
        (
            "Loom AI block suggestion",
            sqlx::query(
                "INSERT INTO loom_ai_suggestions (suggestion_id, job_id, workspace_id, kind, block_id, suggested_value, model_attribution, prompt_sha256, output_sha256, recorded_event_id, value_hash) VALUES ($1, $2, $3, 'auto_caption', $4, '{\"caption\":\"dangling\"}'::jsonb, '{\"model\":\"test\"}'::jsonb, $5, $6, $7, $8)",
            )
            .bind(format!("LAIS-{}", "8".repeat(32)))
            .bind(format!("LAIJ-{}", "8".repeat(32)))
            .bind(&ws)
            .bind(&receipt.placed_block_id)
            .bind("8".repeat(64))
            .bind("9".repeat(64))
            .bind(&bridge.index_event_id)
            .bind("a".repeat(64))
            .execute(&mut conn)
            .await,
        ),
        (
            "Loom edge source-text reference",
            sqlx::query(
                "INSERT INTO loom_edges (edge_id, workspace_id, source_block_id, target_block_id, edge_type, created_by, source_text_block_id) VALUES ($1, $2, $3, $4, 'mention', 'user', $5)",
            )
            .bind(format!("LE-{}", "9".repeat(32)))
            .bind(&ws)
            .bind(&edge_source)
            .bind(&edge_target)
            .bind(&receipt.placed_block_id)
            .execute(&mut conn)
            .await,
        ),
    ] {
        let error = result.expect_err(label);
        assert_eq!(
            error
                .as_database_error()
                .and_then(|database_error| database_error.code())
                .as_deref(),
            Some("23503"),
            "{label} must reject the compensated identity"
        );
    }

    // A title tombstone protects only the absence left by compensation. The
    // same authoritative Stage artifact may be intentionally embedded again,
    // producing a new live RichDocument with the deterministic title. Once
    // that live target exists, title backlinks must recover normally while
    // old block-id references remain protected by their exact tombstone.
    let reembedded_provenance = receipt.stage_provenance.clone();
    let reembedded_key = receipt.stage_provenance_key.clone();
    let reembedded = pg
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: stage_title.clone(),
                markdown: serde_json::to_string(&reembedded_provenance).unwrap(),
                stage_provenance_key: reembedded_key.clone(),
                stage_provenance: reembedded_provenance.clone(),
                x: 20.0,
                y: 20.0,
                w: 300.0,
                h: 180.0,
                z_index: 1,
            },
        )
        .await
        .expect("same authoritative Stage artifact can be embedded again");
    assert!(reembedded.created_by_request);
    assert_ne!(
        reembedded.block.block_id, receipt.placed_block_id,
        "re-embed owns a new live RichDocument identity"
    );

    let recovered_backlink_id = format!("KDBL-{}", "c".repeat(32));
    sqlx::query(
        "INSERT INTO knowledge_document_backlinks (backlink_id, workspace_id, relationship_id, source_document_id, link_kind, target, block_id) VALUES ($1, $2, $3, $4, 'wikilink', $5, 'body.2')",
    )
    .bind(&recovered_backlink_id)
    .bind(&ws)
    .bind(format!("KDLNK-{}", "c".repeat(64)))
    .bind(&backlink_source.rich_document_id)
    .bind(&stage_title)
    .execute(&mut conn)
    .await
    .expect("title backlink recovers when the deterministic title is live again");
    let recovered_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_document_backlinks WHERE backlink_id = $1",
    )
    .bind(&recovered_backlink_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(recovered_count, 1);
    sqlx::query("DELETE FROM knowledge_document_backlinks WHERE backlink_id = $1")
        .bind(&recovered_backlink_id)
        .execute(&mut conn)
        .await
        .unwrap();
    pg.db
        .compensate_stage_canvas_card(
            &ctx,
            CompensateLoomCanvasStageCard {
                canvas_block_id: canvas_id,
                workspace_id: ws,
                placement_id: reembedded.placement.placement_id,
                placed_block_id: reembedded.block.block_id,
                stage_provenance_key: reembedded_key,
                stage_provenance: reembedded_provenance,
            },
        )
        .await
        .expect("re-embedded Stage card remains compensatable after backlink cleanup");
}

#[tokio::test]
async fn stage_canvas_compensation_mismatch_and_invalid_json_fail_closed() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Stage compensation guards").await;
    let provenance = stage_provenance(&pg, &ws, "guard").await;
    let key = stage_provenance_key(&provenance);
    let created = pg
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown: serde_json::to_string(&provenance).unwrap(),
                stage_provenance_key: key.clone(),
                stage_provenance: provenance.clone(),
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .unwrap();
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        placement_id: created.placement.placement_id.clone(),
        placed_block_id: created.block.block_id.clone(),
        stage_provenance_key: key,
        stage_provenance: provenance,
    };

    let mut wrong = receipt.clone();
    wrong.canvas_block_id = "wrong-canvas".to_owned();
    assert!(pg
        .db
        .compensate_stage_canvas_card(&ctx, wrong)
        .await
        .is_err());
    assert_eq!(
        pg.db.get_canvas_board(&ws, &canvas_id).await.unwrap().placements.len(),
        1,
        "mismatch must not delete the owned placement"
    );

    let mut conn = pg.raw_connection().await;
    let invalid_documents = [
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": &receipt.stage_provenance.sha256,
            "manifest_ref": &receipt.stage_provenance.manifest_ref,
            "unknown": &receipt.stage_provenance.causal_action_id,
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": " whitespace ",
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": null,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": null,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": null,
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": 1,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": {"value": &receipt.stage_provenance.artifact_id},
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": ["a".repeat(64)],
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": 1,
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": {"value": "stage-causal-guard"},
        }),
    ];
    for invalid in invalid_documents {
        let error = sqlx::query(
            "UPDATE loom_canvas_placements SET stage_provenance = $1 WHERE placement_id = $2",
        )
        .bind(invalid)
        .bind(&receipt.placement_id)
        .execute(&mut conn)
        .await
        .expect_err("missing/replaced/null/non-string tuple must violate Stage provenance CHECK");
        assert!(error.as_database_error().is_some());
    }
    for statement in [
        "UPDATE loom_canvas_placements SET stage_provenance = NULL WHERE placement_id = $1",
        "UPDATE loom_canvas_placements SET stage_provenance_key = NULL WHERE placement_id = $1",
    ] {
        let error = sqlx::query(statement)
            .bind(&receipt.placement_id)
            .execute(&mut conn)
            .await
            .expect_err("Stage provenance CHECK must reject half-present identity");
        assert!(error.as_database_error().is_some());
    }

    sqlx::query(
        "UPDATE loom_canvas_placements SET w = w + 1, updated_at = updated_at + INTERVAL '1 second' WHERE placement_id = $1",
    )
    .bind(&receipt.placement_id)
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "post-create placement resize must revoke compensation ownership"
    );
    sqlx::query(
        "UPDATE loom_canvas_placements SET w = w - 1, updated_at = created_at WHERE placement_id = $1",
    )
    .bind(&receipt.placement_id)
    .execute(&mut conn)
    .await
    .unwrap();

    let bridge = pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &receipt.placed_block_id)
        .await
        .unwrap()
        .unwrap();
    let source_id = format!("KSRC-{}", "1".repeat(32));
    let code_file_id = format!("KCF-{}", "2".repeat(32));
    sqlx::query(
        "INSERT INTO knowledge_sources (source_id, workspace_id, source_kind, content_hash) VALUES ($1, $2, 'external_import', $3)",
    )
    .bind(&source_id)
    .bind(&ws)
    .bind("c".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO knowledge_code_files (code_file_id, workspace_id, source_id, file_entity_id, language, indexed_content_hash, parser_version) VALUES ($1, $2, $3, $4, 'rust', $5, 'test-v1')",
    )
    .bind(&code_file_id)
    .bind(&ws)
    .bind(&source_id)
    .bind(&bridge.entity_id)
    .bind("c".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(pg
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .is_err(), "SET NULL knowledge entity references must block compensation");
    let retained_entity_ref: Option<String> = sqlx::query_scalar(
        "SELECT file_entity_id FROM knowledge_code_files WHERE code_file_id = $1",
    )
    .bind(&code_file_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(retained_entity_ref.as_deref(), Some(bridge.entity_id.as_str()));
    sqlx::query("DELETE FROM knowledge_code_files WHERE code_file_id = $1")
        .bind(&code_file_id)
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DELETE FROM knowledge_sources WHERE source_id = $1")
        .bind(&source_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let loom_source_id = format!("KSRC-{}", "4".repeat(32));
    sqlx::query(
        "INSERT INTO knowledge_sources (source_id, workspace_id, source_kind, loom_block_id, content_hash) VALUES ($1, $2, 'loom_block', $3, $4)",
    )
    .bind(&loom_source_id)
    .bind(&ws)
    .bind(&receipt.placed_block_id)
    .bind("f".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "downstream LoomBlock authority must not be cascade-deleted"
    );
    sqlx::query("DELETE FROM knowledge_sources WHERE source_id = $1")
        .bind(&loom_source_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let recent = pg
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::LoomBlock,
                source_kind: LoomSearchSourceKind::LoomBlock,
                ref_id: receipt.placed_block_id.clone(),
                title: "Stage compensation quick-switcher guard".to_owned(),
                excerpt: "durable LoomBlock navigation reference".to_owned(),
                metadata: json!({"proof": "writer_first"}),
            },
        )
        .await
        .expect("runtime quick-switcher writer records the live Stage block");
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "durable quick-switcher LoomBlock recents must block compensation"
    );
    assert_eq!(recent.ref_id, receipt.placed_block_id);
    sqlx::query(
        "DELETE FROM knowledge_quick_switcher_recents WHERE workspace_id = $1 AND hit_key = $2",
    )
    .bind(&ws)
    .bind(&recent.hit_key)
    .execute(&mut conn)
    .await
    .unwrap();

    let other_document = pg
        .db
        .import_markdown_to_loom(&ctx, &ws, "Backlink source", "source")
        .await
        .unwrap();
    let expected_stage_title = format!("Stage capture {}", receipt.stage_provenance.artifact_id);
    for (ordinal, source_document_id, target) in [
        (7_u8, other_document.rich_document_id.as_str(), receipt.placed_block_id.as_str()),
        (8_u8, other_document.rich_document_id.as_str(), expected_stage_title.as_str()),
        (9_u8, receipt.placed_block_id.as_str(), "Backlink source"),
    ] {
        let backlink_id = format!("KDBL-{}", format!("{ordinal:x}").repeat(32));
        let relationship_id = format!("KDLNK-{}", format!("{ordinal:x}").repeat(64));
        sqlx::query(
            "INSERT INTO knowledge_document_backlinks (backlink_id, workspace_id, relationship_id, source_document_id, link_kind, target, block_id) VALUES ($1, $2, $3, $4, 'wikilink', $5, 'body.0')",
        )
        .bind(&backlink_id)
        .bind(&ws)
        .bind(&relationship_id)
        .bind(source_document_id)
        .bind(target)
        .execute(&mut conn)
        .await
        .unwrap();
        assert!(
            pg.db
                .compensate_stage_canvas_card(&ctx, receipt.clone())
                .await
                .is_err(),
            "inbound id/title and outbound RichDocument backlinks must block compensation"
        );
        sqlx::query("DELETE FROM knowledge_document_backlinks WHERE backlink_id = $1")
            .bind(&backlink_id)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    let rich_source_id = format!("KSRC-{}", "a".repeat(32));
    sqlx::query(
        "INSERT INTO knowledge_sources (source_id, workspace_id, source_kind, content_hash, provenance) VALUES ($1, $2, 'rich_document', $3, jsonb_build_object('rich_document_id', $4::text))",
    )
    .bind(&rich_source_id)
    .bind(&ws)
    .bind("a".repeat(64))
    .bind(&receipt.placed_block_id)
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "rich_document knowledge-source provenance must block compensation"
    );
    sqlx::query("DELETE FROM knowledge_sources WHERE source_id = $1")
        .bind(&rich_source_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let context_hash = "b".repeat(64);
    let bundle_id = format!("CTX-{}", &context_hash[..16]);
    sqlx::query(
        "INSERT INTO knowledge_context_bundles (bundle_id, workspace_id, kernel_task_run_id, session_run_id, allowed_context, context_hash) VALUES ($1, $2, 'KTR-stage-compensation-guard', 'SR-stage-compensation-guard', '[]'::jsonb, $3)",
    )
    .bind(&bundle_id)
    .bind(&ws)
    .bind(&context_hash)
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO knowledge_context_bundle_items (bundle_id, item_ordinal, ref_kind, ref_id, retrieval_decision) VALUES ($1, 0, 'entity', $2, 'included')",
    )
    .bind(&bundle_id)
    .bind(&bridge.entity_id)
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "context bundles referencing the bridge entity must block compensation"
    );
    sqlx::query("DELETE FROM knowledge_context_bundles WHERE bundle_id = $1")
        .bind(&bundle_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let proposal_id = format!("FMP-{}", "c".repeat(32));
    let request_id = format!("stage-compensation-guard-{}", uuid::Uuid::now_v7());
    sqlx::query(
        "INSERT INTO fems_memory_proposals (proposal_id, request_id, workspace_id, document_id, selection_start, selection_end, content_hash, memory_class, proposal) VALUES ($1, $2, $3, $4, 0, 0, $5, 'fact', '{}'::jsonb)",
    )
    .bind(&proposal_id)
    .bind(&request_id)
    .bind(&ws)
    .bind(&receipt.placed_block_id)
    .bind("c".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "FEMS proposals for the RichDocument must block compensation"
    );
    sqlx::query("DELETE FROM fems_memory_proposals WHERE proposal_id = $1")
        .bind(&proposal_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let suggestion_id = format!("LAIS-{}", "d".repeat(32));
    let job_id = format!("LAIJ-{}", "e".repeat(32));
    sqlx::query(
        "INSERT INTO loom_ai_suggestions (suggestion_id, job_id, workspace_id, kind, block_id, suggested_value, model_attribution, prompt_sha256, output_sha256, recorded_event_id, value_hash) VALUES ($1, $2, $3, 'auto_caption', $4, '{\"caption\":\"guard\"}'::jsonb, '{\"model\":\"test\"}'::jsonb, $5, $6, $7, $8)",
    )
    .bind(&suggestion_id)
    .bind(&job_id)
    .bind(&ws)
    .bind(&receipt.placed_block_id)
    .bind("d".repeat(64))
    .bind("e".repeat(64))
    .bind(&bridge.index_event_id)
    .bind("f".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "pending Loom AI suggestions for the block must block compensation"
    );
    sqlx::query("DELETE FROM loom_ai_suggestions WHERE suggestion_id = $1")
        .bind(&suggestion_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let target_suggestion_id = format!("LAIS-{}", "2".repeat(32));
    let target_job_id = format!("LAIJ-{}", "3".repeat(32));
    sqlx::query(
        "INSERT INTO loom_ai_suggestions (suggestion_id, job_id, workspace_id, kind, block_id, target_block_id, suggested_value, model_attribution, prompt_sha256, output_sha256, recorded_event_id, value_hash) VALUES ($1, $2, $3, 'link_suggest', $4, $5, '{\"reason\":\"guard\"}'::jsonb, '{\"model\":\"test\"}'::jsonb, $6, $7, $8, $9)",
    )
    .bind(&target_suggestion_id)
    .bind(&target_job_id)
    .bind(&ws)
    .bind(&canvas_id)
    .bind(&receipt.placed_block_id)
    .bind("2".repeat(64))
    .bind("3".repeat(64))
    .bind(&bridge.index_event_id)
    .bind("4".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "Loom AI suggestions targeting the block must block compensation"
    );
    sqlx::query("DELETE FROM loom_ai_suggestions WHERE suggestion_id = $1")
        .bind(&target_suggestion_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let edge_source = make_block(&pg.db, &ws, "Guard edge source", LoomBlockContentType::Note).await;
    let edge_target = make_block(&pg.db, &ws, "Guard edge target", LoomBlockContentType::Note).await;
    let edge_id = format!("LE-{}", "f".repeat(32));
    sqlx::query(
        "INSERT INTO loom_edges (edge_id, workspace_id, source_block_id, target_block_id, edge_type, created_by, source_text_block_id) VALUES ($1, $2, $3, $4, 'mention', 'user', $5)",
    )
    .bind(&edge_id)
    .bind(&ws)
    .bind(&edge_source)
    .bind(&edge_target)
    .bind(&receipt.placed_block_id)
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "Loom edges whose source text comes from the block must block compensation"
    );
    sqlx::query("DELETE FROM loom_edges WHERE edge_id = $1")
        .bind(&edge_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let other_entity_id = format!("KEN-{}", "5".repeat(32));
    let decision_id = format!("KBR-{}", "6".repeat(32));
    sqlx::query(
        "INSERT INTO knowledge_entities (entity_id, workspace_id, entity_kind, entity_key, display_name) VALUES ($1, $2, 'concept', 'stage-compensation-guard', 'Stage compensation guard')",
    )
    .bind(&other_entity_id)
    .bind(&ws)
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO knowledge_memory_bridge_decisions (decision_id, workspace_id, entity_id_a, entity_id_b, decision, degree_a, degree_b, hub_degree_threshold) VALUES ($1, $2, $3, $4, 'suppressed_connected', 0, 0, 100)",
    )
    .bind(&decision_id)
    .bind(&ws)
    .bind(&bridge.entity_id)
    .bind(&other_entity_id)
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(
        pg.db
            .compensate_stage_canvas_card(&ctx, receipt.clone())
            .await
            .is_err(),
        "knowledge memory bridge decisions must block entity cascade deletion"
    );
    let retained_decision: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_memory_bridge_decisions WHERE decision_id = $1",
    )
    .bind(&decision_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(retained_decision, 1, "blocked compensation must retain bridge decisions");
    sqlx::query("DELETE FROM knowledge_memory_bridge_decisions WHERE decision_id = $1")
        .bind(&decision_id)
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DELETE FROM knowledge_entities WHERE entity_id = $1")
        .bind(&other_entity_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let original_sha: String = sqlx::query_scalar(
        "SELECT content_sha256 FROM knowledge_rich_documents WHERE rich_document_id = $1",
    )
    .bind(&receipt.placed_block_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    let drifted_sha = "d".repeat(64);
    sqlx::query(
        "UPDATE knowledge_rich_documents SET content_sha256 = $1 WHERE rich_document_id = $2",
    )
    .bind(&drifted_sha)
    .bind(&receipt.placed_block_id)
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query("UPDATE loom_blocks SET content_hash = $1 WHERE block_id = $2")
        .bind(&drifted_sha)
        .bind(&receipt.placed_block_id)
        .execute(&mut conn)
        .await
        .unwrap();
    assert!(pg
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .is_err(), "coordinated hash metadata drift must not authorize deletion");
    sqlx::query(
        "UPDATE knowledge_rich_documents SET content_sha256 = $1 WHERE rich_document_id = $2",
    )
    .bind(&original_sha)
    .bind(&receipt.placed_block_id)
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query("UPDATE loom_blocks SET content_hash = $1 WHERE block_id = $2")
        .bind(&original_sha)
        .bind(&receipt.placed_block_id)
        .execute(&mut conn)
        .await
        .unwrap();

    let code_node_id = format!("KCN-{}", "3".repeat(32));
    sqlx::query(
        "INSERT INTO knowledge_editor_code_nodes (code_node_id, rich_document_id, node_path, language_id, code_text, round_trip_sha256) VALUES ($1, $2, 'body.0.code', 'rust', 'fn post_create() {}', $3)",
    )
    .bind(&code_node_id)
    .bind(&receipt.placed_block_id)
    .bind("e".repeat(64))
    .execute(&mut conn)
    .await
    .unwrap();
    assert!(pg
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .is_err(), "post-create RichDocument children must block cascade deletion");
    let code_node_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_editor_code_nodes WHERE code_node_id = $1",
    )
    .bind(&code_node_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(code_node_count, 1, "blocked compensation must retain child authority");
}

#[tokio::test]
async fn stage_canvas_compensation_rolls_back_every_delete_on_failure() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&pg.db, &ws, "Stage rollback").await;
    let provenance = stage_provenance(&pg, &ws, "rollback").await;
    let key = stage_provenance_key(&provenance);
    let created = pg
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown: serde_json::to_string(&provenance).unwrap(),
                stage_provenance_key: key.clone(),
                stage_provenance: provenance.clone(),
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .unwrap();
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        placement_id: created.placement.placement_id.clone(),
        placed_block_id: created.block.block_id.clone(),
        stage_provenance_key: key,
        stage_provenance: provenance,
    };
    let mut conn = pg.raw_connection().await;
    sqlx::query(
        r#"
        CREATE FUNCTION reject_stage_document_delete() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected compensation rollback'; END $$;
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query(
        r#"
        CREATE TRIGGER reject_stage_document_delete
        BEFORE DELETE ON knowledge_rich_documents
        FOR EACH ROW EXECUTE FUNCTION reject_stage_document_delete()
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    assert!(pg
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .is_err());
    assert_eq!(
        pg.db.get_canvas_board(&ws, &canvas_id).await.unwrap().placements.len(),
        1,
        "placement delete must roll back with the later document failure"
    );
    assert!(pg
        .db
        .get_loom_block(&ws, &receipt.placed_block_id)
        .await
        .is_ok());
    assert!(pg
        .db
        .get_loom_block_knowledge_bridge(&ws, &receipt.placed_block_id)
        .await
        .unwrap()
        .is_some());
    let compensation_event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE source_component = 'loom_canvas_stage_compensation' AND aggregate_id = $1",
    )
    .bind(&receipt.placed_block_id)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        compensation_event_count, 0,
        "a failed delete rolls back the compensation audit event in the same transaction"
    );
    sqlx::query("DROP TRIGGER reject_stage_document_delete ON knowledge_rich_documents")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_stage_document_delete()")
        .execute(&mut conn)
        .await
        .unwrap();
}
