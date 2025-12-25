# Work Packet: WP-1-Migration-Framework

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §2.3.12 (Pillar 2: Portable Schema & Migrations)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Rewrite migrations and migration tooling to be portable (SQLite + PostgreSQL): eliminate SQLite-specific syntax, add schema versioning/idempotency, and document rules.

**Effort:** 10-12 hours  
**Phase 1 Blocking:** YES (storage portability gate)

---

## Scope
### In Scope
1) Rewrite existing migrations with portable SQL (use $1 placeholders; no strftime/triggers).
2) Add schema_version table/versioning and idempotency guidelines.
3) Document migration rules (portable SQL, naming, timestamps).

### Out of Scope
- Dual-backend CI (handled in WP-1-Dual-Backend-Tests).
- AppState refactor (handled in WP-1-AppState-Refactoring).

---

## Quality Gate
- **RISK_TIER:** HIGH
- **DONE_MEANS:**
  - All migrations use portable SQL (no `?1`, `strftime`, SQLite triggers).
  - Schema versioning/idempotency documented and implemented.
  - DAL audit (CX-DBP-VAL-011/013) passes.
  - Tests pass.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-dal-audit`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§2.3.12); src/backend/handshake_core/migrations; src/backend/handshake_core/src/workflows.rs
- **SEARCH_TERMS:** "migrations", "strftime", "?1", "schema_version"
- **RUN_COMMANDS:** cargo test; just validator-dal-audit; just validator-hygiene-full
- **RISK_MAP:** "SQLite syntax remains -> portability fail"; "Missing versioning -> drift"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
