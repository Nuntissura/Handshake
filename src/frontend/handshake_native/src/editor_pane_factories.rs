//! WP-KERNEL-012 MT-079 (E11 host-mount): the session-threaded editor pane factories that mount the
//! REAL native editors into the running `HandshakeApp` shell.
//!
//! ## Why this module exists
//!
//! Through MT-001..MT-068 the code editor (`code_editor::panel::CodeEditorPanel`) and the rich-text
//! editor (`rich_editor::renderer::rich_editor_widget::RichEditorState`) were each built + proven at
//! the egui_kittest WIDGET level, and each ships a thin `PaneFactory` wrapper
//! ([`crate::code_editor::panel::CodeEditorPaneFactory`] /
//! [`crate::rich_editor::renderer::rich_editor_widget::RichEditorPaneFactory`]). But `app.rs` never
//! REGISTERED those factories: `build_default_factories` / `build_factories_with_loom_search_v2`
//! installed a `PlaceholderPaneFactory` for `PaneType::CodeSymbol` (the code surface) and
//! `PaneType::LoomWikiPage` (the Notes surface), so a mounted editor pane rendered a centered
//! placeholder label, never the real editor. This module closes that structural gap.
//!
//! ## What it does (the CORE mount, AC-079-1..AC-079-5)
//!
//! It builds two SESSION-THREADED wrapper factories that:
//!
//! 1. wrap the EXISTING `CodeEditorPaneFactory` / `RichEditorPaneFactory` (no editor logic is
//!    re-implemented — REUSE, not fork);
//! 2. hold a shared [`EditorSessionContext`] cell (active `workspace_id` + tokio `runtime` handle),
//!    threaded in on mount through the SAME `Arc<Mutex<_>>` shared-cell pattern `app.rs` already uses
//!    for `LoomSearchV2PaneFactory` / `FindInFilesPaneFactory` — the `PaneFactory::render` signature
//!    is UNCHANGED (RISK-079-5 / MC-079-3);
//! 3. on the FIRST render with a live session context, call the prior-MT hooks with real session
//!    context: code pane `set_runtime` + `set_workspace_id` (MT-008/010); rich pane
//!    `set_embed_context` (MT-014) + `set_wikilink_context` (MT-057) (AC-079-2 / PT-079-B);
//! 4. wire the shell command `Sender<CodeEditorAction>` into the code pane so Save / Undo / Redo /
//!    OpenCommandPalette reach the WP-011 command bus + MT-035 unified undo (AC-079-3 / PT-079-C);
//! 5. DRAIN `RichEditorState.pending_events` each frame AFTER the editor renders and push the drained
//!    [`EditorEvent`]s into a shared outbound queue ([`RichPaneEvents`]) the shell routes to the nav
//!    bus (WikilinkActivated / BacklinkActivated / TagActivated) (AC-079-5 / PT-079-E).
//!
//! Both editors use interior mutability (`&self` `set_*` methods / `Arc<Mutex<RichEditorState>>`), so
//! threading session context through the established shared-cell pattern needs no trait change.
//!
//! HONESTY (MC-079-5): this module mounts the CORE code + rich panes LIVE. The FULLER mounts
//! (canvas/graph/side panes, MT-060/061/062/063/064/066/067, the MT-043 swarm actions) stay typed
//! carries in the MT `implementation_result` — their panes keep their existing factories / honest
//! empty-states until a follow-on run mounts them. No `todo!()`/`unimplemented!()` is added on any
//! live dispatch path.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::code_editor::keymap::CodeEditorAction;
use crate::code_editor::panel::{CodeEditorPaneFactory, CodeEditorPanel};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::rich_editor::renderer::rich_editor_widget::{RichEditorPaneFactory, RichEditorState};
use crate::rich_editor::wikilinks::inline_view::EditorEvent;

/// The live session context the shell pushes into the editor factories each time the active workspace
/// changes (the SAME shared-cell idea `LoomSearchV2PaneShared` / `FindInFilesPaneShared` use). A factory
/// reads it on render and threads it into its editor's prior-MT `set_*` hooks on mount.
///
/// `None` runtime / empty `workspace_id` is the honest unbound state: a headless/test shell that never
/// installs a context leaves the editor in its existing runtime-less graceful-degradation mode (no
/// perpetual spinner, no panic) exactly as the widget-level tests already prove.
#[derive(Clone, Default)]
pub struct EditorSessionContext {
    /// The active workspace id the editors scope backend lookups to (code-nav, embeds, wikilink
    /// resolution). Empty until the shell installs the active project.
    pub workspace_id: String,
    /// The tokio runtime handle the editors spawn their off-thread backend work onto. `None` until the
    /// shell installs it (the production shell always does; a current-thread test harness may not).
    pub runtime: Option<tokio::runtime::Handle>,
}

impl EditorSessionContext {
    /// A bound context (the production wiring point: `workspace_id` + the app runtime handle).
    pub fn new(workspace_id: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            runtime: Some(runtime),
        }
    }

    /// Whether this context carries enough to thread real session state into an editor (a non-empty
    /// workspace AND a runtime handle). The factory only calls the `set_*` hooks once this is true, so a
    /// half-built context never installs a partial (and misleading) wired state.
    pub fn is_bound(&self) -> bool {
        self.runtime.is_some() && !self.workspace_id.is_empty()
    }
}

/// The shared cell holding the live [`EditorSessionContext`]. The shell owns an `Arc<Mutex<_>>` clone
/// and overwrites it whenever the active workspace changes; each factory holds the SAME `Arc` and reads
/// it on render. This is the established `&self`-render shared-cell threading pattern (the factory map
/// stores `Box<dyn PaneFactory>` and `render` takes `&self`, so per-frame state arrives via this cell,
/// not a `&mut self`).
pub type SharedSessionContext = Arc<Mutex<EditorSessionContext>>;

/// A FNV-1a / lock-free outbound queue of the rich editor's drained [`EditorEvent`]s. The rich pane
/// factory drains `RichEditorState.pending_events` after the editor renders and pushes them here; the
/// shell drains THIS queue once per frame (after the pane host) and routes each event to the MT-030
/// navigation bus (AC-079-5). Keeping the queue here (not inside the editor state) means the editor
/// stays a pure widget and the routing stays the shell's responsibility — the exact ownership split the
/// MT-015 `pending_events` doc comment already names ("routing is owned by the shell").
#[derive(Clone, Default)]
pub struct RichPaneEvents {
    inner: Arc<Mutex<Vec<EditorEvent>>>,
}

impl RichPaneEvents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append the events the rich pane drained this frame (called by [`RichEditorPaneMount::render`]).
    fn push_all(&self, events: Vec<EditorEvent>) {
        if events.is_empty() {
            return;
        }
        if let Ok(mut q) = self.inner.lock() {
            q.extend(events);
        }
    }

    /// Take every queued event (the shell calls this once per frame to route them). Leaves the queue
    /// empty so an event is routed exactly once (no double-route, no leak).
    pub fn take(&self) -> Vec<EditorEvent> {
        match self.inner.lock() {
            Ok(mut q) => std::mem::take(&mut *q),
            Err(p) => std::mem::take(&mut *p.into_inner()),
        }
    }

    /// Whether any event is currently queued (tests / diagnostics).
    pub fn is_empty(&self) -> bool {
        self.inner.lock().map(|q| q.is_empty()).unwrap_or(true)
    }
}

