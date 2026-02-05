# Task Packet: WP-1-Workflow-Engine

## Metadata
- TASK_ID: WP-1-Workflow-Engine
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## ðŸ•µï¸ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (Â§1-6, Â§9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check `workflows.rs` and SQLite tables.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (Â§2.6 Workflow & Automation Engine).
3. Surface-level compliance with roadmap bullets (Â§7.6.3.3) is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement the Workflow Engine core.
- **Why**: Ensure all AI work follows a governed, persistent execution path.
- **SPEC_ANCHOR**: Â§7.6.3.3, Â§2.6

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Workflow-Engine
just validator-hygiene-full
`

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
  * "WP-1-Workflow-Engine"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Workflow-Engine
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


## VALIDATION REPORT — WP-1-Workflow-Engine
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Workflow-Engine.md (status: Ready for Dev)
- Spec: - **SPEC_ANCHOR**: Â§7.6.3.3, Â§2.6

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.




## VALIDATION REPORT — WP-1-Workflow-Engine (2025-12-25)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Workflow-Engine.md (status: Ready for Dev)
- Spec: Handshake_Master_Spec_v02.84.md (§2.6 Workflow & Automation Engine; roadmap §7.6.3.3)

Files Checked:
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/storage/mod.rs (WorkflowRun)
- src/backend/handshake_core/src/storage/sqlite.rs/postgres.rs (workflow_runs table ops)
- Search: rg -n "workflow_run" src/backend/handshake_core/src

Spec Requirements (excerpted):
- Persistent workflow engine with nodes/edges/status, crash recovery, gate pipeline, capability/policy enforcement, manifesting, retries, determinism; integration with AI Job Model and Flight Recorder; conformance tests.

Findings:
- Workflow engine is minimal: start_workflow_for_job runs job inline, creates a workflow_run row, and logs a few events. No node graph, persistence of steps, retry/recovery, gate pipeline, manifest, or determinism controls.
- No crash recovery or resume; no queued/running/finalized state machine per spec; no gate enforcement beyond capability check.
- No integration with Flight Recorder beyond simple events; no Problems/Operator Console linkage.
- Tests absent for workflow engine behavior.

Hygiene:
- No forbidden-pattern issues in production path; test-only unwraps remain.

Tests:
- None executed; no targeted tests present.

Reason for FAIL:
- Specified workflow engine features (graph, recovery, gates, determinism, logging, tests) are unimplemented; current implementation is a thin async wrapper.



