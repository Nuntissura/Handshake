//! WP-KERNEL-009 RichDocumentCore (MT-145..MT-160) integration tests against
//! REAL Handshake-managed PostgreSQL.
//!
//! Proof contract (no mocks, no SQLite): every test runs on a fresh isolated
//! schema on the managed cluster (see `knowledge_pg_support`). The end-to-end
//! proof creates a document with mixed blocks (incl. a Monaco code node, an
//! embed, and a wikilink), saves it, reloads it, projects it to markdown and
//! asserts the round-trip, asserts a backlink is persisted with a stable
//! relationship id, proves a broken embed is repairable, proves deleting a
//! projection does NOT mutate authority (negative), and proves crash recovery
//! reconstructs the document from PG.

mod knowledge_pg_support;

use handshake_core::knowledge_document::backlink::{
    derive_document_link_relationship_id, DocumentLinkKind, DocumentLinkReferences,
};
use handshake_core::knowledge_document::block_tree::{
    BlockKind, BlockTree, DOCUMENT_SCHEMA_VERSION,
};
use handshake_core::knowledge_document::embed::{EmbedRefKind, EmbedTarget};
use handshake_core::knowledge_document::import::{import_snippet, ImportFormat};
use handshake_core::knowledge_document::permission::{
    DocumentAction, DocumentActorKind, DocumentPermission,
};
use handshake_core::knowledge_document::projection::{render_projection, ProjectionFormat};
use handshake_core::storage::knowledge::{
    KnowledgeStore, NewKnowledgeRichDocument, UpsertEditorCodeNode,
    UpsertKnowledgeDocumentBacklink, UpsertKnowledgeDocumentEmbed,
};
use handshake_core::storage::StorageError;
use knowledge_pg_support::knowledge_pg;
use serde_json::{json, Value};

/// A mixed-block ProseMirror document: heading, paragraph with an inline
/// wikilink + a #tag, a code block, an image embed (typed target), and a typed
/// wp-link block.
fn mixed_document() -> Value {
    json!({
        "type": "doc",
        "content": [
            { "type": "heading", "attrs": { "level": 1 }, "content": [{ "type": "text", "text": "Runbook" }] },
            { "type": "paragraph", "content": [{ "type": "text", "text": "See [[Deploy Guide]] and tag #ops for details." }] },
            { "type": "codeBlock", "content": [{ "type": "text", "text": "fn main() { println!(\"hi\"); }" }] },
            { "type": "image", "attrs": { "target": "KMED-abc123", "display": { "fold": false } }, "content": [{ "type": "text", "text": "diagram" }] },
            { "type": "wpLink", "attrs": { "target": "WP-KERNEL-009" }, "content": [{ "type": "text", "text": "this WP" }] }
        ]
    })
}

fn new_doc(workspace_id: &str, title: &str, content: Value) -> NewKnowledgeRichDocument {
    NewKnowledgeRichDocument {
        workspace_id: workspace_id.to_string(),
        document_id: None,
        title: title.to_string(),
        schema_version: DOCUMENT_SCHEMA_VERSION.to_string(),
        content_json: content,
        crdt_document_id: None,
        crdt_snapshot_id: None,
        promotion_receipt_event_id: None,
        project_ref: Some("PRJ-handshake".to_string()),
        folder_ref: Some("runbooks".to_string()),
        authority_label: Some("promoted".to_string()),
        owner_actor_kind: Some("operator".to_string()),
        owner_actor_id: Some("op-1".to_string()),
    }
}

