# Work Packet: WP-1-Dual-Backend-Tests

**Status:** READY FOR DEV 🔴  
**Authority:** Master Spec §2.3.12 (Pillar 4: Dual-Backend Testing)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Add PostgreSQL test variant and parameterize storage tests to run on SQLite and PostgreSQL in CI.

**Effort:** 8-10 hours  
**Phase 1 Blocking:** YES (storage portability gate)

---

## Scope
### In Scope
1) Docker/PostgreSQL test service for CI/local.
2) Parameterize storage tests to run against SQLite and Postgres.
3) CI job that fails if either backend fails.

### Out of Scope
- Migration rewrites (WP-1-Migration-Framework).
- AppState refactor (WP-1-AppState-Refactoring).

---

## Quality Gate
- **RISK_TIER:** HIGH
- **DONE_MEANS:**
  - Storage tests run on SQLite and PostgreSQL in CI.
  - CI fails if either backend fails.
  - DAL audit (CX-DBP-VAL-014) passes (Postgres hints/tests present).
  - Tests pass.
- **TEST_PLAN:**
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-spec-regression`
  - `just validator-dal-audit`
  - `just validator-hygiene-full`

---

## BOOTSTRAP
- **FILES_TO_OPEN:** docs/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (§2.3.12); src/backend/handshake_core/src; CI config
- **SEARCH_TERMS:** "Postgres", "PgPool", "docker-compose", "sqlx test"
- **RUN_COMMANDS:** cargo test; just validator-dal-audit; just validator-hygiene-full
- **RISK_MAP:** "CI flakiness from DB startup"; "Tests not parameterized -> coverage gap"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>
