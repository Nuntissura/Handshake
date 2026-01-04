# Task Packet: WP-1-LLM-Core-v3

## METADATA
- TASK_ID: WP-1-LLM-Core-v3
- WP_ID: WP-1-LLM-Core-v3
- DATE: 2026-01-04T01:42:13.679Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja040120260217

## USER_CONTEXT (Non-Technical Explainer)
This work packet fixes the "AI calling" plumbing inside Handshake so it is consistent and auditable.

When Handshake asks Ollama to generate text, it must always:
1) Use a request ID ("trace_id") so we can correlate the call with logs.
2) Record exactly one standard "llm_inference" event with the required fields (model, request ID, token usage, optional timing/hashes).
3) Track token usage using the real provider counts when available, so budgets are enforced correctly.
4) Check on startup whether Ollama is reachable, and only enable it by default when it is actually available.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-LLM-Core-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Align LLM core runtime to SPEC_CURRENT v02.101 for (1) token accounting, (2) FR-EVT-006 llm_inference event shape, and (3) Ollama availability detection at startup.
- Why: The Master Spec is normative; Phase 1 requires deterministic observability and token budgeting invariants. Current code has partial alignment (trace_id present and llm_inference emitted) but does not match the FR-EVT-006 schema and does not perform the required /api/tags startup detection.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/main.rs
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps (spec is already enriched; see docs/refinements/WP-1-LLM-Core-v3.md).
  - Any UI/frontend work.
  - Any storage/migration changes.
  - Any non-Ollama provider support beyond Phase 1 requirements.
  - Any changes to other Flight Recorder event types unrelated to FR-EVT-006.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- WAIVER-20260104-LLMCORE-01: Spec enrichment already completed; do not create a new spec version for this WP. Keep spec authority as docs/SPEC_CURRENT.md in this worktree only. Approved by ilja on 2026-01-04.

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-LLM-Core-v3

# Spec integrity (must remain PASS):
just validator-spec-regression

# Supply chain (repo uses pnpm; treat this as the Step 3 equivalent of npm audit):
pwsh -NoProfile -Command "cd src/backend/handshake_core; cargo deny check advisories licenses bans sources"
pnpm -C app audit --audit-level high

# Backend build + tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Targeted tests (must exist after implementation):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml llm::ollama::tests::test_llm_inference_payload_matches_fr_evt_006
cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder::tests::test_llm_inference_payload_validation_requires_trace_id_and_model_id
cargo test --manifest-path src/backend/handshake_core/Cargo.toml flight_recorder::tests::test_llm_inference_payload_validation_requires_token_usage_object

# Lint/format:
cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings

# Protocol/hygiene helpers:
just validator-scan
just validator-error-codes

