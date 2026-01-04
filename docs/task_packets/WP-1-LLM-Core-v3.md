# Task Packet: WP-1-LLM-Core-v3

## METADATA
- TASK_ID: WP-1-LLM-Core-v3
- WP_ID: WP-1-LLM-Core-v3
- DATE: 2026-01-04T01:42:13.679Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja040120260217

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
- Proposed interfaces/types/contracts:
  - Update llm_inference payload mapping to match FR-EVT-006:
    - payload.type = "llm_inference"
    - payload.trace_id = req.trace_id
    - payload.model_id = req.model_id
    - payload.token_usage = (prompt_tokens, completion_tokens, total_tokens)
    - payload.latency_ms/prompt_hash/response_hash remain optional
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- **Target File**: `src/backend/handshake_core/src/llm/ollama.rs`
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.101.md
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
