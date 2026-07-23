//! App-wide shared interaction substrate (WP-KERNEL-012 MT-031, cluster E5 — melt-together).
//!
//! ## What this is (the single melt-together substrate)
//!
//! [`InteractionBus`] is the one object every editor pane (code, rich-text, graph, canvas) shares so
//! the four surfaces have ONE selection model, ONE clipboard, and ONE cross-pane command surface — the
//! E5 "melt-together" invariant. Without it every downstream E5 MT (MT-032..MT-035) would grow its own
//! ad-hoc selection/clipboard and the parity proof suite would fragment.
//!
//! It is stored in egui's per-context app-data store under the stable key
//! [`INTERACTION_BUS_ID`] so any pane retrieves the SAME `Arc<Mutex<InteractionBus>>` in its `update()`
//! via [`InteractionBus::get_or_init`] — no global static, no parallel singletons.
//!
//! ## WRAP, do not FORK (the contract's core constraint)
//!
//! The bus WRAPS + coordinates the EXISTING WP-011 substrate instead of forking it:
//! - command surface: the static [`crate::command_registry`] catalog stays the canonical descriptor
//!   store; the bus's [`CommandBus`] holds ONLY the cross-pane commands a pane registers at runtime
//!   (Copy/Cut/Paste/SelectAll/Find/CommandPalette) keyed by id, and exposes them for dispatch +
//!   keybind matching. It never re-defines the static catalog.
//! - event fan-out: cross-pane focus / selection notifications publish through the EXISTING
//!   [`crate::event_bus`] `ShellEventBus` (the bus does NOT invent a second event system); the
//!   focus-changed signal lives on the bus itself because it is bus-private coordination state.
//! - the command palette: the existing [`crate::command_palette`] modal is driven by
//!   [`InteractionBus::command_palette_open`]; the bus does NOT build a second palette.
//!
//! ## Clipboard = egui-native behind a MOCKABLE seam (MT-017 precedent, NOT raw arboard)
//!
//! Clipboard writes route through the [`ClipboardSink`](crate::rich_editor::properties::metadata_client::ClipboardSink)
//! trait (the MT-017 seam): the production sink delegates to `egui::Context::copy_text` (which the
//! egui-winit bridge writes to the OS clipboard), and a headless test injects an in-memory mock so a
//! test NEVER touches the OS clipboard (arboard hangs/fails headless — red-team RISK-2 / MC-2). The
//! richest variant (a [`ClipboardPayload::LoomBlockRef`] / [`ClipboardPayload::AtelierRef`] egui's text
//! clipboard cannot carry) is ALSO cached IN-MEMORY on the bus, so a same-session cross-pane Paste can
//! recover the rich payload that the plain-text OS clipboard would have flattened.
//!
//! ## Re-entrancy safety (red-team RISK-1 / MC-1)
//!
//! Panes reach the bus via [`InteractionBus::with_try_lock`] (a `try_lock` wrapper) in their per-frame
//! `update()` so a second pane touching the bus in the SAME frame never blocks the egui frame thread,
//! and a command handler must NEVER re-enter the lock (it receives `&mut InteractionBus` already
//! locked). Contention is logged once and skipped, never deadlocked.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::event_bus::ShellEventSender;
use crate::pane_registry::PaneId;
use crate::rich_editor::properties::metadata_client::ClipboardSink;
use crate::undo_stack::{UndoAction, UndoResult, UnifiedUndoScope};

/// Maximum workspace/session emitter generations retained by one app bus. The active/recent set keeps
/// delayed A -> B -> A completions attributable while bounding background workers under long sessions.
pub const MAX_RETAINED_EVENT_EMITTER_WORKSPACES: usize = 8;

/// Stable egui app-data key for the shared bus. Every pane retrieves the bus by this id in `update()`
/// (`ctx.data_mut(|d| d.get_temp::<Arc<Mutex<InteractionBus>>>(INTERACTION_BUS_ID))`), so all panes
/// observe the SAME instance. The string is hashed by `egui::Id::new`, the crate's standard data-key
/// convention (mirrors `command-palette.state` in `command_palette.rs`).
pub const INTERACTION_BUS_KEY: &str = "handshake_interaction_bus";

/// The egui `Id` for [`INTERACTION_BUS_KEY`] (computed once at call sites via `egui::Id::new`).
pub fn interaction_bus_id() -> egui::Id {
    egui::Id::new(INTERACTION_BUS_KEY)
}

// ── Stable cross-pane command ids (the canonical melt-together command vocabulary) ───────────────────
/// Cross-pane Copy command id (VS Code Ctrl+C).
pub const CMD_COPY: &str = "interop.copy";
/// Cross-pane Cut command id (VS Code Ctrl+X).
pub const CMD_CUT: &str = "interop.cut";
/// Cross-pane Paste command id (VS Code Ctrl+V).
pub const CMD_PASTE: &str = "interop.paste";
/// Cross-pane Select-All command id (VS Code Ctrl+A).
pub const CMD_SELECT_ALL: &str = "interop.select-all";
/// Cross-pane Find command id (VS Code Ctrl+F).
pub const CMD_FIND: &str = "interop.find";
/// Cross-pane Command-Palette command id (VS Code Ctrl+Shift+P).
pub const CMD_COMMAND_PALETTE: &str = "interop.command-palette";
/// WP-KERNEL-012 MT-032 (E5): cross-pane Open-Document command id. A backlink row / loom:// reference
/// dispatches this with the target document id staged via [`InteractionBus::request_open_document`]; the
/// shell drains [`InteractionBus::take_pending_navigation`] and routes the open. This is the
/// melt-together navigation primitive the "everything is a Loom block" backlinks/refs ride on.
pub const CMD_OPEN_DOCUMENT: &str = "interop.open-document";
/// WP-KERNEL-012 MT-033 (E5 — route-to-Stage): cross-pane Route-to-Stage command id. A rich-text
/// selection / canvas node / CKC item dispatches this with the [`crate::stage_pane::StageContent`] staged
/// via [`InteractionBus::route_to_stage`]; the shell peeks the complete pending route, applies it to
/// Stage, then acknowledges that exact event id before emitting its success receipt.
/// This is the melt-together Editors<->Stage (Pillar 17) navigation primitive. The DEEPER Stage backend
/// interop (capture/embed-back with manifest provenance) is E10 (MT-066), NOT this command.
pub const CMD_ROUTE_TO_STAGE: &str = "interop.route-to-stage";
/// WP-KERNEL-012 MT-066 (E10 — Stage embed-back): the embed-back leg command id. Dispatch runs the Stage
/// pane's "Capture -> Embed back": it fetches a Stage capture artifact (with SHA-256 manifest provenance)
/// and inserts it into the focused note/canvas as an MT-014 embed NodeView. This EXTENDS the Stage
/// round-trip; the route-to-stage leg stays [`CMD_ROUTE_TO_STAGE`] (NOT duplicated — AC-005/MC-003). The
/// runtime handler is registered by `crate::interop::stage_interop::register_embed_stage_capture_command`;
/// the Stage embed-back backend route is ABSENT, so the embed-back raises the typed blocker
/// `StageInteropError::EmbedBackEndpointAbsent` rather than fabricating an artifact.
pub const CMD_EMBED_STAGE_CAPTURE: &str = "interop.embed-stage-capture";
/// WP-KERNEL-012 MT-034 (E5 — code<->note cross-refs): cross-pane Open-Code-Symbol command id. A
/// clicked `[[code:…]]` chip in a note dispatches this with the target symbol entity id staged via
/// [`InteractionBus::request_open_code_symbol`]; the shell drains
/// [`InteractionBus::take_pending_code_symbol`] each frame and routes it through the MT-030
/// [`crate::quick_switcher::ShellNavigator::open_code_symbol`] seam (which returns a typed
/// `EditorPaneNotMounted` until the code pane mounts at E11/MT-069 — never a faked jump). This is the
/// melt-together note->code navigation primitive, the symmetric counterpart of [`CMD_OPEN_DOCUMENT`].
pub const CMD_OPEN_CODE_SYMBOL: &str = "interop.open-code-symbol";
/// WP-KERNEL-012 MT-068 (E10 — editors<->Locus cross-refs): cross-pane Open-Locus-Ref command id. A
/// clicked Locus chip (`locus://wp/{id}` / `locus://mt/{id}`) in a note/comment dispatches this with the
/// target `locus://` ref staged via [`InteractionBus::request_open_locus_ref`]; the shell drains
/// [`InteractionBus::take_pending_locus_ref`] each frame and routes it through the SAME MT-030 nav seam the
/// other cross-refs use (NavTarget for the WP/MT record). This is the melt-together note->work-unit
/// navigation primitive, the SIBLING of [`CMD_OPEN_CODE_SYMBOL`] (NO new navigation channel — RISK-007).
pub const CMD_OPEN_LOCUS_REF: &str = "interop.open-locus-ref";
/// WP-KERNEL-012 MT-035 (E5 — unified undo): the local-first Undo command id (VS Code Ctrl+Z). Dispatch
/// undoes the most recent action in the FOCUSED pane's ring (POLICY-1), falling back to nothing if that
/// ring is empty. The focused pane id is staged on the bus ([`InteractionBus::focus_owner`]).
pub const CMD_UNDO: &str = "interop.undo";
/// WP-KERNEL-012 MT-035 (E5 — unified undo): the local-first Redo command id (VS Code Ctrl+Y). Redoes
/// the most recently undone action in the focused pane's ring.
pub const CMD_REDO: &str = "interop.redo";
/// WP-KERNEL-012 MT-035 (E5 — unified undo): the cross-pane Undo command id (Ctrl+Shift+Z — POLICY-2).
/// Dispatch undoes the most recent action on the single cross-pane ring (embed-from-atelier,
/// route-to-stage, canvas placement), regardless of which pane is focused.
pub const CMD_UNDO_CROSS_PANE: &str = "interop.undo-cross-pane";

/// One generation of the operator's shared native-editor Find query. The generation advances only when
/// the query text changes, so a consumer can reject a late result produced for an older rapid edit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedFindQuery {
    pub generation: u64,
    pub pattern: String,
}

/// One code-file result produced by the MT-029 workspace search backend. The backend identity is kept
/// intact so another pane/agent can route the entry without reconstructing it from display text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedCodeFindEntry {
    pub source_kind: String,
    pub result_kind: String,
    pub ref_id: String,
    pub block_id: Option<String>,
    pub content_type: String,
    pub title: String,
    pub excerpt: String,
}

/// Typed code side of the shared query. `entries` are authoritative global MT-029 backend rows;
/// `mounted_match_count` is the current local editor scan and is deliberately separate so a local
/// Ctrl+F match can never be misrepresented as a workspace search result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedCodeFindResults {
    pub entries: Vec<SharedCodeFindEntry>,
    pub mounted_match_count: usize,
}

/// One note result produced by the MT-029 workspace search backend. Rich-document and Loom-note rows
/// share this typed projection while retaining their producer ids.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedNoteFindEntry {
    pub source_kind: String,
    pub result_kind: String,
    pub ref_id: String,
    pub block_id: Option<String>,
    pub document_id: Option<String>,
    pub content_type: String,
    pub title: String,
    pub excerpt: String,
}

/// Typed note side of the shared query. Backend entries and the mounted rich-document scan remain
/// distinct evidence rather than being collapsed into an unverifiable count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedNoteFindResults {
    pub entries: Vec<SharedNoteFindEntry>,
    pub mounted_match_count: usize,
}

/// The latest accepted result pair for one shared query. Code and note results remain separate typed
/// fields because they route to different editor surfaces and carry different producer identities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SharedFindResults {
    pub query: SharedFindQuery,
    pub code: SharedCodeFindResults,
    pub note: SharedNoteFindResults,
}

// ── AccessKit author_ids for the cross-pane command surface (the contract's named ids) ───────────────
/// AccessKit author_id for the command-palette trigger button (Role::Button).
pub const COMMAND_PALETTE_TRIGGER_AUTHOR_ID: &str = "command-palette-trigger";
/// AccessKit author_id for the command-palette search input (Role::TextField/TextInput).
pub const COMMAND_PALETTE_SEARCH_AUTHOR_ID: &str = "command-palette-search";
/// AccessKit author_id PREFIX for one command list item: `cmd-{descriptor.id}` (Role::ListItem).
pub const COMMAND_LIST_ITEM_AUTHOR_PREFIX: &str = "cmd-";

/// The stable AccessKit author_id for one command's list item (`cmd-{name}`). `name` is the command's
/// React-`stableId`-equivalent short name ([`CommandDescriptor::name`], e.g. `"Copy"`), used verbatim so
/// the address matches the contract's `cmd-Copy` shape. The short names are authored as safe identifier
/// characters (letters/digits) so no sanitization is needed; an arbitrary external name is still made
/// safe by stripping to `[A-Za-z0-9-]` defensively.
pub fn command_list_item_author_id(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{COMMAND_LIST_ITEM_AUTHOR_PREFIX}{safe}")
}

/// Which pane kind a [`SharedSelection`] / focus belongs to. Distinct from `PaneType` (which is the
/// shell's pane-container vocabulary) because the bus cares only about the four editor surface KINDS
/// that share selection, not every shell pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorSurfaceKind {
    /// The VS-Code-class code editor (`code_editor`).
    Code,
    /// The Obsidian/Notion-class rich-text editor (`rich_editor`).
    RichText,
    /// The Loom knowledge graph (`graph::graph_view`).
    Graph,
    /// The Loom canvas board (`graph::canvas_board`).
    Canvas,
}

