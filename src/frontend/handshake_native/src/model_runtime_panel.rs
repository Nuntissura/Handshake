//! Rust-native ModelRuntime registry control surface (WP-1 MT-014).
//!
//! Production fetches the PostgreSQL-backed registry projection from
//! `handshake_core` off the egui frame thread. The renderer never treats a
//! last-observed runtime UUID as live: only rows explicitly joined to a current
//! READY catalog entry carry `live_model_id`.

use std::sync::{Arc, Mutex, OnceLock};

use egui::accesskit;
use serde::{Deserialize, Serialize};

use crate::{
    pane_registry::{PaneFactory, PaneRenderContext, PaneType},
    rails::{
        scrollbar_rail_id, RailColors, RailDimensions, RailOrientation, ScrollbarRail,
        SCROLLBAR_V_NODE_IDS,
    },
};

pub const PROJECTION_SCHEMA_ID: &str = "hsk.model_runtime_registry_projection@3";
/// Stable namespace for ModelRuntime registry nodes. Every emitted key appends the authoritative
/// `PaneRenderContext.record.pane_id` before its control/row suffix so concurrent pane instances
/// remain deterministic and globally addressable.
pub const AUTHOR_ID_PREFIX: &str = "model-runtime.registry";

static PROCESS_LEDGER_NAVIGATION: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Test/diagnostic receipt for the last exact ledger URI selected. Production
/// rendering uses the transport-backed inline record query.
pub fn take_process_ledger_navigation_request() -> Option<String> {
    PROCESS_LEDGER_NAVIGATION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|mut request| request.take())
}

pub type ModelRuntimeRegistryCell =
    Arc<Mutex<Option<Result<ModelRuntimeRegistryProjection, String>>>>;
