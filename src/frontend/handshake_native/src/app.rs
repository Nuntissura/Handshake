//! Handshake native GUI shell (MT-002).
//! Opens the real wgpu window with a top title bar, bottom status bar (live backend /health), and
//! a central work-surface placeholder. Render logic lives in `ui()` (no eframe::Frame) so it is
//! driveable headlessly by egui_kittest.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::accessibility::{self, ChromeWidget};
use crate::backend_client::{self, HealthInfo, WorkbenchLayoutClient, HEALTH_URL};
use crate::error::AppError;
use crate::layout_persistence::{
    DrawersState, LayoutPersistenceManager, LayoutPersistenceStatus, LayoutSnapshot, LayoutTransport,
    PopOutSnapshot,
};
use crate::event_bus::{new_shell_event_bus, ShellEvent, ShellEventReceiver, ShellEventSender};
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
use crate::split_layout::{DividerColors, SplitDragState, SplitLayoutWidget, SplitWeights};
use crate::tab_bar::{TabBar, TabBarColors, TabBarState, TabState, TAB_BAR_HEIGHT};
use crate::theme::{self, HsTheme};
use crate::top_menu_bar::{MenuBar, MenuBarAction, MenuBarState};

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

/// Debounce quiet period for the settings save (MT-018, red-team R2): a `PUT /workspaces/{id}/settings`
/// fires this long after the LAST settings change so rapid keybinding edits coalesce into one request.
/// 500ms per the MT implementation note. A dialog close FLUSHES any pending save immediately (MC2).
pub const SETTINGS_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(500);

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
    fn load(&self, _workspace_id: &str) -> Result<Option<serde_json::Value>, crate::layout_persistence::LayoutError> {
        Ok(None)
    }
    fn save(&self, _workspace_id: &str, _layout_state: serde_json::Value) -> Result<(), crate::layout_persistence::LayoutError> {
        Ok(())
    }
}

pub enum HealthDisplayState {
    Loading,
    Ok(HealthInfo),
    Error(String),
}

