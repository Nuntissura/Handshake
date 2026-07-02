//! Inline `#tag` authoring for the native rich-text editor (WP-KERNEL-012 MT-058, cluster E2).
//!
//! Brings Obsidian/Notion-style inline `#tag` authoring into the BODY of the editor (today a tag can
//! only be set through the MT-017 properties panel). Typing `#` at a word boundary detects, autocompletes,
//! and commits a tag inline; the committed tag renders as a clickable chip that emits a navigation event
//! to the MT-023 tag hub; and on document commit the inline tags converge with property tags into ONE
//! deduped loom edge per distinct normalized identity (one tag, one hub).
//!
//! ## Node shape (REUSE-NOT-FORK — the KERNEL_BUILDER gate)
//!
//! A committed inline tag is the EXISTING `hsLink` inline atom
//! ([`crate::rich_editor::document_model::node::HsLinkNode`] / `Child::HsLink`) with `ref_kind = "tag"`
//! — NOT a new invented mark the backend would strip on save. So it (a) round-trips the backend
//! `content_json`, (b) renders through the EXISTING MT-012 inline-mark / MT-015 chip pipeline
//! ([`crate::rich_editor::renderer::rich_editor_widget::paint_one_wikilink_chip`], host-special-cased on
//! `ref_kind="tag"`), and (c) converges with property tags on the SAME normalized identity.
//!
//! ## Submodules
//!
//! - [`parser`] — PURE detection + normalization (NO egui/AccessKit — AC-007): the [`parser::Tag`] /
//!   [`parser::TagToken`] types, [`parser::parse_inline_tags`] (word-boundary `#tag` extraction),
//!   [`parser::normalize_tag`] (the SINGLE shared canonical-identity function — see the typed-blocker
//!   note in that module), [`parser::open_tag_query`] (the LIVE `#` input-trigger detector), and
//!   [`parser::tag_to_hs_link`] (Tag -> the `hsLink` atom).
//! - [`inline_chip`] — the egui-facing identity/event/menu/convergence surface:
//!   [`inline_chip::TagActivated`] (the navigation event the chip emits — NEVER opening the hub
//!   directly), [`inline_chip::inline_tag_author_id`] (the contract `inline-tag-{name}` AccessKit id,
//!   `Role::Link`), [`inline_chip::TagAutocompleteState`] + [`inline_chip::tag_menu_items`] (the `#`
//!   menu state + item source, free-typed create — AC-006), and [`inline_chip::build_tag_edge_payload`]
//!   (the deduped convergence edge builder — AC-005).
//!
//! Everything REUSES the WP-011 shell + the MT-011/012/015 editor primitives: the `hsLink` atom +
//! `confirm_wikilink` insert (transactional `Step::InsertInlineChild` + undo receipt per the MT-020
//! rewire — no tag-specific model step), the theme palette for chip colors (no hardcoded hex),
//! the `accessibility/*` live AccessKit emission, and the VERIFIED `LoomTagClient` tag-hub list + loom
//! edges API (PostgreSQL/EventLedger only — no SQLite, no backend rewrite).

pub mod inline_chip;
pub mod parser;

pub use inline_chip::{
    build_tag_edge_payload, inline_tag_author_id, is_tag_link, menu_item_to_hs_link, tag_from_link,
    tag_menu_items, TagActivated, TagAutocompleteState, TagEdge, TagEdgeIntent, TagEdgePayload,
    TagMenuItem, TAG_EDGE_DISPATCH_BLOCKER, TAG_REF_KIND,
};
pub use parser::{normalize_tag, open_tag_query, parse_inline_tags, tag_to_hs_link, Tag, TagToken};