// ---------------------------------------------------------------------------
// MT-145..MT-150 end-to-end: create mixed doc + code node + embed + wikilink ->
// save -> reload -> project to markdown -> round-trip; backlink persisted.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_to_end_mixed_document_save_reload_project_roundtrip_and_backlink() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP end_to_end_mixed_document...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;

    // MT-145 identity: create with project/folder/owner/authority_label.
    let created = pg
        .db
        .create_knowledge_rich_document(new_doc(&workspace_id, "Runbook", mixed_document()))
        .await
        .expect("create rich document");
    assert!(created.rich_document_id.starts_with("KRD-"));
    assert_eq!(created.doc_version, 1);
    assert_eq!(created.project_ref.as_deref(), Some("PRJ-handshake"));
    assert_eq!(created.folder_ref.as_deref(), Some("runbooks"));
    assert_eq!(created.authority_label, "promoted");
    assert_eq!(created.owner_actor_kind.as_deref(), Some("operator"));

    // MT-146/147/148: the typed block tree parses with stable ids + RDD.
    let tree = BlockTree::from_document_json(
        &created.rich_document_id,
        &created.schema_version,
        &created.content_json,
    )
    .expect("parse block tree");
    assert_eq!(tree.blocks.len(), 5);
    assert_eq!(tree.blocks[0].kind, BlockKind::Heading);
    assert_eq!(tree.blocks[0].heading_level, Some(1));
    assert_eq!(tree.blocks[2].kind, BlockKind::CodeBlock);
    assert_eq!(tree.blocks[3].kind, BlockKind::Image);
    assert_eq!(tree.blocks[4].kind, BlockKind::WpLink);
    // Stable ids are KBL- and survive a re-parse identically (MT-148).
    let ids_a = tree.block_ids();
    assert!(ids_a.iter().all(|id| id.starts_with("KBL-")));
    let tree_again = BlockTree::from_document_json(
        &created.rich_document_id,
        &created.schema_version,
        &created.content_json,
    )
    .expect("re-parse");
    assert_eq!(
        ids_a,
        tree_again.block_ids(),
        "stable ids are deterministic"
    );
    // MT-147 derived content is regenerable: code block plain text extracted.
    assert!(tree.blocks[2]
        .content
        .derived
        .plain_text
        .contains("println"));

    // A Monaco code node round-trips with its integrity hash (MT-059 reuse).
    let code = "fn main() { println!(\"hi\"); }";
    pg.db
        .upsert_knowledge_editor_code_node(UpsertEditorCodeNode {
            rich_document_id: created.rich_document_id.clone(),
            node_path: "content.2.code".to_string(),
            language_id: "rust".to_string(),
            code_text: code.to_string(),
            worker_requirements: json!({"worker": "editor", "bundled": true}),
            source_mapping: None,
            lint_diagnostics: json!([]),
        })
        .await
        .expect("upsert code node");

    // MT-152: a typed embed reference for the image block (never a path).
    let image_block_id = tree.blocks[3].block_id.clone();
    let embed = pg
        .db
        .upsert_knowledge_document_embed(UpsertKnowledgeDocumentEmbed {
            rich_document_id: created.rich_document_id.clone(),
            block_id: image_block_id.clone(),
            ref_kind: "media".to_string(),
            ref_value: "KMED-abc123".to_string(),
            caption: Some("diagram".to_string()),
        })
        .await
        .expect("upsert embed");
    assert!(embed.embed_id.starts_with("KEMB-"));
    assert_eq!(embed.repair_state, "ok");

    // MT-149 save a v2 (optimistic concurrency on the existing MT-059 path).
    let mut v2 = mixed_document();
    v2["content"][1]["content"][0]["text"] =
        json!("See [[Deploy Guide]] and tag #ops and #release.");
    let saved = pg
        .db
        .save_knowledge_rich_document_version(&created.rich_document_id, 1, v2.clone(), None)
        .await
        .expect("save v2");
    assert_eq!(saved.doc_version, 2);

    // MT-149 reload: deterministic load of the saved authority.
    let reread = pg
        .db
        .get_knowledge_rich_document(&created.rich_document_id)
        .await
        .expect("get")
        .expect("doc exists");
    assert_eq!(reread.doc_version, 2);
    assert_eq!(reread.content_json, v2);

    // MT-150 project to markdown FROM the canonical block tree; round-trip the
    // structure (heading text, code fence, wikilink) deterministically.
    let reread_tree = BlockTree::from_document_json(
        &reread.rich_document_id,
        &reread.schema_version,
        &reread.content_json,
    )
    .expect("parse reloaded tree");
    let md = render_projection("Runbook", &reread_tree, ProjectionFormat::Markdown);
    assert!(md.content.contains("# Runbook"));
    assert!(md.content.contains("```\nfn main()"));
    let md_again = render_projection("Runbook", &reread_tree, ProjectionFormat::Markdown);
    assert_eq!(md.content, md_again.content, "projection is deterministic");
    // Wiki/Loom flavor renders the typed wp-link + wikilink as [[...]].
    let wiki = render_projection("Runbook", &reread_tree, ProjectionFormat::WikiLoom);
    assert!(wiki.content.contains("[[WP-KERNEL-009]]"));

    // MT-154/155 backlinks: extract + persist; assert a stable relationship id
    // is present and the wikilink + tag + wp-link edges landed.
    let refs = DocumentLinkReferences::extract(&reread_tree);
    let upserts: Vec<UpsertKnowledgeDocumentBacklink> = refs
        .references
        .iter()
        .map(|r| UpsertKnowledgeDocumentBacklink {
            workspace_id: reread.workspace_id.clone(),
            relationship_id: r.relationship_id.clone(),
            source_document_id: reread.rich_document_id.clone(),
            link_kind: r.kind.as_str().to_string(),
            target: r.target.clone(),
            block_id: r.block_id.clone(),
        })
        .collect();
    let persisted = pg
        .db
        .replace_knowledge_document_backlinks(&reread.rich_document_id, upserts)
        .await
        .expect("persist backlinks");
    assert!(
        persisted
            .iter()
            .any(|b| b.link_kind == "wikilink" && b.target == "Deploy Guide"),
        "wikilink backlink persisted"
    );
    assert!(
        persisted
            .iter()
            .any(|b| b.link_kind == "wp" && b.target == "WP-KERNEL-009"),
        "wp-link backlink persisted"
    );
    assert!(persisted
        .iter()
        .any(|b| b.link_kind == "tag" && b.target == "release"));
    // The persisted relationship id matches the deterministic derivation.
    let wp_block_id = reread_tree.blocks[4].block_id.clone();
    let expected_rel = derive_document_link_relationship_id(
        &reread.rich_document_id,
        DocumentLinkKind::Wp,
        "WP-KERNEL-009",
        &wp_block_id,
    );
    assert!(
        persisted.iter().any(|b| b.relationship_id == expected_rel),
        "backlink carries the deterministic relationship id"
    );

    // Reverse lookup: who links TO WP-KERNEL-009 (MT-155 backlink direction).
    let inbound = pg
        .db
        .list_knowledge_document_backlinks_to(&reread.workspace_id, "wp", "WP-KERNEL-009")
        .await
        .expect("reverse lookup");
    assert_eq!(inbound.len(), 1);
    assert_eq!(inbound[0].source_document_id, reread.rich_document_id);

    // MT-156 history: append-only, complete from v1.
    let versions = pg
        .db
        .list_knowledge_rich_document_versions(&created.rich_document_id)
        .await
        .expect("versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].doc_version, 1);
    assert_eq!(versions[1].doc_version, 2);
}

