# Task Packet: WP-1-Capability-SSoT-Validator

## Metadata
- TASK_ID: WP-1-Capability-SSoT-Validator
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (1-6, 9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must check `capabilities.rs`.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (7.6.3.15 v02.36 / 11.1).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement the unknown-capability validator and problem surfacing logic.
- **Why**: Prevent "silent security failures" where unknown capabilities are requested but ignored.
- **SPEC_ANCHOR**: 7.6.3.15 (v02.36)

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
```bash
just validator-spec-regression
just validator-scan WP-1-Capability-SSoT-Validator
just validator-hygiene-full
```

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
  * "WP-1-Capability-SSoT-Validator"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Capability-SSoT-Validator
  `
- RISK_MAP:
  * "Spec mismatch" -> validate SPEC_CURRENT and anchors
  * "Placeholder evidence" -> block until file:line mapping exists
  * "Forbidden patterns" -> run validator-scan and fix findings

## AUTHORITY
- SPEC_CURRENT: Handshake_Master_Spec_v02.84.md
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## EVIDENCE_MAPPING
- [Requirement] -> [File:Line]

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
- Validation tracked in primary packet WP-1-Capability-SSoT.md (see VALIDATION section) -> PASS




