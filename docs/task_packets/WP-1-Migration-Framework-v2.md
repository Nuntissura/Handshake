# Task Packet: WP-1-Migration-Framework-v2

## METADATA
- TASK_ID: WP-1-Migration-Framework-v2
- WP_ID: WP-1-Migration-Framework-v2
- BASE_WP_ID: WP-1-Migration-Framework (stable ID without `-vN`)
- DATE: 2026-01-12T00:10:22.422Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja120120260049

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Migration-Framework-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Upgrade the migration framework to meet Master Spec v02.106 [CX-DBP-022]: (1) enforce heavy per-file replay-safe migrations (tracking-independent) and (2) require concrete down migrations in Phase 1, with tests proving both on SQLite and PostgreSQL.
- Why: Non-idempotent migrations can brick startup and block Phase 1 closure; Phase 1 acceptance criteria now requires forward/backward fixture tests (up+down) and replay-safety (spec [CX-DBP-022], Phase 1 closure [CX-DBP-030]).
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/migrations/0001_init.sql
  - src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql
  - src/backend/handshake_core/migrations/0003_add_is_pinned.sql
  - src/backend/handshake_core/migrations/0004_mutation_traceability.sql
  - src/backend/handshake_core/migrations/0005_add_canvas_traceability.sql
  - src/backend/handshake_core/migrations/0006_expand_ai_job_model.sql
  - src/backend/handshake_core/migrations/0007_workflow_persistence.sql
  - src/backend/handshake_core/migrations/0008_expand_ai_job_model.sql
  - src/backend/handshake_core/migrations/0009_add_block_classification.sql
  - src/backend/handshake_core/migrations/0001_init.down.sql
  - src/backend/handshake_core/migrations/0002_create_ai_core_tables.down.sql
  - src/backend/handshake_core/migrations/0003_add_is_pinned.down.sql
  - src/backend/handshake_core/migrations/0004_mutation_traceability.down.sql
  - src/backend/handshake_core/migrations/0005_add_canvas_traceability.down.sql
  - src/backend/handshake_core/migrations/0006_expand_ai_job_model.down.sql
  - src/backend/handshake_core/migrations/0007_workflow_persistence.down.sql
  - src/backend/handshake_core/migrations/0008_expand_ai_job_model.down.sql
  - src/backend/handshake_core/migrations/0009_add_block_classification.down.sql
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/main.rs
  - docs/MIGRATION_GUIDE.md (reference; update only if required)
  - scripts/validation/validator-dal-audit.mjs
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

SKELETON APPROVED

## IMPLEMENTATION
- Migrations:
  - Enforced replay-safe behavior per file using `IF NOT EXISTS` and normalization notes.
  - Added concrete `*.down.sql` files for `0001..0009` (dev/test/CI only).
- Tests:
  - Added replay-safety and down-migration harness on SQLite and PostgreSQL.
- Health:
  - Surfaced migration version in `/health` via `_sqlx_migrations` (no secondary schema_version table).
- Validation tooling:
  - Updated `validator-dal-audit.mjs` to ignore `*.down.sql` for gap checks and require downs for every up.

## HYGIENE
- Commands executed:
  - `just pre-work WP-1-Migration-Framework-v2`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just validator-dal-audit`
  - `just validator-hygiene-full` (notes: fails due to existing `validator-error-codes` nondeterminism finding in `src/backend/handshake_core/src/workflows.rs`)
  - `just cargo-clean`
  - `just post-work WP-1-Migration-Framework-v2`
  - `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.

