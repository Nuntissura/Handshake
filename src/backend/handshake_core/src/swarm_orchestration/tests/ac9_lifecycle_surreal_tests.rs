//! SYNTHETIC, no-egress AC9 gate evidence. This resolver checks canonical kernel
//! evidence; it does not establish production rejection of forged ModelLane refs.

use std::path::PathBuf;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use futures::FutureExt;

use serde_json::json;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use tower::ServiceExt;

use crate::AccountBootstrapState;
use crate::capabilities::CapabilityRegistry;
use crate::kernel::{
    DummyEchoModelAdapter, KernelActor, KernelError, KernelEvent, KernelEventType,
    KernelProofRunner, KernelResult, ModelAdapter, ModelAdapterOutput, ModelAdapterRequest,
    OperatorPromotionApproval, TraceProjection,
};
use crate::storage::Database;
use crate::storage::artifacts::{
    ArtifactLayer, read_artifact_manifest, read_file_artifact_with_manifest,
};
use crate::storage::surreal::resource_authority::{
    AuthorizationRequest, IssuedSessionCredential, ResourceAction, ResourceKind,
};
use crate::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest,
    LaunchAuthority, ModelLaneAuthority, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLanePromotionOutcome, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneRoutingPolicy, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneMessage, NewModelLanePromotionDecision, NewModelLaneRun, RuntimeBinding,
};
use sha2::{Digest, Sha256};
use crate::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
    ExactResourceScopeAttribution, ResourceScope, WorkspaceScopeRef,
};
use crate::swarm_orchestration::routing::{ModelLaneRoutingGraph, ModelLaneRoutingStageLaunchPlan};
use crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionContext;

use super::ac9_surreal_test_store_support::IsolatedSurrealTestStore;

const SYNTHETIC_OPERATOR: &str = "synthetic-ac9-operator";

struct NativeBindingFile {
    _lock: std::sync::MutexGuard<'static, ()>,
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
    channel: String,
}

impl NativeBindingFile {
    fn new(root: &std::path::Path) -> Self {
        let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK.lock()
            .unwrap_or_else(|error| error.into_inner());
        std::fs::create_dir_all(root).expect("create assigned AC9 artifact directory");
        let directory = tempfile::tempdir_in(root).expect("isolate native binding file");
        let channel = hex::encode(Sha256::digest(uuid::Uuid::now_v7().as_bytes()));
        let path = directory.path().join("native-binding.json");
        std::fs::write(&path, serde_json::to_vec(
            &crate::api::stage::current_process_native_session_binding(&channel),
        ).expect("serialize actual process birth binding")).expect("write isolated native binding");
        let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
        Self { _lock: lock, _directory: directory, previous, channel }
    }
}

impl Drop for NativeBindingFile {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
        } else {
            std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
        }
    }
}

struct Ac9AuthenticatedWorkspace {
    session_token: String,
    channel_binding_hash: String,
    workspace_id: Option<String>,
}

struct Ac9VersionedGateAdapter {
    input: serde_json::Value,
    artifact_version: String,
}

#[async_trait::async_trait]
impl ModelAdapter for Ac9VersionedGateAdapter {
    fn adapter_id(&self) -> &str { "synthetic-ac9-versioned-no-egress" }

    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput> {
        let mut output = DummyEchoModelAdapter::new(self.adapter_id()).invoke(request).await?;
        output.artifact_payload = json!({
            "artifact_version": self.artifact_version,
            "routing": self.input,
            "adapter_output": output.artifact_payload,
        });
        output.output_hash = crate::kernel::context_bundle::sha256_hex(
            &crate::kernel::context_bundle::canonical_json_bytes(&output.artifact_payload),
        );
        output.artifact_proposal.content_hash = output.output_hash.clone();
        output.artifact_proposal.artifact_proposal_id = format!("AP-{}", &output.output_hash[16..32]);
        output.tool_request.tool_request_id = format!("TOOLREQ-{}", &output.output_hash[..16]);
        Ok(output)
    }
}

pub(super) struct Ac9GateEvidence {
    pub(super) trace: TraceProjection,
    pub(super) gate: KernelEvent,
    pub(super) artifact: KernelEvent,
    pub(super) validation: KernelEvent,
    pub(super) decision_id: String,
    pub(super) operator_review_receipt_id: String,
    pub(super) artifact_payload: serde_json::Value,
    pub(super) artifact_version: Option<String>,
}

pub(super) struct Ac9Harness {
    isolated: IsolatedSurrealTestStore,
    scope: Option<ResourceScope>,
    bootstrap: Option<AccountBootstrapState>,
    capability_registry: Arc<CapabilityRegistry>,
    store: Option<ModelLaneStore>,
    db: Option<Arc<dyn Database>>,
    artifact_workspace_root: PathBuf,
    native_binding: Option<NativeBindingFile>,
    authenticated: Option<Ac9AuthenticatedWorkspace>,
}

