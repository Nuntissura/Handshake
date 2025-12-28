# Task Packet: WP-1-Storage-Abstraction-Layer-v2

## Metadata
- TASK_ID: WP-1-Storage-Abstraction-Layer-v2
- WP_ID: WP-1-Storage-Abstraction-Layer-v2
- DATE: 2025-12-26T03:05:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator


## SKELETON APPROVED
## User Context
We are fixing the "plumbing" of the app. Currently, some parts of the system are talking directly to the SQLite database, which breaks our rule of being "database-portable." This task cleans up those connections so the app is ready for future growth without costly rewrites.

## Scope
- **What**: Remove all concrete pool leakage (`SqlitePool`, `DuckDbConnection`) from the `Database` trait and public `AppState`.
- **Why**: Enforce Pillar 1 (One Storage API) to satisfy Phase 1 closure requirements and comply with the Trait Purity Invariant [CX-DBP-040].
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/storage/mod.rs (Remove `sqlite_pool()` accessor)
  * src/backend/handshake_core/src/main.rs (Refactor `init_storage` to return only `Arc<dyn Database>`)
  * src/backend/handshake_core/src/storage/retention.rs (Refactor Janitor to use trait operations)
  * src/backend/handshake_core/src/lib.rs (Clean up `AppState` and metadata)
- **OUT_OF_SCOPE**:
  * Migration framework implementation (WP-1-Migration-Framework).
  * Backend-specific optimizations.

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Core architectural refactor; breaking change for initialization and maintenance services.
- **TEST_PLAN**:
  ```bash
  # Compile check
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  
  # Unit/Integration tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  
  # DAL Audit (Crucial check)
  just validator-dal-audit
  
  # Post-work closure
  just post-work WP-1-Storage-Abstraction-Layer-v2
  ```
- **DONE_MEANS**:
  * ??? `Database` trait contains zero backend-specific identifiers (No `sqlite_pool`).
  * ??? `init_storage` signature in `main.rs` returns only `Arc<dyn Database>` (no leaked pool).
  * ??? `Janitor` service consumes `Arc<dyn Database>` instead of `SqlitePool`.
  * ??? `just validator-dal-audit` returns PASS (Zero violations of CX-DBP-VAL-012).
  * ??? All existing tests pass using the generic trait interface.

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/main.rs
  * src/backend/handshake_core/src/storage/retention.rs
  * src/backend/handshake_core/src/lib.rs
- **SEARCH_TERMS**:
  * "sqlite_pool"
  * "Option<&SqlitePool>"
  * "init_storage"
  * "Janitor::new"
- **RUN_COMMANDS**:
  ```bash
  just validator-dal-audit
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Janitor breakage" -> Storage cleanup logic (Ensure trait has necessary prune methods)
  * "Boot failure" -> main.rs wiring (Ensure async trait object initialization is correct)
  * "Test regression" -> Existing tests might rely on pool exposure; refactor tests to use trait.

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.90)
- **SPEC_ANCHOR**: ??2.3.12.3 [CX-DBP-040], ??2.3.11.2 [HSK-GC-005]
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250259

## VALIDATION REPORT — 2025-12-27 (Revalidation, Spec v02.93)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Storage-Abstraction-Layer-v2.md (STATUS: Validated)
- Spec: Handshake_Master_Spec_v02.93 (A2.3.12 Trait Purity)
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/flight_recorder/duckdb.rs (portable epoch-based timestamp query; removed SQLite-only `strftime`)
- src/backend/handshake_core/src/storage/sqlite.rs (test-only pool accessor; prod surfaces stay trait-pure)
- src/backend/handshake_core/src/api/jobs.rs (tests import Database trait)
- src/backend/handshake_core/src/workflows.rs (tests import Database trait)
- src/backend/handshake_core/src/storage/tests.rs (tests import Database trait)

Findings:
- Spec alignment: DAL audit clean; storage abstraction remains backend-agnostic (no pool leakage in prod).
- Portability: Flight Recorder query uses `EXTRACT(EPOCH FROM timestamp)`; validator DAL audit passes.
- Forbidden Pattern Audit [CX-573E]: PASS (validator-scan).
- Zero Placeholder Policy [CX-573D]: PASS; no stubs.

Tests:
- `just validator-dal-audit` (PASS)
- `just validator-scan` (PASS)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` (PASS; warnings limited to unused imports/deprecated helper)

REASON FOR PASS: Storage abstraction and flight recorder queries are portable under Spec v02.93; DAL audit and full test suite succeeded.

