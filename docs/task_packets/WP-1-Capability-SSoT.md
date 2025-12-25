# Work Packet: WP-1-Capability-SSoT

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec (capability enforcement consistency)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Create a single source of truth (SSoT) for capabilities and profiles, replacing hardcoded maps; enforce capability checks consistently across AI jobs/MCP/tools.

**Effort:** 6-10 hours  
**Phase 1 Blocking:** YES (security/consistency)

---

## Scope
### In Scope
1) CapabilityRegistry/SSoT for capabilities and profiles.
2) Refactor workflow/jobs/LLM/MCP to consume SSoT (no hardcoded maps).
3) Tests verifying required capabilities per job/tool.

### Out of Scope
- UI for capability management (Phase 2).

---

## Quality Gate
- **RISK_TIER:** MEDIUM
- **DONE_MEANS:**
  - Central CapabilityRegistry implemented; hardcoded maps removed.
  - Workflow/MCP/AI job paths consume SSoT.
  - Tests cover capability enforcement per job/tool.
  - No forbidden patterns.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (capability clauses); src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/llm.rs; src/backend/handshake_core/src/mcp
- **SEARCH_TERMS:** "CapabilityRegistry", "capability_profile", "capability_required"
- **RUN_COMMANDS:** cargo test; just validator-hygiene-full
- **RISK_MAP:** "Capability drift -> security gap"; "Hardcoded map -> inconsistency"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