impl Ac9Harness {
    pub(super) async fn create(_label: &str, artifact_workspace_root: PathBuf) -> Self {
        let isolated = IsolatedSurrealTestStore::create()
            .await
            .expect("allocate isolated AC9 canonical store");
        let db: Arc<dyn Database> = Arc::new(isolated.storage().clone());
        let capability_registry = Arc::new(CapabilityRegistry::new());
        let bootstrap = AccountBootstrapState::new(
            db.clone(), isolated.storage().clone(), capability_registry.clone(),
        );
        Self {
            isolated,
            scope: None,
            bootstrap: Some(bootstrap),
            capability_registry,
            store: None,
            db: Some(db),
            artifact_workspace_root,
            native_binding: None,
            authenticated: None,
        }
    }

    pub(super) fn store(&self) -> &ModelLaneStore {
        self.store.as_ref().expect("real authenticated Create setup must bind the canonical ModelLane store")
    }

    pub(super) fn scope(&self) -> &ResourceScope {
        self.scope.as_ref().expect("scope must come from actual login and workspace producer outputs")
    }

    pub(super) fn bootstrap(&self) -> &AccountBootstrapState {
        self.bootstrap.as_ref().expect("open canonical bootstrap state")
    }

    pub(super) fn bootstrap_router(&self) -> (
        axum::Router,
        tokio::sync::mpsc::Receiver<crate::api::authority::WorkspaceActivationRequest>,
    ) {
        let (activation, receiver) = tokio::sync::mpsc::channel(16);
        let state = self.bootstrap().clone();
        let router = crate::api::authority::bootstrap_routes(state.clone(), activation)
            .merge(crate::api::workspaces::bootstrap_routes(state));
        (router, receiver)
    }

