# Task Packet: WP-1-Storage-Foundation

## Metadata
- TASK_ID: WP-1-Storage-Foundation
  - DATE: 2025-12-26
  - REQUESTOR: Orchestrator
  - AGENT_ID: Codex
  - ROLE: Coder
  - **Status:** Done [VALIDATED]
  - USER_SIGNATURE: ilja

## SKELETON APPROVED

## Scope
- **What**: Establish baseline storage foundation (migration numbering, portability hooks, rebuildable index policy) per Master Spec v02.90 Â§2.3.12 and roadmap A7.6.3.
- **Why**: Phase 1 requires storage portability and rebuildable derived indexes before higher-layer work can rely on the DAL.
- **IN_SCOPE_PATHS**:
  * `src/backend/handshake_core/migrations/`
  * `src/backend/handshake_core/src/storage/`
- **OUT_OF_SCOPE**:
  * Frontend changes
  * Non-storage feature work

## Quality Gate
- **RISK_TIER**: MEDIUM
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **DONE_MEANS**:
  - Migration sequence covers initial storage foundation with portable SQL (no SQLite-specific syntax).
  - Storage trait exposes backend-agnostic methods; no direct pool leakage.
  - Rebuildable indexes documented in migrations and storage module comments.
  - Tests above pass.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha-for-storage-foundation>
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * `src/backend/handshake_core/src/storage/mod.rs`
  * `src/backend/handshake_core/src/storage/sqlite.rs`
  * `src/backend/handshake_core/src/storage/postgres.rs`
  * `src/backend/handshake_core/migrations/`
- **SEARCH_TERMS**:
  * `storage::`
  * `SqlitePool`
  * `Postgres`
  * `migration`
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * SQL portability regressions -> review migrations for `?1`, `strftime`, `CREATE TRIGGER`.

## VALIDATION
- Status: VALIDATED (per Task Board Done column)
- Evidence: Tests and DAL checks passed in commit e84a1431bc7a2b4cab7cee0cdbd12e52bc78a762; storage trait remains backend-agnostic; migrations numbered and portable.

## Authority
- **SPEC_CURRENT**: .GOV/roles_shared/SPEC_CURRENT.md
- **Master Spec**: Handshake_Master_Spec_v02.90.md (Â§2.3.12 storage portability, Â§11.7.4 build hygiene)
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md (Done)

## VALIDATION REPORT â€” 2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Storage-Foundation.md (STATUS: Validated)
- Spec: Packet references Handshake_Master_Spec_v02.90; .GOV/roles_shared/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.90). Current SPEC_CURRENT is v02.93, so storage foundation requirements and evidence must be rechecked against the updated Main Body before claiming Done.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor storage foundation DONE_MEANS to Master Spec v02.93, refresh EVIDENCE_MAPPING, rerun TEST_PLAN/validator scans, and resubmit. Status must return to Ready for Dev until revalidated.



