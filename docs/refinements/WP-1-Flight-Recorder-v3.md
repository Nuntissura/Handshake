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
- WP_ID: WP-1-Flight-Recorder-v3
- CREATED_AT: 2026-01-01T00:00:00Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- SPEC_TARGET_SHA1: 76e8e6e8259b64a6dc4aed5cf2afb754ff1f3aed
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010120261446

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Spec-to-code mismatch: Current repo defines and emits a non-canonical `llm_inference` Flight Recorder event and maps it to "FR-EVT-002", but SPEC_CURRENT v02.100 defines FR-EVT-002 as EditorEditEvent and does not define an LLM inference FR-EVT schema in 11.5.
- Spec-to-code mismatch: Current repo uses different FR-EVT numbering for terminal/workflow recovery (e.g., terminal payload labeled FR-EVT-007; workflow recovery payload labeled FR-EVT-006) instead of the SPEC_CURRENT 11.5 canonical list (FR-EVT-001..005 plus FR-EVT-WF-RECOVERY).
- Compliance gap: Flight Recorder ingestion contract requires schema validation against FR-EVT-* shapes and a retention primitive; current implementation must be checked and aligned to 11.5.1 and 11.5 retention/linkability requirements.
- Downstream risk: Operator Consoles and multiple subsystems rely on canonical Flight Recorder typing and queryability; taxonomy drift blocks reliable revalidation.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Triggers that MUST be representable as canonical Flight Recorder events per SPEC_CURRENT 11.5:
  - Terminal command execution -> FR-EVT-001 (TerminalCommandEvent) with redacted output refs.
  - Editor document edits -> FR-EVT-002 (EditorEditEvent) with content addressing hashes.
  - Diagnostics emission -> FR-EVT-003 (DiagnosticEvent) linking to Diagnostic.id.
  - Debug bundle export -> FR-EVT-005 (DebugBundleExportEvent).
  - Startup recovery Running->Stalled transition -> FR-EVT-WF-RECOVERY (WorkflowRecoveryEvent) with actor='system'.
- Telemetry rule: synchronous high-frequency telemetry MUST be emitted asynchronously so Flight Recorder does not block hot paths (see 11.5 Observability Instrumentation note).

### RED_TEAM_ADVISORY (security failure modes)
- Taxonomy spoofing risk: If event type IDs are not canonical, validators and operator consoles can misclassify events, hiding security-relevant actions behind unexpected type strings.
- Auditability failure: If FR-EVT-003 does not link deterministically to Diagnostic.id, Problems/diagnostics views cannot prove trace linkage (broken evidence chain).
- Retention deception: If retention/linkability requirements are not implemented, the UI can falsely imply evidence exists (or omit "missing evidence" reasons), undermining incident response and debug bundles.

### PRIMITIVES (traits/structs/enums)
- `FlightRecorder` trait per SPEC_CURRENT 11.5.1:
  - `async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError>`
  - `async fn enforce_retention(&self) -> Result<u64, RecorderError>`
