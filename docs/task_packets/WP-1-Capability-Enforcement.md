# Task Packet: WP-1-Capability-Enforcement

## Metadata
- TASK_ID: WP-1-Capability-Enforcement
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must search for capability check logic.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§11.1 Capability Registry / §11.3 MCP Gate).
3. Surface-level compliance with roadmap bullets (§7.6.3.4) is insufficient. Implementation must use the Central Registry (SSoT) and follow axis/scope resolution.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Scope
- **What**: Implement mandatory capability checks in the Workflow Engine. Jobs must possess required capabilities (e.g., `doc.summarize`) to execute.
- **Why**: Enforce "Safety through Architecture" by preventing unauthorized AI mutations or data access, fulfilling Phase 1 Item 4 of the Master Spec.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/jobs.rs
  * src/backend/handshake_core/src/models.rs
- **OUT_OF_SCOPE**:
  * A complex UI for managing capability roles (keep as code-defined maps for now).
  * Dynamic user-driven capability delegation.

## Quality Gate
- **RISK_TIER**: MEDIUM
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  node scripts/validation/post-work-check.mjs WP-1-Capability-Enforcement
  ```
- **DONE_MEANS**:
  * The `Workflow Engine` verifies if the job's `capability_profile_id` contains the required permissions for the `job_kind`.
  * If unauthorized, the job status moves to `failed` with an `error_message` containing "Unauthorized: Missing capability X".
  * The `doc_summarize` job kind is gated by the `doc.summarize` capability.
  * Added at least one backend test verifying that a job with an invalid profile fails.
- **ROLLBACK_HINT**:
  ```bash
  git checkout src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/jobs.rs
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * Handshake_Master_Spec_v02.50.md (§7.6.3 Item 4)
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/jobs.rs
  * src/backend/handshake_core/src/models.rs
- **SEARCH_TERMS**:
  * "capability_profile_id"
  * "run_job"
  * "doc_summarize"
  * "AiJob"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Valid jobs fail due to strictness" -> Workflow Engine
  * "Capability bypass via job_kind rename" -> Logic Layer
  * "Error message leaks sensitive info" -> Security

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: docs/TASK_BOARD.md
- **Logger**: Optional (not used for this WP)

## Notes
- **Implementation Detail**: Define a simple static constant or internal helper in `workflows.rs` that maps `job_kind` strings to a required capability string. For now, the "default" profile should contain `doc.read` and `doc.summarize`.

## Validation
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml -> PASS
- pnpm -C app test -> PASS

## Status / Handoff
- WP_STATUS: Completed
- What changed: Added capability map/enforcement; unauthorized profiles fail with explicit message; regression test added in workflows.rs.
- Next step / handoff hint: Add persistent capability profile management when new job kinds/profiles are introduced.

---

## VALIDATION REPORT — WP-1-Capability-Enforcement
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Capability-Enforcement.md (status: In Progress)
- Spec: Handshake_Master_Spec_v02.84.md (Packet incorrectly references STALE v02.50)

Files Checked:
- src/backend/handshake_core/src/workflows.rs
- docs/task_packets/WP-1-Capability-Enforcement.md

Findings:
- **Architectural Gap (SSoT) [HSK-4001]**: Implementation in `workflows.rs` uses a local `Lazy<HashMap>` profile map. Spec v02.84 §11.1 mandates a centralized `CapabilityRegistry` (SSoT).
- **Axis/Scope Logic**: Implementation lacks the `axis:scope` resolution logic defined in §11.1.3.
- **Audit Requirement**: Missing Flight Recorder events for capability checks (Allow/Deny). Spec mandates capturing `capability_id`, `actor_id`, and `decision_outcome`.
- **Error Codes**: Uses `Unauthorized` variant but lacks the mandatory `HSK-4001: UnknownCapability` anchor for registry misses.
- **Evidence Mapping [CX-627]**: MISSING. No line-by-line mapping provided by Coder.
- **Spec Regression**: Packet references v02.50. MUST align with v02.84 §11.1.

Risks & Suggested Actions:
- **RE-OPEN**. This is a "hollow" implementation that fails the Senior Grade Audit.
- Move capability logic from `workflows.rs` to a dedicated `CapabilityRegistry` service.
- Implement axis inheritance (e.g., `fs.read` grants `fs.read:logs`).
- Add mandatory Flight Recorder hooks for every check.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** <pending>





