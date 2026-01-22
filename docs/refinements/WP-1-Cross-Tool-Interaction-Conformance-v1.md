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
- WP_ID: WP-1-Cross-Tool-Interaction-Conformance-v1
- CREATED_AT: 2026-01-21T19:35:25Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: CF2F5305FC8EEC517D577D87365BD9C072A99B0F
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja210120262044
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Cross-Tool-Interaction-Conformance-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE.
- Deterministic implementation approach (no spec enrichment): use DuckDB `fr_events` rows (11.3.6.4) for generic tool invocation logging (`tool.*` kinds) and use FR-EVT-001/002/006 where applicable.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Existing required Flight Recorder event schemas (11.5):
  - FR-EVT-001 `terminal_command` for terminal surface commands (stdout/stderr refs, exit code, duration).
  - FR-EVT-002 `editor_edit` for Monaco/canvas/sheet edits (before/after hashes, ops).
  - FR-EVT-006 `llm_inference` for model calls (trace_id + model_id required).
- Generic tool invocation logging contract (6.0.1 + 11.3.6.4):
  - For any non-terminal tool invocation (mechanical engines, MCP tools, ingestion sidecars, exporters), write Flight Recorder rows in DuckDB `fr_events` with:
    - `event_kind` values (minimum): `tool.call`, `tool.result`, plus existing MCP kinds when applicable (`mcp.tool_call`, `mcp.progress`, `mcp.logging`).
    - `source` set to an implementation-specific tool identity string (examples: `host`, `docling-mcp`, `asr-mcp`).
    - `payload` JSON includes (minimum required keys for `tool.*` kinds):
      - `tool_name` (string)
      - `tool_version` (string|null)
      - `inputs` (array of artifact refs / workspace refs)
      - `outputs` (array of artifact refs / workspace refs; may be empty on error)
      - `status` (`success|error|timeout|skipped`)
      - `duration_ms` (number|null)
      - `error_code` (string|null)
      - Correlation: `job_id`, `workflow_run_id`, and `trace_id` where available
      - Governance: `capability_id` when the invocation causes filesystem/process/network side effects
  - Triggers (minimum):
    - Emit `tool.call` immediately before invocation (inputs known).
    - Emit `tool.result` immediately after completion (outputs + status known).

### RED_TEAM_ADVISORY (security failure modes)
- Shadow pipelines: tools run outside Workflow Engine/AiJobModel context, bypassing capability checks and Flight Recorder logging.
- Hidden side effects: tools writing directly to filesystem/network/process without capability gates creates un-auditable and unsafe behavior.
- Log leakage: tool inputs/outputs may contain secrets/PII; logging must avoid inline unbounded text (store refs/hashes, redact under policy).
- Remote fallback drift: remote results not cached as artifacts makes runs non-reproducible and undermines local-first posture.
- Correlation breakage: missing job_id/workflow_run_id/trace_id breaks Operator Console "what happened" narratives and incident response.

