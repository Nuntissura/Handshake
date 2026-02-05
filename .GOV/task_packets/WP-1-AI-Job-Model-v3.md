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
This task hardens the "Job Model"â€”the system that tracks what the AI is doing. Previously, it used simple text labels for job types, which could lead to mistakes. We are moving to a strict list of allowed types (Enums). We are also expanding the "Metrics" to track exactly how many tokens and security checks each job uses, and ensuring the database never has missing data (NULLs) for these important numbers. Finally, we are adding the logic to "poison" a job if a security attack is detected.

## Scope
- **What**: Implement normative AI Job Model structs, enums, and persistence per Â§2.6.6.2.8. Harden the Workflow Engine to support Atomic Poisoning.
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
  * âœ… `AiJob`, `JobKind`, `JobMetrics`, and `JobState` match Â§2.6.6.2.8 in v02.93 exactly.
  * âœ… Database schema (0006 or 0008) enforces `NOT NULL` for all metric columns.
  * âœ… `jobs.rs` updated to use `JobKind` enum for creation.
  * âœ… `workflows.rs` implements the `Poisoned` state trap per Â§2.6.6.7.11.0.
  * âœ… No forbidden patterns (unwrap/expect/Value in domain).
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  ```

## Bootstrap (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/START_HERE.md
  * .GOV/roles_shared/SPEC_CURRENT.md (Master Spec v02.93 Â§2.6.6.2.8)
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
- **SPEC_ANCHOR**: Â§2.6.6.2.8 (Normative Rust Types)
- **SPEC_CURRENT**: .GOV/roles_shared/SPEC_CURRENT.md (Master Spec v02.93)
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md

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

### Evidence Mapping (Spec â†’ Code)

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

## VALIDATION REPORT â€” 2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model-v3.md (STATUS: Done/Validated)
- Spec: Packet references Handshake_Master_Spec_v02.92 (A2.6.6.2.8); .GOV/roles_shared/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.92). Current SPEC_CURRENT is v02.93, so alignment with the latest Main Body cannot be confirmed without re-enrichment and refreshed evidence mapping.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor the AI Job Model requirements to Master Spec v02.93, update DONE_MEANS/EVIDENCE_MAPPING, rerun the TEST_PLAN and validator scans, and resubmit. Status must revert to Ready for Dev until revalidated.

## VALIDATION REPORT â€” 2025-12-27 (Revalidation, Spec v02.93)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model-v3.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.93 (Â§2.6.6.2.8 Normative Rust Types)
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/storage/mod.rs:313-359 (JobKind enum + validated FromStr), 440-476 (JobMetrics defaults), 479-499 (AiJob struct fields)
- src/backend/handshake_core/migrations/0008_expand_ai_job_model.sql (metrics NOT NULL/default enforcement)
- src/backend/handshake_core/src/workflows.rs:326-379 (poisoning trap handling AceError with JobState::Poisoned + FR logging)

Findings:
- Spec alignment: JobKind/JobState/AccessMode/SafetyMode enums and AiJob struct fields satisfy Â§2.6.6.2.8; metrics zeroed at init and persisted via NOT NULL defaults.
- Atomic poisoning trap routes AceError to JobState::Poisoned with Flight Recorder event emission.
- Forbidden Pattern Audit [CX-573E]: PASS (validator-scan; only unwraps in tests).
- Zero Placeholder Policy [CX-573D]: PASS; no stubs or hollow implementations in production paths.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests` (PASS; warnings: unused imports in retention/tests)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_poisoning_trap` (PASS; same warnings)
- `node .GOV/scripts/validation/validator-scan.mjs` (PASS)

REASON FOR PASS: AI Job Model conforms to Master Spec v02.93 Â§2.6.6.2.8 with strict enums, NOT NULL metrics, and poisoning trap; targeted tests and validator scan passed.

---

## VALIDATION REPORT - 2025-12-30 (Revalidation, Batch 6)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model-v3.md
- Spec (SPEC_CURRENT): .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md
- Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Commands Run:
- just validator-spec-regression: PASS
- just cargo-clean: PASS (Removed 0 files)
- just gate-check WP-1-AI-Job-Model-v3: FAIL (Implementation detected without SKELETON APPROVED marker)
- node .GOV/scripts/validation/post-work-check.mjs WP-1-AI-Job-Model-v3: FAIL (non-ASCII + missing COR-701 validation manifest fields/gates)
- just validator-packet-complete WP-1-AI-Job-Model-v3: FAIL (STATUS missing/invalid; requires canonical **Status:** Ready for Dev / In Progress / Done)
- just post-work WP-1-AI-Job-Model-v3: FAIL (blocked at gate-check)

Blocking Findings:
- Phase gate violation [CX-GATE-001]: gate-check fails because implementation is present without a prior "SKELETON APPROVED" marker in this packet.
- Deterministic manifest gate (COR-701): post-work-check reports missing required manifest fields (target_file, start, end, pre_sha1, post_sha1, line_delta) and missing/unchecked gates (C701-G01, C701-G02, C701-G04, C701-G05, C701-G06, C701-G08).
- ASCII-only requirement: post-work-check reports non-ASCII characters in the task packet.
  - NON_ASCII_COUNT=17 (sample: Line 13 Col 34 U+2014, Line 52 Col 5 U+2705, Line 112 Col 28 U+2192)
- Spec mismatch: this packet asserts Master Spec v02.93, but .GOV/roles_shared/SPEC_CURRENT.md points to v02.98. Prior PASS claims are not valid against the current spec.
- Internal inconsistency: this packet already contains an older "Verdict: FAIL" section (.GOV/task_packets/WP-1-AI-Job-Model-v3.md:146) but TASK_BOARD currently lists it as Done/[VALIDATED].

Spec-to-Code Findings (v02.98, spot-check):
- Normative Rust types are defined (AiJob/JobMetrics/JobKind/JobState/AccessMode) in src/backend/handshake_core/src/storage/mod.rs:307-553.
- Spec requires the normative JobState variants (Handshake_Master_Spec_v02.98.md:5307-5318) but current code adds JobState::Stalled (src/backend/handshake_core/src/storage/mod.rs:307-318), which is not present in the v02.98 normative list.
- Spec defines a normative JobKind set (Handshake_Master_Spec_v02.98.md:5284-5291); current code includes additional variants (src/backend/handshake_core/src/storage/mod.rs:339-352). Some variants appear needed by other v02.98 sections (e.g., debug_bundle_export job profile), but alignment is not evidenced/mapped in this packet for v02.98.
- Metrics NOT NULL/default enforcement exists in the migration: src/backend/handshake_core/migrations/0008_expand_ai_job_model.sql:6-27 (metrics TEXT NOT NULL DEFAULT ... at line 21).
- Zeroed metrics at job init is enforced in create_job: src/backend/handshake_core/src/jobs.rs:18-41 (metrics: JobMetrics::zero()).

REASON FOR FAIL:
- Blocking process gates (phase gate + COR-701 manifest + ASCII-only + packet completeness) fail; spec alignment to v02.98 is not demonstrated.

Required Fixes:
1) Bring this packet back into protocol: include proper BOOTSTRAP/SKELETON/IMPLEMENTATION/HYGIENE/VALIDATION sections and obtain explicit "SKELETON APPROVED" before implementation evidence.
2) Make the task packet ASCII-only (remove/replace non-ASCII characters; rerun post-work-check until clean).
3) Add a COR-701 validation manifest (target_file/start/end/pre_sha1/post_sha1/line_delta + gates checklist) and ensure `just post-work WP-1-AI-Job-Model-v3` passes.
4) Re-anchor DONE_MEANS + evidence mapping to Handshake_Master_Spec_v02.98.md (2.6.6.2.8) and explicitly map any JobKind/JobState deviations or reconcile them.
5) Reconcile TASK_BOARD status with packet status history (no Done while any FAIL exists).

**Status:** Ready for Dev
USER_SIGNATURE: ilja261220252215



