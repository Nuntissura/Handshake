//! WP-KERNEL-009 MT-186 GraphSearchAndFilters -- real PostgreSQL proof.
//!
//! MT-186 extends Loom search from block-only search into the Obsidian-style
//! graph search surface: one bounded query must find Loom blocks, code symbols,
//! work packets, microtasks, UserManual pages, and project wiki pages from
//! PostgreSQL authority.

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeStore, NewKnowledgeEntity, NewKnowledgeRichDocument,
};
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
    LoomSearchFilters, LoomSearchSourceKind, NewLoomBlock, NewLoomEdge, WriteContext,
};
use handshake_core::user_manual::{
    store::{NewManualAnchor, NewManualSection, NewUserManualPage},
    USER_MANUAL_VERSION,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                panic!("MT-186 loom graph search proof requires real PostgreSQL");
            }
        }
    }};
}

async fn insert_entity(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    entity_kind: KnowledgeEntityKind,
    entity_key: &str,
    display_name: &str,
) {
    db.upsert_knowledge_entity(NewKnowledgeEntity {
        workspace_id: ws.to_string(),
        entity_kind,
        entity_key: entity_key.to_string(),
        display_name: display_name.to_string(),
        detection_provenance: json!({
            "source": "mt186_graph_search_fixture",
            "work_packet": "WP-KERNEL-009-Project-Knowledge-Index-Loom-Rich-Editor-v1",
        }),
        primary_source_id: None,
        detected_in_run: None,
        evidence_span_ids: Vec::new(),
    })
    .await
    .expect("insert knowledge entity");
}

async fn insert_user_manual_page(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    slug: &str,
    title: &str,
    body: &str,
) {
    let store = handshake_core::user_manual::store::UserManualStore::new(db);
    store
        .upsert_page(
            &NewUserManualPage {
                slug: slug.to_string(),
                title: title.to_string(),
                page_kind: "workflow",
                audience: "model",
                spec_anchors: vec!["MT-186".to_string()],
                sections: vec![NewManualSection {
                    section_kind: "workflows",
                    title: "Graph search workflow".to_string(),
                    body_md: body.to_string(),
                    body_json: None,
                }],
                anchors: vec![NewManualAnchor {
                    anchor_kind: "primitive",
                    anchor_value: "loom.graph_search".to_string(),
                    http_method: "",
                }],
            },
            USER_MANUAL_VERSION,
            "current",
        )
        .await
        .expect("insert UserManual page");
}

async fn insert_loom_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ctx: &WriteContext,
    ws: &str,
    content_type: LoomBlockContentType,
    title: &str,
    full_text: Option<&str>,
) -> String {
    db.create_loom_block(
        ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type,
            document_id: None,
            asset_id: None,
            title: Some(title.to_string()),
            original_filename: None,
            content_hash: None,
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived: LoomBlockDerived {
                full_text_index: full_text.map(str::to_string),
                ..Default::default()
            },
        },
    )
    .await
    .expect("insert Loom block")
    .block_id
}

async fn insert_rich_document(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ws: &str,
    title: &str,
    body: &str,
) -> String {
    db.create_knowledge_rich_document(NewKnowledgeRichDocument {
        workspace_id: ws.to_string(),
        document_id: None,
        title: title.to_string(),
        schema_version: "rich_document_v1".to_string(),
        content_json: json!({
            "type": "doc",
            "content": [
                {
                    "type": "paragraph",
                    "content": [
                        { "type": "text", "text": body }
                    ]
                }
            ]
        }),
        ..Default::default()
    })
    .await
    .expect("insert rich document")
    .rich_document_id
}

