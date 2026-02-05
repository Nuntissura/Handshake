# Work Packet: WP-1-AppState-Refactoring

**Status:** READY FOR DEV ðŸ”´  
**Authority:** Master Spec Â§2.3.12 (Pillar 1: One Storage API)  
**USER_SIGNATURE:** <pending>

---

## Executive Summary
Refactor AppState to expose `Arc<dyn Database>` instead of concrete `SqlitePool`/DuckDB connections; route all DB access through the storage module and the Database trait.

**Effort:** 8-10 hours  
**Phase 1 Blocking:** YES (storage portability gate)

**Guiding Principle (Postgres later, cheap):**
1) One storage API: force all DB access through a single module.  
2) Portable schema/migrations: clear schema and upgrade steps, DB-agnostic SQL.  
3) Treat indexes as rebuildable (recompute from artifacts, not migrated rows).  
4) Dual-backend tests early: run SQLite + Postgres in CI to keep retrofits medium-effort.

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
- **FILES_TO_OPEN:** .GOV/roles_shared/SPEC_CURRENT.md; Handshake_Master_Spec_v02.84.md (Â§2.3.12); src/backend/handshake_core/src/main.rs; src/backend/handshake_core/src/api; src/backend/handshake_core/src/storage; src/backend/handshake_core/src/workflows.rs
- **SEARCH_TERMS:** "AppState", "SqlitePool", "state.pool", "Database trait"
- **RUN_COMMANDS:** cargo test; just validator-dal-audit; just validator-hygiene-full
- **RISK_MAP:** "Hidden pool leakage -> portability failure"; "Refactor breakage -> handler errors"

---

**Last Updated:** 2025-12-25  
**User Signature Locked:** <pending>



## VALIDATION REPORT - WP-1-AppState-Refactoring (2025-12-26)
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-AppState-Refactoring.md (status: Ready for Dev; signature `<pending>`)
- Spec: Handshake_Master_Spec_v02.84.md (A2.3.12 One Storage API / portability gate)

Files Checked:
- src/backend/handshake_core/src/lib.rs (AppState)
- src/backend/handshake_core/src/main.rs (init_storage wiring)
- src/backend/handshake_core/src/storage/mod.rs (Database trait)

Findings:
- AppState surface: `AppState { storage: Arc<dyn Database>, fr_pool: Arc<Mutex<DuckDbConnection>> }` (lib.rs:15-24) shows storage is trait-based, but the WP contract also requires removal of concrete pool leakage across the boundary.
- Concrete pool leakage: Database trait still exposes `fn sqlite_pool(&self) -> Option<&SqlitePool>` (storage/mod.rs:288), violating the â€œno concrete pool on public surfacesâ€ rule in A2.3.12 and this WPâ€™s DONE_MEANS.
- Single-backend wiring: `init_storage` constructs only `SqliteDatabase` and returns `SqlitePool` (main.rs:29-72), coupling the runtime to SQLite and leaking the pool to Janitor. No evidence of Postgres portability or a backend-neutral AppState bootstrap.
- Evidence gaps: No proof that all handlers/services are free of direct pool usage beyond the trait leakage, and no DAL audit evidence was provided.

Hygiene / Forbidden Patterns:
- Not executed; validation stopped at portability/leakage violations.

Tests:
- Not run; TEST_PLAN commands not executed during this audit.

Risks & Suggested Actions:
- Remove `sqlite_pool` exposure from the Database trait and any public surfaces; keep pool access internal to storage implementations.
- Provide backend-neutral init (select backend via config) and add portability tests (dual-backend) to demonstrate A2.3.12 compliance.
- Re-run TEST_PLAN (`cargo test`, `just validator-dal-audit`, `just validator-hygiene-full`) and provide file:line evidence that all handlers rely solely on `Arc<dyn Database>`.