## VALIDATION REPORT — 2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Storage-Abstraction-Layer-v2.md (STATUS: Validated)
- Spec: Packet references Handshake_Master_Spec_v02.90; docs/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.90). Current SPEC_CURRENT is v02.93, so storage abstraction requirements must be re-aligned and evidence remapped before treating this WP as Done.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor storage abstraction DONE_MEANS to Master Spec v02.93 (A2.3.12), refresh EVIDENCE_MAPPING, rerun TEST_PLAN/validator scans, and resubmit. Status must return to Ready for Dev until revalidated.
## VALIDATION [CX-623]
- **cargo check**: ??? PASS
- **cargo test**: ??? PASS (97 tests passed)
- **just validator-dal-audit**: ??? PASS
- **just validator-scan**: ??? PASS

### EVIDENCE_MAPPING
- [CX-DBP-040] (No `sqlite_pool`): Verified in `src/backend/handshake_core/src/storage/mod.rs` (Trait pure) and `sqlite.rs` (Impl removed).
- [HSK-GC-005] (Janitor generic): Verified in `src/backend/handshake_core/src/storage/retention.rs` (Consumes `Arc<dyn Database>`).
- AppState Purity: Verified in `src/backend/handshake_core/src/lib.rs` (Removed `fr_pool`).

---

### VALIDATION REPORT ??? WP-1-Storage-Abstraction-Layer-v2 (2025-12-26)
Verdict: PASS ???

**Scope Inputs:**
- Task Packet: `docs/task_packets/WP-1-Storage-Abstraction-Layer-v2.md`
- Spec: `Handshake_Master_Spec_v02.90 ??2.3.12` (Pillar 1: One Storage API)
- Coder: [[coder gemini]]

**Files Checked:**
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/lib.rs`
- `src/backend/handshake_core/src/storage/retention.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/api/flight_recorder.rs`

**Findings:**
- **[CX-DBP-010] Pillar 1 (One Storage API):** PASS. `AppState` refactored to expose only `Arc<dyn Database>` and `Arc<dyn FlightRecorder>`. Concrete pool leakages (`SqlitePool`, `fr_pool`) have been successfully removed from public surfaces.
- **[CX-DBP-040] Trait Purity Invariant:** PASS. The `Database` trait in `mod.rs` no longer contains the `sqlite_pool()` method. It is now 100% backend-agnostic.
- **Janitor Decoupling:** PASS. The `Janitor` service in `retention.rs` now consumes the generic `Database` trait. The database-specific pruning logic has been correctly moved to the `SqliteDatabase` and `PostgresDatabase` implementations.
- **Postgres Portability:** PASS. `postgres.rs` provides a full implementation of the expanded `AiJob` mapping and implements the `prune_ai_jobs` method (returning `NotImplemented` as mandated).
- **Forbidden Pattern Audit:** PASS. Grep for `state.pool` and `state.fr_pool` in `src/api/` returned zero hits. Handlers now correctly rely on trait-based methods.
- **Tests:** PASS. `cargo test` returns 97 passed tests. Storage conformance tests verify identical behavior across both SQLite and Postgres (via the shared trait interface).

**REASON FOR PASS:**
The implementation successfully enforces the Storage Backend Portability Architecture (??2.3.12). By hardening the trait boundary and removing concrete pool leakages, the coder has eliminated the "leaky abstraction" risk and ensured that business logic remains decoupled from the underlying storage engine. This fulfills a major Phase 1 Strategic Audit criterion.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250259


## VALIDATION REPORT ƒ?" 2025-12-28 (Maintenance)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Storage-Abstraction-Layer-v2.md (STATUS: Validated)
- Spec: Handshake_Master_Spec_v02.93 (A2.3.12 Trait Purity) via docs/SPEC_CURRENT.md
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/storage/tests.rs (integration test harness visibility and import hygiene)

Findings:
- Removed crate-level `cfg(test)` gate to restore `storage::tests` utilities for integration conformance tests.
- Guarded chrono imports with `cfg(test)` to prevent unused-import warnings while keeping helpers available for dual-backend runs.
- Forbidden Pattern Audit [CX-573E]: PASS (no unwrap/expect/panic/todo/dbg/println in the adjusted scope; module remains test-only).
- Zero Placeholder Policy [CX-573D]: PASS; harness functions remain fully implemented.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --all-targets` (PASS; includes sqlite/postgres storage_conformance)

REASON FOR PASS: Storage test utilities are now accessible to integration tests without warnings, and the full suite passes under Spec v02.93.
