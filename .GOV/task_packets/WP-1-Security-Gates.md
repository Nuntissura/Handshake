# Work Packet: WP-1-Security-Gates

**Status:** READY FOR DEV ðŸ”´  
**Authority:** Master Spec (Phase 1 security/RCE guardrails)  
**USER_SIGNATURE:** <pending>

## Metadata
- TASK_ID: WP-1-Security-Gates
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## ðŸ•µï¸ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (Â§1-6, Â§9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must search for secret scan and resource limit gates.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (Â§11.7.5 / Â§7.6.3.10).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Executive Summary
Implement terminal/RCE security gates: deny-by-default, allowlists, timeouts/kill_grace, max output, cwd restriction, and secret scans before execution.

**Effort:** 8-12 hours  
**Phase 1 Blocking:** YES (safety)

**Guiding Principle (Postgres later, cheap):**
1) One storage API: force all DB access through a single module.  
2) Portable schema/migrations: clear schema and upgrade steps, DB-agnostic SQL.  
3) Treat indexes as rebuildable (recompute from artifacts, not migrated rows).  
4) Dual-backend tests early: run SQLite + Postgres in CI to keep retrofits medium-effort.

---

## Scope
### In Scope
1) Terminal exec guardrails: timeout, kill_grace, max_output, cwd sandbox, allowlist of commands/modes.
2) Secret scan on terminal commands (regex/trufflehog-style) before execution.
3) Logging of decisions/results to Flight Recorder; typed errors.

### Out of Scope
- Containerization beyond cwd restriction (Phase 2).
- UI polish for terminal surfaces.

---

## Quality Gate
- **RISK_TIER:** HIGH
- **DONE_MEANS:**
  - Terminal execution enforces timeout, kill_grace, max_output, cwd restriction, allowlist/deny-by-default.
  - Secret scan runs before exec; rejects on potential secret.
  - Flight Recorder logs guard decisions; typed errors.
  - Tests cover allowed/blocked paths.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** .GOV/roles_shared/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (security clauses); src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/api
- **SEARCH_TERMS:** "terminal", "exec", "timeout", "kill_grace", "max_output", "cwd", "secret scan"
- **RUN_COMMANDS:** cargo test; just validator-hygiene-full
- **RISK_MAP:** "RCE/secret leak"; "unbounded output"; "path traversal"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>



## VALIDATION REPORT - WP-1-Security-Gates (2025-12-26)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Security-Gates.md (status: Ready for Dev; signature `<pending>`)
- Spec: Handshake_Master_Spec_v02.84.md (A11.7.5 terminal/RCE guardrails; roadmap pointer A7.6.3.10)

Files Checked:
- src/backend/handshake_core/src/terminal.rs
- src/backend/handshake_core/src/workflows.rs (to look for guard usage; none found)
- Repository scan: `rg -n "terminal" src/backend/handshake_core/src`

Findings:
- Guardrails missing: `TerminalService::run` (terminal.rs:14-66) executes arbitrary commands with only an optional timeout defaulting to 30s. There is no deny-by-default allowlist, no cwd restriction, no max_output enforcement, no kill_grace, no secret scan, and no capability/context gating, violating A11.7.5 and the WP DONE_MEANS.
- Logging/Flight Recorder: No logging of gate decisions or executions to Flight Recorder.
- Typed errors: Uses stringly `Invalid/Exec` variants without stable codes; no guard failure taxonomy per spec.
- Evidence mapping: No file:line satisfies secret scan or resource limits; no tests covering allowed/blocked paths.

Hygiene / Forbidden Patterns:
- `TerminalError::Invalid(String)` and `Exec(String)` are stringly; no justification. No other hygiene scan run due to missing guardrails.

Tests:
- Not run; TEST_PLAN commands not executed in this audit.

Risks & Suggested Actions:
- Add deny-by-default allowlist, cwd sandbox to workspace root, max_output limits, kill_grace handling, and pre-exec secret scan. Log gate decisions to Flight Recorder with typed error codes. Add tests for allowed/blocked/timeout scenarios and rerun TEST_PLAN.



