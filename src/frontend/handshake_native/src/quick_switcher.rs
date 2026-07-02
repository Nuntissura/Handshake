//! Quick Switcher overlay for the native Handshake shell (WP-KERNEL-011 MT-017).
//!
//! ## What this provides (no-context model navigation — HBR-VIS / HBR-SWARM)
//!
//! A modal, centred, always-on-top floating panel (the Ctrl+P quick switcher): a search text input, a
//! live-search list over the **Loom graph** backed by the REAL PostgreSQL backend, keyboard navigation
//! (ArrowUp/ArrowDown/Enter/Escape), and jump-on-Enter or click. It is a direct port of the React
//! `app/src/components/QuickSwitcher.tsx`: it searches documents, blocks, symbols, work packets,
//! microtasks, user-manual pages, wiki pages, files and tag hubs across the project graph and jumps the
//! active pane to the selected target.
//!
//! ### Source of the rows (NOT faked, NOT local panes)
//!
//! Every row is a REAL hit from `GET /workspaces/{id}/loom/graph-search?q=...&source_kinds=...&limit=25`
//! against handshake_core + PostgreSQL — the same endpoint the React app uses
//! ([`LoomGraphSearchTransport`] is the synchronous seam; [`LoomGraphSearchClient`] is the production
//! reqwest implementation bridged onto the app's tokio runtime, the MT-009 `WorkbenchLayoutClient`
//! pattern). When the query is empty the list is empty and a hint is shown; when no workspace is
//! selected the backend is never called. A swarm agent opens the switcher via the app-state flag
//! ([`crate::app::HandshakeApp::open_quick_switcher`]), injects a query string into the SearchBox, reads
//! the ordered rows from the AccessKit ListBox/Option tree, and presses Enter to jump — no screen
//! scraping, no live-server dependency for the unit/kittest proofs (the seam is stubbable).
//!
//! ### Recents ordering (durable PostgreSQL store)
//!
//! On open the switcher loads `GET /workspaces/{id}/loom/quick-switcher/recents?limit=20` and stores the
//! returned `hit_key` list (`"{source_kind}:{ref_id}"`, most-recent first). Any visible hit whose
//! [`hit_key`] appears in that list is sorted to the front, in recents order (the native mirror of the
//! React `orderedResults` useMemo). Selecting a hit records the visit via
//! `POST /workspaces/{id}/loom/quick-switcher/recents` and prepends the returned key to the local list.
//! (The earlier MT-017 implementation claimed these endpoints did not exist and shipped a local
//! pane/tab switcher instead — that claim was false; the endpoints are live in
//! `src/backend/handshake_core/src/api/loom.rs`.)
//!
//! ## Ownership split (mirrors the MT-016 command palette + MT-015 menu bar)
//!
//! The widget owns its small transient UI state (query, selected row, debounce timer) plus a
//! background-search results cell. It NEVER mutates app navigation state: [`show`] returns a
//! [`SwitcherOutcome`] and the shell ([`crate::app`]) matches on it and performs the jump (open the hit
//! on the active pane via the appropriate [`crate::pane_registry::PaneType`]). The shell owns the
//! `quick_switcher_open` flag; the switcher only REQUESTS close via the outcome.
//!
//! ## Async bridge (HBR-QUIET — never block the egui frame)
//!
//! Debounced search + recents I/O run on the app's tokio runtime via [`tokio::runtime::Handle::spawn`];
//! the spawned task writes its result into an `Arc<Mutex<Option<...>>>` cell that [`show`] drains with
//! `try_lock` (red-team MC1: never hold the lock across `ui.*`). A monotonic `search_sequence` guards
//! against out-of-order arrivals (red-team MC2). The egui frame thread never performs HTTP I/O.
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! Three FIXED container nodes in the 14..=16 band (distinct from the command palette's 11..=13):
//! - the dialog root ([`SWITCHER_DIALOG_NODE_ID`] = 14, Role::Dialog, modal),
//! - the search box ([`SWITCHER_SEARCH_NODE_ID`] = 15, Role::TextInput),
//! - the list container ([`SWITCHER_LIST_NODE_ID`] = 16, Role::ListBox).
//!
//! Each result ROW is DYNAMIC (count varies with the query) and lives in egui's hashed id space,
//! addressed by a stable author_id STRING (`quick-switcher.option.{source_kind}.{stable(ref_id)}`,
//! Role::ListBoxOption), so it is discoverable/clickable out-of-process and never trips the MT-025
//! interactive-naming gate. The three fixed container ids ARE enumerated in `DECLARED_IDENTITIES`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::accesskit;
use serde::Deserialize;
use serde_json::Value;

use crate::accessibility::emit_interactive_node;

/// Fixed AccessKit/egui `NodeId` of the switcher DIALOG root (Role::Dialog, modal). Band slot 14.
pub const SWITCHER_DIALOG_NODE_ID: u64 = 14;
/// Fixed AccessKit/egui `NodeId` of the switcher SEARCH box (Role::TextInput). Band slot 15.
pub const SWITCHER_SEARCH_NODE_ID: u64 = 15;
/// Fixed AccessKit/egui `NodeId` of the switcher LIST container (Role::ListBox). Band slot 16.
pub const SWITCHER_LIST_NODE_ID: u64 = 16;

/// Stable out-of-process author_id for the switcher dialog root.
pub const SWITCHER_DIALOG_AUTHOR_ID: &str = "quick-switcher.dialog";
/// Stable out-of-process author_id for the switcher search box.
pub const SWITCHER_SEARCH_AUTHOR_ID: &str = "quick-switcher.search";
/// Stable out-of-process author_id for the switcher list container.
pub const SWITCHER_LIST_AUTHOR_ID: &str = "quick-switcher.list";
/// Stable out-of-process author_id for the switcher header Close button. Like the result rows and the
/// command-palette / settings Close buttons, it lives in egui's hashed id space (no fixed
/// `DeclaredIdentity` slot), so it is addressed by this author_id via `emit_interactive_node`. Without
/// it the Close button is an interactive control with no stable address — the gap the MT-029 overlay
/// accessibility-invariant proof surfaces.
pub const SWITCHER_CLOSE_AUTHOR_ID: &str = "quick-switcher.close";

/// The author_id prefix for a result ROW (a `ListBoxOption`). Each row's full author_id is
/// `{ROW_AUTHOR_ID_PREFIX}{source_kind}.{stable(ref_id)}`, in egui's hashed id space (dynamic count).
pub const ROW_AUTHOR_ID_PREFIX: &str = "quick-switcher.option.";

/// The nine Loom-graph source kinds the quick switcher searches, in the React
/// `QUICK_SWITCHER_SOURCE_KINDS` order. Passed verbatim as the `source_kinds` query param.
pub const QUICK_SWITCHER_SOURCE_KINDS: [&str; 9] = [
    "loom_block",
    "file",
    "tag_hub",
    "document",
    "symbol",
    "work_packet",
    "micro_task",
    "user_manual_page",
    "wiki_page",
];

/// The debounce window between the last keystroke and firing the graph-search request. Matches the
/// React `window.setTimeout(..., 150)` debounce in `QuickSwitcher.tsx`.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(150);

/// The `limit` query param for graph-search (React `limit: 25`).
pub const SEARCH_LIMIT: u32 = 25;
/// The `limit` query param for the recents load (React `listQuickSwitcherRecents(workspaceId, 20)`).
pub const RECENTS_LIMIT: u32 = 20;

/// One Loom-graph search hit (the Rust equivalent of the React `LoomGraphSearchHit`, deserialized from
/// the backend `LoomGraphSearchResult`). `block` and `metadata` stay as raw [`Value`] so the
/// open-target mapping can read the exact nested fields the React `loom_search_open_target.ts` reads
/// (e.g. `metadata.rich_document_id`, `block.document_id`) without a lossy typed projection.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoomGraphSearchHit {
    /// `loom_block | knowledge_entity | user_manual_page | wiki_page` (the result envelope kind).
    pub result_kind: String,
    /// `loom_block | file | tag_hub | document | symbol | work_packet | micro_task |
    /// user_manual_page | wiki_page` (the underlying source kind, drives the chip + open target).
    pub source_kind: String,
    /// The hit's stable reference id (block id / document id / WP id / slug / projection id ...).
    pub ref_id: String,
    /// Display title.
    pub title: String,
    /// Optional excerpt/snippet (the backend defaults this to `""`).
    #[serde(default)]
    pub excerpt: String,
    /// The referenced LoomBlock, when the hit is a block (open-in-place). Raw JSON; only
    /// `block.document_id` is read by the open-target mapping.
    #[serde(default)]
    pub block: Value,
    /// Fused relevance score (unused for ordering here — recents-first wins — but carried for parity).
    #[serde(default)]
    pub score: f64,
    /// Provenance/anchor metadata bag. Raw JSON; the open-target mapping reads typed string fields out
    /// of it (`rich_document_id`, `document_id`, `page_slug`, `work_packet_id`, `entity_key`, ...).
    #[serde(default)]
    pub metadata: Value,
}

/// The stable recents/match key for a hit: `"{source_kind}:{ref_id}"`. Kept a free fn (not a method)
/// per the MT contract so it has no lifetime entanglement with the struct. Mirrors the React `hitKey`.
pub fn hit_key(hit: &LoomGraphSearchHit) -> String {
    format!("{}:{}", hit.source_kind, hit.ref_id)
}

