# Task Packet: WP-1-Atelier-Lens-v0.1

## Metadata
- TASK_ID: WP-1-Atelier-Lens-v0.1
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Procedure:** 
1. Validator must search for `ATELIER_CLAIM` jobs and role-based extraction logic.
2. Verify if `SceneState` and `ConflictSet` logic exists.
3. Align with §7.6.3.19.

---

## SCOPE
- **What**: Implement Atelier Lens v0.1 (Role claiming and dual-contract extraction).
- **Why**: Allow AI to claim creative roles and extract structured intent from workspace assets.
- **SPEC_ANCHOR**: §7.6.3.19, §2.6.6.7.14

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Atelier-Lens-v0.1
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
  * "WP-1-Atelier-Lens-v0.1"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Atelier-Lens-v0.1
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





