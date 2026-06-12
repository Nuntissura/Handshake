//! MT-195 UserManualBuildUpdateRule + MT-200 UserManualInAppAccess: the typed
//! WP-009 surface registry and the in-app access-point registry.
//!
//! Build-update law (spec 10.15.8): "New WP-KERNEL-009 surfaces MUST add or
//! update UserManual coverage in the same implementation unit as the wired
//! surface. A ... route ... without UserManual coverage is a build-rule
//! defect." This registry is the enforcement substrate:
//!
//! * Every WP-009 model-callable HTTP surface is a [`SurfaceDescriptor`] row.
//! * The MT-195 gate test (`tests/user_manual_storage_tests.rs`) fails when a
//!   registry row has no `http_route` anchor on a seeded UserManual page —
//!   adding a surface here without seeding coverage breaks the build.
//! * The MT-204 freshness check flips the direction: an `http_route` anchor
//!   with no registry row is a `dangling_anchor` (stale docs), and a registry
//!   row without an anchor is an `uncovered_surface`.
//! * The doc-vs-runtime consistency test mounts the REAL `api::routes` router
//!   and probes every registry row: a documented route that 404s at the
//!   router level (or 405s on its documented method) is a defect — the manual
//!   can never describe a surface the product does not serve.
//!
//! Rows describe surfaces that are COMMITTED on feat/WP-KERNEL-009. Surfaces
//! added by later groups must land here in the same implementation unit
//! (that is the build rule).

use serde::Serialize;

/// Which product surface family a row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceGroup {
    KnowledgeIngestion,
    CodeNavigation,
    RichDocuments,
    Retrieval,
    MemoryClaims,
    CrdtCollaboration,
    NotesLoom,
    UserManual,
}

impl SurfaceGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KnowledgeIngestion => "knowledge_ingestion",
            Self::CodeNavigation => "code_navigation",
            Self::RichDocuments => "rich_documents",
            Self::Retrieval => "retrieval",
            Self::MemoryClaims => "memory_claims",
            Self::CrdtCollaboration => "crdt_collaboration",
            Self::NotesLoom => "notes_loom",
            Self::UserManual => "user_manual",
        }
    }

    /// The UserManual page slug that documents this group's surfaces.
    pub fn page_slug(self) -> &'static str {
        match self {
            Self::KnowledgeIngestion | Self::CodeNavigation => "knowledge-index-surface",
            Self::RichDocuments => "rich-documents-surface",
            Self::Retrieval => "retrieval-and-context-bundles-surface",
            Self::MemoryClaims => "memory-and-claims-surface",
            Self::CrdtCollaboration => "crdt-collaboration-surface",
            Self::NotesLoom => "notes-loom-surface",
            Self::UserManual => "usermanual-surface",
        }
    }

    /// Whether the group's endpoints REQUIRE the x-hsk identity headers
    /// (400 without them). The Notes/Loom and UserManual read surfaces accept
    /// anonymous reads; UserManual synthesizes bootstrap identity instead.
    pub fn requires_identity_headers(self) -> bool {
        match self {
            Self::KnowledgeIngestion
            | Self::CodeNavigation
            | Self::RichDocuments
            | Self::Retrieval
            | Self::MemoryClaims
            | Self::CrdtCollaboration => true,
            Self::NotesLoom | Self::UserManual => false,
        }
    }
}

/// One model-callable WP-009 HTTP surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SurfaceDescriptor {
    /// Stable id; doubles as the `user_manual_tool_entries.tool_id`.
    pub surface_id: &'static str,
    pub group: SurfaceGroup,
    /// `GET` | `POST` | `PUT` | `DELETE` | `PATCH`.
    pub method: &'static str,
    /// The axum route pattern exactly as registered.
    pub route: &'static str,
    pub summary: &'static str,
    pub expected_input: &'static str,
    pub expected_output: &'static str,
}

/// The committed WP-009 surface inventory. Ordering is stable (group, route).
pub fn wp009_surface_registry() -> &'static [SurfaceDescriptor] {
    SURFACES
}

macro_rules! surface {
    ($id:literal, $group:expr, $method:literal, $route:literal, $summary:literal, $in:literal, $out:literal) => {
        SurfaceDescriptor {
            surface_id: $id,
            group: $group,
            method: $method,
            route: $route,
            summary: $summary,
            expected_input: $in,
            expected_output: $out,
        }
    };
}