# External Cargo target hygiene + deterministic gate:
just cargo-clean
just post-work WP-1-LLM-Core-v3
```

### DONE_MEANS
- LLM client adapter matches SPEC_CURRENT v02.101 4.2.3:
  - CompletionRequest requires non-nil trace_id and is propagated through OllamaAdapter.
  - Budget enforcement: OllamaAdapter returns HSK-402-BUDGET-EXCEEDED when provider exceeds max_tokens.
- Token accounting matches SPEC_CURRENT v02.101 11.10.3:
  - For real Ollama calls, TokenUsage is sourced from provider response fields (prompt_eval_count + eval_count) when present.
  - Tokenization-derived counts may be computed for diagnostics/comparison, but are not the primary usage numbers when provider counts are present.
- Observability matches SPEC_CURRENT v02.101 4.2.3.2 + 11.5.2 FR-EVT-006:
  - Every OllamaAdapter completion emits exactly one llm_inference Flight Recorder event.
  - The llm_inference payload schema matches 11.5.2 (FR-EVT-006): includes type='llm_inference', trace_id, model_id, token_usage (prompt_tokens, completion_tokens, total_tokens), and optional latency_ms/prompt_hash/response_hash.
  - Flight Recorder validation rejects llm_inference events missing trace_id or model_id, and rejects events missing token_usage or required token_usage fields.
- Ollama detection matches SPEC_CURRENT v02.101 11.10.3:
  - On startup, the system checks http://localhost:11434/api/tags (or configured base URL) and only enables OllamaClient by default when available.
- Targeted tests exist and fail if the FR-EVT-006 payload mapping/validation or token-usage selection logic is removed.
- All TEST_PLAN commands pass and just post-work WP-1-LLM-Core-v3 returns PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.101.md (recorded_at: 2026-01-04T01:42:13.679Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 4.2.3 (LLM Client Adapter) + 11.5.2 (FR-EVT-006) + 11.10.3 (Metrics & Tokens)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.101.md
  - docs/refinements/WP-1-LLM-Core-v3.md
  - docs/task_packets/WP-1-LLM-Core-v3.md
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/main.rs
- SEARCH_TERMS:
  - "LlmInferenceEvent"
  - "validate_llm_inference_payload"
  - "prompt_eval_count"
  - "eval_count"
  - "api/tags"
  - "init_llm_client"
  - "FlightRecorderEventType::LlmInference"
- RUN_COMMANDS:
  ```bash
  rg -n "LlmInferenceEvent|validate_llm_inference_payload|FlightRecorderEventType::LlmInference" src/backend/handshake_core/src
  rg -n "prompt_eval_count|eval_count|TokenUsage" src/backend/handshake_core/src/llm/ollama.rs
  rg -n "init_llm_client" src/backend/handshake_core/src/main.rs
  ```
- RISK_MAP:
  - "event schema drift" -> "validator rejects llm_inference; observability breaks"
  - "token usage drift" -> "budget enforcement incorrect; silent overruns"
  - "startup detection incorrect" -> "LLM hard-fails or enables when unavailable"

## SKELETON
Proposed interfaces/types/contracts (concrete signatures; no implementation yet):

1) FR-EVT-006 payload shape (Handshake_Master_Spec_v02.101.md 11.5.2)

- Target file: src/backend/handshake_core/src/flight_recorder/mod.rs

Add a nested token-usage struct and update the llm_inference payload struct to match FR-EVT-006:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInferenceTokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInferenceEvent {
    #[serde(rename = "type")]
    pub event_type: String, // must be "llm_inference"
    pub trace_id: Uuid,     // REQUIRED non-nil
    pub model_id: String,   // REQUIRED non-empty
    pub token_usage: LlmInferenceTokenUsage,
    pub latency_ms: Option<u64>,
    pub prompt_hash: Option<String>,
    pub response_hash: Option<String>,
}
```

Update validation for llm_inference payload:

```rust
fn validate_llm_inference_payload(payload: &Value) -> Result<(), RecorderError>
```

Validation rules (must fail with RecorderError::InvalidEvent):
- Required: payload["type"] == "llm_inference"
- Required: payload["trace_id"] is a UUID string (parseable) and not nil
- Required: payload["model_id"] is a non-empty string
- Required: payload["token_usage"] is an object with numeric fields:
  - prompt_tokens
  - completion_tokens
  - total_tokens
- Optional: payload["latency_ms"], payload["prompt_hash"], payload["response_hash"] may be absent or null

2) Emission mapping in OllamaAdapter (Handshake_Master_Spec_v02.101.md 4.2.3.2 + 11.5.2 + 11.10.3)

- Target file: src/backend/handshake_core/src/llm/ollama.rs

Update the emission helper to construct FR-EVT-006 payload (schema above) and record it:

```rust
async fn emit_llm_inference_event(
    &self,
    req: &CompletionRequest,
    response_text: &str,
    usage: &TokenUsage,
    latency_ms: u64,
)
```

Payload contract for emit_llm_inference_event:
- event_type/type = "llm_inference"
- trace_id = req.trace_id
- model_id = req.model_id
- token_usage.{prompt_tokens, completion_tokens, total_tokens} are derived from the chosen primary token accounting (see below)
- prompt_hash/response_hash/latency_ms remain optional

Token accounting selection contract (Handshake_Master_Spec_v02.101.md 11.10.3):
- Primary usage numbers for real Ollama calls come from provider response counts when present:
  - prompt_tokens := prompt_eval_count
  - completion_tokens := eval_count
  - total_tokens := prompt_eval_count + eval_count
- Tokenization-derived counts may still be computed for diagnostics/comparison only.

Budget enforcement contract (Handshake_Master_Spec_v02.101.md 4.2.3.2):
- If req.max_tokens is Some(max), reject if provider exceeds that budget and return:
  - Err(LlmError::BudgetExceeded(actual_completion_tokens_or_total_tokens))
  - (exact compare basis will be implemented to match spec: provider counts when present)

3) Startup Ollama availability detection (Handshake_Master_Spec_v02.101.md 11.10.3)

- Target file: src/backend/handshake_core/src/main.rs

Make LLM initialization asynchronous and non-fatal:

```rust
async fn init_llm_client(flight_recorder: Arc<dyn FlightRecorder>) -> Arc<dyn LlmClient>
```

Detection contract:
- Base URL defaults to "http://localhost:11434" (or uses configured OLLAMA_URL when present)
- On startup, perform GET {base_url}/api/tags
- Only return an OllamaAdapter when tags request succeeds; otherwise return a disabled client (below)

