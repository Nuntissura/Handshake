//! Runtime-owned admission and quiescence tracking for detached model work.
//!
//! A guard belongs to the worker that actually performs the operation. Dropping
//! a caller future or token stream must not make detached work disappear from
//! shutdown accounting.

use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use thiserror::Error;
use tokio::sync::watch;

use super::types::{CancellationToken, ModelId};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuntimeActivityKind {
    Generate,
    Score,
    Embed,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuntimeActivityId(u64);

impl RuntimeActivityId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeActivity {
    pub id: RuntimeActivityId,
    pub model_id: ModelId,
    pub kind: RuntimeActivityKind,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RuntimeActivityRegistrationError {
    #[error("model runtime is quiescing; rejected new {kind:?} work")]
    Quiescing { kind: RuntimeActivityKind },
    #[error("model runtime exhausted its unique activity identifier space")]
    IdentifierExhausted,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RuntimeQuiesceError {
    #[error("model runtime adapter {adapter} does not provide a quiescence barrier")]
    Unsupported { adapter: String },
    #[error("model runtime quiescence timed out after {timeout:?}; remaining work: {remaining:?}")]
    TimedOut {
        timeout: Duration,
        remaining: Vec<RuntimeActivity>,
    },
    #[error("model runtime quiescence notification channel closed; remaining work: {remaining:?}")]
    NotificationChannelClosed { remaining: Vec<RuntimeActivity> },
}

#[derive(Clone)]
pub struct RuntimeActivityTracker {
    inner: Arc<RuntimeActivityInner>,
}

struct RuntimeActivityInner {
    state: Mutex<RuntimeActivityState>,
    revisions: watch::Sender<u64>,
}

struct RuntimeActivityState {
    accepting: bool,
    quiescing_models: HashSet<ModelId>,
    next_id: u64,
    revision: u64,
    active: BTreeMap<RuntimeActivityId, ActiveRuntimeActivity>,
}

struct ActiveRuntimeActivity {
    model_id: ModelId,
    kind: RuntimeActivityKind,
    cancellation: Option<CancellationToken>,
}

impl RuntimeActivityTracker {
    pub fn new() -> Self {
        let (revisions, _receiver) = watch::channel(0_u64);
        Self {
            inner: Arc::new(RuntimeActivityInner {
                state: Mutex::new(RuntimeActivityState {
                    accepting: true,
                    quiescing_models: HashSet::new(),
                    next_id: 0,
                    revision: 0,
                    active: BTreeMap::new(),
                }),
                revisions,
            }),
        }
    }

    /// Atomically admits an operation unless quiescence has already begun.
    ///
    /// `cancellation` must be the token polled by the generation worker. Score
    /// and embed workers are not synchronously cancellable and pass `None`.
    pub fn try_register(
        &self,
        model_id: ModelId,
        kind: RuntimeActivityKind,
        cancellation: Option<CancellationToken>,
    ) -> Result<RuntimeActivityGuard, RuntimeActivityRegistrationError> {
        let mut state = lock_state(&self.inner.state);
        if !state.accepting || state.quiescing_models.contains(&model_id) {
            return Err(RuntimeActivityRegistrationError::Quiescing { kind });
        }

        let next_id = state
            .next_id
            .checked_add(1)
            .ok_or(RuntimeActivityRegistrationError::IdentifierExhausted)?;
        state.next_id = next_id;
        let id = RuntimeActivityId(next_id);
        state.active.insert(
            id,
            ActiveRuntimeActivity {
                model_id,
                kind,
                cancellation,
            },
        );
        let revision = advance_revision(&mut state);
        drop(state);
        self.inner.revisions.send_replace(revision);

        Ok(RuntimeActivityGuard {
            inner: Arc::clone(&self.inner),
            id,
        })
    }

    pub fn active_operations(&self) -> Vec<RuntimeActivity> {
        active_operations(&lock_state(&self.inner.state))
    }

    pub fn is_accepting(&self) -> bool {
        lock_state(&self.inner.state).accepting
    }

    /// Permanently closes admission, cancels every cancellable generation, and
    /// waits for the actual workers to drop their guards under one total budget.
    pub async fn quiesce(&self, timeout: Duration) -> Result<(), RuntimeQuiesceError> {
        let started = tokio::time::Instant::now();
        let mut revisions = self.inner.revisions.subscribe();
        let (cancellations, revision) = {
            let mut state = lock_state(&self.inner.state);
            state.accepting = false;
            let cancellations = state
                .active
                .values()
                .filter_map(|activity| activity.cancellation.clone())
                .collect::<Vec<_>>();
            let revision = advance_revision(&mut state);
            (cancellations, revision)
        };
        self.inner.revisions.send_replace(revision);

        for cancellation in cancellations {
            cancellation.cancel();
        }

        loop {
            let remaining = self.active_operations();
            if remaining.is_empty() {
                return Ok(());
            }

            let Some(wait_budget) = timeout.checked_sub(started.elapsed()) else {
                return Err(RuntimeQuiesceError::TimedOut { timeout, remaining });
            };
            if wait_budget.is_zero() {
                return Err(RuntimeQuiesceError::TimedOut { timeout, remaining });
            }

            match tokio::time::timeout(wait_budget, revisions.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    let remaining = self.active_operations();
                    if remaining.is_empty() {
                        return Ok(());
                    }
                    return Err(RuntimeQuiesceError::NotificationChannelClosed { remaining });
                }
                Err(_) => {
                    let remaining = self.active_operations();
                    if remaining.is_empty() {
                        return Ok(());
                    }
                    return Err(RuntimeQuiesceError::TimedOut { timeout, remaining });
                }
            }
        }
    }

    /// Close admission only for `model_id`, cancel its cancellable workers,
    /// and wait for that model's guards. Sibling models remain admissible.
    pub async fn quiesce_model(
        &self,
        model_id: ModelId,
        timeout: Duration,
    ) -> Result<(), RuntimeQuiesceError> {
        let started = tokio::time::Instant::now();
        let mut revisions = self.inner.revisions.subscribe();
        let (cancellations, revision) = {
            let mut state = lock_state(&self.inner.state);
            state.quiescing_models.insert(model_id);
            let cancellations = state
                .active
                .values()
                .filter(|activity| activity.model_id == model_id)
                .filter_map(|activity| activity.cancellation.clone())
                .collect::<Vec<_>>();
            let revision = advance_revision(&mut state);
            (cancellations, revision)
        };
        self.inner.revisions.send_replace(revision);
        for cancellation in cancellations {
            cancellation.cancel();
        }
        loop {
            let remaining = self
                .active_operations()
                .into_iter()
                .filter(|activity| activity.model_id == model_id)
                .collect::<Vec<_>>();
            if remaining.is_empty() {
                return Ok(());
            }
            let Some(wait_budget) = timeout.checked_sub(started.elapsed()) else {
                return Err(RuntimeQuiesceError::TimedOut { timeout, remaining });
            };
            if wait_budget.is_zero() {
                return Err(RuntimeQuiesceError::TimedOut { timeout, remaining });
            }
            match tokio::time::timeout(wait_budget, revisions.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    let remaining = self
                        .active_operations()
                        .into_iter()
                        .filter(|activity| activity.model_id == model_id)
                        .collect::<Vec<_>>();
                    if remaining.is_empty() {
                        return Ok(());
                    }
                    return Err(RuntimeQuiesceError::NotificationChannelClosed { remaining });
                }
                Err(_) => {
                    let remaining = self
                        .active_operations()
                        .into_iter()
                        .filter(|activity| activity.model_id == model_id)
                        .collect::<Vec<_>>();
                    if remaining.is_empty() {
                        return Ok(());
                    }
                    return Err(RuntimeQuiesceError::TimedOut { timeout, remaining });
                }
            }
        }
    }

    /// Re-open one model's admission after a control transaction rolls back
    /// before unload. Global shutdown admission remains authoritative.
    pub fn resume_model(&self, model_id: ModelId) {
        let mut state = lock_state(&self.inner.state);
        if state.quiescing_models.remove(&model_id) {
            let revision = advance_revision(&mut state);
            drop(state);
            self.inner.revisions.send_replace(revision);
        }
    }
}

impl Default for RuntimeActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use = "the worker performing the operation must own this guard"]
pub struct RuntimeActivityGuard {
    inner: Arc<RuntimeActivityInner>,
    id: RuntimeActivityId,
}

impl RuntimeActivityGuard {
    pub fn id(&self) -> RuntimeActivityId {
        self.id
    }
}

impl Drop for RuntimeActivityGuard {
    fn drop(&mut self) {
        let mut state = lock_state(&self.inner.state);
        if state.active.remove(&self.id).is_none() {
            return;
        }
        let revision = advance_revision(&mut state);
        drop(state);
        self.inner.revisions.send_replace(revision);
    }
}

fn active_operations(state: &RuntimeActivityState) -> Vec<RuntimeActivity> {
    state
        .active
        .iter()
        .map(|(id, activity)| RuntimeActivity {
            id: *id,
            model_id: activity.model_id,
            kind: activity.kind,
        })
        .collect()
}

fn advance_revision(state: &mut RuntimeActivityState) -> u64 {
    state.revision = state.revision.wrapping_add(1);
    state.revision
}

fn lock_state(state: &Mutex<RuntimeActivityState>) -> MutexGuard<'_, RuntimeActivityState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
