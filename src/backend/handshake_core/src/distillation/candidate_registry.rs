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
use std::future::Future;
use std::sync::RwLock;

use thiserror::Error;

use super::peft_pipeline::DistilledLoraArtifact;
use crate::model_runtime::ModelRuntimeError;

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

    pub fn get(
        &self,
        lora_id: &str,
    ) -> Result<Option<RegisteredCandidate>, CandidateRegistryError> {
        let inner = self
            .inner
            .read()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        Ok(inner.get(lora_id).cloned())
    }

    /// Lists all registered candidates regardless of status. The MT-124
    /// `kernel.distill.list_candidates` Tauri surface filters this to
    /// `ReviewStatus::Pending` in the UI projection layer; the registry
    /// itself returns the full set so the Tauri command can project
    /// whichever subset the frontend asks for in the future (e.g. an
    /// audit view of Promoted + Rejected). Rows are sorted by
    /// `registered_at_utc DESC` so the freshest candidate is first.
    pub fn list(&self) -> Result<Vec<RegisteredCandidate>, CandidateRegistryError> {
        let inner = self
            .inner
            .read()
            .map_err(|err| CandidateRegistryError::LockPoisoned(err.to_string()))?;
        let mut rows: Vec<RegisteredCandidate> = inner.values().cloned().collect();
        rows.sort_by(|a, b| b.registered_at_utc.cmp(&a.registered_at_utc));
        Ok(rows)
    }

    /// Convenience: list only the candidates currently in `Pending`
    /// review status. The PromotionGate UI uses this for the
    /// "Pending Candidates" tab in MT-124.
    pub fn list_pending(&self) -> Result<Vec<RegisteredCandidate>, CandidateRegistryError> {
        let rows = self.list()?;
        Ok(rows
            .into_iter()
            .filter(|c| matches!(c.status, ReviewStatus::Pending))
            .collect())
    }

    /// Returns the mount decision for `lora_id`. The upstream
    /// `LoraStackOps::mount` consults this to honour the
    /// PromotionGate.
    pub fn mount_status(&self, lora_id: &str) -> Result<MountDecision, CandidateRegistryError> {
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

/// Error from the mount-side PromotionGate ([`mount_with_promotion_gate`]).
#[derive(Debug, Error)]
pub enum DistilledMountError {
    #[error(transparent)]
    Registry(#[from] CandidateRegistryError),
    #[error("mount refused for distilled LoRA {lora_id}: {reason}")]
    Refused { lora_id: String, reason: String },
    #[error("LoRA mount failed: {0}")]
    Mount(ModelRuntimeError),
}

/// Mount-side PromotionGate enforcement (MT-123 / AC-DISTILL-LOOP-SAFEGUARDS).
///
/// This is the executable gate the `CandidateRegistry` exists for: it consults
/// the registry BEFORE the actual LoRA mount runs, so a Pending or Rejected
/// distillation candidate cannot be mounted into a production session. The
/// production caller wraps the real `LoraStackOps::mount(desc, strength)` call
/// in `mount` and passes the candidate's `lora_id`; externally-provided
/// (non-candidate) LoRAs pass through untouched. `allow_unpromoted_distill` is
/// the operator's explicit per-session opt-in
/// (`settings.exec_policy.allow_unpromoted_distill`).
///
/// On `Refuse` the `mount` future is never created/awaited — the gate fails
/// closed before the LoRA is loaded.
pub async fn mount_with_promotion_gate<F, Fut>(
    registry: &CandidateRegistry,
    lora_id: &str,
    allow_unpromoted_distill: bool,
    mount: F,
) -> Result<(), DistilledMountError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), ModelRuntimeError>>,
{
    match registry.mount_status_with_override(lora_id, allow_unpromoted_distill)? {
        MountDecision::Refuse { reason } => Err(DistilledMountError::Refused {
            lora_id: lora_id.to_string(),
            reason,
        }),
        MountDecision::Allow | MountDecision::NotACandidate => {
            mount().await.map_err(DistilledMountError::Mount)
        }
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
        assert!(matches!(
            err,
            CandidateRegistryError::EmptySignature("promote")
        ));

        let event = registry
            .promote("l", "operator-ilja", "2026-05-20T04:02:00Z")
            .expect("promote");
        assert!(matches!(event, CandidateAuditEvent::Promoted { .. }));
        let entry = registry.get("l").unwrap().unwrap();
        assert_eq!(entry.status, ReviewStatus::Promoted);
        assert_eq!(
            entry.promoted_by_operator_signature.as_deref(),
            Some("operator-ilja")
        );
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
        assert!(matches!(
            err,
            CandidateRegistryError::EmptySignature("reject")
        ));

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
        assert!(matches!(
            err,
            CandidateRegistryError::DuplicateRegistration(_)
        ));
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

    #[test]
    fn list_returns_all_candidates_sorted_freshest_first() {
        // MT-124 list-surface test: the Tauri command kernel.distill.list_candidates
        // depends on this listing API; the Promotion Queue tab needs a
        // deterministic newest-first order.
        let registry = CandidateRegistry::new();
        registry
            .register("lora-old", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .register("lora-new", artifact(), "2026-05-20T04:30:00Z")
            .unwrap();
        registry
            .register("lora-mid", artifact(), "2026-05-20T04:15:00Z")
            .unwrap();

        let rows = registry.list().expect("list");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].lora_id, "lora-new");
        assert_eq!(rows[1].lora_id, "lora-mid");
        assert_eq!(rows[2].lora_id, "lora-old");
    }

    #[test]
    fn list_pending_filters_to_pending_status_only() {
        let registry = CandidateRegistry::new();
        registry
            .register("lora-a", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .register("lora-b", artifact(), "2026-05-20T04:01:00Z")
            .unwrap();
        registry
            .register("lora-c", artifact(), "2026-05-20T04:02:00Z")
            .unwrap();
        registry
            .promote("lora-b", "op", "2026-05-20T04:03:00Z")
            .unwrap();
        registry
            .reject("lora-c", "op", "quality drop", "2026-05-20T04:04:00Z")
            .unwrap();

        let pending = registry.list_pending().expect("list_pending");
        assert_eq!(pending.len(), 1, "only lora-a remains Pending");
        assert_eq!(pending[0].lora_id, "lora-a");
        assert_eq!(pending[0].status, ReviewStatus::Pending);
    }

    // MT-123: mount-side PromotionGate enforcement. The gate must refuse the
    // actual mount for Pending/Rejected candidates (fail closed, never reach
    // the mount future) and allow it for Promoted / non-candidate LoRAs.

    #[tokio::test]
    async fn mount_gate_refuses_pending_and_skips_mount() {
        let registry = CandidateRegistry::new();
        registry
            .register("cand", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        let mounted = std::cell::Cell::new(false);
        let result = mount_with_promotion_gate(&registry, "cand", false, || async {
            mounted.set(true);
            Ok::<(), ModelRuntimeError>(())
        })
        .await;
        assert!(
            matches!(result, Err(DistilledMountError::Refused { .. })),
            "{result:?}"
        );
        assert!(
            !mounted.get(),
            "pending candidate must not reach the mount path"
        );
    }

    #[tokio::test]
    async fn mount_gate_refuses_rejected_and_skips_mount() {
        let registry = CandidateRegistry::new();
        registry
            .register("cand", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .reject("cand", "op", "quality drop", "2026-05-20T04:01:00Z")
            .unwrap();
        let mounted = std::cell::Cell::new(false);
        let result = mount_with_promotion_gate(&registry, "cand", false, || async {
            mounted.set(true);
            Ok::<(), ModelRuntimeError>(())
        })
        .await;
        assert!(
            matches!(result, Err(DistilledMountError::Refused { .. })),
            "{result:?}"
        );
        assert!(!mounted.get());
    }

    #[tokio::test]
    async fn mount_gate_allows_promoted_and_mounts() {
        let registry = CandidateRegistry::new();
        registry
            .register("cand", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        registry
            .promote("cand", "operator-ilja", "2026-05-20T04:02:00Z")
            .unwrap();
        let mounted = std::cell::Cell::new(false);
        mount_with_promotion_gate(&registry, "cand", false, || async {
            mounted.set(true);
            Ok::<(), ModelRuntimeError>(())
        })
        .await
        .expect("promoted candidate mounts");
        assert!(mounted.get());
    }

    #[tokio::test]
    async fn mount_gate_passes_through_non_candidate() {
        let registry = CandidateRegistry::new();
        let mounted = std::cell::Cell::new(false);
        mount_with_promotion_gate(&registry, "externally-provided", false, || async {
            mounted.set(true);
            Ok::<(), ModelRuntimeError>(())
        })
        .await
        .expect("non-candidate LoRA mounts");
        assert!(mounted.get());
    }

    #[tokio::test]
    async fn mount_gate_override_allows_unpromoted() {
        let registry = CandidateRegistry::new();
        registry
            .register("cand", artifact(), "2026-05-20T04:00:00Z")
            .unwrap();
        let mounted = std::cell::Cell::new(false);
        mount_with_promotion_gate(&registry, "cand", true, || async {
            mounted.set(true);
            Ok::<(), ModelRuntimeError>(())
        })
        .await
        .expect("operator override mounts unpromoted candidate");
        assert!(mounted.get());
    }
}
