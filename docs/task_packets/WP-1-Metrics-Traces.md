# Work Packet: WP-1-Metrics-Traces

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §7.6.3 (metrics/OTel + validator pack)  
**USER_SIGNATURE:** <pending>

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
