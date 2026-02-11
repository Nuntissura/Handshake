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
- WP_ID: WP-1-Flight-Recorder-v4
- CREATED_AT: 2026-02-11T21:34:17.296Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.125.md
- SPEC_TARGET_SHA1: d16eb1eb5045e858112b2ce477f27aa0200621b0
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja110220262332
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Flight-Recorder-v4

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. Existing Master Spec requirements already imply Flight Recorder persistence must be reliable across runs (always-on observability + trace correlation + backend storage/migrations). This WP is an implementation hardening to satisfy those requirements when an older DuckDB file exists.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- No new FR-EVT IDs introduced.
- This WP changes DuckDB schema initialization/migration ordering for the Flight Recorder sink so that older DB files (missing newer columns like `events.trace_id`) do not hard-fail during startup.
- Primary interaction: the `events` table schema and indexes (trace correlation field used for filtering/navigation).

### RED_TEAM_ADVISORY (security failure modes)
- Startup DoS / momentum-killer: older on-disk DuckDB schema missing columns causes init to fail before the app can run. Mitigation: run additive `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` migrations BEFORE creating indexes that reference new columns; keep all operations idempotent.
- Data integrity: adding a new column to legacy rows must not force NOT NULL on existing data (older rows may not have trace_id). Prefer nullable column + gradual backfill, or keep NOT NULL only when table was created fresh.
- Partial migration: ensure schema init is transactional (or structured) such that failure does not leave the DB in a state that prevents retry.

### PRIMITIVES (traits/structs/enums)
- Rust: `DuckDbFlightRecorder`, `DuckDbFlightRecorder::new_on_path`, `DuckDbFlightRecorder::init_schema`
- DuckDB: `DuckDbConnection::execute_batch` schema migration + index creation

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec requires always-on Flight Recorder observability with trace correlation recorded to the DuckDB sink, and codifies that backend persistence includes migrations. A startup crash due to schema drift violates these requirements; fixing schema init ordering and adding a regression test is a direct, measurable compliance implementation.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already requires Flight Recorder always-on observability with trace correlation and a DuckDB sink, and it already establishes migrations as part of backend persistence. The needed behavior (migration ordering / retrying index creation) is an implementation detail to satisfy existing requirements; no new normative spec text is required.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 11.5 Flight Recorder Event Shapes & Retention (Trace Invariant; DuckDB sink)
- CONTEXT_START_LINE: 57578
- CONTEXT_END_LINE: 57588
- CONTEXT_TOKEN: Trace Invariant
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 11.5 Flight Recorder Event Shapes & Retention

  - Observability Instrumentation (Metrics & Traces):
    - Trace Invariant: Every AI action MUST emit a unique trace_id which links the Job, the RAG QueryPlan, and the final result.
    - Span Requirements: Workflow::run and each JobKind execution MUST be wrapped in a "Span" (Start/End events recorded in DuckDB).
    - Retention Policy: Implement an automatic retention policy; traces older than 7 days SHOULD be purged.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md [CX-224] BACKEND_STORAGE (persistence logic + migrations)
- CONTEXT_START_LINE: 28731
- CONTEXT_END_LINE: 28736
- CONTEXT_TOKEN: BACKEND_STORAGE
- EXCERPT_ASCII_ESCAPED:
  ```text
  [CX-224] BACKEND_STORAGE: {{BACKEND_STORAGE_DIR}}/ SHOULD contain persistence logic (DB, filesystem, blobs) and migrations.
  ```
