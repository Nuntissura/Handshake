# Task Packet: WP-1-Metrics-Mock-Tokens

## METADATA
- TASK_ID: WP-1-Metrics-Mock-Tokens
- WP_ID: WP-1-Metrics-Mock-Tokens
- BASE_WP_ID: WP-1-Metrics-Mock-Tokens (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-18T22:28:17.346Z
- REQUESTOR: ilja
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja180120262320

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Metrics-Mock-Tokens.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement deterministic mock TokenUsage emission for metric propagation testing, per Master Spec v02.113 11.10.3 "Metrics & Tokens" (Token Accounting - mocks).
- Why: If mock LLM completions always emit zero tokens, token accounting and downstream metric propagation cannot be tested; regressions can slip through while core observability surfaces appear "healthy".
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/llm/ollama.rs
- OUT_OF_SCOPE:
  - Any changes to `src/backend/handshake_core/src/capabilities.rs` or `src/backend/handshake_core/src/workflows.rs` (owned by WP-1-Capability-SSoT-v2)
  - Any changes to `src/backend/handshake_core/src/storage/**`, `src/backend/handshake_core/src/models.rs`, or `src/backend/handshake_core/migrations/` (owned by WP-1-Mutation-Traceability-v2)
  - Any changes to `src/backend/handshake_core/src/ace/**` (owned by WP-1-ACE-Runtime-v2)
  - OpenTelemetry instrumentation / exporters (separate WP-1-Metrics-OTel-v2 / WP-1-Metrics-Traces-v2 scope)
  - Production Ollama adapter token accounting (must continue to prefer provider token counts when present)
  - Frontend/UI changes (app/)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Metrics-Mock-Tokens

# Coder (development):
cd src/backend/handshake_core
cargo test

# Full deterministic gates:
cd ../..
just cargo-clean
just post-work WP-1-Metrics-Mock-Tokens
```

### DONE_MEANS
- `just pre-work WP-1-Metrics-Mock-Tokens` passes.
- `just post-work WP-1-Metrics-Mock-Tokens` passes.
- Mock LLM completions emit deterministic TokenUsage per Master Spec v02.113 11.10.3:
  - Default behavior (no explicit override): TokenUsage is computed deterministically using the "10 tokens per word" heuristic.
  - Override behavior: tests can still supply explicit TokenUsage via `InMemoryLlmClient::with_usage(...)`.
- Implementation does not introduce forbidden patterns (validator-scan clean), including no `split_whitespace()` usage in code.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-18T22:28:17.346Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.10.3 (Token Accounting - mocks)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- N/A.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.113.md
  - docs/refinements/WP-1-Metrics-Mock-Tokens.md
  - docs/task_packets/WP-1-Metrics-Mock-Tokens.md
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - scripts/validation/validator-scan.mjs
- SEARCH_TERMS:
  - "InMemoryLlmClient"
  - "TokenUsage"
  - "prompt_eval_count"
  - "eval_count"
  - "emit_llm_inference_event"
  - "HSK-402-BUDGET-EXCEEDED"
  - "token_usage"
  - "split_whitespace"
- RUN_COMMANDS:
  ```bash
  cd src/backend/handshake_core; cargo test
  ```
- RISK_MAP:
  - "mock token usage remains zero" -> "metric propagation untestable; hides regressions"
  - "forbidden pattern introduced (split_whitespace/unwrap/todo/etc)" -> "validator-scan FAIL"
  - "determinism bug in counting logic" -> "flaky tests; inconsistent metrics"
  - "mock logic bleeds into production path" -> "incorrect token accounting in real usage"

## SKELETON
- Proposed interfaces/types/contracts:
  - Update `InMemoryLlmClient` (test-utils) so completion returns deterministic TokenUsage when no explicit override is set:
    - `prompt_tokens = 10 * word_count(prompt)`
    - `completion_tokens = 10 * word_count(response_text)`
    - `total_tokens = prompt_tokens + completion_tokens`
  - `word_count` must be implemented without `split_whitespace()` to satisfy `scripts/validation/validator-scan.mjs`.
- Open questions:
  - NONE (spec provides an explicit example heuristic; this WP implements it deterministically).
- Notes:
  - This WP is intentionally scoped to `llm/ollama.rs` only to avoid merge conflicts with active WPs touching `workflows.rs`, `capabilities.rs`, and the storage/migration surface.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
