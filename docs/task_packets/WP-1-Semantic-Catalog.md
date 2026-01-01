# Work Packet: WP-1-Semantic-Catalog

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §2.6.7 Semantic Catalog  
**USER_SIGNATURE:** <pending>

---

## Metadata
- TASK_ID: WP-1-Semantic-Catalog
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must check `assets/semantic_catalog.json` and catalog service.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§2.6.7 Semantic Catalog).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented (capability filtering, resolution logic).
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Executive Summary
Implement the SemanticCatalog that resolves abstract tool requests to concrete engine operations with capability filtering. Load from `assets/semantic_catalog.json` at startup and ensure capability-aware resolution.

**Current State:**
- No enforced Semantic Catalog; tool resolution may be ad hoc or missing.

**End State:**
- SemanticCatalog struct and load path implemented per §2.6.7.
- Capability filtering enforced via CapabilityRegistry grants.
- Tests verifying catalog load and resolution with capability checks.

**Effort:** 6-10 hours  
**Phase 1 Blocking:** YES (Spec §2.6.7)

---

## Technical Contract (LAW)
Governed by Master Spec §2.6.7:
- Implement SemanticCatalog with ToolEntry fields: id, engine_id, operation, capability_required, schema_ref.
- Load from `assets/semantic_catalog.json` at startup; failure to load is an error.
- Enforce capability filtering when resolving tools for a user/session.

---

## Scope
### In Scope
1) SemanticCatalog data structures and loader.
2) Capability-aware resolution logic.
3) Tests for load + resolution + capability filtering.

### Out of Scope
- Authoring UI for catalog (Phase 2).
- Dynamic catalog updates/hot reload (Phase 2+).

---

## Quality Gate
- **RISK_TIER:** MEDIUM
- **DONE_MEANS:**
  - Catalog types and loader implemented per §2.6.7.
  - Capability filtering enforced; failing resolution surfaces typed errors.
  - Tests cover load + capability filtering.
  - No forbidden patterns (unwrap/expect/panic/dbg in prod).
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§2.6.7); src/backend/handshake_core/src; assets/semantic_catalog.json (create if missing)
- **SEARCH_TERMS:** "SemanticCatalog", "capability_required", "CapabilityRegistry", "schema_ref", "tools"
- **RUN_COMMANDS:** cargo test; just validator-spec-regression; just validator-hygiene-full
- **RISK_MAP:** "No capability filter -> unauthorized tool use"; "Missing catalog -> runtime failure"; "Schema mismatch -> parse errors"

---

## Success Metrics
| Metric | Target | Verification |
|--------|--------|--------------|
| Catalog load | Success at startup | Unit test |
| Capability filter | Only allowed tools resolved | Unit test |
| Typed errors | No stringly errors | Code evidence |

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>



## VALIDATION REPORT - WP-1-Semantic-Catalog (2025-12-26)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Semantic-Catalog.md (status: Ready for Dev; duplicate signature blocks with `<pending>` noted)
- Spec: Handshake_Master_Spec_v02.84.md (A2.6.7 Semantic Catalog Registry; 2.6.6.7.14.8 Semantic Catalog)

Files Checked:
- Handshake_Master_Spec_v02.84.md (Semantic Catalog sections around lines 5394-5416, 6188-6204)
- Repository search: `rg -n "SemanticCatalog" src app`, `rg -n "semantic_catalog" src app` (no hits)
- Repository root for assets/semantic_catalog.json (assets/ directory missing)

Findings:
- Packet anomalies: top block lists **Status: READY FOR DEV dY"'** and USER_SIGNATURE `<pending>` conflicting with later metadata. Per VALIDATOR_PROTOCOL the signature is not locked; pre-flight risk noted.
- Spec implementation: No `SemanticCatalog`, `ToolEntry`, `AgentEntry`, or routing rule structs exist in `src/` or `app/`. No loader from `assets/semantic_catalog.json`; no capability-gated resolution logic or typed errors. Required versioning/timestamp fields and store/query interface are absent.
- Assets: `assets/semantic_catalog.json` is missing; startup load invariant cannot be satisfied.
- Capability enforcement: No integration with `CapabilityRegistry` or gating of catalog queries, violating safety requirements in A2.6.6.7.14.8.
- Evidence mapping: None; no file:line locations satisfy any Semantic Catalog requirement.

Hygiene / Forbidden Patterns:
- Not run; validation blocked by missing implementation.

Tests:
- Not run; TEST_PLAN commands not executed and no Semantic Catalog code to exercise.

Risks & Suggested Actions:
- Add full Semantic Catalog implementation per A2.6.7: typed structs (tools/agents/routing_rules with version/timestamps), loader from `assets/semantic_catalog.json`, capability-gated resolution, and typed error codes. Create the assets file with schema-aligned entries.
- Integrate capability checks with `CapabilityRegistry` and add unit tests for load + resolution + gating. Provide evidence mapping (file:line) and rerun TEST_PLAN for re-validation.


