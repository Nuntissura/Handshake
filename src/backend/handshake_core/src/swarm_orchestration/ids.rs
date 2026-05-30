//! Identity, spawn-request, and budget value types for the swarm coordinator.
//!
//! These are pure data: no async, no locks. They are the inputs the operator
//! / upstream scheduler hands to [`super::coordinator::SwarmCoordinator`].

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model_runtime::ModelId;
use crate::model_runtime::registry::RuntimeBinding;

/// Identifies one *instance* of a model in the swarm. The same `ModelId` may
/// run as multiple concurrent instances (e.g. two llama.cpp workers of the
/// same artifact for throughput), so the coordinator keys its registry on
/// `(model_id, instance)` rather than `ModelId` alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelInstanceId {
    pub model_id: ModelId,
    pub instance: u32,
}

impl ModelInstanceId {
    pub fn new(model_id: ModelId, instance: u32) -> Self {
        Self { model_id, instance }
    }
}

impl fmt::Display for ModelInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.model_id, self.instance)
    }
}

/// A request to spawn a single model session into the swarm. The factory turns
/// this into a live session; the coordinator enforces the bounds before the
/// factory is ever called.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnRequest {
    pub instance_id: ModelInstanceId,
    pub runtime_binding: RuntimeBinding,
    /// Role that owns the spawned process (recorded in the process ledger).
    pub owner_role: String,
    /// Optional governance attribution carried into the ledger.
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    /// Parent session that requested this spawn (ledger lineage + reclaim key).
    pub parent_session_id: String,
    /// SHA-256 of the model artifact, for ledger + audit. Optional because a
    /// cloud-backed or test session may not have a local artifact.
    pub model_artifact_sha256: Option<String>,
}

impl SpawnRequest {
    pub fn new(
        instance_id: ModelInstanceId,
        runtime_binding: RuntimeBinding,
        owner_role: impl Into<String>,
        parent_session_id: impl Into<String>,
    ) -> Self {
        Self {
            instance_id,
            runtime_binding,
            owner_role: owner_role.into(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            parent_session_id: parent_session_id.into(),
            model_artifact_sha256: None,
        }
    }

    pub fn with_wp(mut self, wp_id: impl Into<String>) -> Self {
        self.wp_id = Some(wp_id.into());
        self
    }

    pub fn with_mt(mut self, mt_id: impl Into<String>) -> Self {
        self.mt_id = Some(mt_id.into());
        self
    }

    pub fn with_artifact_sha256(mut self, sha256: impl Into<String>) -> Self {
        self.model_artifact_sha256 = Some(sha256.into());
        self
    }
}

/// Budget for an entire swarm run, expressed as plain data so it can be
/// snapshotted, serialised into the ledger, and asserted against in tests.
///
/// Two of the dimensions are hard structural bounds enforced on the spawn
/// path: `max_concurrent` (semaphore permits) and `max_lifetime_spawns`
/// (monotonic ceiling, HBR-SWARM-002 loop-cap semantics). The optional
/// token/cost ceilings are accounting dimensions the coordinator decrements as
/// work reports usage; when any reaches zero the coordinator stops spawning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBudget {
    pub max_concurrent: usize,
    pub max_lifetime_spawns: u64,
    pub max_total_tokens: Option<u64>,
    pub max_total_cost_micros: Option<u64>,
}

impl RunBudget {
    /// Default fan-out bound: concurrency capped at `min(cpus, n)` and a
    /// generous-but-finite lifetime ceiling drawn from the HBR-SWARM-002 loop
    /// cap so a runaway spawn loop cannot drain the host.
    pub fn defaulted(n: usize) -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        let max_concurrent = n.min(cpus).max(1);
        Self {
            max_concurrent,
            max_lifetime_spawns:
                crate::test_harness::invariants::HBR_SWARM_002_LOOP_CAP as u64,
            max_total_tokens: None,
            max_total_cost_micros: None,
        }
    }

    pub fn with_concurrency(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = max_concurrent.max(1);
        self
    }

    pub fn with_lifetime_spawns(mut self, max_lifetime_spawns: u64) -> Self {
        self.max_lifetime_spawns = max_lifetime_spawns;
        self
    }

    pub fn with_token_ceiling(mut self, max_total_tokens: u64) -> Self {
        self.max_total_tokens = Some(max_total_tokens);
        self
    }

    pub fn with_cost_ceiling(mut self, max_total_cost_micros: u64) -> Self {
        self.max_total_cost_micros = Some(max_total_cost_micros);
        self
    }
}

impl Default for RunBudget {
    fn default() -> Self {
        Self::defaulted(1)
    }
}

/// Live snapshot of what remains of a [`RunBudget`] mid-run. Returned by
/// `SwarmCoordinator::remaining()` so an operator dashboard / scheduler can see
/// headroom without reaching into coordinator internals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetRemaining {
    pub concurrency_permits_available: usize,
    pub lifetime_spawns_remaining: u64,
    pub tokens_remaining: Option<u64>,
    pub cost_micros_remaining: Option<u64>,
    pub exhausted: bool,
}