/// The human label for a source kind chip (port of the React `SOURCE_LABELS` map).
pub fn source_label(kind: &str) -> &'static str {
    match kind {
        "loom_block" => "Loom Block",
        "file" => "File",
        "tag_hub" => "Tag Hub",
        "document" => "Document",
        "symbol" => "Symbol",
        "work_packet" => "Work Packet",
        "micro_task" => "Microtask",
        "user_manual_page" => "UserManual Page",
        "wiki_page" => "Wiki Page",
        _ => "Unknown",
    }
}

/// A typed navigation target derived from a [`LoomGraphSearchHit`] (port of the React
/// `LoomSearchOpenTarget`). [`QuickSwitcherTarget::Unsupported`] disables the row (no app surface yet).
///
/// The shell ([`crate::app`]) maps each variant to the appropriate [`crate::pane_registry::PaneType`]
/// tab opened on the active pane (see `open_target_label` for the operator-facing hint text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickSwitcherTarget {
    /// Open a UserManual page by slug (`PaneType::UserManual`, content_id = slug).
    UserManual { slug: String },
    /// Open a Loom wiki page by projection id (`PaneType::LoomWikiPage`, content_id = projection_id).
    WikiPage { projection_id: String },
    /// Open a rich document by id (`PaneType::AtelierEditor`, content_id = document_id).
    Document { document_id: String },
    /// Open a Loom block / file / tag hub by id (`PaneType::LoomBlock`, content_id = block_id).
    LoomBlock { block_id: String },
    /// Open a code symbol by entity id (`PaneType::CodeSymbol`, content_id = symbol_entity_id).
    CodeSymbol { symbol_entity_id: String },
    /// Open a work packet in the Kernel DCC (`PaneType::KernelDcc`, content_id = "WP:{wp_id}").
    WorkPacket { wp_id: String },
    /// Open a microtask in the Kernel DCC (`PaneType::KernelDcc`, content_id = "MT:{wp_id?}:{mt_id}").
    MicroTask {
        mt_id: String,
        wp_id: Option<String>,
    },
    /// No supported app target yet — the row renders disabled and cannot be activated.
    Unsupported,
}

impl QuickSwitcherTarget {
    /// True when the target maps to a real app surface (the row is enabled/clickable).
    pub fn enabled(&self) -> bool {
        !matches!(self, QuickSwitcherTarget::Unsupported)
    }

    /// The operator-facing "what opening this does" hint (port of the React target `label`s).
    pub fn label(&self) -> &'static str {
        match self {
            QuickSwitcherTarget::WikiPage { .. } => "Open wiki page",
            QuickSwitcherTarget::UserManual { .. } => "Open UserManual page",
            QuickSwitcherTarget::Document { .. } => "Open document",
            QuickSwitcherTarget::LoomBlock { .. } => "Open Loom block",
            QuickSwitcherTarget::CodeSymbol { .. } => "Open code symbol",
            QuickSwitcherTarget::WorkPacket { .. } => "Open Kernel DCC work packet",
            QuickSwitcherTarget::MicroTask { .. } => "Open Kernel DCC microtask",
            QuickSwitcherTarget::Unsupported => "No direct app target yet",
        }
    }
}

// ===========================================================================
// MT-030: the ShellNavigator bus + typed dispatch seam (the integration core).
//
// MT-017 shipped the overlay + search + `open_target_for_hit`; the navigation
// was dispatched INLINE in `app.rs`. MT-030 extracts that into a reusable
// `ShellNavigator` trait so the E5 interconnection MTs (MT-031+) and the E11
// GUI-continuation MTs (MT-069) drive editor/panel navigation through ONE
// stable interface instead of re-reaching into `app.rs` internals. The quick
// switcher itself is just the FIRST caller of this bus.
//
// The editor-pane targets (`open_document` -> the rich-text editor MT-012,
// `open_code_symbol` -> the code editor MT-001) are a deliberate TYPED SEAM:
// those editor panes are NOT yet mounted in the shell (E11/MT-069 owns that),
// so the navigator returns `NavDispatchOutcome::EditorPaneNotMounted` rather
// than silently faking the open. That honest typed status is what the overlay
// surfaces and what E11 replaces with a real editor mount (carry-forward to
// MT-069). The non-editor targets (loom block, wiki page, user-manual page,
// work packet, microtask) open real shell tabs NOW.
// ===========================================================================

/// The typed outcome of dispatching a [`QuickSwitcherTarget`] through a [`ShellNavigator`].
///
/// This is an explicit enum (not a bare `bool`) so the editor-pane seam stays HONEST: a
/// `Document` / `CodeSymbol` target routed before E11 mounts the editor panes returns
/// [`EditorPaneNotMounted`](NavDispatchOutcome::EditorPaneNotMounted) — a visible "not wired yet"
/// status, never a silent success or a faked placeholder open. The quick switcher reads this to set
/// its status line; E11/MT-069 makes those two arms return [`Opened`](NavDispatchOutcome::Opened).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavDispatchOutcome {
    /// The navigator opened the target on a real shell surface. Carries the human/agent-readable name
    /// of the surface it opened on (e.g. `"Loom Block"`, `"Kernel DCC"`) for status/diagnostics.
    Opened { surface: String },
    /// The target is an EDITOR-pane target (rich-text document or code symbol) whose pane is not yet
    /// mounted in the shell (E11/MT-069 owns mounting the MT-012 / MT-001 editor panes). The caller
    /// surfaces this as a typed status; it is NOT a silent no-op and NOT a faked open. Carries the
    /// editor kind so the status message is specific.
    EditorPaneNotMounted { editor: NavEditorKind },
    /// There was no pane to open the target on (an empty work surface). A safe no-op, surfaced as a
    /// status rather than a panic. (The seeded shell always has a pane, so this is the headless edge.)
    NoTargetPane,
    /// The target was [`QuickSwitcherTarget::Unsupported`] — the row was disabled and should never have
    /// been dispatched. Carried as a typed value so a mis-dispatch is observable in tests, not a panic.
    Unsupported,
}

impl NavDispatchOutcome {
    /// True when the dispatch landed on a real surface (the jump succeeded).
    pub fn opened(&self) -> bool {
        matches!(self, NavDispatchOutcome::Opened { .. })
    }

    /// The operator/agent-facing status string for this outcome (shown on the switcher status line and
    /// readable by a swarm agent that just dispatched a hit).
    pub fn status_text(&self) -> String {
        match self {
            NavDispatchOutcome::Opened { surface } => format!("Opened on {surface}"),
            NavDispatchOutcome::EditorPaneNotMounted { editor } => format!(
                "{} editor pane not mounted yet (E11/MT-069)",
                editor.label()
            ),
            NavDispatchOutcome::NoTargetPane => "No pane to open the target on".to_owned(),
            NavDispatchOutcome::Unsupported => "No direct app target yet".to_owned(),
        }
    }
}

/// Which not-yet-mounted editor pane an [`EditorPaneNotMounted`](NavDispatchOutcome::EditorPaneNotMounted)
/// outcome refers to (the MT-012 rich-text editor or the MT-001 code editor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavEditorKind {
    /// The rich-text / document editor (MT-012 surface), target of `open_document`.
    RichText,
    /// The code editor (MT-001 surface), target of `open_code_symbol`.
    Code,
}

impl NavEditorKind {
    /// The human label used in the typed "not mounted yet" status.
    pub fn label(self) -> &'static str {
        match self {
            NavEditorKind::RichText => "Rich-text",
            NavEditorKind::Code => "Code",
        }
    }
}

/// The shell navigation bus the quick switcher (and, later, the E5/E11 interconnection MTs) drive to
/// open a resolved target on the correct editor or panel. Extracting this from the inline `app.rs`
/// dispatch is the MT-030 "integration core": ONE extensible interface every navigation caller shares.
///
/// Each method opens one target kind and returns a typed [`NavDispatchOutcome`]. The two editor-pane
/// methods (`open_document`, `open_code_symbol`) MAY return
/// [`EditorPaneNotMounted`](NavDispatchOutcome::EditorPaneNotMounted) until E11/MT-069 mounts the real
/// MT-012 / MT-001 editor panes — that is the honest typed seam, not a faked open. Implementors must
/// NOT panic and must NOT silently no-op on a missing surface; they return the typed outcome.
///
/// The trait is object-safe (`&dyn ShellNavigator` / `&mut dyn ShellNavigator`) so the quick switcher
/// and tests can drive it without monomorphizing on the concrete shell.
pub trait ShellNavigator {
    /// Open a rich-text document by its `KRD-`-prefixed id (the MT-012 rich-text editor surface).
    fn open_document(&mut self, document_id: &str) -> NavDispatchOutcome;
    /// Open a Loom block / file / tag-hub by its block id (the Loom block viewer, mounted now).
    fn open_loom_block(&mut self, block_id: &str) -> NavDispatchOutcome;
    /// Open a code symbol by its entity id (the MT-001 code editor surface).
    fn open_code_symbol(&mut self, symbol_entity_id: &str) -> NavDispatchOutcome;
    /// Open a work packet in the Kernel DCC by its WP id (mounted now).
    fn open_work_packet(&mut self, wp_id: &str) -> NavDispatchOutcome;
    /// Open a microtask in the Kernel DCC by its MT id (+ optional WP id) (mounted now).
    fn open_micro_task(&mut self, mt_id: &str, wp_id: Option<&str>) -> NavDispatchOutcome;
    /// Open a built-in UserManual page by its slug (mounted now).
    fn open_user_manual_page(&mut self, slug: &str) -> NavDispatchOutcome;
    /// Open a Loom wiki page by its projection id (mounted now).
    fn open_wiki_page(&mut self, projection_id: &str) -> NavDispatchOutcome;
}

