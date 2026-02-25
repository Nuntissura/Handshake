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
- WP_ID: WP-1-Unified-Tool-Surface-Contract-v1
- CREATED_AT: 2026-02-24T02:04:50Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.137.md
- SPEC_TARGET_SHA1: 258012967E37EECAF5EABF3B163D7A363AFAD5B1
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja240220260346
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Unified-Tool-Surface-Contract-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (no Master Spec enrichment required for this WP).
- Implementation gap (expected WP output): `assets/schemas/htc_v1.json` is required by spec (6.0.2.5.1) but is missing in the repo today; this WP must add it and wire Tool Gate validation to it.
- Implementation choice (non-ambiguous): Tool Registry storage representation/path is not fixed by spec; pick a single SSoT and enforce "no parallel schema" (4.3.9.16.4 + 6.0.2 + 11.3.0).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-007 `tool_call` MUST be emitted for every tool invocation across transports (local/mcp/mex/stage_bridge/other) with:
  - args/result artifact-first refs and sha256 hashes computed AFTER redaction.
  - correlation fields: `trace_id` + `tool_call_id` always present; Tool Gate enforces required fields.
- Tool Gate MUST reject invalid HTC envelopes with `ok=false` + `error.kind="validation"` + `error.code="VAL-HTC-001"` (6.0.2.5.1).

### RED_TEAM_ADVISORY (security failure modes)
- Dual schema drift: local tool calling vs MCP tool calling diverge, enabling bypass paths and confusing approvals/audit.
- Untrusted MCP metadata/prompt injection: any tool annotations from remote servers are advisory only; local Tool Gate + Tool Registry must remain authoritative (11.3.0).
- Secret/PII leakage: args/result must never be logged inline; persist only redacted payload refs + hashes (FR-EVT-007).
- Capability confusion: missing/incorrect `session_id` correlation causes wrong capability intersection and unintended escalation/approval behavior (6.0.2.5).
- Replay/idempotency hazards: missing idempotency keys for retryable tools can double-apply side effects (6.0.2.3).

### PRIMITIVES (traits/structs/enums)
- `ToolRegistryEntry` (struct): `tool_id`, `tool_version`, schemas, side_effect/idempotency/determinism/availability, required_capabilities, deprecation fields, examples.
- `ToolGateDecision` (enum/struct): allow|deny|escalate + normalized reason codes + capability_ids + redaction summary.
- `HtcEnvelope` (structs): request/response/error objects conforming to HTC-1.0 (validated against `assets/schemas/htc_v1.json`).
- `ToolTransport` (enum): local|mcp|mex|stage_bridge|other (align to FR-EVT-007).
- `ToolConformanceSuite` (tests): schema validation, payload caps, capability deny-by-default, FR-EVT-007 emission, idempotency behavior.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec contains normative HTC-1.0 envelope + validation rules (6.0.2.5/5.1), conformance tests (6.0.2.9), MCP binding requirements (11.3.0), and Flight Recorder ToolCallEvent schema + redaction/validation requirements (11.5.2 FR-EVT-007). Remaining choices are implementation details, not spec gaps.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec already defines the contract, validation behavior, MCP binding, and FR event schema needed to implement Tool Registry + Tool Gate + conformance tests.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 6.0.2.2-6.0.2.3 (tool identity/versioning; side_effect/idempotency/determinism; routing rule)
- CONTEXT_START_LINE: 22442
- CONTEXT_END_LINE: 22478
- CONTEXT_TOKEN: - `side_effect`: one of `READ | WRITE | EXECUTE`
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 6.0.2.2 Tool identity, naming, and versioning (MUST)
  
  Each tool MUST have a stable identity:
  
  - `tool_id` (string): lowercase dot-separated identifier matching:
    - regex: `^[a-z0-9_]+(\.[a-z0-9_]+)+$`
    - examples: `workspace.entity.get`, `stage.jobs.enqueue`, `photo.search`, `context.search`, `engine.version.status`
  - `tool_version` (string): semantic version `MAJOR.MINOR.PATCH`
  
  Rules:
  1. **Stability:** `tool_id` MUST be stable across releases.
  2. **Breaking changes:** any breaking change to input/output schema MUST bump `MAJOR`.
  3. **Deprecation:** deprecated tools MUST declare:
     - `deprecated_since` (semver)
     - `sunset_on` (ISO date)
     - `replaced_by` (optional tool_id)
  4. **No silent behavior drift:** side-effect classification and required capabilities MUST NOT change without a MAJOR bump.
  
  #### 6.0.2.3 Side effects, idempotency, determinism (MUST)
  
  Each tool MUST declare:
  
  - `side_effect`: one of `READ | WRITE | EXECUTE`
    - **READ:** no persistent mutation and no external side effects
    - **WRITE:** mutates workspace state (documents, canvases, tables, metadata, etc.)
    - **EXECUTE:** triggers external side effects (filesystem writes outside workspace store, processes, network, remote APIs)
  
  - `idempotency`: one of `IDEMPOTENT | IDEMPOTENT_WITH_KEY | NON_IDEMPOTENT`
    - `IDEMPOTENT_WITH_KEY` tools MUST accept `idempotency_key` and dedupe retries.
  
  - `determinism`: one of `DETERMINISTIC | BEST_EFFORT | NON_DETERMINISTIC`
    - tools MUST document sources of nondeterminism (e.g., remote calls, timestamps).
  
  - `availability`: one of `OFFLINE_OK | REQUIRES_NETWORK | BEST_EFFORT_OFFLINE`
  
  **Routing rule (MUST):**
  - All `WRITE` and `EXECUTE` tools MUST execute via the AI Job Model (A\u00A72.6). Inline execution is permitted only for bounded, synchronous READ tools.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 6.0.2.5.1 HTC-1.0 JSON Schema file (SSoT) (MUST)
