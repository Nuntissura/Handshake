# Task Packet: WP-1-ACE-RAG-Plumbing

## Metadata
- TASK_ID: WP-1-ACE-RAG-Plumbing
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator must search for `QueryPlan` and `RetrievalTrace` persistence.
2. Verify if implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§2.6.6.7.14 ACE-RAG-001).
3. Surface-level compliance with roadmap bullets (§7.6.3.18) is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## SCOPE
- **What**: Implement the core ACE-RAG-001 audit and budgeting plumbing.
- **Why**: Ensure every RAG-backed answer is auditable, repeatable, and cost-controlled.
- **SPEC_ANCHOR**: §7.6.3.18, §2.6.6.7.14

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
```bash
just validator-spec-regression
just validator-scan WP-1-ACE-RAG-Plumbing
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
  * "WP-1-ACE-RAG-Plumbing"
  * spec anchor keywords
- RUN_COMMANDS:
  ```bash
  just validator-spec-regression
  just validator-scan WP-1-ACE-RAG-Plumbing
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





## VALIDATION REPORT � WP-1-ACE-RAG-Plumbing (2025-12-25)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-ACE-RAG-Plumbing.md (status: Ready for Dev)
- Spec: Handshake_Master_Spec_v02.84.md (�2.6.6.7.14 ACE-RAG-001; roadmap �7.6.3.18 pointer)

Files Checked:
- Handshake_Master_Spec_v02.84.md (�2.6.6.7.14 ACE-RAG-001)
- Repo search: rg -n "QueryPlan|RetrievalTrace" src app

Spec Requirements (excerpted):
- Define/persist QueryPlan and RetrievalTrace with fields for routes, budgets, filters, determinism, cache hits, rerank/diversity, spans, warnings/errors, hashes.
- Mandatory behavior: plan-before-retrieve; deterministic normalization/hash; ordered routing with budgets and required steps; deterministic scoring/tie-breaks; rerank/diversity rules; bounded spans; logging to Flight Recorder with ids/hashes; runtime validators (RetrievalBudgetGuard, ContextPackFreshnessGuard, IndexDriftGuard, CacheKeyGuard); conformance tests T-ACE-RAG-001..007.

Findings:
- No implementation present: `rg` finds no `QueryPlan` or `RetrievalTrace` in `src/` or `app/`; no schemas, planners, traces, validators, or logging.
- Evidence mapping: none; no file:line satisfies spec.

Hygiene / Forbidden Patterns:
- Not run; no implementation exists to audit.

Tests:
- Not run; no ACE-RAG plumbing implemented; TEST_PLAN not executed.

Reason for FAIL:
- ACE-RAG-001 plumbing absent; required schemas/validators/logging/tests do not exist.