/// Dispatch a resolved [`QuickSwitcherTarget`] through a [`ShellNavigator`] (the extensible mapping the
/// quick switcher and the E5/E11 MTs share). This is the single place that knows which navigator method
/// realizes which target variant — adding a new target kind or a new navigator implementor does not
/// require touching the switcher render path. An [`Unsupported`](QuickSwitcherTarget::Unsupported)
/// target (a disabled row that should never have been activated) returns
/// [`NavDispatchOutcome::Unsupported`] rather than panicking.
pub fn dispatch_target(
    navigator: &mut dyn ShellNavigator,
    target: &QuickSwitcherTarget,
) -> NavDispatchOutcome {
    match target {
        QuickSwitcherTarget::Document { document_id } => navigator.open_document(document_id),
        QuickSwitcherTarget::LoomBlock { block_id } => navigator.open_loom_block(block_id),
        QuickSwitcherTarget::CodeSymbol { symbol_entity_id } => {
            navigator.open_code_symbol(symbol_entity_id)
        }
        QuickSwitcherTarget::WorkPacket { wp_id } => navigator.open_work_packet(wp_id),
        QuickSwitcherTarget::MicroTask { mt_id, wp_id } => {
            navigator.open_micro_task(mt_id, wp_id.as_deref())
        }
        QuickSwitcherTarget::UserManual { slug } => navigator.open_user_manual_page(slug),
        QuickSwitcherTarget::WikiPage { projection_id } => navigator.open_wiki_page(projection_id),
        QuickSwitcherTarget::Unsupported => NavDispatchOutcome::Unsupported,
    }
}

/// Resolve a [`LoomGraphSearchHit`] to its typed [`QuickSwitcherTarget`]. The MT-030 contract names this
/// `resolve_open_target`; it is the same exhaustive 9-source-kind mapping as [`open_target_for_hit`]
/// (the MT-017 name), kept as the canonical implementation. This alias exists so the contract-named
/// symbol resolves and the E5/E11 MTs have a stable, intention-revealing entry point.
pub fn resolve_open_target(hit: &LoomGraphSearchHit) -> QuickSwitcherTarget {
    open_target_for_hit(hit)
}

/// Read a non-empty trimmed STRING field out of a hit's `metadata` object (port of the React
/// `metadataString`). Returns `None` when metadata is not an object, the key is absent, the value is
/// not a string, or the string is blank.
fn metadata_string(hit: &LoomGraphSearchHit, key: &str) -> Option<String> {
    let v = hit.metadata.get(key)?.as_str()?;
    let t = v.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

/// Read a non-empty trimmed `block.document_id` STRING off a hit (port of the React `blockDocumentId`).
fn block_document_id(hit: &LoomGraphSearchHit) -> Option<String> {
    let v = hit.block.get("document_id")?.as_str()?;
    let t = v.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}

/// Resolve the rich-document id for a hit, requiring the `KRD-` prefix (port of the React
/// `documentIdFromLoomSearchHit`): metadata `rich_document_id`, then `document_id`, then the block's
/// `document_id`, then — for a `document` source kind — the hit's own `ref_id`.
fn document_id_from_hit(hit: &LoomGraphSearchHit) -> Option<String> {
    let candidate = metadata_string(hit, "rich_document_id")
        .or_else(|| metadata_string(hit, "document_id"))
        .or_else(|| block_document_id(hit))
        .or_else(|| {
            if hit.source_kind == "document" {
                let t = hit.ref_id.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_owned())
                }
            } else {
                None
            }
        })?;
    if candidate.starts_with("KRD-") {
        Some(candidate)
    } else {
        None
    }
}

/// The UserManual slug for a `user_manual_page` hit (port of the React `userManualSlug`): metadata
/// `page_slug`, else the hit `ref_id`.
fn user_manual_slug(hit: &LoomGraphSearchHit) -> Option<String> {
    if hit.source_kind != "user_manual_page" {
        return None;
    }
    metadata_string(hit, "page_slug").or_else(|| {
        let t = hit.ref_id.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_owned())
        }
    })
}

/// Extract the first `WP-...` or `MT-...`-style id from a candidate string (port of the React
/// `firstIdMatch` over a `\bWP-[A-Za-z0-9-]+`-style regex). `prefix` is `"WP-"` or `"MT-"`.
///
/// Implemented without a regex dependency: scan for the (case-insensitive) prefix at a word boundary,
/// then consume the trailing `[A-Za-z0-9-]+` run.
fn first_id_match(value: Option<&str>, prefix: &str) -> Option<String> {
    let value = value?;
    let bytes = value.as_bytes();
    let plen = prefix.len();
    let mut i = 0usize;
    while i + plen <= bytes.len() {
        // `\b` before the prefix: start-of-string or a non-[A-Za-z0-9-] char precedes it.
        let boundary = i == 0 || !is_id_char(bytes[i - 1]);
        if boundary && value[i..].len() >= plen && value[i..i + plen].eq_ignore_ascii_case(prefix) {
            let mut j = i + plen;
            while j < bytes.len() && is_id_char(bytes[j]) {
                j += 1;
            }
            // Require at least one trailing id char so a bare prefix does not match.
            if j > i + plen {
                return Some(value[i..j].to_owned());
            }
        }
        i += 1;
    }
    None
}

/// True for the `[A-Za-z0-9-]` id alphabet used by the WP/MT id scanner.
fn is_id_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

/// Resolve the work-packet id for a `work_packet` hit (port of the React `workPacketId`).
fn work_packet_id(hit: &LoomGraphSearchHit) -> Option<String> {
    if hit.source_kind != "work_packet" {
        return None;
    }
    metadata_string(hit, "work_packet_id")
        .or_else(|| metadata_string(hit, "wp_id"))
        .or_else(|| first_id_match(metadata_string(hit, "entity_key").as_deref(), "WP-"))
        .or_else(|| first_id_match(Some(hit.ref_id.as_str()), "WP-"))
}

/// Resolve the microtask target for a `micro_task` hit (port of the React `microTaskTarget`).
fn micro_task_target(hit: &LoomGraphSearchHit) -> Option<(String, Option<String>)> {
    if hit.source_kind != "micro_task" {
        return None;
    }
    let mt_id = metadata_string(hit, "micro_task_id")
        .or_else(|| metadata_string(hit, "mt_id"))
        .or_else(|| first_id_match(metadata_string(hit, "entity_key").as_deref(), "MT-"))
        .or_else(|| first_id_match(Some(hit.ref_id.as_str()), "MT-"))?;
    let wp_id = metadata_string(hit, "work_packet_id")
        .or_else(|| metadata_string(hit, "wp_id"))
        .or_else(|| metadata_string(hit, "work_packet"))
        .or_else(|| first_id_match(metadata_string(hit, "entity_key").as_deref(), "WP-"));
    Some((mt_id, wp_id))
}

/// Map a [`LoomGraphSearchHit`] to its typed [`QuickSwitcherTarget`] (a faithful port of the React
/// `openTargetForLoomSearchHit`). The branch ORDER is load-bearing and matches the React function so
/// the native switcher resolves the SAME target for the SAME hit.
pub fn open_target_for_hit(hit: &LoomGraphSearchHit) -> QuickSwitcherTarget {
    // 1. wiki_page (by ref_id).
    if hit.source_kind == "wiki_page" && !hit.ref_id.trim().is_empty() {
        return QuickSwitcherTarget::WikiPage {
            projection_id: hit.ref_id.trim().to_owned(),
        };
    }
    // 2. user_manual_page slug.
    if let Some(slug) = user_manual_slug(hit) {
        return QuickSwitcherTarget::UserManual { slug };
    }
    // 3. rich document id (KRD-prefixed).
    if let Some(document_id) = document_id_from_hit(hit) {
        return QuickSwitcherTarget::Document { document_id };
    }
    // 4. a loom_block hit whose block carries a (non-KRD) source document_id.
    if hit.source_kind == "loom_block" {
        if let Some(document_id) = block_document_id(hit) {
            return QuickSwitcherTarget::Document { document_id };
        }
    }
    // 5. loom_block by ref_id.
    if hit.source_kind == "loom_block" && !hit.ref_id.trim().is_empty() {
        return QuickSwitcherTarget::LoomBlock {
            block_id: hit.ref_id.trim().to_owned(),
        };
    }
    // 6. file -> open as a Loom block.
    if hit.source_kind == "file" && !hit.ref_id.trim().is_empty() {
        return QuickSwitcherTarget::LoomBlock {
            block_id: hit.ref_id.trim().to_owned(),
        };
    }
    // 7. tag_hub -> open as a Loom block.
    if hit.source_kind == "tag_hub" && !hit.ref_id.trim().is_empty() {
        return QuickSwitcherTarget::LoomBlock {
            block_id: hit.ref_id.trim().to_owned(),
        };
    }
    // 8. symbol -> code symbol.
    if hit.source_kind == "symbol" && !hit.ref_id.trim().is_empty() {
        return QuickSwitcherTarget::CodeSymbol {
            symbol_entity_id: hit.ref_id.trim().to_owned(),
        };
    }
    // 9. work_packet.
    if let Some(wp_id) = work_packet_id(hit) {
        return QuickSwitcherTarget::WorkPacket { wp_id };
    }
    // 10. micro_task.
    if let Some((mt_id, wp_id)) = micro_task_target(hit) {
        return QuickSwitcherTarget::MicroTask { mt_id, wp_id };
    }
    QuickSwitcherTarget::Unsupported
}