/// The session-threaded CODE-editor pane factory. Registered over `PaneType::CodeSymbol` (the code
/// surface the WP-011 shell already routes a "code" pane to — NOT a new `PaneType` variant, which would
/// ripple through every exhaustive `PaneType` match; RISK-079-5). Wraps the existing
/// [`CodeEditorPaneFactory`] (the real per-frame bus-consumer + undo-recording render) and, on the first
/// render with a bound session context, threads `set_runtime` + `set_workspace_id` into the panel and
/// installs the shell command sender. The wrap keeps the bus/undo wiring the inner factory already
/// proves; this layer only adds the session-context threading the host-mount needs.
pub struct CodeEditorPaneMount {
    inner: CodeEditorPaneFactory,
    /// The Arc-shared panel (the SAME panel the inner factory renders), kept so the mount can call the
    /// `set_*` hooks on it.
    panel: Arc<CodeEditorPanel>,
    /// The live session-context cell the shell overwrites; read on render.
    session: SharedSessionContext,
    /// The command-palette dispatch channel installed into the panel on mount (Save / Undo / Redo /
    /// OpenCommandPalette route here). Held so the install is idempotent (only set once).
    command_sender: std::sync::mpsc::Sender<CodeEditorAction>,
    /// `true` once the panel has been threaded with a BOUND session context (so the threading runs once,
    /// not every frame). Atomic because `render` is `&self`.
    wired: std::sync::atomic::AtomicBool,
    /// `true` once the command sender has been installed into the panel (idempotent).
    command_wired: std::sync::atomic::AtomicBool,
}

