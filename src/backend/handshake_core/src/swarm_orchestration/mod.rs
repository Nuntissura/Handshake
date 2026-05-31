//! Local + cloud model SWARM ORCHESTRATION backend core (Master Spec §4.3.9
//! MULTI_MODEL_PARALLEL).
//!
//! This module is the load-bearing coordination core for running many model
//! sessions in parallel under hard, observable bounds. It is deliberately free
//! of candle/llama specifics: the only seam to the concrete runtime + process
//! world is the injected [`factory::ModelSessionFactory`], and the only seam to
//! telemetry is the injected [`events::SwarmEventSink`]. The same coordinator
//! therefore runs in production (real [`crate::model_runtime::ModelRuntime`]
//! load + process-ledger registration + flight-recorder sink) and in tests
//! (a real controllable worker adapter + recording sink + in-memory ledger).
//!
//! # Hardening primitives (all real, no TODOs)
//!
//! - **Two-level fan-out bound** — a `tokio::sync::Semaphore` concurrency cap
//!   AND a monotonic lifetime spawn counter with a hard ceiling reusing the
//!   [`crate::test_harness::invariants::HBR_SWARM_002_LOOP_CAP`] semantics.
//!   Exceeding either returns a typed [`error::SwarmError`], never silently.
//! - **Claim-lease + TTL + reaper** — each live session holds a
//!   [`coordinator::ClaimLease`]; a single background reaper task reclaims
//!   expired leases (cancel + unload + ledger stop) with a per-instance bounded
//!   respawn counter so a flapping session cannot storm.
//! - **Failure-fingerprint circuit breaker** — keyed on error *identity*
//!   (hash of class + truncated message), not per-task; trips after a threshold
//!   of same-signature failures and suppresses further spawns of that signature
//!   for a cooldown.
//! - **Budget-as-data** — a [`ids::RunBudget`] shared across the run; the
//!   coordinator stops spawning when exhausted and exposes
//!   [`coordinator::SwarmCoordinator::remaining`].
//! - **One spawn path + session registry** — a single
//!   [`coordinator::SwarmCoordinator::spawn_session`] entrypoint and a registry
//!   keyed on [`ids::ModelInstanceId`]; each session owns its own
//!   [`crate::model_runtime::CancellationToken`]; cancel cancels + unloads +
//!   ledger-stops + emits an event; terminal sessions are evicted.
//!
//! # Integration points reused (not reinvented)
//!
//! - [`crate::model_runtime`]: `ModelRuntime`, `CancellationToken`, `ModelId`.
//! - [`crate::process_ledger`]: `LedgerBatcher`, `ProcessStop`,
//!   `ProcessOwnershipRecordId`, `ReclaimTrigger` for spawn/stop attribution.
//! - [`crate::test_harness::invariants::HBR_SWARM_002_LOOP_CAP`]: lifetime
//!   spawn ceiling default.
//! - [`crate::flight_recorder`]: `FlightRecorderEvent` for the production sink.

pub mod breaker;
pub mod coordinator;
pub mod error;
pub mod events;
pub mod factory;
pub mod ids;
pub mod production_factory;
pub mod routing;
pub mod schedule;
pub mod state;

#[cfg(test)]
mod tests;

pub use breaker::{
    AdmitDecision, BreakerConfig, BreakerState, FailureFingerprint, FailureFingerprintBreaker,
};
pub use coordinator::{
    ClaimLease, SessionHandle, SwarmConfig, SwarmCoordinator, DEFAULT_MAX_RESPAWNS_PER_INSTANCE,
};
pub use error::{SwarmError, SwarmErrorClass, SwarmResult};
pub use events::{
    FlightRecorderSwarmSink, RecordingSwarmSink, SwarmEvent, SwarmEventSink, SwarmFrEventId,
};
pub use factory::{LiveSession, ModelSessionFactory, SessionTeardown};
pub use ids::{BudgetRemaining, ModelInstanceId, RunBudget, SpawnRequest};
pub use production_factory::{
    build_production_swarm_coordinator, default_swarm_concurrency, CloudLaneFactoryConfig,
    CloudLiveRuntime, CloudProviderFlavor, CloudRuntimeBuilder, ProductionModelSessionFactory,
    VaultCloudRuntimeBuilder,
};
pub use routing::{
    RoutingDecision, RoutingPolicy, RoutingPolicyConfig, SwarmRoutingError, TaskClass, TaskTier,
};
pub use state::ModelSessionState;
