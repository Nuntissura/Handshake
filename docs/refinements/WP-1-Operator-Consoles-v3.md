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
- WP_ID: WP-1-Operator-Consoles-v3
- CREATED_AT: 2026-01-02T22:28:32.9862695+01:00
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- SPEC_TARGET_SHA1: 76e8e6e8259b64a6dc4aed5cf2afb754ff1f3aed
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja020120262232

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (Operator Consoles + Diagnostics schema requirements are explicitly and normatively defined in Master Spec v02.100).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-003 (DiagnosticEvent): Diagnostics emitted/recorded by the diagnostics pipeline MUST be linkable from Flight Recorder via Diagnostic.id without duplicating full payload.
- Operator Consoles surfaces MUST deep-link via job_id/diagnostic_id/wsid and Flight Recorder event ids (see anchors below).

### RED_TEAM_ADVISORY (security failure modes)
- Evidence Drawer leakage: raw JSON must be redacted-by-default to prevent secrets/PII exposure.
- Fingerprint instability: non-deterministic canonicalization breaks grouping and can hide repeated failures (operator mis-triage).
- False direct links: presenting ambiguous links as direct can misattribute causality and lead to unsafe operator actions.

### PRIMITIVES (traits/structs/enums)
- DiagnosticSeverity, DiagnosticSource, DiagnosticSurface, LinkConfidence
- DiagnosticRange, DiagnosticLocation, EvidenceRefs, Diagnostic
- DiagnosticsStore trait (Rust)
- FR-EVT-003 DiagnosticEvent (Flight Recorder event shape)

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec contains explicit, testable MUST/SHOULD/MUST NOT lists for the Operator Consoles surfaces (10.5.5.1-10.5.5.4) plus the canonical Diagnostics schema and storage/trait definitions (11.4) and the FR event linkage shape (11.5 FR-EVT-003).

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The governing requirements for this WP are already fully specified in the current Master Spec Main Body (v02.100) with concrete schemas, field names, and normative rules; no additional normative text is required to proceed.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 10.5.5.1-10.5.5.4 (Operator Consoles surfaces)
- CONTEXT_START_LINE: 26440
- CONTEXT_END_LINE: 26505
- CONTEXT_TOKEN: #### 10.5.5.1 Problems
- EXCERPT_ASCII_ESCAPED:
  ```text

  1. Open **Problems** and filter `severity \\u00E2\\u02C6\\u02C6 {error,fatal}`.
  2. Select the top issue; open **Evidence Drawer**.
  3. From Evidence, open the **Related Job** (if any) and its **Timeline slice**.
  4. Review **Policy** (allowed/blocked capabilities) relevant to the failure.
  5. Export **Debug Bundle** for the selected issue/time window/job.
  6. Provide the generated **LLM coder prompt** + bundle to the coding agent.

  ### 10.5.5 Console surfaces

  All surfaces below MUST deep-link to each other via `job_id`, `diagnostic_id`, `wsid`, and Flight Recorder event ids (see \\u00C2\\u00A711.4 and \\u00C2\\u00A711.5).

  #### 10.5.5.1 Problems

  MUST:
  - Render a table of normalized diagnostics (canonical schema: \\u00C2\\u00A711.4).
  - Provide filters for: `severity`, `source`, `surface`, `wsid`, `job_id`, `time_range`.
  - Group by deterministic `fingerprint` (see \\u00C2\\u00A711.4), while retaining access to raw instances.
  - Support: open/ack/mute/resolved statuses (local-only metadata is permitted).
  - Open Evidence Drawer on selection.

  SHOULD:
  - Show `count`, `first_seen`, `last_seen`.
  - Show correlation quality (`link_confidence`) and provide \\u00E2\\u20AC\\u0153why linked?\\u00E2\\u20AC\\u009D explanation.

  MUST NOT:
  - Hide or drop raw diagnostic instances when recomputing the Problems index.

  #### 10.5.5.2 Jobs

  MUST:
  - List jobs with filters: status, kind, workspace (`wsid`), time range.
  - Provide a Job Inspector with tabs: Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, Policy.
  - Allow exporting a Debug Bundle scoped to a job.

  SHOULD:
  - Provide \\u00E2\\u20AC\\u0153clone + rerun\\u00E2\\u20AC\\u009D in sandbox mode (subject to capability policy).

  MUST NOT:
  - Allow running privileged actions without an explicit policy decision being visible in the Policy tab.

  #### 10.5.5.3 Timeline (Flight Recorder)

  MUST:
  - Render a time-window view over Flight Recorder events (canonical: \\u00C2\\u00A711.5).
  - Provide filters: job_id, wsid, actor, surface, event types.
  - Allow opening Evidence Drawer for any event.
  - Support \\u00E2\\u20AC\\u0153pin this slice\\u00E2\\u20AC\\u009D (stable query) for bundle export.

  SHOULD:
  - Provide \\u00E2\\u20AC\\u0153expand context\\u00E2\\u20AC\\u009D affordances (e.g., show preceding N seconds/events).

  #### 10.5.5.4 Evidence Drawer (shared detail view)

  MUST:
  - Show a single \\u00E2\\u20AC\\u0153evidence card\\u00E2\\u20AC\\u009D for a selected diagnostic or event:
    - raw JSON (redacted view default),
    - linked entities (job, wsid, spans),
    - relevant policy/capability decisions,
    - related artifacts by hash,
    - link_confidence and correlation explanation.
  - Provide \\u00E2\\u20AC\\u0153Export Debug Bundle\\u00E2\\u20AC\\u009D entrypoint.

  SHOULD:
  - Provide a \\u00E2\\u20AC\\u0153copy as coder prompt\\u00E2\\u20AC\\u009D action (see 10.5.6.4).

  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.4 (Diagnostics schema + DuckDB storage schema + DiagnosticsStore trait)
- CONTEXT_START_LINE: 30524
- CONTEXT_END_LINE: 30721
- CONTEXT_TOKEN: ## 11.4 Diagnostics Schema (Problems/Events)
- EXCERPT_ASCII_ESCAPED:
  ```text

  Together, these checks:

  * Prevent the host from following symlinks out of the allowed sandbox when reading artifacts from MCP servers.
  * Ensure distillation sampling is read-only, with no tool-calls or workflow side-effects, even if the Teacher server is malicious or compromised.

  ## 11.4 Diagnostics Schema (Problems/Events)

  - **DIAG-SCHEMA-001 (Diagnostic shape; canonical)**

  A **Diagnostic** is the canonical, normalized representation of any problem emitted by LSPs, validators, engines, connectors, terminal matchers, or plugins.

  (TypeScript excerpt; fences omitted)
  type DiagnosticSeverity = 'fatal' | 'error' | 'warning' | 'info' | 'hint';

  type DiagnosticSource =
    | 'lsp'
    | 'terminal'
    | 'validator'
    | 'engine'
    | 'connector'
    | 'system'
    | `plugin:${string}`
    | `matcher:${string}`;

  type DiagnosticSurface =
    | 'monaco'
    | 'canvas'
    | 'sheet'
    | 'terminal'
    | 'connector'
    | 'system';

  type LinkConfidence = 'direct' | 'inferred' | 'ambiguous' | 'unlinked';

  interface DiagnosticRange {
    startLine: number;
    startColumn: number;
    endLine: number;
    endColumn: number;
  }

  interface DiagnosticLocation {
    // One of these SHOULD be set; multiple MAY be set to aid linking.
    path?: string;          // local path where applicable
    uri?: string;           // file:// or internal uri
    wsid?: string;          // workspace surface id
    entity_id?: string;     // KG / RawContent entity id (if applicable)
    range?: DiagnosticRange;
  }

  interface EvidenceRefs {
    fr_event_ids?: string[];             // Flight Recorder event ids
    related_job_ids?: string[];          // candidates when ambiguous/inferred
    related_activity_span_ids?: string[];
    related_session_span_ids?: string[];
    artifact_hashes?: {
      input_hash?: string;
      output_hash?: string;
      diff_hash?: string;
    };
  }