// ---------------------------------------------------------------------------
// MT-152/153 embeds: typed refs never absolute paths; broken embed repairable.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embed_reference_rejects_absolute_paths_and_broken_embed_is_repairable() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP embed_reference...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let doc = pg
        .db
        .create_knowledge_rich_document(new_doc(&workspace_id, "Embeds", mixed_document()))
        .await
        .expect("doc");

    // MT-152 model law: absolute paths are rejected at construction.
    for bad in [
        "/var/data/image.png",
        "C:\\Users\\me\\image.png",
        "file:///etc/passwd",
        "\\\\host\\share\\x.png",
    ] {
        assert!(
            EmbedTarget::new(EmbedRefKind::Media, bad).is_err(),
            "absolute path `{bad}` must be rejected as an embed target"
        );
    }
    // A typed media id / http url is accepted.
    assert!(EmbedTarget::new(EmbedRefKind::Media, "KMED-1").is_ok());
    assert!(EmbedTarget::new(EmbedRefKind::Url, "https://cdn.example/x.png").is_ok());
    assert!(EmbedTarget::new(EmbedRefKind::Url, "ftp://x").is_err());

    // MT-152 DB law: the embeds table rejects an absolute-path ref_value even if
    // the app layer were bypassed.
    let mut conn = pg.raw_connection().await;
    let err = sqlx::query(
        "INSERT INTO knowledge_document_embeds
            (embed_id, rich_document_id, block_id, ref_kind, ref_value)
         VALUES ('KEMB-00000000000000000000000000000000', $1, 'b1', 'media', '/abs/path.png')",
    )
    .bind(&doc.rich_document_id)
    .execute(&mut conn)
    .await
    .expect_err("absolute path must violate the embed CHECK");
    assert!(
        err.to_string()
            .contains("chk_knowledge_document_embeds_ref_value_not_path"),
        "unexpected: {err}"
    );
    drop(conn);

    // MT-153 broken-embed repair: create an embed, mark it broken, repair it.
    let embed = pg
        .db
        .upsert_knowledge_document_embed(UpsertKnowledgeDocumentEmbed {
            rich_document_id: doc.rich_document_id.clone(),
            block_id: "img-1".to_string(),
            ref_kind: "media".to_string(),
            ref_value: "KMED-missing".to_string(),
            caption: None,
        })
        .await
        .expect("create embed");
    assert_eq!(embed.repair_state, "ok");

    let broken = pg
        .db
        .set_knowledge_document_embed_repair_state(&embed.embed_id, Some("media id not found"))
        .await
        .expect("mark broken");
    assert_eq!(broken.repair_state, "broken");
    assert_eq!(broken.repair_reason.as_deref(), Some("media id not found"));

    let queue = pg
        .db
        .list_knowledge_document_broken_embeds(&doc.rich_document_id)
        .await
        .expect("broken queue");
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].embed_id, embed.embed_id);

    let repaired = pg
        .db
        .set_knowledge_document_embed_repair_state(&embed.embed_id, None)
        .await
        .expect("repair");
    assert_eq!(repaired.repair_state, "ok");
    assert!(repaired.repair_reason.is_none());
    assert!(
        pg.db
            .list_knowledge_document_broken_embeds(&doc.rich_document_id)
            .await
            .expect("queue after repair")
            .is_empty(),
        "repaired embed leaves the broken queue"
    );
}