/// The one selection model every surface shares. The pane that holds focus owns the active variant;
/// other panes OBSERVE it (e.g. a cross-pane Copy reads whatever the focused pane last published). The
/// `pane_id` ties the selection to a live pane in [`crate::pane_registry`]; consumers MUST guard against
/// a `pane_id` whose pane has been closed (red-team RISK-4 / MC-4) — see
/// [`InteractionBus::shared_selection_if_live`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedSelection {
    /// No active selection on any surface.
    None,
    /// A text range selected in a text surface (code or rich-text). `text` is the materialized selected
    /// string so a cross-pane consumer (clipboard, embed) needs no back-reference into the source buffer.
    TextRange {
        pane_id: PaneId,
        surface: EditorSurfaceKind,
        start: usize,
        end: usize,
        text: String,
    },
    /// A block reference selected in the rich-text surface (a whole block, addressable as `loom://`).
    BlockRef { pane_id: PaneId, block_id: String },
    /// A node reference selected in the graph or canvas surface.
    NodeRef {
        pane_id: PaneId,
        surface: EditorSurfaceKind,
        node_id: String,
    },
}

impl SharedSelection {
    /// The pane that owns this selection (`None` for [`SharedSelection::None`]).
    pub fn pane_id(&self) -> Option<&PaneId> {
        match self {
            SharedSelection::None => None,
            SharedSelection::TextRange { pane_id, .. }
            | SharedSelection::BlockRef { pane_id, .. }
            | SharedSelection::NodeRef { pane_id, .. } => Some(pane_id),
        }
    }

    /// The surface kind that owns this selection, when applicable.
    pub fn surface(&self) -> Option<EditorSurfaceKind> {
        match self {
            SharedSelection::None | SharedSelection::BlockRef { .. } => None,
            SharedSelection::TextRange { surface, .. }
            | SharedSelection::NodeRef { surface, .. } => Some(*surface),
        }
    }

    /// True when there is an actual selection (not [`SharedSelection::None`]).
    pub fn is_some(&self) -> bool {
        !matches!(self, SharedSelection::None)
    }
}

/// One proposal-open request captured by the shared command bus. The selection and emitter are cloned
/// at command-dispatch time so the app cannot accidentally combine a later selection or workspace
/// emitter with the request that the operator/model actually issued.
#[derive(Clone)]
pub struct PendingMemoryProposalRequest {
    pub workspace_id: String,
    pub workspace_generation: u64,
    pub selection: SharedSelection,
    pub emitter: Option<crate::event_emitter::NativeEditorEventEmitter>,
}

/// One admitted Route-to-Stage operation. Content, semantic kind, causal id,
/// and the producer-created receipt identity move together so contention/retry
/// cannot downgrade a Canvas route to Selection or mint duplicate event ids.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingStageRoute {
    pub content: crate::stage_pane::StageContent,
    pub content_kind: String,
    pub causal_action_id: Option<String>,
    pub receipt: crate::event_emitter::NativeEditorEvent,
}

impl PendingStageRoute {
    pub fn new(
        content: crate::stage_pane::StageContent,
        content_kind: impl Into<String>,
        causal_action_id: Option<String>,
        source_pane_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        let content_kind = content_kind.into();
        let source_pane_id = source_pane_id.into();
        let workspace_id = workspace_id.into();
        let receipt = match causal_action_id.as_deref() {
            Some(id) => crate::event_emitter::NativeEditorEvent::route_to_stage_correlated(
                &content_kind,
                &source_pane_id,
                id,
                crate::event_emitter::DEFAULT_ACTOR_ID,
                workspace_id,
            ),
            None => crate::event_emitter::NativeEditorEvent::route_to_stage(
                &content_kind,
                &source_pane_id,
                crate::event_emitter::DEFAULT_ACTOR_ID,
                workspace_id,
            ),
        };
        Self {
            content,
            content_kind,
            causal_action_id,
            receipt,
        }
    }
}

/// One clipboard payload. The bus caches the RICHEST variant in memory so a same-session cross-pane
/// Paste recovers a `LoomBlockRef`/`AtelierRef` the plain-text OS clipboard would have flattened; the
/// OS clipboard always receives the plain-text projection ([`ClipboardPayload::as_plain_text`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardPayload {
    /// Plain UTF-8 text (the universal variant; always also written to the OS clipboard).
    PlainText(String),
    /// A Loom block reference as a `loom://{block_id}` URI (graph/canvas/rich-text block copy).
    LoomBlockRef(String),
    /// An Atelier/CKC artifact reference (`atelier://{ref}`), for CKC media dragged across surfaces.
    AtelierRef(String),
    /// Arbitrary rich content with an explicit MIME type (e.g. an HTML fragment from the rich editor).
    RichContent { mime: String, bytes: Vec<u8> },
}

/// One focused-pane clipboard command staged by the command bus for a mounted pane to consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardCommand {
    /// Copy the focused pane's current selection.
    Copy,
    /// Cut the focused pane's current selection.
    Cut,
    /// Paste the shared clipboard into the focused pane.
    Paste,
}

impl ClipboardPayload {
    /// The plain-text projection written to the OS clipboard. A `LoomBlockRef`/`AtelierRef` projects to
    /// its URI string (so even a plain-text-only consumer gets an addressable reference); `RichContent`
    /// projects to its UTF-8 lossy text. This is what `egui::Context::copy_text` receives.
    pub fn as_plain_text(&self) -> String {
        match self {
            ClipboardPayload::PlainText(s) => s.clone(),
            ClipboardPayload::LoomBlockRef(block_id) => format!("loom://{block_id}"),
            ClipboardPayload::AtelierRef(r) => format!("atelier://{r}"),
            ClipboardPayload::RichContent { bytes, .. } => {
                String::from_utf8_lossy(bytes).into_owned()
            }
        }
    }
}

/// A registered cross-pane command. Ports the React `CommandPaletteAction` shape (id/label/keywords/
/// stableId) plus a keybind and a typed handler. `stable_id` maps to the AccessKit author_id; `keybind`
/// is the egui shortcut the bus checks in [`InteractionBus::matching_keybind_command`].
#[derive(Clone)]
pub struct CommandDescriptor {
    /// Stable command id the bus dispatches on (e.g. [`CMD_COPY`] = `"interop.copy"`).
    pub id: &'static str,
    /// The React-`stableId`-equivalent SHORT name → AccessKit `cmd-{name}` author_id (e.g. `"Copy"`).
    /// Distinct from `id` (the dotted dispatch key) so the addressable list-item id reads as the
    /// contract's `cmd-Copy` while the dispatch id stays a stable namespaced string.
    pub name: &'static str,
    /// Operator/model-facing label (e.g. "Copy").
    pub label: String,
    /// Search keywords folded into the palette filter haystack.
    pub keywords: Vec<String>,
    /// The keyboard shortcut bound to this command (`None` for a palette-only command).
    pub keybind: Option<egui::KeyboardShortcut>,
    /// The handler invoked on dispatch. Receives the egui `Context` (for clipboard / repaint) and the
    /// ALREADY-LOCKED bus, so the handler MUST NOT re-enter the bus lock (red-team RISK-1 / MC-1).
    pub handler: CommandHandler,
}

impl std::fmt::Debug for CommandDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandDescriptor")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("label", &self.label)
            .field("keywords", &self.keywords)
            .field(
                "keybind",
                &self.keybind.map(|k| (k.modifiers, k.logical_key)),
            )
            .field("handler", &"<fn>")
            .finish()
    }
}

/// The handler a registered command runs on dispatch. `Send + Sync` so the bus (held in an `Arc<Mutex>`)
/// stays `Send + Sync` for egui's data store. It receives the egui `Context` and `&mut InteractionBus`
/// (already locked — the handler must NOT re-lock the bus).
pub type CommandHandler = Arc<dyn Fn(&egui::Context, &mut InteractionBus) + Send + Sync>;

/// The cross-pane command registry the bus owns. Holds ONLY the runtime-registered melt-together
/// commands keyed by id (the static [`crate::command_registry`] catalog stays the canonical app-command
/// store — this is the WRAP-not-fork split). Insertion order is preserved for a stable palette listing.
#[derive(Default)]
pub struct CommandBus {
    /// Commands keyed by their stable id (last registration wins, so a pane re-registering on remount
    /// updates the handler rather than duplicating the row).
    by_id: BTreeMap<&'static str, CommandDescriptor>,
    /// Registration order, so the palette lists commands deterministically (BTreeMap alone would sort
    /// alphabetically by id; the React palette preserves registration order).
    order: Vec<&'static str>,
}

impl CommandBus {
    /// Register (or replace) a command descriptor by id.
    pub fn register(&mut self, descriptor: CommandDescriptor) {
        let id = descriptor.id;
        if !self.by_id.contains_key(id) {
            self.order.push(id);
        }
        self.by_id.insert(id, descriptor);
    }

    /// Look up a command by id.
    pub fn get(&self, id: &str) -> Option<&CommandDescriptor> {
        self.by_id.get(id)
    }

    /// Every registered command in registration order (the palette's row order).
    pub fn all(&self) -> Vec<&CommandDescriptor> {
        self.order
            .iter()
            .filter_map(|id| self.by_id.get(id))
            .collect()
    }

    /// How many commands are registered.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// True when no command is registered.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// The one shared interaction substrate. Held in egui app data as `Arc<Mutex<InteractionBus>>` and
/// retrieved by every pane via [`Self::get_or_init`].
pub struct InteractionBus {
    /// Workspace identity for all ephemeral focus/selection state carried by this bus. Rebinding bumps
    /// `workspace_generation` and clears the old workspace's selection before any consumer can reuse it.
    workspace_id: String,
    workspace_generation: u64,
    /// The single shared selection model (the focused pane owns the active variant).
    selection: SharedSelection,
    /// The pane that currently owns focus (so a selection publish from a non-focused pane is ignored —
    /// the focused pane is the selection authority).
    focus_owner: Option<PaneId>,
    /// True when the focused pane's active surface owns only cross-pane history (currently the
    /// backend-authoritative Canvas/Atelier surface). This prevents a local ring left by a previous tab
    /// in the same physical pane from stealing Canvas undo/redo.
    focus_owner_cross_only: bool,
    /// The cross-pane command registry (WRAP-not-fork: runtime melt-together commands only).
    commands: CommandBus,
    /// Whether the product-owned shared Find lifecycle is active. This is fixed bus state rather than a
    /// list of surface callbacks, so repeated pane mounts/command registration cannot duplicate fan-out.
    shared_find_active: bool,
    /// Monotonic count of actual canonical `CMD_FIND` dispatches (the CTRL-1 observable).
    shared_find_dispatch_generation: u64,
    /// Latest shared query and its accepted typed code/note result pair.
    shared_find_query: SharedFindQuery,
    shared_find_results: SharedFindResults,
    /// The in-memory richest-variant clipboard cache: the cross-pane Paste reads THIS first so a
    /// `LoomBlockRef`/`AtelierRef` survives a round-trip the plain-text OS clipboard would flatten.
    clipboard_cache: Option<ClipboardPayload>,
    /// A one-shot clipboard command request staged by a generic bus command. The focused pane drains it
    /// and performs the buffer-specific edit, so `CMD_PASTE` is no longer a read-only cache touch.
    pending_clipboard_command: Option<(PaneId, ClipboardCommand)>,
    /// Whether the command palette modal is open. Drives the EXISTING `command_palette.rs` modal
    /// (WRAP-not-fork: the bus owns the open FLAG; the modal renders it). The shell reads this.
    command_palette_open: bool,
    /// The existing shell event bus sender (cross-pane fan-out). `None` until the shell installs it via
    /// [`Self::set_event_sender`]; the bus never invents a second event system.
    event_sender: Option<ShellEventSender>,
    /// WP-KERNEL-012 MT-032 (E5): the document id a cross-pane Open-Document request staged (from a
    /// backlink row click / loom:// reference). The shell drains it via [`Self::take_pending_navigation`]
    /// each frame and routes the open. `None` when no navigation is pending.
    pending_navigation: Option<String>,
    /// MT-067: exact CalendarEvent id staged by the daily-journal chip and consumed once by the shell.
    pending_calendar_event_focus: Option<String>,
    /// WP-KERNEL-012 MT-033 (E5): the content a Route-to-Stage request staged (from a selection / canvas
    /// node / CKC item). The shell peeks it via [`Self::pending_stage_route`], applies it to Stage, then
    /// acknowledges its exact event id via [`Self::ack_pending_stage_route`].
    pending_stage_route: Option<PendingStageRoute>,
    /// One-shot visible failure for Route-to-Stage. Kept distinct from content so a command with no
    /// valid selection/document cannot silently open an empty Stage or overwrite prior routed content.
    pending_stage_error: Option<String>,
    /// WP-KERNEL-012 MT-034 (E5): the symbol entity id an Open-Code-Symbol request staged (from a
    /// clicked `[[code:…]]` chip). The shell drains it via [`Self::take_pending_code_symbol`] each frame
    /// and routes it through the MT-030 ShellNavigator `open_code_symbol` seam. `None` when nothing is
    /// pending.
    pending_code_symbol: Option<String>,
    /// WP-KERNEL-012 MT-068 (E10): the `locus://` ref an Open-Locus-Ref request staged (from a clicked
    /// Locus chip). The shell drains it via [`Self::take_pending_locus_ref`] each frame and routes it
    /// through the SAME MT-030 nav seam (NavTarget for the WP/MT record). `None` when nothing is pending.
    /// SIBLING of [`Self::pending_code_symbol`] — no new navigation channel (RISK-007).
    pending_locus_ref: Option<String>,
    /// One-shot `fems.propose_to_memory` request staged by the real shared command handler and drained by
    /// the mounted app. This is intentionally not app state: MCP/palette/bus callers all enter through
    /// the same command substrate.
    pending_memory_proposal_request: Option<PendingMemoryProposalRequest>,
    /// WP-KERNEL-012 MT-035 (E5): the ONE unified undo scope every pane shares (POLICY-1..5) within one
    /// continuous workspace binding. It is discarded on workspace rebind and remains in-memory only —
    /// the bus is held in egui app data which is NOT persisted, so the scope is empty on restart
    /// (POLICY-3). The scope itself cannot serialize (no `Serialize` impl).
    undo_scope: UnifiedUndoScope,
    /// The app's tokio runtime handle, installed by the shell via [`Self::set_undo_runtime`] so the bus
    /// can dispatch a canvas COMPENSATING undo (POLICY-4 `undo_async_fn`) onto the runtime off the egui
    /// frame thread (HBR-QUIET). `None` in a headless unit test (an async undo is then reported as a
    /// typed "no runtime" result rather than blocking the frame — never a fake success).
    undo_runtime: Option<tokio::runtime::Handle>,
    /// WP-KERNEL-012 MT-036 (E5 — one event ledger): the single native-editor event emitter, co-located
    /// with the shared bus so every surface emits through ONE producer to ONE ledger. `None` until the
    /// shell installs it via [`Self::set_event_emitter`]; a pane calling [`Self::emit_event`] before the
    /// shell installs it is an honest no-op (never a fake emit), matching the unmounted-pane defer policy.
    event_emitter: Option<crate::event_emitter::NativeEditorEventEmitter>,
    /// Workspace emitters retained for the app session so a completion captured in workspace A can
    /// drain after any number of later workspace switches without being relabeled or losing A's trace.
    /// App teardown drops the map and closes every worker; every generation shares one error ring.
    event_emitters_by_workspace: BTreeMap<String, crate::event_emitter::NativeEditorEventEmitter>,
    event_emitter_workspace_order: VecDeque<String>,
    /// WP-KERNEL-012 MT-036 (E5 — designed extension seams): the DESIGN-ONLY future-surface registry,
    /// co-located with the bus per the contract. EMPTY in production (no future surface registers yet),
    /// so the fan-out on selection-change / emit is a no-op until an image-editor/spreadsheet/engine
    /// surface registers. Stored here so the seam attaches to the SAME substrate without touching the
    /// emitter.
    surface_registry: crate::surface_extension_seam::EditorSurfaceRegistry,
}

impl Default for InteractionBus {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionBus {
    /// A fresh bus with no selection, no focus owner, an empty command registry, and an empty clipboard
    /// cache.
    pub fn new() -> Self {
        Self {
            workspace_id: String::new(),
            workspace_generation: 0,
            selection: SharedSelection::None,
            focus_owner: None,
            focus_owner_cross_only: false,
            commands: CommandBus::default(),
            shared_find_active: false,
            shared_find_dispatch_generation: 0,
            shared_find_query: SharedFindQuery::default(),
            shared_find_results: SharedFindResults::default(),
            clipboard_cache: None,
            pending_clipboard_command: None,
            command_palette_open: false,
            event_sender: None,
            pending_navigation: None,
            pending_calendar_event_focus: None,
            pending_stage_route: None,
            pending_stage_error: None,
            pending_code_symbol: None,
            pending_locus_ref: None,
            pending_memory_proposal_request: None,
            undo_scope: UnifiedUndoScope::new(),
            undo_runtime: None,
            event_emitter: None,
            event_emitters_by_workspace: BTreeMap::new(),
            event_emitter_workspace_order: VecDeque::new(),
            surface_registry: crate::surface_extension_seam::EditorSurfaceRegistry::new(),
        }
    }

