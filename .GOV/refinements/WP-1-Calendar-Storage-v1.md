## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Calendar-Storage-v1
- REFINEMENT_FORMAT_VERSION: 2026-03-06
- CREATED_AT: 2026-03-06T08:26:50.861Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- SPEC_TARGET_SHA1: d01677a72b79523fef93b6d4072ebc5e0ec4b019
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060320260955
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Calendar-Storage-v1
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE in the Master Spec Main Body: calendar mutation governance, canonical raw entities, storage/indexing shape, portable migrations, and dual-backend testing are explicitly specified in Handshake_Master_Spec_v02.141.md.
- Implementation gap: there is no calendar runtime code under `src/backend/handshake_core/src` or `app`, so this packet is greenfield rather than remediation of existing product code.
- Storage gap: `src/backend/handshake_core/src/storage/mod.rs` exposes no calendar-specific `Database` trait methods yet, so calendar DAL primitives must be added for both SQLite and Postgres.
- Migration gap: `src/backend/handshake_core/migrations/` currently has no calendar tables, and existing SQLite runtime DDL upgrades in `src/backend/handshake_core/src/storage/sqlite.rs` must not be copied into this WP because the spec requires portable, version-managed DDL.
- Verification gap: no dual-backend tests currently exercise calendar schema replay safety, CRUD, or idempotent `(source_id, external_id)` upsert behavior.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 45m
- SEARCH_SCOPE: Master Spec calendar clauses, storage portability clauses, current `Database` trait, SQLite/Postgres storage implementations, migration tests, and Flight Recorder event patterns in Loom and media workflow code.
- REFERENCES:
  - Handshake_Master_Spec_v02.141.md sections 2.0.4, 2.1, 2.3, and 2.3.13.1.
  - `src/backend/handshake_core/src/storage/mod.rs`
  - `src/backend/handshake_core/src/storage/sqlite.rs`
  - `src/backend/handshake_core/src/storage/postgres.rs`
  - `src/backend/handshake_core/src/storage/tests.rs`
  - `src/backend/handshake_core/src/api/loom.rs`
  - `src/backend/handshake_core/src/workflows.rs`
- PATTERNS_EXTRACTED:
  - Add new persistence through the `Database` trait first, then implement both backends behind the trait boundary.
  - Introduce schema through `sqlx::migrate!` migrations plus replay-safe/undo-safe tests; do not rely on runtime `ALTER TABLE` paths for new feature work.
  - Use idempotent write semantics keyed by business identity, matching the spec's `(source_id, external_id)` dedupe rule.
  - Mirror existing Flight Recorder patterns by recording typed events at API/workflow boundaries rather than inside raw row mappers.
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT a portable relational schema plus dual-backend migration/CRUD tests; ADAPT Loom-style typed storage API patterns for calendar queries; REJECT direct SQL from API/UI code, runtime DDL for new calendar tables, and SQLite-only migration syntax.
- LICENSE/IP_NOTES: Local repository patterns only. No external code reuse is proposed.
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: The Master Spec already defines the calendar storage contract, mutation governance, and portability/testing constraints with enough specificity to create a packet without new normative text.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- This WP does not add a new user-facing workflow or mechanical engine by itself, so no new Flight Recorder event type must be introduced in this packet.
- The storage design must preserve downstream governance hooks required by the spec:
  - calendar writes remain job-backed patch sets, not direct API/UI writes.
  - successful calendar mutations later need `calendar_mutation` spans linked to `job_id`.
  - future `calendar_sync` runs will need stable source/event identifiers, dedupe keys, and provenance-preserving payload storage.
- Alignment target for later implementation packets:
  - `src/backend/handshake_core/src/api/loom.rs` shows typed FR events at API boundaries (`loom_block_created`, `loom_view_queried`, `loom_search_executed`).
  - `src/backend/handshake_core/src/workflows.rs` shows job-scoped FR payloads for long-running import/sync style work (`media_downloader.job_state`, `media_downloader.progress`, `media_downloader.item_result`).

### RED_TEAM_ADVISORY (security failure modes)
- Privacy leak risk: calendar data is high-sensitivity personal/work data. Raw payload, descriptions, attendees, and location fields must not leak into logs, diagnostics, or unscoped exports.
- Write-bypass risk: any direct SQL path from API/UI code would violate the explicit calendar write gate and bypass capability, workflow, and audit controls.
- Portability drift risk: copying SQLite runtime DDL or SQLite-specific SQL into new migrations will create backend divergence and future Postgres breakage.
- Identity drift risk: failure to enforce `(source_id, external_id)` idempotency and stable `instance_key` semantics will duplicate events and corrupt recurrence behavior.
- Time semantics risk: mishandling `tzid`, floating-time normalization, or recurrence override storage will cause silent user-visible corruption.
- Evidence drift risk: if schema omits provenance hooks such as raw source payload retention, job-linked mutation metadata, or stable source identifiers, downstream validation and replay become impossible.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-Database
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
  - PRIM-CalendarMutation
  - PRIM-CalendarSyncInput
  - PRIM-FlightRecorder
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - NONE
- NOTES:
  - `Database` will need calendar-specific list/get/upsert/query/delete methods for sources and events.
  - SQLite/Postgres row mappers and conversion helpers will need new structs mirroring canonical calendar entities.
  - Portable migrations should introduce calendar tables and indexes only; workflow/event emission stays in later packets.

