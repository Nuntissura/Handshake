# Task Packet: WP-1-Storage-Foundation-v2

## Metadata
- TASK_ID: WP-1-Storage-Foundation-v2
- DATE: 2025-12-28
- REQUESTOR: Orchestrator
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: DONE

## Scope
- **What**: Enforce the Trait Purity Invariant [CX-DBP-040] and establish a verified baseline for storage portability.
- **Why**: Existing implementation may leak concrete `sqlx` or `SqlitePool` types outside the storage module, which blocks the clean transition to future backends (e.g., PostgreSQL).
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/storage/
  * src/backend/handshake_core/src/lib.rs (AppState)
  * src/backend/handshake_core/src/api/
  * src/backend/handshake_core/src/jobs/
- **OUT_OF_SCOPE**:
  * Database schema migrations (covered in WP-1-Migration-Framework).
  * Frontend UI changes.

## Quality Gate
- **RISK_TIER**: MEDIUM
  - Justification: Architectural refactor of the core database boundary.
- **TEST_PLAN**:
  ```bash
  # 1. Compile and unit tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml

  # 2. Mandatory Portability Audit (MAIN BODY §2.3.12.5)
  # Verification: No sqlx usage outside storage module
  grep -r "sqlx::" src/ | grep -v "src/backend/handshake_core/src/storage"
  
  # 3. Trait Purity Check
  # Verification: No SqlitePool leakage
  grep -r "SqlitePool" src/ | grep -v "src/backend/handshake_core/src/storage/sqlite.rs"

  # 4. External Cargo target hygiene
  just cargo-clean

  # 5. Post-work validation
  just post-work WP-1-Storage-Foundation-20251228
  ```
- **DONE_MEANS**:
  * ✅ `Database` trait MUST NOT expose concrete types like `SqlitePool` [§2.3.12.3].
  * ✅ `AppState` MUST use `Arc<dyn Database>` and NOT `SqlitePool` [§2.3.12.3].
  * ✅ **MANDATORY AUDIT**: `grep` scans for `sqlx::` and `SqlitePool` MUST return zero matches outside the `src/storage` module [§2.3.12.5].
  * ✅ All existing storage tests pass.
  * ✅ AI review (`just ai-review`) returns PASS or WARN.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md (v02.95)
  * src/backend/handshake_core/src/storage/mod.rs (Trait definition)
  * src/backend/handshake_core/src/storage/sqlite.rs (Implementation)
  * src/backend/handshake_core/src/lib.rs (AppState)
  * src/backend/handshake_core/src/api/jobs.rs (Example consumer)
- **SEARCH_TERMS**:
  * "pub trait Database"
  * "SqlitePool"
  * "sqlx::"
  * "Arc<dyn Database>"
  * "fn sqlite_pool" (Check for forbidden methods)
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  # Initial audit to find leakages
  grep -r "sqlx::" src/ | grep -v "src/backend/handshake_core/src/storage"
  ```
- **RISK_MAP**:
  * "Leaky trait methods return concrete pools" -> Portability Failure [§2.3.12.3]
  * "Handlers depend on sqlx macros directly" -> Abstraction Leak
  * "AppState refactor causes circular dependencies" -> Build Failure

## Authority
- **SPEC_ANCHOR**: §2.3.12.3 (Trait Purity), §2.3.12.5 (Mandatory Audit)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.96.md [ilja281220250353]
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Dependencies**: Supersedes failed `WP-1-Storage-Foundation`.
- **Note**: The focus is on the *boundary* and *purity*, not the internal logic of the database itself.
- **Waiver**: Flight Recorder `fr_pool` (DuckDB) is retained until its own refactoring WP per §2.3.12.3.

---

## Validation Report

- **Verdict**: PASS
- **Commit**: `28bacb8` feat: enforce trait purity by hiding sqlx types from StorageError
- **Date**: 2025-12-28

### Evidence

| Criterion | Result | Evidence |
|-----------|--------|----------|
| `Database` trait no SqlitePool | ✅ PASS | `mod.rs:693-804` - no concrete pool types |
| `AppState` uses `Arc<dyn Database>` | ✅ PASS | `lib.rs:22` |
| `sqlx::` audit (outside storage) | ✅ PASS | 0 matches in api/, workflows.rs, lib.rs, main.rs |
| `SqlitePool` audit (outside sqlite.rs) | ✅ PASS | 0 matches in api/, workflows.rs, lib.rs, main.rs |
| All tests pass | ✅ PASS | 115 tests passed |

### Implementation Summary

`StorageError` refactored to hide provider-specific types:
- `Database(#[from] sqlx::Error)` → `Database(String)` + manual `From` impl
- `Migration(#[from] sqlx::migrate::MigrateError)` → `Migration(String)` + manual `From` impl

### Residual Risk Resolved

**Reconciliation [ilja281220250525]**: Master Spec reconciled to v02.96. All `SqlitePool` references in §11.3.4 (Implementation Notes) and signatures across the spec have been replaced with `&dyn Database` or abstract logic. Spec-to-Code parity restored.

---

**Last Updated:** 2025-12-28
**User Signature Locked:** ilja281220250353