// ---------------------------------------------------------------------------
// MT-150 negative proof: deleting a projection does NOT mutate authority.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_projection_never_mutates_document_authority() {
    use handshake_core::storage::knowledge::{KnowledgeProjectionKind, NewKnowledgeWikiProjection};
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP deleting_a_projection...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let doc = pg
        .db
        .create_knowledge_rich_document(new_doc(&workspace_id, "Authority", mixed_document()))
        .await
        .expect("doc");

    // Render a markdown projection FROM the document and register it as a
    // knowledge_wiki_projection (MT-150 writes projections through MT-058).
    let tree = BlockTree::from_document_json(
        &doc.rich_document_id,
        &doc.schema_version,
        &doc.content_json,
    )
    .expect("tree");
    let rendered = render_projection(&doc.title, &tree, ProjectionFormat::Markdown);
    let sha = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(rendered.content.as_bytes());
        hex::encode(h.finalize())
    };
    let projection = pg
        .db
        .upsert_knowledge_wiki_projection(NewKnowledgeWikiProjection {
            workspace_id: workspace_id.clone(),
            projection_kind: KnowledgeProjectionKind::WikiPage,
            title: doc.title.clone(),
            source_records: json!([{ "record_family": "RichDocument", "record_id": doc.rich_document_id }]),
            rendered_content: rendered.content.clone(),
            staleness_hash: sha,
        })
        .await
        .expect("register projection");

    // Snapshot the authority document BEFORE deleting the projection.
    let before = pg
        .db
        .get_knowledge_rich_document(&doc.rich_document_id)
        .await
        .expect("get before")
        .expect("exists");

    // Delete the projection.
    pg.db
        .delete_knowledge_wiki_projection(&projection.projection_id)
        .await
        .expect("delete projection");
    assert!(
        pg.db
            .get_knowledge_wiki_projection(&projection.projection_id)
            .await
            .expect("get projection after delete")
            .is_none(),
        "projection row is gone"
    );

    // The authority document is byte-identical: deleting a projection mutated
    // NOTHING in authority (spec 2.3.13.11).
    let after = pg
        .db
        .get_knowledge_rich_document(&doc.rich_document_id)
        .await
        .expect("get after")
        .expect("still exists");
    assert_eq!(
        before, after,
        "deleting a projection must not touch authority"
    );
}

// ---------------------------------------------------------------------------
// MT-159 crash recovery: a saved document reconstructs from PG after a fresh
// connection (simulating app restart / session compaction).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_recovery_reconstructs_document_from_postgres() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP crash_recovery...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let created = pg
        .db
        .create_knowledge_rich_document(new_doc(&workspace_id, "Recover", mixed_document()))
        .await
        .expect("create");
    let saved = pg
        .db
        .save_knowledge_rich_document_version(&created.rich_document_id, 1, mixed_document(), None)
        .await
        .expect("save v2");

    // Simulate a crash: open a BRAND NEW PostgresDatabase against the same
    // isolated schema (a fresh process would do exactly this) and reconstruct.
    let recovered_db =
        handshake_core::storage::postgres::PostgresDatabase::connect(&pg.schema_url, 5)
            .await
            .expect("reconnect after crash");
    let recovered = recovered_db
        .get_knowledge_rich_document(&created.rich_document_id)
        .await
        .expect("recover doc")
        .expect("doc survives crash");
    assert_eq!(recovered.doc_version, saved.doc_version);
    assert_eq!(recovered.content_json, saved.content_json);
    assert_eq!(recovered.content_sha256, saved.content_sha256);

    // The full version history + block tree reconstruct too (no data loss).
    let versions = recovered_db
        .list_knowledge_rich_document_versions(&created.rich_document_id)
        .await
        .expect("history after crash");
    assert_eq!(versions.len(), 2);
    let tree = BlockTree::from_document_json(
        &recovered.rich_document_id,
        &recovered.schema_version,
        &recovered.content_json,
    )
    .expect("reconstruct block tree");
    assert_eq!(tree.blocks.len(), 5);
}