impl CodeEditorPaneMount {
    /// Build the mount over `panel`, the live `session` cell, and the shell `command_sender`. The inner
    /// [`CodeEditorPaneFactory`] is constructed from a CLONE of the same `Arc<CodeEditorPanel>`, so the
    /// mount's `set_*` calls and the inner factory's render drive the SAME panel state.
    pub fn new(
        panel: Arc<CodeEditorPanel>,
        session: SharedSessionContext,
        command_sender: std::sync::mpsc::Sender<CodeEditorAction>,
    ) -> Self {
        Self {
            inner: CodeEditorPaneFactory::from_arc(Arc::clone(&panel)),
            panel,
            session,
            command_sender,
            wired: std::sync::atomic::AtomicBool::new(false),
            command_wired: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// The Arc-shared panel behind this mount (so a test/host can drive the SAME panel state the mounted
    /// pane shows — the AC-079 proofs need the real panel behind the factory).
    pub fn panel(&self) -> Arc<CodeEditorPanel> {
        Arc::clone(&self.panel)
    }

    /// Whether the panel has been threaded with a bound session context (tests / PT-079-B).
    pub fn is_wired(&self) -> bool {
        self.wired.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Thread the session context + command sender into the panel if not already done. Called at the top
    /// of `render` (and directly by tests). The command sender is installed unconditionally on the first
    /// render (it works even without a runtime — it is a plain channel); the runtime/workspace threading
    /// waits until the session context is BOUND so a half-built context never installs a misleading wired
    /// state (MC-079-1: the mount is honest about what is actually wired).
    pub fn wire_if_needed(&self) {
        use std::sync::atomic::Ordering;
        if !self.command_wired.swap(true, Ordering::Relaxed) {
            self.panel
                .set_command_palette_sender(self.command_sender.clone());
        }
        if self.wired.load(Ordering::Relaxed) {
            return;
        }
        let ctx = self.session.lock().map(|c| c.clone()).unwrap_or_default();
        if let (true, Some(runtime)) = (ctx.is_bound(), ctx.runtime) {
            self.panel.set_runtime(runtime);
            self.panel.set_workspace_id(ctx.workspace_id);
            self.wired.store(true, Ordering::Relaxed);
        }
    }
}

impl PaneFactory for CodeEditorPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::CodeSymbol
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        // Thread session context + command sender BEFORE the inner render, so the first live frame
        // already has the runtime/workspace/command bus wired (AC-079-2/3).
        self.wire_if_needed();
        // Delegate to the EXISTING code factory render: it publishes selection to the shared bus,
        // registers the code command set, runs the panel, and records the unified-undo entries
        // (push_code_edit_undo) — the real per-frame consumers MT-031/035/050/051 already prove. The
        // mount adds ONLY the session-context threading above; it does not re-implement editor logic.
        self.inner.render(ui, ctx);
    }

    fn accesskit_role(&self) -> accesskit::Role {
        self.inner.accesskit_role()
    }
}

/// The session-threaded RICH-text pane factory. Registered over `PaneType::LoomWikiPage` (the Notes /
/// Obsidian-class wiki surface the WP-011 shell routes the rich editor to — NOT a new `PaneType`
/// variant; RISK-079-5). Wraps the existing [`RichEditorPaneFactory`] (the real per-frame bus-consumer +
/// unified-undo pane-id install) and, on the first render with a bound session context, threads
/// `set_embed_context` (MT-014) + `set_wikilink_context` (MT-057) into the editor state. Each frame,
/// AFTER the editor renders, it DRAINS `RichEditorState.pending_events` and pushes them into the shared
/// [`RichPaneEvents`] queue the shell routes to the nav bus (AC-079-5).
pub struct RichEditorPaneMount {
    inner: RichEditorPaneFactory,
    /// The Arc-shared editor state (the SAME state the inner factory renders), kept so the mount can
    /// thread the `set_*` hooks and drain `pending_events`.
    state: Arc<Mutex<RichEditorState>>,
    /// The live session-context cell the shell overwrites; read on render.
    session: SharedSessionContext,
    /// The outbound queue the shell drains + routes (AC-079-5). The mount pushes the editor's drained
    /// `pending_events` here after each render.
    events: RichPaneEvents,
    /// The document id the rich editor's wikilink context binds to. The Notes pane opens a workspace's
    /// document tree; until a specific document is opened the wikilink context binds to the workspace
    /// root (the create/resolve seam still resolves against the workspace). A future MT that opens a
    /// specific document by tab content_id updates this.
    document_id: String,
    /// `true` once the editor state has been threaded with a BOUND session context.
    wired: std::sync::atomic::AtomicBool,
}

impl RichEditorPaneMount {
    /// Build the mount over the shared editor `state`, the live `session` cell, the shared outbound
    /// `events` queue, and the `document_id` the wikilink context binds to. The inner
    /// [`RichEditorPaneFactory`] wraps a CLONE of the same `Arc<Mutex<RichEditorState>>` so the mount's
    /// threading + drain and the inner render drive the SAME state.
    pub fn new(
        state: Arc<Mutex<RichEditorState>>,
        session: SharedSessionContext,
        events: RichPaneEvents,
        document_id: impl Into<String>,
    ) -> Self {
        Self {
            inner: RichEditorPaneFactory::new(Arc::clone(&state)),
            state,
            session,
            events,
            document_id: document_id.into(),
            wired: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// The Arc-shared editor state behind this mount (so a test/host drives the SAME state the mounted
    /// pane shows — the AC-079 proofs need the real state behind the factory).
    pub fn state(&self) -> Arc<Mutex<RichEditorState>> {
        Arc::clone(&self.state)
    }

    /// The shared outbound event queue (the shell holds a clone to drain + route).
    pub fn events(&self) -> RichPaneEvents {
        self.events.clone()
    }

    /// Whether the editor state has been threaded with a bound session context (tests / PT-079-B).
    pub fn is_wired(&self) -> bool {
        self.wired.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Thread the session context into the editor state if not already done. Called at the top of
    /// `render` (and directly by tests). Waits until the session context is BOUND (non-empty workspace +
    /// runtime) so a half-built context never installs a misleading wired state. Calls the prior-MT
    /// hooks `set_embed_context` (MT-014) + `set_wikilink_context` (MT-057) — REUSE, not re-implement.
    pub fn wire_if_needed(&self) {
        use std::sync::atomic::Ordering;
        if self.wired.load(Ordering::Relaxed) {
            return;
        }
        let ctx = self.session.lock().map(|c| c.clone()).unwrap_or_default();
        if !ctx.is_bound() {
            return;
        }
        let Some(runtime) = ctx.runtime else { return };
        if let Ok(mut s) = self.state.lock() {
            s.set_embed_context(ctx.workspace_id.clone(), runtime.clone());
            s.set_wikilink_context(ctx.workspace_id, self.document_id.clone(), runtime);
        }
        self.wired.store(true, Ordering::Relaxed);
    }

    /// Drain the editor's `pending_events` into the shared outbound queue (AC-079-5). Called AFTER the
    /// inner render so a click handled THIS frame is routed THIS frame. Pushing them to the shared queue
    /// (rather than routing here) keeps the editor a pure widget and the routing the shell's job.
    fn drain_events(&self) {
        let drained = match self.state.lock() {
            Ok(mut s) => std::mem::take(&mut s.pending_events),
            Err(p) => std::mem::take(&mut p.into_inner().pending_events),
        };
        self.events.push_all(drained);
    }
}

impl PaneFactory for RichEditorPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomWikiPage
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        // Thread session context BEFORE the inner render, so the first live frame already has the
        // embed + wikilink context wired (AC-079-2).
        self.wire_if_needed();
        // WP-KERNEL-012 MT-055 REMEDIATION (reading mode reachable in the mounted editor): render the
        // Edit|Reading segmented toggle in the mounted editor CHROME (above the editor body), persist the
        // choice per document in the egui-persisted `ReadingModeStore`, and pass `store.get(document_id)`
        // into the widget's read-only flag. The store key is the open document's content id (per-document
        // isolation — RISK-004/MC-004); a fresh Notes pane with no document yet keys on its stable pane id
        // so the toggle is still operable there without leaking state onto a later real document.
        let doc_key = ctx
            .record
            .content_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(|id| id.to_owned())
            .unwrap_or_else(|| format!("pane:{}", ctx.record.pane_id));
        let mut store = crate::rich_editor::reading_mode::reading_mode_store(ui.ctx());
        let mode = crate::rich_editor::reading_mode::view_mode_toggle(ui, &doc_key, &mut store);
        crate::rich_editor::reading_mode::write_reading_mode_store(ui.ctx(), &store);
        if mode.is_read_only() {
            // Reading view: render the SAME shared state through the widget's read-only branch (MT-055's
            // `with_read_only` path — no second renderer). The editable inner-factory render is skipped
            // this frame: reading mode applies no edit dispatch, so the per-frame bus command
            // registration/selection publish (an editable-surface concern) honestly pauses with it.
            crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget::new(Arc::clone(
                &self.state,
            ))
            .with_read_only(true)
            .show(ui);
        } else {
            // Delegate to the EXISTING rich factory render: it installs the unified-undo pane id,
            // publishes selection to the shared bus, registers the rich command set, and runs the editor
            // widget — the real per-frame consumers MT-031/035 already prove. The mount adds session
            // threading + the pending_events drain; it does not re-implement editor logic.
            self.inner.render(ui, ctx);
        }
        // DRAIN + route (AC-079-5): the editor enqueued any WikilinkActivated/BacklinkActivated/
        // TagActivated this frame; move them to the shell's outbound queue so the shell routes them to
        // the nav bus after the pane host. No event is left unrouted (reading mode keeps link chips
        // interactive, so the drain runs in both branches).
        self.drain_events();
    }

    fn accesskit_role(&self) -> accesskit::Role {
        self.inner.accesskit_role()
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-080 (E11 host-mount, part 2): the SECONDARY pane factories.
//
// MT-079 mounted the CORE code + rich editors. This MT mounts the rest of the widget-proven panes —
// the canvas board (MT-026), the graph view (MT-021/060), and the side panes (outgoing-links MT-062,
// relevant-memory MT-063, Stage MT-066, daily-journal MT-067, manual MT-073) — over their
// `PlaceholderPaneFactory` entries so they render LIVE in the running shell.
//
// SAME shared-cell pattern as MT-079: each factory holds an `Arc<Mutex<_>>` to the widget state the
// shell also owns, so the shell drives the SAME state the mounted pane shows (the AC-080 proofs need
// the real widget behind the factory) and a `&self` `render` reads the live palette each frame. The
// `PaneFactory` trait signature is UNCHANGED (RISK-080-5 / MC-080-3). No widget logic is
// re-implemented — every factory CALLS the existing widget's `show`.
//
// HONESTY (MC-080-5 / Spec-Realism Gate): every factory below is CONSUMED by the live render loop
// (registered in `app.rs` over a placeholder and rendered each frame). Where a backend route is absent
// (FEMS/Stage/Calendar/Locus), the wrapped widget shows its own honest empty-state — no factory fakes a
// live wiring, and none uses `todo!()`/`unimplemented!()` on a live path.
// ════════════════════════════════════════════════════════════════════════════════════════════════

use crate::theme::HsPalette;

/// The live theme palette the shell pushes into the secondary pane factories each frame (the widgets
/// read theme tokens, never hardcoded hex — CONTROL-4). One shared cell shared by every secondary
/// factory; the shell overwrites it from the active theme each frame, exactly like the MT-079 session
/// cell. Starts at the dark palette so a headless/test render (which may not push a palette) still has
/// real tokens.
pub type SharedPalette = Arc<Mutex<HsPalette>>;

/// Read the current palette out of the shared cell (a clone, so the lock is released before render).
fn palette_of(cell: &SharedPalette) -> HsPalette {
    cell.lock()
        .map(|p| p.clone())
        .unwrap_or_else(|p| p.into_inner().clone())
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-026): the live CANVAS-board pane factory. Registered over
/// `PaneType::AtelierEditor` (the canvas/atelier surface the shell already routes a canvas-id open to).
/// Wraps the existing [`crate::graph::canvas_board::LoomCanvasBoard`] widget and renders it each frame;
/// any [`crate::graph::canvas_board::CanvasEvent`] the board dispatches this frame is pushed into a shared
/// outbound queue the shell drains + maps to the real canvas PATCH/POST (AC-080-2). The board state is the
/// SAME `Arc<Mutex<_>>` the shell holds, so the shell's getCanvasBoard refresh feeds back into the pane.
pub struct CanvasBoardPaneMount {
    board: Arc<Mutex<crate::graph::canvas_board::LoomCanvasBoard>>,
    palette: SharedPalette,
    /// The outbound queue of canvas events the shell drains each frame (the move/resize/section/edit-card
    /// gestures the host turns into real PATCH/POST via the MT-026 `CanvasBoardClient`).
    events: Arc<Mutex<Vec<crate::graph::canvas_board::CanvasEvent>>>,
}

impl CanvasBoardPaneMount {
    pub fn new(
        board: Arc<Mutex<crate::graph::canvas_board::LoomCanvasBoard>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::canvas_board::CanvasEvent>>>,
    ) -> Self {
        Self {
            board,
            palette,
            events,
        }
    }
}

impl PaneFactory for CanvasBoardPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::AtelierEditor
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        // Render the REAL board (the toolbar + placements + AccessKit `canvas.*` subtree). The widget owns
        // its own per-frame consumers; the mount only collects the dispatched events for the shell.
        let mut event = None;
        if let Ok(mut board) = self.board.lock() {
            event = board.show(ui, &palette);
            // Also drain any swarm-dispatched knowledge events the single `show` return cannot carry
            // (the MT-042 anti-scaffolding drain) so a canvas dispatch reaches the shell too.
            let drained = board.drain_knowledge_events();
            if !drained.is_empty() {
                if let Ok(mut q) = self.events.lock() {
                    q.extend(drained);
                }
            }
        }
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Group
    }
}

/// WP-KERNEL-012 MT-080 REMEDIATION (PaneType collision fix): the live GRAPH-view pane factory. Now
/// registered over its OWN key `PaneType::Placeholder("Graph View")` — NOT `PaneType::KernelDcc`. The old
/// KernelDcc registration hijacked the quick-switcher WP/MT navigation (open_work_packet / open_micro_task
/// open `KernelDcc` tabs with `WP:`/`MT:` content ids) by rendering the generic graph view with the WP/MT
/// id ignored. KernelDcc now falls back to the honest content-aware placeholder (which SHOWS the WP/MT id)
/// and the graph view opens via its own operator route (`view.graph` palette command / VIEW menu). Wraps
/// the existing [`crate::graph::graph_view::LoomGraphView`] and renders it each frame; any
/// [`crate::graph::graph_view::GraphEvent`] (notably `DepthChanged`) is pushed into a shared outbound
/// queue the shell drains to re-query the depth-parameterized graph-search (AC-080-3).
pub struct GraphViewPaneMount {
    view: Arc<Mutex<crate::graph::graph_view::LoomGraphView>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::graph_view::GraphEvent>>>,
}

impl GraphViewPaneMount {
    pub fn new(
        view: Arc<Mutex<crate::graph::graph_view::LoomGraphView>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::graph_view::GraphEvent>>>,
    ) -> Self {
        Self {
            view,
            palette,
            events,
        }
    }
}

impl PaneFactory for GraphViewPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(GRAPH_VIEW_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let mut event = None;
        if let Ok(mut view) = self.view.lock() {
            event = view.show(ui, &palette);
            let drained = view.drain_knowledge_events();
            if !drained.is_empty() {
                if let Ok(mut q) = self.events.lock() {
                    q.extend(drained);
                }
            }
        }
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

/// WP-KERNEL-012 MT-080 REMEDIATION (PaneType collision fix): the live OUTGOING-LINKS side pane. Now
/// registered over its OWN key `PaneType::Placeholder("Outgoing Links")` — NOT `PaneType::LoomBlock`. The
/// old LoomBlock registration made EVERY loom-block open (quick-switcher hit, wikilink chip, search result)
/// render the same content-blind OutgoingLinksPanel instead of block-appropriate content, hijacking
/// navigation. LoomBlock now falls back to the honest content-aware placeholder (which SHOWS the block id)
/// and the outgoing-links pane opens via its own operator route. Wraps the existing
/// [`crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel`]; an `on_open(NavTarget)`
/// click is pushed into a shared outbound queue the shell routes to the MT-030 nav bus.
pub struct OutgoingLinksPaneMount {
    panel: Arc<Mutex<crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel>>,
    palette: SharedPalette,
    nav: Arc<Mutex<Vec<crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget>>>,
}

impl OutgoingLinksPaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel>>,
        palette: SharedPalette,
        nav: Arc<Mutex<Vec<crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget>>>,
    ) -> Self {
        Self {
            panel,
            palette,
            nav,
        }
    }
}

impl PaneFactory for OutgoingLinksPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(OUTGOING_LINKS_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let nav = Arc::clone(&self.nav);
        if let Ok(mut panel) = self.panel.lock() {
            let mut on_open =
                |target: crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget| {
                    if let Ok(mut q) = nav.lock() {
                        q.push(target);
                    }
                };
            panel.show(ui, &palette, &mut on_open);
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-063): the live RELEVANT-MEMORY side pane. Registered over
/// `PaneType::Placeholder("Relevant Memory")` (the distinct placeholder key the pane registers under).
/// Wraps the existing [`crate::fems::relevant_memory_panel::RelevantMemoryPanel`]; a "Go to source" click
/// routes through the shared nav queue. The FEMS read route is ABSENT in the current backend, so the panel
/// renders its own `EndpointMissing` empty-state — the mount never fakes a pack.
pub struct RelevantMemoryPaneMount {
    panel: Arc<Mutex<crate::fems::relevant_memory_panel::RelevantMemoryPanel>>,
    palette: SharedPalette,
    nav: Arc<Mutex<Vec<crate::fems::relevant_memory_panel::MemoryNavTarget>>>,
}

impl RelevantMemoryPaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::fems::relevant_memory_panel::RelevantMemoryPanel>>,
        palette: SharedPalette,
        nav: Arc<Mutex<Vec<crate::fems::relevant_memory_panel::MemoryNavTarget>>>,
    ) -> Self {
        Self {
            panel,
            palette,
            nav,
        }
    }
}

