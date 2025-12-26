# Task Packet: WP-1-Diagnostic-Pipe

## Metadata
- TASK_ID: WP-1-Diagnostic-Pipe
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## ðŸ•µï¸ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (Â§1-6, Â§9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must search for `DIAG-SCHEMA-001/002` implementation.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (Â§2.9.1 Diagnostics).
3. Surface-level compliance with roadmap bullets (Â§7.6.3.6) is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement the normalized Diagnostic pipeline.
- **Why**: Provide grouped, auditable, and reproducible error tracking for AI workflows.
- **SPEC_ANCHOR**: Â§7.6.3.6, Â§2.9.1

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Diagnostic-Pipe
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
  * "WP-1-Diagnostic-Pipe"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Diagnostic-Pipe
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


## VALIDATION REPORT — WP-1-Diagnostic-Pipe
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Diagnostic-Pipe.md (status: Ready for Dev)
- Spec: - **SPEC_ANCHOR**: Â§7.6.3.6, Â§2.9.1

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.





## VALIDATION REPORT — WP-1-Diagnostic-Pipe (2025-12-25)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Diagnostic-Pipe.md (status: Ready for Dev)
- Spec: Handshake_Master_Spec_v02.84.md (§2.9.1 Diagnostics; roadmap §7.6.3.6)

Files Checked:
- Search: rg -n "Diagnostic|diagnostic_pipe|problem" src/backend/handshake_core/src app/src

Spec Requirements (excerpted):
- DIAG-SCHEMA-001/002 normalized diagnostics; grouping, fingerprints, severity codes, source file/line/col, policy outcomes; integration with Problems/Flight Recorder; validators and tests.

Findings:
- No diagnostics pipeline, schema, or Problems integration present in backend or frontend code.
- No evidence of DIAG-SCHEMA-001/002 types or persistence/logging.

Hygiene / Forbidden Patterns:
- Not run; no implementation exists.

Tests:
- None executed; no diagnostics tests present.

Reason for FAIL:
- Diagnostic pipeline per §2.9.1 is absent; no schema, logging, or UI wiring implemented.
