//! MT-120: Distillation content-review pipeline.
//!
//! Wraps the [`pii_patterns`] scanner + license allowlist + exact-hash
//! deduplication into a single `ContentReview::review` entrypoint that
//! returns a structured [`ReviewVerdict`].
//!
//! Adult-production discipline (GLOBAL-PRODUCTION-002..009): per
//! MT-120 red_team minimum_controls explicit sexual content is NEVER a
//! PII category. The review pipeline gates information-leakage,
//! license compliance, and corpus quality — NOT operator-content
//! moderation.
//!
//! Quarantine moves NEVER delete: when a turn fails review, the
//! verdict records `quarantine_path` and the caller is responsible for
//! the file move. The pipeline itself never touches the filesystem.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::corpus_extractor::TrainingTurn;
use super::pii_patterns::{scan as scan_pii, PiiDetection, PiiSeverity};
use crate::flight_recorder::fr_event_registry::FrEventId;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};

/// Default license allowlist per MT-120 implementation_notes. Operators
/// override via [`ContentReviewConfig`].
pub const DEFAULT_LICENSE_ALLOWLIST: &[&str] = &["MIT", "Apache-2.0", "custom_internal"];
pub const FR_EVT_DISTILL_PII_DETECT: &str = "FR-EVT-DISTILL-PII-DETECT";
pub const NEAR_DUPLICATE_DETECTOR_LEXICAL_TOKEN_COSINE: &str = "lexical_token_cosine_v1";
pub const DEFAULT_NEAR_DUPLICATE_SIMILARITY_THRESHOLD_MILLI: u16 = 950;
pub const DEFAULT_NEAR_DUPLICATE_MIN_SHARED_TOKENS: usize = 8;

/// Per-operator configuration. The default constructor uses the
/// [`DEFAULT_LICENSE_ALLOWLIST`].
#[derive(Clone, Debug)]
pub struct ContentReviewConfig {
    pub license_allowlist: HashSet<String>,
    pub quarantine_root: String,
    pub near_duplicate_similarity_threshold_milli: u16,
    pub near_duplicate_min_shared_tokens: usize,
}

impl ContentReviewConfig {
    pub fn defaults() -> Self {
        Self {
            license_allowlist: DEFAULT_LICENSE_ALLOWLIST
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            quarantine_root: ".distill_quarantine".to_string(),
            near_duplicate_similarity_threshold_milli:
                DEFAULT_NEAR_DUPLICATE_SIMILARITY_THRESHOLD_MILLI,
            near_duplicate_min_shared_tokens: DEFAULT_NEAR_DUPLICATE_MIN_SHARED_TOKENS,
        }
    }
}

#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("turn id must not be empty")]
    EmptyTurnId,
}

/// Reasons why a turn was quarantined or rejected. Stable string tags
/// for telemetry filters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuarantineReason {
    PiiDetected {
        kind: String,
        severity: String,
    },
    LicenseNotAllowed {
        license_tag: String,
    },
    UntaggableLicense,
    DuplicateOfTurn {
        existing_turn_id: String,
    },
    NearDuplicateOfTurn {
        existing_turn_id: String,
        detector: String,
        similarity_milli: u16,
    },
}

/// Final verdict for a turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewVerdict {
    Pass {
        turn_id: String,
    },
    Quarantine {
        turn_id: String,
        reasons: Vec<QuarantineReason>,
        quarantine_path: String,
    },
    Reject {
        turn_id: String,
        reasons: Vec<QuarantineReason>,
    },
}

