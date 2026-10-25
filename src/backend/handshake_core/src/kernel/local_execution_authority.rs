use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use serde::{Deserialize, Serialize};
use crate::sandbox::{AdapterCapabilities, AttachedNetworkMode, ExecutionPolicyRequest,
    IsolationTier, NetPolicy, RequiredCapability, ResolvedExecutionPolicy, ResourceLimits, TrustClass};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalExecutionPolicyRecord {
    pub policy_id: String,
    pub revision: u64,
    pub owner_account_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub work_profile_id: String,
    pub work_profile_version: u32,
    pub role_id: String,
    pub capability_profile_ref: String,
    pub model_ref: String,
    pub engine_id: String,
    pub artifact_id: String,
    pub artifact_sha256: String,
    pub trust_class: TrustClass,
    pub isolation_tier: IsolationTier,
    pub required_capabilities: BTreeSet<RequiredCapability>,
    pub requested_net_policy: NetPolicy,
    pub effective_attached_network_mode: AttachedNetworkMode,
    pub resource_limits: ResourceLimits,
    pub startup_timeout_ms: u64,
}

#[derive(Clone)]
pub struct LocalExecutionPolicySource {
    path: PathBuf,
    bytes: Arc<[u8]>,
    sha256: String,
    records: Arc<[LocalExecutionPolicyRecord]>,
}

#[derive(Debug)]
pub(crate) struct VerifiedLocalExecutionInvocation {
    instance_id: crate::swarm_orchestration::ids::ModelInstanceId,
    parent_session_id: String,
    owner_role: String,
    work_packet_id: Option<String>,
    micro_task_id: Option<String>,
    routing_attempt: Option<crate::swarm_orchestration::ids::RoutingAttemptIdentity>,
    run_id: Option<String>,
    lane_id: Option<String>,
    scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    canonical: crate::preferences::LocalExecutionAuthoritySnapshot,
    engine: crate::model_runtime::EngineProfileSnapshotV1,
    capability_profile_ref: String,
    source: LocalExecutionPolicySource,
    selected_policy: LocalExecutionPolicyRecord,
    policy: ResolvedExecutionPolicy,
    adapter_capabilities: AdapterCapabilities,
}

impl VerifiedLocalExecutionInvocation {
    pub(crate) fn policy(&self) -> &ResolvedExecutionPolicy { &self.policy }
    pub(crate) fn scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { &self.scope }
    pub(crate) fn engine(&self) -> &crate::model_runtime::EngineProfileSnapshotV1 { &self.engine }
    pub(crate) fn matches_current_canonical(&self, canonical: &crate::preferences::LocalExecutionAuthoritySnapshot,
        engine: &crate::model_runtime::EngineProfileSnapshotV1) -> bool {
        canonical.owner_account_id == self.canonical.owner_account_id
            && canonical.actor_principal_id == self.canonical.actor_principal_id
            && canonical.registry_record == self.canonical.registry_record
            && canonical.selection_record == self.canonical.selection_record
            && canonical.registry_receipt == self.canonical.registry_receipt
            && canonical.selection_receipt == self.canonical.selection_receipt
            && engine == &self.engine
    }    pub(crate) fn validates_execution(&self, owner_session: &str, policy: &ResolvedExecutionPolicy,
        capabilities: &AdapterCapabilities, plan: &handshake_dexterity::LaunchPlan) -> bool {
        self.instance_id.to_string() == owner_session && &self.policy == policy
            && &self.adapter_capabilities == capabilities
            && plan.resolved().profile() == &self.engine.profile
            && plan.resolved().fingerprint() == self.engine.fingerprint
            && plan.resolved().profile().model.content_sha256 == self.selected_policy.artifact_sha256
    }
}
impl LocalExecutionPolicySource {
    pub fn from_captured_configuration(
        path: PathBuf, bytes: Arc<[u8]>, sha256: String,
        records: Vec<LocalExecutionPolicyRecord>,
    ) -> Result<Self, &'static str> {
        use sha2::{Digest, Sha256};
        if !path.is_absolute() || bytes.is_empty()
            || hex::encode(Sha256::digest(bytes.as_ref())) != sha256
        {
            return Err("local execution policy source provenance mismatch");
        }
        let document: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "invalid captured configuration")?;
        let captured: Vec<LocalExecutionPolicyRecord> = serde_json::from_value(document.get("local_execution_policies").cloned().unwrap_or_else(|| serde_json::json!([]))).map_err(|_| "invalid captured local execution policies")?;
        if captured != records { return Err("local execution records differ from captured configuration"); }
        for record in &records { record.validate()?; }
        let mut keys = BTreeSet::new();
        if records.iter().any(|record| !keys.insert((&record.policy_id, record.revision))) {
            return Err("duplicate local execution policy record");
        }
        Ok(Self { path, bytes, sha256, records: records.into() })
    }

    pub fn records(&self) -> &[LocalExecutionPolicyRecord] { &self.records }
    pub fn path(&self) -> &std::path::Path { &self.path }
    pub fn bytes(&self) -> &[u8] { &self.bytes }
    pub fn sha256(&self) -> &str { &self.sha256 }
}