/// Order the hits with recents-matching rows first, in recents order (port of the React
/// `orderedResults` useMemo). `recents` is the most-recent-first list of `hit_key`s; a hit whose
/// [`hit_key`] appears in it is ranked by its position there, ahead of every non-recent hit, and a
/// stable sort preserves the backend's relevance order within each rank class. Returned as owned
/// clones so the caller is not borrow-locked to `hits`.
pub fn ordered_results(hits: &[LoomGraphSearchHit], recents: &[String]) -> Vec<LoomGraphSearchHit> {
    let mut ordered = hits.to_vec();
    ordered.sort_by_key(|h| {
        let key = hit_key(h);
        recents.iter().position(|r| r == &key).unwrap_or(usize::MAX)
    });
    ordered
}

/// The stable, filesystem/id-safe slug used in a row's author_id suffix (port of the React
/// `stablePart`): lower-cased, non-alphanumerics collapsed to `-`, trimmed, with an `item` fallback.
fn stable_part(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_dash = true; // suppress leading dashes
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "item".to_owned()
    } else {
        out
    }
}

/// The author_id of a result row: `quick-switcher.option.{source_kind}.{stable(ref_id)}`.
fn row_author_id(hit: &LoomGraphSearchHit) -> String {
    format!(
        "{ROW_AUTHOR_ID_PREFIX}{}.{}",
        hit.source_kind,
        stable_part(&hit.ref_id)
    )
}

// ===========================================================================
// Async transport seam (MT-009 pattern): a SYNCHRONOUS trait for unit-testing,
// with a reqwest implementation that bridges async onto the app's tokio runtime.
// ===========================================================================

/// A transient transport failure (network down, non-success status, parse error). Carried into the
/// switcher's `error` / `recents_error` status fields so the operator sees it without a crash
/// (red-team MC3). Distinct from "no results".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTransportError(pub String);

impl std::fmt::Display for SearchTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The synchronous Loom-graph transport seam the switcher's background tasks drive. Synchronous so the
/// search/debounce manager + the response→hit mapping are pure and directly unit-testable with a stub
/// (no live server). The production [`LoomGraphSearchClient`] bridges this onto reqwest + the app's
/// tokio runtime (HBR-QUIET: the egui thread never calls these directly — the app spawns them).
pub trait LoomGraphSearchTransport: Send + Sync {
    /// `GET /workspaces/{workspace_id}/loom/graph-search?q=...&source_kinds=...&limit=...`.
    fn search(
        &self,
        workspace_id: &str,
        query: &str,
    ) -> Result<Vec<LoomGraphSearchHit>, SearchTransportError>;

    /// `GET /workspaces/{workspace_id}/loom/quick-switcher/recents?limit=...` → the `hit_key` list.
    fn list_recents(&self, workspace_id: &str) -> Result<Vec<String>, SearchTransportError>;

    /// `POST /workspaces/{workspace_id}/loom/quick-switcher/recents` (body from the hit) → the returned
    /// `hit_key` to prepend to the local recents list.
    fn record_recent(
        &self,
        workspace_id: &str,
        hit: &LoomGraphSearchHit,
    ) -> Result<String, SearchTransportError>;
}

/// Production transport: the backend's PostgreSQL-authoritative Loom graph-search + quick-switcher
/// recents REST surface, bridged onto the app's tokio runtime handle (the MT-009
/// `WorkbenchLayoutClient` pattern). reqwest is async; this holds a runtime [`Handle`] and bridges with
/// `Handle::block_on` so the transport stays a synchronous seam, and the app calls it ONLY from a
/// short-lived tokio task off the egui UI thread.
///
/// [`Handle`]: tokio::runtime::Handle
#[derive(Clone)]
pub struct LoomGraphSearchClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

/// Per-request timeout. A slow/absent backend must surface as a transient error, never hang a worker.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

impl LoomGraphSearchClient {
    /// Build a client against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]) bridging
    /// onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL, runtime)
    }

    fn recents_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/quick-switcher/recents",
            self.base_url,
            urlencode(workspace_id)
        )
    }
}

/// Minimal percent-encoding for a path segment (workspace ids are ascii ids, but encode defensively so
/// a stray space/slash cannot break the URL). Encodes everything outside the unreserved set.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl LoomGraphSearchTransport for LoomGraphSearchClient {
    fn search(
        &self,
        workspace_id: &str,
        query: &str,
    ) -> Result<Vec<LoomGraphSearchHit>, SearchTransportError> {
        let url = format!(
            "{}/workspaces/{}/loom/graph-search",
            self.base_url,
            urlencode(workspace_id)
        );
        let source_kinds = QUICK_SWITCHER_SOURCE_KINDS.join(",");
        let q = query.to_owned();
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .query(&[
                    ("q", q.as_str()),
                    ("source_kinds", source_kinds.as_str()),
                    ("limit", "25"),
                ])
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| SearchTransportError(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(SearchTransportError(format!(
                    "graph-search non-success status {}",
                    resp.status()
                )));
            }
            // The backend returns a bare JSON ARRAY of LoomGraphSearchResult.
            resp.json::<Vec<LoomGraphSearchHit>>()
                .await
                .map_err(|e| SearchTransportError(e.to_string()))
        })
    }

    fn list_recents(&self, workspace_id: &str) -> Result<Vec<String>, SearchTransportError> {
        let url = self.recents_url(workspace_id);
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .query(&[("limit", "20")])
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| SearchTransportError(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(SearchTransportError(format!(
                    "recents GET non-success status {}",
                    resp.status()
                )));
            }
            // The backend returns a bare JSON ARRAY of QuickSwitcherRecent; we only need the hit_key.
            let recents: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| SearchTransportError(e.to_string()))?;
            Ok(recents
                .into_iter()
                .filter_map(|r| r.get("hit_key").and_then(|k| k.as_str()).map(str::to_owned))
                .collect())
        })
    }

    fn record_recent(
        &self,
        workspace_id: &str,
        hit: &LoomGraphSearchHit,
    ) -> Result<String, SearchTransportError> {
        let url = self.recents_url(workspace_id);
        // POST body = QuickSwitcherRecentInput { result_kind, source_kind, ref_id, title, excerpt,
        // metadata }. Mirror the React recordQuickSwitcherRecent payload exactly.
        let body = serde_json::json!({
            "result_kind": hit.result_kind,
            "source_kind": hit.source_kind,
            "ref_id": hit.ref_id,
            "title": hit.title,
            "excerpt": hit.excerpt,
            "metadata": if hit.metadata.is_null() { serde_json::json!({}) } else { hit.metadata.clone() },
        });
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .post(&url)
                .timeout(REQUEST_TIMEOUT)
                .json(&body)
                .send()
                .await
                .map_err(|e| SearchTransportError(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(SearchTransportError(format!(
                    "recents POST non-success status {}",
                    resp.status()
                )));
            }
            let recorded: Value = resp
                .json()
                .await
                .map_err(|e| SearchTransportError(e.to_string()))?;
            recorded
                .get("hit_key")
                .and_then(|k| k.as_str())
                .map(str::to_owned)
                .ok_or_else(|| SearchTransportError("recents POST response missing hit_key".into()))
        })
    }
}

// ===========================================================================
// Search/debounce manager: a pure state machine the app ticks each frame. It
// owns the debounce timer + the search-sequence guard; the actual I/O is done
// by the app (it owns the runtime + the transport) when `tick` says to fire.
// ===========================================================================

/// The async results delivered back from a spawned search task: the sequence it was fired with plus the
/// outcome. The app writes this into [`SearchManager::results_cell`]; [`SearchManager::drain`] folds it
/// in iff the sequence still matches the latest dispatch (red-team MC2: drop stale arrivals).
#[derive(Debug, Clone)]
pub struct SearchDelivery {
    /// The dispatch sequence this delivery corresponds to.
    pub sequence: u64,
    /// `Ok(hits)` on success, `Err(message)` on a transient transport failure.
    pub outcome: Result<Vec<LoomGraphSearchHit>, String>,
}

/// What [`SearchManager::tick`] tells the app to do this frame. The manager never performs I/O itself;
/// it decides WHEN, and hands the app the query + sequence to spawn with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchAction {
    /// Nothing to do this frame.
    Idle,
    /// The debounce elapsed for a new query: fire `GET graph-search` with this query, tagged `sequence`.
    Fire { query: String, sequence: u64 },
}

/// The transient search/debounce state for one open generation of the switcher. The app ticks it each
/// frame with the current (trimmed) query + whether a workspace is selected; the manager emits a
/// [`SearchAction`] and tracks the in-flight sequence so out-of-order results are dropped.
#[derive(Debug, Default)]
pub struct SearchManager {
    /// The query the debounce timer is currently counting down for (`None` = no pending change).
    pending_query: Option<String>,
    /// When the current pending query was last seen change (the debounce anchor).
    pending_since: Option<Instant>,
    /// The query string that was last DISPATCHED (so we do not re-fire an unchanged query).
    last_query_sent: String,
    /// Monotonic dispatch counter; each `Fire` increments it. Used to drop stale deliveries.
    sequence: u64,
    /// The latest results folded in from a delivery whose sequence matched the latest dispatch.
    results: Vec<LoomGraphSearchHit>,
    /// True while a fired search has not yet been folded in (drives the "Searching graph..." status).
    loading: bool,
    /// The last transient search error message (cleared on a fresh successful fold-in).
    error: Option<String>,
}