pub type ModelRuntimeControlCell = Arc<Mutex<Option<Result<ModelRuntimeControlReceipt, String>>>>;
pub type ProcessOwnershipCell =
    Arc<Mutex<Option<Result<ModelRuntimeProcessOwnershipRecord, String>>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeRegistryRowState {
    Live,
    Dormant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeRole {
    Completion,
    Embedding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ModelRuntimeSelectionPurpose {
    #[serde(rename = "application/default")]
    ApplicationDefault,
    #[serde(rename = "embeddings/default")]
    EmbeddingsDefault,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModelRuntimeValue<T> {
    Available { value: T },
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeKvInspection {
    pub bytes_used: u64,
    pub bytes_capacity: u64,
    pub prefix_cache_hit_rate: ModelRuntimeValue<f64>,
    pub quantization: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeLoraInspection {
    pub lora_id: String,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeSteeringInspection {
    pub steering_vector_id: String,
    pub layer: u32,
    pub intensity: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeActionAvailability {
    pub enabled: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ModelRuntimeControlAction {
    Quiesce,
    Unload,
    SwapCompatibleAdapter { target_adapter: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeControlReceipt {
    pub schema_version: u16,
    pub request_id: uuid::Uuid,
    pub model_id: String,
    pub result_model_id: Option<String>,
    pub action: ModelRuntimeControlAction,
    pub runtime_adapter: String,
    pub quiesced: bool,
    pub unloaded: bool,
    pub process_stop_committed: bool,
    pub registry_updated: bool,
    pub selection_rebound: bool,
    pub catalog_revision: Option<u64>,
    #[serde(default)]
    pub reconciliation_required: bool,
    #[serde(default)]
    pub reconciliation_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeRegistryRow {
    pub artifact_sha256: String,
    pub artifact_locator: String,
    pub display_label: String,
    pub selected_adapter: String,
    pub selection_revision: u64,
    pub selection_audit_event_ref: String,
    pub runtime_role: ModelRuntimeRole,
    pub default_selectable: bool,
    pub runtime_state: ModelRuntimeRegistryRowState,
    pub active_purposes: Vec<ModelRuntimeSelectionPurpose>,
    pub active_selection_revision: Option<u64>,
    #[serde(default)]
    pub selected: bool,
    pub live_model_id: Option<String>,
    pub canonical_artifact_path: ModelRuntimeValue<String>,
    pub kv_cache: ModelRuntimeValue<ModelRuntimeKvInspection>,
    pub lora_stack: ModelRuntimeValue<Vec<ModelRuntimeLoraInspection>>,
    pub active_steering: ModelRuntimeValue<Vec<ModelRuntimeSteeringInspection>>,
    pub process_ownership_ledger_link: ModelRuntimeValue<String>,
    pub tokens_per_second: ModelRuntimeValue<f64>,
    pub vram_resident_bytes: ModelRuntimeValue<u64>,
    pub last_call_at_utc: ModelRuntimeValue<String>,
    pub last_call_age_seconds: ModelRuntimeValue<u64>,
    pub engine_internals: ModelRuntimeValue<serde_json::Value>,
    pub quiesce_action: ModelRuntimeActionAvailability,
    pub unload_action: ModelRuntimeActionAvailability,
    pub compatible_adapter_swap_action: ModelRuntimeActionAvailability,
    pub inspect_engine_internals_action: ModelRuntimeActionAvailability,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeRegistryProjection {
    pub schema_id: String,
    pub generated_at_utc: String,
    pub catalog_revision: u64,
    pub rows: Vec<ModelRuntimeRegistryRow>,
    #[serde(default)]
    pub selection_receipt_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeProcessOwnershipRecord {
    pub schema_id: String,
    pub process_uuid: uuid::Uuid,
    pub os_pid: Option<i64>,
    pub engine_kind: String,
    pub started_at_utc: String,
    pub stopped_at_utc: Option<String>,
    pub exit_code: Option<i32>,
    pub stop_reason: Option<String>,
    pub model_artifact_sha256: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub sandbox_adapter_id: Option<String>,
}

pub trait ModelRuntimeRegistryTransport: Send + Sync {
    fn fetch_registry(&self, cell: ModelRuntimeRegistryCell);

    fn fetch_process_ownership(&self, uri: String, cell: ProcessOwnershipCell) {
        let _ = uri;
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Err(
                "ProcessOwnershipLedger transport is not connected.".to_owned()
            ));
        }
    }

    fn select_model(&self, target_model_id: String, cell: ModelRuntimeRegistryCell) {
        let _ = target_model_id;
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Err(
                "ModelRuntime selection transport is not connected.".to_owned()
            ));
        }
    }

    fn control_model(
        &self,
        model_id: String,
        action: ModelRuntimeControlAction,
        expected_catalog_revision: Option<u64>,
        expected_selection_revision: Option<u64>,
        cell: ModelRuntimeControlCell,
    ) {
        let _ = (
            model_id,
            action,
            expected_catalog_revision,
            expected_selection_revision,
        );
        if let Ok(mut slot) = cell.lock() {
            *slot = Some(Err(
                "ModelRuntime control transport is not connected.".to_owned()
            ));
        }
    }
}

#[derive(Default)]
struct ModelRuntimePanelState {
    projection: Option<ModelRuntimeRegistryProjection>,
    projection_is_stale: bool,
    pending_fetch: bool,
    pending_selection_model_id: Option<String>,
    pending_control_model_id: Option<String>,
    pending_control_action: Option<ModelRuntimeControlAction>,
    initial_fetch_started: bool,
    error: Option<String>,
    notice: Option<String>,
    expanded_engine_internals: std::collections::BTreeSet<String>,
    process_ownership: Option<ModelRuntimeProcessOwnershipRecord>,
    pending_process_ownership: bool,
}

pub struct ModelRuntimePaneFactory {
    state: Arc<Mutex<ModelRuntimePanelState>>,
    transport: Option<Arc<dyn ModelRuntimeRegistryTransport>>,
    delivery: ModelRuntimeRegistryCell,
    control_delivery: ModelRuntimeControlCell,
    process_ownership_delivery: ProcessOwnershipCell,
}

impl ModelRuntimePaneFactory {
    pub fn offline() -> Self {
        Self {
            state: Arc::new(Mutex::new(ModelRuntimePanelState {
                initial_fetch_started: true,
                error: Some("ModelRuntime registry backend is not connected.".to_owned()),
                ..Default::default()
            })),
            transport: None,
            delivery: Arc::new(Mutex::new(None)),
            control_delivery: Arc::new(Mutex::new(None)),
            process_ownership_delivery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_transport(transport: Arc<dyn ModelRuntimeRegistryTransport>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ModelRuntimePanelState::default())),
            transport: Some(transport),
            delivery: Arc::new(Mutex::new(None)),
            control_delivery: Arc::new(Mutex::new(None)),
            process_ownership_delivery: Arc::new(Mutex::new(None)),
        }
    }

    /// Seed the exact production renderer for Argus/egui-kittest proof. The
    /// real PostgreSQL and HTTP boundary is proven separately by the backend
    /// integration test; no alternate product renderer is introduced.
    pub fn with_projection(projection: ModelRuntimeRegistryProjection) -> Self {
        let validation_error = validate_projection_for_native_surface(&projection).err();
        Self {
            state: Arc::new(Mutex::new(ModelRuntimePanelState {
                projection: validation_error.is_none().then_some(projection),
                projection_is_stale: false,
                pending_fetch: false,
                pending_selection_model_id: None,
                pending_control_model_id: None,
                pending_control_action: None,
                initial_fetch_started: true,
                error: validation_error,
                notice: None,
                expanded_engine_internals: std::collections::BTreeSet::new(),
                process_ownership: None,
                pending_process_ownership: false,
            })),
            transport: None,
            delivery: Arc::new(Mutex::new(None)),
            control_delivery: Arc::new(Mutex::new(None)),
            process_ownership_delivery: Arc::new(Mutex::new(None)),
        }
    }

    fn start_process_ownership_fetch(&self, uri: String) {
        let Some(transport) = self.transport.as_ref() else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            state.pending_process_ownership = true;
            state.process_ownership = None;
            state.error = None;
        }
        transport.fetch_process_ownership(uri, self.process_ownership_delivery.clone());
    }

    fn drain_process_ownership_delivery(&self) {
        let delivered = self
            .process_ownership_delivery
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let Some(result) = delivered else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            state.pending_process_ownership = false;
            match result {
                Ok(record) if record.schema_id == "hsk.model_runtime_process_ownership@1" => {
                    state.process_ownership = Some(record);
                    state.error = None;
                }
                Ok(_) => {
                    state.error = Some("ProcessOwnershipLedger record schema is invalid".to_owned())
                }
                Err(error) => state.error = Some(error),
            }
        }
    }

    fn start_fetch(&self) {
        let Some(transport) = self.transport.as_ref() else {
            if let Ok(mut state) = self.state.lock() {
                state.initial_fetch_started = true;
                state.error = Some("ModelRuntime registry backend is not connected.".to_owned());
            }
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.pending_fetch
                || state.pending_selection_model_id.is_some()
                || state.pending_control_model_id.is_some()
            {
                return;
            }
            state.pending_fetch = true;
            state.initial_fetch_started = true;
            state.error = None;
        }
        transport.fetch_registry(self.delivery.clone());
    }

    fn start_selection(&self, target_model_id: String) {
        let Some(transport) = self.transport.as_ref() else {
            if let Ok(mut state) = self.state.lock() {
                state.error = Some("ModelRuntime selection backend is not connected.".to_owned());
            }
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.pending_fetch
                || state.pending_selection_model_id.is_some()
                || state.pending_control_model_id.is_some()
            {
                return;
            }
            state.pending_selection_model_id = Some(target_model_id.clone());
            state.error = None;
        }
        transport.select_model(target_model_id, self.delivery.clone());
    }

    fn start_control(
        &self,
        model_id: String,
        action: ModelRuntimeControlAction,
        expected_catalog_revision: Option<u64>,
        expected_selection_revision: Option<u64>,
    ) {
        let Some(transport) = self.transport.as_ref() else {
            if let Ok(mut state) = self.state.lock() {
                state.error = Some("ModelRuntime control backend is not connected.".to_owned());
            }
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.pending_fetch
                || state.pending_selection_model_id.is_some()
                || state.pending_control_model_id.is_some()
            {
                return;
            }
            state.pending_control_model_id = Some(model_id.clone());
            state.pending_control_action = Some(action.clone());
            state.error = None;
            state.notice = None;
        }
        transport.control_model(
            model_id,
            action,
            expected_catalog_revision,
            expected_selection_revision,
            self.control_delivery.clone(),
        );
    }

    fn drain_control_delivery(&self) {
        let delivered = self
            .control_delivery
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let Some(result) = delivered else {
            return;
        };
        let should_refresh = if let Ok(mut state) = self.state.lock() {
            let expected_model_id = state.pending_control_model_id.take();
            let expected_action = state.pending_control_action.take();
            match result {
                Ok(receipt)
                    if receipt.schema_version == 1
                        && expected_model_id.as_deref() == Some(receipt.model_id.as_str())
                        && expected_action.as_ref() == Some(&receipt.action)
                        && valid_control_receipt_outcome(&receipt) =>
                {
                    state.error = None;
                    state.notice = Some(if receipt.reconciliation_required {
                        format!(
                            "{} completed for model {} through {}, but process-ledger reconciliation is required: {} (request {}).",
                            control_action_label(&receipt.action),
                            receipt.model_id,
                            receipt.runtime_adapter,
                            receipt
                                .reconciliation_reason
                                .as_deref()
                                .unwrap_or("durable STOP was not confirmed"),
                            receipt.request_id
                        )
                    } else {
                        format!(
                            "{} completed for model {} through {} (request {}).",
                            control_action_label(&receipt.action),
                            receipt.model_id,
                            receipt.runtime_adapter,
                            receipt.request_id
                        )
                    });
                    true
                }
                Ok(_) => {
                    state.error = Some(
                        "ModelRuntime control returned an invalid or unsuccessful receipt."
                            .to_owned(),
                    );
                    false
                }
                Err(error) => {
                    state.error = Some(error);
                    false
                }
            }
        } else {
            false
        };
        if should_refresh {
            self.start_fetch();
        }
    }

    fn drain_delivery(&self) {
        let delivered = self.delivery.lock().ok().and_then(|mut slot| slot.take());
        let Some(result) = delivered else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            state.pending_fetch = false;
            state.pending_selection_model_id = None;
            match result {
                Ok(projection) => match validate_projection_for_native_surface(&projection) {
                    Ok(()) => {
                        state.projection = Some(projection);
                        state.projection_is_stale = false;
                        state.error = None;
                    }
                    Err(error) => {
                        state.projection_is_stale = state.projection.is_some();
                        state.error = Some(error);
                    }
                },
                Err(error) => {
                    state.projection_is_stale = state.projection.is_some();
                    state.error = Some(error);
                }
            }
        }
    }
}

impl PaneFactory for ModelRuntimePaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::ModelRuntime
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        self.drain_process_ownership_delivery();
        self.drain_control_delivery();
        self.drain_delivery();
        let pane_id = ctx.record.pane_id.as_ref();
        let should_start = self.transport.is_some()
            && self
                .state
                .lock()
                .ok()
                .is_some_and(|state| !state.initial_fetch_started && !state.pending_fetch);
        if should_start {
            self.start_fetch();
        }

        let Ok(mut state) = self.state.lock() else {
            ui.label("ModelRuntime registry state unavailable.");
            return;
        };
        let surface_id = ctx.egui_id.with("model-runtime-registry-surface");
        ui.ctx().accesskit_node_builder(surface_id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(surface_author_id(pane_id));
            node.set_label("ModelRuntime Control Panel".to_owned());
        });

        let mut refresh_requested = false;
        let mut switch_requested = None;
        let mut control_requested = None;
        let mut inspect_internals_requested = None;
        let mut process_ownership_requested = None;
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("ModelRuntime Control Panel");
                let action_label = if state.error.is_some() {
                    "Retry"
                } else {
                    "Refresh"
                };
                let action_accessible_label = format!("{action_label} ModelRuntime registry");
                let refresh = ui.add_enabled(!state.pending_fetch, egui::Button::new(action_label));
                refresh.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        action_accessible_label.clone(),
                    )
                });
                ui.ctx().accesskit_node_builder(refresh.id, |node| {
                    node.set_role(accesskit::Role::Button);
                    node.add_action(accesskit::Action::Click);
                    node.set_author_id(refresh_author_id(pane_id));
                    node.set_label(action_accessible_label);
                });
                refresh_requested = refresh.clicked();
            });

            let status = match (
                &state.projection,
                state.pending_fetch,
                state.projection_is_stale,
            ) {
                (None, true, _) => "Loading durable model registry...".to_owned(),
                (Some(projection), true, true) => format!(
                    "Refreshing stale registry snapshot; last successful snapshot generated {}",
                    projection.generated_at_utc
                ),
                (Some(projection), true, false) => format!(
                    "Refreshing durable model registry; last successful snapshot generated {}",
                    projection.generated_at_utc
                ),
                (Some(projection), false, true) => format!(
                    "STALE registry snapshot | last successful snapshot generated {} | current runtime state unknown",
                    projection.generated_at_utc
                ),
                (Some(projection), false, false) => {
                    let live = projection
                        .rows
                        .iter()
                        .filter(|row| row.runtime_state == ModelRuntimeRegistryRowState::Live)
                        .count();
                    let dormant = projection.rows.len().saturating_sub(live);
                    let selected = projection.rows.iter().filter(|row| row.selected).count();
                    format!(
                        "{} live | {} dormant | {} selected | snapshot generated {}",
                        live, dormant, selected, projection.generated_at_utc
                    )
                }
                (None, false, _) => "No registry projection loaded.".to_owned(),
            };
            tagged_label(
                ui,
                ctx.egui_id.with("model-runtime-registry-status"),
                &status_author_id(pane_id),
                &status,
            );

            if state.pending_fetch {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            if let Some(target) = state.pending_selection_model_id.as_deref() {
                ui.label(format!(
                    "Switching the active READY model to {target}; the selection is not accepted until its audit record succeeds..."
                ));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            if let Some(target) = state.pending_control_model_id.as_deref() {
                let action = state
                    .pending_control_action
                    .as_ref()
                    .map(control_action_label)
                    .unwrap_or("Runtime control");
                ui.label(format!(
                    "{action} is running for {target}; success requires the matching typed receipt..."
                ));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            if let Some(notice) = state.notice.as_deref() {
                tagged_label(
                    ui,
                    ctx.egui_id.with("model-runtime-control-notice"),
                    &control_notice_author_id(pane_id),
                    notice,
                );
            }
            if let Some(error) = state.error.as_deref() {
                let response = ui.colored_label(ui.visuals().error_fg_color, error);
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(error_author_id(pane_id));
                    node.set_label(format!("ModelRuntime registry error: {error}"));
                });
            }

            let Some(projection) = state.projection.as_ref() else {
                return;
            };
            if let Some(receipt_ref) = projection.selection_receipt_ref.as_deref() {
                tagged_monospace_label(
                    ui,
                    ctx.egui_id.with("model-runtime-selection-receipt"),
                    &format!("{AUTHOR_ID_PREFIX}.{pane_id}.selection-receipt"),
                    &format!("Selection receipt: {receipt_ref}"),
                );
            }
            if state.pending_process_ownership {
                ui.label("Loading exact ProcessOwnershipLedger record...");
            }
            if let Some(record) = state.process_ownership.as_ref() {
                ui.group(|ui| {
                    ui.strong(format!("ProcessOwnershipLedger {}", record.process_uuid));
                    ui.monospace(format!(
                        "engine={} owner={}",
                        record.engine_kind, record.owner_role
                    ));
                    ui.monospace(format!(
                        "started={} stopped={}",
                        record.started_at_utc,
                        record.stopped_at_utc.as_deref().unwrap_or("running")
                    ));
                    ui.monospace(format!(
                        "pid={} exit={} stop_reason={}",
                        record
                            .os_pid
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "pidless".to_owned()),
                        record
                            .exit_code
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "n/a".to_owned()),
                        record.stop_reason.as_deref().unwrap_or("n/a")
                    ));
                });
            }
            if projection.rows.is_empty() {
                tagged_label(
                    ui,
                    ctx.egui_id.with("model-runtime-registry-empty"),
                    &empty_author_id(pane_id),
                    "No models are registered yet.",
                );
                return;
            }

            ui.separator();
            let rail_dimensions = RailDimensions::default();
            let scroll_output = egui::ScrollArea::vertical()
                .id_salt(("model-runtime-registry-scroll", pane_id))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // The stable custom rail overlays the rightmost hit-width;
                    // keep wrapped row content out from under it.
                    ui.set_max_width(
                        (ui.available_width() - rail_dimensions.hit_thickness).max(1.0),
                    );
                    for row in &projection.rows {
                        let row_actions = render_registry_row(
                            ui,
                            ctx,
                            row,
                            state.projection_is_stale,
                            !state.pending_fetch
                                && state.pending_selection_model_id.is_none()
                                && state.pending_control_model_id.is_none(),
                            state
                                .expanded_engine_internals
                                .contains(&row.artifact_sha256),
                        );
                        if row_actions.switch_requested {
                            switch_requested = row.live_model_id.clone();
                        }
                        if row_actions.quiesce_requested {
                            if let Some(model_id) = row.live_model_id.clone() {
                                control_requested = Some((
                                    model_id,
                                    ModelRuntimeControlAction::Quiesce,
                                    None,
                                    None,
                                ));
                            }
                        }
                        if row_actions.unload_requested {
                            if let Some(model_id) = row.live_model_id.clone() {
                                control_requested = Some((
                                    model_id,
                                    ModelRuntimeControlAction::Unload,
                                    Some(projection.catalog_revision),
                                    None,
                                ));
                            }
                        }
                        if row_actions.swap_adapter_requested {
                            if let (Some(model_id), Some(target_adapter)) = (
                                row.live_model_id.clone(),
                                compatible_target_adapter(&row.selected_adapter),
                            ) {
                                control_requested = Some((
                                    model_id,
                                    ModelRuntimeControlAction::SwapCompatibleAdapter {
                                        target_adapter: target_adapter.to_owned(),
                                    },
                                    Some(projection.catalog_revision),
                                    Some(row.selection_revision),
                                ));
                            }
                        }
                        if row_actions.inspect_internals_requested {
                            inspect_internals_requested = Some(row.artifact_sha256.clone());
                        }
                        if let Some(uri) = row_actions.process_ownership_requested {
                            process_ownership_requested = Some(uri);
                        }
                        ui.add_space(6.0);
                    }
                });

            let track_rect = egui::Rect::from_min_max(
                egui::pos2(
                    scroll_output.inner_rect.right() - rail_dimensions.hit_thickness,
                    scroll_output.inner_rect.top(),
                ),
                scroll_output.inner_rect.right_bottom(),
            );
            let scrollbar_author_id = format!("scrollbar-v-{pane_id}");
            let scrollbar_id = SCROLLBAR_V_NODE_IDS
                .iter()
                .find(|(candidate, _)| *candidate == scrollbar_author_id.as_str())
                .map(|(_, node_id)| scrollbar_rail_id(*node_id))
                .unwrap_or_else(|| ctx.egui_id.with("model-runtime-registry-scrollbar"));
            let widgets = &ui.visuals().widgets;
            let rail_colors = RailColors {
                idle: widgets.inactive.bg_fill,
                hover: widgets.hovered.bg_fill,
                grab: widgets.active.bg_fill,
                disabled: widgets.noninteractive.bg_fill,
            };
            let rail_response = ScrollbarRail {
                id: scrollbar_id,
                orientation: RailOrientation::Vertical,
                track_rect,
                content_size: scroll_output.content_size.y,
                viewport_size: scroll_output.inner_rect.height(),
                scroll_offset: scroll_output.state.offset.y,
                colors: rail_colors,
                dims: rail_dimensions,
                author_id: scrollbar_author_id,
                line_step: 40.0,
            }
            .show(ui);
            if (rail_response.new_offset - scroll_output.state.offset.y).abs() > f32::EPSILON {
                let mut scroll_state = scroll_output.state;
                scroll_state.offset.y = rail_response.new_offset;
                scroll_state.store(ui.ctx(), scroll_output.id);
                ui.ctx().request_repaint();
            }
        });
        if let Some(artifact_sha256) = inspect_internals_requested {
            state.expanded_engine_internals.insert(artifact_sha256);
            ui.ctx().request_repaint();
        }
        drop(state);
        if refresh_requested {
            self.start_fetch();
        }
        if let Some(target_model_id) = switch_requested {
            self.start_selection(target_model_id);
        }
        if let Some((model_id, action, catalog_revision, selection_revision)) = control_requested {
            self.start_control(model_id, action, catalog_revision, selection_revision);
        }
        if let Some(uri) = process_ownership_requested {
            self.start_process_ownership_fetch(uri);
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Pane
    }
}

