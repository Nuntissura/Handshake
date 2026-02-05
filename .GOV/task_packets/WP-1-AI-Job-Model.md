# Task Packet: WP-1-AI-Job-Model

## Metadata
- TASK_ID: WP-1-AI-Job-Model
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## ðŸ•µï¸ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (Â§1-6, Â§9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check `models.rs` and `storage/mod.rs`.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (Â§2.6.6 AI Job Model).
3. Surface-level compliance with roadmap bullets (Â§7.6.3.2) is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement the global AI Job Model in the backend.
- **Why**: Ensure all AI actions are structured, traceable, and governed.
- **SPEC_ANCHOR**: Â§7.6.3.2, Â§2.6.6

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
```bash
just validator-spec-regression
just validator-scan WP-1-AI-Job-Model
just validator-hygiene-full
```
## DONE_MEANS
- Spec requirements from referenced anchors are fully implemented or gaps recorded with FAIL.
- Forbidden-pattern audit is clean or explicitly justified.
- TEST_PLAN commands executed and outputs captured in the validation report.
- Evidence mapping lists file:line for every requirement.

## BOOTSTRAP
- FILES_TO_OPEN:
  * .GOV/roles_shared/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.84.md
  * .GOV/roles_shared/TASK_BOARD.md
- SEARCH_TERMS:
  * "WP-1-AI-Job-Model"
  * spec anchor keywords
- RUN_COMMANDS:
  ```bash
  just validator-spec-regression
  just validator-scan WP-1-AI-Job-Model
  `
- RISK_MAP:
  * "Spec mismatch" -> validate SPEC_CURRENT and anchors
  * "Placeholder evidence" -> block until file:line mapping exists
  * "Forbidden patterns" -> run validator-scan and fix findings

## AUTHORITY
- SPEC_CURRENT: Handshake_Master_Spec_v02.84.md
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md\n\n## EVIDENCE_MAPPING
- [Requirement] -> [File:Line]


## VALIDATION REPORT — WP-1-AI-Job-Model
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model.md (status: Ready for Dev)
- Spec: - **SPEC_ANCHOR**: Â§7.6.3.2, Â§2.6.6

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.





## VALIDATION REPORT — WP-1-AI-Job-Model (2025-12-25)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AI-Job-Model.md (status: Ready for Dev)
- Spec: Handshake_Master_Spec_v02.84.md (§2.6.6 AI Job Model)

Files Checked:
- src/backend/handshake_core/src/models.rs
- src/backend/handshake_core/src/storage/mod.rs (AiJob)
- src/backend/handshake_core/src/jobs.rs
- Search: rg -n "AiJob|job_kind|protocol_id|profile_id" src/backend/handshake_core/src

Spec Requirements (excerpted):
- Canonical AI Job schema with full metadata: job_id, protocol, profile, capability gates, safety/access modes, inputs/outputs, trace_id, workflow_id, status history, timestamps, error codes; linked to Workflow Engine/Flight Recorder; validators; conformance tests.
- Typed errors and status transitions with auditability; capability/policy hooks.

Findings:
- AiJob struct is minimal (id, job_kind, status, protocol_id, profile_id, capability_profile_id, access_mode, safety_mode, inputs/outputs, timestamps). Missing trace_id, workflow_id linkage, status history, error codes, capability/policy bindings, metrics, validators per spec.
- No validation or guard logic; status updates are simple DB writes without policy/trace enforcement.
- Models.rs only exposes basic DTOs; lacks spec-required job schema fields and linkage to Flight Recorder or capability system.
- No conformance tests for job model state transitions or schema completeness.

Hygiene:
- No forbidden patterns noted in reviewed sections; errors are stringly typed.

Tests:
- None executed; no targeted tests present.

Reason for FAIL:
- AI Job Model per §2.6.6 is largely unimplemented: missing required fields, traceability, policy/logging integration, and tests.