#[tokio::test]
async fn mt186_graph_search_spans_loom_knowledge_and_usermanual_with_filters() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let loom_block = pg
        .db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("GraphSearchAlpha Loom note".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived {
                    full_text_index: Some(
                        "GraphSearchAlpha joins notes to code and manuals".to_string(),
                    ),
                    ..Default::default()
                },
            },
        )
        .await
        .expect("insert loom block");
    pg.db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &loom_block.block_id)
        .await
        .expect("bridge loom block");
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::Symbol,
        "rust:src/backend/graph_search.rs#GraphSearchAlpha",
        "GraphSearchAlpha",
    )
    .await;
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::WorkPacket,
        "WP-KERNEL-009-GraphSearchAlpha",
        "GraphSearchAlpha work packet",
    )
    .await;
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::MicroTask,
        "MT-186-GraphSearchAlpha",
        "GraphSearchAlpha microtask",
    )
    .await;
    insert_user_manual_page(
        &pg.db,
        "graph-search-alpha",
        "GraphSearchAlpha UserManual page",
        "Models use GraphSearchAlpha to find Loom notes, symbols, WPs, and MTs.",
    )
    .await;
    let wiki_projection = pg
        .db
        .compile_loom_wiki_projection(
            &ws,
            "GraphSearchAlpha Wiki Page",
            std::slice::from_ref(&loom_block.block_id),
        )
        .await
        .expect("compile wiki projection");

    let all_hits = pg
        .db
        .search_loom_graph(&ws, "GraphSearchAlpha", LoomSearchFilters::default(), 20, 0)
        .await
        .expect("graph search");
    let hit_keys: Vec<_> = all_hits
        .iter()
        .map(|hit| {
            (
                hit.result_kind.as_str(),
                hit.source_kind.as_str(),
                hit.ref_id.as_str(),
            )
        })
        .collect();
    assert!(
        hit_keys
            .iter()
            .any(|(_, source, id)| *source == "loom_block" && *id == loom_block.block_id),
        "heterogeneous graph search must include the matching Loom block: {hit_keys:?}"
    );
    assert!(
        hit_keys.iter().any(|(_, source, _)| *source == "symbol"),
        "heterogeneous graph search must include matching symbol entities: {hit_keys:?}"
    );
    assert!(
        hit_keys
            .iter()
            .any(|(_, source, _)| *source == "work_packet"),
        "heterogeneous graph search must include matching work packets: {hit_keys:?}"
    );
    assert!(
        hit_keys
            .iter()
            .any(|(_, source, _)| *source == "micro_task"),
        "heterogeneous graph search must include matching microtasks: {hit_keys:?}"
    );
    assert!(
        hit_keys
            .iter()
            .any(|(_, source, _)| *source == "user_manual_page"),
        "heterogeneous graph search must include matching UserManual pages: {hit_keys:?}"
    );
    assert!(
        hit_keys.iter().any(|(_, source, id)| {
            *source == "wiki_page" && *id == wiki_projection.projection_id
        }),
        "heterogeneous graph search must include matching project wiki pages: {hit_keys:?}"
    );

    let symbol_only = pg
        .db
        .search_loom_graph(
            &ws,
            "GraphSearchAlpha",
            LoomSearchFilters {
                source_kinds: vec![LoomSearchSourceKind::Symbol],
                ..Default::default()
            },
            20,
            0,
        )
        .await
        .expect("symbol-filtered graph search");
    assert_eq!(symbol_only.len(), 1);
    assert_eq!(symbol_only[0].source_kind.as_str(), "symbol");

    let literal_wildcard = pg
        .db
        .search_loom_graph(&ws, "%", LoomSearchFilters::default(), 20, 0)
        .await
        .expect("literal wildcard search");
    assert!(
        literal_wildcard.is_empty(),
        "LIKE wildcards in user queries must be literal, not broadened into every graph hit"
    );

    let empty = pg
        .db
        .search_loom_graph(&ws, "   ", LoomSearchFilters::default(), 20, 0)
        .await
        .expect("empty graph search");
    assert!(empty.is_empty(), "empty graph search should return no hits");
}

