# Work Packet: WP-1-MCP-End-to-End

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec (MCP capability/logging chain)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Complete MCP end-to-end capability and logging chain: ensure capability metadata, gate decisions, tool calls, and responses are logged; enforce deny-by-default with capability checks.

**Effort:** 6-10 hours  
**Phase 1 Blocking:** YES (MCP security/compliance)

---

## Scope
### In Scope
1) Capability metadata propagation through MCP requests.
2) Logging of gate decisions/tool calls/responses to Flight Recorder.
3) Deny-by-default enforcement with capability checks for MCP tools/resources.

### Out of Scope
- MCP UI surfacing (Phase 2).

---

## Quality Gate
- **RISK_TIER:** MEDIUM
- **DONE_MEANS:**
  - Capability metadata enforced end-to-end for MCP.
  - Flight Recorder logs gate decisions/tool calls/responses.
  - Deny-by-default with explicit capability allowlist.
  - Tests cover allowed/denied paths.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (MCP clauses); src/backend/handshake_core/src/mcp; src/backend/handshake_core/src/gate
- **SEARCH_TERMS:** "MCP", "gate", "capability", "Flight Recorder", "deny-by-default"
- **RUN_COMMANDS:** cargo test; just validator-hygiene-full
- **RISK_MAP:** "Unauthorized MCP tool use"; "Missing logs -> audit gap"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