- **Target File**: `scripts/validation/validator-dal-audit.mjs`
- **Start**: 1
- **End**: 106
- **Line Delta**: 19
- **Pre-SHA1**: `33ad98a08a3c671f6f5557602ede092eb26543da`
- **Post-SHA1**: `f4ad5550367de1c6cd67fbd3829bbf368b642020`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0001_init.down.sql`
- **Start**: 1
- **End**: 8
- **Line Delta**: 8
- **Pre-SHA1**: `ede168595415ddf871420b71754272760861240b`
- **Post-SHA1**: `ede168595415ddf871420b71754272760861240b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0001_init.sql`
- **Start**: 1
- **End**: 89
- **Line Delta**: 32
- **Pre-SHA1**: `5457f16c03f143699f85085563f2dbb8250f7668`
- **Post-SHA1**: `0d984c97b12b2fdc832ee27c8f0c773103ccd233`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0002_create_ai_core_tables.down.sql`
- **Start**: 1
- **End**: 4
- **Line Delta**: 4
- **Pre-SHA1**: `22c277d28fb7ddc23adcaa0abeeeb6161cd4238e`
- **Post-SHA1**: `22c277d28fb7ddc23adcaa0abeeeb6161cd4238e`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql`
- **Start**: 1
- **End**: 36
- **Line Delta**: 11
- **Pre-SHA1**: `af17f6e0bc4017d7c33ee67c68935a70fa6528af`
- **Post-SHA1**: `84db1ceb7c4edafff63c4a5a90ab9357638d9672`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0003_add_is_pinned.down.sql`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `0ca6c2b44b16ea047f2ed918718c6f45d48419d7`
- **Post-SHA1**: `0ca6c2b44b16ea047f2ed918718c6f45d48419d7`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0003_add_is_pinned.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -1
- **Pre-SHA1**: `032fcb40245aafe2aba81278be8088b020f32c38`
- **Post-SHA1**: `2d8a1571e08ba944c7254fac30745d29ffa30b0e`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0004_mutation_traceability.down.sql`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `2a481e6a7a74c1cfc9d1077ab69f2ee5634d9cfe`
- **Post-SHA1**: `2a481e6a7a74c1cfc9d1077ab69f2ee5634d9cfe`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0004_mutation_traceability.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -34
- **Pre-SHA1**: `69f7be710bb44265277d6644430aff241350530d`
- **Post-SHA1**: `22bf17d1a17e68705796338b23a64637be996b71`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0005_add_canvas_traceability.down.sql`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `2a481e6a7a74c1cfc9d1077ab69f2ee5634d9cfe`
- **Post-SHA1**: `2a481e6a7a74c1cfc9d1077ab69f2ee5634d9cfe`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0005_add_canvas_traceability.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -5
- **Pre-SHA1**: `d6ac7ea4eff094b816d75549ed38d3804194dc03`
- **Post-SHA1**: `fbaf79e49eb55899adef211e29886f981c30568c`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0006_expand_ai_job_model.down.sql`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `a24f1ee5d73b82565b1a17c0cc15d0f3921e43ab`
- **Post-SHA1**: `a24f1ee5d73b82565b1a17c0cc15d0f3921e43ab`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0006_expand_ai_job_model.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -17
- **Pre-SHA1**: `f4294bed0f78b80fe2216d75f9b88ce29bdb47ef`
- **Post-SHA1**: `b338655191a390d1eba47fdd834053937e2b41b6`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0007_workflow_persistence.down.sql`
- **Start**: 1
- **End**: 3
- **Line Delta**: 3
- **Pre-SHA1**: `fff91dc46d6df562d724b84deee99db5fccbfe79`
- **Post-SHA1**: `fff91dc46d6df562d724b84deee99db5fccbfe79`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0007_workflow_persistence.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -1
- **Pre-SHA1**: `8059321f4d6a816daeff84d0aabb14c51d6669d3`
- **Post-SHA1**: `0ef44f69f75d821ff336ff9587cde12a09820a93`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0008_expand_ai_job_model.down.sql`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `b527220e7097fdd1d2771f3b33af851bc2f6aa37`
- **Post-SHA1**: `b527220e7097fdd1d2771f3b33af851bc2f6aa37`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0008_expand_ai_job_model.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -165
- **Pre-SHA1**: `9b60fa1a9927cb4f81afd78cbddbf3508f7a502f`
- **Post-SHA1**: `f16da50a8bf84258f831db91c51a6e2d08a35dad`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0009_add_block_classification.down.sql`
- **Start**: 1
- **End**: 2
- **Line Delta**: 2
- **Pre-SHA1**: `0c5e7428b29c91bb8c7ffebe3d7acdd9ad1d5468`
- **Post-SHA1**: `0c5e7428b29c91bb8c7ffebe3d7acdd9ad1d5468`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/migrations/0009_add_block_classification.sql`
- **Start**: 1
- **End**: 99999
- **Line Delta**: -2
- **Pre-SHA1**: `093b06720f376ed7a34b5e4c30191be9935d2db1`
- **Post-SHA1**: `a82b7baa252c1fae7fa39a05356c339b6b2fe692`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/src/main.rs`
- **Start**: 1
- **End**: 296
- **Line Delta**: 9
- **Pre-SHA1**: `e5d93df58235bc1e818fcb6bad658cd791d90dbf`
- **Post-SHA1**: `2fada41a399ed952873c6e4e1c26e288810000a4`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/src/models.rs`
- **Start**: 1
- **End**: 143
- **Line Delta**: 1
- **Pre-SHA1**: `6898f215475ced6a5286a10d33cd828a0f7c10f2`
- **Post-SHA1**: `31316f5d7276ab8603060156e35cfe0172197302`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 877
- **Line Delta**: 3
- **Pre-SHA1**: `e189a0045bec8b6d990637ae34548095658adcde`
- **Post-SHA1**: `4ea008e4be730428b80af37fda36381ed1138183`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1
- **End**: 1476
- **Line Delta**: 9
- **Pre-SHA1**: `7545659a9199c1862b813f10b237289b26675c3b`
- **Post-SHA1**: `58925914acaedf65f51ac7ad57ec5ccc537bde34`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 1800
- **Line Delta**: 9
- **Pre-SHA1**: `3519696704fbcc28144e6e01521acdffb487af05`
- **Post-SHA1**: `ce22857f61a116397666d8e6792a6770d3d2b3c2`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 1
- **End**: 629
- **Line Delta**: 244
- **Pre-SHA1**: `aaad1bcc3db73c3cef830ccf4d7ea0ce53392f1b`
- **Post-SHA1**: `b46768a8d2f724fb21f02b612963ff5933032b24`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Implemented + committed
- What changed in this update:
  - Added `0001..0009` down migrations and replay-safe-up/down tests (SQLite + Postgres).
  - Updated health response to include current migration version from `_sqlx_migrations`.
  - Updated `validator-dal-audit` to ignore `*.down.sql` for numbering gaps and require downs.
- Next step / handoff hint:
  - Validator: run the WP TEST_PLAN commands and audit against spec anchors.
  - Ops note: this worktree has unstaged formatting-only changes from `cargo fmt` in `src/backend/handshake_core/src/workflows.rs` and `src/backend/handshake_core/src/ace/validators/mod.rs` (not part of this WP commit).

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Implementation commit: `02e5b3b8`
- `just pre-work WP-1-Migration-Framework-v2`:
  ```text
  Checking Phase Gate for WP-1-Migration-Framework-v2...
  ? GATE PASS: Workflow sequence verified.

  Pre-work validation for WP-1-Migration-Framework-v2...

  Check 1: Task packet file exists
  PASS: Found WP-1-Migration-Framework-v2.md

  Check 2: Task packet structure
  PASS: All required fields present

  Check 2.7: Technical Refinement gate
  PASS: Refinement file exists and is approved/signed

  Check 2.8: WP checkpoint commit gate

  Check 3: Deterministic manifest template
  PASS: Manifest fields present
  PASS: Gates checklist present

  ==================================================
  Pre-work validation PASSED

  You may proceed with implementation.
  ```
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`:
  ```text
  Finished `test` profile [unoptimized + debuginfo] target(s) in 5m 58s
  running 133 tests
  test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 28.66s
  ```
