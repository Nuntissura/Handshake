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

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::corpus_extractor::TrainingTurn;
use super::pii_patterns::{scan as scan_pii, PiiDetection, PiiSeverity};

/// Default license allowlist per MT-120 implementation_notes. Operators
/// override via [`ContentReviewConfig`].
pub const DEFAULT_LICENSE_ALLOWLIST: &[&str] = &["MIT", "Apache-2.0", "custom_internal"];

/// Per-operator configuration. The default constructor uses the
/// [`DEFAULT_LICENSE_ALLOWLIST`].
#[derive(Clone, Debug)]
pub struct ContentReviewConfig {
    pub license_allowlist: HashSet<String>,
    pub quarantine_root: String,
}

impl ContentReviewConfig {
    pub fn defaults() -> Self {
        Self {
            license_allowlist: DEFAULT_LICENSE_ALLOWLIST
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            quarantine_root: ".distill_quarantine".to_string(),
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
    PiiDetected { kind: String, severity: String },
    LicenseNotAllowed { license_tag: String },
    UntaggableLicense,
    DuplicateOfTurn { existing_turn_id: String },
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

/// Reviewer state. Holds the running set of corpus hashes for exact
/// dedup; near-dup (cosine_similarity > 0.95 via ModelRuntime.embed) is
/// deferred to a follow-on because it requires the live runtime
/// (MT-074).
pub struct ContentReview {
    config: ContentReviewConfig,
    seen_hashes: std::collections::HashMap<[u8; 32], String>,
}

impl ContentReview {
    pub fn new(config: ContentReviewConfig) -> Self {
        Self {
            config,
            seen_hashes: std::collections::HashMap::new(),
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

        if reasons.is_empty() {
            self.seen_hashes.insert(hash, turn.id.clone());
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
                    quarantine_path.ends_with("/t1.json")
                        || quarantine_path.ends_with("\\t1.json"),
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
