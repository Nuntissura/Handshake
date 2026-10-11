//! Native Swarm lane diagnostics pane (WP-1 MT-008).
//!
//! The pane renders Dexterity model-lane runs, lanes, messages, payload refs,
//! promotion state, routing execution/stage/outbox lifecycle, recovery state,
//! and diagnostic tier posture. Production
//! data is fetched from the `handshake_core` PostgreSQL-backed diagnostics
//! route; tests may inject a projection into the same renderer.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use egui::accesskit;
use serde::{Deserialize, Serialize};

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const SURFACE_CONTRACT_ID: &str = "native_swarm_lane_diagnostics";
pub const PROJECTION_SCHEMA_ID: &str = "hsk.model_lane_diagnostics_projection@3";
pub const SURFACE_AUTHOR_ID: &str = "swarm-lane-diagnostics.surface";
pub const RUN_FILTER_AUTHOR_ID: &str = "swarm-lane-diagnostics.filter.run";
pub const LANE_FILTER_AUTHOR_ID: &str = "swarm-lane-diagnostics.filter.lane";
pub const MESSAGE_FILTER_AUTHOR_ID: &str = "swarm-lane-diagnostics.filter.message";
pub const REFRESH_AUTHOR_ID: &str = "swarm-lane-diagnostics.action.refresh";
pub const ERROR_AUTHOR_ID: &str = "swarm-lane-diagnostics.error";
pub const FRESHNESS_AUTHOR_ID: &str = "swarm-lane-diagnostics.freshness";
pub const PRIVACY_OWNER_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.owner-account";
pub const PRIVACY_PRINCIPAL_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.actor-principal";
pub const PRIVACY_SESSION_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.authenticated-session";
pub const PRIVACY_ACCESS_SPACE_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.access-space";
pub const PRIVACY_WORKSPACE_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.workspace";
pub const PRIVACY_VISIBILITY_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.visibility";
pub const PRIVACY_DENIAL_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.denial-posture";
pub const EMPTY_MESSAGES_AUTHOR_ID: &str = "swarm-lane-diagnostics.messages.empty";

pub type SwarmLaneDiagnosticsCell =
    Arc<Mutex<Option<Result<SwarmLaneDiagnosticsProjection, String>>>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub run: SwarmLaneDiagnosticsRun,
    pub lanes: Vec<SwarmLaneDiagnosticsLane>,
    pub messages: Vec<SwarmLaneDiagnosticsMessage>,
    pub diagnostic_tiers: Vec<SwarmLaneDiagnosticsTier>,
    pub mt_runtime_statuses: Vec<SwarmLaneDiagnosticsMtStatus>,
    pub routing_executions: Vec<SwarmLaneRoutingExecutionDiagnostics>,
    pub active_lease_count: usize,
    pub reclaimable_lease_ids: Vec<String>,
    pub orphan_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_scope: Option<SwarmLaneDiagnosticsResourceScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsResourceScope {
    pub owner_account_fingerprint: String,
    pub actor_principal_fingerprint: String,
    pub authenticated_session_fingerprint: String,
    pub access_space_fingerprint: String,
    pub workspace_fingerprint: String,
    pub visibility: String,
    pub denial_posture: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsRun {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    #[serde(default)]
    pub coordinator_session_id: String,
    #[serde(default)]
    pub routing_policy: String,
    #[serde(default)]
    pub artifact_namespace: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    #[serde(default)]
    pub owner_session: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub context_bundle_id: String,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub status: String,
    pub recovery_hint_ref: Option<String>,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    #[serde(default)]
    pub budget_summary_ref: String,
    #[serde(default)]
    pub determinism_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsLane {
    pub lane_id: String,
    pub kind: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub backend: String,
    pub status: String,
    pub recovery_state: String,
    pub model_id: Option<String>,
    pub model_display_name: String,
    pub model_stable_anchor: Option<String>,
    pub model_anchor_unavailable_reason: Option<String>,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub model_session_id: String,
    #[serde(default)]
    pub adapter_id: String,
    pub provider_kind: String,
    pub runtime_binding: String,
    #[serde(default)]
    pub launch_authority: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub trace_id: String,
    pub lane_span_id: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub last_activity_utc: Option<String>,
    pub message_count: usize,
    pub payload_error_count: usize,
    pub orphan_state: String,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    #[serde(default)]
    pub owner_session: String,
    pub locus_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsMessage {
    pub message_id: String,
    pub from_lane_id: String,
    #[serde(default)]
    pub to_lane: String,
    pub routing_target_role: Option<String>,
    pub routing_target_session: Option<String>,
    pub routing_correlation_id: Option<String>,
    #[serde(default)]
    pub routing_requires_ack: bool,
    pub routing_ack_for: Option<String>,
    pub kind: String,
    pub authority: String,
    pub promotion_state: String,
    pub payload_ref: String,
    pub payload_sha256: String,
    pub artifact_ref: Option<String>,
    pub promotion_decision_id: Option<String>,
    pub promotion_gate_ref: Option<String>,
    pub promotion_receipt_ref: Option<String>,
    pub validator_verdict_ref: Option<String>,
    pub operator_decision_ref: Option<String>,
    pub promoted_artifact_sha256: Option<String>,
    pub promoted_artifact_version: Option<String>,
    #[serde(default)]
    pub tool_gate_decision_refs: Vec<String>,
    #[serde(default)]
    pub coordinator_session_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    #[serde(default)]
    pub owner_session: String,
    pub trace_id: String,
    pub message_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub proposal_ref: Option<String>,
    pub crdt_update_ref: Option<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_proposal_ref: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub payload_error: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsTier {
    pub tier: String,
    pub state: String,
    pub reason: String,
    pub evidence_ref: String,
    pub follow_up_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsMtStatus {
    pub micro_task_id: String,
    pub status: String,
    pub proof_status_ref: Option<String>,
    pub hbr_status_ref: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneRoutingOutboxDiagnostics {
    pub command_id: String,
    pub status: String,
    pub fencing_token: Option<String>,
    pub lease_owner: Option<String>,
    pub lease_expires_at_unix_ms: Option<u64>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneRoutingStageDiagnostics {
    pub execution_id: String,
    pub stage_id: String,
    pub state: String,
    pub attempt: u32,
    pub dispatch_target: String,
    pub dependency_stage_ids: Vec<String>,
    pub expected_run_id: String,
    pub expected_lane_id: String,
    pub expected_model_id: String,
    pub expected_provider: Option<String>,
    pub instance_id: Option<String>,
    pub lane_id: Option<String>,
    pub input_refs: Vec<String>,
    pub output_ref: Option<String>,
    pub output_message_ref: Option<String>,
    pub authority_request_message_ref: Option<String>,
    pub output_sha256: Option<String>,
    pub authority_ref: Option<String>,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<String>,
    pub lease_expires_at_unix_ms: Option<u64>,
    pub lease_expired: bool,
    pub detail: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub updated_at_unix_ms: u64,
    pub outbox: SwarmLaneRoutingOutboxDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneRoutingExecutionDiagnostics {
    pub execution_id: String,
    pub run_id: String,
    pub selecting_decision_id: String,
    pub selecting_decision_event_id: String,
    pub selecting_decision_event_seq: i64,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub locus_ref: String,
    pub work_packet_id: String,
    pub micro_task_id: Option<String>,
    pub task_board_id: String,
    pub owner_session: String,
    pub canonical_graph_sha256: String,
    pub canonical_launch_plan_sha256: String,
    pub cloud_consent_receipt_ref: Option<String>,
    pub validator_authority_ref: Option<String>,
    pub operator_authority_ref: Option<String>,
    pub initial_input_ref: Option<String>,
    pub initial_input_sha256: Option<String>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub revision: u64,
    pub stages: Vec<SwarmLaneRoutingStageDiagnostics>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

pub trait SwarmLaneDiagnosticsTransport: Send + Sync {
    fn fetch_latest(&self, cell: SwarmLaneDiagnosticsCell);
    fn fetch_run(&self, run_id: &str, cell: SwarmLaneDiagnosticsCell);
}

#[derive(Default)]
struct DiagnosticsUiState {
    projection: Option<SwarmLaneDiagnosticsProjection>,
    run_filter: String,
    lane_filter: String,
    message_filter: String,
    selected_message_id: Option<String>,
    pending_fetch: bool,
    error: Option<String>,
    projection_stale: bool,
    last_success_unix_ms: Option<u128>,
}

pub struct SwarmLaneDiagnosticsPaneFactory {
    state: Arc<Mutex<DiagnosticsUiState>>,
    transport: Option<Arc<dyn SwarmLaneDiagnosticsTransport>>,
    delivery: SwarmLaneDiagnosticsCell,
}

impl SwarmLaneDiagnosticsPaneFactory {
    pub fn offline() -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticsUiState::default())),
            transport: None,
            delivery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_projection(projection: SwarmLaneDiagnosticsProjection) -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticsUiState {
                projection: Some(projection),
                last_success_unix_ms: Some(now_unix_ms()),
                ..Default::default()
            })),
            transport: None,
            delivery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_transport(transport: Arc<dyn SwarmLaneDiagnosticsTransport>) -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticsUiState::default())),
            transport: Some(transport),
            delivery: Arc::new(Mutex::new(None)),
        }
    }

    fn start_fetch(&self, run_id: Option<&str>) {
        let Some(transport) = &self.transport else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.pending_fetch {
                return;
            }
            state.pending_fetch = true;
            state.error = None;
        }
        match run_id {
            Some(id) if !id.trim().is_empty() => {
                transport.fetch_run(id.trim(), self.delivery.clone())
            }
            _ => transport.fetch_latest(self.delivery.clone()),
        }
    }

    fn drain_delivery(&self) {
        let delivered = self.delivery.lock().ok().and_then(|mut slot| slot.take());
        if let Some(result) = delivered {
            if let Ok(mut state) = self.state.lock() {
                state.pending_fetch = false;
                match result {
                    Ok(projection) => {
                        if let Err(error) = validate_projection_for_native_surface(&projection) {
                            state.error = Some(error);
                            state.projection_stale = state.projection.is_some();
                        } else {
                            state.run_filter = projection.run.run_id.clone();
                            state.projection = Some(projection);
                            state.error = None;
                            state.projection_stale = false;
                            state.last_success_unix_ms = Some(now_unix_ms());
                        }
                    }
                    Err(error) => {
                        state.error = Some(error);
                        state.projection_stale = state.projection.is_some();
                    }
                }
            }
        }
    }
}