    /// Retrieve the shared bus from egui app data, inserting a fresh one on first access. Every pane
    /// calls this in `update()` so all panes share the SAME `Arc<Mutex<InteractionBus>>` (the contract's
    /// `ctx.data_mut(...).insert_temp(Id::new("handshake_interaction_bus"), bus.clone())` pattern).
    pub fn get_or_init(ctx: &egui::Context) -> Arc<Mutex<InteractionBus>> {
        let id = interaction_bus_id();
        ctx.data_mut(|d| {
            if let Some(existing) = d.get_temp::<Arc<Mutex<InteractionBus>>>(id) {
                existing
            } else {
                let bus = Arc::new(Mutex::new(InteractionBus::new()));
                d.insert_temp(id, bus.clone());
                bus
            }
        })
    }

    /// Run `f` against the shared bus with a NON-BLOCKING `try_lock` (red-team RISK-1 / MC-1): if another
    /// pane holds the lock this frame, `f` is skipped and `None` is returned rather than blocking the
    /// egui frame thread. Use this from per-frame `update()` paths. Returns `Some(f's result)` on
    /// acquisition.
    pub fn with_try_lock<R>(
        bus: &Arc<Mutex<InteractionBus>>,
        f: impl FnOnce(&mut InteractionBus) -> R,
    ) -> Option<R> {
        match bus.try_lock() {
            Ok(mut guard) => Some(f(&mut guard)),
            Err(std::sync::TryLockError::WouldBlock) => {
                tracing::debug!(
                    "InteractionBus: try_lock contention this frame; skipping (no deadlock)"
                );
                None
            }
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                // A panicked handler poisoned the lock; recover the guard rather than propagate the
                // poison (the bus state is plain data — a poisoned lock here would wedge every pane).
                Some(f(&mut poisoned.into_inner()))
            }
        }
    }

    /// Consume one live egui shortcut and dispatch its registered bus command.
    ///
    /// The bus lock is acquired before consuming input. If another pane owns
    /// the bus this frame, the shortcut is left untouched instead of being
    /// swallowed without an action. The native shell uses this for global
    /// commands such as Ctrl/Cmd+Shift+Z that focused editors deliberately do
    /// not own.
    pub fn dispatch_registered_shortcut_from_input(
        ctx: &egui::Context,
        bus: &Arc<Mutex<InteractionBus>>,
        command_id: &'static str,
    ) -> Option<bool> {
        Self::with_try_lock(bus, |bus| {
            if matches!(command_id, CMD_UNDO | CMD_REDO | CMD_UNDO_CROSS_PANE) {
                bus.register_undo_commands();
            }
            let Some(shortcut) = default_keybind_for(command_id) else {
                return false;
            };
            if bus.matching_keybind_command(&shortcut) != Some(command_id) {
                return false;
            }
            if !ctx.input_mut(|input| input.consume_shortcut(&shortcut)) {
                return false;
            }
            bus.dispatch_command(ctx, command_id)
        })
    }

    /// Install the existing shell event-bus sender so cross-pane notifications fan out through the SAME
    /// `event_bus.rs` channel (WRAP-not-fork).
    pub fn set_event_sender(&mut self, sender: ShellEventSender) {
        self.event_sender = Some(sender);
    }

    // ── Focus ownership ──────────────────────────────────────────────────────────────────────────────

    /// Mark `pane_id` as the editor focus owner (called by a pane only when it genuinely holds egui
    /// focus — `ui.memory(|m| m.has_focus(pane_egui_id))` — to avoid spurious resets, impl note 6/7).
    /// Moving editor focus to a different pane invalidates both the previous pane's live selection and
    /// any proposal request that already snapshotted it. The request is retained with a `None` selection
    /// so the mounted app reports the typed `no_selection` blocker instead of silently dropping the
    /// command or submitting stale cross-pane provenance.
    pub fn set_focus_owner(&mut self, pane_id: PaneId) {
        if self
            .selection
            .pane_id()
            .is_some_and(|selection_pane| selection_pane != &pane_id)
        {
            self.selection = SharedSelection::None;
            self.surface_registry
                .dispatch_selection_changed(&self.selection);
        }
        if let Some(request) = self.pending_memory_proposal_request.as_mut() {
            if request
                .selection
                .pane_id()
                .is_some_and(|selection_pane| selection_pane != &pane_id)
            {
                request.selection = SharedSelection::None;
            }
        }
        self.focus_owner = Some(pane_id);
        self.focus_owner_cross_only = false;
    }

    /// Publish the shell-resolved undo target, including whether its active surface is cross-only.
    /// This does not invalidate selection: utility/modal surfaces and menu-driven undo temporarily route
    /// keyboard ownership without transferring editor selection authority.
    pub fn set_undo_focus_owner(&mut self, pane_id: PaneId, cross_only: bool) {
        self.focus_owner = Some(pane_id);
        self.focus_owner_cross_only = cross_only;
    }

    /// The current focus owner pane id, if any.
    pub fn focus_owner(&self) -> Option<&PaneId> {
        self.focus_owner.as_ref()
    }

    pub fn focus_owner_is_cross_only(&self) -> bool {
        self.focus_owner_cross_only
    }

    // ── Shared selection ─────────────────────────────────────────────────────────────────────────────

