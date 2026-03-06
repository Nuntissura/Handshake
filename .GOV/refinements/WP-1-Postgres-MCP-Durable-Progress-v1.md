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
- WP_ID: WP-1-Postgres-MCP-Durable-Progress-v1
- CREATED_AT: 2026-03-05T22:41:53.300Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- SPEC_TARGET_SHA1: f3b0715a544ebae689bee2196c0a4041cf4f2798
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja060320260004
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Postgres-MCP-Durable-Progress-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE in Master Spec Main Body: Durable Progress Mapping (Spec 11.3.4) and dual-backend CI requirements (Spec 2.3.13.1 Pillar 4) clearly require that MCP progress tokens be persisted durably and remain resolvable under PostgreSQL.
- Implementation gap (codebase): Postgres `Database` impl leaves `update_ai_job_mcp_fields`, `get_ai_job_mcp_fields`, and `find_ai_job_id_by_mcp_progress_token` as NotImplemented, so MCP durable progress mapping fails on Postgres.
- Persistence gap (codebase): no portable migration currently creates a durable MCP progress mapping store; SQLite uses runtime DDL to add columns, but Postgres does not.

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: 45m
- REFERENCES:
  - Master Spec 11.3.4 (Durable Progress Mapping) and 2.3.13 (Storage backend portability).
  - Temporal/Cadence: durable execution state + resumability as the source of truth.
  - MCP notifications/progress: token-scoped progress updates bound to a transport/session.
  - SQL patterns: side-table keyed by job_id with an indexed/unique progress token for reverse lookup.
- PATTERNS_EXTRACTED:
  - Persist token->job_id mapping before issuing `tools/call` so progress notifications can always be correlated.
  - Enforce 1:1 token mapping (UNIQUE index) to prevent ambiguous joins and token aliasing.
  - Prefer pure-DDL, replay-safe migrations; avoid runtime DDL for new fields.
- DECISIONS (ADOPT/ADAPT/REJECT):
  - ADOPT: portable side-table `ai_job_mcp_fields` (job_id PK) + indexed `mcp_progress_token`, implemented behind the `Database` trait for both SQLite and Postgres.
  - ADAPT: keep existing SQLite columns as a backward-compat shim if already present; read-through strategy: prefer side-table when row exists, fallback to legacy columns for older DBs (no data transform in this WP).
  - REJECT: Postgres-only runtime `ALTER TABLE` fixes for this feature (drift-prone and undermines portability posture).
- LICENSE/IP_NOTES: No code reuse; patterns only (durable job queues and token mapping).
- SPEC_IMPACT: NO
- SPEC_IMPACT_REASON: Spec already defines durable progress mapping and dual-backend testing expectations; this WP is implementation parity + portable persistence.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Durable progress mapping must remain correlatable: `notifications/progress` token -> `job_id` -> Flight Recorder `mcp.progress` rows.
- This WP does not change Flight Recorder schemas; it ensures PostgreSQL storage can resolve `mcp_progress_token` so the MCP Gate can record progress events with correct job correlation.
- Regression evidence target: MCP e2e test asserts `mcp.tool_call`, `mcp.tool_result`, `mcp.progress`, and `mcp.logging` are emitted, and that token lookup returns the originating job_id.

### RED_TEAM_ADVISORY (security failure modes)
- Token spoofing / cross-job confusion: enforce 1:1 mapping from `mcp_progress_token` to `job_id` (recommend UNIQUE index on token); define overwrite rules if a server attempts to reuse a token.
- Untrusted progress text: `message` in `notifications/progress` is server-controlled; ensure existing payload caps/redaction do not regress (out of scope here, but must be preserved).
- SQL injection: all updates/lookups must remain parameterized.

### PRIMITIVES (traits/structs/enums)
- Storage trait surface (already present):
  - `AiJobMcpFields`, `AiJobMcpUpdate`
  - `Database::{update_ai_job_mcp_fields, get_ai_job_mcp_fields, find_ai_job_id_by_mcp_progress_token}`
- Portable schema primitive (preferred):
  - `ai_job_mcp_fields(job_id PRIMARY KEY REFERENCES ai_jobs(id), mcp_server_id, mcp_call_id, mcp_progress_token)`
  - Index: `ai_job_mcp_fields(mcp_progress_token)` (UNIQUE recommended)