impl PaneFactory for SwarmLaneDiagnosticsPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::SwarmLaneDiagnostics
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        self.drain_delivery();
        if self.transport.is_some()
            && self
                .state
                .lock()
                .ok()
                .is_some_and(|state| state.projection.is_none() && !state.pending_fetch)
        {
            self.start_fetch(ctx.record.content_id.as_deref());
        }

        let Ok(mut state) = self.state.lock() else {
            ui.label("Diagnostics state unavailable");
            return;
        };

        let pane_id: &str = ctx.record.pane_id.as_ref();

        let mut fetch_after_render: Option<String> = None;
        ui.vertical(|ui| {
            // Bind the surface landmark to the rendered heading response. Builder-only custom nodes
            // are discoverable but receive no AccessKit bounds; a genuine widget response supplies
            // positive live geometry for Argus validation and visual correlation.
            let heading = ui.heading("Swarm Lane Diagnostics");
            ui.ctx().accesskit_node_builder(heading.id, |node| {
                node.set_role(accesskit::Role::Group);
                node.set_author_id(scoped_author_id(pane_id, SURFACE_AUTHOR_ID));
                node.set_label("Swarm Lane Diagnostics".to_owned());
            });
            ui.horizontal(|ui| {
                ui.label("Run");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.run_filter)
                        .id_source(ctx.egui_id.with("swarm-lane-diagnostics-run-filter"))
                        .desired_width(260.0),
                );
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(scoped_author_id(pane_id, RUN_FILTER_AUTHOR_ID));
                    node.set_label("Run filter".to_owned());
                });
                let refresh = ui.button("Refresh");
                refresh.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Refresh")
                });
                ui.ctx().accesskit_node_builder(refresh.id, |node| {
                    node.set_role(accesskit::Role::Button);
                    node.add_action(accesskit::Action::Click);
                    node.set_author_id(scoped_author_id(pane_id, REFRESH_AUTHOR_ID));
                    node.set_label("Refresh".to_owned());
                });
                if refresh.clicked() {
                    crate::mcp::argus::acknowledge_action_effect(
                        ui.ctx(),
                        &scoped_author_id(pane_id, REFRESH_AUTHOR_ID),
                    );
                    fetch_after_render = Some(state.run_filter.clone());
                }
            });
            ui.horizontal(|ui| {
                ui.label("Lane");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.lane_filter)
                        .id_source(ctx.egui_id.with("swarm-lane-diagnostics-lane-filter"))
                        .desired_width(220.0),
                );
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(scoped_author_id(pane_id, LANE_FILTER_AUTHOR_ID));
                    node.set_label("Lane filter".to_owned());
                });
                ui.label("Message");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.message_filter)
                        .id_source(ctx.egui_id.with("swarm-lane-diagnostics-message-filter"))
                        .desired_width(220.0),
                );
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(scoped_author_id(pane_id, MESSAGE_FILTER_AUTHOR_ID));
                    node.set_label("Message filter".to_owned());
                });
            });

            if state.pending_fetch {
                ui.label("Loading Dexterity model-lane diagnostics...");
            }
            if let Some(error) = state.error.as_deref() {
                let resp = ui.colored_label(ui.visuals().error_fg_color, error);
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id(scoped_author_id(pane_id, ERROR_AUTHOR_ID));
                    node.set_label(error.to_owned());
                });
            }
            if let Some(last_success) = state.last_success_unix_ms {
                tagged_label(
                    ui,
                    ctx.egui_id.with("swarm-lane-diagnostics-freshness"),
                    &scoped_author_id(pane_id, FRESHNESS_AUTHOR_ID),
                    &format!(
                        "{} | last success unix_ms {last_success}",
                        if state.projection_stale {
                            "STALE"
                        } else {
                            "CURRENT"
                        }
                    ),
                );
            }

            let Some(projection) = state.projection.clone() else {
                ui.label("No Dexterity model-lane run loaded.");
                return;
            };

            // Keep the two operator-critical filter outcomes above the long scrollable telemetry body.
            // AccessKit bounds alone are insufficient if the painted state is clipped below the pane.
            let visible_message_ids = visible_message_ids_for_filters(
                &projection,
                state.lane_filter.as_str(),
                state.message_filter.as_str(),
            );
            if visible_message_ids.is_empty() {
                tagged_label(
                    ui,
                    ctx.egui_id.with("messages-empty"),
                    &scoped_author_id(pane_id, EMPTY_MESSAGES_AUTHOR_ID),
                    "No messages match the active lane/message filters.",
                );
            } else if let Some(selected) = state
                .selected_message_id
                .as_deref()
                .filter(|id| visible_message_ids.iter().any(|visible| visible == *id))
                .and_then(|id| projection.messages.iter().find(|message| message.message_id == id))
            {
                render_selected_message_summary(ui, ctx, pane_id, selected);
            }

            egui::ScrollArea::vertical()
                .id_salt(ctx.egui_id.with("swarm-lane-diagnostics-scroll"))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    render_projection(ui, ctx, pane_id, &projection, &mut state);
                });
        });
        drop(state);
        if let Some(run_id) = fetch_after_render {
            self.start_fetch(Some(&run_id));
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Pane
    }
}

