//! MT-166: Session-spawn conversation distillation harvester.
//!
//! Walks a spawn tree, extracts parent-child message pairs whose outcome
//! is Pass, runs each through the ContentReviewer, and emits surviving
//! DistillationCandidate items via KernelActionCatalogV1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::memory::outcome_feedback::CapsuleOutcome;

pub const FR_EVT_DISTILL_CANDIDATE_CREATED: &str = "FR-EVT-DISTILL-CANDIDATE-CREATED";
pub const FR_EVT_DISTILL_OPT_IN_REQUIRED: &str = "FR-EVT-DISTILL-OPT-IN-REQUIRED";
pub const DISTILL_CANDIDATE_ACTION_ID: &str = "kernel.distillation.candidate";

/// Conversation pair extracted from a spawn tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnConversation {
    pub parent_message: String,
    pub child_response: String,
    pub parent_role: String,
    pub child_role: String,
    pub parent_message_id: Uuid,
    pub child_message_id: Uuid,
    pub child_outcome_kind: String, // "pass" / "fail" / "escalation" / "skipped"
}

/// Distillation candidate emitted by the harvester.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationCandidate {
    pub candidate_id: Uuid,
    pub session_id: Uuid,
    pub parent_message_id: Uuid,
    pub child_message_id: Uuid,
    pub parent_role: String,
    pub child_role: String,
    pub content_hash: String,
    pub license_provenance: String,
    pub pii_scan_result: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReviewGate {
    PendingSkillBankReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationCandidateCreatedEvent {
    pub event_id: Uuid,
    pub event_name: String,
    pub candidate_id: Uuid,
    pub session_id: Uuid,
    pub parent_message_id: Uuid,
    pub child_message_id: Uuid,
    pub parent_role: String,
    pub child_role: String,
    pub content_hash: String,
    pub emitted_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationCandidateSubmission {
    pub action_id: String,
    pub candidate: DistillationCandidate,
    pub event: DistillationCandidateCreatedEvent,
    pub review_gate: CandidateReviewGate,
    pub auto_promote: bool,
    pub training_example_ref: String,
    pub teacher_signal_ref: String,
    pub student_solution_ref: String,
    pub conversation_text_authority: bool,
}

/// Reader for the event ledger / spawn tree.
pub trait EventLedgerReader {
    fn read_spawn_pairs(&self, session_id: Uuid) -> Result<Vec<SpawnPair>, HarvestError>;
}

/// One spawn-tree pair with the attributed outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnPair {
    pub session_id: Uuid,
    pub parent_message: String,
    pub child_response: String,
    pub parent_role: String,
    pub child_role: String,
    pub parent_message_id: Uuid,
    pub child_message_id: Uuid,
    pub child_outcome: CapsuleOutcome,
}

/// Reviewer trait — production wires to cluster C ContentReviewer; tests
/// inject a stub.
pub trait ContentReviewer {
    fn review_pair(&self, pair: &SpawnPair) -> Result<ContentReviewVerdict, HarvestError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum ContentReviewVerdict {
    Pass { license_provenance: String },
    Quarantine { reason: String },
    Reject { reason: String },
}

/// Submitter for the resulting DistillationCandidate.
pub trait DistillationCandidateSubmitter {
    fn submit_candidate(
        &self,
        submission: DistillationCandidateSubmission,
    ) -> Result<Uuid, HarvestError>;
}

pub struct SpawnTreeHarvester<'a> {
    pub event_ledger: &'a dyn EventLedgerReader,
    pub content_reviewer: &'a dyn ContentReviewer,
    pub action_catalog: &'a dyn DistillationCandidateSubmitter,
}

impl<'a> SpawnTreeHarvester<'a> {
    pub fn new(
        event_ledger: &'a dyn EventLedgerReader,
        content_reviewer: &'a dyn ContentReviewer,
        action_catalog: &'a dyn DistillationCandidateSubmitter,
    ) -> Self {
        Self {
            event_ledger,
            content_reviewer,
            action_catalog,
        }
    }

    /// Harvest distillation candidates from a session's spawn tree.
    ///
    /// AC-DISTILL-OPT-IN: refuses unless opt_in_confirmed is true.
    /// Only Pass-outcome pairs are candidates (teacher-student signal
    /// from failures is poisoned).
    pub fn harvest(
        &self,
        session_id: Uuid,
        opt_in_confirmed: bool,
    ) -> Result<Vec<DistillationCandidate>, HarvestError> {
        if !opt_in_confirmed {
            return Err(HarvestError::OptInRequired { session_id });
        }
        let pairs = self.event_ledger.read_spawn_pairs(session_id)?;
        let mut candidates = Vec::new();
        for pair in pairs {
            // Only Pass-outcome pairs proceed.
            if !matches!(pair.child_outcome, CapsuleOutcome::Pass { .. }) {
                continue;
            }
            // Content review (PII + license + dedup)
            let verdict = self.content_reviewer.review_pair(&pair)?;
            let license = match verdict {
                ContentReviewVerdict::Pass { license_provenance } => license_provenance,
                ContentReviewVerdict::Quarantine { .. } => continue,
                ContentReviewVerdict::Reject { .. } => continue,
            };
            let content_hash = hash_pair(&pair);
            let candidate = DistillationCandidate {
                candidate_id: Uuid::now_v7(),
                session_id: pair.session_id,
                parent_message_id: pair.parent_message_id,
                child_message_id: pair.child_message_id,
                parent_role: pair.parent_role.clone(),
                child_role: pair.child_role.clone(),
                content_hash,
                license_provenance: license,
                pii_scan_result: "clean".to_string(),
                created_at_utc: Utc::now(),
            };
            self.action_catalog
                .submit_candidate(submission_for_candidate(candidate.clone()))?;
            candidates.push(candidate);
        }
        Ok(candidates)
    }
}

fn submission_for_candidate(candidate: DistillationCandidate) -> DistillationCandidateSubmission {
    DistillationCandidateSubmission {
        action_id: DISTILL_CANDIDATE_ACTION_ID.to_string(),
        event: DistillationCandidateCreatedEvent {
            event_id: Uuid::now_v7(),
            event_name: FR_EVT_DISTILL_CANDIDATE_CREATED.to_string(),
            candidate_id: candidate.candidate_id,
            session_id: candidate.session_id,
            parent_message_id: candidate.parent_message_id,
            child_message_id: candidate.child_message_id,
            parent_role: candidate.parent_role.clone(),
            child_role: candidate.child_role.clone(),
            content_hash: candidate.content_hash.clone(),
            emitted_at_utc: Utc::now(),
        },
        training_example_ref: format!("distillation-example://{}", candidate.candidate_id),
        teacher_signal_ref: format!("teacher-signal://{}", candidate.parent_message_id),
        student_solution_ref: format!("student-solution://{}", candidate.child_message_id),
        candidate,
        review_gate: CandidateReviewGate::PendingSkillBankReview,
        auto_promote: false,
        conversation_text_authority: false,
    }
}

fn hash_pair(pair: &SpawnPair) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pair.parent_message.as_bytes());
    hasher.update(b"\0");
    hasher.update(pair.child_response.as_bytes());
    let digest = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in digest.as_slice() {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HarvestError {
    #[error("AC-DISTILL-OPT-IN: opt_in_confirmed=false for session {session_id}; harvest refused")]
    OptInRequired { session_id: Uuid },
    #[error("event ledger read failed: {message}")]
    EventLedger { message: String },
    #[error("content review failed: {message}")]
    Review { message: String },
    #[error("candidate submit rejected: {code}: {reason}")]
    SubmitRejected { code: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct StubLedger {
        pairs: Vec<SpawnPair>,
    }
    impl EventLedgerReader for StubLedger {
        fn read_spawn_pairs(&self, _session_id: Uuid) -> Result<Vec<SpawnPair>, HarvestError> {
            Ok(self.pairs.clone())
        }
    }

    struct StubReviewer;
    impl ContentReviewer for StubReviewer {
        fn review_pair(&self, pair: &SpawnPair) -> Result<ContentReviewVerdict, HarvestError> {
            // Quarantine if parent_message contains PII marker; otherwise Pass.
            if pair.parent_message.contains("@") {
                Ok(ContentReviewVerdict::Quarantine {
                    reason: "pii".to_string(),
                })
            } else {
                Ok(ContentReviewVerdict::Pass {
                    license_provenance: "custom_internal".to_string(),
                })
            }
        }
    }

    struct StubSubmitter {
        submissions: Mutex<Vec<DistillationCandidateSubmission>>,
    }
    impl DistillationCandidateSubmitter for StubSubmitter {
        fn submit_candidate(
            &self,
            submission: DistillationCandidateSubmission,
        ) -> Result<Uuid, HarvestError> {
            let id = submission.candidate.candidate_id;
            self.submissions.lock().unwrap().push(submission);
            Ok(id)
        }
    }

    fn pair(id: u128, msg: &str, response: &str, outcome: CapsuleOutcome) -> SpawnPair {
        SpawnPair {
            session_id: Uuid::from_u128(1),
            parent_message: msg.to_string(),
            child_response: response.to_string(),
            parent_role: "parent".to_string(),
            child_role: "child".to_string(),
            parent_message_id: Uuid::from_u128(id),
            child_message_id: Uuid::from_u128(id + 1),
            child_outcome: outcome,
        }
    }

    #[test]
    fn harvest_refuses_without_opt_in() {
        let ledger = StubLedger { pairs: Vec::new() };
        let reviewer = StubReviewer;
        let submitter = StubSubmitter {
            submissions: Mutex::new(Vec::new()),
        };
        let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);
        let err = harvester.harvest(Uuid::now_v7(), false).unwrap_err();
        assert!(matches!(err, HarvestError::OptInRequired { .. }));
    }

    #[test]
    fn only_pass_pairs_become_candidates() {
        let ledger = StubLedger {
            pairs: vec![
                pair(
                    100,
                    "what is X",
                    "X is Y",
                    CapsuleOutcome::Pass {
                        mt_id: "MT-1".to_string(),
                        validator_verdict_id: Uuid::now_v7(),
                    },
                ),
                pair(
                    102,
                    "what is Z",
                    "Z is W",
                    CapsuleOutcome::Fail {
                        mt_id: "MT-1".to_string(),
                        validator_verdict_id: Uuid::now_v7(),
                        failure_class: crate::memory::outcome_feedback::FailureClass::Other,
                    },
                ),
            ],
        };
        let reviewer = StubReviewer;
        let submitter = StubSubmitter {
            submissions: Mutex::new(Vec::new()),
        };
        let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);
        let candidates = harvester.harvest(Uuid::now_v7(), true).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].parent_message_id, Uuid::from_u128(100));
        assert_eq!(submitter.submissions.lock().unwrap().len(), 1);
    }

    #[test]
    fn content_reviewer_quarantine_drops_candidate() {
        let ledger = StubLedger {
            pairs: vec![pair(
                100,
                "email me at a@b.com",
                "ok",
                CapsuleOutcome::Pass {
                    mt_id: "MT-1".to_string(),
                    validator_verdict_id: Uuid::now_v7(),
                },
            )],
        };
        let reviewer = StubReviewer;
        let submitter = StubSubmitter {
            submissions: Mutex::new(Vec::new()),
        };
        let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);
        let candidates = harvester.harvest(Uuid::now_v7(), true).unwrap();
        assert_eq!(candidates.len(), 0);
        assert_eq!(submitter.submissions.lock().unwrap().len(), 0);
    }

    #[test]
    fn candidate_content_hash_deterministic() {
        let p1 = pair(
            100,
            "q",
            "a",
            CapsuleOutcome::Pass {
                mt_id: "MT-1".to_string(),
                validator_verdict_id: Uuid::now_v7(),
            },
        );
        let h1 = hash_pair(&p1);
        let h2 = hash_pair(&p1);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