    async fn authenticate_workspace(&mut self) {
        if self.authenticated.is_some() {
            return;
        }
        self.bootstrap().capability_registry.profile_by_id("Operator")
            .expect("canonical loaded Operator profile required");
        self.native_binding = Some(NativeBindingFile::new(&self.artifact_workspace_root));
        let channel = self.native_binding.as_ref().expect("owned native binding").channel.clone();
        let mut headers = HeaderMap::new();
        headers.insert(crate::api::stage::HSK_HEADER_CHANNEL_BINDING_TOKEN, channel.parse().unwrap());
        let channel_binding_hash = crate::api::stage::capture_channel_binding(&headers)
            .expect("actual native file/process birth/channel validation").binding_hash;
        let (router, activation) = self.bootstrap_router();
        let status = router_json(&router, "GET", "/authority/setup", &channel, None, json!(null)).await;
        assert_eq!(status.0, StatusCode::OK, "read real setup status");
        let credentials = json!({
            "account_name": "Synthetic AC9 operator",
            "password": "synthetic isolated AC9 password without egress",
        });
        if status.1["setup_required"].as_bool().expect("actual setup status boolean") {
            let setup = router_json(&router, "POST", "/authority/setup", &channel, None, credentials.clone()).await;
            assert_eq!(setup.0, StatusCode::OK, "real one-time account setup");
            assert_eq!(setup.1["setup_required"], false);
        }
        let login = router_json(&router, "POST", "/authority/login", &channel, None, credentials).await;
        assert_eq!(login.0, StatusCode::OK, "real password login");
        let credential: IssuedSessionCredential = serde_json::from_value(login.1)
            .expect("actual login credential response");
        let session = router_json(&router, "POST", "/authority/session", &channel, None, json!({
            "account_id": credential.account_id,
            "principal_id": credential.principal_id,
            "access_space_id": credential.access_space_id,
            "authentication_token": credential.token,
        })).await;
        assert_eq!(session.0, StatusCode::OK, "real durable session and OS-bound vault exchange");
        let session = session.1;
        let session_token = string_field(&session, "session_token").unwrap().to_owned();
        self.authenticated = Some(Ac9AuthenticatedWorkspace {
            session_token: session_token.clone(), channel_binding_hash, workspace_id: None,
        });
        assert_eq!(string_field(&session, "account_id").unwrap(), credential.account_id);
        assert_eq!(string_field(&session, "principal_id").unwrap(), credential.principal_id);
        assert_eq!(string_field(&session, "access_space_id").unwrap(), credential.access_space_id);
        let workspace = router_json(&router, "POST", "/workspaces", &channel, Some(&session_token),
            json!({"name": "Synthetic isolated AC9 workspace"})).await;
        assert_eq!(workspace.0, StatusCode::CREATED, "real atomic workspace/resource/Create-grant producer");
        let workspace_id = string_field(&workspace.1, "id").unwrap().to_owned();
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(uuid::Uuid::parse_str(string_field(&session, "account_id").unwrap()).unwrap()),
            ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(string_field(&session, "principal_id").unwrap()).unwrap()),
        ).with_session(AuthenticatedSessionRef::from_uuid(
            uuid::Uuid::parse_str(string_field(&session, "session_id").unwrap()).unwrap(),
        )).with_access_space(AccessSpaceRef::from_uuid(
            uuid::Uuid::parse_str(string_field(&session, "access_space_id").unwrap()).unwrap(),
        )).with_workspace(WorkspaceScopeRef::new(workspace_id.clone()).unwrap());
        self.store = Some(ModelLaneStore::new_scoped_with_lifecycle(
            self.isolated.storage().clone(), scope.clone(),
            self.isolated.storage().resource_access_lifecycle(),
        ));
        self.scope = Some(scope);
        self.authenticated.as_mut().expect("session cleanup already owned").workspace_id = Some(workspace_id);
        drop(router);
        drop(activation);
    }

    pub(super) async fn produce_gate_evidence(&self) -> KernelResult<Ac9GateEvidence> {
        let adapter = DummyEchoModelAdapter::new("synthetic-ac9-no-egress");
        self.produce_gate_with_adapter(&adapter,
            json!({"synthetic": true, "network_egress": false, "purpose": "AC9 gate evidence"}),
        ).await
    }

    /// Acquires versioned kernel gate evidence for actual normalized inputs.
    /// Separate cloud/validator/operator authority and accepted routing execution
    /// are not established by this artifact producer.
    pub(super) async fn produce_versioned_gate_evidence(
        &mut self,
        context: &ModelLaneRoutingExecutionContext,
        policy: ModelLaneRoutingPolicy,
        requests: Vec<(ModelLaneRoutingStageLaunchPlan, DexterityLaunchAdapterRequest)>,
        artifact_version: &str,
    ) -> KernelResult<Ac9GateEvidence> {
        self.authenticate_workspace().await;
        if artifact_version.trim().is_empty() {
            return Err(KernelError::InvalidEvent("producer artifact version required before gate"));
        }
        let graph = ModelLaneRoutingGraph::for_policy(policy);
        graph.validate().map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
        let registry = DexterityLaunchAdapterRegistry::standard();
        let mut plans = Vec::new();
        let mut records = Vec::new();
        for (plan, request) in requests {
            let stage = graph.stages.iter().find(|stage| stage.stage_id == plan.stage_id)
                .ok_or(KernelError::InvalidEvent("normalized input is not a canonical stage"))?;
            if stage.target == crate::swarm_orchestration::routing::ModelLaneRoutingDispatchTarget::CoordinatorJoin {
                return Err(KernelError::InvalidEvent("coordinator join has no normalized launch input"));
            }
            if plan.dispatch_target != stage.target || plans.iter().any(|prior: &ModelLaneRoutingStageLaunchPlan| prior.stage_id == plan.stage_id) {
                return Err(KernelError::InvalidEvent("normalized stage target or identity differs"));
            }
            let normalized = registry.normalize(request)
                .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
            let (run, lane) = normalized.clone().to_records()
                .map_err(|error| KernelError::ModelRuntime(error.to_string()))?;
            use crate::swarm_orchestration::routing::ModelLaneRoutingDispatchTarget as Target;
            let target_matches = match stage.target {
                Target::LocalModel => lane.kind == ModelLaneKind::LocalModel
                    && lane.provider_kind == ModelLaneProviderKind::LocalRuntime && plan.provider.is_none(),
                Target::CloudModel => match &lane.provider_kind {
                    ModelLaneProviderKind::OpenAi | ModelLaneProviderKind::Anthropic =>
                        lane.kind == ModelLaneKind::CloudModel && plan.provider == Some(crate::model_runtime::ProviderKind::ByokCloud),
                    ModelLaneProviderKind::OfficialCli => lane.kind == ModelLaneKind::CliModel
                        && plan.provider == Some(crate::model_runtime::ProviderKind::OfficialCli),
                    _ => false,
                },
                Target::Validator => lane.kind == ModelLaneKind::Validator && plan.provider.is_none() && plan.model_id.is_none(),
                Target::Operator => lane.kind == ModelLaneKind::HumanOperator && plan.provider.is_none() && plan.model_id.is_none(),
                Target::CoordinatorJoin => false,
            };
            if run.run_id != context.run_id || run.trace_id != context.trace_id
                || run.run_span_id != context.run_span_id || run.coordinator_session_id != context.coordinator_session_id
                || run.owner_session != context.owner_session || run.routing_policy != policy.as_str()
                || run.work_packet_id.as_deref() != Some(context.work_packet_id.as_str())
                || run.micro_task_id != context.micro_task_id || run.task_board_id.as_deref() != Some(context.task_board_id.as_str())
                || normalized.locus_binding_ref != context.locus_ref || !target_matches
                || plan.lane_id.as_deref() != Some(lane.lane_id.as_str()) || plan.model_id != lane.model_id {
                return Err(KernelError::InvalidEvent("normalized launch differs from supplied routing context"));
            }
            records.push(json!({"normalized": normalized, "run": run, "lane": lane}));
            plans.push(plan);
        }
        for stage in &graph.stages {
            if stage.target == crate::swarm_orchestration::routing::ModelLaneRoutingDispatchTarget::CoordinatorJoin {
                plans.push(ModelLaneRoutingStageLaunchPlan {
                    stage_id: stage.stage_id.clone(), dispatch_target: stage.target,
                    lane_id: None, model_id: None, provider: None,
                });
            } else if !plans.iter().any(|plan| plan.stage_id == stage.stage_id) {
                return Err(KernelError::InvalidEvent("actual normalized input required for every launch stage"));
            }
        }
        let input = json!({
            "synthetic": true, "network_egress": false,
            "resource_scope": ExactResourceScopeAttribution::try_from_resource_scope(self.scope())
                .map_err(|error| KernelError::ModelRuntime(error.to_string()))?,
            "context": context, "routing_graph": graph, "routing_launch_plan": plans,
            "normalized_records": records,
        });
        let adapter = Ac9VersionedGateAdapter { input: input.clone(), artifact_version: artifact_version.to_owned() };
        let evidence = self.produce_gate_with_adapter(&adapter, input.clone()).await?;
        let intent = unique_event(&evidence.trace, KernelEventType::TaskIntentRecorded)?;
        if evidence.artifact_payload.get("routing") != Some(&input)
            || intent.payload.get("intent") != Some(&input)
            || evidence.artifact_version.as_deref() != Some(artifact_version) {
            return Err(KernelError::InvalidEvent("persisted gate artifact or intent differs from normalized inputs"));
        }
        Ok(evidence)
    }

    async fn produce_gate_with_adapter(
        &self, adapter: &dyn ModelAdapter, intent: serde_json::Value,
    ) -> KernelResult<Ac9GateEvidence> {
        let db = self.db.as_ref().expect("open canonical database").clone();
        let runner = KernelProofRunner::with_artifact_workspace_root(
            db,
            self.artifact_workspace_root.clone(),
        );
        let produced = runner
            .run_first_slice(
                SYNTHETIC_OPERATOR,
                intent,
                adapter,
                OperatorPromotionApproval::new(
                    SYNTHETIC_OPERATOR,
                    "SYNTHETIC isolated no-egress review of matched artifact validation",
                ),
            )
            .await?;
        let gate = unique_event(&produced.trace, KernelEventType::PromotionDecided)?;
        let readback = self
            .resolve_gate_event(
                &produced.kernel_task_run_id,
                &produced.session_run_id,
                &gate.event_id,
            )
            .await?;
        if readback.trace != produced.trace {
            return Err(KernelError::InvalidEvent("canonical gate replay differs from producer"));
        }
        Ok(readback)
    }

    /// Resolves the selected URI as canonical kernel evidence. This does not
    /// establish the additional ModelLane scope/launch or receipt-role binding.
    pub(super) async fn resolve_gate_reference(
        &self,
        kernel_task_run_id: &str,
        session_run_id: &str,
        reference: &str,
    ) -> KernelResult<Ac9GateEvidence> {
        let event_id = reference.strip_prefix("eventledger://kernel/")
            .filter(|event_id| !event_id.is_empty())
            .ok_or(KernelError::InvalidEvent("canonical kernel event reference required"))?;
        self.resolve_gate_event(kernel_task_run_id, session_run_id, event_id).await
    }

    pub(super) async fn resolve_gate_event(
        &self,
        kernel_task_run_id: &str,
        session_run_id: &str,
        gate_event_id: &str,
    ) -> KernelResult<Ac9GateEvidence> {
        let events = self.db.as_ref().expect("open canonical database")
            .list_kernel_events_for_session(session_run_id).await?;
        let trace = TraceProjection::from_events(kernel_task_run_id, session_run_id, events)?;
        let gate = unique_event(&trace, KernelEventType::PromotionDecided)?.clone();
        let artifact = unique_event(&trace, KernelEventType::ArtifactStored)?.clone();
        let validation = unique_event(&trace, KernelEventType::ValidationRecorded)?.clone();
        if gate.event_sequence <= 0 || gate.idempotency_key.trim().is_empty()
            || gate.event_version.trim().is_empty()
            || gate.event_version != artifact.event_version
            || gate.event_version != validation.event_version
            || gate.aggregate_type != "session_run" || gate.aggregate_id != session_run_id
        {
            return Err(KernelError::InvalidEvent("canonical gate version or replay identity differs"));
        }
        if gate.event_id != gate_event_id {
            return Err(KernelError::InvalidEvent("gate reference is not the canonical promotion event"));
        }
        if !matches!(&gate.actor, KernelActor::PromotionGate(_))
            || gate.causation_id.as_deref() != Some(validation.event_id.as_str())
            || string_field(&gate.payload, "decision")? != "APPROVED"
            || string_field(&gate.payload, "operator_id")? != SYNTHETIC_OPERATOR
        {
            return Err(KernelError::InvalidEvent("canonical promotion actor or approval linkage differs"));
        }
        if string_field(&gate.payload, "artifact_id")? != string_field(&artifact.payload, "artifact_id")?
            || string_field(&gate.payload, "validation_id")? != string_field(&validation.payload, "validation_id")?
        {
            return Err(KernelError::InvalidEvent("canonical promotion artifact or validation differs"));
        }
        let review = gate.payload.get("operator_review")
            .ok_or(KernelError::InvalidEvent("canonical operator review missing"))?;
        if string_field(review, "approval_source")? != "operator_review_receipt" {
            return Err(KernelError::InvalidEvent("canonical operator review source differs"));
        }
        let decision_id = string_field(&gate.payload, "promotion_decision_id")?.to_owned();
        let operator_review_receipt_id = string_field(review, "review_receipt_id")?.to_owned();
        if string_field(&artifact.payload, "artifact_layer")? != ArtifactLayer::L2.as_str() {
            return Err(KernelError::InvalidEvent("canonical gate artifact is not the producer's L2 artifact"));
        }
        let artifact_uuid = uuid::Uuid::parse_str(string_field(&artifact.payload, "artifact_uuid")?)
            .map_err(|error| KernelError::Artifact(error.to_string()))?;
        let manifest = read_artifact_manifest(&self.artifact_workspace_root, ArtifactLayer::L2, artifact_uuid)
            .map_err(|error| KernelError::Artifact(error.to_string()))?;
        if manifest.artifact_id != artifact_uuid || manifest.layer != ArtifactLayer::L2 {
            return Err(KernelError::InvalidEvent("canonical artifact manifest identity differs from gate event"));
        }
        let bytes = read_file_artifact_with_manifest(&self.artifact_workspace_root, &manifest)
            .map_err(|error| KernelError::Artifact(error.to_string()))?;
        if hex::encode(Sha256::digest(&bytes)) != string_field(&artifact.payload, "content_hash")? {
            return Err(KernelError::InvalidEvent("canonical artifact payload differs from gate-approved hash"));
        }
        let artifact_payload: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|error| KernelError::Artifact(error.to_string()))?;
        let artifact_version = artifact_payload.get("artifact_version")
            .and_then(serde_json::Value::as_str).filter(|version| !version.trim().is_empty())
            .map(str::to_owned);
        Ok(Ac9GateEvidence {
            trace, gate, artifact, validation, decision_id, operator_review_receipt_id,
            artifact_payload, artifact_version,
        })
    }

    /// Canonical advisory inputs only. The returned request is deliberately
    /// unresolved and must never be reused as an approved execution fixture.
    pub(super) async fn prepare_unresolved_receipt_candidate(
        &mut self,
        evidence: &Ac9GateEvidence,
    ) -> NewModelLanePromotionDecision {
        self.authenticate_workspace().await;
        let label = uuid::Uuid::now_v7().simple().to_string();
        let store = self.store();
        let authenticated = self.authenticated.as_ref().expect("actual authenticated workspace");
        let authorization = AuthorizationRequest {
            session_token: authenticated.session_token.clone(),
            channel_binding_hash: Some(authenticated.channel_binding_hash.clone()),
            capability_id: "fs.write".into(),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: authenticated.workspace_id.clone().expect("actual workspace creation completed"),
            action: ResourceAction::Create,
        };
        let (_, _, _, messages) = store.record_authenticated_creation(
            self.bootstrap(), authorization,
            (ac9_input_run(&label), ac9_input_lane(&label), ac9_input_artifact_binding(&label),
                vec![ac9_input_message(&label, "proposal"), ac9_input_message(&label, "critique")]),
        ).await.expect("genuine authenticated Create boundary records the canonical advisory batch");
        assert_eq!(messages.len(), 2, "actual ordered batch outputs");
        assert_eq!(messages[0].message_id, format!("message-ac9-negative-{label}-proposal"));
        assert_eq!(messages[1].message_id, format!("message-ac9-negative-{label}-critique"));
        let mut candidate = ac9_input_decision(
            &label, &format!("decision-{label}"), &format!("decision-idempotency-{label}"),
            messages[0].event_stream_version,
            evidence,
        );
        candidate.diagnostic_payload = json!({
            "synthetic": true,
            "negative_case": "unresolved_promotion_receipt",
            "kernel_task_run_id": evidence.trace.kernel_task_run_id,
            "kernel_session_run_id": evidence.trace.session_run_id,
            "canonical_gate_event_id": evidence.gate.event_id,
            "canonical_gate_decision_id": evidence.decision_id,
            "canonical_gate_artifact_id": evidence.artifact.payload["artifact_id"],
            "canonical_gate_validation_id": evidence.validation.payload["validation_id"],
        });
        candidate
    }

    async fn logout_authenticated(&mut self) {
        if let Some(authenticated) = self.authenticated.as_ref() {
            let channel = self.native_binding.as_ref().expect("owned channel until vault cleanup").channel.clone();
            let (router, activation) = self.bootstrap_router();
            let logout = router_json(&router, "POST", "/authority/logout", &channel,
                Some(&authenticated.session_token), json!(null)).await;
            assert_eq!(logout.0, StatusCode::OK, "actual logout revokes session and deletes its isolated vault secret");
            drop(router);
            drop(activation);
            self.authenticated.take();
        }
        drop(self.native_binding.take());
    }

    pub(super) async fn reopen(&mut self) {
        self.logout_authenticated().await;
        drop(self.store.take());
        self.scope = None;
        drop(self.bootstrap.take());
        drop(self.db.take());
        self.isolated.reopen().await.expect("close and reopen actual embedded store");
        let db: Arc<dyn Database> = Arc::new(self.isolated.storage().clone());
        self.bootstrap = Some(AccountBootstrapState::new(
            db.clone(), self.isolated.storage().clone(), self.capability_registry.clone(),
        ));
        self.db = Some(db);
    }

    pub(super) async fn cleanup(&mut self) {
        self.logout_authenticated().await;
        drop(self.store.take());
        drop(self.bootstrap.take());
        drop(self.db.take());
        self.isolated.cleanup().await.expect("clean isolated canonical AC9 store");
    }

    pub(super) async fn run_with_cleanup<R>(
        &mut self,
        operation: impl for<'a> FnOnce(&'a mut Self) -> Pin<Box<dyn Future<Output = R> + 'a>>,
    ) -> R {
        let result = std::panic::AssertUnwindSafe(async { operation(self).await }).catch_unwind().await;
        self.cleanup().await;
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}

