//! Per-registry keyed async locks: an OPTIONAL in-process contention shaper
//! for embedded SurrealDB writers (MT-142 Lane A1, AC-142-8 / AC-142-10).
//!
//! Correctness never depends on these locks. The atomic `BEGIN ... COMMIT`
//! query string and its in-transaction guards (compare-and-set, `THROW`
//! codes, `UNIQUE` indexes, idempotency receipts) own correctness; RocksDB
//! optimistic transactions detect every write-write race at commit
//! (research basis
//! `Handshake_Artifacts/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-142/kb01/research/research_basis.json`,
//! `selected_design.principle` and `selected_design.keyed_lock_registry`;
//! `surrealdb-core-3.2.0/src/kvs/rocksdb/mod.rs:462-470,2133-2138`). A
//! registry in [`LockMode::Disabled`] hands out no-op guards so the
//! independent-client proof runs the same store code with the database
//! transactions alone.
//!
//! Ownership: NOT a process-global. One registry per owner; the integrating
//! lane attaches one to each `SurrealDatabase` value so two wrappers over one
//! engine can hold separate registries (recon
//! `Handshake_Artifacts/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-142/kb01/recon/lock_inventory.json`,
//! `client_handle_model.implication_for_keyed_lock_registry`, open question
//! Q3). Clones of a registry share its map.
//!
//! Why hand-rolled: `lockable`, `key-mutex` and `dashmap` are not direct
//! dependencies (`Cargo.toml` `[dependencies]`; research basis
//! `rejected_alternatives`). The `Weak`-reclamation map follows the key-mutex
//! 0.1.3 auto-deallocation pattern and its idle bound (zero entries once every
//! guard is dropped) is directly provable.
//!
//! Deadlock freedom: [`KeyedLockRegistry::acquire_many`] sorts and dedups keys
//! before acquiring, so two callers holding overlapping key sets in opposite
//! input order cannot wait on each other (lockable 0.2.0 warns that arbitrary
//! acquisition order "can easily lead to deadlocks"; research basis
//! `selected_design.keyed_lock_registry.multi_key`). Callers must not nest
//! separate `acquire` calls in arbitrary order.
//!
//! Never call this from a read-only path: reads are served from the
//! transaction snapshot and gain nothing from serialization (research basis
//! `selected_design.keyed_lock_registry.key_taxonomy`). Nothing in this module
//! touches the database.
//!
//! Visibility: `pub` because the MT-142 `tests/` swarm target (validation plan
//! `lock_registry_tests`, `how_independent_clients_are_modelled`) constructs
//! registries and asserts the idle bound from outside the crate.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

/// Lock identity. `Ord` gives the deterministic acquisition order used by
/// [`KeyedLockRegistry::acquire_many`] (variant order, then field order).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LockKey {
    /// One record mutation.
    Record { table: &'static str, id: String },
    /// A uniqueness / upsert race on a natural key discovered before the
    /// record id exists.
    NaturalKey {
        workspace_id: String,
        kind: &'static str,
        key: String,
    },
    /// Only for invariants that genuinely span a workspace.
    Workspace { workspace_id: String },
}

impl LockKey {
    pub fn record(table: &'static str, id: impl Into<String>) -> Self {
        Self::Record {
            table,
            id: id.into(),
        }
    }

    pub fn natural_key(
        workspace_id: impl Into<String>,
        kind: &'static str,
        key: impl Into<String>,
    ) -> Self {
        Self::NaturalKey {
            workspace_id: workspace_id.into(),
            kind,
            key: key.into(),
        }
    }

