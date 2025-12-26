# Task Packet: WP-1-AI-Job-Model-v3

## Metadata
- TASK_ID: WP-1-AI-Job-Model-v3
- WP_ID: WP-1-AI-Job-Model-v3
- STATUS: Done
- DATE: 2025-12-26
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator

## User Context
This task hardens the "Job Model"—the system that tracks what the AI is doing. Previously, it used simple text labels for job types, which could lead to mistakes. We are moving to a strict list of allowed types (Enums). We are also expanding the "Metrics" to track exactly how many tokens and security checks each job uses, and ensuring the database never has missing data (NULLs) for these important numbers. Finally, we are adding the logic to "poison" a job if a security attack is detected.

## Scope
- **What**: Implement normative AI Job Model structs, enums, and persistence per §2.6.6.2.8. Harden the Workflow Engine to support Atomic Poisoning.
- **Why**: Eradicate "vibe-coding" in the job system and provide the foundation for quota enforcement and security-critical state transitions.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/storage/sqlite.rs
  * src/backend/handshake_core/src/models.rs
  * src/backend/handshake_core/src/jobs.rs
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/migrations/0006_expand_ai_job_model.sql (or create 0008)
- **OUT_OF_SCOPE**:
  * Implementing the security guards themselves (WP-1-ACE-Validators-v3).
  * UI surfacing of granular job metrics (Phase 2).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Core architectural entity; failure blocks workflow execution, auditability, and safety.
- **HARDENED_INVARIANTS**:
  * **Strict Enum Mapping**: `JobKind` MUST be a Rust `enum`. `JobState` and `AccessMode` MUST be strictly enforced.
  * **Metrics Integrity**: `JobMetrics` MUST NOT contain `NULL` values in the database. Defaults must be `0`.
  * **Atomic Poisoning**: The Workflow Engine MUST trap `AceError::PromptInjectionDetected` and commit `JobState::Poisoned`.
- **TEST_PLAN**:
  ```bash
  # Compile and unit test
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml

  # Verify job creation and metrics
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests
  
  # Verify poisoning logic
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_poisoning_trap

  # Full hygiene check
  just validate
  just post-work WP-1-AI-Job-Model-v3
  ```
- **DONE_MEANS**:
  * ✅ `AiJob`, `JobKind`, `JobMetrics`, and `JobState` match §2.6.6.2.8 in v02.92 exactly.
  * ✅ Database schema (0006 or 0008) enforces `NOT NULL` for all metric columns.
  * ✅ `jobs.rs` updated to use `JobKind` enum for creation.
  * ✅ `workflows.rs` implements the `Poisoned` state trap per §2.6.6.7.11.0.
  * ✅ No forbidden patterns (unwrap/expect/Value in domain).
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  ```

## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md (Master Spec v02.92 §2.6.6.2.8)
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/models.rs
  * src/backend/handshake_core/src/workflows.rs
- **SEARCH_TERMS**:
  * "pub struct AiJob"
  * "enum JobState"
  * "create_ai_job"
  * "update_ai_job_status"
  * "JobKind"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Enum migration failure" -> Database corruption
  * "Poisoning trap missed" -> Security breach
  * "Metrics NULL violation" -> Persistence failure

## Authority
- **SPEC_ANCHOR**: §2.6.6.2.8 (Normative Rust Types)
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.92)
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Assumptions**: The DB can be reset in dev to apply fixed migration 0006 if needed.
- **Open Questions**: None.
- **Dependencies**: Foundational.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220252215

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-{ID}-variant), do NOT edit this one.**

---

## VALIDATION REPORT (APPENDED per CX-WP-001)

**Validator:** claude-opus-4-5-20251101
**Date:** 2025-12-26
**Verdict:** PASS

### Evidence Mapping (Spec → Code)

| Requirement | Evidence |
|-------------|----------|
| [HSK-JOB-100] JobKind as Rust enum | `storage/mod.rs:315-326` - pub enum JobKind |
| [HSK-JOB-100] Validated FromStr | `storage/mod.rs:343-359` - Returns StorageError::Validation on invalid |
| [HSK-JOB-101] Metrics NOT NULL | `0008_expand_ai_job_model.sql:21` - NOT NULL DEFAULT '{...}' |
| [HSK-JOB-101] Zeroed at init | `storage/mod.rs:462-476` - JobMetrics::zero() |
| JobState with Poisoned | `storage/mod.rs:281-294` - Poisoned variant at line 293 |
| AiJob struct per spec | `storage/mod.rs:478-499` - All required fields present |
| Poisoned trap in workflows | `workflows.rs:779-820` - test_poisoning_trap verifies behavior |

### Tests Executed

| Command | Result |
|---------|--------|
| `cargo test storage::tests` | 3 passed |
| `cargo test workflows::tests::test_poisoning_trap` | 1 passed |
| `just validator-scan` | PASS |
| `just validator-dal-audit` | PASS |

### Hygiene Audit

- Forbidden patterns in domain: **CLEAN**
- SQL parameterization: **VERIFIED**
- Migration schema: **NOT NULL enforced with defaults**

### REASON FOR PASS

All DONE_MEANS criteria satisfied with file:line evidence. JobKind enum enforced with validated FromStr. Metrics integrity guaranteed by NOT NULL constraints. Poisoning trap verified by dedicated test.

**STATUS:** VALIDATED