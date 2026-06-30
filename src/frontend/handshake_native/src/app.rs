//! Handshake native GUI shell (MT-002).
//! Opens the real wgpu window with a top title bar, bottom status bar (live backend /health), and
//! a central work-surface placeholder. Render logic lives in `ui()` (no eframe::Frame) so it is
//! driveable headlessly by egui_kittest.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use base64::Engine as _;

use crate::accessibility::{self, ChromeWidget};
use crate::atelier_side_panel::AtelierSidePanel;
use crate::backend_client::{self, HealthInfo, WorkbenchLayoutClient, HEALTH_URL};
use crate::code_editor::keymap::CodeEditorAction;
use crate::code_editor::panel::CodeEditorPanel;
use crate::editor_pane_factories::{
    CodeEditorPaneMount, EditorSessionContext, RichEditorPaneMount, RichPaneEvents,
    SharedSessionContext,
};
use crate::error::AppError;
use crate::event_bus::{new_shell_event_bus, ShellEvent, ShellEventReceiver, ShellEventSender};
use crate::layout_persistence::{
    DrawersState, LayoutPersistenceManager, LayoutPersistenceStatus, LayoutSnapshot,
    LayoutTransport, PopOutSnapshot,
};
use crate::left_rail::{LeftRail, LeftRailColors, LeftRailEvent};
use crate::module_switcher::{ModuleId, ModuleSwitcher, ModuleSwitcherColors};
use crate::pane_header::{PaneHeader, PaneHeaderColors, PANE_HEADER_HEIGHT};
use crate::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneId, PaneRecord, PaneRegistry,
    PaneRenderContext, PaneType, PlaceholderPaneFactory,
};
use crate::popout_window::{popout_title_for, PopOutGeometry, PopOutManager};
use crate::project_tabs::{
    fetch_workspaces, ProjectItem, ProjectTabBar, ProjectTabColors, PROJECT_TAB_BAR_HEIGHT,
};
use crate::rails::{apply_rail_scrollbar_style, RailColors, RailDimensions};
use crate::rich_editor::renderer::rich_editor_widget::RichEditorState;
use crate::split_layout::{DividerColors, SplitDragState, SplitLayoutWidget, SplitWeights};
use crate::stage_pane::{StageContent, StagePane};
use crate::tab_bar::{TabBar, TabBarColors, TabBarState, TabState, TAB_BAR_HEIGHT};
use crate::theme::{self, HsTheme};
use crate::top_menu_bar::{
    EditorMetaSegmentState, EditorSegmentAction, EditorStatusSegments, MenuBar, MenuBarAction,
    MenuBarState,
};

/// Stable AccessKit id for the theme-toggle button. egui maps `accesskit::NodeId` directly
/// from an `egui::Id`'s u64 value (egui 0.33 `Id::accesskit_id`), so a fixed-value `Id`
/// yields a fixed `NodeId`. The contract requires `NodeId(10)` for out-of-process/kittest
/// steering.
const THEME_TOGGLE_NODE_ID: u64 = 10;

/// Stable, model-meaningful match key for the theme-toggle button (kebab-case, dot-namespaced
/// under `shell.chrome.`, mirroring the title/status convention). The toggle is the shell's one
/// interactive chrome widget; this is the out-of-process address a model uses to click it,
/// independent of its display text ("Light"/"Dark") which flips with the active theme.
const THEME_TOGGLE_AUTHOR_ID: &str = "shell.chrome.theme-toggle";

/// Stable AccessKit author id for the MT-100 terminal launch outcome. The RUN menu and command palette
/// both route to the same typed blocker, then this status segment makes the result visible to the
/// operator and addressable to a no-context model.
pub const TERMINAL_LAUNCH_STATUS_AUTHOR_ID: &str = "terminal-launch-status";

/// WP-KERNEL-012 MT-101: compact model-session launch dialog and status author ids. These are stable so
/// MT-102/MT-103 model-driven navigation can open the dialog, set values, submit, and read back status
/// without guessing from screen pixels.
pub const MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID: &str = "model-session-launch.dialog";
pub const MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID: &str = "model-session-launch.provider";
pub const MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID: &str =
    "model-session-launch.provider.local";
pub const MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID: &str =
    "model-session-launch.provider.cloud";
pub const MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID: &str = "model-session-launch.folder";
pub const MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID: &str = "model-session-launch.model";
pub const MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID: &str = "model-session-launch.wrapper";
pub const MODEL_SESSION_LAUNCH_START_AUTHOR_ID: &str = "model-session-launch.start";
pub const MODEL_SESSION_LAUNCH_CANCEL_AUTHOR_ID: &str = "model-session-launch.cancel";
pub const MODEL_SESSION_LAUNCH_INLINE_STATUS_AUTHOR_ID: &str = "model-session-launch.inline-status";
pub const MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID: &str = "model-session-launch-status";
const MODEL_SESSION_CHOOSE_STATUS: &str = "Model session: choose folder, model, and wrapper";
const MODEL_SESSION_READY_STATUS: &str = "Model session: ready to issue POST /jobs";

/// The content-presentation mode the shell is in (MT-015 VIEW menu). Mirrors the React workspace
/// `viewMode` (`NSFW`/`SFW`): NSFW shows adult content surfaces, SFW hides them. The native shell
/// owns the flag (MT-015 toggles it from the VIEW menu); the surfaces that consume it land in later
/// MTs, so this is an in-memory flag in MT-015 (not yet persisted), mirroring how MT-003 introduced
/// the theme flag before the settings-persistence MT wired it to the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Adult content surfaces shown (the production default).
    Nsfw,
    /// Adult content surfaces hidden.
    Sfw,
}

impl ViewMode {
    /// The other mode (the VIEW > View Mode menu toggles between the two).
    pub fn toggled(self) -> Self {
        match self {
            ViewMode::Nsfw => ViewMode::Sfw,
            ViewMode::Sfw => ViewMode::Nsfw,
        }
    }
}

/// The project id a fresh shell shows before any project switch. Must match the `project_id` the
/// default panes are seeded with (see [`default_panes`]) so the captured snapshot is self-consistent.
pub const DEFAULT_PROJECT_ID: &str = "default-project";

/// Debounce quiet period for the layout save: a flush fires this long after the LAST layout-affecting
/// change, so rapid divider drags / tab reorders coalesce into one backend `PUT` (MT-009 contract:
/// "a short debounce so rapid drags coalesce"). 600ms balances responsiveness against `PUT` volume.
pub const LAYOUT_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(600);

const BACKGROUND_WORKER_SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const BACKGROUND_WORKER_SHUTDOWN_POLL: std::time::Duration = std::time::Duration::from_millis(10);

/// Debounce quiet period for the settings save (MT-018, red-team R2): a `PUT /workspaces/{id}/settings`
/// fires this long after the LAST settings change so rapid keybinding edits coalesce into one request.
/// 500ms per the MT implementation note. A dialog close FLUSHES any pending save immediately (MC2).
pub const SETTINGS_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(500);

/// WP-KERNEL-012 MT-084 (D2 internal_diagnostics, Tier 2 — UI-thread heartbeat, §5.8.2).
///
/// The idle repaint cadence the frame loop guarantees so the per-frame heartbeat keeps advancing even
/// when the UI is otherwise idle (egui repaints on demand by default, so without this an idle-but-
/// healthy app would stop bumping the counter and Palmistry would misread it as frozen — RISK-004-1).
///
/// 250ms (4 Hz) is deliberately chosen to sit BETWEEN Palmistry's freeze poll interval (~200–500ms,
/// MT-091) and the freeze threshold (~5s): the heartbeat ticks fast enough that a healthy idle app is
/// always fresh on a poll, yet slow enough that the wake is cheap (4 Hz steals no focus and barely
/// touches idle CPU — it does NOT violate HBR-QUIET; `request_repaint_after` only schedules a wake, it
/// never raises a window or grabs input). The freeze threshold is ~20x this interval, so a genuine
/// freeze (the counter stops advancing) is unambiguous against this idle cadence.
pub const HEARTBEAT_IDLE_REPAINT_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(250);

/// WP-KERNEL-012 MT-088 (D2 internal_diagnostics — backend-down graceful degradation, §5.8.5 HARD).
/// Cadence for the background `/health` re-probe. The ctor fires one `/health` poll; this re-arms it so
/// a backend that goes down (then recovers) is continuously re-observed — which is what drives the
/// debounced `BackendUnreachable`/`BackendRecovered` transition events and the re-connect after recovery
/// (AC-008-6). Each probe is a single off-UI-thread `rt.spawn` (HBR-QUIET — no UI-thread blocking, no
/// focus steal); 2s is frequent enough to notice a down/up transition promptly yet cheap on idle CPU.
pub const HEALTH_REPROBE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// A generous default all-monitors extent for the restore-time pop-out clamp before egui reports the
/// real monitor size. Large enough that a legitimate position is never clamped on the first frame.
const DEFAULT_MONITOR_EXTENT: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10_000.0, 10_000.0));

/// A transport that never persists, for headless/test shells with no running backend. `load` always
/// reports "no stored layout" (first run) and `save` silently succeeds, so a test shell that does not
/// inject a stub transport keeps the seeded default layout and never makes a network call.
#[derive(Debug, Default)]
struct NullLayoutTransport;

impl LayoutTransport for NullLayoutTransport {
    fn load(
        &self,
        _workspace_id: &str,
    ) -> Result<Option<serde_json::Value>, crate::layout_persistence::LayoutError> {
        Ok(None)
    }
    fn save(
        &self,
        _workspace_id: &str,
        _layout_state: serde_json::Value,
    ) -> Result<(), crate::layout_persistence::LayoutError> {
        Ok(())
    }
}

pub enum HealthDisplayState {
    Loading,
    Ok(HealthInfo),
    Error(String),
}

/// WP-KERNEL-012 MT-088 (D2 internal_diagnostics — backend-down graceful degradation, §5.8.5 HARD).
/// The result a spawned layout-LOAD worker delivers into [`HandshakeApp::layout_load_cell`]:
/// - `project_id` — the project the load was requested for (so a delivery for a since-switched project is
///   ignored rather than clobbering the now-current layout);
/// - `reachable` — whether the backend was REACHABLE for this load (the manager swallows a transport
///   error into `Ok(fallback)` + an `Error` STATUS, so the worker reads the status right after `load`
///   and delivers the reachability here — the UI thread folds the correct edge without re-reading a
///   possibly-since-changed manager status);
/// - `snapshot` — the manager's validated `load` outcome: `Ok(Some(..))` is a restored layout to apply;
///   `Ok(None)` is "no stored layout / fell back to default" (keep the seeded default — the
///   degraded-but-responsive state); `Err` would be unreachable-with-no-LKG.
///
/// The worker runs the load OFF the egui UI thread; the UI thread NEVER `lock()`s the manager on the
/// frame path (it uses `try_lock` and skips when the worker holds it), so a backend-down `GET` can never
/// stall the frame loop — the freeze fix is BOTH "the `block_on` is off the UI thread" AND "the UI thread
/// never blocks waiting for the manager lock the worker holds during that `block_on`".
type LayoutLoadResult = (
    String,
    bool,
    Result<Option<LayoutSnapshot>, crate::layout_persistence::LayoutError>,
);

/// MT-099 Notes end-to-end load delivery:
/// `(load_generation, document_id, loaded document or typed error)`.
/// The document GET runs off the egui UI thread and the frame path drains this cell without blocking.
type RichDocumentLoadResult = (u64, String, Result<backend_client::RichDocBody, String>);

pub struct HandshakeApp {
    health_status: HealthDisplayState,
    rt: tokio::runtime::Runtime,
    health_handle: Option<tokio::task::JoinHandle<Result<HealthInfo, AppError>>>,
    /// WP-KERNEL-012 MT-082 (D2 — internal_diagnostics, Tier 2): the per-session diagnostic identity
    /// (session id + the MT-081 ring backing-file path) MT-094 passes to `palmistry.exe` so the external
    /// watcher maps the SAME ring. `Some` in the production shell (the ring was created + the writer
    /// installed in [`HandshakeApp::new`]); `None` in the headless/test shell and when ring creation
    /// failed (graceful degradation — diagnostics are in-process-only that session, the app still runs).
    diag_session: Option<crate::diagnostics::DiagSession>,
    /// WP-KERNEL-012 MT-094 (Tier 3 — Palmistry launched WITH Handshake, §6.13.3): the handle to the
    /// external `palmistry` watcher process `main()` spawned BEFORE the event loop (held so a clean app
    /// exit sends the explicit `Shutdown` control message — the "closes only on explicit command" path).
    /// `Some` only in the production shell when the watcher launched + the launch path set it via
    /// [`HandshakeApp::set_palmistry_handle`]; `None` in the headless/test shell and when the watcher
    /// could not be launched (graceful degradation — the app runs without the supplementary watcher).
    /// Dropping the handle reaps the child (best-effort Shutdown then a kill backstop), so even an
    /// `on_exit`-less teardown never orphans the watcher.
    palmistry: Option<crate::diagnostics::PalmistryHandle>,
    /// WP-KERNEL-012 MT-084 (D2 — internal_diagnostics, Tier 2 UI-thread heartbeat, §5.8.2). The
    /// monotonic frame-loop counter bumped at the TOP of every [`eframe::App::update`] call on the UI
    /// thread and published into the MT-081 ring heartbeat slot. A stalled UI thread stops advancing
    /// it, which is the exact liveness signal Palmistry (Tier 3) polls for freeze detection (MT-091).
    /// Starts at 0; the first frame publishes 1.
    frame_counter: u64,
    /// WP-KERNEL-012 MT-084: the process-start monotonic clock for the heartbeat timestamp. A
    /// `std::time::Instant` is MONOTONIC (immune to wall-clock changes), so the elapsed-nanos value
    /// published with each heartbeat strictly increases and never goes backward — which is required
    /// because the freeze-threshold staleness math (MT-091) compares this monotonic value (RISK-004-2).
    /// A wall `SystemTime` would be WRONG here (a clock jump would corrupt the staleness math); it is
    /// fine only for a human-readable panel, never for the threshold.
    heartbeat_clock: std::time::Instant,
    /// WP-KERNEL-012 MT-085 (D2 — internal_diagnostics, Tier 2 frame-time, §5.8.2/§5.8.4). The rolling
    /// frame-time tracker fed once per [`eframe::App::update`] frame with the WORK time of
    /// `self.ui(ctx)` (NOT the inter-frame period, so the MT-084 idle keep-alive is never mis-flagged
    /// as slow — RISK-005-1). It keeps last/min/max + p50/p95 stats for the Diagnostics Panel (MT-087)
    /// and emits a typed `SlowFrame` event (debounced) when a frame's work exceeds the slow threshold.
    frame_timer: crate::diagnostics::FrameTimer,
    /// WP-KERNEL-012 MT-085 TEST SEAM: extra synthetic WORK injected inside `self.ui(ctx)` so a kittest
    /// can drive a REAL slow frame from the live frame path (AC-005-2) without test-only branches in the
    /// production frame-time measurement. `0` in production (no injected work); a test sets it via
    /// [`HandshakeApp::set_extra_frame_work_for_test`]. The measurement wraps `self.ui(ctx)`, so the
    /// injected sleep lands INSIDE the measured work window exactly like real heavy UI work would.
    extra_frame_work_micros: u64,
    /// WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2 resource counters, §5.8.2/§5.8.4). The
    /// per-process CPU%/RSS sampler, ticked once per [`eframe::App::update`] frame behind a bounded ~1s
    /// cadence gate (NOT every frame). It refreshes ONLY this process's pid and emits a typed
    /// `ResourceSample` event each interval so the Diagnostics Panel (MT-087) shows the resource line and
    /// Palmistry (Tier 3) sees CPU/RSS on the ring. The same sampler runs in the headless shell (the
    /// emit is an in-process buffer record only when no ring writer is installed).
    resource_sampler: crate::diagnostics::ResourceSampler,
    /// WP-KERNEL-012 MT-086: the STATIC GPU/driver identity captured ONCE in [`HandshakeApp::new`] from
    /// `cc.wgpu_render_state` (the EXISTING eframe wgpu adapter — no second device). `Some` in the wgpu
    /// production/kittest shell; `None` in the headless `with_health` shell (no `CreationContext`). The
    /// integer codes (vendor/device/device_type/backend) are ring-safe; the human driver strings live
    /// here ONLY for the panel's hardware line (never pushed into the typed ring — AC-006-3).
    gpu_info: Option<crate::diagnostics::GpuInfo>,
    /// Active base theme. Toggled by the top-bar button; not persisted in MT-003.
    current_theme: HsTheme,
    /// Last theme actually pushed to egui via `apply_to_ctx` (CONTROL-1: only re-apply on
    /// change so the common steady-state frame skips `set_visuals`). `None` until the first
    /// frame so the initial palette is always applied once.
    last_applied_theme: Option<HsTheme>,
    /// The single source of truth for every pane in the work surface (MT-005). Wrapped in
    /// `Arc<Mutex<_>>` now — even though MT-005 is single-threaded — so the MT-028 concurrency work
    /// (parallel agent + operator pane mutation) is a behavior change, not a structural refactor
    /// (RISK-5 / CONTROL-5).
    pane_registry: Arc<Mutex<PaneRegistry>>,
    /// Pane renderers, one per `PaneType`. Every variant gets a `PlaceholderPaneFactory` until a
    /// real surface factory replaces it in a later MT, so an unhandled type can never blank/panic
    /// a pane (RISK-3 / CONTROL-3).
    factories: HashMap<PaneType, Box<dyn PaneFactory>>,
    /// MT-028: the shared cell the concrete `LoomSearchV2PaneFactory` reads each frame (active workspace
    /// id + live palette pushed IN) and the shell drains (clicked block ids pulled OUT into the Loom
    /// block-open path). This is how the in-product Loom Search pane reaches a real workspace + theme +
    /// open path through the `&self` `PaneFactory::render` signature (AC-9). `None` only in the unusual
    /// case the concrete factory was not installed (never in the two real constructors).
    loom_search_v2_shared: Arc<Mutex<crate::loom_search_v2::LoomSearchV2PaneShared>>,
    /// MT-029: the shared cell the concrete `FindInFilesPaneFactory` reads each frame (active workspace
    /// id + live palette pushed IN) and the shell drains (clicked hits pulled OUT into the open path).
    /// Same role as `loom_search_v2_shared` for the Find-in-Files pane.
    find_in_files_shared: Arc<Mutex<crate::find_in_files::FindInFilesPaneShared>>,
    /// MT-098: shared Runtime Chat panel state rendered by the `PaneType::RuntimeChat` factory. The shell
    /// pushes the live theme palette into it each frame; send attempts return a typed EndpointMissing
    /// blocker until a real native HTTP chat route exists.
    runtime_chat_panel: Arc<Mutex<crate::runtime_chat::RuntimeChatPanel>>,
    /// WP-KERNEL-012 MT-079 (E11 host-mount): the live handles for the MOUNTED native editors (the
    /// session-context cell the shell pushes the active workspace into each frame, the Arc-shared code
    /// panel + rich state behind the mounted panes, the code command channel the shell drains into the
    /// command bus + unified undo, and the rich-pane event queue the shell routes to the nav bus). The
    /// editor analogue of `loom_search_v2_shared` / `find_in_files_shared`.
    editor_mounts: EditorMountHandles,
    /// MT-099 Notes end-to-end: base URL for the mounted Notes editor's authoritative
    /// `/knowledge/documents/*` load/save/draft route family. Production uses the normal backend base;
    /// tests can point it at a localhost capture server via `set_backend_base_url_for_test`.
    rich_doc_base_url: String,
    /// MT-099 Notes end-to-end: FIFO delivery queue for off-frame document loads. The UI thread drains
    /// completed GETs once per frame and applies only the current in-flight document, so a stale request
    /// cannot overwrite/drop a newer active-document completion.
    rich_doc_load_cell: Arc<Mutex<VecDeque<RichDocumentLoadResult>>>,
    /// The document id currently being loaded into the mounted rich editor, if any.
    rich_doc_loading_id: Option<String>,
    /// Monotonic generation for Notes document loads. Reopening the same document invalidates any older
    /// in-flight GET for that id, so stale same-id responses cannot apply over a fresh reload.
    rich_doc_load_generation: u64,
    /// The document id currently installed in the mounted rich editor, if any.
    rich_doc_loaded_id: Option<String>,
    /// The installed document version; used only as a fallback if a host Save is invoked after a reload
    /// but before the editor has a fresh SaveManager.
    rich_doc_loaded_version: Option<u64>,
    /// Last transient Notes load error, surfaced through tests/manual diagnostics rather than a spinner.
    rich_doc_load_error: Option<String>,
    /// WP-KERNEL-012 MT-079: the last code-editor host-routed command the shell drained
    /// (`Save`/`OpenCommandPalette` — Undo/Redo dispatch directly to the bus). Exposed via
    /// [`last_editor_command`](Self::last_editor_command) so the dispatch is perceivable + testable (the
    /// Save WRITE itself is a typed carry owned by the document shell). `None` until one is dispatched.
    last_editor_command: Option<CodeEditorAction>,
    /// Persisted split fractions for the 2x2 pane grid (MT-006). Serialized into the layout snapshot
    /// by MT-009. Initialized to the React `DEFAULT_SPLIT_WEIGHTS` (`{ vertical: 0.5, horizontal:
    /// 0.55 }`).
    split_weights: SplitWeights,
    /// Per-frame pointer-drag state for the dividers (MT-006). Deliberately separate from
    /// `split_weights` so transient drag flags are never serialized into a layout snapshot
    /// (red-team RISK-5).
    split_drag: SplitDragState,
    /// The pane the operator last clicked. `None` until a pane is activated; later MTs use it to
    /// highlight the focused pane / route operator actions.
    active_pane: Option<PaneId>,
    /// Per-pane tab-bar state (MT-007). Keyed by `PaneId` so each pane region owns its own ordered
    /// tab list + active index. Serialized into the layout snapshot by MT-009. Inter-pane tab drag
    /// state is NOT stored here — it lives in egui's `DragAndDrop` payload while a drag is in flight
    /// (the drop crosses pane boundaries, so it cannot belong to any single `TabBarState`).
    tab_bar_states: HashMap<PaneId, TabBarState>,
    /// Active pop-outs (MT-008): panes detached into their own OS windows. The pane record stays in
    /// `pane_registry` (single source of truth); this only tracks which panes render into a detached
    /// viewport and where that window sits. Serialized into the layout snapshot by MT-009.
    popout_manager: PopOutManager,
    /// A pop-out was requested this frame (e.g. by a future MT-019 pane-header "Pop Out" action, or
    /// by a test / out-of-process driver via [`HandshakeApp::request_pop_out`]). Applied at the top
    /// of the next `ui()` so the detached viewport is created cleanly before the frame renders.
    pop_out_request: Option<PaneId>,
    /// The project (workspace) whose layout this shell currently shows. Drives the
    /// `/workspaces/:id/workbench/layout` path (MT-009). Seeded to the same `default-project` the
    /// default panes use; a project-switch changes this and triggers a load on the next frame.
    active_project_id: String,
    /// Per-project layout persistence manager (MT-009): debounce-on-change, retry-on-transient,
    /// in-memory last-known-good, and a UI-readable status. Persists THROUGH the backend's
    /// PostgreSQL-authoritative `/workspaces/:id/workbench/layout` REST endpoint — no local file.
    /// Wrapped in `Arc<Mutex<_>>` so a debounced save can run on a short-lived worker off the egui UI
    /// thread (HBR-QUIET) while the UI thread reads its status.
    layout_manager: Arc<Mutex<LayoutPersistenceManager>>,
    /// The project whose layout has been loaded into this shell. Drives the load-on-first-frame /
    /// load-on-project-change lifecycle: when this differs from `active_project_id`, the next frame
    /// loads the new project's layout. `None` until the first load runs.
    loaded_project_id: Option<String>,
    /// Whether the layout changed since the last save flush was scheduled. The UI sets this when a
    /// layout-affecting field changes (split weight / tab order/active/pin / pop-out / active pane) so
    /// the next frame marks the manager dirty and schedules a debounced save.
    layout_dirty_signal: bool,
    /// A debounced save flush is in flight on a worker thread. Prevents spawning a second overlapping
    /// flush for the same coalesced change set.
    save_in_flight: Arc<std::sync::atomic::AtomicBool>,
    /// WP-KERNEL-012 MT-088 (D2 internal_diagnostics — backend-down graceful degradation, §5.8.5 HARD).
    /// The delivery cell a spawned layout-LOAD worker writes the loaded snapshot result into; the egui UI
    /// thread drains it next frame and applies it. This is THE fix for the latent 2026-06-26 freeze: the
    /// project-layout load runs a backend `GET` that USED to `block_on` ON the egui UI thread inside
    /// `drive_layout_persistence` (so a backend-down `GET` stalled the whole frame loop — Responding=false,
    /// CPU->0). The load now runs OFF the UI thread (the same short-lived-worker shape the debounced SAVE
    /// already uses) and delivers here, so a dead backend degrades (the seeded default stays visible) and
    /// never freezes. `(project_id, result)` so a stale delivery for a since-switched project is ignored.
    layout_load_cell: Arc<Mutex<Option<LayoutLoadResult>>>,
    /// WP-KERNEL-012 MT-088: a layout-LOAD worker is in flight. Prevents spawning a second overlapping
    /// load for the same project while the first is still running (mirrors `save_in_flight`).
    layout_load_in_flight: Arc<std::sync::atomic::AtomicBool>,
    /// WP-KERNEL-012 MT-088: the DEBOUNCED backend-reachability state — `true` once the app has observed
    /// the backend as unreachable and has NOT yet seen it recover. The typed diagnostic transition events
    /// are emitted on the EDGES only (RISK-008-4 / AC-008-3): `BackendUnreachable` once on the
    /// reachable->unreachable edge, `BackendRecovered` once on the unreachable->reachable edge — NEVER
    /// every frame. Driven by the periodic `/health` poll result (the canonical reachability oracle) and
    /// by a layout-load transport failure. Starts `false` (assume reachable until proven otherwise; the
    /// first real `/health` result settles it without a spurious "recovered").
    backend_down: bool,
    /// WP-KERNEL-012 MT-088: when the NEXT background `/health` re-probe is due. The ctor fires one
    /// `/health` poll; without a re-probe a recovered backend would never be re-observed (so
    /// `BackendRecovered` / re-connect could never fire — AC-008-6). After each resolved poll the next
    /// probe is scheduled this far out (a bounded cadence — cheap, off the UI thread via `rt.spawn`).
    /// `None` while a probe is in flight (re-armed when it resolves). Skipped entirely in the headless
    /// shell (no runtime to spawn on).
    health_next_poll_at: Option<std::time::Instant>,
    /// WP-KERNEL-012 MT-088: a clone of the live [`egui::Context`], captured lazily on the first frame
    /// (top of [`ui`](Self::ui)). An `egui::Context` is `Arc`-backed (`Clone + Send + Sync`) and
    /// `request_repaint` is thread-safe, so an off-thread backend worker (layout load / debounced save /
    /// `/health` re-probe) clones this and calls `ctx.request_repaint()` exactly ONCE when it finishes —
    /// an EVENT-DRIVEN wake that drains + applies the delivered result on the very next frame. This
    /// replaces the previous per-frame `request_repaint_after(100ms)` poll-for-delivery cadence, which
    /// requested a repaint EVERY frame while a load/save/probe was in flight and so kept the UI from ever
    /// reporting idle within `Harness::run`'s bounded step budget (the pane-test `max_steps` regression).
    /// `None` until the first frame runs (no worker can be in flight before then). Captured from the live
    /// context (not a constructor arg) so both the production and headless/test shells wake correctly.
    frame_ctx: Option<egui::Context>,
    /// The layout blob as of the last frame, used to DETECT a layout-affecting change without
    /// instrumenting every divider/tab/pop-out call site: if this frame's captured `layout_state`
    /// differs, the layout changed and a debounced save is scheduled. `None` until the first frame
    /// settles (so the initial seed is not mistaken for a change). Set after a load so a restore does
    /// not immediately re-save the just-loaded layout.
    last_seen_layout: Option<serde_json::Value>,
    /// The full all-monitors extent used for the restore-time pop-out clamp. Defaults to a generous
    /// extent; `ui()` refreshes it from egui's monitor size each frame so the clamp uses the real
    /// desktop bounds when a layout is restored.
    monitor_extent: egui::Rect,
    /// The top project-tab strip (MT-011): one tab per open workspace, rendered above the pane grid.
    /// Clicking a non-active tab drives `active_project_id`, which the MT-009 lifecycle keys on to
    /// save the leaving project's layout and load the entered project's layout.
    project_tabs: ProjectTabBar,
    /// In-flight `GET /workspaces` fetch (MT-011). Spawned non-blocking so a slow/absent backend never
    /// stalls the render loop; polled each frame and folded into `project_tabs` when it resolves.
    workspaces_handle: Option<tokio::task::JoinHandle<Result<Vec<ProjectItem>, AppError>>>,
    /// The top-right MODULE switcher (MT-012): the six MAIN/CKC/INGEST/STAGE/LAB/STUDIO buttons. Owns
    /// only the active module id (the highlight); switching a module mutates the ACTIVE pane's tab list
    /// and active tab via [`HandshakeApp::set_module`]. Serialized into the layout snapshot as
    /// `active_module` by MT-009, so the module survives a project switch and an app restart.
    module_switcher: ModuleSwitcher,
    /// The LEFT activity rail (MT-014): activity icons + project tree + quick links + stash/agenda/
    /// mail/notes affordances. Owns its own per-section expand state, the project tree, and the
    /// quick-links list; the app owns the rail's OPEN flag (persisted) + the bottom-drawer flag.
    left_rail: LeftRail,
    /// Whether the left rail is expanded (showing the project tree / quick links / affordances) vs
    /// collapsed to just the activity icon strip. Persisted in the layout snapshot's `drawers.project`
    /// (MT-014) so it round-trips across sessions. Defaults to open so a fresh shell shows the tree.
    left_rail_open: bool,
    /// Whether the BOTTOM stash drawer is open (MT-014 toggles it; MT-022 owns its full UI). This is
    /// the SINGLE shared field both the left-rail stash toggle and the future bottom drawer reference
    /// (red-team CONTROL: one shared flag, not two booleans that drift). Persisted in `drawers.bottom`.
    bottom_drawer_open: bool,
    /// The tokio runtime handle used to spawn the project-tree's async document/canvas loads. Cloned
    /// from `rt` so the rail can fetch without holding the whole runtime. `None` in the headless/test
    /// shell (no multi-thread runtime); the tree then renders from directly-seeded content.
    runtime_handle: Option<tokio::runtime::Handle>,
    /// In-process shell event bus (MT-014 FIX-B) the app drains each frame so a document/canvas/
    /// bookmark deleted from another surface disappears from the project tree with no stale row. The
    /// receiver is owned here; [`event_bus_sender`](HandshakeApp::event_bus_sender) hands out clonable
    /// senders for future delete-performing surfaces (no production emitter exists yet — see FIX-B).
    event_bus_rx: ShellEventReceiver,
    /// A clonable producer handle onto [`event_bus_rx`](Self::event_bus_rx). Stored so the app can hand
    /// copies to producers; the bus stays alive as long as the app does.
    event_bus_tx: ShellEventSender,
    /// The content-presentation mode (MT-015 VIEW menu). In-memory in MT-015 (the consuming surfaces +
    /// persistence land in later MTs). Defaults to NSFW (the production default).
    view_mode: ViewMode,
    /// Whether the command-palette overlay is requested open (MT-015 GO menu sets this; the overlay UI
    /// is MT-016). The MT-016 overlay (`crate::command_palette`) renders when this is true.
    command_palette_open: bool,
    /// Monotonic counter incremented each time [`command_palette_open`](Self::command_palette_open)
    /// flips from closed to open (MT-016). The palette resets its transient query/selection state
    /// whenever it sees a new value, so a re-open never shows the previous session's text (red-team
    /// R1/MC1). Set via [`open_command_palette`](Self::open_command_palette).
    command_palette_open_count: u64,
    /// Whether the quick-switcher overlay is requested open (MT-015 GO menu / Ctrl+P; UI is MT-017).
    /// The MT-017 overlay (`crate::quick_switcher`) renders when this is true.
    quick_switcher_open: bool,
    /// Monotonic counter incremented each time [`quick_switcher_open`](Self::quick_switcher_open) flips
    /// from closed to open (MT-017). The switcher resets its transient query/selection whenever it sees
    /// a new value, so a re-open never shows the previous session's text. Set via
    /// [`open_quick_switcher`](Self::open_quick_switcher).
    quick_switcher_open_count: u64,
    /// The Loom-graph search transport the quick switcher (MT-017) drives: `GET graph-search`,
    /// `GET/POST quick-switcher/recents` against the REAL PostgreSQL backend. A synchronous seam
    /// ([`crate::quick_switcher::LoomGraphSearchTransport`]) so the search state machine stays
    /// unit-testable; the production [`crate::quick_switcher::LoomGraphSearchClient`] bridges async onto
    /// the app's tokio runtime. `None` in the headless/test shell (no runtime) — a test injects a stub
    /// via [`set_quick_switcher_transport`](Self::set_quick_switcher_transport).
    quick_switcher_transport: Option<Arc<dyn crate::quick_switcher::LoomGraphSearchTransport>>,
    /// The MT-017 debounce/sequence state machine. Ticked each open frame with the live query; emits a
    /// [`crate::quick_switcher::SearchAction`] the app spawns on the runtime.
    quick_switcher_search: crate::quick_switcher::SearchManager,
    /// The async results cell the spawned search task writes into; drained (try_lock) each frame and
    /// folded into [`quick_switcher_search`](Self::quick_switcher_search) (red-team MC1/MC2). Shared
    /// with the background task via `Arc`.
    quick_switcher_results_cell: crate::quick_switcher::SearchDeliveryCell,
    /// Most-recent-first list of `"{source_kind}:{ref_id}"` hit keys the quick switcher (MT-017) ranks
    /// its rows by. Loaded from the durable backend recents store (`GET quick-switcher/recents`) on
    /// open, then updated optimistically when a row is picked (the returned key is prepended,
    /// de-duplicated, capped at 20).
    quick_switcher_recents: Vec<String>,
    /// Set true when the switcher opens so the recents load fires once (red-team: load on first open,
    /// not every frame). Cleared after the load is dispatched.
    quick_switcher_recents_pending: bool,
    /// The async cell a spawned recents-load task writes into: `Ok(hit_keys)` or `Err(message)`.
    /// Drained (try_lock) each frame and folded into `quick_switcher_recents` /
    /// `quick_switcher_recents_error` (red-team MC1: never hold the lock across `ui.*`).
    quick_switcher_recents_cell: crate::quick_switcher::RecentsDeliveryCell,
    /// The async cell a spawned recents-RECORD (`POST recents`) task writes into: `Ok(hit_key)` or
    /// `Err(message)`. Drained (try_lock) each frame so the `POST recents` network round-trip runs OFF
    /// the egui UI thread (HBR-QUIET) — selecting a hit jumps immediately + prepends the recent
    /// optimistically; this only reconciles the backend-confirmed key / surfaces the failure (MC3).
    quick_switcher_record_recent_cell: crate::quick_switcher::RecordRecentDeliveryCell,
    /// The last transient recents error (a failed `GET`/`POST quick-switcher/recents`), surfaced on the
    /// status row so a recents failure degrades ordering visibly without a crash (red-team MC3).
    quick_switcher_recents_error: Option<String>,
    /// The last quick-switcher NAVIGATION status (MT-030): set when a jump dispatched through the
    /// [`ShellNavigator`](crate::quick_switcher::ShellNavigator) bus did NOT land on a real surface —
    /// most importantly the editor-pane TYPED SEAM ("Rich-text/Code editor pane not mounted yet
    /// (E11/MT-069)"). When `Some`, it is RENDERED as a persistent status-bar segment and emitted as a
    /// live AccessKit node (`quick-switcher.nav-status`) by
    /// [`quick_switcher_nav_status_segment`](Self::quick_switcher_nav_status_segment), so the seam
    /// outcome is perceivable to the operator AND a swarm agent after the overlay closes — never a
    /// silent no-op or a faked open. Cleared on a successful `Opened` dispatch.
    quick_switcher_nav_status: Option<String>,
    /// Transient label carried from [`open_switcher_hit`](Self::open_switcher_hit) into the
    /// [`ShellNavigator`](crate::quick_switcher::ShellNavigator) arms so the opened tab is labelled with
    /// the hit title (the trait arms take only ids per the MT-030 contract signature). `Some` only for
    /// the duration of one `dispatch_target` call; `None` otherwise.
    nav_pending_label: Option<String>,
    /// Whether the settings overlay is requested open (MT-015 HELP menu sets this; UI is MT-018).
    settings_open: bool,
    /// Monotonic counter incremented each time [`settings_open`](Self::settings_open) flips from closed
    /// to open (MT-018). The dialog resets its transient query/draft state whenever it sees a new value,
    /// so a re-open never shows the previous session's text. Set via [`open_settings`](Self::open_settings).
    settings_open_count: u64,
    /// The live, persisted workspace settings (MT-018): theme, keybindings, view mode, swarm board flag.
    /// Loaded from `GET /workspaces/{id}/settings` on open / workspace change and normalized through
    /// [`crate::workspace_settings::normalize_workspace_settings_state`] (red-team R6/MC6). The settings
    /// dialog reads from this; wired changes mutate it + schedule a debounced `PUT`. Seeded to the
    /// default state so a fresh shell (no stored settings) shows coherent defaults, never empty chords.
    workspace_settings: crate::workspace_settings::WorkspaceSettingsState,
    /// The persisted settings transport (MT-018): the REAL `GET`/`PUT /workspaces/{id}/settings` REST
    /// surface bridged onto the app runtime (the MT-009 `WorkbenchLayoutClient` pattern). `None` in the
    /// headless/test shell (no runtime); a test injects a stub via [`set_settings_transport`].
    ///
    /// [`set_settings_transport`]: HandshakeApp::set_settings_transport
    settings_transport: Option<Arc<dyn crate::workspace_settings::SettingsTransport>>,
    /// The project whose settings have been loaded into `workspace_settings`. `None` until the first
    /// load; when it differs from `active_project_id` the next frame loads the new project's settings.
    settings_loaded_project_id: Option<String>,
    /// Set true when the settings dialog opens so the one-shot settings LOAD fires once on open (not per
    /// frame). Cleared after the load is dispatched.
    settings_load_pending: bool,
    /// The async cell a spawned settings-LOAD task writes into: `Ok(Some(blob))` / `Ok(None)` (first
    /// run) / `Err(message)`. Drained (try_lock) each frame so the network `GET` runs OFF the egui UI
    /// thread (HBR-QUIET).
    settings_load_cell: crate::workspace_settings::SettingsLoadCell,
    /// The async cell a spawned settings-SAVE task writes into: `Ok(())` / `Err(message)`. Drained
    /// (try_lock) each frame so the network `PUT` runs OFF the egui UI thread (HBR-QUIET).
    settings_save_cell: crate::workspace_settings::SettingsSaveCell,
    /// A settings-affecting change happened and a debounced `PUT` is due at this instant (red-team R2:
    /// 500ms after the last change). `None` when no save is pending. On dialog close, a pending save is
    /// flushed IMMEDIATELY (red-team MC2) so a fast change-then-close never loses the change.
    settings_save_due_at: Option<std::time::Instant>,
    /// A settings save/load is in flight on a worker; prevents overlapping spawns for one change set.
    settings_io_in_flight: Arc<std::sync::atomic::AtomicBool>,
    /// The last transient settings persistence error, surfaced on the dialog status row (HBR: visible).
    settings_persist_error: Option<String>,
    /// MT-102 Visual Debugger: transient status for the last Settings -> Diagnostics worksurface dump.
    /// This is not persisted and owns no diagnostic authority; it only makes the button result visible.
    worksurface_inspector_last_dump: Option<String>,
    /// A pending theme flip to apply at the START of the next frame, BEFORE any panel renders (red-team
    /// R4/MC4): applying egui `Visuals` mid-frame would leave already-rendered widgets on the old theme
    /// for one frame. The settings ComboBox / the menu toggle set this; `ui()` applies it at the top.
    pending_theme_change: Option<HsTheme>,
    /// Whether the About box is requested open (MT-015 HELP menu sets this; UI is a later MT). Distinct
    /// from settings so the two HELP actions are independently observable.
    about_open: bool,
    /// A pending Reset-Layout confirmation (MT-015 VIEW menu; red-team MC7). The VIEW > Reset Layout
    /// item ARMS this flag instead of resetting immediately; the actual reset requires a second confirm
    /// action (`confirm_reset_layout`) so a swarm agent arrow-keying the menu cannot wipe the layout by
    /// accident (red-team R7). No foreground dialog is popped (HBR-QUIET) — the confirm is a flag a
    /// future overlay/agent path reads.
    reset_layout_pending: bool,
    /// MT-020 explorer-row rename: the Loom-block rename client (PATCH off the UI thread). `None` in the
    /// no-runtime test app (rename is then a disclosed no-op rather than a panic).
    loom_block_client: Option<crate::backend_client::LoomBlockClient>,
    /// The async cell a spawned explorer-row rename PATCH writes into: `Ok(new_title)` / `Err(message)`.
    /// Drained (try_lock) each frame so the network `PATCH` runs OFF the egui UI thread (HBR-QUIET).
    rename_cell: crate::backend_client::RenameDeliveryCell,
    /// A pending explorer-row rename the operator is editing: the block id being renamed + the live text
    /// buffer (seeded from the row's current title). `Some` while the small rename dialog is open; the
    /// dialog confirms -> spawns the PATCH -> clears this. Kept as app state (not a foreground OS popup)
    /// so it is observable + non-intrusive (HBR-QUIET).
    pending_rename: Option<PendingRename>,
    /// The last explorer-row rename error, surfaced on the rename dialog status row (HBR: visible).
    rename_error: Option<String>,
    /// WP-KERNEL-012 MT-064 (E9 — FEMS memory-write proposal): the open "Propose to Memory" dialog, set
    /// when the `fems.propose_to_memory` palette command dispatches over a live SharedSelection. `Some`
    /// while the dialog is open; rendered by `drive_propose_to_memory`. Kept as app state (not a
    /// foreground OS popup) so it is observable + non-intrusive (HBR-QUIET). The live submit
    /// (POST proposal + FR-EVT-MEM-001 emit) lands at E11/MT-069 — and the proposal WRITE endpoint is
    /// absent in this build, so confirming surfaces the typed `MissingEndpoint` blocker rather than a
    /// silent no-op or a direct memory write.
    pending_memory_proposal: Option<crate::fems::memory_proposal::ProposeToMemoryDialog>,
    /// The last "Propose to Memory" outcome message (the typed blocker / disclosed-no-runtime note),
    /// surfaced on the dialog status row so the operator/agent sees a real result, never a silent drop.
    memory_proposal_status: Option<String>,
    /// WP-KERNEL-012 MT-100: last native terminal-launch outcome. The current reachable PTY surface is
    /// legacy Tauri IPC-only, so launching from RUN or the palette records a compact `EndpointMissing`
    /// status here instead of fabricating a terminal session or silently doing nothing.
    terminal_launch_status: Option<String>,
    /// WP-KERNEL-012 MT-101: model-session launch client for the real reachable `/jobs` request. `None`
    /// in no-runtime tests; the direct-spawn IPC-only blocker is still available as a pure typed result.
    model_session_launch_client: Option<crate::backend_client::ModelSessionLaunchClient>,
    /// Delivery cell for the off-thread MT-101 `POST /jobs` model-session request.
    model_session_launch_cell: crate::backend_client::ModelSessionLaunchCell,
    /// Compact in-app launch dialog state. It opens from RUN > Launch Model Session in Workspace Folder
    /// or the command palette, and keeps one-shot launch fields out of Settings.
    model_session_launch_dialog: Option<ModelSessionLaunchDialogState>,
    /// Last MT-101 visible launch outcome. The text always distinguishes `/jobs` creation from the
    /// IPC-only direct repo-folder spawn blocker.
    model_session_launch_status: Option<String>,
    /// Last direct-spawn blocker paired with the current/last MT-101 `/jobs` attempt. Kept separately so
    /// async job completion cannot drop the exact probed URL needed for state recovery.
    model_session_launch_direct_status: Option<String>,
    /// Prevents duplicate `/jobs` dispatch while the current off-thread request is still unresolved.
    model_session_launch_pending: bool,
    /// MT-021 source-control off-thread client (verified `/source-control/*` endpoints). `None` in the
    /// no-runtime test app (the SCM menu is then a disclosed no-op rather than a panic). Wired so the
    /// production shell can drive stage/unstage/diff/blame off the UI thread (HBR-QUIET) when the native
    /// source-control panel is mounted by its owning pane-content WP.
    source_control_client: Option<crate::backend_client::SourceControlClient>,
    /// Delivery cell a spawned SCM write (stage/unstage/discard) result is written into; drained next
    /// frame so the network POST runs OFF the egui UI thread (HBR-QUIET). `Err` surfaces on the panel.
    scm_receipt_cell: crate::backend_client::ScmReceiptCell,
    /// Delivery cell a spawned SCM diff/blame text result is written into (drained next frame).
    scm_text_cell: crate::backend_client::ScmTextCell,
    /// The last delivered SCM diff/blame text (drained from `scm_text_cell`); the live SCM panel host
    /// (future content WP) reads this into its display area. Observable here so the off-thread path is
    /// proven without the live host.
    scm_display_text: Option<String>,
    /// The last SCM write/read error (drained from the cells), surfaced on the SCM panel status row.
    scm_error: Option<String>,
    /// MT-021 canvas off-thread client (verified canvas-placement + visual-edge endpoints). `None` in
    /// the no-runtime test app. Wired for the same interconnectivity reason as `source_control_client`.
    canvas_client: Option<crate::backend_client::CanvasClient>,
    /// Delivery cell a spawned canvas placement/edge mutation result is written into (drained next
    /// frame, OFF the egui UI thread — HBR-QUIET).
    canvas_op_cell: crate::backend_client::CanvasOpCell,
    /// The last canvas mutation error (drained from `canvas_op_cell`), surfaced on the canvas board.
    canvas_error: Option<String>,
    /// Delivery cell a spawned Loom-block flag PATCH (pin/favorite) result is written into (drained next
    /// frame). Reuses the receipt-cell shape (`Ok(())`/`Err(msg)`).
    loom_flag_cell: crate::backend_client::ScmReceiptCell,
    /// The last Loom-node flag-toggle error (drained from `loom_flag_cell`), surfaced on the graph view.
    loom_flag_error: Option<String>,
    /// MT-021 status-bar segment visibility (segment_id -> hidden). A segment whose id is in this set is
    /// not rendered; `statusbar.toggle_visibility` flips membership. Empty by default (all visible).
    /// TODO(MT-018): the settings dialog should expose a "Restore hidden status bar items" control so a
    /// hidden segment can always be brought back (red-team status-bar-visibility control).
    statusbar_hidden: std::collections::HashSet<String>,
    /// MT-022 bottom search rail emitted-intent slot (AC-022-9 / HBR-SWARM): the lock-guarded shared
    /// `Arc<Mutex<Option<RailQuery>>>` the rail's fire path WRITES the parsed
    /// [`crate::search_rail::RailQuery`] into on Enter / Loom (and writes `None` on clear — AC-022-6).
    /// The rail makes NO backend call; a downstream search-results consumer / concurrent swarm thread
    /// clones-and-reads this slot off the same lock to EXECUTE the search and display results (search
    /// execution + results display are deferred to that consumer per the contract).
    search_rail_query: crate::search_rail::RailQuerySlot,
    /// MT-023 bottom drawer stash shelf (C6): the collapsible panel ABOVE the MT-022 rail with the four
    /// typed cards (Agenda/Mail/Lists/Notes). Owns the resizable height + per-card live data; the app
    /// owns the persisted open flag ([`bottom_drawer_open`](Self::bottom_drawer_open), MT-014
    /// `drawers.bottom`) and the off-thread fetch wiring below.
    drawer: crate::stash_shelf::DrawerStashShelf,
    /// MT-023 off-thread client for the drawer card data (verified `/loom/views/all` view-count +
    /// `/loom/journals/{today}` daily-journal endpoints — the contract's `table`/`calendar`/`total` were
    /// STALE). `None` in the no-runtime test shell (the cards then show their pre-fetch state and never do
    /// I/O); a test injects a runtime via [`set_runtime_handle`](Self::set_runtime_handle).
    drawer_data_client: Option<crate::backend_client::DrawerDataClient>,
    /// The async cell the spawned drawer fetches write `(kind, Ok/Err)` into; drained (try_lock) each
    /// frame and folded into the matching card (HBR-QUIET — the network ran off the UI thread). One slot:
    /// the three fetches deliver sequentially and the UI drains between frames, so a single cell suffices.
    drawer_data_cell: crate::backend_client::DrawerDataCell,
    /// Set true when the drawer opens so the one-shot card fetches fire ONCE on open (not per frame —
    /// RISK-023-C). Cleared after the fetches are dispatched.
    drawer_fetch_pending: bool,
    /// The drawer open flag as of the last frame, so [`drive_drawer`](Self::drive_drawer) detects the
    /// closed→open transition and fires the one-shot fetches exactly once per open (no matter which
    /// surface toggled `bottom_drawer_open`: the affordance, the left-rail stash button, or the palette).
    drawer_prev_open: bool,
    /// MT-024 off-thread client for the drawer CARD ACTION backend mutations (pin/discard/stow/
    /// attach-evidence — all VERIFIED endpoints). `None` in the no-runtime test shell; a test injects a
    /// runtime via [`set_runtime_handle`](Self::set_runtime_handle). Genuinely CONSUMED by
    /// [`apply_drawer_action`](Self::apply_drawer_action).
    drawer_action_client: Option<crate::backend_client::DrawerActionClient>,
    /// The cell the spawned drawer-action tasks write `Ok(())`/`Err(msg)` into; drained each frame into
    /// [`drawer_action_error`](Self::drawer_action_error) (the SCM/canvas receipt-cell pattern).
    drawer_action_cell: crate::backend_client::DrawerActionCell,
    /// The last drawer-action error (drained from `drawer_action_cell`), surfaced on the drawer; `None`
    /// when the last action succeeded or none has run. Readable out-of-process via the drawer debug state.
    drawer_action_error: Option<String>,
    /// MT-024: pending typed promote / send-to-pane intents a swarm reader can observe + the UI applies
    /// (the LOCAL, no-backend actions). Wrapped in `Arc<Mutex<..>>` so a swarm agent reader does not race
    /// the UI writer (HBR-SWARM). The most recent intent of each kind is retained until consumed.
    drawer_intents: std::sync::Arc<std::sync::Mutex<DrawerIntents>>,
    /// MT-024: the active job id (if any) an Attach-evidence action records the block against. `None` when
    /// no job is active — Attach-evidence then shows a tooltip and makes NO backend call (AC-024-9).
    active_job_id: Option<String>,
    /// MT-024 BLOCKER FIX (HBR-STOP / RISK-024-A / AC-024-11 / CONTROL-024-A): the ARMED confirm-discard
    /// state — the card kind plus its backend target. Selecting Discard does NOT dispatch the DELETE; it
    /// sets this to `Some((kind, target))`, which makes [`drive_drawer`](Self::drive_drawer) render the
    /// in-app "Confirm Discard" `egui::Window` (HBR-QUIET — never an OS dialog). The DELETE fires ONLY when
    /// the window's OK is pressed; Cancel clears this with NO backend call. `None` when no discard is
    /// awaiting confirmation.
    confirm_discard: Option<(
        crate::stash_shelf::DrawerCardKind,
        crate::stash_shelf::DrawerActionTarget,
    )>,
    /// MT-024 MAJOR FIX (AC-024-4/5): the kind of the card whose last action SUCCEEDED, retained so the
    /// drawer shows a brief success indicator (the contract's card-removal/reorder lifecycle assumes a
    /// per-block item list; the MT-023 TYPE-card drawer's success effect is feedback + count refresh, not
    /// removal/reorder — disclosed deviation). `None` when the last action failed or none has run.
    drawer_action_success: Option<crate::stash_shelf::DrawerCardKind>,
    /// MT-024 MAJOR FIX (AC-024-4/5): the card kind whose persisting action is in flight, set at dispatch
    /// so the receipt drain can attribute the success/failure to the right card (the `DrawerActionCell`
    /// carries only `Result<(), String>`). Actions deliver sequentially and the UI drains between frames,
    /// so the most-recent in-flight kind is the one the next receipt belongs to. `None` between actions.
    drawer_action_in_flight: Option<crate::stash_shelf::DrawerCardKind>,
    /// MT-027 model-steering: the running out-of-process MCP transport (localhost TCP + Windows named
    /// pipe), bound at startup on the app runtime. `None` in the headless/test shell and until bind
    /// completes. Dropping it on app exit removes the discovery binding file.
    mcp_server: Option<crate::mcp::SwarmMcpServer>,
    /// MT-027: the bounded action queue the MCP server ENQUEUES resolved AccessKit actions into and the
    /// egui frame loop DRAINS each frame (via [`mcp_drain_into_events`](Self::mcp_drain_into_events))
    /// to steer the live shell. Shared (`Arc<Mutex<_>>`) because the server tasks run on tokio threads
    /// concurrently with the UI thread (HBR-SWARM).
    mcp_action_channel: Arc<Mutex<crate::mcp::ActionChannel>>,
    /// MT-027: the latest UI-tree snapshot the frame loop publishes each frame; the MCP `list_widgets`
    /// tool clones it and `click_widget`/`set_value` resolve targets against it. Shared with the server.
    mcp_snapshot: Arc<Mutex<crate::accessibility::UiTreeSnapshot>>,
    /// MT-027: the per-session HMAC token gating every MCP request. Generated at startup; written into
    /// the discovery binding file so an authorized agent can present it.
    mcp_token: crate::mcp::SessionToken,
    /// MT-027: true only while a side-effect-free snapshot-capture pass is running `ui()` on a throwaway
    /// AccessKit context. The async pollers, event-bus drain, and layout-persistence scheduler early-return
    /// when this is set, so publishing the live snapshot never consumes an async result or schedules a
    /// spurious save (the real frame owns those side effects).
    capturing_snapshot: bool,
    /// WP-KERNEL-012 MT-076 (E13 IME): one-shot guard so the shell sends
    /// `ViewportCommand::IMEAllowed(true)` to winit EXACTLY ONCE (the first real frame), not every frame.
    /// Enabling IME is what makes winit forward `Event::Ime` composition events to egui (and thus to the
    /// editors' `ime_handler` / code-editor IME arm); without it the OS sends no composition events and
    /// CJK/Japanese/Korean input is silently dead (RISK-3 / MC-3 / AC6). `false` until the first `ui()`
    /// frame sends the command. Skipped during a snapshot-capture pass (the throwaway capture context
    /// must stay side-effect-free).
    ime_allowed_sent: bool,
    /// WP-KERNEL-012 MT-033 (E5 — CKC drag-in): the live CKC / Atelier side panel mounted on the RIGHT
    /// edge of the shell (the `egui::SidePanel::right` mirror of the left activity rail). This is the
    /// drag SOURCE the rich-editor / canvas drop zones consume — it is rendered every frame so its
    /// `dnd_drag_source` item rows are reachable in the real product, not only in a standalone test
    /// harness. In the production shell it is `AtelierSidePanel::production(runtime)` (loads from the
    /// real `/atelier` backend off the UI thread); the headless/test shell has no runtime, so the panel
    /// stays idle/neutral (no perpetual spinner) until a test injects rows.
    atelier_side_panel: AtelierSidePanel,
    /// MT-033: whether the right-edge Atelier/CKC side panel is expanded. The fresh shell keeps it
    /// closed so the default work surface stays editor/chat-focused; tests or an affordance can open it
    /// when the drag source is needed.
    atelier_panel_open: bool,
    /// MT-033: the live Stage pane (the route-to-Stage DISPLAY surface). Held by the shell and mounted as
    /// a bottom panel; the per-frame bus drain (`take_pending_stage_content`) sets its content when a
    /// "Route to Stage" command is dispatched (palette or context menu). Read-only display — deeper Stage
    /// capture/embed-back is E10 (MT-066).
    stage_pane: StagePane,
    /// MT-033: whether the Stage pane (bottom panel) is shown. Starts closed; the shell drain OPENS it the
    /// frame content is routed in (so "Route to Stage" is observable, not a silent no-op). A future
    /// affordance / pane-close action can flip it back.
    stage_panel_open: bool,
}

/// MT-024 LOCAL drawer-action intents (no backend): the most recent Promote / SendToPane signal a swarm
/// reader observes and the host applies. Concurrency-safe (HBR-SWARM): held behind an `Arc<Mutex<..>>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrawerIntents {
    /// `Some(block_id)` when a Promote action asked to promote a stashed block into the active pane.
    pub promote_block_id: Option<String>,
    /// `Some((block_id, pane_id))` when a Send-to-pane action asked to route a block to a pane.
    pub send_to_pane: Option<(String, String)>,
}

/// An in-progress explorer-row rename (MT-020): the Loom block being renamed + the live edit buffer.
#[derive(Debug, Clone)]
pub struct PendingRename {
    /// The Loom block id the rename PATCHes.
    pub block_id: String,
    /// The live text buffer, seeded from the row's current title and edited in the rename dialog.
    pub text: String,
}

/// WP-KERNEL-012 MT-101: the compact model-session launch form. It stays in app state while the in-app
/// dialog is open; no foreground OS window is spawned and no model session is claimed until the backend
/// returns real evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSessionLaunchDialogState {
    pub provider: backend_client::ModelSessionProvider,
    pub workspace_folder: String,
    pub model_id: String,
    pub wrapper: String,
}

impl ModelSessionLaunchDialogState {
    pub fn new(workspace_folder: impl Into<String>) -> Self {
        Self {
            provider: backend_client::ModelSessionProvider::Local,
            workspace_folder: workspace_folder.into(),
            model_id: String::new(),
            wrapper: "repo-folder-wrapper-v1".to_owned(),
        }
    }

    fn is_ready(&self) -> bool {
        !self.workspace_folder.trim().is_empty()
            && !self.model_id.trim().is_empty()
            && !self.wrapper.trim().is_empty()
    }
}

/// The seed panes for a fresh editor-first work surface. MT-098 keeps feature pane factories registered,
/// but the default live work surface starts with code, Notes, and the typed-blocked Runtime Chat pane.
fn default_panes() -> Vec<PaneRecord> {
    let seeds: [(&str, PaneType); 3] = [
        ("pane-a", PaneType::CodeSymbol),
        ("pane-b", PaneType::LoomWikiPage),
        ("pane-c", PaneType::RuntimeChat),
    ];
    seeds
        .into_iter()
        .map(|(id, ty)| {
            PaneRecord::new(
                PaneId::from(id),
                ty,
                DEFAULT_PROJECT_ID,
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            )
        })
        .collect()
}

/// Register a `PlaceholderPaneFactory` for every `PaneType` variant. Concrete factories override
/// individual entries in later MTs. The `Placeholder` key uses an empty label as the catch-all
/// entry; render time still uses the record's own `Placeholder(label)` for display.
fn build_default_factories() -> HashMap<PaneType, Box<dyn PaneFactory>> {
    let variants = [
        PaneType::Workspace,
        PaneType::LoomDailyJournal,
        PaneType::LoomBlock,
        PaneType::LoomWikiPage,
        PaneType::AtelierEditor,
        PaneType::KernelDcc,
        PaneType::InferenceLab,
        PaneType::ModelRuntime,
        PaneType::Swarm,
        PaneType::Problems,
        PaneType::Jobs,
        PaneType::Timeline,
        PaneType::UserManual,
        PaneType::CodeSymbol,
        PaneType::SourceControl,
        PaneType::MediaDownloader,
        PaneType::FontManager,
        PaneType::FlightRecorder,
        PaneType::VisualDebugger,
        PaneType::LoomSearchV2,
        PaneType::FindInFiles,
        PaneType::RuntimeChat,
        PaneType::Placeholder(String::new()),
    ];
    let mut map: HashMap<PaneType, Box<dyn PaneFactory>> = HashMap::new();
    for v in variants {
        map.insert(v.clone(), Box::new(PlaceholderPaneFactory::new(v)));
    }
    map
}

/// The factory map plus the two shared cells the shell keeps live (LoomSearchV2 + Find-in-Files) plus
/// the WP-KERNEL-012 MT-079 editor-mount handles: named to keep
/// [`build_factories_with_loom_search_v2`]'s return shape readable (clippy::type_complexity).
type FactoriesWithSharedCells = (
    HashMap<PaneType, Box<dyn PaneFactory>>,
    Arc<Mutex<crate::loom_search_v2::LoomSearchV2PaneShared>>,
    Arc<Mutex<crate::find_in_files::FindInFilesPaneShared>>,
    Arc<Mutex<crate::runtime_chat::RuntimeChatPanel>>,
    EditorMountHandles,
);

/// WP-KERNEL-012 MT-079: the live handles the shell keeps for the MOUNTED editor panes, so the running
/// `HandshakeApp` can (1) push the active workspace into the editors' session context each frame,
/// (2) drive the SAME code panel / rich state the mounted panes show (the AC-079 proofs), (3) drain the
/// code command channel (Save/Undo/Redo/OpenCommandPalette) into the shell command bus + unified undo,
/// and (4) drain the rich pane's editor events into the MT-030 nav bus. These are the editor analogue of
/// `loom_search_v2_shared` / `find_in_files_shared`.
struct EditorMountHandles {
    /// The session-context cell every editor mount reads each frame; the shell overwrites it with the
    /// active workspace + runtime so the editors thread real session context on mount (AC-079-2).
    session: SharedSessionContext,
    /// The Arc-shared code panel behind the mounted code pane (AC-079-3 proof drives this directly).
    code_panel: Arc<CodeEditorPanel>,
    /// The Arc-shared rich editor state behind the mounted Notes pane (AC-079-5 proof drives this).
    rich_state: Arc<Mutex<RichEditorState>>,
    /// The receiver half of the code pane's command channel (Save/Undo/Redo/OpenCommandPalette). The
    /// shell drains it each frame and dispatches to the WP-011 command bus + MT-035 unified undo.
    command_rx: std::sync::mpsc::Receiver<CodeEditorAction>,
    /// The outbound rich-pane event queue (WikilinkActivated/BacklinkActivated/TagActivated). The shell
    /// drains it each frame and routes each event to the MT-030 nav bus.
    rich_events: RichPaneEvents,
    /// WP-KERNEL-012 MT-080 (E11 host-mount, part 2): the live handles for the SECONDARY mounted panes
    /// (canvas / graph / outgoing-links / relevant-memory / Stage / daily-journal / manual). The shell
    /// pushes the live palette into `secondary_palette` each frame and drains the per-pane outbound queues.
    secondary: SecondaryMountHandles,
}

/// WP-KERNEL-012 MT-080: the live handles the shell keeps for the SECONDARY mounted panes. Each `state`
/// is the SAME `Arc<Mutex<_>>` the registered factory renders, so the shell can drive it (the AC-080
/// proofs) and drain its outbound event queue. `palette` is the one shared cell every secondary factory
/// reads; the shell overwrites it from the active theme each frame (the canvas/graph/side-pane widgets read
/// theme tokens, never hardcoded hex — CONTROL-4).
struct SecondaryMountHandles {
    /// The shared theme palette every secondary factory reads each frame (pushed from the active theme).
    palette: crate::editor_pane_factories::SharedPalette,
    /// The canvas board state behind the mounted canvas pane + its outbound CanvasEvent queue.
    canvas_board: Arc<Mutex<crate::graph::canvas_board::LoomCanvasBoard>>,
    canvas_events: Arc<Mutex<Vec<crate::graph::canvas_board::CanvasEvent>>>,
    /// The graph view state behind the mounted graph pane + its outbound GraphEvent queue.
    graph_view: Arc<Mutex<crate::graph::graph_view::LoomGraphView>>,
    graph_events: Arc<Mutex<Vec<crate::graph::graph_view::GraphEvent>>>,
    /// The outgoing-links panel state + its outbound nav-target queue.
    outgoing_links:
        Arc<Mutex<crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel>>,
    outgoing_nav: Arc<Mutex<Vec<crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget>>>,
    /// The relevant-memory panel state + its outbound memory-nav queue.
    relevant_memory: Arc<Mutex<crate::fems::relevant_memory_panel::RelevantMemoryPanel>>,
    relevant_memory_nav: Arc<Mutex<Vec<crate::fems::relevant_memory_panel::MemoryNavTarget>>>,
    /// The Stage pane state + the embed-back request flag.
    stage: Arc<Mutex<crate::stage_pane::StagePane>>,
    stage_embed_requested: Arc<std::sync::atomic::AtomicBool>,
    /// The daily-journal panel state + its outbound DailyJournalEvent queue.
    daily_journal: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
    daily_journal_events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
    /// The manual pane search/selection state (the registry content is immutable, held by the factory).
    manual_state: Arc<Mutex<crate::manual_pane::ManualPaneState>>,
    /// True once the relevant-memory pane has requested its first FEMS fetch (so the EndpointMissing
    /// blocker is surfaced exactly once — the route is verified ABSENT in this build).
    relevant_memory_fetched: std::sync::atomic::AtomicBool,
}

/// Build the pane factory map AND install the CONCRETE [`LoomSearchV2PaneFactory`] over its placeholder
/// (MT-028, AC-9): start from the all-placeholder default, then OVERRIDE `PaneType::LoomSearchV2` with a
/// real factory that renders [`crate::loom_search_v2::show`] through the verified
/// [`LoomSearchV2Client`](crate::backend_client::LoomSearchV2Client). Returns the map plus the shared
/// cell the shell keeps (to push the active workspace id + live palette in and drain clicked block ids
/// out each frame). `runtime` bridges the search/save HTTP off the UI thread (HBR-QUIET); `palette`
/// seeds the shared cell (overwritten every frame from the live theme).
fn build_factories_with_loom_search_v2(
    runtime: tokio::runtime::Handle,
    palette: theme::HsPalette,
) -> FactoriesWithSharedCells {
    let mut map = build_default_factories();
    let shared = Arc::new(Mutex::new(
        crate::loom_search_v2::LoomSearchV2PaneShared::new(palette.clone()),
    ));
    let client = crate::backend_client::LoomSearchV2Client::production(runtime.clone());
    let factory = crate::loom_search_v2::LoomSearchV2PaneFactory::new(client, Arc::clone(&shared));
    // Insert AFTER the placeholder fill so this concrete factory wins for the LoomSearchV2 variant.
    map.insert(PaneType::LoomSearchV2, Box::new(factory));

    // MT-029: install the CONCRETE FindInFilesPaneFactory over its placeholder so opening the
    // "Find in Files" pane renders the REAL panel, wired to the verified graph-search + bookmark +
    // rich-document save routes. Mirrors the LoomSearchV2 factory wiring exactly.
    let fif_shared = Arc::new(Mutex::new(
        crate::find_in_files::FindInFilesPaneShared::new(palette.clone()),
    ));
    let search_client = crate::backend_client::WorkspaceSearchClient::production(runtime.clone());
    let doc_client = crate::backend_client::RichDocClient::production(runtime);
    let fif_factory = crate::find_in_files::FindInFilesPaneFactory::new(
        search_client,
        doc_client,
        Arc::clone(&fif_shared),
    );
    map.insert(PaneType::FindInFiles, Box::new(fif_factory));

    // MT-098: install the concrete Runtime Chat pane beside the editor work surface. The production
    // client returns a typed EndpointMissing blocker until the native HTTP chat route exists; it does not
    // target the Flight Recorder ingestion route as a fake assistant backend.
    let runtime_chat_panel = Arc::new(Mutex::new(
        crate::runtime_chat::RuntimeChatPanel::production(palette.clone()),
    ));
    map.insert(
        PaneType::RuntimeChat,
        Box::new(crate::runtime_chat::ChatPaneFactory::new(Arc::clone(
            &runtime_chat_panel,
        ))),
    );

    // WP-KERNEL-012 MT-079 (E11 host-mount, CORE): install the REAL editor pane factories over their
    // PlaceholderPaneFactory entries so the native code + rich-text editors render LIVE in the running
    // app. PaneType::CodeSymbol -> the code editor; PaneType::LoomWikiPage -> the Notes/rich editor (the
    // surfaces the WP-011 shell already routes those panes to — no new PaneType variant is forked). The
    // session-threaded MOUNT wrappers thread runtime/workspace/embed/wikilink context on mount through
    // the SAME shared-cell pattern (the `session` cell the shell overwrites each frame).
    let editor_mounts = install_editor_mounts(&mut map);

    (map, shared, fif_shared, runtime_chat_panel, editor_mounts)
}

/// WP-KERNEL-012 MT-079: build the session-threaded editor mount factories, install them over the
/// placeholder entries for `PaneType::CodeSymbol` (code editor) + `PaneType::LoomWikiPage` (Notes/rich
/// editor), and return the live handles the shell keeps. Reuses the existing `CodeEditorPaneFactory` /
/// `RichEditorPaneFactory` (no editor logic re-implemented); the mount wrappers only add the
/// session-context threading + the command/event drains the host-mount needs.
fn install_editor_mounts(map: &mut HashMap<PaneType, Box<dyn PaneFactory>>) -> EditorMountHandles {
    // The session-context cell every mount reads; starts UNbound (empty workspace, no runtime). The shell
    // overwrites it with the active workspace + runtime each frame (sync_editor_session), at which point
    // the mounts thread real session context into the editors (AC-079-2).
    let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::default()));

    // CODE pane: a small seed snippet so the mounted pane shows a real editor (a fresh shell with no
    // open file). The command channel routes Save/Undo/Redo/OpenCommandPalette to the shell bus.
    let code_panel = Arc::new(CodeEditorPanel::new(CODE_EDITOR_SEED, "rs"));
    let (command_tx, command_rx) = std::sync::mpsc::channel::<CodeEditorAction>();
    let code_mount =
        CodeEditorPaneMount::new(Arc::clone(&code_panel), Arc::clone(&session), command_tx);
    map.insert(PaneType::CodeSymbol, Box::new(code_mount));

    // RICH/Notes pane: a demo document so the mounted Notes pane shows a real rich editor. The outbound
    // event queue carries the editor's drained pending_events to the shell's nav-bus routing (AC-079-5).
    let rich_state = Arc::new(Mutex::new(RichEditorState::demo()));
    let rich_events = RichPaneEvents::new();
    let rich_mount = RichEditorPaneMount::new(
        Arc::clone(&rich_state),
        Arc::clone(&session),
        rich_events.clone(),
        // No specific document open yet on a fresh Notes pane: the wikilink context binds to the
        // workspace root (create/resolve still resolves against the workspace). A future MT that opens a
        // document by tab content_id updates the bound document id.
        String::new(),
    );
    map.insert(PaneType::LoomWikiPage, Box::new(rich_mount));

    // WP-KERNEL-012 MT-080 (E11 host-mount, part 2): install the SECONDARY pane factories over their
    // placeholders so the canvas / graph / side panes render LIVE too.
    let secondary = install_secondary_mounts(map);

    EditorMountHandles {
        session,
        code_panel,
        rich_state,
        command_rx,
        rich_events,
        secondary,
    }
}

/// WP-KERNEL-012 MT-080: build the SECONDARY pane factories (canvas / graph / outgoing-links /
/// relevant-memory / Stage / daily-journal / manual), install them over their `PlaceholderPaneFactory`
/// entries, and return the live handles the shell keeps. Reuses the existing widget structs (no widget
/// logic re-implemented); the mount wrappers only thread the shared palette + collect the per-pane
/// outbound events the shell drains. Each factory is registered over the SAME `PaneType` the pane's
/// `register_*_pane` record uses, so a docked secondary pane renders the real widget instead of a
/// placeholder.
fn install_secondary_mounts(
    map: &mut HashMap<PaneType, Box<dyn PaneFactory>>,
) -> SecondaryMountHandles {
    use crate::editor_pane_factories::{
        CanvasBoardPaneMount, DailyJournalPaneMount, GraphViewPaneMount, ManualPaneMount,
        OutgoingLinksPaneMount, RelevantMemoryPaneMount, StagePaneMount,
    };

    // One shared palette cell, seeded with the default dark palette; the shell overwrites it from the
    // active theme each frame (sync_editor_session) so the widgets track a runtime theme toggle.
    let palette: crate::editor_pane_factories::SharedPalette =
        Arc::new(Mutex::new(HsTheme::Dark.palette()));

    // ── Canvas board (PaneType::AtelierEditor) ───────────────────────────────────────────────────────
    let canvas_board = Arc::new(Mutex::new(
        crate::graph::canvas_board::LoomCanvasBoard::new(
            DEFAULT_PROJECT_ID,
            SECONDARY_CANVAS_BLOCK_ID,
        ),
    ));
    let canvas_events = Arc::new(Mutex::new(Vec::new()));
    map.insert(
        PaneType::AtelierEditor,
        Box::new(CanvasBoardPaneMount::new(
            Arc::clone(&canvas_board),
            Arc::clone(&palette),
            Arc::clone(&canvas_events),
        )),
    );

    // ── Graph view (PaneType::KernelDcc) ─────────────────────────────────────────────────────────────
    let mut gv = crate::graph::graph_view::LoomGraphView::default();
    gv.workspace_id = DEFAULT_PROJECT_ID.to_owned();
    let graph_view = Arc::new(Mutex::new(gv));
    let graph_events = Arc::new(Mutex::new(Vec::new()));
    map.insert(
        PaneType::KernelDcc,
        Box::new(GraphViewPaneMount::new(
            Arc::clone(&graph_view),
            Arc::clone(&palette),
            Arc::clone(&graph_events),
        )),
    );

    // ── Outgoing-links side pane (PaneType::LoomBlock) ───────────────────────────────────────────────
    let outgoing_links = Arc::new(Mutex::new(
        crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel::new(),
    ));
    let outgoing_nav = Arc::new(Mutex::new(Vec::new()));
    map.insert(
        PaneType::LoomBlock,
        Box::new(OutgoingLinksPaneMount::new(
            Arc::clone(&outgoing_links),
            Arc::clone(&palette),
            Arc::clone(&outgoing_nav),
        )),
    );

    // ── Relevant-memory side pane (PaneType::Placeholder("Relevant Memory")) ─────────────────────────
    let relevant_memory = Arc::new(Mutex::new(
        crate::fems::relevant_memory_panel::RelevantMemoryPanel::new(),
    ));
    let relevant_memory_nav = Arc::new(Mutex::new(Vec::new()));
    map.insert(
        PaneType::Placeholder("Relevant Memory".to_owned()),
        Box::new(RelevantMemoryPaneMount::new(
            Arc::clone(&relevant_memory),
            Arc::clone(&palette),
            Arc::clone(&relevant_memory_nav),
        )),
    );

    // ── Stage pane (PaneType::Placeholder("Stage")) ──────────────────────────────────────────────────
    let stage = Arc::new(Mutex::new(crate::stage_pane::StagePane::new()));
    let stage_embed_requested = Arc::new(std::sync::atomic::AtomicBool::new(false));
    map.insert(
        PaneType::Placeholder("Stage".to_owned()),
        Box::new(StagePaneMount::new(
            Arc::clone(&stage),
            Arc::clone(&palette),
            Arc::clone(&stage_embed_requested),
        )),
    );

    // ── Daily-journal pane (PaneType::LoomDailyJournal) ──────────────────────────────────────────────
    let daily_journal = Arc::new(Mutex::new(
        crate::graph::daily_journal_panel::DailyJournalState::new(
            crate::rich_editor::daily_notes::date_nav::DateNav::today_now(),
        ),
    ));
    let daily_journal_events = Arc::new(Mutex::new(Vec::new()));
    map.insert(
        PaneType::LoomDailyJournal,
        Box::new(DailyJournalPaneMount::new(
            Arc::clone(&daily_journal),
            Arc::clone(&palette),
            Arc::clone(&daily_journal_events),
        )),
    );

    // ── User-manual pane (PaneType::UserManual) ──────────────────────────────────────────────────────
    let mut manual_registry = crate::manual_pane::ManualRegistry::new();
    manual_registry.register_section(crate::manual_content_editors::editors_manual_section());
    let manual_registry = Arc::new(manual_registry);
    let manual_state = Arc::new(Mutex::new(crate::manual_pane::ManualPaneState::default()));
    map.insert(
        PaneType::UserManual,
        Box::new(ManualPaneMount::new(
            Arc::clone(&manual_registry),
            Arc::clone(&manual_state),
            Arc::clone(&palette),
        )),
    );

    SecondaryMountHandles {
        palette,
        canvas_board,
        canvas_events,
        graph_view,
        graph_events,
        outgoing_links,
        outgoing_nav,
        relevant_memory,
        relevant_memory_nav,
        stage,
        stage_embed_requested,
        daily_journal,
        daily_journal_events,
        manual_state,
        relevant_memory_fetched: std::sync::atomic::AtomicBool::new(false),
    }
}

/// WP-KERNEL-012 MT-080: the canvas block id a freshly mounted canvas pane binds to before a specific
/// canvas is opened (a stable per-workspace default board; opening a specific canvas updates it in a
/// follow-on run). Disk-agnostic, no spaces.
const SECONDARY_CANVAS_BLOCK_ID: &str = "default-canvas";

/// WP-KERNEL-012 MT-079: the seed snippet a freshly mounted code pane shows before a file is opened.
const CODE_EDITOR_SEED: &str = "\
// Handshake native code editor (VS Code parity).
fn main() {
    let greeting = \"hello\";
    println!(\"{greeting}\");
}";

/// Build a seeded registry from the default panes.
fn seeded_registry() -> PaneRegistry {
    let mut reg = PaneRegistry::new();
    for record in default_panes() {
        reg.insert(record);
    }
    reg
}

/// Seed one tab bar per default pane (MT-007). Each pane opens with a single tab matching its seed
/// `PaneType`, so a fresh work surface shows a coherent "one tab per pane" state that the operator or
/// an agent can then add/close/reorder/pin. Mirrors the registry's `default_panes` shape so the two
/// stay aligned (the live-tree test asserts each seeded pane has a tab bar).
fn default_tab_bar_states() -> HashMap<PaneId, TabBarState> {
    default_panes()
        .into_iter()
        .map(|record| {
            let tabs = vec![TabState::new(record.pane_type.clone())];
            let bar = TabBarState::new(record.pane_id.clone(), tabs);
            (record.pane_id.clone(), bar)
        })
        .collect()
}

/// The project-tab strip a fresh shell shows before the `/workspaces` fetch resolves (MT-011): a
/// single tab for the seeded `default-project`, marked active. Once the background fetch returns the
/// real workspace list, `apply_fetched` replaces this with the backend's projects (or the "No
/// projects" placeholder if the list is empty). Seeding with the default project keeps the strip
/// non-empty and the active highlight consistent with `active_project_id` from the first frame.
fn default_project_tabs() -> ProjectTabBar {
    ProjectTabBar::new(
        vec![ProjectItem::new(DEFAULT_PROJECT_ID, "Default Project")],
        DEFAULT_PROJECT_ID,
    )
}

/// MT-021 status-bar `open_panel` mapping: the human-readable name of the panel a status-bar segment's
/// "Open …" item opens. Returns `None` for a segment with no related panel (its `open_panel` item is
/// then disabled + disclosed). The contract's lookup names ("System Status" for health, "Source
/// Control" for branch); WP-011 has NO `PaneType::SystemStatus`, so the HEALTH segment maps to the REAL
/// `Problems` pane (the system-status/diagnostics surface that exists) and the name is "Problems" so
/// the menu label matches the pane actually opened. The BRANCH segment maps to the real `SourceControl`
/// pane exactly as the contract intends. This keeps the menu honest (no fake "System Status" pane).
fn statusbar_related_panel_name(segment_id: &str) -> Option<String> {
    statusbar_related_pane_type(segment_id).map(|pt| pt.label())
}

/// MT-021 status-bar `open_panel` mapping: the REAL `PaneType` a status-bar segment's "Open …" item
/// opens. `None` disables `open_panel`. See [`statusbar_related_panel_name`] for the SystemStatus
/// deviation rationale.
fn statusbar_related_pane_type(segment_id: &str) -> Option<PaneType> {
    match segment_id {
        // health → Problems (no PaneType::SystemStatus exists in WP-011; Problems is the real
        // system-status/diagnostics surface — disclosed deviation).
        "health" => Some(PaneType::Problems),
        // branch → SourceControl (exactly the contract's mapping; SourceControl is a real PaneType).
        "branch" => Some(PaneType::SourceControl),
        _ => None,
    }
}

/// Today's date in `YYYY-MM-DD` (UTC), for the MT-023 Agenda daily-journal fetch
/// (`PUT /loom/journals/{today}`). Computed from `SystemTime` without a chrono dependency (this crate
/// has none — adding one for a single date format would be unnecessary) via the well-known days→civil
/// algorithm (Howard Hinnant, public domain). The journal endpoint validates `%Y-%m-%d` exactly, so the
/// format is zero-padded. UTC is the right basis: the daily-journal block is keyed by a calendar date,
/// and the backend's `open_daily_journal` likewise works in calendar-date terms.
fn today_utc_ymd() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Convert a count of days since the Unix epoch (1970-01-01) into a `(year, month, day)` civil date.
/// Howard Hinnant's `civil_from_days` algorithm (public domain), valid for the full proleptic Gregorian
/// range; here only ever called with present-day positive day counts.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The placeholder UI-tree snapshot the MCP slot (MT-027) holds before the first frame publishes the
/// real live tree. A single `Window` root with `widget_count` 1, so `list_widgets` over the wire before
/// the first frame returns a well-formed (if empty) snapshot rather than a lock on uninitialized state.
fn empty_snapshot() -> crate::accessibility::UiTreeSnapshot {
    crate::accessibility::UiTreeSnapshot {
        root: crate::accessibility::UiTreeNode {
            id: "node:root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children: Vec::new(),
        },
        captured_at_utc: "0.000000000Z".to_owned(),
        widget_count: 1,
    }
}

/// Bundled Inter font bytes, embedded at compile time (MT-004). Gated behind `bundled-fonts`
/// (ON by default from MT-004). When the feature is OFF, font loading is skipped and eframe's
/// default fonts are used — never a panic (RISK-6 / CONTROL-6). build.rs fails fast with a clear
/// message if the asset is missing while the feature is enabled.
#[cfg(feature = "bundled-fonts")]
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
#[cfg(feature = "bundled-fonts")]
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");

/// AccessKit/egui font family name for the bold Inter face. A named family (rather than replacing
/// the Proportional default) so callers can opt into bold text via `FontFamily::Name("Inter-Bold")`
/// without changing the default proportional rendering.
#[cfg(feature = "bundled-fonts")]
pub const INTER_BOLD_FAMILY: &str = "Inter-Bold";

// ── WP-KERNEL-012 MT-075: Unicode font fallback chain (CJK + symbols + emoji) ─────────────────────
//
// ROOT CAUSE this MT fixes: before MT-075, `install_fonts` registered ONLY Inter (Latin + Cyrillic +
// Greek). egui's `FontDefinitions::default()` ships no CJK and only partial symbol coverage, so any
// Han / Kana / Hangul codepoint rendered as the tofu/notdef box. egui resolves a glyph by walking the
// family's `Vec<String>` fallback list IN ORDER and using the FIRST font that has the glyph, so the
// fix is to APPEND broad-coverage faces AFTER Inter (Inter stays index-0 / first, so the Latin look is
// byte-for-byte unchanged — MC-1 / RISK-2 / AC1).
//
// CJK-SOURCE DECISION (bundle, not OS-load — AC7 / PROOF4): the MT contract preferred (a/b) bundling a
// deterministic, portable Noto Sans CJK weight IF fetchable, over (c) OS-loading the Windows fonts.
// The fonts WERE fetchable (Google Noto, SIL OFL 1.1), so they are bundled under assets/fonts/ via
// `include_bytes!`. Bundling is chosen over OS-load because it is DETERMINISTIC and DISK-AGNOSTIC
// (GLOBAL-PORTABILITY): the binary renders identical glyphs on any machine with no dependency on the
// host having a CJK font installed, and there is no `std::fs` font read that could fail/panic.
//
// FONT SET + WHY (verified per-glyph with fontTools against the AC2/AC4/AC5 codepoints):
//   - Noto Sans SC (Simplified Chinese): supplies Han ideographs (中文, 日本語 kanji) + Hiragana/Katakana
//     + CJK box-drawing (│ ─ ┌, which Inter LACKS) with region-correct Simplified Han glyph shapes.
//   - Noto Sans KR (Korean): supplies Hangul (한국어), which neither Inter nor the SC subset cover.
//     Ordered AFTER SC so Han/Kana resolve to the region-correct SC face first; KR only catches Hangul.
//   - Noto Sans Symbols 2: a broad-Unicode symbol backstop for symbol/dingbat codepoints beyond Inter's
//     set (e.g. ✗ U+2717, geometric/technical symbols) — AC5.
//   - Noto Sans Math: broad math-symbol backstop (Inter + SC already cover ∑ ∫ ∞ →; Math is the
//     extended-coverage fallback so uncommon math operators never tofu) — AC5.
//   - Emoji (😀): supplied by egui's DEFAULT `NotoEmoji` face, which `FontDefinitions::default()`
//     already appends to every family — we RETAIN it (do not clear the defaults) so emoji still render.
//
// SIZE DELTA (RISK-1 / MC-4 / PROOF4): the four bundled Noto faces add ~14 MB to the binary
// (SC 8.3 MB, KR 4.6 MB, Symbols2 0.66 MB, Math 0.97 MB). This is the documented cost of deterministic
// CJK coverage; egui builds glyph atlases lazily (on first use), so startup is unaffected and only the
// first frame that paints a given script pays a one-time atlas-build cost. A regional subset (SC+KR
// rather than the ~16 MB+ pan-CJK OTC) is used per the contract's "do not stack the whole OTC" note.
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_SC: &[u8] = include_bytes!("../assets/fonts/NotoSansSC-Regular.otf");
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_KR: &[u8] = include_bytes!("../assets/fonts/NotoSansKR-Regular.otf");
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_SYMBOLS2: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_MATH: &[u8] = include_bytes!("../assets/fonts/NotoSansMath-Regular.ttf");

// ── WP-KERNEL-012 MT-078 (E13): RTL + complex-script GLYPH coverage (Hebrew / Arabic / Devanagari) ──
//
// ROOT CAUSE this MT extends (KERNEL_BUILDER gate 2026-06-26): MT-075 bundled CJK + symbol + math faces
// but NO Hebrew / Arabic / Devanagari, so the Tier-1 RTL/bidi deliverable (a right-aligned Hebrew or
// Arabic paragraph) would render as tofu — the bidi REORDER would be invisible because there are no
// glyphs to reorder. MT-078 APPENDS three more Noto faces AFTER the MT-075 fallback chain (Inter stays
// index-0; the CJK faces keep their order), so:
//   - Noto Sans Hebrew supplies Hebrew (שלום עולם) — the HONEST RTL proof case (Hebrew is NON-JOINING, so
//     it needs NO cursive shaping; egui renders its glyphs correctly once they are in the chain, and the
//     bidi reorder + right-align makes a Hebrew paragraph correct end-to-end). This is the AC1/AC4 face.
//   - Noto Sans Arabic supplies Arabic (العربية) glyphs. IMPORTANT (the Tier-3 honesty boundary): this
//     face carries GSUB/GPOS cursive-joining tables, but egui/epaint does NOT execute them, so Arabic
//     renders in ISOLATED letter forms (disconnected), NOT cursive-joined. That is the documented
//     typed-limitation (AC5 / PROOF3 / RISK-1): the glyphs are PRESENT (not tofu) but unshaped, and the
//     editor surfaces a VISIBLE "Arabic cursive shaping limited" note — never silently-broken Arabic.
//   - Noto Sans Devanagari supplies Indic (नमस्ते) glyphs; same Tier-3 caveat (Indic reordering/conjuncts
//     need a shaping engine egui lacks), so it is glyph-coverage only with the same typed limitation.
//
// APPEND-ONLY (MC-3 / RISK-2 / AC6): these faces go at the END of FALLBACK_FACE_ORDER, after the CJK
// faces, so Latin still resolves to Inter (index-0) and CJK still resolves to SC/KR first — no regression
// to MT-075/077. SIL OFL 1.1 (see assets/fonts/NotoSans-OFL.txt); bundled via include_bytes! (DETERMINISTIC
// + DISK-AGNOSTIC, no OS-font dependency, no std::fs panic path — same decision as MT-075). The three faces
// are hinted-TTF subsets (~0.03 MB Hebrew, ~0.23 MB Arabic, ~0.24 MB Devanagari ≈ 0.5 MB total) — far
// smaller than the CJK faces, so the binary-size delta is negligible.
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_HEBREW: &[u8] = include_bytes!("../assets/fonts/NotoSansHebrew-Regular.ttf");
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_ARABIC: &[u8] = include_bytes!("../assets/fonts/NotoSansArabic-Regular.ttf");
#[cfg(feature = "bundled-fonts")]
const NOTO_SANS_DEVANAGARI: &[u8] =
    include_bytes!("../assets/fonts/NotoSansDevanagari-Regular.ttf");

/// egui `font_data` key for the primary proportional face (Inter). Always index-0 / first in both the
/// Proportional and Monospace fallback vecs so the Latin look is unchanged (MC-1 / AC1).
pub const FONT_KEY_INTER: &str = "Inter";
/// egui `font_data` key for the Simplified-Chinese CJK face (Han + Kana + box-drawing).
pub const FONT_KEY_NOTO_SC: &str = "NotoSansSC";
/// egui `font_data` key for the Korean CJK face (Hangul; Han fallback after SC).
pub const FONT_KEY_NOTO_KR: &str = "NotoSansKR";
/// egui `font_data` key for the broad-Unicode symbol backstop face.
pub const FONT_KEY_NOTO_SYMBOLS2: &str = "NotoSansSymbols2";
/// egui `font_data` key for the broad math-symbol backstop face.
pub const FONT_KEY_NOTO_MATH: &str = "NotoSansMath";
/// egui `font_data` key for the Hebrew face (MT-078 RTL — the non-joining honest RTL proof case).
pub const FONT_KEY_NOTO_HEBREW: &str = "NotoSansHebrew";
/// egui `font_data` key for the Arabic face (MT-078 — glyphs present, but egui does NOT cursive-shape
/// them; see the typed limitation in `text_intl::bidi`).
pub const FONT_KEY_NOTO_ARABIC: &str = "NotoSansArabic";
/// egui `font_data` key for the Devanagari/Indic face (MT-078 — same unshaped Tier-3 caveat as Arabic).
pub const FONT_KEY_NOTO_DEVANAGARI: &str = "NotoSansDevanagari";

/// The Unicode-coverage fallback faces appended (in this exact order) AFTER the primary Inter face to
/// BOTH the Proportional and Monospace families. Inter is NOT in this list — it is inserted at index 0
/// separately so it always wins for Latin/Cyrillic/Greek (RISK-2). The MT-075 family-order unit test
/// asserts each family vec equals `[Inter, ..FALLBACK_FACE_ORDER, <egui defaults: Proportional/Monospace,
/// Emoji>]`. Emoji is NOT listed here because egui's `FontDefinitions::default()` already appends its
/// `NotoEmoji` face to every family and we retain those defaults.
///
/// MT-078 APPENDS the three RTL/complex-script faces (Hebrew, Arabic, Devanagari) at the END of this
/// list, after the CJK + symbol + math faces, so Latin (Inter index-0) and CJK (SC/KR first) ordering
/// is byte-for-byte unchanged (MC-3 / RISK-2 / AC6) and a Hebrew/Arabic/Indic codepoint resolves to its
/// real glyph instead of tofu (AC1). The MT-078 RTL faces only ADD glyph coverage — the bidi REORDER +
/// right-align happens in `text_intl::bidi`, and Arabic/Indic cursive SHAPING is the typed limitation
/// (egui does not run GSUB/GPOS), never silently-broken text.
pub const FALLBACK_FACE_ORDER: [&str; 7] = [
    FONT_KEY_NOTO_SC,
    FONT_KEY_NOTO_KR,
    FONT_KEY_NOTO_SYMBOLS2,
    FONT_KEY_NOTO_MATH,
    FONT_KEY_NOTO_HEBREW,
    FONT_KEY_NOTO_ARABIC,
    FONT_KEY_NOTO_DEVANAGARI,
];

impl HandshakeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::install_fonts(&cc.egui_ctx);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build tokio runtime");
        // WP-KERNEL-012 MT-082/105: install the diagnostics ring BEFORE the operation watchdog can emit.
        // A stalled constructor health probe must land in the shared ring when a writer is available,
        // not initialize the recorder writer-less first.
        let diag_session = Self::install_diagnostics_ring();
        crate::diagnostics::start_global_operation_watchdog();
        // Fire-once, non-blocking health poll: window opens immediately, label shows Loading...
        let health_handle = Some(Self::spawn_health_probe(
            rt.handle(),
            HEALTH_URL.to_owned(),
            None,
        ));
        // Fire-once, non-blocking workspace list fetch (MT-011): the shell opens immediately with the
        // seeded default-project tab; when the fetch resolves, the real workspace tabs replace it.
        let workspaces_handle =
            Some(rt.spawn(async { fetch_workspaces(backend_client::BACKEND_BASE_URL).await }));
        // Real transport: the backend's PostgreSQL-authoritative layout REST endpoint, bridged onto
        // this app's tokio runtime handle. No local file authority (CX-503S / Data Posture).
        let transport = WorkbenchLayoutClient::production(rt.handle().clone());
        let layout_manager = Arc::new(Mutex::new(LayoutPersistenceManager::new(
            Box::new(transport),
            LAYOUT_SAVE_DEBOUNCE,
        )));
        // Clone the runtime handle BEFORE moving `rt` into the struct, so the left-rail project tree can
        // spawn its async document/canvas loads on the same multi-thread runtime (MT-014).
        let rt_handle = rt.handle().clone();
        // MT-017: the REAL Loom-graph search transport, bridged onto the app runtime (MT-009 pattern).
        let quick_switcher_transport: Option<
            Arc<dyn crate::quick_switcher::LoomGraphSearchTransport>,
        > = Some(Arc::new(
            crate::quick_switcher::LoomGraphSearchClient::production(rt_handle.clone()),
        ));
        // MT-018: the REAL settings transport, bridged onto the app runtime (MT-009 pattern).
        let settings_transport: Option<Arc<dyn crate::workspace_settings::SettingsTransport>> =
            Some(Arc::new(
                crate::workspace_settings::SettingsClient::production(rt_handle.clone()),
            ));
        // MT-014 FIX-B: the in-process shell event bus, constructed once at app construction (the
        // "subscribe at app/LeftRail construction" control). Drained each frame in `ui()`.
        let (event_bus_tx, event_bus_rx) = new_shell_event_bus();
        // MT-028 + MT-029: install the CONCRETE LoomSearchV2 + FindInFiles pane factories over their
        // placeholders so opening the "Loom Search" / "Find in Files" panes renders the REAL panels,
        // wired to the verified search/save/bookmark routes.
        let (
            factories,
            loom_search_v2_shared,
            find_in_files_shared,
            runtime_chat_panel,
            editor_mounts,
        ) = build_factories_with_loom_search_v2(rt_handle.clone(), HsTheme::Dark.palette());
        // WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2): capture the STATIC GPU/driver
        // identity ONCE from the already-initialized eframe wgpu render state (`cc.wgpu_render_state`).
        // This reads the EXISTING adapter eframe created (no second wgpu device — RISK-006-5) and is
        // `None` only when there is no wgpu render state (a non-wgpu/headless harness). The integer codes
        // are ring-safe; the human strings stay in the in-process GpuInfo for the panel (AC-006-3).
        let gpu_info = crate::diagnostics::GpuInfo::capture(cc);
        if let Some(gpu) = &gpu_info {
            tracing::info!(
                vendor_id = gpu.vendor_id,
                device_id = gpu.device_id,
                device_type_code = gpu.device_type_code,
                backend_code = gpu.backend_code,
                adapter = %gpu.name,
                "internal_diagnostics GPU identity captured (Tier 2 §5.8.2 resource counters)"
            );
        }
        let mut app = Self {
            health_status: HealthDisplayState::Loading,
            rt,
            health_handle,
            diag_session,
            // WP-KERNEL-012 MT-094: the Palmistry watcher is launched by `main()` BEFORE the event loop
            // (NOT here in `new`) so the kittest suite never spawns a child; `main()`'s eframe closure
            // installs the handle via `set_palmistry_handle` after constructing the app.
            palmistry: None,
            // MT-084: the UI-thread heartbeat counter starts at 0 (first frame publishes 1) and the
            // monotonic clock starts now (process start), so the heartbeat timestamp is elapsed-since-
            // construction nanos — strictly increasing, never affected by a wall-clock change.
            frame_counter: 0,
            heartbeat_clock: std::time::Instant::now(),
            // MT-085: the per-frame frame-time tracker starts empty (all-zero stats) and is fed the
            // WORK time of `self.ui(ctx)` once per frame; no synthetic test work in production.
            frame_timer: crate::diagnostics::FrameTimer::new(),
            extra_frame_work_micros: 0,
            // MT-086: the per-process CPU%/RSS sampler (current pid only) ticked at a bounded ~1s cadence
            // in `update`, and the GPU identity captured once above from cc.wgpu_render_state.
            resource_sampler: crate::diagnostics::ResourceSampler::new(),
            gpu_info,
            // Desktop default mirrors the React app's dark default.
            current_theme: HsTheme::Dark,
            last_applied_theme: None,
            pane_registry: Arc::new(Mutex::new(seeded_registry())),
            factories,
            loom_search_v2_shared,
            find_in_files_shared,
            runtime_chat_panel,
            editor_mounts,
            rich_doc_base_url: backend_client::BACKEND_BASE_URL.to_owned(),
            rich_doc_load_cell: Arc::new(Mutex::new(VecDeque::new())),
            rich_doc_loading_id: None,
            rich_doc_load_generation: 0,
            rich_doc_loaded_id: None,
            rich_doc_loaded_version: None,
            rich_doc_load_error: None,
            last_editor_command: None,
            split_weights: SplitWeights::default(),
            split_drag: SplitDragState::default(),
            active_pane: None,
            tab_bar_states: default_tab_bar_states(),
            popout_manager: PopOutManager::new(),
            pop_out_request: None,
            active_project_id: DEFAULT_PROJECT_ID.to_owned(),
            layout_manager,
            loaded_project_id: None,
            layout_dirty_signal: false,
            save_in_flight: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            // WP-KERNEL-012 MT-088: the off-thread layout-load delivery cell + in-flight guard + the
            // debounced backend-reachability state. The load now runs off the UI thread so a backend-down
            // `GET` never freezes the frame loop (the 2026-06-26 freeze fix). `backend_down` starts false
            // (assume reachable until the first `/health` result settles it — no spurious "recovered").
            layout_load_cell: Arc::new(Mutex::new(None)),
            layout_load_in_flight: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            backend_down: false,
            // MT-088: the ctor already fired ONE `/health` poll above; arm the next re-probe so a
            // down/up transition is continuously observed (the seed for the typed transition events).
            health_next_poll_at: Some(std::time::Instant::now() + HEALTH_REPROBE_INTERVAL),
            // MT-088: captured lazily on the first frame (top of `ui`); used by off-thread backend
            // workers to wake the UI exactly once on completion (event-driven, not a per-frame poll).
            frame_ctx: None,
            monitor_extent: DEFAULT_MONITOR_EXTENT,
            last_seen_layout: None,
            project_tabs: default_project_tabs(),
            workspaces_handle,
            // The default seed pane (`pane-a`) is still the MAIN module target, so the switcher starts on MAIN.
            module_switcher: ModuleSwitcher::new(ModuleId::Main),
            left_rail: LeftRail::new(),
            left_rail_open: true,
            bottom_drawer_open: false,
            runtime_handle: Some(rt_handle.clone()),
            event_bus_rx,
            event_bus_tx,
            view_mode: ViewMode::Nsfw,
            command_palette_open: false,
            command_palette_open_count: 0,
            quick_switcher_open: false,
            quick_switcher_open_count: 0,
            quick_switcher_transport,
            quick_switcher_search: crate::quick_switcher::SearchManager::default(),
            quick_switcher_results_cell: Arc::new(Mutex::new(None)),
            quick_switcher_recents: Vec::new(),
            quick_switcher_recents_pending: false,
            quick_switcher_recents_cell: Arc::new(Mutex::new(None)),
            quick_switcher_record_recent_cell: Arc::new(Mutex::new(None)),
            quick_switcher_recents_error: None,
            quick_switcher_nav_status: None,
            nav_pending_label: None,
            settings_open: false,
            settings_open_count: 0,
            workspace_settings: crate::workspace_settings::default_workspace_settings_state(),
            settings_transport,
            settings_loaded_project_id: None,
            settings_load_pending: false,
            settings_load_cell: Arc::new(Mutex::new(None)),
            settings_save_cell: Arc::new(Mutex::new(None)),
            settings_save_due_at: None,
            settings_io_in_flight: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            settings_persist_error: None,
            worksurface_inspector_last_dump: None,
            pending_theme_change: None,
            about_open: false,
            reset_layout_pending: false,
            loom_block_client: Some(crate::backend_client::LoomBlockClient::production(
                rt_handle.clone(),
            )),
            rename_cell: Arc::new(Mutex::new(None)),
            pending_rename: None,
            rename_error: None,
            pending_memory_proposal: None,
            memory_proposal_status: None,
            terminal_launch_status: None,
            model_session_launch_client: Some(
                crate::backend_client::ModelSessionLaunchClient::production(rt_handle.clone()),
            ),
            model_session_launch_cell: Arc::new(Mutex::new(None)),
            model_session_launch_dialog: None,
            model_session_launch_status: None,
            model_session_launch_direct_status: None,
            model_session_launch_pending: false,
            source_control_client: Some(crate::backend_client::SourceControlClient::production(
                rt_handle.clone(),
            )),
            scm_receipt_cell: Arc::new(Mutex::new(None)),
            scm_text_cell: Arc::new(Mutex::new(None)),
            scm_display_text: None,
            scm_error: None,
            canvas_client: Some(crate::backend_client::CanvasClient::production(
                rt_handle.clone(),
            )),
            canvas_op_cell: Arc::new(Mutex::new(None)),
            canvas_error: None,
            loom_flag_cell: Arc::new(Mutex::new(None)),
            loom_flag_error: None,
            statusbar_hidden: std::collections::HashSet::new(),
            // MT-022: the rail emits a RailQuery intent into this lock-guarded slot (AC-022-9). It makes
            // NO backend call; a downstream search-results consumer reads the slot and executes the search.
            search_rail_query: Arc::new(Mutex::new(None)),
            // MT-023: the bottom drawer + its off-thread card-data client bridged onto the app runtime
            // (the MT-009 off-thread pattern). The drawer is collapsed by default (only the affordance
            // shows); the fetches fire when it opens.
            drawer: crate::stash_shelf::DrawerStashShelf::new(),
            drawer_data_client: Some(crate::backend_client::DrawerDataClient::production(
                rt_handle.clone(),
            )),
            drawer_data_cell: Arc::new(Mutex::new(None)),
            drawer_fetch_pending: false,
            drawer_prev_open: false,
            // MT-024: the card-action client bridged onto the app runtime (same pattern as the
            // SCM/canvas/loom-block clients). Genuinely consumed by apply_drawer_action.
            drawer_action_client: Some(crate::backend_client::DrawerActionClient::production(
                rt_handle.clone(),
            )),
            drawer_action_cell: Arc::new(Mutex::new(None)),
            drawer_action_error: None,
            drawer_intents: Arc::new(Mutex::new(DrawerIntents::default())),
            active_job_id: None,
            confirm_discard: None,
            drawer_action_success: None,
            drawer_action_in_flight: None,
            // MT-027: bind the out-of-process MCP transport on the app runtime. The token + shared
            // channel/snapshot are created here; `bind` is async (TcpListener) so it runs via the
            // runtime. A bind failure is logged + degrades to "no MCP server" rather than blocking the
            // window from opening (the shell must always start).
            mcp_server: None,
            mcp_action_channel: Arc::new(Mutex::new(crate::mcp::ActionChannel::new())),
            mcp_snapshot: Arc::new(Mutex::new(empty_snapshot())),
            mcp_token: crate::mcp::SessionToken::generate(),
            capturing_snapshot: false,
            ime_allowed_sent: false,
            // MT-033: the production Atelier/CKC side panel loads from the real `/atelier` backend off the
            // UI thread (the same runtime handle every other off-thread client uses). The feature stays
            // mounted but CLOSED by default so a fresh launch is editor/chat-focused.
            atelier_side_panel: AtelierSidePanel::production(rt_handle.clone()),
            atelier_panel_open: false,
            stage_pane: StagePane::new(),
            stage_panel_open: false,
        };
        app.spawn_mcp_server();
        // WP-KERNEL-012 MT-082 (D2 — internal_diagnostics): the REQUIRED LIVE call site (AC-002-4 / the
        // Spec-Realism anti-dead-code gate). A NORMAL launch records exactly one PaneMounted startup
        // marker through the OPEN `record()` API, so a real DiagEvent lands in the ring + in-process
        // buffer with ZERO test scaffolding — proving `record()` is genuinely CONSUMED by the shipped
        // binary, not dead scaffolding. This is NOT gated behind any test/feature flag; it runs in the
        // production shell. (The fuller per-frame heartbeat/frame-time/resource instrumentation is
        // MT-084/005/006/007/008.)
        Self::record_startup_marker();
        app
    }

    /// WP-KERNEL-012 MT-082: create the MT-081 shared-memory ring for this session and install its
    /// writer onto the process-global diagnostics recorder. Returns the [`DiagSession`](crate::
    /// diagnostics::DiagSession) (session id + ring backing-file path) MT-094 hands to Palmistry, or
    /// `None` if ring creation/install failed (graceful degradation — the app then records
    /// in-process-only and never crashes on a missing/unwritable ring; RISK-002-5 / AC-002-3).
    fn install_diagnostics_ring() -> Option<crate::diagnostics::DiagSession> {
        // WP-KERNEL-012 MT-094: in the production launch path, `main()` already created + installed the
        // ring (BEFORE `eframe::run_native`, so it could launch Palmistry against it) and recorded the
        // session in the process-global preinstalled slot. REUSE it here instead of creating a SECOND
        // ring (which would leak a backing file and fail to install onto the one-shot global recorder).
        // The kittest path (which builds `HandshakeApp::new` directly, with no `main()`) leaves the slot
        // empty and falls through to create its own ring exactly as before.
        if let Some(session) = crate::diagnostics::take_preinstalled_diag_session() {
            tracing::info!(
                session_id = %session.session_id,
                ring_path = %session.ring_path.display(),
                "internal_diagnostics ring reused from main()'s pre-launch install (MT-094)"
            );
            return Some(session);
        }
        Self::create_and_install_diag_ring()
    }

    /// WP-KERNEL-012 MT-082/MT-094: create the MT-081 shared-memory ring for a fresh session and install
    /// its writer onto the process-global diagnostics recorder. Returns the [`DiagSession`](crate::
    /// diagnostics::DiagSession) (session id + ring backing-file path), or `None` on a graceful failure.
    /// Exposed (pub) so `main()` (MT-094) can create + install the ring BEFORE `eframe::run_native` and
    /// launch Palmistry against it; the in-process [`install_diagnostics_ring`](Self::install_diagnostics_ring)
    /// reuses that result via the preinstalled-session slot rather than creating a second ring.
    pub fn create_and_install_diag_ring() -> Option<crate::diagnostics::DiagSession> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let ring_path = handshake_diag_ring::default_backing_path(&session_id);
        match handshake_diag_ring::DiagRingWriter::create(
            &ring_path,
            handshake_diag_ring::DEFAULT_CAPACITY,
        ) {
            Ok(writer) => {
                if crate::diagnostics::install(writer) {
                    tracing::info!(
                        session_id = %session_id,
                        ring_path = %ring_path.display(),
                        "internal_diagnostics ring installed (Tier 2 -> Palmistry visible)"
                    );
                    Some(crate::diagnostics::DiagSession {
                        session_id,
                        ring_path,
                    })
                } else {
                    // The global recorder was already initialized (e.g. an early `record()` before this
                    // install). The ring writer cannot be retrofitted; degrade to in-process-only.
                    tracing::warn!(
                        "internal_diagnostics ring writer could not be installed (recorder already \
                         initialized); diagnostics are in-process-only this session"
                    );
                    None
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    ring_path = %ring_path.display(),
                    "internal_diagnostics ring creation failed; diagnostics are in-process-only this \
                     session (graceful degradation)"
                );
                None
            }
        }
    }

    /// WP-KERNEL-012 MT-082: record the single live startup marker (a [`DiagEventCode::PaneMounted`]
    /// `Start` event) through the OPEN `record()` API. Called once at the end of [`HandshakeApp::new`];
    /// this is the live consumer that proves the diagnostics pipe is wired in the shipped binary.
    fn record_startup_marker() {
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        crate::diagnostics::record_with(
            handshake_diag_ring::DiagEventCode::PaneMounted,
            handshake_diag_ring::DiagPhase::Start,
            handshake_diag_ring::DiagSeverity::Info,
            /* thread_id    */ 0,
            /* sequence_id  */ 0,
            /* counter_a    */ 0,
            /* counter_b    */ 0,
            /* metric_micros*/ 0,
            now_nanos,
        );
    }

    /// WP-KERNEL-012 MT-082: the per-session diagnostic identity (session id + MT-081 ring backing-file
    /// path) for this shell, or `None` if no ring was created (headless/test shell or ring-creation
    /// failure). MT-094 reads this to launch `palmistry.exe` against the SAME ring.
    pub fn diag_session(&self) -> Option<&crate::diagnostics::DiagSession> {
        self.diag_session.as_ref()
    }

    /// WP-KERNEL-012 MT-094: install the [`PalmistryHandle`](crate::diagnostics::PalmistryHandle) the
    /// `main()` launch path produced (the watcher `main()` spawned BEFORE the event loop). Called once
    /// by `main()`'s eframe creation closure AFTER constructing the app, so the running shell owns the
    /// handle and a clean exit ([`eframe::App::on_exit`]) sends the explicit `Shutdown` to the watcher.
    pub fn set_palmistry_handle(&mut self, handle: crate::diagnostics::PalmistryHandle) {
        self.palmistry = Some(handle);
    }

    /// WP-KERNEL-012 MT-084: bump the UI-thread heartbeat ONE step and publish it into the MT-081 ring
    /// heartbeat slot. Called at the TOP of every [`eframe::App::update`] frame on the UI thread.
    ///
    /// Increments [`frame_counter`](Self::frame_counter) and reads the MONOTONIC process-start clock
    /// (`Instant::elapsed`, in nanos), then forwards both to the wait-free, allocation-free
    /// [`crate::diagnostics::heartbeat`] (a single seqlock store of two integers into the ring header).
    /// No record-buffer lock, no `format!`, no heap allocation on this path (AC-004-3). A silent no-op
    /// on the ring side when no writer is installed, so the headless/test shell runs normally (AC-004-5).
    ///
    /// The timestamp is monotonic nanos elapsed since construction, so it strictly increases and never
    /// goes backward on a wall-clock change (AC-004-2) — which the freeze-threshold math (MT-091) needs.
    #[inline]
    fn bump_heartbeat(&mut self) {
        // Saturating so a (practically impossible) u64 overflow can never panic the frame path.
        self.frame_counter = self.frame_counter.saturating_add(1);
        // Monotonic source: Instant elapsed in nanos (immune to wall-clock changes). Clamp the u128
        // nanos into u64 (saturating) — u64 nanos is ~584 years of runtime, so this never saturates in
        // practice and never allocates/panics.
        let elapsed_nanos =
            u64::try_from(self.heartbeat_clock.elapsed().as_nanos()).unwrap_or(u64::MAX);
        crate::diagnostics::heartbeat(self.frame_counter, elapsed_nanos);
    }

    /// WP-KERNEL-012 MT-084: the current UI-thread heartbeat frame counter (0 before the first frame;
    /// advances by 1 each [`eframe::App::update`]). Exposed so a test can correlate the in-app counter
    /// with the value a separate ring reader observes (AC-004-1).
    pub fn frame_counter(&self) -> u64 {
        self.frame_counter
    }

    /// WP-KERNEL-012 MT-085: the current frame-time stats (last/min/max/p50/p95 + counts) the
    /// Diagnostics Panel (MT-087, §5.8.4) reads. All typed integer MICROS — no content. All zero before
    /// the first frame. Exposed so the panel and a test can read the live tracker's stats by value.
    pub fn frame_stats(&self) -> crate::diagnostics::FrameStats {
        self.frame_timer.stats()
    }

    /// WP-KERNEL-012 MT-086: the STATIC GPU/driver identity captured once at startup from
    /// `cc.wgpu_render_state` (the Diagnostics Panel's hardware line, §5.8.4). `Some` when the shell was
    /// built on the wgpu renderer (production + the wgpu kittest harness); `None` in the headless
    /// `with_health` shell. Exposed so the panel and the AC-006-3 kittest read the captured identity.
    pub fn gpu_info(&self) -> Option<&crate::diagnostics::GpuInfo> {
        self.gpu_info.as_ref()
    }

    /// WP-KERNEL-012 MT-086: how many CPU/RSS resource samples have been taken + emitted so far. Advances
    /// at the bounded ~1s cadence (NOT per frame). Exposed so the AC-006-4 kittest can assert the sampler
    /// is bounded (sample count grows far slower than the frame count over many stepped frames).
    pub fn resource_sample_count(&self) -> u64 {
        self.resource_sampler.sample_count()
    }

    /// WP-KERNEL-012 MT-087 (D3 — §5.8.4 in-app Diagnostics Panel): build the read-only
    /// [`crate::diagnostics::DiagnosticsView`] the Settings -> Diagnostics section projects this frame.
    ///
    /// This is the PROJECTION boundary (§5.8.4 / RISK-007-2): it reads the live producers — the
    /// heartbeat counter + monotonic clock (MT-084), the frame-time stats (MT-085), the GPU identity +
    /// sample count (MT-086) — and the LAST `ResourceSample` straight from the process-global recorder
    /// ([`crate::diagnostics::snapshot_last_n`]), so the panel never caches its own copy and can never
    /// drift from the producers. Rebuilt every frame; cheap (a few field reads + one bounded ring scan
    /// for the last resource sample). The last-N EVENTS are NOT placed here — the panel reads them
    /// directly from the recorder each frame.
    pub fn diagnostics_view(&self) -> crate::diagnostics::DiagnosticsView {
        crate::diagnostics::DiagnosticsView {
            heartbeat_counter: self.frame_counter,
            // The same monotonic elapsed-since-start nanos the last heartbeat published (read live, not
            // cached): a human "uptime" line for the panel. Monotonic, never goes backward.
            heartbeat_elapsed_nanos: u64::try_from(self.heartbeat_clock.elapsed().as_nanos())
                .unwrap_or(u64::MAX),
            frame_stats: self.frame_timer.stats(),
            last_resource_sample: Self::last_resource_sample_from_ring(),
            resource_sample_count: self.resource_sampler.sample_count(),
            gpu_info: self.gpu_info.clone(),
            dropped_count: crate::diagnostics::dropped_count(),
            ring_writer_installed: crate::diagnostics::has_ring_writer(),
            // WP-KERNEL-012 MT-093 (§6.13.7 / §10.12.5 Tier-3, AC-013-6): read the freeze/crash survivor
            // records the external Palmistry watcher persisted to its durable per-user store so the panel's
            // Tier-3 section is POPULATED post-recovery (the honest empty-state until a freeze/crash). A
            // pure file read of the typed-allowlist records — no project content. Empty when the store is
            // absent/empty (the honest empty-state MT-087 renders).
            palmistry_records: crate::diagnostics::read_default_survivor_records(),
        }
    }

    /// WP-KERNEL-012 MT-087: read the MOST-RECENT `ResourceSample` (CPU%/RSS) from the process-global
    /// recorder ring (MT-082/086), or `None` if no resource sample has been emitted yet. This keeps the
    /// panel a pure projection of the producer: the resource line reads the same typed event the
    /// sampler emitted into the ring (the panel reads it back), never a separately-cached copy. Decodes
    /// the typed `DiagEvent` (cpu_milli -> counter_a, rss_kb -> counter_b) back into a
    /// [`crate::diagnostics::ResourceSample`].
    fn last_resource_sample_from_ring() -> Option<crate::diagnostics::ResourceSample> {
        crate::diagnostics::snapshot_last_n(crate::diagnostics::BUFFER_CAP)
            .iter()
            .rev()
            .find(|e| e.event_code == handshake_diag_ring::DiagEventCode::ResourceSample.as_u16())
            .map(|e| crate::diagnostics::ResourceSample {
                cpu_milli: e.counter_a,
                rss_kb: e.counter_b,
            })
    }

    /// WP-KERNEL-012 MT-085 TEST SEAM: inject `micros` of extra synthetic WORK INSIDE `self.ui(ctx)` on
    /// every subsequent frame so a kittest can drive a REAL slow frame from the live frame path
    /// (AC-005-2) and prove the SlowFrame flag fires from production code — NOT by feeding the
    /// `FrameTimer` directly (which is the unit-level AC-005-1 path). Set to `0` to stop injecting. The
    /// injected work lands inside the measured `self.ui(ctx)` window exactly like real heavy UI work.
    #[doc(hidden)]
    pub fn set_extra_frame_work_for_test(&mut self, micros: u64) {
        self.extra_frame_work_micros = micros;
    }

    /// Bind the MCP transport (MT-027) on the app's tokio runtime and store the handle. Logged + non-fatal
    /// on failure so the shell always opens. Only the production shell (with a multi-thread runtime) binds;
    /// the headless/test shell drives the server's `dispatch_request` directly instead.
    fn spawn_mcp_server(&mut self) {
        let token = self.mcp_token.clone();
        let snapshot = self.mcp_snapshot.clone();
        let channel = self.mcp_action_channel.clone();
        let capture = crate::mcp::SwarmMcpServer::os_window_capture();
        let result = self.rt.block_on(async move {
            crate::mcp::SwarmMcpServer::bind(token, snapshot, channel, capture).await
        });
        match result {
            Ok(server) => {
                tracing::info!(tcp = %server.tcp_addr(), pipe = ?server.pipe_name(), "MCP swarm server bound");
                self.mcp_server = Some(server);
            }
            Err(e) => {
                tracing::warn!(error = %e, "MCP swarm server bind failed; model-steering transport disabled this session");
            }
        }
    }

    /// Capture the live UI tree into the shared MCP snapshot slot. Runs `ui()` once on a fresh
    /// AccessKit-enabled context with `capturing_snapshot` set, so the async pollers / event drains /
    /// layout scheduler are skipped (no double side effects); the resulting `accesskit::TreeUpdate` is
    /// projected to a [`UiTreeSnapshot`] (the MT-026 path) and stored for the MCP `list_widgets` tool.
    fn refresh_mcp_snapshot(&mut self) {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        self.capturing_snapshot = true;
        let output = ctx.run(egui::RawInput::default(), |ctx| self.ui(ctx));
        self.capturing_snapshot = false;
        if let Some(update) = output.platform_output.accesskit_update {
            let snapshot = crate::accessibility::collect_ui_tree_snapshot(&update);
            match self.mcp_snapshot.lock() {
                Ok(mut slot) => *slot = snapshot,
                Err(poisoned) => *poisoned.into_inner() = snapshot,
            }
        }
    }

    /// Test/headless constructor: preset health, no runtime spawn, no backend needed. The layout
    /// manager is wired with a [`NullLayoutTransport`] (no network), so a headless shell keeps the
    /// seeded default layout until a test injects a stub transport via [`set_layout_manager`].
    ///
    /// [`set_layout_manager`]: HandshakeApp::set_layout_manager
    pub fn with_health(state: HealthDisplayState) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("build tokio runtime");
        let layout_manager = Arc::new(Mutex::new(LayoutPersistenceManager::new(
            Box::new(NullLayoutTransport),
            LAYOUT_SAVE_DEBOUNCE,
        )));
        // MT-014 FIX-B: the in-process shell event bus (same construction as the production ctor).
        let (event_bus_tx, event_bus_rx) = new_shell_event_bus();
        // MT-028: install the CONCRETE LoomSearchV2 pane factory over its placeholder here too, so a
        // headless/test shell opens the REAL panel via the registry-dispatched pane (AC-9). The
        // current-thread runtime handle bridges the (no-backend) search/save spawns; with no live
        // backend the request simply never delivers (the panel stays idle/neutral — no perpetual spinner).
        let (
            factories,
            loom_search_v2_shared,
            find_in_files_shared,
            runtime_chat_panel,
            editor_mounts,
        ) = build_factories_with_loom_search_v2(rt.handle().clone(), HsTheme::Dark.palette());
        Self {
            health_status: state,
            rt,
            health_handle: None,
            // Headless/test shell: NO diagnostics ring writer is created (graceful degradation — the
            // process-global recorder buffers in-process only; a test that wants ring read-back installs
            // its own writer directly). MT-082 AC-002-3.
            diag_session: None,
            // WP-KERNEL-012 MT-094: the headless/test shell never launches the external watcher (the
            // launch lives in `main()`, which this constructor bypasses).
            palmistry: None,
            // MT-084: the heartbeat still bumps every frame in the headless shell; with no ring writer
            // installed the publish is a silent no-op (AC-004-5), but the in-app frame_counter advances
            // exactly as in production, and the monotonic clock starts at construction.
            frame_counter: 0,
            heartbeat_clock: std::time::Instant::now(),
            // MT-085: same frame-time tracker as production (the headless shell runs the SAME `update`
            // frame-time measurement; with no ring writer installed a SlowFrame emit is an in-process
            // buffer record only — the live-frame kittest reads it back via `diagnostics::snapshot_last_n`).
            frame_timer: crate::diagnostics::FrameTimer::new(),
            extra_frame_work_micros: 0,
            // MT-086: the headless shell runs the SAME CPU/RSS sampler (current pid only) on the same
            // bounded cadence; with no ring writer installed the ResourceSample emit is an in-process
            // buffer record only. No `CreationContext` here, so no GPU identity is captured (gpu_info None).
            resource_sampler: crate::diagnostics::ResourceSampler::new(),
            gpu_info: None,
            current_theme: HsTheme::Dark,
            last_applied_theme: None,
            pane_registry: Arc::new(Mutex::new(seeded_registry())),
            factories,
            loom_search_v2_shared,
            find_in_files_shared,
            runtime_chat_panel,
            editor_mounts,
            rich_doc_base_url: backend_client::BACKEND_BASE_URL.to_owned(),
            rich_doc_load_cell: Arc::new(Mutex::new(VecDeque::new())),
            rich_doc_loading_id: None,
            rich_doc_load_generation: 0,
            rich_doc_loaded_id: None,
            rich_doc_loaded_version: None,
            rich_doc_load_error: None,
            last_editor_command: None,
            split_weights: SplitWeights::default(),
            split_drag: SplitDragState::default(),
            active_pane: None,
            tab_bar_states: default_tab_bar_states(),
            popout_manager: PopOutManager::new(),
            pop_out_request: None,
            active_project_id: DEFAULT_PROJECT_ID.to_owned(),
            layout_manager,
            loaded_project_id: None,
            layout_dirty_signal: false,
            save_in_flight: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            // WP-KERNEL-012 MT-088: same off-thread layout-load cell + guard + backend-reachability state
            // as the production ctor. The headless shell wires a `NullLayoutTransport` (load -> Ok(None)),
            // so a load resolves instantly to "keep default" with no network and never marks backend_down.
            layout_load_cell: Arc::new(Mutex::new(None)),
            layout_load_in_flight: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            backend_down: false,
            // Headless/test shell: no runtime to spawn a `/health` re-probe on, so none is armed (the
            // preset `health_status` is the fixed reachability for the session).
            health_next_poll_at: None,
            // MT-088: captured lazily on the first frame (top of `ui`), exactly as the production shell —
            // so an off-thread layout-load/save worker wakes the kittest UI on completion (event-driven).
            frame_ctx: None,
            monitor_extent: DEFAULT_MONITOR_EXTENT,
            last_seen_layout: None,
            project_tabs: default_project_tabs(),
            // Headless/test shell: no background fetch (no runtime to spawn on). A test seeds tabs
            // directly via `project_tabs_mut`.
            workspaces_handle: None,
            module_switcher: ModuleSwitcher::new(ModuleId::Main),
            left_rail: LeftRail::new(),
            left_rail_open: true,
            bottom_drawer_open: false,
            // Headless/test shell: the current-thread runtime cannot spawn background loads, so the
            // project tree is seeded directly via `left_rail_mut().project_tree.set_content(...)`.
            runtime_handle: None,
            event_bus_rx,
            event_bus_tx,
            view_mode: ViewMode::Nsfw,
            command_palette_open: false,
            command_palette_open_count: 0,
            quick_switcher_open: false,
            quick_switcher_open_count: 0,
            // Headless/test shell: no runtime to bridge a live transport onto. A test injects a stub
            // via `set_quick_switcher_transport`; without one, the switcher shows the empty/no-result
            // state and never performs I/O.
            quick_switcher_transport: None,
            quick_switcher_search: crate::quick_switcher::SearchManager::default(),
            quick_switcher_results_cell: Arc::new(Mutex::new(None)),
            quick_switcher_recents: Vec::new(),
            quick_switcher_recents_pending: false,
            quick_switcher_recents_cell: Arc::new(Mutex::new(None)),
            quick_switcher_record_recent_cell: Arc::new(Mutex::new(None)),
            quick_switcher_recents_error: None,
            quick_switcher_nav_status: None,
            nav_pending_label: None,
            settings_open: false,
            settings_open_count: 0,
            workspace_settings: crate::workspace_settings::default_workspace_settings_state(),
            // Headless/test shell: no runtime to bridge a live transport onto. A test injects a stub via
            // `set_settings_transport`; without one, the dialog shows the seeded defaults + never does I/O.
            settings_transport: None,
            settings_loaded_project_id: None,
            settings_load_pending: false,
            settings_load_cell: Arc::new(Mutex::new(None)),
            settings_save_cell: Arc::new(Mutex::new(None)),
            settings_save_due_at: None,
            settings_io_in_flight: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            settings_persist_error: None,
            worksurface_inspector_last_dump: None,
            pending_theme_change: None,
            about_open: false,
            reset_layout_pending: false,
            // Headless/test shell: no runtime to bridge the rename PATCH onto, so rename is a disclosed
            // no-op (a test injects a runtime via `set_runtime_handle` if it wants live rename).
            loom_block_client: None,
            rename_cell: Arc::new(Mutex::new(None)),
            pending_rename: None,
            rename_error: None,
            pending_memory_proposal: None,
            memory_proposal_status: None,
            terminal_launch_status: None,
            model_session_launch_client: None,
            model_session_launch_cell: Arc::new(Mutex::new(None)),
            model_session_launch_dialog: None,
            model_session_launch_status: None,
            model_session_launch_direct_status: None,
            model_session_launch_pending: false,
            // Headless/test shell: no runtime to bridge the SCM/canvas clients onto. A test injects a
            // runtime via `set_runtime_handle` if it wants live calls; without one these stay None.
            source_control_client: None,
            scm_receipt_cell: Arc::new(Mutex::new(None)),
            scm_text_cell: Arc::new(Mutex::new(None)),
            scm_display_text: None,
            scm_error: None,
            canvas_client: None,
            canvas_op_cell: Arc::new(Mutex::new(None)),
            canvas_error: None,
            loom_flag_cell: Arc::new(Mutex::new(None)),
            loom_flag_error: None,
            statusbar_hidden: std::collections::HashSet::new(),
            // MT-022: the rail emits a RailQuery intent into this lock-guarded slot (AC-022-9); it makes
            // no backend call, so the headless shell needs no transport — only the shared slot.
            search_rail_query: Arc::new(Mutex::new(None)),
            // MT-023: headless/test shell — no runtime to bridge the drawer-data client onto, so card
            // fetches are a disclosed no-op (the cards show their pre-fetch state). A test injects a
            // runtime via `set_runtime_handle` to drive live fetches.
            drawer: crate::stash_shelf::DrawerStashShelf::new(),
            drawer_data_client: None,
            drawer_data_cell: Arc::new(Mutex::new(None)),
            drawer_fetch_pending: false,
            drawer_prev_open: false,
            // MT-024: headless/test shell — no runtime, so the card-action client is None (the persisting
            // actions then surface a disclosed "no backend runtime" error instead of panicking). A test
            // injects a runtime via `set_runtime_handle` to drive live action dispatch.
            drawer_action_client: None,
            drawer_action_cell: Arc::new(Mutex::new(None)),
            drawer_action_error: None,
            drawer_intents: Arc::new(Mutex::new(DrawerIntents::default())),
            active_job_id: None,
            confirm_discard: None,
            drawer_action_success: None,
            drawer_action_in_flight: None,
            // MT-027: the headless/test shell does NOT bind the OS transport (no multi-thread runtime,
            // no OS window). The shared channel/snapshot/token still exist so a test can drive the MCP
            // dispatch + frame-drain steering loop in-process; the over-the-wire test binds its OWN
            // `SwarmMcpServer` on a `#[tokio::test]` runtime.
            mcp_server: None,
            mcp_action_channel: Arc::new(Mutex::new(crate::mcp::ActionChannel::new())),
            mcp_snapshot: Arc::new(Mutex::new(empty_snapshot())),
            mcp_token: crate::mcp::SessionToken::generate(),
            capturing_snapshot: false,
            ime_allowed_sent: false,
            // MT-033: headless/test shell — no runtime to bridge the atelier client onto, so the panel has
            // no client (it renders no rows + never touches the network; a test injects rows via the
            // `atelier_side_panel_mut` accessor + `with_rows`-style state if it wants seeded rows). The
            // panel stays mounted but CLOSED by default so the fresh default frame is editor/chat-focused.
            atelier_side_panel: AtelierSidePanel::with_client(None),
            atelier_panel_open: false,
            stage_pane: StagePane::new(),
            stage_panel_open: false,
        }
    }

    /// The shared MCP action channel (MT-027): the slot the out-of-process server enqueues into and the
    /// frame loop drains. Exposed so a test (or the over-the-wire server) can share the SAME channel the
    /// running shell drains, proving a connected client steers the live app.
    pub fn mcp_action_channel(&self) -> Arc<Mutex<crate::mcp::ActionChannel>> {
        self.mcp_action_channel.clone()
    }

    /// The shared MCP snapshot slot (MT-027): the live UI-tree the server's `list_widgets` reads.
    pub fn mcp_snapshot_slot(&self) -> Arc<Mutex<crate::accessibility::UiTreeSnapshot>> {
        self.mcp_snapshot.clone()
    }

    /// MT-103 foreground-safe navigation: refresh and return the same in-process MCP widget snapshot
    /// used by `list_widgets`/`click_widget`/`set_value`, without mutating app state beyond the
    /// existing model-vision snapshot slot.
    pub fn capture_mcp_snapshot_for_navigation(&mut self) -> crate::accessibility::UiTreeSnapshot {
        self.refresh_mcp_snapshot();
        match self.mcp_snapshot.lock() {
            Ok(slot) => slot.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// MT-102 Visual Debugger: compose a live worksurface/window-structure snapshot and write it to
    /// an external artifact directory. This is the runtime seam behind Settings -> Diagnostics; tests
    /// pass an explicit root, while the Settings button uses [`crate::visual_debugger::default_artifact_root`].
    pub fn capture_worksurface_snapshot_to(
        &mut self,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> std::io::Result<crate::visual_debugger::SnapshotWriteReceipt> {
        let artifact_root =
            crate::visual_debugger::validate_external_artifact_root(artifact_root.as_ref())?;
        let capture_id = crate::visual_debugger::WorksurfaceInspector::new_capture_id();
        self.refresh_mcp_snapshot();

        let widget_tree = match self.mcp_snapshot.lock() {
            Ok(slot) => slot.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        let pane_accesskit_ids = {
            let guard = self
                .pane_registry
                .lock()
                .expect("pane registry mutex poisoned");
            guard
                .iter()
                .map(|(pane_id, _)| {
                    (
                        pane_id.as_ref().to_owned(),
                        guard.accesskit_id(pane_id).map(|node_id| node_id.0),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };

        let snapshot = crate::visual_debugger::WorksurfaceInspector::capture(
            capture_id.clone(),
            self.capture_layout_snapshot(),
            pane_accesskit_ids,
            widget_tree,
            Self::capture_worksurface_screenshot_to(&artifact_root, &capture_id),
        );

        let receipt =
            crate::visual_debugger::WorksurfaceInspector::write_json(&snapshot, &artifact_root)?;
        crate::diagnostics::record_with(
            handshake_diag_ring::DiagEventCode::Other,
            handshake_diag_ring::DiagPhase::End,
            handshake_diag_ring::DiagSeverity::Info,
            0,
            snapshot.internal_diagnostics.timestamp_nanos,
            snapshot.internal_diagnostics.counter_a_value,
            snapshot.internal_diagnostics.counter_b_value,
            0,
            snapshot.internal_diagnostics.timestamp_nanos,
        );

        self.worksurface_inspector_last_dump = Some(format!(
            "Wrote worksurface snapshot: {} ({} bytes)",
            receipt
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("worksurface-snapshot.json"),
            receipt.bytes
        ));
        Ok(receipt)
    }

    fn capture_worksurface_screenshot_to(
        artifact_root: &std::path::Path,
        capture_id: &str,
    ) -> crate::visual_debugger::ScreenshotEvidence {
        let screenshot = match crate::mcp::screenshot::capture_handshake_window() {
            Ok(screenshot) => screenshot,
            Err(err) => {
                return crate::visual_debugger::ScreenshotEvidence::deferred_from_mcp_error(err)
            }
        };

        let bytes = match base64::engine::general_purpose::STANDARD
            .decode(screenshot.png_base64.as_bytes())
        {
            Ok(bytes) => bytes,
            Err(err) => {
                return crate::visual_debugger::ScreenshotEvidence::Deferred {
                    marker: "screenshot_capture_decode_failed".to_owned(),
                    reason: format!("existing MCP screenshot result could not decode: {err}"),
                };
            }
        };
        if let Err(err) = std::fs::create_dir_all(artifact_root) {
            return crate::visual_debugger::ScreenshotEvidence::Deferred {
                marker: "screenshot_capture_write_failed".to_owned(),
                reason: format!(
                    "could not create screenshot artifact root {}: {err}",
                    artifact_root.display()
                ),
            };
        }

        let path = artifact_root.join(format!(
            "worksurface-screenshot-{}.png",
            crate::visual_debugger::safe_capture_id(capture_id)
        ));
        if let Err(err) = std::fs::write(&path, bytes) {
            return crate::visual_debugger::ScreenshotEvidence::Deferred {
                marker: "screenshot_capture_write_failed".to_owned(),
                reason: format!("could not write {}: {err}", path.display()),
            };
        }

        crate::visual_debugger::ScreenshotEvidence::Captured {
            path,
            width: screenshot.width,
            height: screenshot.height,
        }
    }

    pub fn worksurface_inspector_last_dump(&self) -> Option<&str> {
        self.worksurface_inspector_last_dump.as_deref()
    }

    /// The per-session MCP token (MT-027) gating every request.
    pub fn mcp_token(&self) -> crate::mcp::SessionToken {
        self.mcp_token.clone()
    }

    /// The bound MCP transport handle (MT-027), if the server is running (production shell only).
    pub fn mcp_server(&self) -> Option<&crate::mcp::SwarmMcpServer> {
        self.mcp_server.as_ref()
    }

    /// Shared handle to the pane registry (for tests and future concurrent agent/operator wiring).
    pub fn pane_registry(&self) -> Arc<Mutex<PaneRegistry>> {
        self.pane_registry.clone()
    }

    /// Whether the shell can still open `pane_type` through the factory map. MT-098 proves stripped
    /// feature panes were removed only from the default seed, not from registration.
    pub fn pane_factory_registered(&self, pane_type: &PaneType) -> bool {
        self.factories.contains_key(pane_type)
    }

    /// Active base theme (for tests / future settings binding).
    pub fn current_theme(&self) -> HsTheme {
        self.current_theme
    }

    /// MT-033: mutable access to the mounted Atelier/CKC side panel (for tests that seed rows so the
    /// live-shell render shows real draggable item nodes without a backend).
    pub fn atelier_side_panel_mut(&mut self) -> &mut AtelierSidePanel {
        &mut self.atelier_side_panel
    }

    /// MT-033: the Stage pane's currently-staged content (read-only; for tests asserting the shell drain
    /// delivered routed content into the mounted pane).
    pub fn stage_content(&self) -> &StageContent {
        &self.stage_pane.content
    }

    /// MT-033: whether the Stage pane (bottom panel) is currently shown.
    pub fn stage_panel_open(&self) -> bool {
        self.stage_panel_open
    }

    /// A clonable producer handle onto the shell event bus (MT-014 FIX-B). A future surface that
    /// performs a delete (document / canvas / bookmark) clones this and publishes a [`ShellEvent`] so
    /// the project tree drops the row on the next frame. No production emitter exists yet (FIX-B); this
    /// is the wired entry point for one.
    pub fn event_bus_sender(&self) -> ShellEventSender {
        self.event_bus_tx.clone()
    }

    /// Drain the shell event bus (MT-014 FIX-B) and apply each event to the live project tree so a
    /// document / canvas / bookmark deleted from another surface disappears with no stale row. Called
    /// once per frame at the top of [`ui`](Self::ui), before the tree renders. Returns the number of
    /// events that actually removed a row (for tests / repaint scheduling).
    pub fn drain_shell_events(&mut self) -> usize {
        // MT-027: a snapshot-capture pass must not consume the shell event bus (the real frame owns it).
        if self.capturing_snapshot {
            return 0;
        }
        let events = self.event_bus_rx.drain();
        let mut removed = 0usize;
        for event in events {
            let did_remove = match event {
                ShellEvent::DocumentDeleted { document_id } => {
                    self.left_rail.project_tree.remove_document(&document_id)
                }
                ShellEvent::CanvasDeleted { canvas_id } => {
                    self.left_rail.project_tree.remove_canvas(&canvas_id)
                }
                ShellEvent::BookmarkRemoved { block_id } => {
                    self.left_rail.project_tree.remove_bookmark(&block_id)
                }
            };
            if did_remove {
                removed += 1;
            }
        }
        removed
    }

    /// Read-only view of the per-pane tab-bar state (for tests / MT-009 snapshot wiring).
    pub fn tab_bar_states(&self) -> &HashMap<PaneId, TabBarState> {
        &self.tab_bar_states
    }

    /// Mutable view of the per-pane tab-bar state (for tests that seed a multi-tab pane before
    /// driving a frame, and for future agent/operator tab mutation).
    pub fn tab_bar_states_mut(&mut self) -> &mut HashMap<PaneId, TabBarState> {
        &mut self.tab_bar_states
    }

    /// Request that `pane_id` be popped out into its own OS window on the next frame (MT-008). The
    /// request is applied at the top of `ui()`; the pane's record stays in the registry. Public so a
    /// future MT-019 pane-header action, a test, or an out-of-process driver can trigger a pop-out
    /// without inventing UI that belongs to a later MT.
    pub fn request_pop_out(&mut self, pane_id: PaneId) {
        self.pop_out_request = Some(pane_id);
    }

    /// Whether a pane currently renders into a detached pop-out window (test / snapshot visibility).
    pub fn is_popped_out(&self, pane_id: &PaneId) -> bool {
        self.popout_manager.is_popped_out(pane_id)
    }

    /// Read-only view of the pop-out manager (tests / MT-009 snapshot wiring).
    pub fn popout_manager(&self) -> &PopOutManager {
        &self.popout_manager
    }

    /// The project whose layout this shell currently shows (MT-009).
    pub fn active_project_id(&self) -> &str {
        &self.active_project_id
    }

    /// Read-only view of the top project-tab strip (MT-011): tests assert the project list, active id,
    /// and fetch state.
    pub fn project_tabs(&self) -> &ProjectTabBar {
        &self.project_tabs
    }

    /// Mutable view of the top project-tab strip (MT-011): tests seed the workspace list directly (no
    /// live backend), and a future workspace-sidebar MT pushes a refreshed list here.
    pub fn project_tabs_mut(&mut self) -> &mut ProjectTabBar {
        &mut self.project_tabs
    }

    /// The active work-surface MODULE (MT-012) — the switcher's current highlight.
    pub fn active_module(&self) -> ModuleId {
        self.module_switcher.active()
    }

    /// Read-only view of the module switcher (tests / future settings binding).
    pub fn module_switcher(&self) -> &ModuleSwitcher {
        &self.module_switcher
    }

    /// Whether the left activity rail is expanded (MT-014). Persisted as `drawers.project`.
    pub fn left_rail_open(&self) -> bool {
        self.left_rail_open
    }

    /// Set the left-rail open flag directly (tests / a future settings surface). The change is picked
    /// up by the MT-009 layout change-detector (drawers are part of the captured snapshot), so it
    /// persists through the debounced save.
    pub fn set_left_rail_open(&mut self, open: bool) {
        self.left_rail_open = open;
    }

    /// MT-033: open/close the right-edge Atelier/CKC side panel. Like the left rail, an OPEN panel is a
    /// `SidePanel::right` that narrows + shifts the central 2x2 pane grid; geometry-sensitive tests
    /// (live tab click/drag) close it for stable pane rects, exactly as they close the left rail.
    pub fn set_atelier_panel_open(&mut self, open: bool) {
        self.atelier_panel_open = open;
    }

    /// Whether the bottom stash drawer is open (MT-014). Persisted as `drawers.bottom`.
    pub fn bottom_drawer_open(&self) -> bool {
        self.bottom_drawer_open
    }

    /// Set the bottom-drawer open flag directly (MT-023): the out-of-process/agent + test driver for the
    /// drawer (HBR-MAN: drawer open/close state is settable/observable outside the UI). Setting it true
    /// arms the one-shot card fetches on the next frame (via the open transition the drawer driver
    /// detects), exactly as clicking the affordance does. The change is picked up by the MT-009 layout
    /// change-detector (`drawers.bottom`), so it persists through the debounced save.
    pub fn set_bottom_drawer_open(&mut self, open: bool) {
        self.bottom_drawer_open = open;
    }

    /// A no-context debug projection of the MT-023 drawer state (HBR-MAN): the open flag, the resizable
    /// height, and each card's `(title, badge_count, loading, error)` — readable from AppState without
    /// scraping the UI, so a model can inspect the drawer (and the live card counts) out-of-process.
    pub fn drawer_debug_state(&self) -> serde_json::Value {
        let cards: Vec<serde_json::Value> = self
            .drawer
            .cards
            .iter()
            .map(|c| {
                serde_json::json!({
                    "kind": c.kind.snake(),
                    "title": c.kind.title(),
                    "badge_count": c.badge_count,
                    "subtitle": c.subtitle,
                    "loading": c.loading,
                    "error": c.error,
                })
            })
            .collect();
        serde_json::json!({
            "open": self.bottom_drawer_open,
            "height": self.drawer.height,
            "cards": cards,
        })
    }

    // ── MT-015 top menu bar: state the VIEW/GO/HELP menus read + mutate ─────────────────────────────

    /// The active content-presentation mode (MT-015 VIEW menu). Read by tests + the View Mode checkmark.
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Whether the command-palette overlay is requested open (MT-015 GO menu; overlay UI is MT-016).
    pub fn command_palette_open(&self) -> bool {
        self.command_palette_open
    }

    /// The monotonic open generation of the command palette (MT-016). The palette resets its transient
    /// state when this changes; exposed for tests that assert a re-open bumps the counter.
    pub fn command_palette_open_count(&self) -> u64 {
        self.command_palette_open_count
    }

    /// Open the command palette (MT-016), bumping the open generation so the overlay resets its query +
    /// selection on this open (red-team R1/MC1). Idempotent while already open: a second open of an
    /// already-open palette does NOT bump the generation (so it does not wipe the user's in-progress
    /// query). The GO menu, the Ctrl+Shift+P chord, and tests all route through here.
    pub fn open_command_palette(&mut self) {
        if !self.command_palette_open {
            self.command_palette_open = true;
            self.command_palette_open_count = self.command_palette_open_count.wrapping_add(1);
        }
    }

    /// Close the command palette (MT-016). Used by the overlay's Escape / Close / backdrop dismiss and
    /// after a command runs. Safe to call when already closed.
    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
    }

    /// Toggle the command palette open/closed (MT-016 Ctrl+Shift+P chord). Opening bumps the open
    /// generation (state reset); closing does not.
    pub fn toggle_command_palette(&mut self) {
        if self.command_palette_open {
            self.close_command_palette();
        } else {
            self.open_command_palette();
        }
    }

    /// Dispatch a command id picked in the palette (MT-016) into the existing shell state-mutation
    /// paths, the native mirror of the React `onAction` handler. Returns `true` if app state changed
    /// (so the caller can request a repaint + let the layout change-detector schedule a save). Editor
    /// (`editor.*`) commands are guarded on an active document (red-team R5/MC5); since the native
    /// editor surface is a future MT, they are currently always skipped with a logged warning rather
    /// than panicking. An unknown id is a safe no-op with a logged warning.
    fn dispatch_palette_action(&mut self, ctx: &egui::Context, command_id: &str) -> bool {
        match command_id {
            "usermanual.open" => self.navigate_to_tab("user-manual"),
            "usermanual.search" => {
                // Open the UserManual tab; a dedicated search-focus flag lands with the UserManual
                // search surface MT. Opening the tab is the runnable part now.
                self.navigate_to_tab("user-manual")
            }
            "settings.open" => {
                self.open_settings();
                true
            }
            crate::command_registry::CMD_TERMINAL_OPEN_WORKSPACE => self.open_workspace_terminal(),
            crate::command_registry::CMD_MODEL_SESSION_LAUNCH_WORKSPACE => {
                self.open_model_session_launch_dialog()
            }
            "theme.toggle" => {
                self.current_theme = self.current_theme.toggled();
                self.apply_theme_if_changed(ctx);
                true
            }
            "viewmode.toggle" => {
                self.view_mode = self.view_mode.toggled();
                true
            }
            "layout.reset" => {
                // Mirror the VIEW > Reset Layout arm-then-confirm safety (red-team MC7): arming, not an
                // immediate wipe.
                self.reset_layout_pending = true;
                true
            }
            "swarmboard.open" => self.navigate_to_tab("swarm"),
            "inferencelab.open" => self.navigate_to_tab("inference-lab"),
            "flightrecorder.open" => self.navigate_to_tab("flight-recorder"),
            "pane.next" => self.focus_pane(true),
            "pane.prev" => self.focus_pane(false),
            "drawer.project.toggle" => {
                self.left_rail_open = !self.left_rail_open;
                true
            }
            "drawer.bottom.toggle" => {
                self.bottom_drawer_open = !self.bottom_drawer_open;
                true
            }
            crate::fems::memory_proposal::FEMS_PROPOSE_COMMAND_ID => {
                // WP-KERNEL-012 MT-064 (E9 — FEMS memory-write proposal): register the runtime handler on
                // the shared MT-031 InteractionBus (idempotent), read the LIVE SharedSelection, and open
                // the review-gated "Propose to Memory" dialog over it. This is a REAL dispatch arm that
                // produces a visible result (the dialog mounts on the live selection), not a silent
                // `other =>` no-op — exactly the bar the MT-033 route-to-stage arm set. The live
                // confirm→submit (POST proposal + FR-EVT-MEM-001 emit) lands at E11/MT-069; the proposal
                // WRITE endpoint is absent in this build, so confirming surfaces the typed `MissingEndpoint`
                // blocker (never a direct memory write). With no selection the command discloses that
                // instead of opening an empty dialog.
                let bus = crate::interop::InteractionBus::get_or_init(ctx);
                let selection = crate::interop::InteractionBus::with_try_lock(&bus, |bus| {
                    // Register the runtime command on the bus so it is addressable out-of-process
                    // (WRAP-not-fork; idempotent — last registration wins). The emitter/client the live
                    // E11 submit needs are wired at the E11 call site; today the dialog-open is the proven,
                    // visible result.
                    crate::fems::memory_proposal::register_propose_to_memory_command(bus);
                    bus.shared_selection().clone()
                });
                let Some(selection) = selection else {
                    return false; // bus contended this frame; a later dispatch re-opens (no-op, no panic).
                };
                let workspace_id = self
                    .left_rail
                    .project_tree
                    .workspace_id()
                    .map(|s| s.to_owned())
                    .or_else(|| {
                        let p = self.active_project_id.clone();
                        (!p.is_empty()).then_some(p)
                    })
                    .unwrap_or_default();
                let actor_id = selection
                    .pane_id()
                    .map(|p| crate::event_emitter::native_editor_actor_id(p.as_ref()))
                    .unwrap_or_else(|| "native_editor_human".to_owned());
                match crate::fems::memory_proposal::ProposeToMemoryDialog::open(
                    &selection,
                    &workspace_id,
                    &actor_id,
                ) {
                    Ok(dialog) => {
                        self.pending_memory_proposal = Some(dialog);
                        self.memory_proposal_status = None;
                        true
                    }
                    Err(_no_selection) => {
                        // No live selection: disclose it on the next dialog-status surface rather than
                        // opening an empty dialog or fabricating a proposal.
                        self.memory_proposal_status = Some(
                            "Select text or a block first, then run Propose to Memory.".to_owned(),
                        );
                        true
                    }
                }
            }
            "interop.route-to-stage" => {
                // WP-KERNEL-012 MT-033 (E5 — route-to-Stage): dispatch the Route-to-Stage command on the
                // shared MT-031 InteractionBus. The bus command opens/focuses the local Stage pane with
                // whatever content the focused pane staged (a selection / document / CKC item) via
                // `request_route_to_stage`. The bus is the single melt-together command surface (the
                // context-menu "Route to Stage" path stages content THEN dispatches the same command); the
                // palette entry dispatches the command so it is discoverable + runnable out-of-process.
                // Registering the command is idempotent, so a first palette use wires it up.
                let bus = crate::interop::InteractionBus::get_or_init(ctx);
                crate::interop::InteractionBus::with_try_lock(&bus, |bus| {
                    bus.register_route_to_stage_command();
                    bus.dispatch_command(ctx, crate::interop::CMD_ROUTE_TO_STAGE)
                })
                .unwrap_or(false)
            }
            // WP-KERNEL-012 MT-069 (E11 menu wire-up): the editor FILE/EDIT menu + palette commands MT-079
            // host-mounted. Route through the ONE shared dispatcher the menu bar also calls, so the palette
            // path and the menu path are the SAME single substrate (RISK-001: no forked dispatch). The
            // `workbench.*` command-palette/quick-open ids and every `editor.{file,edit,find}.*` id resolve
            // there. (The rich-text `editor.format.*` / `editor.block.*` etc. catalog commands stay disabled
            // and are not dispatched, so they never reach here as an enabled Run.)
            id if crate::command_registry::all_commands().iter().any(|c| {
                c.id == id && c.kind == crate::command_registry::CommandKind::EditorMenu
            }) =>
            {
                self.dispatch_editor_command(ctx, id)
            }
            id if id.starts_with("editor.") => {
                // A rich-text editor format/block command dispatched through the palette is guarded
                // (red-team R5/MC5) so it never panics with no active document. These rows are disabled
                // (no pinned rich-text document yet), so a Run should not reach here; log + skip if it does
                // rather than fake an edit.
                tracing::warn!("palette: editor command {id} skipped (no active editor document)");
                false
            }
            other => {
                tracing::warn!("palette: unknown command id {other}");
                false
            }
        }
    }

    /// WP-KERNEL-012 MT-069 (E11 menu wire-up): the ONE dispatcher both the top menu bar and the command
    /// palette route the editor FILE/EDIT commands through, so menu-driven and palette-driven editor
    /// actions share ONE code path (RISK-001: no forked dispatch; the menu handler contains no inline
    /// editor logic — it only routes by command id to here). Each id is wired to the EXISTING shell single
    /// substrate, NOT a new code path:
    ///
    /// - **Undo / Redo** dispatch the registered [`CMD_UNDO`]/[`CMD_REDO`] on the shared MT-031
    ///   [`InteractionBus`](crate::interop::InteractionBus), which resolves the FOCUSED pane and pops the
    ///   MT-035 unified-undo scope — the SAME entry the keyboard Ctrl+Z/Ctrl+Y path reaches, so menu undo
    ///   and keyboard undo share one stack (AC-004 / MC-002 / RISK-002).
    /// - **Cut / Copy / Paste / Select All** dispatch the registered [`CMD_CUT`]/[`CMD_COPY`]/[`CMD_PASTE`]/
    ///   [`CMD_SELECT_ALL`] on the bus (the MT-031 shared clipboard + selection substrate).
    /// - **Find** dispatches [`CMD_FIND`]; **Find/Replace in Files** opens the MT-029 `FindInFiles` surface
    ///   pane (the existing workspace-search route).
    /// - **Save / Save All / Save As / Export** route to the MT-020 editor save path via the mounted code
    ///   pane's command channel (`request_save_for_host`), recording `last_editor_command` so the dispatch
    ///   is observable; the editor command owns the handshake_core write — the shell never writes directly
    ///   and never opens a SQLite/shell-local path (AC-004 / MC-004 / RISK-004).
    /// - **Command Palette / Quick Switcher** open the ONE WP-011 palette / switcher (no second palette).
    /// - **GO-nav ids** ([`is_go_nav_pending`](crate::command_registry::is_go_nav_pending)) whose owning
    ///   command is not yet registered emit a typed LOGGED no-op — never `todo!()`/`unimplemented!()`/
    ///   `panic!()` (AC-003 / AC-006 / MC-003).
    ///
    /// Returns `true` when the dispatch produced an observable effect (so the caller requests a repaint),
    /// `false` for a logged no-op. The bus commands are registered idempotently at the dispatch site (the
    /// same WRAP-not-fork pattern the route-to-stage / propose-to-memory arms use), so a first dispatch
    /// wires them up without forking the editor-pane mount logic.
    fn dispatch_editor_command(&mut self, ctx: &egui::Context, command_id: &str) -> bool {
        use crate::command_registry as cr;
        use crate::interop::{
            InteractionBus, CMD_COPY, CMD_CUT, CMD_FIND, CMD_PASTE, CMD_REDO, CMD_SELECT_ALL,
            CMD_UNDO,
        };

        // GO-nav ids whose owner has not yet registered the live code-nav command: a typed logged no-op
        // (never a panic), keeping the item honestly inert until its owner MT lands (AC-003 / MC-003).
        if cr::is_go_nav_pending(command_id) {
            tracing::info!(
                "editor command {command_id} not yet available (GO-nav owner unregistered); no-op"
            );
            return false;
        }

        match command_id {
            // ── Undo / Redo -> MT-035 unified-undo scope via the shared bus (one stack, menu+keyboard) ──
            cr::CMD_EDITOR_EDIT_UNDO => {
                let bus = InteractionBus::get_or_init(ctx);
                let dispatched = InteractionBus::with_try_lock(&bus, |b| {
                    b.register_undo_commands(); // idempotent (last registration wins)
                    b.dispatch_command(ctx, CMD_UNDO)
                })
                .unwrap_or(false);
                ctx.request_repaint();
                dispatched
            }
            cr::CMD_EDITOR_EDIT_REDO => {
                let bus = InteractionBus::get_or_init(ctx);
                let dispatched = InteractionBus::with_try_lock(&bus, |b| {
                    b.register_undo_commands();
                    b.dispatch_command(ctx, CMD_REDO)
                })
                .unwrap_or(false);
                ctx.request_repaint();
                dispatched
            }
            // ── Cut / Copy / Paste / Select All / Find -> MT-031 shared clipboard + selection substrate ──
            cr::CMD_EDITOR_EDIT_CUT
            | cr::CMD_EDITOR_EDIT_COPY
            | cr::CMD_EDITOR_EDIT_PASTE
            | cr::CMD_EDITOR_EDIT_SELECT_ALL
            | cr::CMD_EDITOR_FIND_FIND => {
                let bus_cmd = match command_id {
                    cr::CMD_EDITOR_EDIT_CUT => CMD_CUT,
                    cr::CMD_EDITOR_EDIT_COPY => CMD_COPY,
                    cr::CMD_EDITOR_EDIT_PASTE => CMD_PASTE,
                    cr::CMD_EDITOR_EDIT_SELECT_ALL => CMD_SELECT_ALL,
                    _ => CMD_FIND,
                };
                let bus = InteractionBus::get_or_init(ctx);
                let dispatched = InteractionBus::with_try_lock(&bus, |b| {
                    // Register the standard clipboard/find command set (idempotent) so the dispatch reaches
                    // a real handler even before a pane registered them on mount.
                    crate::interop::adapters::register_standard_commands(
                        b,
                        crate::interop::EditorSurfaceKind::Code,
                    );
                    b.dispatch_command(ctx, bus_cmd)
                })
                .unwrap_or(false);
                ctx.request_repaint();
                dispatched
            }
            // ── Find/Replace in Files -> the MT-029 FindInFiles workspace-search surface pane ──
            cr::CMD_EDITOR_FIND_REPLACE => {
                // In-doc Replace shares the focused editor's find/replace family; dispatch the bus Find
                // intent (the focused editor opens its find/replace bar in its render path). One substrate.
                let bus = InteractionBus::get_or_init(ctx);
                let dispatched = InteractionBus::with_try_lock(&bus, |b| {
                    crate::interop::adapters::register_standard_commands(
                        b,
                        crate::interop::EditorSurfaceKind::Code,
                    );
                    b.dispatch_command(ctx, CMD_FIND)
                })
                .unwrap_or(false);
                ctx.request_repaint();
                dispatched
            }
            cr::CMD_EDITOR_FIND_IN_FILES | cr::CMD_EDITOR_REPLACE_IN_FILES => {
                self.open_content_on_active_pane(PaneType::FindInFiles, None)
            }
            // ── Toggle Comment / Format Document -> the focused code editor's REAL transform ──────────────
            cr::CMD_EDITOR_EDIT_TOGGLE_COMMENT | cr::CMD_EDITOR_EDIT_FORMAT_DOCUMENT => {
                // Route the intent to the SAME `dispatch_action` entry the code editor's keymap reaches:
                // `ToggleComment` -> MT-051 `apply_line_transform(line_ops::toggle_comment)` (a real,
                // observable buffer mutation), `FormatDocument` -> MT-050 `request_format_document` (arms a
                // real `textDocument/formatting` request; a no-op + toast when no formatter is available —
                // the honest MT-050 disabled path, never a fake effect). The shell does NOT re-implement the
                // transform; it dispatches the editor's own command id to the mounted code panel
                // (`dispatch_action` takes `&self` via interior mutability), so menu + keymap share ONE path
                // (MC-001 / RISK-001). This is NOT a bare repaint — the editor buffer/request actually moves.
                let action = if command_id == cr::CMD_EDITOR_EDIT_TOGGLE_COMMENT {
                    crate::code_editor::keymap::CodeEditorAction::ToggleComment
                } else {
                    crate::code_editor::keymap::CodeEditorAction::FormatDocument
                };
                self.editor_mounts.code_panel.dispatch_action(action);
                ctx.request_repaint();
                true
            }
            // ── Save / Save All / Save As / Export -> the MT-020 editor SaveManager save entry ───────────
            cr::CMD_EDITOR_FILE_SAVE
            | cr::CMD_EDITOR_FILE_SAVE_ALL
            | cr::CMD_EDITOR_FILE_SAVE_AS
            | cr::CMD_EDITOR_FILE_EXPORT_HTML
            | cr::CMD_EDITOR_FILE_EXPORT_MD
            | cr::CMD_EDITOR_FILE_EXPORT_TXT
            | cr::CMD_EDITOR_FILE_EXPORT_JSON => {
                // Invoke the REAL MT-020 SaveManager save entry on the mounted document pane — the SAME
                // `RichEditorState::request_save_for_host` -> `SaveManager::request_save` path the rich
                // editor's own Ctrl+S reaches (one save substrate, no fork — MC-004 / RISK-004). The
                // SaveManager owns the `PUT /knowledge/documents/{id}/save` handshake_core write; the shell
                // never writes directly and never opens a SQLite/shell-local path. `request_save` moves the
                // SaveManager into `SaveState::Saving` (its OWN state machine, not a host-set marker), which
                // is the AC-004 "dispatch reaches the MT-020 save entry" obligation, provable via
                // `mounted_rich_state().save_is_in_flight()`. The code pane's Save channel is ALSO pinged so
                // a focused code pane's save intent stays observable to a swarm agent (`last_editor_command`).
                self.invoke_editor_save();
                self.editor_mounts.code_panel.request_save_for_host();
                ctx.request_repaint();
                true
            }
            cr::CMD_EDITOR_FILE_NEW => {
                // New Document: open a fresh Notes/rich editor pane (the document model the menu item names).
                self.open_content_on_active_pane(PaneType::LoomWikiPage, None)
            }
            // ── Command Palette / Quick Switcher -> the ONE WP-011 palette / switcher (no second one) ──
            cr::CMD_WORKBENCH_SHOW_COMMANDS => {
                self.open_command_palette();
                true
            }
            cr::CMD_WORKBENCH_QUICK_OPEN => {
                self.open_quick_switcher();
                true
            }
            other => {
                tracing::warn!("editor command {other} unrecognized; no-op");
                false
            }
        }
    }

    /// WP-KERNEL-012 MT-069: invoke the MT-020 editor save path for a menu/palette FILE > Save / Save As /
    /// Export dispatch. Reaches the REAL [`crate::rich_editor::save::save_manager::SaveManager`] save entry
    /// on the mounted document pane (the `request_save` -> `SaveState::Saving` transition + the
    /// `PUT /knowledge/documents/{id}/save` backend call the rich editor's own Ctrl+S reaches), NOT a
    /// shell-local write. If the mounted rich pane has no save context yet (the mount's `wire_if_needed`
    /// installs embed + wikilink context but not save context), this installs one from the live bound
    /// session runtime first (the MT-020 carried-forward host-mount wiring this menu dispatch completes for
    /// the Save path), so the menu Save reaches a real SaveManager rather than a typed-carry marker. Returns
    /// `true` when the SaveManager save entry was reached, `false` when no runtime is bound (a headless
    /// current-thread shell with no runtime — the honest "save not yet wireable" path; the leaf is still
    /// enabled because an editor pane is present, and the code-pane channel still carries the intent).
    fn invoke_editor_save(&mut self) -> bool {
        let active_doc_id = self.active_rich_document_id();
        let active_doc_version = active_doc_id.as_deref().and_then(|id| {
            (self.rich_doc_loaded_id.as_deref() == Some(id))
                .then_some(self.rich_doc_loaded_version)
                .flatten()
        });
        let rich = Arc::clone(&self.editor_mounts.rich_state);
        let mut state = match rich.lock() {
            Ok(s) => s,
            Err(p) => p.into_inner(),
        };
        if let Some(active_doc_id) = active_doc_id.as_deref() {
            let save_mismatch = state
                .save
                .as_ref()
                .map(|save| save.document_id() != active_doc_id)
                .unwrap_or(false);
            let draft_mismatch = state
                .draft
                .as_ref()
                .map(|draft| draft.document_id() != active_doc_id)
                .unwrap_or(false);
            if save_mismatch || draft_mismatch {
                state.save = None;
                state.draft = None;
            }
            if active_doc_version.is_none() {
                tracing::debug!(
                    document_id = ?active_doc_id,
                    "editor Save skipped: active Notes document is not loaded yet"
                );
                return false;
            }
        }
        // Install the MT-020 save context on first save if the mount has not (the mount threads embed +
        // wikilink context but not save context). Use the live bound runtime + active workspace so the
        // SaveManager spawns the real backend call off the frame thread (HBR-QUIET).
        if !state.has_save_context() {
            if let Some(rt) = self.runtime_handle.clone() {
                // MT-099: when a knowledge document tab is active, Save MUST target that document id and
                // its loaded version. The old workspace-id fallback remains only for the fresh unsaved/demo
                // Notes pane where no document id exists yet.
                let doc_id = active_doc_id.clone().unwrap_or_else(|| {
                    if self.active_project_id.is_empty() {
                        DEFAULT_PROJECT_ID.to_owned()
                    } else {
                        self.active_project_id.clone()
                    }
                });
                let doc_version = active_doc_version.unwrap_or(0);
                let base_content =
                    crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
                state.save = Some(crate::rich_editor::save::save_manager::SaveManager::new(
                    Arc::new(crate::backend_client::RichDocSaveBackend::new(
                        self.rich_doc_base_url.clone(),
                    )),
                    Some(rt.clone()),
                    doc_id.clone(),
                    doc_version,
                ));
                let mut draft = crate::rich_editor::save::draft_manager::DraftManager::new(
                    Arc::new(crate::backend_client::RichDocDraftBackend::new(
                        self.rich_doc_base_url.clone(),
                    )),
                    Some(rt),
                    doc_id,
                    doc_version,
                    &base_content,
                );
                draft.check_on_mount();
                state.draft = Some(draft);
            } else {
                // No runtime bound (headless current-thread shell): the save path is not wireable this run.
                tracing::debug!("editor Save: no runtime bound; MT-020 save context not installed");
                return false;
            }
        }
        // Reach the REAL SaveManager save entry (request_save -> SaveState::Saving + backend PUT).
        state.request_save_for_host()
    }

    /// MT-099: the active Notes tab's authoritative document id, if the current active pane is a
    /// `LoomWikiPage` tab with a non-empty `content_id`.
    fn active_rich_document_id(&self) -> Option<String> {
        let pane_id = self.active_pane.as_ref()?;
        let tab = self.tab_bar_states.get(pane_id)?.active()?;
        if !matches!(tab.pane_type, PaneType::LoomWikiPage) {
            return None;
        }
        tab.content_id
            .as_ref()
            .filter(|id| !id.trim().is_empty())
            .cloned()
    }

    /// MT-099: force the next frame to re-GET a document. Used when opening/reopening a Notes tab so a
    /// stale mounted editor cannot masquerade as authoritative backend state.
    fn invalidate_rich_document_load(&mut self, document_id: &str) {
        self.rich_doc_load_generation = self.rich_doc_load_generation.wrapping_add(1);
        self.rich_doc_loaded_id = None;
        self.rich_doc_loaded_version = None;
        if self.rich_doc_loading_id.as_deref() == Some(document_id) {
            self.rich_doc_loading_id = None;
        }
        if let Ok(mut state) = self.editor_mounts.rich_state.lock() {
            state.save = None;
            state.draft = None;
        }
        if let Ok(mut slot) = self.rich_doc_load_cell.lock() {
            slot.retain(|(_, id, _)| id != document_id);
        }
        self.rich_doc_load_error = None;
    }

    fn clear_rich_document_context_if_mismatch(&self, expected_document_id: &str) {
        let mut state = self
            .editor_mounts
            .rich_state
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let save_mismatch = state
            .save
            .as_ref()
            .map(|save| save.document_id() != expected_document_id)
            .unwrap_or(false);
        let draft_mismatch = state
            .draft
            .as_ref()
            .map(|draft| draft.document_id() != expected_document_id)
            .unwrap_or(false);
        if save_mismatch || draft_mismatch {
            state.save = None;
            state.draft = None;
        }
    }

    /// MT-099: mirror each pane's active tab into the pane registry record immediately before the central
    /// pane host renders. The tab bar remains the operator-visible navigation source; the registry remains
    /// the render source that `SplitLayoutWidget` passes into each pane factory.
    fn sync_active_tab_records(&mut self) {
        let active_tabs: Vec<(PaneId, PaneType, Option<String>, DirtyState)> = self
            .tab_bar_states
            .iter()
            .filter_map(|(pane_id, bar)| {
                bar.active().map(|tab| {
                    (
                        pane_id.clone(),
                        tab.pane_type.clone(),
                        tab.content_id.clone(),
                        if tab.dirty {
                            DirtyState::Dirty
                        } else {
                            DirtyState::Clean
                        },
                    )
                })
            })
            .collect();
        if active_tabs.is_empty() {
            return;
        }
        let Ok(mut registry) = self.pane_registry.lock() else {
            return;
        };
        for (pane_id, pane_type, content_id, dirty) in active_tabs {
            if let Some(record) = registry.get_mut(&pane_id) {
                if record.pane_type != pane_type
                    || record.content_id != content_id
                    || record.dirty != dirty
                {
                    record.pane_type = pane_type;
                    record.content_id = content_id;
                    record.dirty = dirty;
                    record.last_update = std::time::Instant::now();
                }
            }
        }
    }

    /// MT-099: drive the active Notes document GET lifecycle. Network work runs on the app runtime and
    /// delivers into `rich_doc_load_cell`; this frame path only drains a completed result and starts at
    /// most one missing load.
    fn drive_rich_document_load(&mut self, ctx: &egui::Context) {
        if self.capturing_snapshot {
            return;
        }

        let delivered: Vec<RichDocumentLoadResult> = self
            .rich_doc_load_cell
            .lock()
            .ok()
            .map(|mut slot| slot.drain(..).collect())
            .unwrap_or_default();
        if !delivered.is_empty() {
            let mut did_update = false;
            for (generation, document_id, result) in delivered {
                let is_current_delivery = generation == self.rich_doc_load_generation
                    && self.rich_doc_loading_id.as_deref() == Some(document_id.as_str());
                if is_current_delivery {
                    self.rich_doc_loading_id = None;
                }
                match result {
                    Ok(doc) => {
                        if is_current_delivery
                            && self.active_rich_document_id().as_deref()
                                == Some(document_id.as_str())
                        {
                            match self.apply_loaded_rich_document(doc) {
                                Ok(()) => {
                                    self.rich_doc_load_error = None;
                                    did_update = true;
                                }
                                Err(message) => {
                                    self.rich_doc_load_error = Some(message);
                                    did_update = true;
                                }
                            }
                        }
                    }
                    Err(message) => {
                        if is_current_delivery
                            && self.active_rich_document_id().as_deref()
                                == Some(document_id.as_str())
                        {
                            self.rich_doc_load_error = Some(message);
                            did_update = true;
                        }
                    }
                }
            }
            if did_update {
                ctx.request_repaint();
            }
        }

        let Some(document_id) = self.active_rich_document_id() else {
            return;
        };
        self.clear_rich_document_context_if_mismatch(&document_id);
        if self.rich_doc_loaded_id.as_deref() == Some(document_id.as_str())
            || self.rich_doc_loading_id.as_deref() == Some(document_id.as_str())
        {
            return;
        }
        let Some(runtime) = self.runtime_handle.clone() else {
            self.rich_doc_load_error =
                Some("Notes document load blocked: no runtime handle is bound.".to_owned());
            return;
        };

        self.rich_doc_load_generation = self.rich_doc_load_generation.wrapping_add(1);
        let load_generation = self.rich_doc_load_generation;
        self.rich_doc_loading_id = Some(document_id.clone());
        self.rich_doc_load_error = None;
        let base_url = self.rich_doc_base_url.clone();
        let cell = Arc::clone(&self.rich_doc_load_cell);
        let repaint = ctx.clone();
        let client_runtime = runtime.clone();
        runtime.spawn(async move {
            let client = backend_client::RichDocClient::new(base_url, client_runtime);
            let loaded = client
                .load_document(&document_id)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((load_generation, document_id, loaded));
            }
            repaint.request_repaint();
        });
    }

    /// MT-099: install a freshly loaded backend document into the mounted rich editor and bind its
    /// SaveManager/DraftManager to the same knowledge-documents route family used for the GET.
    fn apply_loaded_rich_document(
        &mut self,
        doc: backend_client::RichDocBody,
    ) -> Result<(), String> {
        let runtime = self
            .runtime_handle
            .clone()
            .ok_or_else(|| "Notes document load blocked: no runtime handle is bound.".to_owned())?;
        let parsed =
            crate::rich_editor::document_model::doc_json::from_json_value(&doc.content_json)
                .map_err(|e| e.to_string())?;
        let base_content =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&parsed);
        let document_id = doc.document_id.clone();
        let doc_version = doc.doc_version;
        let mut state = self
            .editor_mounts
            .rich_state
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        state.doc = parsed;
        state.selection = crate::rich_editor::document_model::selection::Selection::caret(
            crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 0),
        );
        state.undo = crate::rich_editor::document_model::history::UndoManager::new();
        state.pending_events.clear();
        state.find_replace = None;
        state.save = Some(crate::rich_editor::save::save_manager::SaveManager::new(
            Arc::new(crate::backend_client::RichDocSaveBackend::new(
                self.rich_doc_base_url.clone(),
            )),
            Some(runtime.clone()),
            document_id.clone(),
            doc_version,
        ));
        let mut draft = crate::rich_editor::save::draft_manager::DraftManager::new(
            Arc::new(crate::backend_client::RichDocDraftBackend::new(
                self.rich_doc_base_url.clone(),
            )),
            Some(runtime.clone()),
            document_id.clone(),
            doc_version,
            &base_content,
        );
        draft.check_on_mount();
        state.draft = Some(draft);
        state.set_embed_context(self.active_project_id.clone(), runtime.clone());
        state.set_wikilink_context(self.active_project_id.clone(), document_id.clone(), runtime);
        self.rich_doc_loaded_id = Some(document_id);
        self.rich_doc_loaded_version = Some(doc_version);
        Ok(())
    }

    /// WP-KERNEL-012 MT-069 test seam: dispatch a command id through the REAL `dispatch_palette_action`
    /// path the live palette Run outcome uses, so the menu-wireup proofs drive the production dispatch
    /// (menu + palette share ONE dispatcher) without re-implementing it. Returns the dispatch's observable
    /// effect bool. Not a tautology — it exercises the same arm a clicked/Enter'd palette row reaches.
    pub fn dispatch_palette_action_for_test(&mut self, command_id: &str) -> bool {
        // The egui Context is needed by the bus arms; a freshly-built headless context is sufficient for
        // the non-rendering dispatch arms (navigation/quick-open + the GO-nav typed no-op + the bus arms,
        // which init their own bus in this context), matching how the bus tests build a bare context. The
        // production frame passes the live ctx; the test seam mirrors that. The no-panic proof only needs
        // each arm to RUN without panicking, which a fresh context satisfies.
        let ctx = egui::Context::default();
        self.dispatch_palette_action(&ctx, command_id)
    }

    /// Whether the quick-switcher overlay is requested open (MT-015 GO menu / Ctrl+P; overlay UI is
    /// MT-017).
    pub fn quick_switcher_open(&self) -> bool {
        self.quick_switcher_open
    }

    /// The monotonic open generation of the quick switcher (MT-017). The switcher resets its transient
    /// state when this changes; exposed for tests that assert a re-open bumps the counter.
    pub fn quick_switcher_open_count(&self) -> u64 {
        self.quick_switcher_open_count
    }

    /// Open the quick switcher (MT-017), bumping the open generation so the overlay resets its query +
    /// selection on this open. Idempotent while already open (a second open does NOT bump the
    /// generation, so it does not wipe an in-progress query). The GO menu, the Ctrl+P chord, and tests
    /// all route through here.
    pub fn open_quick_switcher(&mut self) {
        if !self.quick_switcher_open {
            self.quick_switcher_open = true;
            self.quick_switcher_open_count = self.quick_switcher_open_count.wrapping_add(1);
            // Fire the durable recents load once on this open (red-team: load on open, not per-frame).
            self.quick_switcher_recents_pending = true;
            // Fresh open: clear the previous open's search state so a re-open starts clean.
            self.quick_switcher_search = crate::quick_switcher::SearchManager::default();
            self.quick_switcher_recents_error = None;
        }
    }

    /// Inject a quick-switcher transport (MT-017) for tests/headless: a stub
    /// [`crate::quick_switcher::LoomGraphSearchTransport`] drives the switcher with no live backend.
    pub fn set_quick_switcher_transport(
        &mut self,
        transport: Arc<dyn crate::quick_switcher::LoomGraphSearchTransport>,
    ) {
        self.quick_switcher_transport = Some(transport);
    }

    /// Inject a tokio runtime handle (MT-017 tests): the headless `with_health` shell has no runtime, so
    /// the quick switcher cannot spawn its async search/recents tasks. A kittest provides a real
    /// multi-thread runtime handle so the stub-transport tasks actually run and deliver results.
    pub fn set_runtime_handle(&mut self, handle: tokio::runtime::Handle) {
        // Build the Loom-block rename client onto the injected runtime so an injected-runtime shell
        // (kittest) gets live off-thread rename too (MT-020).
        self.loom_block_client = Some(crate::backend_client::LoomBlockClient::production(
            handle.clone(),
        ));
        // MT-021: bridge the SCM + canvas off-thread clients onto the injected runtime too, so an
        // injected-runtime shell (kittest) gets live source-control + canvas calls.
        self.source_control_client = Some(crate::backend_client::SourceControlClient::production(
            handle.clone(),
        ));
        self.canvas_client = Some(crate::backend_client::CanvasClient::production(
            handle.clone(),
        ));
        // MT-101: bridge the model-session launch client onto the injected runtime so tests can point it
        // at a capture backend and prove the real `/jobs` request.
        self.model_session_launch_client = Some(
            crate::backend_client::ModelSessionLaunchClient::production(handle.clone()),
        );
        // MT-022: the rail makes NO backend call (AC-022-9), so there is no rail transport to bridge onto
        // the runtime — the rail emits its RailQuery intent into `search_rail_query` silently.
        // MT-023: bridge the drawer-data client onto the injected runtime so an injected-runtime shell
        // (kittest) gets live off-thread card fetches.
        self.drawer_data_client = Some(crate::backend_client::DrawerDataClient::production(
            handle.clone(),
        ));
        // MT-024: bridge the drawer card-action client onto the injected runtime so an injected-runtime
        // shell (kittest) gets live off-thread pin/discard/stow/attach-evidence dispatch.
        self.drawer_action_client = Some(crate::backend_client::DrawerActionClient::production(
            handle.clone(),
        ));
        self.runtime_handle = Some(handle);
    }

    /// A clone of the rail's emitted-intent slot (MT-022, the contract's `search_rail_query`:
    /// `Arc<Mutex<Option<RailQuery>>>`). A downstream search-results consumer / concurrent swarm thread
    /// holds this clone and reads the latest emitted intent off the shared lock to EXECUTE the search
    /// (search execution + results display are deferred to that consumer per AC-022-9). Cheap `Arc` clone.
    pub fn search_rail_query_slot(&self) -> crate::search_rail::RailQuerySlot {
        self.search_rail_query.clone()
    }

    /// The rail's latest emitted-intent query (MT-022, the contract's `search_rail_query` slot), cloned
    /// off the shared lock. `None` until the first rail fire (and after a clear). Read by tests + a
    /// concurrent swarm reader to observe what the rail last emitted (free-text + scope + facets). The
    /// rail does NOT execute the search — this is the emitted intent, not a result set.
    pub fn search_rail_query(&self) -> Option<crate::search_rail::RailQuery> {
        self.search_rail_query.lock().ok().and_then(|q| q.clone())
    }

    /// WP-KERNEL-012 MT-079: the Arc-shared code panel behind the MOUNTED code pane (the AC-079-3 proof
    /// drives this SAME panel — e.g. seeds a unified-undo entry then dispatches Undo through the bus and
    /// observes the panel state mutate).
    pub fn mounted_code_panel(&self) -> Arc<CodeEditorPanel> {
        Arc::clone(&self.editor_mounts.code_panel)
    }

    /// WP-KERNEL-012 MT-079: the Arc-shared rich editor state behind the MOUNTED Notes pane (the AC-079-5
    /// proof enqueues a `pending_events` entry on this SAME state and asserts it reaches the nav bus).
    pub fn mounted_rich_state(&self) -> Arc<Mutex<RichEditorState>> {
        Arc::clone(&self.editor_mounts.rich_state)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared canvas board behind the MOUNTED canvas pane (the AC-080-2 proof
    /// enqueues a `CanvasEvent` on this SAME board and asserts the host PATCH/re-fetch path fires).
    pub fn mounted_canvas_events(
        &self,
    ) -> Arc<Mutex<Vec<crate::graph::canvas_board::CanvasEvent>>> {
        Arc::clone(&self.editor_mounts.secondary.canvas_events)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared graph view behind the MOUNTED graph pane (the AC-080-3 proof
    /// puts the view in Local mode + enqueues a `DepthChanged` and asserts the depth re-query carries the
    /// new backlink_depth).
    pub fn mounted_graph_view(&self) -> Arc<Mutex<crate::graph::graph_view::LoomGraphView>> {
        Arc::clone(&self.editor_mounts.secondary.graph_view)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared graph-event outbound queue (the AC-080-3 proof enqueues a
    /// `DepthChanged` on this queue and asserts the host drains it into the depth re-query).
    pub fn editor_mounts_graph_events_for_test(
        &self,
    ) -> Arc<Mutex<Vec<crate::graph::graph_view::GraphEvent>>> {
        Arc::clone(&self.editor_mounts.secondary.graph_events)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared outgoing-links nav queue behind the MOUNTED outgoing-links pane
    /// (the AC-080-5 proof seeds a resolved link, clicks it, and asserts a nav target reaches this queue).
    pub fn mounted_outgoing_links(
        &self,
    ) -> Arc<Mutex<crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel>> {
        Arc::clone(&self.editor_mounts.secondary.outgoing_links)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared relevant-memory panel behind the MOUNTED relevant-memory pane
    /// (the AC-080-5 proof drives the EndpointMissing empty-state — the FEMS read route is verified ABSENT).
    pub fn mounted_relevant_memory(
        &self,
    ) -> Arc<Mutex<crate::fems::relevant_memory_panel::RelevantMemoryPanel>> {
        Arc::clone(&self.editor_mounts.secondary.relevant_memory)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared daily-journal state behind the MOUNTED daily-journal pane.
    pub fn mounted_daily_journal(
        &self,
    ) -> Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>> {
        Arc::clone(&self.editor_mounts.secondary.daily_journal)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared Stage pane behind the MOUNTED Stage pane.
    pub fn mounted_stage(&self) -> Arc<Mutex<crate::stage_pane::StagePane>> {
        Arc::clone(&self.editor_mounts.secondary.stage)
    }

    /// WP-KERNEL-012 MT-080: the Arc-shared manual pane search/selection state behind the MOUNTED manual
    /// pane (the registry CONTENT is immutable, held by the factory).
    pub fn mounted_manual_state(&self) -> Arc<Mutex<crate::manual_pane::ManualPaneState>> {
        Arc::clone(&self.editor_mounts.secondary.manual_state)
    }

    /// WP-KERNEL-012 MT-071: whether the FOCUSED pane this frame is the mounted code editor
    /// (`PaneType::CodeSymbol`). Reads the MT-035 [`InteractionBus`](crate::interop::InteractionBus)
    /// focus owner (the SAME focus seam the unified-undo enable predicates use) and resolves it against
    /// the pane registry. The editor status-bar segments render ONLY when this is true, so they HIDE the
    /// moment a non-code pane takes focus (AC-005) and never carry stale editor metadata. A contended bus
    /// lock or poisoned registry fails closed (no segments this frame, re-evaluated next frame).
    fn focused_pane_is_code_editor(&self, ctx: &egui::Context) -> bool {
        let bus = crate::interop::InteractionBus::get_or_init(ctx);
        let focus_owner: Option<PaneId> =
            crate::interop::InteractionBus::with_try_lock(&bus, |b| b.focus_owner().cloned())
                .flatten();
        let Some(focus_owner) = focus_owner else {
            return false;
        };
        self.pane_registry
            .lock()
            .map(|reg| {
                reg.get(&focus_owner)
                    .map(|record| matches!(record.pane_type, PaneType::CodeSymbol))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-071: the live editor file-metadata snapshot for the status-bar segments, or
    /// `None` when the focused pane is NOT a code editor (so the cluster hides — AC-005). Built each frame
    /// from the SAME mounted [`CodeEditorPanel`](crate::code_editor::panel::CodeEditorPanel) the host
    /// drives (the single doc-model source of truth — RISK-004), so the segments reflect the active
    /// document's real language / EOL / indent / encoding / whitespace state and never a parallel store.
    fn editor_segment_state(&self, ctx: &egui::Context) -> Option<EditorMetaSegmentState> {
        if !self.focused_pane_is_code_editor(ctx) {
            return None;
        }
        Some(EditorMetaSegmentState::from_panel(
            &self.editor_mounts.code_panel,
        ))
    }

    /// WP-KERNEL-012 MT-071: apply a typed [`EditorSegmentAction`] the status-bar segment emitted back
    /// onto the FOCUSED code document's MT-010 model (the "segment reports, the shell applies" discipline
    /// — RISK-004). Each arm routes to the existing panel mutator: language override, single-undo EOL
    /// convert, indent style (flips the Tab key), in-process encoding reopen (surfacing the typed `Err`),
    /// render-whitespace flip, or clipboard copy. Returns `true` when the action mutated state worth a
    /// repaint. NO backend call (the encoding reopen is the in-process MT-010 re-decode — RISK-005).
    fn apply_editor_segment_action(
        &self,
        ctx: &egui::Context,
        action: EditorSegmentAction,
    ) -> bool {
        let panel = &self.editor_mounts.code_panel;
        match action {
            EditorSegmentAction::SetLanguage(lang) => {
                panel.set_language_override(Some(lang));
                true
            }
            EditorSegmentAction::ConvertEol(eol) => panel.convert_eol(eol),
            EditorSegmentAction::SetIndent(style) => {
                panel.set_indent_style(style);
                true
            }
            EditorSegmentAction::ReopenWithEncoding(encoding) => {
                match panel.reopen_with_encoding(encoding) {
                    Ok(()) => true,
                    Err(message) => {
                        // Surface the typed reopen failure (e.g. in-memory buffer with no file path) as an
                        // honest non-fatal note rather than a silent no-op — the encoding does not change.
                        eprintln!("MT-071 reopen-with-encoding: {message}");
                        false
                    }
                }
            }
            EditorSegmentAction::SetRenderWhitespace(on) => {
                panel.set_render_whitespace(on);
                true
            }
            EditorSegmentAction::CopySegmentText(text) => {
                ctx.copy_text(text);
                true
            }
        }
    }

    /// WP-KERNEL-012 MT-079: the shared session-context cell the editor mounts read each frame (tests
    /// assert the shell pushed the active workspace + runtime into it so the editors thread real session
    /// context on mount — AC-079-2).
    pub fn editor_session_context(&self) -> SharedSessionContext {
        Arc::clone(&self.editor_mounts.session)
    }

    /// WP-KERNEL-012 MT-079: the last code-editor host-routed command the shell drained (Save /
    /// OpenCommandPalette). `None` until one is dispatched. The dispatch-observability surface for the
    /// AC-079-3 Save path (the Save WRITE itself is a typed carry owned by the document shell).
    pub fn last_editor_command(&self) -> Option<&CodeEditorAction> {
        self.last_editor_command.as_ref()
    }

    /// Test-only: point the MT-021 backend clients (SCM, canvas, Loom-block) at an arbitrary `base_url`
    /// bridged onto `handle`, so a test can drive `apply_*_event` against a localhost capture server and
    /// assert the EXACT URL + body reaches the wire through the REAL app dispatch path. Production code
    /// never calls this (it uses [`set_runtime_handle`](Self::set_runtime_handle) -> hardcoded backend
    /// URL); it exists so the MAJOR #1/#2/#3 "client genuinely consumed by the app" proof is end-to-end.
    pub fn set_backend_base_url_for_test(
        &mut self,
        base_url: &str,
        handle: tokio::runtime::Handle,
    ) {
        self.loom_block_client = Some(crate::backend_client::LoomBlockClient::new(
            base_url,
            handle.clone(),
        ));
        self.source_control_client = Some(crate::backend_client::SourceControlClient::new(
            base_url,
            handle.clone(),
        ));
        self.canvas_client = Some(crate::backend_client::CanvasClient::new(
            base_url,
            handle.clone(),
        ));
        self.model_session_launch_client = Some(
            crate::backend_client::ModelSessionLaunchClient::new(base_url, handle.clone()),
        );
        // MT-024: the drawer card-action client too, so the confirm-discard -> DELETE wire test (mirroring
        // PROOF-024-2(e)) can drive the REAL dispatch path against a localhost capture server.
        self.drawer_action_client = Some(crate::backend_client::DrawerActionClient::new(
            base_url,
            handle.clone(),
        ));
        self.rich_doc_base_url = base_url.to_owned();
        if let Ok(mut slot) = self.rich_doc_load_cell.lock() {
            slot.clear();
        }
        self.rich_doc_loading_id = None;
        self.rich_doc_load_generation = self.rich_doc_load_generation.wrapping_add(1);
        self.rich_doc_loaded_id = None;
        self.rich_doc_loaded_version = None;
        self.rich_doc_load_error = None;
        self.runtime_handle = Some(handle);
    }

    /// Directly seed the quick-switcher recents key list (MT-017) for tests asserting recents-first
    /// ordering without a live backend.
    pub fn set_quick_switcher_recents(&mut self, recents: Vec<String>) {
        self.quick_switcher_recents = recents;
    }

    /// Close the quick switcher (MT-017). Used by the overlay's Escape / Close / backdrop dismiss and
    /// after a jump. Safe to call when already closed.
    pub fn close_quick_switcher(&mut self) {
        self.quick_switcher_open = false;
    }

    /// Toggle the quick switcher open/closed (MT-017 Ctrl+P chord). Opening bumps the open generation
    /// (state reset); closing does not.
    pub fn toggle_quick_switcher(&mut self) {
        if self.quick_switcher_open {
            self.close_quick_switcher();
        } else {
            self.open_quick_switcher();
        }
    }

    /// The most-recent-first quick-switcher recents key list (`"{source_kind}:{ref_id}"`), for tests +
    /// the overlay's ordering. Loaded from the durable backend recents store on open and updated
    /// optimistically on selection (MT-017).
    pub fn quick_switcher_recents(&self) -> &[String] {
        &self.quick_switcher_recents
    }

    /// The last transient quick-switcher recents error, if any (MT-017), for tests + the status row.
    pub fn quick_switcher_recents_error(&self) -> Option<&str> {
        self.quick_switcher_recents_error.as_deref()
    }

    /// The current quick-switcher graph-search results (MT-017), for tests asserting the search
    /// delivered hits. Unordered (the overlay applies recents-first ordering at render time).
    pub fn quick_switcher_search_results(&self) -> &[crate::quick_switcher::LoomGraphSearchHit] {
        self.quick_switcher_search.results()
    }

    /// The active pane id, if any (MT-006 focus). Read by tests asserting a jump landed, and by future
    /// agent/operator wiring.
    pub fn active_pane(&self) -> Option<&PaneId> {
        self.active_pane.as_ref()
    }

    /// The last quick-switcher NAVIGATION status (MT-030), if the most recent dispatch through the
    /// [`ShellNavigator`](crate::quick_switcher::ShellNavigator) bus did NOT land on a real surface — most
    /// importantly the editor-pane TYPED SEAM ("Rich-text/Code editor pane not mounted yet (E11/MT-069)").
    /// `None` after a successful `Opened` dispatch. When `Some` this same text is rendered as a
    /// status-bar segment and emitted as the `quick-switcher.nav-status` AccessKit node (see
    /// [`quick_switcher_nav_status_segment`](Self::quick_switcher_nav_status_segment)); this getter is the
    /// programmatic mirror of that perceivable surface for the MT-030 dispatch tests and future E11 wiring.
    pub fn quick_switcher_nav_status(&self) -> Option<&str> {
        self.quick_switcher_nav_status.as_deref()
    }

    /// Open a `(PaneType, content_id)` tab on the module-target pane, labelled `label`, and make it the
    /// active tab + pane. The shared tab-open primitive behind the [`ShellNavigator`] arms: it
    /// de-duplicates by `(pane_type, content_id)` via `insert_tab`, activates the resulting tab, and
    /// focuses the pane. Returns the surface name on success, or `None` when there is no pane to open on
    /// (a headless empty surface) so the navigator can map that to [`NavDispatchOutcome::NoTargetPane`].
    fn open_navigator_tab(
        &mut self,
        pane_type: PaneType,
        content_id: String,
        label: &str,
    ) -> Option<String> {
        let surface = pane_type.label();
        let pane_id = self
            .navigator_existing_tab_pane(&pane_type, &content_id)
            .or_else(|| self.navigator_target_pane(&pane_type))?;
        let doc_to_reload = if matches!(pane_type, PaneType::LoomWikiPage) && !content_id.is_empty()
        {
            Some(content_id.clone())
        } else {
            None
        };
        if let Some(bar) = self.tab_bar_states.get_mut(&pane_id) {
            let tab = TabState {
                pane_type,
                content_id: Some(content_id),
                pinned: false,
                dirty: false,
                label_override: Some(label.to_owned()),
            };
            let idx = bar.insert_tab(tab);
            bar.activate(idx);
        }
        self.active_pane = Some(pane_id);
        if let Some(document_id) = doc_to_reload.as_deref() {
            self.invalidate_rich_document_load(document_id);
        }
        Some(surface)
    }

    /// Target pane for navigation-bus opens. If the operator has focused a pane, route there. If no
    /// active pane exists yet (fresh launch / model-open path), prefer a pane that already hosts the
    /// requested surface so "open document" uses the seeded Notes pane and "open symbol" uses the seeded
    /// code pane instead of duplicating a second editor in pane-a. Falls back to the existing deterministic
    /// module target when no matching surface is open.
    fn navigator_target_pane(&self, pane_type: &PaneType) -> Option<PaneId> {
        if let Some(active) = &self.active_pane {
            if self
                .tab_bar_states
                .get(active)
                .and_then(|bar| bar.active())
                .is_some_and(|tab| tab.pane_type == *pane_type)
            {
                return Some(active.clone());
            }
        }
        if matches!(pane_type, PaneType::LoomWikiPage) {
            if let Some(pane_id) = self.navigator_existing_surface_pane(pane_type) {
                return Some(pane_id);
            }
        }
        if let Some(active) = &self.active_pane {
            if self.tab_bar_states.contains_key(active) {
                return Some(active.clone());
            }
        }
        self.navigator_existing_surface_pane(pane_type)
            .or_else(|| self.module_target_pane())
    }

    fn navigator_existing_tab_pane(
        &self,
        pane_type: &PaneType,
        content_id: &str,
    ) -> Option<PaneId> {
        let mut candidates: Vec<PaneId> = self
            .tab_bar_states
            .iter()
            .filter(|(_, bar)| {
                bar.tabs.iter().any(|tab| {
                    tab.pane_type == *pane_type && tab.content_id.as_deref() == Some(content_id)
                })
            })
            .map(|(pane_id, _)| pane_id.clone())
            .collect();
        candidates.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
        candidates.into_iter().next()
    }

    fn navigator_existing_surface_pane(&self, pane_type: &PaneType) -> Option<PaneId> {
        let mut candidates: Vec<PaneId> = self
            .tab_bar_states
            .iter()
            .filter(|(_, bar)| bar.tabs.iter().any(|tab| tab.pane_type == *pane_type))
            .map(|(pane_id, _)| pane_id.clone())
            .collect();
        candidates.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
        candidates.into_iter().next()
    }

    /// Open a selected Loom-graph hit (MT-017 jump, refactored through the MT-030 [`ShellNavigator`]
    /// bus): record the durable recent (`POST recents`, OFF the egui UI thread — HBR-QUIET) with an
    /// immediate optimistic local prepend, then JUMP by dispatching the hit's typed target through the
    /// shell navigation bus ([`crate::quick_switcher::dispatch_target`]). Returns the typed
    /// [`NavDispatchOutcome`] so the caller can surface the editor-pane seam status (a `Document` /
    /// `CodeSymbol` target before E11 mounts the editor panes returns `EditorPaneNotMounted`, NOT a
    /// silent no-op or a faked open). An `Unsupported` target is a safe typed no-op (the row was disabled
    /// and never reaches here in normal flow).
    ///
    /// The network POST is NEVER awaited on the frame thread. The optimistic local recents update uses
    /// the hit's own [`hit_key`](crate::quick_switcher::hit_key) (which equals the backend-confirmed key
    /// for any well-formed hit) so the recents-first ordering is correct on the very next open without
    /// waiting for the round-trip; the spawned task writes the backend-confirmed key (or an error) into
    /// `quick_switcher_record_recent_cell`, drained next frame by [`drive_quick_switcher`] to reconcile
    /// the key and surface failures via `recents_error` (red-team MC3).
    fn open_switcher_hit(
        &mut self,
        hit: &crate::quick_switcher::LoomGraphSearchHit,
    ) -> crate::quick_switcher::NavDispatchOutcome {
        // 1. Optimistic local recents prepend (immediate, no I/O): the hit_key matches what the backend
        //    returns for a well-formed hit, so ordering is correct on the next open without blocking.
        let optimistic_key = crate::quick_switcher::hit_key(hit);
        self.quick_switcher_recents.retain(|k| k != &optimistic_key);
        self.quick_switcher_recents.insert(0, optimistic_key);
        self.quick_switcher_recents.truncate(20);

        // 2. Record the durable recent through the backend OFF the egui UI thread (HBR-QUIET: the network
        //    round-trip must not freeze the frame). Spawn on the runtime handle and write the result into
        //    the delivery cell; drive_quick_switcher drains it next frame (red-team MC3 surfaces errors).
        //    The same off-thread spawn pattern as the one-shot recents LOAD in drive_quick_switcher.
        let workspace = self.active_project_id.clone();
        if let (Some(transport), Some(handle)) = (
            self.quick_switcher_transport.clone(),
            self.runtime_handle.clone(),
        ) {
            let cell = self.quick_switcher_record_recent_cell.clone();
            let hit = hit.clone();
            handle.spawn_blocking(move || {
                let result = transport
                    .record_recent(&workspace, &hit)
                    .map_err(|e| e.to_string());
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(result);
                }
            });
        }

        // 3. Navigate: resolve the typed target and dispatch it through the MT-030 ShellNavigator bus.
        //    The hit title is stashed so the trait arms can label the opened tab (the arms take only ids
        //    per the contract signature; the label is presentation carried alongside).
        let target = crate::quick_switcher::resolve_open_target(hit);
        self.nav_pending_label = Some(hit.title.clone());
        let outcome = crate::quick_switcher::dispatch_target(self, &target);
        self.nav_pending_label = None;
        // Surface the dispatch status (the editor-pane seam shows "...not mounted yet (E11/MT-069)").
        match &outcome {
            crate::quick_switcher::NavDispatchOutcome::Opened { .. } => {
                self.quick_switcher_nav_status = None;
            }
            other => {
                self.quick_switcher_nav_status = Some(other.status_text());
            }
        }
        outcome
    }

    /// Drive one frame of the open quick switcher (MT-017): drain async deliveries, dispatch the
    /// debounced graph-search + the one-shot recents load on the tokio runtime, render the overlay with
    /// the recents-first ordered results, and apply the outcome (open a hit / close). All HTTP I/O runs
    /// on a spawned runtime task; this method only does non-blocking try_lock drains + spawns, so the
    /// egui frame thread is never blocked (HBR-QUIET). No-op-safe when there is no runtime/transport
    /// (headless) — the overlay then shows the empty/no-result state and search never fires.
    fn drive_quick_switcher(&mut self, ctx: &egui::Context) {
        let workspace = self.active_project_id.clone();
        let has_workspace = !workspace.is_empty();

        // 1. Drain a delivered recents load (try_lock; never hold across ui.* — red-team MC1).
        if let Ok(mut cell) = self.quick_switcher_recents_cell.try_lock() {
            if let Some(result) = cell.take() {
                match result {
                    Ok(keys) => {
                        self.quick_switcher_recents = keys;
                        self.quick_switcher_recents_error = None;
                    }
                    Err(msg) => {
                        // Keep whatever recents we had; surface the failure (red-team MC3).
                        self.quick_switcher_recents_error = Some(msg);
                    }
                }
                ctx.request_repaint();
            }
        }

        // 2. Drain a delivered search result into the search manager (stale deliveries dropped — MC2).
        if let Ok(mut cell) = self.quick_switcher_results_cell.try_lock() {
            if let Some(delivery) = cell.take() {
                if self.quick_switcher_search.drain(delivery) {
                    ctx.request_repaint();
                }
            }
        }

        // 2b. Drain a delivered recents-RECORD result (the off-thread `POST recents` for a picked hit;
        //     HBR-QUIET — the POST never blocked the frame). On success reconcile the backend-confirmed
        //     key to the front (the optimistic prepend used the hit's own key, normally identical); on
        //     failure surface it via recents_error without disturbing the optimistic local order (MC3).
        if let Ok(mut cell) = self.quick_switcher_record_recent_cell.try_lock() {
            if let Some(result) = cell.take() {
                match result {
                    Ok(key) => {
                        self.quick_switcher_recents.retain(|k| k != &key);
                        self.quick_switcher_recents.insert(0, key);
                        self.quick_switcher_recents.truncate(20);
                        self.quick_switcher_recents_error = None;
                    }
                    Err(msg) => {
                        self.quick_switcher_recents_error = Some(msg);
                    }
                }
                ctx.request_repaint();
            }
        }

        // 3. Fire the one-shot durable recents load on open (red-team MC4: guard on workspace).
        if self.quick_switcher_recents_pending {
            self.quick_switcher_recents_pending = false;
            if has_workspace {
                if let (Some(transport), Some(handle)) = (
                    self.quick_switcher_transport.clone(),
                    self.runtime_handle.clone(),
                ) {
                    let cell = self.quick_switcher_recents_cell.clone();
                    let ws = workspace.clone();
                    handle.spawn_blocking(move || {
                        let result = transport.list_recents(&ws).map_err(|e| e.to_string());
                        if let Ok(mut slot) = cell.lock() {
                            *slot = Some(result);
                        }
                    });
                }
            }
        }

        // 4. Render the overlay. The query is owned by egui memory; `show` returns it so the search
        //    manager can be ticked with the live text after rendering.
        let ordered = crate::quick_switcher::ordered_results(
            self.quick_switcher_search.results(),
            &self.quick_switcher_recents,
        );
        let frame = crate::quick_switcher::show(
            ctx,
            crate::quick_switcher::SwitcherView {
                open_count: self.quick_switcher_open_count,
                results: &ordered,
                has_workspace,
                loading: self.quick_switcher_search.loading(),
                error: self.quick_switcher_search.error(),
                recents_error: self.quick_switcher_recents_error.as_deref(),
            },
        );

        // 5. Tick the debounce state machine with the live query; spawn the search when it says Fire
        //    (red-team MC4: only with a workspace).
        let trimmed = frame.query.trim().to_owned();
        let action =
            self.quick_switcher_search
                .tick(&trimmed, has_workspace, std::time::Instant::now());
        if let crate::quick_switcher::SearchAction::Fire { query, sequence } = action {
            if let (Some(transport), Some(handle)) = (
                self.quick_switcher_transport.clone(),
                self.runtime_handle.clone(),
            ) {
                let cell = self.quick_switcher_results_cell.clone();
                let ws = workspace.clone();
                handle.spawn_blocking(move || {
                    let outcome = transport.search(&ws, &query).map_err(|e| e.to_string());
                    if let Ok(mut slot) = cell.lock() {
                        *slot = Some(crate::quick_switcher::SearchDelivery { sequence, outcome });
                    }
                });
            }
            ctx.request_repaint();
        } else if self.quick_switcher_search.loading()
            || self.quick_switcher_search.debounce_pending()
        {
            // While a request is in flight OR the debounce is still timing out, keep repainting so the
            // timer elapses + the delivery is drained promptly even with no further input events.
            ctx.request_repaint();
        }

        // 6. Apply the outcome.
        match frame.outcome {
            crate::quick_switcher::SwitcherOutcome::Open(hit) => {
                self.close_quick_switcher();
                // Dispatch through the MT-030 ShellNavigator bus. An `Opened` outcome changed pane/tab
                // state (repaint to land the jump); a typed non-open outcome (editor-pane seam / no pane)
                // is recorded in `quick_switcher_nav_status` and still repaints so the status shows.
                let _outcome = self.open_switcher_hit(&hit);
                ctx.request_repaint();
            }
            crate::quick_switcher::SwitcherOutcome::Close => {
                self.close_quick_switcher();
                ctx.request_repaint();
            }
            crate::quick_switcher::SwitcherOutcome::None => {}
        }
    }

    /// Drive one frame of the always-visible bottom search rail (MT-022): register the pinned bottom
    /// panel and render the rail (scope pills + query input + clear + Loom shortcut). The rail makes NO
    /// backend call (AC-022-9) and renders NO results — on an explicit fire (Enter / Loom) it EMITS the
    /// parsed [`crate::search_rail::RailQuery`] into the lock-guarded `search_rail_query` slot; on clear
    /// it writes `None`. Search EXECUTION + results display are deferred to a downstream search-results
    /// consumer that reads the slot. The slot write holds the lock only briefly and never across `ui.*`
    /// (HBR-QUIET / HBR-SWARM lock-discipline: a concurrent swarm reader can clone-and-read it safely).
    fn drive_search_rail(&mut self, ctx: &egui::Context) {
        let has_workspace = !self.active_project_id.is_empty();

        // Render the pinned bottom panel (registered before the central panel so it claims its space).
        let palette = self.current_theme.palette();
        let colors = crate::search_rail::RailVisuals {
            active_pill_bg: palette.accent_soft,
            inactive_pill_bg: palette.surface_strong,
            text: palette.text,
            accent: palette.accent,
        };
        let frame = egui::TopBottomPanel::bottom("handshake_bottom_search_rail")
            .exact_height(crate::search_rail::RAIL_HEIGHT)
            .show(ctx, |ui| {
                crate::search_rail::show(
                    ui,
                    crate::search_rail::RailView {
                        has_workspace,
                        colors,
                    },
                )
            })
            .inner;

        // Apply the rail outcome by WRITING the emitted intent into the shared slot (no backend call).
        //   - Fire (Enter / Loom — AC-022-3: ONLY these fire) writes the parsed RailQuery intent.
        //   - Clear writes None back (AC-022-6).
        // The downstream search-results consumer reads this slot and executes the search (deferred).
        match frame.outcome {
            crate::search_rail::RailOutcome::Clear => {
                if let Ok(mut slot) = self.search_rail_query.lock() {
                    *slot = None;
                }
                ctx.request_repaint();
            }
            crate::search_rail::RailOutcome::Fire(query) => {
                if has_workspace {
                    if let Ok(mut slot) = self.search_rail_query.lock() {
                        *slot = Some(*query);
                    }
                    ctx.request_repaint();
                }
            }
            crate::search_rail::RailOutcome::None => {}
        }
    }

    /// Read-only view of the bottom drawer stash shelf (MT-023): tests assert card data + the open flag.
    pub fn drawer(&self) -> &crate::stash_shelf::DrawerStashShelf {
        &self.drawer
    }

    /// Mutable view of the bottom drawer stash shelf (MT-024): tests + agents bind a card to a concrete
    /// Loom block (via `card_mut(..).action_target = Some(..)`) so its persisting actions are enabled.
    pub fn drawer_mut(&mut self) -> &mut crate::stash_shelf::DrawerStashShelf {
        &mut self.drawer
    }

    /// Drain any delivered drawer-card fetch result into the matching card, then render the MT-023 bottom
    /// drawer: the always-visible affordance tab (open or collapsed) plus — when open — the shelf panel
    /// ABOVE the rail with the four cards + the resize handle.
    ///
    /// PANEL ORDER (RISK-023-A): called AFTER [`drive_search_rail`](Self::drive_search_rail). In THIS
    /// codebase egui stacks bottom panels in registration order with the LATER panel ABOVE the earlier
    /// (the rail sits above the status bar because it registers after it), so registering the drawer
    /// AFTER the rail puts the drawer ABOVE the rail — its required position. This is the egui-correct
    /// order for this codebase; the contract's "drawer→rail→central" assumed the opposite egui
    /// convention (disclosed deviation — see stash_shelf module docs).
    fn drive_drawer(&mut self, ctx: &egui::Context) {
        // 1) Drain a delivered fetch result into the matching card (HBR-QUIET: the network already ran
        //    off the UI thread; this only reads the delivery cell + folds the result in).
        if let Ok(mut slot) = self.drawer_data_cell.lock() {
            if let Some((kind, result)) = slot.take() {
                // Map the backend-data kind to the shelf card kind.
                let card_kind = match kind {
                    crate::backend_client::DrawerDataKind::Agenda => {
                        crate::stash_shelf::DrawerCardKind::Agenda
                    }
                    crate::backend_client::DrawerDataKind::Lists => {
                        crate::stash_shelf::DrawerCardKind::Lists
                    }
                    crate::backend_client::DrawerDataKind::Notes => {
                        crate::stash_shelf::DrawerCardKind::Notes
                    }
                };
                if let Some(card) = self.drawer.card_mut(card_kind) {
                    card.apply_result(result);
                }
                ctx.request_repaint();
            }
        }

        // 2) Detect the closed→open transition (from ANY toggle surface) and fire the one-shot fetches
        //    ONCE (RISK-023-C: never per frame). Re-clamp the height to the window each frame so the
        //    drawer + rail + status bar can never collapse the CentralPanel (CONTROL-023-B).
        let open = self.bottom_drawer_open;
        if open && !self.drawer_prev_open {
            self.drawer_fetch_pending = true;
        }
        self.drawer_prev_open = open;
        if self.drawer_fetch_pending {
            self.drawer_fetch_pending = false;
            self.fire_drawer_fetches();
        }

        // 3) Theme tokens for the drawer.
        let palette = self.current_theme.palette();
        let colors = crate::stash_shelf::DrawerColors {
            panel_bg: palette.bg,
            card_bg: palette.surface,
            card_text: palette.text,
            muted_text: palette.text_subtle,
            badge_bg: palette.accent_soft,
            badge_text: palette.text,
            error_text: palette.error_text,
            affordance_bg: palette.surface_strong,
            affordance_hover_bg: palette.accent_soft,
            affordance_text: palette.text,
            resize_idle: palette.divider_idle,
            resize_hover: palette.divider_hover,
        };

        // 3b) Drain a delivered card-action receipt (HBR-QUIET: the backend call already ran off the UI
        //     thread). MAJOR FIX (AC-024-4/5): the contract's card-removal (Stow) / leftmost-reorder (Pin)
        //     lifecycle assumes a per-block item list; the MT-023 drawer is FOUR FIXED TYPE cards
        //     (Agenda/Mail/Lists/Notes), so there is no per-item card to remove or reorder. The honest
        //     success effect for a TYPE card is FEEDBACK: on Ok, clear the affected card's error state, set
        //     a brief success indicator, and refresh the affected count (re-fire the open fetches so the
        //     badge reflects the mutation). On Err, surface the error and clear the success indicator
        //     (disclosed deviation — feedback + count refresh, not removal/reorder).
        if let Some(result) = self
            .drawer_action_cell
            .lock()
            .ok()
            .and_then(|mut s| s.take())
        {
            let kind = self.drawer_action_in_flight.take();
            match result {
                Ok(()) => {
                    self.drawer_action_error = None;
                    self.drawer_action_success = kind;
                    if let Some(card_kind) = kind {
                        // Set the card's success indicator + clear any stale error (success supersedes a
                        // prior failure). The brief "✓ Done" line is cleared on the next count refresh.
                        if let Some(card) = self.drawer.card_mut(card_kind) {
                            card.mark_action_succeeded();
                        }
                    }
                    // Refresh the affected counts: re-fire the open fetches so the badge reflects the
                    // mutation (e.g. a Discard lowers a count). Only meaningful while the drawer is open.
                    if self.bottom_drawer_open {
                        self.drawer.mark_data_cards_loading();
                        self.fire_drawer_fetches();
                    }
                }
                Err(msg) => {
                    self.drawer_action_error = Some(msg);
                    self.drawer_action_success = None;
                }
            }
            ctx.request_repaint();
        }

        // 4) The open drawer panel, registered ABOVE the rail (after it). Re-clamp height first so the
        //    exact_height never starves the CentralPanel (RISK-023-B). The 32px rail + ~24px status bar
        //    are the reserved-below budget.
        let mut card_event: Option<crate::stash_shelf::DrawerEvent> = None;
        let mut action_event: Option<crate::stash_shelf::DrawerCardActionEvent> = None;
        if open {
            let avail = ctx.available_rect().height();
            self.drawer
                .clamp_height(avail, crate::search_rail::RAIL_HEIGHT + 24.0);
            let height = self.drawer.height;
            let drawer = &mut self.drawer;
            // ORDER: drawer -> rail -> central -- do not reorder. (egui registration order in THIS
            // codebase: the LATER bottom panel stacks ABOVE the earlier; the rail was registered in
            // drive_search_rail above, so registering the drawer here puts it ABOVE the rail.)
            let inner = egui::TopBottomPanel::bottom("hsk.drawer")
                .exact_height(height)
                .frame(egui::Frame::new().fill(colors.panel_bg))
                .show(ctx, |ui| drawer.show_open_panel(ui, colors))
                .inner;
            card_event = inner.nav;
            action_event = inner.action;
        }

        // 5) The affordance tab overlay — ALWAYS visible (open or collapsed — AC-023-1).
        if self.drawer.show_affordance(ctx, open, colors) {
            self.bottom_drawer_open = !self.bottom_drawer_open;
            ctx.request_repaint();
        }

        // 6) Apply a card click (AC-023-12): open the card's pane (Agenda/Lists/Notes) or show the Mail
        //    "Coming soon" tooltip (no navigation).
        if let Some(event) = card_event {
            match event {
                crate::stash_shelf::DrawerEvent::OpenCard(kind) => {
                    self.open_drawer_card_pane(kind);
                    ctx.request_repaint();
                }
                crate::stash_shelf::DrawerEvent::MailTooltip => {
                    // The hover tooltip is shown by the card itself; a click is a no-op navigation
                    // (AC-023-7) — Mail has no pane to open.
                    tracing::debug!(
                        "drawer: Mail card clicked — no backend / pane yet (Coming soon)"
                    );
                }
                crate::stash_shelf::DrawerEvent::ToggleOpen => {
                    self.bottom_drawer_open = !self.bottom_drawer_open;
                    ctx.request_repaint();
                }
            }
        }

        // 7) Apply a selected card ACTION (MT-024): dispatch the typed action through the action client
        //    (non-destructive persisting actions) or as a local intent / clipboard write (local actions).
        //    Selecting Discard does NOT dispatch — `apply_drawer_action` ARMS the confirm gate
        //    (`self.confirm_discard`) and the DELETE fires only from the confirm window's OK below
        //    (HBR-STOP / RISK-024-A / AC-024-11).
        if let Some(event) = action_event {
            self.apply_drawer_action(ctx, event);
            ctx.request_repaint();
        }

        // 8) HBR-STOP / RISK-024-A / CONTROL-024-A: while a Discard is armed, render the IN-APP
        //    "Confirm Discard" modal (HBR-QUIET — an egui::Window, never an OS dialog). OK dispatches the
        //    real DELETE; Cancel clears the arm with NO backend call.
        self.show_confirm_discard_window(ctx, colors);
    }

    /// Render the in-app "Confirm Discard" modal while a Discard is armed (HBR-STOP / RISK-024-A /
    /// AC-024-11 / CONTROL-024-A). This is the ONLY surface that can trigger the destructive DELETE: the
    /// menu's Discard item merely ARMS [`confirm_discard`](Self::confirm_discard); the actual
    /// [`crate::backend_client::DrawerActionClient::discard`] call happens in
    /// [`confirm_pending_discard`](Self::confirm_pending_discard) when OK is pressed. It is an
    /// `egui::Window` (HBR-QUIET — in-app, never a focus-stealing OS dialog). The OK/Cancel buttons carry
    /// stable AccessKit author_ids (`hsk.drawer.confirm.ok` / `.cancel`) so a swarm agent can drive the
    /// confirmation out-of-process; the window container carries `hsk.drawer.confirm.window`.
    fn show_confirm_discard_window(
        &mut self,
        ctx: &egui::Context,
        colors: crate::stash_shelf::DrawerColors,
    ) {
        if self.confirm_discard.is_none() {
            return;
        }
        let title = self
            .confirm_discard
            .as_ref()
            .map(|(_, t)| t.title.clone())
            .unwrap_or_default();
        let mut do_confirm = false;
        let mut do_cancel = false;
        egui::Window::new("Confirm Discard")
            .id(egui::Id::new("hsk.drawer.confirm.window"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                // A stable container author_id on the window body (Role::Group) so the modal itself is
                // addressable out-of-process by `hsk.drawer.confirm.window`.
                let window_id = ui.id().with("hsk.drawer.confirm.body");
                ui.ctx().accesskit_node_builder(window_id, |node| {
                    node.set_role(egui::accesskit::Role::Group);
                    node.set_author_id("hsk.drawer.confirm.window".to_owned());
                    node.set_label("Confirm Discard".to_owned());
                });
                ui.label(format!(
                    "Permanently delete \"{title}\"? This cannot be undone."
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    // OK = the single destructive trigger. A fixed-id button carrying the stable
                    // author_id `hsk.drawer.confirm.ok` (the codebase's fixed-id-button + node-builder
                    // pattern, so the interactive node and the named node are the SAME node).
                    if Self::confirm_button(
                        ui,
                        "Discard",
                        "hsk.drawer.confirm.ok",
                        colors.error_text,
                    ) {
                        do_confirm = true;
                    }
                    if Self::confirm_button(
                        ui,
                        "Cancel",
                        "hsk.drawer.confirm.cancel",
                        colors.card_text,
                    ) {
                        do_cancel = true;
                    }
                });
            });

        if do_confirm {
            self.confirm_pending_discard(ctx);
        } else if do_cancel {
            // Cancel: clear the arm with NO backend call (the destructive op never runs).
            self.confirm_discard = None;
            ctx.request_repaint();
        }
    }

    /// A confirm-window button: a labelled egui button carrying an explicit stable AccessKit author_id so
    /// the `assert_no_unnamed_interactive` gate is satisfied AND a swarm agent can address it
    /// out-of-process. Returns `true` when clicked.
    fn confirm_button(
        ui: &mut egui::Ui,
        label: &str,
        author_id: &'static str,
        text_color: egui::Color32,
    ) -> bool {
        let resp = ui.button(egui::RichText::new(label).color(text_color));
        let id = resp.id;
        let label_owned = label.to_owned();
        ui.ctx().accesskit_node_builder(id, move |node| {
            node.set_role(egui::accesskit::Role::Button);
            node.set_author_id(author_id.to_owned());
            node.set_label(label_owned.clone());
        });
        resp.clicked()
    }

    /// Dispatch the DESTRUCTIVE DELETE for the armed discard (HBR-STOP / RISK-024-A). This is the ONLY
    /// `DrawerActionClient::discard` call site, reached ONLY from the confirm window's OK. Clears the arm,
    /// then routes through the verified off-thread client (the dispatch -> client -> reqwest -> cell path
    /// is real). Returns `true` when a backend call was dispatched. A missing runtime/client surfaces a
    /// disclosed error (the honest-failure path) rather than panicking.
    fn confirm_pending_discard(&mut self, ctx: &egui::Context) -> bool {
        let Some((kind, t)) = self.confirm_discard.take() else {
            return false;
        };
        ctx.request_repaint();
        let Some(client) = self.drawer_action_client.clone() else {
            self.drawer_action_error =
                Some("Drawer actions unavailable (no backend runtime)".to_owned());
            return false;
        };
        self.drawer_action_error = None;
        // Attribute the delivered receipt to this card (MAJOR FIX AC-024-4/5 success feedback).
        self.drawer_action_in_flight = Some(kind);
        client.discard(
            &t.workspace_id,
            &t.block_id,
            self.drawer_action_cell.clone(),
        );
        true
    }

    /// Dispatch a confirmed drawer card action (MT-024). Persisting actions (Stow/Pin/Discard/
    /// AttachEvidence) route through the VERIFIED [`crate::backend_client::DrawerActionClient`] off the UI
    /// thread (genuinely CONSUMED here — the dispatch -> client -> reqwest -> cell path is real);
    /// Promote/SendToPane write a typed intent into the concurrency-safe `drawer_intents` (HBR-SWARM);
    /// CopyToPrompt writes the coder-prompt to the egui clipboard (no backend, headless-safe — no
    /// `arboard` dependency); ConvertArtifact is a disabled item and never reaches here. Returns `true`
    /// when a backend call was dispatched. Block-requiring actions with NO target are disabled in the
    /// menu, so a target is present whenever they reach here; this method still re-checks defensively.
    pub fn apply_drawer_action(
        &mut self,
        ctx: &egui::Context,
        event: crate::stash_shelf::DrawerCardActionEvent,
    ) -> bool {
        use crate::stash_shelf::DrawerCardAction as A;
        let target = event.target.clone();
        match event.action {
            // ── Local, no-backend actions ────────────────────────────────────────────────────────────
            A::CopyToPrompt => {
                if let Some(t) = &target {
                    // egui's native clipboard (the established codebase pattern — debug_console, project
                    // tree CopyPath). Headless-safe (no panic) and adds NO dependency, so it satisfies
                    // CONTROL-024-D's graceful-degradation intent better than the contract's `arboard`.
                    ctx.copy_text(t.coder_prompt());
                }
                false
            }
            A::Promote => {
                if let (Some(t), Ok(mut intents)) = (&target, self.drawer_intents.lock()) {
                    intents.promote_block_id = Some(t.block_id.clone());
                }
                false
            }
            A::SendToPane => {
                // Route to the active pane (the host owns pane selection). The MT-023 cards are TYPE
                // cards, so a per-block pane-PICKER popup is moot; the intent carries the destination the
                // host resolves. Disclosed deviation from the contract's secondary picker popup.
                let active_pane = self.active_pane_id();
                if let (Some(t), Ok(mut intents)) = (&target, self.drawer_intents.lock()) {
                    intents.send_to_pane = Some((t.block_id.clone(), active_pane));
                }
                false
            }
            A::ConvertArtifact => {
                // No backend surface exists to change a block's content_type (disclosed). The menu item
                // is disabled, so this branch is unreachable in normal flow; kept as an explicit no-op so
                // the match is exhaustive and a future wiring has a single obvious home.
                false
            }
            // ── Persisting, backend actions ──────────────────────────────────────────────────────────
            A::Stow | A::Pin | A::AttachEvidence | A::Discard => {
                let Some(t) = target else {
                    self.drawer_action_error = Some("This card has no block to act on".to_owned());
                    return false;
                };
                // HBR-STOP / RISK-024-A / AC-024-11 / CONTROL-024-A: a DESTRUCTIVE action
                // (`needs_confirm()` — only Discard, the irreversible DELETE) must NOT dispatch on
                // selection. ARM the in-app confirm gate instead: store the target so `drive_drawer`
                // renders the "Confirm Discard" `egui::Window`. The DELETE fires ONLY from that window's
                // OK button (see `confirm_pending_discard`). Selecting Discard here makes NO backend call.
                if event.action.needs_confirm() {
                    self.drawer_action_error = None;
                    self.confirm_discard = Some((event.kind, t));
                    return false;
                }
                // AC-024-9: Attach-evidence with no active job short-circuits with the contract message
                // BEFORE any client/runtime check — it must make NO backend call regardless of runtime.
                if event.action == A::AttachEvidence && self.active_job_id.is_none() {
                    self.drawer_action_error =
                        Some("No active job to attach evidence to".to_owned());
                    return false;
                }
                let Some(client) = self.drawer_action_client.clone() else {
                    self.drawer_action_error =
                        Some("Drawer actions unavailable (no backend runtime)".to_owned());
                    return false;
                };
                self.drawer_action_error = None;
                // Attribute the delivered receipt to this card so the receipt drain shows the success
                // indicator + refreshes the affected count (MAJOR FIX AC-024-4/5 success feedback).
                self.drawer_action_in_flight = Some(event.kind);
                let cell = self.drawer_action_cell.clone();
                match event.action {
                    A::Stow => {
                        client.stow(&t.workspace_id, &t.block_id, cell);
                        true
                    }
                    A::Pin => {
                        client.pin_to_top(&t.workspace_id, &t.block_id, cell);
                        true
                    }
                    A::AttachEvidence => {
                        // The no-job case was handled above; here a job is guaranteed present.
                        let job_id = self
                            .active_job_id
                            .clone()
                            .expect("active job checked above");
                        client.attach_evidence(
                            &t.workspace_id,
                            &t.block_id,
                            &t.title,
                            Some(&job_id),
                            cell,
                        );
                        true
                    }
                    // Discard returned early above via the `needs_confirm()` arm-gate (it ARMS the
                    // confirm window instead of dispatching); the real DELETE call site is
                    // `confirm_pending_discard`, reached only when the window's OK is pressed.
                    A::Discard => unreachable!(
                        "Discard is intercepted by the needs_confirm() gate before this match; the \
                         DELETE is dispatched only from confirm_pending_discard on OK"
                    ),
                    _ => unreachable!("outer match already narrowed to the persisting actions"),
                }
            }
        }
    }

    /// The active pane id the send-to-pane intent routes to (MT-024). Falls back to the first pane when
    /// there is no explicit active pane, and to a stable placeholder when no pane exists yet.
    fn active_pane_id(&self) -> String {
        self.tab_bar_states()
            .keys()
            .next()
            .map(|k| k.to_string())
            .unwrap_or_else(|| "pane-a".to_owned())
    }

    /// The most recent LOCAL drawer-action intents (MT-024, HBR-SWARM): Promote / Send-to-pane signals a
    /// swarm reader or test observes off the shared lock. Cloned snapshot.
    pub fn drawer_intents(&self) -> DrawerIntents {
        self.drawer_intents
            .lock()
            .map(|i| i.clone())
            .unwrap_or_default()
    }

    /// A cheap clone of the concurrency-safe drawer-intents handle for a swarm reader (HBR-SWARM).
    pub fn drawer_intents_handle(&self) -> std::sync::Arc<std::sync::Mutex<DrawerIntents>> {
        self.drawer_intents.clone()
    }

    /// The last drawer card-action error (MT-024), surfaced to tests + the drawer debug state. `None`
    /// when the last action succeeded or none has run.
    pub fn drawer_action_error(&self) -> Option<&str> {
        self.drawer_action_error.as_deref()
    }

    /// The card kind whose last drawer action SUCCEEDED (MT-024 MAJOR FIX AC-024-4/5), readable by tests
    /// + the drawer debug state. `None` when the last action failed or none has run.
    pub fn drawer_action_success(&self) -> Option<crate::stash_shelf::DrawerCardKind> {
        self.drawer_action_success
    }

    /// Whether a Discard is currently ARMED awaiting in-app confirmation (MT-024 BLOCKER FIX / HBR-STOP /
    /// RISK-024-A), with the target block id if so. Readable by tests: a `Some` proves the menu's Discard
    /// item ARMED the confirm gate instead of dispatching the DELETE.
    pub fn confirm_discard_block_id(&self) -> Option<&str> {
        self.confirm_discard
            .as_ref()
            .map(|(_, t)| t.block_id.as_str())
    }

    /// Set the active job id an Attach-evidence action records against (MT-024 AC-024-9). `None` disables
    /// the backend call (the action then shows a "no active job" message).
    pub fn set_active_job_id(&mut self, job_id: Option<String>) {
        self.active_job_id = job_id;
    }

    /// Fire the three one-shot off-thread drawer-card fetches (Agenda/Lists/Notes) against the verified
    /// backend, marking those cards as loading. Mail makes NO call (AC-023-7). A no-op (no loading flap)
    /// when there is no workspace (all cards then show their "No workspace" pre-fetch state) or no runtime
    /// (the headless shell has no client).
    fn fire_drawer_fetches(&mut self) {
        let workspace_id = self.active_project_id.clone();
        if workspace_id.is_empty() {
            // No workspace: show a "No workspace" state on the data cards, badge 0 (contract).
            for kind in [
                crate::stash_shelf::DrawerCardKind::Agenda,
                crate::stash_shelf::DrawerCardKind::Lists,
                crate::stash_shelf::DrawerCardKind::Notes,
            ] {
                if let Some(card) = self.drawer.card_mut(kind) {
                    card.loading = false;
                    card.badge_count = 0;
                    card.subtitle = "No workspace".to_owned();
                    card.error = None;
                }
            }
            return;
        }
        let Some(client) = self.drawer_data_client.clone() else {
            return;
        };
        self.drawer.mark_data_cards_loading();
        let cell = self.drawer_data_cell.clone();
        client.fetch_agenda(&workspace_id, &today_utc_ymd(), cell.clone());
        client.fetch_count(
            &workspace_id,
            crate::backend_client::DrawerDataKind::Lists,
            cell.clone(),
        );
        client.fetch_count(
            &workspace_id,
            crate::backend_client::DrawerDataKind::Notes,
            cell,
        );
    }

    /// Open the pane a drawer card links to (AC-023-12): Agenda → the daily-journal pane, Lists/Notes →
    /// a Loom-block collection pane. Routes through the same `open_content_on_active_pane` primitive the
    /// command palette / menu navigation use, so the card opens a REAL pane (not a fake nav).
    fn open_drawer_card_pane(&mut self, kind: crate::stash_shelf::DrawerCardKind) -> bool {
        use crate::stash_shelf::DrawerCardKind;
        match kind {
            DrawerCardKind::Agenda => {
                self.open_content_on_active_pane(PaneType::LoomDailyJournal, None)
            }
            // Lists/Notes open the Loom-block collection pane; the content_id carries the content_type
            // filter the collection pane reads (its filtered content lands with the pane-content WP).
            DrawerCardKind::Lists => {
                self.open_content_on_active_pane(PaneType::LoomBlock, Some("view_def".to_owned()))
            }
            DrawerCardKind::Notes => {
                self.open_content_on_active_pane(PaneType::LoomBlock, Some("note".to_owned()))
            }
            // Mail has no pane (handled as a tooltip upstream); never routed here.
            DrawerCardKind::Mail => false,
        }
    }

    /// Whether the settings overlay is requested open (MT-015 HELP menu; overlay UI is MT-018).
    pub fn settings_open(&self) -> bool {
        self.settings_open
    }

    /// The monotonic open generation of the settings dialog (MT-018). The dialog resets its transient
    /// state when this changes; exposed for tests that assert a re-open bumps the counter.
    pub fn settings_open_count(&self) -> u64 {
        self.settings_open_count
    }

    /// Open the settings dialog (MT-018), bumping the open generation so the overlay reseeds its query +
    /// keybinding drafts on this open. Idempotent while already open (a second open does NOT bump the
    /// generation, so it does not wipe an in-progress draft). The HELP menu, the palette `settings.open`
    /// action, and tests all route through here. Fires the one-shot settings LOAD on open.
    pub fn open_settings(&mut self) {
        if !self.settings_open {
            self.settings_open = true;
            self.settings_open_count = self.settings_open_count.wrapping_add(1);
            // Reload the persisted settings on open so the dialog reflects the durable state (and so a
            // PG round-trip restart shows the saved theme — PT6). Cleared after the load dispatches.
            self.settings_load_pending = true;
            self.settings_loaded_project_id = None;
        }
    }

    /// Close the settings dialog (MT-018). On close, FLUSH any pending debounced save IMMEDIATELY
    /// (red-team MC2) so a fast change-then-close never loses the change. Safe to call when already
    /// closed.
    pub fn close_settings(&mut self) {
        self.settings_open = false;
        if self.settings_save_due_at.take().is_some() {
            self.flush_settings_save_now();
        }
    }

    /// The live, persisted workspace settings (MT-018), for tests + the dialog. Backs the in-memory
    /// theme/view_mode flags where they map.
    pub fn workspace_settings(&self) -> &crate::workspace_settings::WorkspaceSettingsState {
        &self.workspace_settings
    }

    /// Inject a settings transport (MT-018) for tests/headless: a stub
    /// [`crate::workspace_settings::SettingsTransport`] drives the persistence with no live backend.
    pub fn set_settings_transport(
        &mut self,
        transport: Arc<dyn crate::workspace_settings::SettingsTransport>,
    ) {
        self.settings_transport = Some(transport);
    }

    /// The last transient settings persistence error, if any (MT-018), for tests + the status row.
    pub fn settings_persist_error(&self) -> Option<&str> {
        self.settings_persist_error.as_deref()
    }

    /// Test helper (MT-018): seed the workspace + in-memory theme directly so a kittest starts from a
    /// known theme before exercising a change. Sets both the persisted-settings theme and the in-memory
    /// `current_theme` so the dialog + the shell agree from frame one.
    #[doc(hidden)]
    pub fn set_workspace_theme_for_test(
        &mut self,
        theme: crate::workspace_settings::WorkspaceTheme,
    ) {
        self.workspace_settings.theme = theme;
        self.current_theme = theme.to_hs_theme();
        self.last_applied_theme = None;
    }

    /// Test helper (MT-018): set a keybinding chord directly in the live settings, BYPASSING the
    /// conflict guard. Used to seed a deliberately-conflicting state so the dialog's conflict banner can
    /// be asserted on open (the wired edit path refuses to commit a conflict, so a test cannot create the
    /// conflicting state through it).
    #[doc(hidden)]
    pub fn set_keybinding_for_test(&mut self, action_id: &str, chord: &str) {
        self.workspace_settings
            .set_chord(action_id, chord.to_owned());
    }

    /// Test helper (MT-072): seed the workspace syntax palette directly so a kittest can render the
    /// Custom swatch controls (whose author_ids exist only in Custom mode) and assert the section reflects
    /// the stored palette state.
    #[doc(hidden)]
    pub fn set_workspace_syntax_palette_for_test(
        &mut self,
        palette: crate::workspace_settings::SyntaxPalette,
    ) {
        self.workspace_settings.syntax_palette = palette;
    }

    /// Test helper (MT-018): apply a [`crate::settings_dialog::SettingsOutcome`] directly, the same way
    /// `drive_settings_dialog` does after rendering. A kittest cannot reliably click into an egui
    /// ComboBox popup item across frames, so the wired-change ACs (theme/view-mode) are exercised through
    /// the same outcome path the live ComboBox produces. Returns whether app state changed.
    #[doc(hidden)]
    pub fn apply_settings_outcome_for_test(
        &mut self,
        outcome: crate::settings_dialog::SettingsOutcome,
    ) -> bool {
        self.apply_settings_outcome(outcome)
    }

    /// Schedule a debounced settings `PUT` (MT-018, red-team R2): a flush fires
    /// [`SETTINGS_SAVE_DEBOUNCE`] after the LAST change so rapid keybinding edits coalesce. Called by
    /// each wired settings mutation.
    fn schedule_settings_save(&mut self) {
        self.settings_save_due_at = Some(std::time::Instant::now() + SETTINGS_SAVE_DEBOUNCE);
    }

    /// Spawn the settings `PUT` for the active workspace OFF the egui UI thread (HBR-QUIET), capturing
    /// the current settings blob on the UI thread. The result is drained next frame from
    /// `settings_save_cell`. No-op when no transport/runtime (headless without an injected stub).
    fn flush_settings_save_now(&mut self) {
        let workspace = self.active_project_id.clone();
        if workspace.is_empty() {
            return;
        }
        let blob = self.workspace_settings.to_settings_state();
        if let (Some(transport), Some(handle)) =
            (self.settings_transport.clone(), self.runtime_handle.clone())
        {
            if self
                .settings_io_in_flight
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                // A save/load is already running; re-arm so the next frame retries after it clears.
                self.schedule_settings_save();
                return;
            }
            let cell = self.settings_save_cell.clone();
            let in_flight = self.settings_io_in_flight.clone();
            handle.spawn_blocking(move || {
                let result = transport.save(&workspace, blob).map_err(|e| e.to_string());
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(result);
                }
                in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        }
    }

    /// WP-KERNEL-012 MT-072: push the persisted [`EditorPrefs`](crate::workspace_settings::EditorPrefs)
    /// into the LIVE MT-079-mounted code panel so a Settings edit takes effect on the running editor in
    /// the SAME frame (not just persisted). Reuses the panel's existing `&self` interior-mutability slots
    /// (no panel.rs edit, no parallel state):
    /// - `tab_size` + `insert_spaces` -> [`CodeEditorPanel::set_indent_settings`];
    /// - `render_whitespace` (any non-`None` mode draws) -> [`CodeEditorPanel::set_render_whitespace`];
    /// - `word_wrap` -> [`CodeEditorPanel::set_wrap_enabled`] + [`CodeEditorPanel::set_wrap_column`]
    ///   (`Off` => disabled; `On` => enabled, viewport-edge wrap; `BoundedColumn(n)` => enabled at `n`).
    ///
    /// `editor_font_size` is NOT applied here: the mounted [`CodeEditorPanel`] exposes no font-size slot
    /// today, so wiring it would require an editor-mount/panel.rs change OUTSIDE this MT's allowed_paths.
    /// That sub-field is the typed follow-up blocker recorded in the MT handoff (the pref still persists +
    /// round-trips; only its live application is deferred).
    fn sync_editor_prefs_to_panel(&self) {
        use crate::workspace_settings::WordWrapMode;
        let prefs = &self.workspace_settings.editor_prefs;
        let panel = &self.editor_mounts.code_panel;
        panel.set_indent_settings(prefs.tab_size as usize, prefs.insert_spaces);
        panel.set_render_whitespace(prefs.render_whitespace.draws_whitespace());
        match prefs.word_wrap {
            WordWrapMode::Off => {
                panel.set_wrap_enabled(false);
                panel.set_wrap_column(None);
            }
            WordWrapMode::On => {
                panel.set_wrap_enabled(true);
                panel.set_wrap_column(None);
            }
            WordWrapMode::BoundedColumn(col) => {
                panel.set_wrap_enabled(true);
                panel.set_wrap_column(Some(col as usize));
            }
        }
    }

    /// WP-KERNEL-012 MT-072: rebind the LIVE code-editor keymap from the persisted `editor_keybindings`
    /// overrides so a custom binding overrides the default for that action on the running editor (AC-005
    /// live side), not only in the Settings table. Reuses the existing override-application path exactly:
    /// the `code.`-prefixed overrides are projected into a [`KeymapSettings`] (bare action name + chord)
    /// and applied via the panel's [`CodeEditorPanel::reload_keymap_from_settings`] — the SAME seam the
    /// `~/.handshake/keymap.json` reload uses (so an unparseable chord / unknown action is skipped, never
    /// a panic). An override with no `code.` prefix is a rich-editor binding, which has no live keymap
    /// seam on the mounted rich editor today (`rich_editor/formatting/keymap.rs` is a fixed
    /// `resolve_shortcut`, OUTSIDE this MT's allowed_paths) — those are the typed follow-up blocker; the
    /// override still persists + lists default-vs-custom in the table.
    fn sync_editor_keymap_to_panel(&self) {
        use crate::code_editor::keymap_settings::{KeymapOverride, KeymapSettings};
        use crate::settings_editor_section::CODE_ACTION_ID_PREFIX;
        let overrides = self
            .workspace_settings
            .editor_keybindings
            .iter()
            .filter_map(|kb| {
                kb.action_id
                    .strip_prefix(CODE_ACTION_ID_PREFIX)
                    .map(|bare| KeymapOverride {
                        action: bare.to_owned(),
                        chord: kb.chord.clone(),
                    })
            })
            .collect();
        let settings = KeymapSettings { overrides };
        // `from_settings` layers the overrides over the VS Code defaults, so clearing an override and
        // re-syncing reverts that action to its default (custom-if-present-else-default — AC-005).
        self.editor_mounts
            .code_panel
            .reload_keymap_from_settings(&settings);
    }

    /// Apply a wired [`crate::settings_dialog::SettingsOutcome`] against the live shell state (MT-018).
    /// Returns `true` if app state changed (so the caller repaints). A wired change mutates
    /// `workspace_settings` (and the in-memory theme/view_mode flags where they map) and schedules a
    /// debounced `PUT`; `Close` clears the open flag (flushing a pending save); `None` is a no-op.
    fn apply_settings_outcome(&mut self, outcome: crate::settings_dialog::SettingsOutcome) -> bool {
        use crate::settings_dialog::SettingsOutcome as O;
        match outcome {
            O::None => false,
            O::Close => {
                self.close_settings();
                true
            }
            O::ThemeChanged(theme) => {
                self.workspace_settings.theme = theme;
                // Back the in-memory MT-003 theme flag; apply at the START of next frame (red-team
                // R4/MC4) so no widget renders with mixed visuals this frame.
                self.pending_theme_change = Some(theme.to_hs_theme());
                self.schedule_settings_save();
                true
            }
            O::ViewModeChanged(mode) => {
                self.workspace_settings.view_mode = mode;
                // Back the in-memory MT-015 view_mode flag.
                self.view_mode = match mode {
                    crate::workspace_settings::SettingsViewMode::Nsfw => ViewMode::Nsfw,
                    crate::workspace_settings::SettingsViewMode::Sfw => ViewMode::Sfw,
                };
                self.schedule_settings_save();
                true
            }
            O::KeybindingChanged { action_id, chord } => {
                self.workspace_settings.set_chord(&action_id, chord);
                self.schedule_settings_save();
                true
            }
            O::KeybindingReset { action_id } => {
                if let Some(action) = crate::workspace_settings::APP_KEYBINDING_ACTIONS
                    .iter()
                    .find(|a| a.id == action_id)
                {
                    self.workspace_settings
                        .set_chord(&action_id, action.default_chord.to_owned());
                    self.schedule_settings_save();
                    return true;
                }
                false
            }
            O::SwarmBoardDefaultOpenChanged(value) => {
                self.workspace_settings.swarm_board_default_open = value;
                self.schedule_settings_save();
                true
            }
            O::ResetLayout => {
                // Same action as VIEW > Reset Layout: arm the confirmation (red-team MC7), do not wipe
                // here. A future confirmation overlay / agent path triggers `confirm_reset_layout`.
                self.reset_layout_pending = true;
                true
            }
            // MT-072 editor settings: mutate the corresponding field on workspace_settings and schedule
            // the SAME debounced PUT every other wired outcome uses (no new save code — AC-009). The new
            // fields ride the same serialized settings struct.
            O::EditorPrefsChanged(prefs) => {
                self.workspace_settings.editor_prefs = prefs;
                // WIRE-INTO-LIVE (MT-072 note 87): push the new prefs into the running MT-079 code panel
                // so the edit takes effect this frame, not only on the persisted blob.
                self.sync_editor_prefs_to_panel();
                self.schedule_settings_save();
                true
            }
            O::SyntaxPaletteChanged(palette) => {
                self.workspace_settings.syntax_palette = palette;
                self.schedule_settings_save();
                true
            }
            O::EditorKeybindingChanged { action_id, chord } => {
                // Stored in the SEPARATE editor_keybindings list (NOT the WP-011 keybindings map — the
                // backend deny-unknown-validates that map; RISK-001).
                self.workspace_settings.set_editor_chord(&action_id, chord);
                // WIRE-INTO-LIVE (AC-005 live side): rebind the running code-editor keymap so a code
                // chord override takes effect on the editor, not only in the Settings table. (Rich-editor
                // overrides have no live keymap seam yet — typed follow-up blocker; see helper doc.)
                self.sync_editor_keymap_to_panel();
                self.schedule_settings_save();
                true
            }
            O::EditorKeybindingReset { action_id } => {
                if self.workspace_settings.clear_editor_chord(&action_id) {
                    // Re-sync so the cleared action reverts to its default on the live editor too.
                    self.sync_editor_keymap_to_panel();
                    self.schedule_settings_save();
                    return true;
                }
                false
            }
            O::WorksurfaceInspectorDumpRequested => {
                let root = crate::visual_debugger::default_artifact_root();
                match self.capture_worksurface_snapshot_to(root) {
                    Ok(_) => true,
                    Err(err) => {
                        self.worksurface_inspector_last_dump =
                            Some(format!("Worksurface snapshot failed: {err}"));
                        true
                    }
                }
            }
        }
    }

    /// Drive one frame of the open settings dialog (MT-018): drain async load/save deliveries, dispatch
    /// the one-shot load + the due debounced save on the tokio runtime (OFF the egui frame thread —
    /// HBR-QUIET), render the overlay, and apply the outcome. No-op-safe when there is no
    /// transport/runtime (headless) — the dialog then shows the seeded defaults and never does I/O.
    fn drive_settings_dialog(&mut self, ctx: &egui::Context) {
        // 1. Drain a delivered settings LOAD (try_lock; never hold across ui.* — red-team MC1).
        if let Ok(mut cell) = self.settings_load_cell.try_lock() {
            if let Some(result) = cell.take() {
                match result {
                    Ok(blob) => {
                        let fallback =
                            crate::workspace_settings::default_workspace_settings_state();
                        self.workspace_settings = match blob {
                            // A stored blob is normalized against defaults (red-team R6/MC6).
                            Some(value) => {
                                crate::workspace_settings::normalize_workspace_settings_state(
                                    &value, &fallback,
                                )
                            }
                            // First run (no settings yet): keep the defaults.
                            None => fallback,
                        };
                        // Back the in-memory flags from the loaded settings (apply theme next frame).
                        self.pending_theme_change =
                            Some(self.workspace_settings.theme.to_hs_theme());
                        self.view_mode = match self.workspace_settings.view_mode {
                            crate::workspace_settings::SettingsViewMode::Nsfw => ViewMode::Nsfw,
                            crate::workspace_settings::SettingsViewMode::Sfw => ViewMode::Sfw,
                        };
                        // WIRE-INTO-LIVE (MT-072): apply the loaded editor prefs + code-keymap overrides
                        // to the running MT-079 editor so a stored workspace opens with its persisted
                        // editor configuration in effect (parity with theme/view_mode above), not only
                        // after the user re-touches a control.
                        self.sync_editor_prefs_to_panel();
                        self.sync_editor_keymap_to_panel();
                        self.settings_persist_error = None;
                    }
                    Err(msg) => self.settings_persist_error = Some(msg),
                }
                ctx.request_repaint();
            }
        }

        // 2. Drain a delivered settings SAVE result.
        if let Ok(mut cell) = self.settings_save_cell.try_lock() {
            if let Some(result) = cell.take() {
                match result {
                    Ok(()) => self.settings_persist_error = None,
                    Err(msg) => self.settings_persist_error = Some(msg),
                }
                ctx.request_repaint();
            }
        }

        // 3. Fire the one-shot settings LOAD on open (red-team: load on open, not per-frame). Guard on a
        //    workspace + transport + runtime; spawn OFF the egui thread.
        if self.settings_load_pending {
            self.settings_load_pending = false;
            let workspace = self.active_project_id.clone();
            if !workspace.is_empty() {
                if let (Some(transport), Some(handle)) =
                    (self.settings_transport.clone(), self.runtime_handle.clone())
                {
                    if !self
                        .settings_io_in_flight
                        .swap(true, std::sync::atomic::Ordering::SeqCst)
                    {
                        let cell = self.settings_load_cell.clone();
                        let in_flight = self.settings_io_in_flight.clone();
                        let ws = workspace.clone();
                        handle.spawn_blocking(move || {
                            let result = transport.load(&ws).map_err(|e| e.to_string());
                            if let Ok(mut slot) = cell.lock() {
                                *slot = Some(result);
                            }
                            in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
                        });
                        ctx.request_repaint();
                    } else {
                        // I/O busy: re-arm the load for the next frame.
                        self.settings_load_pending = true;
                    }
                }
            }
            self.settings_loaded_project_id = Some(workspace);
        }

        // 4. Fire a DUE debounced save (red-team R2). When the quiet period has elapsed, spawn the PUT.
        if let Some(due) = self.settings_save_due_at {
            if std::time::Instant::now() >= due {
                self.settings_save_due_at = None;
                self.flush_settings_save_now();
            } else {
                // Keep repainting so the debounce window elapses even with no further input.
                ctx.request_repaint_after(SETTINGS_SAVE_DEBOUNCE);
            }
        }

        // 5. Render the dialog + apply the outcome.
        // MT-087: build the live internal_diagnostics projection + the active palette the Settings ->
        // Diagnostics section renders from. Built into locals first (owned values) so the borrows do not
        // conflict with the `&self.workspace_settings` borrow the SettingsView also takes.
        let diagnostics_view = self.diagnostics_view();
        let palette = self.current_theme.palette();
        let view = crate::settings_dialog::SettingsView {
            open_count: self.settings_open_count,
            settings: &self.workspace_settings,
            persist_error: self.settings_persist_error.as_deref(),
            diagnostics: &diagnostics_view,
            palette: &palette,
            worksurface_inspector_last_dump: self.worksurface_inspector_last_dump.as_deref(),
        };
        let outcome = crate::settings_dialog::show(ctx, view);
        if self.apply_settings_outcome(outcome) {
            ctx.request_repaint();
        }
    }

    /// Whether the About box is requested open (MT-015 HELP menu).
    pub fn about_open(&self) -> bool {
        self.about_open
    }

    /// Whether a Reset-Layout confirmation is currently armed (MT-015 VIEW menu; red-team MC7/R7). The
    /// reset does not happen until [`confirm_reset_layout`](Self::confirm_reset_layout) is called.
    pub fn reset_layout_pending(&self) -> bool {
        self.reset_layout_pending
    }

    /// Confirm the armed Reset-Layout request (red-team MC7): resets the live work surface to the active
    /// project's seeded default. No-op (returns `false`) when no reset is armed, so a stray confirm
    /// cannot wipe the layout. The arm step is `MenuBarAction::ResetLayout`; this is the second,
    /// deliberate confirm a future confirmation overlay / agent path triggers.
    pub fn confirm_reset_layout(&mut self) -> bool {
        if !self.reset_layout_pending {
            return false;
        }
        self.reset_layout_pending = false;
        let project = self.active_project_id.clone();
        self.reset_to_default_layout(&project);
        true
    }

    /// Cancel an armed Reset-Layout request without resetting (red-team MC7).
    pub fn cancel_reset_layout(&mut self) {
        self.reset_layout_pending = false;
    }

    /// Move the active pane focus to the next (or previous) pane in stable id order, wrapping at the
    /// ends (MT-015 GO menu). Returns `true` if the active pane changed. With no panes it is a safe
    /// no-op. When no pane is active yet, Next focuses the first pane and Prev focuses the last.
    fn focus_pane(&mut self, forward: bool) -> bool {
        let mut ids: Vec<PaneId> = self.tab_bar_states.keys().cloned().collect();
        ids.sort();
        if ids.is_empty() {
            return false;
        }
        let next = match &self.active_pane {
            Some(active) => match ids.iter().position(|p| p == active) {
                Some(i) => {
                    let len = ids.len();
                    let ni = if forward {
                        (i + 1) % len
                    } else {
                        (i + len - 1) % len
                    };
                    ids[ni].clone()
                }
                // Active pane not in the tab-bar set (shouldn't happen): fall back to the first/last.
                None => {
                    if forward {
                        ids[0].clone()
                    } else {
                        ids[ids.len() - 1].clone()
                    }
                }
            },
            None => {
                if forward {
                    ids[0].clone()
                } else {
                    ids[ids.len() - 1].clone()
                }
            }
        };
        let changed = self.active_pane.as_ref() != Some(&next);
        self.active_pane = Some(next);
        changed
    }

    /// Close the active tab on the active (or fallback) pane (MT-015 FILE > Close Tab). Returns `true`
    /// if a tab was removed. Pinned tabs are protected by [`TabBarState::close_tab`] (no-op). With no
    /// active pane, the deterministic fallback target ([`module_target_pane`](Self::module_target_pane))
    /// is used so the menu item still acts on a sensible pane.
    fn close_active_tab(&mut self) -> bool {
        let Some(target) = self.module_target_pane() else {
            return false;
        };
        if let Some(bar) = self.tab_bar_states.get_mut(&target) {
            let index = bar.active().map(|_| bar.active_index);
            if let Some(index) = index {
                return bar.close_tab(index);
            }
        }
        false
    }

    /// Map a `NavigateToTab` payload (the React `PaneTabId` string) onto the native [`PaneType`] and
    /// open it on the active pane (MT-015 RUN/HELP menus). Returns `true` if the pane state changed.
    /// An unknown id is a safe no-op (returns `false`) rather than a panic.
    fn navigate_to_tab(&mut self, tab_id: &str) -> bool {
        let pane_type = match tab_id {
            "inference-lab" => PaneType::InferenceLab,
            "flight-recorder" => PaneType::FlightRecorder,
            "user-manual" => PaneType::UserManual,
            "swarm" => PaneType::Swarm,
            _ => return false,
        };
        self.open_content_on_active_pane(pane_type, None)
    }

    fn active_workspace_terminal_cwd(&self) -> String {
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_owned())
    }

    fn active_workspace_model_folder(&self) -> String {
        // Project tabs currently carry workspace ids/names, not a canonical filesystem root. Seed the
        // dialog from the process cwd, then require the operator/model to submit the explicit folder value.
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_owned())
    }

    fn terminal_launch_status_text(err: &backend_client::TerminalLaunchError) -> String {
        match err {
            backend_client::TerminalLaunchError::EndpointMissing {
                probed_path,
                ipc_channel,
                ..
            } => format!(
                "Terminal: EndpointMissing {probed_path} (PTY runtime terminal/** is IPC-only via {ipc_channel})"
            ),
        }
    }

    fn model_session_direct_blocker_status(
        err: &backend_client::ModelSessionLaunchError,
    ) -> String {
        match err {
            backend_client::ModelSessionLaunchError::EndpointMissing {
                ipc_channel,
                probed_url,
                ..
            } => format!(
                "Model session: EndpointMissing {ipc_channel} (direct spawn with wrapper is IPC-only; probed {probed_url})"
            ),
            backend_client::ModelSessionLaunchError::InvalidRequest { field, reason } => {
                format!("Model session: InvalidRequest {field}: {reason}")
            }
        }
    }

    /// WP-KERNEL-012 MT-100: execute the native terminal-launch affordance. Until handshake_core exposes
    /// a native HTTP terminal-session route, this sets a typed visible blocker instead of pretending a
    /// terminal opened. Both RUN > Open Terminal in Workspace Folder and the palette command call here.
    fn open_workspace_terminal(&mut self) -> bool {
        let cwd = self.active_workspace_terminal_cwd();
        match backend_client::TerminalLaunchClient::production().open_workspace_terminal(cwd) {
            Ok(session) => {
                self.terminal_launch_status =
                    Some(format!("Terminal session opened: {}", session.session_id));
                true
            }
            Err(err) => {
                self.terminal_launch_status = Some(Self::terminal_launch_status_text(&err));
                true
            }
        }
    }

    pub fn terminal_launch_status_for_test(&self) -> Option<&str> {
        self.terminal_launch_status.as_deref()
    }

    /// WP-KERNEL-012 MT-101: open the compact launch dialog from RUN or the command palette. This is a
    /// one-shot operational surface, not Settings and not a new worksurface pane.
    fn open_model_session_launch_dialog(&mut self) -> bool {
        if self.model_session_launch_dialog.is_none() {
            self.model_session_launch_dialog = Some(ModelSessionLaunchDialogState::new(
                self.active_workspace_model_folder(),
            ));
        }
        self.model_session_launch_status
            .get_or_insert_with(|| MODEL_SESSION_CHOOSE_STATUS.to_owned());
        true
    }

    fn model_session_launch_request(
        &self,
        dialog: &ModelSessionLaunchDialogState,
    ) -> backend_client::ModelSessionLaunchRequest {
        backend_client::ModelSessionLaunchRequest::new(
            dialog.provider,
            self.active_project_id.clone(),
            dialog.workspace_folder.clone(),
            dialog.model_id.clone(),
            dialog.wrapper.clone(),
        )
    }

    fn submit_model_session_launch(&mut self, dialog: &ModelSessionLaunchDialogState) -> bool {
        if self.model_session_launch_pending {
            self.model_session_launch_status = Some(
                "Model session: POST /jobs already pending; duplicate launch ignored".to_owned(),
            );
            return true;
        }
        let request = self.model_session_launch_request(dialog);
        let Some(client) = self.model_session_launch_client.clone() else {
            let direct_status =
                match backend_client::ModelSessionLaunchClient::direct_spawn_workspace(
                    backend_client::BACKEND_BASE_URL,
                    &request,
                ) {
                    Ok(_) => {
                        "Model session: direct spawn returned unexpectedly; no runtime proof accepted"
                            .to_owned()
                    }
                    Err(err) => Self::model_session_direct_blocker_status(&err),
                };
            self.model_session_launch_direct_status = Some(direct_status.clone());
            self.model_session_launch_status = Some(format!(
                "Model session: POST /jobs not issued (no backend runtime); {direct_status}"
            ));
            return true;
        };
        let direct_status = match backend_client::ModelSessionLaunchClient::direct_spawn_workspace(
            client.base_url(),
            &request,
        ) {
            Ok(_) => "Model session: direct spawn returned unexpectedly; no runtime proof accepted"
                .to_owned(),
            Err(err) => Self::model_session_direct_blocker_status(&err),
        };
        self.model_session_launch_direct_status = Some(direct_status.clone());

        match client.launch_workspace_model_job(request, self.model_session_launch_cell.clone()) {
            Ok(_spec) => {
                self.model_session_launch_pending = true;
                self.model_session_launch_status = Some(format!(
                    "Model session: POST /jobs pending; {direct_status}"
                ));
                true
            }
            Err(err) => {
                self.model_session_launch_pending = false;
                self.model_session_launch_status = Some(format!(
                    "Model session: POST /jobs not issued; {}; {direct_status}",
                    err
                ));
                true
            }
        }
    }

    fn drain_model_session_launch_cell(&mut self) {
        let delivered = self
            .model_session_launch_cell
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let Some(result) = delivered else {
            return;
        };
        self.model_session_launch_pending = false;
        let direct_status = self
            .model_session_launch_direct_status
            .as_deref()
            .unwrap_or("EndpointMissing kernel_swarm_spawn_session");
        self.model_session_launch_status = Some(match result {
            Ok(job) => {
                let status = job.status.as_deref().unwrap_or("unknown");
                format!(
                    "Model session: /jobs job {} status={status}; NEEDS_MANAGED_RESOURCE_PROOF; {direct_status}",
                    job.job_id
                )
            }
            Err(message) => format!("Model session: POST /jobs failed: {message}; {direct_status}"),
        });
    }

    fn name_launch_node(
        ctx: &egui::Context,
        id: egui::Id,
        role: egui::accesskit::Role,
        author_id: &'static str,
        label: impl Into<String>,
    ) {
        let label = label.into();
        ctx.accesskit_node_builder(id, |node| {
            node.set_role(role);
            node.set_author_id(author_id.to_owned());
            node.set_label(label.clone());
        });
    }

    fn drive_model_session_launch_dialog(&mut self, ctx: &egui::Context) {
        self.drain_model_session_launch_cell();
        let Some(mut dialog) = self.model_session_launch_dialog.take() else {
            return;
        };

        let mut window_open = true;
        let mut cancel = false;
        let mut launch = false;
        let mut status = self
            .model_session_launch_status
            .clone()
            .unwrap_or_else(|| MODEL_SESSION_CHOOSE_STATUS.to_owned());
        if !self.model_session_launch_pending
            && (status == MODEL_SESSION_CHOOSE_STATUS || status == MODEL_SESSION_READY_STATUS)
        {
            status = if dialog.is_ready() {
                MODEL_SESSION_READY_STATUS
            } else {
                MODEL_SESSION_CHOOSE_STATUS
            }
            .to_owned();
            self.model_session_launch_status = Some(status.clone());
        }

        let shown = egui::Window::new("Launch Model Session")
            .id(egui::Id::new(MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID))
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Provider");
                        let combo =
                            egui::ComboBox::from_id_salt(MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID)
                                .selected_text(dialog.provider.label())
                                .show_ui(ui, |ui| {
                                    let local = ui.selectable_value(
                                        &mut dialog.provider,
                                        backend_client::ModelSessionProvider::Local,
                                        "Local",
                                    );
                                    Self::name_launch_node(
                                        ui.ctx(),
                                        local.id,
                                        egui::accesskit::Role::MenuItem,
                                        MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID,
                                        "Local model provider",
                                    );
                                    let cloud = ui.selectable_value(
                                        &mut dialog.provider,
                                        backend_client::ModelSessionProvider::Cloud,
                                        "Cloud",
                                    );
                                    Self::name_launch_node(
                                        ui.ctx(),
                                        cloud.id,
                                        egui::accesskit::Role::MenuItem,
                                        MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID,
                                        "Cloud model provider",
                                    );
                                });
                        Self::name_launch_node(
                            ui.ctx(),
                            combo.response.id,
                            egui::accesskit::Role::ComboBox,
                            MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID,
                            format!("Provider {}", dialog.provider.label()),
                        );
                    });
                    ui.add_space(6.0);
                    ui.label("Workspace folder");
                    let folder = ui.add(
                        egui::TextEdit::singleline(&mut dialog.workspace_folder)
                            .desired_width(400.0),
                    );
                    Self::name_launch_node(
                        ui.ctx(),
                        folder.id,
                        egui::accesskit::Role::TextInput,
                        MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID,
                        "Workspace folder",
                    );
                    ui.add_space(6.0);
                    ui.label("Model");
                    let model = ui
                        .add(egui::TextEdit::singleline(&mut dialog.model_id).desired_width(400.0));
                    Self::name_launch_node(
                        ui.ctx(),
                        model.id,
                        egui::accesskit::Role::TextInput,
                        MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID,
                        "Model id or cloud model name",
                    );
                    ui.add_space(6.0);
                    ui.label("Wrapper");
                    let wrapper = ui
                        .add(egui::TextEdit::singleline(&mut dialog.wrapper).desired_width(400.0));
                    Self::name_launch_node(
                        ui.ctx(),
                        wrapper.id,
                        egui::accesskit::Role::TextInput,
                        MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID,
                        "Wrapper",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let launch_response = ui.add_enabled(
                            dialog.is_ready() && !self.model_session_launch_pending,
                            egui::Button::new("Launch"),
                        );
                        Self::name_launch_node(
                            ui.ctx(),
                            launch_response.id,
                            egui::accesskit::Role::Button,
                            MODEL_SESSION_LAUNCH_START_AUTHOR_ID,
                            "Launch model session",
                        );
                        if launch_response.clicked() {
                            launch = true;
                        }
                        let cancel_response = ui.button("Cancel");
                        Self::name_launch_node(
                            ui.ctx(),
                            cancel_response.id,
                            egui::accesskit::Role::Button,
                            MODEL_SESSION_LAUNCH_CANCEL_AUTHOR_ID,
                            "Cancel model session launch",
                        );
                        if cancel_response.clicked() {
                            cancel = true;
                        }
                    });
                    ui.add_space(6.0);
                    let status_response = ui.add(egui::Label::new(status.clone()).wrap());
                    Self::name_launch_node(
                        ui.ctx(),
                        status_response.id,
                        egui::accesskit::Role::Status,
                        MODEL_SESSION_LAUNCH_INLINE_STATUS_AUTHOR_ID,
                        status.clone(),
                    );
                });
            });

        if let Some(inner) = shown {
            Self::name_launch_node(
                ctx,
                inner.response.id,
                egui::accesskit::Role::Dialog,
                MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID,
                "Launch Model Session",
            );
        }
        if launch {
            self.submit_model_session_launch(&dialog);
            ctx.request_repaint();
        }
        if cancel {
            window_open = false;
        }
        if window_open {
            self.model_session_launch_dialog = Some(dialog);
        }
    }

    pub fn model_session_launch_status_for_test(&self) -> Option<&str> {
        self.model_session_launch_status.as_deref()
    }

    pub fn model_session_launch_pending_for_test(&self) -> bool {
        self.model_session_launch_pending
    }

    pub fn model_session_launch_dialog_open_for_test(&self) -> bool {
        self.model_session_launch_dialog.is_some()
    }

    pub fn set_model_session_launch_dialog_for_test(
        &mut self,
        dialog: ModelSessionLaunchDialogState,
    ) {
        self.model_session_launch_dialog = Some(dialog);
    }

    /// Dispatch a [`MenuBarAction`] returned by the top menu bar into the shell's existing state-
    /// mutation paths (MT-015). Returns `true` if app state changed (so the caller can request a
    /// repaint + let the layout change-detector schedule a save). EXHAUSTIVE on `MenuBarAction` so a
    /// new menu action cannot be added without the shell handling it (compiler-enforced).
    ///
    /// `ctx` is needed for the genuine window action (Quit -> viewport Close). Disabled-leaf variants
    /// (document/editor/file-drawer/terminal targets that do not exist yet) are matched but are
    /// unreachable in MT-015 because their leaves render disabled; they are handled as explicit no-ops
    /// so the exhaustive match is honest about which surfaces are not yet wired.
    fn dispatch_menu_action(&mut self, ctx: &egui::Context, action: MenuBarAction) -> bool {
        match action {
            // ── Wired (target exists) ──────────────────────────────────────────────────────────────
            MenuBarAction::ToggleTheme => {
                self.current_theme = self.current_theme.toggled();
                // Apply the new palette THIS frame so the menu's checkmark + the shell flip together
                // with no one-frame flicker (red-team R4).
                self.apply_theme_if_changed(ctx);
                true
            }
            MenuBarAction::ToggleViewMode => {
                self.view_mode = self.view_mode.toggled();
                true
            }
            MenuBarAction::ToggleProjectDrawer => {
                self.left_rail_open = !self.left_rail_open;
                true
            }
            MenuBarAction::ToggleBottomPanel => {
                self.bottom_drawer_open = !self.bottom_drawer_open;
                true
            }
            MenuBarAction::ResetLayout => {
                // Arm the confirmation (red-team MC7/R7) — do NOT reset here. The actual reset requires
                // a deliberate second confirm via `confirm_reset_layout`.
                self.reset_layout_pending = true;
                true
            }
            MenuBarAction::OpenQuickSwitcher => {
                // MT-017: route through the opener so the open generation bumps (overlay state reset).
                self.open_quick_switcher();
                true
            }
            MenuBarAction::OpenCommandPalette => {
                self.open_command_palette();
                true
            }
            MenuBarAction::OpenSettings => {
                self.open_settings();
                true
            }
            MenuBarAction::ShowAbout => {
                self.about_open = true;
                true
            }
            MenuBarAction::FocusNextPane => self.focus_pane(true),
            MenuBarAction::FocusPrevPane => self.focus_pane(false),
            MenuBarAction::CloseActiveTab => self.close_active_tab(),
            MenuBarAction::OpenSwarmBoard => self.navigate_to_tab("swarm"),
            MenuBarAction::NavigateToTab(tab_id) => self.navigate_to_tab(&tab_id),
            MenuBarAction::OpenModelSessionLaunch => self.open_model_session_launch_dialog(),
            MenuBarAction::OpenTerminal => self.open_workspace_terminal(),
            MenuBarAction::QuitApp => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                false
            }
            // WP-KERNEL-012 MT-069 (E11 menu wire-up): an editor FILE/EDIT menu item dispatches its real
            // editor command by id through the ONE shared dispatcher the command palette also calls
            // (`dispatch_editor_command`), so menu-driven and palette-driven editor actions share one path
            // (RISK-001). The menu handler routes by command id ONLY — no inline editor logic here.
            MenuBarAction::EditorCommand(command_id) => {
                self.dispatch_editor_command(ctx, command_id)
            }
            // ── Disabled in MT-015 (target surface is a future MT) — leaves render disabled, so these
            //    are unreachable; handled as explicit no-ops to keep the match honest + exhaustive. ──
            MenuBarAction::NewDocument
            | MenuBarAction::OpenWorkspacePicker
            | MenuBarAction::SaveActiveDocument
            | MenuBarAction::SaveAllDocuments
            | MenuBarAction::EditorUndo
            | MenuBarAction::EditorRedo
            | MenuBarAction::EditCut
            | MenuBarAction::EditCopy
            | MenuBarAction::EditPaste
            | MenuBarAction::OpenFindReplace
            | MenuBarAction::OpenWorkspaceSearch
            | MenuBarAction::ToggleFileDrawer => false,
        }
    }

    /// WP-KERNEL-012 MT-069: whether an editor pane is the focusable/active target this frame — the live
    /// ENABLE PREDICATE for the FILE/EDIT editor menu + palette items. True when the running pane registry
    /// holds at least one editor pane (`PaneType::CodeSymbol` code editor or `PaneType::LoomWikiPage`
    /// Notes/rich editor), which MT-079 host-mounts as the real editor factories. When no editor pane is
    /// mounted, the editor menu/palette items render DISABLED (honest, not fake-enabled). A poisoned/locked
    /// registry is treated as "no editor available" (fail-closed) rather than panicking on the frame.
    pub fn editor_available(&self) -> bool {
        self.pane_registry
            .lock()
            .map(|reg| {
                reg.iter().any(|(_, record)| {
                    matches!(
                        record.pane_type,
                        PaneType::CodeSymbol | PaneType::LoomWikiPage
                    )
                })
            })
            .unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-069: whether the MT-035 unified-undo scope has an undoable action for the focused
    /// pane (or the cross-pane ring) — the live ENABLE PREDICATE for EDIT > Undo. Reads the SAME shared
    /// InteractionBus scope the keyboard Undo path mutates, so menu/keyboard enable state cannot diverge
    /// (RISK-002 / RISK-006). A contended bus lock reports `false` for this frame (re-evaluated next frame).
    fn editor_can_undo(&self, ctx: &egui::Context) -> bool {
        let bus = crate::interop::InteractionBus::get_or_init(ctx);
        crate::interop::InteractionBus::with_try_lock(&bus, |b| {
            let local = b
                .focus_owner()
                .map(|p| b.undo_scope().can_undo_local(p))
                .unwrap_or(false);
            local || b.undo_scope().can_undo_cross_pane()
        })
        .unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-069: whether the MT-035 unified-undo scope has a redoable action for the focused
    /// pane — the live ENABLE PREDICATE for EDIT > Redo (mirror of [`editor_can_undo`]).
    fn editor_can_redo(&self, ctx: &egui::Context) -> bool {
        let bus = crate::interop::InteractionBus::get_or_init(ctx);
        crate::interop::InteractionBus::with_try_lock(&bus, |b| {
            b.focus_owner()
                .map(|p| b.undo_scope().can_redo_local(p))
                .unwrap_or(false)
        })
        .unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-069: whether the MT-031 shared clipboard holds a consumable payload — the live
    /// ENABLE PREDICATE for EDIT > Paste (VS Code enables Paste only when the clipboard has content). Reads
    /// the SAME cross-pane clipboard cache the Cut/Copy commands populate.
    fn editor_can_paste(&self, ctx: &egui::Context) -> bool {
        let bus = crate::interop::InteractionBus::get_or_init(ctx);
        crate::interop::InteractionBus::with_try_lock(&bus, |b| b.clipboard_read().is_some())
            .unwrap_or(false)
    }

    /// Build the per-frame [`MenuBarState`] the menu bar reads for checkmarks + enable/disable. `ctx` is
    /// threaded so the MT-069 editor enable predicates can read the live InteractionBus (unified-undo +
    /// clipboard) — the SAME shared state the keyboard paths mutate, so menu enable state never diverges.
    fn menu_bar_state(&self, ctx: &egui::Context) -> MenuBarState {
        let has_active_tab = self
            .module_target_pane()
            .and_then(|p| self.tab_bar_states.get(&p))
            .map(|bar| bar.active().is_some())
            .unwrap_or(false);
        let editor_available = self.editor_available();
        MenuBarState {
            theme_is_dark: self.current_theme == HsTheme::Dark,
            view_mode_is_nsfw: self.view_mode == ViewMode::Nsfw,
            project_drawer_open: self.left_rail_open,
            bottom_drawer_open: self.bottom_drawer_open,
            has_active_tab,
            // MT-069 editor enable predicates (live, read from the shared substrate).
            editor_available,
            editor_can_undo: editor_available && self.editor_can_undo(ctx),
            editor_can_redo: editor_available && self.editor_can_redo(ctx),
            editor_can_paste: editor_available && self.editor_can_paste(ctx),
        }
    }

    /// Read-only view of the left rail (tests assert the project tree / quick-links state).
    pub fn left_rail(&self) -> &LeftRail {
        &self.left_rail
    }

    /// Mutable view of the left rail (tests seed the project tree's documents/canvases directly, with no
    /// live backend, via `left_rail_mut().project_tree.set_content(...)`).
    pub fn left_rail_mut(&mut self) -> &mut LeftRail {
        &mut self.left_rail
    }

    /// The pane a module switch targets: the active pane if one is set, else the alphabetically-first
    /// pane that has a tab bar (the seeded default is `pane-a`). Mirrors the React `activePaneId`
    /// default — `setModule` always mutates exactly one pane, never zero. Returns `None` only when there
    /// are no panes at all (which the seeded shell never is), so a switch on an empty surface is a safe
    /// no-op rather than a panic.
    fn module_target_pane(&self) -> Option<PaneId> {
        if let Some(active) = &self.active_pane {
            if self.tab_bar_states.contains_key(active) {
                return Some(active.clone());
            }
        }
        // Deterministic fallback: the lowest pane id (BTree-style order) that owns a tab bar.
        self.tab_bar_states.keys().min().cloned()
    }

    /// Switch the active MODULE (MT-012), mirroring the React `setModule` (`app/src/App.tsx` lines
    /// 1463-1483) exactly:
    ///
    /// 1. set `active_module = module_id` (the switcher highlight);
    /// 2. on the ACTIVE pane only, rebuild its tab list to
    ///    `uniqueTabs([defaultTab, ...module.tabs, ...existing_pane_tabs])` and activate the module's
    ///    default tab.
    ///
    /// Returns `true` when state changed. Switching to the already-active module is a NO-OP that returns
    /// `false` (the contract's no-op acceptance criterion: no state change, no layout-save trigger). The
    /// save is NOT triggered synchronously here — the existing MT-006/MT-009 change-detector (which
    /// diffs the captured `layout_state` each frame) schedules the debounced save on the next frame, so
    /// rapid module clicks coalesce into one save rather than a save storm (red-team control).
    pub fn set_module(&mut self, module_id: ModuleId) -> bool {
        if self.module_switcher.active() == module_id {
            return false;
        }
        self.module_switcher.set_active(module_id);

        // Mutate exactly the active (target) pane's tab bar.
        let Some(target) = self.module_target_pane() else {
            // No panes at all: the highlight moved but there is nothing to retab. Highlight already
            // changed above, so report a change so the (degenerate) state still persists.
            return true;
        };
        if let Some(bar) = self.tab_bar_states.get_mut(&target) {
            let existing: Vec<PaneType> = bar.tabs.iter().map(|t| t.pane_type.clone()).collect();
            let next_tabs = crate::module_switcher::module_tab_list(module_id, &existing);
            let def = module_id.definition();
            // Rebuild the bar from the new tab list (dedup + pin-stabilize via TabBarState::new), then
            // activate the module's default tab (the first effective tab after the new() dedup).
            let mut rebuilt = TabBarState::new(
                target.clone(),
                next_tabs.into_iter().map(TabState::new).collect(),
            );
            let default_index = rebuilt
                .tabs
                .iter()
                .position(|t| t.pane_type == def.default_tab)
                .unwrap_or(0);
            rebuilt.activate(default_index);
            *bar = rebuilt;
        }
        true
    }

    /// Build the live ACTIVE-WINDOW quick-link entries (MT-014) from the current pane tab bars: one
    /// [`crate::quick_links::QuickLinkEntry`] per open tab, with `is_active` set on each pane's active
    /// tab so the rail's collapsed view shows only the active tabs. The owning project name is the
    /// active project's display name from the project-tab strip (all seeded panes belong to the active
    /// project in this WP; a future multi-project-pane MT can vary it per pane). Panes are visited in
    /// stable id order (BTreeMap-style) so the list is deterministic frame-to-frame.
    fn build_quick_link_entries(&self) -> Vec<crate::quick_links::QuickLinkEntry> {
        let project_name = self
            .project_tabs
            .projects()
            .iter()
            .find(|p| p.id == self.active_project_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| self.active_project_id.clone());

        let mut pane_ids: Vec<&PaneId> = self.tab_bar_states.keys().collect();
        pane_ids.sort();
        let mut entries = Vec::new();
        for pane_id in pane_ids {
            let Some(bar) = self.tab_bar_states.get(pane_id) else {
                continue;
            };
            for (index, tab) in bar.tabs.iter().enumerate() {
                entries.push(crate::quick_links::QuickLinkEntry {
                    pane_id: pane_id.clone(),
                    tab_index: index,
                    project_name: project_name.clone(),
                    tab_label: tab.label(),
                    pane_type: tab.pane_type.clone(),
                    is_active: index == bar.active_index,
                });
            }
        }
        entries
    }

    /// Apply a left-rail event (MT-014) against the live shell state (single source of truth). Returns
    /// `true` if the event changed app state (so the caller can request a repaint + let the layout
    /// change-detector schedule a save). Mirrors the React handlers the WorkspaceSidebar/App wired:
    /// document/canvas open -> open a tab on the active pane; quick-link click -> focus pane + activate
    /// tab; stash/rail toggles -> flip the persisted drawer flags; agenda/mail/notes -> open the tab.
    fn apply_left_rail_event(&mut self, ctx: &egui::Context, event: LeftRailEvent) -> bool {
        match event {
            LeftRailEvent::OpenDocument(doc_id) => {
                self.open_content_on_active_pane(PaneType::Workspace, Some(doc_id))
            }
            LeftRailEvent::OpenCanvas(canvas_id) => {
                // Canvases open on the Atelier editor surface (the canvas editor), carrying the id.
                self.open_content_on_active_pane(PaneType::AtelierEditor, Some(canvas_id))
            }
            LeftRailEvent::OpenBookmark {
                document_id,
                block_id,
            } => {
                // Mirror React `handleOpenBookmark`: a document pin opens as that document on the
                // Workspace surface; otherwise the pinned Loom block opens on the LoomBlock surface.
                match document_id {
                    Some(doc_id) => {
                        self.open_content_on_active_pane(PaneType::Workspace, Some(doc_id))
                    }
                    None => self.open_content_on_active_pane(PaneType::LoomBlock, Some(block_id)),
                }
            }
            LeftRailEvent::CopyPath(id) => {
                // Copy the row's stable id/path to the clipboard via egui's output clipboard (no
                // backend). This is the externally-visible result — the clipboard now holds the id.
                ctx.copy_text(id);
                true
            }
            LeftRailEvent::RenameBlock {
                block_id,
                current_title,
            } => {
                // Open the small inline rename dialog seeded with the current title; the dialog confirm
                // spawns the verified PATCH off the UI thread (see `drive_rename` + the dialog render).
                self.rename_error = None;
                self.pending_rename = Some(PendingRename {
                    block_id,
                    text: current_title,
                });
                true
            }
            LeftRailEvent::RouteToStage { document_id, title } => {
                // MT-033 (E5 — route-to-Stage) named context-menu surface: "Route to Stage" on a Document
                // row STAGES the document on the shared MT-031 InteractionBus and dispatches the
                // Route-to-Stage command (registering it idempotently first). `drive_ckc_interop` DRAINS
                // the staged content into the mounted Stage pane next frame and opens it — so the
                // context-menu path produces a visible result, not a silent no-op. The Stage pane displays
                // the document's identity (title + id); deeper Stage capture/embed-back is E10 (MT-066).
                let content =
                    StageContent::Document(crate::rich_editor::save::save_manager::RichDocLoad {
                        rich_document_id: document_id,
                        doc_version: 0,
                        title,
                        content_json: None,
                        updated_at: None,
                    });
                let bus = crate::interop::InteractionBus::get_or_init(ctx);
                crate::interop::InteractionBus::with_try_lock(&bus, |bus| {
                    bus.register_route_to_stage_command();
                    bus.route_to_stage(ctx, content)
                })
                .unwrap_or(false)
            }
            LeftRailEvent::OpenModuleTab(pane_type) => {
                self.open_content_on_active_pane(pane_type, None)
            }
            LeftRailEvent::FocusPaneTab { pane_id, tab_index } => {
                self.active_pane = Some(pane_id.clone());
                if let Some(bar) = self.tab_bar_states.get_mut(&pane_id) {
                    bar.activate(tab_index);
                    return true;
                }
                false
            }
            LeftRailEvent::ToggleStash => {
                self.bottom_drawer_open = !self.bottom_drawer_open;
                true
            }
            LeftRailEvent::ToggleRail => {
                self.left_rail_open = !self.left_rail_open;
                true
            }
            LeftRailEvent::RetryProjectTree => {
                if let Some(handle) = self.runtime_handle.clone() {
                    self.left_rail.project_tree.retry(&handle);
                }
                false
            }
        }
    }

    /// MT-020 explorer-row rename driver: drain any delivered rename PATCH result, then render the
    /// small inline rename dialog while a rename is pending.
    ///
    /// Drain: a delivered `Ok(_)` clears the pending rename and triggers a project-tree RELOAD (the
    /// renamed title is owned by the backend; reloading is what makes the new title appear in the tree —
    /// the externally-meaningful result). A delivered `Err(msg)` is surfaced on the dialog status row
    /// and leaves the dialog open so the operator can retry.
    ///
    /// Render: while `pending_rename` is `Some`, a non-foreground `egui::Window` shows a single-line
    /// text edit seeded from the current title plus Rename / Cancel. Confirm spawns the verified PATCH
    /// off the UI thread; Cancel/empty-title closes the dialog with no backend call.
    fn drive_rename(&mut self, ctx: &egui::Context) {
        // ── Drain a delivered PATCH result ───────────────────────────────────────────────────────────
        let delivered = self
            .rename_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = delivered {
            match result {
                Ok(_new_title) => {
                    self.pending_rename = None;
                    self.rename_error = None;
                    // Reload the tree so the backend-owned new title shows (the visible result).
                    if let Some(handle) = self.runtime_handle.clone() {
                        self.left_rail.project_tree.retry(&handle);
                    }
                    ctx.request_repaint();
                }
                Err(msg) => {
                    self.rename_error = Some(msg);
                    ctx.request_repaint();
                }
            }
        }

        // ── Render the rename dialog while a rename is pending ────────────────────────────────────────
        let Some(mut pending) = self.pending_rename.take() else {
            return;
        };
        // Source the error color from the active theme palette (theme-hygiene gate: no hardcoded
        // Color32 outside the theme module).
        let error_color = self.current_theme.palette().error_text;
        let mut keep_open = true;
        let mut confirm = false;
        egui::Window::new("Rename")
            .id(egui::Id::new("explorer-rename-dialog"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("New title:");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut pending.text)
                        .id(egui::Id::new("explorer-rename-field"))
                        .desired_width(240.0),
                );
                // Enter in the field confirms (keyboard parity).
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirm = true;
                }
                if let Some(err) = &self.rename_error {
                    ui.colored_label(error_color, format!("Rename failed: {err}"));
                }
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        keep_open = false;
                    }
                });
            });

        if confirm {
            let new_title = pending.text.trim().to_owned();
            // An empty title is rejected (no backend call); keep the dialog open with a hint.
            if new_title.is_empty() {
                self.rename_error = Some("Title cannot be empty".to_owned());
                self.pending_rename = Some(pending);
                return;
            }
            let workspace_id = self
                .left_rail
                .project_tree
                .workspace_id()
                .map(|s| s.to_owned());
            match (self.loom_block_client.clone(), workspace_id) {
                (Some(client), Some(ws)) => {
                    self.rename_error = None;
                    client.rename_block(
                        &ws,
                        &pending.block_id,
                        &new_title,
                        self.rename_cell.clone(),
                    );
                    // Keep the dialog open until the delivered result clears it (or surfaces an error),
                    // so a failed PATCH does not silently lose the operator's edit.
                    self.pending_rename = Some(pending);
                }
                _ => {
                    // No runtime/client or no workspace: rename is a disclosed no-op (the headless/test
                    // shell), surfaced on the status row rather than silently dropped.
                    self.rename_error =
                        Some("Rename unavailable (no backend runtime/workspace)".to_owned());
                    self.pending_rename = Some(pending);
                }
            }
        } else if keep_open {
            // Still editing: keep the buffer.
            self.pending_rename = Some(pending);
        } else {
            // Cancel: drop the pending rename + clear any error.
            self.rename_error = None;
        }
    }

    /// WP-KERNEL-012 MT-064 (E9 — FEMS memory-write proposal): render the open "Propose to Memory" dialog
    /// while `pending_memory_proposal` is `Some`. The dialog (built by `fems::memory_proposal`) lets the
    /// operator pick the memory class, previews the selected content + its loom content_hash, and confirms
    /// or cancels. Rendered as a non-foreground `egui::Window` (HBR-QUIET — never an OS popup that steals
    /// focus), the same observable, non-intrusive pattern `drive_rename` uses.
    ///
    /// Confirm: the live focus→selection→confirm→submit POST + FR-EVT-MEM-001 emit lands at E11/MT-069, AND
    /// the proposal WRITE endpoint is absent in this build — so confirming does NOT silently no-op and does
    /// NOT write memory directly; it surfaces the typed blocker on the status row (the honest live result)
    /// and closes the dialog. Cancel closes it with no write. A stale `memory_proposal_status` (e.g. the
    /// no-selection note) is also rendered when no dialog is open so the dispatch result is never lost.
    fn drive_propose_to_memory(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.pending_memory_proposal.take() else {
            // No dialog open: surface a pending status note (e.g. "select text first" / the typed blocker
            // from the last confirm) in a small non-foreground window with an OK dismiss. Clicking OK
            // clears the note; otherwise it persists so the operator/agent always sees the dispatch result.
            if let Some(note) = self.memory_proposal_status.clone() {
                let palette = self.current_theme.palette();
                let mut dismissed = false;
                egui::Window::new("Propose to Memory")
                    .id(egui::Id::new("fems-propose-status"))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 48.0))
                    .show(ctx, |ui| {
                        ui.colored_label(palette.text_subtle, &note);
                        if ui.button("OK").clicked() {
                            dismissed = true;
                        }
                    });
                if dismissed {
                    self.memory_proposal_status = None;
                }
            }
            return;
        };

        let palette = self.current_theme.palette();
        let status_note = self.memory_proposal_status.clone();
        let mut closed = false;
        let mut outcome = crate::fems::memory_proposal::ProposeDialogOutcome::Pending;
        egui::Window::new("Propose to Memory")
            .id(egui::Id::new("fems-propose-to-memory-dialog"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                outcome = dialog.show(ui, &palette);
                if let Some(status) = &status_note {
                    ui.separator();
                    ui.colored_label(palette.error_text, status);
                }
            });

        match outcome {
            crate::fems::memory_proposal::ProposeDialogOutcome::Confirmed(_proposal) => {
                // The proposal WRITE endpoint is absent in this build and the live off-thread submit lands
                // at E11/MT-069. We do NOT fabricate a submit and NEVER write memory directly: surface the
                // typed blocker as the honest live result and close the dialog.
                let probed = crate::fems::memory_proposal::proposal_path(
                    &dialog.proposal.source.workspace_id,
                );
                self.memory_proposal_status = Some(format!(
                    "Proposal endpoint not present in this build (probed {probed}); live submit lands at \
                     E11/MT-069. Nothing was committed — the editor never writes memory directly."
                ));
                closed = true;
            }
            crate::fems::memory_proposal::ProposeDialogOutcome::Cancelled => {
                self.memory_proposal_status = None;
                closed = true;
            }
            crate::fems::memory_proposal::ProposeDialogOutcome::Pending => {}
        }

        if !closed {
            self.pending_memory_proposal = Some(dialog);
        }
    }

    /// A canvas placement at the FRONT of the board gets this `z_index` (top of the stack); the BACK gets
    /// [`CANVAS_Z_BACK`]. The live canvas-board host (future content WP) refines these to the actual
    /// `max+1`/`min-1` of the loaded board; these sentinels are the V1 front/back the menu sends so the
    /// PATCH is real and the ordering is correct relative to a freshly seeded board.
    const CANVAS_Z_FRONT: i32 = 1_000_000;
    const CANVAS_Z_BACK: i32 = -1_000_000;

    /// Drain any delivered SCM write/read result into the panel display/error state (HBR-QUIET: the
    /// network ran off the UI thread; this just reads the cells). Called per-frame. A delivered diff/blame
    /// `Ok(text)` becomes `scm_display_text`; an `Err` becomes `scm_error`.
    fn drive_source_control(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.scm_receipt_cell.lock().ok().and_then(|mut s| s.take()) {
            match result {
                Ok(()) => self.scm_error = None,
                Err(msg) => self.scm_error = Some(msg),
            }
            ctx.request_repaint();
        }
        if let Some(result) = self.scm_text_cell.lock().ok().and_then(|mut s| s.take()) {
            match result {
                Ok(text) => {
                    self.scm_display_text = Some(text);
                    self.scm_error = None;
                }
                Err(msg) => self.scm_error = Some(msg),
            }
            ctx.request_repaint();
        }
    }

    /// Dispatch a confirmed source-control row menu event to the verified backend off the UI thread
    /// (MT-021 MAJOR #1/#3 — the `source_control_client` is genuinely CONSUMED here). `repo_path` is the
    /// git repo root the live SCM panel host (future content WP) supplies. Returns `true` if a backend
    /// call was dispatched (so the caller can repaint). `CopyPath` is handled by the caller (clipboard,
    /// no backend). A `None` client (headless/no-runtime) surfaces a disclosed error instead of panicking.
    pub fn apply_source_control_event(
        &mut self,
        event: crate::source_control::SourceControlEvent,
        repo_path: &str,
    ) -> bool {
        use crate::backend_client::ScmWriteOp;
        use crate::source_control::SourceControlEvent as E;
        let Some(client) = self.source_control_client.clone() else {
            self.scm_error = Some("Source control unavailable (no backend runtime)".to_owned());
            return false;
        };
        match event {
            E::Stage { path } => {
                self.scm_error = None;
                client.stage_paths(
                    ScmWriteOp::Stage,
                    repo_path,
                    &path,
                    self.scm_receipt_cell.clone(),
                );
                true
            }
            E::Unstage { path } => {
                self.scm_error = None;
                client.stage_paths(
                    ScmWriteOp::Unstage,
                    repo_path,
                    &path,
                    self.scm_receipt_cell.clone(),
                );
                true
            }
            E::Diff { path, scope } => {
                self.scm_error = None;
                // `scope` is already the verified `backend_client::ScmDiffScope` the event carries.
                client.diff(repo_path, &path, scope, self.scm_text_cell.clone());
                true
            }
            E::Blame { path } => {
                self.scm_error = None;
                client.blame(repo_path, &path, self.scm_text_cell.clone());
                true
            }
            E::CopyPath { .. } => false, // clipboard is the caller's job; no backend call.
        }
    }

    /// Drain any delivered canvas mutation result into the canvas error state (HBR-QUIET). Per-frame.
    fn drive_canvas(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.canvas_op_cell.lock().ok().and_then(|mut s| s.take()) {
            match result {
                Ok(()) => self.canvas_error = None,
                Err(msg) => self.canvas_error = Some(msg),
            }
            ctx.request_repaint();
        }
    }

    /// Dispatch a confirmed canvas-node menu event to the verified backend off the UI thread (MT-021
    /// MAJOR #1/#3 — the `canvas_client` is genuinely CONSUMED here). `workspace_id` is the active
    /// workspace the live canvas host supplies. `MoveToFront`/`MoveToBack` PATCH the placement `z_index`
    /// (front/back sentinels); `Remove` DELETEs the placement (NOT the block); `RemoveEdges` DELETEs the
    /// placement's visual-only edges (`visual_edge_ids`, never a semantic Loom edge — red-team control).
    /// `OpenBlock`/`EditCard`/`CopyBlockId` are local UI actions handled by the caller (no backend).
    /// Returns `true` if a backend call was dispatched.
    pub fn apply_canvas_event(
        &mut self,
        event: crate::canvas_board::CanvasBoardEvent,
        workspace_id: &str,
        visual_edge_ids: &[String],
    ) -> bool {
        use crate::canvas_board::CanvasBoardEvent as E;
        let Some(client) = self.canvas_client.clone() else {
            self.canvas_error = Some("Canvas backend unavailable (no runtime)".to_owned());
            return false;
        };
        match event {
            E::MoveToFront { placement_id } => {
                self.canvas_error = None;
                client.set_z_index(
                    workspace_id,
                    &placement_id,
                    Self::CANVAS_Z_FRONT,
                    self.canvas_op_cell.clone(),
                );
                true
            }
            E::MoveToBack { placement_id } => {
                self.canvas_error = None;
                client.set_z_index(
                    workspace_id,
                    &placement_id,
                    Self::CANVAS_Z_BACK,
                    self.canvas_op_cell.clone(),
                );
                true
            }
            E::Remove { placement_id } => {
                self.canvas_error = None;
                client.remove_placement(workspace_id, &placement_id, self.canvas_op_cell.clone());
                true
            }
            E::RemoveEdges { placement_id: _ } => {
                self.canvas_error = None;
                // Remove only the VISUAL-only edges the live board already loaded for this placement.
                // Each is DELETEd via the verified canvas-visual-edges endpoint; a semantic Loom edge is
                // NEVER passed here (the caller enumerates `visual_edges` only — red-team control).
                let mut dispatched = false;
                for edge_id in visual_edge_ids {
                    client.remove_visual_edge(workspace_id, edge_id, self.canvas_op_cell.clone());
                    dispatched = true;
                }
                dispatched
            }
            // Local UI actions (open a tab / inline edit / clipboard): no backend call here.
            E::OpenBlock { .. } | E::EditCard { .. } | E::CopyBlockId { .. } => false,
        }
    }

    /// Drain any delivered Loom-node flag PATCH (pin/favorite) result into the flag error state. Per-frame.
    fn drive_loom_node(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.loom_flag_cell.lock().ok().and_then(|mut s| s.take()) {
            match result {
                Ok(()) => self.loom_flag_error = None,
                Err(msg) => self.loom_flag_error = Some(msg),
            }
            ctx.request_repaint();
        }
    }

    /// WP-KERNEL-012 MT-033 (E5 — CKC embeds / drag-in + route-to-Stage): the live shell wiring that makes
    /// the MT-033 melt-together surfaces REACHABLE in the running product.
    ///
    /// 1. **Route-to-Stage drain.** Pull any content a Route-to-Stage dispatch staged on the shared MT-031
    ///    [`crate::interop::InteractionBus`] (`take_pending_stage_content`) into the mounted [`StagePane`]
    ///    and OPEN the Stage panel, so the palette/context-menu "Route to Stage" command produces a visible
    ///    result rather than a silent no-op. This is the exact per-frame drain the AC-4 kittest simulates in
    ///    its `build_ui` closure — here it runs in the real shell. A non-blocking `with_try_lock` keeps the
    ///    egui frame thread free under concurrent agent activity (HBR-SWARM).
    /// 2. **Atelier/CKC drag source.** Render the [`AtelierSidePanel`] on the right edge so its
    ///    `dnd_drag_source` item rows are reachable in the live app (the drop targets the rich-editor /
    ///    canvas add this MT consume from it).
    /// 3. **Stage pane.** Render the Stage pane as a bottom panel when open, displaying the routed content.
    ///
    /// WP-KERNEL-012 MT-079 (E11 host-mount): drain the MOUNTED editors' command + event channels and
    /// route them through the EXISTING shell paths each frame (AC-079-3 / AC-079-5). Two drains:
    ///
    /// 1. **Code command channel** (AC-079-3). The mounted code pane's keymap routes Save / Undo / Redo /
    ///    OpenCommandPalette to the shell via a `Sender<CodeEditorAction>`; here the shell drains the
    ///    receiver and dispatches each:
    ///    - `Save` -> staged on the WP-011 command bus (`request_save` intent) — the document shell owns
    ///      the actual write; the editor never writes files directly.
    ///    - `OpenCommandPalette` -> opens the ONE WP-011 command palette (not a second one).
    ///    - `Undo` / `Redo` -> the MT-035 unified-undo stack on the shared `InteractionBus` for the
    ///      FOCUSED pane, so menu/keyboard undo share ONE stack (the focus owner is the code pane while it
    ///      holds focus; the bus arbitrates).
    ///
    /// 2. **Rich pane event queue** (AC-079-5). The mounted Notes pane drained its `pending_events` into
    ///    the shared `RichPaneEvents` queue; here the shell routes each to the MT-030 navigation bus:
    ///    `WikilinkActivated` / `BacklinkActivated` -> open the document/block; `TagActivated` -> open the
    ///    tag hub. No event is left unrouted.
    ///
    /// A snapshot-capture pass must NOT consume either channel (the real frame owns the drain), so both
    /// are skipped while `capturing_snapshot`. All bus access is via `with_try_lock` so it never blocks
    /// the egui frame thread (HBR-QUIET).
    fn drive_editor_mounts(&mut self, ctx: &egui::Context) {
        if self.capturing_snapshot {
            return;
        }

        // ── 1. Code command channel (Save / Undo / Redo / OpenCommandPalette) ───────────────────────
        let mut commands = Vec::new();
        while let Ok(action) = self.editor_mounts.command_rx.try_recv() {
            commands.push(action);
        }
        if !commands.is_empty() {
            let bus = crate::interop::InteractionBus::get_or_init(ctx);
            // WP-KERNEL-012 MT-080 (AC-080-4): thread the app runtime handle into the unified-undo bus +
            // (idempotently) register the undo command set, so the MT-035 unified-undo stack the mounted
            // code pane's Ctrl+Z/Ctrl+Y route through has its runtime + commands wired. Done lazily here
            // (when a code command actually arrives) rather than every frame.
            //
            // Honest scope note (RISK-080-2 / MC-080-5): this wires the UNDO half of AC-080-4 (set_undo_
            // runtime + register_undo_commands + Ctrl+Z/Ctrl+Y -> bus.undo/redo, proven below). The FR-emit
            // half — `crate::event_emitter::*::emit_code_edit` (debounced 2s) and
            // `crate::interop::render_undo_count_indicator` — is NOT wired into this live loop this run:
            // `emit_code_edit` has no live call site (its helper is unit-proven only; see its DEFERRED-live-
            // wiring docstring) and the undo-count indicator is rendered only by the test bin. Both are
            // explicit deferred typed carries (NOT faked, NOT a live-loop fire) so the residual E11 FR-emit
            // work stays precisely scoped — the host does NOT claim emit_code_edit fires here.
            if let Some(rt) = self.runtime_handle.clone() {
                crate::interop::InteractionBus::with_try_lock(&bus, |b| {
                    b.set_undo_runtime(rt);
                    b.register_undo_commands();
                });
            }
            for action in commands {
                match action {
                    CodeEditorAction::OpenCommandPalette => self.open_command_palette(),
                    CodeEditorAction::Save => {
                        // The save intent reached the shell (observably — `last_editor_command`), which
                        // is the AC-079-3 requirement: Save dispatches to the shell command bus rather
                        // than vanishing in the editor. The actual document WRITE is owned by the
                        // document shell's save path (the rich editor's MT-020 SaveManager / the code
                        // document shell), which is the host-mount typed carry for a follow-on run — NOT
                        // faked here. Recording the command keeps the dispatch perceivable to the
                        // operator + a swarm agent (no silent no-op).
                        self.last_editor_command = Some(CodeEditorAction::Save);
                    }
                    CodeEditorAction::Undo => {
                        crate::interop::InteractionBus::with_try_lock(&bus, |b| {
                            if let Some(pane_id) = b.focus_owner().cloned() {
                                let _ = b.undo(&pane_id);
                            } else {
                                let _ = b.undo_cross_pane();
                            }
                        });
                        ctx.request_repaint();
                    }
                    CodeEditorAction::Redo => {
                        crate::interop::InteractionBus::with_try_lock(&bus, |b| {
                            if let Some(pane_id) = b.focus_owner().cloned() {
                                let _ = b.redo(&pane_id);
                            } else {
                                let _ = b.redo_cross_pane();
                            }
                        });
                        ctx.request_repaint();
                    }
                    // Any other dispatched action is handled in-process by the editor; the shell channel
                    // only carries the four host-routed intents above. A non-host action arriving here is
                    // a benign no-op (it never reaches the channel in normal flow).
                    _ => {}
                }
            }
        }

        // ── 2. Rich pane event queue (Wikilink / Backlink / Tag activated) ──────────────────────────
        let events = self.editor_mounts.rich_events.take();
        for event in events {
            use crate::rich_editor::wikilinks::inline_view::EditorEvent;
            match event {
                EditorEvent::WikilinkActivated {
                    ref_kind,
                    ref_value,
                    ..
                } => {
                    // Route through the MT-030 ShellNavigator: a note/document target opens the rich
                    // editor; any other ref kind opens as a Loom block reference (the same routing the
                    // search panes use). The editor enqueues even an unresolved link so the shell can
                    // surface a status rather than silently no-op (the `pending_events` contract).
                    self.nav_pending_label = Some(ref_value.clone());
                    let outcome = match ref_kind.as_str() {
                        "note" | "file" | "doc" | "document" => {
                            crate::quick_switcher::ShellNavigator::open_document(self, &ref_value)
                        }
                        _ => {
                            crate::quick_switcher::ShellNavigator::open_loom_block(self, &ref_value)
                        }
                    };
                    self.nav_pending_label = None;
                    self.surface_nav_outcome(&outcome);
                    ctx.request_repaint();
                }
                EditorEvent::BacklinkActivated { source_document_id } => {
                    self.nav_pending_label = Some(source_document_id.clone());
                    let outcome = crate::quick_switcher::ShellNavigator::open_document(
                        self,
                        &source_document_id,
                    );
                    self.nav_pending_label = None;
                    self.surface_nav_outcome(&outcome);
                    ctx.request_repaint();
                }
                EditorEvent::TransclusionOpenRequested { ref_value } => {
                    self.nav_pending_label = Some(ref_value.clone());
                    let outcome =
                        crate::quick_switcher::ShellNavigator::open_loom_block(self, &ref_value);
                    self.nav_pending_label = None;
                    self.surface_nav_outcome(&outcome);
                    ctx.request_repaint();
                }
                EditorEvent::TagActivated { canonical, .. } => {
                    // The MT-023 tag hub for the tag is a first-class tag_hub Loom block; open it as a
                    // Loom block reference on the active pane (the same open path the tags panel uses).
                    self.open_content_on_active_pane(PaneType::LoomBlock, Some(canonical));
                    ctx.request_repaint();
                }
                EditorEvent::CreateNote { .. } => {
                    // Create-from-unresolved-link is owned by the editor's own async intent handler
                    // (MT-057 dispatch_create_note); the shell does not double-handle it here.
                }
            }
        }

        // ── WP-KERNEL-012 MT-080: drain the SECONDARY mounted panes' outbound event queues ────────────
        self.drive_secondary_mounts(ctx);
    }

    /// WP-KERNEL-012 MT-080 (E11 host-mount, part 2): drain the SECONDARY mounted panes' outbound event
    /// queues each frame and route each through the EXISTING shell paths. Mirrors `drive_editor_mounts`'s
    /// drain discipline (drained AFTER the pane host so a gesture handled THIS frame is routed THIS frame;
    /// skipped during a snapshot-capture pass so the throwaway capture context stays side-effect-free).
    ///
    /// The five live routes:
    /// 1. **Canvas events** (AC-080-2 / MT-061). A `ResizePlacement`/`AssignSection` maps to the EXISTING
    ///    `CanvasBoardClient` PATCH (resize / group / clear-group); the host dispatches it + re-fetches the
    ///    board. An `EditTextCard`/`TextCardEditBlocked` has no bindable persistence route, so it stays the
    ///    honest typed blocker (no fake write). The live PATCH round-trip is `NEEDS_MANAGED_RESOURCE_PROOF`
    ///    (needs a live PG board) — the host wiring is proven; the DB write is gated.
    /// 2. **Graph depth** (AC-080-3 / MT-060). A `GraphEvent::DepthChanged { depth }` re-fires the
    ///    depth-parameterized `graph-search` (`fetch_local_with_depth`) carrying the NEW `backlink_depth`;
    ///    `OpenNode`/`SelectNode` route to the active pane open path.
    /// 3. **Outgoing-links nav** (AC-080-5 / MT-062). A clicked link routes to the MT-030 nav bus.
    /// 4. **Relevant-memory nav** (AC-080-5 / MT-063). A "Go to source" click routes to the nav bus; the
    ///    FEMS read route is ABSENT, so the panel shows the `EndpointMissing` empty-state (surfaced once).
    /// 5. **Daily-journal date** (AC-080-5 / MT-067). A `DateNavigated` maps to `open_or_create_daily_note`.
    fn drive_secondary_mounts(&mut self, ctx: &egui::Context) {
        if self.capturing_snapshot {
            return;
        }

        // Drain every queue up-front into owned locals so the per-event routing can take `&mut self`
        // (the queues live behind `Arc<Mutex<_>>`, so the drain holds no borrow of `self`).
        let canvas_events: Vec<crate::graph::canvas_board::CanvasEvent> = self
            .editor_mounts
            .secondary
            .canvas_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        let graph_events: Vec<crate::graph::graph_view::GraphEvent> = self
            .editor_mounts
            .secondary
            .graph_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        let out_nav: Vec<crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget> = self
            .editor_mounts
            .secondary
            .outgoing_nav
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        let mem_nav: Vec<crate::fems::relevant_memory_panel::MemoryNavTarget> = self
            .editor_mounts
            .secondary
            .relevant_memory_nav
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        let journal_events: Vec<crate::graph::daily_journal_panel::DailyJournalEvent> = self
            .editor_mounts
            .secondary
            .daily_journal_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        let stage_embed = self
            .editor_mounts
            .secondary
            .stage_embed_requested
            .swap(false, std::sync::atomic::Ordering::Relaxed);

        // 1. Canvas events -> real PATCH/POST via the MT-026 CanvasBoardClient (live round-trip gated).
        if !canvas_events.is_empty() {
            self.route_canvas_events(canvas_events, ctx);
        }

        // 2. Graph events -> depth re-query + node open.
        if !graph_events.is_empty() {
            self.route_graph_events(graph_events, ctx);
        }

        // 3. Outgoing-links nav targets -> MT-030 nav bus.
        for target in out_nav {
            use crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget;
            let outcome = match target {
                NavTarget::Block { id } => {
                    crate::quick_switcher::ShellNavigator::open_loom_block(self, &id)
                }
                NavTarget::Unresolved { value } => {
                    // The link is never dropped: an unresolved target opens as a Loom block reference so
                    // the shell surfaces a status (the create/resolve path is the editor's own).
                    crate::quick_switcher::ShellNavigator::open_loom_block(self, &value)
                }
            };
            self.surface_nav_outcome(&outcome);
            ctx.request_repaint();
        }

        // 4. Relevant-memory nav -> MT-030 nav bus.
        for target in mem_nav {
            use crate::fems::relevant_memory_panel::MemoryNavTarget;
            let outcome = match target {
                MemoryNavTarget::Document { document_id, .. } => {
                    crate::quick_switcher::ShellNavigator::open_document(self, &document_id)
                }
                MemoryNavTarget::Uri { uri } => {
                    crate::quick_switcher::ShellNavigator::open_loom_block(self, &uri)
                }
                MemoryNavTarget::Event { event_id } => {
                    crate::quick_switcher::ShellNavigator::open_loom_block(self, &event_id)
                }
            };
            self.surface_nav_outcome(&outcome);
            ctx.request_repaint();
        }

        // 5. Daily-journal date selection -> open-or-create the daily note.
        for event in journal_events {
            use crate::graph::daily_journal_panel::DailyJournalEvent;
            if let DailyJournalEvent::DateNavigated(date) = event {
                // open_or_create_daily_note: open the daily note for the selected date on the active pane.
                // The note id is the canonical journal slug for the date (YYYY-MM-DD); the open path resolves
                // it through the same Loom open the journal drawer uses.
                let slug = date.format("%Y-%m-%d").to_string();
                self.open_content_on_active_pane(
                    PaneType::LoomDailyJournal,
                    Some(format!("journal:{slug}")),
                );
                ctx.request_repaint();
            }
            // FocusCalendarEvent stays read-only (the /calendar/events route is the typed blocker — no fake).
        }

        // The Stage embed-back request is the typed-blocker surface (the embed-back HTTP route is ABSENT);
        // record it once as a perceivable repaint rather than a silent no-op or a faked embed.
        if stage_embed {
            ctx.request_repaint();
        }

        // 6. Relevant-memory refresh-for-context (AC-080-5 / MT-063). Subscribe the panel's debounced
        // refresh to the active workspace context: when bound + the context is NEW, the panel marks
        // in-flight and the shell spawns the FEMS read off-thread. The FEMS read route is verified ABSENT
        // in this build, so the fetch resolves to the `EndpointMissing` typed blocker and the panel renders
        // its empty-state banner (the DESIGNED primary path — no backend add, no fake pack). Guarded so the
        // first fetch fires once per active context (the debounce inside `refresh_for_context` skips an
        // unchanged context every frame after).
        if !self.capturing_snapshot {
            if let (false, Some(rt), false) = (
                self.active_project_id.is_empty(),
                self.runtime_handle.clone(),
                self.editor_mounts
                    .secondary
                    .relevant_memory_fetched
                    .load(std::sync::atomic::Ordering::Relaxed),
            ) {
                let mem_ctx = crate::fems::memory_client::MemoryContext::for_workspace(
                    self.active_project_id.clone(),
                );
                let should_fetch = self
                    .editor_mounts
                    .secondary
                    .relevant_memory
                    .lock()
                    .map(|mut p| p.refresh_for_context(mem_ctx.clone()))
                    .unwrap_or(false);
                if should_fetch {
                    self.editor_mounts
                        .secondary
                        .relevant_memory_fetched
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    let panel = Arc::clone(&self.editor_mounts.secondary.relevant_memory);
                    let workspace = self.active_project_id.clone();
                    let repaint = ctx.clone();
                    rt.spawn(async move {
                        let client = crate::fems::memory_client::MemoryClient::production();
                        let result = client.fetch_pack(&workspace, &mem_ctx).await;
                        if let Ok(mut p) = panel.lock() {
                            match result {
                                Ok(pack) => p.set_pack(pack),
                                Err(err) => p.set_blocker(err),
                            }
                        }
                        repaint.request_repaint();
                    });
                }
            }
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): map each drained [`crate::graph::canvas_board::CanvasEvent`]
    /// to the EXISTING `CanvasBoardClient` mutation + re-fetch (the live PATCH round-trip is gated
    /// `NEEDS_MANAGED_RESOURCE_PROOF` — it needs a live PG board). A `ResizePlacement` PATCHes `{w,h}`; an
    /// `AssignSection` PATCHes `{group_id}` (or clears it); both re-fetch the board so the persisted geometry
    /// replaces the optimistic in-flight value. An `EditTextCard`/`TextCardEditBlocked` has NO bindable
    /// persistence route — it stays the honest typed blocker (the canvas card-edit endpoint is absent), never
    /// a faked write. Other events (place/remove/open) route through their existing paths or are no-ops here.
    fn route_canvas_events(
        &mut self,
        events: Vec<crate::graph::canvas_board::CanvasEvent>,
        ctx: &egui::Context,
    ) {
        use crate::graph::canvas_board::CanvasEvent;
        let Some(rt) = self.runtime_handle.clone() else {
            return; // No runtime: a headless shell cannot dispatch off-thread mutations (graceful no-op).
        };
        let client = crate::backend_client::CanvasBoardClient::production(rt);
        let (workspace_id, canvas_block_id) = match self.editor_mounts.secondary.canvas_board.lock()
        {
            Ok(b) => (b.workspace_id.clone(), b.canvas_block_id.clone()),
            Err(_) => return,
        };
        let mut dispatched_any = false;
        for event in events {
            let spec = match event {
                CanvasEvent::ResizePlacement { placement_id, w, h } => {
                    Some(client.resize_request(&workspace_id, &placement_id, w as f64, h as f64))
                }
                CanvasEvent::AssignSection {
                    placement_id,
                    group_id,
                } => Some(match group_id {
                    Some(gid) => client.group_request(&workspace_id, &placement_id, &gid),
                    None => client.clear_group_request(&workspace_id, &placement_id),
                }),
                // The remaining canvas events keep their existing handling (open/select route to the active
                // pane open path; place/remove are owned by their own MT-026 paths). EditTextCard /
                // TextCardEditBlocked have no bindable card-body persistence route -> honest typed blocker,
                // surfaced by the pane, never a faked write here.
                _ => None,
            };
            if let Some(spec) = spec {
                let cell: crate::backend_client::CanvasBoardOpCell =
                    std::sync::Arc::new(std::sync::Mutex::new(None));
                client.dispatch(spec, cell);
                dispatched_any = true;
            }
        }
        if dispatched_any {
            // Re-fetch the board so a 2xx PATCH's persisted geometry/group replaces the optimistic value.
            let cell: crate::backend_client::CanvasBoardCell =
                std::sync::Arc::new(std::sync::Mutex::new(None));
            client.fetch_board(&workspace_id, &canvas_block_id, cell);
            ctx.request_repaint();
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-3 / MT-060): map each drained
    /// [`crate::graph::graph_view::GraphEvent`] to the EXISTING graph paths. A `DepthChanged { depth }`
    /// re-fires the depth-parameterized `graph-search` (`fetch_local_with_depth`) carrying the NEW
    /// `backlink_depth` (NO new endpoint). An `OpenNode`/`SelectNode` opens the block on the active pane.
    ///
    /// Deliver-path honesty (RISK-080-2 / Spec-Realism Gate): this MT does NOT yet have a per-frame
    /// graph-cell drain that calls [`crate::graph::graph_view::LoomGraphView::set_graph`] on the mounted
    /// pane, so the dispatched re-query's result is NOT delivered back into the rendered graph-view state
    /// this run — the live fetch + the re-populate are both gated `NEEDS_MANAGED_RESOURCE_PROOF` and the
    /// deliver loop is an explicit typed carry for a follow-on run. BECAUSE there is no deliver path to
    /// clear it, the host deliberately does NOT set `graph_view.loading = true` here: the widget's loading
    /// overlay requests a repaint every frame while `loading` is true (graph_view.rs render path) on the
    /// contract that "the host clears loading when the fetch resolves" — a contract this run cannot honor,
    /// so animating it would be a perpetual idle-repaint trap. The pane stays idle-neutral until the
    /// deliver path lands; the re-query is still dispatched (the event is consumed, not dropped).
    fn route_graph_events(
        &mut self,
        events: Vec<crate::graph::graph_view::GraphEvent>,
        ctx: &egui::Context,
    ) {
        use crate::graph::graph_view::{GraphEvent, GraphMode};
        for event in events {
            match event {
                GraphEvent::DepthChanged { depth } => {
                    // Re-query the focused block's neighbourhood at the new depth. The focus + title come
                    // from the live graph-view mode (Local); a Global-mode depth change never fires.
                    let focus = self
                        .editor_mounts
                        .secondary
                        .graph_view
                        .lock()
                        .ok()
                        .and_then(|v| match &v.mode {
                            GraphMode::Local { block_id, title } => {
                                Some((v.workspace_id.clone(), block_id.clone(), title.clone()))
                            }
                            GraphMode::Global => None,
                        });
                    if let (Some((ws, block_id, title)), Some(rt)) =
                        (focus, self.runtime_handle.clone())
                    {
                        // Dispatch the depth re-query (the event is CONSUMED, not dropped) BUT do NOT set
                        // `graph_view.loading = true`: there is no per-frame deliver path that calls
                        // `set_graph` to clear it this run (see the fn docstring), and the widget's loading
                        // overlay requests a repaint every frame while `loading` is true on the contract
                        // that the host clears it on resolve. Setting it here with no deliver path would be
                        // a perpetual idle-repaint trap (the MT-015 backlinks-spinner regression class), so
                        // the pane stays idle-neutral. The live fetch + the re-populate are gated
                        // NEEDS_MANAGED_RESOURCE_PROOF; the cell is the gated sink (drained once the deliver
                        // path lands as a follow-on typed carry).
                        let client = crate::backend_client::LoomGraphClient::production(rt);
                        let cell: crate::backend_client::LoomGraphCell =
                            std::sync::Arc::new(std::sync::Mutex::new(None));
                        client.fetch_local_with_depth(&ws, &block_id, &title, depth, cell);
                        ctx.request_repaint();
                    }
                }
                GraphEvent::OpenNode { block_id } | GraphEvent::SelectNode { block_id } => {
                    self.open_content_on_active_pane(PaneType::LoomBlock, Some(block_id));
                    ctx.request_repaint();
                }
                // ModeChanged/Relayout/AddEdge/RemoveEdge keep their existing handling (mode re-fetch /
                // layout reset / edge mutation) owned by the graph-view's own paths; no extra host route.
                _ => {}
            }
        }
    }

    /// WP-KERNEL-012 MT-079: surface a `NavDispatchOutcome` from an editor-event route the same way the
    /// quick-switcher does — clear the nav-status segment on a successful `Opened`, or record the typed
    /// status (e.g. the editor-pane seam) so it is perceivable to the operator + a swarm agent. Reuses
    /// the existing `quick_switcher_nav_status` surface (no second status mechanism).
    fn surface_nav_outcome(&mut self, outcome: &crate::quick_switcher::NavDispatchOutcome) {
        match outcome {
            crate::quick_switcher::NavDispatchOutcome::Opened { .. } => {
                self.quick_switcher_nav_status = None;
            }
            other => {
                self.quick_switcher_nav_status = Some(other.status_text());
            }
        }
    }

    /// HBR-QUIET: no foreground window is popped; both surfaces are docked panels drawn inside the frame.
    fn drive_ckc_interop(&mut self, ctx: &egui::Context) {
        // A snapshot-capture pass must NOT consume the bus (the real frame owns the drain), or a routed
        // item would be lost to the throwaway capture context (the `drain_shell_events` guard pattern).
        if !self.capturing_snapshot {
            let bus = crate::interop::InteractionBus::get_or_init(ctx);
            let routed: Option<StageContent> =
                crate::interop::InteractionBus::with_try_lock(&bus, |bus| {
                    bus.take_pending_stage_content()
                })
                .flatten();
            if let Some(content) = routed {
                self.stage_pane.set_content(content);
                self.stage_panel_open = true;
                ctx.request_repaint();
            }
        }

        let palette = self.current_theme.palette();

        // ── Right edge: the Atelier/CKC drag-source side panel (mirrors the left activity rail) ─────────
        if self.atelier_panel_open {
            let panel = &mut self.atelier_side_panel;
            let panel_palette = palette.clone();
            egui::SidePanel::right("atelier-side-panel-host")
                .resizable(true)
                .min_width(200.0)
                .default_width(260.0)
                .show(ctx, |ui| {
                    panel.show(ui, &panel_palette);
                });
        }

        // ── Bottom edge: the Stage pane (route-to-Stage display surface), shown when content is routed ──
        if self.stage_panel_open {
            let stage = &mut self.stage_pane;
            let stage_palette = palette;
            egui::TopBottomPanel::bottom("stage-pane-host")
                .resizable(true)
                .default_height(160.0)
                .show(ctx, |ui| {
                    stage.show(ui, &stage_palette);
                });
        }
    }

    /// Dispatch a confirmed Loom-graph-node menu event (MT-021 MAJOR #2, AC#73). `SetPinned`/`SetFavorite`
    /// PATCH the single flag via the verified `LoomBlockClient::set_flag` (the `loom_block_client` is
    /// CONSUMED for the flag toggle, not only for rename). `Rename` opens the inline rename dialog (reuses
    /// the MT-020 path). `Open`/`OpenToSide`/`CopyBlockId`/`RevealInPanel` are local UI actions handled by
    /// the caller (no backend). Returns `true` if a backend call was dispatched.
    pub fn apply_loom_node_event(
        &mut self,
        event: crate::loom_graph::LoomGraphEvent,
        workspace_id: &str,
    ) -> bool {
        use crate::backend_client::LoomBlockFlag;
        use crate::loom_graph::LoomGraphEvent as E;
        match event {
            E::SetPinned { block_id, target } => {
                let Some(client) = self.loom_block_client.clone() else {
                    self.loom_flag_error =
                        Some("Loom flag update unavailable (no backend runtime)".to_owned());
                    return false;
                };
                self.loom_flag_error = None;
                client.set_flag(
                    workspace_id,
                    &block_id,
                    LoomBlockFlag::Pinned,
                    target,
                    self.loom_flag_cell.clone(),
                );
                true
            }
            E::SetFavorite { block_id, target } => {
                let Some(client) = self.loom_block_client.clone() else {
                    self.loom_flag_error =
                        Some("Loom flag update unavailable (no backend runtime)".to_owned());
                    return false;
                };
                self.loom_flag_error = None;
                client.set_flag(
                    workspace_id,
                    &block_id,
                    LoomBlockFlag::Favorite,
                    target,
                    self.loom_flag_cell.clone(),
                );
                true
            }
            E::Rename {
                block_id,
                current_title,
            } => {
                self.rename_error = None;
                self.pending_rename = Some(PendingRename {
                    block_id,
                    text: current_title,
                });
                true
            }
            // Local UI actions (open a tab / clipboard / focus a pane): no backend call here.
            E::Open { .. }
            | E::OpenToSide { .. }
            | E::CopyBlockId { .. }
            | E::RevealInPanel { .. } => false,
        }
    }

    /// The last delivered SCM diff/blame display text (MT-021), for the live SCM panel host + tests.
    pub fn scm_display_text(&self) -> Option<&str> {
        self.scm_display_text.as_deref()
    }

    /// The last SCM write/read error (MT-021), for the SCM panel status row + tests.
    pub fn scm_error(&self) -> Option<&str> {
        self.scm_error.as_deref()
    }

    /// The last canvas mutation error (MT-021), for the canvas board + tests.
    pub fn canvas_error(&self) -> Option<&str> {
        self.canvas_error.as_deref()
    }

    /// The last Loom-node flag-toggle error (MT-021), for the graph view + tests.
    pub fn loom_flag_error(&self) -> Option<&str> {
        self.loom_flag_error.as_deref()
    }

    /// Open a tab for `pane_type` (carrying optional `content_id`) on the ACTIVE pane (MT-014), the
    /// native equivalent of React `setActiveTabForPane(activePaneId, tab)`. De-duplicates by
    /// `(pane_type, content_id)` (an already-open tab is re-activated, not duplicated) via the
    /// MT-007 `TabBarState::insert_tab`. Returns `true` if a pane was targeted.
    fn open_content_on_active_pane(
        &mut self,
        pane_type: PaneType,
        content_id: Option<String>,
    ) -> bool {
        let Some(target) = self.module_target_pane() else {
            return false;
        };
        let doc_to_reload = if matches!(pane_type, PaneType::LoomWikiPage) {
            content_id.as_ref().filter(|id| !id.is_empty()).cloned()
        } else {
            None
        };
        self.active_pane = Some(target.clone());
        if let Some(bar) = self.tab_bar_states.get_mut(&target) {
            let mut tab = TabState::new(pane_type);
            tab.content_id = content_id;
            bar.insert_tab(tab);
            if let Some(document_id) = doc_to_reload.as_deref() {
                self.invalidate_rich_document_load(document_id);
            }
            return true;
        }
        false
    }

    /// Reset the live work-surface layout to the seeded default for `project_id` (MT-011), the native
    /// mirror of React's `defaultWorkbenchLayoutState(projectId)`. Rebuilds the two notes-first default
    /// panes (re-stamped to `project_id`), the default per-pane tab bars, the default split weights,
    /// clears the active pane and all pop-outs, and points `active_project_id` at the new project.
    ///
    /// This is called on a project switch BEFORE the lifecycle load so a project with NO stored layout
    /// shows its own fresh default — never the leaving project's panes/tabs/splits carried over (the
    /// MT-011 implementation note: "never carry over the old project's open documents"). When the
    /// entered project DOES have a stored layout, the lifecycle load then overwrites this default with
    /// the restored layout; when it does not, this fresh default is what remains.
    fn reset_to_default_layout(&mut self, project_id: &str) {
        self.active_project_id = project_id.to_owned();
        self.split_weights = SplitWeights::default();
        self.active_pane = None;
        self.popout_manager = PopOutManager::new();
        self.tab_bar_states = default_tab_bar_states();
        // A fresh default work surface shows the left rail OPEN and the bottom stash CLOSED (MT-014),
        // mirroring the React `defaultWorkbenchLayoutState` drawers. A subsequent lifecycle load
        // overwrites these if the entered project has stored drawer flags.
        self.left_rail_open = true;
        self.bottom_drawer_open = false;
        // A fresh default work surface starts on the MAIN module (the default code pane's module), so the
        // switcher highlight resets too. A subsequent lifecycle load overwrites this if the entered
        // project has a stored `active_module`.
        self.module_switcher.set_active(ModuleId::Main);
        // Rebuild the registry from the default panes, re-stamped to the entered project so the captured
        // snapshot's pane records are self-consistent with `active_project_id`.
        {
            let mut guard = self
                .pane_registry
                .lock()
                .expect("pane registry mutex poisoned");
            *guard = PaneRegistry::new();
            for mut record in default_panes() {
                record.project_id = project_id.to_owned();
                guard.insert(record);
            }
        }
    }

    /// Switch the shell to `project_id` (MT-011), the native mirror of the React `selectProject()`
    /// (`app/src/App.tsx`). This is the single project-switch transition:
    ///
    /// 1. SAVE the leaving project's current layout NOW (so its split/tabs/pop-outs are persisted
    ///    before the shell shows a different project). The save routes through the same MT-009
    ///    persistence manager (retry / last-known-good) the debounced autosave uses, keyed on the
    ///    CURRENT `active_project_id` — so it must run BEFORE `active_project_id` is changed.
    /// 2. RESET the live layout to the entered project's seeded DEFAULT
    ///    ([`reset_to_default_layout`](Self::reset_to_default_layout)) so the leaving project's
    ///    panes/tabs/splits/pop-outs are never carried over (MT-011 implementation note). This also
    ///    sets `active_project_id` + the tab-strip highlight to the entered project.
    ///
    /// The LOAD of the entered project's layout is performed by the existing per-frame lifecycle
    /// ([`drive_layout_persistence`](Self::drive_layout_persistence)): on the next frame
    /// `loaded_project_id != active_project_id`, so it loads + applies the new project's STORED layout,
    /// overwriting the fresh default from step 2. If the entered project has NO stored layout, the
    /// fresh default from step 2 remains — mirroring React's `defaultWorkbenchLayoutState(projectId)`
    /// fallback. No-op if `project_id` is already active.
    ///
    /// Returns `true` if the switch happened (the id actually changed).
    pub fn switch_project(&mut self, project_id: &str) -> bool {
        if self.active_project_id == project_id {
            return false;
        }
        // 1. Persist the leaving project's layout (keyed on the current active id) before switching.
        //    WP-KERNEL-012 MT-088: this runs OFF the egui UI thread (`spawn_layout_save_now`) — a project
        //    switch is a UI action and the old synchronous `save_layout_now()` `block_on`'d the backend
        //    `PUT` on the UI thread, so a backend-down switch would stall the frame loop (the same freeze
        //    class this MT fixes). The snapshot is captured on the UI thread (it reads live shell state),
        //    then the network `PUT` runs on a short-lived worker; the layout reset below proceeds at once.
        self.spawn_layout_save_now();
        // 2. Reset to the entered project's fresh default + point the shell at it. The next frame's
        //    lifecycle load overwrites this default with the stored layout if one exists (loaded_project_id
        //    now differs from active_project_id), else the fresh default remains.
        self.reset_to_default_layout(project_id);
        self.project_tabs.set_active_id(project_id);
        true
    }

    /// Poll the in-flight `GET /workspaces` fetch (MT-011) and fold the result into the project-tab
    /// strip when it resolves: a successful list replaces the seeded default tab; an error retains the
    /// previous list and surfaces an inline message. Non-blocking (only reads a finished JoinHandle),
    /// so a slow/absent backend never stalls the render loop.
    fn poll_workspaces(&mut self) {
        // MT-027: a snapshot-capture pass must not consume the async workspaces result.
        if self.capturing_snapshot {
            return;
        }
        let finished = self
            .workspaces_handle
            .as_ref()
            .is_some_and(|h| h.is_finished());
        if !finished {
            return;
        }
        if let Some(handle) = self.workspaces_handle.take() {
            match self.rt.block_on(handle) {
                Ok(Ok(projects)) => {
                    self.project_tabs.apply_fetched(projects);
                    // Keep the active highlight + active_project_id consistent if the fetch changed the
                    // active id (e.g. the seeded default project is not in the backend list).
                    self.active_project_id = self.project_tabs.active_id().to_owned();
                }
                Ok(Err(e)) => self.project_tabs.apply_fetch_error(e.to_string()),
                Err(e) => self
                    .project_tabs
                    .apply_fetch_error(format!("join error: {e}")),
            }
        }
    }

    /// Current 2x2 split divider fractions (MT-006). Read by tests / the MT-009 snapshot capture.
    pub fn split_weights(&self) -> SplitWeights {
        self.split_weights
    }

    /// Mutable split weights (for tests that change the layout before capturing a snapshot, and for
    /// future agent/operator split mutation).
    pub fn split_weights_mut(&mut self) -> &mut SplitWeights {
        &mut self.split_weights
    }

    /// Replace the layout persistence manager (tests inject a manager wired with a stub transport so
    /// the full capture -> save -> load -> apply round trip is provable with no live backend).
    /// Production wires the real [`WorkbenchLayoutClient`] via `new`.
    pub fn set_layout_manager(&mut self, manager: LayoutPersistenceManager) {
        self.layout_manager = Arc::new(Mutex::new(manager));
    }

    /// WP-KERNEL-012 MT-088 TEST SEAM (the backend-down re-prove, AC-008-1): re-point the app's
    /// UI-thread-reachable backend interactions — the `/health` poll AND the layout-persistence transport
    /// — at `base_url`, then force a fresh off-thread layout load against it. Used by the backend-down
    /// responsiveness re-prove so it can drive a GENUINELY connection-refusing endpoint (a dead port) even
    /// when a real backend happens to be listening on the hardcoded `127.0.0.1:37501` in the build
    /// environment. This points the REAL production code paths (the real `WorkbenchLayoutClient`
    /// `block_on(GET)` transport + the real `fetch_health`) at a down backend — it does NOT mock the
    /// failure (Spec-Realism, RISK-008-2); the connection is really refused.
    ///
    /// `base_url` is a scheme+host+port like `http://127.0.0.1:1` (a port nothing listens on). The
    /// `/health` URL is derived as `{base_url}/health`. Only meaningful in the production (multi-thread
    /// runtime) shell; a no-op for the layout re-fire in the headless shell with no runtime handle.
    #[doc(hidden)]
    pub fn set_backend_unreachable_for_test(&mut self, base_url: &str) {
        // Re-point the layout manager's transport (the freeze-path `block_on(GET)`/`block_on(PUT)`) at the
        // down base URL. The off-thread load worker uses this manager, so the load now hits the dead port.
        let handle = self
            .runtime_handle
            .clone()
            .unwrap_or_else(|| self.rt.handle().clone());
        let transport =
            crate::backend_client::WorkbenchLayoutClient::new(base_url.to_owned(), handle.clone());
        self.layout_manager = Arc::new(Mutex::new(LayoutPersistenceManager::new(
            Box::new(transport),
            LAYOUT_SAVE_DEBOUNCE,
        )));
        // Force a fresh off-thread load against the down backend on the next frame.
        self.loaded_project_id = None;
        // Preserve an already-observed constructor down edge. Forcing this back to false can manufacture
        // a second BackendUnreachable when the initial production /health probe has already proved the
        // backend is down before this seam repoints the endpoint.
        // Re-fire the `/health` poll at the down backend so the reachability oracle observes it down.
        let health_url = format!("{}/health", base_url.trim_end_matches('/'));
        self.health_status = HealthDisplayState::Loading;
        self.health_next_poll_at = None;
        let task = Self::spawn_health_probe(&handle, health_url, None);
        self.replace_health_handle(task);
    }

    fn abort_and_drain_runtime_task<T>(
        rt: &tokio::runtime::Runtime,
        handle: tokio::task::JoinHandle<T>,
    ) {
        handle.abort();
        let _ = rt.block_on(handle);
    }

    fn replace_health_handle(
        &mut self,
        handle: tokio::task::JoinHandle<Result<HealthInfo, AppError>>,
    ) {
        if let Some(old) = self.health_handle.take() {
            Self::abort_and_drain_runtime_task(&self.rt, old);
        }
        self.health_handle = Some(handle);
    }

    fn shutdown_background_runtime_tasks(&mut self) {
        self.wait_for_layout_workers_to_settle();
        if let Some(handle) = self.health_handle.take() {
            Self::abort_and_drain_runtime_task(&self.rt, handle);
        }
        if let Some(handle) = self.workspaces_handle.take() {
            Self::abort_and_drain_runtime_task(&self.rt, handle);
        }
    }

    fn wait_for_layout_workers_to_settle(&self) {
        let deadline = std::time::Instant::now() + BACKGROUND_WORKER_SHUTDOWN_TIMEOUT;
        while (self
            .layout_load_in_flight
            .load(std::sync::atomic::Ordering::SeqCst)
            || self
                .save_in_flight
                .load(std::sync::atomic::Ordering::SeqCst))
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(BACKGROUND_WORKER_SHUTDOWN_POLL);
        }
        if self
            .layout_load_in_flight
            .load(std::sync::atomic::Ordering::SeqCst)
            || self
                .save_in_flight
                .load(std::sync::atomic::Ordering::SeqCst)
        {
            tracing::warn!(
                "layout background worker still in flight at shutdown; continuing bounded teardown"
            );
        }
    }

    /// Shared handle to the layout persistence manager (tests assert status / call counts; the save
    /// worker clones this to run the flush off the UI thread).
    pub fn layout_manager(&self) -> Arc<Mutex<LayoutPersistenceManager>> {
        self.layout_manager.clone()
    }

    /// The current UI-readable persistence status (HBR: important state is visible). The status bar
    /// can render this so the operator sees Saved / Pending / Error.
    pub fn layout_persistence_status(&self) -> LayoutPersistenceStatus {
        self.layout_manager
            .lock()
            .expect("layout manager mutex poisoned")
            .status()
            .clone()
    }

    /// Signal that a layout-affecting change happened this frame, so the next frame marks the manager
    /// dirty and (re)starts the debounce window. Public so a future pane-header / divider / tab MT can
    /// announce a change, and so tests can drive the save lifecycle directly. The actual `mark_dirty`
    /// + debounced flush happens in [`drive_layout_persistence`](Self::drive_layout_persistence).
    pub fn signal_layout_changed(&mut self) {
        self.layout_dirty_signal = true;
    }

    /// The full all-monitors extent used for the restore-time pop-out clamp.
    pub fn monitor_extent(&self) -> egui::Rect {
        self.monitor_extent
    }

    /// Override the monitor extent (tests set a specific multi-monitor desktop so the restore clamp is
    /// deterministic). Production refreshes it from egui each frame in `ui()`.
    pub fn set_monitor_extent(&mut self, extent: egui::Rect) {
        self.monitor_extent = extent;
    }

    /// Capture the FULL current work-surface layout into a [`LayoutSnapshot`] (MT-009).
    ///
    /// Collects, from the live shell state, every piece the earlier C2 MTs own:
    /// - [`split_weights`](Self) (MT-006),
    /// - [`active_pane`](Self) (MT-006),
    /// - the pane registry records (MT-005),
    /// - per-pane tab-bar state (MT-007),
    /// - pop-out geometry + open flag (MT-008).
    ///
    /// Live `HashMap`s are converted to `BTreeMap` so the snapshot (and its JSON) has deterministic
    /// key order. The pop-out `open` flag reflects the manager's current state; a popped-out pane
    /// merged back this frame (`open == false`) is captured as closed.
    pub fn capture_layout_snapshot(&self) -> LayoutSnapshot {
        let panes: std::collections::BTreeMap<PaneId, PaneRecord> = {
            let guard = self
                .pane_registry
                .lock()
                .expect("pane registry mutex poisoned");
            guard
                .iter()
                .map(|(id, rec)| (id.clone(), rec.clone()))
                .collect()
        };

        let tab_bars: std::collections::BTreeMap<PaneId, TabBarState> = self
            .tab_bar_states
            .iter()
            .map(|(id, bar)| (id.clone(), bar.clone()))
            .collect();

        let pop_outs: std::collections::BTreeMap<PaneId, PopOutSnapshot> = self
            .popout_manager
            .popped_out_ids()
            .into_iter()
            .filter_map(|id| {
                self.popout_manager.get(&id).map(|state| {
                    (
                        id.clone(),
                        PopOutSnapshot {
                            geometry: state.geometry,
                            open: state.open,
                        },
                    )
                })
            })
            .collect();

        LayoutSnapshot::new(
            self.active_project_id.clone(),
            self.split_weights,
            self.active_pane.clone(),
            self.module_switcher.active(),
            panes,
            tab_bars,
            pop_outs,
        )
        .with_drawers(DrawersState {
            project: self.left_rail_open,
            bottom: self.bottom_drawer_open,
        })
    }

    /// Apply a restored [`LayoutSnapshot`] to the live shell (MT-009), the inverse of
    /// [`capture_layout_snapshot`](Self::capture_layout_snapshot).
    ///
    /// `monitor_extent` is the FULL virtual-desktop / all-monitors bounding rect; every restored
    /// pop-out geometry is clamped against it ONCE here (the MT-008-deferred restore clamp), so a
    /// position saved off a now-disconnected monitor reopens at the fallback position instead of
    /// off-screen, while a legitimate second-monitor position is preserved.
    ///
    /// Returns `Err(_)` (without mutating any state) if the snapshot fails validation, so a caller
    /// can fall back to last-known-good / default rather than applying a corrupt layout. Applying is
    /// all-or-nothing: validation happens before any field is written.
    pub fn apply_layout_snapshot(
        &mut self,
        snapshot: LayoutSnapshot,
        monitor_extent: egui::Rect,
    ) -> Result<(), crate::layout_persistence::LayoutError> {
        snapshot.validate()?;
        // Clamp pop-out geometries once against the full desktop extent before applying.
        let snapshot = snapshot.clamp_pop_outs_to(monitor_extent);

        self.active_project_id = snapshot.project_id;
        // Belt-and-suspenders (MT-009): clamp both split axes in-range BEFORE the first render frame so
        // a restored blob with out-of-range weights never shows even a one-frame raw window. The render
        // seam also clamps, but clamping the in-memory weights here closes that one-frame gap.
        self.split_weights = SplitWeights {
            vertical: crate::split_layout::clamp_split(snapshot.split_weights.vertical),
            horizontal: crate::split_layout::clamp_split(snapshot.split_weights.horizontal),
        };
        self.active_pane = snapshot.active_pane;
        // Restore the collapsible-drawer flags (MT-014): the left-rail open flag and the bottom stash
        // drawer flag, so a reopened project shows the rail in the state it was left.
        self.left_rail_open = snapshot.drawers.project;
        self.bottom_drawer_open = snapshot.drawers.bottom;
        // Restore the active MODULE highlight (MT-012) so a reopened project shows the module it was
        // left on, not the default. The pane tab bars are restored from the snapshot below, so we only
        // need to re-point the switcher highlight here.
        self.module_switcher.set_active(snapshot.active_module);

        // Rebuild the registry from the snapshot records (single source of truth). `insert` reassigns
        // stable AccessKit ids, so out-of-process steering keeps working after a restore.
        {
            let mut guard = self
                .pane_registry
                .lock()
                .expect("pane registry mutex poisoned");
            *guard = PaneRegistry::new();
            for (_id, record) in snapshot.panes {
                guard.insert(record);
            }
        }

        // Restore per-pane tab state.
        self.tab_bar_states = snapshot.tab_bars.into_iter().collect();

        // Restore pop-outs: reopen the ones that were open at their (clamped) geometry. A pop-out
        // saved as closed is simply not reopened.
        self.popout_manager = PopOutManager::new();
        for (id, snap) in snapshot.pop_outs {
            if snap.open {
                self.popout_manager.pop_out(id, snap.geometry);
            }
        }

        Ok(())
    }

    /// Persist the current layout for the active project NOW (MT-009), bypassing the debounce window.
    /// Captures a snapshot and routes it through the persistence manager's retry/LKG `save_now` against
    /// the backend's PostgreSQL-authoritative layout endpoint. Used by tests and for an explicit
    /// save-on-exit; the steady-state path is the debounced flush in
    /// [`drive_layout_persistence`](Self::drive_layout_persistence). Blocks until the save attempt(s)
    /// resolve, so it is NOT called on the steady-state UI path.
    pub fn save_layout_now(&self) {
        let snapshot = self.capture_layout_snapshot();
        self.layout_manager
            .lock()
            .expect("layout manager mutex poisoned")
            .save_now(&snapshot);
    }

    /// WP-KERNEL-012 MT-088 (§5.8.5 HARD): persist the current layout NOW but OFF the egui UI thread —
    /// the non-blocking sibling of [`save_layout_now`](Self::save_layout_now), used on the UI-action path
    /// (a project switch). The snapshot is captured on the UI thread (it reads live shell state), then the
    /// manager's retry/LKG `save_now` (which `block_on`s the backend `PUT`) runs on a short-lived OS
    /// worker thread, exactly like the debounced flush. A backend-down `PUT` therefore blocks only that
    /// throwaway worker (bounded by the MT-088 connect/request timeouts), never the frame loop.
    ///
    /// Best-effort: if a save is already in flight (`save_in_flight`), this coalesces by simply marking
    /// the layout dirty so the debounced flush re-saves the leaving project's final layout — no overlap.
    fn spawn_layout_save_now(&mut self) {
        let snapshot = self.capture_layout_snapshot();
        // If a debounced flush is already running, don't spawn a second overlapping save; mark dirty so
        // the in-flight/next flush captures the leaving project's final state.
        if self
            .save_in_flight
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            self.layout_dirty_signal = true;
            return;
        }
        let manager = self.layout_manager.clone();
        let in_flight = self.save_in_flight.clone();
        // MT-088: wake the UI once when the save worker finishes so a status surface reading the
        // (now-cleared) in-flight flag / manager status updates promptly — without a per-frame poll.
        let wake_ctx = self.frame_ctx.clone();
        std::thread::spawn(move || {
            {
                let mut mgr = manager.lock().expect("layout manager mutex poisoned");
                mgr.save_now(&snapshot);
            }
            in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
            if let Some(ctx) = wake_ctx {
                ctx.request_repaint();
            }
        });
    }

    /// Load and apply the persisted layout for `project_id` (MT-009) SYNCHRONOUSLY, with the documented
    /// fallback chain (delegated to the manager): a valid stored blob is applied; a corrupt/foreign/
    /// wrong-project one falls back to the manager's in-memory last-known-good, then to the seeded default
    /// layout (which is infallible). `monitor_extent` is the full all-monitors rect used for the restore
    /// clamp. Returns `true` if a stored snapshot was applied, `false` if the default was kept.
    ///
    /// The manager never returns an unvalidated snapshot, so `apply_layout_snapshot` here is always
    /// applied to a validated layout — no infinite restore loop. Marks `loaded_project_id` so the
    /// lifecycle does not reload the same project every frame.
    ///
    /// WP-KERNEL-012 MT-088: this path runs the transport `GET` to completion in the caller's thread (the
    /// manager's `load` `block_on`s internally). It is therefore only safe to call OFF the egui UI thread
    /// — tests use it directly with a stub/Null transport (instant, no network), and the steady-state
    /// frame path NO LONGER calls it. The live per-frame lifecycle uses [`spawn_layout_load`] +
    /// [`poll_layout_load`] so a backend-down `GET` can never stall the frame loop (the freeze fix).
    pub fn load_layout(&mut self, project_id: &str, monitor_extent: egui::Rect) -> bool {
        let (loaded, reachable) = {
            let mut mgr = self
                .layout_manager
                .lock()
                .expect("layout manager mutex poisoned");
            let loaded = mgr.load(project_id);
            // The manager SWALLOWS a transport error into Ok(fallback) + an `Error` STATUS (so a corrupt/
            // unreachable load never applies garbage). The reachability signal is therefore the manager
            // STATUS, not the `Result` (which is always Ok). An `Error` status == the backend was
            // unreachable for this load; any other status == reachable.
            let reachable = !matches!(mgr.status(), LayoutPersistenceStatus::Error { .. });
            (loaded, reachable)
        };
        self.loaded_project_id = Some(project_id.to_owned());
        // Fold the transport reachability into the debounced backend-down state (the explicit/test path
        // observes the same edges the live frame path does).
        self.note_backend_reachability(reachable);
        match loaded {
            Ok(Some(snapshot)) => {
                // The manager already validated it; apply (which re-validates + clamps, all-or-nothing).
                self.apply_layout_snapshot(snapshot, monitor_extent).is_ok()
            }
            // No stored layout (first run) or a failed load with no LKG: keep the seeded default.
            Ok(None) | Err(_) => false,
        }
    }

    /// WP-KERNEL-012 MT-088 (D2 internal_diagnostics — backend-down graceful degradation, §5.8.5 HARD):
    /// spawn the project-layout LOAD on a short-lived OS worker thread instead of running it on the egui
    /// UI thread. THIS IS THE FREEZE FIX. The manager's `load` performs a backend `GET` (via the
    /// transport's `block_on`); running that on the UI thread inside `drive_layout_persistence` was the
    /// latent 2026-06-26 stall (a backend-down `GET` hung the whole frame loop). The worker delivers the
    /// `(project_id, result)` into [`layout_load_cell`](Self::layout_load_cell); the UI thread drains it
    /// next frame in [`poll_layout_load`](Self::poll_layout_load) and applies it. Until then the seeded
    /// default layout stays visible (a degraded-but-responsive state, NOT a hang and NOT a spinner).
    ///
    /// `loaded_project_id` is set to `project_id` IMMEDIATELY (before the worker finishes) so the
    /// lifecycle does not spawn a second load for the same project every frame; the in-flight guard is a
    /// second belt-and-suspenders against overlap. This mirrors the established off-thread debounced-SAVE
    /// shape (step 3) exactly — a plain `std::thread::spawn`, the transport's `block_on` valid off-runtime.
    fn spawn_layout_load(&mut self, project_id: &str) {
        // Mark loaded NOW so step 1 does not re-enter and re-spawn every frame while the load is running.
        self.loaded_project_id = Some(project_id.to_owned());
        // Guard against an overlapping load (e.g. a rapid double project switch): if one is already in
        // flight, the newer `active_project_id` will simply trigger a fresh load once it lands.
        if self
            .layout_load_in_flight
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }
        let project = project_id.to_owned();
        let manager = self.layout_manager.clone();
        let cell = self.layout_load_cell.clone();
        let in_flight = self.layout_load_in_flight.clone();
        let operation_handle = crate::diagnostics::global_operation_watchdog().register(
            crate::diagnostics::OperationCode::BackendCall,
            crate::diagnostics::BACKEND_OPERATION_STALL_DEADLINE,
            None,
        );
        // MT-088: the event-driven wake. The worker calls `request_repaint()` ONCE when it has delivered
        // its result, so `poll_layout_load` drains it on the very next frame — no per-frame poll cadence.
        let wake_ctx = self.frame_ctx.clone();
        // A plain OS thread (not a runtime worker): the transport's `block_on` is valid off-runtime, so
        // the network GET runs HERE, never on the egui UI thread. The worker holds the manager lock during
        // the GET (the manager owns the transport + the LKG/status bookkeeping), but that is SAFE because
        // the UI thread NEVER `lock()`s the manager on the frame path — `drive_layout_persistence` uses
        // `try_lock` and SKIPS a frame's dirty/due bookkeeping when the worker holds the lock. So a
        // backend-down GET blocks only THIS throwaway worker (bounded by the MT-088 connect/request
        // timeouts), and the UI keeps painting at full cadence (the freeze fix — both halves: off the UI
        // thread AND the UI thread never waits on the lock the worker holds).
        std::thread::spawn(move || {
            let operation_handle = operation_handle;
            let (result, reachable) = {
                let mut mgr = manager.lock().unwrap_or_else(|p| p.into_inner());
                let result = mgr.load(&project);
                // The manager swallows a transport error into Ok(fallback) + an `Error` STATUS; the
                // `Error` status == the backend was unreachable for this load.
                let reachable = !matches!(
                    mgr.status(),
                    crate::layout_persistence::LayoutPersistenceStatus::Error { .. }
                );
                (result, reachable)
            };
            // Deliver the result for the UI thread to apply next frame. Recover a poisoned cell lock
            // rather than propagating a panic out of the worker.
            let mut slot = match cell.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            *slot = Some((project, reachable, result));
            operation_handle.complete();
            in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
            // EVENT-DRIVEN WAKE (MT-088): the result is delivered; wake the UI exactly once so the next
            // frame drains + applies it (and clears the degraded state on recovery) without any per-frame
            // poll cadence. Thread-safe; a no-op if no context was captured yet (no first frame ran).
            if let Some(ctx) = wake_ctx {
                ctx.request_repaint();
            }
        });
    }

    /// WP-KERNEL-012 MT-088: drain a delivered off-thread layout-load result (from [`spawn_layout_load`])
    /// and apply it on the UI thread. Non-blocking: only reads an already-delivered cell, so it never
    /// stalls the frame. A result for a project that is no longer active (the operator switched again
    /// before the load landed) is DISCARDED rather than applied over the now-current project's layout.
    ///
    /// Folds the transport reachability into the debounced backend-down state and re-baselines change
    /// detection to the just-applied layout (so a restore does not immediately re-save itself). Returns
    /// `true` if a delivered result was consumed this frame (a repaint is then worthwhile).
    fn poll_layout_load(&mut self, monitor_extent: egui::Rect) -> bool {
        // MT-027: a snapshot-capture pass must not consume the async load result (the real frame owns it).
        if self.capturing_snapshot {
            return false;
        }
        let delivered = {
            let mut slot = match self.layout_load_cell.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            slot.take()
        };
        let Some((project, reachable, result)) = delivered else {
            return false;
        };
        // Reachability edge: the worker captured the manager status (Error == backend unreachable).
        self.note_backend_reachability(reachable);
        // Ignore a stale delivery for a since-switched project (do not clobber the current layout).
        if project != self.active_project_id {
            return true;
        }
        // `Ok(Some(snapshot))` is a restored layout to apply (the manager already validated it;
        // `apply_layout_snapshot` re-validates + clamps, all-or-nothing). `Ok(None)` (first run) or `Err`
        // (backend down, no LKG) keeps the seeded default — the degraded-but-responsive state, nothing to
        // apply.
        if let Ok(Some(snapshot)) = result {
            let _ = self.apply_layout_snapshot(snapshot, monitor_extent);
        }
        // Re-baseline change detection to the just-settled layout so the restore is not re-saved.
        self.last_seen_layout = Some(self.capture_layout_snapshot().to_layout_state());
        true
    }

    /// WP-KERNEL-012 MT-088 (D2 internal_diagnostics — §5.8.5): fold one backend-reachability observation
    /// into the DEBOUNCED backend-down state and emit the typed transition events on the EDGES ONLY
    /// (AC-008-3 / RISK-008-4 — never every frame). `reachable == false` on the reachable->unreachable
    /// edge records exactly one [`DiagEventCode::BackendUnreachable`]; `reachable == true` on the
    /// unreachable->reachable edge records exactly one [`DiagEventCode::BackendRecovered`]. A steady
    /// state (no edge) records nothing. The recorded event is a typed-allowlist `DiagEvent` (the backend
    /// port as a numeric counter — NO free text); it lands in the in-process buffer (the Diagnostics
    /// Panel) AND the MT-081 ring (Palmistry) through the OPEN MT-082 `record_with` API.
    fn note_backend_reachability(&mut self, reachable: bool) {
        let was_down = self.backend_down;
        if reachable && was_down {
            // Recovery edge: unreachable -> reachable.
            self.backend_down = false;
            Self::record_backend_event(false);
            tracing::info!("backend recovered (BackendRecovered emitted; degraded state cleared)");
        } else if !reachable && !was_down {
            // Down edge: reachable -> unreachable.
            self.backend_down = true;
            Self::record_backend_event(true);
            tracing::warn!(
                "backend unreachable (BackendUnreachable emitted; surfaces degrade, frame loop stays \
                 responsive — §5.8.5 HARD)"
            );
        }
    }

    /// WP-KERNEL-012 MT-088: record one typed backend transition event through the OPEN MT-082 recorder.
    /// `down == true` records [`DiagEventCode::BackendUnreachable`]; `down == false` records
    /// [`DiagEventCode::BackendRecovered`]. The numeric backend port (37501) rides `counter_a` — there is
    /// NO free-text field (the typed-allowlist invariant, §5.8.3). Timestamp is monotonic-ish wall nanos
    /// (consistent with the other transition markers); the event is for the panel + Palmistry, not the
    /// heartbeat staleness math (which uses the dedicated monotonic heartbeat slot).
    fn record_backend_event(down: bool) {
        // The native backend port (from BACKEND_BASE_URL `http://127.0.0.1:37501`). Numeric only.
        const BACKEND_PORT: u64 = 37501;
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let (code, phase, severity) = if down {
            (
                handshake_diag_ring::DiagEventCode::BackendUnreachable,
                handshake_diag_ring::DiagPhase::Degraded,
                handshake_diag_ring::DiagSeverity::Error,
            )
        } else {
            (
                handshake_diag_ring::DiagEventCode::BackendRecovered,
                handshake_diag_ring::DiagPhase::Recovered,
                handshake_diag_ring::DiagSeverity::Info,
            )
        };
        crate::diagnostics::record_with(
            code,
            phase,
            severity,
            /* thread_id    */ 0,
            /* sequence_id  */ 0,
            /* counter_a    */ BACKEND_PORT,
            /* counter_b    */ 0,
            /* metric_micros*/ 0,
            now_nanos,
        );
    }

    fn spawn_health_probe(
        handle: &tokio::runtime::Handle,
        health_url: String,
        wake_ctx: Option<egui::Context>,
    ) -> tokio::task::JoinHandle<Result<HealthInfo, AppError>> {
        let operation_handle = crate::diagnostics::global_operation_watchdog().register(
            crate::diagnostics::OperationCode::BackendCall,
            crate::diagnostics::BACKEND_OPERATION_STALL_DEADLINE,
            None,
        );
        handle.spawn(async move {
            let result = backend_client::fetch_health(&health_url).await;
            operation_handle.complete();
            if let Some(ctx) = wake_ctx {
                ctx.request_repaint();
            }
            result
        })
    }

    /// WP-KERNEL-012 MT-088: whether the app currently believes the backend is unreachable (the debounced
    /// down state). Drives the degraded/disconnected UI indicator and is asserted by the backend-down
    /// re-prove (AC-008-4). `false` until the first unreachable observation; cleared on recovery.
    pub fn backend_is_down(&self) -> bool {
        self.backend_down
    }

    /// WP-KERNEL-012 MT-088: the live status-bar health-segment label — the EXACT text the global backend
    /// indicator renders AND that its AccessKit `Status` node carries. When the debounced backend-down
    /// state is set this is the explicit, FINITE "Disconnected" indicator (NOT a perpetual spinner, NOT a
    /// hang — AC-008-4 / RISK-008-3); otherwise it reflects the `/health` state. Computed in ONE place so
    /// the rendered label and the AC-008-4 test accessor cannot drift.
    pub fn status_bar_health_text(&self) -> String {
        let text = if self.backend_down {
            "Backend: Disconnected (degraded — UI responsive)".to_owned()
        } else {
            match &self.health_status {
                HealthDisplayState::Loading => "Backend: Loading...".to_owned(),
                HealthDisplayState::Ok(h) => {
                    format!(
                        "Backend: OK (db {}, migration {:?})",
                        h.db_status, h.migration_version
                    )
                }
                HealthDisplayState::Error(e) => format!("Backend: error: {e}"),
            }
        };
        Self::append_stalled_operation_status(text)
    }

    fn append_stalled_operation_status(text: String) -> String {
        let stalled_count = crate::diagnostics::active_stalled_operation_count();
        if stalled_count == 0 {
            text
        } else {
            format!("{text} | Stalled ops: {stalled_count}")
        }
    }

    /// Drive the per-frame layout persistence lifecycle (MT-009 BLOCKER wiring):
    ///
    /// 1. LOAD on first frame / project change: when `active_project_id` differs from the last
    ///    `loaded_project_id`, load + apply that project's layout (resolving the monitor extent for the
    ///    restore clamp).
    /// 2. mark dirty: if a layout-affecting change was signalled this frame
    ///    ([`signal_layout_changed`](Self::signal_layout_changed)), mark the manager dirty (which
    ///    (re)starts the debounce window so rapid drags coalesce).
    /// 3. debounced SAVE: when the debounce quiet period has elapsed and no save is already in flight,
    ///    capture the snapshot on the UI thread and run the manager's retry/LKG flush on a short-lived
    ///    worker thread so the network `PUT` never blocks the egui UI thread (HBR-QUIET). The worker
    ///    bridges to the tokio runtime via the transport's runtime handle.
    ///
    /// `now` is the current instant (the app passes `Instant::now()`; tests pass a controlled clock).
    fn drive_layout_persistence(&mut self, now: std::time::Instant) {
        // MT-027: a snapshot-capture pass must not load/save layout or schedule a debounced PUT (the real
        // frame owns persistence). Skip entirely so capturing the live tree has no persistence side effect.
        if self.capturing_snapshot {
            return;
        }
        // ── 1. Load on first frame / project change (OFF the UI thread — MT-088 freeze fix) ─────────
        // WP-KERNEL-012 MT-088 (§5.8.5 HARD backend-down graceful degradation): the layout LOAD performs
        // a backend `GET` that USED to `block_on` ON the egui UI thread right here — so a backend-down
        // `GET` stalled the whole frame loop (the 2026-06-26 Responding=false / CPU->0 freeze). The load
        // now runs OFF the UI thread (`spawn_layout_load`) and its result is drained + applied next frame
        // (`poll_layout_load`), exactly like the debounced SAVE below. The frame loop NEVER blocks on the
        // network here; until the load lands the seeded default layout stays visible (degraded, not hung).
        //
        // (a) Drain a delivered off-thread load result (non-blocking — only reads an already-set cell).
        let extent = self.monitor_extent;
        self.poll_layout_load(extent);
        // (b) Kick off a load when the active project has no (or a stale) loaded layout and none is in
        //     flight. `spawn_layout_load` marks `loaded_project_id` immediately so this does not re-spawn
        //     every frame; the worker delivers into the cell for (a) next frame.
        if self.loaded_project_id.as_deref() != Some(self.active_project_id.as_str())
            && !self
                .layout_load_in_flight
                .load(std::sync::atomic::Ordering::SeqCst)
        {
            let project = self.active_project_id.clone();
            self.spawn_layout_load(&project);
            // Re-baseline change detection to the CURRENT (default, pre-load) layout so the seeded default
            // shown while the load is in flight is not mistaken for a user change and re-saved.
            self.last_seen_layout = Some(self.capture_layout_snapshot().to_layout_state());
        }

        // ── 2. Detect a layout-affecting change + mark dirty ────────────────────────────────────
        // Change detection compares this frame's captured layout blob to last frame's, catching EVERY
        // layout-affecting mutation (split weight / tab order/active/pin / pop-out / active pane)
        // without instrumenting each call site. An explicit `signal_layout_changed` also forces dirty
        // (so a future call site / a test can announce a change directly).
        let current_layout = self.capture_layout_snapshot().to_layout_state();
        let changed = match &self.last_seen_layout {
            Some(prev) => prev != &current_layout,
            // First settled frame: establish the baseline, do not treat the seed as a change.
            None => false,
        };
        // WP-KERNEL-012 MT-088 (§5.8.5 HARD): use `try_lock` for the per-frame manager bookkeeping. A
        // debounced SAVE worker holds the manager lock during its network `PUT` (the manager's `save_now`
        // `block_on`s while holding `&mut self`); on a backend-down PUT that hold lasts up to the connect/
        // request timeout. If the UI thread `lock()`'d here it would BLOCK on the contended manager for
        // that whole window — the SAME freeze class this MT fixes, just on the save path. `try_lock`
        // makes the UI thread SKIP this frame's dirty/due bookkeeping when a save is mid-flight (it
        // retries next frame); the frame loop never stalls. The save itself already runs off-thread.
        if changed || self.layout_dirty_signal {
            // Only clear the signal once the dirty mark is actually recorded, so a contended frame does
            // not silently drop the change (it is re-detected next frame by change detection or retried).
            if let Ok(mut mgr) = self.layout_manager.try_lock() {
                self.layout_dirty_signal = false;
                mgr.mark_dirty(now);
            }
        }
        self.last_seen_layout = Some(current_layout);

        // ── 3. Debounced save off the UI thread ─────────────────────────────────────────────────
        // `try_lock`: if a save worker holds the manager (mid backend PUT), skip the due-check this frame
        // (retried next frame) rather than block the UI thread on the contended lock.
        let due = match self.layout_manager.try_lock() {
            Ok(mgr) => mgr.due_to_flush(now),
            Err(_) => false,
        };
        if due
            && !self
                .save_in_flight
                .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            // Capture the snapshot on the UI thread (it reads live shell state), then flush on a worker.
            let snapshot = self.capture_layout_snapshot();
            let manager = self.layout_manager.clone();
            let in_flight = self.save_in_flight.clone();
            // MT-088: wake the UI once when the debounced flush finishes (event-driven), so a status
            // surface reading the cleared in-flight flag / manager status updates without a per-frame poll.
            let wake_ctx = self.frame_ctx.clone();
            // A plain OS thread (not a runtime worker): the transport's `block_on` is valid off-runtime,
            // so the network PUT runs here without blocking the egui UI thread. The manager handles
            // retry/LKG/status; the UI thread reads status next frame.
            std::thread::spawn(move || {
                {
                    let mut mgr = manager.lock().expect("layout manager mutex poisoned");
                    mgr.flush_if_due(now, &snapshot);
                }
                in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
                if let Some(ctx) = wake_ctx {
                    ctx.request_repaint();
                }
            });
        }
    }

    /// Drive the app's REAL `popout_manager` exactly as the immediate viewport callback does when the
    /// OS title-bar close button (`ViewportInfo::close_requested`) fires for a detached pop-out: mark
    /// the pane's pop-out `open = false`. The next `ui()` frame's `show_all` drain then removes the
    /// entry, so `is_popped_out(pane)` flips to `false` through the app's own update loop — not a
    /// throwaway manager. Mirrors the `close_requested -> request_close` wiring in
    /// [`PopOutManager::show_all`]; this is the OS-close driver seam (parallel to [`request_pop_out`],
    /// the pop-out driver seam) that lets a test or an out-of-process driver simulate the native close
    /// without a real winit window. Returns `true` if a pop-out existed for `pane_id`.
    ///
    /// [`request_pop_out`]: HandshakeApp::request_pop_out
    pub fn request_os_close(&mut self, pane_id: &PaneId) -> bool {
        self.popout_manager.request_os_close(pane_id)
    }

    /// Register the bundled Inter font when the `bundled-fonts` feature is on; otherwise leave
    /// eframe's default fonts in place. Never panics: a missing asset is a compile-time error
    /// only when the feature is explicitly enabled (the operator opting into bundling).
    ///
    /// Public so the MT-004 font-bundling proof can drive it on a headless `egui::Context` and
    /// assert that Inter is actually registered as the active proportional font (rather than the
    /// fallback). `HandshakeApp::new` calls this on the real eframe context at startup.
    pub fn install_fonts(ctx: &egui::Context) {
        #[cfg(feature = "bundled-fonts")]
        {
            ctx.set_fonts(Self::build_font_definitions());
            tracing::info!(
                "bundled fonts loaded: Inter (Regular+Bold) + Noto CJK/symbol fallback chain \
                 (SC, KR, Symbols2, Math) + RTL/complex-script faces (Hebrew, Arabic, Devanagari — \
                 glyph coverage only; Arabic/Indic cursive shaping is a typed limitation, see \
                 text_intl::bidi); emoji via egui default NotoEmoji"
            );
        }
        #[cfg(not(feature = "bundled-fonts"))]
        {
            let _ = ctx; // default fonts; nothing to do until MT-004 bundles the asset.
            tracing::debug!("bundled-fonts feature off; using eframe default fonts");
        }
    }

    /// Build the bundled-font `FontDefinitions` with the MT-075 ordered Unicode fallback chain.
    ///
    /// Pure (no `egui::Context`, no GPU) so the MT-075 family-order unit test can assert the fallback
    /// ordering deterministically and headlessly. `install_fonts` simply hands the result to
    /// `ctx.set_fonts`. Only compiled under `bundled-fonts` (the default); the no-bundle build keeps
    /// eframe's default fonts and never calls this.
    ///
    /// Fallback contract (RISK-2 / AC1): Inter is inserted at index 0 of BOTH the Proportional and the
    /// Monospace family vecs, so it always wins for Latin/Cyrillic/Greek and the Latin look is
    /// unchanged. The four broad-coverage Noto faces (`FALLBACK_FACE_ORDER`: SC, KR, Symbols2, Math)
    /// are appended AFTER Inter, BEFORE egui's own defaults — so a Han/Kana/Hangul/box-drawing/symbol
    /// codepoint resolves to the first Noto face that has it, and emoji still fall through to egui's
    /// default `NotoEmoji`. The Monospace family gets the SAME fallbacks (AC3: CJK comments / box
    /// drawing in the code editor render). egui renders the single notdef box for a codepoint present
    /// in NO registered font, with no panic and no layout break (AC6) — that is egui's built-in
    /// behavior, preserved here because we never clear the default fonts.
    #[cfg(feature = "bundled-fonts")]
    pub fn build_font_definitions() -> egui::FontDefinitions {
        use egui::{FontData, FontFamily};
        use std::sync::Arc;

        let mut fonts = egui::FontDefinitions::default();

        // 1) Register the primary Inter regular face and the broad-coverage Noto fallback faces in
        //    `font_data`. (Registering a face here does NOT by itself put it in a family — the family
        //    vecs below decide order. `from_static` borrows the `&'static [u8]` include_bytes! data,
        //    so there is no copy.)
        fonts.font_data.insert(
            FONT_KEY_INTER.to_owned(),
            Arc::new(FontData::from_static(INTER_REGULAR)),
        );
        fonts.font_data.insert(
            FONT_KEY_NOTO_SC.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_SC)),
        );
        fonts.font_data.insert(
            FONT_KEY_NOTO_KR.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_KR)),
        );
        fonts.font_data.insert(
            FONT_KEY_NOTO_SYMBOLS2.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_SYMBOLS2)),
        );
        fonts.font_data.insert(
            FONT_KEY_NOTO_MATH.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_MATH)),
        );
        // MT-078: the three RTL/complex-script faces. Registered here; the family loop below adds them to
        // the fallback chain (after the CJK faces) because they are part of FALLBACK_FACE_ORDER.
        fonts.font_data.insert(
            FONT_KEY_NOTO_HEBREW.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_HEBREW)),
        );
        fonts.font_data.insert(
            FONT_KEY_NOTO_ARABIC.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_ARABIC)),
        );
        fonts.font_data.insert(
            FONT_KEY_NOTO_DEVANAGARI.to_owned(),
            Arc::new(FontData::from_static(NOTO_SANS_DEVANAGARI)),
        );

        // 2) Build the ordered fallback chain in BOTH the Proportional and Monospace families:
        //    [Inter] ++ FALLBACK_FACE_ORDER ++ <egui's existing default faces (incl. NotoEmoji)>.
        //    Inter goes to index 0 (front); the Noto faces are inserted right after it (index 1..),
        //    pushing egui's defaults toward the back. This keeps Inter first (MC-1) while ensuring the
        //    Noto faces are consulted BEFORE egui's default proportional/monospace face (which has no
        //    CJK) but the emoji default is still reachable for emoji.
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let vec = fonts.families.entry(family).or_default();
            // Inter first.
            vec.insert(0, FONT_KEY_INTER.to_owned());
            // Broad-coverage faces immediately after Inter, preserving FALLBACK_FACE_ORDER.
            for (offset, face) in FALLBACK_FACE_ORDER.iter().enumerate() {
                vec.insert(1 + offset, (*face).to_owned());
            }
        }

        // 3) Bold face: a NAMED family so callers can opt into bold via FontFamily::Name("Inter-Bold")
        //    without disturbing the default proportional rendering (unchanged from MT-004). The CJK
        //    fallbacks are appended here too so bold CJK text does not tofu.
        fonts.font_data.insert(
            INTER_BOLD_FAMILY.to_owned(),
            Arc::new(FontData::from_static(INTER_BOLD)),
        );
        let bold_vec = fonts
            .families
            .entry(FontFamily::Name(INTER_BOLD_FAMILY.into()))
            .or_default();
        bold_vec.insert(0, INTER_BOLD_FAMILY.to_owned());
        for (offset, face) in FALLBACK_FACE_ORDER.iter().enumerate() {
            bold_vec.insert(1 + offset, (*face).to_owned());
        }

        fonts
    }

    /// Apply the active theme's palette to egui, but only when the theme changed since the
    /// last applied frame (CONTROL-1). Returns whether `apply_to_ctx` actually ran.
    fn apply_theme_if_changed(&mut self, ctx: &egui::Context) -> bool {
        if self.last_applied_theme == Some(self.current_theme) {
            return false;
        }
        // Overrides are empty in MT-003 (loaded from the backend settings API in a later MT).
        let palette = self.current_theme.palette();
        theme::apply_to_ctx(&palette, ctx);
        self.last_applied_theme = Some(self.current_theme);
        true
    }

    /// Render the theme toggle as an egui button with a fixed `egui::Id` (value
    /// `THEME_TOGGLE_NODE_ID`) so its `accesskit::NodeId` is stable for steering. We build
    /// the interactive widget directly (rather than `ui.add(Button::new(..))`) because
    /// `Button` does not expose an id override; this is a real egui interactive widget with
    /// `Role::Button` + `Action::Click`, not a mock.
    fn theme_toggle(&mut self, ui: &mut egui::Ui) {
        let label = match self.current_theme {
            HsTheme::Dark => "Light",
            HsTheme::Light => "Dark",
        };

        // Fixed-value Id -> fixed AccessKit NodeId. A single low-entropy id is safe (entropy
        // only affects IdMap distribution; one fixed widget cannot self-collide).
        let id = unsafe { egui::Id::from_high_entropy_bits(THEME_TOGGLE_NODE_ID) };

        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(14.0),
            ui.visuals().text_color(),
        );
        let padding = ui.spacing().button_padding;
        let desired = galley.size() + padding * 2.0;
        let (_auto, rect) = ui.allocate_space(desired);
        let response = ui.interact(rect, id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
            let text_pos = rect.center() - galley.size() * 0.5;
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }

        // Emit Role::Button + label + Action::Click into the AccessKit tree for this id.
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
        });

        // Attach a stable author_id to the SAME live node egui builds from the widget_info above.
        // `accesskit_node_builder` (via `emit_interactive_node`) only sets `author_id`, leaving the
        // Role::Button + Action::Click + Action::Focus that egui derived from `widget_info`/`Sense`
        // intact — so the toggle becomes addressable by `shell.chrome.theme-toggle` AND still passes
        // the `assert_no_unnamed_interactive` gate (which flags clickable nodes without an author_id).
        accessibility::emit_interactive_node(ui.ctx(), id, THEME_TOGGLE_AUTHOR_ID);

        if response.clicked() {
            self.current_theme = self.current_theme.toggled();
            // Next frame's apply_theme_if_changed picks up the new palette.
            ui.ctx().request_repaint();
        }
    }

    /// Render the MT-012 module switcher in the header, returning the clicked module id (if a non-active
    /// module button was clicked this frame). Splits the borrow so the switcher field is mutated while
    /// the theme palette is read immutably (same pattern the project-tab colors use).
    fn module_switcher_ui(&mut self, ui: &mut egui::Ui) -> Option<ModuleId> {
        let palette = self.current_theme.palette();
        let colors = ModuleSwitcherColors {
            active_bg: palette.accent,
            inactive_bg: palette.surface,
            hover_bg: palette.surface_strong,
            text: palette.text_subtle,
            active_text: palette.bg,
        };
        self.module_switcher.show(ui, colors)
    }

    /// Render the top-bar "Handshake" identity as a real egui widget with the fixed
    /// `ChromeWidget::TitleBar` id, then emit a LIVE AccessKit node (Role::TitleBar +
    /// author_id `shell.chrome.title-bar` + label) into the frame's accessibility tree.
    ///
    /// We allocate via `ui.interact` with the fixed id (rather than `ui.heading`, whose id is
    /// auto-generated) so the node carries a stable `NodeId` AND is registered in egui's parent
    /// map, which is what makes `emit_chrome_node` attach it under the title panel instead of the
    /// root. This is the chrome counterpart to the MT-005 pane emission and closes the MT-002 gap
    /// where the title existed visually but only carried egui's default (author_id-less) node.
    fn title_identity(&self, ui: &mut egui::Ui) {
        let label = "Handshake";
        let chrome = ChromeWidget::TitleBar;
        let id = chrome.egui_id();

        let font = egui::FontId::proportional(20.0); // heading-sized
        let galley = ui
            .painter()
            .layout_no_wrap(label.to_owned(), font, ui.visuals().text_color());
        let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter()
                .galley(rect.min, galley, ui.visuals().text_color());
        }
        // Reserve the fixed id in egui's interaction/parent map so the live node attaches correctly.
        ui.interact(rect, id, egui::Sense::hover());

        accessibility::emit_chrome_node(ui.ctx(), chrome, id, label);
    }

    /// MT-021 Surface 10: render the bottom status bar's health SEGMENT as a right-clickable widget
    /// that opens the status-bar-segment context menu, and dispatch the confirmed action. Reuses the
    /// EXISTING `ChromeWidget::StatusBar` fixed id + `shell.chrome.status-bar` author_id (so the MT-025
    /// default snapshot does not gain a node — the menu is closed by default), but allocates the segment
    /// with `Sense::click()` so it reports `secondary_clicked()` for the menu open. The node stays NAMED
    /// (its author_id), so the MT-025 interactive gate stays green even though it is now clickable.
    ///
    /// Returns the typed menu action confirmed this frame (the caller applies it after the panel closes,
    /// so the menu closure never holds `&mut self`).
    fn status_bar_segment(
        &self,
        ui: &mut egui::Ui,
        text: &str,
    ) -> Option<crate::context_menu_surfaces::StatusBarMenuAction> {
        use crate::context_menu_surfaces::{
            status_bar_action_for_id, status_bar_context_items, StatusBarSegmentState,
        };
        let chrome = ChromeWidget::StatusBar;
        let id = chrome.egui_id();
        let segment_id = "health"; // the live status bar's one segment today (backend health).

        let font = egui::TextStyle::Body.resolve(ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font, ui.visuals().text_color());
        // Allocate with Sense::hover() (NOT click) so the auto-id allocation node is non-interactive;
        // the ONE clickable node is the interact at the FIXED chrome id below (which carries the stable
        // author_id), so the MT-025 interactive-naming gate stays green (no unnamed clickable node).
        let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter()
                .galley(rect.min, galley, ui.visuals().text_color());
        }
        // The addressable, secondary-clickable segment node at the FIXED chrome id (stable author_id).
        let seg_resp = ui.interact(rect, id, egui::Sense::click());
        // Keep the existing chrome node identity (Role::Status + stable author_id + live text).
        accessibility::emit_chrome_node(ui.ctx(), chrome, id, text);

        let state = StatusBarSegmentState {
            segment_id: segment_id.to_owned(),
            segment_label: text.to_owned(),
            visible: !self.statusbar_hidden.contains(segment_id),
            related_panel_name: statusbar_related_panel_name(segment_id),
        };
        let mut action = None;
        let menu = crate::context_menu::ContextMenu::new("statusbar")
            .items(status_bar_context_items(&state));
        if let Some(confirmed_id) = menu.show_on(&seg_resp) {
            action = status_bar_action_for_id(confirmed_id, &state);
        }
        // Shift+F10 keyboard-open parity when the segment is focused.
        if seg_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift)
        {
            crate::context_menu::request_open(ui.ctx(), seg_resp.id, seg_resp.rect.left_bottom());
        }
        action
    }

    /// MT-030: render the quick-switcher NAVIGATION status as a SECOND, persistent status-bar segment
    /// and emit it as a LIVE AccessKit node so the editor-pane typed seam is actually PERCEIVABLE after
    /// the overlay closes — by the operator (a rendered status-bar label) AND by a swarm agent reading
    /// the AccessKit tree (`quick-switcher.nav-status`, `Role::Status`). Before this segment existed the
    /// seam status lived only in a private getter that no UI/AccessKit consumer read, so selecting a
    /// document/symbol hit was an OBSERVABLE silent no-op. Reuses the `ChromeWidget::QuickSwitcherNavStatus`
    /// fixed id + `emit_chrome_node` (the same live-node mechanism as the health segment + title bar).
    ///
    /// No-op (renders nothing, emits no node) when `quick_switcher_nav_status` is `None`, so the default
    /// MT-025 snapshot does not gain a node and a successful `Opened` dispatch leaves the bar clean.
    fn quick_switcher_nav_status_segment(&self, ui: &mut egui::Ui) {
        let Some(text) = self.quick_switcher_nav_status.as_deref() else {
            return;
        };
        let chrome = ChromeWidget::QuickSwitcherNavStatus;
        // Use the theme's subtle text token (no Color32 literal — theme guard) so the seam status is
        // visibly distinct from the primary health text without a hard-coded color.
        let color = self.current_theme.palette().text_subtle;
        // Render a REAL egui `Label` widget: egui creates a genuine live accessibility node for it this
        // frame (Role::Label, the painted text), and its `Response::id` is the node egui keys builders
        // by — so `accesskit_node_builder` for that id deterministically enriches THIS node (not a
        // fallback) with the stable `author_id` + the `Status` role a swarm agent addresses. (A synthetic
        // high-entropy id with no backing widget node attaches the label to the wrong/focused node.)
        let rich = egui::RichText::new(text).color(color);
        let response = ui.add(egui::Label::new(rich).sense(egui::Sense::hover()));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(chrome.role());
            node.set_author_id(chrome.author_id().to_owned());
            node.set_label(text.to_owned());
        });
    }

    /// WP-KERNEL-012 MT-100: render the terminal-launch outcome as a compact status segment. The full
    /// typed request/error stays in `backend_client`; this bar surface is the operator/model-visible
    /// signal that selecting the terminal affordance hit `EndpointMissing`, not a silent no-op.
    fn terminal_launch_status_segment(&self, ui: &mut egui::Ui) {
        let Some(text) = self.terminal_launch_status.as_deref() else {
            return;
        };
        let color = self.current_theme.palette().text_subtle;
        let response = ui.add(egui::Label::new(egui::RichText::new(text).color(color)));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_author_id(TERMINAL_LAUNCH_STATUS_AUTHOR_ID.to_owned());
            node.set_label(text.to_owned());
        });
    }

    /// WP-KERNEL-012 MT-101: compact status-bar mirror of the model-session launch outcome. It must stay
    /// honest: `/jobs` creation is reported as job creation, and direct repo-folder session spawn remains
    /// `EndpointMissing kernel_swarm_spawn_session` until a native bridge exists.
    fn model_session_launch_status_segment(&self, ui: &mut egui::Ui) {
        let Some(text) = self.model_session_launch_status.as_deref() else {
            return;
        };
        let display_text = Self::compact_model_session_status(text);
        let color = self.current_theme.palette().text_subtle;
        let response = ui
            .add(egui::Label::new(
                egui::RichText::new(display_text).color(color),
            ))
            .on_hover_text(text);
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_author_id(MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID.to_owned());
            node.set_label(text.to_owned());
        });
    }

    fn compact_model_session_status(text: &str) -> String {
        if text == MODEL_SESSION_READY_STATUS {
            return "Model session: ready".to_owned();
        }
        if text == MODEL_SESSION_CHOOSE_STATUS {
            return "Model session: choose fields".to_owned();
        }
        if text.contains("POST /jobs pending") {
            return "Model session: /jobs pending".to_owned();
        }
        if text.contains("POST /jobs failed") {
            return "Model session: /jobs failed; see status".to_owned();
        }
        if let Some(rest) = text.strip_prefix("Model session: /jobs job ") {
            let job_id = rest.split_whitespace().next().unwrap_or("created");
            let status = rest
                .split_once("status=")
                .map(|(_, value)| value.split(';').next().unwrap_or("unknown"))
                .unwrap_or("unknown");
            return format!("Model session: /jobs {job_id} {status}; needs runtime proof");
        }
        if text.len() <= 72 {
            return text.to_owned();
        }
        let mut compact: String = text.chars().take(69).collect();
        compact.push_str("...");
        compact
    }

    /// Apply a confirmed status-bar-segment menu action (MT-021). `segment_id` is the right-clicked
    /// segment; `display_text` its current text (for Copy). Returns `true` when app state changed (so
    /// the caller can repaint). `OpenPanel` opens the segment's related pane on the active pane.
    fn apply_status_bar_action(
        &mut self,
        ctx: &egui::Context,
        segment_id: &str,
        display_text: &str,
        action: crate::context_menu_surfaces::StatusBarMenuAction,
    ) -> bool {
        use crate::context_menu_surfaces::StatusBarMenuAction as A;
        match action {
            A::CopySegment => {
                ctx.copy_text(display_text.to_owned());
                false
            }
            A::ToggleVisibility { target } => {
                if target {
                    self.statusbar_hidden.remove(segment_id);
                } else {
                    self.statusbar_hidden.insert(segment_id.to_owned());
                }
                true
            }
            A::OpenPanel => match statusbar_related_pane_type(segment_id) {
                Some(pane_type) => self.open_content_on_active_pane(pane_type, None),
                None => {
                    tracing::warn!(
                        "statusbar.open_panel: segment {segment_id} has no related pane"
                    );
                    false
                }
            },
            A::Refresh => {
                // Re-fetch the segment's data. For the health segment, spawn a fresh /health poll on
                // the runtime (OFF the UI thread — HBR-QUIET), the same fire-once poll the ctor uses.
                if segment_id == "health" {
                    if let Some(handle) = self.runtime_handle.clone() {
                        self.health_status = HealthDisplayState::Loading;
                        // MT-088: wake the UI once when the manual re-fetch resolves (event-driven), so
                        // the Loading indicator clears promptly without relying only on the per-frame
                        // Loading repaint cadence.
                        let wake_ctx = self.frame_ctx.clone();
                        let task =
                            Self::spawn_health_probe(&handle, HEALTH_URL.to_owned(), wake_ctx);
                        self.replace_health_handle(task);
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Whether a status-bar segment is currently hidden (MT-021; tests / a future settings surface).
    pub fn statusbar_segment_hidden(&self, segment_id: &str) -> bool {
        self.statusbar_hidden.contains(segment_id)
    }

    fn poll_health(&mut self) {
        // MT-027: a snapshot-capture pass must not consume the async health result (the real frame owns it).
        if self.capturing_snapshot {
            return;
        }
        let finished = self.health_handle.as_ref().is_some_and(|h| h.is_finished());
        if finished {
            if let Some(handle) = self.health_handle.take() {
                // block_on a finished handle returns immediately (no real blocking).
                self.health_status = match self.rt.block_on(handle) {
                    Ok(Ok(info)) => {
                        tracing::debug!(status = %info.status, db = %info.db_status, migration = ?info.migration_version, "health received");
                        HealthDisplayState::Ok(info)
                    }
                    Ok(Err(e)) => HealthDisplayState::Error(e.to_string()),
                    Err(e) => HealthDisplayState::Error(format!("join error: {e}")),
                };
                // WP-KERNEL-012 MT-088: `/health` is the canonical reachability oracle — fold the just-
                // resolved result into the debounced backend-down state (emits the typed transition events
                // on the edges only). An `Ok` health is reachable; an `Error` (refused/timeout/non-success)
                // is unreachable. Then ARM the next re-probe so a recovered backend is re-observed.
                let reachable = matches!(self.health_status, HealthDisplayState::Ok(_));
                self.note_backend_reachability(reachable);
                self.health_next_poll_at =
                    Some(std::time::Instant::now() + HEALTH_REPROBE_INTERVAL);
            }
        }
        // WP-KERNEL-012 MT-088: fire the next background `/health` re-probe when due and none is in
        // flight. A single off-UI-thread `rt.spawn` (HBR-QUIET — the UI thread never blocks on it); the
        // result is folded above next frame. Only the production shell (with a runtime handle) re-probes.
        if self.health_handle.is_none() {
            let due = self
                .health_next_poll_at
                .is_some_and(|at| std::time::Instant::now() >= at);
            if due {
                if let Some(handle) = self.runtime_handle.clone() {
                    self.health_next_poll_at = None;
                    // MT-088: wake the UI once when the re-probe resolves (event-driven) so `poll_health`
                    // folds the result next frame and `BackendRecovered` fires promptly on a real recovery
                    // — instead of the frame loop polling `health_handle.is_some()` every frame.
                    let wake_ctx = self.frame_ctx.clone();
                    let task = Self::spawn_health_probe(&handle, HEALTH_URL.to_owned(), wake_ctx);
                    self.replace_health_handle(task);
                }
            }
        }
    }

    /// Render the shell. Split from eframe::App::update so egui_kittest can drive it without a Frame.
    pub fn ui(&mut self, ctx: &egui::Context) {
        // WP-KERNEL-012 MT-088: capture a clone of the live context on the first frame so off-thread
        // backend workers (layout load / debounced save / `/health` re-probe) can `request_repaint()` the
        // UI exactly once on completion (event-driven wake) instead of the frame loop polling for delivery
        // every frame. `Context` is `Arc`-backed (cheap clone); the capture is idempotent. Done BEFORE
        // `poll_health` so a `/health` re-probe spawned this frame already has the wake context available.
        if self.frame_ctx.is_none() {
            self.frame_ctx = Some(ctx.clone());
        }
        self.poll_health();
        self.poll_workspaces();
        // Apply a pending theme flip (MT-018, red-team R4/MC4) at the very TOP of the frame, BEFORE any
        // panel renders, so a settings/menu theme change never leaves widgets on mixed visuals for a
        // frame. `apply_theme_if_changed` below then pushes the new palette to egui this same frame.
        if let Some(theme) = self.pending_theme_change.take() {
            self.current_theme = theme;
        }
        // Apply theme tokens at the top of the frame so all panels below render themed.
        self.apply_theme_if_changed(ctx);

        // WP-KERNEL-012 MT-076 (E13 IME / AC6 / RISK-3 / MC-3): enable IME on the OS window ONCE on the
        // first real frame. `ViewportCommand::IMEAllowed(true)` is the egui-side equivalent of winit's
        // `Window::set_ime_allowed(true)` — it tells winit to FORWARD OS composition events as
        // `egui::Event::Ime` (Enabled/Preedit/Commit/Disabled), which the rich + code editors' IME handlers
        // consume. Without it the OS sends no composition events and CJK/Japanese/Korean input is silently
        // dead. Sent once (guarded by `ime_allowed_sent`) rather than every frame. The window handle IS
        // reachable here: MT-079 host-mounted the editors in this same app, so the eframe viewport exists.
        // Skipped during a snapshot-capture pass (the throwaway AccessKit context has no real viewport and
        // must stay side-effect-free); the real frame sends it.
        if !self.ime_allowed_sent && !self.capturing_snapshot {
            ctx.send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));
            self.ime_allowed_sent = true;
        }

        // MT-028: push the live workspace id + palette into the LoomSearchV2 pane's shared cell BEFORE
        // the pane host renders, so the in-product Loom Search pane searches the active workspace and
        // its `<mark>` highlight tracks the current theme. Prefer the project tree's resolved workspace
        // id; fall back to the active project id when the tree has none yet (both are the workspace key
        // the backend `/workspaces/{id}/loom/search-v2` route expects).
        {
            let workspace_id = self
                .left_rail
                .project_tree
                .workspace_id()
                .map(|s| s.to_owned())
                .or_else(|| {
                    let p = self.active_project_id.clone();
                    (!p.is_empty()).then_some(p)
                });
            let palette = self.current_theme.palette();
            if let Ok(mut shared) = self.loom_search_v2_shared.lock() {
                shared.workspace_id = workspace_id.clone();
                shared.palette = palette.clone();
            }
            // MT-029: mirror the same per-frame push into the Find-in-Files pane's shared cell so its
            // search targets the active workspace and its highlight tracks the live theme.
            if let Ok(mut shared) = self.find_in_files_shared.lock() {
                shared.workspace_id = workspace_id.clone();
                shared.palette = palette.clone();
            }
            // MT-098: keep Runtime Chat visually aligned with the live app theme. This is a pure palette
            // overwrite; send attempts remain typed-blocked by RuntimeChatClient::production.
            if let Ok(mut panel) = self.runtime_chat_panel.lock() {
                panel.set_palette(palette.clone());
            }
            // WP-KERNEL-012 MT-079 (AC-079-2): push the live workspace id + runtime handle into the
            // editor mounts' session-context cell BEFORE the pane host renders, so a mounted code/rich
            // pane threads real session context on its first live frame (set_runtime/set_workspace_id +
            // set_embed_context/set_wikilink_context). A snapshot-capture pass must NOT install context
            // (the editors' `set_*` hooks spawn off-thread work; the throwaway capture context must stay
            // side-effect-free — the `drain_shell_events` capture guard). With no runtime handle (a
            // current-thread headless shell) the context stays unbound and the editors render in their
            // graceful runtime-less mode.
            if !self.capturing_snapshot {
                if let (Some(ws), Some(rt)) = (workspace_id, self.runtime_handle.clone()) {
                    if let Ok(mut sess) = self.editor_mounts.session.lock() {
                        *sess = EditorSessionContext::new(ws, rt);
                    }
                }
            }
            // WP-KERNEL-012 MT-080: push the live theme palette into the SECONDARY pane factories' shared
            // cell so the canvas/graph/side panes render with the active theme tokens (and flip on a runtime
            // theme toggle), the same per-frame push the search panes use. Safe in a capture pass too (it is
            // a pure palette overwrite with no off-thread side effect).
            if let Ok(mut p) = self.editor_mounts.secondary.palette.lock() {
                *p = palette;
            }
        }

        // Apply the integrated-rail scrollbar style (MT-010) every frame from the LIVE palette, so
        // egui's built-in `ScrollArea` scrollbars render in the rail dimensions + colors and track a
        // runtime theme toggle on the next frame. This overrides only scrollbar-specific
        // spacing/handle fills — never panel/window backgrounds (rails red-team control).
        let rail_palette = self.current_theme.palette();
        apply_rail_scrollbar_style(
            ctx,
            RailColors::from_palette(&rail_palette),
            RailDimensions::default(),
        );

        // ── Top application menu bar (MT-015) ───────────────────────────────────────────────────────
        // Registered as the VERY FIRST panel in the frame, ABOVE the title bar / module switcher and the
        // project-tab strip, so egui reserves the menu strip at the top edge before any other panel
        // carves the remaining area (red-team MC5/R5: must come before the central panel). The bar
        // returns the leaf action the user triggered this frame; we dispatch it into the existing
        // state-mutation paths AFTER the panel closes so the menu closures never hold a `&mut self`.
        //
        // Alt+<letter> mnemonics (AC2): handle the access-key chords BEFORE the panel renders so the
        // chosen menu's popup is already marked open in egui memory when `MenuBar::show` runs this frame.
        // `handle_menu_mnemonics` consumes the chord (so the global keymap handler never double-fires the
        // same Alt combo — red-team R3) and opens the popup; the open menu is then keyboard-navigable.
        if crate::top_menu_bar::handle_menu_mnemonics(ctx).is_some() {
            ctx.request_repaint();
        }

        // ── Command palette chord (MT-016): Ctrl+Shift+P toggles the palette ──────────────────────────
        // "Mod" = Ctrl on Windows/Linux, Cmd on macOS — egui's `Modifiers::COMMAND` maps to the platform
        // accelerator, so `COMMAND + SHIFT + P` is the cross-platform "Mod-Shift-P" chord from the React
        // `APP_KEYBINDING_ACTIONS` default. `consume_key` swallows the chord so it does not also reach the
        // global keymap / editor layer (red-team R4/MC4 — no double-fire). Handled BEFORE the menu bar
        // renders so the toggle is applied this frame.
        // RED-TEAM R1/MC1 (MT-018): while the settings dialog is open, a keybinding text input may be
        // focused and the user may type a chord like "Mod-p" / "Mod-Shift-p" INTO it. Do not let the
        // global chord handler steal those keystrokes (which would open the palette/switcher mid-edit and
        // swallow the character). Skip global chord handling entirely while settings is open; the dialog
        // owns keyboard input then.
        let suppress_global_chords = self.settings_open;
        let palette_chord = !suppress_global_chords
            && ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::P,
                )
            });
        if palette_chord {
            self.toggle_command_palette();
            ctx.request_repaint();
        }

        // ── Quick switcher chord (MT-017): Ctrl+P (Mod-P) toggles the switcher ─────────────────────────
        // "Mod" = Ctrl on Windows/Linux, Cmd on macOS — egui's `Modifiers::COMMAND` maps to the platform
        // accelerator, so `COMMAND + P` is the cross-platform "Mod-P" chord (the React
        // `app.quick_switcher.open` default). DISTINCT from the command-palette chord above (Mod-Shift-P),
        // which is why this is handled separately and consumes its own key. `consume_key` swallows the
        // chord so it does not also reach the global keymap / editor layer (no double-fire). Handled
        // BEFORE the menu bar renders so the toggle is applied this frame.
        let switcher_chord = !suppress_global_chords
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::P));
        if switcher_chord {
            self.toggle_quick_switcher();
            ctx.request_repaint();
        }

        // MT-099: route the platform Save chord through the same editor command arm as FILE > Save, so
        // keyboard and menu saves share the mounted RichEditor SaveManager and the knowledge-documents
        // backend adapter. The rich editor also handles Ctrl+S when focused; consuming here keeps one
        // app-level save path and avoids a double PUT.
        let save_chord = !suppress_global_chords
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        if save_chord {
            self.dispatch_palette_action(ctx, crate::command_registry::CMD_EDITOR_FILE_SAVE);
            ctx.request_repaint();
        }

        let menu_state = self.menu_bar_state(ctx);
        let menu_action = egui::TopBottomPanel::top("handshake_menu_bar")
            .show(ctx, |ui| MenuBar::new(menu_state).show(ui))
            .inner;
        if let Some(action) = menu_action {
            if self.dispatch_menu_action(ctx, action) {
                ctx.request_repaint();
            }
        }

        // The header row carries (left) the "Handshake" identity and (right, right-to-left) the theme
        // toggle followed by the MODULE switcher (MT-012). The switcher is right-aligned in the header —
        // DISTINCT from the project-tab strip below and the per-pane tab bars — per the WP design intent.
        let module_switch_request = egui::TopBottomPanel::top("handshake_title_bar")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.title_identity(ui);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.theme_toggle(ui);
                        ui.add_space(12.0);
                        self.module_switcher_ui(ui)
                    })
                    .inner
                })
                .inner
            })
            .inner;
        if let Some(module_id) = module_switch_request {
            // A non-active module button was clicked: retab the active pane + move the highlight. The
            // change is detected by the MT-006/MT-009 layout change-detector below, which schedules the
            // debounced save (no synchronous save here — rapid clicks coalesce).
            if self.set_module(module_id) {
                ctx.request_repaint();
            }
        }

        // ── Top project-tabs strip (MT-011) ─────────────────────────────────────────────────────────
        // Sits directly below the title bar and above the pane grid. Switching a project tab drives
        // `active_project_id`; the layout-persistence lifecycle (below) then saves the leaving project's
        // layout and loads the entered project's layout on the next frame. Colors come from the active
        // MT-003 theme tokens so the strip flips dark<->light with the rest of the shell.
        let project_palette = self.current_theme.palette();
        let project_tab_colors = ProjectTabColors {
            bar_bg: project_palette.bg,
            active_bg: project_palette.accent_soft,
            inactive_bg: project_palette.surface,
            hover_bg: project_palette.surface_strong,
            text: project_palette.text,
            disabled_text: project_palette.text_subtle,
            accent: project_palette.accent,
            error: project_palette.error_text,
        };
        let switch_request = egui::TopBottomPanel::top("handshake_project_tabs")
            .exact_height(PROJECT_TAB_BAR_HEIGHT)
            .show(ctx, |ui| self.project_tabs.show(ui, project_tab_colors))
            .inner;
        if let Some(new_project_id) = switch_request {
            // A non-active project tab was clicked: perform the save-old / set-active transition. The
            // entered project's layout LOADS on the next frame via the lifecycle (loaded != active).
            if self.switch_project(&new_project_id) {
                ctx.request_repaint();
            }
        }

        // WP-KERNEL-012 MT-088 (§5.8.5 HARD): the status-bar health segment is the global backend
        // indicator. When the debounced backend-down state is set it shows an explicit, FINITE
        // "Disconnected" indicator (NOT a perpetual spinner and NOT a hang — AC-008-4 / RISK-008-3),
        // computed once in `status_bar_health_text()` so the rendered label and the test accessor cannot
        // drift. The segment is a live AccessKit `Status` node, so the degraded state is perceivable by
        // the operator AND a swarm agent reading the tree.
        let status_text = self.status_bar_health_text();
        // The health segment is hidden iff the operator hid it via the status-bar menu (MT-021).
        let health_hidden = self.statusbar_hidden.contains("health");
        // WP-KERNEL-012 MT-071: the live editor file-metadata segment cluster reads the FOCUSED code
        // document's metadata this frame (None hides it — AC-005). Resolved BEFORE the panel closure so
        // the immutable bus/registry reads do not overlap the `&mut self` status-bar segment borrow.
        let editor_segment_state = self.editor_segment_state(ctx);
        let (status_action, editor_segment_action) =
            egui::TopBottomPanel::bottom("handshake_status_bar")
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let action = if health_hidden {
                            // Hidden segment: render a neutral non-interactive placeholder so the bar is
                            // never blank (and the segment can still be restored via a settings surface).
                            ui.label("");
                            None
                        } else {
                            self.status_bar_segment(ui, &status_text)
                        };
                        // MT-030: the quick-switcher nav-status segment (the editor-pane typed seam)
                        // renders here when present so the seam outcome is PERCEIVABLE (rendered label +
                        // live AccessKit node) after the overlay closes; a no-op when no nav status is set.
                        self.quick_switcher_nav_status_segment(ui);
                        self.terminal_launch_status_segment(ui);
                        self.model_session_launch_status_segment(ui);
                        // WP-KERNEL-012 MT-071: mount the five editor file-metadata segments (LanguageMode
                        // / EOL / Indent / Encoding / RenderWhitespace) in the RIGHT cluster of the LIVE
                        // status bar (NOT a standalone harness). They render only when a code pane is
                        // focused; the returned typed action is applied to the doc model after the panel
                        // closes (so no `&mut` to the panel is held inside the egui closure).
                        let editor_action = ui
                            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                EditorStatusSegments::new(editor_segment_state.clone()).show(ui)
                            })
                            .inner;
                        (action, editor_action)
                    })
                    .inner
                })
                .inner;
        if let Some(action) = status_action {
            if self.apply_status_bar_action(ctx, "health", &status_text, action) {
                ctx.request_repaint();
            }
        }
        if let Some(action) = editor_segment_action {
            if self.apply_editor_segment_action(ctx, action) {
                ctx.request_repaint();
            }
        }

        // ── Bottom search rail (MT-022) ─────────────────────────────────────────────────────────────
        // A pinned, always-visible 32px bottom panel registered ABOVE the status bar (egui stacks bottom
        // panels in registration order, so this sits just above the status strip) and BEFORE the central
        // panel, so it claims its space first and never collapses (AC-022-1). It owns query input + scope
        // selection and emits a parsed RailQuery search intent; the result popup floats above the strip.
        self.drive_search_rail(ctx);

        // ── Bottom drawer stash shelf (MT-023, C6) ───────────────────────────────────────────────────
        // ORDER: drawer -> rail -> central -- do not reorder. Registered AFTER the rail so it stacks
        // ABOVE the rail (this codebase's egui registration-order convention: the later bottom panel is
        // higher), then BEFORE the CentralPanel so the open drawer compresses the tile area upward
        // without overlapping it (AC-023-9). The always-visible affordance tab toggles the drawer; when
        // open the shelf shows the four typed cards (Agenda/Mail/Lists/Notes) fed by off-thread fetches.
        self.drive_drawer(ctx);

        // ── Apply a pending pop-out request (MT-008) ───────────────────────────────────────────────
        // A request set by `request_pop_out` (future MT-019 pane-header action / test / out-of-process
        // driver) is applied here, at the top of the frame, BEFORE the layout renders, so the detached
        // viewport is created cleanly and the split layout draws the placeholder this same frame. The
        // pane's record is NOT removed from the registry — only its render destination changes.
        if let Some(pane_id) = self.pop_out_request.take() {
            // Only pop out a pane that actually exists in the registry and is not already popped out.
            let exists = self
                .pane_registry
                .lock()
                .expect("pane registry mutex poisoned")
                .get(&pane_id)
                .is_some();
            if exists && !self.popout_manager.is_popped_out(&pane_id) {
                // Open the window near the current pointer if known, else at the fallback position.
                let pos = ctx
                    .pointer_latest_pos()
                    .unwrap_or(crate::popout_window::FALLBACK_POPOUT_POS);
                self.popout_manager
                    .pop_out(pane_id, PopOutGeometry::at(pos));
            }
        }

        // ── Left activity rail (MT-014) ─────────────────────────────────────────────────────────────
        // The collapsible SidePanel::left holds (collapsed) the activity icon strip or (open) the full
        // project tree + quick links + stash/agenda/mail/notes affordances. It is rendered AFTER the top
        // strips and BEFORE the central pane grid, so egui carves it from the left edge of the remaining
        // area. Colors come from the active MT-003 theme tokens so the rail flips dark<->light.
        //
        // Point the project tree at the active workspace each frame (a no-op if unchanged); when the
        // workspace changes this spawns the async document/canvas load on the cloned runtime handle. In
        // the headless/test shell there is no multi-thread runtime, so the tree renders from
        // directly-seeded content instead (set via `left_rail_mut().project_tree.set_content`).
        if let Some(handle) = self.runtime_handle.clone() {
            let active_project = self.active_project_id.clone();
            self.left_rail
                .project_tree
                .set_workspace(&active_project, &handle);
        }
        // Drain the shell event bus (MT-014 FIX-B) BEFORE rendering so a document/canvas/bookmark
        // deleted from another surface disappears from the tree this frame with no stale row.
        if self.drain_shell_events() > 0 {
            ctx.request_repaint();
        }
        // Drain any delivered async tree-load result before rendering this frame (non-blocking).
        self.left_rail.project_tree.poll();

        let rail_palette = self.current_theme.palette();
        let left_rail_colors = LeftRailColors {
            icon_bg: rail_palette.surface,
            icon_hover_bg: rail_palette.surface_strong,
            icon_active_bg: rail_palette.accent_soft,
            icon_text: rail_palette.text,
            row_bg: rail_palette.surface,
            row_hover_bg: rail_palette.surface_strong,
            row_text: rail_palette.text,
            group_text: rail_palette.text_subtle,
            muted_text: rail_palette.text_subtle,
            error: rail_palette.error_text,
            project_prefix: rail_palette.text_subtle,
        };
        let quick_link_entries = self.build_quick_link_entries();
        let rail_open = self.left_rail_open;
        // The collapsed rail is just the icon strip (~40px incl. padding); the open rail is resizable
        // from a 200px default. min_width keeps the icon strip visible in both states.
        let rail_event = {
            let left_rail = &mut self.left_rail;
            egui::SidePanel::left("left-rail")
                .resizable(rail_open)
                .min_width(if rail_open { 180.0 } else { 40.0 })
                .default_width(if rail_open { 220.0 } else { 40.0 })
                .show(ctx, |ui| {
                    left_rail.show(ui, rail_open, &quick_link_entries, left_rail_colors)
                })
                .inner
        };
        if let Some(event) = rail_event {
            if self.apply_left_rail_event(ctx, event) {
                ctx.request_repaint();
            }
        }
        // MT-020 explorer-row rename: drain any delivered PATCH result, then render the rename dialog.
        self.drive_rename(ctx);
        // MT-064 FEMS "Propose to Memory": render the open proposal dialog (or its status note) — the
        // visible result of dispatching the `fems.propose_to_memory` palette command over a selection.
        self.drive_propose_to_memory(ctx);
        // MT-101 model-session launch: render the compact one-shot launch dialog opened from RUN/palette
        // and drain the off-thread `/jobs` result without blocking the UI thread.
        self.drive_model_session_launch_dialog(ctx);
        // MT-021: drain any delivered SCM / canvas / Loom-node-flag off-thread results into panel state
        // (the network already ran off the UI thread — these just read the delivery cells, HBR-QUIET).
        self.drive_source_control(ctx);
        self.drive_canvas(ctx);
        self.drive_loom_node(ctx);

        // ── CKC / Atelier melt-together: drag-source panel + Route-to-Stage drain + Stage pane (MT-033) ──
        // Rendered AFTER the left rail and BEFORE the CentralPanel borrow-split, so egui carves the right
        // edge (Atelier drag source) and the bottom edge (Stage pane) from the remaining area before the
        // central 2x2 pane grid claims the rest (the same edge-panel-before-central convention the left
        // rail / drawer / search-rail use). This is what makes the MT-033 surfaces REACHABLE in the running
        // product (not only in standalone test harnesses): the drag source can be dragged FROM, and a
        // dispatched Route-to-Stage command (palette or context menu) is DRAINED into the visible pane.
        self.drive_ckc_interop(ctx);
        self.drive_rich_document_load(ctx);
        self.sync_active_tab_records();

        // Split the borrow of `self` up-front so the CentralPanel closure can hold a `&mut` to the
        // split state (weights/drag/active pane) AND a `&` to the factories + registry at the same
        // time. The registry is the single source of truth (MT-005); MT-006 partitions the central
        // panel into a 2x2 grid with two draggable/keyboard-resizable dividers around it.
        let registry = &self.pane_registry;
        let factories = &self.factories;
        let split_weights = &mut self.split_weights;
        let split_drag = &mut self.split_drag;
        let active_pane = &mut self.active_pane;
        let tab_bar_states = &mut self.tab_bar_states;
        // Catch-all factory for any PaneType without a dedicated entry: the empty-label Placeholder
        // key registered in build_default_factories.
        let fallback_key = PaneType::Placeholder(String::new());

        // Snapshot the popped-out pane set so the CentralPanel's `is_popped_out` predicate can borrow
        // it by `&` while `popout_manager` is reserved for the post-frame `show_all` (&mut). Merge-back
        // clicks collected by the placeholder are applied to the manager after the CentralPanel closes.
        let popped_out: std::collections::HashSet<PaneId> =
            self.popout_manager.popped_out_ids().into_iter().collect();
        let mut merge_requests: Vec<PaneId> = Vec::new();
        // MT-013: pane-header Lock/Unlock clicks collected from the split layout this frame, applied to
        // the registry's LockState after the CentralPanel closes (single source of truth for pane
        // state). The active module is read once for the tab-chip module/type badge.
        let mut lock_requests: Vec<PaneId> = Vec::new();
        // MT-020: "Pop Out" chosen from a pane-tab or pane-header context menu, collected from the split
        // layout this frame, applied via `request_pop_out` after the CentralPanel closes (MT-008).
        let mut pop_out_requests: Vec<PaneId> = Vec::new();
        let active_module = self.module_switcher.active();

        // Divider colors come from the active theme's MT-003 tokens (idle/hover/grab), so the
        // dividers are themed and flip dark<->light with the rest of the shell (MT-006 contract).
        let palette = self.current_theme.palette();
        let divider_colors = DividerColors {
            idle: palette.divider_idle,
            hover: palette.divider_hover,
            grab: palette.divider_grab,
        };
        // Tab-bar colors come from the same MT-003 theme tokens so the tab strip is themed and flips
        // dark<->light with the rest of the shell (mirrors the divider token wiring above): the active
        // tab uses the accent-soft fill, inactive tabs use the surface fill, glyphs/dots use accent.
        let tab_colors = TabBarColors {
            active_bg: palette.accent_soft,
            inactive_bg: palette.surface,
            text: palette.text,
            accent: palette.accent,
            drop_highlight: palette.accent_soft,
        };
        // MT-013 pane-header colors from the same MT-003 theme tokens so the header (active-tab title +
        // lock control) flips dark<->light with the rest of the shell.
        let header_colors = PaneHeaderColors {
            bg: palette.surface,
            title: palette.text,
            lock_text: palette.text_subtle,
            lock_bg: palette.surface_strong,
            lock_hover_bg: palette.accent_soft,
            locked_accent: palette.accent,
        };
        // The placeholder tile's label + Merge Back button paint with the active theme's text token.
        let placeholder_text = palette.text;

        egui::CentralPanel::default().show(ctx, |ui| {
            // SplitLayoutWidget renders the current pane slots into their split rects and dividers.
            // The pane render path keeps LIVE AccessKit emission (MT-025): the emit callback is
            // invoked once per pane inside its egui scope, so panes remain findable out-of-process
            // by author_id and the MT-025 live-tree tests still pass. A pane that is popped out
            // (MT-008) renders a PopOutPlaceholder tile here instead of its tab bar + body; a Merge
            // Back click is collected into `merge_requests` and applied after the panel closes.
            SplitLayoutWidget::show(
                ui,
                split_weights,
                split_drag,
                active_pane,
                registry,
                divider_colors,
                tab_bar_states,
                tab_colors,
                active_module,
                header_colors,
                &mut lock_requests,
                &mut pop_out_requests,
                |pane_id| popped_out.contains(pane_id),
                &mut merge_requests,
                placeholder_text,
                |pane_type| {
                    factories
                        .get(pane_type)
                        .or_else(|| factories.get(&fallback_key))
                        .expect("placeholder fallback factory always registered")
                        .as_ref()
                },
                |ui_ctx, pane_egui_id, pane_author_id, role, label| {
                    accessibility::emit_pane_node(
                        ui_ctx,
                        pane_egui_id,
                        pane_author_id,
                        role,
                        label,
                    );
                },
            );
        });

        // ── MT-028: drain LoomSearchV2 open-block requests into the Loom block-open path ─────────────
        // A result-row click in the in-product Loom Search pane pushed the block id into the shared cell;
        // route each id to the Loom block viewer on the active pane (open-in-place, a REFERENCE — the
        // same open path the bookmark rail uses for a pinned Loom block). Drained AFTER the pane host so
        // the click registered this frame opens the block this frame.
        let open_block_requests: Vec<String> = self
            .loom_search_v2_shared
            .lock()
            .map(|mut s| std::mem::take(&mut s.open_requests))
            .unwrap_or_default();
        for block_id in open_block_requests {
            self.open_content_on_active_pane(PaneType::LoomBlock, Some(block_id));
        }

        // ── MT-029: drain Find-in-Files open-hit requests into the appropriate open path ─────────────
        // A result-row click in the in-product Find-in-Files pane pushed the hit into the shared cell;
        // route each hit by its source_kind to the matching open path (rich document -> the document
        // editor, loom_block/file/tag_hub -> the Loom block viewer). Open-in-place, a REFERENCE — the
        // same open paths the bookmark rail + Loom Search pane use. Drained AFTER the pane host so the
        // click registered this frame opens this frame.
        let open_hit_requests: Vec<crate::backend_client::LoomGraphSearchHit> = self
            .find_in_files_shared
            .lock()
            .map(|mut s| std::mem::take(&mut s.open_requests))
            .unwrap_or_default();
        for hit in open_hit_requests {
            // Prefer a rich-document open when the hit resolves to a KRD- document; otherwise open the
            // Loom block by its ref id (file/tag_hub/loom_block all open as a Loom block reference).
            if let Some(document_id) = crate::find_in_files::document_id_from_hit(&hit) {
                self.open_content_on_active_pane(PaneType::LoomWikiPage, Some(document_id));
            } else if !hit.ref_id.trim().is_empty() {
                self.open_content_on_active_pane(PaneType::LoomBlock, Some(hit.ref_id.clone()));
            }
        }

        // ── WP-KERNEL-012 MT-079: drain the mounted editors' command + event channels ────────────────
        // Drained AFTER the pane host so a Save/Undo/Redo/OpenCommandPalette keypress or a wikilink/
        // backlink/tag chip click handled THIS frame is dispatched THIS frame.
        self.drive_editor_mounts(ctx);

        // ── Apply MT-013 pane-header Lock/Unlock requests ───────────────────────────────────────────
        // A lock click from the pane header (pointer OR out-of-process AccessKit Click) toggles the
        // pane record's LockState in the registry (single source of truth). The change is picked up by
        // the MT-009 layout change-detector below (LockState is part of the captured pane record), so
        // it persists through the debounced save with no synchronous save here.
        if !lock_requests.is_empty() {
            let mut guard = self
                .pane_registry
                .lock()
                .expect("pane registry mutex poisoned");
            for pane_id in &lock_requests {
                if let Some(record) = guard.get_mut(pane_id) {
                    record.lock_state = match record.lock_state {
                        LockState::Locked => LockState::Unlocked,
                        LockState::Unlocked => LockState::Locked,
                    };
                }
            }
        }

        // ── Apply MT-020 pop-out requests (from a tab / pane-header context menu) ────────────────────
        // "Pop Out" chosen from the tab or pane-header menu pops the whole pane into its own OS window
        // (MT-008). Only one context menu is open at a time, so at most one request lands per frame; if
        // several somehow arrive, the last wins (the single `pop_out_request` slot). Applied at the top
        // of the next frame by the existing MT-008 pop-out lifecycle.
        if let Some(pane_id) = pop_out_requests.last() {
            self.request_pop_out(pane_id.clone());
        }

        // ── Apply merge-back requests, then render detached pop-out windows (MT-008) ────────────────
        // A Merge Back click from the placeholder (pointer OR out-of-process AccessKit Click) marks
        // the pop-out for close; `show_all`'s post-show drain removes it next frame so the pane
        // returns to the main split.
        for pane_id in &merge_requests {
            self.popout_manager.merge_back(pane_id);
        }

        // Render every open pop-out into its own deferred viewport. The pane is STILL in the registry,
        // so we render it through the SAME factory + tab-bar path the main split uses (one source of
        // truth). `show_all` drains entries that requested close (Merge Back button or OS close
        // button) after showing, returning the merged-back pane ids.
        let registry = self.pane_registry.clone();
        let factories = &self.factories;
        let tab_bar_states = &mut self.tab_bar_states;
        let fallback_key = PaneType::Placeholder(String::new());
        // Resolve the detached-window title from the registry's surface label, so it reads
        // "Handshake – <pane_type_label>" (e.g. "Handshake – Workspace"). Falls back to the pane id
        // string only if the record vanished (defensive; should not happen while popped out).
        let title_registry = registry.clone();
        let title_for = move |pane_id: &PaneId| -> String {
            let label = title_registry
                .lock()
                .expect("pane registry mutex poisoned")
                .get(pane_id)
                .map(|r| r.pane_type.label())
                .unwrap_or_else(|| pane_id.as_ref().to_owned());
            popout_title_for(&label)
        };
        let _merged_back = self
            .popout_manager
            .show_all(ctx, title_for, |ctx, _class, pane_id| {
                Self::render_popout_body(
                    ctx,
                    pane_id,
                    &registry,
                    factories,
                    &fallback_key,
                    tab_bar_states,
                    tab_colors,
                    active_module,
                    header_colors,
                );
            });

        // ── Command palette overlay (MT-016) ────────────────────────────────────────────────────────
        // Rendered LAST (after the menu bar, the title/project strips, the left rail, the central pane
        // grid, and the pop-outs) so its backdrop + window sit on the Foreground order ABOVE the whole
        // workspace (AC10). The overlay owns only its transient query/selection state; it returns a
        // PaletteOutcome the shell dispatches into the existing state-mutation paths (same split as the
        // MT-015 menu bar). The shell owns the open flag, so a Run/Close outcome clears it here.
        if self.command_palette_open {
            // MT-069: pass the live editor-available predicate so the EditorMenu palette rows are enabled
            // only when an editor pane is the focusable target (no fake-enabled rows when none is mounted).
            let editor_available = self.editor_available();
            let outcome = crate::command_palette::show(
                ctx,
                self.command_palette_open_count,
                editor_available,
            );
            match outcome {
                crate::command_palette::PaletteOutcome::Run(command_id) => {
                    self.close_command_palette();
                    if self.dispatch_palette_action(ctx, &command_id) {
                        ctx.request_repaint();
                    }
                }
                crate::command_palette::PaletteOutcome::Close => {
                    self.close_command_palette();
                    ctx.request_repaint();
                }
                crate::command_palette::PaletteOutcome::None => {}
            }
        }

        // ── Quick switcher overlay (MT-017) ─────────────────────────────────────────────────────────
        // Rendered after the command palette so its backdrop + window sit on the Foreground order ABOVE
        // the whole workspace. The overlay searches the REAL Loom graph over PostgreSQL: on open it
        // loads durable recents (`GET quick-switcher/recents`); typing debounces (~150ms) then queries
        // `GET graph-search`; selecting a hit records the visit (`POST recents`) + opens its typed
        // target on a pane. All backend I/O happens on the tokio runtime off the egui frame thread
        // (HBR-QUIET); the spawned tasks write into shared cells the frame drains with try_lock.
        if self.quick_switcher_open {
            self.drive_quick_switcher(ctx);
        }

        // ── Settings dialog overlay (MT-018) ────────────────────────────────────────────────────────
        // Rendered LAST among the overlays so its backdrop + window sit on the Foreground order ABOVE the
        // command palette + quick switcher and the whole workspace. The dialog ports
        // app/src/components/SettingsMenu.tsx: Appearance (wired theme + view mode), Keybindings (editable
        // with live conflict detection), Swarm (wired board-default-open checkbox + not-yet-wired
        // intervals), Terminal (not-yet-wired), Layout (wired reset), About (real Cargo version). Wired
        // changes persist THROUGH `PUT /workspaces/{id}/settings` (PostgreSQL-authoritative) on a tokio
        // task off the egui frame thread (HBR-QUIET), debounced 500ms (red-team R2); a dialog close
        // flushes a pending save immediately (red-team MC2). Closed by default, so the default-seed live
        // tree is unchanged (MT-025 default snapshot stays at its baseline node count).
        if self.settings_open {
            self.drive_settings_dialog(ctx);
        }

        // ── Layout persistence lifecycle (MT-009 BLOCKER) ───────────────────────────────────────────
        // Runs AFTER the frame's interactions (split drag, tab reorder/active/pin, pop-out/merge) are
        // applied, so change detection sees this frame's final layout. Refresh the monitor extent from
        // egui so the restore clamp uses the real desktop bounds; fall back to the generous default if
        // egui has not reported a monitor size yet (headless).
        if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
            if monitor.x > 0.0 && monitor.y > 0.0 {
                self.monitor_extent = egui::Rect::from_min_size(egui::Pos2::ZERO, monitor);
            }
        }
        self.drive_layout_persistence(std::time::Instant::now());

        // While a save is debounced/pending, keep frames coming so the debounce window actually
        // elapses even without further input (otherwise a quiescent app would never flush). WP-KERNEL-012
        // MT-088 (§5.8.5): `try_lock` so a save worker mid backend-down PUT (holding the manager lock)
        // never blocks the UI thread here; if contended, the in-flight save itself already keeps frames
        // coming via `save_in_flight`, and the next frame re-checks.
        let pending_save = self
            .layout_manager
            .try_lock()
            .map(|mgr| mgr.is_dirty())
            .unwrap_or(false);
        if pending_save {
            ctx.request_repaint_after(LAYOUT_SAVE_DEBOUNCE);
        }

        if matches!(self.health_status, HealthDisplayState::Loading) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // WP-KERNEL-012 MT-088: an OUTSTANDING backend interaction (a layout load / debounced save in
        // flight, or a `/health` re-probe in flight) is now drained EVENT-DRIVEN: the worker calls
        // `ctx.request_repaint()` exactly ONCE when it delivers its result (see `spawn_layout_load`,
        // `spawn_layout_save_now`, the debounced-save spawn, and the `/health` re-probe spawn), so the
        // next frame applies it. This deliberately REPLACES the previous per-frame
        // `request_repaint_after(100ms)` poll-for-delivery cadence, which requested a repaint EVERY frame
        // while a load/save/probe was in flight — that kept the UI from ever reporting idle within
        // `Harness::run`'s bounded step budget and regressed every `Harness::run`-based pane test
        // (`max_steps`). The MT-084 heartbeat still guarantees a ~250ms idle tick, so even if a worker's
        // wake were ever lost the result is still drained within ~250ms; no busy-loop, no repaint storm.
        //
        // A `/health` re-probe that is merely SCHEDULED (none in flight yet — no worker exists to wake the
        // UI) still needs a one-shot scheduled wake when it comes due, so recovery is observed on an
        // otherwise-idle app (AC-008-6). This is a single bounded `request_repaint_after`, not a per-frame
        // poll: it schedules ONE wake at the due time (HBR-QUIET: no focus steal, no input grab).
        if self.health_handle.is_none() {
            if let Some(at) = self.health_next_poll_at {
                let now = std::time::Instant::now();
                let delay = at.saturating_duration_since(now);
                ctx.request_repaint_after(delay.max(std::time::Duration::from_millis(50)));
            }
        }
    }

    /// Render a popped-out pane's body (tab bar + factory content) inside its detached viewport's
    /// `CentralPanel`. This is the SAME render path the main split uses for a docked pane (tab bar
    /// strip on top, factory body below, live AccessKit pane node emitted), so a popped-out pane is
    /// rendered from the one registry source of truth and remains addressable out-of-process by its
    /// stable `author_id` — only the host window changed (MT-008 / HBR-SWARM accessibility).
    #[allow(clippy::too_many_arguments)]
    fn render_popout_body(
        ctx: &egui::Context,
        pane_id: &PaneId,
        registry: &Arc<Mutex<PaneRegistry>>,
        factories: &HashMap<PaneType, Box<dyn PaneFactory>>,
        fallback_key: &PaneType,
        tab_bar_states: &mut HashMap<PaneId, TabBarState>,
        tab_colors: TabBarColors,
        active_module: ModuleId,
        header_colors: PaneHeaderColors,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let guard = registry.lock().expect("pane registry mutex poisoned");
            let Some(record) = guard.get(pane_id) else {
                // The pane was removed from the registry while popped out: show nothing rather than
                // panic. (Closing a popped-out pane is an MT-019+ concern; this is the safe default.)
                return;
            };
            let node_id = guard.accesskit_id(pane_id).map(|n| n.0).unwrap_or(0);
            let pane_egui_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
            let factory = factories
                .get(&record.pane_type)
                .or_else(|| factories.get(fallback_key))
                .expect("placeholder fallback factory always registered")
                .as_ref();
            let role = factory.accesskit_role();
            let label = record.pane_type.label();
            let locked = record.lock_state == LockState::Locked;

            // Carve, from the TOP: MT-013 header strip, then MT-007 tab strip, then the body — the SAME
            // stack the docked pane uses, so a popped-out pane keeps its header binding + tab badges.
            let full = ui.available_rect_before_wrap();
            let header_h = PANE_HEADER_HEIGHT.min(full.height());
            let header_rect =
                egui::Rect::from_min_max(full.min, egui::pos2(full.right(), full.top() + header_h));
            let after_header_top = full.top() + header_h;
            let tab_h = TAB_BAR_HEIGHT.min((full.bottom() - after_header_top).max(0.0));
            let tab_rect = egui::Rect::from_min_max(
                egui::pos2(full.left(), after_header_top),
                egui::pos2(full.right(), after_header_top + tab_h),
            );
            let body_rect = egui::Rect::from_min_max(
                egui::pos2(full.left(), after_header_top + tab_h),
                full.max,
            );

            // MT-013 header: active-tab title binding + lock control (parity with the docked pane). The
            // lock click inside a pop-out is reconciled by a later cross-window mutation MT (same as the
            // tab interactions below); here the header is rendered for binding + accessibility parity.
            {
                let active_tab_label: String = tab_bar_states
                    .get(pane_id)
                    .and_then(|bar| bar.active().map(|t| t.label()))
                    .unwrap_or_default();
                let mut header_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("popout-pane-header", node_id))
                        .max_rect(header_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                header_ui.set_clip_rect(header_rect);
                // A popped-out pane is by definition not the only pane (it detached FROM the grid), so
                // `is_last_pane=false`; its header context menu's Close Pane stays future-target/disabled
                // either way. Header interactions inside a pop-out are reconciled by a later cross-window
                // MT (same as the tab interactions below); here the header is rendered for binding parity.
                let _resp = PaneHeader::show(
                    &mut header_ui,
                    pane_id.as_ref(),
                    &active_tab_label,
                    locked,
                    false,
                    header_colors,
                );
            }

            if let Some(tab_state) = tab_bar_states.get(pane_id) {
                let mut tab_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("popout-tab-bar", node_id))
                        .max_rect(tab_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                tab_ui.set_clip_rect(tab_rect);
                // Render the tab bar; tab interactions inside a pop-out are reconciled by a later MT
                // (the detached-window tab mutation path). Here the bar is rendered for parity +
                // accessibility; its interactions are intentionally not yet reconciled cross-window.
                let _resp = TabBar::show(&mut tab_ui, tab_state, tab_colors, active_module);
            }

            let render_ctx = PaneRenderContext {
                record,
                egui_id: pane_egui_id,
            };
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("popout-body", node_id))
                    .max_rect(body_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(body_rect);
            factory.render(&mut child, &render_ctx);
            child.interact(body_rect, pane_egui_id, egui::Sense::hover());
            // Live AccessKit pane node in the pop-out's own tree, addressed by the SAME author_id as
            // when docked, so out-of-process steering finds the pane regardless of host window.
            accessibility::emit_pane_node(ui.ctx(), pane_egui_id, pane_id.as_ref(), role, &label);
        });
    }
}

/// The MT-030 navigation bus: the shell IS the [`ShellNavigator`](crate::quick_switcher::ShellNavigator)
/// the quick switcher (and, later, the E5/E11 interconnection MTs) drive to open a resolved target on
/// the correct editor or panel. Each arm reuses the shared [`open_navigator_tab`](HandshakeApp::open_navigator_tab)
/// tab-open primitive (de-dupe + activate + focus) so there is ONE tab-mutation path, and returns the
/// typed [`NavDispatchOutcome`](crate::quick_switcher::NavDispatchOutcome).
///
/// WP-KERNEL-012 MT-079 (E11 host-mount): the TWO editor-pane arms now open REAL mounted editors.
/// `open_document` -> the rich-text/Notes editor (`PaneType::LoomWikiPage` -> `RichEditorPaneMount`);
/// `open_code_symbol` -> the code editor (`PaneType::CodeSymbol` -> `CodeEditorPaneMount`). Both editor
/// factories are now REGISTERED over their former `PlaceholderPaneFactory` entries (see
/// `install_editor_mounts`), so these arms open + focus the live editor pane through the SAME
/// `open_navigator_tab` primitive every other arm uses — the `EditorPaneNotMounted` seam these arms
/// returned before E11 is RETIRED for them. All seven arms now open real shell tabs.
impl crate::quick_switcher::ShellNavigator for HandshakeApp {
    fn open_document(&mut self, document_id: &str) -> crate::quick_switcher::NavDispatchOutcome {
        // WP-KERNEL-012 MT-079 (AC-079-4): the rich-text/Notes editor pane is now MOUNTED
        // (PaneType::LoomWikiPage -> RichEditorPaneMount), so this arm opens + focuses the REAL editor
        // pane instead of returning the EditorPaneNotMounted seam. Open the document on the Notes
        // surface through the SAME tab-open primitive every other arm uses (de-dupe + activate + focus).
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| document_id.to_owned());
        match self.open_navigator_tab(PaneType::LoomWikiPage, document_id.to_owned(), &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }

    fn open_loom_block(&mut self, block_id: &str) -> crate::quick_switcher::NavDispatchOutcome {
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| block_id.to_owned());
        match self.open_navigator_tab(PaneType::LoomBlock, block_id.to_owned(), &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }

    fn open_code_symbol(
        &mut self,
        symbol_entity_id: &str,
    ) -> crate::quick_switcher::NavDispatchOutcome {
        // WP-KERNEL-012 MT-079 (AC-079-4): the code editor pane is now MOUNTED (PaneType::CodeSymbol ->
        // CodeEditorPaneMount), so this arm opens + focuses the REAL code editor pane instead of the
        // EditorPaneNotMounted seam. Open the symbol on the code surface through the SAME tab-open
        // primitive every other arm uses.
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| symbol_entity_id.to_owned());
        match self.open_navigator_tab(PaneType::CodeSymbol, symbol_entity_id.to_owned(), &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }

    fn open_work_packet(&mut self, wp_id: &str) -> crate::quick_switcher::NavDispatchOutcome {
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| wp_id.to_owned());
        match self.open_navigator_tab(PaneType::KernelDcc, format!("WP:{wp_id}"), &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }

    fn open_micro_task(
        &mut self,
        mt_id: &str,
        wp_id: Option<&str>,
    ) -> crate::quick_switcher::NavDispatchOutcome {
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| mt_id.to_owned());
        let content_id = format!("MT:{}:{mt_id}", wp_id.unwrap_or_default());
        match self.open_navigator_tab(PaneType::KernelDcc, content_id, &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }

    fn open_user_manual_page(&mut self, slug: &str) -> crate::quick_switcher::NavDispatchOutcome {
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| slug.to_owned());
        match self.open_navigator_tab(PaneType::UserManual, slug.to_owned(), &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }

    fn open_wiki_page(&mut self, projection_id: &str) -> crate::quick_switcher::NavDispatchOutcome {
        let label = self
            .nav_pending_label
            .clone()
            .unwrap_or_else(|| projection_id.to_owned());
        match self.open_navigator_tab(PaneType::LoomWikiPage, projection_id.to_owned(), &label) {
            Some(surface) => crate::quick_switcher::NavDispatchOutcome::Opened { surface },
            None => crate::quick_switcher::NavDispatchOutcome::NoTargetPane,
        }
    }
}

impl eframe::App for HandshakeApp {
    /// MT-027 live steering: inject any model actions queued by the MCP server into THIS frame's input,
    /// BEFORE egui processes the frame, so a connected out-of-process client steers the running shell.
    /// `raw_input_hook` is eframe's supported pre-frame seam; pushing `AccessKitActionRequest` / `Text`
    /// events here is exactly the path the in-process steering test proves.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let events = match self.mcp_action_channel.lock() {
            Ok(mut chan) => chan.drain_into_events(),
            Err(poisoned) => poisoned.into_inner().drain_into_events(),
        };
        if !events.is_empty() {
            raw_input.events.extend(events);
        }
    }

    /// WP-KERNEL-012 MT-094 (§6.13.3 clean-shutdown rule, AC-014-4): on a clean Handshake exit, send the
    /// explicit `Shutdown` control message to the Palmistry watcher so it exits cleanly and records NO
    /// crash (a clean shutdown is NOT a crash). eframe calls this once on shutdown (after `save`). Taking
    /// the handle here performs the bounded reap; the handle's `Drop` is the backstop if `on_exit` is
    /// ever skipped, so the watcher never orphans either way. App-owned Tokio tasks are also drained
    /// before their runtime is dropped, avoiding shutdown-time background panics in headless and GUI
    /// teardown paths.
    fn on_exit(&mut self) {
        self.shutdown_background_runtime_tasks();
        if let Some(handle) = self.palmistry.take() {
            tracing::info!(
                "clean Handshake exit — sending Shutdown to the Palmistry watcher (§6.13.3)"
            );
            let outcome = handle.shutdown();
            tracing::info!(?outcome, "palmistry watcher reaped on clean exit");
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // WP-KERNEL-012 MT-084 (D2 internal_diagnostics, Tier 2 — UI-thread heartbeat, §5.8.2).
        //
        // Bump the heartbeat FIRST, at the very TOP of `update`, BEFORE `self.ui(ctx)`. This ordering
        // is deliberate and load-bearing: the counter advances for THIS frame even if `self.ui(ctx)`
        // below later panics or hangs. So a freeze INSIDE `ui()` surfaces as a STALE heartbeat (the
        // counter stopped advancing) — which is exactly the signal Palmistry (Tier 3) detects from
        // outside the hung process (RISK-004-4: the bump must be on the UI thread, on the frame path).
        //
        // The write is wait-free + allocation-free: a single seqlock store of two integers into the
        // MT-081 ring header (no record-buffer lock, no `format!`, no heap alloc — RISK-004-3). It is a
        // silent no-op when no ring writer is installed (headless/test shell — AC-004-5).
        self.bump_heartbeat();
        if let Some(handle) = self.palmistry.as_mut() {
            crate::diagnostics::drain_palmistry_child_watch_commands(handle);
        }

        // Idle liveness (RISK-004-1 / AC-004-4): egui only repaints on demand, so without this an
        // idle-but-healthy app would stop bumping the counter and look frozen to Palmistry. Schedule a
        // wake at the bounded ~250ms cadence (between Palmistry's poll and the freeze threshold) so the
        // frame loop — and therefore the heartbeat — keeps ticking at >= ~4 Hz with no input. This is a
        // schedule-only call: it steals no focus and grabs no input (HBR-QUIET preserved).
        ctx.request_repaint_after(HEARTBEAT_IDLE_REPAINT_INTERVAL);

        // WP-KERNEL-012 MT-085 (D2 internal_diagnostics, Tier 2 — frame-time, §5.8.2/§5.8.4).
        //
        // Measure the WORK TIME of this frame — the wall duration of `self.ui(ctx)` (the actual
        // per-frame work), NOT the inter-frame PERIOD. This is the load-bearing design choice
        // (RISK-005-1 / AC-005-3): egui only repaints on demand, and MT-084 schedules an idle keep-alive
        // repaint every ~250ms so the heartbeat keeps advancing. If we timed the PERIOD, that idle 250ms
        // gap (time spent WAITING for the next frame, doing nothing) would look like a 250ms "slow frame"
        // and FLOOD the bounded ring with false SlowFrame events. Timing the WORK inside `update` is
        // small on an idle frame regardless of the wait, so the idle keep-alive never flags.
        let work_start = std::time::Instant::now();
        // MT-085 test seam: a kittest may inject synthetic work so a REAL slow frame flags from this live
        // path (AC-005-2). `0` in production — a single `if` with no sleep, so the measurement is
        // unchanged for the shipped binary. The sleep lands INSIDE the measured window, exactly like
        // heavy real UI work.
        if self.extra_frame_work_micros > 0 {
            std::thread::sleep(std::time::Duration::from_micros(
                self.extra_frame_work_micros,
            ));
        }
        self.ui(ctx);
        let frame_work = work_start.elapsed();
        // Record the frame's work time + (debounced) emit a typed SlowFrame if it exceeded the slow
        // threshold. Allocation-free + lock-free on a fast frame (the common case); the ring write only
        // happens on a debounced slow frame. The emit goes through the OPEN MT-082 recorder, so it lands
        // in the in-process buffer (panel) AND the MT-081 ring (Palmistry) when a writer is installed.
        self.frame_timer.record_frame_live(frame_work);

        // WP-KERNEL-012 MT-086 (D2 internal_diagnostics, Tier 2 — resource counters, §5.8.2/§5.8.4).
        //
        // Sample per-process CPU%/RSS at a BOUNDED ~1s cadence (NOT every frame — RISK-006-2): on a
        // non-sampling frame this is a single `Instant` comparison and nothing else; at the bounded
        // cadence it refreshes ONLY this process's pid (RISK-006-3 — never the whole system table) and
        // emits one typed `ResourceSample` event through the OPEN MT-082 recorder (in-process buffer +
        // the MT-081 ring when a writer is installed). The single-process refresh at a ~1s cadence is
        // cheap enough to run inline on the frame without stuttering (no extra thread — AC-006-4). CPU%
        // needs the interval between two refreshes to be meaningful, which the cadence provides.
        self.resource_sampler.maybe_sample_live();

        // MT-027: publish the just-rendered UI tree into the shared MCP snapshot slot so the server's
        // `list_widgets` returns the live tree and `click_widget`/`set_value` resolve against it. This
        // runs a side-effect-free capture pass (guarded by `capturing_snapshot`) and requests a repaint
        // so queued model actions are drained promptly even when the UI is otherwise idle.
        if self.mcp_server.is_some() {
            self.refresh_mcp_snapshot();
            let has_pending = self
                .mcp_action_channel
                .lock()
                .map(|c| c.pending() > 0)
                .unwrap_or(false);
            if has_pending {
                ctx.request_repaint();
            }
        }
    }
}

impl Drop for HandshakeApp {
    fn drop(&mut self) {
        self.shutdown_background_runtime_tasks();
    }
}