fn unique_event(trace: &TraceProjection, kind: KernelEventType) -> KernelResult<&KernelEvent> {
    let mut matches = trace.events.iter().filter(|event| event.event_type == kind);
    let event = matches.next().ok_or(KernelError::InvalidEvent("required canonical gate event missing"))?;
    if matches.next().is_some() {
        return Err(KernelError::InvalidEvent("canonical gate event is ambiguous"));
    }
    Ok(event)
}

async fn router_json(
    router: &axum::Router, method: &str, path: &str, channel: &str,
    session: Option<&str>, body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let mut request = Request::builder().method(method).uri(path)
        .header("content-type", "application/json")
        .header(crate::api::stage::HSK_HEADER_CHANNEL_BINDING_TOKEN, channel);
    if let Some(session) = session {
        request = request.header("x-hsk-session-token", session);
    }
    let response = router.clone().oneshot(request.body(Body::from(
        serde_json::to_vec(&body).expect("serialize real bootstrap request"),
    )).expect("construct real bootstrap request")).await.expect("dispatch actual bootstrap router");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 128 * 1024).await.expect("read actual bootstrap response");
    (status, serde_json::from_slice(&bytes).expect("actual bootstrap JSON response"))
}

fn string_field<'a>(payload: &'a serde_json::Value, field: &str) -> KernelResult<&'a str> {
    payload.get(field).and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(KernelError::InvalidEvent("canonical gate field missing"))
}

