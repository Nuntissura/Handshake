use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::context_bundle::{canonical_json_bytes, sha256_hex};
use super::{
    flight_recorder_mirror_event, ArtifactRecord, ContextBundle, KernelActor, KernelError,
    KernelEvent, KernelEventType, KernelResult, KernelTaskRun, ModelAdapter, ModelAdapterRequest,
    NewKernelEvent, OperatorPromotionApproval, PromotionDecisionKind, PromotionGate, SessionRun,
    SessionRunState, ToolDecisionKind, ToolDecisionRecord, TraceProjection, ValidationOutcome,
    ValidationRunner,
};
use crate::flight_recorder::FlightRecorder;
use crate::mcp::gate::{evaluate_kernel_tool_gate_decision, KernelMcpToolGateRequest};
use crate::storage::{
    Database, ModelSessionState, NewModelSession, NewSessionMessage, SessionMessage,
    SessionMessageRole,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KernelProofResult {
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub trace: TraceProjection,
}

pub struct KernelProofRunner {
    db: Arc<dyn Database>,
    artifact_workspace_root: PathBuf,
    flight_recorder: Option<Arc<dyn FlightRecorder>>,
}

impl KernelProofRunner {
    pub fn new(db: Arc<dyn Database>) -> Self {
        Self {
            db,
            artifact_workspace_root: default_artifact_workspace_root(),
            flight_recorder: None,
        }
    }

    pub fn with_artifact_workspace_root(
        db: Arc<dyn Database>,
        artifact_workspace_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            db,
            artifact_workspace_root: artifact_workspace_root.into(),
            flight_recorder: None,
        }
    }

    pub fn with_flight_recorder(
        db: Arc<dyn Database>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            db,
            artifact_workspace_root: default_artifact_workspace_root(),
            flight_recorder: Some(flight_recorder),
        }
    }

    pub fn with_artifact_workspace_root_and_flight_recorder(
        db: Arc<dyn Database>,
        artifact_workspace_root: impl Into<PathBuf>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            db,
            artifact_workspace_root: artifact_workspace_root.into(),
            flight_recorder: Some(flight_recorder),
        }
    }

    pub async fn run_first_slice(
        &self,
        source: impl Into<String>,
        intent_payload: Value,
        adapter: &dyn ModelAdapter,
        operator_approval: OperatorPromotionApproval,
    ) -> KernelResult<KernelProofResult> {
        let source = source.into();
        let task = KernelTaskRun::new(source.clone(), intent_payload.clone());
        let session = SessionRun::queued(&task.kernel_task_run_id, adapter.adapter_id());
        let correlation_id = format!("corr-{}", task.kernel_task_run_id);
        self.ensure_model_session(&session).await?;

        let task_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                KernelEventType::TaskIntentRecorded,
                KernelActor::Operator(source),
                None,
                &correlation_id,
                json!({
                    "intent": task.intent_payload,
                    "kernel_task_run_id": task.kernel_task_run_id,
                    "authority": "postgres_event_ledger"
                }),
            )
            .await?;
        let intent_message = self
            .append_session_message(
                &session.session_run_id,
                SessionMessageRole::User,
                Some(task_event.event_id.as_str()),
                json!({
                    "kernel_event_id": task_event.event_id,
                    "intent": task.intent_payload,
                    "source": "kernel_proof_runner"
                }),
            )
            .await?;

        let (_queued_session, queued_event) = self
            .db
            .enqueue_kernel_session_run_and_record_event(
                session.clone(),
                Some(task_event.event_id.clone()),
                correlation_id.clone(),
            )
            .await?;
        self.record_flight_recorder_mirror(&queued_event).await?;

        let (_claimed_lease, claimed_event) = self
            .db
            .claim_kernel_session_run_and_record_event(
                &session.session_run_id,
                "kernel-proof-runner",
                300,
                Some(queued_event.event_id.clone()),
                correlation_id.clone(),
            )
            .await?
            .ok_or_else(|| {
                super::KernelError::Storage("kernel session claim was unavailable".to_string())
            })?;
        self.record_flight_recorder_mirror(&claimed_event).await?;

        let (_running_lease, running_event) = self
            .db
            .update_kernel_session_run_state_and_record_event(
                &session.session_run_id,
                SessionRunState::Running,
                Some(claimed_event.event_id.clone()),
                correlation_id.clone(),
            )
            .await?;
        self.record_flight_recorder_mirror(&running_event).await?;

        let context_bundle = ContextBundle::new(
            &task.kernel_task_run_id,
            &session.session_run_id,
            json!({
                "intent": task.intent_payload,
                "session_run_id": session.session_run_id,
                "tool_grants": ["read_trace"],
                "redactions": [],
                "adapter_id": adapter.adapter_id()
            }),
        )?;
        let context_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                KernelEventType::ContextBundleRecorded,
                KernelActor::System("context-bundle".to_string()),
                Some(running_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "context_bundle_id": context_bundle.context_bundle_id,
                    "context_hash": context_bundle.context_hash,
                    "allowed_context": context_bundle.allowed_context,
                    "intent_message_id": intent_message.message_id,
                    "intent_message_content_hash": intent_message.content_hash,
                    "intent_message_content_artifact_id": intent_message.content_artifact_id
                }),
            )
            .await?;
        let tool_grants: Vec<String> = context_bundle
            .allowed_context
            .get("tool_grants")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect();

        let invoke_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                KernelEventType::ModelAdapterInvoked,
                KernelActor::ModelAdapter(adapter.adapter_id().to_string()),
                Some(context_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "adapter_id": adapter.adapter_id(),
                    "context_bundle_id": context_bundle.context_bundle_id
                }),
            )
            .await?;
        let adapter_output = adapter
            .invoke(ModelAdapterRequest::new(
                context_bundle,
                KernelActor::ModelAdapter(adapter.adapter_id().to_string()),
            ))
            .await?;
        let response_message = self
            .append_session_message(
                &session.session_run_id,
                SessionMessageRole::Assistant,
                Some(invoke_event.event_id.as_str()),
                json!({
                    "kernel_event_causation_id": invoke_event.event_id,
                    "response_text": adapter_output.response_text,
                    "output_hash": adapter_output.output_hash
                }),
            )
            .await?;

        let response_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                adapter_output.response_event_type.clone(),
                KernelActor::ModelAdapter(adapter.adapter_id().to_string()),
                Some(invoke_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "context_bundle_id": adapter_output.context_bundle_id,
                    "output_hash": adapter_output.output_hash,
                    "response_text": adapter_output.response_text,
                    "response_message_id": response_message.message_id,
                    "response_message_content_hash": response_message.content_hash,
                    "response_message_content_artifact_id": response_message.content_artifact_id
                }),
            )
            .await?;

        let tool_request_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                adapter_output.tool_request.event_type.clone(),
                KernelActor::ModelAdapter(adapter.adapter_id().to_string()),
                Some(response_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "tool_request_id": adapter_output.tool_request.tool_request_id,
                    "tool_id": adapter_output.tool_request.tool_id,
                    "reason": adapter_output.tool_request.reason
                }),
            )
            .await?;

        let tool_decision = ToolDecisionRecord::from_mcp_gate_decision(
            evaluate_kernel_tool_gate_decision(
                "kernel-toolgate",
                tool_grants,
                KernelMcpToolGateRequest {
                    tool_request_id: adapter_output.tool_request.tool_request_id.clone(),
                    tool_id: adapter_output.tool_request.tool_id.clone(),
                    reason: adapter_output.tool_request.reason.clone(),
                },
            ),
            KernelEventType::ToolDecisionRecorded,
        );
        let tool_decision_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                tool_decision.event_type.clone(),
                KernelActor::ToolGate(tool_decision.gate_id.clone()),
                Some(tool_request_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "tool_decision_id": tool_decision.tool_decision_id.clone(),
                    "tool_request_id": tool_decision.tool_request_id.clone(),
                    "tool_id": tool_decision.tool_id.clone(),
                    "canonical_tool_id": tool_decision.canonical_tool_id.clone(),
                    "decision": tool_decision.decision.as_str(),
                    "reason": tool_decision.reason.clone(),
                    "policy_source": tool_decision.policy_source.clone(),
                    "gate_receipt_kind": "mcp_gate_wrapped_decision",
                    "args_ref": tool_decision.args_ref.clone(),
                    "args_hash": tool_decision.args_hash.clone(),
                    "result_ref": tool_decision.result_ref.clone(),
                    "result_hash": tool_decision.result_hash.clone(),
                    "context_bundle_id": adapter_output.context_bundle_id.clone()
                }),
            )
            .await?;
        if tool_decision.decision == ToolDecisionKind::Deny {
            return Err(super::KernelError::InvalidEvent(
                "toolgate denied required proof tool",
            ));
        }

        let artifact = ArtifactRecord::store_adapter_output(
            &self.artifact_workspace_root,
            &task.kernel_task_run_id,
            &session.session_run_id,
            &adapter_output,
        )?;
        let artifact_proposed_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                KernelEventType::ArtifactProposed,
                KernelActor::ModelAdapter(adapter.adapter_id().to_string()),
                Some(tool_decision_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "artifact_proposal_id": artifact.artifact_proposal_id,
                    "artifact_kind": artifact.artifact_kind,
                    "content_hash": artifact.content_hash
                }),
            )
            .await?;
        let artifact_stored_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                artifact.event_type.clone(),
                KernelActor::System("artifact-store".to_string()),
                Some(artifact_proposed_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "artifact_id": artifact.artifact_id.clone(),
                    "artifact_proposal_id": artifact.artifact_proposal_id.clone(),
                    "content_hash": artifact.content_hash.clone(),
                    "artifact_manifest_ref": artifact.artifact_manifest_ref.clone(),
                    "artifact_payload_ref": artifact.artifact_payload_ref.clone(),
                    "artifact_layer": artifact.artifact_layer.as_str(),
                    "artifact_uuid": artifact.artifact_uuid
                }),
            )
            .await?;

        let validation = ValidationRunner::record(
            &task.kernel_task_run_id,
            &session.session_run_id,
            &artifact,
            ValidationOutcome::Passed,
            json!({
                "proof": "deterministic kernel first-slice validation",
                "artifact_id": artifact.artifact_id.clone(),
                "content_hash": artifact.content_hash.clone(),
                "artifact_manifest_ref": artifact.artifact_manifest_ref.clone(),
                "artifact_content_hash_validated": true,
                "evidence_refs": [artifact.artifact_manifest_ref.clone()]
            }),
        )?;
        let validation_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                validation.event_type.clone(),
                KernelActor::ValidationRunner("kernel-validation-runner".to_string()),
                Some(artifact_stored_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "validation_id": validation.validation_id,
                    "artifact_id": validation.artifact_id,
                    "outcome": validation.outcome,
                    "content_hash": validation.evidence["content_hash"].clone(),
                    "artifact_manifest_ref": validation.evidence["artifact_manifest_ref"].clone(),
                    "artifact_content_hash_validated": validation.evidence["artifact_content_hash_validated"].clone(),
                    "evidence_refs": validation.evidence["evidence_refs"].clone()
                }),
            )
            .await?;

        let promotion = PromotionGate::decide(
            &artifact,
            Some(&validation),
            PromotionDecisionKind::Approved,
            operator_approval,
        )?;
        let promotion_event = self
            .append_event(
                &task.kernel_task_run_id,
                &session.session_run_id,
                promotion.event_type.clone(),
                KernelActor::PromotionGate("kernel-promotion-gate".to_string()),
                Some(validation_event.event_id.as_str()),
                &correlation_id,
                json!({
                    "promotion_decision_id": promotion.promotion_decision_id,
                    "artifact_id": promotion.artifact_id,
                    "validation_id": promotion.validation_id,
                    "decision": promotion.decision,
                    "operator_id": promotion.operator_id,
                    "operator_reason": promotion.operator_reason,
                    "operator_review": {
                        "review_required": true,
                        "approval_source": promotion.operator_approval_source,
                        "review_receipt_id": promotion.operator_review_receipt_id
                    }
                }),
            )
            .await?;

        let completed_causation_id = if self.flight_recorder.is_some() {
            let mirror_ledger_event = self
                .append_event(
                    &task.kernel_task_run_id,
                    &session.session_run_id,
                    KernelEventType::FlightRecorderMirrorRecorded,
                    KernelActor::System("flight-recorder-mirror".to_string()),
                    Some(promotion_event.event_id.as_str()),
                    &correlation_id,
                    json!({
                        "mirrored_kernel_event_id": promotion_event.event_id,
                        "mirrored_kernel_event_type": promotion_event.event_type.as_str(),
                        "authority_source": "postgres_event_ledger",
                        "mirror_sink": "flight_recorder",
                        "projection_only": true
                    }),
                )
                .await?;
            mirror_ledger_event.event_id
        } else {
            promotion_event.event_id.clone()
        };

        let (_completed_lease, completed_event) = self
            .db
            .update_kernel_session_run_state_and_record_event(
                &session.session_run_id,
                SessionRunState::Completed,
                Some(completed_causation_id),
                correlation_id.clone(),
            )
            .await?;
        self.record_flight_recorder_mirror(&completed_event).await?;

        let events = self
            .db
            .list_kernel_events_for_session(&session.session_run_id)
            .await?;
        let trace = TraceProjection::from_events(
            task.kernel_task_run_id.clone(),
            session.session_run_id.clone(),
            events,
        )?;

        Ok(KernelProofResult {
            kernel_task_run_id: task.kernel_task_run_id,
            session_run_id: session.session_run_id,
            trace,
        })
    }

    async fn append_event(
        &self,
        kernel_task_run_id: &str,
        session_run_id: &str,
        event_type: KernelEventType,
        actor: KernelActor,
        causation_id: Option<&str>,
        correlation_id: &str,
        payload: Value,
    ) -> KernelResult<KernelEvent> {
        let mut builder =
            NewKernelEvent::builder(kernel_task_run_id, session_run_id, event_type, actor)
                .correlation_id(correlation_id)
                .payload(payload);
        if let Some(causation_id) = causation_id {
            builder = builder.causation_id(causation_id);
        }
        let event = self.db.append_kernel_event(builder.build()?).await?;
        self.record_flight_recorder_mirror(&event).await?;
        Ok(event)
    }

    async fn record_flight_recorder_mirror(&self, event: &KernelEvent) -> KernelResult<()> {
        if event.event_type == KernelEventType::FlightRecorderMirrorRecorded {
            return Ok(());
        }
        let Some(flight_recorder) = &self.flight_recorder else {
            return Ok(());
        };
        flight_recorder
            .record_event(flight_recorder_mirror_event(event))
            .await
            .map_err(|err| KernelError::FlightRecorder(err.to_string()))?;
        Ok(())
    }

    async fn ensure_model_session(&self, session: &SessionRun) -> KernelResult<()> {
        self.db
            .upsert_model_session(NewModelSession {
                session_id: session.session_run_id.clone(),
                parent_session_id: None,
                spawn_depth: 0,
                state: ModelSessionState::Active,
                model_id: session.adapter_id.clone(),
                backend: "kernel_model_adapter".to_string(),
                parameter_class: "deterministic_dummy".to_string(),
                role: "KERNEL_BUILDER_PROOF".to_string(),
                wp_id: Some("WP-KERNEL-001-Event-Ledger-Session-Broker-v1".to_string()),
                mt_id: None,
                work_profile_id: Some("KERNEL_V1_FIRST_SLICE".to_string()),
                execution_mode: "kernel_first_slice".to_string(),
                memory_policy: "event_ledger_authority".to_string(),
                consent_receipt_id: None,
                capability_grants: vec!["read_trace".to_string()],
                capability_token_ids: None,
                job_id: None,
                checkpoint_artifact_id: None,
                last_checkpoint_at: None,
                checkpoint_count: 0,
                agent: Some(format!("KERNEL_BUILDER_PROOF:{}", session.adapter_id)),
                purpose: Some("kernel first-slice proof session".to_string()),
            })
            .await?;
        Ok(())
    }

    async fn append_session_message(
        &self,
        session_run_id: &str,
        role: SessionMessageRole,
        linked_kernel_event_id: Option<&str>,
        payload: Value,
    ) -> KernelResult<SessionMessage> {
        let content_hash = sha256_hex(&canonical_json_bytes(&payload));
        let mut attachments = vec![format!("kernel_message_payload_hash:{content_hash}")];
        if let Some(event_id) = linked_kernel_event_id {
            attachments.push(format!("kernel_event_id:{event_id}"));
        }
        let message = self
            .db
            .append_session_message(NewSessionMessage {
                message_id: None,
                session_id: session_run_id.to_string(),
                role,
                content_hash: content_hash.clone(),
                content_artifact_id: format!("kernel-message-{content_hash}"),
                token_count: None,
                redacted: false,
                tool_call_id: None,
                attachments,
            })
            .await?;
        Ok(message)
    }
}

fn default_artifact_workspace_root() -> PathBuf {
    std::env::var("HANDSHAKE_KERNEL_ARTIFACT_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("handshake-kernel-artifacts"))
}
