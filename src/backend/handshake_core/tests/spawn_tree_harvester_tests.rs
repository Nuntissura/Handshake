use std::sync::Mutex;

use handshake_core::{
    distillation::spawn_tree_harvester::{
        CandidateReviewGate, ContentReviewVerdict, ContentReviewer,
        DistillationCandidateSubmission, DistillationCandidateSubmitter, EventLedgerReader,
        HarvestError, SpawnPair, SpawnTreeHarvester, DISTILL_CANDIDATE_ACTION_ID,
        FR_EVT_DISTILL_CANDIDATE_CREATED,
    },
    memory::outcome_feedback::{CapsuleOutcome, FailureClass},
};
use uuid::Uuid;

struct TestLedger {
    pairs: Vec<SpawnPair>,
    calls: Mutex<Vec<Uuid>>,
}

impl TestLedger {
    fn new(pairs: Vec<SpawnPair>) -> Self {
        Self {
            pairs,
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl EventLedgerReader for TestLedger {
    fn read_spawn_pairs(&self, session_id: Uuid) -> Result<Vec<SpawnPair>, HarvestError> {
        self.calls.lock().unwrap().push(session_id);
        Ok(self.pairs.clone())
    }
}

struct TestReviewer {
    quarantine_child_message_ids: Vec<Uuid>,
    reviewed: Mutex<Vec<Uuid>>,
}

impl TestReviewer {
    fn passing() -> Self {
        Self {
            quarantine_child_message_ids: Vec::new(),
            reviewed: Mutex::new(Vec::new()),
        }
    }
}

impl ContentReviewer for TestReviewer {
    fn review_pair(&self, pair: &SpawnPair) -> Result<ContentReviewVerdict, HarvestError> {
        self.reviewed.lock().unwrap().push(pair.child_message_id);
        if self
            .quarantine_child_message_ids
            .contains(&pair.child_message_id)
        {
            Ok(ContentReviewVerdict::Quarantine {
                reason: "dedup".to_string(),
            })
        } else {
            Ok(ContentReviewVerdict::Pass {
                license_provenance: "custom_internal".to_string(),
            })
        }
    }
}

struct TestSubmitter {
    submissions: Mutex<Vec<DistillationCandidateSubmission>>,
}

impl TestSubmitter {
    fn new() -> Self {
        Self {
            submissions: Mutex::new(Vec::new()),
        }
    }
}

impl DistillationCandidateSubmitter for TestSubmitter {
    fn submit_candidate(
        &self,
        submission: DistillationCandidateSubmission,
    ) -> Result<Uuid, HarvestError> {
        let id = submission.candidate.candidate_id;
        self.submissions.lock().unwrap().push(submission);
        Ok(id)
    }
}

#[test]
fn harvest_with_opt_in_false_fails_before_reading_spawn_tree() {
    let ledger = TestLedger::new(vec![pass_pair(10)]);
    let reviewer = TestReviewer::passing();
    let submitter = TestSubmitter::new();
    let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);
    let session_id = Uuid::from_u128(1);

    let err = harvester.harvest(session_id, false).unwrap_err();

    assert!(matches!(err, HarvestError::OptInRequired { .. }));
    assert!(ledger.calls.lock().unwrap().is_empty());
    assert!(reviewer.reviewed.lock().unwrap().is_empty());
    assert!(submitter.submissions.lock().unwrap().is_empty());
}

#[test]
fn harvest_submits_only_pass_reviewed_pairs_with_event_and_review_gate() {
    let accepted = pass_pair(20);
    let failed = fail_pair(30);
    let quarantined = pass_pair(40);
    let reviewer = TestReviewer {
        quarantine_child_message_ids: vec![quarantined.child_message_id],
        reviewed: Mutex::new(Vec::new()),
    };
    let ledger = TestLedger::new(vec![accepted.clone(), failed, quarantined.clone()]);
    let submitter = TestSubmitter::new();
    let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);

    let candidates = harvester.harvest(Uuid::from_u128(1), true).unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].parent_message_id, accepted.parent_message_id);
    assert_eq!(candidates[0].candidate_id.get_version_num(), 7);
    assert_eq!(
        reviewer.reviewed.lock().unwrap().as_slice(),
        &[accepted.child_message_id, quarantined.child_message_id]
    );

    let submissions = submitter.submissions.lock().unwrap();
    assert_eq!(submissions.len(), 1);
    let submission = &submissions[0];
    assert_eq!(submission.action_id, DISTILL_CANDIDATE_ACTION_ID);
    assert_eq!(
        submission.event.event_name,
        FR_EVT_DISTILL_CANDIDATE_CREATED
    );
    assert_eq!(submission.event.candidate_id, candidates[0].candidate_id);
    assert_eq!(
        submission.review_gate,
        CandidateReviewGate::PendingSkillBankReview
    );
    assert!(!submission.auto_promote);
}

#[test]
fn candidate_submission_carries_non_authoritative_training_refs() {
    let pair = pass_pair(50);
    let ledger = TestLedger::new(vec![pair.clone()]);
    let reviewer = TestReviewer::passing();
    let submitter = TestSubmitter::new();
    let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);

    harvester.harvest(Uuid::from_u128(1), true).unwrap();

    let submissions = submitter.submissions.lock().unwrap();
    let submission = &submissions[0];
    assert!(submission
        .training_example_ref
        .starts_with("distillation-example://"));
    assert!(submission
        .teacher_signal_ref
        .starts_with("teacher-signal://"));
    assert!(submission
        .student_solution_ref
        .starts_with("student-solution://"));
    assert!(!submission.conversation_text_authority);
    assert_eq!(submission.event.parent_role, pair.parent_role);
    assert_eq!(submission.event.child_role, pair.child_role);
}

fn pass_pair(seed: u128) -> SpawnPair {
    pair(
        seed,
        CapsuleOutcome::Pass {
            mt_id: "MT-166".to_string(),
            validator_verdict_id: Uuid::from_u128(seed + 1000),
        },
    )
}

fn fail_pair(seed: u128) -> SpawnPair {
    pair(
        seed,
        CapsuleOutcome::Fail {
            mt_id: "MT-166".to_string(),
            validator_verdict_id: Uuid::from_u128(seed + 1000),
            failure_class: FailureClass::Other,
        },
    )
}

fn pair(seed: u128, child_outcome: CapsuleOutcome) -> SpawnPair {
    SpawnPair {
        session_id: Uuid::from_u128(1),
        parent_message: format!("parent request {seed}"),
        child_response: format!("child summary {seed}"),
        parent_role: "parent-role".to_string(),
        child_role: "child-role".to_string(),
        parent_message_id: Uuid::from_u128(seed),
        child_message_id: Uuid::from_u128(seed + 1),
        child_outcome,
    }
}