fn render_projection(
    ui: &mut egui::Ui,
    ctx: &PaneRenderContext,
    pane_id: &str,
    projection: &SwarmLaneDiagnosticsProjection,
    state: &mut DiagnosticsUiState,
) {
    ui.separator();
    tagged_label(
        ui,
        ctx.egui_id.with("run-row"),
        &scoped_author_id(pane_id, &run_author_id(&projection.run.run_id)),
        &format!(
            "run {} | status {} | routing {} | coordinator {} | lanes {} | messages {} | event {}#{} | trace {} | FR {}",
            projection.run.run_id,
            projection.run.status,
            projection.run.routing_policy,
            projection.run.coordinator_session_id,
            projection.lanes.len(),
            projection.messages.len(),
            projection.run.event_ledger_event_id,
            projection.run.event_ledger_seq,
            projection.run.trace_id,
            projection.run.flight_recorder_correlation_id
        ),
    );
    tagged_label(
        ui,
        ctx.egui_id.with("context-row"),
        &scoped_author_id(pane_id, "swarm-lane-diagnostics.context"),
        &format!(
            "context {} | namespace {} | memory {} {} | Locus {} | Loom {} | FEMS {} | WP {} | MT {} | owner {}",
            projection.run.context_bundle_id,
            projection.run.artifact_namespace,
            projection.run.memory_pack_ref,
            projection.run.memory_pack_hash,
            projection.run.locus_ref.as_deref().unwrap_or("missing"),
            projection.run.loom_ref.as_deref().unwrap_or("missing"),
            projection.run.fems_ref.as_deref().unwrap_or("missing"),
            projection.run.work_packet_id.as_deref().unwrap_or("missing"),
            projection.run.micro_task_id.as_deref().unwrap_or("missing"),
            projection.run.owner_session
        ),
    );
    tagged_label(
        ui,
        ctx.egui_id.with("recovery-row"),
        &scoped_author_id(pane_id, "swarm-lane-diagnostics.recovery"),
        &format!(
            "active leases {} | reclaimable {:?} | orphan state {}",
            projection.active_lease_count,
            projection.reclaimable_lease_ids,
            projection.orphan_state
        ),
    );
    if let Some(scope) = projection.resource_scope.as_ref() {
        for (id, author_id, label, value) in [
            (
                "privacy-owner-account",
                PRIVACY_OWNER_AUTHOR_ID,
                "active account",
                scope.owner_account_fingerprint.as_str(),
            ),
            (
                "privacy-actor-principal",
                PRIVACY_PRINCIPAL_AUTHOR_ID,
                "acting Principal",
                scope.actor_principal_fingerprint.as_str(),
            ),
            (
                "privacy-authenticated-session",
                PRIVACY_SESSION_AUTHOR_ID,
                "authenticated session",
                scope.authenticated_session_fingerprint.as_str(),
            ),
            (
                "privacy-access-space",
                PRIVACY_ACCESS_SPACE_AUTHOR_ID,
                "active AccessSpace",
                scope.access_space_fingerprint.as_str(),
            ),
            (
                "privacy-workspace",
                PRIVACY_WORKSPACE_AUTHOR_ID,
                "workspace",
                scope.workspace_fingerprint.as_str(),
            ),
        ] {
            tagged_label(
                ui,
                ctx.egui_id.with(id),
                &scoped_author_id(pane_id, author_id),
                &format!("{label} verified | process-keyed fingerprint {value} | visibility exact-scope only"),
            );
        }
        tagged_label(
            ui,
            ctx.egui_id.with("privacy-visibility"),
            &scoped_author_id(pane_id, PRIVACY_VISIBILITY_AUTHOR_ID),
            &format!("visibility {} | exact account + Principal + session + AccessSpace + workspace | this read cannot widen sharing", scope.visibility),
        );
        tagged_label(
            ui,
            ctx.egui_id.with("privacy-denial-posture"),
            &scoped_author_id(pane_id, PRIVACY_DENIAL_AUTHOR_ID),
            &format!("denial posture {} | mismatched caller scope: forbidden | foreign stored scope: not found | restricted metadata withheld", scope.denial_posture),
        );
    } else {
        tagged_label(
            ui,
            ctx.egui_id.with("privacy-scope-unavailable"),
            &scoped_author_id(pane_id, ERROR_AUTHOR_ID),
            "privacy scope unavailable | projection is not authorized for operator display",
        );
    }

    ui.separator();
    ui.label("Routing lifecycle");
    for execution in &projection.routing_executions {
        tagged_label(
            ui,
            ctx.egui_id.with(("routing-execution", &execution.execution_id)),
            &scoped_author_id(
                pane_id,
                &routing_execution_author_id(&execution.execution_id),
            ),
            &format!(
                "execution {} | status {} | revision {} | failure {} | cancel {} | authority cloud={} validator={} operator={} | decision {} event {}#{} | routing event {}#{}",
                execution.execution_id,
                execution.status,
                execution.revision,
                execution.failure_reason.as_deref().unwrap_or("none"),
                execution.cancel_reason.as_deref().unwrap_or("none"),
                execution.cloud_consent_receipt_ref.as_deref().unwrap_or("none"),
                execution.validator_authority_ref.as_deref().unwrap_or("none"),
                execution.operator_authority_ref.as_deref().unwrap_or("none"),
                execution.selecting_decision_id,
                execution.selecting_decision_event_id,
                execution.selecting_decision_event_seq,
                execution.event_ledger_event_id,
                execution.event_ledger_seq,
            ),
        );
        for stage in &execution.stages {
            tagged_label(
                ui,
                ctx.egui_id.with((
                    "routing-stage",
                    &execution.execution_id,
                    &stage.stage_id,
                    stage.attempt,
                )),
                &scoped_author_id(
                    pane_id,
                    &routing_stage_author_id(
                        &execution.execution_id,
                        &stage.stage_id,
                        stage.attempt,
                    ),
                ),
                &format!(
                    "stage {} | state {} attempt {} target {} deps {:?} | lease owner={} fence={} expires={} expired={} | input {:?} | output {} message {} sha {} | authority {} request {} | event {}#{} | detail {}",
                    stage.stage_id,
                    stage.state,
                    stage.attempt,
                    stage.dispatch_target,
                    stage.dependency_stage_ids,
                    stage.lease_owner.as_deref().unwrap_or("none"),
                    stage.fencing_token.as_deref().unwrap_or("none"),
                    stage
                        .lease_expires_at_unix_ms
                        .map(|value| value.to_string())
                        .as_deref()
                        .unwrap_or("none"),
                    stage.lease_expired,
                    stage.input_refs,
                    stage.output_ref.as_deref().unwrap_or("none"),
                    stage.output_message_ref.as_deref().unwrap_or("none"),
                    stage.output_sha256.as_deref().unwrap_or("none"),
                    stage.authority_ref.as_deref().unwrap_or("none"),
                    stage
                        .authority_request_message_ref
                        .as_deref()
                        .unwrap_or("none"),
                    stage.event_ledger_event_id,
                    stage.event_ledger_seq,
                    stage.detail.as_deref().unwrap_or("none"),
                ),
            );
            tagged_label(
                ui,
                ctx.egui_id.with(("routing-outbox", &stage.outbox.command_id)),
                &scoped_author_id(
                    pane_id,
                    &routing_outbox_author_id(&stage.outbox.command_id),
                ),
                &format!(
                    "outbox {} | status {} | lease owner={} fence={} expires={} | event {}#{} | updated {}",
                    stage.outbox.command_id,
                    stage.outbox.status,
                    stage.outbox.lease_owner.as_deref().unwrap_or("none"),
                    stage.outbox.fencing_token.as_deref().unwrap_or("none"),
                    stage
                        .outbox
                        .lease_expires_at_unix_ms
                        .map(|value| value.to_string())
                        .as_deref()
                        .unwrap_or("none"),
                    stage.outbox.event_ledger_event_id,
                    stage.outbox.event_ledger_seq,
                    stage.outbox.updated_at_unix_ms,
                ),
            );
        }
    }

    ui.separator();
    ui.label("Lanes");
    let visible_lane_ids = visible_lane_ids_for_filter(projection, &state.lane_filter);
    for lane in projection
        .lanes
        .iter()
        .filter(|lane| visible_lane_ids.contains(&lane.lane_id))
    {
        tagged_label(
            ui,
            ctx.egui_id.with(("lane", &lane.lane_id)),
            &scoped_author_id(pane_id, &lane_author_id(&lane.lane_id)),
            &format!(
                "{} | {} role {} | session {} model-session {} | {} | recovery {} | runtime {} via {} | messages {} | payload errors {} | event {}#{} | span {} | FR {} | Locus {} | runtime-status {} | recovery-hint {}",
                lane.lane_id,
                lane.kind,
                lane.role,
                lane.session_id,
                lane.model_session_id,
                lane.status,
                lane.recovery_state,
                lane.runtime_binding,
                lane.launch_authority,
                lane.message_count,
                lane.payload_error_count,
                lane.event_ledger_event_id,
                lane.event_ledger_seq,
                lane.lane_span_id,
                lane.flight_recorder_correlation_id,
                lane.locus_ref.as_deref().unwrap_or("missing"),
                lane.last_runtime_status_ref.as_deref().unwrap_or("missing"),
                lane.recovery_hint_ref.as_deref().unwrap_or("missing")
            ),
        );
        tagged_label(
            ui,
            ctx.egui_id.with(("lane-model-id", &lane.lane_id)),
            &scoped_author_id(pane_id, &lane_model_identity_author_id(&lane.lane_id)),
            &format!("model id {}", lane.model_id.as_deref().unwrap_or("none")),
        );
        tagged_label(
            ui,
            ctx.egui_id.with(("lane-model-label", &lane.lane_id)),
            &scoped_author_id(pane_id, &lane_model_label_author_id(&lane.lane_id)),
            &format!(
                "model label {} | stable anchor {} | anchor status {}",
                lane.model_display_name,
                lane.model_stable_anchor.as_deref().unwrap_or("none"),
                lane.model_anchor_unavailable_reason
                    .as_deref()
                    .unwrap_or("resolved")
            ),
        );
    }

    ui.separator();
    ui.label("Messages");
    let visible_message_ids = projection
        .messages
        .iter()
        .filter(|message| {
            message_is_visible_for_filters(
                message,
                &visible_lane_ids,
                state.message_filter.as_str(),
            )
        })
        .map(|message| message.message_id.clone())
        .collect::<BTreeSet<_>>();
    if state
        .selected_message_id
        .as_ref()
        .is_some_and(|message_id| !visible_message_ids.contains(message_id))
    {
        state.selected_message_id = None;
    }
    for message in projection
        .messages
        .iter()
        .filter(|message| visible_message_ids.contains(&message.message_id))
    {
        ui.horizontal_wrapped(|ui| {
            let row = ui.selectable_label(
                state.selected_message_id.as_deref() == Some(message.message_id.as_str()),
                format!(
                    "{} | {} | {} -> {} | route {} | promotion {} | payload {} | CRDT {}",
                    message.message_id,
                    message.kind,
                    message.authority,
                    message.to_lane,
                    message.routing_correlation_id.as_deref().unwrap_or("none"),
                    message.promotion_state,
                    message.payload_ref,
                    message.crdt_update_ref.as_deref().unwrap_or("none")
                ),
            );
            ui.ctx().accesskit_node_builder(row.id, |node| {
                node.set_author_id(scoped_author_id(
                    pane_id,
                    &message_author_id(&message.message_id),
                ));
                node.set_label(format!("Message {}", message.message_id));
            });
            if row.clicked() {
                crate::mcp::argus::acknowledge_action_effect(
                    ui.ctx(),
                    &scoped_author_id(pane_id, &message_author_id(&message.message_id)),
                );
                state.selected_message_id = Some(message.message_id.clone());
            }

            let payload = diagnostics_button(
                ui,
                ctx.egui_id.with(("message-payload", &message.message_id)),
                scoped_author_id(pane_id, &message_payload_author_id(&message.message_id)),
                "Payload",
                format!("Payload {}", message.payload_ref),
            );
            if payload.clicked() {
                crate::mcp::argus::acknowledge_action_effect(
                    ui.ctx(),
                    &scoped_author_id(
                        pane_id,
                        &message_payload_author_id(&message.message_id),
                    ),
                );
                state.selected_message_id = Some(message.message_id.clone());
            }

            let promotion = diagnostics_button(
                ui,
                ctx.egui_id.with(("message-promotion", &message.message_id)),
                scoped_author_id(pane_id, &message_promotion_author_id(&message.message_id)),
                "Promotion",
                format!("Promotion {}", message.promotion_state),
            );
            if promotion.clicked() {
                crate::mcp::argus::acknowledge_action_effect(
                    ui.ctx(),
                    &scoped_author_id(
                        pane_id,
                        &message_promotion_author_id(&message.message_id),
                    ),
                );
                state.selected_message_id = Some(message.message_id.clone());
            }
        });
    }

    ui.separator();
    ui.label("Diagnostic Tiers");
    for tier in &projection.diagnostic_tiers {
        tagged_label(
            ui,
            ctx.egui_id.with(("tier", &tier.tier)),
            &scoped_author_id(pane_id, &tier_author_id(&tier.tier)),
            &format!(
                "{} | {} | reason {} | evidence {} | follow-up {}",
                tier.tier,
                tier.state,
                tier.reason,
                tier.evidence_ref,
                tier.follow_up_ref.as_deref().unwrap_or("none")
            ),
        );
    }

    ui.separator();
    ui.label("MT Runtime Status");
    for status in &projection.mt_runtime_statuses {
        tagged_label(
            ui,
            ctx.egui_id.with(("mt-status", &status.micro_task_id)),
            &scoped_author_id(pane_id, &mt_status_author_id(&status.micro_task_id)),
            &format!(
                "{} | {} | proof {} | HBR {} | event {}#{}",
                status.micro_task_id,
                status.status,
                status.proof_status_ref.as_deref().unwrap_or("none"),
                status.hbr_status_ref.as_deref().unwrap_or("none"),
                status.event_ledger_event_id,
                status.event_ledger_seq
            ),
        );
    }
}

