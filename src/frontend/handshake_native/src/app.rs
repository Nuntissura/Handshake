//! Handshake native GUI shell (MT-002).
//! Opens the real wgpu window with a top title bar, bottom status bar (live backend /health), and
//! a central work-surface placeholder. Render logic lives in `ui()` (no eframe::Frame) so it is
//! driveable headlessly by egui_kittest.

use crate::backend_client::{self, HealthInfo, HEALTH_URL};
use crate::error::AppError;
use crate::theme::{self, HsTheme};

/// Stable AccessKit id for the theme-toggle button. egui maps `accesskit::NodeId` directly
/// from an `egui::Id`'s u64 value (egui 0.33 `Id::accesskit_id`), so a fixed-value `Id`
/// yields a fixed `NodeId`. The contract requires `NodeId(10)` for out-of-process/kittest
/// steering.
const THEME_TOGGLE_NODE_ID: u64 = 10;

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
        }
    }

    /// Active base theme (for tests / future settings binding).
    pub fn current_theme(&self) -> HsTheme {
        self.current_theme
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

        if response.clicked() {
            self.current_theme = self.current_theme.toggled();
            // Next frame's apply_theme_if_changed picks up the new palette.
            ui.ctx().request_repaint();
        }
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
                ui.heading("Handshake");
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
            ui.label(text);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("(work surface — dockable panes arrive in later MTs)");
        });

        if matches!(self.health_status, HealthDisplayState::Loading) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl eframe::App for HandshakeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}
