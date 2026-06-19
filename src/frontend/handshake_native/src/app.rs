//! Handshake native GUI shell (MT-002).
//! Opens the real wgpu window with a top title bar, bottom status bar (live backend /health), and
//! a central work-surface placeholder. Render logic lives in `ui()` (no eframe::Frame) so it is
//! driveable headlessly by egui_kittest.

use crate::backend_client::{self, HealthInfo, HEALTH_URL};
use crate::error::AppError;

pub enum HealthDisplayState {
    Loading,
    Ok(HealthInfo),
    Error(String),
}

pub struct HandshakeApp {
    health_status: HealthDisplayState,
    rt: tokio::runtime::Runtime,
    health_handle: Option<tokio::task::JoinHandle<Result<HealthInfo, AppError>>>,
}

impl HandshakeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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

        egui::TopBottomPanel::top("handshake_title_bar").show(ctx, |ui| {
            ui.heading("Handshake");
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