#[derive(Default)]
struct RowActionRequests {
    switch_requested: bool,
    quiesce_requested: bool,
    unload_requested: bool,
    swap_adapter_requested: bool,
    inspect_internals_requested: bool,
    process_ownership_requested: Option<String>,
}

fn render_registry_row(
    ui: &mut egui::Ui,
    ctx: &PaneRenderContext,
    row: &ModelRuntimeRegistryRow,
    projection_is_stale: bool,
    actions_enabled: bool,
    force_engine_internals_open: bool,
) -> RowActionRequests {
    let pane_id = ctx.record.pane_id.as_ref();
    let artifact = row.artifact_sha256.as_str();
    let mut switch_requested = false;
    let mut quiesce_requested = false;
    let mut unload_requested = false;
    let mut swap_adapter_requested = false;
    let mut inspect_internals_requested = false;
    let mut process_ownership_requested = None;
    let group = ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            let display_label_color = ui.visuals().text_color();
            ui.label(
                egui::RichText::new(&row.display_label)
                    .strong()
                    .color(display_label_color),
            );
            let (state_text, state_color) = match (projection_is_stale, row.runtime_state) {
                (true, ModelRuntimeRegistryRowState::Live) => (
                    "STALE / LAST SEEN READY",
                    egui::Color32::from_rgb(214, 161, 66),
                ),
                (true, ModelRuntimeRegistryRowState::Dormant) => (
                    "STALE / LAST SEEN DORMANT",
                    egui::Color32::from_rgb(214, 161, 66),
                ),
                (false, ModelRuntimeRegistryRowState::Live) => {
                    ("LIVE / READY", egui::Color32::from_rgb(68, 184, 112))
                }
                (false, ModelRuntimeRegistryRowState::Dormant) => {
                    ("DORMANT", egui::Color32::from_rgb(214, 161, 66))
                }
            };
            let response = ui.colored_label(state_color, state_text);
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_author_id(row_state_author_id(pane_id, artifact));
                node.set_label(format!("Runtime state: {state_text}"));
                node.add_action(accesskit::Action::ScrollIntoView);
            });
        });
        tagged_label(
            ui,
            ctx.egui_id.with(("adapter", artifact)),
            &row_adapter_author_id(pane_id, artifact),
            &format!(
                "Selected adapter: {}",
                adapter_display_name(&row.selected_adapter)
            ),
        );
        tagged_label(
            ui,
            ctx.egui_id.with(("runtime-role", artifact)),
            &row_role_author_id(pane_id, artifact),
            &format!(
                "Runtime role: {}",
                runtime_role_display_name(row.runtime_role)
            ),
        );
        tagged_label(
            ui,
            ctx.egui_id.with(("revision", artifact)),
            &row_revision_author_id(pane_id, artifact),
            &format!("Selection revision: {}", row.selection_revision),
        );
        tagged_monospace_label(
            ui,
            ctx.egui_id.with(("sha256", artifact)),
            &row_sha_author_id(pane_id, artifact),
            &format!("Artifact SHA-256: {artifact}"),
        );
        tagged_monospace_label(
            ui,
            ctx.egui_id.with(("locator", artifact)),
            &row_locator_author_id(pane_id, artifact),
            &format!("Artifact locator: {}", row.artifact_locator),
        );
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("artifact-path", artifact)),
            &row_artifact_path_author_id(pane_id, artifact),
            "Canonical artifact path",
            &row.canonical_artifact_path,
            |value| value.clone(),
        );
        match (projection_is_stale, row.live_model_id.as_deref()) {
            (true, Some(model_id)) => tagged_monospace_label(
                ui,
                ctx.egui_id.with(("live-model", artifact)),
                &row_live_model_author_id(pane_id, artifact),
                &format!("Last seen live model id: {model_id}"),
            ),
            (false, Some(model_id)) => tagged_monospace_label(
                ui,
                ctx.egui_id.with(("live-model", artifact)),
                &row_live_model_author_id(pane_id, artifact),
                &format!("Live model id: {model_id}"),
            ),
            (true, None) => tagged_label(
                ui,
                ctx.egui_id.with(("dormant-reason", artifact)),
                &row_dormant_reason_author_id(pane_id, artifact),
                "No model was READY for this artifact in the last successful snapshot.",
            ),
            (false, None) => tagged_label(
                ui,
                ctx.egui_id.with(("dormant-reason", artifact)),
                &row_dormant_reason_author_id(pane_id, artifact),
                "No model is currently READY for this artifact.",
            ),
        }
        tagged_monospace_label(
            ui,
            ctx.egui_id.with(("audit", artifact)),
            &row_audit_author_id(pane_id, artifact),
            &format!("Selection audit: {}", row.selection_audit_event_ref),
        );
        match &row.kv_cache {
            ModelRuntimeValue::Available { value } => {
                let hit_rate = match &value.prefix_cache_hit_rate {
                    ModelRuntimeValue::Available { value } => format!("{:.2}%", value * 100.0),
                    ModelRuntimeValue::Unavailable { reason } => {
                        format!("unavailable ({reason})")
                    }
                };
                tagged_label(
                    ui,
                    ctx.egui_id.with(("kv-cache", artifact)),
                    &row_kv_cache_author_id(pane_id, artifact),
                    &format!(
                        "KV cache: {} / {} bytes | prefix hit rate: {} | quantization: {}",
                        value.bytes_used, value.bytes_capacity, hit_rate, value.quantization
                    ),
                );
            }
            ModelRuntimeValue::Unavailable { reason } => tagged_label(
                ui,
                ctx.egui_id.with(("kv-cache", artifact)),
                &row_kv_cache_author_id(pane_id, artifact),
                &format!("KV cache: unavailable ({reason})"),
            ),
        }
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("lora-stack", artifact)),
            &row_lora_author_id(pane_id, artifact),
            "LoRA stack",
            &row.lora_stack,
            |entries| {
                if entries.is_empty() {
                    "none active".to_owned()
                } else {
                    entries
                        .iter()
                        .map(|entry| format!("{} @ {}", entry.lora_id, entry.strength))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            },
        );
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("steering", artifact)),
            &row_steering_author_id(pane_id, artifact),
            "Active steering",
            &row.active_steering,
            |entries| {
                entries
                    .iter()
                    .map(|entry| {
                        format!(
                            "{} layer {} intensity {}",
                            entry.steering_vector_id, entry.layer, entry.intensity
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("tokens-per-second", artifact)),
            &row_tokens_per_second_author_id(pane_id, artifact),
            "Tokens/s",
            &row.tokens_per_second,
            |value| format!("{value:.2}"),
        );
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("vram", artifact)),
            &row_vram_author_id(pane_id, artifact),
            "VRAM resident bytes",
            &row.vram_resident_bytes,
            u64::to_string,
        );
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("last-call", artifact)),
            &row_last_call_author_id(pane_id, artifact),
            "Last call",
            &row.last_call_at_utc,
            Clone::clone,
        );
        tagged_runtime_value(
            ui,
            ctx.egui_id.with(("last-call-age", artifact)),
            &row_last_call_age_author_id(pane_id, artifact),
            "Time since last call",
            &row.last_call_age_seconds,
            |seconds| format_duration_seconds(*seconds),
        );
        match &row.engine_internals {
            ModelRuntimeValue::Available { value } => {
                tagged_label(
                    ui,
                    ctx.egui_id.with(("engine-internals-value", artifact)),
                    &row_engine_internals_author_id(pane_id, artifact),
                    "Engine internals: available",
                );
                let header = egui::CollapsingHeader::new("Engine internals")
                    .id_salt(("engine-internals", pane_id, artifact))
                    .open(force_engine_internals_open.then_some(true))
                    .show(ui, |ui| {
                        ui.monospace(serde_json::to_string_pretty(value).unwrap_or_else(|_| {
                            "engine internals serialization failed".to_owned()
                        }));
                    });
                ui.ctx()
                    .accesskit_node_builder(header.header_response.id, |node| {
                        node.set_author_id(row_engine_internals_expand_author_id(
                            pane_id, artifact,
                        ));
                    });
            }
            ModelRuntimeValue::Unavailable { reason } => tagged_label(
                ui,
                ctx.egui_id.with(("engine-internals-value", artifact)),
                &row_engine_internals_author_id(pane_id, artifact),
                &format!("Engine internals: unavailable ({reason})"),
            ),
        }
        match &row.process_ownership_ledger_link {
            ModelRuntimeValue::Available { value } => {
                let response = ui.button("Inspect ownership record");
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(row_ledger_link_author_id(pane_id, artifact));
                    node.set_label(format!(
                        "Inspect exact ProcessOwnershipLedger record: {value}"
                    ));
                });
                if response.clicked() {
                    process_ownership_requested = Some(value.clone());
                    if let Ok(mut selected) = PROCESS_LEDGER_NAVIGATION
                        .get_or_init(|| Mutex::new(None))
                        .lock()
                    {
                        *selected = Some(value.clone());
                    }
                }
            }
            ModelRuntimeValue::Unavailable { reason } => tagged_label(
                ui,
                ctx.egui_id.with(("ledger-link", artifact)),
                &row_ledger_link_author_id(pane_id, artifact),
                &format!("ProcessOwnershipLedger: unavailable ({reason})"),
            ),
        }
        if !row.active_purposes.is_empty() {
            let purposes = row
                .active_purposes
                .iter()
                .map(|purpose| match purpose {
                    ModelRuntimeSelectionPurpose::ApplicationDefault => "application/default",
                    ModelRuntimeSelectionPurpose::EmbeddingsDefault => "embeddings/default",
                })
                .collect::<Vec<_>>()
                .join(", ");
            tagged_label(
                ui,
                ctx.egui_id.with(("active-purposes", artifact)),
                &row_active_purposes_author_id(pane_id, artifact),
                &format!(
                    "Active purpose: {purposes} | revision {}",
                    row.active_selection_revision.unwrap_or(0)
                ),
            );
        }
        if row.selected {
            tagged_label(
                ui,
                ctx.egui_id.with(("active-selection", artifact)),
                &row_active_selection_author_id(pane_id, artifact),
                "ACTIVE DEFAULT MODEL",
            );
        } else if row.runtime_state == ModelRuntimeRegistryRowState::Live && row.default_selectable
        {
            let label = format!("Switch to {}", row.display_label);
            let response = ui.add_enabled(
                actions_enabled && !projection_is_stale,
                egui::Button::new(&label),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    actions_enabled && !projection_is_stale,
                    label.clone(),
                )
            });
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(accesskit::Role::Button);
                node.add_action(accesskit::Action::Click);
                node.set_author_id(row_switch_author_id(pane_id, artifact));
                node.set_label(label);
            });
            switch_requested = response.clicked();
        } else if row.runtime_state == ModelRuntimeRegistryRowState::Live {
            tagged_label(
                ui,
                ctx.egui_id.with(("default-ineligible", artifact)),
                &row_default_ineligible_author_id(pane_id, artifact),
                "Not selectable as the default completion model.",
            );
        }
        quiesce_requested = render_runtime_action(
            ui,
            pane_id,
            artifact,
            "Quiesce model",
            "quiesce",
            &row.quiesce_action,
            actions_enabled && !projection_is_stale,
        );
        unload_requested = render_runtime_action(
            ui,
            pane_id,
            artifact,
            "Unload model",
            "unload",
            &row.unload_action,
            actions_enabled && !projection_is_stale,
        );
        let swap_target = compatible_target_adapter(&row.selected_adapter);
        let swap_label = swap_target
            .map(|target| format!("Swap to {}", adapter_display_name(target)))
            .unwrap_or_else(|| "No compatible adapter target".to_owned());
        let swap_availability = swap_target
            .map(|_| row.compatible_adapter_swap_action.clone())
            .unwrap_or_else(|| ModelRuntimeActionAvailability {
                enabled: false,
                reason: Some("no compatible adapter target is available".to_owned()),
            });
        swap_adapter_requested = render_runtime_action(
            ui,
            pane_id,
            artifact,
            &swap_label,
            "adapter-swap",
            &swap_availability,
            actions_enabled && !projection_is_stale,
        );
        inspect_internals_requested = render_runtime_action(
            ui,
            pane_id,
            artifact,
            "Inspect engine internals",
            "inspect-internals",
            &row.inspect_engine_internals_action,
            actions_enabled && !projection_is_stale,
        );
    });
    ui.ctx().accesskit_node_builder(group.response.id, |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(row_author_id(pane_id, artifact));
        node.set_label(format!(
            "ModelRuntime registry row for {}",
            row.display_label
        ));
        node.add_action(accesskit::Action::ScrollIntoView);
    });
    RowActionRequests {
        switch_requested,
        quiesce_requested,
        unload_requested,
        swap_adapter_requested,
        inspect_internals_requested,
        process_ownership_requested,
    }
}

