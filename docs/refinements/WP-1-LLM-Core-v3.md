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
- WP_ID: WP-1-LLM-Core-v3
- CREATED_AT: 2026-01-04T00:18:24.919Z
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md
- SPEC_TARGET_SHA1: 648dfd52b7cd0ad8183b9a037746473b875fa2c8- USER_REVIEW_STATUS: APPROVED- USER_SIGNATURE: ilja040120260217

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. Spec enrichment for LLM trace_id + llm_inference event shape has been applied in SPEC_CURRENT (Handshake_Master_Spec_v02.101.md).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Trigger: every LlmClient::completion call for Phase 1 runtime integration.
- Required emission: one Flight Recorder event per completion call using the canonical llm_inference shape (FR-EVT-006).
- Required correlation fields: trace_id and model_id.
- Required usage fields: token_usage {prompt_tokens, completion_tokens, total_tokens} and latency_ms (optional).
- Failure policy: emission failures must not fail the LLM call; surface as warning-level telemetry.

### RED_TEAM_ADVISORY (security failure modes)
- Budget bypass risk: if max_tokens is not enforced, token usage can exceed budgets silently.
- Traceability bypass risk: if trace_id is nil/missing, correlation breaks and debugging/auditability fails.
- Startup brittleness risk: if Ollama detection is blocking or fatal, the app can hard-fail instead of degrading with surfaced diagnostics.

### PRIMITIVES (traits/structs/enums)
- LlmClient trait and CompletionRequest/CompletionResponse/TokenUsage/LlmError.
- FR-EVT-006 LlmInferenceEvent shape for llm_inference.
- Ollama detection + token accounting contract.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The LlmClient trait, required request fields (including trace_id), and the required llm_inference event shape are explicitly specified in SPEC_CURRENT, enabling deterministic implementation and validation without further enrichment.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: SPEC_CURRENT (Handshake_Master_Spec_v02.101.md) now explicitly specifies trace_id transport for CompletionRequest and the FR-EVT-006 llm_inference event shape; downstream work can proceed without additional normative text changes.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
N/A (ENRICHMENT_NEEDED=NO)
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 4.2.3.1 / 4.2.3.1.1 (LlmClient + trace_id)
- CONTEXT_START_LINE: 10511
- CONTEXT_END_LINE: 10580
- CONTEXT_TOKEN: pub trace_id: Uuid
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.2.3.1.1 Traceability Addendum (Normative)
  ...CompletionRequest MUST include trace_id used for Flight Recorder correlation...
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 4.2.3.2 (Implementation Requirements)
- CONTEXT_START_LINE: 10583
- CONTEXT_END_LINE: 10590
- CONTEXT_TOKEN: Every call MUST emit a Flight Recorder event
- EXCERPT_ASCII_ESCAPED:
  ```text
  3. Observability: Every call MUST emit a Flight Recorder event ... containing trace_id, model_id, and TokenUsage.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 11.5.2 (FR-EVT-006 LlmInferenceEvent)
- CONTEXT_START_LINE: 30950
- CONTEXT_END_LINE: 30985
- CONTEXT_TOKEN: #### 11.5.2 FR-EVT-006 (LlmInferenceEvent)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.2 FR-EVT-006 (LlmInferenceEvent)
  interface LlmInferenceEvent extends FlightRecorderEventBase { type: 'llm_inference'; ... }
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 11.10.3 (Metrics and Tokens)
- CONTEXT_START_LINE: 33933
- CONTEXT_END_LINE: 33945
- CONTEXT_TOKEN: http://localhost:11434/api/tags
- EXCERPT_ASCII_ESCAPED:
  ```text
  2. Ollama Detection:
     - The system MUST check http://localhost:11434/api/tags on startup.
  ```