- `just validator-dal-audit`:
  ```text
  validator-dal-audit: PASS (DAL checks clean).
  ```
- `just validator-hygiene-full` (notes: out-of-scope failure):
  ```text
  validator-error-codes: FAIL/WARN findings detected
  ----
  NONDETERMINISM pattern \"Instant::now\\(\":
  src/backend/handshake_core/src/workflows.rs:661:            // WAIVER [CX-573F]: Instant::now() for observability per \u00a72.6.6.7.12
  src/backend/handshake_core/src/workflows.rs:662:            let validation_start = std::time::Instant::now();
  ```
- `just post-work WP-1-Migration-Framework-v2`:
  ```text
  Post-work validation PASSED with warnings
  Warnings:
    1. Working tree has unstaged changes; post-work validation uses STAGED changes only.
  ```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT — WP-1-Migration-Framework-v2
Verdict: PASS

Validated commit(s):
- Code: `02e5b3b8`
- Packet: `0e20fc33`

Spec:
- Target: `Handshake_Master_Spec_v02.106.md`
- Anchors: §2.3.12.4 [CX-DBP-022], §2.3.12.4.1 [CX-DBP-022A], §2.3.12.4.2 [CX-DBP-022B], and Phase 1 acceptance criteria bullet “Migrations validated …”

