## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Migration-Framework-v2
- CREATED_AT: 2026-01-11T23:43:09.9622311Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.106.md
- SPEC_TARGET_SHA1: ffa0e7b5469d2ccba2c56ef62be9cbd4be4256a3
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja120120260049
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Migration-Framework-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Implementation gap: existing migrations (0002..0009) are not replay-safe if executed multiple times (non-idempotent CREATE/ALTER/INDEX; destructive rebuild in 0008).
- Implementation gap: no `*.down.sql` files exist yet; Phase 1 now requires concrete down migrations for every up migration.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (migration apply/revert uses existing backend logging; Flight Recorder event coverage is deferred).

### RED_TEAM_ADVISORY (security failure modes)
- Availability risk: replaying non-idempotent migrations can block startup and brick environments.
- Integrity risk: table rebuild migrations (rename/copy/drop) can lose data or constraints if any column/index/foreign key is missed.
- Safety risk: down migrations can be destructive; do not expose revert in production paths and keep revert tooling restricted to dev/test/CI.

### PRIMITIVES (traits/structs/enums)
- Migration replay harness: apply up migrations twice and assert schema stability (SQLite + Postgres).
- Down migration harness: apply downs in strict reverse order and assert baseline restored (SQLite + Postgres).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec v02.106 explicitly defines heavy per-file replay safety (CX-DBP-022) and mandates down migrations in Phase 1, with concrete test requirements (CX-DBP-022A/B).
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec v02.106 already contains the required normative text for heavy per-file replay-safe migrations and mandatory down migrations; this WP is implementation-only.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.106.md 2.3.12.4 [CX-DBP-022]
- CONTEXT_START_LINE: 3086
- CONTEXT_END_LINE: 3119
- CONTEXT_TOKEN: Migration Framework Requirements
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.12.4 Migration Framework Requirements [CX-DBP-022]

  - REQUIRED (Heavy per-file): Each migration file MUST be replay-safe (tracking-independent).
  - REQUIRED: Concrete rollback via down migrations is mandatory in Phase 1 (NOT optional).
  - REQUIRED: Schema versioning tracked in database (`_sqlx_migrations` qualifies as equivalent; do not maintain a second manual schema_version table)
  - REQUIRED: Migrations tested on both SQLite and PostgreSQL before merge
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.106.md 2.3.12.5 [CX-DBP-030]
- CONTEXT_START_LINE: 3120
- CONTEXT_END_LINE: 3150
- CONTEXT_TOKEN: WP-1-Migration-Framework
- EXCERPT_ASCII_ESCAPED:
  ```text
  3. **WP-1-Migration-Framework**
     - Enforce replay-safe migrations (heavy per-file) and tests (2.3.12.4.1)
     - Require concrete down migrations and tests (2.3.12.4.2)
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.106.md Acceptance criteria (Migration validation)
- CONTEXT_START_LINE: 20699
- CONTEXT_END_LINE: 20703
- CONTEXT_TOKEN: Migrations validated:
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Migrations validated: forward/backward fixture tests pass (up + down), replay-safety test passes (replay all up migrations), and migration version surfaces in a health check.
  ```