### PRIMITIVE_INDEX (Appendix 12.4: HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX)
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- PRIMITIVE_INDEX_REASON_NO_CHANGE: The spec already defines calendar primitives at the Main Body level; this refinement does not require an Appendix 12.4 update before packet creation.
- PRIMITIVE_INDEX_UPDATE_NOTES:
  - NONE

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- Required rubric lines (one per pillar; do not delete lines, fill values):
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: Storage must preserve job-linked provenance and future `calendar_mutation` / `calendar_sync` auditability, even though this packet does not emit events directly. | STUB_WP_IDS: WP-1-Calendar-Sync-Engine-v1
  - PILLAR: Calendar | STATUS: TOUCHED | NOTES: This is the foundational persistence layer for CalendarSource and CalendarEvent and unblocks the rest of the calendar family. | STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1
  - PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Locus | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Work packets (product, not repo) | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Task board (product, not repo) | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: MicroTask | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Command Center | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Spec to prompt | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: New calendar storage must ship with portable DDL and dual-backend test coverage from day one. | STUB_WP_IDS: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: Canonical source/event rows, stable IDs, and provenance-preserving payload storage are prerequisites for later retrieval and summarization layers. | STUB_WP_IDS: WP-1-Calendar-Lens-v3
  - PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: Atelier/Lens | STATUS: TOUCHED | NOTES: No UI is in scope here, but queryable time-window storage and stable IDs are required to make Calendar Lens viable. | STUB_WP_IDS: WP-1-Calendar-Lens-v3
  - PILLAR: Skill distillation / LoRA | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: ACE | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
  - PILLAR: RAG | STATUS: NOT_TOUCHED | NOTES: NONE | STUB_WP_IDS: NONE
- PILLAR_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX (high-ROI combos; cross-primitive / cross-feature)
- MATRIX_SCAN_TIMEBOX: 20m
- MATRIX_SCAN_NOTES:
  - Highest-ROI interaction is not a new appendix edge; it is the explicit dependency chain Calendar Storage -> Calendar Sync Engine / Calendar Policy Integration / Calendar Lens.
  - Existing spec already captures `CalendarEvent` <-> `ActivitySpan` and policy/profile interactions; this packet should not broaden scope into those layers.
  - No local/cloud model compatibility changes are needed in this packet because it is storage-only.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: NONE
- Candidate interaction edges to add/update in Spec Appendix 12.6 (HS-APPX-INTERACTION-MATRIX):
  - Edge: NONE
  - Kind: NONE
  - ROI: LOW
  - Effort: LOW
  - Spec refs: NONE
  - In-scope for this WP: NO
  - If NO: create a stub WP and record it in TASK_BOARD Stub Backlog (order is not priority).
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- PRIMITIVE_MATRIX_REASON: The important downstream interactions already exist as separate calendar stubs and do not require a new appendix interaction edge for packet creation.

### UI_UX_RUBRIC (early UI/UX thinking; prefer too many controls early)
- UI_UX_APPLICABLE: NO
- UI_UX_REASON_NO: This packet is storage-layer only. User-visible surfaces belong to WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, and WP-1-Calendar-Policy-Integration-v1.
- UI_SURFACES:
  - NONE
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: NONE | Type: NONE | Tooltip: NONE | Notes: Storage-only packet
- UI_STATES (empty/loading/error):
  - NONE
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - NONE
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
- UI_UX_VERDICT: OK

### ROADMAP_PHASE_SPLIT (only if scope must be phased)
- PHASE_SPLIT_NEEDED: NO
- If YES: update the Roadmap (Spec 7.6) using the fixed per-phase fields below (do not invent new per-phase block types).
- Per phase, include exactly:
  - Goal:
  - MUST deliver:
  - Key risks addressed in Phase n:
  - Acceptance criteria:
  - Explicitly OUT of scope:
  - Mechanical Track:
  - Atelier Track:
  - Distillation Track:
  - Vertical slice:

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The current Master Spec explicitly defines calendar mutation governance, raw entities, temporal and recurrence invariants, relational storage/indexing shape, and storage portability/testing rules. This packet is implementation work, not speculative spec design.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The governing calendar clauses plus the portable-migration and dual-backend requirements are explicit enough to create a packet without changing the spec.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.0.4 Mutation governance (Hard Invariant) [ilja251220250127]
- CONTEXT_START_LINE: 54605
- CONTEXT_END_LINE: 54607
- CONTEXT_TOKEN: calendar_mutation
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **[HSK-CAL-WRITE-GATE]:** Direct database writes to `calendar_events` are **PROHIBITED** from the API layer or UI components.
  - All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
  - Every successful mutation MUST emit a `Flight Recorder` span of type `calendar_mutation` with a back-link to the `job_id`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.0.4 Mutation and governance rules
