//! Daily-notes / journal surface (WP-KERNEL-012 MT-019).
//!
//! The native Rust port of `app/src/components/LoomDailyJournalPanel.tsx` (MT-257): a dedicated egui
//! panel that opens or creates today's daily-note Loom block via the backend `openDailyJournal` API,
//! renders its linked RichDocument with the MT-012 [`crate::rich_editor::renderer::rich_editor_widget`]
//! renderer, and provides date navigation (prev/next/today + a calendar picker).
//!
//! It is a SIBLING top-level surface mounted through the WP-011 `pane_registry` / `split_layout` host
//! (the `LoomDailyJournal` pane type — tab label "Journal"), NOT a child of `RichEditorWidget`.
//!
//! ## Modules
//!
//! - [`journal_store`] — the async backend transport (the verified `PUT /loom/journals/{date}` +
//!   `GET`/`POST /knowledge/documents`), the load state machine (Idle/Loading/Ready/Error with the
//!   MC-002 generation counter), and the mockable MT-020 save seam.
//! - [`date_nav`] — the pure date arithmetic (chrono `checked_add_days`, MC-005), the fixed 6×7
//!   calendar grid (MC-004), and the egui date-nav widget with the AC-11 AccessKit ids.
//! - [`journal_panel`] — the top-level egui panel: header (date nav) + subtitle + content (spinner /
//!   error chip / MT-012 editor / "Start writing") + footer (save status + word/char count), the
//!   frame-based auto-save debounce timer, the AccessKit root, and the `LoomDailyJournal` pane factory.
//!
//! ## Verified correction
//!
//! The contract's `document_ref` LoomBlock field does NOT exist — the real linked-document field is
//! `document_id` (verified against `app/src/lib/api.ts`). [`journal_store`] binds the real field.

pub mod date_nav;
pub mod journal_panel;
pub mod journal_store;