fn tagged_runtime_value<T>(
    ui: &mut egui::Ui,
    id: egui::Id,
    author_id: &str,
    label: &str,
    value: &ModelRuntimeValue<T>,
    format_available: impl FnOnce(&T) -> String,
) {
    let text = match value {
        ModelRuntimeValue::Available { value } => {
            format!("{label}: {}", format_available(value))
        }
        ModelRuntimeValue::Unavailable { reason } => {
            format!("{label}: unavailable ({reason})")
        }
    };
    tagged_label(ui, id, author_id, &text);
}

fn render_runtime_action(
    ui: &mut egui::Ui,
    pane_id: &str,
    artifact_sha256: &str,
    label: &str,
    action: &str,
    availability: &ModelRuntimeActionAvailability,
    actions_enabled: bool,
) -> bool {
    let enabled = actions_enabled && availability.enabled;
    let response = if enabled {
        ui.add_enabled(true, egui::Button::new(label))
    } else {
        let reason = availability
            .reason
            .as_deref()
            .unwrap_or("production action transport is unavailable");
        ui.add_enabled(false, egui::Button::new(label))
            .on_disabled_hover_text(reason)
    };
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(row_action_author_id(pane_id, artifact_sha256, action));
        node.set_label(if enabled {
            label.to_owned()
        } else {
            format!(
                "{label}: unavailable ({})",
                availability
                    .reason
                    .as_deref()
                    .unwrap_or("production action transport is unavailable")
            )
        });
    });
    response.clicked()
}

