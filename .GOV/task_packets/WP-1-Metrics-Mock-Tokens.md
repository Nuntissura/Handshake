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
- **Status:** Done
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja180120262320

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Metrics-Mock-Tokens.md
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
- WAIVER_ID: WP-1-Metrics-Mock-Tokens-VALSCAN-001 | Date: 2026-01-19 | Check Waived: `just validator-scan` (repo-wide baseline findings outside IN_SCOPE_PATHS) + post-work out-of-scope file warning for protocol gate state file | Scope: out-of-scope findings under `src/backend/handshake_core/src/api/**`, `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/governance_pack.rs`; plus `.GOV/validator_gates/WP-1-Metrics-Mock-Tokens.json` (validator gates state) | Justification: baseline findings not introduced by this WP; validator gates state is protocol-required output; Validator still enforced scoped forbidden-pattern audit on `src/backend/handshake_core/src/llm/ollama.rs` | Approver: ilja | Expiry: WP-1-Metrics-Mock-Tokens closure.

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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 11.10.3 (Token Accounting - mocks)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- N/A.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.113.md
  - .GOV/refinements/WP-1-Metrics-Mock-Tokens.md
  - .GOV/task_packets/WP-1-Metrics-Mock-Tokens.md
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/llm/ollama.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - .GOV/scripts/validation/validator-scan.mjs
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
  - `word_count` must be implemented without `split_whitespace()` to satisfy `.GOV/scripts/validation/validator-scan.mjs`.
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
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/llm/ollama.rs`
- **Start**: 410
- **End**: 728
- **Line Delta**: 90
- **Pre-SHA1**: `508c2301c4910acacde99f03b3d148bc609cfeb9`
- **Post-SHA1**: `9013b0396e3621175ac96a6fc1b82e54f0ee4333`
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
- **Lint Results**:
- **Artifacts**:
- **Timestamp**: 2026-01-19T00:56:17.6681175Z
- **Operator**: CodexCLI-GPT-5.2 (Validator)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
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

### VALIDATION REPORT - WP-1-Metrics-Mock-Tokens (2026-01-19)
Verdict: PASS

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Metrics-Mock-Tokens.md` (**Status:** Done)
- Spec Target: `.GOV/roles_shared/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.113.md`
- Spec Anchor: `Handshake_Master_Spec_v02.113.md 11.10.3 (Metrics & Tokens)`
- Worktree/Branch: `D:\Projects\LLM projects\wt-WP-1-Metrics-Mock-Tokens` / `feat/WP-1-Metrics-Mock-Tokens`
- Commit reviewed: N/A (changes staged, not committed yet)

Files Checked:
- `.GOV/roles_shared/SPEC_CURRENT.md`
- `Handshake_Master_Spec_v02.113.md`
- `.GOV/refinements/WP-1-Metrics-Mock-Tokens.md`
- `.GOV/task_packets/WP-1-Metrics-Mock-Tokens.md`
- `src/backend/handshake_core/src/llm/ollama.rs`
- `src/backend/handshake_core/src/main.rs`
- `.GOV/scripts/validation/validator-scan.mjs`

Findings:
- SPEC 11.10.3 Token Accounting:
  - Real Ollama calls prefer provider counts when present (`prompt_eval_count` + `eval_count`): `src/backend/handshake_core/src/llm/ollama.rs:346`
  - Mocks emit deterministic values (10 tokens per word) by default: `src/backend/handshake_core/src/llm/ollama.rs:439` (word_count), `src/backend/handshake_core/src/llm/ollama.rs:458` (deterministic_usage), `src/backend/handshake_core/src/llm/ollama.rs:476` (completion uses override-or-deterministic)
  - Override is respected (including explicit zeros): `src/backend/handshake_core/src/llm/ollama.rs:427` and `src/backend/handshake_core/src/llm/ollama.rs:477`
- SPEC 11.10.3 Ollama Detection (startup /api/tags): implemented at `src/backend/handshake_core/src/main.rs:198`

Forbidden Patterns:
- Scoped audit (changed file): `src/backend/handshake_core/src/llm/ollama.rs` contains no `split_whitespace()`/`unwrap()`/`expect()`/`todo!`/`unimplemented!`/`dbg!`/`println!`/`panic!` introduction in the WP window.
- Repo-wide `just validator-scan`: FAIL due to pre-existing expect()/placeholder findings in unrelated modules (not introduced by this WP diff).

Tests:
- `just pre-work WP-1-Metrics-Mock-Tokens`: PASS
- `cd src/backend/handshake_core; cargo test`: PASS (warnings only)
- `just cargo-clean`: PASS
- `just post-work WP-1-Metrics-Mock-Tokens`: PASS
- `just validator-spec-regression`: PASS
- `just validator-scan`: FAIL (baseline findings outside WP scope)