- CONTEXT_START_LINE: 22552
- CONTEXT_END_LINE: 22567
- CONTEXT_TOKEN: assets/schemas/htc_v1.json
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 6.0.2.5.1 HTC-1.0 JSON Schema file (SSoT) (MUST)
  
  Handshake MUST define the HTC-1.0 envelope as a single JSON Schema file checked into the repository:
  
  - `assets/schemas/htc_v1.json` (JSON Schema draft 2020-12)
  
  Rules:
  1. **SSoT:** `htc_v1.json` is the single source of truth for the request/response/error envelopes of `schema_version: "htc-1.0"`.
  2. **Runtime validation (local + MCP):** the Tool Gate MUST validate every tool call envelope against `htc_v1.json`:
     - Local tool calling (IPC / in-process / MEX adapters) MUST validate the envelope **before execution** and MUST validate the response envelope **before return**.
     - MCP transports MUST validate at the Rust Gate boundary **before forwarding** `tools/call` and **before accepting** a tool response.
  3. **Failure behavior:** if envelope validation fails, the call MUST be rejected with:
     - `ok=false`
     - `error.kind="validation"`
     - `error.code="VAL-HTC-001"`
  4. **Versioning:** any breaking change to the envelope MUST bump `schema_version` (e.g., `htc-2.0`) and introduce a new schema file `assets/schemas/htc_v2.json`. Non-breaking clarifications MAY update `htc_v1.json` without changing `schema_version`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 6.0.2.9 Conformance tests (MUST)