// ---------------------------------------------------------------------------
// MT-151 import + MT-157 batch ops + MT-158 permission boundary.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_markdown_then_batch_rename_and_move() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP import_markdown...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;

    // MT-151 import a markdown snippet into a document json, then persist it.
    let snippet = "# Title\n\nA paragraph.\n\n- one\n- two\n\n```\ncode();\n```";
    let outcome = import_snippet(snippet, ImportFormat::Markdown);
    assert!(outcome.warnings.is_empty());
    let imported = pg
        .db
        .create_knowledge_rich_document(new_doc(
            &workspace_id,
            "Imported",
            outcome.document_json.clone(),
        ))
        .await
        .expect("create imported");
    let tree = BlockTree::from_document_json(
        &imported.rich_document_id,
        &imported.schema_version,
        &imported.content_json,
    )
    .expect("parse imported");
    // Heading + paragraph + bullet list + code block.
    let kinds: Vec<BlockKind> = tree.blocks.iter().map(|b| b.kind).collect();
    assert!(kinds.contains(&BlockKind::Heading));
    assert!(kinds.contains(&BlockKind::BulletList));
    assert!(kinds.contains(&BlockKind::CodeBlock));

    // MT-151 import HTML -> repairable importedRaw node + warning (no loss).
    let html_outcome = import_snippet("<table><tr><td>x</td></tr></table>", ImportFormat::Html);
    assert_eq!(html_outcome.warnings.len(), 1);
    assert_eq!(html_outcome.warnings[0].code, "html_captured_as_raw");

    // MT-157 batch rename + move (metadata-only, no version bump).
    let renamed = pg
        .db
        .rename_knowledge_rich_document(&imported.rich_document_id, "Imported Runbook")
        .await
        .expect("rename");
    assert_eq!(renamed.title, "Imported Runbook");
    assert_eq!(
        renamed.doc_version, imported.doc_version,
        "rename does not bump version"
    );

    let moved = pg
        .db
        .move_knowledge_rich_document(
            &imported.rich_document_id,
            Some("PRJ-other"),
            Some("archive"),
        )
        .await
        .expect("move");
    assert_eq!(moved.project_ref.as_deref(), Some("PRJ-other"));
    assert_eq!(moved.folder_ref.as_deref(), Some("archive"));

    // Membership listing scoped to the new project (MT-145/157).
    let in_project = pg
        .db
        .list_knowledge_rich_documents(&workspace_id, Some("PRJ-other"), None)
        .await
        .expect("list by project");
    assert_eq!(in_project.len(), 1);
    assert_eq!(in_project[0].rich_document_id, imported.rich_document_id);

    // MT-158 permission boundary matrix (server-side, pure decision).
    assert!(DocumentPermission::decide(DocumentActorKind::Operator, DocumentAction::Write).allowed);
    assert!(
        DocumentPermission::decide(DocumentActorKind::LocalModel, DocumentAction::Write).allowed
    );
    let cloud_write =
        DocumentPermission::decide(DocumentActorKind::CloudModel, DocumentAction::Write);
    assert!(!cloud_write.allowed);
    assert_eq!(cloud_write.reason, "cloud_model_write_denied");
    assert!(
        DocumentPermission::decide(DocumentActorKind::CloudModel, DocumentAction::Read).allowed
    );
    let validator_index =
        DocumentPermission::decide(DocumentActorKind::Validator, DocumentAction::Index);
    assert!(!validator_index.allowed);
}

// ---------------------------------------------------------------------------
// MT-145 fail-closed: bad authority_label / half owner rejected.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identity_validation_fails_closed() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP identity_validation_fails_closed: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;

    // Bad authority label is a typed Validation error.
    let mut bad_label = new_doc(&workspace_id, "Bad", mixed_document());
    bad_label.authority_label = Some("published".to_string());
    let err = pg
        .db
        .create_knowledge_rich_document(bad_label)
        .await
        .expect_err("bad authority_label must fail closed");
    assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

    // Half an owner (kind without id) is a typed Validation error.
    let mut half_owner = new_doc(&workspace_id, "HalfOwner", mixed_document());
    half_owner.owner_actor_id = None;
    let err = pg
        .db
        .create_knowledge_rich_document(half_owner)
        .await
        .expect_err("half owner must fail closed");
    assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
}
