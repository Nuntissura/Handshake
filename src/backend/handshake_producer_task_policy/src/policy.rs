//! Deterministic checks on supplied task-policy data, never authorization.
//!
//! Hosts must verify current grants, sessions, complete canonical chains and
//! tool-decision receipts independently. Hashes and expiry are declarations;
//! this module neither computes a basis codec nor observes clocks or effects.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::task::{RequestedOperationTuple, RequestedTaskSnapshot, ResourceAction, ResourceKind, ToolOperation};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredFactState { Present, Revoked, Expired, Missing }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerOperationMapping {
    pub tool_operation: ToolOperation,
    pub resource_kind: ResourceKind,
    pub action: ResourceAction,
    pub capability: String,
    pub handler_ref: String,
    pub mapping_ref: String,
    pub mapping_revision: i64,
    pub policy_ref: String,
    pub policy_revision: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantLink {
    pub grant_ref: String,
    pub grant_revision: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelegationEdge {
    pub grant_ref: String,
    pub grant_revision: i64,
    #[serde(deserialize_with = "required_parent_grant")]
    pub parent_grant: Option<GrantLink>,
    pub parent_principal_ref: String,
    pub child_principal_ref: String,
    pub resource_kind: ResourceKind,
    pub external_resource_ref: String,
    pub action: ResourceAction,
    pub capability: String,
    pub policy_ref: String,
    pub policy_revision: i64,
    pub state: DeclaredFactState,
    pub full_provenance: Value,
}

// No serde default: the field must occur, even when its value is null.
fn required_parent_grant<'de, D: Deserializer<'de>>(decoder: D) -> Result<Option<GrantLink>, D::Error> {
    Option::<GrantLink>::deserialize(decoder)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceGrantFacts {
    pub source_grant_ref: String,
    pub source_grant_revision: i64,
    pub source_policy_ref: String,
    pub source_policy_revision: i64,
    pub resource_kind: ResourceKind,
    pub external_resource_ref: String,
    pub action: ResourceAction,
    pub capability: String,
    pub source_principal_ref: String,
    pub target_principal_ref: String,
    pub state: DeclaredFactState,
    pub chain: Vec<DelegationEdge>,
    pub full_provenance: Value,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct TaskPolicyFacts {
    pub task: RequestedTaskSnapshot,
    pub handler_ref: String,
    pub mapping_ref: String,
    pub mapping_revision: i64,
    pub policy_ref: String,
    pub policy_revision: i64,
    pub expires_at_utc: String,
    pub full_snapshot_sha256: String,
    pub original_basis_sha256: String,
    pub policy_state: DeclaredFactState,
    pub mappings: Vec<ServerOperationMapping>,
    pub source_grants: Vec<SourceGrantFacts>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct PerSourceDecisionEdge {
    pub operation_index: usize,
    pub source_grant_index: usize,
    pub mapping_index: usize,
}

/// A checked data projection. It carries no current permission or receipt.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ProducerTaskDecisionData {
    facts: TaskPolicyFacts,
    edges: Vec<PerSourceDecisionEdge>,
}

impl ProducerTaskDecisionData {
    pub fn task(&self) -> &RequestedTaskSnapshot { &self.facts.task }
    pub fn facts(&self) -> &TaskPolicyFacts { &self.facts }
    pub fn edges(&self) -> &[PerSourceDecisionEdge] { &self.edges }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataRejectionKind { Unknown, Denied }

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PolicyError {
    #[error("invalid supplied field: {field}")]
    InvalidField { field: &'static str },
    #[error("supplied task binding differs")]
    TaskBindingMismatch,
    #[error("mapping missing for operation {operation_index}")]
    MissingMapping { operation_index: usize },
    #[error("mapping ambiguous for operation {operation_index}")]
    AmbiguousMapping { operation_index: usize },
    #[error("supplied policy is unavailable: {state:?}")]
    PolicyUnavailable { state: DeclaredFactState },
    #[error("source grant missing for operation {operation_index}")]
    MissingSourceGrant { operation_index: usize },
    #[error("source grant {source_grant_index} has no requested pair")]
    ExtraneousSourceGrant { source_grant_index: usize },
    #[error("supplied grant {source_grant_index} edge {edge_index:?} unavailable: {state:?}")]
    GrantUnavailable { source_grant_index: usize, edge_index: Option<usize>, state: DeclaredFactState },
    #[error("grant {source_grant_index} edge {edge_index:?} differs at {field}")]
    GrantBindingMismatch { source_grant_index: usize, edge_index: Option<usize>, field: &'static str },
}

impl PolicyError {
    /// Classifies supplied-data failure, not a live authorization decision.
    pub fn declared_kind(&self) -> DataRejectionKind {
        match self {
            Self::MissingMapping { .. } | Self::MissingSourceGrant { .. }
            | Self::PolicyUnavailable { state: DeclaredFactState::Missing }
            | Self::GrantUnavailable { state: DeclaredFactState::Missing, .. } => DataRejectionKind::Unknown,
            _ => DataRejectionKind::Denied,
        }
    }
}

/// Consumes raw facts and preserves them unchanged in a successful data plan.
/// Real host admission and recovery custody must remain outside these facts.
pub fn evaluate_task_policy(
    task: &RequestedTaskSnapshot,
    facts: TaskPolicyFacts,
) -> Result<ProducerTaskDecisionData, PolicyError> {
    if &facts.task != task { return Err(PolicyError::TaskBindingMismatch); }
    let original = &task.bindings().original_job_scope;
    let producer = task.producer_scope();
    if original.owner_account_id != producer.owner_account_id
        || original.access_space_id != producer.access_space_id
        || original.workspace_id != producer.workspace_id {
        return Err(PolicyError::TaskBindingMismatch);
    }
    for operation in task.operations() {
        if operation.resource_kind != ResourceKind::Workspace
            || operation.action != ResourceAction::Read
            || operation.tool_operation != ToolOperation::ObserveOriginalAdmittedJobCreation
            || operation.capability != "fs.read"
            || operation.external_resource_ref != original.workspace_id.as_str() {
            return Err(PolicyError::TaskBindingMismatch);
        }
    }
    for (field, value) in [
        ("handler_ref", facts.handler_ref.as_str()),
        ("mapping_ref", facts.mapping_ref.as_str()),
        ("policy_ref", facts.policy_ref.as_str()),
        ("expires_at_utc", facts.expires_at_utc.as_str()),
    ] { require_ref(field, value)?; }
    require_sha("full_snapshot_sha256", &facts.full_snapshot_sha256)?;
    require_sha("original_basis_sha256", &facts.original_basis_sha256)?;
    if facts.original_basis_sha256 != task.bindings().original_job_creation_basis.declared_sha256 {
        return Err(PolicyError::TaskBindingMismatch);
    }
    if facts.policy_state != DeclaredFactState::Present {
        return Err(PolicyError::PolicyUnavailable { state: facts.policy_state });
    }
    for mapping in &facts.mappings {
        require_ref("mapping.handler_ref", &mapping.handler_ref)?;
        require_ref("mapping.mapping_ref", &mapping.mapping_ref)?;
        require_ref("mapping.policy_ref", &mapping.policy_ref)?;
        require_ref("mapping.capability", &mapping.capability)?;
    }
    let mut mapping_indices = Vec::with_capacity(task.operations().len());
    for (operation_index, operation) in task.operations().iter().enumerate() {
        let mut matches = facts.mappings.iter().enumerate().filter(|(_, mapping)| {
            mapping.tool_operation == operation.tool_operation
                && mapping.resource_kind == operation.resource_kind
                && mapping.action == operation.action && mapping.capability == operation.capability
                && mapping.handler_ref == facts.handler_ref && mapping.mapping_ref == facts.mapping_ref
                && mapping.mapping_revision == facts.mapping_revision && mapping.policy_ref == facts.policy_ref
                && mapping.policy_revision == facts.policy_revision
        });
        let (mapping_index, _) = matches.next().ok_or(PolicyError::MissingMapping { operation_index })?;
        if matches.next().is_some() { return Err(PolicyError::AmbiguousMapping { operation_index }); }
        mapping_indices.push(mapping_index);
    }
    let producer_principal = producer.actor_principal_id.to_string();
    for (source_grant_index, source) in facts.source_grants.iter().enumerate() {
        if facts.source_grants[..source_grant_index].iter().any(|prior| prior.source_grant_ref == source.source_grant_ref) {
            return Err(PolicyError::InvalidField { field: "source_grants.duplicate_identity" });
        }
        if !task.operations().iter().any(|operation| source_matches(source, operation)) {
            return Err(PolicyError::ExtraneousSourceGrant { source_grant_index });
        }
        validate_source(source, source_grant_index, &producer_principal)?;
    }
    let mut edges = Vec::new();
    for (operation_index, operation) in task.operations().iter().enumerate() {
        let start = edges.len();
        for (source_grant_index, source) in facts.source_grants.iter().enumerate() {
            if source_matches(source, operation) {
                edges.push(PerSourceDecisionEdge {
                    operation_index, source_grant_index, mapping_index: mapping_indices[operation_index],
                });
            }
        }
        if edges.len() == start { return Err(PolicyError::MissingSourceGrant { operation_index }); }
    }
    Ok(ProducerTaskDecisionData { facts, edges })
}

fn source_matches(source: &SourceGrantFacts, operation: &RequestedOperationTuple) -> bool {
    source.resource_kind == operation.resource_kind && source.external_resource_ref == operation.external_resource_ref
        && source.action == operation.action && source.capability == operation.capability
}

fn validate_source(source: &SourceGrantFacts, index: usize, producer: &str) -> Result<(), PolicyError> {
    let mismatch = |edge_index, field| PolicyError::GrantBindingMismatch {
        source_grant_index: index, edge_index, field,
    };
    for (field, value) in [
        ("source_grant_ref", source.source_grant_ref.as_str()),
        ("source_policy_ref", source.source_policy_ref.as_str()),
        ("external_resource_ref", source.external_resource_ref.as_str()),
        ("source_principal_ref", source.source_principal_ref.as_str()),
        ("target_principal_ref", source.target_principal_ref.as_str()),
    ] { if value.trim().is_empty() { return Err(mismatch(None, field)); } }
    if source.state != DeclaredFactState::Present {
        return Err(PolicyError::GrantUnavailable { source_grant_index: index, edge_index: None, state: source.state });
    }
    let last = source.chain.last().ok_or_else(|| mismatch(None, "chain"))?;
    for (edge_index, edge) in source.chain.iter().enumerate() {
        let at = Some(edge_index);
        if source.chain[..edge_index].iter().any(|prior| prior.grant_ref == edge.grant_ref) {
            return Err(mismatch(at, "repeated_grant_ref"));
        }
        if edge.state != DeclaredFactState::Present {
            return Err(PolicyError::GrantUnavailable { source_grant_index: index, edge_index: at, state: edge.state });
        }
        for (field, value) in [
            ("grant_ref", edge.grant_ref.as_str()), ("parent_principal_ref", edge.parent_principal_ref.as_str()),
            ("child_principal_ref", edge.child_principal_ref.as_str()), ("policy_ref", edge.policy_ref.as_str()),
        ] { if value.trim().is_empty() { return Err(mismatch(at, field)); } }
        if edge.resource_kind != source.resource_kind || edge.external_resource_ref != source.external_resource_ref
            || edge.action != source.action || edge.capability != source.capability {
            return Err(mismatch(at, "resource_pair"));
        }
        if edge_index == 0 {
            if edge.parent_grant.is_some() { return Err(mismatch(at, "root_parent")); }
        } else {
            let previous = &source.chain[edge_index - 1];
            match &edge.parent_grant {
                Some(parent) if parent.grant_ref == previous.grant_ref && parent.grant_revision == previous.grant_revision => {}
                _ => return Err(mismatch(at, "parent_grant")),
            }
            if edge.parent_principal_ref != previous.child_principal_ref {
                return Err(mismatch(at, "parent_principal"));
            }
        }
    }
    if last.grant_ref != source.source_grant_ref || last.grant_revision != source.source_grant_revision {
        return Err(mismatch(None, "terminal_grant"));
    }
    if last.parent_principal_ref != source.source_principal_ref || last.child_principal_ref != source.target_principal_ref
        || source.target_principal_ref != producer {
        return Err(mismatch(None, "terminal_principal"));
    }
    if last.policy_ref != source.source_policy_ref || last.policy_revision != source.source_policy_revision {
        return Err(mismatch(None, "terminal_policy"));
    }
    Ok(())
}

fn require_ref(field: &'static str, value: &str) -> Result<(), PolicyError> {
    if value.trim().is_empty() { return Err(PolicyError::InvalidField { field }); }
    Ok(())
}

fn require_sha(field: &'static str, value: &str) -> Result<(), PolicyError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(PolicyError::InvalidField { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{OriginalJobCreationBasis, RequestedTaskBindings};
    use serde_json::json;

    fn task_with(launch: &str, original_workspace: &str, count: usize) -> RequestedTaskSnapshot {
        let scope = json!({
            "owner_account_id":"00000000-0000-0000-0000-000000000001",
            "actor_principal_id":"00000000-0000-0000-0000-000000000002",
            "authenticated_session_id":"00000000-0000-0000-0000-000000000003",
            "access_space_id":"00000000-0000-0000-0000-000000000004","workspace_id":"workspace"
        });
        let mut original_scope = scope.clone();
        original_scope["workspace_id"] = json!(original_workspace);
        original_scope["actor_principal_id"] = json!("00000000-0000-0000-0000-000000000005");
        original_scope["authenticated_session_id"] = json!("00000000-0000-0000-0000-000000000006");
        RequestedTaskSnapshot::new(
            "00000000-0000-0000-0000-000000000007".parse().unwrap(), 1,
            serde_json::from_value(scope).unwrap(),
            RequestedTaskBindings {
                producer_operation_id:"00000000-0000-0000-0000-000000000008".parse().unwrap(),
                original_job_id:"00000000-0000-0000-0000-000000000009".parse().unwrap(),
                original_initial_trace_id:"00000000-0000-0000-0000-000000000010".parse().unwrap(),
                reserved_workflow_run_id:None, producer_launch_ref:launch.into(), selection_ref:"selection".into(),
                run_ref:"run".into(), lane_ref:"lane".into(), trace_ref:"trace".into(), cause_ref:"cause".into(),
                context_ref:"context".into(), original_job_scope:serde_json::from_value(original_scope).unwrap(),
                original_job_creation_basis:OriginalJobCreationBasis {
                    canonical_json:json!({"supplied":"basis"}), canonical_bytes:b"independent supplied bytes".to_vec(),
                    declared_sha256:"a".repeat(64),
                },
            },
            (0..count).map(|_| RequestedOperationTuple {
                resource_kind:ResourceKind::Workspace,external_resource_ref:original_workspace.into(),
                action:ResourceAction::Read,capability:"fs.read".into(),
                tool_operation:ToolOperation::ObserveOriginalAdmittedJobCreation,
            }).collect(),
        ).unwrap()
    }

    fn task() -> RequestedTaskSnapshot { task_with("launch", "workspace", 1) }

    fn facts() -> TaskPolicyFacts {
        let root = DelegationEdge {
            grant_ref:"root".into(),grant_revision:-2,parent_grant:None,parent_principal_ref:"root-issuer".into(),
            child_principal_ref:"delegator".into(),resource_kind:ResourceKind::Workspace,external_resource_ref:"workspace".into(),
            action:ResourceAction::Read,capability:"fs.read".into(),policy_ref:"ancestor-policy".into(),policy_revision:-5,
            state:DeclaredFactState::Present,full_provenance:json!({"policy_versions":[-5,3],"grants":["write","read"]}),
        };
        let terminal = DelegationEdge {
            grant_ref:"selected".into(),grant_revision:4,parent_grant:Some(GrantLink{grant_ref:"root".into(),grant_revision:-2}),
            parent_principal_ref:"delegator".into(),child_principal_ref:"00000000-0000-0000-0000-000000000002".into(),
            resource_kind:ResourceKind::Workspace,external_resource_ref:"workspace".into(),action:ResourceAction::Read,
            capability:"fs.read".into(),policy_ref:"selected-policy".into(),policy_revision:6,state:DeclaredFactState::Present,
            full_provenance:json!({"revocations":[9,-1],"session":{"ref":"original-session"}}),
        };
        TaskPolicyFacts {
            task:task(),handler_ref:"observe-original-job".into(),mapping_ref:"mapping".into(),mapping_revision:2,
            policy_ref:"current-task-policy".into(),policy_revision:8,expires_at_utc:"host-final-expiry-declaration".into(),
            full_snapshot_sha256:"b".repeat(64),original_basis_sha256:"a".repeat(64),policy_state:DeclaredFactState::Present,
            mappings:vec![ServerOperationMapping {
                tool_operation:ToolOperation::ObserveOriginalAdmittedJobCreation,resource_kind:ResourceKind::Workspace,
                action:ResourceAction::Read,capability:"fs.read".into(),handler_ref:"observe-original-job".into(),
                mapping_ref:"mapping".into(),mapping_revision:2,policy_ref:"current-task-policy".into(),policy_revision:8,
            }],
            source_grants:vec![SourceGrantFacts {
                source_grant_ref:"selected".into(),source_grant_revision:4,source_policy_ref:"selected-policy".into(),source_policy_revision:6,
                resource_kind:ResourceKind::Workspace,external_resource_ref:"workspace".into(),action:ResourceAction::Read,
                capability:"fs.read".into(),source_principal_ref:"delegator".into(),
                target_principal_ref:"00000000-0000-0000-0000-000000000002".into(),state:DeclaredFactState::Present,
                chain:vec![root,terminal],full_provenance:json!({"full_chain":["root","selected"],"versions":[-2,4]}),
            }],
        }
    }

    #[test]
    fn accept_one_current_selected_source_single_pair_indices() {
        let plan = evaluate_task_policy(&task(), facts()).unwrap();
        assert_eq!(plan.edges(), &[PerSourceDecisionEdge{operation_index:0,source_grant_index:0,mapping_index:0}]);
        assert_eq!(plan.task(), &task());
    }

    #[test]
    fn preserve_whole_provenance_and_ancestor_policy() {
        let input = facts();
        let expected = serde_json::to_value(&input).unwrap();
        let plan = evaluate_task_policy(&task(), input).unwrap();
        assert_eq!(serde_json::to_value(plan.facts()).unwrap(), expected);
        assert_ne!(plan.facts().source_grants[0].chain[0].policy_ref, plan.facts().source_grants[0].source_policy_ref);
    }

    #[test]
    fn preserve_operation_and_source_order_without_dedup() {
        let expected = task_with("launch","workspace",2);
        let mut input = facts(); input.task = task_with("launch","workspace",2);
        let mut another = input.source_grants[0].clone();
        another.source_grant_ref = "second-selected".into(); another.chain[1].grant_ref = "second-selected".into();
        input.source_grants.push(another);
        let plan = evaluate_task_policy(&expected,input).unwrap();
        assert_eq!(plan.edges().iter().map(|edge|(edge.operation_index,edge.source_grant_index)).collect::<Vec<_>>(),
            vec![(0,0),(0,1),(1,0),(1,1)]);
    }

    #[test]
    fn missing_mapping_unknown() {
        let mut input=facts();input.mappings.clear();
        let error=evaluate_task_policy(&task(),input).unwrap_err();
        assert_eq!(error,PolicyError::MissingMapping{operation_index:0});
        assert_eq!(error.declared_kind(),DataRejectionKind::Unknown);
    }

    #[test]
    fn ambiguous_mapping_denied() {
        let mut input=facts();input.mappings.push(input.mappings[0].clone());
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::AmbiguousMapping{operation_index:0});
    }

    #[test]
    fn missing_policy_unknown_and_revoked_expired_policy_denied() {
        for state in [DeclaredFactState::Missing,DeclaredFactState::Revoked,DeclaredFactState::Expired] {
            let mut input=facts();input.policy_state=state;
            let error=evaluate_task_policy(&task(),input).unwrap_err();
            assert_eq!(error,PolicyError::PolicyUnavailable{state});
            assert_eq!(error.declared_kind(),if state==DeclaredFactState::Missing{DataRejectionKind::Unknown}else{DataRejectionKind::Denied});
        }
    }

    #[test]
    fn changed_full_task_binding_denied() {
        let mut input=facts();input.task=task_with("changed-launch","workspace",1);
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::TaskBindingMismatch);
    }

    #[test]
    fn changed_original_workspace_denied() {
        let expected=task_with("launch","other-workspace",1);
        let mut input=facts();input.task=task_with("launch","other-workspace",1);
        assert_eq!(evaluate_task_policy(&expected,input).unwrap_err(),PolicyError::TaskBindingMismatch);
    }

    #[test]
    fn missing_grant_unknown() {
        let mut input=facts();input.source_grants.clear();
        let error=evaluate_task_policy(&task(),input).unwrap_err();
        assert_eq!(error,PolicyError::MissingSourceGrant{operation_index:0});
        assert_eq!(error.declared_kind(),DataRejectionKind::Unknown);
    }

    #[test]
    fn revoked_source_or_middle_edge_denied() {
        let mut input=facts();input.source_grants[0].state=DeclaredFactState::Revoked;
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantUnavailable{
            source_grant_index:0,edge_index:None,state:DeclaredFactState::Revoked});
        let mut input=facts();
        let source=&mut input.source_grants[0];
        let mut middle=source.chain[0].clone();
        source.chain[0].child_principal_ref="middle-delegator".into();
        middle.grant_ref="middle".into();middle.grant_revision=3;
        middle.parent_grant=Some(GrantLink{grant_ref:"root".into(),grant_revision:-2});
        middle.parent_principal_ref="middle-delegator".into();
        source.chain[1].parent_grant=Some(GrantLink{grant_ref:"middle".into(),grant_revision:3});
        source.chain.insert(1,middle);
        source.chain[1].state=DeclaredFactState::Revoked;
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantUnavailable{
            source_grant_index:0,edge_index:Some(1),state:DeclaredFactState::Revoked});
    }

    #[test]
    fn missing_parent_field_decode_rejected_but_explicit_null_preserved() {
        let mut value=serde_json::to_value(&facts().source_grants[0].chain[0]).unwrap();
        assert!(serde_json::from_value::<DelegationEdge>(value.clone()).unwrap().parent_grant.is_none());
        value.as_object_mut().unwrap().remove("parent_grant");
        assert!(serde_json::from_value::<DelegationEdge>(value).unwrap_err().to_string().contains("parent_grant"));
    }

    #[test]
    fn broken_parent_grant_link_denied() {
        let mut input=facts();input.source_grants[0].chain[1].parent_grant.as_mut().unwrap().grant_revision=99;
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantBindingMismatch{
            source_grant_index:0,edge_index:Some(1),field:"parent_grant"});
    }

    #[test]
    fn reversed_terminal_source_denied() {
        let mut input=facts();input.source_grants[0].source_grant_ref="root".into();
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantBindingMismatch{
            source_grant_index:0,edge_index:None,field:"terminal_grant"});
    }

    #[test]
    fn repeated_grant_identity_with_contradictory_facts_denied() {
        let mut input=facts();
        let source=&mut input.source_grants[0];
        source.chain[0].grant_ref="selected".into();source.chain[0].grant_revision=4;
        source.chain[1].parent_grant=Some(GrantLink{grant_ref:"selected".into(),grant_revision:4});
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantBindingMismatch{
            source_grant_index:0,edge_index:Some(1),field:"repeated_grant_ref"});
    }

    #[test]
    fn broken_principal_chain_denied() {
        let mut input=facts();input.source_grants[0].chain[1].parent_principal_ref="different".into();
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantBindingMismatch{
            source_grant_index:0,edge_index:Some(1),field:"parent_principal"});
    }

    #[test]
    fn terminal_actor_mismatch_denied() {
        let mut input=facts();input.source_grants[0].target_principal_ref="someone-else".into();
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::GrantBindingMismatch{
            source_grant_index:0,edge_index:None,field:"terminal_principal"});
    }

    #[test]
    fn resource_capability_cartesian_union_denied() {
        let mut input=facts();let mut second=input.source_grants[0].clone();
        input.source_grants[0].capability="fs.write".into();
        second.external_resource_ref="other-workspace".into();input.source_grants.push(second);
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::ExtraneousSourceGrant{source_grant_index:0});
    }

    #[test]
    fn extraneous_source_and_duplicate_identity_denied() {
        let mut input=facts();input.source_grants[0].external_resource_ref="unrequested".into();
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::ExtraneousSourceGrant{source_grant_index:0});
        let mut input=facts();input.source_grants.push(input.source_grants[0].clone());
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::InvalidField{field:"source_grants.duplicate_identity"});
    }

    #[test]
    fn declared_hash_is_shape_not_verified_codec_and_expiry_is_not_clock() {
        let input=facts();
        assert_eq!(input.full_snapshot_sha256,"b".repeat(64));
        let plan=evaluate_task_policy(&task(),input).unwrap();
        assert_eq!(plan.facts().expires_at_utc,"host-final-expiry-declaration");
        assert_ne!(plan.task().bindings().original_job_creation_basis.canonical_bytes,
            serde_json::to_vec(&plan.task().bindings().original_job_creation_basis.canonical_json).unwrap());
    }

    #[test]
    fn malformed_or_inconsistent_declared_hash_denied() {
        let mut input=facts();input.full_snapshot_sha256="B".repeat(64);
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::InvalidField{field:"full_snapshot_sha256"});
        let mut input=facts();input.original_basis_sha256="c".repeat(64);
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::TaskBindingMismatch);
    }

    #[test]
    fn mapping_revision_drift_is_unavailable_data() {
        let mut input=facts();input.mappings[0].mapping_revision+=1;
        assert_eq!(evaluate_task_policy(&task(),input).unwrap_err(),PolicyError::MissingMapping{operation_index:0});
    }
}