impl SearchManager {
    /// Tick the debounce state machine for this frame.
    ///
    /// `query` is the current trimmed query; `has_workspace` is whether a workspace is selected; `now`
    /// is the frame clock. Returns the action the app should take. The manager:
    /// - clears results + loading when the query becomes empty or the workspace is absent;
    /// - (re)anchors the debounce timer when the query changes;
    /// - emits `Fire` once `now - pending_since >= SEARCH_DEBOUNCE` and the query differs from the last
    ///   dispatched query, bumping the sequence and setting `loading`.
    pub fn tick(&mut self, query: &str, has_workspace: bool, now: Instant) -> SearchAction {
        if query.is_empty() || !has_workspace {
            // Nothing to search: clear transient search state (recents are owned elsewhere).
            self.pending_query = None;
            self.pending_since = None;
            self.last_query_sent.clear();
            self.results.clear();
            self.loading = false;
            self.error = None;
            return SearchAction::Idle;
        }

        // (Re)anchor the debounce timer whenever the pending query text changes.
        if self.pending_query.as_deref() != Some(query) {
            self.pending_query = Some(query.to_owned());
            self.pending_since = Some(now);
        }

        // Already dispatched this exact query: nothing to do until it changes.
        if self.last_query_sent == query {
            return SearchAction::Idle;
        }

        // Fire once the debounce window has elapsed.
        if let Some(since) = self.pending_since {
            if now.duration_since(since) >= SEARCH_DEBOUNCE {
                self.last_query_sent = query.to_owned();
                self.sequence = self.sequence.wrapping_add(1);
                self.loading = true;
                self.error = None;
                return SearchAction::Fire {
                    query: query.to_owned(),
                    sequence: self.sequence,
                };
            }
        }
        SearchAction::Idle
    }

    /// Fold a delivered search result into the manager IFF its sequence is the latest dispatched one
    /// (red-team MC2). A stale delivery (an earlier, slower request) is dropped. Returns `true` when a
    /// delivery was accepted (so the app can request a repaint).
    pub fn drain(&mut self, delivery: SearchDelivery) -> bool {
        if delivery.sequence != self.sequence {
            return false; // stale: a newer query has since been dispatched.
        }
        self.loading = false;
        match delivery.outcome {
            Ok(hits) => {
                self.results = hits;
                self.error = None;
            }
            Err(msg) => {
                self.results.clear();
                self.error = Some(msg);
            }
        }
        true
    }

    /// The latest accepted results (unordered — the caller applies recents-first ordering).
    pub fn results(&self) -> &[LoomGraphSearchHit] {
        &self.results
    }

    /// True while a dispatched search has not yet been folded in.
    pub fn loading(&self) -> bool {
        self.loading
    }

    /// The last transient search error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The latest dispatch sequence (the app stamps a spawned task with this so a stale arrival is
    /// dropped by [`drain`]).
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// True when a query change is waiting on the debounce timer to elapse (i.e. a `Fire` is still
    /// pending). The app uses this to keep requesting repaints so the debounce actually times out even
    /// with no further input events.
    pub fn debounce_pending(&self) -> bool {
        match (&self.pending_query, self.pending_since) {
            (Some(q), Some(_)) => self.last_query_sent != *q,
            _ => false,
        }
    }
}

// ===========================================================================
// Overlay rendering.
// ===========================================================================

/// What the switcher wants the shell to do after a frame (port of the React `selectHit` + `onClose`).
///
/// `Open` carries the selected hit so the shell records the recent (`POST recents`) AND opens its typed
/// [`QuickSwitcherTarget`] on the active pane; it also closes the switcher. `Close` just clears the open
/// flag; `None` leaves the switcher open.
#[derive(Debug, Clone, PartialEq)]
pub enum SwitcherOutcome {
    /// Nothing happened this frame; keep the switcher open.
    None,
    /// The user picked a row (Enter on the selection or a click). Carries the chosen hit; the shell
    /// records the recent + opens the hit's target, then closes the switcher.
    Open(Box<LoomGraphSearchHit>),
    /// The user dismissed the switcher (Escape, the Close button, or a backdrop click).
    Close,
}

/// Per-open transient UI state stored in egui memory keyed to the switcher id and RESET on each re-open
/// (keyed by the shell's monotonic `open_count`). The query + selection live here; the heavier search
/// state ([`SearchManager`], recents, the async results cell) is owned by the shell across frames.
#[derive(Debug, Clone, Default)]
struct UiState {
    /// The open generation this UI state was initialized for. A new `open_count` resets it.
    open_count: u64,
    /// The current search query text.
    query: String,
    /// The selected row index into the CURRENT ordered+visible list (clamped each frame).
    selected_index: usize,
    /// Set once after a (re-)open so the search box is focused on the first frame only.
    focus_requested: bool,
}

/// Inputs the shell hands the overlay each frame (the search/recents state it owns across frames).
pub struct SwitcherView<'a> {
    /// The shell's monotonic open generation (resets the overlay's transient UI state on change).
    pub open_count: u64,
    /// The ordered, recents-first hits to render (already passed through [`ordered_results`]).
    pub results: &'a [LoomGraphSearchHit],
    /// Whether a workspace is selected (drives the "No workspace selected" status + disables search).
    pub has_workspace: bool,
    /// Whether a search request is in flight (drives "Searching graph...").
    pub loading: bool,
    /// The last transient search error, if any.
    pub error: Option<&'a str>,
    /// The last transient recents error, if any (shown as a trailing note on the status row).
    pub recents_error: Option<&'a str>,
}

/// The result of rendering a frame: the outcome PLUS the current query text, so the shell can drive its
/// [`SearchManager`] with the live query without owning the egui memory.
pub struct SwitcherFrame {
    /// What the shell should do (open a hit / close / nothing).
    pub outcome: SwitcherOutcome,
    /// The current (untrimmed) query text this frame, for the shell's debounce tick.
    pub query: String,
}

/// Render the quick-switcher overlay and return the [`SwitcherFrame`] for this frame.
///
/// `view` carries the shell-owned search/recents state; the overlay owns only the transient query +
/// selection (egui memory). The overlay is a backdrop [`egui::Area`] (full-screen, behind the panel,
/// click-to-dismiss) plus a centred [`egui::Window`] with the title bar hidden — both on the
/// `Foreground` order so the switcher sits above the workspace.
pub fn show(ctx: &egui::Context, view: SwitcherView<'_>) -> SwitcherFrame {
    let state_id = egui::Id::new("quick-switcher.state");
    let mut state: UiState = ctx
        .data_mut(|d| d.get_temp::<UiState>(state_id))
        .unwrap_or_default();

    // Reset transient state on (re-)open: a new open generation clears the query + selection.
    if state.open_count != view.open_count {
        state = UiState {
            open_count: view.open_count,
            query: String::new(),
            selected_index: 0,
            focus_requested: false,
        };
    }

    // Visible rows: only when there is a non-empty query AND a workspace (React `visibleResults`).
    let has_query = !state.query.trim().is_empty();
    let rows: &[LoomGraphSearchHit] = if has_query && view.has_workspace {
        view.results
    } else {
        &[]
    };

    // ── Keyboard navigation: read key events BEFORE rendering the list so Enter/arrows act on the
    //    selection computed from THIS frame's rows. The text input still receives typed characters. ──
    let mut nav_down = 0i64;
    let mut escape = false;
    let mut enter = false;
    ctx.input(|i| {
        for event in &i.events {
            if let egui::Event::Key {
                key, pressed: true, ..
            } = event
            {
                match key {
                    egui::Key::ArrowDown => nav_down += 1,
                    egui::Key::ArrowUp => nav_down -= 1,
                    egui::Key::Escape => escape = true,
                    egui::Key::Enter => enter = true,
                    _ => {}
                }
            }
        }
    });

    // Apply arrow navigation, clamped to the current range.
    if rows.is_empty() {
        state.selected_index = 0;
    } else {
        let max = rows.len() - 1;
        let cur = state.selected_index.min(max) as i64;
        let next = (cur + nav_down).clamp(0, max as i64);
        state.selected_index = next as usize;
    }

    if escape {
        persist(ctx, state_id, &state);
        return SwitcherFrame {
            outcome: SwitcherOutcome::Close,
            query: state.query,
        };
    }

    let mut outcome = SwitcherOutcome::None;

    // Enter opens the selected row (only if its target is enabled).
    if enter {
        if let Some(hit) = rows.get(state.selected_index) {
            if open_target_for_hit(hit).enabled() {
                outcome = SwitcherOutcome::Open(Box::new(hit.clone()));
            }
        }
    }

    // ── Backdrop: a full-screen interactable Area BEHIND the window; a click on it dismisses. ──
    let screen = ctx.content_rect();
    let backdrop = egui::Area::new(egui::Id::new("quick-switcher.backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(96));
            response
        });
    if backdrop.inner.clicked() && outcome == SwitcherOutcome::None {
        persist(ctx, state_id, &state);
        return SwitcherFrame {
            outcome: SwitcherOutcome::Close,
            query: state.query,
        };
    }

    // ── The switcher window: centred, fixed size, no title bar, always-on-top (above the backdrop). ──
    let search_egui_id = unsafe { egui::Id::from_high_entropy_bits(SWITCHER_SEARCH_NODE_ID) };
    let dialog_egui_id = unsafe { egui::Id::from_high_entropy_bits(SWITCHER_DIALOG_NODE_ID) };
    let list_egui_id = unsafe { egui::Id::from_high_entropy_bits(SWITCHER_LIST_NODE_ID) };

    let mut close_clicked = false;
    let mut clicked_open: Option<LoomGraphSearchHit> = None;
    let mut hovered_index: Option<usize> = None;

    egui::Window::new("quick_switcher")
        .id(egui::Id::new("quick-switcher.window"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([600.0, 460.0])
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // Header row: eyebrow + title on the left, Close button on the right (React parity).
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("OPEN").small().weak());
                    ui.label(egui::RichText::new("Quick Switcher").heading());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.button("Close");
                    // Tag the Close button with its stable author_id so it is a NAMED interactive
                    // control out-of-process (egui already derived Role::Button + Click/Focus actions).
                    emit_interactive_node(ui.ctx(), close.id, SWITCHER_CLOSE_AUTHOR_ID);
                    if close.clicked() {
                        close_clicked = true;
                    }
                });
            });
            ui.add_space(6.0);

            // Search input pinned to the fixed search id so its AccessKit NodeId is stable. Focus on the
            // first frame after (re-)open only (port of the React focus-on-open setTimeout(0)).
            let edit = egui::TextEdit::singleline(&mut state.query)
                .id(search_egui_id)
                .hint_text("Open by title, block, symbol, WP, MT, manual, or wiki page")
                .desired_width(f32::INFINITY);
            let edit_response = ui.add(edit);
            if edit_response.changed() {
                state.selected_index = 0;
            }
            if !state.focus_requested {
                edit_response.request_focus();
                state.focus_requested = true;
            }
            emit_search_node(ui.ctx(), search_egui_id);

            ui.add_space(6.0);

            // Status row (the React aria-live status line).
            let status = if !has_query {
                "Type to search the project graph".to_owned()
            } else if !view.has_workspace {
                "No workspace selected".to_owned()
            } else if view.loading {
                "Searching graph...".to_owned()
            } else if let Some(err) = view.error {
                err.to_owned()
            } else {
                let n = rows.len();
                let mut s = format!("{n} result{}", if n == 1 { "" } else { "s" });
                if let Some(re) = view.recents_error {
                    s.push_str(&format!("; durable recents unavailable: {re}"));
                }
                s
            };
            ui.label(egui::RichText::new(status).small().weak());
            ui.add_space(4.0);

            let sel = state.selected_index.min(rows.len().saturating_sub(1));

            // List container node (Role::ListBox) reserved at the fixed list id; vertical scroll of rows.
            // red-team R6/MC6: row count is capped at the search limit (25), well under the 0xFF (255)
            // author-id budget; this assert catches a future limit increase.
            debug_assert!(
                rows.len() <= 255,
                "quick-switcher row count must stay <= 255"
            );
            ui.push_id(list_egui_id, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, hit) in rows.iter().enumerate() {
                            let is_selected = idx == sel;
                            let resp = switcher_row(ui, hit, is_selected);
                            if resp.hovered() {
                                hovered_index = Some(idx);
                            }
                            if resp.clicked() {
                                clicked_open = Some(hit.clone());
                            }
                        }
                    });
            });
            emit_list_node(ui.ctx(), list_egui_id);
        });

    // The dialog root container node (Role::Dialog, modal). Emitted unconditionally each open frame.
    emit_dialog_node(ctx, dialog_egui_id);

    // Hovering a row updates the selection (React parity: onMouseEnter -> setSelectedIndex(idx)).
    if let Some(h) = hovered_index {
        state.selected_index = h;
    }

    // Resolve the frame's outcome precedence: an explicit click/Enter Open wins, then Close, then None.
    if outcome == SwitcherOutcome::None {
        if let Some(hit) = clicked_open {
            outcome = SwitcherOutcome::Open(Box::new(hit));
        } else if close_clicked {
            outcome = SwitcherOutcome::Close;
        }
    }

    let query = state.query.clone();
    persist(ctx, state_id, &state);
    SwitcherFrame { outcome, query }
}