interface Diagnostic {
    // Identity
    id: string;                         // uuid (this is the canonical diagnostic_id)
    fingerprint: string;                // deterministic grouping key (see DIAG-SCHEMA-003)

    // Content
    title: string;                      // short, operator-readable
    message: string;                    // details; may be redacted at render/export time
    severity: DiagnosticSeverity;
    source: DiagnosticSource;
    surface: DiagnosticSurface;

    // Optional classification / routing
    tool?: string | null;               // tool/engine/plugin name if known
    code?: string | null;               // stable error code if available
    tags?: string[];

    // Correlation (may be empty until linked)
    wsid?: string | null;
    job_id?: string | null;
    model_id?: string | null;
    actor?: 'human' | 'agent' | 'system';
    capability_id?: string | null;
    policy_decision_id?: string | null;

    // Location + evidence
    locations?: DiagnosticLocation[];
    evidence_refs?: EvidenceRefs;

    // Link quality (canonical semantics in DIAG-SCHEMA-004)
    link_confidence: LinkConfidence;

    // Lifecycle / aggregation (Problems view metadata)
    status?: 'open' | 'acknowledged' | 'muted' | 'resolved';
    count?: number;                     // count of merged raw instances (Problems grouping)
    first_seen?: string;                // RFC3339
    last_seen?: string;                 // RFC3339