fn ac9_input_decision(
    label: &str,
    decision_id: &str,
    idempotency_key: &str,
    expected_version: i64,
    evidence: &Ac9GateEvidence,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: format!("run-ac9-negative-{label}"),
        trace_id: format!("trace-ac9-negative-{label}"),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some(format!("span-lane-ac9-negative-{label}")),
        linked_span_contexts: vec![
            format!("span-message-ac9-negative-{label}-proposal"),
            format!("span-message-ac9-negative-{label}-critique"),
        ],
        coordinator_session_id: format!("coordinator-ac9-negative-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane,
        routing_launch_plan: Vec::new(),
        input_refs: vec![
            format!("model-lane-message://message-ac9-negative-{label}-critique"),
            format!("model-lane-message://message-ac9-negative-{label}-proposal"),
        ],
        selected_input_refs: vec![format!(
            "model-lane-message://message-ac9-negative-{label}-proposal"
        )],
        rejected_input_refs: vec![format!(
            "model-lane-message://message-ac9-negative-{label}-critique"
        )],
        validator_authority_ref: None,
        operator_authority_ref: Some(evidence.operator_review_receipt_id.clone()),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: format!("message-ac9-negative-{label}-proposal"),
        expected_event_ledger_version: expected_version,
        base_snapshot_ref: "not-applicable".into(),
        current_base_snapshot_ref: "not-applicable".into(),
        state_vector: "not-applicable".into(),
        current_state_vector: "not-applicable".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_selected_ref_then_lowest_event_seq".into(),
        promotion_gate_ref: evidence.gate.event_id.clone(),
        promotion_receipt_ref: Some(format!("unresolved-ac9-receipt-{label}")),
        promoted_artifact_ref: Some(format!("artifact://ac9-negative/{label}/promoted")),
        promoted_artifact_sha256: Some(promoted_artifact_hash(label)),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: format!("model-lane://ac9-negative/{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-017".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-ac9-negative-{label}"),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("0020-{decision_id}"),
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:02:00Z".into(),
        diagnostic_payload: json!({"synthetic": true, "negative_case": "unresolved_promotion_receipt"}),
    }
}

fn ac9_input_artifact_binding(label: &str) -> NewModelLaneContextBundleArtifactBinding {
    let payload = promoted_artifact_payload(label);
    let hash = promoted_artifact_hash(label);
    let artifact_ref = format!("artifact://ac9-negative/{label}/promoted");
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-ac9-negative-{label}"),
        run_id: format!("run-ac9-negative-{label}"),
        trace_id: format!("trace-ac9-negative-{label}"),
        artifact_ref: artifact_ref.clone(),
        artifact_sha256: hash.clone(),
        content_hash: hash,
        artifact_kind: "model_lane_promoted_artifact".into(),
        artifact_manifest_ref: format!("artifact-manifest://ac9-negative/{label}"),
        artifact_payload_ref: artifact_ref,
        payload_json: payload,
        event_ledger_stream_id: format!("model-lane://ac9-negative/{label}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-017".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: format!("owner-ac9-negative-{label}"),
        idempotency_key: format!("artifact-binding-ac9-negative-{label}"),
        created_at_utc: "2026-09-02T00:01:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn promoted_artifact_payload(label: &str) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_promoted_artifact@1",
        "artifact_version": "1",
        "body": format!("deterministic promoted artifact {label}")
    })
}