fn render_selected_message_summary(
    ui: &mut egui::Ui,
    ctx: &PaneRenderContext,
    pane_id: &str,
    selected: &SwarmLaneDiagnosticsMessage,
) {
    tagged_label(
        ui,
        ctx.egui_id.with("selected-message"),
        &scoped_author_id(pane_id, &selected_message_author_id(&selected.message_id)),
        &format!(
            "selected {} | payload {} sha {} | artifact {} | promotion {} | decision {} gate {} receipt {} validator {} operator {} | proposal {} CRDT update {} proposal {} base {} vector {} stale {} | trace {} span {} parent {} links {:?} | WP {} MT {} owner {} | recovery {}",
            selected.message_id,
            selected.payload_ref,
            selected.payload_sha256,
            selected.artifact_ref.as_deref().unwrap_or("none"),
            selected.promotion_state,
            selected.promotion_decision_id.as_deref().unwrap_or("none"),
            selected.promotion_gate_ref.as_deref().unwrap_or("none"),
            selected.promotion_receipt_ref.as_deref().unwrap_or("none"),
            selected.validator_verdict_ref.as_deref().unwrap_or("none"),
            selected.operator_decision_ref.as_deref().unwrap_or("none"),
            selected.proposal_ref.as_deref().unwrap_or("none"),
            selected.crdt_update_ref.as_deref().unwrap_or("none"),
            selected.crdt_proposal_ref.as_deref().unwrap_or("none"),
            selected.crdt_base_snapshot_ref.as_deref().unwrap_or("none"),
            selected.crdt_state_vector.as_deref().unwrap_or("none"),
            selected.crdt_stale_base_ref.as_deref().unwrap_or("none"),
            selected.trace_id,
            selected.message_span_id,
            selected.parent_span_id.as_deref().unwrap_or("none"),
            selected.linked_span_contexts,
            selected.work_packet_id.as_deref().unwrap_or("missing"),
            selected.micro_task_id.as_deref().unwrap_or("missing"),
            selected.owner_session,
            selected.recovery_hint_ref.as_deref().unwrap_or("missing")
        ),
    );
}