    // Timestamps
    timestamp: string;                  // RFC3339 (creation time)
  updated_at?: string;                // RFC3339
}

  Notes:
  - `Diagnostic.id` is the canonical `diagnostic_id` used by Flight Recorder events (see FR-EVT-003).
  - `fingerprint` is the Problems grouping key; it MUST be deterministic.
  - `count/first_seen/last_seen/status` are allowed to be local-only UI metadata; raw instances MUST remain queryable.

  - **DIAG-SCHEMA-002 (Routing)**
  Diagnostics in this shape MUST be:
  - Pushed into Monaco (when a Monaco-backed surface is present),
  - Listed in the global Problems view (see \\u00C2\\u00A710.5),
  - Optionally stored as `DiagnosticEvent` in Flight Recorder (see \\u00C2\\u00A711.5 FR-EVT-003).

  - **DIAG-SCHEMA-003 (Fingerprinting / Problems index key)**

  The Problems view MUST group diagnostics by `Diagnostic.fingerprint`, computed as a deterministic hash over a canonicalized tuple of fields.

  Normative rules:
  1. Canonicalization MUST be stable across platforms (Windows/macOS/Linux) and UI layers.
  2. Canonicalization MUST normalize:
     - path separators to `/` in `locations.path`,
     - whitespace in `title/message` (trim; collapse `\\r\\n` \\u00E2\\u2020\\u2019 `\\n`),
     - absent fields to explicit `null` in the canonical tuple.
  3. The fingerprint MUST be computed as: `sha256(utf8(json_canonical_tuple))`, encoded as lowercase hex.

  The canonical tuple MUST include at least:
  - `source`, `surface`, `tool`, `code`, `severity`,
  - `title` (verbatim),
  - `locations` reduced to stable identifiers: `(path|uri|entity_id|wsid, range)` for each location, sorted,
  - `capability_id` and `policy_decision_id` (if present).

  It MUST NOT include:
  - `timestamp`, `updated_at`,
  - volatile ids like `event_id` or `job_id` (those are correlation, not identity),
  - `count/first_seen/last_seen/status`.

  #### 11.4.2 Storage Schema (DuckDB)

  The system MUST maintain a `diagnostics` table in the DuckDB sink for analytical queries.

  (SQL excerpt; fences omitted)
  CREATE TABLE diagnostics (
      id              UUID PRIMARY KEY,
      fingerprint     VARCHAR NOT NULL,
      title           VARCHAR NOT NULL,
      message         TEXT,
      severity        ENUM('fatal', 'error', 'warning', 'info', 'hint'),
      source          VARCHAR NOT NULL,
      surface         VARCHAR NOT NULL,
      tool            VARCHAR,
      code            VARCHAR,
      wsid            VARCHAR,
      job_id          UUID,
      link_confidence ENUM('direct', 'inferred', 'ambiguous', 'unlinked'),
      timestamp       TIMESTAMP NOT NULL,
      metadata        JSON
  );

  CREATE INDEX idx_diag_fingerprint ON diagnostics(fingerprint);
  CREATE INDEX idx_diag_job_id ON diagnostics(job_id);

  #### 11.4.3 Diagnostics Store Trait (Rust)

  (Rust excerpt; fences omitted)
  #[async_trait]
  pub trait DiagnosticsStore: Send + Sync {
      /// Inserts a diagnostic and emits FR-EVT-003.
      async fn record_diagnostic(&self, diag: Diagnostic) -> Result<(), StorageError>;

      /// Returns grouped diagnostics with counts.
      async fn list_problems(&self, filter: DiagFilter) -> Result<Vec<ProblemGroup>, StorageError>;
  }

  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-003 (DiagnosticEvent)
- CONTEXT_START_LINE: 30880
- CONTEXT_END_LINE: 30896
- CONTEXT_TOKEN: - **FR-EVT-003 (DiagnosticEvent)**
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-003 (DiagnosticEvent)**

  A DiagnosticEvent links a Flight Recorder trace to a Diagnostic (`Diagnostic.id`) without duplicating the full Diagnostic payload.

  (TypeScript excerpt; fences omitted)
  interface DiagnosticEvent extends FlightRecorderEventBase {
    type: 'diagnostic';

    diagnostic_id: string;             // equals Diagnostic.id
    wsid?: string | null;
    severity?: 'fatal' | 'error' | 'warning' | 'info' | 'hint';
    source?: string;                   // optional echo for quick filtering
  }
  ```