impl LocalExecutionPolicyRecord {
    pub fn validate(&self) -> Result<(), &'static str> {
        if [&self.policy_id, &self.owner_account_id, &self.access_space_id,
            &self.workspace_id, &self.work_profile_id,
            &self.role_id, &self.capability_profile_ref, &self.model_ref,
            &self.engine_id, &self.artifact_id].iter().any(|value| value.trim().is_empty())
            || self.revision == 0 || self.work_profile_version == 0 || self.startup_timeout_ms == 0
            || !self.resource_limits.memory_bytes.is_some_and(|value| value > 0)
            || !self.resource_limits.cpu_cores.is_some_and(|value| value > 0)
            || !self.resource_limits.timeout_ms.is_some_and(|value| value > 0)
            || self.artifact_sha256.len() != 64
            || !self.artifact_sha256.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || self.artifact_sha256.bytes().all(|byte| byte == b'0')
            || self.trust_class != TrustClass::Reviewed
            || self.isolation_tier != IsolationTier::Tier1Container
            || self.requested_net_policy != NetPolicy::DenyAll
            || self.effective_attached_network_mode != AttachedNetworkMode::DenyAll
            || !self.required_capabilities.contains(&RequiredCapability::Win32NativeFidelity)
            || !self.required_capabilities.contains(&RequiredCapability::HighStdioThroughput)
        { return Err("invalid explicit local execution policy"); }
        Ok(())
    }

    pub fn resolve(&self, capabilities: &AdapterCapabilities) -> Result<ResolvedExecutionPolicy, &'static str> {
        self.validate()?;
        ResolvedExecutionPolicy::resolve_local(ExecutionPolicyRequest {
            requested_ref: crate::sandbox::LOCAL_REQUESTED_EXECUTION_POLICY_REF.to_string(),
            trust_class: self.trust_class, isolation_tier: self.isolation_tier,
            required_capabilities: self.required_capabilities.clone(),
            requested_net_policy: self.requested_net_policy.clone(),
            effective_attached_network_mode: self.effective_attached_network_mode,
            resource_limits: self.resource_limits.clone(), startup_timeout_ms: self.startup_timeout_ms,
        }, capabilities).map_err(|_| "local execution policy adapter mismatch")
    }
}
impl LocalExecutionPolicySource {
    pub(crate) fn resolve_invocation(
        &self,
        scope: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        canonical: &crate::preferences::LocalExecutionAuthoritySnapshot,
        engine: &crate::model_runtime::EngineProfileSnapshotV1,
        capabilities: &AdapterCapabilities,
        capability_profile_ref: &str,
        request: &crate::swarm_orchestration::ids::SpawnRequest,
    ) -> Result<VerifiedLocalExecutionInvocation, &'static str> {
        let workspace = scope.workspace_id.to_string();
        let account = scope.owner_account_id.to_string();
        if canonical.owner_account_id != account || canonical.actor_principal_id != scope.actor_principal_id.to_string()
            || canonical.selection.registry_revision != canonical.registry_record.revision
            || canonical.registry.selected_profile_id != canonical.selection.profile_id
        { return Err("local execution canonical identity mismatch"); }
        for (record, receipt, key) in [
            (&canonical.registry_record, &canonical.registry_receipt, "work-profiles.registry"),
            (&canonical.selection_record, &canonical.selection_receipt, "work-profiles.selection"),
        ] {
            if record.preference_id != key || record.scope != "workspace" || record.scope_ref != workspace
                || record.revision <= 0 || record.source != "operator" || record.updated_by.is_empty()
                || record.event_ledger_event_id.is_empty() || receipt.receipt_id.is_empty()
                || receipt.preference_id != key || receipt.scope != "workspace" || receipt.scope_ref != workspace
                || receipt.actor != record.updated_by || receipt.source != "operator" || receipt.after_revision != record.revision
                || receipt.new_value != record.value || receipt.event_ledger_event_id != record.event_ledger_event_id
            { return Err("local execution canonical receipt mismatch"); }
        }
        let selected = &canonical.selection;
        let profiles: Vec<_> = canonical.registry.profiles.iter().filter(|profile|
            profile.profile_id == selected.profile_id && profile.profile_version == selected.profile_version
        ).collect();
        if profiles.len() != 1 { return Err("local execution selected profile missing or ambiguous"); }
        let routes: Vec<_> = profiles[0].role_routes.iter().filter(|route| route.role_id == selected.role_id).collect();
        if routes.len() != 1 { return Err("local execution selected role missing or ambiguous"); }
        let route = routes[0];
        if route.provider_ref != "local_runtime" || route.model_ref != engine.profile.id || route.capability_profile_ref != capability_profile_ref { return Err("local execution selected model route mismatch"); }
        let engine_id = serde_json::to_value(engine.profile.engine).map_err(|_| "invalid engine identity")?;
        let engine_id = engine_id.as_str().ok_or("invalid engine identity")?;
        let records: Vec<_> = self.records.iter().filter(|record|
            record.owner_account_id == account && record.access_space_id == scope.access_space_id.to_string()
            && record.workspace_id == workspace && record.work_profile_id == selected.profile_id
            && record.work_profile_version == selected.profile_version && record.role_id == selected.role_id
            && record.capability_profile_ref == route.capability_profile_ref && record.model_ref == route.model_ref
            && record.engine_id == engine_id && record.artifact_id == engine.profile.model.id
            && record.artifact_sha256 == engine.profile.model.content_sha256
        ).collect();
        if records.len() != 1 { return Err("explicit local execution policy missing or ambiguous"); }
        if request.runtime_binding != crate::model_runtime::registry::RuntimeBinding::GenieX
            || request.parent_session_id.trim().is_empty() || request.owner_role.trim().is_empty() {
            return Err("invalid local execution invocation identity");
        }
        let policy = records[0].resolve(capabilities)?;
        Ok(VerifiedLocalExecutionInvocation {
            instance_id: request.instance_id, parent_session_id: request.parent_session_id.clone(),
            owner_role: request.owner_role.clone(), work_packet_id: request.wp_id.clone(), micro_task_id: request.mt_id.clone(),
            routing_attempt: request.routing_attempt.clone(),
            run_id: request.dexterity_launch.as_ref().map(|launch| launch.run_id.to_string()),
            lane_id: request.dexterity_launch.as_ref().map(|launch| launch.lane_id.to_string()),
            scope: scope.clone(), canonical: canonical.clone(), engine: engine.clone(),
            capability_profile_ref: capability_profile_ref.to_owned(), source: self.clone(),
            selected_policy: (*records[0]).clone(), policy, adapter_capabilities: capabilities.clone(),
        })
    }
}impl std::fmt::Debug for LocalExecutionPolicySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalExecutionPolicySource").field("path", &self.path)
            .field("sha256", &self.sha256).field("record_count", &self.records.len()).finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::{LocalExecutionAuthoritySnapshot, PreferenceChangeReceipt, PreferenceRecord};
    use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;
    use serde_json::json;

    fn invocation_request() -> crate::swarm_orchestration::ids::SpawnRequest {
        crate::swarm_orchestration::ids::SpawnRequest::new(
            crate::swarm_orchestration::ids::ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(),1),
            crate::model_runtime::registry::RuntimeBinding::GenieX,"fixture-owner","fixture-parent",
        )
    }
    fn source(records: Vec<LocalExecutionPolicyRecord>) -> LocalExecutionPolicySource {
        use sha2::{Digest, Sha256};
        let bytes: Arc<[u8]> = serde_json::to_vec(&json!({"local_execution_policies":records})).unwrap().into();
        LocalExecutionPolicySource::from_captured_configuration(std::env::current_dir().unwrap().join("fixture.json"),
            bytes.clone(), hex::encode(Sha256::digest(bytes.as_ref())), records).unwrap()
    }

    fn fixture() -> (LocalExecutionPolicySource, ExactResourceScopeAttribution, LocalExecutionAuthoritySnapshot,
        crate::model_runtime::EngineProfileSnapshotV1, AdapterCapabilities) {
        use handshake_dexterity::*;
        let scope: ExactResourceScopeAttribution = serde_json::from_value(json!({
            "owner_account_id":uuid::Uuid::from_u128(1), "actor_principal_id":uuid::Uuid::from_u128(2),
            "authenticated_session_id":uuid::Uuid::from_u128(3), "access_space_id":uuid::Uuid::from_u128(4),
            "workspace_id":"fixture-workspace"
        })).unwrap();
        let engine = crate::model_runtime::EngineProfileSnapshotV1::new(EngineProfile {
            id:"fixture-engine-profile".into(), revision:1,
            model:Model { id:"fixture-artifact".into(), format:ModelFormat::GgufQ4_0,
                content_sha256:"a".repeat(64), compiled_context:None },
            engine:Engine::GenieXLlamaCpp, device:Device::Cpu,
            preset:Preset { name:"fixture".into(), context_limit:2048, max_tokens:64,
                sampling:Sampling::LlamaCpp { temperature:0.5, top_k:20, top_p:0.95, min_p:0.0 } },
        }).unwrap();
        let registry = json!({"schema_id":"fixture", "registry_id":"fixture", "folded_stub_ids":[],
            "profile_storage_ref":"fixture", "selected_profile_id":"profile", "profiles":[{
                "profile_id":"profile", "profile_version":1, "profile_id_is_immutable":true,
                "display_name":"fixture", "role_routes":[{"role_id":"writer", "model_ref":engine.profile.id,
                    "provider_ref":"local_runtime", "capability_profile_ref":"caps"}],
                "autonomy":{"max_auto_actions":1,"requires_operator_approval_for_promotion":true,
                    "allow_parallel_agents":false,"allow_network":false}, "created_at_utc":"fixture"}],
            "profile_receipts":[],"action_requests":[],"product_authority_refs":[],"folded_source_refs":[]});
        let selection = json!({"profile_id":"profile","profile_version":1,"role_id":"writer","registry_revision":1});
        let record = |key: &str, value: serde_json::Value| PreferenceRecord {
            schema_id:"fixture".into(),preference_id:key.into(),namespace:"work-profiles".into(),value_type:"object".into(),
            value, scope:"workspace".into(),scope_ref:scope.workspace_id.to_string(),default_value:json!({}),
            source:"operator".into(),revision:1,redaction_class:"public".into(),updated_by:"historical-actor".into(),
            event_ledger_event_id:format!("fixture-event-{key}"),
        };
        let receipt = |record: &PreferenceRecord| PreferenceChangeReceipt {
            schema_id:"fixture".into(),receipt_id:format!("receipt-{}",record.preference_id),
            preference_id:record.preference_id.clone(),scope:record.scope.clone(),scope_ref:record.scope_ref.clone(),
            before_revision:None,after_revision:1,old_value:None,new_value:record.value.clone(),source:"operator".into(),
            actor:record.updated_by.clone(),event_ledger_event_id:record.event_ledger_event_id.clone(),created_at:"fixture".into(),
        };
        let registry_record = record("work-profiles.registry",registry.clone());
        let selection_record = record("work-profiles.selection",selection.clone());
        let canonical = LocalExecutionAuthoritySnapshot {
            owner_account_id:scope.owner_account_id.to_string(),actor_principal_id:scope.actor_principal_id.to_string(),
            actor_id:"current-actor".into(),registry_receipt:receipt(&registry_record),selection_receipt:receipt(&selection_record),
            registry_record,selection_record,registry:serde_json::from_value(registry).unwrap(),
            selection:serde_json::from_value(selection).unwrap(),
        };
        let record = LocalExecutionPolicyRecord {
            policy_id:"fixture-policy".into(),revision:1,owner_account_id:scope.owner_account_id.to_string(),
            access_space_id:scope.access_space_id.to_string(),workspace_id:scope.workspace_id.to_string(),
            work_profile_id:"profile".into(),work_profile_version:1,role_id:"writer".into(),
            capability_profile_ref:"caps".into(),model_ref:engine.profile.id.clone(),
            engine_id:serde_json::to_value(engine.profile.engine).unwrap().as_str().unwrap().into(),
            artifact_id:engine.profile.model.id.clone(),artifact_sha256:engine.profile.model.content_sha256.clone(),
            trust_class:TrustClass::Reviewed,isolation_tier:IsolationTier::Tier1Container,
            required_capabilities:BTreeSet::from([RequiredCapability::Win32NativeFidelity,RequiredCapability::HighStdioThroughput]),
            requested_net_policy:NetPolicy::DenyAll,effective_attached_network_mode:AttachedNetworkMode::DenyAll,
            resource_limits:ResourceLimits { memory_bytes:Some(1024),cpu_cores:Some(1),timeout_ms:Some(1000),..Default::default() },
            startup_timeout_ms:100,
        };
        let mut caps = AdapterCapabilities::default_no_op_capabilities();
        caps.adapter_id = crate::sandbox::AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        caps.runtime_available = true; caps.win32_native_fidelity = true;
        caps.stdio_throughput_class = crate::sandbox::ThroughputClass::High;
        (source(vec![record]),scope,canonical,engine,caps)
    }

    #[test]
    fn captured_configuration_cannot_attach_unrelated_records() {
        use sha2::{Digest,Sha256};
        let (source,_,_,_,_) = fixture();
        let bytes: Arc<[u8]> = b"{}".to_vec().into();
        let path = std::env::current_dir().unwrap().join("fixture.json");
        assert!(LocalExecutionPolicySource::from_captured_configuration(path.clone(),bytes.clone(),
            hex::encode(Sha256::digest(bytes.as_ref())),source.records().to_vec()).is_err());
        let empty = LocalExecutionPolicySource::from_captured_configuration(path,bytes.clone(),
            hex::encode(Sha256::digest(bytes.as_ref())),vec![]).unwrap();
        assert!(empty.records().is_empty());
    }

    #[test]
    fn invocation_requires_exact_selected_route_capabilities_and_scope() {
        let (source,scope,mut canonical,engine,caps) = fixture();
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_ok());
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"foreign-caps",&invocation_request()).is_err());
        canonical.registry.profiles[0].role_routes[0].provider_ref = "cloud".into();
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
        canonical.registry.profiles[0].role_routes[0].provider_ref = "local_runtime".into();
        canonical.registry.profiles[0].role_routes[0].model_ref = "foreign-model".into();
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
        let (_,_,mut canonical,_,_) = fixture();
        canonical.actor_principal_id = uuid::Uuid::from_u128(9).to_string();
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
    }

    #[test]
    fn invocation_rejects_default_missing_record_and_unavailable_adapter() {
        let (source,scope,mut canonical,engine,mut caps) = fixture();
        canonical.selection_record.source = "default".into();
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
        let (_,_,canonical,_,_) = fixture();
        assert!(super::tests::source(vec![]).resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
        caps.win32_native_fidelity = false;
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
    }

    #[test]
    fn invocation_never_promotes_host_inherited_network() {
        let (source,scope,canonical,engine,caps) = fixture();
        assert_eq!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).unwrap()
            .policy().effective_attached_network_mode,AttachedNetworkMode::DenyAll);
        for (requested,effective) in [(NetPolicy::HostInherited,AttachedNetworkMode::OutboundInternetClient),
            (NetPolicy::DenyAll,AttachedNetworkMode::OutboundInternetClient)] {
            let mut records = source.records().to_vec();
            records[0].requested_net_policy = requested; records[0].effective_attached_network_mode = effective;
            // A malformed source can never pass construction; resolution independently rejects it too.
            let invalid = LocalExecutionPolicySource { records:records.into(),..source.clone() };
            assert!(invalid.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).is_err());
        }
    }
    #[test]
    fn verified_invocation_pins_source_receipts_and_rejects_wrong_effect_owner() {
        let (source,scope,canonical,engine,caps) = fixture();
        let mut request = invocation_request();
        let verified = source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&request).unwrap();
        assert_eq!(verified.source.sha256(),source.sha256());
        assert_eq!(verified.source.bytes(),source.bytes());
        assert_eq!(verified.selected_policy.policy_id,"fixture-policy");
        assert_eq!(verified.selected_policy.revision,1);
        assert_eq!(verified.canonical.selection_receipt.receipt_id,canonical.selection_receipt.receipt_id);
        assert_eq!(verified.canonical.registry_record.revision,canonical.registry_record.revision);
        let root = std::env::current_dir().unwrap();
        let binding = handshake_dexterity::HostBinding {
            executable:root.join("fixture.exe"),cwd:root.clone(),data:root.join("fixture-data"),
            model_selector:"local/fixture-model:Q4_0".into(),content_sha256:engine.profile.model.content_sha256.clone(),
            capabilities:vec![handshake_dexterity::RuntimeCapability {engine:engine.profile.engine,device:engine.profile.device}],
            environment:Default::default(),
        };
        let resolved = handshake_dexterity::Registry::new(vec![engine.profile.clone()]).unwrap()
            .resolve(&engine.profile.id).unwrap();
        let plan = resolved.plan(binding,"fixture").unwrap();
        assert!(verified.validates_execution(&request.instance_id.to_string(),verified.policy(),&caps,&plan));
        assert!(!verified.validates_execution("foreign-instance",verified.policy(),&caps,&plan));
        let mut wrong_caps = caps.clone(); wrong_caps.win32_native_fidelity = false;
        assert!(!verified.validates_execution(&request.instance_id.to_string(),verified.policy(),&wrong_caps,&plan));
        let mut wrong_policy = verified.policy().clone(); wrong_policy.policy_revision += 1;
        assert!(!verified.validates_execution(&request.instance_id.to_string(),&wrong_policy,&caps,&plan));
        request.owner_role.clear();
        assert!(source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&request).is_err());
    }
    #[test]
    fn later_invocations_cannot_rebase_a_pinned_selection_or_engine_profile() {
        let (source,scope,mut canonical,mut engine,caps) = fixture();
        let verified = source.resolve_invocation(&scope,&canonical,&engine,&caps,"caps",&invocation_request()).unwrap();
        assert!(verified.matches_current_canonical(&canonical,&engine));
        canonical.selection_record.revision += 1;
        assert!(!verified.matches_current_canonical(&canonical,&engine));
        canonical.selection_record.revision -= 1;
        canonical.registry_receipt.event_ledger_event_id="foreign-event".into();
        assert!(!verified.matches_current_canonical(&canonical,&engine));
        canonical.registry_receipt.event_ledger_event_id=verified.canonical.registry_receipt.event_ledger_event_id.clone();
        engine.profile.revision += 1;
        assert!(!verified.matches_current_canonical(&canonical,&engine));
    }
}