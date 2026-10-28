//! Native Swarm lane diagnostics pane (WP-1 MT-008).
//!
//! The pane renders Dexterity model-lane runs, lanes, messages, payload refs,
//! promotion state, routing execution/stage/outbox lifecycle, recovery state,
//! and diagnostic tier posture. Production
//! data is fetched from the `handshake_core` PostgreSQL-backed diagnostics
//! route; tests may inject a projection into the same renderer.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use egui::accesskit;

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub use handshake_swarm_lane_diagnostics::*;


pub type SwarmLaneDiagnosticsCell =
    Arc<Mutex<Option<Result<SwarmLaneDiagnosticsProjection, String>>>>;


pub trait SwarmLaneDiagnosticsTransport: Send + Sync {
    fn fetch_latest(&self, cell: SwarmLaneDiagnosticsCell);
    fn fetch_run(&self, run_id: &str, cell: SwarmLaneDiagnosticsCell);
}

#[derive(PartialEq, Eq)]
enum FetchStartOutcome {
    Queued,
    Busy,
    Unavailable,
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

    fn start_fetch(&self, run_id: Option<&str>) -> FetchStartOutcome {
        let Some(transport) = &self.transport else {
            return FetchStartOutcome::Unavailable;
        };
        let Ok(mut state) = self.state.lock() else {
            return FetchStartOutcome::Unavailable;
        };
        if state.pending_fetch {
            return FetchStartOutcome::Busy;
        }
        state.pending_fetch = true;
        state.error = None;
        drop(state);
        match run_id {
            Some(id) if !id.trim().is_empty() => {
                transport.fetch_run(id.trim(), self.delivery.clone())
            }
            _ => transport.fetch_latest(self.delivery.clone()),
        }
        FetchStartOutcome::Queued
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

        let refresh_author_id = scoped_author_id(pane_id, REFRESH_AUTHOR_ID);
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
                let refresh_enabled = self.transport.is_some() && !state.pending_fetch;
                let refresh = ui
                    .add_enabled(refresh_enabled, egui::Button::new("Refresh"))
                    .on_disabled_hover_text(if self.transport.is_none() {
                        "Diagnostics transport unavailable"
                    } else {
                        "Diagnostics request already queued"
                    });
                refresh.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        ui.is_enabled() && refresh_enabled,
                        "Refresh",
                    )
                });
                ui.ctx().accesskit_node_builder(refresh.id, |node| {
                    node.set_role(accesskit::Role::Button);
                    node.add_action(accesskit::Action::Click);
                    node.set_author_id(refresh_author_id.clone());
                    node.set_label("Refresh".to_owned());
                });
                if refresh.clicked() {
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
            if self.start_fetch(Some(&run_id)) == FetchStartOutcome::Queued {
                crate::mcp::argus::acknowledge_action_effect_allow_target_transition(
                    ui.ctx(),
                    &refresh_author_id,
                );
                ui.ctx().request_repaint();
            }
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
