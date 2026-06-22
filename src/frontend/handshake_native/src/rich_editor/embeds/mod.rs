//! CKC media embed NodeViews for the rich-text editor (WP-KERNEL-012 MT-014).
//!
//! This cluster renders the four CKC media embed kinds — `images`, `video`, `album`,
//! `slideshow` — inline in the document. The native equivalent of the React
//! `app/src/components/HsLinkView.tsx` media-embed branch + `app/src/lib/editor/embed_assets.ts`.
//!
//! ## NODE-SHAPE RECONCILIATION (the MT-014 critical gate)
//!
//! Media embeds are the EXISTING MT-011 inline atom
//! [`crate::rich_editor::document_model::node::HsLinkNode`] (`Child::HsLink`), discriminated by
//! `ref_kind ∈ {images, video, album, slideshow}` — NOT an invented `NodeKind::Embed`. The React
//! reference renders embeds AND wikilinks through the SAME `hsLink` node by `refKind`
//! (`hs_link_node.ts` + `HsLinkView.tsx`), and the backend persists embeds as that node
//! (`RichDocEmbed.ref_kind` / `RichDocBacklink.link_kind`). Inventing a separate node would
//! repeat the MT-011 wikilink-mark mistake; this cluster therefore binds to the existing node.
//!
//! ## Submodules
//!
//! - [`asset_resolver`] — the fail-closed validation pipeline (port of `embed_assets.ts`), the
//!   typed [`asset_resolver::EmbedError`] vocabulary, the async [`asset_resolver::resolve_one`] /
//!   [`asset_resolver::resolve_sequence`] (bounded concurrency, MC-002), and the per-asset
//!   resolution cache (AC-9). Fully unit-testable with a counted mock fetcher (NO backend).
//! - [`image_view`] — single-image decode (off-thread, MC-001), texture cache, aspect-fit.
//! - [`slideshow_view`] — one-at-a-time prev/next with wrap (AC-5), pure nav state.
//! - [`album_view`] — 3-per-row grid + click-to-enlarge modal state (AC-6).
//! - [`video_view`] — poster/placeholder + play button (HBR-QUIET: no `open` crate, no focus
//!   theft; in-process decode deferred).
//! - [`embed_block_renderer`] — the interactive `egui::Ui` dispatch ([`embed_block_renderer::render_embed`])
//!   from an embed `hsLink` to the correct view, owning the per-editor [`embed_block_renderer::EmbedRuntime`]
//!   (caches + view states + async transport), the spinner/Ok/typed-error-chip states, and the
//!   AccessKit author_ids the ACs name.
//!
//! All of this REUSES the WP-011 shell: `theme/*` (every color is an `HsPalette` token, no
//! hardcoded hex), `accessibility/*` (live AccessKit emission), and `backend_client` (the reqwest
//! REST stack — no new HTTP crate). No shell infrastructure is forked.

pub mod album_view;
pub mod asset_resolver;
pub mod embed_block_renderer;
pub mod image_view;
pub mod slideshow_view;
pub mod video_view;