impl PaneFactory for RelevantMemoryPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder("Relevant Memory".to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        use crate::fems::relevant_memory_panel::FnNavigationBus;
        let palette = palette_of(&self.palette);
        let nav = Arc::clone(&self.nav);
        if let Ok(mut panel) = self.panel.lock() {
            let mut bus = FnNavigationBus(|target| {
                if let Ok(mut q) = nav.lock() {
                    q.push(target);
                }
            });
            panel.show(ui, &palette, &mut bus);
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-066): the live STAGE pane. Registered over
/// `PaneType::Placeholder("Stage")`. Wraps the existing [`crate::stage_pane::StagePane`] full round-trip
/// surface; the embed-back action is signalled through a shared flag the shell drains. The Stage embed-back
/// HTTP route is ABSENT, so the embed action surfaces the honest typed blocker — never a faked embed.
pub struct StagePaneMount {
    pane: Arc<Mutex<crate::stage_pane::StagePane>>,
    palette: SharedPalette,
    /// Set true on the frame the operator/agent pressed "Embed back into note" so the shell can surface
    /// the typed blocker / route it once.
    embed_requested: Arc<std::sync::atomic::AtomicBool>,
}

impl StagePaneMount {
    pub fn new(
        pane: Arc<Mutex<crate::stage_pane::StagePane>>,
        palette: SharedPalette,
        embed_requested: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            pane,
            palette,
            embed_requested,
        }
    }
}