4) Disabled client type (to satisfy "only enable when available")

- Target file: src/backend/handshake_core/src/llm/mod.rs

Add a disabled LlmClient implementation used when Ollama is unavailable:

```rust
pub struct DisabledLlmClient {
    reason: String,
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for DisabledLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    fn profile(&self) -> &ModelProfile;
}
```

5) In-scope persistence test update (legacy payload -> FR-EVT-006)

- Target file: src/backend/handshake_core/src/flight_recorder/duckdb.rs

Update the existing llm_inference event construction in test_fr_evt_shapes_persisted to use:
- payload.type, payload.trace_id, payload.model_id, payload.token_usage.{...}

Targeted tests required by TEST_PLAN (must be added/updated during implementation):
- llm::ollama::tests::test_llm_inference_payload_matches_fr_evt_006
- flight_recorder::tests::test_llm_inference_payload_validation_requires_trace_id_and_model_id
- flight_recorder::tests::test_llm_inference_payload_validation_requires_token_usage_object

Open questions:
- None (scope decision Option A selected: include src/backend/handshake_core/src/flight_recorder/duckdb.rs in scope).

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- Commands run (outcomes summarized; detailed logs omitted):
  - just pre-work WP-1-LLM-Core-v3: PASS
  - just validator-spec-regression: PASS
  - cargo deny check advisories licenses bans sources: PASS (warnings only)
    - Note: TEST_PLAN lists pwsh wrapper; pwsh was not available in this environment, so cargo deny was run directly.
  - pnpm -C app audit --audit-level high: PASS (4 moderate vulnerabilities reported; none high)
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
  - cargo test ... llm::ollama::tests::test_llm_inference_payload_matches_fr_evt_006: PASS
  - cargo test ... flight_recorder::tests::test_llm_inference_payload_validation_requires_trace_id_and_model_id: PASS
  - cargo test ... flight_recorder::tests::test_llm_inference_payload_validation_requires_token_usage_object: PASS
  - cargo fmt -- --check: PASS
  - cargo clippy ... -D warnings: PASS
  - just validator-scan: PASS
  - just validator-error-codes: PASS
  - just cargo-clean: PASS
  - just post-work WP-1-LLM-Core-v3: PASS

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- **Target File**: `src/backend/handshake_core/src/llm/ollama.rs`
- **Start**: 1
- **End**: 579
- **Line Delta**: 101
- **Pre-SHA1**: `4f0704715123ca099f9e31034834d3975cc10070`
- **Post-SHA1**: `fcc3c315c1902477121e01aeb4f1752d3babd145`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md

- **Target File**: `src/backend/handshake_core/src/llm/mod.rs`
- **Start**: 1
- **End**: 253
- **Line Delta**: 26
- **Pre-SHA1**: `ef0380423879747c8b433f8c39951adc672d911c`
- **Post-SHA1**: `ffc1d15c870e63fdb7e84e3354454a190388cac1`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 657
- **Line Delta**: 147
- **Pre-SHA1**: `333bbfecae2bf08f5d72e145aa3482f866feff9c`
- **Post-SHA1**: `38b36f722d417af9a07d2ed511335ad27be6bc8c`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md

- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 1
- **End**: 1067
- **Line Delta**: 8
- **Pre-SHA1**: `b021ba7144fb87256e641007a62fbe058a415e21`
- **Post-SHA1**: `13edf3c181b0ef79ec2d606eea2fcb4274d3a54b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md

- **Target File**: `src/backend/handshake_core/src/main.rs`
- **Start**: 1
- **End**: 288
- **Line Delta**: 43
- **Pre-SHA1**: `90c67690551e416d5712e650e6abb792d4019647`
- **Post-SHA1**: `99123b6b6817c41bc14e32c7e6fe4072805e8461`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Implementation complete; post-work gate passes.
- What changed in this update:
  - Updated LlmInference event payload + validation to match FR-EVT-006 (type/trace_id/model_id/token_usage; optional latency/hashes).
  - OllamaAdapter now prefers provider token counts when present and enforces max_tokens against completion_tokens; emits FR-EVT-006 payload.
  - Startup now checks {base_url}/api/tags (default localhost) and disables LLM client when unavailable (non-fatal).
  - Updated DuckDB flight recorder test payload to FR-EVT-006 and added targeted tests required by TEST_PLAN.
- Touched files (non-doc):
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/main.rs
- Next step / handoff hint:
  - Validator: run the packet TEST_PLAN and review spec-to-code mapping against Handshake_Master_Spec_v02.101.md anchors 4.2.3, 11.5.2, 11.10.3.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