fn tagged_label(ui: &mut egui::Ui, id: egui::Id, author_id: &str, text: &str) {
    let response = ui
        .push_id(id, |ui| ui.add(egui::Label::new(text).wrap()))
        .inner;
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_author_id(author_id.to_owned());
        node.set_label(text.to_owned());
        node.add_action(accesskit::Action::ScrollIntoView);
    });
}

fn tagged_monospace_label(ui: &mut egui::Ui, id: egui::Id, author_id: &str, text: &str) {
    let response = ui
        .push_id(id, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap())
        })
        .inner;
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_author_id(author_id.to_owned());
        node.set_label(text.to_owned());
        node.add_action(accesskit::Action::ScrollIntoView);
    });
}

fn adapter_display_name(adapter: &str) -> &'static str {
    match adapter {
        "llama_cpp" => "LlamaCppRuntime",
        "candle" => "CandleRuntime",
        _ => "Unknown adapter",
    }
}

fn compatible_target_adapter(adapter: &str) -> Option<&'static str> {
    match adapter {
        "llama_cpp" => Some("candle"),
        "candle" => Some("llama_cpp"),
        _ => None,
    }
}

fn control_action_label(action: &ModelRuntimeControlAction) -> &'static str {
    match action {
        ModelRuntimeControlAction::Quiesce => "Quiesce",
        ModelRuntimeControlAction::Unload => "Unload",
        ModelRuntimeControlAction::SwapCompatibleAdapter { .. } => "Compatible adapter swap",
    }
}