const SURFACES: &[SurfaceDescriptor] = &[
    // -- knowledge_ingestion (api/knowledge_ingestion.rs) ------------------
    surface!("knowledge.ingestion.roots.list", SurfaceGroup::KnowledgeIngestion, "GET",
        "/knowledge/ingestion/roots",
        "List registered ingestion source roots with allowlist policy state.",
        "Identity headers; optional workspace_id query.",
        "JSON array of source-root rows (root_id, root_kind, policy)."),
    surface!("knowledge.ingestion.root_sources.list", SurfaceGroup::KnowledgeIngestion, "GET",
        "/knowledge/ingestion/roots/:root_id/sources",
        "List sources under a root with hashes and extraction status.",
        "Identity headers; root_id path param.",
        "JSON array of source rows (source_id, path, content hash, status)."),
    surface!("knowledge.ingestion.runs.start", SurfaceGroup::KnowledgeIngestion, "POST",
        "/knowledge/ingestion/runs",
        "Start an ingestion/index run over configured roots; emits KNOWLEDGE_INDEX_RUN_* receipts.",
        "Identity headers; JSON body naming workspace and roots.",
        "JSON run row (run_id, state) plus EventLedger receipt id."),
    surface!("knowledge.ingestion.source_receipts.list", SurfaceGroup::KnowledgeIngestion, "GET",
        "/knowledge/ingestion/sources/:source_id/receipts",
        "List extraction receipts for one source (success, partial, failed).",
        "Identity headers; source_id path param.",
        "JSON array of extraction receipts with error classes."),
    surface!("knowledge.ingestion.repairs.list", SurfaceGroup::KnowledgeIngestion, "GET",
        "/knowledge/ingestion/repairs",
        "The ingestion repair queue: failed/partial extractions awaiting retry.",
        "Identity headers.",
        "JSON array of repair rows (repair_id, source, error_class, state)."),
    surface!("knowledge.ingestion.repairs.retry", SurfaceGroup::KnowledgeIngestion, "POST",
        "/knowledge/ingestion/repairs/:repair_id/retry",
        "Retry one failed ingestion repair; never silent-skips.",
        "Identity headers; repair_id path param.",
        "JSON updated repair row + receipt id."),
    // -- knowledge_code_nav (api/knowledge_code_nav.rs; closes the MT-112
    //    deferred model-manual registration for /knowledge/code/*) ---------
    surface!("knowledge.code.symbols.lookup", SurfaceGroup::CodeNavigation, "GET",
        "/knowledge/code/symbols",
        "Symbol lookup by simple name and/or file path over the indexed code graph (no external LSP).",
        "Identity headers; workspace_id + name/path/limit query params.",
        "JSON symbol list (entity_id, kind, file, span) + retrieval receipt id."),
    surface!("knowledge.code.symbols.get", SurfaceGroup::CodeNavigation, "GET",
        "/knowledge/code/symbols/:entity_id",
        "One symbol with definition span, doc, owning file, and staleness verdict.",
        "Identity headers; entity_id path param.",
        "JSON symbol detail incl. staleness + receipt id."),
    surface!("knowledge.code.symbols.references", SurfaceGroup::CodeNavigation, "GET",
        "/knowledge/code/symbols/:entity_id/references",
        "Outgoing references (callees) and incoming references (callers) with evidence spans.",
        "Identity headers; entity_id path param.",
        "JSON {outgoing, incoming} edge lists with span citations + receipt id."),
    surface!("knowledge.code.symbols.tests", SurfaceGroup::CodeNavigation, "GET",
        "/knowledge/code/symbols/:entity_id/tests",
        "Tests that validate this symbol (validates edges).",
        "Identity headers; entity_id path param.",
        "JSON test-entity list + receipt id."),
    surface!("knowledge.code.symbols.spans", SurfaceGroup::CodeNavigation, "GET",
        "/knowledge/code/symbols/:entity_id/spans",
        "The source spans (citations) the symbol was detected from.",
        "Identity headers; entity_id path param.",
        "JSON span list (path, range, content hash) + receipt id."),
    surface!("knowledge.code.file_lens", SurfaceGroup::CodeNavigation, "GET",
        "/knowledge/code/files/:path/lens",
        "Monaco code-lens payload for a file (symbols, references, tests per line).",
        "Identity headers; URL-encoded repo-relative path; workspace_id query.",
        "JSON code-lens payload + staleness + receipt id."),
    // -- knowledge_documents (api/knowledge_documents.rs) ------------------
    surface!("knowledge.documents.create", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents",
        "Create a RichDocument (Tiptap/ProseMirror JSON authority row); KNOWLEDGE_RICH_DOCUMENT_SAVED receipt.",
        "Identity headers (actor-kind write permission enforced); JSON {workspace_id, title, content_json}.",
        "JSON document row (document_id, doc_version 1) + receipt id."),
    surface!("knowledge.documents.import", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents/import",
        "Import markdown/HTML/plain/ProseMirror-JSON (incl. ImportedRaw fail-closed fallback) into a new document.",
        "Identity headers; JSON {workspace_id, format, content}; HTML sanitized fail-closed.",
        "JSON imported document row + import report (lossy notes, raw fallback)."),
    surface!("knowledge.documents.load", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id",
        "Load a RichDocument authority row with its typed block tree.",
        "Identity headers; document_id path param.",
        "JSON document + blocks (stable block ids, Raw/Derived/Display)."),
    surface!("knowledge.documents.save", SurfaceGroup::RichDocuments, "PUT",
        "/knowledge/documents/:document_id/save",
        "Optimistic-concurrency save: expected doc_version must match or 409 conflict.",
        "Identity headers; JSON {expected_doc_version, content_json}; embeds validated against the embed-target law.",
        "JSON saved row (doc_version+1) + KNOWLEDGE_RICH_DOCUMENT_SAVED receipt id; 409 {error: conflict} on stale version."),
    surface!("knowledge.documents.blocks", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/blocks",
        "The typed block tree only (block ids, kinds, depth) for stable rendering.",
        "Identity headers; document_id path param.",
        "JSON block list."),
    surface!("knowledge.documents.history.list", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/history",
        "Append-only revision history, PAGINATED (limit/offset), newest first.",
        "Identity headers; limit+offset query params (bounded).",
        "JSON revision list (doc_version, saved_by, receipt id, content hash)."),
    surface!("knowledge.documents.history.version", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/history/:doc_version",
        "One historical revision payload.",
        "Identity headers; doc_version path param.",
        "JSON revision row with content_json."),
    surface!("knowledge.documents.projection", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/projection",
        "Render a projection (markdown lossy / HTML structure-aware / text / json) of the authority row.",
        "Identity headers; format query param.",
        "JSON {format, rendered} — projection only, never authority."),
    surface!("knowledge.documents.embeds.list", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/embeds",
        "List the document's typed embed references (artifact/media/source/url).",
        "Identity headers; document_id path param.",
        "JSON embed rows with resolution state."),
    surface!("knowledge.documents.embeds.broken", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/embeds/broken",
        "The broken-embed repair queue for a document (typed broken state, never a blank node).",
        "Identity headers; document_id path param.",
        "JSON broken-embed rows each offering retarget/remove/keep repair actions."),
    surface!("knowledge.documents.embeds.repair", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents/embeds/:embed_id/repair",
        "Apply one repair action to a broken embed.",
        "Identity headers (write permission); JSON {action, new_target?}.",
        "JSON repaired embed row + receipt id."),
    surface!("knowledge.documents.backlinks.list", SurfaceGroup::RichDocuments, "GET",
        "/knowledge/documents/:document_id/backlinks",
        "Forward links and reverse backlinks for a document (stable relationship ids).",
        "Identity headers; document_id path param.",
        "JSON {outgoing, incoming} link rows."),
    surface!("knowledge.documents.backlinks.rebuild", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents/:document_id/backlinks",
        "Rebuild the document's backlink index rows (index permission enforced).",
        "Identity headers; document_id path param.",
        "JSON rebuilt link rows + receipt id."),
    surface!("knowledge.documents.rename", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents/:document_id/rename",
        "Rename a document; backlink display text stays consistent.",
        "Identity headers (write permission); JSON {new_title}.",
        "JSON updated row + receipt id."),
    surface!("knowledge.documents.move", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents/:document_id/move",
        "Move a document between projects/folders.",
        "Identity headers (write permission); JSON {target}.",
        "JSON updated row + receipt id."),
    surface!("knowledge.documents.batch", SurfaceGroup::RichDocuments, "POST",
        "/knowledge/documents/batch",
        "Safe batch rename/move/set-owner over MANY documents (canonical set, not a UI subset).",
        "Identity headers (write permission); JSON {operation, document_ids, args}.",
        "JSON per-document results (no silent partial success)."),
    // -- knowledge_retrieval (api/knowledge_retrieval.rs) ------------------
    surface!("knowledge.retrieval.bundle.get", SurfaceGroup::Retrieval, "GET",
        "/knowledge/retrieval/bundles/:bundle_id",
        "Load a compiled context bundle with per-item retrieval decisions and citations.",
        "Identity headers; bundle_id (CTX-...) path param.",
        "JSON bundle + items (ref_kind, decision, citation, tokens)."),
    surface!("knowledge.retrieval.bundle.export", SurfaceGroup::Retrieval, "GET",
        "/knowledge/retrieval/bundles/:bundle_id/export",
        "AI-ready evidence export manifest reconstructed from bundle + trace rows.",
        "Identity headers; bundle_id path param.",
        "JSON ai_ready_evidence_export@1 manifest."),
    surface!("knowledge.retrieval.bundle.staleness", SurfaceGroup::Retrieval, "GET",
        "/knowledge/retrieval/bundles/:bundle_id/staleness",
        "Explicit staleness verdict for a bundle (source hash drift per item).",
        "Identity headers; bundle_id path param.",
        "JSON staleness verdict rows (current/stale per cited source)."),
    surface!("knowledge.retrieval.bundle.repair", SurfaceGroup::Retrieval, "POST",
        "/knowledge/retrieval/bundles/:bundle_id/repair",
        "Recompile a stale bundle against current sources; new bundle + trace.",
        "Identity headers; bundle_id path param.",
        "JSON {old_bundle_id, new_bundle_id, trace_id} + receipt id."),
    surface!("knowledge.retrieval.catalog", SurfaceGroup::Retrieval, "GET",
        "/knowledge/retrieval/catalog",
        "The semantic retrieval catalog: addressable retrieval modes and scopes.",
        "Identity headers.",
        "JSON catalog rows."),
    // -- knowledge_memory (api/knowledge_memory.rs) ------------------------
    surface!("knowledge.memory.claim.get", SurfaceGroup::MemoryClaims, "GET",
        "/knowledge/memory/claims/:claim_id",
        "One memory claim with lifecycle state and evidence spans.",
        "Identity headers; claim_id path param.",
        "JSON claim row (probationary/stable/rejected/superseded/conflicted)."),
    surface!("knowledge.memory.conflicts.list", SurfaceGroup::MemoryClaims, "GET",
        "/knowledge/memory/conflicts",
        "Open memory conflicts awaiting resolution.",
        "Identity headers; optional workspace filter.",
        "JSON conflict rows with contradicting claim ids."),
    surface!("knowledge.memory.fact.get", SurfaceGroup::MemoryClaims, "GET",
        "/knowledge/memory/facts/:fact_id",
        "One memory fact with provenance.",
        "Identity headers; fact_id path param.",
        "JSON fact row."),
    surface!("knowledge.memory.neighborhood", SurfaceGroup::MemoryClaims, "GET",
        "/knowledge/memory/entities/:entity_id/neighborhood",
        "Graph neighborhood of an entity (bridge edges, bounded traversal).",
        "Identity headers; entity_id path param.",
        "JSON neighborhood (entities + edges)."),
    surface!("knowledge.memory.visual_debug", SurfaceGroup::MemoryClaims, "GET",
        "/knowledge/memory/visual-debug",
        "Visual-debug projection of memory state for the diagnostics surface.",
        "Identity headers.",
        "JSON visual-debug payload (stable selectors)."),
    // -- knowledge_crdt (api/knowledge_crdt.rs) ----------------------------
    surface!("knowledge.crdt.push", SurfaceGroup::CrdtCollaboration, "POST",
        "/knowledge/crdt/updates/push",
        "Push a Yjs-compatible CRDT update for a document draft; KNOWLEDGE_CRDT_UPDATE_RECORDED receipt.",
        "Identity headers; JSON {document_id, update (base64), state_vector}.",
        "JSON applied-update receipt; 409 on conflicting head."),
    surface!("knowledge.crdt.pull", SurfaceGroup::CrdtCollaboration, "GET",
        "/knowledge/crdt/updates/pull",
        "Pull CRDT updates since a state vector for draft sync.",
        "Identity headers; document_id + state_vector query params.",
        "JSON update batch (base64) + head state."),
    surface!("knowledge.crdt.conflict_state", SurfaceGroup::CrdtCollaboration, "GET",
        "/knowledge/crdt/conflict_state",
        "Current CRDT conflict/lease state for a document draft.",
        "Identity headers; document_id query param.",
        "JSON conflict state (head, leases, pending conflicts)."),
    // -- Notes/Loom (api/loom.rs; operator surface name is 'Notes',
    //    'Loom' is the engine term — DEC-001) ------------------------------
    surface!("workspaces.create", SurfaceGroup::NotesLoom, "POST",
        "/workspaces",
        "Create a workspace (PREREQUISITE surface from api/workspaces.rs: every Notes/Loom and document route is workspace-scoped).",
        "JSON {name}.",
        "JSON created workspace row (id)."),
    surface!("loom.blocks.create", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/blocks",
        "Create a LoomBlock (note/document/tag/media atom of the Notes surface).",
        "JSON NewLoomBlock {content_type, title, content...}.",
        "JSON created block row (block_id)."),
    surface!("loom.blocks.get", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/blocks/:block_id",
        "Load one LoomBlock with derived metrics.",
        "workspace_id + block_id path params.",
        "JSON block row; 404 {error} when absent."),
    surface!("loom.blocks.patch", SurfaceGroup::NotesLoom, "PATCH",
        "/workspaces/:workspace_id/loom/blocks/:block_id",
        "Update LoomBlock fields (title, content, tags).",
        "JSON LoomBlockUpdate subset.",
        "JSON updated block row."),
    surface!("loom.blocks.delete", SurfaceGroup::NotesLoom, "DELETE",
        "/workspaces/:workspace_id/loom/blocks/:block_id",
        "Delete a LoomBlock (bridge rows cascade; knowledge entity retired by service layer).",
        "workspace_id + block_id path params.",
        "Empty success; 404 when absent."),
    surface!("loom.blocks.metrics.recompute", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/blocks/:block_id/metrics/recompute",
        "Recompute derived metrics for one block.",
        "Path params.",
        "JSON recomputed derived metrics."),
    surface!("loom.blocks.knowledge", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/blocks/:block_id/knowledge",
        "The block's ProjectKnowledgeIndex bridge: knowledge entity + EventLedger receipt (authority binding).",
        "Path params.",
        "JSON {entity_id, ledger_event_id} bridge row."),
    surface!("loom.blocks.pin_order", SurfaceGroup::NotesLoom, "PUT",
        "/workspaces/:workspace_id/loom/blocks/:block_id/pin-order",
        "Set the block's ordinal in the reorderable Pins grid.",
        "JSON {pin_order}.",
        "JSON updated pin state."),
    surface!("loom.blocks.breadcrumbs", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/blocks/:block_id/breadcrumbs",
        "Navigation breadcrumbs across the entity spine for a block.",
        "Path params.",
        "JSON breadcrumb chain."),
    surface!("loom.blocks.backlinks", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/blocks/:block_id/backlinks",
        "Linked backlinks WITH context snippets for a block.",
        "Path params.",
        "JSON backlink rows."),
    surface!("loom.blocks.unlinked_mentions", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/blocks/:block_id/unlinked-mentions",
        "Scan for unlinked mentions of this block's title across the workspace.",
        "Path params.",
        "JSON mention candidates (block, snippet)."),
    surface!("loom.folders.list", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/folders",
        "List the folder tree with color labels and sort modes.",
        "workspace_id path param.",
        "JSON folder rows."),
    surface!("loom.folders.create", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/folders",
        "Create a folder (color label, parent, sort mode).",
        "JSON folder spec.",
        "JSON created folder row."),
    surface!("loom.folders.get", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/folders/:folder_id",
        "Load one folder.",
        "Path params.",
        "JSON folder row."),
    surface!("loom.folders.patch", SurfaceGroup::NotesLoom, "PATCH",
        "/workspaces/:workspace_id/loom/folders/:folder_id",
        "Update folder name/color/sort.",
        "JSON folder update subset.",
        "JSON updated folder row."),
    surface!("loom.folders.delete", SurfaceGroup::NotesLoom, "DELETE",
        "/workspaces/:workspace_id/loom/folders/:folder_id",
        "Delete a folder.",
        "Path params.",
        "Empty success; 404 when absent."),
    surface!("loom.folders.blocks.list", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/folders/:folder_id/blocks",
        "List blocks in a folder under its sort mode.",
        "Path params.",
        "JSON block rows."),
    surface!("loom.folders.blocks.add", SurfaceGroup::NotesLoom, "PUT",
        "/workspaces/:workspace_id/loom/folders/:folder_id/blocks/:block_id",
        "Add a block to a folder.",
        "Path params.",
        "JSON membership row."),
    surface!("loom.folders.blocks.remove", SurfaceGroup::NotesLoom, "DELETE",
        "/workspaces/:workspace_id/loom/folders/:folder_id/blocks/:block_id",
        "Remove a block from a folder.",
        "Path params.",
        "Empty success."),
    surface!("loom.wiki.compile", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/wiki",
        "Compile a project-wiki projection over Loom blocks (knowledge-as-compile-target).",
        "JSON wiki compile spec.",
        "JSON projection row (projection_id) + receipt."),
    surface!("loom.wiki.get", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/wiki/:projection_id",
        "Load a compiled wiki projection (projection only, never authority).",
        "Path params.",
        "JSON projection content."),
    surface!("loom.wiki.delete", SurfaceGroup::NotesLoom, "DELETE",
        "/workspaces/:workspace_id/loom/wiki/:projection_id",
        "Delete a wiki projection.",
        "Path params.",
        "Empty success."),
    surface!("loom.wiki.regenerate", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/wiki/:projection_id/regenerate",
        "Regenerate a stale wiki projection from current authority rows.",
        "Path params.",
        "JSON regenerated projection row + receipt."),
    surface!("loom.wiki.stale", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/wiki/:projection_id/stale",
        "Staleness verdict for a wiki projection against source blocks.",
        "Path params.",
        "JSON staleness verdict."),
    surface!("loom.wiki.overlays.list", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/wiki/:projection_id/overlays",
        "List editable overlays attached to a wiki projection.",
        "Path params.",
        "JSON overlay rows."),
    surface!("loom.wiki.overlays.add", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/wiki/:projection_id/overlays",
        "Attach an editable overlay to a wiki page (operator edits survive regeneration).",
        "JSON overlay spec.",
        "JSON created overlay row."),
    surface!("loom.wiki.overlays.delete", SurfaceGroup::NotesLoom, "DELETE",
        "/workspaces/:workspace_id/loom/wiki-overlays/:overlay_id",
        "Delete a wiki overlay.",
        "Path params.",
        "Empty success."),
    surface!("loom.import.markdown", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/import/markdown",
        "Import a markdown corpus (Obsidian-style vault) into LoomBlocks with wikilink resolution.",
        "JSON markdown import payload.",
        "JSON import report (blocks created, links resolved)."),
    surface!("loom.import.asset", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/import",
        "Import a file asset (media) into the workspace asset store.",
        "JSON/base64 asset import payload.",
        "JSON asset row (asset_id)."),
    surface!("loom.tags.list", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/tags",
        "List tag hubs (tag blocks with usage counts).",
        "workspace_id path param.",
        "JSON tag hub rows."),
    surface!("loom.tags.get", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/tags/:tag_block_id",
        "Load one tag hub.",
        "Path params.",
        "JSON tag hub row."),
    surface!("loom.tags.blocks", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/tags/:tag_block_id/blocks",
        "List blocks carrying a tag.",
        "Path params.",
        "JSON block rows."),
    surface!("loom.edges.create", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/edges",
        "Create a typed LoomEdge (link/reference/tag membership).",
        "JSON NewLoomEdge.",
        "JSON created edge row."),
    surface!("loom.edges.delete", SurfaceGroup::NotesLoom, "DELETE",
        "/workspaces/:workspace_id/loom/edges/:edge_id",
        "Delete a LoomEdge.",
        "Path params.",
        "Empty success."),
    surface!("loom.assets.get", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/assets/:asset_id",
        "Asset metadata (typed embed target for [[HS_images]]/[[HS_slideshow]]).",
        "Path params.",
        "JSON asset metadata."),
    surface!("loom.assets.content", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/assets/:asset_id/content",
        "Raw asset bytes.",
        "Path params.",
        "Binary body with content-type."),
    surface!("loom.assets.thumbnail", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/assets/:asset_id/thumbnail",
        "Asset thumbnail (preview pipeline output).",
        "Path params.",
        "Binary thumbnail or typed pending state."),
    surface!("loom.views.query", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/views/:view_type",
        "Query a saved Loom view (filters + sort over blocks).",
        "view_type path param + filter query params.",
        "JSON view response rows."),
    surface!("loom.graph.traverse", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/graph/traverse",
        "Bounded graph traversal from a start block (depth-capped at 8).",
        "start block + depth query params.",
        "JSON traversal (nodes, edges)."),
    surface!("loom.graph.local", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/graph/local",
        "Local graph around one block (Obsidian-style local view).",
        "block + depth query params.",
        "JSON local graph."),
    surface!("loom.graph.global", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/graph/global",
        "Global workspace graph with filtering.",
        "Filter query params.",
        "JSON global graph."),
    surface!("loom.metrics.recompute_all", SurfaceGroup::NotesLoom, "POST",
        "/workspaces/:workspace_id/loom/metrics/recompute",
        "Recompute derived metrics for all blocks in the workspace.",
        "workspace_id path param.",
        "JSON recompute report."),
    surface!("loom.search", SurfaceGroup::NotesLoom, "GET",
        "/workspaces/:workspace_id/loom/search",
        "Search Loom blocks (text + filters, observability-tiered).",
        "q + filter query params.",
        "JSON search hits."),
    // -- UserManual (api/user_manual.rs — this group's own surface) --------
    surface!("usermanual.pages.list", SurfaceGroup::UserManual, "GET",
        "/usermanual/pages",
        "List UserManual pages (filter by kind/audience). THE no-context entry point.",
        "Optional kind/audience/limit query params; identity headers optional (anonymous bootstrap allowed).",
        "JSON page rows (slug, title, page_kind, content_hash, manual_version)."),
    surface!("usermanual.pages.get", SurfaceGroup::UserManual, "GET",
        "/usermanual/pages/:slug",
        "Read one UserManual page with ordered sections and anchors.",
        "slug path param.",
        "JSON {page, sections, anchors} + bootstrap receipt id."),
    surface!("usermanual.pages.links", SurfaceGroup::UserManual, "GET",
        "/usermanual/pages/:slug/links",
        "Outbound page links and inbound backlinks for a manual page.",
        "slug path param.",
        "JSON {outbound, inbound} slug lists."),
    surface!("usermanual.pages.projection", SurfaceGroup::UserManual, "GET",
        "/usermanual/pages/:slug/projection",
        "Render a page as HTML (stable data-hs-manual-* selectors) or markdown — projection only.",
        "slug path param + format=html|markdown query.",
        "JSON {format, rendered}."),
    surface!("usermanual.search", SurfaceGroup::UserManual, "GET",
        "/usermanual/search",
        "Bounded search across pages, sections, and tool entries.",
        "q + limit query params.",
        "JSON search hits (page/section/tool)."),
    surface!("usermanual.tools.list", SurfaceGroup::UserManual, "GET",
        "/usermanual/tools",
        "The machine-readable tool/command catalog (legacy + WP-009 surfaces).",
        "Optional status/origin/limit query params.",
        "JSON tool entry rows."),
    surface!("usermanual.tools.get", SurfaceGroup::UserManual, "GET",
        "/usermanual/tools/:tool_id",
        "One tool entry with schema fields, errors, and recovery steps.",
        "tool_id path param.",
        "JSON tool entry."),
    surface!("usermanual.features.list", SurfaceGroup::UserManual, "GET",
        "/usermanual/features",
        "Feature groups over tool entries.",
        "Optional limit query param.",
        "JSON feature entry rows."),
    surface!("usermanual.quickstart.get", SurfaceGroup::UserManual, "GET",
        "/usermanual/quickstarts/:area",
        "No-context quickstart bundle for an area (index|editor|loom|retrieval|validation|state-recovery).",
        "area path param.",
        "JSON bundled quickstart pages + startup commands."),
    surface!("usermanual.freshness", SurfaceGroup::UserManual, "GET",
        "/usermanual/freshness",
        "Freshness verdicts: seeded pages vs compiled-in corpus vs surface registry.",
        "None.",
        "JSON verdict rows (current/stale_content/uncovered_surface/dangling_anchor/missing_page)."),
    surface!("usermanual.access_points.list", SurfaceGroup::UserManual, "GET",
        "/usermanual/access-points",
        "Where the manual is reachable in-app (editor, Notes, retrieval debug, diagnostics, command palette).",
        "None.",
        "JSON access point rows (host_surface, target page slug, wiring contract)."),
    surface!("usermanual.legacy.model_manual", SurfaceGroup::UserManual, "GET",
        "/usermanual/legacy/model-manual",
        "Legacy ModelManual bridge: returns the canonical mapped payload AND emits a compatibility receipt (spec 10.15.8).",
        "None (legacy callers).",
        "JSON {deprecated: true, canonical, payload} + compatibility receipt id."),
    surface!("usermanual.legacy.aliases", SurfaceGroup::UserManual, "GET",
        "/usermanual/legacy/aliases",
        "The deterministic legacy-name -> canonical mapping rows.",
        "None.",
        "JSON alias rows with deprecation notes."),
    surface!("usermanual.migration_plan", SurfaceGroup::UserManual, "GET",
        "/usermanual/migration-plan",
        "The MT-193 naming migration plan (typed rows, phases, blockers).",
        "None.",
        "JSON plan rows + aliases."),
    surface!("usermanual.spec_enrichment_seed", SurfaceGroup::UserManual, "GET",
        "/usermanual/spec-enrichment-seed",
        "Master Spec UserManual wording/appendix seed rows for future enrichment (MT-207).",
        "None.",
        "JSON seed rows (target module, anchor, proposed wording)."),
    surface!("usermanual.resync", SurfaceGroup::UserManual, "POST",
        "/usermanual/resync",
        "Re-seed the manual corpus from the compiled-in seed (idempotent; receipts per changed page). Write-gated.",
        "x-hsk-actor-kind operator|system|local_model required; cloud_model and unauthenticated are DENIED (403).",
        "JSON resync report (pages changed, version row) + receipt ids."),
];

