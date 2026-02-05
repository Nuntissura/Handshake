## TECHNICAL_REFINEMENT (MASTER SPEC)

### METADATA
- WP_ID: WP-1-Spec-Enrichment-LLM-Core-v1
- CREATED_AT: 2026-01-04T01:05:55.6859528+01:00
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md
- SPEC_TARGET_SHA1: f35e1ea4cb1e74541c542ef1aadeee62128dab1e
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja040120260108

### GAPS_IDENTIFIED
- Procedure gap requiring enrichment: the LLM client adapter section includes a normative request type example, while also requiring trace_id-based correlation for observability. For deterministic implementation and validator consistency, the spec must explicitly state how trace_id is provided to LlmClient::completion and how llm_inference events are shaped.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Trigger: every LlmClient::completion call.
- Required: event correlation must include trace_id and model_id; token usage must be emitted.

### RED_TEAM_ADVISORY (security failure modes)
- Without an explicit trace_id transport rule, implementations can drift (nil/missing trace_id), breaking audit correlation and enabling silent bypass of traceability.

### PRIMITIVES (traits/structs/enums)
- LlmClient, CompletionRequest, CompletionResponse, TokenUsage, LlmError.
- Flight Recorder event shape for llm_inference.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [ ] FAIL
- Measurable acceptance criteria: [ ] FAIL
- No ambiguity: [ ] FAIL
- CLEARLY_COVERS_VERDICT: FAIL
- CLEARLY_COVERS_REASON: The required trace_id correlation is stated, but the normative request example and the absence of an explicit llm_inference event schema leave multiple valid interpretations for transport and validation.

### ENRICHMENT
- ENRICHMENT_NEEDED: YES

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
#### 4.2.3.1.1 Traceability Addendum (Normative)

To satisfy the traceability and observability requirements, every LLM completion MUST be attributable to a non-nil `trace_id`.

Normative requirement: the LLM completion request MUST include `trace_id` used for Flight Recorder correlation.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub trace_id: Uuid,          // REQUIRED: non-nil
    pub prompt: String,
    pub model_id: String,
    pub max_tokens: Option<u32>,
    pub temperature: f32,
    pub stop_sequences: Vec<String>,
}
```

#### 11.5.2 FR-EVT-006 (LlmInferenceEvent)

```ts
interface LlmInferenceEvent extends FlightRecorderEventBase {
  type: 'llm_inference';

  trace_id: string;               // uuid; REQUIRED
  model_id: string;               // REQUIRED

  token_usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };

  latency_ms?: number | null;
  prompt_hash?: string | null;
  response_hash?: string | null;
}
```

Validation requirement: the Flight Recorder MUST reject `llm_inference` events missing `trace_id` or `model_id`.
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 4.2.3.1.1 (Traceability Addendum)
- CONTEXT_START_LINE: 10565
- CONTEXT_END_LINE: 10590
- CONTEXT_TOKEN: #### 4.2.3.1.1 Traceability Addendum (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.2.3.1.1 Traceability Addendum (Normative)

  To satisfy the traceability and observability requirements, every LLM completion MUST be attributable to a non-nil trace_id.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 11.5.2 (FR-EVT-006 LlmInferenceEvent)
- CONTEXT_START_LINE: 30950
- CONTEXT_END_LINE: 30990
- CONTEXT_TOKEN: #### 11.5.2 FR-EVT-006 (LlmInferenceEvent)
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.2 FR-EVT-006 (LlmInferenceEvent)

  interface LlmInferenceEvent extends FlightRecorderEventBase {
    type: 'llm_inference';
  }
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 11.5 (Trace Invariant)
- CONTEXT_START_LINE: 30786
- CONTEXT_END_LINE: 30792
- CONTEXT_TOKEN: **Trace Invariant:**
- EXCERPT_ASCII_ESCAPED:
  ```text
  - **Trace Invariant:** Every AI action MUST emit a unique trace_id which links the Job, the RAG QueryPlan, and the final result.
  ```

