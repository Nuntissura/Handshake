# Task Packet: WP-1-Migration-Framework-v2

## METADATA
- TASK_ID: WP-1-Migration-Framework-v2
- WP_ID: WP-1-Migration-Framework-v2
- BASE_WP_ID: WP-1-Migration-Framework (stable ID without `-vN`)
- DATE: 2026-01-12T00:10:22.422Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja120120260049

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Migration-Framework-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Upgrade the migration framework to meet Master Spec v02.106 [CX-DBP-022]: (1) enforce heavy per-file replay-safe migrations (tracking-independent) and (2) require concrete down migrations in Phase 1, with tests proving both on SQLite and PostgreSQL.
- Why: Non-idempotent migrations can brick startup and block Phase 1 closure; Phase 1 acceptance criteria now requires forward/backward fixture tests (up+down) and replay-safety (spec [CX-DBP-022], Phase 1 closure [CX-DBP-030]).
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/migrations/*.sql
  - src/backend/handshake_core/migrations/*.down.sql (new)
  - src/backend/handshake_core/src/storage/sqlite.rs (migration runner)
  - src/backend/handshake_core/src/storage/postgres.rs (migration runner)
  - src/backend/handshake_core/src/storage/tests.rs (add migration replay/down harness)
  - docs/MIGRATION_GUIDE.md (reference; update only if required)
- OUT_OF_SCOPE:
  - New product features or schema expansions not required by existing migrations
  - Production rollback tooling/UI (down migrations are for dev/test/CI only in Phase 1)
  - Renaming migration versions (0001..0009) in a way that breaks existing `_sqlx_migrations` history

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Migration-Framework-v2

# Compile + tests (must include new migration replay + down tests)
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Postgres tests (requires env var):
# $env:POSTGRES_TEST_URL="postgres://user:pass@host:5432/dbname"
# cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# SQL portability audit for migrations/ (MIGRATION_GUIDE "LAW")
just validator-dal-audit

# Full hygiene (required before merge)
just validator-hygiene-full

just cargo-clean
just post-work WP-1-Migration-Framework-v2
```

### DONE_MEANS
- Every migration up file is replay-safe: applying all migrations twice in a row does not error on SQLite and PostgreSQL (spec [CX-DBP-022A]).
- Every migration has a concrete down migration, and down migrations can be applied in strict reverse order to return to baseline on SQLite and PostgreSQL (spec [CX-DBP-022B]).
- `just validator-dal-audit` passes (portable SQL invariants still enforced).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes (including new migration tests).
- `just post-work WP-1-Migration-Framework-v2` passes with a fully filled COR-701 VALIDATION manifest.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.106.md (recorded_at: 2026-01-12T00:10:22.422Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.106.md sections 2.3.12.4 (CX-DBP-022), 2.3.12.4.1 (CX-DBP-022A), 2.3.12.4.2 (CX-DBP-022B), and 2.3.12.5 (CX-DBP-030); Roadmap acceptance criteria "Migrations validated" (Handshake_Master_Spec_v02.106.md:20702)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md
- Approval: Task packet creation authorized by USER_SIGNATURE `ilja120120260049` (refinement approved; no spec enrichment proposed)

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet (locked history): docs/task_packets/WP-1-Migration-Framework.md
- Stub pointer that triggered this revision: docs/task_packets/stubs/WP-1-Migration-Framework-v2.md
- Preserved from prior packet:
  - Portable SQL invariants and `sqlx::migrate!` usage remain required.
  - No triggers, no sqlite-only datetime functions, `$n` placeholders, portable timestamps.
- Added/changed in this revision (spec v02.106 deltas):
  - Heavy per-file replay-safe requirement (tracking-independent): every migration file must be safe to run multiple times.
  - Concrete down migrations are mandatory in Phase 1 (not optional).
  - New tests: replay-safety test (up applied twice) and down test (reverse downs) on both SQLite and PostgreSQL.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.106.md (sections 2.3.12.4 and 2.3.12.5)
  - docs/MIGRATION_GUIDE.md
  - docs/task_packets/WP-1-Migration-Framework.md (prior packet; locked history)
  - src/backend/handshake_core/migrations/*.sql
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - scripts/validation/validator-dal-audit.mjs
- SEARCH_TERMS:
  - "sqlx::migrate!"
  - "_sqlx_migrations"
  - "run_migrations"
  - "CREATE TABLE IF NOT EXISTS"
  - "CREATE INDEX IF NOT EXISTS"
  - "ALTER TABLE"
  - "sqlite::memory:"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Migration-Framework-v2
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-dal-audit
  ```
- RISK_MAP:
  - "Non-idempotent DDL bricks startup" -> "App cannot boot; must ensure replay-safety and add tests that replay migrations"
  - "Down migrations destroy data" -> "Data loss if exposed in prod; keep down migrations for dev/test/CI only and do not wire a prod rollback path"
  - "SQLite/Postgres divergence" -> "Portability regressions; enforce validator-dal-audit and dual-backend tests for every migration change"

## SKELETON
- Proposed interfaces/types/contracts:
  - Migration replay harness: create a fresh DB, apply all up migrations twice, assert PASS (SQLite + PostgreSQL).
  - Down migration harness: create a fresh DB, apply all up migrations once, then apply all down migrations in strict reverse order, assert PASS (SQLite + PostgreSQL).
  - Keep `_sqlx_migrations` as the single schema version source of truth (no manual schema_version table).
- Open questions:
  - Confirm `sqlx` down migration file naming/loader behavior for the existing `000X_name.sql` format (MIGRATION_GUIDE currently requires `000X_name.down.sql`).
  - For any SQLite `ALTER TABLE ADD COLUMN` migrations, decide whether to: (a) rewrite as idempotent table rebuild migration, or (b) use a `sqlx`-level conditional migration strategy that remains "tracking-independent" per spec.
- Notes:
  - Do not edit historical/locked packets; this v2 packet is the only executable authority for the Base WP once activated in WP_TRACEABILITY_REGISTRY.
  - Treat down migrations as potentially destructive and keep them restricted to test/dev/CI usage in Phase 1.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