    pub fn workspace(workspace_id: impl Into<String>) -> Self {
        Self::Workspace {
            workspace_id: workspace_id.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    /// Same key serialises; different keys proceed concurrently.
    Keyed,
    /// Every acquire returns immediately with a no-op guard.
    Disabled,
}

/// Returned instead of hanging when a deadline passes while waiting.
#[derive(Debug, Error)]
#[error("keyed lock wait timed out after {} ms for {key:?}", waited.as_millis())]
pub struct LockWaitTimeout {
    pub key: LockKey,
    pub waited: Duration,
}

type LockCell = Arc<AsyncMutex<()>>;

#[derive(Debug)]
struct RegistryInner {
    mode: LockMode,
    entries: StdMutex<HashMap<LockKey, Weak<AsyncMutex<()>>>>,
}

impl RegistryInner {
    fn lock_entries(&self) -> MutexGuard<'_, HashMap<LockKey, Weak<AsyncMutex<()>>>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Upgrades the existing cell or inserts a fresh one.
    fn cell_for(&self, key: &LockKey) -> LockCell {
        let mut entries = self.lock_entries();
        if let Some(cell) = entries.get(key).and_then(Weak::upgrade) {
            return cell;
        }
        let cell = Arc::new(AsyncMutex::new(()));
        entries.insert(key.clone(), Arc::downgrade(&cell));
        cell
    }

    /// Removes the entry when `cell` is the map's cell and this is its last
    /// strong holder; a concurrent acquirer that upgraded first keeps it.
    fn reclaim(&self, key: &LockKey, cell: LockCell) {
        let mut entries = self.lock_entries();
        let last_holder = entries
            .get(key)
            .is_some_and(|weak| Weak::ptr_eq(weak, &Arc::downgrade(&cell)) && Arc::strong_count(&cell) == 1);
        if last_holder {
            entries.remove(key);
        }
        drop(entries);
        drop(cell);
    }
}

/// Registry of keyed locks; `Clone` shares the underlying map.
#[derive(Clone, Debug)]
pub struct KeyedLockRegistry {
    inner: Arc<RegistryInner>,
}

impl KeyedLockRegistry {
    pub fn new(mode: LockMode) -> Self {
        Self {
            inner: Arc::new(RegistryInner {
                mode,
                entries: StdMutex::new(HashMap::new()),
            }),
        }
    }

    pub fn keyed() -> Self {
        Self::new(LockMode::Keyed)
    }

    pub fn disabled() -> Self {
        Self::new(LockMode::Disabled)
    }

    pub fn mode(&self) -> LockMode {
        self.inner.mode
    }

    /// Number of map entries (held or awaited). Documented idle bound: 0.
    pub fn entry_count(&self) -> usize {
        self.inner.lock_entries().len()
    }

    /// Entries whose cell no longer has a strong holder. Documented bound at
    /// all times: 0 (every holder reclaims on drop; AC-142-10).
    pub fn idle_entry_count(&self) -> usize {
        self.inner
            .lock_entries()
            .values()
            .filter(|weak| weak.strong_count() == 0)
            .count()
    }

    /// Waits without bound. Cancel-safe: dropping the future reclaims the entry.
    pub async fn acquire(&self, key: LockKey) -> KeyedLockGuard {
        let Some((mut held, cell)) = self.begin(key) else {
            return KeyedLockGuard::noop();
        };
        let started = Instant::now();
        let permit = cell.lock_owned().await;
        held.permit = Some(permit);
        KeyedLockGuard::held(held, started.elapsed())
    }

    /// Like [`Self::acquire`] but returns [`LockWaitTimeout`] once `deadline`
    /// passes instead of hanging.
    pub async fn acquire_with_deadline(
        &self,
        key: LockKey,
        deadline: Option<Instant>,
    ) -> Result<KeyedLockGuard, LockWaitTimeout> {
        let Some(deadline) = deadline else {
            return Ok(self.acquire(key).await);
        };
        let Some((mut held, cell)) = self.begin(key) else {
            return Ok(KeyedLockGuard::noop());
        };
        let started = Instant::now();
        // Bind the outcome first: the timed-out `lock_owned` future (and its
        // strong reference) is dropped by the `await`, so `held` is the last
        // holder when it reclaims the entry on return.
        let outcome = tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            cell.lock_owned(),
        )
        .await;
        match outcome {
            Ok(permit) => {
                held.permit = Some(permit);
                Ok(KeyedLockGuard::held(held, started.elapsed()))
            }
            Err(_elapsed) => Err(LockWaitTimeout {
                key: held.key.clone(),
                waited: started.elapsed(),
            }),
        }
    }

    /// Acquires every distinct key in ascending [`LockKey`] order.
    pub async fn acquire_many(&self, keys: Vec<LockKey>) -> Vec<KeyedLockGuard> {
        let mut guards = Vec::new();
        for key in ordered_unique(keys) {
            guards.push(self.acquire(key).await);
        }
        guards
    }

    /// [`Self::acquire_many`] with a shared deadline; on timeout every guard
    /// acquired so far is released before the error is returned.
    pub async fn acquire_many_with_deadline(
        &self,
        keys: Vec<LockKey>,
        deadline: Option<Instant>,
    ) -> Result<Vec<KeyedLockGuard>, LockWaitTimeout> {
        let mut guards = Vec::new();
        for key in ordered_unique(keys) {
            guards.push(self.acquire_with_deadline(key, deadline).await?);
        }
        Ok(guards)
    }

    /// Registers a strong holder for `key` and returns a second strong
    /// reference for `lock_owned`; `None` in [`LockMode::Disabled`].
    fn begin(&self, key: LockKey) -> Option<(HeldLock, LockCell)> {
        if self.inner.mode == LockMode::Disabled {
            return None;
        }
        let cell = self.inner.cell_for(&key);
        let held = HeldLock {
            key,
            permit: None,
            cell: Some(Arc::clone(&cell)),
            registry: Arc::clone(&self.inner),
        };
        Some((held, cell))
    }
}

fn ordered_unique(mut keys: Vec<LockKey>) -> Vec<LockKey> {
    keys.sort();
    keys.dedup();
    keys
}

/// Strong holder of a registry cell from the moment a key is looked up until
/// the guard drops; its `Drop` releases the permit first and then reclaims
/// the entry, on every path (success, timeout, cancelled acquire).
#[derive(Debug)]
struct HeldLock {
    key: LockKey,
    permit: Option<OwnedMutexGuard<()>>,
    cell: Option<LockCell>,
    registry: Arc<RegistryInner>,
}

impl Drop for HeldLock {
    fn drop(&mut self) {
        drop(self.permit.take());
        if let Some(cell) = self.cell.take() {
            self.registry.reclaim(&self.key, cell);
        }
    }
}

/// Holds one keyed lock (or nothing in [`LockMode::Disabled`]).
#[derive(Debug)]
pub struct KeyedLockGuard {
    held: Option<HeldLock>,
    lock_wait: Duration,
}

impl KeyedLockGuard {
    fn noop() -> Self {
        Self {
            held: None,
            lock_wait: Duration::ZERO,
        }
    }