- CONTEXT_START_LINE: 54624
- CONTEXT_END_LINE: 54627
- CONTEXT_TOKEN: Patch-sets are the only write primitive
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Patch-sets are the only write primitive:** all calendar writes (local or external) are expressed as validated patch-sets with:
    - preconditions (`expected_etag`, `expected_local_rev`)
    - effect (`set`, `unset`, `append`, `remove`)
    - provenance (`job_id`, `client_op_id`, `idempotency_key`)
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.1 Raw entities
- CONTEXT_START_LINE: 54639
- CONTEXT_END_LINE: 54674
- CONTEXT_TOKEN: CalendarSource (RawContent)
- EXCERPT_ASCII_ESCAPED:
  ```text
  CalendarEvent (RawContent)
  - id (RID)
  - workspace_id
  - source_id (CalendarSource.id, e.g. "local", "google:...", "ics:...")
  - external_id (nullable; provider-specific event id)
  - external_etag (nullable; for conflict detection)
  - title
  - description
  - start_ts (timestamp + timezone)
  - end_ts (timestamp + timezone)
  - all_day (bool)
  - recurrence_rule (RRULE string, optional)
  - location (free text)
  - status (confirmed | tentative | cancelled)
  - visibility (public | private | busy_only)
  - export_mode (local_only | busy_only | full_export)
  - attendees[] (ParticipantRef)
  - links[] (EntityLinkRef -> doc, canvas, task, mail_thread, etc.)
  - created_by (User/Agent RID)
  - created_at
  - updated_at

  CalendarSource (RawContent)
  - id: "local:<id>" | "google:<account_id>:<calendar_id>" | "ics:<url>" | ...
  - type: "local" | "google" | "ics" | "caldav" | "other"
  - label: "Local", "Google / Personal", "Google / Work", ...
  - connection_config_ref (credential/secret reference)
  - google_calendar_id (for Google sources; e.g. "primary" or explicit id)
  - sync_state:
      - sync_token (nullable, provider-specific)
      - last_synced_at
      - last_full_sync_at
      - last_error
  - capability_profile_id: which jobs/agents may touch this source
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3 Storage and indexing
- CONTEXT_START_LINE: 54777
- CONTEXT_END_LINE: 54818
- CONTEXT_TOKEN: CREATE TABLE calendar_sources (
- EXCERPT_ASCII_ESCAPED:
  ```text
  - Relational table `calendar_events` with indices on `(workspace_id, start_ts, end_ts)` and full-text on `title`, `description`, `location`.

  CREATE TABLE calendar_sources (
      id TEXT PRIMARY KEY NOT NULL,
      workspace_id TEXT NOT NULL,
      display_name TEXT NOT NULL,
      provider_type TEXT NOT NULL,
      write_policy TEXT NOT NULL,
      default_tzid TEXT NOT NULL DEFAULT 'UTC',
      auto_export BOOLEAN NOT NULL DEFAULT 0,
      credentials_ref TEXT,
      last_sync_ts DATETIME,
      created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE calendar_events (
      id TEXT PRIMARY KEY NOT NULL,
      workspace_id TEXT NOT NULL,
      source_id TEXT NOT NULL,
      external_id TEXT,
      external_etag TEXT,
      title TEXT NOT NULL,
      description TEXT,
      location TEXT,
      start_ts_utc DATETIME NOT NULL,
      end_ts_utc DATETIME NOT NULL,
      tzid TEXT NOT NULL DEFAULT 'UTC',
      all_day BOOLEAN NOT NULL DEFAULT 0,
      status TEXT NOT NULL DEFAULT 'confirmed',
      visibility TEXT NOT NULL DEFAULT 'private',
      export_mode TEXT NOT NULL DEFAULT 'full_export',
      rrule TEXT,
      is_recurring BOOLEAN NOT NULL DEFAULT 0,
      instance_key TEXT,
      created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (source_id) REFERENCES calendar_sources(id)
  );
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.13.1 Four Portability Pillars - Pillar 2: Portable Schema & Migrations [CX-DBP-011]
- CONTEXT_START_LINE: 3243
- CONTEXT_END_LINE: 3257
- CONTEXT_TOKEN: Pillar 2: Portable Schema & Migrations [CX-DBP-011]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 2: Portable Schema & Migrations [CX-DBP-011]**

  All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.

  - FORBIDDEN: `strftime()`, SQLite datetime functions \u2192 REQUIRED: Parameterized timestamps
  - FORBIDDEN: SQLite placeholder syntax `?1`, `?2` \u2192 REQUIRED: Portable syntax `$1`, `$2`
  - FORBIDDEN: SQLite triggers with `OLD`/`NEW` semantics \u2192 REQUIRED: Application-layer mutation tracking
  - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - REQUIRED: Schema definitions are pure DDL (no data transforms)
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.13.1 Four Portability Pillars - Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3275
- CONTEXT_END_LINE: 3285
- CONTEXT_TOKEN: Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```