// ---------------------------------------------------------------------------
// MT-200: in-app access points.
// ---------------------------------------------------------------------------

/// Where the UserManual is exposed inside the product (MT-200). The backend
/// serves the registry + target routes; the concurrent frontend lane mounts
/// the panels/commands and calls `ui_wiring_route`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AccessPoint {
    pub access_point_id: &'static str,
    /// `editor` | `notes_loom` | `retrieval_debug` | `diagnostics` |
    /// `command_palette`.
    pub host_surface: &'static str,
    /// `panel` | `command` | `deep_link`.
    pub entry_kind: &'static str,
    /// The manual page the access point opens.
    pub target_page_slug: &'static str,
    /// The backend route the UI calls to render the entry.
    pub ui_wiring_route: &'static str,
    /// Stable element id the frontend must use (visual-debug law).
    pub stable_element_id: &'static str,
    pub note: &'static str,
}

pub fn user_manual_access_points() -> &'static [AccessPoint] {
    ACCESS_POINTS
}

const ACCESS_POINTS: &[AccessPoint] = &[
    AccessPoint {
        access_point_id: "ap.editor.help",
        host_surface: "editor",
        entry_kind: "panel",
        target_page_slug: "rich-documents-surface",
        ui_wiring_route: "/usermanual/pages/rich-documents-surface",
        stable_element_id: "hs-usermanual-editor-help",
        note: "Rich editor help panel: save/load, history, projections, embeds, permission model.",
    },
    AccessPoint {
        access_point_id: "ap.editor.failure_help",
        host_surface: "editor",
        entry_kind: "deep_link",
        target_page_slug: "failure-modes-and-recovery",
        ui_wiring_route: "/usermanual/pages/failure-modes-and-recovery",
        stable_element_id: "hs-usermanual-editor-failure-help",
        note: "Shown next to typed editor errors (409 conflict, broken embeds).",
    },
    AccessPoint {
        access_point_id: "ap.notes.help",
        host_surface: "notes_loom",
        entry_kind: "panel",
        target_page_slug: "notes-loom-surface",
        ui_wiring_route: "/usermanual/pages/notes-loom-surface",
        stable_element_id: "hs-usermanual-notes-help",
        note: "Notes surface help: blocks, backlinks, graph, tags, folders, wiki projections.",
    },
    AccessPoint {
        access_point_id: "ap.retrieval_debug.help",
        host_surface: "retrieval_debug",
        entry_kind: "panel",
        target_page_slug: "retrieval-and-context-bundles-surface",
        ui_wiring_route: "/usermanual/pages/retrieval-and-context-bundles-surface",
        stable_element_id: "hs-usermanual-retrieval-help",
        note: "Retrieval trace viewer help: bundles, staleness, repair.",
    },
    AccessPoint {
        access_point_id: "ap.diagnostics.manual_tab",
        host_surface: "diagnostics",
        entry_kind: "panel",
        target_page_slug: "manual-toc",
        ui_wiring_route: "/usermanual/pages/manual-toc",
        stable_element_id: "hs-usermanual-diagnostics-tab",
        note: "Diagnostics manual tab: full TOC (spec 10.15.8 requires the diagnostics manual tab to resolve against UserManual authority).",
    },
    AccessPoint {
        access_point_id: "ap.diagnostics.recovery",
        host_surface: "diagnostics",
        entry_kind: "deep_link",
        target_page_slug: "state-recovery-guide",
        ui_wiring_route: "/usermanual/pages/state-recovery-guide",
        stable_element_id: "hs-usermanual-diagnostics-recovery",
        note: "State recovery guide linked from error reports.",
    },
    AccessPoint {
        access_point_id: "ap.command_palette.open_manual",
        host_surface: "command_palette",
        entry_kind: "command",
        target_page_slug: "manual-toc",
        ui_wiring_route: "/usermanual/pages/manual-toc",
        stable_element_id: "hs-usermanual-palette-open",
        note: "Palette command 'UserManual: Open' lists pages via /usermanual/pages.",
    },
    AccessPoint {
        access_point_id: "ap.command_palette.search_manual",
        host_surface: "command_palette",
        entry_kind: "command",
        target_page_slug: "manual-toc",
        ui_wiring_route: "/usermanual/search",
        stable_element_id: "hs-usermanual-palette-search",
        note: "Palette command 'UserManual: Search' calls /usermanual/search?q=.",
    },
];

