//! Wikilinks, transclusion, and backlinks for the native rich-text editor (WP-KERNEL-012 MT-015).
//!
//! This cluster ports the React wikilink/transclusion/backlinks surfaces (`wikilink.ts`,
//! `LoomTransclusionView.tsx`, `RichDocumentView.tsx` backlinks) to native Rust, binding to the REAL
//! backend loom + knowledge-document APIs.
//!
//! ## NODE-SHAPE RECONCILIATION (the MT-015 critical gates, anchored to the real backend)
//!
//! - A wikilink is the EXISTING MT-011 inline atom
//!   [`crate::rich_editor::document_model::node::HsLinkNode`] (`Child::HsLink`), NOT a
//!   `Mark::Wikilink`. Autocomplete confirm inserts it as ONE atomic transaction via the MT-020
//!   `Step::InsertInlineChild` (receipt pushed on the `UndoManager` — Ctrl+Z removes it), never
//!   `AddMark`. The MT-014 media embeds already render from this same `hsLink` node by `ref_kind`
//!   — one unified dispatch.
//! - A transclusion is the `loomTransclusion` inline atom
//!   ([`crate::rich_editor::document_model::node::TransclusionNode`] / `Child::Transclusion`),
//!   carrying `ref_value` (the backend block id), matching the REAL `LoomTransclusionView.tsx`
//!   `{refValue}` shape — NOT the contract's invented `{block_id}` attr. It was ADDED to MT-011's
//!   model (node.rs + doc_json.rs) anchored to the captured backend shape so it round-trips.
//! - The prefix vocabulary is the REAL `WP009_WIKILINK_KIND_BY_PREFIX` table (note/file/folder/
//!   project/spec/wp/symbol/album/video/HS_images/HS_slideshow). The contract's `block`/`doc`/`tag`
//!   examples are NOT in the table and classify as `WikilinkKind::Unknown` (preserved, not dropped).
//!
//! ## Submodules
//!
//! - [`parser`] — the `[[prefix:value|label]]` regex + `classify_wikilink` (port of `wikilink.ts`),
//!   the `WikilinkKind`/`ParsedWikilink` types, the `[[` open-trigger detector. Fully unit-testable.
//! - [`client`] — the async backend transport trait (search-v2 / transclusion / backlinks), the
//!   typed [`client::WikilinkError`] vocabulary, the verified backend response types, and the
//!   production reqwest impl. Unit-testable with a counted mock (NO backend).
//! - [`autocomplete`] — the popup state, the 150ms debounce (MC-002), the generation-counter
//!   cancellation (MC-004), and the search runtime. Unit-testable.
//! - [`runtime`] — the per-editor [`runtime::WikilinkRuntime`] (transclusion cache + backlinks state
//!   + autocomplete runtime + delivery cells), with 404->Remove (MC-003) + backlinks cancellation (MC-004), unit-testable.
//! - [`inline_view`] — the inline wikilink CHIP (color/label/author-id + scroll-adjusted rect,
//!   MC-001) and the [`inline_view::EditorEvent`] enqueued for the WP-011 shell to route.
//! - [`transclusion_view`] — the interactive read-through preview (spinner/resolved/unresolved/error
//!   + Open block + Remove embed on 404).
//! - [`backlinks_panel`] — the collapsible backlinks side panel (entry list + refresh + empty state).
//!
//! All of this REUSES the WP-011 shell: `theme/*` (every color a theme token, no hardcoded hex),
//! `accessibility/*` (live AccessKit emission), and the reqwest REST stack (no new HTTP crate).

pub mod autocomplete;
pub mod backlinks_panel;
pub mod client;
pub mod confirm;
pub mod inline_view;
/// WP-KERNEL-012 MT-062: the Outgoing Links pane (the third leg of the Obsidian links triad alongside
/// MT-015 backlinks + MT-024 unlinked mentions). Lists every wikilink/transclusion emanating FROM the
/// active document, split into Resolved/Unresolved buckets; clicks route through the MT-030 nav seam.
/// See [`outgoing_links_panel`].
pub mod outgoing_links_panel;
pub mod parser;
/// WP-KERNEL-012 MT-057: the wikilink RESOLUTION engine (exact ref/title/alias resolution + the
/// create-from-unresolved command-bus intent) layered on the MT-015 wikilink engine. See
/// [`resolver`].
pub mod resolver;
pub mod runtime;
pub mod transclusion_view;
