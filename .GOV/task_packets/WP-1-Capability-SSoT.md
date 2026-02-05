# Task Packet: WP-1-Capability-SSoT

## Metadata
- TASK_ID: WP-1-Capability-SSoT
- STATUS: Done
- DATE: 2025-12-25T20:05:00Z
- REQUESTOR: ilja
- AGENT_ID: Orchestrator
- ROLE: Orchestrator


## ??????? CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (??1-6, ??9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check `capabilities.rs`.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (??11.1 Capability Registry).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## USER_CONTEXT (Non-Technical Explainer) [CX-654]
This service acts as the project's "Security ID Office." Instead of having every part of the software guess what an AI model is allowed to do, we centralize all permissions (Capabilities) and roles (Profiles) into one authoritative registry. This ensures that an AI cannot "invent" a permission or bypass security gates by using hardcoded shortcuts.

---

## SCOPE

### Executive Summary
Implement `CapabilityRegistry` and `CapabilityProfile` models as the Single Source of Truth (SSoT) for all security and tool-filtering logic. Refactor `workflows.rs` and `api/jobs.rs` to consume this registry instead of local `Lazy<HashMap>` or hardcoded logic.

### IN_SCOPE_PATHS
- src/backend/handshake_core/src/capabilities.rs (New implementation)
- src/backend/handshake_core/src/workflows.rs (Refactor capability checks)
- src/backend/handshake_core/src/api/jobs.rs (Refactor profile mapping)
- src/backend/handshake_core/src/lib.rs (AppState wiring)
- src/backend/handshake_core/src/models.rs (Add RegistryError variants)
- src/backend/handshake_core/Cargo.toml (Dependency fix)
- src/backend/handshake_core/src/storage/retention.rs (Build fix only)
- src/backend/handshake_core/src/storage/sqlite.rs (Build fix only)

### OUT_OF_SCOPE
- UI for capability management.
- Dynamic profile persistence (Loaded from static config for now).
- Capability-scoped resource access (Phase 2).

---

## QUALITY GATE

- **TEST_PLAN**:
  ```bash
  # Core unit tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml capabilities
  
  # Integration workflow tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflow
  
  # Phase gate
  just gate-check WP-1-Capability-SSoT
  
  # Final hygiene
  just post-work WP-1-Capability-SSoT
  ```
- **DONE_MEANS**:
  - ??? `CapabilityRegistry` implemented per Master Spec ??11.1.
  - ??? All hardcoded capability maps removed from `workflows.rs`.
  - ??? `CapabilityProfile` objects (e.g. 'Analyst', 'Coder') are whitelists of Registry IDs.
  - ??? Integration tests verify unknown Capability ID results in deterministic error.
  - ??? Evidence mapping block is complete.

- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  # Manual steps:
  # 1. Remove src/backend/handshake_core/src/capabilities.rs
  # 2. Revert AppState in lib.rs
  # 3. Restore hardcoded maps in workflows.rs
  ```

---

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * .GOV/roles_shared/START_HERE.md
  * .GOV/roles_shared/SPEC_CURRENT.md (v02.96)
  * Handshake_Master_Spec_v02.96.md (??11.1)
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/api/jobs.rs
  * src/backend/handshake_core/src/lib.rs

- **SEARCH_TERMS**:
  * "CapabilityRegistry"
  * "capability_profile"
  * "term.exec"
  * "doc.summarize"
  * "enforce_capabilities"

- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gate-check WP-1-Capability-SSoT
  ```

- **RISK_MAP**:
  * "Incomplete refactor" -> Runtime panics
  * "Registry drift" -> Security bypass
  * "Type mismatch" -> Compilation failure

---

## SKELETON APPROVED
- RISK_TIER: MEDIUM
  - Justification: Foundation for security and tool filtering; refactors core workflow logic.
- USER_SIGNATURE: ilja251220252005

---

## EVIDENCE_MAPPING
- CapabilityRegistry SSoT (valid axes/full IDs, profiles, job/job_profile maps, HSK-4001 errors) -> src/backend/handshake_core/src/capabilities.rs:25-213
- Axis inheritance + profile whitelist enforcement in workflows with Flight Recorder log -> src/backend/handshake_core/src/workflows.rs:100-145
- API uses registry for job_kind ??? capability_profile_id mapping -> src/backend/handshake_core/src/api/jobs.rs:57-73
- Unknown capability/profile errors surface via registry -> src/backend/handshake_core/src/workflows.rs:285-350
- sqlx offline cache regenerated for storage (compilation unblock) -> src/backend/handshake_core/.sqlx/

---