fn tagged_label(ui: &mut egui::Ui, _id: egui::Id, author_id: &str, text: &str) {
    let response = ui.label(text);
    let label = text.to_owned();
    // Decorate the genuine label response instead of emitting a parallel builder-only node. The
    // response carries the painted widget's live bounds; the former synthetic node was discoverable
    // but had `bounds: null` and required an `::egui-response` duplicate.
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Label);
        node.set_author_id(author_id.to_owned());
        node.set_label(label);
    });
}

fn diagnostics_button(
    ui: &mut egui::Ui,
    id: egui::Id,
    author_id: String,
    visible_label: &str,
    access_label: String,
) -> egui::Response {
    let font = egui::FontId::proportional(12.0);
    let text_color = ui.visuals().text_color();
    let galley = ui
        .painter()
        .layout_no_wrap(visible_label.to_owned(), font, text_color);
    let padding = egui::vec2(8.0, 3.0);
    let desired = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let response = ui.interact(rect, id, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            3.0,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let text_pos = egui::pos2(
            rect.left() + padding.x,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, text_color);
    }

    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            access_label.clone(),
        )
    });
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::Button);
        node.add_action(accesskit::Action::Click);
        node.set_author_id(author_id);
        node.set_label(access_label);
    });

    response
}

fn token(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

pub fn scoped_author_id(pane_id: &str, logical_author_id: &str) -> String {
    format!("{logical_author_id}.pane.{}", token(pane_id))
}

fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn lane_matches_filter(lane: &SwarmLaneDiagnosticsLane, lane_filter: &str) -> bool {
    let lane_filter = lane_filter.trim().to_ascii_lowercase();
    lane_filter.is_empty()
        || lane.lane_id.to_ascii_lowercase().contains(&lane_filter)
        || lane.kind.to_ascii_lowercase().contains(&lane_filter)
        || lane.status.to_ascii_lowercase().contains(&lane_filter)
        || lane
            .model_id
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(&lane_filter)
        || lane
            .model_display_name
            .to_ascii_lowercase()
            .contains(&lane_filter)
}

fn visible_lane_ids_for_filter(
    projection: &SwarmLaneDiagnosticsProjection,
    lane_filter: &str,
) -> BTreeSet<String> {
    projection
        .lanes
        .iter()
        .filter(|lane| lane_matches_filter(lane, lane_filter))
        .map(|lane| lane.lane_id.clone())
        .collect()
}

fn message_matches_filter(message: &SwarmLaneDiagnosticsMessage, message_filter: &str) -> bool {
    let message_filter = message_filter.trim().to_ascii_lowercase();
    message_filter.is_empty()
        || message
            .message_id
            .to_ascii_lowercase()
            .contains(&message_filter)
        || message
            .from_lane_id
            .to_ascii_lowercase()
            .contains(&message_filter)
        || message
            .payload_ref
            .to_ascii_lowercase()
            .contains(&message_filter)
        || message
            .promotion_state
            .to_ascii_lowercase()
            .contains(&message_filter)
}

fn message_is_visible_for_filters(
    message: &SwarmLaneDiagnosticsMessage,
    visible_lane_ids: &BTreeSet<String>,
    message_filter: &str,
) -> bool {
    visible_lane_ids.contains(&message.from_lane_id)
        && message_matches_filter(message, message_filter)
}

pub fn visible_message_ids_for_filters(
    projection: &SwarmLaneDiagnosticsProjection,
    lane_filter: &str,
    message_filter: &str,
) -> Vec<String> {
    let visible_lane_ids = visible_lane_ids_for_filter(projection, lane_filter);
    projection
        .messages
        .iter()
        .filter(|message| {
            message_is_visible_for_filters(message, &visible_lane_ids, message_filter)
        })
        .map(|message| message.message_id.clone())
        .collect()
}

fn register_author_id(
    author_ids: &mut BTreeMap<String, String>,
    scope: &str,
    raw_id: &str,
    author_id: String,
) -> Result<(), String> {
    let raw_id = raw_id.trim();
    if raw_id.is_empty() {
        return Err(format!("{scope} author_id missing"));
    }
    let rendered_token = token(raw_id);
    if rendered_token.trim_matches('-').is_empty() {
        return Err(format!("{scope} author_id unusable for {raw_id}"));
    }
    let source = format!("{scope}:{raw_id}");
    if let Some(existing) = author_ids.insert(author_id.clone(), source.clone()) {
        return Err(format!(
            "duplicate AccessKit author_id {author_id} for {existing} and {source}"
        ));
    }
    Ok(())
}

pub fn run_author_id(run_id: &str) -> String {
    format!("swarm-lane-diagnostics.run.{}", token(run_id))
}

pub fn routing_execution_author_id(execution_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.routing.execution.{}",
        token(execution_id)
    )
}

