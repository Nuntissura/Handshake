use sha2::{Digest, Sha256};

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
    CheckoutLeaseUnavailable,
    InvalidStateTransition,
    FactoryFailed,
    ProviderNotConfigured,
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
            Self::CheckoutLeaseUnavailable => "checkout_lease_unavailable",
            Self::InvalidStateTransition => "invalid_state_transition",
            Self::FactoryFailed => "factory_failed",
            Self::ProviderNotConfigured => "provider_not_configured",
            Self::ReclaimFailed => "reclaim_failed",
            Self::LedgerFailed => "ledger_failed",
            Self::EventSinkFailed => "event_sink_failed",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }
}

/// Max bytes of the error detail folded into the fingerprint. Truncating keeps
/// signatures stable when a message carries a variable tail (ids, offsets)
/// while still distinguishing genuinely different failure messages.
pub const FINGERPRINT_DETAIL_TRUNCATE_BYTES: usize = 96;

/// A stable, opaque signature for a class of failure. `Display`/`as_str`
/// yields the hex digest so it can be logged and carried in
/// `SwarmError::BreakerOpen`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FailureFingerprint(String);

impl FailureFingerprint {
    /// Compute the fingerprint from an error class and free-form detail.
    /// The detail is truncated on a char boundary before hashing so a
    /// high-cardinality tail does not explode the signature space.
    pub fn compute(class: SwarmErrorClass, detail: &str) -> Self {
        let mut truncated = detail;
        if truncated.len() > FINGERPRINT_DETAIL_TRUNCATE_BYTES {
            // Find the largest char boundary <= the byte budget.
            let mut end = FINGERPRINT_DETAIL_TRUNCATE_BYTES;
            while end > 0 && !detail.is_char_boundary(end) {
                end -= 1;
            }
            truncated = &detail[..end];
        }
        let mut hasher = Sha256::new();
        hasher.update(class.as_str().as_bytes());
        hasher.update([0u8]); // domain separator between class and detail
        hasher.update(truncated.as_bytes());
        let digest = hasher.finalize();
        // 16 hex chars (64 bits) is ample to avoid collisions across the
        // handful of distinct failure classes a run will produce.
        let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        Self(hex)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FailureFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