#[tokio::test]
async fn mt256_graph_search_matches_three_letter_abbreviations() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let loom_block = pg
        .db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("GraphSearchAlpha Loom note".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived {
                    full_text_index: Some(
                        "GraphSearchAlpha joins notes to code and manuals".to_string(),
                    ),
                    ..Default::default()
                },
            },
        )
        .await
        .expect("insert loom block");
    let file_block = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::File,
        "GraphSearchAlpha source file",
        Some("GraphSearchAlpha appears in an indexed file block"),
    )
    .await;
    let tag_hub = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::TagHub,
        "GraphSearchAlpha tag hub",
        Some("GraphSearchAlpha appears in a tag hub"),
    )
    .await;
    let rich_document = insert_rich_document(
        &pg.db,
        &ws,
        "GraphSearchAlpha standalone document",
        "GraphSearchAlpha appears in a standalone RichDocument authority record",
    )
    .await;

    pg.db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &loom_block.block_id)
        .await
        .expect("bridge loom block");
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::Symbol,
        "rust:src/backend/graph_search.rs#GraphSearchAlpha",
        "GraphSearchAlpha",
    )
    .await;
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::WorkPacket,
        "WP-KERNEL-009-GraphSearchAlpha",
        "GraphSearchAlpha work packet",
    )
    .await;
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::MicroTask,
        "MT-186-GraphSearchAlpha",
        "GraphSearchAlpha microtask",
    )
    .await;
    insert_user_manual_page(
        &pg.db,
        "graph-search-alpha",
        "GraphSearchAlpha UserManual page",
        "Models use GraphSearchAlpha to find Loom notes, symbols, WPs, and MTs.",
    )
    .await;
    let wiki_projection = pg
        .db
        .compile_loom_wiki_projection(
            &ws,
            "GraphSearchAlpha Wiki Page",
            std::slice::from_ref(&loom_block.block_id),
        )
        .await
        .expect("compile wiki projection");

    let abbreviation_hits = pg
        .db
        .search_loom_graph(&ws, "GSA", LoomSearchFilters::default(), 20, 0)
        .await
        .expect("abbreviation graph search");
    let hit_keys: Vec<_> = abbreviation_hits
        .iter()
        .map(|hit| {
            (
                hit.result_kind.as_str(),
                hit.source_kind.as_str(),
                hit.ref_id.as_str(),
            )
        })
        .collect();

    for expected in [
        "loom_block",
        "file",
        "tag_hub",
        "document",
        "symbol",
        "work_packet",
        "micro_task",
        "user_manual_page",
        "wiki_page",
    ] {
        assert!(
            hit_keys.iter().any(|(_, source, _)| *source == expected),
            "three-letter abbreviation must find {expected} hits: {hit_keys:?}"
        );
    }
    assert!(
        hit_keys
            .iter()
            .any(|(result, source, id)| *result == "loom_block"
                && *source == "file"
                && *id == file_block),
        "file source hit must preserve Loom block id for direct open: {hit_keys:?}"
    );
    assert!(
        hit_keys.iter().any(|(result, source, id)| {
            *result == "loom_block" && *source == "tag_hub" && *id == tag_hub
        }),
        "tag hub source hit must preserve Loom block id for direct open: {hit_keys:?}"
    );
    assert!(
        hit_keys.iter().any(|(result, source, id)| {
            *result == "knowledge_entity" && *source == "document" && *id == rich_document
        }),
        "document source hit must preserve rich document id for direct open: {hit_keys:?}"
    );
    assert!(
        hit_keys.iter().any(|(_, source, id)| {
            *source == "wiki_page" && *id == wiki_projection.projection_id
        }),
        "abbreviation hit must preserve direct-open wiki projection id: {hit_keys:?}"
    );

    let typo_hits = pg
        .db
        .search_loom_graph(&ws, "GraphSearxh", LoomSearchFilters::default(), 20, 0)
        .await
        .expect("typo-tolerant graph search");
    let typo_hit_keys: Vec<_> = typo_hits
        .iter()
        .map(|hit| {
            (
                hit.result_kind.as_str(),
                hit.source_kind.as_str(),
                hit.ref_id.as_str(),
            )
        })
        .collect();

    for expected in [
        "loom_block",
        "file",
        "tag_hub",
        "document",
        "symbol",
        "work_packet",
        "micro_task",
        "user_manual_page",
        "wiki_page",
    ] {
        assert!(
            typo_hit_keys
                .iter()
                .any(|(_, source, _)| *source == expected),
            "typo-tolerant fuzzy search must find {expected} hits: {typo_hit_keys:?}"
        );
    }
    assert!(
        typo_hit_keys.iter().any(|(result, source, id)| {
            *result == "knowledge_entity" && *source == "document" && *id == rich_document
        }),
        "typo-tolerant document hit must preserve rich document id: {typo_hit_keys:?}"
    );
    assert!(
        typo_hit_keys.iter().any(|(_, source, id)| {
            *source == "wiki_page" && *id == wiki_projection.projection_id
        }),
        "typo-tolerant hit must preserve direct-open wiki projection id: {typo_hit_keys:?}"
    );
}