/// Substitute `:params` in a registered route with plausible probe values so
/// the doc-vs-runtime test can drive the real router. Unknown ids are FINE —
/// a handler 404/400 still proves the route+method are mounted; only a
/// router-level 404 (empty body) or 405 marks the documentation stale.
pub fn probe_path(route: &str) -> String {
    route
        .split('/')
        .map(|segment| match segment {
            s if s.starts_with(':') => match s {
                ":workspace_id" => "WS-PROBE",
                ":view_type" => "documents",
                ":format" => "markdown",
                ":area" => "index",
                ":slug" => "manual-toc",
                ":doc_version" => "1",
                _ => "PROBE-ID",
            },
            other => other,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn surface_ids_and_route_method_pairs_are_unique() {
        let mut ids = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        for s in wp009_surface_registry() {
            assert!(ids.insert(s.surface_id), "dup surface id {}", s.surface_id);
            assert!(
                pairs.insert((s.method, s.route)),
                "dup route {} {}",
                s.method,
                s.route
            );
            assert!(matches!(s.method, "GET" | "POST" | "PUT" | "DELETE" | "PATCH"));
            assert!(s.route.starts_with('/'), "route must be absolute");
            assert!(!s.summary.trim().is_empty());
            assert!(!s.expected_input.trim().is_empty());
            assert!(!s.expected_output.trim().is_empty());
        }
    }

    #[test]
    fn access_points_target_known_slug_shapes_and_routes() {
        let mut ids = BTreeSet::new();
        for ap in user_manual_access_points() {
            assert!(ids.insert(ap.access_point_id), "dup {}", ap.access_point_id);
            assert!(matches!(
                ap.host_surface,
                "editor" | "notes_loom" | "retrieval_debug" | "diagnostics" | "command_palette"
            ));
            assert!(matches!(ap.entry_kind, "panel" | "command" | "deep_link"));
            assert!(ap.ui_wiring_route.starts_with("/usermanual/"));
            assert!(ap.stable_element_id.starts_with("hs-usermanual-"));
        }
        // Every host surface named by the MT-200 contract is present.
        let hosts: BTreeSet<_> = user_manual_access_points()
            .iter()
            .map(|ap| ap.host_surface)
            .collect();
        for host in [
            "editor",
            "notes_loom",
            "retrieval_debug",
            "diagnostics",
            "command_palette",
        ] {
            assert!(hosts.contains(host), "missing access point host {host}");
        }
    }

    #[test]
    fn probe_path_substitutes_every_param() {
        for s in wp009_surface_registry() {
            let probed = probe_path(s.route);
            assert!(
                !probed.contains(':'),
                "unsubstituted param in {} -> {}",
                s.route,
                probed
            );
        }
    }
}