/// Persist the transient switcher UI state back into egui memory.
fn persist(ctx: &egui::Context, state_id: egui::Id, state: &UiState) {
    ctx.data_mut(|d| d.insert_temp(state_id, state.clone()));
}

/// Render one result row as a full-width selectable button: a kind chip (the source label), the title
/// (bold), the excerpt (muted, truncated), and the open-target hint. Disabled rows (Unsupported target)
/// are non-interactive. The whole row is a single addressable ListBoxOption.
fn switcher_row(ui: &mut egui::Ui, hit: &LoomGraphSearchHit, is_selected: bool) -> egui::Response {
    let author_id = row_author_id(hit);
    let target = open_target_for_hit(hit);
    let enabled = target.enabled();
    let full_width = ui.available_width();

    let strong = ui.visuals().strong_text_color();
    let weak = ui.visuals().weak_text_color();

    let mut job = egui::text::LayoutJob::default();
    // Kind chip: the source label, muted.
    job.append(
        &format!("{}  ", source_label(&hit.source_kind)),
        0.0,
        egui::TextFormat {
            color: weak,
            ..Default::default()
        },
    );
    // Title, bold/strong.
    job.append(
        &hit.title,
        0.0,
        egui::TextFormat {
            color: strong,
            ..Default::default()
        },
    );
    // Excerpt, muted + truncated to ~80 chars (port of the React excerpt rendering).
    let excerpt = truncate_excerpt(&hit.excerpt);
    if !excerpt.is_empty() {
        job.append(
            &format!("   {excerpt}"),
            0.0,
            egui::TextFormat {
                color: weak,
                ..Default::default()
            },
        );
    }
    // Target hint trails the row (a single LayoutJob cannot right-align a span).
    job.append(
        &format!("   ({})", target.label()),
        0.0,
        egui::TextFormat {
            color: weak,
            italics: true,
            ..Default::default()
        },
    );

    let response = ui.add_enabled(
        enabled,
        egui::Button::selectable(is_selected, job)
            .truncate()
            .min_size(egui::vec2(full_width, 0.0)),
    );

    // Attach the stable author_id + ListBoxOption role + selected/disabled state to the SAME live node
    // egui built for this row.
    let label = hit.title.clone();
    ui.ctx().accesskit_node_builder(response.id, move |node| {
        node.set_role(accesskit::Role::ListBoxOption);
        node.set_author_id(author_id);
        node.set_label(label);
        if is_selected {
            node.set_selected(true);
        }
        if !enabled {
            node.set_disabled();
        }
    });

    response
}

/// Truncate an excerpt to ~80 chars with an ellipsis (port of the React `excerpt.slice(0, 77) + "..."`
/// logic). Char-boundary safe.
fn truncate_excerpt(excerpt: &str) -> String {
    let trimmed = excerpt.trim();
    if trimmed.chars().count() <= 80 {
        return trimmed.to_owned();
    }
    let head: String = trimmed.chars().take(77).collect();
    format!("{head}...")
}

/// Emit the switcher DIALOG root node (Role::Dialog, modal=true, label="Quick switcher").
fn emit_dialog_node(ctx: &egui::Context, dialog_id: egui::Id) {
    ctx.accesskit_node_builder(dialog_id, |node| {
        node.set_role(accesskit::Role::Dialog);
        node.set_author_id(SWITCHER_DIALOG_AUTHOR_ID.to_owned());
        node.set_label("Quick switcher".to_owned());
        node.set_modal();
    });
}

/// Emit the switcher SEARCH box address. egui already derived `Role::TextInput` + actions for the
/// `TextEdit`; this only adds the stable author_id.
fn emit_search_node(ctx: &egui::Context, search_id: egui::Id) {
    emit_interactive_node(ctx, search_id, SWITCHER_SEARCH_AUTHOR_ID);
}

/// Emit the switcher LIST container node (Role::ListBox, label="Quick switcher results").
fn emit_list_node(ctx: &egui::Context, list_id: egui::Id) {
    ctx.accesskit_node_builder(list_id, |node| {
        node.set_role(accesskit::Role::ListBox);
        node.set_author_id(SWITCHER_LIST_AUTHOR_ID.to_owned());
        node.set_label("Quick switcher results".to_owned());
    });
}

/// The async results cell type the app shares with a spawned search task: the task writes its
/// [`SearchDelivery`] into the `Option`, and the egui frame drains it (try_lock) into the
/// [`SearchManager`]. Defined here so the seam (cell type + delivery + manager) lives in one module.
pub type SearchDeliveryCell = Arc<Mutex<Option<SearchDelivery>>>;

/// The async cell type a spawned recents-load task writes into: `Ok(hit_keys)` on success, `Err(msg)`
/// on a transient transport failure. Drained (try_lock) by the egui frame.
pub type RecentsDeliveryCell = Arc<Mutex<Option<Result<Vec<String>, String>>>>;

