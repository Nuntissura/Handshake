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
    LayoutPersistenceManager, LayoutPersistenceStatus, LayoutSnapshot, LayoutTransport,
    PopOutSnapshot,
};
use crate::module_switcher::{ModuleId, ModuleSwitcher, ModuleSwitcherColors};
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

/// The project id a fresh shell shows before any project switch. Must match the `project_id` the
/// default panes are seeded with (see [`default_panes`]) so the captured snapshot is self-consistent.
pub const DEFAULT_PROJECT_ID: &str = "default-project";

/// Debounce quiet period for the layout save: a flush fires this long after the LAST layout-affecting
/// change, so rapid divider drags / tab reorders coalesce into one backend `PUT` (MT-009 contract:
/// "a short debounce so rapid drags coalesce"). 600ms balances responsiveness against `PUT` volume.
pub const LAYOUT_SAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(600);

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
    /// + active tab via [`HandshakeApp::set_module`]. Serialized into the layout snapshot as
    /// `active_module` by MT-009, so the module survives a project switch and an app restart.
    module_switcher: ModuleSwitcher,
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
        Self {
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
        }
    }

    /// Shared handle to the pane registry (for tests and future concurrent agent/operator wiring).
    pub fn pane_registry(&self) -> Arc<Mutex<PaneRegistry>> {
        self.pane_registry.clone()
    }

    /// Active base theme (for tests / future settings binding).
    pub fn current_theme(&self) -> HsTheme {
        self.current_theme
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
        self.split_weights = snapshot.split_weights;
        self.active_pane = snapshot.active_pane;
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

    /// Render the bottom status bar's backend-health line as a real egui widget with the fixed
    /// `ChromeWidget::StatusBar` id, then emit a LIVE AccessKit node (Role::Status + author_id
    /// `shell.chrome.status-bar` + the current health text as label). Closes the MT-002 gap where
    /// the status line was a plain `ui.label` with no stable author_id in the live tree.
    fn status_indicator(&self, ui: &mut egui::Ui, text: &str) {
        let chrome = ChromeWidget::StatusBar;
        let id = chrome.egui_id();

        let font = egui::TextStyle::Body.resolve(ui.style());
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            font,
            ui.visuals().text_color(),
        );
        let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter()
                .galley(rect.min, galley, ui.visuals().text_color());
        }
        ui.interact(rect, id, egui::Sense::hover());

        accessibility::emit_chrome_node(ui.ctx(), chrome, id, text);
    }

    fn poll_health(&mut self) {
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

        egui::TopBottomPanel::bottom("handshake_status_bar").show(ctx, |ui| {
            let text = match &self.health_status {
                HealthDisplayState::Loading => "Backend: Loading...".to_owned(),
                HealthDisplayState::Ok(h) => {
                    format!("Backend: OK (db {}, migration {:?})", h.db_status, h.migration_version)
                }
                HealthDisplayState::Error(e) => format!("Backend: error: {e}"),
            };
            self.status_indicator(ui, &text);
        });

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
                );
            });

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

            // Tab bar strip on top (same as the docked pane), if this pane has tab state.
            let full = ui.available_rect_before_wrap();
            let tab_h = TAB_BAR_HEIGHT.min(full.height());
            let tab_rect = egui::Rect::from_min_max(
                full.min,
                egui::pos2(full.right(), full.top() + tab_h),
            );
            let body_rect =
                egui::Rect::from_min_max(egui::pos2(full.left(), full.top() + tab_h), full.max);

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
                let _resp = TabBar::show(&mut tab_ui, tab_state, tab_colors);
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}
