//! Bounded strict wire decoding and canonical integrity, not model execution or approval.

use crate::domain::*;
use handshake_role_authority::domain::ContentSha256;
use handshake_storage_support::canonical_json::{canonical_json_bytes, sha256_hex};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::fmt;

const TASK_LIMIT: usize = 256 * 1024;
const REPORT_LIMIT: usize = 256 * 1024;
const CONTEXT_LIMIT: usize = 8 * 1024 * 1024;
const DEPTH_LIMIT: usize = 64;

fn controls(value: &str) -> bool {
    value.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

fn identity(value: &str, field: &'static str) -> Result<(), ReviewError> {
    if value.is_empty() || value.len() > 512 || controls(value) { return Err(ReviewError::InvalidField { field }); }
    Ok(())
}

pub(crate) fn preflight_value(value: &Value, field: &'static str, limit_bytes: usize) -> Result<(), ReviewError> {
    let mut pending = vec![(value, 1usize)];
    let mut lower_bound = 0usize;
    while let Some((value, depth)) = pending.pop() {
        if depth > DEPTH_LIMIT { return Err(ReviewError::DepthLimit { limit: DEPTH_LIMIT }); }
        lower_bound = lower_bound.saturating_add(1);
        match value {
            Value::String(text) => {
                if controls(text) { return Err(ReviewError::InvalidField { field }); }
                lower_bound = lower_bound.saturating_add(text.len());
            }
            Value::Array(items) => pending.extend(items.iter().map(|item| (item, depth + 1))),
            Value::Object(items) => for (key, item) in items {
                if controls(key) { return Err(ReviewError::InvalidField { field }); }
                lower_bound = lower_bound.saturating_add(key.len());
                pending.push((item, depth + 1));
            },
            _ => {}
        }
        if lower_bound > limit_bytes { return Err(ReviewError::SizeLimit { field, limit: limit_bytes }); }
    }
    if canonical_json_bytes(value).len() > limit_bytes { return Err(ReviewError::SizeLimit { field, limit: limit_bytes }); }
    Ok(())
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> Result<ContentSha256, ReviewError> {
    ContentSha256::from_hex(&sha256_hex(bytes)).map_err(|e| ReviewError::Json(e.to_string()))
}
pub(crate) fn canonical_hash(value: &Value) -> Result<ContentSha256, ReviewError> {
    preflight_value(value, "canonical_value", CONTEXT_LIMIT)?;
    hash_bytes(&canonical_json_bytes(value))
}
pub(crate) fn validate_task(task: &ReviewTask) -> Result<(), ReviewError> {
    identity(&task.task_id, "task_id")?;
    if task.criteria.is_empty() || task.criteria.len() > 128 { return Err(ReviewError::InvalidField { field: "criteria" }); }
    for criterion in &task.criteria {
        if criterion.trim().is_empty() || controls(criterion) { return Err(ReviewError::InvalidField { field: "criteria" }); }
    }
    let value = serde_json::to_value(task).map_err(|e| ReviewError::Json(e.to_string()))?;
    preflight_value(&value, "task", TASK_LIMIT)
}
pub(crate) fn validate_request(request: &ValidatorArtifactReviewRequest) -> Result<(), ReviewError> {
    for (value, field) in [(&request.expected_action.action_id, "action_id"),
        (&request.expected_action.request_id, "request_id"), (&request.expected_action.idempotency_key, "idempotency_key"),
        (&request.designation_ref, "designation_ref"), (&request.registration_ref, "registration_ref"),
        (&request.subject_ref, "subject_ref"), (&request.expected_artifact_version, "artifact_version"),
        (&request.context_bundle_ref, "context_bundle_ref")] { identity(value, field)?; }
    validate_task(&request.task)
}

fn canonical(value: &impl Serialize, field: &'static str, limit: usize) -> Result<Vec<u8>, ReviewError> {
    let value = serde_json::to_value(value).map_err(|e| ReviewError::Json(e.to_string()))?;
    preflight_value(&value, field, limit)?;
    Ok(canonical_json_bytes(&value))
}
pub fn canonical_task_bytes(task: &ReviewTask) -> Result<Vec<u8>, ReviewError> {
    validate_task(task)?;
    canonical(task, "task", TASK_LIMIT)
}
pub fn canonical_request_bytes(request: &ValidatorArtifactReviewRequest) -> Result<Vec<u8>, ReviewError> {
    validate_request(request)?;
    canonical(request, "request", CONTEXT_LIMIT)
}
pub fn task_hash(task: &ReviewTask) -> Result<ContentSha256, ReviewError> { hash_bytes(&canonical_task_bytes(task)?) }

// Scan structural depth before a deserializer can recursively allocate. Strings and escapes
// are left intact; the typed JSON decoder still owns JSON syntax validation.
fn raw_depth(raw: &str) -> bool {
    let (mut depth, mut string, mut escape) = (0usize, false, false);
    for byte in raw.bytes() {
        if string {
            if escape { escape = false; }
            else if byte == b'\\' { escape = true; }
            else if byte == b'"' { string = false; }
        } else {
            match byte {
                b'"' => string = true,
                b'{' | b'[' => { depth += 1; if depth > DEPTH_LIMIT { return false; } }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    true
}

pub fn parse_task(raw: &str) -> Result<ReviewTask, ReviewError> {
    if raw.len() > TASK_LIMIT { return Err(ReviewError::SizeLimit { field: "task", limit: TASK_LIMIT }); }
    if !raw_depth(raw) { return Err(ReviewError::DepthLimit { limit: DEPTH_LIMIT }); }
    let value: Value = serde_json::from_str(raw).map_err(|e| ReviewError::Json(e.to_string()))?;
    preflight_value(&value, "task", TASK_LIMIT)?;
    let task = serde_json::from_str(raw).map_err(|e| ReviewError::Json(e.to_string()))?;
    validate_task(&task)?;
    Ok(task)
}

fn reject<E: de::Error>(error: &mut Option<ReviewParseError>, reason: ReviewParseError) -> E {
    *error = Some(reason);
    E::custom("review report rejected")
}
fn text_check(text: &str, field: ReviewReportField, nonblank: bool) -> Result<(), ReviewParseError> {
    if controls(text) { return Err(ReviewParseError::InvalidControlCharacter { field }); }
    if nonblank && text.trim().is_empty() { return Err(ReviewParseError::BlankField { field }); }
    Ok(())
}
fn report_identity(text: &str, field: ReviewReportField) -> Result<(), ReviewParseError> {
    text_check(text, field, false)?;
    if text.is_empty() { return Err(ReviewParseError::BlankField { field }); }
    if text.len() > 512 { return Err(ReviewParseError::Oversize { limit: 512 }); }
    Ok(())
}
fn field_name(key: &str, finding: bool) -> ReviewReportField {
    match (finding, key) {
        (false, "schema_id") => ReviewReportField::SchemaId,
        (false, "subject_ref") => ReviewReportField::SubjectRef,
        (false, "subject_sha256") => ReviewReportField::SubjectSha256,
        (false, "artifact_version") => ReviewReportField::ArtifactVersion,
        (false, "task_hash") => ReviewReportField::TaskHash,
        (false, "recommendation") => ReviewReportField::Recommendation,
        (false, "summary") => ReviewReportField::Summary,
        (false, "findings") => ReviewReportField::Findings,
        (false, "unable_reason") => ReviewReportField::UnableReason,
        (true, "code") => ReviewReportField::FindingCode,
        (true, "severity") => ReviewReportField::FindingSeverity,
        (true, "location") => ReviewReportField::FindingLocation,
        (true, "detail") => ReviewReportField::FindingDetail,
        _ => ReviewReportField::UnknownField,
    }
}

struct ReportSeed<'a>(&'a mut Option<ReviewParseError>);
impl<'de> DeserializeSeed<'de> for ReportSeed<'_> {
    type Value = AdvisoryReport;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> { d.deserialize_map(self) }
}
impl<'de> Visitor<'de> for ReportSeed<'_> {
    type Value = AdvisoryReport;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("a strict advisory report object") }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let (mut schema, mut subject, mut sha, mut version, mut task, mut recommendation, mut summary, mut findings, mut unable) =
            (None, None, None, None, None, None, None, None, None);
        while let Some(key) = map.next_key::<String>()? {
            let field = field_name(&key, false);
            if let Err(e) = text_check(&key, field, false) { return Err(reject(self.0, e)); }
            macro_rules! take {
                ($slot:ident, $ty:ty) => {{
                    if $slot.is_some() { return Err(reject(self.0, ReviewParseError::DuplicateField { field })); }
                    $slot = Some(map.next_value::<$ty>()?);
                }};
            }
            match key.as_str() {
                "schema_id" => {
                    take!(schema, String);
                    if let Err(e) = text_check(schema.as_ref().unwrap(), field, false) { return Err(reject(self.0, e)); }
                    if schema.as_deref() != Some("hsk.validator_routing_proposal_review_report@1") { return Err(reject(self.0, ReviewParseError::InvalidSchema)); }
                }
                "subject_ref" => take!(subject, String),
                "subject_sha256" => { take!(sha, String); },
                "artifact_version" => take!(version, String),
                "task_hash" => take!(task, String),
                "recommendation" => take!(recommendation, String),
                "summary" => take!(summary, String),
                "findings" => {
                    if findings.is_some() { return Err(reject(self.0, ReviewParseError::DuplicateField { field })); }
                    findings = Some(map.next_value_seed(FindingsSeed(self.0))?);
                }
                "unable_reason" => take!(unable, Option<String>),
                _ => return Err(reject(self.0, ReviewParseError::UnknownOrMissingField)),
            }
        }
        macro_rules! required { ($value:ident) => { $value.ok_or_else(|| reject::<M::Error>(self.0, ReviewParseError::UnknownOrMissingField))? }; }
        let _schema: String = required!(schema);
        let subject_ref = required!(subject);
        let artifact_version = required!(version);
        for (text, field) in [(&subject_ref, ReviewReportField::SubjectRef), (&artifact_version, ReviewReportField::ArtifactVersion)] {
            if let Err(e) = report_identity(text, field) { return Err(reject(self.0, e)); }
        }
        let sha = required!(sha);
        let task = required!(task);
        for (text, field) in [(&sha, ReviewReportField::SubjectSha256), (&task, ReviewReportField::TaskHash)] {
            if let Err(e) = text_check(text, field, false) { return Err(reject(self.0, e)); }
        }
        let subject_sha256 = ContentSha256::from_hex(&sha).map_err(|_| reject::<M::Error>(self.0, ReviewParseError::MalformedJson))?;
        let task_hash = ContentSha256::from_hex(&task).map_err(|_| reject::<M::Error>(self.0, ReviewParseError::MalformedJson))?;
        let recommendation = required!(recommendation);
        if let Err(e) = text_check(&recommendation, ReviewReportField::Recommendation, false) { return Err(reject(self.0, e)); }
        let recommendation = match recommendation.as_str() {
            "recommend_accept" => AdvisoryRecommendation::RecommendAccept,
            "recommend_changes" => AdvisoryRecommendation::RecommendChanges,
            "unable_to_review" => AdvisoryRecommendation::UnableToReview,
            _ => return Err(reject(self.0, ReviewParseError::InvalidRecommendation)),
        };
        let unable = required!(unable);
        let unable_reason = match unable {
            None => None,
            Some(value) => {
                if let Err(e) = text_check(&value, ReviewReportField::UnableReason, false) { return Err(reject(self.0, e)); }
                Some(match value.as_str() {
                    "unsupported_subject" => UnableToReviewReason::UnsupportedSubject,
                    "insufficient_context" => UnableToReviewReason::InsufficientContext,
                    "model_declined" => UnableToReviewReason::ModelDeclined,
                    _ => return Err(reject(self.0, ReviewParseError::UnableReasonMismatch)),
                })
            }
        };
        let report = AdvisoryReport { schema_id: AdvisoryReportSchema::RoutingProposalReviewV1,
            subject_ref, subject_sha256, artifact_version, task_hash, recommendation,
            summary: required!(summary), findings: required!(findings), unable_reason };
        if let Err(e) = validate_report(&report) { return Err(reject(self.0, e)); }
        Ok(report)
    }
}

