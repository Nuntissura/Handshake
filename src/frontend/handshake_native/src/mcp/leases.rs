//! Per-resource leasing for the swarm action channel (WP-KERNEL-011 MT-028).
//!
//! MT-027 made the MCP tool surface dispatchable out-of-process over TCP / a named pipe. Once N agents
//! connect and steer CONCURRENTLY (while the operator co-works in the UI), two agents can race to drive
//! the SAME widget — e.g. both `set_value` the same text field, or both `click_widget` the same button —
//! producing a torn / unattributable outcome. [`LeaseRegistry`] is the gate that serializes access to a
//! named resource: an agent must hold the resource's lease before its action is enqueued.
//!
//! ## Why a synchronous registry (NOT `tokio::sync::Mutex` / `RwLock`)
//!
//! The MT-028 contract body sketched a `tokio::sync::Semaphore`-per-resource design and an async-mutex
//! registry. The CRATE AS BUILT does not match that sketch: MT-027's [`crate::mcp::server`] dispatches
//! every request through a SYNCHRONOUS `dispatch_locked` that takes the shared `std::sync::Mutex`
//! snapshot + channel locks for the minimum span and releases them BEFORE any `.await` (the deliberate
//! "never hold a lock across await" design the MT-027 module docs call out). Threading a
//! `tokio::sync::Mutex` lease through that synchronous dispatch would force either holding an async lock
//! across the sync tool call (impossible) or rewriting MT-026/027's whole lock model (out of scope, and
//! a regression risk for the proven steering semantics). So the registry is a plain `std::sync::Mutex`
//! over a `HashMap<String, ResourceState>`, and lease acquisition is a BOUNDED, NON-BLOCKING retry: it
//! tries to take the slot, and if the resource is busy it sleeps a tiny fixed quantum and retries until
//! the `timeout` elapses, then returns [`LeaseError::Timeout`]. No `.await` is held across a lock; the
//! whole call is `fn`, so it composes with the synchronous dispatch and the `std::sync` shared state.
//!
//! ## Lease kinds
//!
//! - [`LeaseKind::Exclusive`] — a widget mutation (`click_widget`, `set_value`) or a pane-layout change.
//!   At most one holder; any other exclusive OR shared acquire waits.
//! - [`LeaseKind::Shared`] — a read (`list_widgets`). Any number of shared holders may coexist, but a
//!   shared lease waits while an exclusive lease is held (and vice-versa). This is the reader/writer
//!   discipline the contract's RwLock note wanted, implemented over the synchronous registry.
//!
//! ## Deadlock avoidance (red-team ABBA)
//!
//! The contract's minimum control mandates a GLOBAL LEASE ORDERING: when one operation needs more than
//! one lease, acquire them in ascending key order. [`LeaseRegistry::acquire_all`] enforces this by
//! sorting the requested keys before acquiring, so two operations requesting the same set can never take
//! them in opposite orders — the classic ABBA deadlock is eliminated by construction. The per-acquire
//! `timeout` is a SECONDARY backstop (a buggy single-lease holder that never releases still frees its
//! waiters after the timeout instead of hanging forever).
//!
//! ## Release on drop (incl. panic)
//!
//! [`LeaseGuard`] releases its slot in `Drop`, so a holder that panics (or whose owning tokio task is
//! dropped) frees the resource for the next acquirer — the contract's starvation control. The drop path
//! never panics and recovers a poisoned registry lock, so one agent's panic cannot wedge the registry.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The kind of access a lease grants on a named resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseKind {
    /// Multiple concurrent holders allowed (a read, e.g. `list_widgets`). Blocked only by an exclusive
    /// holder.
    Shared,
    /// At most one holder (a mutation: `click_widget` / `set_value` / a pane-layout change). Blocks all
    /// other acquirers — shared or exclusive — until released.
    Exclusive,
}

/// A typed failure from the lease registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseError {
    /// The lease could not be acquired within the requested timeout (the resource stayed busy). Maps to
    /// JSON-RPC [`crate::mcp::tools::ERR_LEASE_TIMEOUT`] in the tool layer.
    Timeout {
        /// The resource key that stayed busy.
        resource: String,
        /// The kind of lease that was requested.
        kind: LeaseKind,
    },
}

impl std::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaseError::Timeout { resource, kind } => {
                write!(f, "lease timeout acquiring {kind:?} lease on '{resource}'")
            }
        }
    }
}

impl std::error::Error for LeaseError {}