fn valid_control_receipt_outcome(receipt: &ModelRuntimeControlReceipt) -> bool {
    match &receipt.action {
        ModelRuntimeControlAction::Quiesce => {
            receipt.result_model_id.is_none()
                && receipt.quiesced
                && !receipt.unloaded
                && !receipt.process_stop_committed
                && !receipt.registry_updated
                && !receipt.selection_rebound
                && !receipt.reconciliation_required
                && receipt.reconciliation_reason.is_none()
        }
        ModelRuntimeControlAction::Unload => {
            receipt.result_model_id.is_none()
                && receipt.quiesced
                && receipt.unloaded
                && (receipt.process_stop_committed != receipt.reconciliation_required)
                && receipt.registry_updated
                && !receipt.selection_rebound
                && receipt.catalog_revision.is_some()
                && (receipt.reconciliation_required == receipt.reconciliation_reason.is_some())
        }
        ModelRuntimeControlAction::SwapCompatibleAdapter { target_adapter } => {
            receipt.result_model_id.is_some()
                && receipt.runtime_adapter.as_str() == target_adapter.as_str()
                && receipt.quiesced
                && receipt.unloaded
                && (receipt.process_stop_committed != receipt.reconciliation_required)
                && receipt.registry_updated
                && receipt.selection_rebound
                && receipt.catalog_revision.is_some()
                && (receipt.reconciliation_required == receipt.reconciliation_reason.is_some())
        }
    }
}