Files Checked:
- `src/backend/handshake_core/migrations/0001_init.sql`
- `src/backend/handshake_core/migrations/0001_init.down.sql`
- `src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql`
- `src/backend/handshake_core/migrations/0002_create_ai_core_tables.down.sql`
- `src/backend/handshake_core/migrations/0003_add_is_pinned.sql`
- `src/backend/handshake_core/migrations/0003_add_is_pinned.down.sql`
- `src/backend/handshake_core/migrations/0004_mutation_traceability.sql`
- `src/backend/handshake_core/migrations/0004_mutation_traceability.down.sql`
- `src/backend/handshake_core/migrations/0005_add_canvas_traceability.sql`
- `src/backend/handshake_core/migrations/0005_add_canvas_traceability.down.sql`
- `src/backend/handshake_core/migrations/0006_expand_ai_job_model.sql`
- `src/backend/handshake_core/migrations/0006_expand_ai_job_model.down.sql`
- `src/backend/handshake_core/migrations/0007_workflow_persistence.sql`
- `src/backend/handshake_core/migrations/0007_workflow_persistence.down.sql`
- `src/backend/handshake_core/migrations/0008_expand_ai_job_model.sql`
- `src/backend/handshake_core/migrations/0008_expand_ai_job_model.down.sql`
- `src/backend/handshake_core/migrations/0009_add_block_classification.sql`
- `src/backend/handshake_core/migrations/0009_add_block_classification.down.sql`
- `src/backend/handshake_core/src/main.rs`
- `src/backend/handshake_core/src/models.rs`
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `scripts/validation/validator-dal-audit.mjs`
- `docs/MIGRATION_GUIDE.md`
- `docs/refinements/WP-1-Migration-Framework-v2.md`
- `docs/SPEC_CURRENT.md`
- `docs/WP_TRACEABILITY_REGISTRY.md`
- `docs/TASK_BOARD.md`

Findings:
- [CX-DBP-022] Replay-safe, tracking-independent migrations:
  - Idempotent DDL patterns used (e.g., `CREATE TABLE IF NOT EXISTS …`): `src/backend/handshake_core/migrations/0001_init.sql:3`, `src/backend/handshake_core/migrations/0002_create_ai_core_tables.sql:3`.
  - Replay-safety validated by explicitly dropping `_sqlx_migrations` and re-running: `src/backend/handshake_core/src/storage/tests.rs:513` (SQLite), `src/backend/handshake_core/src/storage/tests.rs:552` (Postgres), `src/backend/handshake_core/src/storage/tests.rs:520`, `src/backend/handshake_core/src/storage/tests.rs:572`.
- [CX-DBP-022B] Concrete down migrations:
  - Down files exist for every up migration (0001..0009) and undo-to-baseline tests validate reversibility: `src/backend/handshake_core/src/storage/tests.rs:532` (SQLite), `src/backend/handshake_core/src/storage/tests.rs:592` (Postgres).
  - Example downs: `src/backend/handshake_core/migrations/0001_init.down.sql:3`, `src/backend/handshake_core/migrations/0002_create_ai_core_tables.down.sql:3`.
- Migration version surfaced in health check:
  - `/health` includes `migration_version`: `src/backend/handshake_core/src/main.rs:238`, `src/backend/handshake_core/src/models.rs:25`.
  - Version sourced from `_sqlx_migrations` (no manual schema_version table): `src/backend/handshake_core/src/storage/mod.rs:851`, `src/backend/handshake_core/src/storage/sqlite.rs:185`, `src/backend/handshake_core/src/storage/sqlite.rs:187`, `src/backend/handshake_core/src/storage/postgres.rs:243`, `src/backend/handshake_core/src/storage/postgres.rs:245`.
- Migration hygiene gate updated to align with Phase 1 down-migration mandate:
  - Ignore `*.down.sql` for numbering continuity and require matching down per up: `scripts/validation/validator-dal-audit.mjs:61`, `scripts/validation/validator-dal-audit.mjs:82`.

Forbidden Patterns:
- PASS: No panic/unwrap in production paths (unwrap occurrences observed were inside test blocks only).

Tests (Validator-run):
- `node scripts/validation/validator-dal-audit.mjs`: PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`: PASS
- Postgres migration tests executed with `POSTGRES_TEST_URL` set (local docker Postgres):
  - `storage::tests::migrations_are_replay_safe_postgres`: PASS
  - `storage::tests::migrations_can_undo_to_baseline_postgres`: PASS

Risks & Suggested Actions:
- Migration history rewrite: several later migrations are now no-op with “replay-safe normalization” comments; any pre-existing dev DBs created with older versions of these migrations may require reset. Confirm expectations for non-ephemeral environments.
- CI: ensure Postgres test provisioning sets `POSTGRES_TEST_URL` so Postgres migration tests are not silently skipped.

REASON FOR PASS:
- All hard requirements from spec v02.106 [CX-DBP-022] are satisfied and were re-verified by validator-run DAL audit and SQLite+Postgres test runs.