pub struct HandshakeApp {
    health_status: HealthDisplayState,
    rt: tokio::runtime::Runtime,
    health_handle: Option<tokio::task::JoinHandle<Result<HealthInfo, AppError>>>,
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
    confirm_discard:
        Option<(crate::stash_shelf::DrawerCardKind, crate::stash_shelf::DrawerActionTarget)>,
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

/// The four seed panes for a fresh work surface. Mirrors the React `DEFAULT_PANES` four-pane shape
/// (`app/src/App.tsx`): pane-a..pane-d, all System-authored, Unlocked, and Clean.
fn default_panes() -> Vec<PaneRecord> {
    let seeds: [(&str, PaneType); 4] = [
        ("pane-a", PaneType::Workspace),
        ("pane-b", PaneType::InferenceLab),
        ("pane-c", PaneType::MediaDownloader),
        ("pane-d", PaneType::FontManager),
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
        PaneType::Placeholder(String::new()),
    ];
    let mut map: HashMap<PaneType, Box<dyn PaneFactory>> = HashMap::new();
    for v in variants {
        map.insert(v.clone(), Box::new(PlaceholderPaneFactory::new(v)));
    }
    map
}

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
/// stay aligned (the live-tree test asserts pane-a..pane-d each have a tab bar).
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

impl HandshakeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::install_fonts(&cc.egui_ctx);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build tokio runtime");
        // Fire-once, non-blocking health poll: window opens immediately, label shows Loading...
        let health_handle = Some(rt.spawn(async { backend_client::fetch_health(HEALTH_URL).await }));
        // Fire-once, non-blocking workspace list fetch (MT-011): the shell opens immediately with the
        // seeded default-project tab; when the fetch resolves, the real workspace tabs replace it.
        let workspaces_handle = Some(rt.spawn(async {
            fetch_workspaces(backend_client::BACKEND_BASE_URL).await
        }));
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
        > = Some(Arc::new(crate::quick_switcher::LoomGraphSearchClient::production(
            rt_handle.clone(),
        )));
        // MT-018: the REAL settings transport, bridged onto the app runtime (MT-009 pattern).
        let settings_transport: Option<Arc<dyn crate::workspace_settings::SettingsTransport>> =
            Some(Arc::new(crate::workspace_settings::SettingsClient::production(
                rt_handle.clone(),
            )));
        // MT-014 FIX-B: the in-process shell event bus, constructed once at app construction (the
        // "subscribe at app/LeftRail construction" control). Drained each frame in `ui()`.
        let (event_bus_tx, event_bus_rx) = new_shell_event_bus();
        let mut app = Self {
            health_status: HealthDisplayState::Loading,
            rt,
            health_handle,
            // Desktop default mirrors the React app's dark default.
            current_theme: HsTheme::Dark,
            last_applied_theme: None,
            pane_registry: Arc::new(Mutex::new(seeded_registry())),
            factories: build_default_factories(),
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
            monitor_extent: DEFAULT_MONITOR_EXTENT,
            last_seen_layout: None,
            project_tabs: default_project_tabs(),
            workspaces_handle,
            // The default seed pane (`pane-a`) is the React `MAIN` module, so the switcher starts on MAIN.
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
            pending_theme_change: None,
            about_open: false,
            reset_layout_pending: false,
            loom_block_client: Some(crate::backend_client::LoomBlockClient::production(
                rt_handle.clone(),
            )),
            rename_cell: Arc::new(Mutex::new(None)),
            pending_rename: None,
            rename_error: None,
            source_control_client: Some(crate::backend_client::SourceControlClient::production(
                rt_handle.clone(),
            )),
            scm_receipt_cell: Arc::new(Mutex::new(None)),
            scm_text_cell: Arc::new(Mutex::new(None)),
            scm_display_text: None,
            scm_error: None,
            canvas_client: Some(crate::backend_client::CanvasClient::production(rt_handle.clone())),
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
        };
        app.spawn_mcp_server();
        app
    }

    /// Bind the MCP transport (MT-027) on the app's tokio runtime and store the handle. Logged + non-fatal
    /// on failure so the shell always opens. Only the production shell (with a multi-thread runtime) binds;
    /// the headless/test shell drives the server's `dispatch_request` directly instead.
    fn spawn_mcp_server(&mut self) {
        let token = self.mcp_token.clone();
        let snapshot = self.mcp_snapshot.clone();
        let channel = self.mcp_action_channel.clone();
        let capture = crate::mcp::SwarmMcpServer::os_window_capture();
        let result = self
            .rt
            .block_on(async move { crate::mcp::SwarmMcpServer::bind(token, snapshot, channel, capture).await });
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
        Self {
            health_status: state,
            rt,
            health_handle: None,
            current_theme: HsTheme::Dark,
            last_applied_theme: None,
            pane_registry: Arc::new(Mutex::new(seeded_registry())),
            factories: build_default_factories(),
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
            pending_theme_change: None,
            about_open: false,
            reset_layout_pending: false,
            // Headless/test shell: no runtime to bridge the rename PATCH onto, so rename is a disclosed
            // no-op (a test injects a runtime via `set_runtime_handle` if it wants live rename).
            loom_block_client: None,
            rename_cell: Arc::new(Mutex::new(None)),
            pending_rename: None,
            rename_error: None,
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

    /// Active base theme (for tests / future settings binding).
    pub fn current_theme(&self) -> HsTheme {
        self.current_theme
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
            id if id.starts_with("editor.") => {
                // The native editor surface is a future MT; an editor command dispatched through the
                // palette is guarded (red-team R5/MC5) so it never panics with no active document. Today
                // there is no active document, so we log + skip rather than fake an edit.
                tracing::warn!("palette: editor command {id} skipped (no active editor document)");
                false
            }
            other => {
                tracing::warn!("palette: unknown command id {other}");
                false
            }
        }
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
        self.loom_block_client =
            Some(crate::backend_client::LoomBlockClient::production(handle.clone()));
        // MT-021: bridge the SCM + canvas off-thread clients onto the injected runtime too, so an
        // injected-runtime shell (kittest) gets live source-control + canvas calls.
        self.source_control_client =
            Some(crate::backend_client::SourceControlClient::production(handle.clone()));
        self.canvas_client = Some(crate::backend_client::CanvasClient::production(handle.clone()));
        // MT-022: the rail makes NO backend call (AC-022-9), so there is no rail transport to bridge onto
        // the runtime — the rail emits its RailQuery intent into `search_rail_query` silently.
        // MT-023: bridge the drawer-data client onto the injected runtime so an injected-runtime shell
        // (kittest) gets live off-thread card fetches.
        self.drawer_data_client =
            Some(crate::backend_client::DrawerDataClient::production(handle.clone()));
        // MT-024: bridge the drawer card-action client onto the injected runtime so an injected-runtime
        // shell (kittest) gets live off-thread pin/discard/stow/attach-evidence dispatch.
        self.drawer_action_client =
            Some(crate::backend_client::DrawerActionClient::production(handle.clone()));
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

    /// Test-only: point the MT-021 backend clients (SCM, canvas, Loom-block) at an arbitrary `base_url`
    /// bridged onto `handle`, so a test can drive `apply_*_event` against a localhost capture server and
    /// assert the EXACT URL + body reaches the wire through the REAL app dispatch path. Production code
    /// never calls this (it uses [`set_runtime_handle`](Self::set_runtime_handle) -> hardcoded backend
    /// URL); it exists so the MAJOR #1/#2/#3 "client genuinely consumed by the app" proof is end-to-end.
    pub fn set_backend_base_url_for_test(&mut self, base_url: &str, handle: tokio::runtime::Handle) {
        self.loom_block_client =
            Some(crate::backend_client::LoomBlockClient::new(base_url, handle.clone()));
        self.source_control_client =
            Some(crate::backend_client::SourceControlClient::new(base_url, handle.clone()));
        self.canvas_client = Some(crate::backend_client::CanvasClient::new(base_url, handle.clone()));
        // MT-024: the drawer card-action client too, so the confirm-discard -> DELETE wire test (mirroring
        // PROOF-024-2(e)) can drive the REAL dispatch path against a localhost capture server.
        self.drawer_action_client =
            Some(crate::backend_client::DrawerActionClient::new(base_url, handle.clone()));
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

    /// Map a [`crate::quick_switcher::QuickSwitcherTarget`] to the `(PaneType, content_id)` tab that
    /// realizes it on a pane (the native peer of the React `onOpen*` callbacks). `Unsupported` returns
    /// `None` (the row was disabled, so this is never reached for it). The Kernel-DCC targets encode the
    /// WP/MT focus in the tab `content_id` (`"WP:{id}"` / `"MT:{wp?}:{id}"`) so a single Kernel-DCC pane
    /// can focus the right entity without a separate focus-state field.
    fn target_to_tab(
        target: &crate::quick_switcher::QuickSwitcherTarget,
    ) -> Option<(PaneType, String)> {
        use crate::quick_switcher::QuickSwitcherTarget as T;
        match target {
            T::UserManual { slug } => Some((PaneType::UserManual, slug.clone())),
            T::WikiPage { projection_id } => {
                Some((PaneType::LoomWikiPage, projection_id.clone()))
            }
            T::Document { document_id } => Some((PaneType::AtelierEditor, document_id.clone())),
            T::LoomBlock { block_id } => Some((PaneType::LoomBlock, block_id.clone())),
            T::CodeSymbol { symbol_entity_id } => {
                Some((PaneType::CodeSymbol, symbol_entity_id.clone()))
            }
            T::WorkPacket { wp_id } => Some((PaneType::KernelDcc, format!("WP:{wp_id}"))),
            T::MicroTask { mt_id, wp_id } => Some((
                PaneType::KernelDcc,
                format!("MT:{}:{mt_id}", wp_id.clone().unwrap_or_default()),
            )),
            T::Unsupported => None,
        }
    }

    /// Open a selected Loom-graph hit (MT-017): record the durable recent (`POST recents`, OFF the egui
    /// UI thread — HBR-QUIET) with an immediate optimistic local prepend, then JUMP by opening the hit's
    /// typed target as a tab on the active (or fallback) pane. Returns `true` when the pane/tab state
    /// changed (so the caller can repaint + let the layout change-detector schedule a save). An
    /// `Unsupported` target is a safe no-op (the row was disabled and never reaches here).
    ///
    /// The network POST is NEVER awaited on the frame thread. The optimistic local recents update uses
    /// the hit's own [`hit_key`](crate::quick_switcher::hit_key) (which equals the backend-confirmed key
    /// for any well-formed hit) so the recents-first ordering is correct on the very next open without
    /// waiting for the round-trip; the spawned task writes the backend-confirmed key (or an error) into
    /// `quick_switcher_record_recent_cell`, drained next frame by [`drive_quick_switcher`] to reconcile
    /// the key and surface failures via `recents_error` (red-team MC3).
    fn open_switcher_hit(&mut self, hit: &crate::quick_switcher::LoomGraphSearchHit) -> bool {
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
        if let (Some(transport), Some(handle)) =
            (self.quick_switcher_transport.clone(), self.runtime_handle.clone())
        {
            let cell = self.quick_switcher_record_recent_cell.clone();
            let hit = hit.clone();
            handle.spawn(async move {
                let result = transport
                    .record_recent(&workspace, &hit)
                    .map_err(|e| e.to_string());
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(result);
                }
            });
        }

        // 3. Navigate: open the hit's typed target as a tab on the target pane.
        let target = crate::quick_switcher::open_target_for_hit(hit);
        let Some((pane_type, content_id)) = Self::target_to_tab(&target) else {
            return false;
        };
        let Some(pane_id) = self.module_target_pane() else {
            tracing::warn!("quick switcher: no pane to open target on");
            return false;
        };
        if let Some(bar) = self.tab_bar_states.get_mut(&pane_id) {
            let tab = TabState {
                pane_type,
                content_id: Some(content_id),
                pinned: false,
                dirty: false,
                label_override: Some(hit.title.clone()),
            };
            // insert_tab de-duplicates by (pane_type, content_id) and returns the (existing or new)
            // index; activate it so the jump lands on the opened tab.
            let idx = bar.insert_tab(tab);
            bar.activate(idx);
        }
        self.active_pane = Some(pane_id);
        true
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
                    handle.spawn(async move {
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
                handle.spawn(async move {
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
                if self.open_switcher_hit(&hit) {
                    ctx.request_repaint();
                }
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
        if let Some(result) = self.drawer_action_cell.lock().ok().and_then(|mut s| s.take()) {
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
            self.drawer.clamp_height(avail, crate::search_rail::RAIL_HEIGHT + 24.0);
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
                    tracing::debug!("drawer: Mail card clicked — no backend / pane yet (Coming soon)");
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
                    if Self::confirm_button(ui, "Discard", "hsk.drawer.confirm.ok", colors.error_text) {
                        do_confirm = true;
                    }
                    if Self::confirm_button(ui, "Cancel", "hsk.drawer.confirm.cancel", colors.card_text)
                    {
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
        client.discard(&t.workspace_id, &t.block_id, self.drawer_action_cell.clone());
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
                    self.drawer_action_error =
                        Some("This card has no block to act on".to_owned());
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
                        let job_id = self.active_job_id.clone().expect("active job checked above");
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
        self.drawer_intents.lock().map(|i| i.clone()).unwrap_or_default()
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
        self.confirm_discard.as_ref().map(|(_, t)| t.block_id.as_str())
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
        client.fetch_count(&workspace_id, crate::backend_client::DrawerDataKind::Notes, cell);
    }

    /// Open the pane a drawer card links to (AC-023-12): Agenda → the daily-journal pane, Lists/Notes →
    /// a Loom-block collection pane. Routes through the same `open_content_on_active_pane` primitive the
    /// command palette / menu navigation use, so the card opens a REAL pane (not a fake nav).
    fn open_drawer_card_pane(&mut self, kind: crate::stash_shelf::DrawerCardKind) -> bool {
        use crate::stash_shelf::DrawerCardKind;
        match kind {
            DrawerCardKind::Agenda => self.open_content_on_active_pane(PaneType::LoomDailyJournal, None),
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
    pub fn set_workspace_theme_for_test(&mut self, theme: crate::workspace_settings::WorkspaceTheme) {
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
        self.workspace_settings.set_chord(action_id, chord.to_owned());
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
            handle.spawn(async move {
                let result = transport.save(&workspace, blob).map_err(|e| e.to_string());
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(result);
                }
                in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        }
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
                        let fallback = crate::workspace_settings::default_workspace_settings_state();
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
                        self.pending_theme_change = Some(self.workspace_settings.theme.to_hs_theme());
                        self.view_mode = match self.workspace_settings.view_mode {
                            crate::workspace_settings::SettingsViewMode::Nsfw => ViewMode::Nsfw,
                            crate::workspace_settings::SettingsViewMode::Sfw => ViewMode::Sfw,
                        };
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
                        handle.spawn(async move {
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
        let view = crate::settings_dialog::SettingsView {
            open_count: self.settings_open_count,
            settings: &self.workspace_settings,
            persist_error: self.settings_persist_error.as_deref(),
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
                    let ni = if forward { (i + 1) % len } else { (i + len - 1) % len };
                    ids[ni].clone()
                }
                // Active pane not in the tab-bar set (shouldn't happen): fall back to the first/last.
                None => if forward { ids[0].clone() } else { ids[ids.len() - 1].clone() },
            },
            None => if forward { ids[0].clone() } else { ids[ids.len() - 1].clone() },
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
            MenuBarAction::QuitApp => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                false
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
            | MenuBarAction::ToggleFileDrawer
            | MenuBarAction::OpenTerminal => false,
        }
    }

    /// Build the per-frame [`MenuBarState`] the menu bar reads for checkmarks + enable/disable.
    fn menu_bar_state(&self) -> MenuBarState {
        let has_active_tab = self
            .module_target_pane()
            .and_then(|p| self.tab_bar_states.get(&p))
            .map(|bar| bar.active().is_some())
            .unwrap_or(false);
        MenuBarState {
            theme_is_dark: self.current_theme == HsTheme::Dark,
            view_mode_is_nsfw: self.view_mode == ViewMode::Nsfw,
            project_drawer_open: self.left_rail_open,
            bottom_drawer_open: self.bottom_drawer_open,
            has_active_tab,
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
        self.tab_bar_states
            .keys()
            .min()
            .cloned()
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
            let Some(bar) = self.tab_bar_states.get(pane_id) else { continue };
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
            LeftRailEvent::OpenBookmark { document_id, block_id } => {
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
            LeftRailEvent::RenameBlock { block_id, current_title } => {
                // Open the small inline rename dialog seeded with the current title; the dialog confirm
                // spawns the verified PATCH off the UI thread (see `drive_rename` + the dialog render).
                self.rename_error = None;
                self.pending_rename = Some(PendingRename {
                    block_id,
                    text: current_title,
                });
                true
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
        let delivered = self.rename_cell.lock().ok().and_then(|mut slot| slot.take());
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
                    client.rename_block(&ws, &pending.block_id, &new_title, self.rename_cell.clone());
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
                client.stage_paths(ScmWriteOp::Stage, repo_path, &path, self.scm_receipt_cell.clone());
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
            E::Rename { block_id, current_title } => {
                self.rename_error = None;
                self.pending_rename = Some(PendingRename { block_id, text: current_title });
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
    fn open_content_on_active_pane(&mut self, pane_type: PaneType, content_id: Option<String>) -> bool {
        let Some(target) = self.module_target_pane() else {
            return false;
        };
        self.active_pane = Some(target.clone());
        if let Some(bar) = self.tab_bar_states.get_mut(&target) {
            let mut tab = TabState::new(pane_type);
            tab.content_id = content_id;
            bar.insert_tab(tab);
            return true;
        }
        false
    }

    /// Reset the live work-surface layout to the seeded default for `project_id` (MT-011), the native
    /// mirror of React's `defaultWorkbenchLayoutState(projectId)`. Rebuilds the four default panes
    /// (re-stamped to `project_id`), the default per-pane tab bars, the default split weights, clears
    /// the active pane and all pop-outs, and points `active_project_id` at the new project.
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
        // A fresh default work surface starts on the MAIN module (the default seed pane's module), so the
        // switcher highlight resets too. A subsequent lifecycle load overwrites this if the entered
        // project has a stored `active_module`.
        self.module_switcher.set_active(ModuleId::Main);
        // Rebuild the registry from the default panes, re-stamped to the entered project so the captured
        // snapshot's pane records are self-consistent with `active_project_id`.
        {
            let mut guard = self.pane_registry.lock().expect("pane registry mutex poisoned");
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
        self.save_layout_now();
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
        let finished = self.workspaces_handle.as_ref().is_some_and(|h| h.is_finished());
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
                Err(e) => self.project_tabs.apply_fetch_error(format!("join error: {e}")),
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
            let guard = self.pane_registry.lock().expect("pane registry mutex poisoned");
            guard.iter().map(|(id, rec)| (id.clone(), rec.clone())).collect()
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
            let mut guard = self.pane_registry.lock().expect("pane registry mutex poisoned");
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

    /// Load and apply the persisted layout for `project_id` (MT-009), with the documented fallback
    /// chain (delegated to the manager): a valid stored blob is applied; a corrupt/foreign/wrong-project
    /// one falls back to the manager's in-memory last-known-good, then to the seeded default layout
    /// (which is infallible). `monitor_extent` is the full all-monitors rect used for the restore
    /// clamp. Returns `true` if a stored snapshot was applied, `false` if the default was kept.
    ///
    /// The manager never returns an unvalidated snapshot, so `apply_layout_snapshot` here is always
    /// applied to a validated layout — no infinite restore loop. Marks `loaded_project_id` so the
    /// lifecycle does not reload the same project every frame.
    pub fn load_layout(&mut self, project_id: &str, monitor_extent: egui::Rect) -> bool {
        let loaded = {
            let mut mgr = self.layout_manager.lock().expect("layout manager mutex poisoned");
            mgr.load(project_id)
        };
        self.loaded_project_id = Some(project_id.to_owned());
        match loaded {
            Ok(Some(snapshot)) => {
                // The manager already validated it; apply (which re-validates + clamps, all-or-nothing).
                self.apply_layout_snapshot(snapshot, monitor_extent).is_ok()
            }
            // No stored layout (first run) or a failed load with no LKG: keep the seeded default.
            Ok(None) | Err(_) => false,
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
        // ── 1. Load on first frame / project change ─────────────────────────────────────────────
        if self.loaded_project_id.as_deref() != Some(self.active_project_id.as_str()) {
            let project = self.active_project_id.clone();
            let extent = self.monitor_extent;
            self.load_layout(&project, extent);
            // Re-baseline change detection to the just-loaded layout so a restore does not immediately
            // re-save itself as a "change".
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
        if changed || self.layout_dirty_signal {
            self.layout_dirty_signal = false;
            self.layout_manager
                .lock()
                .expect("layout manager mutex poisoned")
                .mark_dirty(now);
        }
        self.last_seen_layout = Some(current_layout);

        // ── 3. Debounced save off the UI thread ─────────────────────────────────────────────────
        let due = {
            let mgr = self.layout_manager.lock().expect("layout manager mutex poisoned");
            mgr.due_to_flush(now)
        };
        if due && !self.save_in_flight.swap(true, std::sync::atomic::Ordering::SeqCst) {
            // Capture the snapshot on the UI thread (it reads live shell state), then flush on a worker.
            let snapshot = self.capture_layout_snapshot();
            let manager = self.layout_manager.clone();
            let in_flight = self.save_in_flight.clone();
            // A plain OS thread (not a runtime worker): the transport's `block_on` is valid off-runtime,
            // so the network PUT runs here without blocking the egui UI thread. The manager handles
            // retry/LKG/status; the UI thread reads status next frame.
            std::thread::spawn(move || {
                {
                    let mut mgr = manager.lock().expect("layout manager mutex poisoned");
                    mgr.flush_if_due(now, &snapshot);
                }
                in_flight.store(false, std::sync::atomic::Ordering::SeqCst);
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
            let mut fonts = egui::FontDefinitions::default();
            // Regular face: front of the Proportional family so all default UI text renders Inter.
            fonts.font_data.insert(
                "Inter".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(INTER_REGULAR)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Inter".to_owned());

            // Bold face: registered under a named family so callers can request bold explicitly via
            // FontFamily::Name("Inter-Bold") without disturbing the default proportional rendering.
            fonts.font_data.insert(
                INTER_BOLD_FAMILY.to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(INTER_BOLD)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Name(INTER_BOLD_FAMILY.into()))
                .or_default()
                .insert(0, INTER_BOLD_FAMILY.to_owned());

            ctx.set_fonts(fonts);
            tracing::info!("bundled Inter fonts loaded (Regular + Bold)");
        }
        #[cfg(not(feature = "bundled-fonts"))]
        {
            let _ = ctx; // default fonts; nothing to do until MT-004 bundles the asset.
            tracing::debug!("bundled-fonts feature off; using eframe default fonts");
        }
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

        let galley =
            ui.painter().layout_no_wrap(label.to_owned(), egui::FontId::proportional(14.0), ui.visuals().text_color());
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
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            font,
            ui.visuals().text_color(),
        );
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
        let galley =
            ui.painter()
                .layout_no_wrap(text.to_owned(), font, ui.visuals().text_color());
        // Allocate with Sense::hover() (NOT click) so the auto-id allocation node is non-interactive;
        // the ONE clickable node is the interact at the FIXED chrome id below (which carries the stable
        // author_id), so the MT-025 interactive-naming gate stays green (no unnamed clickable node).
        let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().galley(rect.min, galley, ui.visuals().text_color());
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
        let menu =
            crate::context_menu::ContextMenu::new("statusbar").items(status_bar_context_items(&state));
        if let Some(confirmed_id) = menu.show_on(&seg_resp) {
            action = status_bar_action_for_id(confirmed_id, &state);
        }
        // Shift+F10 keyboard-open parity when the segment is focused.
        if seg_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
            crate::context_menu::request_open(ui.ctx(), seg_resp.id, seg_resp.rect.left_bottom());
        }
        action
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
                    tracing::warn!("statusbar.open_panel: segment {segment_id} has no related pane");
                    false
                }
            },
            A::Refresh => {
                // Re-fetch the segment's data. For the health segment, spawn a fresh /health poll on
                // the runtime (OFF the UI thread — HBR-QUIET), the same fire-once poll the ctor uses.
                if segment_id == "health" {
                    if let Some(handle) = self.runtime_handle.clone() {
                        self.health_status = HealthDisplayState::Loading;
                        self.health_handle = Some(
                            handle.spawn(async { backend_client::fetch_health(HEALTH_URL).await }),
                        );
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
            }
        }
    }

    /// Render the shell. Split from eframe::App::update so egui_kittest can drive it without a Frame.
    pub fn ui(&mut self, ctx: &egui::Context) {
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

        let menu_state = self.menu_bar_state();
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

        let status_text = match &self.health_status {
            HealthDisplayState::Loading => "Backend: Loading...".to_owned(),
            HealthDisplayState::Ok(h) => {
                format!("Backend: OK (db {}, migration {:?})", h.db_status, h.migration_version)
            }
            HealthDisplayState::Error(e) => format!("Backend: error: {e}"),
        };
        // The health segment is hidden iff the operator hid it via the status-bar menu (MT-021).
        let health_hidden = self.statusbar_hidden.contains("health");
        let status_action = egui::TopBottomPanel::bottom("handshake_status_bar")
            .show(ctx, |ui| {
                if health_hidden {
                    // Hidden segment: render a neutral non-interactive placeholder so the bar is never
                    // blank (and the segment can still be restored via a future settings surface).
                    ui.label("");
                    None
                } else {
                    self.status_bar_segment(ui, &status_text)
                }
            })
            .inner;
        if let Some(action) = status_action {
            if self.apply_status_bar_action(ctx, "health", &status_text, action) {
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
        // MT-021: drain any delivered SCM / canvas / Loom-node-flag off-thread results into panel state
        // (the network already ran off the UI thread — these just read the delivery cells, HBR-QUIET).
        self.drive_source_control(ctx);
        self.drive_canvas(ctx);
        self.drive_loom_node(ctx);

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
            // SplitLayoutWidget renders the four panes into their split rects and the two dividers.
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
                    accessibility::emit_pane_node(ui_ctx, pane_egui_id, pane_author_id, role, label);
                },
            );
        });

        // ── Apply MT-013 pane-header Lock/Unlock requests ───────────────────────────────────────────
        // A lock click from the pane header (pointer OR out-of-process AccessKit Click) toggles the
        // pane record's LockState in the registry (single source of truth). The change is picked up by
        // the MT-009 layout change-detector below (LockState is part of the captured pane record), so
        // it persists through the debounced save with no synchronous save here.
        if !lock_requests.is_empty() {
            let mut guard = self.pane_registry.lock().expect("pane registry mutex poisoned");
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
            let outcome = crate::command_palette::show(ctx, self.command_palette_open_count);
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
                self.monitor_extent =
                    egui::Rect::from_min_size(egui::Pos2::ZERO, monitor);
            }
        }
        self.drive_layout_persistence(std::time::Instant::now());

        // While a save is debounced/pending, keep frames coming so the debounce window actually
        // elapses even without further input (otherwise a quiescent app would never flush).
        let pending_save = self
            .layout_manager
            .lock()
            .expect("layout manager mutex poisoned")
            .is_dirty();
        if pending_save {
            ctx.request_repaint_after(LAYOUT_SAVE_DEBOUNCE);
        }

        if matches!(self.health_status, HealthDisplayState::Loading) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
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
            let header_rect = egui::Rect::from_min_max(
                full.min,
                egui::pos2(full.right(), full.top() + header_h),
            );
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

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
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
