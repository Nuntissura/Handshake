//! Deterministic declaration integrity; no invocation, authority, or settlement is attested.

use serde_json::Value;

use crate::domain::{
    DeclaredReviewTransition, PurePreparedReview, ReviewError, ReviewState, ReviewTask,
    UnsettledPhase, ValidatorArtifactReviewRequest,
};
use crate::report::{
    canonical_hash, canonical_request_bytes, canonical_task_bytes, preflight_value, task_hash,
    validate_request, validate_task,
};

const CONTEXT_LIMIT: usize = 8 * 1024 * 1024;
const TASK_LIMIT: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PureCheckedTransition {
    transition: DeclaredReviewTransition,
}

impl PureCheckedTransition {
    pub fn transition(&self) -> &DeclaredReviewTransition {
        &self.transition
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactExisting;

pub fn prepare_review(
    request: &ValidatorArtifactReviewRequest,
    canonical_context_bundle: &Value,
) -> Result<PurePreparedReview, ReviewError> {
    validate_request(request)?;
    preflight_value(canonical_context_bundle, "context_bundle", CONTEXT_LIMIT)?;
    let context_bundle_hash = canonical_hash(canonical_context_bundle)?;
    if context_bundle_hash != request.context_bundle_sha256 {
        return Err(ReviewError::ContextHashMismatch);
    }

    let bound_value = canonical_context_bundle
        .pointer("/allowed_context/validator_artifact_review_task")
        .ok_or(ReviewError::TaskMismatch)?;
    preflight_value(bound_value, "task", TASK_LIMIT)?;
    let bound_task: ReviewTask =
        serde_json::from_value(bound_value.clone()).map_err(|_| ReviewError::TaskMismatch)?;
    validate_task(&bound_task)?;
    if bound_task != request.task
        || canonical_task_bytes(&bound_task)? != canonical_task_bytes(&request.task)?
    {
        return Err(ReviewError::TaskMismatch);
    }

    let request_value = serde_json::to_value(request).map_err(|e| ReviewError::Json(e.to_string()))?;
    PurePreparedReview::new_checked(
        request.clone(),
        canonical_hash(&request_value)?,
        task_hash(&request.task)?,
        context_bundle_hash,
    )
}

pub fn compare_retry(
    existing: &ValidatorArtifactReviewRequest,
    incoming: &ValidatorArtifactReviewRequest,
) -> Result<ExactExisting, ReviewError> {
    validate_request(existing)?;
    validate_request(incoming)?;
    if canonical_request_bytes(existing)? == canonical_request_bytes(incoming)? {
        Ok(ExactExisting)
    } else {
        Err(ReviewError::OperationConflict {
            operation_id: incoming.operation_id,
        })
    }
}

pub fn check_transition(
    transition: &DeclaredReviewTransition,
) -> Result<PureCheckedTransition, ReviewError> {
    use ReviewState::*;
    let valid = matches!(
        (&transition.from, &transition.to),
        (Prepared, Invoking | Cancelled | Failed)
            | (Invoking, OutputReady | Cancelled | Failed | Unsettled(UnsettledPhase::Invocation))
            | (OutputReady, PublicationPending | Cancelled | Failed)
            | (PublicationPending, Published | Cancelled | Failed | Unsettled(UnsettledPhase::Publication))
            | (Unsettled(UnsettledPhase::Invocation), OutputReady | Cancelled | Failed)
            | (Unsettled(UnsettledPhase::Publication), Published | Cancelled | Failed)
    );
    if !valid {
        return Err(ReviewError::InvalidTransition {
            from: transition.from.clone(),
            to: transition.to.clone(),
        });
    }
    Ok(PureCheckedTransition { transition: transition.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_role_authority::domain::ContentSha256;
    use serde_json::json;

    const BUNDLE: &str = r#"{"allowed_context":{"validator_artifact_review_task":{"criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1","task_id":"fixture-review-task"}},"bundle_id":"fixture-bundle","context_hash":"6bbcc95b20b7faca4475d7557d8c08f31cbd41080105ae24faa19e43d58c0304","schema_id":"fixture-context-bundle@1"}"#;
    const REQUEST: &str = r#"{"context_bundle_ref":"fixture-context-bundle","context_bundle_sha256":"789e6778c1fbb2b112cf2b2a08c4e21ef3432c7546747686e36f5271fb619598","designation_ref":"fixture-designation","expected_action":{"action_id":"fixture-action","idempotency_key":"fixture-idempotency","request_id":"fixture-request"},"expected_artifact_version":"1","expected_subject_sha256":"0000000000000000000000000000000000000000000000000000000000000000","operation_id":"00000000-0000-0000-0000-000000000010","registration_ref":"fixture-registration","subject_ref":"fixture-subject","task":{"criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1","task_id":"fixture-review-task"}}"#;

    fn fixture() -> (ValidatorArtifactReviewRequest, Value) {
        (serde_json::from_str(REQUEST).unwrap(), serde_json::from_str(BUNDLE).unwrap())
    }

    fn bind_hash(request: &mut ValidatorArtifactReviewRequest, bundle: &Value) {
        request.context_bundle_sha256 = canonical_hash(bundle).unwrap();
    }

    #[test]
    fn preparation_binds_fixed_whole_bundle_task_and_request_hashes() {
        let (request, bundle) = fixture();
        let original = bundle.clone();
        let prepared = prepare_review(&request, &bundle).unwrap();
        assert_eq!(prepared.request(), &request);
        assert_eq!(prepared.request_hash().to_hex(), "53a870eecd8b9c0532ad98cfa071208994367fc5b0e03fd1dc92638d00eae1ad");
        assert_eq!(prepared.task_hash().to_hex(), "8a1023546d725f65d1175baaf428bd67806b0f4499ec2fdf642cab37aa8b208b");
        assert_eq!(prepared.context_bundle_hash().to_hex(), "789e6778c1fbb2b112cf2b2a08c4e21ef3432c7546747686e36f5271fb619598");
        assert_eq!(bundle, original);
        assert_ne!(prepared.context_bundle_hash().to_hex(), bundle["context_hash"].as_str().unwrap());
        assert_eq!(prepared.request().expected_action.action_id, "fixture-action");
    }

    #[test]
    fn allowed_context_digest_and_changed_outer_metadata_are_not_full_bundle_matches() {
        let (mut request, mut bundle) = fixture();
        request.context_bundle_sha256 = ContentSha256::from_hex(bundle["context_hash"].as_str().unwrap()).unwrap();
        assert_eq!(prepare_review(&request, &bundle), Err(ReviewError::ContextHashMismatch));
        let (request, _) = fixture();
        bundle["bundle_id"] = json!("different-bundle");
        assert_eq!(prepare_review(&request, &bundle), Err(ReviewError::ContextHashMismatch));
    }

    #[test]
    fn complete_bound_task_must_match_even_with_a_matching_bundle_hash() {
        let (request, bundle) = fixture();
        for replacement in [
            json!({"task_id":"other","criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1"}),
            json!({"task_id":"fixture-review-task","criteria":["Different criterion"],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1"}),
            json!({"task_id":"fixture-review-task","criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1","extra":true}),
            Value::Null,
        ] {
            let mut changed = bundle.clone();
            changed["allowed_context"]["validator_artifact_review_task"] = replacement;
            let mut incoming = request.clone();
            bind_hash(&mut incoming, &changed);
            assert_eq!(prepare_review(&incoming, &changed), Err(ReviewError::TaskMismatch));
        }
        let mut missing = bundle;
        missing["allowed_context"].as_object_mut().unwrap().remove("validator_artifact_review_task");
        let mut incoming = request;
        bind_hash(&mut incoming, &missing);
        assert_eq!(prepare_review(&incoming, &missing), Err(ReviewError::TaskMismatch));
    }

    #[test]
    fn context_bounds_and_controls_are_checked_before_hash_comparison() {
        let (request, mut bundle) = fixture();
        bundle["extra"] = json!("\u{0001}");
        assert!(matches!(prepare_review(&request, &bundle), Err(ReviewError::InvalidField { .. })));
        bundle["extra"] = json!("x".repeat(CONTEXT_LIMIT));
        assert_eq!(prepare_review(&request, &bundle), Err(ReviewError::SizeLimit { field: "context_bundle", limit: CONTEXT_LIMIT }));
        let mut deep = Value::Null;
        for _ in 0..70 { deep = json!([deep]); }
        assert_eq!(prepare_review(&request, &deep), Err(ReviewError::DepthLimit { limit: 64 }));
    }

    #[test]
    fn retry_compares_every_request_field_including_independent_action_keys() {
        let (request, _) = fixture();
        assert_eq!(compare_retry(&request, &request), Ok(ExactExisting));
        let base = serde_json::to_value(&request).unwrap();
        let changes = [
            ("/operation_id", json!("00000000-0000-0000-0000-000000000011")),
            ("/expected_action/action_id", json!("different-action")),
            ("/expected_action/request_id", json!("different-request")),
            ("/expected_action/idempotency_key", json!("different-idempotency")),
            ("/designation_ref", json!("different-designation")),
            ("/registration_ref", json!("different-registration")),
            ("/subject_ref", json!("different-subject")),
            ("/expected_subject_sha256", json!("1111111111111111111111111111111111111111111111111111111111111111")),
            ("/expected_artifact_version", json!("2")),
            ("/context_bundle_ref", json!("different-bundle")),
            ("/context_bundle_sha256", json!("1111111111111111111111111111111111111111111111111111111111111111")),
            ("/task/task_id", json!("different-task")),
            ("/task/criteria", json!(["Different criterion"])),
        ];
        for (pointer, replacement) in changes {
            let mut changed = base.clone();
            *changed.pointer_mut(pointer).unwrap() = replacement;
            let incoming: ValidatorArtifactReviewRequest = serde_json::from_value(changed).unwrap();
            assert_eq!(compare_retry(&request, &incoming), Err(ReviewError::OperationConflict { operation_id: incoming.operation_id }), "{pointer}");
        }
        let mut invalid = request.clone();
        invalid.expected_action.action_id.clear();
        assert!(matches!(compare_retry(&request, &invalid), Err(ReviewError::InvalidField { .. })));
        assert!(matches!(compare_retry(&invalid, &request), Err(ReviewError::InvalidField { .. })));
    }

    #[test]
    fn transition_matrix_rejects_terminal_self_and_reinvocation_edges() {
        use ReviewState::*;
        let states = [Prepared, Invoking, OutputReady, PublicationPending, Unsettled(UnsettledPhase::Invocation), Unsettled(UnsettledPhase::Publication), Published, Cancelled, Failed];
        let allowed = [(0,1),(0,7),(0,8),(1,2),(1,7),(1,8),(1,4),(2,3),(2,7),(2,8),(3,6),(3,7),(3,8),(3,5),(4,2),(4,7),(4,8),(5,6),(5,7),(5,8)];
        for (from, from_state) in states.iter().enumerate() {
            for (to, to_state) in states.iter().enumerate() {
                let transition = DeclaredReviewTransition { from: from_state.clone(), to: to_state.clone() };
                if allowed.contains(&(from,to)) {
                    assert_eq!(check_transition(&transition).unwrap().transition(), &transition);
                } else {
                    assert_eq!(check_transition(&transition), Err(ReviewError::InvalidTransition { from: from_state.clone(), to: to_state.clone() }));
                }
            }
        }
    }
}
