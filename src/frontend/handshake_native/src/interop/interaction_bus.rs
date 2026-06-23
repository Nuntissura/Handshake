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

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::event_bus::ShellEventSender;
use crate::pane_registry::PaneId;
use crate::rich_editor::properties::metadata_client::ClipboardSink;
use crate::undo_stack::{UndoAction, UndoResult, UnifiedUndoScope};

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
/// via [`InteractionBus::request_route_to_stage`]; the shell drains
/// [`InteractionBus::take_pending_stage_content`] and opens/focuses the Stage pane with that content.
/// This is the melt-together Editors<->Stage (Pillar 17) navigation primitive. The DEEPER Stage backend
/// interop (capture/embed-back with manifest provenance) is E10 (MT-066), NOT this command.
pub const CMD_ROUTE_TO_STAGE: &str = "interop.route-to-stage";
/// WP-KERNEL-012 MT-034 (E5 — code<->note cross-refs): cross-pane Open-Code-Symbol command id. A
/// clicked `[[code:…]]` chip in a note dispatches this with the target symbol entity id staged via
/// [`InteractionBus::request_open_code_symbol`]; the shell drains
/// [`InteractionBus::take_pending_code_symbol`] each frame and routes it through the MT-030
/// [`crate::quick_switcher::ShellNavigator::open_code_symbol`] seam (which returns a typed
/// `EditorPaneNotMounted` until the code pane mounts at E11/MT-069 — never a faked jump). This is the
/// melt-together note->code navigation primitive, the symmetric counterpart of [`CMD_OPEN_DOCUMENT`].
pub const CMD_OPEN_CODE_SYMBOL: &str = "interop.open-code-symbol";
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
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
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
    BlockRef {
        pane_id: PaneId,
        block_id: String,
    },
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
            SharedSelection::TextRange { surface, .. } | SharedSelection::NodeRef { surface, .. } => {
                Some(*surface)
            }
        }
    }

    /// True when there is an actual selection (not [`SharedSelection::None`]).
    pub fn is_some(&self) -> bool {
        !matches!(self, SharedSelection::None)
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
            .field("keybind", &self.keybind.map(|k| (k.modifiers, k.logical_key)))
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
        self.order.iter().filter_map(|id| self.by_id.get(id)).collect()
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
    /// The single shared selection model (the focused pane owns the active variant).
    selection: SharedSelection,
    /// The pane that currently owns focus (so a selection publish from a non-focused pane is ignored —
    /// the focused pane is the selection authority).
    focus_owner: Option<PaneId>,
    /// The cross-pane command registry (WRAP-not-fork: runtime melt-together commands only).
    commands: CommandBus,
    /// The in-memory richest-variant clipboard cache: the cross-pane Paste reads THIS first so a
    /// `LoomBlockRef`/`AtelierRef` survives a round-trip the plain-text OS clipboard would flatten.
    clipboard_cache: Option<ClipboardPayload>,
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
    /// WP-KERNEL-012 MT-033 (E5): the content a Route-to-Stage request staged (from a selection / canvas
    /// node / CKC item). The shell drains it via [`Self::take_pending_stage_content`] each frame and
    /// opens/focuses the Stage pane with it. `None` when nothing is pending.
    pending_stage_content: Option<crate::stage_pane::StageContent>,
    /// WP-KERNEL-012 MT-034 (E5): the symbol entity id an Open-Code-Symbol request staged (from a
    /// clicked `[[code:…]]` chip). The shell drains it via [`Self::take_pending_code_symbol`] each frame
    /// and routes it through the MT-030 ShellNavigator `open_code_symbol` seam. `None` when nothing is
    /// pending.
    pending_code_symbol: Option<String>,
    /// WP-KERNEL-012 MT-035 (E5): the ONE unified undo scope every pane shares (POLICY-1..5). Session-
    /// scoped, in-memory only — the bus is held in egui app data which is NOT persisted, so the scope is
    /// empty on restart (POLICY-3). The scope itself cannot serialize (no `Serialize` impl).
    undo_scope: UnifiedUndoScope,
    /// The app's tokio runtime handle, installed by the shell via [`Self::set_undo_runtime`] so the bus
    /// can dispatch a canvas COMPENSATING undo (POLICY-4 `undo_async_fn`) onto the runtime off the egui
    /// frame thread (HBR-QUIET). `None` in a headless unit test (an async undo is then reported as a
    /// typed "no runtime" result rather than blocking the frame — never a fake success).
    undo_runtime: Option<tokio::runtime::Handle>,
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
            selection: SharedSelection::None,
            focus_owner: None,
            commands: CommandBus::default(),
            clipboard_cache: None,
            command_palette_open: false,
            event_sender: None,
            pending_navigation: None,
            pending_stage_content: None,
            pending_code_symbol: None,
            undo_scope: UnifiedUndoScope::new(),
            undo_runtime: None,
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
                tracing::debug!("InteractionBus: try_lock contention this frame; skipping (no deadlock)");
                None
            }
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                // A panicked handler poisoned the lock; recover the guard rather than propagate the
                // poison (the bus state is plain data — a poisoned lock here would wedge every pane).
                Some(f(&mut poisoned.into_inner()))
            }
        }
    }

    /// Install the existing shell event-bus sender so cross-pane notifications fan out through the SAME
    /// `event_bus.rs` channel (WRAP-not-fork).
    pub fn set_event_sender(&mut self, sender: ShellEventSender) {
        self.event_sender = Some(sender);
    }

    // ── Focus ownership ──────────────────────────────────────────────────────────────────────────────

    /// Mark `pane_id` as the focus owner (called by a pane only when it genuinely holds egui focus —
    /// `ui.memory(|m| m.has_focus(pane_egui_id))` — to avoid spurious resets, impl note 6/7).
    pub fn set_focus_owner(&mut self, pane_id: PaneId) {
        self.focus_owner = Some(pane_id);
    }

    /// The current focus owner pane id, if any.
    pub fn focus_owner(&self) -> Option<&PaneId> {
        self.focus_owner.as_ref()
    }

    // ── Shared selection ─────────────────────────────────────────────────────────────────────────────

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
        }
        accept
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

    // ── Command bus (WRAP the registry) ──────────────────────────────────────────────────────────────

    /// Register a cross-pane command. Panes call this once at construction to publish their melt-together
    /// commands (Copy/Cut/Paste/SelectAll/Find/CommandPalette) into the one shared surface.
    pub fn register_command(&mut self, descriptor: CommandDescriptor) {
        self.commands.register(descriptor);
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
    pub fn matching_keybind_command(&self, shortcut: &egui::KeyboardShortcut) -> Option<&'static str> {
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
            keywords: vec!["open".to_owned(), "document".to_owned(), "backlink".to_owned()],
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

    // ── Cross-pane Route-to-Stage navigation (MT-033 melt-together) ────────────────────────────────────

    /// Stage [`crate::stage_pane::StageContent`] for a Route-to-Stage open (called just before
    /// dispatching [`CMD_ROUTE_TO_STAGE`], e.g. from a right-click "Route to Stage" menu item on a
    /// rich-text selection / canvas node). The shell drains it next frame via
    /// [`Self::take_pending_stage_content`] to open/focus the Stage pane with it.
    pub fn request_route_to_stage(&mut self, content: crate::stage_pane::StageContent) {
        self.pending_stage_content = Some(content);
    }

    /// The content staged for a Route-to-Stage open, WITHOUT consuming it (tests / peek).
    pub fn pending_stage_content(&self) -> Option<&crate::stage_pane::StageContent> {
        self.pending_stage_content.as_ref()
    }

    /// Take (and clear) the staged Stage content. The shell calls this each frame; `Some(content)` means
    /// open/focus the Stage pane and set its content, `None` means nothing pending.
    pub fn take_pending_stage_content(&mut self) -> Option<crate::stage_pane::StageContent> {
        self.pending_stage_content.take()
    }

    /// Register the cross-pane Route-to-Stage command (MT-033). Its handler is a no-op on the bus itself
    /// (the stage content was staged by [`Self::request_route_to_stage`] BEFORE dispatch, and is consumed
    /// by the shell drain) — the command exists so a "Route to Stage" menu item dispatches a REAL, named,
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
        self.request_route_to_stage(content);
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
    pub fn open_code_symbol(&mut self, ctx: &egui::Context, symbol_entity_id: impl Into<String>) -> bool {
        self.request_open_code_symbol(symbol_entity_id);
        self.dispatch_command(ctx, CMD_OPEN_CODE_SYMBOL)
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
    /// - `None` when the focused pane's ring is empty (the caller may then try [`Self::undo_cross_pane`]).
    ///
    /// A `Some(result)` whose `!result.ok` should be logged by the caller to the Flight Recorder
    /// (MT-036); this method never panics on a failed undo.
    pub fn undo(&mut self, pane_id: &PaneId) -> Option<UndoResult> {
        let action = self.undo_scope.pop_undo_local(pane_id)?;
        Some(self.invoke_undo(action))
    }

    /// LOCAL redo for `pane_id` (POLICY-1, the Ctrl+Y path). Pops the focused pane's most recently
    /// undone action and re-applies it (sync `redo_fn`, or async `redo_async_fn` for canvas). `None`
    /// when nothing to redo.
    pub fn redo(&mut self, pane_id: &PaneId) -> Option<UndoResult> {
        let action = self.undo_scope.pop_redo_local(pane_id)?;
        Some(self.invoke_redo(action))
    }

    /// CROSS-PANE undo (POLICY-2, the Ctrl+Shift+Z path). Pops the most recent cross-pane action and
    /// invokes its undo (sync or, for a canvas placement, the async compensating call — POLICY-4).
    /// `None` when the cross-pane ring is empty.
    pub fn undo_cross_pane(&mut self) -> Option<UndoResult> {
        let action = self.undo_scope.pop_undo_cross_pane()?;
        Some(self.invoke_undo(action))
    }

    /// CROSS-PANE redo. Pops the most recently undone cross-pane action and re-applies it.
    pub fn redo_cross_pane(&mut self) -> Option<UndoResult> {
        let action = self.undo_scope.pop_redo_cross_pane()?;
        Some(self.invoke_redo(action))
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
            None => UndoResult::err(
                "no tokio runtime installed for canvas compensating undo (set_undo_runtime not called)",
            ),
        }
    }

    /// Register the three unified-undo commands on the cross-pane command bus so they appear in the
    /// command palette AND match their keybinds (Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z). The handlers read the
    /// CURRENT focus owner from the locked bus and dispatch local-first (Ctrl+Z falls back to the
    /// cross-pane ring when the focused pane has nothing to undo — the user's mental model of "undo my
    /// last thing"). Idempotent (last registration wins). Call once when the first editor pane mounts.
    pub fn register_undo_commands(&mut self) {
        self.register_command(CommandDescriptor {
            id: CMD_UNDO,
            name: "Undo",
            label: "Undo".to_owned(),
            keywords: vec!["undo".to_owned(), "revert".to_owned()],
            keybind: default_keybind_for(CMD_UNDO),
            handler: Arc::new(|ctx, bus| {
                // Local-first (POLICY-1): undo the focused pane's last action; fall back to the
                // cross-pane ring so Ctrl+Z always reverts the user's most recent thing.
                let undone = match bus.focus_owner().cloned() {
                    Some(pane_id) => bus.undo(&pane_id).is_some(),
                    None => false,
                };
                if !undone {
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
                let redone = match bus.focus_owner().cloned() {
                    Some(pane_id) => bus.redo(&pane_id).is_some(),
                    None => false,
                };
                if !redone {
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
            Self { last: StdMutex::new(None) }
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
            bus.shared_selection().pane_id().map(|p| p.as_ref().to_owned()),
            Some("pane-code".to_owned())
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
        assert!(bus.command_palette_open(), "the handler opened the palette via the locked bus");
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
        assert_eq!(bus.matching_keybind_command(&ctrl_x), None, "no Cut command registered");
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
            assert_eq!(reentrant, None, "a re-entrant try_lock contends and is skipped, not deadlocked");
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
        assert_eq!(command_list_item_author_id("CommandPalette"), "cmd-CommandPalette");
        // A name with unsafe chars is sanitized (dots/slashes -> '-').
        let id = command_list_item_author_id("weird.name/x");
        assert!(id.starts_with(COMMAND_LIST_ITEM_AUTHOR_PREFIX));
        let suffix = &id[COMMAND_LIST_ITEM_AUTHOR_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
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
        assert!(bus.commands().get(CMD_OPEN_DOCUMENT).is_some(), "open-document command registered");
        assert!(bus.pending_navigation().is_none(), "nothing pending before");
        // The backlink-row click path: stage + dispatch in one call.
        assert!(bus.open_document(&ctx, "DOC-A"), "open-document dispatched");
        assert_eq!(bus.pending_navigation(), Some("DOC-A"), "the staged target is observable");
        // The shell drains it once.
        assert_eq!(bus.take_pending_navigation().as_deref(), Some("DOC-A"));
        assert!(bus.take_pending_navigation().is_none(), "drained once, then empty");
    }

    /// MT-034 AC-2: staging a symbol entity id + dispatching the Open-Code-Symbol command stages the
    /// target on the bus, where the shell drains it and routes it through the ShellNavigator seam. A
    /// real, named, addressable cross-pane action — the symmetric counterpart of Open-Document.
    #[test]
    fn open_code_symbol_stages_target() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.register_open_code_symbol_command();
        assert!(bus.commands().get(CMD_OPEN_CODE_SYMBOL).is_some(), "open-code-symbol command registered");
        assert!(bus.pending_code_symbol().is_none(), "nothing pending before");
        // The clicked code-ref chip path: stage + dispatch in one call.
        assert!(bus.open_code_symbol(&ctx, "ent-42"), "open-code-symbol dispatched");
        assert_eq!(bus.pending_code_symbol(), Some("ent-42"), "the staged symbol id is observable");
        // The shell drains it once.
        assert_eq!(bus.take_pending_code_symbol().as_deref(), Some("ent-42"));
        assert!(bus.take_pending_code_symbol().is_none(), "drained once, then empty");
    }

    /// Dispatching Open-Code-Symbol WITHOUT registering it is a benign false (unknown id), not a panic;
    /// the staged id still drains independently.
    #[test]
    fn open_code_symbol_unregistered_is_benign() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.request_open_code_symbol("ent-X");
        assert!(!bus.dispatch_command(&ctx, CMD_OPEN_CODE_SYMBOL), "unknown command id is a no-op false");
        assert_eq!(bus.take_pending_code_symbol().as_deref(), Some("ent-X"));
    }

    /// Dispatching Open-Document WITHOUT registering it is a benign false (unknown id), not a panic.
    #[test]
    fn open_document_unregistered_is_benign() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        // request_open_document stages even without the command; dispatch returns false (unknown id).
        bus.request_open_document("DOC-X");
        assert!(!bus.dispatch_command(&ctx, CMD_OPEN_DOCUMENT), "unknown command id is a no-op false");
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
        assert!(bus.commands().get(CMD_ROUTE_TO_STAGE).is_some(), "route-to-stage command registered");
        assert!(bus.pending_stage_content().is_none(), "nothing staged before");
        let content =
            crate::stage_pane::StageContent::Selection("hello".to_owned(), "DOC-7".to_owned());
        assert!(bus.route_to_stage(&ctx, content.clone()), "route-to-stage dispatched");
        assert_eq!(bus.pending_stage_content(), Some(&content), "the staged content is observable");
        // The shell drains it once.
        assert_eq!(bus.take_pending_stage_content(), Some(content));
        assert!(bus.take_pending_stage_content().is_none(), "drained once, then empty");
    }

    /// Dispatching Route-to-Stage WITHOUT registering it is a benign false (unknown id), not a panic;
    /// the staged content still drains (the stage is independent of dispatch).
    #[test]
    fn route_to_stage_unregistered_is_benign() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        bus.request_route_to_stage(crate::stage_pane::StageContent::Empty);
        assert!(!bus.dispatch_command(&ctx, CMD_ROUTE_TO_STAGE), "unknown command id is a no-op false");
        assert!(bus.take_pending_stage_content().is_some(), "the staged content still drains");
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
