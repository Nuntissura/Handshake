//! Immutable requested-task data, not current authorization or a trusted receipt.
//!
//! Revision 1 describes the first snapshot in an original producer-operation/job
//! namespace. It is independent of database revisions and the supplied basis SHA.
//! Future host adoption must bind these declarations to actual canonical facts.

use handshake_resource_scope::ExactResourceScopeAttribution;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAction {
    Read,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOperation {
    ObserveOriginalAdmittedJobCreation,
}

/// One complete requested resource/action/capability/tool tuple, never independent sets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestedOperationTuple {
    pub resource_kind: ResourceKind,
    pub external_resource_ref: String,
    pub action: ResourceAction,
    pub capability: String,
    pub tool_operation: ToolOperation,
}

/// All three representations are supplied data. This leaf verifies no hash or codec relation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginalJobCreationBasis {
    pub canonical_json: Value,
    pub canonical_bytes: Vec<u8>,
    pub declared_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestedTaskBindings {
    pub producer_operation_id: Uuid,
    pub original_job_id: Uuid,
    pub original_initial_trace_id: Uuid,
    pub reserved_workflow_run_id: Option<Uuid>,
    pub producer_launch_ref: String,
    pub selection_ref: String,
    pub run_ref: String,
    pub lane_ref: String,
    pub trace_ref: String,
    pub cause_ref: String,
    pub context_ref: String,
    pub original_job_scope: ExactResourceScopeAttribution,
    pub original_job_creation_basis: OriginalJobCreationBasis,
}

/// Checked requested bounds only. No unchecked decoder, mutation access or cloned snapshot.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct RequestedTaskSnapshot {
    task_id: Uuid,
    task_revision: i64,
    producer_scope: ExactResourceScopeAttribution,
    bindings: RequestedTaskBindings,
    operations: Vec<RequestedOperationTuple>,
}

impl RequestedTaskSnapshot {
    pub fn new(
        task_id: Uuid,
        task_revision: i64,
        producer_scope: ExactResourceScopeAttribution,
        bindings: RequestedTaskBindings,
        operations: Vec<RequestedOperationTuple>,
    ) -> Result<Self, TaskError> {
        if task_revision != 1 {
            return Err(TaskError::InvalidRevision { revision: task_revision });
        }
        for (field, value) in [
            ("producer_workspace", producer_scope.workspace_id.as_str()),
            ("original_job_workspace", bindings.original_job_scope.workspace_id.as_str()),
            ("producer_launch_ref", bindings.producer_launch_ref.as_str()),
            ("selection_ref", bindings.selection_ref.as_str()),
            ("run_ref", bindings.run_ref.as_str()),
            ("lane_ref", bindings.lane_ref.as_str()),
            ("trace_ref", bindings.trace_ref.as_str()),
            ("cause_ref", bindings.cause_ref.as_str()),
            ("context_ref", bindings.context_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(TaskError::InvalidField { field });
            }
        }
        let basis = &bindings.original_job_creation_basis;
        if basis.canonical_bytes.is_empty() {
            return Err(TaskError::InvalidField { field: "canonical_bytes" });
        }
        if basis.declared_sha256.len() != 64
            || !basis.declared_sha256.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(TaskError::InvalidField { field: "declared_sha256" });
        }
        if operations.is_empty() {
            return Err(TaskError::InvalidField { field: "operations" });
        }
        for (index, operation) in operations.iter().enumerate() {
            if operation.external_resource_ref.trim().is_empty() {
                return Err(TaskError::InvalidField { field: "external_resource_ref" });
            }
            if operation.resource_kind != ResourceKind::Workspace
                || operation.action != ResourceAction::Read
                || operation.capability != "fs.read"
                || operation.tool_operation != ToolOperation::ObserveOriginalAdmittedJobCreation
                || operation.external_resource_ref != bindings.original_job_scope.workspace_id.as_str()
            {
                return Err(TaskError::UnsupportedOperation { index });
            }
        }
        Ok(Self { task_id, task_revision, producer_scope, bindings, operations })
    }

    pub fn task_id(&self) -> Uuid { self.task_id }
    pub fn task_revision(&self) -> i64 { self.task_revision }
    pub fn producer_scope(&self) -> &ExactResourceScopeAttribution { &self.producer_scope }
    pub fn bindings(&self) -> &RequestedTaskBindings { &self.bindings }
    pub fn operations(&self) -> &[RequestedOperationTuple] { &self.operations }
}

/// Equality of supplied immutable data, not authorization to observe the original job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactTaskRetry;

