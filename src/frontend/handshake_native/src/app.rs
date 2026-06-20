//! Handshake native GUI shell (MT-002).
//! Opens the real wgpu window with a top title bar, bottom status bar (live backend /health), and
//! a central work-surface placeholder. Render logic lives in `ui()` (no eframe::Frame) so it is
//! driveable headlessly by egui_kittest.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::accessibility::{self, ChromeWidget};
use crate::backend_client::{self, HealthInfo, HEALTH_URL};
use crate::error::AppError;
use crate::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneId, PaneRecord, PaneRegistry,
    PaneRenderContext, PaneType, PlaceholderPaneFactory,
};
use crate::popout_window::{popout_title_for, PopOutGeometry, PopOutManager};
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
                "default-project",
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
        }
    }

    /// Test/headless constructor: preset health, no runtime spawn, no backend needed.
    pub fn with_health(state: HealthDisplayState) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("build tokio runtime");
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
        // Apply theme tokens at the top of the frame so all panels below render themed.
        self.apply_theme_if_changed(ctx);

        egui::TopBottomPanel::top("handshake_title_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.title_identity(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.theme_toggle(ui);
                });
            });
        });

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
