//! Review declarations and integrity-only results; none attest invocation or authority.

use handshake_role_authority::domain::{ActionKey, ContentSha256};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatorArtifactReviewRequest {
    pub operation_id: Uuid,
    #[serde(deserialize_with = "strict_action")]
    pub expected_action: ActionKey,
    pub designation_ref: String,
    pub registration_ref: String,
    pub subject_ref: String,
    pub expected_subject_sha256: ContentSha256,
    pub expected_artifact_version: String,
    pub context_bundle_ref: String,
    pub context_bundle_sha256: ContentSha256,
    pub task: ReviewTask,
}

fn strict_action<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ActionKey, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct StrictAction { action_id: String, request_id: String, idempotency_key: String }
    let wire = StrictAction::deserialize(deserializer)?;
    Ok(ActionKey { action_id: wire.action_id, request_id: wire.request_id, idempotency_key: wire.idempotency_key })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewTask {
    pub schema_id: ReviewTaskSchema,
    pub task_id: String,
    pub kind: ReviewTaskKind,
    pub criteria: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewTaskSchema {
    #[serde(rename = "hsk.validator_routing_proposal_review_task@1")]
    RoutingProposalReviewV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTaskKind { ReviewRoutingProposal }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewOutputBinding {
    pub subject_ref: String,
    pub subject_sha256: ContentSha256,
    pub artifact_version: String,
    pub task_hash: ContentSha256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdvisoryReport {
    pub schema_id: AdvisoryReportSchema,
    pub subject_ref: String,
    pub subject_sha256: ContentSha256,
    pub artifact_version: String,
    pub task_hash: ContentSha256,
    pub recommendation: AdvisoryRecommendation,
    pub summary: String,
    pub findings: Vec<AdvisoryFinding>,
    pub unable_reason: Option<UnableToReviewReason>,
}
impl<'de> Deserialize<'de> for AdvisoryReport {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::report::deserialize_report(deserializer, &mut None)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvisoryReportSchema {
    #[serde(rename = "hsk.validator_routing_proposal_review_report@1")]
    RoutingProposalReviewV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryRecommendation { RecommendAccept, RecommendChanges, UnableToReview }
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdvisoryFinding {
    pub code: String,
    pub severity: FindingSeverity,
    pub location: Option<String>,
    pub detail: String,
}
impl<'de> Deserialize<'de> for AdvisoryFinding {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::report::deserialize_finding(deserializer, &mut None)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity { Info, Concern, Blocking }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnableToReviewReason { UnsupportedSubject, InsufficientContext, ModelDeclined }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState { Prepared, Invoking, OutputReady, PublicationPending, Unsettled(UnsettledPhase), Published, Cancelled, Failed }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsettledPhase { Invocation, Publication }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredReviewTransition { pub from: ReviewState, pub to: ReviewState }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewReportField {
    SchemaId, SubjectRef, SubjectSha256, ArtifactVersion, TaskHash, Recommendation, Summary,
    Findings, UnableReason, FindingCode, FindingSeverity, FindingLocation, FindingDetail, UnknownField,
}
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReviewParseError {
    #[error("malformed JSON")] MalformedJson,
    #[error("unknown or missing field")] UnknownOrMissingField,
    #[error("duplicate field: {field:?}")] DuplicateField { field: ReviewReportField },
    #[error("invalid schema")] InvalidSchema,
    #[error("invalid recommendation")] InvalidRecommendation,
    #[error("blank field: {field:?}")] BlankField { field: ReviewReportField },
    #[error("subject mismatch")] SubjectMismatch,
    #[error("task mismatch")] TaskMismatch,
    #[error("unable reason mismatch")] UnableReasonMismatch,
    #[error("unsupported control: {field:?}")] InvalidControlCharacter { field: ReviewReportField },
    #[error("byte limit exceeded: {limit}")] Oversize { limit: usize },
    #[error("depth limit exceeded: {limit}")] DepthLimit { limit: usize },
}
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReviewError {
    #[error("invalid field: {field}")] InvalidField { field: &'static str },
    #[error("size limit for {field}: {limit}")] SizeLimit { field: &'static str, limit: usize },
    #[error("depth limit: {limit}")] DepthLimit { limit: usize },
    #[error("JSON: {0}")] Json(String),
    #[error("schema mismatch: {field}")] SchemaMismatch { field: &'static str },
    #[error("context hash mismatch")] ContextHashMismatch,
    #[error("task mismatch")] TaskMismatch,
    #[error("report mismatch: {field}")] ReportMismatch { field: &'static str },
    #[error("report: {0}")] ParseReport(#[from] ReviewParseError),
    #[error("invalid declared transition: {from:?} -> {to:?}")] InvalidTransition { from: ReviewState, to: ReviewState },
    #[error("operation conflict: {operation_id}")] OperationConflict { operation_id: Uuid },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurePreparedReview {
    request: ValidatorArtifactReviewRequest,
    request_hash: ContentSha256,
    task_hash: ContentSha256,
    context_bundle_hash: ContentSha256,
}
impl PurePreparedReview {
    pub(crate) fn new_checked(request: ValidatorArtifactReviewRequest, request_hash: ContentSha256, task_hash: ContentSha256, context_bundle_hash: ContentSha256) -> Result<Self, ReviewError> {
        crate::report::validate_request(&request)?;
        if crate::report::hash_bytes(&crate::report::canonical_request_bytes(&request)?)? != request_hash {
            return Err(ReviewError::InvalidField { field: "request_hash" });
        }
        if crate::report::task_hash(&request.task)? != task_hash { return Err(ReviewError::TaskMismatch); }
        if request.context_bundle_sha256 != context_bundle_hash { return Err(ReviewError::ContextHashMismatch); }
        Ok(Self { request, request_hash, task_hash, context_bundle_hash })
    }
    pub fn request(&self) -> &ValidatorArtifactReviewRequest { &self.request }
    pub fn request_hash(&self) -> &ContentSha256 { &self.request_hash }
    pub fn task_hash(&self) -> &ContentSha256 { &self.task_hash }
    pub fn context_bundle_hash(&self) -> &ContentSha256 { &self.context_bundle_hash }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PureCheckedAdvisoryReport {
    report: AdvisoryReport,
    report_hash: ContentSha256,
    request_hash: ContentSha256,
}
impl PureCheckedAdvisoryReport {
    pub(crate) fn new_checked(report: AdvisoryReport, report_hash: ContentSha256, request_hash: ContentSha256) -> Result<Self, ReviewError> {
        if crate::report::hash_bytes(&crate::report::canonical_report_bytes(&report)?)? != report_hash {
            return Err(ReviewError::ReportMismatch { field: "report_hash" });
        }
        Ok(Self { report, report_hash, request_hash })
    }
    pub fn report(&self) -> &AdvisoryReport { &self.report }
    pub fn report_hash(&self) -> &ContentSha256 { &self.report_hash }
    pub fn request_hash(&self) -> &ContentSha256 { &self.request_hash }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_action_decoder_rejects_nested_unknown_and_duplicate_keys() {
        let fixture = crate::report::tests::REQUEST;
        let request: ValidatorArtifactReviewRequest = serde_json::from_str(fixture).unwrap();
        assert_eq!(request.expected_action.action_id, "fixture-action");
        assert_ne!(request.operation_id.to_string(), request.expected_action.action_id);
        let unknown = fixture.replace("\"action_id\":\"fixture-action\"", "\"unexpected\":0,\"action_id\":\"fixture-action\"");
        assert!(serde_json::from_str::<ValidatorArtifactReviewRequest>(&unknown).is_err());
        let duplicate = fixture.replace("\"action_id\":\"fixture-action\"", "\"action_id\":\"fixture-action\",\"action_id\":\"other\"");
        assert!(serde_json::from_str::<ValidatorArtifactReviewRequest>(&duplicate).is_err());
    }
}