    /// Bind ephemeral interaction state to `workspace_id`. A changed workspace invalidates every
    /// workspace-bound, not-yet-drained request plus the in-memory clipboard and undo scopes from the
    /// leaving workspace. History is deliberately discarded rather than restored on A -> B -> A: the
    /// existing rings and staged payloads are keyed only by pane, not by workspace, so retaining them
    /// would allow an action captured in A to execute against the live pane state in B. Returns `true`
    /// when a rebind occurred.
    pub fn bind_workspace(&mut self, workspace_id: &str) -> bool {
        if self.workspace_id == workspace_id {
            return false;
        }
        self.workspace_id = workspace_id.to_owned();
        self.workspace_generation = self.workspace_generation.wrapping_add(1);
        self.selection = SharedSelection::None;
        self.focus_owner = None;
        self.focus_owner_cross_only = false;
        self.clipboard_cache = None;
        self.pending_clipboard_command = None;
        self.pending_navigation = None;
        self.pending_calendar_event_focus = None;
        self.pending_stage_route = None;
        self.pending_stage_error = None;
        self.pending_code_symbol = None;
        self.pending_locus_ref = None;
        self.pending_memory_proposal_request = None;
        self.shared_find_active = false;
        self.shared_find_dispatch_generation = 0;
        self.shared_find_query = SharedFindQuery::default();
        self.shared_find_results = SharedFindResults::default();
        // Pane ids are reused by the shell across workspace mounts, while undo closures capture the
        // state that was live when the action was recorded. Reset the whole scope so neither ordinary
        // history nor a provisional async compensation can cross the workspace boundary. A late
        // completion for a discarded provisional action is then rejected by
        // `complete_cross_pane_async` instead of repopulating the new workspace's ring.
        self.undo_scope = UnifiedUndoScope::new();
        // Never leave the previous workspace's emitter as the active capture source. A retained emitter
        // for a returning workspace can be reused; a first visit stays unbound until the shell installs
        // that workspace's emitter later in the same frame.
        self.event_emitter = self.event_emitters_by_workspace.get(workspace_id).cloned();
        self.surface_registry
            .dispatch_selection_changed(&self.selection);
        true
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn workspace_generation(&self) -> u64 {
        self.workspace_generation
    }

    /// Publish a new shared selection. Accepted only when the publishing pane is the current focus owner
    /// (or no focus owner is set yet), so a background pane cannot clobber the focused pane's selection.
    /// Returns `true` when the selection was accepted.
    pub fn set_selection(&mut self, selection: SharedSelection) -> bool {
        let publisher = selection.pane_id().cloned();
        let accept = match (&self.focus_owner, &publisher) {
            // A clear (`None`) is always accepted.
            (_, None) => true,
            // No focus owner yet: accept and adopt the publisher as the owner.
            (None, Some(p)) => {
                self.focus_owner = Some(p.clone());
                true
            }
            // Owner set: accept only from the owner.
            (Some(owner), Some(p)) => owner == p,
        };
        if accept {
            self.selection = selection;
            // MT-036 (E5 designed seam): fan the new selection out to every registered future surface
            // (a no-op in production — the registry is empty until an image-editor/spreadsheet/engine
            // surface registers). Kept INSIDE the accept branch so a rejected selection does not notify.
            self.surface_registry
                .dispatch_selection_changed(&self.selection);
        }
        accept
    }

    /// Invalidate text selection state captured from `pane_id` after that pane switches to a
    /// different editor document. A proposal command can be staged between the tab activation frame
    /// and the next shell frame, so the already-captured request must be invalidated together with the
    /// live selection. The request itself is retained with [`SharedSelection::None`] so the mounted app
    /// still reports its typed `no_selection` blocker instead of silently dropping the command.
    pub fn invalidate_selection_for_pane(&mut self, pane_id: &PaneId) -> bool {
        let mut invalidated = false;
        if self.selection.pane_id() == Some(pane_id) {
            self.selection = SharedSelection::None;
            self.surface_registry
                .dispatch_selection_changed(&self.selection);
            invalidated = true;
        }
        if let Some(request) = self.pending_memory_proposal_request.as_mut() {
            if request.selection.pane_id() == Some(pane_id) {
                request.selection = SharedSelection::None;
                invalidated = true;
            }
        }
        invalidated
    }

    /// The raw shared selection (without a liveness guard — prefer [`Self::shared_selection_if_live`]
    /// for a consumer that will dereference the pane).
    pub fn shared_selection(&self) -> &SharedSelection {
        &self.selection
    }

    /// The shared selection ONLY if its owning `pane_id` is still present in `live_pane_ids` (red-team
    /// RISK-4 / MC-4 — a selection referencing a closed pane is dangling and must not be used). Returns
    /// [`SharedSelection::None`] (owned) when the selection is `None` or its pane is gone.
    pub fn shared_selection_if_live(&self, live_pane_ids: &[PaneId]) -> SharedSelection {
        match self.selection.pane_id() {
            None => SharedSelection::None,
            Some(pane_id) if live_pane_ids.iter().any(|p| p == pane_id) => self.selection.clone(),
            Some(_) => SharedSelection::None,
        }
    }

    /// Capture a proposal-open request from the current workspace-bound interaction state. Even a
    /// `SharedSelection::None` request is staged so the mounted app can surface the typed no-selection
    /// blocker instead of silently dropping an MCP/shared-bus command.
    pub fn request_memory_proposal(&mut self) {
        self.pending_memory_proposal_request = Some(PendingMemoryProposalRequest {
            workspace_id: self.workspace_id.clone(),
            workspace_generation: self.workspace_generation,
            selection: self.selection.clone(),
            emitter: self.event_emitter.clone(),
        });
    }

    pub fn take_pending_memory_proposal_request(&mut self) -> Option<PendingMemoryProposalRequest> {
        self.pending_memory_proposal_request.take()
    }

    // ── Clipboard ────────────────────────────────────────────────────────────────────────────────────

    /// Write `payload` to the clipboard: caches the RICHEST variant in memory (so a same-session
    /// cross-pane Paste recovers a `LoomBlockRef`/`AtelierRef`) AND writes the plain-text projection to
    /// the OS clipboard through the mockable [`ClipboardSink`] (red-team RISK-2 / MC-2: a headless test
    /// injects an in-memory mock so the OS clipboard is never touched).
    pub fn clipboard_write(&mut self, payload: ClipboardPayload, sink: &dyn ClipboardSink) {
        sink.copy(&payload.as_plain_text());
        self.clipboard_cache = Some(payload);
    }

    /// Cache `payload` as the richest cross-pane clipboard variant WITHOUT writing the OS clipboard. This
    /// is the dispatch-by-id Copy/Cut path (a registered command handler has no [`ClipboardSink`] in its
    /// signature, so it populates the in-memory cross-pane channel only). The pane's DIRECT Ctrl+C path
    /// uses [`Self::clipboard_write`] (cache + OS write through the mockable sink). Keeping the two paths
    /// distinct avoids forcing every command handler to thread a sink it cannot reach.
    pub fn cache_clipboard(&mut self, payload: ClipboardPayload) {
        self.clipboard_cache = Some(payload);
    }

    /// Read the richest clipboard variant available for cross-pane Paste: the in-memory cache (which
    /// preserves the rich variant) when present, else `None`. A consumer that needs the OS clipboard's
    /// plain text reads it through egui directly; the in-memory cache is the cross-pane rich channel.
    pub fn clipboard_read(&self) -> Option<&ClipboardPayload> {
        self.clipboard_cache.as_ref()
    }

    /// The richest clipboard variant as plain text, when present (the cross-pane Paste convenience used
    /// by a text surface that consumes only `PlainText`).
    pub fn clipboard_read_text(&self) -> Option<String> {
        self.clipboard_cache.as_ref().map(|p| p.as_plain_text())
    }

    /// Stage a one-shot clipboard command for the current focus owner. Returns `false` when no pane owns
    /// focus, so a generic command can request repaint without fabricating a target.
    pub fn request_clipboard_command(&mut self, command: ClipboardCommand) -> bool {
        let Some(pane_id) = self.focus_owner.clone() else {
            return false;
        };
        self.pending_clipboard_command = Some((pane_id, command));
        true
    }

    /// Drain a staged clipboard command only for its owning focused pane. A different pane leaves the
    /// request intact so a render-order race cannot hand Paste to the wrong surface.
    pub fn take_clipboard_command_for(&mut self, pane_id: &PaneId) -> Option<ClipboardCommand> {
        match self.pending_clipboard_command.as_ref() {
            Some((target, _)) if target == pane_id => self
                .pending_clipboard_command
                .take()
                .map(|(_, command)| command),
            _ => None,
        }
    }

    // ── Command bus (WRAP the registry) ──────────────────────────────────────────────────────────────

    /// Register a cross-pane command. Panes call this once at construction to publish their melt-together
    /// commands (Copy/Cut/Paste/SelectAll/Find/CommandPalette) into the one shared surface.
    pub fn register_command(&mut self, descriptor: CommandDescriptor) {
        self.commands.register(descriptor);
    }

    /// Start/refocus the product-owned shared Find lifecycle. Called only by the canonical `CMD_FIND`
    /// handler, making this generation the exact count of real command dispatches.
    pub fn request_shared_find(&mut self) {
        self.shared_find_active = true;
        self.shared_find_dispatch_generation = self.shared_find_dispatch_generation.wrapping_add(1);
    }

    pub fn shared_find_is_active(&self) -> bool {
        self.shared_find_active
    }

    pub fn shared_find_dispatch_generation(&self) -> u64 {
        self.shared_find_dispatch_generation
    }

    /// Publish the newest operator query. Re-publishing the same text is a deduplicated no-op; a rapid
    /// replacement advances once for each text actually observed by the product driver.
    pub fn update_shared_find_query(&mut self, pattern: impl Into<String>) -> SharedFindQuery {
        let pattern = pattern.into();
        if self.shared_find_query.pattern != pattern {
            self.shared_find_query.generation = self.shared_find_query.generation.wrapping_add(1);
            self.shared_find_query.pattern = pattern;
            self.shared_find_results = SharedFindResults {
                query: self.shared_find_query.clone(),
                ..SharedFindResults::default()
            };
        }
        self.shared_find_query.clone()
    }

    pub fn shared_find_query(&self) -> &SharedFindQuery {
        &self.shared_find_query
    }

    /// Accept typed results only for the current query generation. This prevents a late MT-029 backend
    /// completion from overwriting a rapid replacement or workspace rebind.
    pub fn publish_shared_find_results(
        &mut self,
        query: &SharedFindQuery,
        code: SharedCodeFindResults,
        note: SharedNoteFindResults,
    ) -> bool {
        if query != &self.shared_find_query || !self.shared_find_active {
            return false;
        }
        self.shared_find_results = SharedFindResults {
            query: query.clone(),
            code,
            note,
        };
        true
    }

    pub fn shared_find_results(&self) -> &SharedFindResults {
        &self.shared_find_results
    }

    /// Close the shared lifecycle after the active surface closes its find UI (Escape). The last query is
    /// retained for a refocus, but no stale hits remain exposed while Find is closed.
    pub fn close_shared_find(&mut self) {
        self.shared_find_active = false;
        self.shared_find_results = SharedFindResults {
            query: self.shared_find_query.clone(),
            ..SharedFindResults::default()
        };
    }

    /// Borrow the command registry (for the palette listing / tests).
    pub fn commands(&self) -> &CommandBus {
        &self.commands
    }

    /// Dispatch a registered command by id: looks up the handler and runs it with the locked bus. The
    /// handler is cloned out FIRST (so the borrow on `self.commands` ends before the handler runs with
    /// `&mut self`, avoiding a double-borrow), then invoked. Returns `true` when a command was found and
    /// dispatched, `false` for an unknown id (a bad id is a no-op, never a panic).
    pub fn dispatch_command(&mut self, ctx: &egui::Context, id: &str) -> bool {
        let Some(handler) = self.commands.get(id).map(|d| d.handler.clone()) else {
            return false;
        };
        handler(ctx, self);
        true
    }

    /// The id of the FIRST registered command whose keybind matches `shortcut`, if any. The keybind
    /// dispatcher uses this AFTER the pane has consumed the shortcut from egui's input (red-team RISK-3 /
    /// MC-3: the pane calls `ui.input_mut(|i| i.consume_shortcut(&shortcut))` first to suppress egui's
    /// default text-widget copy, THEN dispatches the resolved command).
    pub fn matching_keybind_command(
        &self,
        shortcut: &egui::KeyboardShortcut,
    ) -> Option<&'static str> {
        self.commands
            .all()
            .into_iter()
            .find(|d| d.keybind.as_ref() == Some(shortcut))
            .map(|d| d.id)
    }

    // ── Command palette open state (WRAP the modal) ──────────────────────────────────────────────────

    /// Whether the command palette modal is open (the EXISTING `command_palette.rs` modal reads this).
    pub fn command_palette_open(&self) -> bool {
        self.command_palette_open
    }

    /// Open the command palette modal (sets the flag the existing modal renders).
    pub fn open_command_palette(&mut self) {
        self.command_palette_open = true;
    }

    /// Close the command palette modal.
    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
    }

    // ── Cross-pane Open-Document navigation (MT-032 melt-together) ─────────────────────────────────────

    /// Stage a document id for a cross-pane open (called just before dispatching [`CMD_OPEN_DOCUMENT`],
    /// e.g. from a backlink row click). The shell drains it next frame via [`Self::take_pending_navigation`].
    pub fn request_open_document(&mut self, document_id: impl Into<String>) {
        self.pending_navigation = Some(document_id.into());
    }

    /// The document id staged for a cross-pane open, WITHOUT consuming it (tests / peek).
    pub fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    /// Take (and clear) the staged document id. The shell calls this each frame; `Some(id)` means route
    /// an open to that document, `None` means nothing pending.
    pub fn take_pending_navigation(&mut self) -> Option<String> {
        self.pending_navigation.take()
    }

    /// Register the cross-pane Open-Document command (MT-032). Its handler is a no-op on the bus itself
    /// (the navigation target was staged by [`Self::request_open_document`] BEFORE dispatch, and is
    /// consumed by the shell drain) — the command exists so a backlink row / loom:// ref dispatches a
    /// REAL, named, addressable cross-pane action rather than a per-pane ad-hoc callback. Idempotent
    /// (last registration wins). Returns nothing; call once per surface that can open documents.
    pub fn register_open_document_command(&mut self) {
        // The handler requests a repaint so the staged navigation is drained on the next frame; the
        // document id itself was staged by `request_open_document` before dispatch (a generic handler
        // signature carries no payload — the stage-then-dispatch split is the contract point).
        self.register_command(CommandDescriptor {
            id: CMD_OPEN_DOCUMENT,
            name: "OpenDocument",
            label: "Open Document".to_owned(),
            keywords: vec![
                "open".to_owned(),
                "document".to_owned(),
                "backlink".to_owned(),
            ],
            keybind: None,
            handler: Arc::new(|ctx, _bus| ctx.request_repaint()),
        });
    }

    /// Stage `document_id` and dispatch [`CMD_OPEN_DOCUMENT`] in one call (the backlink-row /
    /// loom://-reference open path — AC-4). Returns `true` when the command was found and dispatched
    /// (it always is once [`Self::register_open_document_command`] ran). The staged id is then readable
    /// via [`Self::pending_navigation`] until the shell drains it.
    pub fn open_document(&mut self, ctx: &egui::Context, document_id: impl Into<String>) -> bool {
        self.request_open_document(document_id);
        self.dispatch_command(ctx, CMD_OPEN_DOCUMENT)
    }

    // ── CalendarEvent destination navigation (MT-067) ──────────────────────────────────────────────

    /// Stage the exact CalendarEvent id for the shell's content-addressed destination.
    pub fn request_focus_calendar_event(&mut self, event_id: impl Into<String>) {
        self.pending_calendar_event_focus = Some(event_id.into());
    }

    /// Peek at the staged CalendarEvent id without consuming it.
    pub fn pending_calendar_event_focus(&self) -> Option<&str> {
        self.pending_calendar_event_focus.as_deref()
    }

    /// Consume the staged CalendarEvent id exactly once.
    pub fn take_pending_calendar_event_focus(&mut self) -> Option<String> {
        self.pending_calendar_event_focus.take()
    }

    /// Register the named CalendarEvent focus command on the shared command substrate.
    pub fn register_focus_calendar_event_command(&mut self) {
        self.register_command(CommandDescriptor {
            id: crate::interop::calendar_interop::CMD_FOCUS_CALENDAR_EVENT,
            name: "FocusCalendarEvent",
            label: "Focus Calendar Event".to_owned(),
            keywords: vec![
                "calendar".to_owned(),
                "event".to_owned(),
                "focus".to_owned(),
            ],
            keybind: None,
            handler: Arc::new(|ctx, _bus| ctx.request_repaint()),
        });
    }

    /// Stage `event_id` and dispatch the named CalendarEvent focus command.
    pub fn focus_calendar_event(
        &mut self,
        ctx: &egui::Context,
        event_id: impl Into<String>,
    ) -> bool {
        self.request_focus_calendar_event(event_id);
        self.dispatch_command(
            ctx,
            crate::interop::calendar_interop::CMD_FOCUS_CALENDAR_EVENT,
        )
    }

    // ── Cross-pane Route-to-Stage navigation (MT-033 melt-together) ────────────────────────────────────

    fn build_pending_stage_route(
        &self,
        content: crate::stage_pane::StageContent,
        content_kind: &str,
        causal_action_id: Option<&str>,
    ) -> PendingStageRoute {
        let source_pane_id = self
            .focus_owner
            .as_ref()
            .map(|pane| pane.as_ref().to_owned())
            .unwrap_or_else(|| "stage-route-source".to_owned());
        let workspace_id = self
            .event_emitter
            .as_ref()
            .map(|emitter| emitter.workspace_id().to_owned())
            .unwrap_or_else(|| self.workspace_id.clone());
        PendingStageRoute::new(
            content,
            content_kind,
            causal_action_id.map(str::to_owned),
            source_pane_id,
            workspace_id,
        )
    }