- CONTEXT_START_LINE: 22601
- CONTEXT_END_LINE: 22609
- CONTEXT_TOKEN: #### 6.0.2.9 Conformance tests (MUST)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 6.0.2.9 Conformance tests (MUST)
  
  All tool implementations (local or MCP-backed) MUST pass conformance checks:
  - input/output schema validation
  - side_effect classification verification (policy tests)
  - enforced payload size limits (32KB rule)
  - capability gating (deny-by-default)
  - FR event emission (see FR-EVT-007 ToolCallEvent, A\u00A711.5)
  - idempotency behavior for retryable calls
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 11.3.0 Canonical Tool Contract Binding (Normative)
- CONTEXT_START_LINE: 62758
- CONTEXT_END_LINE: 62779
- CONTEXT_TOKEN: tools/list
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.3.0 Canonical Tool Contract Binding (Normative)
  
  MCP is one transport for tool invocation. Any tool exposed over MCP MUST conform to the Unified Tool Surface Contract (\u00A76.0.2).
  
  Normative requirements:
  
  1. **Single source of truth:** `tools/list` MUST be generated from the Tool Registry (not handwritten tool stubs).
  2. **Stable identity:** MCP `Tool.name` MUST equal `tool_id`.
  3. **Schemas:** MCP `Tool.inputSchema` MUST match the canonical JSON Schema for the tool. MCP `Tool.outputSchema` SHOULD be provided where possible.
  4. **Metadata binding:** additional Handshake metadata MUST be included under `Tool._meta.handshake`, including:
     - `tool_version`
     - `side_effect`, `idempotency`, `determinism`, `availability`
     - `required_capabilities[]`
     - `deprecated_since` / `sunset_on` / `replaced_by` (when applicable)
  5. **Invocation correlation:** MCP `tools/call` MUST accept and preserve Handshake correlation metadata in `_meta`, including:
     - `trace_id`
     - `tool_call_id`
     - `idempotency_key` (when applicable)
     - `session_id` and `actor` identity (agent_id/model_id)
  6. **Progress:** long-running tools SHOULD emit progress notifications where the transport supports it.
  7. **Security:** tool annotations/hints received from untrusted MCP servers are advisory only. Capability decisions MUST be made by the Tool Gate (A\u00A711.3.2) using the local Tool Registry + Capabilities & Consent Model (A\u00A711.1).
  8. **Observability:** every MCP tool invocation MUST emit a ToolCallEvent (FR-EVT-007, A\u00A711.5) and MUST be linkable to its parent spans/jobs via `trace_id`.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.137.md 11.5.2 FR-EVT-007 (ToolCallEvent)
- CONTEXT_START_LINE: 64391
- CONTEXT_END_LINE: 64452
- CONTEXT_TOKEN: interface ToolCallEvent extends FlightRecorderEventBase {
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **FR-EVT-007 (ToolCallEvent)**
  
  ```ts
  interface ToolCallEvent extends FlightRecorderEventBase {
    type: 'tool_call';
  
    trace_id: string;                 // uuid; REQUIRED
    tool_call_id: string;             // uuid; REQUIRED
  
    tool_id: string;                  // REQUIRED
    tool_version: string;             // REQUIRED
  
    transport: 'local' | 'mcp' | 'mex' | 'stage_bridge' | 'other';
  
    side_effect: 'READ' | 'WRITE' | 'EXECUTE';
    idempotency: 'IDEMPOTENT' | 'IDEMPOTENT_WITH_KEY' | 'NON_IDEMPOTENT';
    idempotency_key?: string | null;
  
    actor: {
      kind: 'human' | 'agent' | 'system';
      agent_id?: string | null;
      model_id?: string | null;
    };
  
    ok: boolean;
  
    // Tool arguments/results are artifact-first. Inline snapshots MUST be redacted.
    args_ref?: string | null;         // ArtifactHandle (redacted)
    args_hash?: string | null;        // sha256 of canonical JSON post-redaction
    result_ref?: string | null;       // ArtifactHandle (redacted)
    result_hash?: string | null;      // sha256 of canonical JSON post-redaction
  
    error?: {
      code: string;
      kind: string;
      message?: string | null;
      retryable?: boolean | null;
    } | null;
  
    timing: {
      started_at: string;             // ISO8601
      ended_at: string;               // ISO8601
      duration_ms: number;
    };
  
    resources_touched?: {
      workspace_ids?: string[];
      artifacts?: string[];
      files?: string[];
      urls?: string[];
    } | null;
  
    capability_ids?: string[];        // capabilities asserted/required for this call
    parent_span_id?: string | null;   // correlation for nested calls (e.g., sandbox \u201CCode Mode\u201D)
  }
  ```
  
  Redaction requirements (MUST):
  - `args_hash` / `result_hash` MUST be computed after secret redaction.
  - `args_ref` / `result_ref` MUST point to the redacted payload. Raw secrets MUST NOT be persisted in Flight Recorder.
  
  Validation requirement: the Flight Recorder MUST reject `tool_call` events missing `trace_id`, `tool_call_id`, `tool_id`, or `tool_version`.
  ```