impl PaneFactory for StagePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder("Stage".to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        if let Ok(mut pane) = self.pane.lock() {
            let embed = pane.show_round_trip(ui, &palette);
            if embed {
                self.embed_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-067): the live DAILY-JOURNAL pane. Registered over
/// `PaneType::LoomDailyJournal`. Wraps the existing [`crate::graph::daily_journal_panel::DailyJournalPanel`]
/// (stateless `show`) over a shared [`crate::graph::daily_journal_panel::DailyJournalState`]; a date-nav
/// signal is pushed into a shared outbound queue the shell maps to `open_or_create_daily_note` (AC-080-5).
pub struct DailyJournalPaneMount {
    state: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
    /// WP-KERNEL-012 MT-019 REMEDIATION (journal EDITING surface folded into the LoomDailyJournal pane
    /// host): the shared MT-019 journal panel state (`JournalStore` + embedded rich editor + 3s
    /// auto-save). `None` until the shell binds it on the first frame with a live runtime + workspace
    /// (the store's production backend spawns off-thread loads/saves) — the honest unbound state renders
    /// a disclosure line, never a fake editor.
    journal: SharedJournalPanel,
}

/// The one-slot shared cell holding the BOUND MT-019 journal panel state (`None` until the shell binds
/// the production store). The inner `Arc<Mutex<JournalPanelState>>` is the exact handle
/// [`crate::rich_editor::daily_notes::journal_panel::JournalPanelWidget`] renders through, so the mount
/// and any test drive the SAME state across frames.
pub type SharedJournalPanel = Arc<
    Mutex<Option<Arc<Mutex<crate::rich_editor::daily_notes::journal_panel::JournalPanelState>>>>,
>;

impl DailyJournalPaneMount {
    pub fn new(
        state: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
        journal: SharedJournalPanel,
    ) -> Self {
        Self {
            state,
            palette,
            events,
            journal,
        }
    }
}

impl PaneFactory for DailyJournalPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomDailyJournal
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        use crate::graph::daily_journal_panel::{DailyJournalEvent, DailyJournalPanel};
        let palette = palette_of(&self.palette);
        if let Ok(mut state) = self.state.lock() {
            let event = DailyJournalPanel::show(ui, &mut state, &palette);
            if !matches!(event, DailyJournalEvent::None) {
                if let Ok(mut q) = self.events.lock() {
                    q.push(event);
                }
            }
        }
        // MT-019: the journal EDITING surface (open/create today's note + embedded editor + auto-save),
        // folded below the MT-067 calendar-interop header. The JournalPanelWidget drives its own store
        // drain / edit-detection / auto-save each frame. Rendered only once the shell bound the
        // production store (runtime + workspace available); until then an honest disclosure renders.
        let journal = self
            .journal
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(Arc::clone));
        match journal {
            Some(journal_state) => {
                crate::rich_editor::daily_notes::journal_panel::JournalPanelWidget::new(
                    journal_state,
                )
                .show(ui);
            }
            None => {
                ui.label(
                    egui::RichText::new(
                        "Journal editor binds when a workspace and runtime are active.",
                    )
                    .color(palette.text_subtle),
                );
            }
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-073): the live USER-MANUAL pane. Registered over
/// `PaneType::UserManual`. Wraps the existing [`crate::manual_pane::ManualPane`] over a shared
/// [`crate::manual_pane::ManualRegistry`] (immutable content) + [`crate::manual_pane::ManualPaneState`]
/// (search/selection). Pure in-pane widget (no backend) — it always renders its real `manual-pane` subtree.
pub struct ManualPaneMount {
    registry: Arc<crate::manual_pane::ManualRegistry>,
    state: Arc<Mutex<crate::manual_pane::ManualPaneState>>,
    palette: SharedPalette,
}

impl ManualPaneMount {
    pub fn new(
        registry: Arc<crate::manual_pane::ManualRegistry>,
        state: Arc<Mutex<crate::manual_pane::ManualPaneState>>,
        palette: SharedPalette,
    ) -> Self {
        Self {
            registry,
            state,
            palette,
        }
    }
}

impl PaneFactory for ManualPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::UserManual
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        if let Ok(mut state) = self.state.lock() {
            crate::manual_pane::ManualPane::new(&self.registry, &mut state, &palette).show(ui);
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Region
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 E11 remediation wave (lane W1 — shell host wiring): the ORPHAN-WIDGET mounts.
//
// The 2026-07-02 per-MT drift audit found a class of widget-proven surfaces with NO host mount and NO
// operator open route (MT-022 folder tree, MT-023 tags, MT-024 sidebar/pins, MT-025/059 wiki page,
// MT-027 block collections, MT-056 outline, MT-036 flight recorder, MT-009 diff/merge). Each mount below
// follows the exact MT-079/080 shared-cell pattern: the shell owns the SAME `Arc<Mutex<_>>` state the
// registered factory renders, plus a shared outbound event queue the shell drains + routes each frame.
// Every mount is keyed on its OWN `PaneType::Placeholder(<label>)` key (the established side-pane keying:
// Relevant Memory / Stage) so no mount collides with a content-addressed navigation PaneType.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Stable pane-key labels for the Placeholder-keyed side panes (single source of truth shared by the
/// factory registrations, the `view.*` open commands in `app.rs`, and the route kittests).
pub const OUTGOING_LINKS_PANE_LABEL: &str = "Outgoing Links";
pub const GRAPH_VIEW_PANE_LABEL: &str = "Graph View";
pub const TAGS_PANE_LABEL: &str = "Tags";
pub const SIDEBAR_PANE_LABEL: &str = "Sidebar";
pub const BLOCK_COLLECTIONS_PANE_LABEL: &str = "Block Collections";
pub const OUTLINE_PANE_LABEL: &str = "Outline";
pub const WIKI_PAGE_PANE_LABEL: &str = "Wiki Page";
pub const FOLDER_TREE_PANE_LABEL: &str = "Folders";
pub const DIFF_MERGE_PANE_LABEL: &str = "Diff Merge";
pub const RELEVANT_MEMORY_PANE_LABEL: &str = "Relevant Memory";
pub const STAGE_PANE_LABEL: &str = "Stage";

/// The `PaneType` key for a Placeholder-keyed side pane label (convenience shared by app + tests).
pub fn placeholder_pane_type(label: &str) -> PaneType {
    PaneType::Placeholder(label.to_owned())
}

// ── MT-023: Tags panel + Tag Hub ─────────────────────────────────────────────────────────────────────

/// One drained tags-pane event: either a list-panel event or a hub-page event (the two MT-023 widgets
/// share one mounted pane; the hub opens over the list when the host consumes `OpenTag`).
#[derive(Debug, Clone)]
pub enum TagsPaneEvent {
    Panel(crate::graph::tags_panel::TagsPanelEvent),
    Hub(crate::graph::tags_panel::TagHubEvent),
    /// The operator pressed the "All tags" back affordance while a hub was open: the host clears the
    /// bound hub (pure UI state — no backend call).
    BackToList,
}

/// WP-KERNEL-012 MT-023 REMEDIATION: the live TAGS side pane (list + hub). Registered over
/// `PaneType::Placeholder("Tags")`. Renders the bound [`crate::graph::tags_panel::LoomTagHubPanel`] when
/// one is open, else the [`crate::graph::tags_panel::LoomTagsPanel`] list; every widget event is pushed
/// into the shared outbound queue the shell routes to the MT-023 `LoomTagClient` (fetch/open/tag-edge).
pub struct TagsPaneMount {
    tags: Arc<Mutex<crate::graph::tags_panel::LoomTagsPanel>>,
    hub: Arc<Mutex<Option<crate::graph::tags_panel::LoomTagHubPanel>>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<TagsPaneEvent>>>,
}

impl TagsPaneMount {
    pub fn new(
        tags: Arc<Mutex<crate::graph::tags_panel::LoomTagsPanel>>,
        hub: Arc<Mutex<Option<crate::graph::tags_panel::LoomTagHubPanel>>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<TagsPaneEvent>>>,
    ) -> Self {
        Self {
            tags,
            hub,
            palette,
            events,
        }
    }
}

impl PaneFactory for TagsPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(TAGS_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let mut out: Option<TagsPaneEvent> = None;
        let hub_open = self.hub.lock().map(|h| h.is_some()).unwrap_or(false);
        if hub_open {
            // Back affordance so the operator is never stuck on a hub page (stable AccessKit address).
            let back = ui.button(egui::RichText::new("< All tags").color(palette.text));
            ui.ctx().accesskit_node_builder(back.id, |node| {
                node.set_author_id("tags.back-to-list".to_owned());
            });
            if back.clicked() {
                out = Some(TagsPaneEvent::BackToList);
            }
            if let Ok(mut hub) = self.hub.lock() {
                if let Some(hub) = hub.as_mut() {
                    if let Some(ev) = hub.show(ui, &palette) {
                        out = Some(TagsPaneEvent::Hub(ev));
                    }
                }
            }
        } else if let Ok(mut tags) = self.tags.lock() {
            if let Some(ev) = tags.show(ui, &palette) {
                out = Some(TagsPaneEvent::Panel(ev));
            }
        }
        if let Some(ev) = out {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

// ── MT-024: Sidebar (pins / favorites / backlinks / unlinked / breadcrumbs) ──────────────────────────

/// WP-KERNEL-012 MT-024 REMEDIATION: the live SIDEBAR pane. Registered over
/// `PaneType::Placeholder("Sidebar")`. Wraps the existing
/// [`crate::graph::sidebar_panel::LoomSidebarPanel`]; every [`crate::graph::sidebar_panel::SidebarEvent`]
/// is pushed into the shared outbound queue the shell routes to the `LoomSidebarClient` (two-call pin
/// removal, unfavorite PATCH, section re-fetch) + the `ShellEvent::BookmarkRemoved` emission.
pub struct SidebarPaneMount {
    panel: Arc<Mutex<crate::graph::sidebar_panel::LoomSidebarPanel>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::sidebar_panel::SidebarEvent>>>,
}

impl SidebarPaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::graph::sidebar_panel::LoomSidebarPanel>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::sidebar_panel::SidebarEvent>>>,
    ) -> Self {
        Self {
            panel,
            palette,
            events,
        }
    }
}

impl PaneFactory for SidebarPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(SIDEBAR_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let event = match self.panel.lock() {
            Ok(mut p) => p.show(ui, &palette),
            Err(_) => None,
        };
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

// ── MT-027: Block Collections (table / kanban / calendar) ────────────────────────────────────────────

/// WP-KERNEL-012 MT-027 REMEDIATION: the live BLOCK-COLLECTIONS pane. Registered over
/// `PaneType::Placeholder("Block Collections")`. Wraps the existing
/// [`crate::graph::block_collection_view::BlockCollectionView`]; every
/// [`crate::graph::block_collection_view::BlockViewEvent`] (from the `show` return AND the MT-042 swarm
/// drain) is pushed into the shared outbound queue the shell routes to the `BlockViewClient`.
pub struct BlockCollectionPaneMount {
    view: Arc<Mutex<crate::graph::block_collection_view::BlockCollectionView>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::block_collection_view::BlockViewEvent>>>,
}

impl BlockCollectionPaneMount {
    pub fn new(
        view: Arc<Mutex<crate::graph::block_collection_view::BlockCollectionView>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::block_collection_view::BlockViewEvent>>>,
    ) -> Self {
        Self {
            view,
            palette,
            events,
        }
    }
}

impl PaneFactory for BlockCollectionPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(BLOCK_COLLECTIONS_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let mut drained: Vec<crate::graph::block_collection_view::BlockViewEvent> = Vec::new();
        if let Ok(mut view) = self.view.lock() {
            if let Some(ev) = view.show(ui, &palette) {
                drained.push(ev);
            }
            // MT-042 swarm dispatches the single Option return cannot carry.
            drained.extend(view.drain_knowledge_events());
        }
        if !drained.is_empty() {
            if let Ok(mut q) = self.events.lock() {
                q.extend(drained);
            }
        }
    }
}

// ── MT-056: Outline / table-of-contents side pane ────────────────────────────────────────────────────

/// WP-KERNEL-012 MT-056 REMEDIATION: the live OUTLINE side pane. Registered over
/// `PaneType::Placeholder("Outline")`. Wraps the existing
/// [`crate::rich_editor::outline_panel::OutlinePanel`] over the SAME mounted rich-editor state the Notes
/// pane renders, so heading clicks scroll the REAL mounted document (the panel stages the scroll target
/// on the shared state itself — no outbound queue needed).
pub struct OutlinePaneMount {
    panel: Arc<Mutex<crate::rich_editor::outline_panel::OutlinePanel>>,
    rich_state: Arc<Mutex<RichEditorState>>,
}

impl OutlinePaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::rich_editor::outline_panel::OutlinePanel>>,
        rich_state: Arc<Mutex<RichEditorState>>,
    ) -> Self {
        Self { panel, rich_state }
    }
}

