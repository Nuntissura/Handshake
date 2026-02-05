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
- WP_ID: WP-1-Metrics-Mock-Tokens
- CREATED_AT: 2026-01-18T16:32:30.000Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: cf2f5305fc8eec517d577d87365bd9c072a99b0f
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja180120262320
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Metrics-Mock-Tokens

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (spec text is explicit; no missing normative clauses detected).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- This WP does not introduce new Flight Recorder event IDs.
- Expected interaction: mock LLM TokenUsage values must be non-zero/deterministic so existing LLM inference events carry realistic token metrics for propagation testing.

### RED_TEAM_ADVISORY (security failure modes)
- Risk: deterministic mock token accounting could accidentally bleed into production provider paths, masking real token/budget regressions.
- Mitigation: restrict deterministic token heuristics to explicit mock/test clients only; never override provider-reported token counts.

### PRIMITIVES (traits/structs/enums)
- TokenUsage (existing): ensure deterministic values are emitted by mock LLM clients.
- InMemoryLlmClient (existing): add deterministic token accounting behavior for mock usage (e.g., 10 tokens per word) to support metric propagation tests.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec 11.10.3 explicitly defines required token accounting behavior for real Ollama calls and deterministic mock token values, plus startup detection behavior.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec 11.10.3 provides concrete, implementable rules; no additional normative text is required.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.10.3 (Metrics & Tokens)
- CONTEXT_START_LINE: 49537
- CONTEXT_END_LINE: 49545
- CONTEXT_TOKEN: For mocks, emit deterministic values
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 11.10.3 Metrics & Tokens
  
  1.  **Token Accounting:** 
      -   For real Ollama calls, use the `prompt_eval_count` and `eval_count` fields from the response.
      -   For mocks, emit deterministic values (e.g., 10 tokens per word) to test metric propagation.
  2.  **Ollama Detection:** 
      -   The system MUST check `http://localhost:11434/api/tags` on startup. 
      -   If available, enable `OllamaClient` by default.
  ```

