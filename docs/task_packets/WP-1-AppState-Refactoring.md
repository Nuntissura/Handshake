# Work Packet: WP-1-AppState-Refactoring

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §2.3.12 (Pillar 1: One Storage API)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Refactor AppState to expose `Arc<dyn Database>` instead of concrete `SqlitePool`/DuckDB connections; route all DB access through the storage module and the Database trait.

**Effort:** 8-10 hours  
**Phase 1 Blocking:** YES (storage portability gate)

---

## Scope
### In Scope
1) AppState refactor to store `Arc<dyn Database>`.
2) Update API handlers/services to consume Database trait (no direct pool access).
3) Remove concrete pool leakage from public surfaces.

### Out of Scope
- Migration rewrites (handled in WP-1-Migration-Framework).
- Dual-backend tests (handled in WP-1-Dual-Backend-Tests).

---

## Quality Gate
- **RISK_TIER:** HIGH
- **DONE_MEANS:**
  - AppState exposes only `Arc<dyn Database>` (no SqlitePool/DuckDB exposure).
  - All handlers/services use Database trait; no direct `state.pool`/sqlx outside storage/.
  - DAL audit (CX-DBP-VAL-010..012) passes for boundary/trait leakage.
  - Tests pass.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-dal-audit`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§2.3.12); src/backend/handshake_core/src/main.rs; src/backend/handshake_core/src/api; src/backend/handshake_core/src/storage; src/backend/handshake_core/src/workflows.rs
- **SEARCH_TERMS:** "AppState", "SqlitePool", "state.pool", "Database trait"
- **RUN_COMMANDS:** cargo test; just validator-dal-audit; just validator-hygiene-full
- **RISK_MAP:** "Hidden pool leakage -> portability failure"; "Refactor breakage -> handler errors"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
