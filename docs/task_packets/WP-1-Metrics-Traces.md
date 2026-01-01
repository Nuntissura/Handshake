# Work Packet: WP-1-Metrics-Traces

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §7.6.3 (metrics/OTel + validator pack)  
**USER_SIGNATURE:** <pending>

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must search for OpenTelemetry instrumentation and metrics export.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§7.6.3.6 / §5.3).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Executive Summary
Implement baseline metrics and traces: OpenTelemetry instrumentation for workflows/AI jobs, validator pack wired into CI for schema/diagnostic validation.

**Effort:** 6-10 hours  
**Phase 1 Blocking:** YES (observability)

---

## Scope
### In Scope
1) OTel instrumentation for workflows/AI jobs (spans/metrics).
2) Validator pack for schema/diagnostic validation wired into CI.
3) Basic metrics export (request/error counts, latency per action, token usage per job/model).

### Out of Scope
- Advanced dashboards (Phase 2).
- Long-term metrics storage (Phase 2).

---

## Quality Gate
- **RISK_TIER:** MEDIUM
- **DONE_MEANS:**
  - OTel instrumentation added to workflows/AI job paths.
  - Validator pack runs in CI (passes) for schema/diagnostic checks.
  - Metrics for request/error/latency/token usage exported.
  - Tests/lints pass.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§7.6.3 metrics/traces); src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/diagnostics.rs; CI config
- **SEARCH_TERMS:** "opentelemetry", "tracing", "metrics", "validator pack"
- **RUN_COMMANDS:** cargo test; just validator-hygiene-full
- **RISK_MAP:** "Missing instrumentation -> no observability"; "CI not wired -> drift"

---

## Success Metrics
| Metric | Target | Verification |
|--------|--------|--------------|
| OTel spans/metrics present | Workflows/AI jobs instrumented | Code evidence/tests |
| Validator pack | Runs in CI | CI job |

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>



## VALIDATION REPORT - WP-1-Metrics-Traces (2025-12-26)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Metrics-Traces.md (status: Ready for Dev; signature `<pending>`)
- Spec: Handshake_Master_Spec_v02.84.md (A5.3 observability, A7.6.3.6 roadmap pointer)

Files Checked:
- Repository scan: `rg -n "opentelemetry|otel|meter|metric|span" src` (no matches)
- src/backend/handshake_core/src/workflows.rs, diagnostics.rs (no OTel instrumentation)
- CI configs for validator pack (none found)

Findings:
- No OTel/metrics implementation exists: no spans, meters, or exporters for workflows/AI jobs; no token/request/error metrics.
- Validator pack: no CI job or scripts to validate diagnostics/schema; no evidence of validator pack wiring.
- Evidence mapping: none; no file:line satisfies DONE_MEANS requirements.

Hygiene / Forbidden Patterns:
- Not executed; validation blocked by missing implementation.

Tests:
- Not run; TEST_PLAN commands not executed and no instrumentation present.

Risks & Suggested Actions:
- Implement OTel instrumentation per A5.3: spans/metrics for workflows and AI jobs, token usage, errors, and latency. Add CI validator pack and export path. Provide evidence mapping and rerun TEST_PLAN.


