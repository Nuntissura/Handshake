# Work Packet: WP-1-Operator-Consoles-v1

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §7.6.3 (Flight Recorder operator consoles)  
**USER_SIGNATURE:** <pending>

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
