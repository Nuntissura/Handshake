[[ coder gemini]]

SKELETON Phase [CX-GATE-001]
  ========================================

  I have analyzed the task and the codebase. I propose the following skeleton changes to remove
  concrete pool leakages and enforce the Trait Purity Invariant [CX-DBP-040].

  1. src/backend/handshake_core/src/storage/mod.rs
  Move PruneReport and ArtifactKind from retention.rs to mod.rs to make them part of the core      
  storage contract.
  Update Database trait:
   - Add: async fn prune_ai_jobs(&self, cutoff: DateTime<Utc>, min_versions: u32, dry_run: bool) ->
     StorageResult<PruneReport>;
   - Remove: fn sqlite_pool(&self) -> Option<&SqlitePool>;

  2. src/backend/handshake_core/src/lib.rs
   - Refactor: Remove pub fr_pool: Arc<Mutex<DuckDbConnection>> from AppState. All interactions
     with Flight Recorder must go through the FlightRecorder trait.

  3. src/backend/handshake_core/src/storage/retention.rs
   - Refactor: Janitor struct to use storage: Arc<dyn Database> instead of pool: SqlitePool.
   - Refactor: Janitor::prune_ai_jobs to delegate the database-specific logic to
     self.storage.prune_ai_jobs.

  4. src/backend/handshake_core/src/storage/sqlite.rs & postgres.rs
   - Implement: prune_ai_jobs for SQLite (moving the logic from Janitor).
   - Implement: prune_ai_jobs for Postgres (stub for now).
   - Remove: sqlite_pool implementation.

  5. src/backend/handshake_core/src/main.rs
   - Refactor: init_storage to return Result<Arc<dyn Database>, ...>.
   - Refactor: init_flight_recorder to return Result<Arc<dyn FlightRecorder>, ...>.
   - Update: AppState and Janitor initialization to reflect the removal of leaked pools.

  STOP and wait for "SKELETON APPROVED".
  ========================================

[[/coder gemini]]