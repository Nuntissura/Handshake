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
- WP_ID: WP-1-AI-Job-Model-v4
- CREATED_AT: 2026-01-19
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: CF2F5305FC8EEC517D577D87365BD9C072A99B0F
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja200120260048
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-AI-Job-Model-v4

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- GAP-1 (JobKind normalization): Code writes `term_exec` for `JobKind::TerminalExec`; spec allows alias accept, but requires normalization to `terminal_exec` on write.
- GAP-2 (JobKind coverage): Spec canonical JobKind list includes `doc_rewrite`, `spec_router`, `doc_ingest`, `distillation_eval`; code currently lacks these variants and will reject them at parse time.
- GAP-3 (Non-spec JobKinds in storage): Code allows storing `doc_test` and `governance_pack_export`, but spec canonical list omits them and requires rejecting unknown values at parse time.
- GAP-4 (Stub spec drift): `.GOV/task_packets/stubs/WP-1-AI-Job-Model-v4.md` references v02.105, but `.GOV/roles_shared/SPEC_CURRENT.md` resolves to v02.113. v4 refinement + packet must anchor to v02.113.
- GAP-5 (Resolution to remove ambiguity): Conform to the canonical JobKind list without spec enrichment:
  - Do not store non-spec `job_kind` values. Replace `governance_pack_export` storage with canonical `workflow_run` (job remains a workflow-run job; operation details stay in `protocol_id` + `job_inputs`/`job_outputs`).
  - Remove `doc_test` as a persisted JobKind (not referenced in spec; appears unused in production paths).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-WF-RECOVERY (type: `workflow_recovery`): Must emit on startup scan when workflow transitions `Running -> Stalled` per HSK-WF-003 (actor=system).
- Trace correlation: `AiJob.trace_id` is required for Flight Recorder correlation (normative types); job execution should carry this through FR events.
- Note: `governance_pack_export` Flight Recorder event type exists in code; spec does not define it. Out of scope for this WP unless required to resolve JobKind conformance.

### RED_TEAM_ADVISORY (security failure modes)
- Alias drift risk: If `job_kind` strings are not canonicalized on write, downstream allowlists/capability enforcement may fork on aliases (`term_exec` vs `terminal_exec`), creating audit gaps and possible policy bypass.
- Unknown kind risk: If non-spec JobKinds can be stored, consumers that assume the canonical list may silently mis-handle jobs (mis-permissioning, missing validators, UI crashes).
- Metrics integrity risk: Any NULL metrics or partial/null JSON risks breaking observability, GC, and postmortems. DB schema should enforce NOT NULL + defaults, and code must not write NULL.

### PRIMITIVES (traits/structs/enums)
- Rust primitives: `AiJob`, `JobKind`, `JobState`, `JobMetrics`, `AccessMode`, `SafetyMode`, `EntityRef`, plus `FromStr` parsing for strict enum mapping.
- Storage primitives: `ai_jobs` table columns `job_kind`, `status`, `metrics`, `job_inputs`, `job_outputs`, `trace_id`, `workflow_run_id`, `protocol_id`, `profile_id`, `capability_profile_id`, `access_mode`, `safety_mode`.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec provides deterministic canonical JobKinds + alias normalization + JobState list; this WP aligns code/storage to those anchors.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec already defines canonical JobKinds, alias normalization, strict enum mapping, JobState list, and metrics integrity. This WP only aligns implementation to existing normative text.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.2.8 [HSK-JOB-100/101]
- CONTEXT_START_LINE: 5285
- CONTEXT_END_LINE: 5293
- CONTEXT_TOKEN: **[HSK-JOB-100] Strict Enum Mapping:**
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.2.8 Normative Rust Types

  To satisfy \\u00A72.6.6.2 core schema requirements, all implementations MUST use the following normative Rust structures.

  **[HSK-JOB-100] Strict Enum Mapping:**
  `JobKind` MUST be implemented as a Rust `enum`. Fallback to `String` for storage purposes MUST use a validated `FromStr` implementation to prevent illegal states.

  **[HSK-JOB-101] Metrics Integrity:**
  `JobMetrics` MUST NOT contain `NULL` values in the database. A zeroed `JobMetrics` object MUST be created at job initialization.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.2.8.1 (JobKind Canonical Strings)
- CONTEXT_START_LINE: 5369
- CONTEXT_END_LINE: 5389
- CONTEXT_TOKEN: ##### 2.6.6.2.8.1 JobKind Canonical Strings (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.6.6.2.8.1 JobKind Canonical Strings (Normative)

  The canonical storage/API strings for JobKind are:

  - doc_edit
  - doc_rewrite
  - doc_summarize
  - sheet_transform
  - canvas_cluster
  - asr_transcribe
  - workflow_run
  - spec_router
  - debug_bundle_export
  - terminal_exec
  - doc_ingest
  - distillation_eval

  Implementations MUST reject any other value at parse time.

  Legacy alias (only if needed for backward compatibility):
  - term_exec MAY be accepted as an alias for terminal_exec, but MUST be normalized to terminal_exec on write.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.2.8 (JobState enum)
- CONTEXT_START_LINE: 5347
- CONTEXT_END_LINE: 5359
- CONTEXT_TOKEN: pub enum JobState {
- EXCERPT_ASCII_ESCAPED:
  ```text
  #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
  pub enum JobState {
      Queued,
      Running,
      Stalled,
      AwaitingValidation,
      AwaitingUser,
      Completed,
      CompletedWithIssues,
      Failed,
      Cancelled,
      Poisoned
  }
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.1 [HSK-WF-003] + FR-EVT-WF-RECOVERY
- CONTEXT_START_LINE: 4965
- CONTEXT_END_LINE: 4966
- CONTEXT_TOKEN: **[HSK-WF-003] Startup Recovery Loop (Normative):**
- EXCERPT_ASCII_ESCAPED:
  ```text
  **[HSK-WF-003] Startup Recovery Loop (Normative):**
  Upon system initialization (within the `run()` loop of `main.rs`), the Workflow Engine MUST execute a non-blocking scan for `Running` workflows whose `last_heartbeat` is > 30 seconds old. These MUST be transitioned to `Stalled` and logged to the Flight Recorder with `actor: System` and event type `FR-EVT-WF-RECOVERY`. This recovery MUST occur before the system begins accepting new AI jobs.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.5.2 (FR-EVT-WF-RECOVERY schema)
- CONTEXT_START_LINE: 46484
- CONTEXT_END_LINE: 46502
- CONTEXT_TOKEN: FR-EVT-WF-RECOVERY (WorkflowRecoveryEvent)
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

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 7.6.3 Phase 1 (AI Job Model MVP)
- CONTEXT_START_LINE: 35167
- CONTEXT_END_LINE: 35170
- CONTEXT_TOKEN: AI Job Model (minimum viable implementation)
- EXCERPT_ASCII_ESCAPED:
  ```text
  2. **AI Job Model (minimum viable implementation)**
     - Implement the **global AI Job Model** (Section 2.6.6) in the backend:
       - `job_id`, `job_kind`, `protocol_id`, `status`, timestamps, error, inputs, outputs, metrics.
       - Profile fields (`profile_id`, `capability_profile_id`, `access_mode`, `safety_mode`).
  ```

