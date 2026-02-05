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
- WP_ID: WP-1-AI-UX-Summarize-Display-v2
- CREATED_AT: 2026-02-01T13:59:01Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_TARGET_SHA1: 4D406DCC1A75570D2F17659E0AC40D68A22F211A
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010220261515
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-AI-UX-Summarize-Display-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- The Operator Consoles Jobs surface requires a Job Inspector with a Summary tab, but the Summary tab currently does not present job outputs in a leak-aware way across safety contexts.
- The spec describes hash-based IO surfacing (SAFE_DEFAULT) vs preview (WORKSPACE) for job bundle material; this WP aligns the Job Inspector Summary tab behavior to those leak-safety principles (default to hashes; allow explicit preview when policy/safety permits).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- NONE (view-only UI; no new state-mutating operator actions are introduced by this WP).

### RED_TEAM_ADVISORY (security failure modes)
- Leak risk: job outputs may contain RawContent or sensitive material; default display should be hash-based and avoid automatic inline rendering.
- XSS / UI injection: job outputs and previews must be rendered as plain text, not as HTML.
- Stale display risk: displaying outputs from a different job or outdated run can mislead; always bind summary display to selected job_id and show outputs_hash for verification.

### PRIMITIVES (traits/structs/enums)
- `JobInspectorSummary` (UI component): renders status/ids/metrics and a safe representation of outputs (hash by default).
- `JobOutputsViewModel` (type): normalizes job_outputs into (outputs_hash, outputs_preview, structured_summary?) without leaking raw payloads by default.
- `OutputsRevealPolicy` (helper): gates whether previews can be shown based on available safety/policy indicators.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec explicitly requires a Job Inspector with a Summary tab, and defines hash-based vs preview IO handling for job bundle material; this is sufficient to implement leak-aware summary display without adding new normative text.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The spec already defines the required console surface (Job Inspector tabs) and the safe IO representation approach (hash vs preview); this WP is an implementation alignment task.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 10.5.5.2 Jobs (Normative)
- CONTEXT_START_LINE: 52973
- CONTEXT_END_LINE: 52978
- CONTEXT_TOKEN: Job Inspector with tabs: Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, Policy.
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 10.5.5.2 Jobs

  MUST:
  - List jobs with filters: status, kind, workspace (wsid), time range.
  - Provide a Job Inspector with tabs: Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, Policy.
  - Allow exporting a Debug Bundle scoped to a job.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 10.5.6.5.3 jobs.json / job.json (BundleJob) (Normative)
- CONTEXT_START_LINE: 53274
- CONTEXT_END_LINE: 53321
- CONTEXT_TOKEN: inputs_hash: string;
- EXCERPT_ASCII_ESCAPED:
  ```text
  interface BundleJob {
    job_id: string;
    job_kind: string;
    protocol_id: string;
    status: "queued" | "running" | "completed" | "failed" | "cancelled";

    // Timestamps
    created_at: string;
    started_at?: string;
    ended_at?: string;

    // Profile
    profile_id: string;
    capability_profile_id: string;

    // Context (IDs only in SAFE_DEFAULT)
    wsid?: string;
    doc_id?: string;

    // Inputs/Outputs as hashes (SAFE_DEFAULT) or previews (WORKSPACE)
    inputs_hash: string;
    outputs_hash?: string;
    inputs_preview?: string;              // first 200 chars, redacted
    outputs_preview?: string;             // first 200 chars, redacted
  }
  ```