/// The live access state of one named resource. `shared_holders` counts active [`LeaseKind::Shared`]
/// guards; `exclusive` is true while a single [`LeaseKind::Exclusive`] guard is held. The two are
/// mutually exclusive: an exclusive grant requires `shared_holders == 0 && !exclusive`; a shared grant
/// requires `!exclusive`.
#[derive(Debug, Default)]
struct ResourceState {
    shared_holders: usize,
    exclusive: bool,
}

impl ResourceState {
    /// Can a NEW lease of `kind` be granted right now?
    fn can_grant(&self, kind: LeaseKind) -> bool {
        match kind {
            LeaseKind::Shared => !self.exclusive,
            LeaseKind::Exclusive => !self.exclusive && self.shared_holders == 0,
        }
    }

    /// Record that a lease of `kind` was just granted.
    fn grant(&mut self, kind: LeaseKind) {
        match kind {
            LeaseKind::Shared => self.shared_holders += 1,
            LeaseKind::Exclusive => self.exclusive = true,
        }
    }

    /// Record that a held lease of `kind` was just released.
    fn release(&mut self, kind: LeaseKind) {
        match kind {
            LeaseKind::Shared => self.shared_holders = self.shared_holders.saturating_sub(1),
            LeaseKind::Exclusive => self.exclusive = false,
        }
    }

    /// True when no one holds this resource (so its map entry can be reclaimed).
    fn is_idle(&self) -> bool {
        self.shared_holders == 0 && !self.exclusive
    }
}

/// The default per-acquire timeout for a lease in the live shell. Generous enough that a normal
/// short-lived mutation (enqueue one action, then drop the guard) never times out a contending agent,
/// small enough that a wedged holder frees its waiters promptly. The concurrent test overrides this with
/// a short value to exercise the timeout path deterministically.
pub const DEFAULT_LEASE_TIMEOUT: Duration = Duration::from_millis(500);

/// How long [`LeaseRegistry::try_acquire`] sleeps between retries while a resource is busy. Small enough
/// that contention resolves quickly once the holder drops, large enough not to busy-spin the CPU.
const RETRY_QUANTUM: Duration = Duration::from_millis(1);

/// A registry of per-resource leases. Cloneable and `Send + Sync` (it is an `Arc` over the shared map),
/// so the same registry is shared by every [`crate::mcp::session::McpSession`] AND the egui frame loop.
///
/// Resources are addressed by stable string keys: a widget's `author_id` (e.g.
/// `"shell.chrome.theme-toggle"`) for widget-level leases, or a pane-region key (e.g. `"pane.region.0"`)
/// for layout leases. Unknown keys are fine — a resource entry is created lazily on first acquire and
/// reclaimed when it goes idle, so the map never grows unboundedly with one-shot keys.
#[derive(Clone, Default)]
pub struct LeaseRegistry {
    inner: Arc<Mutex<HashMap<String, ResourceState>>>,
}

impl LeaseRegistry {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to acquire a lease of `kind` on `resource`, retrying (with a tiny sleep) until it is granted
    /// or `timeout` elapses. Returns a [`LeaseGuard`] that releases the lease on drop, or
    /// [`LeaseError::Timeout`] if the resource stayed busy for the whole timeout.
    ///
    /// This is a synchronous, bounded wait — it never holds the registry lock across the sleep, so it
    /// composes with the synchronous MCP dispatch and never deadlocks the registry itself.
    pub fn try_acquire(
        &self,
        resource: &str,
        kind: LeaseKind,
        timeout: Duration,
    ) -> Result<LeaseGuard, LeaseError> {
        let deadline = Instant::now() + timeout;
        loop {
            if self.try_grant_once(resource, kind) {
                return Ok(LeaseGuard {
                    registry: self.clone(),
                    resource: resource.to_owned(),
                    kind,
                });
            }
            if Instant::now() >= deadline {
                return Err(LeaseError::Timeout {
                    resource: resource.to_owned(),
                    kind,
                });
            }
            std::thread::sleep(RETRY_QUANTUM);
        }
    }

    /// Async, non-blocking sibling of [`Self::try_acquire`] for callers on a tokio runtime (the MCP
    /// server connection task). Identical bounded-retry semantics, but the inter-retry wait is
    /// `tokio::time::sleep` instead of `std::thread::sleep`, so a contending agent YIELDS its tokio worker
    /// thread while it waits rather than BLOCKING it (red-team: blocking a worker stalls every other
    /// connection task scheduled on that thread). The registry lock itself is only ever held for the
    /// `try_grant_once` span (no `.await` across it), so this composes with the synchronous registry.
    pub async fn acquire_async(
        &self,
        resource: &str,
        kind: LeaseKind,
        timeout: Duration,
    ) -> Result<LeaseGuard, LeaseError> {
        let deadline = Instant::now() + timeout;
        loop {
            if self.try_grant_once(resource, kind) {
                return Ok(LeaseGuard {
                    registry: self.clone(),
                    resource: resource.to_owned(),
                    kind,
                });
            }
            if Instant::now() >= deadline {
                return Err(LeaseError::Timeout {
                    resource: resource.to_owned(),
                    kind,
                });
            }
            tokio::time::sleep(RETRY_QUANTUM).await;
        }
    }