pub fn routing_stage_author_id(execution_id: &str, stage_id: &str, attempt: u32) -> String {
    format!(
        "swarm-lane-diagnostics.routing.execution.{}.stage.{}.attempt.{attempt}",
        token(execution_id),
        token(stage_id)
    )
}

pub fn routing_outbox_author_id(command_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.routing.outbox.{}",
        token(command_id)
    )
}

pub fn lane_author_id(lane_id: &str) -> String {
    format!("swarm-lane-diagnostics.lane.{}", token(lane_id))
}

pub fn lane_model_identity_author_id(lane_id: &str) -> String {
    format!("swarm-lane-diagnostics.lane.{}.model-id", token(lane_id))
}

pub fn lane_model_label_author_id(lane_id: &str) -> String {
    format!("swarm-lane-diagnostics.lane.{}.model-label", token(lane_id))
}

pub fn message_author_id(message_id: &str) -> String {
    format!("swarm-lane-diagnostics.message.{}", token(message_id))
}

pub fn message_payload_author_id(message_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.message.{}.payload",
        token(message_id)
    )
}

pub fn message_promotion_author_id(message_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.message.{}.promotion",
        token(message_id)
    )
}

pub fn selected_message_author_id(message_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.message.{}.selected",
        token(message_id)
    )
}

pub fn tier_author_id(tier: &str) -> String {
    format!("swarm-lane-diagnostics.tier.{}", token(tier))
}

pub fn mt_status_author_id(mt_id: &str) -> String {
    format!("swarm-lane-diagnostics.mt-status.{}", token(mt_id))
}