struct FindingSeed<'a>(&'a mut Option<ReviewParseError>);
impl<'de> DeserializeSeed<'de> for FindingSeed<'_> {
    type Value = AdvisoryFinding;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> { d.deserialize_map(self) }
}
impl<'de> Visitor<'de> for FindingSeed<'_> {
    type Value = AdvisoryFinding;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("a strict advisory finding object") }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let (mut code, mut severity, mut location, mut detail) = (None, None, None, None);
        while let Some(key) = map.next_key::<String>()? {
            let field = field_name(&key, true);
            if let Err(e) = text_check(&key, field, false) { return Err(reject(self.0, e)); }
            macro_rules! take { ($slot:ident, $ty:ty) => {{
                if $slot.is_some() { return Err(reject(self.0, ReviewParseError::DuplicateField { field })); }
                $slot = Some(map.next_value::<$ty>()?);
            }}; }
            match key.as_str() {
                "code" => take!(code, String),
                "severity" => take!(severity, String),
                "location" => take!(location, Option<String>),
                "detail" => take!(detail, String),
                _ => return Err(reject(self.0, ReviewParseError::UnknownOrMissingField)),
            }
        }
        macro_rules! required { ($value:ident) => { $value.ok_or_else(|| reject::<M::Error>(self.0, ReviewParseError::UnknownOrMissingField))? }; }
        let severity = required!(severity);
        if let Err(e) = text_check(&severity, ReviewReportField::FindingSeverity, false) { return Err(reject(self.0, e)); }
        let severity = match severity.as_str() {
            "info" => FindingSeverity::Info, "concern" => FindingSeverity::Concern, "blocking" => FindingSeverity::Blocking,
            _ => return Err(reject(self.0, ReviewParseError::UnknownOrMissingField)),
        };
        let finding = AdvisoryFinding { code: required!(code), severity, location: required!(location), detail: required!(detail) };
        if let Err(e) = validate_finding(&finding) { return Err(reject(self.0, e)); }
        Ok(finding)
    }
}
struct FindingsSeed<'a>(&'a mut Option<ReviewParseError>);
impl<'de> DeserializeSeed<'de> for FindingsSeed<'_> {
    type Value = Vec<AdvisoryFinding>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> { d.deserialize_seq(self) }
}
impl<'de> Visitor<'de> for FindingsSeed<'_> {
    type Value = Vec<AdvisoryFinding>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("at most 512 advisory findings") }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
        let mut findings = Vec::new();
        while let Some(finding) = seq.next_element_seed(FindingSeed(self.0))? {
            if findings.len() == 512 { return Err(reject(self.0, ReviewParseError::Oversize { limit: 512 })); }
            findings.push(finding);
        }
        Ok(findings)
    }
}
pub(crate) fn deserialize_report<'de, D: Deserializer<'de>>(d: D, error: &mut Option<ReviewParseError>) -> Result<AdvisoryReport, D::Error> {
    ReportSeed(error).deserialize(d)
}
pub(crate) fn deserialize_finding<'de, D: Deserializer<'de>>(d: D, error: &mut Option<ReviewParseError>) -> Result<AdvisoryFinding, D::Error> {
    FindingSeed(error).deserialize(d)
}

