# Task Packet: WP-1-Calendar-Lens

## Metadata
- TASK_ID: WP-1-Calendar-Lens
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must search for `CalendarEvent` storage and interval selection logic.
2. Verify if implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§7.6.3.12 / §11.9).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement local-only Calendar lens for ActivitySpan selection.
- **Why**: Provide a time-based view of AI activity without external sync dependencies.
- **SPEC_ANCHOR**: §7.6.3.12

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Calendar-Lens
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
  * "WP-1-Calendar-Lens"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Calendar-Lens
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


## VALIDATION REPORT � WP-1-Calendar-Lens
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Calendar-Lens.md (status: Ready for Dev)
- Spec: - **SPEC_ANCHOR**: §7.6.3.12

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.




## VALIDATION REPORT � WP-1-Calendar-Lens (2025-12-25)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Calendar-Lens.md (status: Ready for Dev)
- Spec: Handshake_Master_Spec_v02.84.md (�7.6.3.12 / �11.9 Calendar & Flight Recorder ActivitySpans)

Files Checked:
- Search: rg -n "Calendar|ActivitySpan|SessionSpan" src app

Spec Requirements (excerpted):
- Local-only Calendar lens with ActivitySpan/SessionSpan querying over Flight Recorder; time-range selection, filters, deep-links; integration with Job History/Flight Recorder; no external sync.

Findings:
- No calendar/ActivitySpan/SessionSpan data models, storage, or UI found in codebase.
- No integration with Flight Recorder or Job History for time-based queries.

Hygiene / Forbidden Patterns:
- Not run; no implementation exists.

Tests:
- None executed; no calendar lens tests present.

Reason for FAIL:
- Calendar lens per spec is absent.