impl ReviewVerdict {
    pub fn turn_id(&self) -> &str {
        match self {
            ReviewVerdict::Pass { turn_id } => turn_id,
            ReviewVerdict::Quarantine { turn_id, .. } => turn_id,
            ReviewVerdict::Reject { turn_id, .. } => turn_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentReviewEvent {
    pub event_kind: String,
    pub turn_id: String,
    pub reason_tag: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentReviewOutcome {
    pub verdict: ReviewVerdict,
    pub events: Vec<ContentReviewEvent>,
}

impl ContentReviewOutcome {
    pub fn flight_recorder_events(&self, trace_id: Uuid, job_id: &str) -> Vec<FlightRecorderEvent> {
        let mut pii_kinds = Vec::new();
        let mut severities = Vec::new();

        for event in &self.events {
            if event.event_kind != FR_EVT_DISTILL_PII_DETECT {
                continue;
            }
            let Some((kind, severity)) = parse_pii_reason_tag(&event.reason_tag) else {
                continue;
            };
            if !pii_kinds.iter().any(|existing| existing == &kind) {
                pii_kinds.push(kind);
            }
            severities.push(severity);
        }

        if pii_kinds.is_empty() {
            return Vec::new();
        }

        let payload = json!({
            "type": "distill.pii_detected",
            "fr_event_id": FrEventId::DistillPiiDetect.as_str(),
            "turn_id": self.verdict.turn_id(),
            "pii_kinds": pii_kinds,
            "severity": strongest_pii_severity(&severities),
        });

        vec![FlightRecorderEvent::new(
            FlightRecorderEventType::DistillPiiDetected,
            FlightRecorderActor::System,
            trace_id,
            payload,
        )
        .with_job_id(job_id)]
    }

    pub async fn record_flight_recorder_events<R>(
        &self,
        recorder: &R,
        trace_id: Uuid,
        job_id: &str,
    ) -> Result<usize, RecorderError>
    where
        R: FlightRecorder + ?Sized,
    {
        let events = self.flight_recorder_events(trace_id, job_id);
        let count = events.len();
        for event in events {
            recorder.record_event(event).await?;
        }
        Ok(count)
    }
}

#[derive(Clone, Debug)]
struct TextFingerprint {
    prompt_counts: HashMap<String, u32>,
    completion_counts: HashMap<String, u32>,
    combined_counts: HashMap<String, u32>,
}

impl TextFingerprint {
    fn from_turn(turn: &TrainingTurn) -> Self {
        let prompt_counts = token_counts(&turn.prompt);
        let completion_counts = token_counts(&turn.completion);
        let mut combined_counts = prompt_counts.clone();
        for (token, count) in &completion_counts {
            *combined_counts.entry(token.clone()).or_insert(0) += *count;
        }
        Self {
            prompt_counts,
            completion_counts,
            combined_counts,
        }
    }

    fn near_duplicate_similarity_milli(
        &self,
        other: &Self,
        threshold_milli: u16,
        min_shared_tokens: usize,
    ) -> Option<u16> {
        let threshold_milli = threshold_milli.min(1000);
        let completion_similarity =
            cosine_similarity_milli(&self.completion_counts, &other.completion_counts);
        let completion_shared =
            shared_token_count(&self.completion_counts, &other.completion_counts);
        let prompt_similarity = cosine_similarity_milli(&self.prompt_counts, &other.prompt_counts);

        if completion_shared >= min_shared_tokens
            && completion_similarity >= threshold_milli
            && prompt_similarity >= threshold_milli.saturating_sub(50)
        {
            return Some(completion_similarity);
        }

        let combined_similarity =
            cosine_similarity_milli(&self.combined_counts, &other.combined_counts);
        let combined_shared = shared_token_count(&self.combined_counts, &other.combined_counts);

        if combined_shared >= min_shared_tokens
            && combined_similarity >= threshold_milli
            && prompt_similarity >= threshold_milli.saturating_sub(50)
            && completion_similarity >= threshold_milli.saturating_sub(50)
        {
            return Some(combined_similarity);
        }

        None
    }
}

/// Reviewer state. Holds the running set of accepted corpus hashes for
/// exact dedup and accepted text fingerprints for deterministic
/// near-duplicate prefiltering.
pub struct ContentReview {
    config: ContentReviewConfig,
    seen_hashes: HashMap<[u8; 32], String>,
    seen_fingerprints: Vec<(String, TextFingerprint)>,
}

impl ContentReview {
    pub fn new(config: ContentReviewConfig) -> Self {
        Self {
            config,
            seen_hashes: HashMap::new(),
            seen_fingerprints: Vec::new(),
        }
    }

    /// Compute the dedup hash for a turn's `(prompt, completion)`
    /// pair. SHA-256 stand-in for the BLAKE3 hash named in MT-120
    /// implementation_notes; the property required is a cryptographic
    /// collision-resistant exact-match hash, which both satisfy.
    /// Exposed so callers can pre-hash for batch dedup.
    pub fn dedup_hash(prompt: &str, completion: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        hasher.update(b"\0");
        hasher.update(completion.as_bytes());
        let result = hasher.finalize();
        let mut out = [0_u8; 32];
        out.copy_from_slice(&result);
        out
    }

    /// Review a single training turn.
    ///
    /// Decision tree:
    /// - Empty turn id -> [`ReviewError::EmptyTurnId`].
    /// - License tag empty -> Quarantine(UntaggableLicense).
    /// - License tag not in allowlist -> Quarantine(LicenseNotAllowed).
    /// - Any High-severity PII -> Reject.
    /// - Any Low/Medium PII -> Quarantine(PiiDetected per detection).
    /// - Exact dedup hit -> Quarantine(DuplicateOfTurn).
    /// - Near-dup fingerprint hit -> Quarantine(NearDuplicateOfTurn).
    /// - Otherwise Pass + record hash for future dedup.
    pub fn review(&mut self, turn: &TrainingTurn) -> Result<ReviewVerdict, ReviewError> {
        if turn.id.trim().is_empty() {
            return Err(ReviewError::EmptyTurnId);
        }

        let mut reasons = Vec::new();
        let mut hard_reject = false;

        // License gates first: an untaggable or non-allowed turn is
        // quarantined regardless of PII or dedup state.
        if turn.license_tag.trim().is_empty() {
            reasons.push(QuarantineReason::UntaggableLicense);
        } else if !self.config.license_allowlist.contains(&turn.license_tag) {
            reasons.push(QuarantineReason::LicenseNotAllowed {
                license_tag: turn.license_tag.clone(),
            });
        }

        // PII scan over BOTH prompt and completion.
        let mut detections: Vec<PiiDetection> = scan_pii(&turn.prompt);
        detections.extend(scan_pii(&turn.completion));
        for detection in &detections {
            if detection.severity == PiiSeverity::High {
                hard_reject = true;
            }
            reasons.push(QuarantineReason::PiiDetected {
                kind: detection.kind.label().to_string(),
                severity: format!("{:?}", detection.severity),
            });
        }

        if hard_reject {
            return Ok(ReviewVerdict::Reject {
                turn_id: turn.id.clone(),
                reasons,
            });
        }

        // Exact dedup BEFORE recording the hash. If reasons is still
        // empty here, dedup is the final gate; either we Pass and
        // record, or we Quarantine and skip the record.
        let hash = Self::dedup_hash(&turn.prompt, &turn.completion);
        if let Some(existing) = self.seen_hashes.get(&hash) {
            reasons.push(QuarantineReason::DuplicateOfTurn {
                existing_turn_id: existing.clone(),
            });
        }

        let fingerprint = TextFingerprint::from_turn(turn);
        if !reasons
            .iter()
            .any(|reason| matches!(reason, QuarantineReason::DuplicateOfTurn { .. }))
        {
            let threshold = self.config.near_duplicate_similarity_threshold_milli;
            let min_shared_tokens = self.config.near_duplicate_min_shared_tokens;
            if let Some((existing_turn_id, similarity_milli)) = self
                .seen_fingerprints
                .iter()
                .find_map(|(existing_turn_id, existing_fingerprint)| {
                    fingerprint
                        .near_duplicate_similarity_milli(
                            existing_fingerprint,
                            threshold,
                            min_shared_tokens,
                        )
                        .map(|similarity_milli| (existing_turn_id.clone(), similarity_milli))
                })
            {
                reasons.push(QuarantineReason::NearDuplicateOfTurn {
                    existing_turn_id,
                    detector: NEAR_DUPLICATE_DETECTOR_LEXICAL_TOKEN_COSINE.to_string(),
                    similarity_milli,
                });
            }
        }

        if reasons.is_empty() {
            self.seen_hashes.insert(hash, turn.id.clone());
            self.seen_fingerprints.push((turn.id.clone(), fingerprint));
            Ok(ReviewVerdict::Pass {
                turn_id: turn.id.clone(),
            })
        } else {
            Ok(ReviewVerdict::Quarantine {
                turn_id: turn.id.clone(),
                reasons,
                quarantine_path: format!(
                    "{root}/{turn_id}.json",
                    root = self.config.quarantine_root.trim_end_matches('/'),
                    turn_id = turn.id,
                ),
            })
        }
    }

    /// Review a turn and return event projections for rejection/quarantine
    /// reasons that downstream Flight Recorder adapters must persist.
    pub fn review_with_events(
        &mut self,
        turn: &TrainingTurn,
    ) -> Result<ContentReviewOutcome, ReviewError> {
        let verdict = self.review(turn)?;
        let events = content_review_events(&verdict);
        Ok(ContentReviewOutcome { verdict, events })
    }
}

fn token_counts(text: &str) -> HashMap<String, u32> {
    let mut counts = HashMap::new();
    let mut token = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            token.extend(ch.to_lowercase());
        } else if !token.is_empty() {
            *counts.entry(std::mem::take(&mut token)).or_insert(0) += 1;
        }
    }

    if !token.is_empty() {
        *counts.entry(token).or_insert(0) += 1;
    }

    counts
}

fn shared_token_count(left: &HashMap<String, u32>, right: &HashMap<String, u32>) -> usize {
    left.iter()
        .filter_map(|(token, left_count)| {
            right
                .get(token)
                .map(|right_count| (*left_count).min(*right_count) as usize)
        })
        .sum()
}

fn cosine_similarity_milli(left: &HashMap<String, u32>, right: &HashMap<String, u32>) -> u16 {
    if left.is_empty() || right.is_empty() {
        return 0;
    }

    let dot: f64 = left
        .iter()
        .filter_map(|(token, left_count)| {
            right
                .get(token)
                .map(|right_count| f64::from(*left_count) * f64::from(*right_count))
        })
        .sum();
    let left_norm = left
        .values()
        .map(|count| {
            let count = f64::from(*count);
            count * count
        })
        .sum::<f64>()
        .sqrt();
    let right_norm = right
        .values()
        .map(|count| {
            let count = f64::from(*count);
            count * count
        })
        .sum::<f64>()
        .sqrt();

    if left_norm == 0.0 || right_norm == 0.0 {
        return 0;
    }

    ((dot / (left_norm * right_norm)) * 1000.0)
        .round()
        .clamp(0.0, 1000.0) as u16
}

fn content_review_events(verdict: &ReviewVerdict) -> Vec<ContentReviewEvent> {
    let turn_id = verdict.turn_id().to_string();
    let reasons = match verdict {
        ReviewVerdict::Pass { .. } => return Vec::new(),
        ReviewVerdict::Quarantine { reasons, .. } | ReviewVerdict::Reject { reasons, .. } => {
            reasons
        }
    };

    reasons
        .iter()
        .filter_map(|reason| match reason {
            QuarantineReason::PiiDetected { kind, severity } => Some(ContentReviewEvent {
                event_kind: FR_EVT_DISTILL_PII_DETECT.to_string(),
                turn_id: turn_id.clone(),
                reason_tag: format!("pii:{kind}:{severity}"),
            }),
            _ => None,
        })
        .collect()
}

fn parse_pii_reason_tag(reason_tag: &str) -> Option<(String, String)> {
    let mut parts = reason_tag.splitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("pii"), Some(kind), Some(severity))
            if !kind.trim().is_empty() && is_known_pii_severity(severity) =>
        {
            Some((kind.to_string(), severity.to_string()))
        }
        _ => None,
    }
}