### PRIMITIVES (traits/structs/enums)
- `ToolInvocation` (struct): tool identity + correlation + artifacts + outcome; canonical source for both Flight Recorder events and Operator Console display.
- `ToolKind` (enum): terminal | mcp | mechanical_engine | ingestion | exporter | ui_surface.
- `ToolInvocationStatus` (enum): success | error | timeout | skipped.
- `ConformanceInvariant` (enum): NoShadowPipelines | ArtifactFirstIO | CapabilityGatedSideEffects | FlightRecorderAlwaysOn | LocalFirstDefault | EvidenceSurfaces.
- `ConformanceViolation` (struct/enum): invariant + location + evidence refs (artifact refs / fr_event ids).
- `ToolBus` / `ToolInvoker` (trait): the only entrypoint for invoking tools from workflow steps; enforces artifact-first IO + capability checks + logging.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: 6.0.1 defines the invariants, 11.3.6.4 defines the canonical Flight Recorder storage surface (`fr_events`), and 11.5 defines required event schemas for terminal/editor/llm. This refinement binds generic tool invocation logging to `fr_events` (`tool.*` kinds).
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec already defines the invariants (6.0.1) and a canonical Flight Recorder table suitable for tool-call style events (11.3.6.4). This WP is implementation alignment, not new normative requirements.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 6.0.1 Cross-Tool Interaction Map (Normative)
- CONTEXT_START_LINE: 13815
- CONTEXT_END_LINE: 13853
- CONTEXT_TOKEN: No shadow pipelines
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 6.0.1 Cross-Tool Interaction Map (Normative)
  
  Handshake includes many \u00E2\u20AC\u0153mechanical tools\u00E2\u20AC\u009D and \u00E2\u20AC\u0153AI tools\u00E2\u20AC\u009D (Docling, ASR, ACE runtime, RAG, calendar_sync, terminal tools, renderers, exporters, etc.). To avoid a pile of special-case pipelines, Handshake MUST treat cross-tool interaction as a first-class contract.
  
  **Hard rules (integration invariants)**
  1. **No shadow pipelines:** tool execution MUST occur via the Workflow Engine + AI Job Model (A\u00C2\u00A72.6), not ad-hoc background threads.
  2. **Artifact-first I/O:** tools MUST consume inputs by reference (workspace IDs / artifact refs) and produce outputs as Raw/Derived/Display content (A\u00C2\u00A72.2), not hidden local files.
  3. **Capability-gated side effects:** any filesystem/process/network side effect MUST be capability-checked (A\u00C2\u00A711.1) and recorded.
  4. **Flight Recorder is always-on:** every tool invocation MUST emit Flight Recorder events with tool identity/version, input refs, output refs, timing/budgets, and error codes (A\u00C2\u00A72.1.5, A\u00C2\u00A711.5).
  5. **Local-first default:** the default posture is offline/local execution. Remote execution (cloud models, remote services) MUST be opt-in, capability-gated, and have a deterministic local fallback path.
  6. **Evidence surfaces:** Operator Consoles MUST be able to show \u00E2\u20AC\u0153what happened\u00E2\u20AC\u009D end-to-end (Job History + Problems/Evidence) for every tool interaction (A\u00C2\u00A710.5, A\u00C2\u00A711.4).
  
  **Interaction table (minimum set; expand as tools are added)**
  
  | Tool / Surface | Primary trigger | Consumes | Produces | Required shared primitives |
  |---|---|---|---|---|
  | Docs editor | user edit / AI edit | Document blocks + selection | Display changes + diff artifacts | AI Job Model, consent, FR events, deterministic edit UX |
  | Canvas editor | user edit / AI layout | Canvas nodes/edges | Display changes + render/export artifacts | AI Job Model, FR events, artifact refs |
  | ACE runtime (Agentic Context Engineering) | any AI job requiring context | Workspace entities + scope hints | ContextPlan/ContextSnapshot (+ hashes) | budgets, determinism, validator pack, FR traces |
  | Shadow Workspace + RAG | query / \u00E2\u20AC\u0153Project Brain\u00E2\u20AC\u009D | indexed workspace content | QueryPlan + RetrievalTrace + citations | cache keys, drift flags, Evidence view |
  | Docling ingestion | import file | external file artifact ref | structured blocks/tables/assets | workflow job, provenance, FR logs |
  | ASR ingestion | import audio/video | audio asset ref | transcript docs + timing sidecars | workflow job, provenance, FR logs |
  | Calendar subsystem | view or patch-set apply | CalendarEvents + ActivitySpans | patch-set artifacts + synced state | patch-set discipline, capability gating, FR logs |
  | Terminal surface | user command / workflow step | command + working dir + refs | stdout/stderr artifacts + exit code | capability gates, reproducible command records |
  | Monaco surface | code edit / AI refactor | file refs + diffs | patch-set artifacts + review UX | no-silent-edits, diff/accept, FR logs |
  | Operator Consoles | diagnostics / evidence drilldown | Problems + Events + bundles | human-readable evidence views | DuckDB store, trace linking, bundle export |
  | Debug Bundle exporter | user request / CI artifact | selected logs + refs | deterministic bundle hash + archive | artifact hashing, redaction, provenance |
  | Workspace Bundle exporter | user request | workspace entities | portable export bundle | deterministic hashing, manifest, retention policy |
  | Mechanical Extension (Tool Bus) engines | workflow nodes | PlannedOperation | EngineResult + artifacts | MEX envelopes, conformance gates, FR logs |
  | Photo Studio | image pipeline job | RAW/asset refs | renders + exports + sidecars | render determinism, provenance, FR logs |
  | Charts/Dashboards/Decks | user request / AI build | tables + data refs | chart/deck artifacts + validations | schema validators, export policy, provenance |
  
  **MCP and remote services (local-first stance)**
  - MCP MAY be used as an adapter layer for tools/services, but it MUST NOT become a required dependency for core local workflows.
  - Any MCP-backed tool MUST support:
    - local-first execution when available
    - deterministic caching of remote results (artifact refs + hashes)
    - explicit consent + capability gating for network access
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.3.6.4 Canonical Flight Recorder tables (DuckDB)
- CONTEXT_START_LINE: 45676
- CONTEXT_END_LINE: 45697
- CONTEXT_TOKEN: CREATE TABLE fr_events (
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### Canonical Flight Recorder tables (DuckDB)
  
  Minimal core:
  
  ```sql
  CREATE TABLE fr_events (
      event_id        BIGINT PRIMARY KEY,
      ts_utc          TIMESTAMP NOT NULL,
      session_id      TEXT,
      task_id         TEXT,
      job_id          TEXT,
      workflow_run_id TEXT,
      event_kind      TEXT NOT NULL, -- "mcp.logging", "mcp.tool_call", "mcp.progress", ...
      source          TEXT NOT NULL, -- "docling-mcp", "asr-mcp", "teacher-mcp", "host"
      level           TEXT,          -- "DEBUG", "INFO", "WARN", "ERROR"
      message         TEXT,
      payload         JSON
  );
  
  CREATE INDEX idx_fr_events_job_id ON fr_events(job_id);
  CREATE INDEX idx_fr_events_kind ON fr_events(event_kind);
  ```
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.5 Flight Recorder Event Shapes & Retention (FR-EVT-001/002)
- CONTEXT_START_LINE: 46341
- CONTEXT_END_LINE: 46411
- CONTEXT_TOKEN: FR-EVT-001 (TerminalCommandEvent)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ```ts
  type FlightRecorderActor = 'human' | 'agent' | 'system';
  
  interface FlightRecorderEventBase {
    type: string;
    event_id: string;                 // uuid
    timestamp: string;                // RFC3339
    actor: FlightRecorderActor;
  
    // Correlation / navigation
    job_id?: string;
    model_id?: string;
    wsids?: string[];
    activity_span_id?: string;
    session_span_id?: string;
  
    // Governance
    capability_id?: string;
    policy_decision_id?: string;
  }
  ```
  
  - **FR-EVT-001 (TerminalCommandEvent)**
  
  ```ts
  interface TerminalCommandEvent extends FlightRecorderEventBase {
    type: 'terminal_command';
  
    session_id: string;               // terminal session identifier
    cwd?: string | null;
    command: string;                  // command line as executed
    args?: string[];                  // optional parsed argv
  
    // Result
    exit_code?: number | null;
    duration_ms?: number | null;
  
    // Output references (never inline unbounded output)
    stdout_ref?: string | null;        // pointer/ref to stored output chunk(s)
    stderr_ref?: string | null;
  
    // Optional: environment and process metadata (redacted per policy)
    env_ref?: string | null;           // ref to redacted env snapshot
  }
  ```
  
  - **FR-EVT-002 (EditorEditEvent)**
  
  ```ts
  interface EditorEditOp {
    range: { startLine: number; startColumn: number; endLine: number; endColumn: number };
    insert_text?: string;              // MAY be omitted or hashed/redacted by policy
    delete_text?: string;              // MAY be omitted or hashed/redacted by policy
  }
  
  interface EditorEditEvent extends FlightRecorderEventBase {
    type: 'editor_edit';
  
    editor_surface: 'monaco' | 'canvas' | 'sheet';
    document_uri?: string | null;      // file:// or internal uri
    path?: string | null;
  
    // Content addressing (preferred over raw text)
    before_hash?: string | null;
    after_hash?: string | null;
    diff_hash?: string | null;
  
    // Edit details
    ops: EditorEditOp[];
  }
  ```
  ```
