# Work Packet: WP-1-Security-Gates

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec (Phase 1 security/RCE guardrails)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Implement terminal/RCE security gates: deny-by-default, allowlists, timeouts/kill_grace, max output, cwd restriction, and secret scans before execution.

**Effort:** 8-12 hours  
**Phase 1 Blocking:** YES (safety)

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
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (security clauses); src/backend/handshake_core/src/terminal.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/api
- **SEARCH_TERMS:** "terminal", "exec", "timeout", "kill_grace", "max_output", "cwd", "secret scan"
- **RUN_COMMANDS:** cargo test; just validator-hygiene-full
- **RISK_MAP:** "RCE/secret leak"; "unbounded output"; "path traversal"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
