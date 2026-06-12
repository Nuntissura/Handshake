//! WP-KERNEL-009 MT-192 LoomNavigationFixtures — REAL PostgreSQL proof.
//!
//! A consolidated navigation fixture that builds one realistic Loom workspace
//! and exercises EVERY navigation path together (proving they compose over the
//! same authority): exact LoomBlock open, backlinks, unlinked mentions, tags +
//! nested tags, folder color labels, local + global graph filters, and a stale
//! wiki projection that regenerates. Authority = PostgreSQL; no parallel store.
//!
//! This is the MT-192 fixture the contract names; it depends on MT-177..190.

mod knowledge_pg_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomEdgeCreatedBy,
    LoomEdgeType, LoomFolderSortMode, LoomViewFilters, LoomViewResponse, LoomViewType, NewLoomBlock,
    NewLoomEdge, NewLoomFolder, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-192 loom navigation fixtures: PostgreSQL unavailable");
                return;
            }
        }
    }};
}

struct Db<'a>(&'a handshake_core::storage::postgres::PostgresDatabase, String);

impl<'a> Db<'a> {
    async fn block(&self, title: &str, body: Option<&str>, ct: LoomBlockContentType) -> String {
        let ctx = WriteContext::human(None);
        let mut derived = LoomBlockDerived::default();
        derived.full_text_index = body.map(|b| b.to_string());
        let id = self
            .0
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: self.1.clone(),
                    content_type: ct,
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
            .expect("block")
            .block_id;
        // Bridge every fixture block so navigation resolves to authority.
        self.0
            .bridge_loom_block_to_knowledge(&ctx, &self.1, &id)
            .await
            .expect("bridge");
        id
    }

    async fn edge(&self, src: &str, tgt: &str, et: LoomEdgeType) {
        let ctx = WriteContext::human(None);
        self.0
            .create_loom_edge(
                &ctx,
                NewLoomEdge {
                    edge_id: None,
                    workspace_id: self.1.clone(),
                    source_block_id: src.to_string(),
                    target_block_id: tgt.to_string(),
                    edge_type: et,
                    created_by: LoomEdgeCreatedBy::User,
                    crdt_site_id: None,
                    source_anchor: None,
                },
            )
            .await
            .expect("edge");
    }
}

