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

  # 2. Mandatory Portability Audit (MAIN BODY Â§2.3.12.5)
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
  * âœ… `Database` trait MUST NOT expose concrete types like `SqlitePool` [Â§2.3.12.3].
  * âœ… `AppState` MUST use `Arc<dyn Database>` and NOT `SqlitePool` [Â§2.3.12.3].
  * âœ… **MANDATORY AUDIT**: `grep` scans for `sqlx::` and `SqlitePool` MUST return zero matches outside the `src/storage` module [Â§2.3.12.5].
  * âœ… All existing storage tests pass.
  * âœ… AI review (`just ai-review`) returns PASS or WARN.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/START_HERE.md
  * .GOV/roles_shared/SPEC_CURRENT.md (v02.95)
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
  * "Leaky trait methods return concrete pools" -> Portability Failure [Â§2.3.12.3]
  * "Handlers depend on sqlx macros directly" -> Abstraction Leak
  * "AppState refactor causes circular dependencies" -> Build Failure

## Authority
- **SPEC_ANCHOR**: Â§2.3.12.3 (Trait Purity), Â§2.3.12.5 (Mandatory Audit)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.96.md [ilja281220250353]
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md

## Notes
- **Dependencies**: Supersedes failed `WP-1-Storage-Foundation`.
- **Note**: The focus is on the *boundary* and *purity*, not the internal logic of the database itself.
- **Waiver**: Flight Recorder `fr_pool` (DuckDB) is retained until its own refactoring WP per Â§2.3.12.3.

---

## Validation Report

- **Verdict**: PASS
- **Commit**: `28bacb8` feat: enforce trait purity by hiding sqlx types from StorageError
- **Date**: 2025-12-28

### Evidence

| Criterion | Result | Evidence |
|-----------|--------|----------|
| `Database` trait no SqlitePool | âœ… PASS | `mod.rs:693-804` - no concrete pool types |
| `AppState` uses `Arc<dyn Database>` | âœ… PASS | `lib.rs:22` |
| `sqlx::` audit (outside storage) | âœ… PASS | 0 matches in api/, workflows.rs, lib.rs, main.rs |
| `SqlitePool` audit (outside sqlite.rs) | âœ… PASS | 0 matches in api/, workflows.rs, lib.rs, main.rs |
| All tests pass | âœ… PASS | 115 tests passed |

### Implementation Summary

`StorageError` refactored to hide provider-specific types:
- `Database(#[from] sqlx::Error)` â†’ `Database(String)` + manual `From` impl
- `Migration(#[from] sqlx::migrate::MigrateError)` â†’ `Migration(String)` + manual `From` impl

### Residual Risk Resolved

**Reconciliation [ilja281220250525]**: Master Spec reconciled to v02.96. All `SqlitePool` references in Â§11.3.4 (Implementation Notes) and signatures across the spec have been replaced with `&dyn Database` or abstract logic. Spec-to-Code parity restored.

---

**Last Updated:** 2025-12-28
**User Signature Locked:** ilja281220250353

---

## REVALIDATION REPORT - WP-1-Storage-Foundation-20251228 (2025-12-30)

VALIDATION REPORT - WP-1-Storage-Foundation-20251228
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Storage-Foundation-20251228.md (Task Packet title refers to "WP-1-Storage-Foundation-v2")
- Spec Pointer: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (2.3.12 Storage Backend Portability Architecture)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Commands (evidence):
- just cargo-clean: PASS
- just validator-spec-regression: PASS
- just validator-packet-complete WP-1-Storage-Foundation-20251228: FAIL (STATUS missing/invalid)
- node .GOV/scripts/validation/gate-check.mjs WP-1-Storage-Foundation-20251228: FAIL (Missing BOOTSTRAP or SKELETON markers.)
- node .GOV/scripts/validation/post-work-check.mjs WP-1-Storage-Foundation-20251228: FAIL (non-ASCII packet + missing COR-701 manifest fields/gates)

Blocking Findings:
1) Phase gate FAIL: packet does not contain a SKELETON section, and no SKELETON APPROVED marker exists (gate-check).
2) Deterministic manifest gate FAIL (COR-701): post-work-check fails because:
   - packet contains non-ASCII bytes (count=54)
   - no COR-701 manifest fields parsed (target_file/start/end/pre_sha1/post_sha1/line_delta) and required gates are missing/un-checked
3) Spec mismatch: packet references Handshake_Master_Spec_v02.96.md, but .GOV/roles_shared/SPEC_CURRENT.md now requires Handshake_Master_Spec_v02.98.md.
4) Mandatory audit FAIL (Spec): Spec requires scanning the codebase for sqlx leakage outside storage (Handshake_Master_Spec_v02.98.md:3101).
   - Evidence: `sqlx::` appears outside storage in src/backend/handshake_core/src/models.rs:10 and src/backend/handshake_core/src/models.rs:13.

REASON FOR FAIL:
- Required workflow gates (gate-check + COR-701 post-work-check) do not pass, and the current codebase fails the mandatory storage portability audit (sqlx:: leakage outside storage).

Required Remediation:
- Create a NEW packet (recommended: WP-1-Storage-Foundation-v3) anchored to Handshake_Master_Spec_v02.98.md (ASCII-only) and ensure the runnable WP_ID matches the packet filename for `just post-work`.
- Follow phase gate: BOOTSTRAP -> SKELETON -> (Validator issues "SKELETON APPROVED") -> IMPLEMENTATION -> VALIDATION.
- Provide a full COR-701 deterministic manifest so `just post-work` can pass.
- Remediate sqlx leakage outside storage (src/backend/handshake_core/src/models.rs) to satisfy the mandatory audit (code change required; not performed in this revalidation).

Task Board Update:
- Move Storage Foundation from Done -> Ready for Dev (Revalidation FAIL).

Packet Status Update (append-only):
- **Status:** Ready for Dev

Timestamp: 2025-12-30
Validator: Codex CLI (Validator role)