impl PaneFactory for OutlinePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(OUTLINE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        if let Ok(mut panel) = self.panel.lock() {
            // Sync the outline from the live document FIRST (cheap hash-guarded rebuild), then render.
            // The sync borrow is dropped before `show` re-locks the state for the click path.
            {
                let state = match self.rich_state.lock() {
                    Ok(s) => Some(s),
                    Err(_) => None,
                };
                if let Some(state) = state {
                    panel.sync(&state);
                }
            }
            panel.show(ui, &self.rich_state);
        }
    }
}

// ── MT-025/059: Loom wiki-projection page pane ───────────────────────────────────────────────────────

/// A host request drained from the wiki pane: the shell maps each to the verified `LoomWikiClient`
/// routes (GET load / POST overlays / POST regenerate) and re-delivers into the bound panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WikiPaneRequest {
    /// A (re)load is needed for `projection_id` (first bind, Retry, or after a save/regenerate).
    Load { projection_id: String },
    /// The Save button was pressed with `annotation` (the verified overlay-annotation write).
    Save {
        projection_id: String,
        annotation: String,
    },
    /// The Rebuild button was pressed (`POST /loom/wiki/{id}/regenerate`).
    Regenerate { projection_id: String },
}

/// WP-KERNEL-012 MT-025/059 REMEDIATION: the live WIKI-PAGE pane. Registered over its OWN key
/// `PaneType::Placeholder("Wiki Page")` — `open_wiki_page` now routes wiki projection ids HERE instead of
/// feeding them into the rich-document loader (the audited nav misroute). The mount binds one
/// [`crate::graph::wiki_page_panel::LoomWikiPagePanel`] per open projection id (rebinding when the tab's
/// `content_id` changes) and pushes load/save/regenerate requests into the shared outbound queue.
pub struct WikiPagePaneMount {
    /// The bound panel + its projection id (`None` until a wiki tab with a content id renders).
    bound: Arc<Mutex<Option<(String, crate::graph::wiki_page_panel::LoomWikiPagePanel)>>>,
    session: SharedSessionContext,
    palette: SharedPalette,
    requests: Arc<Mutex<Vec<WikiPaneRequest>>>,
}

