//! MT-123: Distillation candidate registry + PromotionGate.
//!
//! Per AC-DISTILL-LOOP-SAFEGUARDS: PromotionGate operator review is
//! required per promotion. Freshly trained
//! [`DistilledLoraArtifact`]s (from MT-122) land in this registry with
//! `review_status = Pending`. An operator-signed `promote()` flips
//! `Promoted`; an operator-signed `reject()` with reason flips
//! `Rejected`. Only `Promoted` candidates are eligible for
//! `LoraStackOps::mount` in production sessions; the mount-side gate
//! that consults [`CandidateRegistry::mount_status`] is wired
//! follow-on (kernel/core: this MT lands the registry + the
//! deterministic policy, and a future commit threads the check into
//! lora_stack/ops).
//!
//! Operator override: `settings.exec_policy.allow_unpromoted_distill`
//! is a per-session flag (out of scope here) that lets an explicit
//! experimental session mount Pending candidates. The registry exposes
//! the policy via `mount_status_with_override` so the upstream caller
//! can apply the override locally.

use std::collections::HashMap;
use std::sync::RwLock;

use thiserror::Error;

use super::peft_pipeline::DistilledLoraArtifact;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewStatus {
    Pending,
    Promoted,
    Rejected { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegisteredCandidate {
    pub lora_id: String,
    pub artifact: DistilledLoraArtifact,
    pub status: ReviewStatus,
    pub registered_at_utc: String,
    pub last_status_change_at_utc: String,
    pub promoted_by_operator_signature: Option<String>,
    pub rejected_by_operator_signature: Option<String>,
}

/// Decision returned by [`CandidateRegistry::mount_status`] for the
/// upstream `LoraStackOps::mount` gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountDecision {
    /// LoRA id is not a registered distillation candidate; mount
    /// proceeds normally (externally-provided LoRAs are unaffected
    /// by the PromotionGate).
    NotACandidate,
    /// Candidate is promoted; mount allowed in production sessions.
    Allow,
    /// Candidate is pending or rejected; mount must be refused
    /// unless the operator opted-in to
    /// `allow_unpromoted_distill`.
    Refuse { reason: String },
}

#[derive(Debug, Error)]
pub enum CandidateRegistryError {
    #[error("lora_id must not be empty")]
    EmptyLoraId,
    #[error("lora_id {0} is already registered")]
    DuplicateRegistration(String),
    #[error("lora_id {0} is not registered")]
    NotRegistered(String),
    #[error("operator_signature required for {0} action")]
    EmptySignature(&'static str),
    #[error("rejection reason must not be empty")]
    EmptyRejectionReason,
    #[error("internal registry lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Audit record emitted from each state transition; the upstream
/// flight-recorder wiring forwards these as
/// FR-EVT-DISTILL-CANDIDATE-REGISTER / PROMOTE / REJECT events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateAuditEvent {
    Registered {
        lora_id: String,
        registered_at_utc: String,
    },
    Promoted {
        lora_id: String,
        operator_signature: String,
        promoted_at_utc: String,
    },
    Rejected {
        lora_id: String,
        operator_signature: String,
        reason: String,
        rejected_at_utc: String,
    },
}

pub struct CandidateRegistry {
    inner: RwLock<HashMap<String, RegisteredCandidate>>,
}

impl Default for CandidateRegistry {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl CandidateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        lora_id: &str,
        artifact: DistilledLoraArtifact,
        now_utc: &str,
    ) -> Result<CandidateAuditEvent, CandidateRegistryError> {
        if lora_id.trim().is_empty() {
            return Err(CandidateRegistryError::EmptyLoraId);
        }
        let mut inner = self
            .inner
            .write()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        if inner.contains_key(lora_id) {
            return Err(CandidateRegistryError::DuplicateRegistration(
                lora_id.to_string(),
            ));
        }
        inner.insert(
            lora_id.to_string(),
            RegisteredCandidate {
                lora_id: lora_id.to_string(),
                artifact,
                status: ReviewStatus::Pending,
                registered_at_utc: now_utc.to_string(),
                last_status_change_at_utc: now_utc.to_string(),
                promoted_by_operator_signature: None,
                rejected_by_operator_signature: None,
            },
        );
        Ok(CandidateAuditEvent::Registered {
            lora_id: lora_id.to_string(),
            registered_at_utc: now_utc.to_string(),
        })
    }

    pub fn promote(
        &self,
        lora_id: &str,
        operator_signature: &str,
        now_utc: &str,
    ) -> Result<CandidateAuditEvent, CandidateRegistryError> {
        if operator_signature.trim().is_empty() {
            return Err(CandidateRegistryError::EmptySignature("promote"));
        }
        let mut inner = self
            .inner
            .write()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        let entry = inner
            .get_mut(lora_id)
            .ok_or_else(|| CandidateRegistryError::NotRegistered(lora_id.to_string()))?;
        entry.status = ReviewStatus::Promoted;
        entry.promoted_by_operator_signature = Some(operator_signature.to_string());
        entry.last_status_change_at_utc = now_utc.to_string();
        Ok(CandidateAuditEvent::Promoted {
            lora_id: lora_id.to_string(),
            operator_signature: operator_signature.to_string(),
            promoted_at_utc: now_utc.to_string(),
        })
    }

    pub fn reject(
        &self,
        lora_id: &str,
        operator_signature: &str,
        reason: &str,
        now_utc: &str,
    ) -> Result<CandidateAuditEvent, CandidateRegistryError> {
        if operator_signature.trim().is_empty() {
            return Err(CandidateRegistryError::EmptySignature("reject"));
        }
        if reason.trim().is_empty() {
            return Err(CandidateRegistryError::EmptyRejectionReason);
        }
        let mut inner = self
            .inner
            .write()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        let entry = inner
            .get_mut(lora_id)
            .ok_or_else(|| CandidateRegistryError::NotRegistered(lora_id.to_string()))?;
        entry.status = ReviewStatus::Rejected {
            reason: reason.to_string(),
        };
        entry.rejected_by_operator_signature = Some(operator_signature.to_string());
        entry.last_status_change_at_utc = now_utc.to_string();
        Ok(CandidateAuditEvent::Rejected {
            lora_id: lora_id.to_string(),
            operator_signature: operator_signature.to_string(),
            reason: reason.to_string(),
            rejected_at_utc: now_utc.to_string(),
        })
    }

    pub fn get(&self, lora_id: &str) -> Result<Option<RegisteredCandidate>, CandidateRegistryError> {
        let inner = self
            .inner
            .read()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        Ok(inner.get(lora_id).cloned())
    }

    /// Returns the mount decision for `lora_id`. The upstream
    /// `LoraStackOps::mount` consults this to honour the
    /// PromotionGate.
    pub fn mount_status(
        &self,
        lora_id: &str,
    ) -> Result<MountDecision, CandidateRegistryError> {
        let inner = self
            .inner
            .read()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        let Some(entry) = inner.get(lora_id) else {
            return Ok(MountDecision::NotACandidate);
        };
        Ok(match &entry.status {
            ReviewStatus::Promoted => MountDecision::Allow,
            ReviewStatus::Pending => MountDecision::Refuse {
                reason: format!(
                    "lora {lora_id} is a distillation candidate awaiting operator review (PromotionGate per AC-DISTILL-LOOP-SAFEGUARDS)"
                ),
            },
            ReviewStatus::Rejected { reason } => MountDecision::Refuse {
                reason: format!(
                    "lora {lora_id} is a rejected distillation candidate ({reason})"
                ),
            },
        })
    }

    /// Same as [`mount_status`] but honours the operator's
    /// per-session `allow_unpromoted_distill` opt-in.
    pub fn mount_status_with_override(
        &self,
        lora_id: &str,
        allow_unpromoted_distill: bool,
    ) -> Result<MountDecision, CandidateRegistryError> {
        let decision = self.mount_status(lora_id)?;
        if !allow_unpromoted_distill {
            return Ok(decision);
        }
        Ok(match decision {
            MountDecision::Refuse { .. } => {
                // Operator explicitly opted-in for experimental mounts.
                MountDecision::Allow
            }
            other => other,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distillation::peft_pipeline::PeftHyperparams;
    use std::path::PathBuf;

    fn artifact() -> DistilledLoraArtifact {
        DistilledLoraArtifact {
            lora_dir: PathBuf::from("lora"),
            teacher_model_path: PathBuf::from("teacher"),
            student_base_model_path: PathBuf::from("student"),
            corpus_path: PathBuf::from("corpus.jsonl"),
            corpus_turn_count: 10,
            corpus_quarantined_count: 1,
            corpus_rejected_count: 0,
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op".to_string(),
            finished_at_utc: "2026-05-20T04:00:00Z".to_string(),
        }
    }

    #[test]
    fn register_lands_pending_status() {
        let registry = CandidateRegistry::new();
        let event = registry
            .register("lora-1", artifact(), "2026-05-20T04:00:00Z")
            .expect("register");
        match event {
            CandidateAuditEvent::Registered { lora_id, .. } => assert_eq!(lora_id, "lora-1"),
            other => panic!("expected Registered, got {other:?}"),
        }
        let entry = registry.get("lora-1").unwrap().expect("entry");
        assert_eq!(entry.status, ReviewStatus::Pending);
    }

    #[test]
    fn promote_requires_signature_and_flips_status() {
        let registry = CandidateRegistry::new();
        registry
            .register("l", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        let err = registry
            .promote("l", " ", "2026-05-20T04:01:00Z")
            .expect_err("empty signature");
        assert!(matches!(err, CandidateRegistryError::EmptySignature("promote")));

        let event = registry
            .promote("l", "operator-ilja", "2026-05-20T04:02:00Z")
            .expect("promote");
        assert!(matches!(event, CandidateAuditEvent::Promoted { .. }));
        let entry = registry.get("l").unwrap().unwrap();
        assert_eq!(entry.status, ReviewStatus::Promoted);
        assert_eq!(entry.promoted_by_operator_signature.as_deref(), Some("operator-ilja"));
    }

    #[test]
    fn reject_requires_signature_and_reason() {
        let registry = CandidateRegistry::new();
        registry
            .register("l", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        let err = registry
            .reject("l", " ", "bad", "2026-05-20T04:01:00Z")
            .expect_err("empty signature");
        assert!(matches!(err, CandidateRegistryError::EmptySignature("reject")));

        let err = registry
            .reject("l", "op", "  ", "2026-05-20T04:01:00Z")
            .expect_err("empty reason");
        assert!(matches!(err, CandidateRegistryError::EmptyRejectionReason));

        let event = registry
            .reject("l", "op", "quality drop", "2026-05-20T04:02:00Z")
            .expect("reject");
        assert!(matches!(event, CandidateAuditEvent::Rejected { .. }));
        let entry = registry.get("l").unwrap().unwrap();
        if let ReviewStatus::Rejected { reason } = &entry.status {
            assert_eq!(reason, "quality drop");
        } else {
            panic!("expected Rejected status, got {:?}", entry.status);
        }
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let registry = CandidateRegistry::new();
        registry
            .register("l", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        let err = registry
            .register("l", artifact(), "2026-05-20T04:01:00Z")
            .expect_err("duplicate");
        assert!(matches!(err, CandidateRegistryError::DuplicateRegistration(_)));
    }

    #[test]
    fn promote_or_reject_unknown_lora_errors() {
        let registry = CandidateRegistry::new();
        let err = registry
            .promote("unknown", "op", "2026-05-20T04:00:00Z")
            .expect_err("not registered");
        assert!(matches!(err, CandidateRegistryError::NotRegistered(_)));
        let err = registry
            .reject("unknown", "op", "r", "2026-05-20T04:00:00Z")
            .expect_err("not registered");
        assert!(matches!(err, CandidateRegistryError::NotRegistered(_)));
    }

    #[test]
    fn mount_status_returns_not_a_candidate_for_unknown_lora() {
        let registry = CandidateRegistry::new();
        let decision = registry.mount_status("externally-provided").unwrap();
        assert_eq!(decision, MountDecision::NotACandidate);
    }

    #[test]
    fn mount_status_refuses_pending_and_rejected_allows_promoted() {
        let registry = CandidateRegistry::new();
        registry
            .register("pending", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .register("promoted", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .promote("promoted", "op", "2026-05-20T04:01:00Z")
            .unwrap();
        registry
            .register("rejected", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .reject("rejected", "op", "bad", "2026-05-20T04:01:00Z")
            .unwrap();

        let decision = registry.mount_status("pending").unwrap();
        assert!(matches!(decision, MountDecision::Refuse { .. }));
        let decision = registry.mount_status("promoted").unwrap();
        assert_eq!(decision, MountDecision::Allow);
        let decision = registry.mount_status("rejected").unwrap();
        assert!(matches!(decision, MountDecision::Refuse { .. }));
    }

    #[test]
    fn mount_status_with_override_allows_pending_when_operator_opts_in() {
        let registry = CandidateRegistry::new();
        registry
            .register("pending", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        let decision = registry
            .mount_status_with_override("pending", false)
            .unwrap();
        assert!(matches!(decision, MountDecision::Refuse { .. }));
        let decision = registry
            .mount_status_with_override("pending", true)
            .unwrap();
        assert_eq!(decision, MountDecision::Allow);
        // NotACandidate is unaffected by the override (no PromotionGate
        // bites for externally-provided LoRAs).
        let decision = registry
            .mount_status_with_override("externally-provided", true)
            .unwrap();
        assert_eq!(decision, MountDecision::NotACandidate);
    }

    #[test]
    fn empty_lora_id_rejected_on_register() {
        let registry = CandidateRegistry::new();
        let err = registry
            .register(" ", artifact(), "2026-05-20T04:00:00Z")
            .expect_err("empty lora_id");
        assert!(matches!(err, CandidateRegistryError::EmptyLoraId));
    }
}
