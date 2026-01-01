# Task Packet: WP-1-Tokenization-Service-v3

## METADATA
- TASK_ID: WP-1-Tokenization-Service-v3
- WP_ID: WP-1-Tokenization-Service-v3
- DATE: 2026-01-01T02:00:00.000Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja010120260219

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Tokenization-Service-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate Tokenization Service to match SPEC_CURRENT v02.100 (TokenizationService contract, model-specific tokenizers, and fallback observability).
- Why: Token counts must be accurate for budget enforcement; current implementation is misaligned (async Tokenizer shape, missing SentencePiece/Ollama tokenizer-config fetch, and missing Flight Recorder metric on fallback).
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/tokenization.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/tests/tokenization_service_tests.rs
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps (spec is already clearly covered; see refinement).
  - Any UI/frontend work.
  - Any storage/migration changes.
  - Any non-Ollama remote provider support beyond what SPEC_CURRENT requires for tokenization rules.

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Tokenization-Service-v3

# Spec integrity (must remain PASS):
just validator-spec-regression

# Backend build + tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Lint/format:
cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings

# Protocol/hygiene helpers:
just validator-scan
just validator-error-codes

# External Cargo target hygiene:
just cargo-clean
just post-work WP-1-Tokenization-Service-v3
```

### DONE_MEANS
- `TokenizationService` trait exists and matches SPEC_CURRENT v02.100 4.6 (sync `count_tokens` + `truncate` signatures; no async surface).
- Model-specific behavior matches SPEC_CURRENT v02.100 4.6 + 4.6 Tokenization and Metrics Contract:
  - GPT-class uses tiktoken (or compatible BPE).
  - Llama/Mistral (Ollama) resolves tokenizer via local runtime config (e.g., `/api/show`) and uses SentencePiece/Tiktoken as required.
  - No whitespace-splitting approximation in production.
- Vibe Tokenizer fallback implements char_count / 4.0 heuristic and emits Flight Recorder `metric.accuracy_warning` whenever it is used.
- **Sync/Async Bridge:** Telemetry emission (accuracy warnings) uses a non-blocking fire-and-forget mechanism (e.g., `tokio::spawn` or channel) to preserve the sync `count_tokens` signature without blocking the hot path (Section 4.6 / 11.5).
- Consistency invariant: token counts emitted to JobMetrics match the counts used for budgeting/truncation (single source: TokenizationService).
- Targeted tests exist in `src/backend/handshake_core/tests/tokenization_service_tests.rs` and fail if fallback/selection logic is removed.
- All TEST_PLAN commands pass and `just post-work WP-1-Tokenization-Service-v3` returns PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-01T02:00:00.000Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 4.6 (Tokenization Service) and 4.6 Tokenization and Metrics Contract (normative)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - docs/CODER_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - Handshake_Master_Spec_v02.99.md
  - docs/refinements/WP-1-Tokenization-Service-v3.md
  - src/backend/handshake_core/src/tokenization.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/tests/tokenization_service_tests.rs
- SEARCH_TERMS:
  - "TokenizationService"
  - "count_tokens("
  - "truncate("
  - "Tokenizer"
  - "SentencePiece"
  - "tiktoken"
  - "/api/show"
  - "metric.accuracy_warning"
  - "FlightRecorder"
  - "JobMetrics"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Tokenization-Service-v3
  just validator-spec-regression
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "Wrong tokenizer per model" -> "budget enforcement + truncation correctness"
  - "Fallback used silently" -> "auditability gap (missing metric.accuracy_warning)"
  - "Ollama config fetch fails" -> "tokenization miscount for llama/mistral models"

## SKELETON
SKELETON APPROVED
- Proposed interfaces/types/contracts:
  - TokenizationService (sync, per spec):
    - fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError>
    - fn truncate(&self, text: &str, limit: u32, model: &str) -> String
  - TokenizerEngine (sync, internal):
    - fn count_tokens(&self, text: &str) -> Result<u32, TokenizerError>
    - fn truncate(&self, text: &str, limit: u32) -> Result<String, TokenizerError>
  - TokenizationOutcome:
    - count: u32
    - tokenizer_kind: TokenizerKind
    - used_fallback: bool
    - fallback_reason: Option<AccuracyWarningReason>
  - TruncationOutcome:
    - text: String
    - tokenizer_kind: TokenizerKind
    - used_fallback: bool
    - fallback_reason: Option<AccuracyWarningReason>
  - TokenizationRouter (implements TokenizationService):
    - count_tokens_internal(&self, text: &str, model: &str) -> Result<TokenizationOutcome, TokenizerError>
    - truncate_internal(&self, text: &str, limit: u32, model: &str) -> TruncationOutcome
  - TokenizerKind: Tiktoken | SentencePiece | Vibe
  - AccuracyWarningReason:
    - UnknownModel
    - ConfigFetchFailed
    - ConfigMissing
    - UnsupportedTokenizer
    - TokenizerInitFailed
  - AccuracyWarningEmitter (sync adapter boundary):
    - fn emit_accuracy_warning(&self, trace_id: Uuid, model: &str, reason: AccuracyWarningReason, tokenizer: TokenizerKind)
  - AsyncFlightRecorderEmitter (bridge sync -> async):
    - holds tokio::sync::mpsc::UnboundedSender<FlightRecorderEvent>
    - spawns a background task only if tokio::runtime::Handle::try_current() succeeds
    - emit_accuracy_warning() uses send; if emitter unavailable, record a failure signal (log error)
  - TokenizationWithTrace (higher-layer wrapper):
    - count_tokens_with_trace(&self, text: &str, model: &str, trace_id: Uuid) -> Result<u32, TokenizerError>
    - truncate_with_trace(&self, text: &str, limit: u32, model: &str, trace_id: Uuid) -> String
    - uses TokenizationRouter::count_tokens_internal/truncate_internal to detect fallback and call AccuracyWarningEmitter with trace_id
  - OllamaTokenizerConfigClient (async, llm/ollama.rs):
    - fetch_show(&self, model: &str) -> Result<serde_json::Value, TokenizerError>
  - OllamaTokenizerConfigResolver (sync, tokenization.rs):
    - resolve(&self, model: &str) -> Result<ResolvedTokenizerConfig, TokenizerError>
  - ResolvedTokenizerConfig:
    - kind: TokenizerKind
    - data: TokenizerConfigData
  - TokenizerConfigData:
    - Tiktoken { encoding: String }
    - SentencePiece { model_path: String }
- Routing strategy:
  - GPT-class (gpt- / o1- / o3- prefix) -> TiktokenTokenizer
  - Ollama model -> resolver.resolve(model):
    - SentencePiece -> SentencePieceTokenizer
    - Tiktoken -> TiktokenTokenizer (use encoding from config)
    - Missing/unsupported -> VibeTokenizer (fallback)
  - Unknown model -> VibeTokenizer (fallback)
- Sync tokenization -> async Flight Recorder (non-blocking, no panics, no locks across network):
  - TokenizationService stays sync; it does not call FlightRecorder directly (no trace_id).
  - TokenizationWithTrace is used in higher layers that have trace_id (CompletionRequest.trace_id).
  - OllamaAdapter token budgeting/token counting uses TokenizationWithTrace with CompletionRequest.trace_id.
  - TokenizationWithTrace calls AccuracyWarningEmitter when fallback is used.
  - AsyncFlightRecorderEmitter starts a background worker only if try_current() succeeds; otherwise it is unavailable and emits a failure signal.
  - Worker drains channel and calls flight_recorder.record_event(event). No locks held during record_event.
- Trace correlation:
  - Trace source is CompletionRequest.trace_id (LLM call) or job.trace_id in workflow code.
  - If trace_id is not available at tokenization call site, emission responsibility is moved up to a layer that has it (TokenizationWithTrace).
- Ollama /api/show schema boundary plan (shape-tolerant):
  - Parse /api/show response as serde_json::Value at the edge.
  - map_ollama_show_to_tokenizer_config(value):
    - allowlisted kind keys (first match wins): tokenizer.kind, tokenizer.type, tokenizer.name, tokenizer_config.kind, tokenizer_config.type
    - allowlisted kinds: "sentencepiece", "tiktoken" (case-insensitive)
    - SentencePiece config keys: tokenizer.model_path OR tokenizer_config.model_path
    - Tiktoken config keys: tokenizer.encoding OR tokenizer_config.encoding
    - If any required key is missing or kind not allowlisted -> TokenizerConfigMissing -> Vibe fallback + accuracy_warning
- Cache safety:
  - Resolver uses Mutex<HashMap<String, ResolvedTokenizerConfig>>
  - Resolve flow: lock -> check -> clone -> unlock; if miss, fetch /api/show with NO lock held; then lock and insert
  - No lock held during network or blocking work; no deadlock path
  - Open questions:
  - Verify which model prefixes define GPT-class in this codebase (currently only gpt-).
- Notes:
  - TokenizationService contract remains sync and unchanged per spec (no async surface).
  - Accuracy warning emission requires trace_id; do not emit from plain TokenizationService methods.

## IMPLEMENTATION
- Reworked tokenization surface to a sync TokenizationService with routing outcomes, SentencePiece cache, and tiktoken engine selection plus Vibe fallback reasons in `src/backend/handshake_core/src/tokenization.rs`.
- Added async accuracy-warning bridge via AsyncFlightRecorderEmitter and TokenizationWithTrace to emit `metric.accuracy_warning` without blocking.
- Added Ollama /api/show config fetch + cache refresh and TokenizationWithTrace usage for budgeting/usage in `src/backend/handshake_core/src/llm/ollama.rs`.
- Added tests covering config mapping and fallback warning emission in `src/backend/handshake_core/tests/tokenization_service_tests.rs`.

## HYGIENE
- just validator-scan: PASS
- just validator-dal-audit: PASS
- just validator-git-hygiene: PASS

## VALIDATION
- Target File: `src/backend/handshake_core/src/tokenization.rs`
- Start: 1
- End: 996
- Line Delta: 753
- Pre-SHA1: `fe8ebe47759d736e8883a11d0033cede74f03eac`
- Post-SHA1: `dd9ff15a31a4a8cfbaefd81f8653d5289e7b6b3d`
- Gates Passed:
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
- Lint Results: cargo fmt --check PASS; cargo clippy --all-targets --all-features -- -D warnings PASS
- Artifacts: None
- Timestamp: 2026-01-01
- Operator: codex-cli (Coder)
- Spec Target Resolved: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- Notes: Window covers full file; line_delta from git diff --numstat HEAD for target file.

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; validation commands + post-work PASS.
- What changed in this update: Sync TokenizationService routing + accuracy warning bridge added; Ollama adapter now uses TokenizationWithTrace and config refresh; tokenization service tests added.
- Next step / handoff hint: Validator review and TASK_BOARD update to Done if accepted.

## VALIDATION_REPORTS

VALIDATION REPORT - WP-1-Tokenization-Service-v3
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Tokenization-Service-v3.md (Status: In Progress)
- Refinement: docs/refinements/WP-1-Tokenization-Service-v3.md (USER_REVIEW_STATUS: APPROVED, USER_SIGNATURE: ilja010120260219)
- Spec Target Resolved: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md

Commands Run (Validator):
- just cargo-clean: PASS
- just pre-work WP-1-Tokenization-Service-v3: PASS
- just validator-spec-regression: PASS
- just validator-scan: PASS
- just validator-dal-audit: PASS
- just validator-error-codes: PASS
- just validator-coverage-gaps: PASS
- just validator-traceability: PASS
- just validator-git-hygiene: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check: PASS
- cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings: PASS
- just post-work WP-1-Tokenization-Service-v3: PASS
Notes: cargo commands require the external Cargo target dir at ../Cargo Target/handshake-cargo-target.

Evidence Mapping (Spec -> Code):
- TokenizationService sync trait: src/backend/handshake_core/src/tokenization.rs:40
- TokenizationRouter implements TokenizationService: src/backend/handshake_core/src/tokenization.rs:688
- GPT-class routing uses tiktoken: src/backend/handshake_core/src/tokenization.rs:523 and src/backend/handshake_core/src/tokenization.rs:976
- Ollama /api/show config fetch: src/backend/handshake_core/src/llm/ollama.rs:151
- Ollama config refresh: src/backend/handshake_core/src/llm/ollama.rs:286
- Config mapping allowlist: src/backend/handshake_core/src/tokenization.rs:895
- SentencePiece tokenizer + cache: src/backend/handshake_core/src/tokenization.rs:231 and src/backend/handshake_core/src/tokenization.rs:301
- Vibe fallback heuristic (char_count / 4.0): src/backend/handshake_core/src/tokenization.rs:981
- Non-blocking accuracy warning bridge: src/backend/handshake_core/src/tokenization.rs:731
- Accuracy warning triggered on fallback with trace_id: src/backend/handshake_core/src/tokenization.rs:842
- Budget/usage counts from TokenizationWithTrace: src/backend/handshake_core/src/llm/ollama.rs:295

Targeted Tests:
- src/backend/handshake_core/tests/tokenization_service_tests.rs:69
- src/backend/handshake_core/tests/tokenization_service_tests.rs:108
- src/backend/handshake_core/tests/tokenization_service_tests.rs:130

Findings:
- Implementation appears to satisfy the WP DONE_MEANS and SPEC_CURRENT v02.100 TokenizationService + Tokenization and Metrics Contract requirements (model-specific tokenizers, /api/show config, Vibe fallback heuristic, and non-blocking metric.accuracy_warning emission).

REASON FOR FAIL:
1) OUT-OF-SCOPE / LAW EDITS PRESENT IN DIFF (CX-105 HARD_NO_LAW_EDIT):
   - Current git diff includes changes to governance/Law docs not listed in this WP scope (e.g., Handshake Codex v1.4.md, docs/CODER_PROTOCOL.md, docs/ORCHESTRATOR_PROTOCOL.md, docs/TASK_PACKET_TEMPLATE.md). No explicit user request for LAW edits was provided for this WP.
2) SPEC ENRICHMENT GOVERNANCE INCONSISTENT (Spec Enrichment + Signature Gate):
   - docs/SPEC_CURRENT.md points to Handshake_Master_Spec_v02.100.md with note [ilja01012026], which does not match the required signature format {username}{DDMMYYYYHHMM} and is not recorded as a spec-enrichment signature in docs/SIGNATURE_AUDIT.md.
   - Handshake_Master_Spec_v02.100.md CHANGELOG table contains no v02.100 row (only v02.99 and earlier), so spec version metadata is inconsistent.
3) TASK PACKET INCOMPLETE PER CODEX (CX-504):
   - This packet does not include the required "User Context" non-technical explainer section.

Git State At Validation Time:
- Modified tracked files:
  - Handshake Codex v1.4.md
  - docs/CODER_PROTOCOL.md
  - docs/ORCHESTRATOR_GATES.json
  - docs/ORCHESTRATOR_PROTOCOL.md
  - docs/SIGNATURE_AUDIT.md
  - docs/SPEC_CURRENT.md
  - docs/TASK_BOARD.md
  - docs/TASK_PACKET_TEMPLATE.md
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/tokenization.rs
- Untracked files:
  - .codex_tmp_file
  - Handshake_Master_Spec_v02.100.md
  - docs/refinements/WP-1-Tokenization-Service-v3.md
  - docs/task_packets/WP-1-Tokenization-Service-v3.md
  - src/backend/handshake_core/tests/tokenization_service_tests.rs
  - validation audit.md

Required Remediation (to unblock):
- Decide commit scope: either revert unrequested LAW/protocol edits or split them into a separate governance WP with explicit user approval.
- Provide a valid, unused user signature for spec enrichment v02.100 (or revert SPEC_CURRENT back to v02.99 and create a new WP anchored to v02.99), and record it in docs/SIGNATURE_AUDIT.md. Add a v02.100 row to the Handshake_Master_Spec_v02.100.md changelog.
- Add the required User Context section to this packet (append-only), then re-run just pre-work and just post-work.

Operator: Validator (codex-cli)
Timestamp: 2026-01-01

## USER_CONTEXT
- This work ensures the app can accurately count and limit AI prompt/response sizes for different model types, so budgets and truncation behave predictably.
- When exact token counting is not available, it uses a clearly-audited fallback and records an accuracy warning for later inspection.

VALIDATION REPORT - WP-1-Tokenization-Service-v3 (Revalidation)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Tokenization-Service-v3.md (Status: Done)
- Spec Target Resolved: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- Spec Enrichment Approval: ilja010120260602 (recorded in docs/SIGNATURE_AUDIT.md; changelog row added to Handshake_Master_Spec_v02.100.md)

Commands Run (Validator, delta since prior FAIL):
- just validator-spec-regression: PASS
- just pre-work WP-1-Tokenization-Service-v3: PASS
- just post-work WP-1-Tokenization-Service-v3: PASS (warnings: packet is new/untracked so HEAD concurrency check cannot load; git status read warning)

REASON FOR PASS:
1) Implementation + tests/hygiene already passed earlier and no code changes were made since that run (see prior FAIL report for the full command list and evidence mapping).
2) Prior blockers are resolved:
   - SPEC_CURRENT update is now covered by a valid user signature and audit log entry (ilja010120260602).
   - Handshake_Master_Spec_v02.100.md changelog now includes v02.100.
   - Refinement gate SHA1 updated to match current spec file.
   - Required USER_CONTEXT section is present.
3) User explicitly approved keeping the governance/Law/protocol doc changes in this change set (do not revert).
