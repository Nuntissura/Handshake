//! MT-186 MT cancellation primitive (cooperative + forced) with cleanup hooks.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::job::MicroTaskJobId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MtCancellationReason {
    OperatorRequested { operator_id: String },
    SessionShutdown,
    BudgetExceeded,
    EscalationToHardGate,
    DependencyFailed { dep_job_id: Uuid },
}

/// Cooperative cancellation token observable by the executor at safe checkpoints.
#[derive(Debug, Clone)]
pub struct MtCancellationToken {
    job_id: MicroTaskJobId,
    flag: Arc<AtomicBool>,
    reason: Arc<Mutex<Option<MtCancellationReason>>>,
}

impl MtCancellationToken {
    pub fn new(job_id: MicroTaskJobId) -> Self {
        Self {
            job_id,
            flag: Arc::new(AtomicBool::new(false)),
            reason: Arc::new(Mutex::new(None)),
        }
    }

    pub fn job_id(&self) -> MicroTaskJobId {
        self.job_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    pub fn reason(&self) -> Option<MtCancellationReason> {
        self.reason.lock().unwrap().clone()
    }

    fn cancel(&self, reason: MtCancellationReason) -> bool {
        // Idempotent: only the first cancellation records the reason.
        let already = self.flag.swap(true, Ordering::AcqRel);
        if !already {
            *self.reason.lock().unwrap() = Some(reason);
            true
        } else {
            false
        }
    }
}

/// Cleanup hook trait. Hooks run in reverse-registration order on cancellation.
pub trait MtCancellationCleanupHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn cleanup(&self, job_id: MicroTaskJobId) -> Result<(), String>;
}

pub struct MtCanceller {
    tokens: Mutex<std::collections::HashMap<MicroTaskJobId, MtCancellationToken>>,
    hooks:
        Mutex<std::collections::HashMap<MicroTaskJobId, Vec<Arc<dyn MtCancellationCleanupHook>>>>,
}

impl Default for MtCanceller {
    fn default() -> Self {
        Self::new()
    }
}

impl MtCanceller {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(Default::default()),
            hooks: Mutex::new(Default::default()),
        }
    }

    pub fn register(&self, job_id: MicroTaskJobId) -> MtCancellationToken {
        let mut tokens = self.tokens.lock().unwrap();
        if let Some(token) = tokens.get(&job_id).cloned() {
            return token;
        }
        let token = MtCancellationToken::new(job_id);
        tokens.insert(job_id, token.clone());
        token
    }

    pub fn register_cleanup_hook(
        &self,
        job_id: MicroTaskJobId,
        hook: Arc<dyn MtCancellationCleanupHook>,
    ) {
        self.hooks
            .lock()
            .unwrap()
            .entry(job_id)
            .or_default()
            .push(hook);
    }

    /// Cooperative cancellation: flip the token; executor observes at next
    /// iteration boundary.
    pub fn request_cooperative(
        &self,
        job_id: MicroTaskJobId,
        reason: MtCancellationReason,
    ) -> bool {
        let tokens = self.tokens.lock().unwrap();
        match tokens.get(&job_id) {
            Some(t) => t.cancel(reason),
            None => false,
        }
    }

    /// Forced cancellation: run cleanup hooks in reverse order. Errors are
    /// captured but never abort the cancellation chain (per spec § cleanup
    /// chain robustness).
    pub fn force(&self, job_id: MicroTaskJobId, reason: MtCancellationReason) -> ForceCancelReport {
        let _ = self.request_cooperative(job_id, reason);
        let hooks = self
            .hooks
            .lock()
            .unwrap()
            .remove(&job_id)
            .unwrap_or_default();
        let mut errors = Vec::new();
        for hook in hooks.iter().rev() {
            if let Err(e) = hook.cleanup(job_id) {
                errors.push((hook.name(), e));
            }
        }
        ForceCancelReport {
            job_id,
            hooks_invoked: hooks.len() as u32,
            errors: errors
                .into_iter()
                .map(|(name, msg)| HookFailure {
                    hook_name: name.to_string(),
                    message: msg,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForceCancelReport {
    pub job_id: MicroTaskJobId,
    pub hooks_invoked: u32,
    pub errors: Vec<HookFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookFailure {
    pub hook_name: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    struct RecordingHook {
        name: &'static str,
        order: Arc<Mutex<Vec<&'static str>>>,
    }
    impl MtCancellationCleanupHook for RecordingHook {
        fn name(&self) -> &'static str {
            self.name
        }
        fn cleanup(&self, _job_id: MicroTaskJobId) -> Result<(), String> {
            self.order.lock().unwrap().push(self.name);
            Ok(())
        }
    }

    struct FailingHook {
        called: Arc<AtomicU32>,
    }
    impl MtCancellationCleanupHook for FailingHook {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn cleanup(&self, _job_id: MicroTaskJobId) -> Result<(), String> {
            self.called.fetch_add(1, Ordering::SeqCst);
            Err("simulated failure".to_string())
        }
    }

    #[test]
    fn cooperative_cancellation_is_idempotent() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register(id);
        let r1 = c.request_cooperative(id, MtCancellationReason::SessionShutdown);
        let r2 = c.request_cooperative(id, MtCancellationReason::SessionShutdown);
        assert!(r1);
        assert!(!r2);
    }

    #[test]
    fn token_observes_cancellation() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let t = c.register(id);
        assert!(!t.is_cancelled());
        c.request_cooperative(id, MtCancellationReason::BudgetExceeded);
        assert!(t.is_cancelled());
    }

    #[test]
    fn register_preserves_existing_cancelled_token() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let first = c.register(id);
        assert!(c.request_cooperative(id, MtCancellationReason::SessionShutdown));

        let second = c.register(id);

        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
        assert_eq!(second.reason(), Some(MtCancellationReason::SessionShutdown));
    }

    #[test]
    fn cleanup_hooks_run_in_reverse_order() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register(id);
        let order = Arc::new(Mutex::new(Vec::new()));
        c.register_cleanup_hook(
            id,
            Arc::new(RecordingHook {
                name: "first",
                order: Arc::clone(&order),
            }),
        );
        c.register_cleanup_hook(
            id,
            Arc::new(RecordingHook {
                name: "second",
                order: Arc::clone(&order),
            }),
        );
        c.register_cleanup_hook(
            id,
            Arc::new(RecordingHook {
                name: "third",
                order: Arc::clone(&order),
            }),
        );
        let _ = c.force(id, MtCancellationReason::SessionShutdown);
        let recorded = order.lock().unwrap().clone();
        assert_eq!(recorded, vec!["third", "second", "first"]);
    }

    #[test]
    fn cleanup_hook_error_does_not_abort_chain() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register(id);
        let calls = Arc::new(AtomicU32::new(0));
        c.register_cleanup_hook(
            id,
            Arc::new(FailingHook {
                called: Arc::clone(&calls),
            }),
        );
        c.register_cleanup_hook(
            id,
            Arc::new(FailingHook {
                called: Arc::clone(&calls),
            }),
        );
        let report = c.force(id, MtCancellationReason::SessionShutdown);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(report.errors.len(), 2);
    }
}