    fn held(held: HeldLock, lock_wait: Duration) -> Self {
        Self {
            held: Some(held),
            lock_wait,
        }
    }

    /// Time spent waiting for the permit (zero for no-op guards); feeds the
    /// `lock_wait_ms_p50_p95_p99` report field.
    pub fn lock_wait(&self) -> Duration {
        self.lock_wait
    }

    pub fn key(&self) -> Option<&LockKey> {
        self.held.as_ref().map(|held| &held.key)
    }

    pub fn is_noop(&self) -> bool {
        self.held.is_none()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[tokio::test]
    async fn same_key_serialises_the_second_acquire() {
        let registry = KeyedLockRegistry::keyed();
        let key = LockKey::record("knowledge_rich_documents", "KRD-1");
        let first = registry.acquire(key.clone()).await;
        assert_eq!(first.key(), Some(&key));
        assert!(!first.is_noop());

        let blocked = timeout(ms(50), registry.acquire(key.clone())).await;
        assert!(blocked.is_err(), "second acquire must wait while the first is held");
        assert_eq!(registry.entry_count(), 1);

        drop(first);
        let second = timeout(Duration::from_secs(5), registry.acquire(key.clone()))
            .await
            .expect("acquires after release");
        assert_eq!(registry.entry_count(), 1);
        drop(second);
        assert_eq!(registry.entry_count(), 0);
        assert_eq!(registry.idle_entry_count(), 0);
    }

    #[tokio::test]
    async fn different_keys_are_held_simultaneously() {
        let registry = KeyedLockRegistry::keyed();
        let first = registry.acquire(LockKey::record("t", "1")).await;
        let second = timeout(ms(50), registry.acquire(LockKey::record("t", "2")))
            .await
            .expect("different key must not wait");
        let third = timeout(ms(50), registry.acquire(LockKey::workspace("ws")))
            .await
            .expect("different variant must not wait");
        assert_eq!(registry.entry_count(), 3);
        drop(first);
        drop(second);
        drop(third);
        assert_eq!(registry.entry_count(), 0);
    }

    #[tokio::test]
    async fn disabled_mode_never_waits_and_tracks_nothing() {
        let registry = KeyedLockRegistry::disabled();
        assert_eq!(registry.mode(), LockMode::Disabled);
        let key = LockKey::natural_key("ws", "title", "hello");
        let first = timeout(ms(50), registry.acquire(key.clone()))
            .await
            .expect("no-op guard");
        let second = timeout(ms(50), registry.acquire(key.clone()))
            .await
            .expect("no-op guard");
        let with_deadline = registry
            .acquire_with_deadline(key.clone(), Some(Instant::now() + ms(10)))
            .await
            .expect("no-op guard");
        assert!(first.is_noop() && second.is_noop() && with_deadline.is_noop());
        assert_eq!(first.lock_wait(), Duration::ZERO);
        assert!(first.key().is_none());
        assert_eq!(registry.entry_count(), 0);
    }

    #[tokio::test]
    async fn acquire_many_with_opposite_order_keys_does_not_deadlock() {
        let registry = KeyedLockRegistry::keyed();
        let a = LockKey::record("t", "a");
        let b = LockKey::record("t", "b");
        let forward = {
            let registry = registry.clone();
            let (a, b) = (a.clone(), b.clone());
            tokio::spawn(async move {
                for _ in 0..200 {
                    let guards = registry.acquire_many(vec![a.clone(), b.clone()]).await;
                    assert_eq!(guards.len(), 2);
                    tokio::task::yield_now().await;
                }
            })
        };
        let reverse = {
            let registry = registry.clone();
            let (a, b) = (a.clone(), b.clone());
            tokio::spawn(async move {
                for _ in 0..200 {
                    let guards = registry.acquire_many(vec![b.clone(), a.clone()]).await;
                    assert_eq!(guards.len(), 2);
                    tokio::task::yield_now().await;
                }
            })
        };
        let joined = timeout(Duration::from_secs(10), async {
            forward.await.expect("forward task");
            reverse.await.expect("reverse task");
        })
        .await;
        assert!(joined.is_ok(), "opposite-order acquire_many deadlocked");
        assert_eq!(registry.entry_count(), 0);
    }

    #[tokio::test]
    async fn acquire_many_sorts_and_dedups_keys() {
        let registry = KeyedLockRegistry::keyed();
        let guards = registry
            .acquire_many(vec![
                LockKey::workspace("ws"),
                LockKey::record("t", "b"),
                LockKey::record("t", "a"),
                LockKey::record("t", "a"),
            ])
            .await;
        let keys: Vec<&LockKey> = guards.iter().filter_map(KeyedLockGuard::key).collect();
        assert_eq!(
            keys,
            vec![
                &LockKey::record("t", "a"),
                &LockKey::record("t", "b"),
                &LockKey::workspace("ws"),
            ]
        );
        assert_eq!(registry.entry_count(), 3);
        drop(guards);
        assert_eq!(registry.entry_count(), 0);
    }

    #[tokio::test]
    async fn high_cardinality_churn_returns_the_registry_to_its_idle_bound() {
        let registry = KeyedLockRegistry::keyed();
        for i in 0..10_000u32 {
            let guard = registry.acquire(LockKey::record("churn", i.to_string())).await;
            assert!(!guard.is_noop());
        }
        assert_eq!(registry.entry_count(), 0);

        let mut workers = Vec::new();
        for worker in 0..4u32 {
            let registry = registry.clone();
            workers.push(tokio::spawn(async move {
                for i in 0..2_500u32 {
                    let key = LockKey::natural_key(
                        format!("ws-{}", i % 7),
                        "title",
                        format!("k-{}", (i + worker) % 500),
                    );
                    let guard = registry.acquire(key).await;
                    if i % 97 == 0 {
                        tokio::task::yield_now().await;
                    }
                    drop(guard);
                }
            }));
        }
        let joined = timeout(Duration::from_secs(30), async {
            for worker in workers {
                worker.await.expect("churn worker");
            }
        })
        .await;
        assert!(joined.is_ok(), "churn workers did not finish");
        assert_eq!(registry.entry_count(), 0);
        assert_eq!(registry.idle_entry_count(), 0);
    }

    #[tokio::test]
    async fn deadline_returns_typed_lock_wait_timeout() {
        let registry = KeyedLockRegistry::keyed();
        let key = LockKey::record("t", "held");
        let holder = registry.acquire(key.clone()).await;
        let error = registry
            .acquire_with_deadline(key.clone(), Some(Instant::now() + ms(20)))
            .await
            .expect_err("must time out while held");
        assert_eq!(error.key, key);
        assert!(error.waited >= ms(20), "waited {:?}", error.waited);
        assert!(error.to_string().contains("keyed lock wait timed out"));
        assert_eq!(registry.entry_count(), 1);
        drop(holder);
        assert_eq!(registry.entry_count(), 0);
        assert_eq!(registry.idle_entry_count(), 0);
    }

    #[tokio::test]
    async fn cancelled_acquire_reclaims_its_entry() {
        let registry = KeyedLockRegistry::keyed();
        let key = LockKey::record("t", "held");
        let holder = registry.acquire(key.clone()).await;
        let dropped = timeout(ms(20), registry.acquire(key.clone())).await;
        assert!(dropped.is_err());
        assert_eq!(registry.entry_count(), 1);
        drop(holder);
        assert_eq!(registry.entry_count(), 0);
        let fresh = registry.acquire(key).await;
        assert_eq!(registry.entry_count(), 1);
        drop(fresh);
        assert_eq!(registry.entry_count(), 0);
    }

    #[tokio::test]
    async fn guard_records_lock_wait() {
        let registry = KeyedLockRegistry::keyed();
        let key = LockKey::record("t", "waited");
        let holder = registry.acquire(key.clone()).await;
        tokio::spawn(async move {
            tokio::time::sleep(ms(30)).await;
            drop(holder);
        });
        let guard = timeout(Duration::from_secs(5), registry.acquire(key))
            .await
            .expect("acquires after the holder releases");
        assert!(guard.lock_wait() >= ms(15), "lock_wait {:?}", guard.lock_wait());
    }

    #[test]
    fn lock_key_order_is_deterministic() {
        let mut keys = vec![
            LockKey::workspace("ws"),
            LockKey::natural_key("ws", "title", "b"),
            LockKey::record("t", "2"),
            LockKey::natural_key("ws", "title", "a"),
            LockKey::record("s", "9"),
        ];
        keys.sort();
        assert_eq!(
            keys,
            vec![
                LockKey::record("s", "9"),
                LockKey::record("t", "2"),
                LockKey::natural_key("ws", "title", "a"),
                LockKey::natural_key("ws", "title", "b"),
                LockKey::workspace("ws"),
            ]
        );
    }
}