#[tokio::test]
async fn mt256_graph_search_exposes_file_tag_hub_and_document_source_filters() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let file_block = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::File,
        "BreadthProofAlpha source file",
        Some("BreadthProofAlpha appears in a file block"),
    )
    .await;
    let tag_hub = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::TagHub,
        "BreadthProofAlpha tag hub",
        Some("BreadthProofAlpha appears in a tag hub"),
    )
    .await;
    let document = insert_rich_document(
        &pg.db,
        &ws,
        "BreadthProofAlpha standalone document",
        "BreadthProofAlpha appears in a standalone rich document",
    )
    .await;

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "BreadthProofAlpha",
            LoomSearchFilters::default(),
            20,
            0,
        )
        .await
        .expect("breadth graph search");
    let hit_keys: Vec<_> = hits
        .iter()
        .map(|hit| {
            (
                hit.result_kind.as_str(),
                hit.source_kind.as_str(),
                hit.ref_id.as_str(),
            )
        })
        .collect();

    assert_eq!(
        hit_keys
            .iter()
            .filter(|(_, source, id)| *source == "file" && *id == file_block)
            .count(),
        1,
        "file source kind should return one first-class direct-open hit: {hit_keys:?}"
    );
    assert_eq!(
        hit_keys
            .iter()
            .filter(|(_, source, id)| *source == "tag_hub" && *id == tag_hub)
            .count(),
        1,
        "tag hub source kind should return one first-class direct-open hit: {hit_keys:?}"
    );
    assert_eq!(
        hit_keys
            .iter()
            .filter(|(result, source, id)| {
                *result == "knowledge_entity" && *source == "document" && *id == document
            })
            .count(),
        1,
        "document source kind should return one first-class rich-document hit: {hit_keys:?}"
    );

    for (source_kind, expected_ref) in [
        (LoomSearchSourceKind::File, file_block.as_str()),
        (LoomSearchSourceKind::TagHub, tag_hub.as_str()),
        (LoomSearchSourceKind::Document, document.as_str()),
    ] {
        let filtered = pg
            .db
            .search_loom_graph(
                &ws,
                "BreadthProofAlpha",
                LoomSearchFilters {
                    source_kinds: vec![source_kind],
                    ..Default::default()
                },
                20,
                0,
            )
            .await
            .expect("source-filtered breadth search");
        assert_eq!(
            filtered.len(),
            1,
            "{source_kind:?} filter should return only its matching source: {filtered:?}"
        );
        assert_eq!(filtered[0].source_kind, source_kind);
        assert_eq!(filtered[0].ref_id, expected_ref);
    }
}

#[tokio::test]
async fn mt256_graph_search_all_source_limit_does_not_starve_documents() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    for index in 0..30 {
        insert_loom_block(
            &pg.db,
            &ctx,
            &ws,
            LoomBlockContentType::Note,
            &format!("LimitFairnessAlpha block {index:02}"),
            Some("LimitFairnessAlpha appears in enough block rows to crowd later source kinds."),
        )
        .await;
    }
    let document = insert_rich_document(
        &pg.db,
        &ws,
        "LimitFairnessAlpha standalone document",
        "LimitFairnessAlpha appears in a standalone rich document.",
    )
    .await;

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "LimitFairnessAlpha",
            LoomSearchFilters::default(),
            25,
            0,
        )
        .await
        .expect("limit-fair all-source graph search");
    let hit_keys: Vec<_> = hits
        .iter()
        .map(|hit| {
            (
                hit.result_kind.as_str(),
                hit.source_kind.as_str(),
                hit.ref_id.as_str(),
            )
        })
        .collect();

    assert_eq!(hits.len(), 25, "limit must remain honored: {hit_keys:?}");
    assert!(
        hit_keys.iter().any(|(result, source, id)| {
            *result == "knowledge_entity" && *source == "document" && *id == document
        }),
        "all-source limit must not let earlier block hits starve document hits: {hit_keys:?}"
    );
}

