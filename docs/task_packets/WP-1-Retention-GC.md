# Task Packet: WP-1-Retention-GC

## Metadata
- TASK_ID: WP-1-Retention-GC
- DATE: 2025-12-25T20:13:00Z
- REQUESTOR: ilja
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- STATUS: Completed ✅
- RISK_TIER: MEDIUM
  - Justification: Involves automated deletion of data; requires strict adherence to safety policies to prevent accidental data loss.
- USER_SIGNATURE: ilja251220252013

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check `storage/retention.rs`.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§2.3.11 Data Retention and GC).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## USER_CONTEXT (Non-Technical Explainer) [CX-654]
As the system runs, it creates thousands of temporary logs, old versions of documents, and records of AI "thoughts" that we don't need to keep forever. If we keep everything, the database will eventually become slow and expensive. This "Janitor" service automatically cleans up old, unnecessary data based on rules we set (e.g., "delete logs older than 30 days"), keeping the system lean and fast.

---

## SCOPE

### Executive Summary
Implement the `Janitor` service and `RetentionPolicy` logic to prune old logs, AI jobs, and derived artifacts. This prevents database bloat and ensures system performance as mandated by §2.3.11.

### IN_SCOPE_PATHS
- src/backend/handshake_core/src/storage/retention.rs (New implementation)
- src/backend/handshake_core/src/storage/mod.rs (Trait extension if needed)
- src/backend/handshake_core/src/main.rs (Service startup)

### OUT_OF_SCOPE
- UI for configuring retention days.
- Multi-tenant data isolation policies.
- S3/Blob storage cleanup (Local storage only for Phase 1).

---

## QUALITY GATE

- **TEST_PLAN**:
  ```bash
  # Core unit tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml retention
  
  # Full validation
  just gate-check WP-1-Retention-GC
  just post-work WP-1-Retention-GC
  ```
- **DONE_MEANS**:
  - ✅ `RetentionPolicy` struct implemented with configurable TTLs.
  - ✅ `Janitor` background loop implemented (runs every 1 hour or on startup).
  - ✅ Pruning logic removes: AI jobs older than policy, logs older than policy.
  - ✅ "Dry Run" mode supported via configuration.
  - ✅ Unit tests verify that data *outside* TTL is deleted and data *inside* is preserved.
  - ✅ Evidence mapping block is complete.

- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  # Manual steps:
  # 1. Kill the background Janitor task in main.rs
  # 2. Remove src/backend/handshake_core/src/storage/retention.rs
  ```

---

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.84.md (§2.3.11)
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/main.rs
  * src/backend/handshake_core/src/storage/sqlite.rs

- **SEARCH_TERMS**:
  * "created_at"
  * "TIMESTAMP"
  * "sqlx::query"
  * "tokio::spawn"
  * "RetentionPolicy"

- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gate-check WP-1-Retention-GC
  ```

- **RISK_MAP**:
  * "Accidental over-deletion" -> Testing (Use dry-run verification)
  * "Database locking" -> Resource Management (Use small batch sizes for deletes)
  * "Service failure" -> Stability (Ensure janitor crash doesn't kill the app)

---

## EVIDENCE_MAPPING

| Requirement | File:Line | Notes |
|-------------|-----------|-------|
| [HSK-GC-001] RetentionPolicy struct | retention.rs:44-55 | Matches spec: kind, window_days, min_versions |
| [HSK-GC-001] ArtifactKind enum | retention.rs:21-35 | Log, Result, Evidence, Cache, Checkpoint |
| [HSK-GC-001] PruneReport struct | retention.rs:77-86 | All required fields per spec |
| [HSK-GC-002] Pinning Invariant | retention.rs:229-235, 314 | `is_pinned = 0` checked in all delete queries |
| [HSK-GC-003] Audit Trail | retention.rs:341-362 | Emits `meta.gc_summary` to Flight Recorder |
| [HSK-GC-004] Atomic Materialize | retention.rs:213-338 | PruneReport written via Flight Recorder before deletions |
| Janitor background loop | retention.rs:367-398 | tokio::spawn with interval timer |
| Dry-run mode | retention.rs:298-304 | Checks config.dry_run, logs but doesn't delete |
| Batched deletion | retention.rs:305-329 | Uses LIMIT clause per batch_size config |
| min_versions constraint | retention.rs:261-272 | Respects min_versions before deletion |
| Environment config | main.rs:82-119 | JANITOR_DRY_RUN, JANITOR_INTERVAL_SECS, JANITOR_RETENTION_DAYS |
| Service startup | main.rs:54-58 | Janitor spawned in run() after storage init |

---

## VALIDATION [CX-623]

**Executed: 2025-12-25**

### Tests

| Command | Result | Notes |
|---------|--------|-------|
| `cargo test --lib retention` | ✅ PASS (5 tests) | All retention tests pass |
| `just gate-check WP-1-Retention-GC` | ✅ PASS | Workflow sequence verified |
| `just post-work WP-1-Retention-GC` | ✅ PASS | Post-work validation passed |
| `cargo check` | ✅ PASS | Code compiles (1 unrelated warning in tokenization.rs) |

### Test Coverage

- `test_prune_respects_pinned_items` - Verifies [HSK-GC-002] pinning invariant
- `test_prune_respects_window` - Verifies TTL window respected
- `test_dry_run_does_not_delete` - Verifies dry-run safety
- `test_min_versions_constraint` - Verifies min_versions preserved
- `test_flight_recorder_event_emitted` - Verifies [HSK-GC-003] audit trail

### Files Changed

- `src/backend/handshake_core/src/storage/retention.rs` (NEW - 650 lines)
- `src/backend/handshake_core/src/storage/mod.rs` (1 line added)
- `src/backend/handshake_core/src/main.rs` (45 lines added)
- `src/backend/handshake_core/migrations/0003_add_is_pinned.sql` (NEW - 6 lines)
- `src/backend/handshake_core/Cargo.toml` (2 lines added - tempfile dev dep)
- `src/backend/handshake_core/.sqlx/*.json` (query cache regenerated)

---

## AUTHORITY
- **SPEC_ANCHOR**: §2.3.11 (Data Retention and GC)
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220252013

## VALIDATION REPORT — WP-1-Retention-GC
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Retention-GC.md (status: In-Progress)
- Spec: Handshake_Master_Spec_v02.84.md (§2.3.11)

Files Checked:
- src/backend/handshake_core/src/storage/retention.rs
- src/backend/handshake_core/src/main.rs
- src/backend/handshake_core/migrations/0003_add_is_pinned.sql

Findings:
- **Correctness**: Janitor service correctly implements automated pruning with TTL window and pinning support.
- **Spec Alignment**: All invariants ([HSK-GC-001] to [HSK-GC-004]) are implemented in `retention.rs`.
- **Pinning**: `is_pinned` column added to schema and respected in logic.
- **Audit**: `meta.gc_summary` event emitted to Flight Recorder with structured payload.
- **Batched Deletion**: implemented to prevent DB locking.
- **Safety**: Dry-run mode verified via tests.

Tests:
- `cargo test ... retention` -> PASS (5 tests)

Risks & Suggested Actions:
- **Risk**: Automated deletion is always risky; users should be advised to set `JANITOR_DRY_RUN=true` in production initially to verify behavior.
- **Action**: Monitor `flight_recorder.db` for `meta.gc_summary` events to confirm cleanup efficiency.

REASON FOR PASS:
The implementation is robust, well-tested, and adheres strictly to the architectural invariants for garbage collection and data safety.
