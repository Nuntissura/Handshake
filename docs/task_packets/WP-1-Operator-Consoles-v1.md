# Work Packet: WP-1-Operator-Consoles-v1

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §7.6.3 (Flight Recorder operator consoles)  
## Metadata
- TASK_ID: WP-1-Operator-Consoles-v1
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must search for the Console UI components.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§10.5 Operator Consoles v1).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Executive Summary
Build Operator Consoles v1: Timeline, Jobs, Problems, Evidence views for Flight Recorder/diagnostics, with deep links to job/query plans/source refs.

**Effort:** 8-12 hours  
**Phase 1 Blocking:** YES (observability)

---

## Scope
### In Scope
1) Timeline view (time-range filters, event types, deep links to jobs).
2) Jobs view (history + per-job inspector with inputs/outputs/metrics).
3) Problems view (diagnostics/ProblemRegistry grouping).
4) Evidence drawer (Job → QueryPlan → SourceRefs).

### Out of Scope
- Advanced analytics/filters (Phase 2).
- UI polish beyond functional v1.

---

## Quality Gate
- **RISK_TIER:** MEDIUM
- **DONE_MEANS:**
  - Timeline/Jobs/Problems/Evidence views implemented and wired to backend APIs.
  - Deep links from jobs to evidence (query plans/source refs) functional.
  - Tests (frontend) cover rendering and basic interactions.
- **TEST_PLAN:**
  - `pnpm -C app test`
  - `pnpm -C app run lint`
  - `just validator-spec-regression`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§7.6.3 Flight Recorder/Operator Consoles); app/src/components; app/src/lib/api
- **SEARCH_TERMS:** "FlightRecorder", "Timeline", "Problems", "Diagnostics", "Evidence"
- **RUN_COMMANDS:** pnpm -C app test; pnpm -C app run lint
- **RISK_MAP:** "Missing deep links"; "UI untested"; "API gaps"

---

## Success Metrics
| Metric | Target | Verification |
|--------|--------|--------------|
| Views render | Timeline/Jobs/Problems/Evidence load | Frontend tests |
| Deep links | Job → Evidence works | Manual/automated tests |

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>



## VALIDATION REPORT - WP-1-Operator-Consoles-v1 (2025-12-26)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Operator-Consoles-v1.md (status: Ready for Dev; signature `<pending>`)
- Spec: Handshake_Master_Spec_v02.84.md (A10.5 Operator Consoles; roadmap pointer A7.6.3)

Files Checked:
- app/src/App.tsx (routes)
- app/src/components/FlightRecorderView.tsx
- Repository scan: `rg -n "Flight Recorder|FlightRecorder|Operator Console|Timeline|Problems|Evidence" app/src`

Findings:
- Operator Consoles absent: Only a simple `FlightRecorderView` exists (list of events). No Timeline, Jobs, Problems, or Evidence views, and no deep links to QueryPlan/SourceRefs as required by A10.5.
- Backend/API gaps: No dedicated APIs surfaced for consoles beyond the basic flight recorder endpoint; no evidence of Problems/Diagnostics registries wired to UI.
- Tests: No frontend tests for these views; TEST_PLAN not executed.
- Evidence mapping: None; no file:line satisfies DONE_MEANS items.

Hygiene / Forbidden Patterns:
- Not executed; validation blocked by missing implementation.

Tests:
- Not run; pnpm lint/test not executed in this audit.

Risks & Suggested Actions:
- Build required consoles per A10.5: Timeline (filters), Jobs view with inputs/outputs/metrics, Problems aggregation, Evidence drawer linking jobs to QueryPlan/SourceRefs. Add backend support as needed, wire routes, and add frontend tests. Provide evidence mapping and rerun TEST_PLAN.