pub fn validate_projection_for_native_surface(
    projection: &SwarmLaneDiagnosticsProjection,
) -> Result<(), String> {
    let mut author_ids = BTreeMap::new();
    if projection.schema_id != PROJECTION_SCHEMA_ID {
        return Err(format!("schema_id mismatch: {}", projection.schema_id));
    }
    if projection.surface_contract_id != SURFACE_CONTRACT_ID {
        return Err(format!(
            "surface_contract_id mismatch: {}",
            projection.surface_contract_id
        ));
    }
    if projection.run.run_id.trim().is_empty() {
        return Err("run_id missing".to_owned());
    }
    let scope = projection.resource_scope.as_ref().ok_or_else(|| {
        "diagnostics resource_scope attribution is required before rendering".to_owned()
    })?;
    if [
        scope.owner_account_fingerprint.as_str(),
        scope.actor_principal_fingerprint.as_str(),
        scope.authenticated_session_fingerprint.as_str(),
        scope.access_space_fingerprint.as_str(),
        scope.workspace_fingerprint.as_str(),
    ]
    .iter()
    .any(|value| value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err("diagnostics resource_scope attribution fingerprint is invalid".to_owned());
    }
    if scope.visibility != "private_exact_scope_only"
        || scope.denial_posture != "foreign_scope_is_absent_restricted_metadata_withheld"
    {
        return Err("diagnostics resource_scope privacy posture is invalid".to_owned());
    }
    register_author_id(
        &mut author_ids,
        "run",
        &projection.run.run_id,
        run_author_id(&projection.run.run_id),
    )?;
    if projection.run.coordinator_session_id.trim().is_empty()
        || projection.run.routing_policy.trim().is_empty()
        || projection.run.artifact_namespace.trim().is_empty()
        || projection.run.owner_session.trim().is_empty()
        || projection.run.candidate_model_ids.is_empty()
        || projection
            .run
            .candidate_model_ids
            .iter()
            .any(|model_id| model_id.trim().is_empty())
        || projection.run.budget_summary_ref.trim().is_empty()
        || projection.run.determinism_mode.trim().is_empty()
    {
        return Err("run coordinator/routing/owner/model/recovery refs missing".to_owned());
    }
    if projection
        .run
        .work_packet_id
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
        || projection
            .run
            .micro_task_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        || projection
            .run
            .task_board_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        || projection
            .run
            .recovery_hint_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err("run taskboard or recovery refs missing".to_owned());
    }
    if projection.run.event_ledger_event_id.trim().is_empty()
        || projection
            .run
            .flight_recorder_correlation_id
            .trim()
            .is_empty()
    {
        return Err("run EventLedger or FlightRecorder ref missing".to_owned());
    }
    let mut execution_ids = BTreeSet::new();
    for execution in &projection.routing_executions {
        register_author_id(
            &mut author_ids,
            "routing execution",
            &execution.execution_id,
            routing_execution_author_id(&execution.execution_id),
        )?;
        if !execution_ids.insert(execution.execution_id.clone()) {
            return Err(format!(
                "duplicate routing execution {}",
                execution.execution_id
            ));
        }
        if execution.run_id != projection.run.run_id
            || execution.trace_id != projection.run.trace_id
            || execution.run_span_id != projection.run.run_span_id
            || execution.coordinator_session_id != projection.run.coordinator_session_id
            || Some(execution.work_packet_id.as_str()) != projection.run.work_packet_id.as_deref()
            || execution.micro_task_id.as_deref() != projection.run.micro_task_id.as_deref()
            || Some(execution.task_board_id.as_str()) != projection.run.task_board_id.as_deref()
            || execution.owner_session != projection.run.owner_session
        {
            return Err(format!(
                "routing execution {} run/trace/coordinator lineage mismatch",
                execution.execution_id
            ));
        }
        if execution.selecting_decision_id.trim().is_empty()
            || execution.selecting_decision_event_id.trim().is_empty()
            || execution.selecting_decision_event_seq <= 0
            || execution.event_ledger_event_id.trim().is_empty()
            || execution.event_ledger_seq <= 0
            || execution.canonical_graph_sha256.trim().is_empty()
            || execution.canonical_launch_plan_sha256.trim().is_empty()
            || execution.locus_ref.trim().is_empty()
            || execution.work_packet_id.trim().is_empty()
            || execution.task_board_id.trim().is_empty()
            || execution.owner_session.trim().is_empty()
        {
            return Err(format!(
                "routing execution {} authority/EventLedger/task lineage missing",
                execution.execution_id
            ));
        }
        match execution.status.as_str() {
            "failed"
                if execution
                    .failure_reason
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty() =>
            {
                return Err(format!(
                    "routing execution {} failed without failure_reason",
                    execution.execution_id
                ));
            }
            "cancelled"
                if execution
                    .cancel_reason
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty() =>
            {
                return Err(format!(
                    "routing execution {} cancelled without cancel_reason",
                    execution.execution_id
                ));
            }
            "running" | "awaiting_authority" | "succeeded" | "failed" | "cancelled" => {}
            unsupported => {
                return Err(format!(
                    "routing execution {} status unsupported: {unsupported}",
                    execution.execution_id
                ));
            }
        }
        let stage_ids = execution
            .stages
            .iter()
            .map(|stage| stage.stage_id.clone())
            .collect::<BTreeSet<_>>();
        if stage_ids.len() != execution.stages.len() {
            return Err(format!(
                "routing execution {} has duplicate current stage lineage",
                execution.execution_id
            ));
        }
        for stage in &execution.stages {
            register_author_id(
                &mut author_ids,
                "routing stage",
                &format!(
                    "{}:{}:{}",
                    execution.execution_id, stage.stage_id, stage.attempt
                ),
                routing_stage_author_id(&execution.execution_id, &stage.stage_id, stage.attempt),
            )?;
            register_author_id(
                &mut author_ids,
                "routing outbox",
                &stage.outbox.command_id,
                routing_outbox_author_id(&stage.outbox.command_id),
            )?;
            if stage.execution_id != execution.execution_id
                || stage.expected_run_id != execution.run_id
                || stage.event_ledger_event_id.trim().is_empty()
                || stage.event_ledger_seq <= 0
                || (stage.state != "scheduled" && stage.input_refs.is_empty())
                || stage
                    .input_refs
                    .iter()
                    .any(|reference| reference.trim().is_empty())
            {
                return Err(format!(
                    "routing stage {}/{} execution/run/input/EventLedger lineage mismatch",
                    execution.execution_id, stage.stage_id
                ));
            }
            if stage
                .dependency_stage_ids
                .iter()
                .any(|dependency| dependency == &stage.stage_id || !stage_ids.contains(dependency))
            {
                return Err(format!(
                    "routing stage {}/{} dependency lineage missing or tampered",
                    execution.execution_id, stage.stage_id
                ));
            }
            let expected_command_id = format!(
                "routing-command:{}:{}:{}",
                execution.execution_id, stage.stage_id, stage.attempt
            );
            if stage.outbox.command_id != expected_command_id
                || stage.outbox.event_ledger_event_id.trim().is_empty()
                || stage.outbox.event_ledger_seq <= 0
            {
                return Err(format!(
                    "routing stage {}/{} durable outbox lineage mismatch",
                    execution.execution_id, stage.stage_id
                ));
            }
            let expected_outbox_status = match stage.state.as_str() {
                "scheduled" => "pending",
                "claimed" | "in_flight" | "awaiting_authority" => "claimed",
                "cancelled" => "cancelled",
                "compensated" => "compensated",
                "succeeded" | "failed" | "joined" => "acked",
                unsupported => {
                    return Err(format!(
                        "routing stage {}/{} state unsupported: {unsupported}",
                        execution.execution_id, stage.stage_id
                    ));
                }
            };
            if stage.outbox.status != expected_outbox_status {
                return Err(format!(
                    "routing stage {}/{} outbox state {} does not match lifecycle {}",
                    execution.execution_id, stage.stage_id, stage.outbox.status, stage.state
                ));
            }
            let active = matches!(
                stage.state.as_str(),
                "claimed" | "in_flight" | "awaiting_authority"
            );
            if active
                && (stage
                    .lease_owner
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                    || stage
                        .fencing_token
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    || stage.lease_expires_at_unix_ms.is_none()
                    || stage.outbox.lease_owner != stage.lease_owner
                    || stage.outbox.fencing_token != stage.fencing_token
                    || stage.outbox.lease_expires_at_unix_ms != stage.lease_expires_at_unix_ms)
            {
                return Err(format!(
                    "routing stage {}/{} active lease fencing lineage missing",
                    execution.execution_id, stage.stage_id
                ));
            }
            if !active
                && (stage.lease_owner.is_some()
                    || stage.fencing_token.is_some()
                    || stage.lease_expires_at_unix_ms.is_some()
                    || stage.outbox.lease_owner.is_some()
                    || stage.outbox.fencing_token.is_some()
                    || stage.outbox.lease_expires_at_unix_ms.is_some())
            {
                return Err(format!(
                    "routing stage {}/{} terminal lifecycle retained active lease",
                    execution.execution_id, stage.stage_id
                ));
            }
            if stage.state == "awaiting_authority"
                && (stage
                    .authority_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                    || stage
                        .authority_request_message_ref
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty())
            {
                return Err(format!(
                    "routing stage {}/{} awaiting authority without causal authority lineage",
                    execution.execution_id, stage.stage_id
                ));
            }
            let output_lineage_count = usize::from(stage.output_ref.is_some())
                + usize::from(stage.output_message_ref.is_some())
                + usize::from(stage.output_sha256.is_some());
            if output_lineage_count != 0 && output_lineage_count != 3 {
                return Err(format!(
                    "routing stage {}/{} output lineage incomplete",
                    execution.execution_id, stage.stage_id
                ));
            }
        }
    }
    let lane_count: usize = projection.lanes.iter().map(|lane| lane.message_count).sum();
    if lane_count != projection.messages.len() {
        return Err(format!(
            "lane message_count mismatch: lanes sum {lane_count}, messages {}",
            projection.messages.len()
        ));
    }
    let mut lane_message_counts = BTreeMap::<String, usize>::new();
    let mut lane_ids = BTreeSet::new();
    for lane in &projection.lanes {
        register_author_id(
            &mut author_ids,
            "lane",
            &lane.lane_id,
            lane_author_id(&lane.lane_id),
        )?;
        register_author_id(
            &mut author_ids,
            "lane model identity",
            &lane.lane_id,
            lane_model_identity_author_id(&lane.lane_id),
        )?;
        register_author_id(
            &mut author_ids,
            "lane model label",
            &lane.lane_id,
            lane_model_label_author_id(&lane.lane_id),
        )?;
        lane_ids.insert(lane.lane_id.clone());
    }
    for message in &projection.messages {
        if !lane_ids.contains(&message.from_lane_id) {
            return Err(format!(
                "message {} references unknown lane {}",
                message.message_id, message.from_lane_id
            ));
        }
        *lane_message_counts
            .entry(message.from_lane_id.clone())
            .or_insert(0) += 1;
        if let Some(target_lane_id) = message.to_lane.strip_prefix("lane:") {
            if target_lane_id.trim().is_empty() || !lane_ids.contains(target_lane_id) {
                return Err(format!(
                    "message {} references unknown to_lane {}",
                    message.message_id, message.to_lane
                ));
            }
        } else if message.to_lane != "coordinator" && message.to_lane != "broadcast" {
            return Err(format!(
                "message {} routing target unsupported: {}",
                message.message_id, message.to_lane
            ));
        }
    }
    for lane in &projection.lanes {
        let actual_message_count = lane_message_counts
            .get(&lane.lane_id)
            .copied()
            .unwrap_or_default();
        if actual_message_count != lane.message_count {
            return Err(format!(
                "lane {} message_count mismatch: lane says {}, messages contain {}",
                lane.lane_id, lane.message_count, actual_message_count
            ));
        }
        if lane.role.trim().is_empty()
            || lane.backend.trim().is_empty()
            || lane.model_display_name.trim().is_empty()
            || lane.session_id.trim().is_empty()
            || lane.model_session_id.trim().is_empty()
            || lane.adapter_id.trim().is_empty()
            || lane.launch_authority.trim().is_empty()
            || lane.owner_session.trim().is_empty()
        {
            return Err(format!(
                "lane {} session/model/runtime authority refs missing",
                lane.lane_id
            ));
        }
        if lane.capability_token_ids.is_empty()
            || lane
                .capability_token_ids
                .iter()
                .any(|token_id| token_id.trim().is_empty())
            || lane.tool_gate_decision_refs.is_empty()
            || lane
                .tool_gate_decision_refs
                .iter()
                .any(|decision_ref| decision_ref.trim().is_empty())
        {
            return Err(format!(
                "lane {} capability/ToolGate authority refs missing",
                lane.lane_id
            ));
        }
        if lane
            .locus_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || lane
                .work_packet_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || lane
                .micro_task_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || lane
                .task_board_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "lane {} Locus/taskboard refs missing",
                lane.lane_id
            ));
        }
        if lane
            .last_runtime_status_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || lane
                .recovery_hint_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "lane {} runtime recovery refs missing",
                lane.lane_id
            ));
        }
        if lane.flight_recorder_correlation_id.trim().is_empty() {
            return Err(format!("lane {} FlightRecorder ref missing", lane.lane_id));
        }
    }
    for message in &projection.messages {
        register_author_id(
            &mut author_ids,
            "message",
            &message.message_id,
            message_author_id(&message.message_id),
        )?;
        register_author_id(
            &mut author_ids,
            "message payload",
            &message.message_id,
            message_payload_author_id(&message.message_id),
        )?;
        register_author_id(
            &mut author_ids,
            "message promotion",
            &message.message_id,
            message_promotion_author_id(&message.message_id),
        )?;
        register_author_id(
            &mut author_ids,
            "selected message",
            &message.message_id,
            selected_message_author_id(&message.message_id),
        )?;
        if message.payload_ref.trim().is_empty() || message.event_ledger_event_id.trim().is_empty()
        {
            return Err(format!(
                "message {} payload/EventLedger ref missing",
                message.message_id
            ));
        }
        if message.flight_recorder_correlation_id.trim().is_empty() {
            return Err(format!(
                "message {} FlightRecorder ref missing",
                message.message_id
            ));
        }
        if message.to_lane.trim().is_empty()
            || message.coordinator_session_id.trim().is_empty()
            || message.owner_session.trim().is_empty()
        {
            return Err(format!(
                "message {} routing/session refs missing",
                message.message_id
            ));
        }
        if message
            .routing_target_role
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || message
                .routing_target_session
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .routing_correlation_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "message {} routing target refs missing",
                message.message_id
            ));
        }
        if message
            .work_packet_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || message
                .micro_task_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .task_board_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .locus_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "message {} Locus/taskboard refs missing",
                message.message_id
            ));
        }
        if message.authority == "promotion_candidate"
            && message.promotion_state == "decision_recorded"
            && (message
                .promotion_decision_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
                || message
                    .promotion_gate_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                || message
                    .promotion_receipt_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty())
        {
            return Err(format!(
                "message {} promotion refs missing",
                message.message_id
            ));
        }
        let targets_crdt = message.crdt_update_ref.is_some()
            || message.crdt_base_snapshot_ref.is_some()
            || message.crdt_state_vector.is_some()
            || message.crdt_proposal_ref.is_some()
            || message.crdt_stale_base_ref.is_some();
        if targets_crdt
            && (message
                .proposal_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
                || message
                    .crdt_update_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                || message
                    .crdt_base_snapshot_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                || message
                    .crdt_state_vector
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                || message
                    .crdt_proposal_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty())
        {
            return Err(format!("message {} CRDT refs missing", message.message_id));
        }
    }
    for expected in ["flight_recorder", "internal_diagnostics", "palmistry"] {
        if !projection
            .diagnostic_tiers
            .iter()
            .any(|tier| tier.tier == expected)
        {
            return Err(format!("diagnostic tier {expected} missing"));
        }
    }
    for tier in &projection.diagnostic_tiers {
        register_author_id(
            &mut author_ids,
            "diagnostic tier",
            &tier.tier,
            tier_author_id(&tier.tier),
        )?;
        if tier.reason.trim().is_empty() {
            return Err(format!("diagnostic tier {} reason missing", tier.tier));
        }
        if tier.evidence_ref.trim().is_empty() {
            return Err(format!("diagnostic tier {} evidence missing", tier.tier));
        }
        if tier.state.trim().is_empty() || tier.state == "missing" {
            return Err(format!(
                "diagnostic tier {} state cannot be missing",
                tier.tier
            ));
        }
        if tier.state != "wired"
            && tier.state != "not_applicable_with_reason"
            && tier.state != "deferred_with_reason"
        {
            return Err(format!(
                "diagnostic tier {} state unsupported: {}",
                tier.tier, tier.state
            ));
        }
        if tier.state == "deferred_with_reason" {
            if tier
                .follow_up_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(format!(
                    "diagnostic tier {} deferred follow_up_ref missing",
                    tier.tier
                ));
            }
        }
    }
    if projection.mt_runtime_statuses.is_empty() {
        return Err("MT runtime status rows missing".to_owned());
    }
    let expected_micro_task_id = projection.run.micro_task_id.as_deref().unwrap_or_default();
    let mut covers_run_micro_task = false;
    for status in &projection.mt_runtime_statuses {
        register_author_id(
            &mut author_ids,
            "MT runtime status",
            &status.micro_task_id,
            mt_status_author_id(&status.micro_task_id),
        )?;
        if status.micro_task_id == expected_micro_task_id {
            covers_run_micro_task = true;
        }
        if status.status.trim().is_empty()
            || status.event_ledger_event_id.trim().is_empty()
            || status.event_ledger_seq <= 0
            || status
                .proof_status_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || status
                .hbr_status_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "MT runtime status {} status/proof/HBR/EventLedger ref missing",
                status.micro_task_id
            ));
        }
    }
    if !covers_run_micro_task {
        return Err(format!(
            "MT runtime status for run micro_task_id {expected_micro_task_id} missing"
        ));
    }
    Ok(())
}