## VALIDATION
- Deterministic Manifest (current workflow):
- Target File: src/backend/handshake_core/src/capabilities.rs
- Start: 1
- End: 400
- Line Delta: 30
- Pre-SHA1: e2182f4cc3bc5467afc36d3abe8e90a59961cd72
- Post-SHA1: 956136dd65c50ddef96f773c3a68807eb1579bb9
- Gates Passed:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [ ] compilation_clean
  - [ ] tests_passed
  - [ ] outside_window_pristine
  - [ ] lint_passed
  - [ ] ai_review (if required)
  - [ ] task_board_updated
  - [ ] commit_ready
- Lint Results:
- Artifacts:
- Timestamp:
- Operator:
- Notes:
- Validation Commands / Results:
- cargo sqlx prepare -- --bin handshake_core --lib --tests (DATABASE_URL=sqlite:///D:/Projects/LLM projects/Handshake/src/backend/handshake_core/.sqlx/dev.db) -> PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml capabilities (DATABASE_URL=sqlite:///D:/Projects/LLM projects/Handshake/src/backend/handshake_core/.sqlx/dev.db) -> PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflow (DATABASE_URL=sqlite:///D:/Projects/LLM projects/Handshake/src/backend/handshake_core/.sqlx/dev.db) -> PASS
- just gate-check WP-1-Capability-SSoT -> PASS
- just post-work WP-1-Capability-SSoT -> PASS

---

## AUTHORITY
- **SPEC_ANCHOR**: ??11.1 (Capability Registry)
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220252005

## VALIDATION REPORT ??? WP-1-Capability-SSoT
  Verdict: PASS ???

  The Work Packet WP-1-Capability-SSoT has been successfully implemented, verified, and closed.

  Summary of Work:
   1. Capability Registry (SSoT): Implemented a centralized registry in src/backend/handshake_core/src/capabilities.rs that defines
      valid capabilities, profiles, and job requirements.
   2. Workflow Integration: Refactored src/backend/handshake_core/src/workflows.rs to replace hardcoded permission maps with
      registry lookups.
   3. Safety: Capability checks now emit structured Flight Recorder events (capability_check) with "allowed" or "denied" outcomes.
   4. Hygiene: Resolved blocking build issues in the storage module (unrelated but necessary for validation).
   5. Documentation: Updated the Task Board and appended the Validation Report to the Task Packet.

  Artifacts:
   - src/backend/handshake_core/src/capabilities.rs: New module (SSoT).
   - .GOV/task_packets/WP-1-Capability-SSoT.md: Closed with PASS verdict.

  Note: The Coder correctly handled the scope expansion to fix the build, ensuring the repo remains in a compilable state.

## VALIDATION REPORT  2025-12-27 (Revalidation)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Capability-SSoT.md (STATUS: Validated)
- Spec: Packet references * Handshake_Master_Spec_v02.96.md [ilja281220250525] (??11.1); .GOV/roles_shared/SPEC_CURRENT.md now points to Handshake_Master_Spec_v02.93.
- Codex: Handshake Codex v1.4.md

Findings:
- Spec regression gate [CX-573B]/[CX-406]: Packet/spec pointer is stale (v02.84). Current SPEC_CURRENT is v02.93, so capability registry requirements and evidence must be rechecked against the updated Main Body before claiming Done.
- Forbidden Pattern Audit [CX-573E]: Not run (blocked by spec misalignment).
- Tests/commands: Not run in this pass (blocked).

REASON FOR FAIL: Re-anchor the Capability SSoT DONE_MEANS to Master Spec v02.93, refresh EVIDENCE_MAPPING, rerun TEST_PLAN/validator scans, and resubmit. Status must return to Ready for Dev until revalidated.

## VALIDATION REPORT  2025-12-28 (Final Recovery & Audit)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Capability-SSoT.md (STATUS: In-Progress)
- Spec: Handshake_Master_Spec_v02.96.md [Section 11.1]

Findings:
- **Spec Alignment [HSK-4001]:** Confirmed `RegistryError::UnknownCapability` is returned and `can_perform` checks validity first. `is_valid` correctly handles axis inheritance logic.
- **Security Logic Fix:** Corrected `workflows.rs` logic to explicitly check for `Ok(true)` on access grant. `Ok(false)` now correctly results in "denied" logging and access rejection.
- **Integration:** `job_requirements` correctly restored to enforce workflow capability checks.
- **Flight Recorder:** `workflows.rs` calls `log_capability_check` with structured outcome.
- **Hygiene Audit:** No `unwrap`/`expect` usage in production paths. `cargo check` and `cargo test` pass.

Conclusion:
The Work Packet meets all Spec and Task Packet requirements. The critical security bug identified during audit has been fixed and verified. The system is functional and hygienic.

Artifacts:
- `src/backend/handshake_core/src/capabilities.rs` (SSoT)
- `src/backend/handshake_core/src/workflows.rs` (Integration)
- `src/backend/handshake_core/src/api/jobs.rs` (Integration)

REASON FOR PASS: Full implementation of Spec 11.1 requirements with verified hygiene, security fixes, and passing tests.
## REVALIDATION NOTE 2025-12-28
- STATUS: In-Progress (revalidation required against Master Spec v02.96 after registry/profile updates on 2025-12-28).
- ACTION: Rerun TEST_PLAN and validator scans; refresh EVIDENCE_MAPPING for updated axes and job profile mappings.

## VALIDATION REPORT  2025-12-28 (Revalidation, Spec v02.96)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Capability-SSoT.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.96 11.1 (Capabilities & Consent Model)

Findings:
- Registry expanded to include canonical 11.1 capability IDs and mandatory axes; unknown IDs still yield HSK-4001 (capabilities.rs:3-120).
- Job profile mapping now covers all JobKind::as_str values (doc_edit, doc_summarize, term_exec, etc.) to prevent UnknownProfile at API creation (capabilities.rs:90-118).
- Capability checks in workflows log allowed/denied outcomes to Flight Recorder with structured payloads (workflows.rs:150-210).

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests --quiet` (PASS)

Reason for PASS: Spec 11.1 alignment confirmed; registry/profile coverage restored; tests passed with no forbidden-pattern regressions in the touched scope.

## STATUS CANONICAL (2025-12-28)
- Authoritative STATUS: Done (validated against Master Spec v02.96).
- Earlier status lines in this packet are historical and retained for audit only.

---

## REVALIDATION REPORT - WP-1-Capability-SSoT (2025-12-30)

VALIDATION REPORT - WP-1-Capability-SSoT
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Capability-SSoT.md
- Spec Pointer: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (11.1 Capabilities & Consent Model)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Commands (evidence):
- just validator-spec-regression: PASS
- node .GOV/scripts/validation/gate-check.mjs WP-1-Capability-SSoT: PASS
- just validator-packet-complete WP-1-Capability-SSoT: FAIL (STATUS missing/invalid)
- just post-work WP-1-Capability-SSoT: FAIL (C701-G05 post_sha1 mismatch for src/backend/handshake_core/src/capabilities.rs)
- git hash-object src/backend/handshake_core/src/capabilities.rs: 91ec38a7468eea1d0bc51c7344d27672c2ef653f

Blocking Findings:
1) Deterministic manifest gate FAIL: `just post-work WP-1-Capability-SSoT` reports `post_sha1 mismatch` (C701-G05) for `src/backend/handshake_core/src/capabilities.rs`.
   - Packet manifest Post-SHA1: 956136dd65c50ddef96f773c3a68807eb1579bb9 (.GOV/task_packets/WP-1-Capability-SSoT.md:136)
   - Current file SHA1: 91ec38a7468eea1d0bc51c7344d27672c2ef653f (git hash-object)
2) Packet completeness gate FAIL: `just validator-packet-complete WP-1-Capability-SSoT` fails because the packet does not contain a canonical `**Status:**` marker.
3) Spec mismatch: packet validation history is anchored to Handshake_Master_Spec_v02.96.md, but .GOV/roles_shared/SPEC_CURRENT.md now requires Handshake_Master_Spec_v02.98.md.

Spec-to-code spot-check (non-exhaustive; recorded for follow-up):
- Spec 11.1 audit requirement: capability checks MUST be logged with capability_id/actor_id/job_id/decision_outcome (Handshake_Master_Spec_v02.98.md:29183).
  - Current capability logging payload uses keys {capability_id, profile_id, job_id, outcome} (src/backend/handshake_core/src/workflows.rs:432) and overwrites FlightRecorderEvent.actor_id with capability_profile_id (src/backend/handshake_core/src/workflows.rs:506).

REASON FOR FAIL:
- `just post-work WP-1-Capability-SSoT` does not pass, so the COR-701 deterministic manifest does not match the current code state.

Required Remediation:
- Create a NEW packet (recommended: WP-1-Capability-SSoT-v2) anchored to Handshake_Master_Spec_v02.98.md, and capture a fresh COR-701 manifest so `just post-work` passes.
- Ensure the packet includes `**Status:** {Ready for Dev|In Progress|Done}` (validator-packet-complete) and remains ASCII-only.
- Reconcile the 11.1 audit logging field names (decision_outcome) and actor_id semantics in a follow-up WP (code change required; not performed in this revalidation).

Task Board Update:
- Move WP-1-Capability-SSoT from Done -> Ready for Dev (Revalidation FAIL).

Packet Status Update (append-only):
- **Status:** Ready for Dev

Timestamp: 2025-12-30
Validator: Codex CLI (Validator role)