fn runtime_role_display_name(role: ModelRuntimeRole) -> &'static str {
    match role {
        ModelRuntimeRole::Completion => "completion",
        ModelRuntimeRole::Embedding => "embedding",
    }
}

fn format_duration_seconds(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

fn pane_author_id(pane_id: &str, suffix: &str) -> String {
    format!("{AUTHOR_ID_PREFIX}.{pane_id}.{suffix}")
}

pub fn surface_author_id(pane_id: &str) -> String {
    pane_author_id(pane_id, "surface")
}

pub fn refresh_author_id(pane_id: &str) -> String {
    pane_author_id(pane_id, "action.refresh")
}

pub fn status_author_id(pane_id: &str) -> String {
    pane_author_id(pane_id, "status")
}

pub fn control_notice_author_id(pane_id: &str) -> String {
    pane_author_id(pane_id, "control.notice")
}

pub fn empty_author_id(pane_id: &str) -> String {
    pane_author_id(pane_id, "empty")
}

pub fn error_author_id(pane_id: &str) -> String {
    pane_author_id(pane_id, "error")
}

pub fn row_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    pane_author_id(pane_id, &format!("row.{artifact_sha256}"))
}

pub fn row_state_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.state", row_author_id(pane_id, artifact_sha256))
}

pub fn row_adapter_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.adapter", row_author_id(pane_id, artifact_sha256))
}

pub fn row_role_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.runtime-role", row_author_id(pane_id, artifact_sha256))
}

pub fn row_default_ineligible_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.default-ineligible",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_revision_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.revision", row_author_id(pane_id, artifact_sha256))
}

pub fn row_sha_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.sha256", row_author_id(pane_id, artifact_sha256))
}

pub fn row_locator_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.locator", row_author_id(pane_id, artifact_sha256))
}

pub fn row_artifact_path_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.artifact-path", row_author_id(pane_id, artifact_sha256))
}

pub fn row_kv_cache_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.kv-cache", row_author_id(pane_id, artifact_sha256))
}

pub fn row_lora_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.lora-stack", row_author_id(pane_id, artifact_sha256))
}

pub fn row_steering_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.active-steering",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_tokens_per_second_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.tokens-per-second",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_vram_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.vram", row_author_id(pane_id, artifact_sha256))
}

pub fn row_last_call_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.last-call", row_author_id(pane_id, artifact_sha256))
}

pub fn row_last_call_age_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.last-call-age", row_author_id(pane_id, artifact_sha256))
}

pub fn row_engine_internals_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.engine-internals",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_engine_internals_expand_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.engine-internals-expand",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_ledger_link_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.ledger-link", row_author_id(pane_id, artifact_sha256))
}

pub fn row_action_author_id(pane_id: &str, artifact_sha256: &str, action: &str) -> String {
    format!(
        "{}.action.{action}",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_live_model_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.live-model", row_author_id(pane_id, artifact_sha256))
}

pub fn row_dormant_reason_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.dormant-reason", row_author_id(pane_id, artifact_sha256))
}

pub fn row_audit_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.audit", row_author_id(pane_id, artifact_sha256))
}

pub fn row_active_selection_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.active-selection",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_active_purposes_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!(
        "{}.active-purposes",
        row_author_id(pane_id, artifact_sha256)
    )
}

pub fn row_switch_author_id(pane_id: &str, artifact_sha256: &str) -> String {
    format!("{}.action.switch", row_author_id(pane_id, artifact_sha256))
}

