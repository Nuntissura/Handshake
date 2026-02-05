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
- WP_ID: WP-1-Tokenization-Service-v3
- CREATED_AT: 2026-01-01T02:11:20.7230799+01:00
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- SPEC_TARGET_SHA1: 76e8e6e8259b64a6dc4aed5cf2afb754ff1f3aed
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010120260219

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- Product/spec-to-code gap: Current implementation uses an async `Tokenizer` trait with only Tiktoken + heuristic fallback; SPEC_CURRENT requires a unified `TokenizationService` and model-specific tokenizers including SentencePiece for Llama/Mistral (Ollama) and an Ollama tokenizer-config fetch workflow.
- Observability gap: SPEC_CURRENT requires that Vibe Tokenizer usage emits `metric.accuracy_warning` to the Flight Recorder; current tokenization code only logs `tracing::warn!` and does not guarantee a Flight Recorder metric/event.
- Governance gap (packet hygiene): Prior packet `WP-1-Tokenization-Service-20251228` failed COR-701 gates (non-ASCII + missing manifest) and phase gate markers; remediation requires a new ASCII-only packet with a complete COR-701 manifest and correct phase ordering.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Trigger: When tokenization falls back to Vibe Tokenizer (model-specific tokenizer unavailable or unknown model).
- Required telemetry: Emit `metric.accuracy_warning` to the Flight Recorder (exact event type/shape must match the existing Flight Recorder API used elsewhere in `handshake_core`).

### RED_TEAM_ADVISORY (security failure modes)
- Budget bypass risk: Under-counting tokens (or using whitespace-split approximations) can defeat budget enforcement, enabling oversized prompts/responses and increasing prompt-injection blast radius.
- Model mismatch risk: Applying the wrong tokenizer to a model class can produce materially wrong counts, causing truncation errors and inconsistent JobMetrics.

### PRIMITIVES (traits/structs/enums)
- `TokenizationService` trait (per SPEC_CURRENT 4.6) with `count_tokens` and `truncate`.
- Implementations required by spec:
  - Tiktoken-backed tokenizer for GPT-class models.
  - SentencePiece-backed tokenizer for Llama/Mistral (Ollama) models.
  - Vibe Tokenizer fallback (char_count / 4.0) with Flight Recorder `metric.accuracy_warning`.
- A routing layer selecting the correct tokenizer per model (or per runtime-resolved tokenizer configuration).

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: SPEC_CURRENT provides explicit trait surface + required implementations + a strict "no whitespace split" rule + a required Flight Recorder accuracy warning for fallback usage.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: SPEC_CURRENT v02.100 defines the TokenizationService contract and required tokenizer behaviors explicitly in 4.6, including required implementations and fallback telemetry.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 4.6 Tokenization Service (Normative)
- CONTEXT_START_LINE: 5570
- CONTEXT_END_LINE: 5592
- CONTEXT_TOKEN: ### 4.6 Tokenization Service (Normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 4.6 Tokenization Service (Normative)

  The system MUST provide a unified `TokenizationService` to ensure budget compliance across different model architectures.

  ```rust
  pub trait TokenizationService {
      /// Count tokens for a given model architecture.
      /// MUST NOT split words on whitespace for BPE models (Llama3/Mistral).
      ///
      /// **Implementation Note:** This method is synchronous. Telemetry (like `metric.accuracy_warning`)
      /// MUST be emitted via a non-blocking mechanism (e.g., `tokio::spawn`, a background channel,
      /// or a thread-local collector) to preserve the synchronous signature and avoid blocking the hot path.
      fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError>;
      
      /// Truncate text to fit within a token limit.
      fn truncate(&self, text: &str, limit: u32, model: &str) -> String;
  }
  ```

  **Required Implementations:**
  1.  **Tiktoken:** For OpenAI/GPT-4 class models.
  2.  **SentencePiece:** For Llama3/Mistral (Ollama) models.
  3.  **VibeTokenizer (Fallback):** A rough estimator (char_count / 4) used ONLY when exact tokenizers fail or model is unknown.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 4.6 Tokenization and Metrics Contract (normative)
- CONTEXT_START_LINE: 11338
- CONTEXT_END_LINE: 11349
- CONTEXT_TOKEN: ### 4.6 Tokenization and Metrics Contract (normative)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 4.6 Tokenization and Metrics Contract (normative)

  For AI-autonomous operation, token counts MUST be accurate to ensure budget enforcement and billing (where applicable).

  1. **No String-Split Approximation:** Implementations MUST NOT use whitespace splitting for token counts in production.
  2. **Model-Specific Tokenizers:**
     - **GPT-class:** MUST use `tiktoken` or compatible BPE tokenizer.
     - **Llama/Mistral (Ollama):** MUST fetch the tokenizer configuration from the local runtime (e.g. `/api/show` in Ollama) and use the correct tokenizer (SentencePiece/Tiktoken).
  3. **Vibe Tokenizer (Fallback):** If a model-specific tokenizer is unavailable, the system MUST fallback to a "Vibe Tokenizer" which uses a `char_count / 4.0` heuristic.
     - **Audit Trail:** Vibe Tokenizer usage MUST emit a `metric.accuracy_warning` to the Flight Recorder.
     - **Sync/Async Bridge:** Because `count_tokens` is synchronous, this emission MUST be decoupled from the execution flow (e.g., via fire-and-forget `tokio::spawn` or a dedicated telemetry channel). It MUST NOT block the tokenization logic.
  4. **Consistency Invariant:** Token counts emitted to `JobMetrics` (A\\u00152.6.6.2.7) MUST match the counts used for retrieval budgeting (A\\u00152.6.6.7.14).
  ```