#[tokio::test]
async fn mt256_graph_search_block_group_limit_does_not_starve_file_or_tag_hub() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let file_block = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::File,
        "GroupedFairnessAlpha source file",
        Some("GroupedFairnessAlpha appears in an older file block."),
    )
    .await;
    let tag_hub = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::TagHub,
        "GroupedFairnessAlpha tag hub",
        Some("GroupedFairnessAlpha appears in an older tag hub."),
    )
    .await;

    for index in 0..30 {
        insert_loom_block(
            &pg.db,
            &ctx,
            &ws,
            LoomBlockContentType::Note,
            &format!("GroupedFairnessAlpha note {index:02}"),
            Some(
                "GroupedFairnessAlpha appears in enough note rows to crowd grouped block sources.",
            ),
        )
        .await;
    }

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "GroupedFairnessAlpha",
            LoomSearchFilters {
                source_kinds: vec![
                    LoomSearchSourceKind::LoomBlock,
                    LoomSearchSourceKind::File,
                    LoomSearchSourceKind::TagHub,
                ],
                ..Default::default()
            },
            25,
            0,
        )
        .await
        .expect("grouped block-source fairness search");

    let hit_keys: Vec<_> = hits
        .iter()
        .map(|hit| (hit.source_kind.as_str(), hit.ref_id.as_str()))
        .collect();
    assert!(
        hit_keys
            .iter()
            .any(|(source, id)| *source == "file" && *id == file_block),
        "grouped block fetch must not let loom_block rows starve file hits: {hit_keys:?}"
    );
    assert!(
        hit_keys
            .iter()
            .any(|(source, id)| *source == "tag_hub" && *id == tag_hub),
        "grouped block fetch must not let loom_block rows starve tag_hub hits: {hit_keys:?}"
    );
}

#[tokio::test]
async fn mt256_graph_search_knowledge_group_limit_does_not_starve_wp_or_mt() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::WorkPacket,
        "WP-GROUPED-FAIRNESS",
        "GroupedFairnessBeta work packet",
    )
    .await;
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::MicroTask,
        "MT-GROUPED-FAIRNESS",
        "GroupedFairnessBeta microtask",
    )
    .await;

    for index in 0..30 {
        insert_entity(
            &pg.db,
            &ws,
            KnowledgeEntityKind::Symbol,
            &format!("symbol.grouped.fairness.{index:02}"),
            &format!("GroupedFairnessBeta symbol {index:02}"),
        )
        .await;
    }

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "GroupedFairnessBeta",
            LoomSearchFilters {
                source_kinds: vec![
                    LoomSearchSourceKind::Symbol,
                    LoomSearchSourceKind::WorkPacket,
                    LoomSearchSourceKind::MicroTask,
                ],
                ..Default::default()
            },
            25,
            0,
        )
        .await
        .expect("grouped knowledge-source fairness search");

    let source_kinds: Vec<_> = hits.iter().map(|hit| hit.source_kind.as_str()).collect();
    assert!(
        source_kinds.iter().any(|source| *source == "work_packet"),
        "grouped knowledge fetch must not let symbol rows starve work_packet hits: {source_kinds:?}"
    );
    assert!(
        source_kinds.iter().any(|source| *source == "micro_task"),
        "grouped knowledge fetch must not let symbol rows starve micro_task hits: {source_kinds:?}"
    );
}

#[tokio::test]
async fn mt256_document_fuzzy_search_matches_body_only_terms() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let document = insert_rich_document(
        &pg.db,
        &ws,
        "Reference standalone document",
        "Needle appears only in the rich document body.",
    )
    .await;

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "Needle",
            LoomSearchFilters {
                source_kinds: vec![LoomSearchSourceKind::Document],
                ..Default::default()
            },
            5,
            0,
        )
        .await
        .expect("body-only rich document fuzzy search");

    assert!(
        hits.iter()
            .any(|hit| hit.source_kind == LoomSearchSourceKind::Document && hit.ref_id == document),
        "single-token fuzzy document search must include content_json body-only matches: {hits:?}"
    );
}

