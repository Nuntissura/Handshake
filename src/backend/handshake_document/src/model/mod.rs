//! WP-KERNEL-009 RichDocumentCore (MT-145..MT-160): the canonical structured
//! rich-document model backing the editor, Loom, UserManual, and projections.
//!
//! Spec anchors: 2.3.13.11 (RichDocument / EditorCodeNode authority via
//! write-box/promotion; projections are NEVER authority) and 7.1.1.8 (the
//! editor authority is the versioned RichDocument schema; Monaco code nodes
//! round-trip; deterministic save/load across CRDT reconnect/restart/
//! projection-rebuild/schema-migration).
//!
//! Storage authority for RichDocuments lives in `storage::knowledge`
//! (`knowledge_rich_documents` + `knowledge_rich_document_versions` +
//! `knowledge_editor_code_nodes`, migration 0140 / MT-059) and the WP-009
//! identity/embeds/backlinks tables (migrations 0280-0282). This module owns
//! the in-memory MODEL and the pure transforms over the document JSON
//! authority:
//!
//!   * [`block_tree`] - the typed block-tree model (MT-146): paragraph,
//!     heading, list, quote, code, table, image, video, album, slideshow, and
//!     the typed link blocks (file/folder/project/spec/wp/symbol). Parses and
//!     re-serializes the ProseMirror/Tiptap document JSON authority, preserving
//!     Raw/Derived/Display separation (MT-147, CX-100) and stable block ids
//!     (MT-148).
//!   * [`projection`] - deterministic projection RENDERERS (MT-150): markdown,
//!     HTML, plain text, wiki/Loom, and context-bundle views derived FROM the
//!     canonical block tree. Projections are regenerable and never authority.
//!   * [`import`] - projection IMPORT (MT-151): markdown / plain-text / HTML
//!     snippets parsed into a block tree with typed unsupported-feature
//!     warnings and repairable nodes.
//!   * [`embed`] - the embed reference model (MT-152) and broken-embed repair
//!     state (MT-153): embeds are stored as artifact/media/source ids or typed
//!     URLs, never random absolute paths; a missing target becomes a repairable
//!     typed broken node.
//!   * [`backlink`] - document backlink + search-index bridge inputs
//!     (MT-154/155): the link references emitted by a document, carrying the
//!     stable relationship id used to persist a backlink edge.
//!   * [`permission`] - the server-enforced document permission boundary
//!     (MT-158).

pub mod backlink;
pub mod block_tree;
pub mod embed;
pub mod import;
pub mod permission;
pub mod projection;

pub use backlink::{DocumentLinkReference, DocumentLinkReferences};
pub use block_tree::{
    Block, BlockKind, BlockTree, BlockTreeError, RawDerivedDisplay, DOCUMENT_SCHEMA_VERSION,
};
pub use embed::{
    BrokenEmbedRepair, EmbedRef, EmbedRefKind, EmbedRepairAction, EmbedTarget, EmbedTargetError,
};
pub use import::{import_snippet, ImportFormat, ImportOutcome, ImportWarning};
pub use permission::{DocumentAction, DocumentActorKind, DocumentPermission, PermissionDecision};
pub use projection::{render_projection, ProjectionFormat, RenderedProjection};
