## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any WP activation/signature.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by docs/ORCHESTRATOR_PROTOCOL.md Part 2.5.2.

### METADATA
- WP_ID: WP-1-AI-UX-Actions-v2
- CREATED_AT: 2026-01-25
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.117.md
- SPEC_TARGET_SHA1: 315a015dcfa3362f5aa9593f483a1d155d007323
- USER_REVIEW_STATUS: APPROVED
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-AI-UX-Actions-v2
- USER_SIGNATURE: ilja260120260054

### REQUIRED SECTIONS (per docs/ORCHESTRATOR_PROTOCOL.md Part 2.5.2)

### GAPS_IDENTIFIED
- Master Spec gaps: NONE (CLEARLY_COVERS_VERDICT=PASS; ENRICHMENT_NEEDED=NO).
- Current codebase gaps (inspection; do not trust prior packets):
  - No editor command palette surface exists for explicit AI actions (Command Palette invocation).
  - Document summarization exists as a direct button in `app/src/components/DocumentView.tsx`, but it is not exposed as a command palette action and does not support future parameterized job_inputs usage from UI.
  - No UI entrypoint exists for "Ask about this document" (explicit prompt) (deferred if backend job semantics are not implemented).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Invoking a Command Palette action MUST result in an AI job run whose lifecycle is linkable via `job_id` and `trace_id` in Operator Consoles.
- On any model call during the job, the backend MUST emit FR-EVT-006 (LlmInferenceEvent) with REQUIRED `trace_id` and `model_id` (per 11.5.2).
- On any UI-surfaced failure that is also recorded as a Diagnostic, the system MUST emit FR-EVT-003 (DiagnosticEvent) linking to the canonical `diagnostic_id` (per 11.5.1).

### RED_TEAM_ADVISORY (security failure modes)
- Prompt injection via document content: UI MUST NOT introduce "force_prompt_injection" style test hooks; job_inputs should be minimal and schema-shaped.
- Privacy leakage in UI: do not render raw payloads as HTML; display job outputs as escaped text; keep any debug JSON behind deliberate UX affordances and truncation.
- Capability bypass attempts: UI MUST NOT allow users to select arbitrary job_kind strings; it should use a fixed allowlist (e.g., doc_summarize) to avoid accidental unsupported job kinds and to preserve capability gating semantics.

### PRIMITIVES (traits/structs/enums)
- Job kinds (canonical strings): `doc_summarize` (and future `doc_edit` for ask flows), per 2.6.6.2.8.1.
- CreateJob request shape (frontend->backend): `{ job_kind, protocol_id, doc_id?, job_inputs? }`.
- Flight Recorder minimum observability: `trace_id` + `job_id` correlation, plus FR-EVT-006 emission on model calls.
- UI primitives: Command Palette invocation (open/close), action list, optional input box (future ask), and job status display (queued/running/completed/failed).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.117 defines (a) that UI actions map to AI jobs, (b) canonical JobKind strings (including doc_summarize), and (c) Flight Recorder event requirements for LLM inference correlation. No spec enrichment is required to activate a UI-facing command palette action that creates doc_summarize jobs.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec v02.117 already defines the Docs & Sheets AI job profile behavior and the canonical job kind strings and Flight Recorder event requirements needed for this WP; remaining work is implementation/UX wiring.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.5.10.3.1 (AI jobs; UI actions map to AI jobs)
- CONTEXT_START_LINE: 7415
- CONTEXT_END_LINE: 7434
- CONTEXT_TOKEN: map to AI jobs under the hood.
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.5.10.3.1 AI jobs

  ...

  UI actions like \\u201cRewrite paragraph\\u201d, \\u201cSummarise this section\\u201d, \\u201cFill this column\\u201d map to AI jobs under the hood.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.6.6.2.8.1 (JobKind Canonical Strings; doc_summarize)
- CONTEXT_START_LINE: 8622
- CONTEXT_END_LINE: 8640
- CONTEXT_TOKEN: - doc_summarize
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
  - micro_task_execution
  - spec_router
  - debug_bundle_export
  - terminal_exec
  - doc_ingest
  - distillation_eval

  Implementations MUST reject any other value at parse time.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 11.5.2 (FR-EVT-006 LlmInferenceEvent; trace_id+model_id required)
- CONTEXT_START_LINE: 52215
- CONTEXT_END_LINE: 52237
- CONTEXT_TOKEN: #### 11.5.2 FR-EVT-006 (LlmInferenceEvent)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.2 FR-EVT-006 (LlmInferenceEvent)

  interface LlmInferenceEvent extends FlightRecorderEventBase {
    type: 'llm_inference';

    trace_id: string;               // uuid; REQUIRED
    model_id: string;               // REQUIRED
    ...
  }

  Validation requirement: the Flight Recorder MUST reject `llm_inference` events missing `trace_id` or `model_id`.
  ```
