## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED before any WP activation/signature.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md Part 2.5.2.

### METADATA
- WP_ID: WP-1-Flight-Recorder-UI-v3
- CREATED_AT: 2026-01-17T00:00:00Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: cf2f5305fc8eec517d577d87365bd9c072a99b0f
- USER_REVIEW_STATUS: APPROVED
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Flight-Recorder-UI-v3
- USER_SIGNATURE: ilja170120262341

### REQUIRED SECTIONS (per .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md Part 2.5.2)

### GAPS_IDENTIFIED
- Master Spec gaps: NONE (CLEARLY_COVERS_VERDICT=PASS; ENRICHMENT_NEEDED=NO).
- Current codebase gap (inspection, do not trust prior attempts):
  - `app/src/components/FlightRecorderView.tsx` renders a timeline, but only filters by job_id/trace_id/from/to; the Spec requires Timeline with filters + deep links, and cross-surface navigation guarantees for `diagnostic_id`, `job_id`, `event_id`, `wsid`.
  - UI currently provides trace_id deep-link filtering only; it does not provide deterministic deep-link navigation targets for event_id/wsid/diagnostic_id, nor does it surface deep-link failures as Diagnostics.
  - Security violations are highlighted, but the console must remain safe-by-default: avoid accidental secret exposure in UI surfaces (redaction expectations come from the system; UI must not add new leaks).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Timeline view MUST make Flight Recorder events searchable/filterable and navigable via deep links (job_id, trace_id, event_id, wsid, diagnostic_id).
- When security violations occur (FR-EVT-SEC-VIOLATION), the UI MUST render them as prominent timeline entries and preserve linkability fields for debugging (trace_id, job_id, and any related ids in payload).

### PRIMITIVES (traits/structs/enums)
- Deep link identifiers (VAL-NAV-001): diagnostic_id, job_id, event_id, wsid.
- Console surfaces (Operator Consoles v1): Timeline (Flight Recorder events), Jobs (Job History + inspector), Problems (Diagnostics), Evidence Drawer.
- Timeline filtering fields (minimum set for this WP): job_id, trace_id, event_id, wsid, actor, event_type, from/to time range.
- Security violation representation: FR-EVT-SEC-VIOLATION must appear as a timeline entry and be easily discoverable (highlighted + filterable).

