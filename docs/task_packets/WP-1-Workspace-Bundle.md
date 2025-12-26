# Task Packet: WP-1-Workspace-Bundle

## Metadata
- TASK_ID: WP-1-Workspace-Bundle
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## CODE ARCHAEOLOGY NOTE
Authority [CX-598]: The Roadmap is only a pointer. The Master Spec Main Body (A1-6, A9-11) is the sole definition of "Done."
Procedure:
1. Validator must check workspace bundle export/backup implementation.
2. Verify implementation matches the Main Body section governing workspace bundle/export. Every line must be implemented.
3. If 100% alignment exists -> PASS. Otherwise -> FAIL.

---

## SCOPE
- What: Implement workspace bundle backup/transfer export.
- Why: Support portability and recovery.
- SPEC_ANCHOR: Workspace bundle/export section (per Master Spec Main Body).

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Workspace-Bundle
just validator-hygiene-full
`

## DONE_MEANS
- Spec requirements from referenced anchors are fully implemented or gaps recorded with FAIL.
- Forbidden-pattern audit is clean or explicitly justified.
- TEST_PLAN commands executed and outputs captured in the validation report.
- Evidence mapping lists file:line for every requirement.

## BOOTSTRAP
- FILES_TO_OPEN:
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.84.md
  * docs/TASK_BOARD.md
- SEARCH_TERMS:
  * "WP-1-Workspace-Bundle"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Workspace-Bundle
  `
- RISK_MAP:
  * "Spec mismatch" -> validate SPEC_CURRENT and anchors
  * "Placeholder evidence" -> block until file:line mapping exists
  * "Forbidden patterns" -> run validator-scan and fix findings

## AUTHORITY
- SPEC_CURRENT: Handshake_Master_Spec_v02.84.md
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md\n\n## EVIDENCE_MAPPING
- [Requirement] -> [File:Line]


## VALIDATION REPORT — WP-1-Workspace-Bundle
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Workspace-Bundle.md (status: Ready for Dev)
- Spec: - SPEC_ANCHOR: Workspace bundle/export section (per Master Spec Main Body).

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.



