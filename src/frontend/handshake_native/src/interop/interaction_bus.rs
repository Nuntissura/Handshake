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