### PILLAR_ALIGNMENT (Handshake pillars cross-check)
- Rule: Refinement MUST explicitly consider pillar alignment and interconnections (force multipliers). If unknown, write UNKNOWN and create stubs instead of guessing.
- For each pillar, mark: [TOUCHED] | [NOT_TOUCHED] | [UNKNOWN], then add 1-3 lines when TOUCHED/UNKNOWN (impact + required Spec/Appendix updates + required tests/evidence).
- Pillars:
  - Flight Recorder
  - Calendar
  - Monaco
  - Word clone
  - Excel clone
  - Locus
  - Loom
  - Work packets (product, not repo)
  - Task board (product, not repo)
  - MicroTask
  - Command Center
  - Spec to prompt
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
  - Stage
  - Studio
  - Atelier/Lens
  - Skill distillation / LoRA
  - ACE
  - RAG
- Flight Recorder: [TOUCHED] - requires token->job correlation for `mcp.progress` evidence.
- Calendar: [NOT_TOUCHED]
- Monaco: [NOT_TOUCHED]
- Word clone: [NOT_TOUCHED]
- Excel clone: [NOT_TOUCHED]
- Locus: [NOT_TOUCHED]
- Loom: [NOT_TOUCHED]
- Work packets (product, not repo): [NOT_TOUCHED]
- Task board (product, not repo): [NOT_TOUCHED]
- MicroTask: [NOT_TOUCHED]
- Command Center: [NOT_TOUCHED]
- Spec to prompt: [NOT_TOUCHED]
- SQL to PostgreSQL shift readiness: [TOUCHED] - implements missing Postgres storage methods and enforces dual-backend parity.
- LLM-friendly data: [NOT_TOUCHED]
- Stage: [NOT_TOUCHED]
- Studio: [NOT_TOUCHED]
- Atelier/Lens: [NOT_TOUCHED]
- Skill distillation / LoRA: [NOT_TOUCHED]
- ACE: [NOT_TOUCHED]
- RAG: [NOT_TOUCHED]
- PILLAR_ALIGNMENT_VERDICT: OK

### FORCE_MULTIPLIER_INTERACTIONS (cross-primitive / cross-feature)
- NONE (no new interaction-matrix edges required for this remediation).

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
- CLEARLY_COVERS_REASON: Master Spec 11.3.4 defines durable progress token persistence and lookup semantics, and Master Spec 2.3.13.1 Pillar 4 requires dual-backend CI parity; together they require PostgreSQL to persist and resolve MCP progress tokens.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The requirement is explicitly covered by durable progress mapping (Spec 11.3.4) plus dual-backend testing constraints (Spec 2.3.13.1 Pillar 4). This WP is implementation + migration hygiene, not spec gap closure.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 11.3.4 Implementation Target 3: Durable Progress Mapping (SQLite Integration)
- CONTEXT_START_LINE: 63960
- CONTEXT_END_LINE: 63979
- CONTEXT_TOKEN: mcp_progress_token
- EXCERPT_ASCII_ESCAPED:
  ```text
  Minimal tables (names indicative):

  -- Existing AI jobs
  CREATE TABLE ai_jobs (
      job_id              TEXT PRIMARY KEY,
      job_kind            TEXT NOT NULL,
      status              TEXT NOT NULL, -- queued|running|completed|failed|interrupted
      created_at          INTEGER NOT NULL,
      updated_at          INTEGER NOT NULL,
      error_code          TEXT,
      error_message       TEXT,
      -- ...
      mcp_server_id       TEXT,      -- "docling-mcp"
      mcp_call_id         TEXT,      -- JSON-RPC id for tools/call
      mcp_progress_token  TEXT       -- "123"
  );

  CREATE INDEX idx_ai_jobs_progress_token ON ai_jobs(mcp_progress_token);
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.13.1 Four Portability Pillars - Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3275
- CONTEXT_END_LINE: 3285
- CONTEXT_TOKEN: Dual-Backend Testing Early [CX-DBP-013]
- EXCERPT_ASCII_ESCAPED:
  ```text
  Pillar 4: Dual-Backend Testing Early [CX-DBP-013]

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.141.md 2.3.13.1 Four Portability Pillars - Pillar 2: Portable Schema & Migrations [CX-DBP-011]
- CONTEXT_START_LINE: 3243
- CONTEXT_END_LINE: 3251
- CONTEXT_TOKEN: Portable Schema & Migrations [CX-DBP-011]
- EXCERPT_ASCII_ESCAPED:
  ```text
  Pillar 2: Portable Schema & Migrations [CX-DBP-011]

  All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.

  - FORBIDDEN: SQLite placeholder syntax `?1`, `?2` \\u2192 REQUIRED: Portable syntax `$1`, `$2`
  - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - REQUIRED: Schema definitions are pure DDL (no data transforms)
  ```