#[tokio::test]
async fn mt256_graph_search_fuzzy_recall_is_not_capped_to_newest_candidates() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let matching_block = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::Note,
        "GraphSearchAlpha recall target",
        Some("GraphSearchAlpha must remain reachable even when many newer blocks exist"),
    )
    .await;

    for index in 0..520 {
        insert_loom_block(
            &pg.db,
            &ctx,
            &ws,
            LoomBlockContentType::Note,
            &format!("long standing analysis false fuzzy candidate {index:03}"),
            Some("This nonmatching block should not crowd out a fuzzy match"),
        )
        .await;
    }

    let hits = pg
        .db
        .search_loom_graph(
            &ws,
            "GSA",
            LoomSearchFilters {
                source_kinds: vec![LoomSearchSourceKind::LoomBlock],
                ..Default::default()
            },
            5,
            0,
        )
        .await
        .expect("recall-safe fuzzy graph search");

    assert!(
        hits.iter().any(|hit| hit.ref_id == matching_block),
        "fuzzy search must not miss a valid old match outside the newest candidate window: {hits:?}"
    );

    let block_hits = pg
        .db
        .search_loom_blocks(&ws, "GSA", LoomSearchFilters::default(), 5, 0)
        .await
        .expect("recall-safe fuzzy block search");
    assert!(
        block_hits
            .iter()
            .any(|hit| hit.block.block_id == matching_block),
        "direct Loom block fuzzy search must not miss the old true match behind SQL-only false candidates: {block_hits:?}"
    );
}

#[tokio::test]
async fn mt186_graph_search_offsets_after_prefetching_each_source() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    for index in 0..15 {
        insert_loom_block(
            &pg.db,
            &ctx,
            &ws,
            LoomBlockContentType::Note,
            &format!("GraphSearchPageTerm note {index:02}"),
            Some("GraphSearchPageTerm appears in enough notes to exercise offset pagination"),
        )
        .await;
    }

    let page = pg
        .db
        .search_loom_graph(
            &ws,
            "GraphSearchPageTerm",
            LoomSearchFilters::default(),
            5,
            10,
        )
        .await
        .expect("offset graph search");
    assert_eq!(
        page.len(),
        5,
        "offset pagination must not drop valid source rows by fetching only limit rows first"
    );
    assert!(
        page.iter()
            .all(|hit| hit.source_kind == LoomSearchSourceKind::LoomBlock),
        "pagination fixture should only return Loom blocks: {page:?}"
    );
}

#[tokio::test]
async fn mt186_block_scoped_filters_do_not_leak_non_block_hits() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    let tag = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::TagHub,
        "Filter leak tag hub",
        None,
    )
    .await;
    let tagged = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::Note,
        "FilterLeakAlpha tagged note",
        Some("FilterLeakAlpha should survive the tag filter"),
    )
    .await;
    let _untagged = insert_loom_block(
        &pg.db,
        &ctx,
        &ws,
        LoomBlockContentType::Note,
        "FilterLeakAlpha untagged note",
        Some("FilterLeakAlpha should be excluded by the tag filter"),
    )
    .await;
    pg.db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: tagged.clone(),
                target_block_id: tag.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("tag edge");
    insert_entity(
        &pg.db,
        &ws,
        KnowledgeEntityKind::Symbol,
        "rust:filter_leak#FilterLeakAlpha",
        "FilterLeakAlpha symbol",
    )
    .await;
    insert_user_manual_page(
        &pg.db,
        "filter-leak-alpha",
        "FilterLeakAlpha UserManual page",
        "FilterLeakAlpha appears in manual content but cannot satisfy a Loom tag filter.",
    )
    .await;

    let filtered = pg
        .db
        .search_loom_graph(
            &ws,
            "FilterLeakAlpha",
            LoomSearchFilters {
                tag_ids: vec![tag.clone()],
                ..Default::default()
            },
            20,
            0,
        )
        .await
        .expect("tag-filtered graph search");
    assert_eq!(filtered.len(), 1, "tag filter must not leak graph hits");
    assert_eq!(filtered[0].source_kind, LoomSearchSourceKind::LoomBlock);
    assert_eq!(filtered[0].ref_id, tagged);

    let impossible_symbol = pg
        .db
        .search_loom_graph(
            &ws,
            "FilterLeakAlpha",
            LoomSearchFilters {
                tag_ids: vec![tag],
                source_kinds: vec![LoomSearchSourceKind::Symbol],
                ..Default::default()
            },
            20,
            0,
        )
        .await
        .expect("symbol plus tag-filter graph search");
    assert!(
        impossible_symbol.is_empty(),
        "source_kind=symbol cannot satisfy Loom tag filters"
    );
}