pub fn compare_retry(
    original: &RequestedTaskSnapshot,
    candidate: &RequestedTaskSnapshot,
) -> Result<ExactTaskRetry, TaskError> {
    if original.bindings.producer_operation_id != candidate.bindings.producer_operation_id
        || original.bindings.original_job_id != candidate.bindings.original_job_id
    {
        return Err(TaskError::DifferentTaskIdentity);
    }
    if original != candidate {
        return Err(TaskError::ImmutableTaskConflict { task_id: original.task_id });
    }
    Ok(ExactTaskRetry)
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TaskError {
    #[error("invalid supplied task field: {field}")]
    InvalidField { field: &'static str },
    #[error("unsupported requested tuple at index {index}")]
    UnsupportedOperation { index: usize },
    #[error("task snapshot revision must be 1, received {revision}")]
    InvalidRevision { revision: i64 },
    #[error("different original producer-operation/job namespace")]
    DifferentTaskIdentity,
    #[error("immutable task data conflict for {task_id}")]
    ImmutableTaskConflict { task_id: Uuid },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scope() -> ExactResourceScopeAttribution {
        serde_json::from_value(json!({
            "owner_account_id": "00000000-0000-0000-0000-000000000001",
            "actor_principal_id": "00000000-0000-0000-0000-000000000002",
            "authenticated_session_id": "00000000-0000-0000-0000-000000000003",
            "access_space_id": "00000000-0000-0000-0000-000000000004",
            "workspace_id": "workspace-original"
        })).expect("fixed supplied scope data")
    }

    fn bindings() -> RequestedTaskBindings {
        RequestedTaskBindings {
            producer_operation_id: Uuid::from_u128(10),
            original_job_id: Uuid::from_u128(11),
            original_initial_trace_id: Uuid::from_u128(12),
            reserved_workflow_run_id: Some(Uuid::from_u128(13)),
            producer_launch_ref: " launch-original ".into(),
            selection_ref: "selection-original".into(),
            run_ref: "run-original".into(), lane_ref: "lane-original".into(),
            trace_ref: "trace-original".into(), cause_ref: "cause-original".into(),
            context_ref: "context-original".into(), original_job_scope: scope(),
            original_job_creation_basis: OriginalJobCreationBasis {
                canonical_json: json!({"ordered": [3, 1, 3], "untouched": {"extra": true}}),
                canonical_bytes: b"original producer bytes, not a leaf codec".to_vec(),
                declared_sha256: "ab".repeat(32),
            },
        }
    }

    fn operation() -> RequestedOperationTuple {
        RequestedOperationTuple {
            resource_kind: ResourceKind::Workspace,
            external_resource_ref: "workspace-original".into(),
            action: ResourceAction::Read, capability: "fs.read".into(),
            tool_operation: ToolOperation::ObserveOriginalAdmittedJobCreation,
        }
    }

    fn snapshot() -> RequestedTaskSnapshot {
        RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), bindings(), vec![operation(), operation()]).unwrap()
    }

    #[test]
    fn snapshot_retains_ordered_duplicates_full_bindings_and_independent_ids() {
        let task = snapshot();
        assert_eq!(task.task_revision(), 1i64);
        assert_eq!(task.operations(), &[operation(), operation()]);
        assert_eq!(task.operations().len(), 2);
        assert_eq!(task.bindings(), &bindings());
        assert_eq!(task.producer_scope(), &scope());
        assert_eq!(task.bindings().producer_launch_ref, " launch-original ");
        assert_ne!(task.task_id(), task.bindings().producer_operation_id);
        assert_ne!(task.task_id(), task.bindings().original_job_id);
        assert_ne!(task.bindings().original_initial_trace_id, task.bindings().original_job_id);
        assert_eq!(task.bindings().original_job_creation_basis.canonical_json["ordered"], json!([3, 1, 3]));
        assert_eq!(task.bindings().original_job_creation_basis.canonical_bytes, b"original producer bytes, not a leaf codec");
    }

    #[test]
    fn first_revision_is_signed_one_and_basis_sha_is_shape_only() {
        for revision in [-1, 0, 2, i64::MAX] {
            assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), revision, scope(), bindings(), vec![operation()]),
                Err(TaskError::InvalidRevision { revision }));
        }
        for hash in ["AB".repeat(32), "a".repeat(63), "g".repeat(64)] {
            let mut input = bindings(); input.original_job_creation_basis.declared_sha256 = hash;
            assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), input, vec![operation()]),
                Err(TaskError::InvalidField { field: "declared_sha256" }));
        }
        // Deliberately unrelated supplied JSON/bytes/SHA remain data: this is no hash attestation.
        let mut input = bindings();
        input.original_job_creation_basis.canonical_json = Value::Null;
        input.original_job_creation_basis.canonical_bytes = vec![0, 255, 1];
        input.original_job_creation_basis.declared_sha256 = "0".repeat(64);
        let task = RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), input.clone(), vec![operation()]).unwrap();
        assert_eq!(task.bindings(), &input);
    }

    #[test]
    fn constructor_rejects_missing_refs_bytes_operations_and_cross_resource_pairs() {
        let mut input = bindings(); input.selection_ref = " \t\n".into();
        assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), input, vec![operation()]),
            Err(TaskError::InvalidField { field: "selection_ref" }));
        let mut input = bindings(); input.original_job_creation_basis.canonical_bytes.clear();
        assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), input, vec![operation()]),
            Err(TaskError::InvalidField { field: "canonical_bytes" }));
        assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), bindings(), Vec::new()),
            Err(TaskError::InvalidField { field: "operations" }));
        let mut wrong = operation(); wrong.external_resource_ref = "other-workspace".into();
        assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), bindings(), vec![operation(), wrong]),
            Err(TaskError::UnsupportedOperation { index: 1 }));
        let mut wrong = operation(); wrong.capability = "fs.write".into();
        assert_eq!(RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, scope(), bindings(), vec![wrong]),
            Err(TaskError::UnsupportedOperation { index: 0 }));
    }

    #[test]
    fn exact_retry_requires_same_namespace_and_entire_immutable_snapshot() {
        assert_eq!(compare_retry(&snapshot(), &snapshot()), Ok(ExactTaskRetry));
        let mutations: &[fn(&mut RequestedTaskSnapshot)] = &[
            |t| t.task_id = Uuid::from_u128(21),
            |t| t.task_revision = 2,
            |t| t.bindings.original_initial_trace_id = Uuid::from_u128(22),
            |t| t.bindings.reserved_workflow_run_id = None,
            |t| t.bindings.producer_launch_ref.push('x'),
            |t| t.bindings.selection_ref.push('x'),
            |t| t.bindings.run_ref.push('x'),
            |t| t.bindings.lane_ref.push('x'),
            |t| t.bindings.trace_ref.push('x'),
            |t| t.bindings.cause_ref.push('x'),
            |t| t.bindings.context_ref.push('x'),
            |t| t.bindings.original_job_creation_basis.canonical_json = json!({"different": true}),
            |t| t.bindings.original_job_creation_basis.canonical_bytes.push(0),
            |t| t.bindings.original_job_creation_basis.declared_sha256 = "cd".repeat(32),
            |t| { t.operations.pop(); },
        ];
        let original = snapshot();
        for mutate in mutations {
            let mut candidate = snapshot(); mutate(&mut candidate);
            assert_eq!(compare_retry(&original, &candidate), Err(TaskError::ImmutableTaskConflict { task_id: original.task_id() }));
        }
        for change_operation in [false, true] {
            let mut candidate = snapshot();
            if change_operation { candidate.bindings.producer_operation_id = Uuid::from_u128(99); }
            else { candidate.bindings.original_job_id = Uuid::from_u128(99); }
            assert_eq!(compare_retry(&original, &candidate), Err(TaskError::DifferentTaskIdentity));
        }
    }

    #[test]
    fn retry_compares_both_complete_scopes_and_order_without_actor_session_aliasing() {
        let original = snapshot();
        for producer in [false, true] {
            for field in ["owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"] {
                let mut candidate = snapshot();
                let current = if producer { &candidate.producer_scope } else { &candidate.bindings.original_job_scope };
                let mut changed = serde_json::to_value(current).unwrap();
                changed[field] = if field == "workspace_id" { json!("other-workspace") }
                    else { json!("00000000-0000-0000-0000-000000000099") };
                let changed = serde_json::from_value(changed).unwrap();
                if producer { candidate.producer_scope = changed; } else { candidate.bindings.original_job_scope = changed; }
                assert_eq!(compare_retry(&original, &candidate), Err(TaskError::ImmutableTaskConflict { task_id: original.task_id() }));
            }
        }
        // White-box changed bounds test the full retry comparator, not constructor acceptance.
        let mut first = snapshot(); first.operations[0].external_resource_ref = "changed".into();
        let mut second = snapshot(); second.operations[1].external_resource_ref = "changed".into();
        assert_ne!(first.operations, second.operations);
        assert_eq!(compare_retry(&first, &second), Err(TaskError::ImmutableTaskConflict { task_id: first.task_id() }));
        let mut producer = scope();
        producer.actor_principal_id = serde_json::from_value(json!("00000000-0000-0000-0000-000000000088")).unwrap();
        producer.authenticated_session_id = serde_json::from_value(json!("00000000-0000-0000-0000-000000000089")).unwrap();
        let task = RequestedTaskSnapshot::new(Uuid::from_u128(20), 1, producer.clone(), bindings(), vec![operation()]).unwrap();
        assert_eq!(task.producer_scope(), &producer);
        assert_ne!(task.producer_scope().actor_principal_id, task.bindings().original_job_scope.actor_principal_id);
    }

    #[test]
    fn raw_dto_vocabulary_is_closed_and_unknown_fields_are_not_task_decoders() {
        let value = serde_json::to_value(operation()).unwrap();
        assert_eq!(value["tool_operation"], "observe_original_admitted_job_creation");
        let mut unknown = value.clone(); unknown["extra"] = json!(true);
        assert!(serde_json::from_value::<RequestedOperationTuple>(unknown).is_err());
        let mut wrong = value; wrong["action"] = json!("write");
        assert!(serde_json::from_value::<RequestedOperationTuple>(wrong).is_err());
    }
}