- `RecorderError` with stable HSK-400 / HSK-500 codes (InvalidEvent / SinkError).
- Canonical FR-EVT event payloads per SPEC_CURRENT 11.5:
  - FR-EVT-001 TerminalCommandEvent (type: 'terminal_command')
  - FR-EVT-002 EditorEditEvent (type: 'editor_edit')
  - FR-EVT-003 DiagnosticEvent (type: 'diagnostic')
  - FR-EVT-005 DebugBundleExportEvent (type: 'debug_bundle_export')
  - FR-EVT-WF-RECOVERY WorkflowRecoveryEvent (type: 'workflow_recovery')

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: SPEC_CURRENT v02.100 explicitly defines the Flight Recorder ingestion trait, canonical FR-EVT schemas, and retention/linkability rules in 11.5 (including explicit event type strings and required fields).

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: SPEC_CURRENT v02.100 11.5 already provides canonical event shapes, type strings, and the ingestion contract; the work is to align code to the existing normative text.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5.1 Flight Recorder Ingestion Contract (Normative Trait)
- CONTEXT_START_LINE: 30766
- CONTEXT_END_LINE: 30804
- CONTEXT_TOKEN: #### 11.5.1 Flight Recorder Ingestion Contract (Normative Trait)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.1 Flight Recorder Ingestion Contract (Normative Trait)

  The runtime MUST implement the `FlightRecorder` trait for all observability ingestion.

  ```rust
  /// HSK-TRAIT-003: Flight Recorder Ingestion
  #[async_trait]
  pub trait FlightRecorder: Send + Sync {
      /// Records a canonical event. MUST validate shape against FR-EVT-* schemas.
      /// Returns:
      /// - Ok(()) if persisted.
      /// - Err(RecorderError::InvalidEvent): If shape validation fails.
      async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError>;

      /// Enforces the 7-day retention policy (purge old events).
      /// Returns the number of events purged.
      async fn enforce_retention(&self) -> Result<u64, RecorderError>;
  }

  #[derive(Debug, ThisError)]
  pub enum RecorderError {
      #[error("HSK-400-INVALID-EVENT: Event shape violation: {0}")]
      InvalidEvent(String),
      #[error("HSK-500-DB: Sink error: {0}")]
      SinkError(String),
  }
  ```
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FlightRecorderEventBase and governance/correlation fields
- CONTEXT_START_LINE: 30808
- CONTEXT_END_LINE: 30830
- CONTEXT_TOKEN: interface FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
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

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-001 (TerminalCommandEvent)
- CONTEXT_START_LINE: 30832
- CONTEXT_END_LINE: 30854
- CONTEXT_TOKEN: interface TerminalCommandEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-001 (TerminalCommandEvent)**

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

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-002 (EditorEditEvent)
- CONTEXT_START_LINE: 30856
- CONTEXT_END_LINE: 30880
- CONTEXT_TOKEN: interface EditorEditEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-002 (EditorEditEvent)**

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

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-003 (DiagnosticEvent)
- CONTEXT_START_LINE: 30882
- CONTEXT_END_LINE: 30895
- CONTEXT_TOKEN: interface DiagnosticEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-003 (DiagnosticEvent)**

  interface DiagnosticEvent extends FlightRecorderEventBase {
    type: 'diagnostic';

    diagnostic_id: string;             // equals Diagnostic.id
    wsid?: string | null;
    severity?: 'fatal' | 'error' | 'warning' | 'info' | 'hint';
    source?: string;                   // optional echo for quick filtering
  }
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-004 (Retention & linkability)
- CONTEXT_START_LINE: 30897
- CONTEXT_END_LINE: 30910
- CONTEXT_TOKEN: Flight Recorder MUST:
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-004 (Retention & linkability)**

  Flight Recorder MUST:
  - Retain events for a configurable time window (default: 30 days),
  - Allow navigation:
    - job trace \\u0192+ terminal commands \\u0192+ opened files / edited documents,
    - diagnostics \\u0192+ Problems \\u0192+ Monaco/other surface locations,
    - terminal events \\u0192+ raw session output (subject to logging and redaction policy),
    - any operator action \\u0192+ the initiating UI surface (see VAL-CONSOLE-001).

  If evidence is missing due to retention, the UI MUST:
  - show that evidence is missing,
  - include the reason in Debug Bundle `retention_report.json`.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-005 (DebugBundleExportEvent)
- CONTEXT_START_LINE: 30911
- CONTEXT_END_LINE: 30928
- CONTEXT_TOKEN: interface DebugBundleExportEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-005 (DebugBundleExportEvent)**

  interface DebugBundleExportEvent extends FlightRecorderEventBase {
    type: 'debug_bundle_export';

    bundle_id: string;                 // uuid
    scope: 'problem' | 'job' | 'time_window' | 'workspace';
    redaction_mode: 'SAFE_DEFAULT' | 'WORKSPACE' | 'FULL_LOCAL';

    // What was intended vs what was included
    included_job_ids?: string[];
    included_diagnostic_ids?: string[];
    included_wsids?: string[];
    event_count?: number;
    missing_evidence?: { kind: string; reason: string }[];
  }
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.5 FR-EVT-WF-RECOVERY (WorkflowRecoveryEvent)
- CONTEXT_START_LINE: 30930
- CONTEXT_END_LINE: 30948
- CONTEXT_TOKEN: interface WorkflowRecoveryEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-WF-RECOVERY (WorkflowRecoveryEvent)**

  interface WorkflowRecoveryEvent extends FlightRecorderEventBase {
    type: 'workflow_recovery';

    workflow_run_id: string;
    job_id?: string;

    from_state: 'running';
    to_state: 'stalled';

    reason: string;
    last_heartbeat_ts: string; // RFC3339
    threshold_secs: number;
  }

  FR-EVT-WF-RECOVERY MUST be emitted when HSK-WF-003 transitions a workflow run from Running to Stalled. The actor MUST be 'system'.
  ```
