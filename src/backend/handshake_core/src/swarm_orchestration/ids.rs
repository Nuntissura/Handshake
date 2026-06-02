//! Identity, spawn-request, and budget value types for the swarm coordinator.
//!
//! These are pure data: no async, no locks. They are the inputs the operator
//! / upstream scheduler hands to [`super::coordinator::SwarmCoordinator`].

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model_runtime::registry::RuntimeBinding;
use crate::model_runtime::ModelId;
use crate::model_runtime::ProviderKind;

/// Specific BYOK provider under the coarse `ProviderKind::ByokCloud` lane.
/// Kept optional on [`SpawnRequest`] for backward compatibility: old callers
/// that only say "byok_cloud" keep the existing production fallback order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByokCloudProvider {
    Anthropic,
    #[serde(rename = "openai", alias = "open_ai")]
    OpenAi,
}

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
    /// Provider lane the production factory dispatches on. `None` means a local
    /// runtime selected by `runtime_binding` (candle / llama.cpp). `Some(Local)`
    /// is equivalent. `Some(ByokCloud | OfficialCli)` routes to the cloud
    /// adapters; `runtime_binding` is then ignored for runtime selection (cloud
    /// has no local engine). Kept optional so existing local-only callers and
    /// the `new` constructor stay source-compatible.
    pub provider: Option<ProviderKind>,
    /// Cloud-lane model name (e.g. `claude-sonnet-4`, `gpt-4o`) the factory
    /// passes to the cloud adapter's allowlisted `load`. Required for cloud
    /// providers; ignored for local.
    pub cloud_model_name: Option<String>,
    /// Optional provider flavor for `ProviderKind::ByokCloud`. Without this the
    /// production factory uses its legacy configured-lane fallback; with it, the
    /// requested Anthropic/OpenAI lane must be configured and is honored exactly.
    pub byok_cloud_provider: Option<ByokCloudProvider>,
    /// Role that owns the spawned process (recorded in the process ledger).
    pub owner_role: String,
    /// Optional governance attribution carried into the ledger.
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    /// Parent session that requested this spawn (ledger lineage + reclaim key).
    pub parent_session_id: String,
    /// Filesystem path to the local model artifact (safetensors / GGUF) the
    /// production factory loads for a local runtime. Optional because a
    /// cloud-backed or test session has no local artifact.
    pub model_artifact_path: Option<String>,
    /// SHA-256 of the model artifact, for ledger + audit + the candle/llama
    /// integrity gate. Optional because a cloud-backed or test session may not
    /// have a local artifact.
    pub model_artifact_sha256: Option<String>,
    /// Board/lineage grouping (rank-2 structural unlock): the swarm this session
    /// belongs to. Becomes a swimlane on the operator board, a per-swarm budget
    /// scope, a Flight-Recorder drill-down join key, and a calendar schedule
    /// target. `None` for an ungrouped/ad-hoc session. Carried in the ledger
    /// `metadata_jsonb` (no schema change) and on every `SwarmEvent`.
    pub swarm_id: Option<String>,
    /// Board/lineage grouping (rank-2): the VM/sandbox worktree this session runs
    /// inside. Ties a session to its isolated worktree for per-worktree state
    /// recovery and board grouping. `None` for a session not bound to a worktree.
    pub worktree_id: Option<String>,
    /// Operator-assigned on-disk place for this session (absolute OR repo-relative,
    /// disk-agnostic). RECORDED ATTRIBUTION ONLY today: it is carried into the
    /// ledger / side-table / transcript for "where does this session live" answers
    /// and as the bridge to future VM-execution routing. It is NOT resolved,
    /// created, or used as a real cwd here — the swarm session runs in-process.
    /// `None` for a session with no assigned disk location.
    pub working_dir: Option<String>,
    /// Operator-intended isolation tier for this session (mirrors
    /// [`crate::sandbox::adapter::IsolationTier`]).
    ///
    /// WP-KERNEL-004 wave 1 made this LOAD-BEARING for exactly ONE route: a
    /// `Local`+`LlamaCpp` spawn with `isolation_tier == Some(Tier3Microvm)` is
    /// dispatched by [`super::production_factory::ProductionModelSessionFactory`]
    /// into a Cloud Hypervisor microVM (`create_sandboxed_local`) instead of an
    /// in-process llama.cpp load. Every OTHER tier value (incl. `None`,
    /// `Tier1Container`, `Tier2Syscall`) remains RECORDED-ONLY today — it is
    /// carried into the ledger / transcript as the operator's intent but does not
    /// yet select an execution substrate; those sessions still run in-process.
    /// `None` for no recorded tier.
    pub isolation_tier: Option<crate::sandbox::adapter::IsolationTier>,
    /// rank-7 time-boxing: an optional per-spawn lease lifetime. When set, the
    /// session's claim lease expires after this duration instead of the
    /// coordinator's configured `lease_ttl`; with no lease renewal the EXISTING
    /// reaper reclaims it at expiry -- a time-boxed (e.g. calendar-scheduled)
    /// session needs NO new teardown code. `None` uses the configured lease_ttl.
    pub time_box: Option<std::time::Duration>,
    /// Rank-6 committed-memory admission estimate for this session, in bytes.
    /// For local provider lanes, the coordinator reserves this amount before
    /// factory/model/VM creation and releases it only after terminal teardown.
    /// When a run configures a committed-memory ceiling, local `None` is
    /// rejected fail-closed as an unestimated request. Cloud/external provider
    /// lanes do not reserve host committed memory, so their estimates are
    /// ignored by admission and remain backward-compatible.
    pub committed_memory_bytes: Option<u64>,
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
            provider: None,
            cloud_model_name: None,
            byok_cloud_provider: None,
            owner_role: owner_role.into(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            parent_session_id: parent_session_id.into(),
            model_artifact_path: None,
            model_artifact_sha256: None,
            swarm_id: None,
            worktree_id: None,
            working_dir: None,
            isolation_tier: None,
            time_box: None,
            committed_memory_bytes: None,
        }
    }

    /// Set the local model artifact path + its expected sha256 (the integrity
    /// gate). Required for a local spawn (candle / llama.cpp).
    pub fn with_local_artifact(
        mut self,
        path: impl Into<String>,
        sha256: impl Into<String>,
    ) -> Self {
        self.model_artifact_path = Some(path.into());
        self.model_artifact_sha256 = Some(sha256.into());
        self
    }

    /// The local model artifact path, if set.
    pub fn model_artifact_path(&self) -> Option<&str> {
        self.model_artifact_path.as_deref()
    }

    /// Route this request to a cloud provider lane (BYOK cloud / official CLI).
    /// `model_name` is the allowlisted cloud model the adapter `load`s.
    pub fn with_cloud_provider(
        mut self,
        provider: ProviderKind,
        model_name: impl Into<String>,
    ) -> Self {
        self.provider = Some(provider);
        self.cloud_model_name = Some(model_name.into());
        self
    }

    /// Select the exact BYOK provider under `ProviderKind::ByokCloud`.
    pub fn with_byok_cloud_provider(mut self, provider: ByokCloudProvider) -> Self {
        self.byok_cloud_provider = Some(provider);
        self
    }

    pub fn with_wp(mut self, wp_id: impl Into<String>) -> Self {
        self.wp_id = Some(wp_id.into());
        self
    }

    pub fn with_mt(mut self, mt_id: impl Into<String>) -> Self {
        self.mt_id = Some(mt_id.into());
        self
    }

    /// Group this session under a swarm (board swimlane / per-swarm scope).
    pub fn with_swarm(mut self, swarm_id: impl Into<String>) -> Self {
        self.swarm_id = Some(swarm_id.into());
        self
    }

    /// Bind this session to a VM/sandbox worktree (per-worktree recovery + board).
    pub fn with_worktree(mut self, worktree_id: impl Into<String>) -> Self {
        self.worktree_id = Some(worktree_id.into());
        self
    }

    /// Record the operator-assigned on-disk place for this session. RECORDED
    /// ATTRIBUTION ONLY — see the `working_dir` field doc; not resolved or used as
    /// a cwd here.
    pub fn with_working_dir(mut self, working_dir: impl Into<String>) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    /// Record the operator-intended isolation tier. RECORDED ONLY — see the
    /// `isolation_tier` field doc; not enforced (the session runs in-process).
    pub fn with_isolation_tier(mut self, tier: crate::sandbox::adapter::IsolationTier) -> Self {
        self.isolation_tier = Some(tier);
        self
    }

    /// rank-7: time-box this session -- its lease expires after `ttl` and the
    /// existing reaper reclaims it (no renewal). For calendar-scheduled / bounded
    /// runs; reuses the lease+reaper teardown path (no new teardown code).
    pub fn with_time_box(mut self, ttl: std::time::Duration) -> Self {
        self.time_box = Some(ttl);
        self
    }

    /// Rank-6: reserve committed memory for this session before load/VM boot.
    /// A zero estimate is treated as no reservation so callers can pass through
    /// optional estimates without creating a permanent zero-valued field.
    pub fn with_committed_memory_bytes(mut self, bytes: u64) -> Self {
        self.committed_memory_bytes = (bytes > 0).then_some(bytes);
        self
    }

    pub fn swarm_id(&self) -> Option<&str> {
        self.swarm_id.as_deref()
    }

    pub fn worktree_id(&self) -> Option<&str> {
        self.worktree_id.as_deref()
    }

    /// The operator-assigned on-disk place, if recorded (attribution only).
    pub fn working_dir(&self) -> Option<&str> {
        self.working_dir.as_deref()
    }

    /// The operator-intended isolation tier, if recorded (not enforced).
    pub fn isolation_tier(&self) -> Option<crate::sandbox::adapter::IsolationTier> {
        self.isolation_tier
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
    /// rank-6 admission: max concurrent COLD STARTS (factory.create / model+VM
    /// boot), bounded SEPARATELY from `max_concurrent`. VM/sandbox boot (TAP/CNI
    /// setup) is the scale wall at high parallelism, so the number of SIMULTANEOUS
    /// boots is throttled independently from the number of RUNNING sessions: an
    /// admitted spawn waits for a boot slot, then releases it once booted so a
    /// running session never holds a boot slot. Defaults to `max_concurrent` (no
    /// extra throttle) until set lower via `with_cold_start_concurrency`.
    pub max_concurrent_cold_starts: usize,
    pub max_lifetime_spawns: u64,
    pub max_total_tokens: Option<u64>,
    pub max_total_cost_micros: Option<u64>,
    /// Rank-6 no-overcommit cap for total live committed memory, in bytes.
    /// Reservations are charged before factory.create()/VM boot and released
    /// after terminal teardown, so the admitted set cannot exceed this ceiling.
    #[serde(default)]
    pub max_committed_memory_bytes: Option<u64>,
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
            max_concurrent_cold_starts: max_concurrent,
            max_lifetime_spawns: crate::test_harness::invariants::HBR_SWARM_002_LOOP_CAP as u64,
            max_total_tokens: None,
            max_total_cost_micros: None,
            max_committed_memory_bytes: None,
        }
    }

    pub fn with_concurrency(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent = max_concurrent.max(1);
        self
    }

    /// rank-6 admission: bound the number of SIMULTANEOUS cold starts (model/VM
    /// boots) below the run-concurrency, so a burst of admitted spawns does not
    /// stampede the boot/networking layer (TAP/CNI is the scale wall). Clamped >= 1.
    pub fn with_cold_start_concurrency(mut self, max_concurrent_cold_starts: usize) -> Self {
        self.max_concurrent_cold_starts = max_concurrent_cold_starts.max(1);
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

    /// Rank-6: cap total committed memory of live sessions, in bytes.
    pub fn with_committed_memory_ceiling(mut self, max_committed_memory_bytes: u64) -> Self {
        self.max_committed_memory_bytes = Some(max_committed_memory_bytes);
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
    #[serde(default)]
    pub committed_memory_bytes_remaining: Option<u64>,
    pub exhausted: bool,
}
