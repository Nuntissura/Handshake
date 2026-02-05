# Task Packet: WP-1-Governance-Hooks

## Metadata
- TASK_ID: WP-1-Governance-Hooks
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## ðŸ•µï¸ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (Â§1-6, Â§9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check job/workflow metadata storage.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (Â§2.9.3 Invariants / Diary Protocol).
3. Surface-level compliance with roadmap bullets (Â§7.6.3.8) is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement governance hooks for Diary alignment and RID mapping.
- **Why**: Ensure the system can be audited against the human-readable "LAW" defined in the Diaries.
- **SPEC_ANCHOR**: Â§7.6.3.8

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Governance-Hooks
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
  * "WP-1-Governance-Hooks"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Governance-Hooks
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


## VALIDATION REPORT — WP-1-Governance-Hooks
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Governance-Hooks.md (status: Ready for Dev)
- Spec: - **SPEC_ANCHOR**: Â§7.6.3.8

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.