### RED_TEAM_ADVISORY (security failure modes)
- Secret leakage in UI: the UI MUST NOT introduce new rendering paths that expose raw secrets (e.g., "View Data" JSON dumps). If redaction is incomplete, provide safe truncation and clear warnings; do not copy/paste secrets into additional UI fields.
- Untrusted payload rendering: do not interpret payload as HTML; render as text/code with safe escaping.
- Link spoofing: deep links must be deterministic and validated; invalid IDs must surface as Diagnostics (no silent no-ops).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The Master Spec v02.113 explicitly requires a Flight Recorder timeline UI with filters + deep links and defines cross-surface navigation guarantees for deep link ids (diagnostic_id/job_id/event_id/wsid), plus security violation emission to Flight Recorder. No spec enrichment is required to implement these UI behaviors.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: The Master Spec v02.113 already contains normative requirements for Operator Consoles Timeline (filters + deep links) and deterministic deep-link navigation guarantees (VAL-NAV-001), so a spec change is not needed to implement WP-1-Flight-Recorder-UI-v3.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:35191 Flight Recorder (always-on, with UI) + Operator Consoles v1 Timeline (filters + deep links)
- CONTEXT_START_LINE: 35191
- CONTEXT_END_LINE: 35205
- CONTEXT_TOKEN: Operator Consoles v1
- EXCERPT_ASCII_ESCAPED:
```md
5. **Flight Recorder (always-on, with UI)**  
   - Implement a Flight Recorder subsystem (Section 2.1.5 and Bootloader clauses) with:
     - Append-only log of AI job lifecycle events.  
     - Append-only log of model calls (model name, tokens, latency, outcome).  
     - Minimal tags to correlate events (job ID, workflow ID, document ID, user ID where applicable).  
     - Back the Flight Recorder log store with **DuckDB** to support filtered queries at MVP scale.  
   - Provide a **Job History** panel in the UI:
     - List jobs with status, timestamps, model used, and linked document.  
     - Ability to inspect job input and output payloads.  
     - Provide a basic Flight Recorder filter for `job_id` and `status` to quickly locate related runs.
     - Provide an **Operator Consoles v1** surface (see A\u001510.5):
       - **Timeline** view (Flight Recorder events with filters + deep links).
       - **Jobs** view (Job History + per-job inspector).
       - **Problems** view (normalized diagnostics, grouped/deduped, clickable evidence).
       - **Evidence drawer** that shows: job summary, linked trace slice, linked diagnostics, and referenced entities/files.
```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:35261 MCP Gate requirement includes Flight Recorder UI visibility for MCP request/response
- CONTEXT_START_LINE: 35261
- CONTEXT_END_LINE: 35274
- CONTEXT_TOKEN: MCP request/response path visible in the Flight Recorder UI
- EXCERPT_ASCII_ESCAPED:
```md
11. **MCP skeleton and Gate (Target 1 + job/log plumbing)**  
    - Implement a minimal MCP client stack in the Rust coordinator, even if only exercised against a local stub server:
      - JSON-RPC transport and tool/resource discovery for at least one MCP server.  
      - Connection lifecycle tied to workspace/session where appropriate.  
    - Implement the MCP **Gate** interceptor (Section 11.3.2) as middleware around the MCP client:
      - Intercept `tools/call` requests, attach `job_id` / workflow run IDs and capability metadata.  
      - Enforce basic consent decisions and log them into Flight Recorder.  
      - Capture and log `tools/call` and `sampling/createMessage` traffic end-to-end, even when using a stub MCP server.  
    - Extend the AI Job Model to support MCP jobs:
      - Add a `transport_kind = "mcp"` discriminator and fields for `mcp_server_id` and `tool_name` where applicable.  
      - Ensure at least one test job profile uses MCP end-to-end (job \u0192+' MCP call \u0192+' response \u0192+' logs).  
    - Ensure Flight Recorder can represent MCP events using the canonical event shape in Section 11.3:
      - At least one MCP request/response path visible in the Flight Recorder UI.  
      - Clear correlation between a UI action, the AI job, and the MCP tool call(s) it triggered.  
```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:35373 Operator Consoles MUST deep-link chain from Job to Evidence items (SourceRefs/ArtifactHandles)
- CONTEXT_START_LINE: 35365
- CONTEXT_END_LINE: 35374
- CONTEXT_TOKEN: Operator Consoles MUST deep-link
- EXCERPT_ASCII_ESCAPED:
```md
18. **[ADD v02.52] Retrieval Correctness & Efficiency (ACE-RAG-001) \u0192?" Phase 1 plumbing**
   - Emit and persist `QueryPlan` and `RetrievalTrace` for every retrieval-backed model call; link both to the `ContextSnapshot` / `PromptEnvelope`.
   - Implement deterministic `normalized_query_hash` (sha256 of normalized query text) and record it in `RetrievalTrace`.
   - Compute and record `CacheKey` for cacheable stages (even if cache is initially a stub); log cache hit/miss per stage.
   - Enforce hard budgets at runtime:
     - `RetrievalBudgetGuard` (evidence tokens/snippet counts/read caps; deterministic truncation with flags).
     - `CacheKeyGuard` (strict mode requires cache key computation + logging).
   - Add a minimal Semantic Catalog registry (built-in) so routing does not depend on \u0192?oLLM guessing\u0192?? store/tool names.
   - Operator Consoles MUST deep-link: Job \u0192+' Model Call \u0192+' QueryPlan/Trace \u0192+' Evidence items (SourceRefs/ArtifactHandles) without opening raw documents by default.
```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:41809 Console surfaces MUST deep-link via job_id/diagnostic_id/wsid/event ids
- CONTEXT_START_LINE: 41807
- CONTEXT_END_LINE: 41810
- CONTEXT_TOKEN: MUST deep-link to each other via `job_id`
- EXCERPT_ASCII_ESCAPED:
```md
### 10.5.5 Console surfaces

All surfaces below MUST deep-link to each other via `job_id`, `diagnostic_id`, `wsid`, and Flight Recorder event ids (see A\u001511.4 and A\u001511.5; \u00a711.4 and \u00a711.5).
```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:46286 VAL-NAV-001 deep-link resolution guarantees + required tests
- CONTEXT_START_LINE: 46286
- CONTEXT_END_LINE: 46296
- CONTEXT_TOKEN: VAL-NAV-001: Cross-surface navigation guarantees
- EXCERPT_ASCII_ESCAPED:
```md
#### VAL-NAV-001: Cross-surface navigation guarantees
- For each supported deep link type (`diagnostic_id`, `job_id`, `event_id`, `wsid`):
  - the console MUST resolve the id to an entity/event,
  - the UI MUST provide a deterministic navigation target (surface + location),
  - failures MUST surface as Diagnostics (not silent no-ops).
- A test suite MUST cover:
  - diagnostics with file ranges \u0192+' Monaco (when present),
  - diagnostics without file ranges \u0192+' Evidence Drawer,
  - job_id \u0192+' Job Inspector,
  - event_id \u0192+' Timeline focus.
```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md:6708 HSK-ACE-VAL-101 emits FR-EVT-SEC-VIOLATION to Flight Recorder
- CONTEXT_START_LINE: 6708
- CONTEXT_END_LINE: 6713
- CONTEXT_TOKEN: FR-EVT-SEC-VIOLATION
- EXCERPT_ASCII_ESCAPED:
```md
**[HSK-ACE-VAL-101] Atomic Poisoning Directive:**
The `WorkflowEngine` MUST implement a global trap for `AceError::PromptInjectionDetected`. Upon detection:
1.  Immediate commit of `JobState::Poisoned`.
2.  Abrupt termination of all active workflow nodes.
3.  Emission of `FR-EVT-SEC-VIOLATION` to Flight Recorder.
4.  **No further workspace mutations** are permitted for that `job_id`.
```