    /// Acquire leases on MANY resources atomically-ish, in ascending key order (the global lease ordering
    /// that eliminates ABBA deadlock). Either all are granted (returns the guards) or none are (any
    /// partial grants are released and the timeout error for the first contended key is returned).
    ///
    /// Duplicate keys are de-duplicated so a caller cannot self-deadlock by requesting the same key
    /// twice with conflicting kinds; the strongest requested kind for a key wins (Exclusive > Shared).
    pub fn acquire_all(
        &self,
        resources: &[(&str, LeaseKind)],
        timeout: Duration,
    ) -> Result<Vec<LeaseGuard>, LeaseError> {
        // De-duplicate by key (Exclusive wins over Shared) and sort ascending — the deadlock-free order.
        let mut wanted: HashMap<&str, LeaseKind> = HashMap::new();
        for &(key, kind) in resources {
            wanted
                .entry(key)
                .and_modify(|existing| {
                    if kind == LeaseKind::Exclusive {
                        *existing = LeaseKind::Exclusive;
                    }
                })
                .or_insert(kind);
        }
        let mut ordered: Vec<(&str, LeaseKind)> = wanted.into_iter().collect();
        ordered.sort_by(|a, b| a.0.cmp(b.0));

        let mut guards = Vec::with_capacity(ordered.len());
        for (key, kind) in ordered {
            match self.try_acquire(key, kind, timeout) {
                Ok(guard) => guards.push(guard),
                Err(e) => {
                    // Drop already-acquired guards (their Drop releases) before returning.
                    drop(guards);
                    return Err(e);
                }
            }
        }
        Ok(guards)
    }

    /// Single attempt to grant a lease, taking the registry lock for the minimum span. Returns true if
    /// granted. A poisoned lock (a prior holder panicked WHILE holding the registry lock — not while
    /// holding a lease) is recovered so one panic cannot wedge the whole registry.
    fn try_grant_once(&self, resource: &str, kind: LeaseKind) -> bool {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let state = map.entry(resource.to_owned()).or_default();
        if state.can_grant(kind) {
            state.grant(kind);
            true
        } else {
            false
        }
    }

    /// Release a held lease (called from [`LeaseGuard::drop`]). Reclaims the map entry once the resource
    /// goes idle so one-shot keys do not accumulate. Never panics (drop-safe), recovers a poisoned lock.
    fn release(&self, resource: &str, kind: LeaseKind) {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(state) = map.get_mut(resource) {
            state.release(kind);
            if state.is_idle() {
                map.remove(resource);
            }
        }
    }

    /// Diagnostic: number of resources currently holding at least one lease. Used by the concurrency
    /// harness to assert no exclusive holder leaked after a run, and available to a live diagnostic
    /// surface that wants to surface current lease pressure.
    pub fn active_resource_count(&self) -> usize {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).len()
    }
}

/// An acquired lease. Holds its slot in the [`LeaseRegistry`] until dropped; `Drop` releases it so a
/// panicking / disconnecting holder frees the resource (the starvation control). `#[must_use]` so a
/// caller cannot accidentally acquire-and-immediately-drop a lease meant to span an operation.
#[must_use = "a LeaseGuard releases its lease on drop; bind it for the span the resource must be held"]
pub struct LeaseGuard {
    registry: LeaseRegistry,
    resource: String,
    kind: LeaseKind,
}

impl LeaseGuard {
    /// The resource key this guard holds.
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// The kind of lease this guard holds.
    pub fn kind(&self) -> LeaseKind {
        self.kind
    }
}