    /// Stage a typed Route-to-Stage failure for the shell to render on the Stage pane.
    pub fn request_route_to_stage_error(&mut self, message: impl Into<String>) {
        if self.pending_stage_route.is_none() {
            self.pending_stage_error = Some(message.into());
        }
    }

    /// The content staged for a Route-to-Stage open, WITHOUT consuming it (tests / peek).
    pub fn pending_stage_content(&self) -> Option<&crate::stage_pane::StageContent> {
        self.pending_stage_route
            .as_ref()
            .map(|route| &route.content)
    }

    /// Peek the complete pending route without consuming it. The shell must release the bus lock before
    /// acquiring the Stage lock, apply the route, and only then call [`Self::ack_pending_stage_route`].
    pub fn pending_stage_route(&self) -> Option<&PendingStageRoute> {
        self.pending_stage_route.as_ref()
    }

    /// Acknowledge and consume only the route whose prebuilt receipt has `event_id`. A stale or
    /// mismatched acknowledgement cannot remove a newer request.
    pub fn ack_pending_stage_route(&mut self, event_id: &str) -> Option<PendingStageRoute> {
        let matches = self
            .pending_stage_route
            .as_ref()
            .is_some_and(|route| route.receipt.event_id == event_id);
        matches.then(|| self.pending_stage_route.take()).flatten()
    }

    /// Take the pending typed Route-to-Stage failure.
    pub fn take_pending_stage_error(&mut self) -> Option<String> {
        self.pending_stage_error.take()
    }

    /// Register the cross-pane Route-to-Stage command (MT-033). Its handler is a no-op on the bus itself
    /// (the complete pending route is admitted after successful dispatch and consumed by the shell
    /// drain) — the command exists so a "Route to Stage" menu item dispatches a REAL, named,
    /// addressable cross-pane action rather than a per-pane ad-hoc callback. Idempotent (last
    /// registration wins). Mirrors [`Self::register_open_document_command`] exactly (the MT-032 pattern).
    pub fn register_route_to_stage_command(&mut self) {
        self.register_command(CommandDescriptor {
            id: CMD_ROUTE_TO_STAGE,
            name: "RouteToStage",
            label: "Route to Stage".to_owned(),
            keywords: vec!["route".to_owned(), "stage".to_owned(), "send".to_owned()],
            keybind: None,
            // The content id was staged before dispatch (a generic handler carries no payload — the
            // stage-then-dispatch split is the contract point); request a repaint so the shell drains it.
            handler: Arc::new(|ctx, _bus| ctx.request_repaint()),
        });
    }

    /// Stage `content` and dispatch [`CMD_ROUTE_TO_STAGE`] in one call (the "Route to Stage" menu path —
    /// AC-4). Returns `true` when the command was found and dispatched (it always is once
    /// [`Self::register_route_to_stage_command`] ran). The staged content is then readable via
    /// [`Self::pending_stage_content`] until the shell drains it.
    pub fn route_to_stage(
        &mut self,
        ctx: &egui::Context,
        content: crate::stage_pane::StageContent,
    ) -> bool {
        self.route_to_stage_correlated(ctx, content, None)
    }

    pub fn route_to_stage_correlated(
        &mut self,
        ctx: &egui::Context,
        content: crate::stage_pane::StageContent,
        causal_action_id: Option<&str>,
    ) -> bool {
        let content_kind = content.content_kind();
        self.route_to_stage_correlated_with_kind(ctx, content, content_kind, causal_action_id)
    }

    /// Route Stage content while preserving the source payload's exact recorder kind. Most callers use
    /// [`Self::route_to_stage_correlated`]; typed adapters such as Canvas node routing use this method
    /// because the display projection is a [`crate::stage_pane::StageContent::Selection`] while the
    /// authoritative route kind remains `canvas_node`.
    pub fn route_to_stage_correlated_with_kind(
        &mut self,
        ctx: &egui::Context,
        content: crate::stage_pane::StageContent,
        content_kind: &str,
        causal_action_id: Option<&str>,
    ) -> bool {
        if self.pending_stage_route.is_some() {
            return false;
        }
        let route = self.build_pending_stage_route(content, content_kind, causal_action_id);
        if !self.dispatch_command(ctx, CMD_ROUTE_TO_STAGE) {
            return false;
        }
        self.pending_stage_error = None;
        self.pending_stage_route = Some(route);
        true
    }

    /// Re-admit the exact retained route after shell contention. Producer event
    /// identity and the semantic content kind are reused byte-for-byte.
    pub fn retry_pending_stage_route(
        &mut self,
        ctx: &egui::Context,
        route: PendingStageRoute,
    ) -> bool {
        if self.pending_stage_route.is_some() || !self.dispatch_command(ctx, CMD_ROUTE_TO_STAGE) {
            return false;
        }
        self.pending_stage_error = None;
        self.pending_stage_route = Some(route);
        true
    }

    /// Stage a typed failure and dispatch the same named command as a successful route. This keeps
    /// context-menu and palette failures visible on the normal Stage landing surface.
    pub fn route_to_stage_error(
        &mut self,
        ctx: &egui::Context,
        message: impl Into<String>,
    ) -> bool {
        self.request_route_to_stage_error(message);
        self.dispatch_command(ctx, CMD_ROUTE_TO_STAGE)
    }

    // ── Cross-pane Open-Code-Symbol navigation (MT-034 code<->note cross-refs) ──────────────────────────

    /// Stage a symbol entity id for a cross-pane Open-Code-Symbol (called just before dispatching
    /// [`CMD_OPEN_CODE_SYMBOL`], e.g. from a clicked `[[code:…]]` chip). The shell drains it next frame
    /// via [`Self::take_pending_code_symbol`] and routes it through the MT-030 ShellNavigator.
    pub fn request_open_code_symbol(&mut self, symbol_entity_id: impl Into<String>) {
        self.pending_code_symbol = Some(symbol_entity_id.into());
    }

    /// The symbol entity id staged for a cross-pane code-symbol open, WITHOUT consuming it (tests / peek).
    pub fn pending_code_symbol(&self) -> Option<&str> {
        self.pending_code_symbol.as_deref()
    }

    /// Take (and clear) the staged symbol entity id. The shell calls this each frame; `Some(id)` means
    /// route an open-code-symbol to that symbol, `None` means nothing pending.
    pub fn take_pending_code_symbol(&mut self) -> Option<String> {
        self.pending_code_symbol.take()
    }

    /// Register the cross-pane Open-Code-Symbol command (MT-034). Its handler is a no-op on the bus
    /// itself (the symbol id was staged by [`Self::request_open_code_symbol`] BEFORE dispatch, consumed
    /// by the shell drain) — the command exists so a clicked code-ref chip dispatches a REAL, named,
    /// addressable cross-pane action rather than a per-pane ad-hoc callback. Idempotent (last
    /// registration wins). Mirrors [`Self::register_open_document_command`] exactly (the MT-032 pattern).
    pub fn register_open_code_symbol_command(&mut self) {
        self.register_command(CommandDescriptor {
            id: CMD_OPEN_CODE_SYMBOL,
            name: "OpenCodeSymbol",
            label: "Open Code Symbol".to_owned(),
            keywords: vec!["open".to_owned(), "code".to_owned(), "symbol".to_owned()],
            keybind: None,
            // The symbol id was staged before dispatch (a generic handler carries no payload — the
            // stage-then-dispatch split is the contract point); request a repaint so the shell drains it.
            handler: Arc::new(|ctx, _bus| ctx.request_repaint()),
        });
    }

    /// Stage `symbol_entity_id` and dispatch [`CMD_OPEN_CODE_SYMBOL`] in one call (the clicked code-ref
    /// chip path — AC-2). Returns `true` when the command was found and dispatched (it always is once
    /// [`Self::register_open_code_symbol_command`] ran). The staged id is then readable via
    /// [`Self::pending_code_symbol`] until the shell drains it.
    pub fn open_code_symbol(
        &mut self,
        ctx: &egui::Context,
        symbol_entity_id: impl Into<String>,
    ) -> bool {
        self.request_open_code_symbol(symbol_entity_id);
        self.dispatch_command(ctx, CMD_OPEN_CODE_SYMBOL)
    }

    // ── Cross-pane Open-Locus-Ref navigation (MT-068 editors<->Locus cross-refs) ─────────────────────────

    /// Stage a `locus://` ref for a cross-pane Open-Locus-Ref (called just before dispatching
    /// [`CMD_OPEN_LOCUS_REF`], e.g. from a clicked Locus chip). The shell drains it next frame via
    /// [`Self::take_pending_locus_ref`] and routes it through the SAME MT-030 nav seam the other cross-refs
    /// use (a NavTarget for the WP/MT record). Callers stage the canonical original-case `locus://` URI;
    /// the parsed normalized value is reserved for lookup/search keying and never replaces the
    /// case-significant navigation identity.
    pub fn request_open_locus_ref(&mut self, locus_ref: impl Into<String>) {
        self.pending_locus_ref = Some(locus_ref.into());
    }

    /// The `locus://` ref staged for a cross-pane Locus open, WITHOUT consuming it (tests / peek).
    pub fn pending_locus_ref(&self) -> Option<&str> {
        self.pending_locus_ref.as_deref()
    }

    /// Take (and clear) the staged `locus://` ref. The shell calls this each frame; `Some(ref)` means route
    /// an open-locus-ref to that work unit, `None` means nothing pending.
    pub fn take_pending_locus_ref(&mut self) -> Option<String> {
        self.pending_locus_ref.take()
    }

    /// Register the cross-pane Open-Locus-Ref command (MT-068). Its handler is a no-op on the bus itself
    /// (the `locus://` ref was staged by [`Self::request_open_locus_ref`] BEFORE dispatch, consumed by the
    /// shell drain) — the command exists so a clicked Locus chip dispatches a REAL, named, addressable
    /// cross-pane action rather than a per-pane ad-hoc callback. Idempotent (last registration wins).
    /// Mirrors [`Self::register_open_code_symbol_command`] exactly (the MT-032/MT-034 pattern) — NO new
    /// navigation channel (RISK-007).
    pub fn register_open_locus_ref_command(&mut self) {
        self.register_command(CommandDescriptor {
            id: CMD_OPEN_LOCUS_REF,
            name: "OpenLocusRef",
            label: "Open Locus Reference".to_owned(),
            keywords: vec![
                "open".to_owned(),
                "locus".to_owned(),
                "work".to_owned(),
                "packet".to_owned(),
                "microtask".to_owned(),
            ],
            keybind: None,
            // The locus ref was staged before dispatch (a generic handler carries no payload — the
            // stage-then-dispatch split is the contract point); request a repaint so the shell drains it.
            handler: Arc::new(|ctx, _bus| ctx.request_repaint()),
        });
    }

    /// Stage `locus_ref` and dispatch [`CMD_OPEN_LOCUS_REF`] in one call (the clicked Locus chip path —
    /// AC-003). Returns `true` when the command was found and dispatched (it always is once
    /// [`Self::register_open_locus_ref_command`] ran). The staged ref is then readable via
    /// [`Self::pending_locus_ref`] until the shell drains it.
    pub fn open_locus_ref(&mut self, ctx: &egui::Context, locus_ref: impl Into<String>) -> bool {
        self.request_open_locus_ref(locus_ref);
        self.dispatch_command(ctx, CMD_OPEN_LOCUS_REF)
    }

    // ── Unified undo scope (MT-035 — POLICY-1..5) ──────────────────────────────────────────────────────

    /// Install the app's tokio runtime handle so the bus can dispatch a canvas COMPENSATING undo
    /// (POLICY-4 `undo_async_fn`) onto the runtime off the egui frame thread (HBR-QUIET). The shell calls
    /// this once at startup with the same handle the backend clients use. Absent a runtime (headless
    /// test) an async undo is reported as a typed "no runtime" result, never faked.
    pub fn set_undo_runtime(&mut self, runtime: tokio::runtime::Handle) {
        self.undo_runtime = Some(runtime);
    }

    /// Borrow the unified undo scope (tests / the "Show Undo History" inspector — MC-5).
    pub fn undo_scope(&self) -> &UnifiedUndoScope {
        &self.undo_scope
    }

    // ── One event ledger across surfaces (MT-036) ──────────────────────────────────────────────────────

    /// Install the single native-editor event emitter (the shell calls this once at startup with the
    /// production emitter bound to the app runtime + backend). Until installed, [`Self::emit_event`] is an
    /// honest no-op (never a fake emit) — matching the unmounted-pane defer policy.
    pub fn set_event_emitter(&mut self, emitter: crate::event_emitter::NativeEditorEventEmitter) {
        let workspace = emitter.workspace_id().to_owned();
        // One workspace keeps one emitter/session generation for the lifetime of this bus. In an
        // A -> B -> A switch the shell constructs another A emitter, but replacing A1 with A2 would
        // make a delayed completion captured under A1 inherit A2's transport/session trace. Reuse A1
        // instead; immutable event.workspace_id then selects the exact original generation below.
        let active = self
            .event_emitters_by_workspace
            .entry(workspace.clone())
            .or_insert(emitter)
            .clone();
        self.event_emitter_workspace_order
            .retain(|known| known != &workspace);
        self.event_emitter_workspace_order
            .push_back(workspace.clone());
        while self.event_emitter_workspace_order.len() > MAX_RETAINED_EVENT_EMITTER_WORKSPACES {
            let Some(expired) = self.event_emitter_workspace_order.pop_front() else {
                break;
            };
            if expired != workspace {
                // Dropping the final sender closes that generation's bounded worker after its accepted
                // queue drains. A later completion for an expired workspace fails with WorkspaceMismatch
                // against the active emitter and is surfaced in the shared error ring, never relabelled.
                self.event_emitters_by_workspace.remove(&expired);
            }
        }
        self.event_emitter = Some(active);
    }