Coverage note:
- New tests fail if deterministic usage logic is removed:
  - `src/backend/handshake_core/src/llm/ollama.rs:685`
  - `src/backend/handshake_core/src/llm/ollama.rs:705`

REASON FOR PASS:
- The mock LLM client now emits deterministic TokenUsage per `Handshake_Master_Spec_v02.113.md 11.10.3` while preserving provider-reported token counts for real Ollama calls, and the change is covered by targeted unit tests.

SKELETON APPROVED
(Operator-approved: ilja; recorded: 2026-01-19)

### VALIDATION REPORT - WP-1-Metrics-Mock-Tokens (2026-01-19)
Verdict: PASS

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Metrics-Mock-Tokens.md` (**Status:** Done)
- Spec Target: `.GOV/roles_shared/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.113.md`
- Spec Anchor: `Handshake_Master_Spec_v02.113.md 11.10.3 (Metrics & Tokens)`
- Worktree/Branch: `D:\Projects\LLM projects\wt-WP-1-Metrics-Mock-Tokens` / `feat/WP-1-Metrics-Mock-Tokens`

Files Checked:
- `.GOV/roles_shared/SPEC_CURRENT.md`
- `Handshake_Master_Spec_v02.113.md`
- `.GOV/refinements/WP-1-Metrics-Mock-Tokens.md`
- `.GOV/task_packets/WP-1-Metrics-Mock-Tokens.md`
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/validator_gates/WP-1-Metrics-Mock-Tokens.json`
- `src/backend/handshake_core/src/llm/ollama.rs`
- `src/backend/handshake_core/src/main.rs`
- `.GOV/scripts/validation/gate-check.mjs`
- `.GOV/scripts/validation/pre-work-check.mjs`
- `.GOV/scripts/validation/post-work-check.mjs`
- `.GOV/scripts/validation/validator-scan.mjs`

Findings:
- SPEC 11.10.3 Token Accounting:
  - Real Ollama calls prefer provider counts when present (`prompt_eval_count` + `eval_count`): `src/backend/handshake_core/src/llm/ollama.rs:347`
  - Mocks emit deterministic values (10 tokens per word) by default: `src/backend/handshake_core/src/llm/ollama.rs:439` (word_count), `src/backend/handshake_core/src/llm/ollama.rs:458` (deterministic_usage), `src/backend/handshake_core/src/llm/ollama.rs:477` (completion uses override-or-deterministic)
  - Override is respected (including explicit zeros): `src/backend/handshake_core/src/llm/ollama.rs:427` and `src/backend/handshake_core/src/llm/ollama.rs:477`
- SPEC 11.10.3 Ollama Detection (startup /api/tags): implemented at `src/backend/handshake_core/src/main.rs:198`

Hygiene / Forbidden Patterns:
- In-scope forbidden-pattern grep clean (no `split_whitespace()`/`unwrap()`/`expect()`/`todo!`/`unimplemented!`/`dbg!`/`println!`/`panic!`) in `src/backend/handshake_core/src/llm/ollama.rs`.
- Repo-wide `just validator-scan`: FAIL due to baseline findings in unrelated modules; waived under `WP-1-Metrics-Mock-Tokens-VALSCAN-001`.

Waivers:
- `WP-1-Metrics-Mock-Tokens-VALSCAN-001` (see `## WAIVERS GRANTED`): repo-wide `just validator-scan` baseline findings + protocol gate-state file `.GOV/validator_gates/WP-1-Metrics-Mock-Tokens.json`.

Tests / Commands:
- `just gate-check WP-1-Metrics-Mock-Tokens`: PASS
- `just pre-work WP-1-Metrics-Mock-Tokens`: PASS
- `just cargo-clean`: PASS
- `cd src/backend/handshake_core; cargo test`: PASS (warnings only)
- `just post-work WP-1-Metrics-Mock-Tokens`: PASS (warnings only; out-of-scope gate state file, waived)
- `just validator-spec-regression`: PASS
- `just validator-scan`: FAIL (waived; baseline outside WP scope)

Coverage note:
- Removal-check coverage exists for the deterministic usage behavior:
  - `src/backend/handshake_core/src/llm/ollama.rs:685`
  - `src/backend/handshake_core/src/llm/ollama.rs:706`

REASON FOR PASS:
- The implementation satisfies `Handshake_Master_Spec_v02.113.md` Â§11.10.3 by emitting deterministic mock TokenUsage while preserving provider-reported token counts for real Ollama calls, and the behavior is covered by targeted tests.