impl std::fmt::Debug for LeaseGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeaseGuard")
            .field("resource", &self.resource)
            .field("kind", &self.kind)
            .finish()
    }
}

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        self.registry.release(&self.resource, self.kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_lease_blocks_second_exclusive_then_releases() {
        let reg = LeaseRegistry::new();
        let g1 = reg
            .try_acquire("w", LeaseKind::Exclusive, Duration::from_millis(20))
            .expect("first exclusive granted");
        // Second exclusive on the same key times out while the first is held.
        let err = reg
            .try_acquire("w", LeaseKind::Exclusive, Duration::from_millis(20))
            .unwrap_err();
        assert_eq!(
            err,
            LeaseError::Timeout { resource: "w".to_owned(), kind: LeaseKind::Exclusive }
        );
        drop(g1);
        // After release the key is reclaimed and a new exclusive succeeds.
        let _g2 = reg
            .try_acquire("w", LeaseKind::Exclusive, Duration::from_millis(20))
            .expect("exclusive granted after release");
    }

    #[test]
    fn shared_leases_coexist_but_block_exclusive() {
        let reg = LeaseRegistry::new();
        let _s1 = reg
            .try_acquire("r", LeaseKind::Shared, Duration::from_millis(20))
            .expect("first shared granted");
        let _s2 = reg
            .try_acquire("r", LeaseKind::Shared, Duration::from_millis(20))
            .expect("second shared coexists");
        // An exclusive waits while shared holders exist -> times out.
        let err = reg
            .try_acquire("r", LeaseKind::Exclusive, Duration::from_millis(20))
            .unwrap_err();
        assert!(matches!(err, LeaseError::Timeout { .. }));
    }

    #[test]
    fn different_keys_do_not_contend() {
        let reg = LeaseRegistry::new();
        let _a = reg
            .try_acquire("a", LeaseKind::Exclusive, Duration::from_millis(20))
            .expect("a granted");
        // A different key is independent.
        let _b = reg
            .try_acquire("b", LeaseKind::Exclusive, Duration::from_millis(20))
            .expect("b granted (different key)");
        assert_eq!(reg.active_resource_count(), 2);
    }

    #[test]
    fn idle_resource_entry_is_reclaimed() {
        let reg = LeaseRegistry::new();
        {
            let _g = reg
                .try_acquire("temp", LeaseKind::Exclusive, Duration::from_millis(20))
                .expect("granted");
            assert_eq!(reg.active_resource_count(), 1);
        }
        assert_eq!(reg.active_resource_count(), 0, "idle entry reclaimed on drop");
    }

    #[test]
    fn lease_released_on_panic() {
        // Red-team minimum control: a holder that panics frees the lease for the next acquirer.
        let reg = LeaseRegistry::new();
        let reg2 = reg.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = reg2
                .try_acquire("p", LeaseKind::Exclusive, Duration::from_millis(20))
                .expect("granted");
            panic!("holder panics while holding the lease");
        }));
        assert!(result.is_err(), "the inner closure panicked");
        // The guard's Drop ran during unwind, so the lease is free again.
        let _g = reg
            .try_acquire("p", LeaseKind::Exclusive, Duration::from_millis(50))
            .expect("lease available after the holder panicked");
    }

    #[test]
    fn acquire_all_is_order_independent_no_deadlock() {
        use std::thread;
        // Two threads request {a,b} in OPPOSITE textual order; acquire_all sorts both to a<b, so they
        // can never take them in opposite order -> no ABBA deadlock. Both eventually succeed.
        let reg = LeaseRegistry::new();
        let r1 = reg.clone();
        let r2 = reg.clone();
        let t1 = thread::spawn(move || {
            for _ in 0..50 {
                let guards = r1
                    .acquire_all(
                        &[("a", LeaseKind::Exclusive), ("b", LeaseKind::Exclusive)],
                        Duration::from_millis(500),
                    )
                    .expect("t1 acquires a+b");
                drop(guards);
            }
        });
        let t2 = thread::spawn(move || {
            for _ in 0..50 {
                let guards = r2
                    .acquire_all(
                        &[("b", LeaseKind::Exclusive), ("a", LeaseKind::Exclusive)],
                        Duration::from_millis(500),
                    )
                    .expect("t2 acquires b+a (same sorted order internally)");
                drop(guards);
            }
        });
        t1.join().expect("t1 finished without deadlock");
        t2.join().expect("t2 finished without deadlock");
        assert_eq!(reg.active_resource_count(), 0, "all leases released");
    }

    #[test]
    fn acquire_all_dedups_conflicting_kinds_exclusive_wins() {
        let reg = LeaseRegistry::new();
        // Same key requested Shared AND Exclusive -> de-duped to one Exclusive (no self-deadlock).
        let guards = reg
            .acquire_all(
                &[("k", LeaseKind::Shared), ("k", LeaseKind::Exclusive)],
                Duration::from_millis(50),
            )
            .expect("dedup grants a single exclusive lease");
        assert_eq!(guards.len(), 1);
        assert_eq!(guards[0].kind(), LeaseKind::Exclusive);
    }
}