    /// Number of retained workspace emitter generations (diagnostics + reclamation proof).
    pub fn retained_event_emitter_workspace_count(&self) -> usize {
        self.event_emitters_by_workspace.len()
    }

    /// Borrow the installed event emitter, if any (tests / the FlightRecorderPane reading the error ring).
    pub fn event_emitter(&self) -> Option<&crate::event_emitter::NativeEditorEventEmitter> {
        self.event_emitter.as_ref()
    }

    /// Emit a native editor event onto the ONE ledger (MT-036). Delegates to the installed emitter (off
    /// the egui frame thread, Semaphore-bounded — HBR-QUIET / RISK-2) AND fans the event out to every
    /// registered future surface ([`crate::surface_extension_seam::EditorSurface::on_event_emitted`]) —
    /// a no-op fan-out in production (the registry is empty). Returns `true` when an emitter was installed
    /// and the emit was DISPATCHED (it may still land in the error ring on a transport failure / drop);
    /// `false` when no emitter is installed (honest no-op, never a fake). The dispatched/dropped outcome
    /// is logged to the emitter's error ring; this method never panics the frame.
    pub fn emit_event_result(
        &self,
        event: crate::event_emitter::NativeEditorEvent,
    ) -> Result<(), crate::event_emitter::EmitError> {
        let emitter = if event.workspace_id.trim().is_empty() {
            self.event_emitter.as_ref()
        } else {
            self.event_emitters_by_workspace
                .get(&event.workspace_id)
                .or(self.event_emitter.as_ref())
        };
        let Some(emitter) = emitter else {
            return Err(crate::event_emitter::EmitError::Backpressure(
                "event-emitter-not-installed".to_owned(),
            ));
        };
        let accepted = emitter.emit_accepted(event)?;
        // A future-surface callback observes the editor action exactly once, only after the ordered
        // emitter accepted it. Frame retries therefore cannot duplicate the extension callback.
        self.surface_registry
            .dispatch_event_emitted(&accepted, emitter);
        Ok(())
    }

    pub fn emit_event(&self, event: crate::event_emitter::NativeEditorEvent) -> bool {
        self.emit_event_result(event).is_ok()
    }

    // ── Designed extension-seam registry (MT-036, DESIGN-ONLY) ───────────────────────────────────────────

    /// Register a future editor surface into the co-located registry (DESIGN-ONLY — no production caller
    /// today; an image-editor/spreadsheet/engine surface calls this at its own startup).
    pub fn register_surface(
        &mut self,
        surface: Box<dyn crate::surface_extension_seam::EditorSurface>,
    ) {
        self.surface_registry.register_surface(surface);
    }

    /// Borrow the future-surface registry (tests / diagnostics). Empty in production.
    pub fn surface_registry(&self) -> &crate::surface_extension_seam::EditorSurfaceRegistry {
        &self.surface_registry
    }

    /// Push a LOCAL-pane undo action onto `pane_id`'s ring (POLICY-1). Each pane calls this after
    /// applying an edit, capturing the previous snapshot in the action's `undo_fn` via a `Weak` back-ref
    /// (RISK-3 / MC-3).
    pub fn push_undo_local(&mut self, pane_id: PaneId, action: UndoAction) {
        self.undo_scope.push_local(pane_id, action);
    }

    /// Replace `pane_id`'s most recent LOCAL undo entry in place (MT-035 typing-coalescing — RISK-1 /
    /// MC-1). The rich-text pane calls this for a keystroke WITHIN the 500ms batch window so rapid edits
    /// coalesce into ONE undo entry instead of N. Returns `true` when a tail entry existed and was
    /// replaced; `false` when the pane has no entry yet (the caller then pushes a fresh one).
    pub fn replace_undo_local_tail(&mut self, pane_id: &PaneId, action: UndoAction) -> bool {
        self.undo_scope.replace_local_tail(pane_id, action)
    }

    /// Push a CROSS-PANE undo action onto the single cross-pane ring (POLICY-2). An atomic multi-pane
    /// action (embed-from-atelier, route-to-stage, canvas placement) calls this.
    pub fn push_undo_cross_pane(&mut self, action: UndoAction) {
        self.undo_scope.push_cross_pane(action);
    }

    /// LOCAL-FIRST undo for `pane_id` (POLICY-1, the Ctrl+Z path). Pops the focused pane's most recent
    /// action and invokes it: synchronously via `undo_fn`, or — for a canvas compensating action
    /// (POLICY-4) — by dispatching `undo_async_fn` onto the installed runtime. Returns:
    /// - `Some(UndoResult)` when an action was popped and invoked (sync result, or a `dispatched_async`
    ///   acknowledgement for the async path), and
    /// - cross-pane fallback when the focused pane's ring is empty, and
    /// - `None` only when both rings are empty.
    ///
    /// A `Some(result)` whose `!result.ok` should be logged by the caller to the Flight Recorder
    /// (MT-036); this method never panics on a failed undo.
    pub fn undo(&mut self, pane_id: &PaneId) -> Option<UndoResult> {
        if !self.undo_scope.can_undo_local(pane_id) {
            return self.undo_cross_pane();
        }
        if self.undo_scope.local_undo_requires_runtime(pane_id) && !self.can_dispatch_async() {
            return Some(Self::missing_undo_runtime_result());
        }
        let action = self.undo_scope.pop_undo_local(pane_id)?;
        let result = self.invoke_undo(action);
        // MT-036: emit the ONE-ledger `undo_fired` HERE so EVERY local undo entry point records exactly
        // once — the Ctrl+Z chord, the command-palette `Undo`, and the shell keybind all route through this
        // method (POLICY-1 scope=local). An empty ring returned above via `?`, so a no-op undo is NOT an
        // event. Before MT-036 only the chord site emitted, leaving palette/keybind undo silent.
        self.emit_undo_fired_event(crate::event_emitter::UndoScope::Local, pane_id.as_ref());
        Some(result)
    }

    /// LOCAL redo for `pane_id` (POLICY-1, the Ctrl+Y path). Pops the focused pane's most recently
    /// undone action and re-applies it (sync `redo_fn`, or async `redo_async_fn` for canvas). `None`
    /// Falls back to the cross-pane redo ring when the local redo ring is empty.
    pub fn redo(&mut self, pane_id: &PaneId) -> Option<UndoResult> {
        if !self.undo_scope.can_redo_local(pane_id) {
            return self.redo_cross_pane();
        }
        if self.undo_scope.local_redo_requires_runtime(pane_id) && !self.can_dispatch_async() {
            return Some(Self::missing_undo_runtime_result());
        }
        let action = self.undo_scope.pop_redo_local(pane_id)?;
        let result = self.invoke_redo(action);
        // MT-036: emit the ONE-ledger `undo_fired` (POLICY-1 scope=local) at the single redo choke point.
        self.emit_undo_fired_event(crate::event_emitter::UndoScope::Local, pane_id.as_ref());
        Some(result)
    }

    /// CROSS-PANE undo (POLICY-2, the Ctrl+Shift+Z path). Pops the most recent cross-pane action and
    /// invokes its undo (sync or, for a canvas placement, the async compensating call — POLICY-4).
    /// `None` when the cross-pane ring is empty.
    pub fn undo_cross_pane(&mut self) -> Option<UndoResult> {
        if self.undo_scope.cross_pane_async_pending() {
            return Some(UndoResult::err(
                "a canvas compensating undo/redo is already in flight; wait for canonical reconciliation",
            ));
        }
        if self.undo_scope.cross_pane_undo_requires_runtime() && !self.can_dispatch_async() {
            return Some(Self::missing_undo_runtime_result());
        }
        let action = self.undo_scope.pop_undo_cross_pane()?;
        let result = self.invoke_undo(action);
        // MT-036: emit the ONE-ledger `undo_fired` with scope=cross_pane (POLICY-2). This path was
        // ENTIRELY silent before MT-036 (UndoScope::CrossPane was dead-in-prod). The originating pane label
        // is the current focus owner (empty -> DEFAULT_ACTOR_ID for a pure cross-pane action).
        let pane_label = self
            .focus_owner()
            .map(|p| p.as_ref().to_owned())
            .unwrap_or_else(|| "cross-pane".to_owned());
        self.emit_undo_fired_event(crate::event_emitter::UndoScope::CrossPane, &pane_label);
        Some(result)
    }

    /// CROSS-PANE redo. Pops the most recently undone cross-pane action and re-applies it.
    pub fn redo_cross_pane(&mut self) -> Option<UndoResult> {
        if self.undo_scope.cross_pane_async_pending() {
            return Some(UndoResult::err(
                "a canvas compensating undo/redo is already in flight; wait for canonical reconciliation",
            ));
        }
        if self.undo_scope.cross_pane_redo_requires_runtime() && !self.can_dispatch_async() {
            return Some(Self::missing_undo_runtime_result());
        }
        let action = self.undo_scope.pop_redo_cross_pane()?;
        let result = self.invoke_redo(action);
        // MT-036: emit the ONE-ledger `undo_fired` with scope=cross_pane (POLICY-2) at the cross-pane redo
        // choke point (silent before MT-036, same as the cross-pane undo above).
        let pane_label = self
            .focus_owner()
            .map(|p| p.as_ref().to_owned())
            .unwrap_or_else(|| "cross-pane".to_owned());
        self.emit_undo_fired_event(crate::event_emitter::UndoScope::CrossPane, &pane_label);
        Some(result)
    }

    /// Finalize one provisional backend-touching cross-pane undo/redo transition. The Canvas host calls
    /// this only after draining the compensation completion produced by the async backend operation.
    /// Failed operations return the action to its original ring, preserving an operator retry path.
    pub fn complete_cross_pane_async(
        &mut self,
        action_id: &str,
        direction: crate::undo_stack::AsyncUndoDirection,
        success: bool,
    ) -> bool {
        self.undo_scope
            .complete_cross_pane_async(action_id, direction, success)
    }

    /// Emit exactly ONE `undo_fired` event onto the ONE ledger (MT-036) for an undo/redo that actually
    /// fired. Centralized here so EVERY undo entry point — the Ctrl+Z/Ctrl+Y chord
    /// ([`crate::rich_editor::renderer::rich_editor_widget`]), the command-palette
    /// `Undo`/`Redo`/`Undo Cross-Pane` commands ([`Self::register_undo_commands`]), and the shell keybind
    /// dispatch — records once with the correct [`crate::event_emitter::UndoScope`]. Before MT-036 only the
    /// chord path emitted, so palette + cross-pane undo were SILENT and `UndoScope::CrossPane` was
    /// dead-in-prod; now the emit lives at the four bus choke points and NO call site emits separately.
    /// Routes through [`Self::emit_event`] so the design-only future-surface registry fan-out stays wired.
    /// Honest no-op until the shell installs the emitter (unmounted-pane defer). `pane_label` is the
    /// originating pane slug (empty for a pure cross-pane action -> `DEFAULT_ACTOR_ID`).
    fn emit_undo_fired_event(&self, scope: crate::event_emitter::UndoScope, pane_label: &str) {
        let workspace_id = self
            .event_emitter()
            .map(|e| e.workspace_id().to_owned())
            .unwrap_or_default();
        let event = crate::event_emitter::NativeEditorEvent::undo_fired(
            scope,
            pane_label,
            crate::event_emitter::native_editor_actor_id(pane_label),
            workspace_id,
        );
        self.emit_event(event);
    }

    /// The local "Undo ({n})" indicator count for `pane_id` (AC-6).
    pub fn local_undo_count(&self, pane_id: &PaneId) -> usize {
        self.undo_scope.local_undo_count(pane_id)
    }

    /// Invoke an action's UNDO half: the async compensating path when present (POLICY-4), else the
    /// synchronous `undo_fn`. The async dispatch is fire-and-forget onto the runtime (the board
    /// re-fetches after the compensating call lands), so it returns a `dispatched_async` acknowledgement
    /// immediately rather than blocking the frame (HBR-QUIET). With no runtime installed it reports a
    /// typed failure instead of faking success.
    fn invoke_undo(&self, action: UndoAction) -> UndoResult {
        match &action.undo_async_fn {
            Some(async_fn) => self.dispatch_async(async_fn.clone()),
            None => (action.undo_fn)(),
        }
    }

    /// Invoke an action's REDO half (mirror of [`Self::invoke_undo`]).
    fn invoke_redo(&self, action: UndoAction) -> UndoResult {
        match &action.redo_async_fn {
            Some(async_fn) => self.dispatch_async(async_fn.clone()),
            None => (action.redo_fn)(),
        }
    }

    fn can_dispatch_async(&self) -> bool {
        self.undo_runtime.is_some()
    }

    fn missing_undo_runtime_result() -> UndoResult {
        UndoResult::err(
            "no tokio runtime installed for canvas compensating undo (set_undo_runtime not called)",
        )
    }

    /// Dispatch a POLICY-4 async compensating closure onto the installed runtime (off the egui frame
    /// thread — HBR-QUIET). Returns a `dispatched_async` acknowledgement on success, or a typed "no
    /// runtime" failure when none is installed (a headless test) — never a fabricated success.
    fn dispatch_async(&self, async_fn: crate::undo_stack::UndoAsyncFn) -> UndoResult {
        match &self.undo_runtime {
            Some(handle) => {
                handle.spawn(async move {
                    let result = async_fn().await;
                    if !result.ok {
                        tracing::warn!(error = ?result.error, "MT-035 canvas compensating undo failed");
                    }
                });
                UndoResult::dispatched_async()
            }
            None => Self::missing_undo_runtime_result(),
        }
    }