/// The full MT-192 navigation fixture in one scenario.
#[tokio::test]
async fn loom_navigation_fixture_exercises_every_path() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let db = Db(&pg.db, ws.clone());
    let ctx = WriteContext::human(None);

    // --- Blocks ---------------------------------------------------------
    let roadmap = db.block("Roadmap", Some("the product roadmap"), LoomBlockContentType::Note).await;
    let q3 = db.block("Q3 Plan", Some("align the Roadmap with budget"), LoomBlockContentType::Note).await;
    let lonely = db.block("Stray Idea", Some("mentions the Roadmap in passing"), LoomBlockContentType::Note).await;
    let tag_project = db.block("project", None, LoomBlockContentType::TagHub).await;
    let tag_alpha = db.block("alpha", None, LoomBlockContentType::TagHub).await;

    // --- Edges: a formal mention (Q3 -> Roadmap), tags, nested tag ------
    db.edge(&q3, &roadmap, LoomEdgeType::Mention).await;
    db.edge(&q3, &tag_project, LoomEdgeType::Tag).await;
    db.edge(&roadmap, &tag_alpha, LoomEdgeType::Tag).await;
    db.edge(&tag_alpha, &tag_project, LoomEdgeType::SubTag).await; // alpha is child of project

    // === FIXTURE 1: exact LoomBlock open (direct id load) ==============
    let opened = pg.db.get_loom_block(&ws, &roadmap).await.expect("exact open");
    assert_eq!(opened.title.as_deref(), Some("Roadmap"));

    // === FIXTURE 2: backlinks (linked, with context snippet) ===========
    let backlinks = pg.db.get_backlinks_with_context(&ws, &roadmap).await.expect("backlinks");
    assert!(backlinks.iter().any(|b| b.source_block.block_id == q3),
        "Q3 backlinks Roadmap via the mention edge");
    assert!(backlinks.iter().any(|b| b.context_snippet.as_deref().is_some_and(|s| s.contains("Roadmap"))),
        "a backlink carries a context snippet");

    // === FIXTURE 3: unlinked mentions ==================================
    let unlinked = pg.db.scan_unlinked_mentions(&ws, &roadmap, &[], 100).await.expect("unlinked");
    assert!(unlinked.iter().any(|m| m.source_block.block_id == lonely),
        "Stray Idea is an unlinked mention of Roadmap");
    assert!(!unlinked.iter().any(|m| m.source_block.block_id == q3),
        "Q3 (formally linked) is not an unlinked mention");

    // === FIXTURE 4: tags + nested tags =================================
    let hub = pg.db.get_tag_hub(&ws, &tag_project).await.expect("tag hub");
    assert!(hub.sub_tags.iter().any(|b| b.block_id == tag_alpha), "alpha is a sub-tag of project");
    let nested = pg.db.list_blocks_for_tag(&ws, &tag_project, true, 100, 0).await.expect("nested tag");
    let nested_ids: Vec<&str> = nested.iter().map(|b| b.block_id.as_str()).collect();
    assert!(nested_ids.contains(&q3.as_str()), "Q3 tagged #project directly");
    assert!(nested_ids.contains(&roadmap.as_str()), "Roadmap tagged #alpha, nested under #project");

    // === FIXTURE 5: folder tree + color labels =========================
    let work = pg
        .db
        .create_loom_folder(&ws, NewLoomFolder {
            folder_id: None, workspace_id: ws.clone(), parent_folder_id: None,
            name: "Work".into(), color: Some("#3366ff".into()),
            sort_mode: LoomFolderSortMode::NameAsc, sort_order: None, project_ref: Some("proj-x".into()),
        })
        .await.expect("folder").folder_id;
    pg.db.add_block_to_loom_folder(&ws, &work, &roadmap, None).await.expect("file roadmap");
    let folders = pg.db.list_loom_folders(&ws).await.expect("folders");
    let work_folder = folders.iter().find(|f| f.folder_id == work).unwrap();
    assert_eq!(work_folder.color.as_deref(), Some("#3366ff"), "folder color label persisted");

    // === FIXTURE 6: breadcrumbs across the spine =======================
    let trail = pg.db.loom_block_breadcrumbs(&ws, &roadmap).await.expect("breadcrumbs");
    let kinds: Vec<&str> = trail.crumbs.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(kinds.first(), Some(&"workspace"));
    assert!(kinds.contains(&"folder") && kinds.contains(&"project") && kinds.contains(&"entity"));

    // === FIXTURE 7: local + global graph filters =======================
    let local = pg.db.local_graph(&ws, &roadmap, 2, &[], 200).await.expect("local graph");
    assert!(local.nodes.iter().any(|n| n.block.block_id == q3), "Q3 in Roadmap's neighborhood (undirected)");
    // Edge-type filter: tags only -> the Q3->Roadmap mention is excluded.
    let local_tags = pg.db.local_graph(&ws, &roadmap, 1, &[LoomEdgeType::Tag], 200).await.expect("tag graph");
    assert!(local_tags.nodes.iter().any(|n| n.block.block_id == tag_alpha),
        "Roadmap -> #alpha tag edge present in tag-filtered graph");
    let global = pg.db.global_graph(&ws, &[], 500, 0).await.expect("global graph");
    assert!(global.nodes.len() >= 5, "global graph spans the fixture blocks");

    // === FIXTURE 8: pins view ==========================================
    pg.db.update_loom_block(&ctx, &ws, &roadmap, LoomBlockUpdate { pinned: Some(true), ..Default::default() }).await.expect("pin");
    pg.db.set_loom_block_pin_order(&ctx, &ws, &roadmap, Some(0)).await.expect("order");
    let pins = pg.db.query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0).await.expect("pins");
    match pins {
        LoomViewResponse::Pins { blocks } => assert!(blocks.iter().any(|b| b.block_id == roadmap)),
        _ => panic!("expected Pins"),
    }

    // === FIXTURE 9: stale wiki projection that regenerates =============
    let proj = pg.db.compile_loom_wiki_projection(&ws, "Roadmap Topic", &[roadmap.clone(), q3.clone()]).await.expect("compile");
    assert!(!pg.db.loom_wiki_projection_is_stale(&ws, &proj.projection_id).await.expect("fresh"));
    // Mutate a source block -> projection goes stale.
    pg.db.update_loom_block(&ctx, &ws, &q3, LoomBlockUpdate { title: Some("Q3 Plan v2".into()), ..Default::default() }).await.expect("edit");
    assert!(pg.db.loom_wiki_projection_is_stale(&ws, &proj.projection_id).await.expect("stale"), "wiki page is stale after source edit");
    let regen = pg.db.regenerate_loom_wiki_projection(&ws, &proj.projection_id).await.expect("regen");
    assert!(regen.rendered_content.contains("Q3 Plan v2"));
    assert!(!pg.db.loom_wiki_projection_is_stale(&ws, &proj.projection_id).await.expect("fresh2"));

    // === FIXTURE 10: projection delete leaves authority intact =========
    let before = serde_json::to_string(&pg.db.get_loom_block(&ws, &roadmap).await.expect("before")).unwrap();
    pg.db.delete_loom_wiki_projection(&ws, &proj.projection_id).await.expect("delete projection");
    let after = serde_json::to_string(&pg.db.get_loom_block(&ws, &roadmap).await.expect("after")).unwrap();
    assert_eq!(before, after, "deleting the wiki projection leaves the LoomBlock authority byte-identical");
}
