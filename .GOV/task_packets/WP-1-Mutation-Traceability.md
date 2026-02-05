# Task Packet: WP-1-Mutation-Traceability

## Metadata
- TASK_ID: WP-1-Mutation-Traceability
- DATE: 2025-12-25
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator


## SKELETON APPROVED
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja251220252310

## Scope
- **What**: Implement mutation traceability and storage guard to enforce ???No Silent Edits??? per ??2.9.3.
- **Why**: Ensure all RAW mutations carry actor/job/workflow metadata and are blocked for AI writes without approval context.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/storage/sqlite.rs
  * src/backend/handshake_core/src/storage/postgres.rs
  * src/backend/handshake_core/src/models.rs
  * src/backend/handshake_core/migrations/
- **OUT_OF_SCOPE**:
  * UI surfacing of metadata (Phase 2).
  * Historical backfill of older edits (Phase 2).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Core safety invariant; failure allows silent AI edits.
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-spec-regression
  just validator-scan WP-1-Mutation-Traceability
  just validator-hygiene-full
  just validator-error-codes
  ```
- **DONE_MEANS**:
  * ??? `MutationMetadata` struct matches ??2.9.3 in v02.85.
  * ??? Database tables (blocks, canvas, documents) updated with ??2.9.3.1 columns and `CHECK` constraint.
  * ??? `StorageGuard` trait implemented and integrated into all mutation paths per ??2.9.3.2.
  * ??? AI writes without job/approval context fail with `HSK-403-SILENT-EDIT`.
  * ??? Unit and integration tests cover Human, AI, and System write scenarios.
  * ??? No forbidden patterns (unwrap/expect/panic/dbg/Value in domain).

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/START_HERE.md
  * .GOV/roles_shared/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.85.md
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/storage/sqlite.rs
- **SEARCH_TERMS**:
  * "MutationMetadata"
  * "StorageGuard"
  * "last_actor_kind"
  * "CHECK (last_actor_kind"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Silent edits -> safety breach" -> Storage layer
  * "Missing metadata -> audit gap" -> Storage layer
  * "Schema mismatch -> runtime failure" -> Database layer

## Authority
- **SPEC_ANCHOR**: ??2.9.3 (Mutation Traceability)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.85.md
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md

## Notes
- **Assumptions**: None.
- **Open Questions**: None.
- **Dependencies**: Foundational.

---

## HISTORY

### VALIDATION REPORT ??? WP-1-Mutation-Traceability (2025-12-25)
Verdict: PASS ???

I have verified the remediation of the compilation blockers and hygiene issues for WP-1-Mutation-Traceability. The implementation now correctly satisfies the "No Silent Edits" invariant defined in ??2.9.3 of the Master Spec v02.85.

Findings:
1. Mutation Traceability (??2.9.3): Persisted in DB tables with mandatory columns and SQL CHECK constraints.
2. Storage Guard (??2.9.3.2): DefaultStorageGuard implemented and integrated. Rejects AI writes without context (HSK-403-SILENT-EDIT).
3. Hygiene & Build: cargo test passes (54 tests). Forbidden patterns removed from production paths.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220252310

## VALIDATION REPORT â€” 2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Mutation-Traceability.md (STATUS: Validated)
- Spec: Packet references Master Spec v02.85; .GOV/roles_shared/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.85). Current SPEC_CURRENT is v02.93, so mutation traceability requirements and evidence must be rechecked against the updated Main Body before claiming Done.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor mutation traceability DONE_MEANS to Master Spec v02.93, refresh EVIDENCE_MAPPING, rerun TEST_PLAN/validator scans, and resubmit. Status must return to Ready for Dev until revalidated.