/// The async cell a spawned recents-RECORD (`POST recents`) task writes into: `Ok(hit_key)` with the
/// backend-confirmed key on success, `Err(msg)` on a transient transport failure. Drained (try_lock) by
/// the egui frame so the network POST never blocks the UI thread (HBR-QUIET). The optimistic local
/// recents prepend happens immediately when the hit is picked; this delivery only reconciles the
/// backend-confirmed key (re-promote to front) or surfaces the failure via `recents_error` (MC3).
pub type RecordRecentDeliveryCell = Arc<Mutex<Option<Result<String, String>>>>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hit(source_kind: &str, ref_id: &str) -> LoomGraphSearchHit {
        LoomGraphSearchHit {
            result_kind: "loom_block".to_owned(),
            source_kind: source_kind.to_owned(),
            ref_id: ref_id.to_owned(),
            title: format!("{source_kind} {ref_id}"),
            excerpt: String::new(),
            block: Value::Null,
            score: 0.0,
            metadata: Value::Null,
        }
    }

    // ── Fixed container ids + author_ids (registry parity) ─────────────────────────────────────────

    #[test]
    fn switcher_container_ids_in_disjoint_fresh_band() {
        for id in [
            SWITCHER_DIALOG_NODE_ID,
            SWITCHER_SEARCH_NODE_ID,
            SWITCHER_LIST_NODE_ID,
        ] {
            assert!((14..=16).contains(&id), "switcher id {id} in band 14..=16");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "switcher id {id} below pane base {}",
                crate::accessibility::PANE_NODE_ID_BASE
            );
            assert!(
                !(11..=13).contains(&id),
                "switcher id {id} not in palette band"
            );
        }
        assert_ne!(SWITCHER_DIALOG_NODE_ID, SWITCHER_SEARCH_NODE_ID);
        assert_ne!(SWITCHER_SEARCH_NODE_ID, SWITCHER_LIST_NODE_ID);
        assert_ne!(SWITCHER_DIALOG_NODE_ID, SWITCHER_LIST_NODE_ID);
    }

    #[test]
    fn switcher_author_ids_are_stable() {
        assert_eq!(SWITCHER_DIALOG_AUTHOR_ID, "quick-switcher.dialog");
        assert_eq!(SWITCHER_SEARCH_AUTHOR_ID, "quick-switcher.search");
        assert_eq!(SWITCHER_LIST_AUTHOR_ID, "quick-switcher.list");
        assert_eq!(ROW_AUTHOR_ID_PREFIX, "quick-switcher.option.");
    }

    // ── hit_key + source_label ─────────────────────────────────────────────────────────────────────

    #[test]
    fn hit_key_is_source_kind_colon_ref_id() {
        let h = hit("document", "KRD-123");
        assert_eq!(hit_key(&h), "document:KRD-123");
        let h2 = hit("work_packet", "WP-KERNEL-011");
        assert_eq!(hit_key(&h2), "work_packet:WP-KERNEL-011");
    }

    #[test]
    fn source_label_covers_all_nine_kinds() {
        assert_eq!(source_label("loom_block"), "Loom Block");
        assert_eq!(source_label("file"), "File");
        assert_eq!(source_label("tag_hub"), "Tag Hub");
        assert_eq!(source_label("document"), "Document");
        assert_eq!(source_label("symbol"), "Symbol");
        assert_eq!(source_label("work_packet"), "Work Packet");
        assert_eq!(source_label("micro_task"), "Microtask");
        assert_eq!(source_label("user_manual_page"), "UserManual Page");
        assert_eq!(source_label("wiki_page"), "Wiki Page");
        assert_eq!(source_label("nonsense"), "Unknown");
        // The constant list matches the labelled kinds.
        assert_eq!(QUICK_SWITCHER_SOURCE_KINDS.len(), 9);
    }

    #[test]
    fn row_author_id_is_stable_slug() {
        let h = hit("work_packet", "WP-KERNEL-011");
        assert_eq!(
            row_author_id(&h),
            "quick-switcher.option.work_packet.wp-kernel-011"
        );
        let h2 = hit("document", "KRD 12/34");
        assert_eq!(
            row_author_id(&h2),
            "quick-switcher.option.document.krd-12-34"
        );
    }

    // ── ordered_results (recents-first) ────────────────────────────────────────────────────────────

    #[test]
    fn ordered_results_sorts_recents_to_front_in_recents_order() {
        let hits = vec![
            hit("document", "KRD-a"),
            hit("work_packet", "WP-1"),
            hit("symbol", "sym-x"),
            hit("micro_task", "MT-9"),
        ];
        // Recents: micro_task:MT-9 first, then work_packet:WP-1.
        let recents = vec!["micro_task:MT-9".to_owned(), "work_packet:WP-1".to_owned()];
        let ordered = ordered_results(&hits, &recents);
        let keys: Vec<String> = ordered.iter().map(hit_key).collect();
        assert_eq!(
            keys,
            vec![
                "micro_task:MT-9".to_owned(),  // recent rank 0
                "work_packet:WP-1".to_owned(), // recent rank 1
                "document:KRD-a".to_owned(),   // non-recent, original order
                "symbol:sym-x".to_owned(),
            ]
        );
    }

    #[test]
    fn ordered_results_no_recents_preserves_order() {
        let hits = vec![hit("document", "KRD-a"), hit("symbol", "s1")];
        let ordered = ordered_results(&hits, &[]);
        let keys: Vec<String> = ordered.iter().map(hit_key).collect();
        assert_eq!(
            keys,
            vec!["document:KRD-a".to_owned(), "symbol:s1".to_owned()]
        );
    }

    // ── open_target_for_hit (faithful port) ────────────────────────────────────────────────────────

    #[test]
    fn target_wiki_page_by_ref_id() {
        let h = hit("wiki_page", "PROJ-42");
        assert_eq!(
            open_target_for_hit(&h),
            QuickSwitcherTarget::WikiPage {
                projection_id: "PROJ-42".to_owned()
            }
        );
    }

    #[test]
    fn target_user_manual_prefers_page_slug_metadata() {
        let mut h = hit("user_manual_page", "fallback-ref");
        h.metadata = json!({ "page_slug": "getting-started" });
        assert_eq!(
            open_target_for_hit(&h),
            QuickSwitcherTarget::UserManual {
                slug: "getting-started".to_owned()
            }
        );
        // Without metadata, falls back to ref_id.
        let h2 = hit("user_manual_page", "ref-slug");
        assert_eq!(
            open_target_for_hit(&h2),
            QuickSwitcherTarget::UserManual {
                slug: "ref-slug".to_owned()
            }
        );
    }

    #[test]
    fn target_document_requires_krd_prefix() {
        let mut h = hit("document", "KRD-77");
        h.metadata = json!({ "rich_document_id": "KRD-99" });
        // metadata rich_document_id wins over ref_id.
        assert_eq!(
            open_target_for_hit(&h),
            QuickSwitcherTarget::Document {
                document_id: "KRD-99".to_owned()
            }
        );
        // A document hit whose ref_id is NOT KRD-prefixed and has no metadata -> Unsupported.
        let h2 = hit("document", "plain-id");
        assert_eq!(open_target_for_hit(&h2), QuickSwitcherTarget::Unsupported);
    }

    #[test]
    fn target_loom_block_opens_source_document_when_block_has_one() {
        let mut h = hit("loom_block", "blk-1");
        h.block = json!({ "document_id": "doc-xyz" });
        // block.document_id (non-KRD) -> Document target via branch 4.
        assert_eq!(
            open_target_for_hit(&h),
            QuickSwitcherTarget::Document {
                document_id: "doc-xyz".to_owned()
            }
        );
        // Without a block document_id, falls through to a LoomBlock target by ref_id.
        let h2 = hit("loom_block", "blk-2");
        assert_eq!(
            open_target_for_hit(&h2),
            QuickSwitcherTarget::LoomBlock {
                block_id: "blk-2".to_owned()
            }
        );
    }

    #[test]
    fn target_file_and_tag_hub_open_as_loom_block() {
        assert_eq!(
            open_target_for_hit(&hit("file", "f-1")),
            QuickSwitcherTarget::LoomBlock {
                block_id: "f-1".to_owned()
            }
        );
        assert_eq!(
            open_target_for_hit(&hit("tag_hub", "t-1")),
            QuickSwitcherTarget::LoomBlock {
                block_id: "t-1".to_owned()
            }
        );
    }

    #[test]
    fn target_symbol_opens_code_symbol() {
        assert_eq!(
            open_target_for_hit(&hit("symbol", "sym-9")),
            QuickSwitcherTarget::CodeSymbol {
                symbol_entity_id: "sym-9".to_owned()
            }
        );
    }

    #[test]
    fn target_work_packet_extracts_id_from_ref_or_metadata() {
        // From ref_id regex.
        let h = hit("work_packet", "loom://WP-KERNEL-011/node");
        assert_eq!(
            open_target_for_hit(&h),
            QuickSwitcherTarget::WorkPacket {
                wp_id: "WP-KERNEL-011".to_owned()
            }
        );
        // metadata work_packet_id wins.
        let mut h2 = hit("work_packet", "ignored");
        h2.metadata = json!({ "work_packet_id": "WP-007" });
        assert_eq!(
            open_target_for_hit(&h2),
            QuickSwitcherTarget::WorkPacket {
                wp_id: "WP-007".to_owned()
            }
        );
    }

    #[test]
    fn target_micro_task_extracts_mt_and_wp() {
        let mut h = hit("micro_task", "MT-017");
        h.metadata = json!({ "entity_key": "WP-KERNEL-011/MT-017" });
        assert_eq!(
            open_target_for_hit(&h),
            QuickSwitcherTarget::MicroTask {
                mt_id: "MT-017".to_owned(),
                wp_id: Some("WP-KERNEL-011".to_owned()),
            }
        );
        // mt_id from ref_id, no wp available -> wp_id None.
        let h2 = hit("micro_task", "ref MT-042 here");
        assert_eq!(
            open_target_for_hit(&h2),
            QuickSwitcherTarget::MicroTask {
                mt_id: "MT-042".to_owned(),
                wp_id: None
            }
        );
    }

    #[test]
    fn target_unsupported_disables_row() {
        // A document hit with no KRD id and no work-packet/microtask path -> Unsupported.
        let h = hit("document", "not-krd");
        let t = open_target_for_hit(&h);
        assert_eq!(t, QuickSwitcherTarget::Unsupported);
        assert!(!t.enabled());
        assert_eq!(t.label(), "No direct app target yet");
    }

    #[test]
    fn first_id_match_finds_boundary_ids_only() {
        assert_eq!(first_id_match(Some("WP-1"), "WP-"), Some("WP-1".to_owned()));
        assert_eq!(first_id_match(Some("xWP-1"), "WP-"), None); // not a boundary
        assert_eq!(
            first_id_match(Some("see WP-KERNEL-011!"), "WP-"),
            Some("WP-KERNEL-011".to_owned())
        );
        assert_eq!(first_id_match(Some("WP-"), "WP-"), None); // bare prefix, no trailing id
        assert_eq!(first_id_match(Some("no ids"), "MT-"), None);
        assert_eq!(first_id_match(None, "WP-"), None);
    }

    // ── response-JSON -> hit mapping (round-trip vs the real backend shape) ─────────────────────────

    #[test]
    fn deserializes_backend_graph_search_result_shape() {
        // This JSON matches the backend LoomGraphSearchResult serialization (snake_case kinds, optional
        // block/metadata, default excerpt). See src/backend/handshake_core/src/storage/loom.rs:614.
        let payload = json!([
            {
                "result_kind": "loom_block",
                "source_kind": "document",
                "ref_id": "KRD-123",
                "title": "Design doc",
                "excerpt": "the design...",
                "score": 1.5,
                "metadata": { "rich_document_id": "KRD-123" }
            },
            {
                "result_kind": "knowledge_entity",
                "source_kind": "work_packet",
                "ref_id": "WP-KERNEL-011",
                "title": "WorkSurface Shell"
                // no excerpt / block / metadata: must default cleanly.
            }
        ]);
        let hits: Vec<LoomGraphSearchHit> = serde_json::from_value(payload).expect("deserialize");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_kind, "document");
        assert_eq!(hits[0].ref_id, "KRD-123");
        assert_eq!(hits[0].excerpt, "the design...");
        assert_eq!(
            open_target_for_hit(&hits[0]),
            QuickSwitcherTarget::Document {
                document_id: "KRD-123".to_owned()
            }
        );
        // Defaults: excerpt empty, metadata null, block null.
        assert_eq!(hits[1].excerpt, "");
        assert!(hits[1].metadata.is_null());
        assert_eq!(
            open_target_for_hit(&hits[1]),
            QuickSwitcherTarget::WorkPacket {
                wp_id: "WP-KERNEL-011".to_owned()
            }
        );
    }

    // ── SearchManager debounce + sequence guard ────────────────────────────────────────────────────

    #[test]
    fn search_manager_fires_after_debounce_only() {
        let mut mgr = SearchManager::default();
        let t0 = Instant::now();
        // First tick anchors the timer; not yet elapsed -> Idle.
        assert_eq!(mgr.tick("doc", true, t0), SearchAction::Idle);
        // Before the debounce elapses -> still Idle.
        assert_eq!(
            mgr.tick("doc", true, t0 + Duration::from_millis(100)),
            SearchAction::Idle
        );
        // After the debounce window -> Fire with sequence 1.
        let action = mgr.tick("doc", true, t0 + Duration::from_millis(160));
        assert_eq!(
            action,
            SearchAction::Fire {
                query: "doc".to_owned(),
                sequence: 1
            }
        );
        assert!(mgr.loading());
        // A subsequent tick with the SAME query does not re-fire.
        assert_eq!(
            mgr.tick("doc", true, t0 + Duration::from_millis(400)),
            SearchAction::Idle
        );
    }

    #[test]
    fn search_manager_reanchors_debounce_on_query_change() {
        let mut mgr = SearchManager::default();
        let t0 = Instant::now();
        mgr.tick("do", true, t0);
        // Query changes before the first fire: the timer re-anchors, so at t0+160 (relative to t0) the
        // NEW query "doc" has only had ~10ms.
        let t1 = t0 + Duration::from_millis(150);
        assert_eq!(mgr.tick("doc", true, t1), SearchAction::Idle);
        // Now wait the full debounce for "doc".
        let action = mgr.tick("doc", true, t1 + Duration::from_millis(160));
        assert_eq!(
            action,
            SearchAction::Fire {
                query: "doc".to_owned(),
                sequence: 1
            }
        );
    }

    #[test]
    fn search_manager_clears_on_empty_query_or_no_workspace() {
        let mut mgr = SearchManager::default();
        let t0 = Instant::now();
        let action = mgr.tick("doc", true, t0 + Duration::from_millis(200));
        // First real tick anchors; need a prior tick to anchor. Do it properly:
        let _ = action;
        let mut mgr2 = SearchManager::default();
        mgr2.tick("doc", true, t0);
        mgr2.tick("doc", true, t0 + Duration::from_millis(200)); // fires
        assert!(mgr2.loading());
        // Empty query clears everything.
        assert_eq!(
            mgr2.tick("", true, t0 + Duration::from_millis(300)),
            SearchAction::Idle
        );
        assert!(!mgr2.loading());
        assert!(mgr2.results().is_empty());
        // No workspace also clears + suppresses firing.
        mgr2.tick("doc", true, t0 + Duration::from_millis(400));
        assert_eq!(
            mgr2.tick("doc", false, t0 + Duration::from_millis(600)),
            SearchAction::Idle
        );
    }

    #[test]
    fn search_manager_drops_stale_delivery_accepts_current() {
        let mut mgr = SearchManager::default();
        let t0 = Instant::now();
        mgr.tick("doc", true, t0);
        let a1 = mgr.tick("doc", true, t0 + Duration::from_millis(160));
        let seq1 = match a1 {
            SearchAction::Fire { sequence, .. } => sequence,
            _ => panic!("expected Fire"),
        };
        // Query changes and fires again -> sequence 2.
        mgr.tick("docs", true, t0 + Duration::from_millis(200));
        let a2 = mgr.tick("docs", true, t0 + Duration::from_millis(400));
        let seq2 = match a2 {
            SearchAction::Fire { sequence, .. } => sequence,
            _ => panic!("expected Fire 2"),
        };
        assert_ne!(seq1, seq2);
        // A delivery for the STALE seq1 is dropped.
        let dropped = mgr.drain(SearchDelivery {
            sequence: seq1,
            outcome: Ok(vec![hit("document", "KRD-old")]),
        });
        assert!(!dropped, "stale delivery dropped");
        assert!(mgr.results().is_empty());
        // A delivery for the CURRENT seq2 is accepted.
        let accepted = mgr.drain(SearchDelivery {
            sequence: seq2,
            outcome: Ok(vec![hit("document", "KRD-new")]),
        });
        assert!(accepted, "current delivery accepted");
        assert_eq!(mgr.results().len(), 1);
        assert_eq!(mgr.results()[0].ref_id, "KRD-new");
        assert!(!mgr.loading());
    }

    #[test]
    fn search_manager_records_error_delivery() {
        let mut mgr = SearchManager::default();
        let t0 = Instant::now();
        mgr.tick("doc", true, t0);
        let a = mgr.tick("doc", true, t0 + Duration::from_millis(160));
        let seq = match a {
            SearchAction::Fire { sequence, .. } => sequence,
            _ => panic!("expected Fire"),
        };
        assert!(mgr.drain(SearchDelivery {
            sequence: seq,
            outcome: Err("backend down".to_owned()),
        }));
        assert_eq!(mgr.error(), Some("backend down"));
        assert!(mgr.results().is_empty());
        assert!(!mgr.loading());
    }

    // ── Stub transport: response mapping + recents round-trip (no live server) ──────────────────────

    /// An in-memory [`LoomGraphSearchTransport`] stub for unit tests: canned search results, a recents
    /// list, and a record-recent that returns the hit's key. Proves the seam is driveable without a
    /// live backend (the same property the kittest + the gated live-PG test rely on).
    struct StubTransport {
        results: Vec<LoomGraphSearchHit>,
        recents: Vec<String>,
    }

    impl LoomGraphSearchTransport for StubTransport {
        fn search(
            &self,
            _workspace_id: &str,
            _query: &str,
        ) -> Result<Vec<LoomGraphSearchHit>, SearchTransportError> {
            Ok(self.results.clone())
        }
        fn list_recents(&self, _workspace_id: &str) -> Result<Vec<String>, SearchTransportError> {
            Ok(self.recents.clone())
        }
        fn record_recent(
            &self,
            _workspace_id: &str,
            hit: &LoomGraphSearchHit,
        ) -> Result<String, SearchTransportError> {
            Ok(hit_key(hit))
        }
    }

    #[test]
    fn stub_transport_drives_search_and_recents_round_trip() {
        let stub = StubTransport {
            results: vec![hit("document", "KRD-1"), hit("work_packet", "WP-2")],
            recents: vec!["work_packet:WP-2".to_owned()],
        };
        let transport: &dyn LoomGraphSearchTransport = &stub;

        let results = transport.search("ws", "anything").expect("search ok");
        let recents = transport.list_recents("ws").expect("recents ok");

        // recents-first ordering puts WP-2 ahead of KRD-1.
        let ordered = ordered_results(&results, &recents);
        let keys: Vec<String> = ordered.iter().map(hit_key).collect();
        assert_eq!(
            keys,
            vec!["work_packet:WP-2".to_owned(), "document:KRD-1".to_owned()]
        );

        // record_recent returns the picked hit's key.
        let recorded = transport
            .record_recent("ws", &results[0])
            .expect("record ok");
        assert_eq!(recorded, "document:KRD-1");
    }

    #[test]
    fn truncate_excerpt_caps_at_80_chars() {
        assert_eq!(truncate_excerpt("short"), "short");
        let long = "x".repeat(100);
        let t = truncate_excerpt(&long);
        assert_eq!(t.chars().count(), 80); // 77 + "..."
        assert!(t.ends_with("..."));
    }
}