fn strongest_pii_severity(severities: &[String]) -> String {
    severities
        .iter()
        .max_by_key(|severity| pii_severity_rank(severity))
        .cloned()
        .unwrap_or_else(|| "Low".to_string())
}

fn is_known_pii_severity(severity: &str) -> bool {
    matches!(severity, "Low" | "Medium" | "High")
}

fn pii_severity_rank(severity: &str) -> u8 {
    match severity {
        "High" => 3,
        "Medium" => 2,
        "Low" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_turn(id: &str, prompt: &str, completion: &str, license: &str) -> TrainingTurn {
        TrainingTurn {
            id: id.to_string(),
            session_id: "session".to_string(),
            model_id: "model".to_string(),
            prompt: prompt.to_string(),
            completion: completion.to_string(),
            finish_reason: Some("stop".to_string()),
            license_tag: license.to_string(),
            source_event_ids: vec!["e1".to_string(), "e2".to_string()],
            sourced_at_utc: "2026-05-20T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn default_allowlist_is_mit_apache_internal() {
        let cfg = ContentReviewConfig::defaults();
        assert!(cfg.license_allowlist.contains("MIT"));
        assert!(cfg.license_allowlist.contains("Apache-2.0"));
        assert!(cfg.license_allowlist.contains("custom_internal"));
    }

    #[test]
    fn pass_when_license_allowed_and_no_pii_and_unique() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let verdict = reviewer
            .review(&passing_turn("t1", "what is 7*8?", "56", "MIT"))
            .expect("review");
        assert!(matches!(verdict, ReviewVerdict::Pass { .. }));
    }

    #[test]
    fn quarantines_when_license_not_in_allowlist() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let verdict = reviewer
            .review(&passing_turn("t1", "ok", "ok", "ProprietaryX"))
            .expect("review");
        match verdict {
            ReviewVerdict::Quarantine {
                turn_id,
                reasons,
                quarantine_path,
            } => {
                assert_eq!(turn_id, "t1");
                assert!(reasons
                    .iter()
                    .any(|r| matches!(r, QuarantineReason::LicenseNotAllowed { .. })));
                assert!(
                    quarantine_path.ends_with("/t1.json") || quarantine_path.ends_with("\\t1.json"),
                    "{quarantine_path}"
                );
            }
            other => panic!("expected Quarantine, got {other:?}"),
        }
    }

    #[test]
    fn quarantines_when_license_tag_empty() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let verdict = reviewer
            .review(&passing_turn("t1", "ok", "ok", ""))
            .expect("review");
        match verdict {
            ReviewVerdict::Quarantine { reasons, .. } => {
                assert!(reasons
                    .iter()
                    .any(|r| matches!(r, QuarantineReason::UntaggableLicense)));
            }
            other => panic!("expected Quarantine, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_high_severity_pii_detected() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let verdict = reviewer
            .review(&passing_turn(
                "t1",
                "use this api key sk-abcdefghij1234567890",
                "ok",
                "MIT",
            ))
            .expect("review");
        match verdict {
            ReviewVerdict::Reject { reasons, .. } => {
                assert!(reasons
                    .iter()
                    .any(|r| matches!(r, QuarantineReason::PiiDetected { severity, .. } if severity == "High")));
            }
            other => panic!("expected Reject, got {other:?}"),
        }
    }

    #[test]
    fn quarantines_when_only_low_or_medium_pii_detected() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let verdict = reviewer
            .review(&passing_turn(
                "t1",
                "email alice@example.com please",
                "ok",
                "MIT",
            ))
            .expect("review");
        assert!(matches!(verdict, ReviewVerdict::Quarantine { .. }));
    }

    #[test]
    fn detects_exact_duplicate_after_first_pass() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let first = reviewer
            .review(&passing_turn("t1", "What is 7*8?", "56", "MIT"))
            .expect("first");
        assert!(matches!(first, ReviewVerdict::Pass { .. }));
        let second = reviewer
            .review(&passing_turn("t2", "What is 7*8?", "56", "MIT"))
            .expect("second");
        match second {
            ReviewVerdict::Quarantine { reasons, .. } => {
                assert!(reasons
                    .iter()
                    .any(|r| matches!(r, QuarantineReason::DuplicateOfTurn { existing_turn_id } if existing_turn_id == "t1")));
            }
            other => panic!("expected Quarantine duplicate, got {other:?}"),
        }
    }

    #[test]
    fn explicit_sexual_content_passes_review_per_global_production() {
        // Per MT-120 red_team minimum_controls + GLOBAL-PRODUCTION-002..009:
        // the PII pipeline is for personal-information leakage, NOT
        // content moderation. Explicit operator-authored sexual
        // content must Pass when license + dedup are clean.
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let verdict = reviewer
            .review(&passing_turn(
                "t1",
                "explicit production prompt: pussy, tits, cock",
                "operator-authored explicit completion",
                "custom_internal",
            ))
            .expect("review");
        assert!(matches!(verdict, ReviewVerdict::Pass { .. }));
    }

    #[test]
    fn empty_turn_id_errors() {
        let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
        let err = reviewer
            .review(&passing_turn("", "p", "c", "MIT"))
            .expect_err("empty id");
        assert!(matches!(err, ReviewError::EmptyTurnId));
    }

    #[test]
    fn dedup_hash_is_deterministic_and_distinct_for_different_pairs() {
        let h1 = ContentReview::dedup_hash("p1", "c1");
        let h2 = ContentReview::dedup_hash("p1", "c1");
        let h3 = ContentReview::dedup_hash("p1", "c2");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }
}
