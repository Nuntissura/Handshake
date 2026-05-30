//! Typed error surface for the swarm orchestration coordinator.
//!
//! Every refusal path the coordinator can take returns a typed [`SwarmError`]
//! variant rather than silently exceeding a bound, panicking, or returning a
//! stringly-typed catch-all. The variants are the contract the tests and
//! downstream wiring assert against (concurrency cap, lifetime ceiling, budget
//! exhaustion, breaker suppression, unknown instance, bad state transition).

use thiserror::Error;

use super::ids::ModelInstanceId;
use super::state::ModelSessionState;

/// Stable error class identifier used by the failure-fingerprint circuit
/// breaker. The breaker keys on `(error_class, truncated_message)` so the
/// `class` must be a coarse, stable discriminant — NOT the per-instance id or
/// a per-task value, otherwise one systemic failure would never accumulate a
/// signature across sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwarmErrorClass {
    ConcurrencyCapReached,
    LifetimeSpawnCeilingReached,
    BudgetExhausted,
    BreakerOpen,
    UnknownInstance,
    DuplicateInstance,
    InvalidStateTransition,
    FactoryFailed,
    ReclaimFailed,
    LedgerFailed,
    EventSinkFailed,
    Cancelled,
    Internal,
}

impl SwarmErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConcurrencyCapReached => "concurrency_cap_reached",
            Self::LifetimeSpawnCeilingReached => "lifetime_spawn_ceiling_reached",
            Self::BudgetExhausted => "budget_exhausted",
            Self::BreakerOpen => "breaker_open",
            Self::UnknownInstance => "unknown_instance",
            Self::DuplicateInstance => "duplicate_instance",
            Self::InvalidStateTransition => "invalid_state_transition",
            Self::FactoryFailed => "factory_failed",
            Self::ReclaimFailed => "reclaim_failed",
            Self::LedgerFailed => "ledger_failed",
            Self::EventSinkFailed => "event_sink_failed",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Error)]
pub enum SwarmError {
    /// The per-run concurrency semaphore is fully subscribed; spawning would
    /// exceed `max_concurrent`. Caller should back off and retry, NOT force.
    #[error("SWARM_CONCURRENCY_CAP_REACHED: {in_flight} of {cap} permits in use")]
    ConcurrencyCapReached { in_flight: usize, cap: usize },

    /// The monotonic lifetime spawn counter reached its hard ceiling
    /// (HBR-SWARM-002 loop-cap semantics). This is terminal for the run — the
    /// budget of *total* spawns is spent and cannot be replenished.
    #[error("SWARM_LIFETIME_SPAWN_CEILING_REACHED: spawned {spawned} reached ceiling {ceiling}")]
    LifetimeSpawnCeilingReached { spawned: u64, ceiling: u64 },

    /// A non-spawn budget dimension (tokens / cost) is exhausted.
    #[error("SWARM_BUDGET_EXHAUSTED: dimension={dimension}")]
    BudgetExhausted { dimension: String },

    /// The failure-fingerprint circuit breaker is open for the supplied
    /// signature; spawns/retries that would carry this signature are
    /// suppressed until the cooldown elapses.
    #[error("SWARM_BREAKER_OPEN: signature={signature} cooldown_remaining_ms={cooldown_remaining_ms}")]
    BreakerOpen {
        signature: String,
        cooldown_remaining_ms: u128,
    },

    #[error("SWARM_UNKNOWN_INSTANCE: {0}")]
    UnknownInstance(ModelInstanceId),

    #[error("SWARM_DUPLICATE_INSTANCE: {0}")]
    DuplicateInstance(ModelInstanceId),

    #[error("SWARM_INVALID_STATE_TRANSITION: {from:?} -> {to:?}")]
    InvalidStateTransition {
        from: ModelSessionState,
        to: ModelSessionState,
    },

    /// The injected [`super::factory::ModelSessionFactory`] failed to create a
    /// live session. Carries the underlying message for fingerprinting.
    #[error("SWARM_FACTORY_FAILED: {0}")]
    FactoryFailed(String),

    #[error("SWARM_RECLAIM_FAILED: {0}")]
    ReclaimFailed(String),

    #[error("SWARM_LEDGER_FAILED: {0}")]
    LedgerFailed(String),

    #[error("SWARM_EVENT_SINK_FAILED: {0}")]
    EventSinkFailed(String),

    #[error("SWARM_CANCELLED")]
    Cancelled,

    #[error("SWARM_INTERNAL: {0}")]
    Internal(String),
}

impl SwarmError {
    /// Coarse, stable error class for circuit-breaker fingerprinting.
    pub fn class(&self) -> SwarmErrorClass {
        match self {
            Self::ConcurrencyCapReached { .. } => SwarmErrorClass::ConcurrencyCapReached,
            Self::LifetimeSpawnCeilingReached { .. } => {
                SwarmErrorClass::LifetimeSpawnCeilingReached
            }
            Self::BudgetExhausted { .. } => SwarmErrorClass::BudgetExhausted,
            Self::BreakerOpen { .. } => SwarmErrorClass::BreakerOpen,
            Self::UnknownInstance(_) => SwarmErrorClass::UnknownInstance,
            Self::DuplicateInstance(_) => SwarmErrorClass::DuplicateInstance,
            Self::InvalidStateTransition { .. } => SwarmErrorClass::InvalidStateTransition,
            Self::FactoryFailed(_) => SwarmErrorClass::FactoryFailed,
            Self::ReclaimFailed(_) => SwarmErrorClass::ReclaimFailed,
            Self::LedgerFailed(_) => SwarmErrorClass::LedgerFailed,
            Self::EventSinkFailed(_) => SwarmErrorClass::EventSinkFailed,
            Self::Cancelled => SwarmErrorClass::Cancelled,
            Self::Internal(_) => SwarmErrorClass::Internal,
        }
    }

    /// The free-form detail carried by the error, used (truncated) as the
    /// second half of the failure fingerprint. Bounded variants without a
    /// message contribute an empty detail so the class alone keys them.
    pub fn detail(&self) -> &str {
        match self {
            Self::FactoryFailed(m)
            | Self::ReclaimFailed(m)
            | Self::LedgerFailed(m)
            | Self::EventSinkFailed(m)
            | Self::Internal(m) => m,
            Self::BudgetExhausted { dimension } => dimension,
            _ => "",
        }
    }

    /// Whether this error is a *capacity / budget* refusal (caller backs off)
    /// rather than a *failure* (counts toward the circuit breaker). Capacity
    /// refusals must NOT feed the breaker, or normal saturation would trip it.
    pub fn is_capacity_refusal(&self) -> bool {
        matches!(
            self,
            Self::ConcurrencyCapReached { .. }
                | Self::LifetimeSpawnCeilingReached { .. }
                | Self::BudgetExhausted { .. }
                | Self::BreakerOpen { .. }
        )
    }
}

pub type SwarmResult<T> = Result<T, SwarmError>;