impl WikiPagePaneMount {
    pub fn new(
        bound: Arc<Mutex<Option<(String, crate::graph::wiki_page_panel::LoomWikiPagePanel)>>>,
        session: SharedSessionContext,
        palette: SharedPalette,
        requests: Arc<Mutex<Vec<WikiPaneRequest>>>,
    ) -> Self {
        Self {
            bound,
            session,
            palette,
            requests,
        }
    }

    fn push_request(&self, req: WikiPaneRequest) {
        if let Ok(mut q) = self.requests.lock() {
            q.push(req);
        }
    }
}

impl PaneFactory for WikiPagePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(WIKI_PAGE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let projection_id = ctx
            .record
            .content_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(|id| id.to_owned());
        let Some(projection_id) = projection_id else {
            // Honest empty state: a wiki pane with no projection bound (no fake page).
            ui.label(
                egui::RichText::new("No wiki page open. Open one via the quick switcher.")
                    .color(palette.text_subtle),
            );
            return;
        };
        let workspace_id = self
            .session
            .lock()
            .map(|s| s.workspace_id.clone())
            .unwrap_or_default();
        if let Ok(mut bound) = self.bound.lock() {
            let needs_rebind = bound
                .as_ref()
                .map(|(bound_id, _)| bound_id != &projection_id)
                .unwrap_or(true);
            if needs_rebind {
                *bound = Some((
                    projection_id.clone(),
                    crate::graph::wiki_page_panel::LoomWikiPagePanel::new(
                        workspace_id,
                        projection_id.clone(),
                    ),
                ));
                // First bind: request the real GET load (the shell fires the LoomWikiClient fetch).
                self.push_request(WikiPaneRequest::Load {
                    projection_id: projection_id.clone(),
                });
            }
            if let Some((_, panel)) = bound.as_mut() {
                use crate::graph::wiki_page_panel::WikiPageEvent;
                if let Some(event) = panel.show(ui, &palette) {
                    match event {
                        WikiPageEvent::Save { annotation } => {
                            self.push_request(WikiPaneRequest::Save {
                                projection_id: projection_id.clone(),
                                annotation,
                            });
                        }
                        WikiPageEvent::Rebuild => {
                            self.push_request(WikiPaneRequest::Regenerate {
                                projection_id: projection_id.clone(),
                            });
                        }
                        WikiPageEvent::Retry => {
                            self.push_request(WikiPaneRequest::Load {
                                projection_id: projection_id.clone(),
                            });
                        }
                        // Edit/Cancel are local panel state (observability-only events).
                        WikiPageEvent::EditBegan | WikiPageEvent::Cancel => {}
                    }
                }
            }
        }
    }
}

// ── MT-022: Loom folder tree pane ────────────────────────────────────────────────────────────────────

/// WP-KERNEL-012 MT-022 REMEDIATION: the live FOLDER-TREE pane. Registered over
/// `PaneType::Placeholder("Folders")`. Wraps the existing [`crate::graph::folder_tree::LoomFolderTree`];
/// every [`crate::graph::folder_tree::FolderTreeEvent`] (lazy-fetch expand, recolor, open, retry) is
/// pushed into the shared outbound queue the shell routes to the `LoomFolderClient`.
pub struct FolderTreePaneMount {
    tree: Arc<Mutex<crate::graph::folder_tree::LoomFolderTree>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::folder_tree::FolderTreeEvent>>>,
}

impl FolderTreePaneMount {
    pub fn new(
        tree: Arc<Mutex<crate::graph::folder_tree::LoomFolderTree>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::folder_tree::FolderTreeEvent>>>,
    ) -> Self {
        Self {
            tree,
            palette,
            events,
        }
    }
}

impl PaneFactory for FolderTreePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(FOLDER_TREE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let event = match self.tree.lock() {
            Ok(mut t) => t.show(ui, &palette),
            Err(_) => None,
        };
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

// ── MT-009: Diff / merge editor pane (own key — no CodeSymbol collision) ─────────────────────────────

/// WP-KERNEL-012 MT-009 REMEDIATION: the live DIFF/MERGE pane, registered over its OWN key
/// `PaneType::Placeholder("Diff Merge")`. The widget's own `DiffEditorPaneFactory::pane_type()` returns
/// `PaneType::CodeSymbol`, which would REPLACE the mounted code editor if registered directly — this
/// mount gives the diff surface its own registry key instead (the factory-map key routes rendering; the
/// inner `pane_type()` is never used for routing here). The slot holds the currently-open
/// [`crate::code_editor::DiffEditorPanel`] (set by the shell's open-diff/open-merge routes); an empty
/// slot renders an honest empty state, never a fake diff.
pub struct DiffMergePaneMount {
    slot: Arc<Mutex<Option<Arc<crate::code_editor::DiffEditorPanel>>>>,
    palette: SharedPalette,
}

impl DiffMergePaneMount {
    pub fn new(
        slot: Arc<Mutex<Option<Arc<crate::code_editor::DiffEditorPanel>>>>,
        palette: SharedPalette,
    ) -> Self {
        Self { slot, palette }
    }
}

impl PaneFactory for DiffMergePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(DIFF_MERGE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let panel = self.slot.lock().ok().and_then(|s| s.clone());
        match panel {
            Some(panel) => {
                panel.show(ui);
            }
            None => {
                let resp = ui.label(
                    egui::RichText::new(
                        "No diff or merge open. Open one from a conflict dialog or the palette.",
                    )
                    .color(palette.text_subtle),
                );
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id("diff-merge-empty".to_owned());
                });
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }
}

