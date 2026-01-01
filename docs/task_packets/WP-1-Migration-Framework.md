# Task Packet: WP-1-Migration-Framework

## Metadata
- TASK_ID: WP-1-Migration-Framework
- DATE: 2025-12-25T17:29:00Z
- REQUESTOR: ilja
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- STATUS: Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja251220251729

---

## SCOPE

### Executive Summary

Rewrite all database migrations to use DB-agnostic SQL and implement a professional migration framework (`sqlx::migrate!`). This enforces Pillar 2 of Master Spec §2.3.12 (Portable Schema & Migrations).

**Guiding Principle (Postgres later, cheap):**
1) One storage API: force all DB access through a single module.  
2) Portable schema/migrations: clear schema and upgrade steps, DB-agnostic SQL.  
3) Treat indexes as rebuildable (recompute from artifacts, not migrated rows).  
4) Dual-backend tests early: run SQLite + Postgres in CI to keep retrofits medium-effort.

**End State:**
- All migration files use portable SQL syntax compatible with both SQLite and PostgreSQL.
- Handshake uses `sqlx::migrate!` for automated, versioned schema management.
- SQL portability rules are documented for future developers.

### IN_SCOPE_PATHS
- src/backend/handshake_core/migrations/*.sql
- src/backend/handshake_core/src/main.rs (Wired migration execution)
- src/backend/handshake_core/src/storage/mod.rs (Validation)
- src/backend/handshake_core/src/storage/sqlite.rs (Validation)
- docs/MIGRATION_GUIDE.md (New doc)

### OUT_OF_SCOPE
- PostgreSQL CI infrastructure (→ WP-1-Dual-Backend-Tests)
- Feature logic implementation (Done in WP-1-SAL)

---

## QUALITY GATE

- **RISK_TIER**: HIGH
- **TEST_PLAN**:
  ```bash
  # Verify migration execution
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  
  # Check SQL portability
  just validator-dal-audit
  
  # Full hygiene check
  just validator-hygiene-full
  
  # Workflow closure
  just post-work WP-1-Migration-Framework
  ```
- **DONE_MEANS**:
  - ✅ All migrations refactored to use `$n` placeholders (§2.3.12.2).
  - ✅ SQLite datetime functions (`strftime`) replaced with `CURRENT_TIMESTAMP` (§2.3.12.2).
  - ✅ SQLite-specific triggers removed; logic moved to `src/storage/` (§2.3.12.1).
  - ✅ `sqlx::migrate!` integrated into `main.rs` (§2.3.12.4).
  - ✅ `docs/MIGRATION_GUIDE.md` created with portability rules.
  - ✅ All existing tests pass.

---

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * src/backend/handshake_core/migrations/0001_init.sql
  * src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
  * src/backend/handshake_core/src/main.rs
  * docs/SPEC_CURRENT.md
- **SEARCH_TERMS**:
  * "strftime"
  * "?1", "?2"
  * "CREATE TRIGGER"
  * "sqlx::migrate"
  * "OLD.", "NEW."
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-dal-audit
  ```
- **RISK_MAP**:
  * "Database corruption" -> Inconsistent migration state
  * "PostgreSQL failure" -> SQLite-specific syntax leak
  * "Startup failure" -> Broken migration integration

---

## SKELETON
- Migration plan captured as: portable SQL ($n placeholders), TIMESTAMP defaults, triggers removed, sqlx::migrate! wiring.
- Implementation gate: requires `SKELETON APPROVED` prior to validation/implementation.

SKELETON APPROVED: 2025-12-25 (Orchestrator message)

## AUTHORITY
- **SPEC_ANCHOR**: §2.3.12.1, §2.3.12.2, §2.3.12.4
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220251729

---

## VALIDATION (Coder)
- Command: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` → PASS (warnings: existing dead_code warning in workflows.rs unchanged)
- Command: `just validator-dal-audit` → PASS
- Command: `just validator-hygiene-full` → PASS (traceability WARN only; tool reports warning-level)
- Command: `just post-work WP-1-Migration-Framework` → PASS (tool indicated AI review not required)

---

## VALIDATION REPORT — WP-1-Migration-Framework
**Verdict: PASS**

**Scope Inputs:**
- Task Packet: docs/task_packets/WP-1-Migration-Framework.md (status: Done)
- Spec: Handshake_Master_Spec_v02.84.md (§2.3.12)

**Files Checked:**
- `src/backend/handshake_core/migrations/0001_init.sql`
- `src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/main.rs`
- `docs/MIGRATION_GUIDE.md`

**Findings:**
- **Requirement §2.3.12.2 (Placeholders):** All migrations refactored to use `$n` placeholders. Verified in `0001_init.sql` and `0002_create_ai_core_tables.sql`.
- **Requirement §2.3.12.2 (Timestamps):** SQLite datetime functions (`strftime`) replaced with `TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP`. Verified in both migration files.
- **Requirement §2.3.12.1 (Triggers):** Zero SQLite-specific triggers found. Application-layer mutation tracking is now the authority.
- **Requirement §2.3.12.4 (Framework):** `sqlx::migrate!` integrated into `SqliteDatabase::run_migrations` and invoked in `main.rs`.
- **Hygiene:** `just validator-dal-audit` passes. `migrations/` folder is clean of SQLite-only syntax.
- **Forbidden Patterns:** No `?1`, `strftime(`, or `CREATE TRIGGER` detected in production paths.

**Tests:**
- `cargo test`: **PASS** (Success execution of existing workflow and health tests under the new migration framework).
- `just validator-dal-audit`: **PASS** (Verified zero leaks in migrations/).

**Risks & Suggested Actions:**
- **Risk:** Future migrations might introduce SQLite-specific PRAGMAs. 
- **Action:** CI MUST run `validator-dal-audit` on every PR.
- **Risk:** Differences in `CURRENT_TIMESTAMP` precision between SQLite (ms) and Postgres (µs).
- **Action:** Document precision constraints in `MIGRATION_GUIDE.md` if future logic depends on high-precision deltas.

**Improvements & Future Proofing:**
- **Code:** Removed the manual `schema_version` table, reducing technical debt and aligning with industry standards (`sqlx`).
- **Protocol:** Added `docs/MIGRATION_GUIDE.md` as "LAW" to prevent regression.
- **Next Step:** Implementation of `WP-1-Gate-Check-Tool` is mandatory to enforce turn-based sequential development.

**Status Update:** Moved to **Done** on Task Board.