pub fn validate_projection_for_native_surface(
    projection: &ModelRuntimeRegistryProjection,
) -> Result<(), String> {
    if projection.schema_id != PROJECTION_SCHEMA_ID {
        return Err(format!(
            "ModelRuntime registry schema_id mismatch: expected {PROJECTION_SCHEMA_ID}, got {}",
            projection.schema_id
        ));
    }
    if projection.generated_at_utc.trim().is_empty() {
        return Err("ModelRuntime registry generated_at_utc is empty".to_owned());
    }
    let selection_receipt_ref = projection
        .selection_receipt_ref
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "ModelRuntime registry selection_receipt_ref is absent".to_owned())?;
    if !selection_receipt_ref.starts_with("eventledger://kernel/")
        && !selection_receipt_ref.starts_with("model-runtime-selection://receipt/")
    {
        return Err("ModelRuntime registry selection_receipt_ref is not canonical".to_owned());
    }

    let mut artifact_hashes = std::collections::BTreeSet::new();
    let mut author_ids = std::collections::BTreeSet::new();
    let mut selected_rows = 0usize;
    let mut active_purposes = std::collections::BTreeSet::new();
    for row in &projection.rows {
        if !is_lower_hex_sha256(&row.artifact_sha256) {
            return Err(format!(
                "ModelRuntime registry artifact SHA-256 is invalid: `{}`",
                row.artifact_sha256
            ));
        }
        if !artifact_hashes.insert(row.artifact_sha256.clone()) {
            return Err(format!(
                "ModelRuntime registry contains duplicate artifact SHA-256 {}",
                row.artifact_sha256
            ));
        }
        let expected_locator = format!("sha256:{}", row.artifact_sha256);
        if row.artifact_locator != expected_locator {
            return Err(format!(
                "ModelRuntime registry artifact locator `{}` does not bind to {}",
                row.artifact_locator, row.artifact_sha256
            ));
        }
        if row.display_label.trim().is_empty() {
            return Err("ModelRuntime registry display label is empty".to_owned());
        }
        if !matches!(row.selected_adapter.as_str(), "llama_cpp" | "candle") {
            return Err(format!(
                "ModelRuntime registry selected adapter is unsupported: `{}`",
                row.selected_adapter
            ));
        }
        if row.selection_revision == 0 {
            return Err("ModelRuntime registry selection revision must be at least one".to_owned());
        }
        if row.default_selectable != (row.runtime_role == ModelRuntimeRole::Completion) {
            return Err(format!(
                "ModelRuntime registry runtime role {:?} disagrees with default_selectable {} for artifact {}",
                row.runtime_role, row.default_selectable, row.artifact_sha256
            ));
        }
        if row
            .selection_audit_event_ref
            .strip_prefix("eventledger://kernel/")
            .is_none_or(str::is_empty)
        {
            return Err(format!(
                "ModelRuntime registry selection audit ref is invalid: `{}`",
                row.selection_audit_event_ref
            ));
        }
        match (row.runtime_state, row.live_model_id.as_deref()) {
            (ModelRuntimeRegistryRowState::Live, Some(model_id)) if !model_id.trim().is_empty() => {
            }
            (ModelRuntimeRegistryRowState::Live, _) => {
                return Err("LIVE ModelRuntime registry row has no live_model_id".to_owned());
            }
            (ModelRuntimeRegistryRowState::Dormant, None) => {}
            (ModelRuntimeRegistryRowState::Dormant, Some(_)) => {
                return Err(
                    "DORMANT ModelRuntime registry row must not expose a stale live_model_id"
                        .to_owned(),
                );
            }
        }
        if row.selected {
            selected_rows += 1;
            if !row.default_selectable {
                return Err(
                    "application/default ModelRuntime registry row must be completion-role"
                        .to_owned(),
                );
            }
        }
        let contains_application = row
            .active_purposes
            .contains(&ModelRuntimeSelectionPurpose::ApplicationDefault);
        if row.selected != contains_application {
            return Err("selected flag disagrees with application/default purpose".to_owned());
        }
        if row.active_purposes.is_empty() != row.active_selection_revision.is_none() {
            return Err(
                "active purpose presence disagrees with active_selection_revision".to_owned(),
            );
        }
        for purpose in &row.active_purposes {
            let expected_role = match purpose {
                ModelRuntimeSelectionPurpose::ApplicationDefault => ModelRuntimeRole::Completion,
                ModelRuntimeSelectionPurpose::EmbeddingsDefault => ModelRuntimeRole::Embedding,
            };
            if row.runtime_role != expected_role || !active_purposes.insert(*purpose) {
                return Err(format!(
                    "active purpose {:?} is duplicated or attached to the wrong runtime role",
                    purpose
                ));
            }
        }
        for (field, value) in [
            (
                "canonical_artifact_path",
                unavailable_reason(&row.canonical_artifact_path),
            ),
            ("kv_cache", unavailable_reason(&row.kv_cache)),
            ("lora_stack", unavailable_reason(&row.lora_stack)),
            ("active_steering", unavailable_reason(&row.active_steering)),
            (
                "process_ownership_ledger_link",
                unavailable_reason(&row.process_ownership_ledger_link),
            ),
            (
                "tokens_per_second",
                unavailable_reason(&row.tokens_per_second),
            ),
            (
                "vram_resident_bytes",
                unavailable_reason(&row.vram_resident_bytes),
            ),
            (
                "last_call_at_utc",
                unavailable_reason(&row.last_call_at_utc),
            ),
            (
                "last_call_age_seconds",
                unavailable_reason(&row.last_call_age_seconds),
            ),
            (
                "engine_internals",
                unavailable_reason(&row.engine_internals),
            ),
        ] {
            if reason_is_empty(value) {
                return Err(format!("{field} unavailability reason is empty"));
            }
        }
        if let ModelRuntimeValue::Available { value } = &row.process_ownership_ledger_link {
            if !value.starts_with("process-ownership-ledger://process/") {
                return Err("ProcessOwnershipLedger link is not canonical".to_owned());
            }
        }
        for (name, action) in [
            ("quiesce", &row.quiesce_action),
            ("unload", &row.unload_action),
            (
                "compatible_adapter_swap",
                &row.compatible_adapter_swap_action,
            ),
        ] {
            let valid = if action.enabled {
                row.runtime_state == ModelRuntimeRegistryRowState::Live && action.reason.is_none()
            } else {
                action
                    .reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty())
            };
            if !valid {
                return Err(format!(
                    "{name} action availability is inconsistent with runtime state or reason"
                ));
            }
        }
        if row.selected && row.unload_action.enabled {
            return Err("selected application/default row cannot enable unload".to_owned());
        }
        match &row.engine_internals {
            ModelRuntimeValue::Available { .. }
                if row.inspect_engine_internals_action.enabled
                    && row.inspect_engine_internals_action.reason.is_none() => {}
            ModelRuntimeValue::Unavailable { .. }
                if !row.inspect_engine_internals_action.enabled
                    && row
                        .inspect_engine_internals_action
                        .reason
                        .as_deref()
                        .is_some_and(|reason| !reason.trim().is_empty()) => {}
            _ => {
                return Err(
                    "inspect engine internals action must match engine-internals availability"
                        .to_owned(),
                )
            }
        }

        // The projection is pane-agnostic. Use one deterministic stand-in scope to retain the pure
        // row-suffix collision check; live emission always substitutes `ctx.record.pane_id`.
        for author_id in [
            row_author_id("projection-validation", &row.artifact_sha256),
            row_state_author_id("projection-validation", &row.artifact_sha256),
            row_adapter_author_id("projection-validation", &row.artifact_sha256),
            row_role_author_id("projection-validation", &row.artifact_sha256),
            row_default_ineligible_author_id("projection-validation", &row.artifact_sha256),
            row_revision_author_id("projection-validation", &row.artifact_sha256),
            row_sha_author_id("projection-validation", &row.artifact_sha256),
            row_locator_author_id("projection-validation", &row.artifact_sha256),
            row_artifact_path_author_id("projection-validation", &row.artifact_sha256),
            row_kv_cache_author_id("projection-validation", &row.artifact_sha256),
            row_lora_author_id("projection-validation", &row.artifact_sha256),
            row_steering_author_id("projection-validation", &row.artifact_sha256),
            row_tokens_per_second_author_id("projection-validation", &row.artifact_sha256),
            row_vram_author_id("projection-validation", &row.artifact_sha256),
            row_last_call_author_id("projection-validation", &row.artifact_sha256),
            row_last_call_age_author_id("projection-validation", &row.artifact_sha256),
            row_engine_internals_author_id("projection-validation", &row.artifact_sha256),
            row_ledger_link_author_id("projection-validation", &row.artifact_sha256),
            row_action_author_id("projection-validation", &row.artifact_sha256, "quiesce"),
            row_action_author_id("projection-validation", &row.artifact_sha256, "unload"),
            row_action_author_id(
                "projection-validation",
                &row.artifact_sha256,
                "adapter-swap",
            ),
            row_action_author_id(
                "projection-validation",
                &row.artifact_sha256,
                "inspect-internals",
            ),
            row_audit_author_id("projection-validation", &row.artifact_sha256),
            row_active_selection_author_id("projection-validation", &row.artifact_sha256),
            row_active_purposes_author_id("projection-validation", &row.artifact_sha256),
            row_switch_author_id("projection-validation", &row.artifact_sha256),
        ] {
            if !author_ids.insert(author_id.clone()) {
                return Err(format!(
                    "ModelRuntime registry generates duplicate AccessKit author_id `{author_id}`"
                ));
            }
        }
    }
    if selected_rows > 1 {
        return Err("ModelRuntime registry projection contains multiple selected rows".to_owned());
    }
    Ok(())
}

fn unavailable_reason<T>(value: &ModelRuntimeValue<T>) -> Option<&str> {
    match value {
        ModelRuntimeValue::Available { .. } => None,
        ModelRuntimeValue::Unavailable { reason } => Some(reason),
    }
}

fn reason_is_empty(reason: Option<&str>) -> bool {
    reason.is_some_and(|reason| reason.trim().is_empty())
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