fn validate_finding(finding: &AdvisoryFinding) -> Result<(), ReviewParseError> {
    report_identity(&finding.code, ReviewReportField::FindingCode)?;
    text_check(&finding.code, ReviewReportField::FindingCode, true)?;
    text_check(&finding.detail, ReviewReportField::FindingDetail, true)?;
    if let Some(location) = &finding.location { text_check(location, ReviewReportField::FindingLocation, true)?; }
    Ok(())
}
fn validate_report(report: &AdvisoryReport) -> Result<(), ReviewParseError> {
    report_identity(&report.subject_ref, ReviewReportField::SubjectRef)?;
    report_identity(&report.artifact_version, ReviewReportField::ArtifactVersion)?;
    text_check(&report.summary, ReviewReportField::Summary, true)?;
    if report.findings.len() > 512 { return Err(ReviewParseError::Oversize { limit: 512 }); }
    for finding in &report.findings { validate_finding(finding)?; }
    if (report.recommendation == AdvisoryRecommendation::UnableToReview) != report.unable_reason.is_some() {
        return Err(ReviewParseError::UnableReasonMismatch);
    }
    Ok(())
}
pub fn parse_advisory_report(raw: &str, binding: &ReviewOutputBinding) -> Result<AdvisoryReport, ReviewParseError> {
    if raw.len() > REPORT_LIMIT { return Err(ReviewParseError::Oversize { limit: REPORT_LIMIT }); }
    if !raw_depth(raw) { return Err(ReviewParseError::DepthLimit { limit: DEPTH_LIMIT }); }
    let mut decoder = serde_json::Deserializer::from_str(raw);
    let mut error = None;
    let report = deserialize_report(&mut decoder, &mut error).map_err(|_| error.unwrap_or(ReviewParseError::MalformedJson))?;
    decoder.end().map_err(|_| ReviewParseError::MalformedJson)?;
    if report.subject_ref != binding.subject_ref || report.subject_sha256 != binding.subject_sha256 || report.artifact_version != binding.artifact_version {
        return Err(ReviewParseError::SubjectMismatch);
    }
    if report.task_hash != binding.task_hash { return Err(ReviewParseError::TaskMismatch); }
    Ok(report)
}
pub fn canonical_report_bytes(report: &AdvisoryReport) -> Result<Vec<u8>, ReviewError> {
    validate_report(report)?;
    canonical(report, "report", REPORT_LIMIT)
}
pub fn check_report(raw: &str, prepared: &PurePreparedReview) -> Result<PureCheckedAdvisoryReport, ReviewError> {
    let request = prepared.request();
    let binding = ReviewOutputBinding { subject_ref: request.subject_ref.clone(), subject_sha256: request.expected_subject_sha256,
        artifact_version: request.expected_artifact_version.clone(), task_hash: *prepared.task_hash() };
    let report = parse_advisory_report(raw, &binding)?;
    let hash = hash_bytes(&canonical_report_bytes(&report)?)?;
    PureCheckedAdvisoryReport::new_checked(report, hash, *prepared.request_hash())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) const TASK: &str = r#"{"criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1","task_id":"fixture-review-task"}"#;
    pub(crate) const REQUEST: &str = r#"{"context_bundle_ref":"fixture-context-bundle","context_bundle_sha256":"789e6778c1fbb2b112cf2b2a08c4e21ef3432c7546747686e36f5271fb619598","designation_ref":"fixture-designation","expected_action":{"action_id":"fixture-action","idempotency_key":"fixture-idempotency","request_id":"fixture-request"},"expected_artifact_version":"1","expected_subject_sha256":"0000000000000000000000000000000000000000000000000000000000000000","operation_id":"00000000-0000-0000-0000-000000000010","registration_ref":"fixture-registration","subject_ref":"fixture-subject","task":{"criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1","task_id":"fixture-review-task"}}"#;
    pub(crate) const REPORT: &str = r#"{"artifact_version":"1","findings":[{"code":"fixture-check","detail":"Proposal reviewed as declared.","location":null,"severity":"info"}],"recommendation":"recommend_accept","schema_id":"hsk.validator_routing_proposal_review_report@1","subject_ref":"fixture-subject","subject_sha256":"0000000000000000000000000000000000000000000000000000000000000000","summary":"Fixture advisory review.","task_hash":"8a1023546d725f65d1175baaf428bd67806b0f4499ec2fdf642cab37aa8b208b","unable_reason":null}"#;
    fn binding() -> ReviewOutputBinding {
        let request: ValidatorArtifactReviewRequest = serde_json::from_str(REQUEST).unwrap();
        ReviewOutputBinding { subject_ref: request.subject_ref, subject_sha256: request.expected_subject_sha256,
            artifact_version: request.expected_artifact_version, task_hash: ContentSha256::from_hex("8a1023546d725f65d1175baaf428bd67806b0f4499ec2fdf642cab37aa8b208b").unwrap() }
    }
    #[test]
    fn fixed_task_request_and_report_bytes_and_hashes() {
        let task = parse_task(TASK).unwrap();
        assert_eq!(TASK.len(), 169);
        assert_eq!(canonical_task_bytes(&task).unwrap(), TASK.as_bytes());
        assert_eq!(task_hash(&task).unwrap().to_hex(), "8a1023546d725f65d1175baaf428bd67806b0f4499ec2fdf642cab37aa8b208b");
        let request = serde_json::from_str(REQUEST).unwrap();
        assert_eq!(REQUEST.len(), 728);
        assert_eq!(canonical_request_bytes(&request).unwrap(), REQUEST.as_bytes());
        assert_eq!(hash_bytes(REQUEST.as_bytes()).unwrap().to_hex(), "53a870eecd8b9c0532ad98cfa071208994367fc5b0e03fd1dc92638d00eae1ad");
        let report = parse_advisory_report(REPORT, &binding()).unwrap();
        assert_eq!(REPORT.len(), 488);
        assert_eq!(canonical_report_bytes(&report).unwrap(), REPORT.as_bytes());
        assert_eq!(hash_bytes(REPORT.as_bytes()).unwrap().to_hex(), "218bec3a521612840c2e02521f2e304de375e30b0158ae0f22fd50fb25ef7b67");
        let bundle: Value = serde_json::from_str(r#"{"allowed_context":{"validator_artifact_review_task":{"criteria":["Check the declared proposal."],"kind":"review_routing_proposal","schema_id":"hsk.validator_routing_proposal_review_task@1","task_id":"fixture-review-task"}},"bundle_id":"fixture-bundle","context_hash":"6bbcc95b20b7faca4475d7557d8c08f31cbd41080105ae24faa19e43d58c0304","schema_id":"fixture-context-bundle@1"}"#).unwrap();
        let prepared = crate::execution::prepare_review(&request, &bundle).unwrap();
        let checked = check_report(REPORT, &prepared).unwrap();
        assert_eq!(checked.report(), &report);
        assert_eq!(checked.report_hash().to_hex(), "218bec3a521612840c2e02521f2e304de375e30b0158ae0f22fd50fb25ef7b67");
        assert_eq!(checked.request_hash(), prepared.request_hash());
    }
    #[test]
    fn nullable_fields_are_required_and_duplicates_survive_raw_decoding() {
        for raw in [REPORT.replace(",\"unable_reason\":null", ""), REPORT.replace("\"location\":null,", "")] {
            assert_eq!(parse_advisory_report(&raw, &binding()), Err(ReviewParseError::UnknownOrMissingField));
        }
        let duplicate = REPORT.replace("\"summary\":", "\"summary\":\"duplicate\",\"summary\":");
        assert_eq!(parse_advisory_report(&duplicate, &binding()), Err(ReviewParseError::DuplicateField { field: ReviewReportField::Summary }));
        let duplicate = REPORT.replace("\"code\":", "\"code\":\"duplicate\",\"code\":");
        assert_eq!(parse_advisory_report(&duplicate, &binding()), Err(ReviewParseError::DuplicateField { field: ReviewReportField::FindingCode }));
        assert!(serde_json::from_str::<AdvisoryReport>(&REPORT.replace(",\"unable_reason\":null", "")).is_err());
    }
    #[test]
    fn report_errors_do_not_become_advisory_inability() {
        let b = binding();
        let unknown_control = REPORT.replacen('{', "{\"bad\\u0000key\":0,", 1);
        assert_eq!(parse_advisory_report(&unknown_control, &b), Err(ReviewParseError::InvalidControlCharacter { field: ReviewReportField::UnknownField }));
        let control = REPORT.replace("Fixture advisory review.", "bad\\u0008summary");
        assert_eq!(parse_advisory_report(&control, &b), Err(ReviewParseError::InvalidControlCharacter { field: ReviewReportField::Summary }));
        let unknown = REPORT.replacen('{', "{\"extra\":null,", 1);
        assert_eq!(parse_advisory_report(&unknown, &b), Err(ReviewParseError::UnknownOrMissingField));
        assert_eq!(parse_advisory_report(&REPORT.replace("hsk.validator_routing_proposal_review_report@1", "other-schema"), &b), Err(ReviewParseError::InvalidSchema));
        assert_eq!(parse_advisory_report(&REPORT.replace("recommend_accept", "unknown"), &b), Err(ReviewParseError::InvalidRecommendation));
        assert_eq!(parse_advisory_report(&REPORT.replace("recommend_accept", "unable_to_review"), &b), Err(ReviewParseError::UnableReasonMismatch));
        assert_eq!(parse_advisory_report(&REPORT.replace("fixture-subject", "other"), &b), Err(ReviewParseError::SubjectMismatch));
        let mut changed = b.clone(); changed.task_hash = ContentSha256::from_hex(&"1".repeat(64)).unwrap();
        assert_eq!(parse_advisory_report(REPORT, &changed), Err(ReviewParseError::TaskMismatch));
        let raw = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        assert_eq!(parse_advisory_report(&raw, &b), Err(ReviewParseError::DepthLimit { limit: 64 }));
        assert_eq!(parse_advisory_report(&" ".repeat(REPORT_LIMIT + 1), &b), Err(ReviewParseError::Oversize { limit: REPORT_LIMIT }));
    }
    #[test]
    fn task_and_value_bounds_preserve_order_and_supported_controls() {
        let mut task = parse_task(TASK).unwrap();
        task.criteria = vec!["first\n\r\t".into(), "second".into()];
        let before = task_hash(&task).unwrap(); task.criteria.reverse();
        assert_ne!(task_hash(&task).unwrap(), before);
        task.criteria = vec![" ".into()]; assert!(canonical_task_bytes(&task).is_err());
        task.criteria = vec!["criterion".into(); 129]; assert!(canonical_task_bytes(&task).is_err());
        assert!(parse_task(&TASK.replace("\"task_id\":", "\"task_id\":\"duplicate\",\"task_id\":")).is_err());
        let mut value = Value::Null;
        for _ in 0..65 { value = Value::Array(vec![value]); }
        assert_eq!(preflight_value(&value, "fixture", CONTEXT_LIMIT), Err(ReviewError::DepthLimit { limit: 64 }));
    }
}