    /// Register the three unified-undo commands on the cross-pane command bus so they appear in the
    /// command palette AND match their keybinds (Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z). The handlers read the
    /// CURRENT focus owner from the locked bus: Ctrl+Z/Ctrl+Y are local-first for the focused pane and
    /// fall back to the cross-pane ring when that local ring is empty (POLICY-1), while Ctrl+Shift+Z
    /// directly owns the cross-pane ring (POLICY-2). With no focus owner, Undo/Redo also fail over to the
    /// cross-pane ring so a Canvas/Stage action never becomes unreachable. Idempotent (last registration
    /// wins). Call once when the first editor pane mounts.
    pub fn register_undo_commands(&mut self) {
        self.register_command(CommandDescriptor {
            id: CMD_UNDO,
            name: "Undo",
            label: "Undo".to_owned(),
            keywords: vec!["undo".to_owned(), "revert".to_owned()],
            keybind: default_keybind_for(CMD_UNDO),
            handler: Arc::new(|ctx, bus| {
                if bus.focus_owner_is_cross_only() {
                    bus.undo_cross_pane();
                } else if let Some(pane_id) = bus.focus_owner().cloned() {
                    bus.undo(&pane_id);
                } else {
                    bus.undo_cross_pane();
                }
                ctx.request_repaint();
            }),
        });
        self.register_command(CommandDescriptor {
            id: CMD_REDO,
            name: "Redo",
            label: "Redo".to_owned(),
            keywords: vec!["redo".to_owned()],
            keybind: default_keybind_for(CMD_REDO),
            handler: Arc::new(|ctx, bus| {
                if bus.focus_owner_is_cross_only() {
                    bus.redo_cross_pane();
                } else if let Some(pane_id) = bus.focus_owner().cloned() {
                    bus.redo(&pane_id);
                } else {
                    bus.redo_cross_pane();
                }
                ctx.request_repaint();
            }),
        });
        self.register_command(CommandDescriptor {
            id: CMD_UNDO_CROSS_PANE,
            name: "UndoCrossPane",
            label: "Undo Cross-Pane".to_owned(),
            keywords: vec!["undo".to_owned(), "cross".to_owned(), "pane".to_owned()],
            keybind: default_keybind_for(CMD_UNDO_CROSS_PANE),
            handler: Arc::new(|ctx, bus| {
                bus.undo_cross_pane();
                ctx.request_repaint();
            }),
        });
    }
}