// ── MT-036: Flight Recorder observability pane ───────────────────────────────────────────────────────

/// The one-slot delivery cell a spawned `GET /flight_recorder` fetch resolves into; doubles as the
/// pane's [`crate::flight_recorder_pane::FlightRecorderQuery`] impl (the pane's `load_now` reads the
/// resolved value off the frame thread — never blocking).
#[derive(Clone, Default)]
pub struct FlightRecorderFetchCell {
    cell: Arc<Mutex<Option<Result<Vec<crate::flight_recorder_pane::FlightRecorderRow>, String>>>>,
}

impl FlightRecorderFetchCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a fetch result (ok or err) has been delivered.
    pub fn is_resolved(&self) -> bool {
        self.cell.lock().map(|c| c.is_some()).unwrap_or(false)
    }

    /// Deliver a fetch result (called by the spawned off-thread task).
    pub fn deliver(
        &self,
        result: Result<Vec<crate::flight_recorder_pane::FlightRecorderRow>, String>,
    ) {
        if let Ok(mut c) = self.cell.lock() {
            *c = Some(result);
        }
    }
}

impl crate::flight_recorder_pane::FlightRecorderQuery for FlightRecorderFetchCell {
    fn rows(&self) -> Result<Vec<crate::flight_recorder_pane::FlightRecorderRow>, String> {
        match self.cell.lock() {
            Ok(c) => match c.as_ref() {
                Some(result) => result.clone(),
                None => Err("flight recorder fetch not resolved yet".to_owned()),
            },
            Err(_) => Err("flight recorder cell poisoned".to_owned()),
        }
    }
}

/// Parse the `GET /flight_recorder` JSON array body into the pane rows (the reduced projection the
/// pane renders: event_id / event_type / actor_id / timestamp). Pure so a unit test asserts the shape.
pub fn flight_recorder_rows_from_json(
    body: &serde_json::Value,
) -> Result<Vec<crate::flight_recorder_pane::FlightRecorderRow>, String> {
    let arr = body
        .as_array()
        .ok_or_else(|| "flight recorder response is not a JSON array".to_owned())?;
    Ok(arr
        .iter()
        .map(|e| crate::flight_recorder_pane::FlightRecorderRow {
            event_id: e
                .get("event_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
            action: e
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
            actor_id: e
                .get("actor_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
            ts_utc: e
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned(),
        })
        .collect())
}

/// WP-KERNEL-012 MT-036 REMEDIATION: the live FLIGHT-RECORDER pane, registered over the REAL
/// `PaneType::FlightRecorder` key (the existing `flightrecorder.open` palette command + RUN menu entry
/// already open that key — mounting the real factory makes that operator route render the real pane).
/// The mount signals visibility through `load_requested` so the shell fires ONE `GET /flight_recorder`
/// per open (the production `FlightRecorderQuery` impl over the verified route) and calls the pane's
/// `load_now` when the fetch cell resolves.
pub struct FlightRecorderPaneMount {
    pane: Arc<Mutex<crate::flight_recorder_pane::FlightRecorderPane>>,
    palette: SharedPalette,
    /// Set true on the first render (the pane became visible) so the shell fires the fetch once.
    load_requested: Arc<std::sync::atomic::AtomicBool>,
}

impl FlightRecorderPaneMount {
    pub fn new(
        pane: Arc<Mutex<crate::flight_recorder_pane::FlightRecorderPane>>,
        palette: SharedPalette,
        load_requested: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            pane,
            palette,
            load_requested,
        }
    }
}

impl PaneFactory for FlightRecorderPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::FlightRecorder
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        self.load_requested
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let palette = palette_of(&self.palette);
        if let Ok(pane) = self.pane.lock() {
            pane.show(ui, &palette);
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Region
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::BlockNode;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    #[test]
    fn session_context_is_bound_only_with_workspace_and_runtime() {
        let rt = rt();
        assert!(!EditorSessionContext::default().is_bound());
        assert!(!EditorSessionContext {
            workspace_id: "ws".into(),
            runtime: None
        }
        .is_bound());
        assert!(EditorSessionContext::new("ws-1", rt.handle().clone()).is_bound());
        // Empty workspace + a runtime is still UNbound (a half-built context never installs wired state).
        assert!(!EditorSessionContext {
            workspace_id: String::new(),
            runtime: Some(rt.handle().clone())
        }
        .is_bound());
    }

    #[test]
    fn code_mount_pane_type_and_unbound_stays_unwired() {
        let panel = Arc::new(CodeEditorPanel::new("fn main() {}", "rs"));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::default()));
        let (tx, _rx) = std::sync::mpsc::channel::<CodeEditorAction>();
        let mount = CodeEditorPaneMount::new(panel, session, tx);
        assert_eq!(mount.pane_type(), PaneType::CodeSymbol);
        // No bound session yet: wire_if_needed installs the command sender but NOT the runtime/workspace.
        mount.wire_if_needed();
        assert!(
            !mount.is_wired(),
            "an unbound session must not mark the panel wired"
        );
    }

    #[test]
    fn code_mount_threads_runtime_and_workspace_when_bound() {
        let rt = rt();
        let panel = Arc::new(CodeEditorPanel::new("fn main() {}", "rs"));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::new(
            "ws-42",
            rt.handle().clone(),
        )));
        let (tx, _rx) = std::sync::mpsc::channel::<CodeEditorAction>();
        let mount = CodeEditorPaneMount::new(Arc::clone(&panel), session, tx);
        mount.wire_if_needed();
        assert!(mount.is_wired());
        // The prior-MT hook actually ran: the panel now carries the bound workspace id.
        assert_eq!(panel.workspace_id(), "ws-42");
    }

    #[test]
    fn rich_mount_threads_context_and_drains_events() {
        let rt = rt();
        let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
            BlockNode::paragraph("hi"),
        ]))));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::new(
            "ws-9",
            rt.handle().clone(),
        )));
        let events = RichPaneEvents::new();
        let mount = RichEditorPaneMount::new(Arc::clone(&state), session, events.clone(), "DOC-1");
        assert_eq!(mount.pane_type(), PaneType::LoomWikiPage);
        mount.wire_if_needed();
        assert!(mount.is_wired());
        // The wikilink context bound the workspace (the MT-057 hook ran).
        assert_eq!(state.lock().unwrap().wikilinks.workspace_id, "ws-9");

        // Enqueue an event the way the editor would, then drain: it reaches the shared outbound queue.
        state
            .lock()
            .unwrap()
            .pending_events
            .push(EditorEvent::WikilinkActivated {
                ref_kind: "note".into(),
                ref_value: "DOC-2".into(),
                resolved: true,
            });
        mount.drain_events();
        assert!(
            state.lock().unwrap().pending_events.is_empty(),
            "drained from the editor state"
        );
        let routed = events.take();
        assert_eq!(
            routed.len(),
            1,
            "the event reached the shell's outbound queue"
        );
        assert!(
            events.is_empty(),
            "take() leaves the queue empty (routed exactly once)"
        );
    }
}