fn promoted_artifact_hash(label: &str) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(&promoted_artifact_payload(label)).expect("serialize artifact"),
    ))
}

fn ac9_input_run(label: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: format!("run-ac9-negative-{label}"),
        trace_id: format!("trace-ac9-negative-{label}"),
        run_span_id: format!("span-run-ac9-negative-{label}"),
        coordinator_session_id: format!("coordinator-ac9-negative-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane.as_str().into(),
        context_bundle_id: format!("context-ac9-negative-{label}"),
        lane_ids: vec![format!("lane-ac9-negative-{label}")],
        event_ledger_stream_id: format!("model-lane://ac9-negative/{label}"),
        artifact_namespace: format!("artifact://ac9-negative/{label}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-017".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-ac9-negative-{label}"),
        idempotency_key: format!("run-ac9-negative-{label}"),
        replay_order_key: format!("0001-{label}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        locus_binding: Some(ac9_input_locus(label)),
        memory_pack_ref: format!("memory-pack://ac9-negative/{label}"),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://ac9-negative/{label}"),
        selected_model_id: Some("model://local/ac9-negative".into()),
        candidate_model_ids: vec!["model://local/ac9-negative".into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn ac9_input_lane(label: &str) -> NewModelLane {
    let session = format!("session-ac9-negative-{label}");
    let model_session = format!("model-session-ac9-negative-{label}");
    NewModelLane {
        lane_id: format!("lane-ac9-negative-{label}"),
        run_id: format!("run-ac9-negative-{label}"),
        trace_id: format!("trace-ac9-negative-{label}"),
        lane_span_id: format!("span-lane-ac9-negative-{label}"),
        event_ledger_stream_id: format!("model-lane://ac9-negative/{label}"),
        kind: ModelLaneKind::LocalModel,
        role: "proposal-author".into(),
        backend: "embedded-model-runtime".into(),
        model_id: Some("model://local/ac9-negative".into()),
        session_id: session.clone(),
        model_session_id: model_session.clone(),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://ac9-negative/context".into()],
        effective_capability_snapshot_ref: Some("capability://ac9-negative/snapshot".into()),
        capability_negotiation_ref: Some("capability://ac9-negative/negotiation".into()),
        provider_feature_profile_ref: Some("provider://ac9-negative/local".into()),
        requested_execution_policy_ref: Some("execution://ac9-negative/requested".into()),
        effective_execution_policy_ref: Some("execution://ac9-negative/effective".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["tool-gate://ac9-negative/context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-09-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-09-02T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-09-02T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel://ac9-negative/{label}")),
        reclaim_policy_ref: Some("reclaim://ac9-negative".into()),
        terminal_status_mapping_ref: Some("terminal://ac9-negative".into()),
        process_ownership_ref: Some(format!("process://ac9-negative/{label}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop://ac9-negative".into()),
        last_runtime_status_ref: Some("runtime://ac9-negative/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-017".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-ac9-negative-{label}"),
        locus_binding: Some(ac9_input_locus_for(label, &session, &model_session)),
    }
}

fn ac9_input_message(label: &str, kind: &str) -> NewModelLaneMessage {
    let message_id = format!("message-ac9-negative-{label}-{kind}");
    NewModelLaneMessage {
        message_id: message_id.clone(),
        run_id: format!("run-ac9-negative-{label}"),
        trace_id: format!("trace-ac9-negative-{label}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-lane-ac9-negative-{label}")),
        linked_span_contexts: vec![format!("trace-ac9-negative-{label}")],
        from_lane_id: format!("lane-ac9-negative-{label}"),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-ac9-negative-{label}"),
            correlation_id: format!("correlation-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://ac9-negative/{label}/promoted"),
        payload_sha256: promoted_artifact_hash(label),
        event_ledger_stream_id: format!("model-lane://ac9-negative/{label}"),
        summary: format!("{kind} advisory"),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["tool-gate://ac9-negative/context".into()],
        coordinator_session_id: format!("coordinator-ac9-negative-{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-017".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-ac9-negative-{label}"),
        locus_binding: Some(ac9_input_locus(label)),
        idempotency_key: format!("message-ac9-negative-{label}-{kind}"),
        replay_order_key: format!("0010-{kind}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:01:30Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn ac9_input_locus(label: &str) -> ModelLaneLocusBinding {
    ac9_input_locus_for(
        label,
        &format!("session-ac9-negative-{label}"),
        &format!("model-session-ac9-negative-{label}"),
    )
}

fn ac9_input_locus_for(
    label: &str,
    session_id: &str,
    model_session_id: &str,
) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-017".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-ac9-negative-{label}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: format!("owner-ac9-negative-{label}"),
        locus_binding_ref: format!("locus://wp1/ac9-negative/{label}"),
    }
}

pub(super) fn artifact_workspace(label: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .expect("authorized external artifact root is required"))
        .join("handshake-test").join("WP-KERNEL-012").join("MT-017")
        .join(format!("ac9-{label}-{}", uuid::Uuid::now_v7()))
}

#[tokio::test]
async fn ac9_canonical_gate_evidence_survives_actual_store_reopen() {
    let mut harness = Ac9Harness::create("gate-reopen", artifact_workspace("gate-reopen")).await;
    harness.run_with_cleanup(|harness| Box::pin(async move {
        let evidence = harness.produce_gate_evidence().await.expect("actual gate producer succeeds");
        harness.reopen().await;
        let replay = harness.resolve_gate_event(
            &evidence.trace.kernel_task_run_id, &evidence.trace.session_run_id, &evidence.gate.event_id,
        ).await.expect("resolve actual persisted gate after reopen");
        assert_eq!(replay.trace, evidence.trace);
        assert_eq!(replay.decision_id, evidence.decision_id);
        assert_eq!(replay.operator_review_receipt_id, evidence.operator_review_receipt_id);
        assert_eq!(replay.artifact, evidence.artifact);
        assert_eq!(replay.validation, evidence.validation);
        assert_eq!(replay.artifact_payload, evidence.artifact_payload);
        assert_eq!(replay.artifact_version, evidence.artifact_version);
        let reference = format!("eventledger://kernel/{}", evidence.gate.event_id);
        let resolved_reference = harness.resolve_gate_reference(
            &evidence.trace.kernel_task_run_id, &evidence.trace.session_run_id, &reference,
        ).await.expect("URI resolves to the same canonical kernel evidence");
        assert_eq!(resolved_reference.gate, evidence.gate);
    })).await;
}

#[tokio::test]
async fn ac9_production_promotion_rejects_unresolved_receipt_without_authority_mutation() {
    let mut harness = Ac9Harness::create("unresolved-receipt", artifact_workspace("unresolved-receipt")).await;
    harness.run_with_cleanup(|harness| Box::pin(async move {
        let evidence = harness.produce_gate_evidence().await.expect("produce actual canonical gate evidence");
        let candidate = harness.prepare_unresolved_receipt_candidate(&evidence).await;
        let before = harness.store().replay_promotion_decisions(&candidate.run_id)
            .await.expect("read canonical promotion state before negative");
        let result = harness.store().record_promotion_decision(candidate.clone()).await;
        let after = harness.store().replay_promotion_decisions(&candidate.run_id)
            .await.expect("read canonical promotion state after negative");
        let rejected = result.expect("unexpected writer failure is not unresolved-receipt rejection");
        assert_eq!(rejected.outcome, ModelLanePromotionOutcome::Denied,
            "production must deny an unresolved promotion receipt");
        assert!(after.iter().all(|record| record.outcome != ModelLanePromotionOutcome::Approved),
            "unresolved receipt must not create approved authority");
        assert!(before.is_empty(), "negative starts without promotion authority");
    })).await;
}

#[tokio::test]
async fn ac9_canonical_gate_resolver_rejects_unresolved_and_cross_run_event_ids() {
    let mut harness = Ac9Harness::create("gate-ref", artifact_workspace("gate-ref")).await;
    harness.run_with_cleanup(|harness| Box::pin(async move {
        let first = harness.produce_gate_evidence().await.expect("first actual gate");
        let second = harness.produce_gate_evidence().await.expect("second actual gate");
        for reference in ["unresolved-ac9-event", second.gate.event_id.as_str()] {
            let result = harness.resolve_gate_event(
                &first.trace.kernel_task_run_id, &first.trace.session_run_id, reference,
            ).await;
            assert!(result.is_err(), "helper resolver must not alias another event as gate evidence");
        }
    })).await;
}