/// Build the standard egui shortcut for a VS-Code-parity command id, or `None` for a palette-only id.
/// Centralized so the keybinds match the contract's mapping (Copy=Ctrl+C, Cut=Ctrl+X, Paste=Ctrl+V,
/// SelectAll=Ctrl+A, Find=Ctrl+F, CommandPalette=Ctrl+Shift+P) and stay in one place.
pub fn default_keybind_for(command_id: &str) -> Option<egui::KeyboardShortcut> {
    use egui::{Key, KeyboardShortcut, Modifiers};
    let shortcut = match command_id {
        CMD_COPY => KeyboardShortcut::new(Modifiers::COMMAND, Key::C),
        CMD_CUT => KeyboardShortcut::new(Modifiers::COMMAND, Key::X),
        CMD_PASTE => KeyboardShortcut::new(Modifiers::COMMAND, Key::V),
        CMD_SELECT_ALL => KeyboardShortcut::new(Modifiers::COMMAND, Key::A),
        CMD_FIND => KeyboardShortcut::new(Modifiers::COMMAND, Key::F),
        CMD_COMMAND_PALETTE => {
            KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::P)
        }
        // MT-035 unified undo: Ctrl+Z local-first undo, Ctrl+Y redo, Ctrl+Shift+Z cross-pane undo.
        CMD_UNDO => KeyboardShortcut::new(Modifiers::COMMAND, Key::Z),
        CMD_REDO => KeyboardShortcut::new(Modifiers::COMMAND, Key::Y),
        CMD_UNDO_CROSS_PANE => {
            KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z)
        }
        _ => return None,
    };
    Some(shortcut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// An in-memory clipboard mock (the MT-017 control: a headless test NEVER touches the OS clipboard).
    struct MockClipboard {
        last: StdMutex<Option<String>>,
    }
    impl MockClipboard {
        fn new() -> Self {
            Self {
                last: StdMutex::new(None),
            }
        }
        fn taken(&self) -> Option<String> {
            self.last.lock().unwrap().clone()
        }
    }
    impl ClipboardSink for MockClipboard {
        fn copy(&self, text: &str) {
            *self.last.lock().unwrap() = Some(text.to_owned());
        }
    }

    fn pane(id: &str) -> PaneId {
        std::sync::Arc::from(id)
    }

    fn text_selection(pane_id: &str, text: &str) -> SharedSelection {
        SharedSelection::TextRange {
            pane_id: pane(pane_id),
            surface: EditorSurfaceKind::Code,
            start: 0,
            end: text.len(),
            text: text.to_owned(),
        }
    }

    /// Unit AC (a): a selection published from the focused pane is observable from any other pane,
    /// reflecting the source pane_id + text.
    #[test]
    fn selection_propagates_from_focus_owner() {
        let mut bus = InteractionBus::new();
        bus.set_focus_owner(pane("pane-code"));
        assert!(bus.set_selection(text_selection("pane-code", "hello")));
        match bus.shared_selection() {
            SharedSelection::TextRange { pane_id, text, .. } => {
                assert_eq!(pane_id.as_ref(), "pane-code");
                assert_eq!(text, "hello");
            }
            other => panic!("expected a TextRange selection, got {other:?}"),
        }
    }

    /// A non-focus-owner pane cannot clobber the focused pane's selection.
    #[test]
    fn non_owner_selection_is_rejected() {
        let mut bus = InteractionBus::new();
        bus.set_focus_owner(pane("pane-code"));
        assert!(bus.set_selection(text_selection("pane-code", "owned")));
        // A background pane tries to overwrite — rejected, the owner's selection stays.
        assert!(!bus.set_selection(text_selection("pane-rich", "intruder")));
        assert_eq!(
            bus.shared_selection()
                .pane_id()
                .map(|p| p.as_ref().to_owned()),
            Some("pane-code".to_owned())
        );
    }

    #[test]
    fn editor_document_change_invalidates_live_and_staged_selection_for_the_pane() {
        let mut bus = InteractionBus::new();
        let pane_id = pane("pane-code");
        bus.set_focus_owner(pane_id.clone());
        assert!(bus.set_selection(text_selection("pane-code", "same-range")));
        bus.request_memory_proposal();

        assert!(bus.invalidate_selection_for_pane(&pane_id));
        assert_eq!(bus.shared_selection(), &SharedSelection::None);
        assert_eq!(
            bus.take_pending_memory_proposal_request()
                .expect("staged request remains observable")
                .selection,
            SharedSelection::None,
            "a request captured before the shell observes the tab change cannot retain stale text"
        );
        assert!(
            !bus.invalidate_selection_for_pane(&pane("pane-other")),
            "a different pane cannot invalidate the current pane's state"
        );
    }

    #[test]
    fn different_editor_focus_invalidates_live_and_staged_proposal_selection() {
        let mut bus = InteractionBus::new();
        let pane_a = pane("pane-a");
        let pane_b = pane("pane-b");
        bus.set_focus_owner(pane_a.clone());
        assert!(bus.set_selection(text_selection("pane-a", "must-not-cross")));
        bus.request_memory_proposal();

        bus.set_focus_owner(pane_b.clone());

        assert_eq!(bus.focus_owner(), Some(&pane_b));
        assert_eq!(bus.shared_selection(), &SharedSelection::None);
        assert_eq!(
            bus.take_pending_memory_proposal_request()
                .expect("queued request remains observable as a typed blocker")
                .selection,
            SharedSelection::None,
            "a request snapshotted before the focus transfer cannot submit pane A provenance from pane B"
        );
    }

    #[test]
    fn same_editor_focus_and_utility_routing_preserve_selection_and_staged_request() {
        let mut bus = InteractionBus::new();
        let pane_a = pane("pane-a");
        let selection = text_selection("pane-a", "retain-me");
        bus.set_focus_owner(pane_a.clone());
        assert!(bus.set_selection(selection.clone()));
        bus.request_memory_proposal();

        // A same-pane frame is not a focus transfer. Utility/menu routing uses the undo-only seam and
        // likewise must not destroy the editor selection captured for the command palette/dialog flow.
        bus.set_focus_owner(pane_a);
        bus.set_undo_focus_owner(pane("utility-settings"), false);

        assert_eq!(bus.shared_selection(), &selection);
        bus.set_focus_owner(pane("pane-a"));
        assert_eq!(
            bus.shared_selection(),
            &selection,
            "returning from a utility surface to the same editor retains its context"
        );
        assert_eq!(
            bus.take_pending_memory_proposal_request()
                .expect("same-pane/utility routing retains the queued proposal")
                .selection,
            selection
        );
    }

    #[test]
    fn different_editor_focus_after_utility_routing_invalidates_retained_selection() {
        let mut bus = InteractionBus::new();
        bus.set_focus_owner(pane("pane-a"));
        assert!(bus.set_selection(text_selection("pane-a", "retained-through-utility")));
        bus.request_memory_proposal();
        bus.set_undo_focus_owner(pane("utility-memory"), false);
        assert!(bus.shared_selection().is_some());

        bus.set_focus_owner(pane("pane-b"));

        assert_eq!(bus.shared_selection(), &SharedSelection::None);
        assert_eq!(
            bus.take_pending_memory_proposal_request()
                .expect("staged request remains as typed blocker")
                .selection,
            SharedSelection::None,
            "utility retention cannot let pane A selection survive a later editor-B focus transfer"
        );
    }

    /// Red-team RISK-4 / MC-4: a selection whose pane is no longer live returns `None`, never a dangling
    /// reference.
    #[test]
    fn stale_pane_selection_is_guarded() {
        let mut bus = InteractionBus::new();
        bus.set_focus_owner(pane("pane-gone"));
        bus.set_selection(text_selection("pane-gone", "stale"));
        // The pane is still considered live here:
        let live = vec![pane("pane-gone")];
        assert!(bus.shared_selection_if_live(&live).is_some());
        // Now the pane closed — only other panes are live:
        let live = vec![pane("pane-code"), pane("pane-rich")];
        assert_eq!(bus.shared_selection_if_live(&live), SharedSelection::None);
    }

    fn noop_undo_action(description: &str) -> UndoAction {
        UndoAction::sync(
            description,
            Arc::new(UndoResult::ok),
            Arc::new(UndoResult::ok),
        )
    }

    #[test]
    fn workspace_rebind_discards_all_workspace_bound_interaction_state() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        assert!(bus.bind_workspace("workspace-a"));
        bus.register_route_to_stage_command();
        let generation_a = bus.workspace_generation();
        let pane_id = pane("pane-code");

        bus.set_focus_owner(pane_id.clone());
        assert!(bus.set_selection(text_selection("pane-code", "workspace-a-selection")));
        bus.cache_clipboard(ClipboardPayload::LoomBlockRef(
            "workspace-a-block".to_owned(),
        ));
        assert!(bus.request_clipboard_command(ClipboardCommand::Paste));
        bus.request_open_document("workspace-a-document");
        bus.request_focus_calendar_event("workspace-a-calendar-event");
        assert!(bus.route_to_stage_correlated(
            &ctx,
            crate::stage_pane::StageContent::Selection(
                "workspace-a-selection".to_owned(),
                "workspace-a-document".to_owned(),
            ),
            Some("workspace-a-action"),
        ));
        bus.request_open_code_symbol("workspace-a-symbol");
        bus.request_open_locus_ref("locus://workspace-a/WP-1/MT-1");
        bus.request_memory_proposal();
        bus.push_undo_local(pane_id.clone(), noop_undo_action("workspace-a-local"));
        bus.push_undo_cross_pane(noop_undo_action("workspace-a-cross-pane"));

        // A same-workspace bind is the normal per-frame path and must not discard state.
        assert!(!bus.bind_workspace("workspace-a"));
        assert_eq!(bus.local_undo_count(&pane_id), 1);
        assert_eq!(bus.undo_scope().cross_pane_undo_count(), 1);

        assert!(bus.bind_workspace("workspace-b"));
        assert_ne!(bus.workspace_generation(), generation_a);
        assert_eq!(bus.workspace_id(), "workspace-b");
        assert_eq!(bus.shared_selection(), &SharedSelection::None);
        assert!(bus.focus_owner().is_none());
        assert!(bus.clipboard_read().is_none());
        assert!(bus.take_clipboard_command_for(&pane_id).is_none());
        assert!(bus.take_pending_navigation().is_none());
        assert!(bus.take_pending_calendar_event_focus().is_none());
        assert!(bus.pending_stage_route().is_none());
        assert!(bus.take_pending_stage_error().is_none());
        assert!(bus.take_pending_code_symbol().is_none());
        assert!(bus.take_pending_locus_ref().is_none());
        assert!(bus.take_pending_memory_proposal_request().is_none());
        assert_eq!(bus.local_undo_count(&pane_id), 0);
        assert_eq!(bus.undo_scope().cross_pane_undo_count(), 0);
        assert!(bus.undo(&pane_id).is_none());
        assert!(bus.redo(&pane_id).is_none());

        // Returning to A cannot resurrect history or pending payloads discarded at the first boundary.
        assert!(bus.bind_workspace("workspace-a"));
        assert_eq!(bus.local_undo_count(&pane_id), 0);
        assert_eq!(bus.undo_scope().cross_pane_undo_count(), 0);
        assert!(bus.take_pending_navigation().is_none());
        assert!(bus.take_pending_calendar_event_focus().is_none());
    }

    #[test]
    fn workspace_rebind_discards_stage_errors_and_fences_late_async_undo_completion() {
        let mut bus = InteractionBus::new();
        assert!(bus.bind_workspace("workspace-a"));
        bus.request_route_to_stage_error("workspace-a-stage-error");

        let sync_undo = Arc::new(UndoResult::ok);
        let async_undo: crate::undo_stack::UndoAsyncFn =
            Arc::new(|| Box::pin(async { UndoResult::ok() }));
        bus.push_undo_cross_pane(UndoAction::async_compensating(
            "workspace-a-async-action",
            "workspace-a-async",
            sync_undo.clone(),
            sync_undo,
            async_undo.clone(),
            async_undo,
        ));
        assert!(bus.undo_scope.pop_undo_cross_pane().is_some());
        assert!(bus.undo_scope().cross_pane_async_pending());

        assert!(bus.bind_workspace("workspace-b"));
        assert!(bus.take_pending_stage_error().is_none());
        assert!(!bus.undo_scope().cross_pane_async_pending());
        assert!(!bus.complete_cross_pane_async(
            "workspace-a-async-action",
            crate::undo_stack::AsyncUndoDirection::Undo,
            true,
        ));
        assert_eq!(bus.undo_scope().cross_pane_undo_count(), 0);
    }

    /// AC (b): a clipboard write goes to the mock sink (plain-text projection) AND caches the richest
    /// variant; the cross-pane read recovers the rich variant.
    #[test]
    fn clipboard_round_trip_caches_rich_variant() {
        let mut bus = InteractionBus::new();
        let mock = MockClipboard::new();
        bus.clipboard_write(ClipboardPayload::PlainText("plain".to_owned()), &mock);
        assert_eq!(mock.taken().as_deref(), Some("plain"));
        assert_eq!(bus.clipboard_read_text().as_deref(), Some("plain"));

        // A LoomBlockRef projects to its loom:// URI on the OS clipboard but the rich variant survives
        // in the in-memory cache for a cross-pane Paste.
        bus.clipboard_write(ClipboardPayload::LoomBlockRef("blk-7".to_owned()), &mock);
        assert_eq!(mock.taken().as_deref(), Some("loom://blk-7"));
        assert_eq!(
            bus.clipboard_read(),
            Some(&ClipboardPayload::LoomBlockRef("blk-7".to_owned())),
            "the rich LoomBlockRef variant survives in the cross-pane cache"
        );
    }

    /// AC (c): a registered command is dispatched by id and its handler side-effect is observed (here:
    /// the handler opens the command palette via the locked bus).
    #[test]
    fn dispatch_command_invokes_handler() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.register_command(CommandDescriptor {
            id: CMD_COMMAND_PALETTE,
            name: "CommandPalette",
            label: "Command Palette".to_owned(),
            keywords: vec!["palette".to_owned()],
            keybind: default_keybind_for(CMD_COMMAND_PALETTE),
            handler: Arc::new(|_ctx, bus| bus.open_command_palette()),
        });
        assert!(!bus.command_palette_open());
        assert!(bus.dispatch_command(&ctx, CMD_COMMAND_PALETTE));
        assert!(
            bus.command_palette_open(),
            "the handler opened the palette via the locked bus"
        );
        // An unknown id is a no-op, not a panic.
        assert!(!bus.dispatch_command(&ctx, "interop.does-not-exist"));
    }

    /// The keybind dispatcher resolves a shortcut to its command id (used AFTER the pane consumes the
    /// shortcut — RISK-3 / MC-3).
    #[test]
    fn keybind_resolves_to_command_id() {
        let mut bus = InteractionBus::new();
        bus.register_command(CommandDescriptor {
            id: CMD_COPY,
            name: "Copy",
            label: "Copy".to_owned(),
            keywords: vec![],
            keybind: default_keybind_for(CMD_COPY),
            handler: Arc::new(|_, _| {}),
        });
        let ctrl_c = default_keybind_for(CMD_COPY).unwrap();
        assert_eq!(bus.matching_keybind_command(&ctrl_c), Some(CMD_COPY));
        let ctrl_x = default_keybind_for(CMD_CUT).unwrap();
        assert_eq!(
            bus.matching_keybind_command(&ctrl_x),
            None,
            "no Cut command registered"
        );
    }

    /// `get_or_init` returns the SAME `Arc` instance on repeated calls against one context (every pane
    /// shares one bus).
    #[test]
    fn get_or_init_returns_shared_instance() {
        let ctx = egui::Context::default();
        let a = InteractionBus::get_or_init(&ctx);
        let b = InteractionBus::get_or_init(&ctx);
        assert!(Arc::ptr_eq(&a, &b), "all panes share the same bus Arc");
    }

    /// `with_try_lock` returns the closure result on acquisition and `None` while the lock is held
    /// (re-entrancy guard — never blocks the frame).
    #[test]
    fn try_lock_skips_on_contention() {
        let bus = Arc::new(Mutex::new(InteractionBus::new()));
        let got = InteractionBus::with_try_lock(&bus, |b| {
            // While we hold the guard inside the closure, a re-entrant try_lock would contend.
            b.open_command_palette();
            // Simulate a re-entrant attempt from "another pane" in the same frame.
            let reentrant = InteractionBus::with_try_lock(&bus, |_| 42);
            assert_eq!(
                reentrant, None,
                "a re-entrant try_lock contends and is skipped, not deadlocked"
            );
            7
        });
        assert_eq!(got, Some(7));
        assert!(bus.lock().unwrap().command_palette_open());
    }

    /// The command-list-item author_id matches the contract's `cmd-{name}` shape (e.g. `cmd-Copy`), and
    /// an arbitrary external name is defensively stripped to `[A-Za-z0-9-]`.
    #[test]
    fn command_list_item_author_id_matches_contract_shape() {
        assert_eq!(command_list_item_author_id("Copy"), "cmd-Copy");
        assert_eq!(
            command_list_item_author_id("CommandPalette"),
            "cmd-CommandPalette"
        );
        // A name with unsafe chars is sanitized (dots/slashes -> '-').
        let id = command_list_item_author_id("weird.name/x");
        assert!(id.starts_with(COMMAND_LIST_ITEM_AUTHOR_PREFIX));
        let suffix = &id[COMMAND_LIST_ITEM_AUTHOR_PREFIX.len()..];
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "author_id suffix must be [A-Za-z0-9-]; got '{suffix}'"
        );
    }

    /// MT-032 AC-4: staging a document id + dispatching the Open-Document command stages the target on
    /// the bus, where the shell drains it. A real, named, addressable cross-pane action — not a no-op.
    #[test]
    fn open_document_stages_navigation_target() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.register_open_document_command();
        assert!(
            bus.commands().get(CMD_OPEN_DOCUMENT).is_some(),
            "open-document command registered"
        );
        assert!(bus.pending_navigation().is_none(), "nothing pending before");
        // The backlink-row click path: stage + dispatch in one call.
        assert!(bus.open_document(&ctx, "DOC-A"), "open-document dispatched");
        assert_eq!(
            bus.pending_navigation(),
            Some("DOC-A"),
            "the staged target is observable"
        );
        // The shell drains it once.
        assert_eq!(bus.take_pending_navigation().as_deref(), Some("DOC-A"));
        assert!(
            bus.take_pending_navigation().is_none(),
            "drained once, then empty"
        );
    }

    /// MT-034 AC-2: staging a symbol entity id + dispatching the Open-Code-Symbol command stages the
    /// target on the bus, where the shell drains it and routes it through the ShellNavigator seam. A
    /// real, named, addressable cross-pane action — the symmetric counterpart of Open-Document.
    #[test]
    fn open_code_symbol_stages_target() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.register_open_code_symbol_command();
        assert!(
            bus.commands().get(CMD_OPEN_CODE_SYMBOL).is_some(),
            "open-code-symbol command registered"
        );
        assert!(
            bus.pending_code_symbol().is_none(),
            "nothing pending before"
        );
        // The clicked code-ref chip path: stage + dispatch in one call.
        assert!(
            bus.open_code_symbol(&ctx, "ent-42"),
            "open-code-symbol dispatched"
        );
        assert_eq!(
            bus.pending_code_symbol(),
            Some("ent-42"),
            "the staged symbol id is observable"
        );
        // The shell drains it once.
        assert_eq!(bus.take_pending_code_symbol().as_deref(), Some("ent-42"));
        assert!(
            bus.take_pending_code_symbol().is_none(),
            "drained once, then empty"
        );
    }

    /// Dispatching Open-Code-Symbol WITHOUT registering it is a benign false (unknown id), not a panic;
    /// the staged id still drains independently.
    #[test]
    fn open_code_symbol_unregistered_is_benign() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.request_open_code_symbol("ent-X");
        assert!(
            !bus.dispatch_command(&ctx, CMD_OPEN_CODE_SYMBOL),
            "unknown command id is a no-op false"
        );
        assert_eq!(bus.take_pending_code_symbol().as_deref(), Some("ent-X"));
    }

    /// Dispatching Open-Document WITHOUT registering it is a benign false (unknown id), not a panic.
    #[test]
    fn open_document_unregistered_is_benign() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        // request_open_document stages even without the command; dispatch returns false (unknown id).
        bus.request_open_document("DOC-X");
        assert!(
            !bus.dispatch_command(&ctx, CMD_OPEN_DOCUMENT),
            "unknown command id is a no-op false"
        );
        // The staged id still drains (the stage is independent of dispatch).
        assert_eq!(bus.take_pending_navigation().as_deref(), Some("DOC-X"));
    }

    /// MT-033 AC-4: staging StageContent + dispatching the Route-to-Stage command stages the content on
    /// the bus, where the shell drains it to open/focus the Stage pane. A real, named, addressable
    /// cross-pane action — mirrors the MT-032 open-document staging.
    #[test]
    fn route_to_stage_stages_content() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.register_route_to_stage_command();
        assert!(
            bus.commands().get(CMD_ROUTE_TO_STAGE).is_some(),
            "route-to-stage command registered"
        );
        assert!(
            bus.pending_stage_content().is_none(),
            "nothing staged before"
        );
        let content =
            crate::stage_pane::StageContent::Selection("hello".to_owned(), "DOC-7".to_owned());
        assert!(
            bus.route_to_stage(&ctx, content.clone()),
            "route-to-stage dispatched"
        );
        assert_eq!(
            bus.pending_stage_content(),
            Some(&content),
            "the staged content is observable"
        );
        let route = bus
            .pending_stage_route()
            .cloned()
            .expect("complete route pending for the shell");
        assert_eq!(route.content, content);
        assert_eq!(route.content_kind, "selection");
        assert!(route.causal_action_id.is_none());
        let event_id = route.receipt.event_id.clone();
        assert_eq!(
            bus.ack_pending_stage_route(&event_id),
            Some(route),
            "the shell acknowledges the exact route after applying it"
        );
        assert!(
            bus.pending_stage_route().is_none(),
            "acknowledged once, then empty"
        );
    }

    /// Dispatching Route-to-Stage WITHOUT registering it is a benign false (unknown id), not a panic,
    /// and no route or success receipt is admitted.
    #[test]
    fn route_to_stage_unregistered_is_benign() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        assert!(
            !bus.route_to_stage(&ctx, crate::stage_pane::StageContent::Empty),
            "unknown command id is a no-op false"
        );
        assert!(
            bus.pending_stage_route().is_none(),
            "failed dispatch admits no route"
        );
    }

    #[test]
    fn second_stage_route_cannot_overwrite_first_and_retry_reuses_event_identity() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.register_route_to_stage_command();
        let first =
            crate::stage_pane::StageContent::Selection("first".to_owned(), "DOC-1".to_owned());
        let second =
            crate::stage_pane::StageContent::Selection("second".to_owned(), "DOC-2".to_owned());
        assert!(bus.route_to_stage_correlated(&ctx, first.clone(), Some("causal-first")));
        let retained = bus.pending_stage_route().cloned().unwrap();
        assert!(!bus.route_to_stage_correlated(&ctx, second, Some("causal-second")));
        assert_eq!(bus.pending_stage_route(), Some(&retained));
        assert!(bus.ack_pending_stage_route("wrong-event-id").is_none());
        assert_eq!(bus.pending_stage_route(), Some(&retained));

        let event_id = retained.receipt.event_id.clone();
        assert_eq!(
            bus.ack_pending_stage_route(&event_id),
            Some(retained.clone())
        );
        assert!(bus.retry_pending_stage_route(&ctx, retained.clone()));
        assert_eq!(
            bus.pending_stage_route().unwrap().receipt.event_id,
            retained.receipt.event_id,
            "retry must reuse the producer-created receipt identity"
        );
    }

    /// The default keybinds match the contract's VS Code mapping.
    #[test]
    fn default_keybinds_match_vscode_mapping() {
        use egui::{Key, Modifiers};
        assert_eq!(
            default_keybind_for(CMD_COPY).unwrap(),
            egui::KeyboardShortcut::new(Modifiers::COMMAND, Key::C)
        );
        assert_eq!(
            default_keybind_for(CMD_COMMAND_PALETTE).unwrap(),
            egui::KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::P)
        );
        assert!(default_keybind_for("interop.unknown").is_none());
    }
}
