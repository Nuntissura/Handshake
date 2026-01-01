# Task Packet: WP-1-Validator-Error-Codes-v1 (Remediation)

## Metadata
- **TASK_ID**: WP-1-Validator-Error-Codes-v1
- **DATE**: 2025-12-29
- **REQUESTOR**: User
- **AGENT_ID**: validator-gpt
- **ROLE**: Validator (acting as Coder for remediation)
- **Status:** Done
- **USER_SIGNATURE**: `ilja2912202519`
- **BLOCKS**: WP-1-Debug-Bundle-v3 (previously blocked closure while `just validator-error-codes` failed; resolved by this WP)

---

## Goal

### SCOPE
Make `just validator-error-codes` pass by removing stringly errors in production paths and by enforcing deterministic policy for time sources via explicit waivers (per Validator Protocol).

### In-scope paths
- Backend (Rust):
  - `src/backend/handshake_core/src/llm/mod.rs`
  - `src/backend/handshake_core/src/llm/ollama.rs`
  - `src/backend/handshake_core/src/main.rs`
  - `src/backend/handshake_core/src/terminal/mod.rs`
- Governance / validators:
  - `scripts/validation/validator-error-codes.mjs`
  - `docs/TASK_BOARD.md`
  - `docs/task_packets/WP-1-Debug-Bundle-v3.md` (append-only note: dependency)

### Out of scope
- Broad error taxonomy refactors beyond the concrete findings from `validator-error-codes`.
- Changing public API shapes unrelated to LLM/startup/terminal error handling.
- Rewriting other validator scripts.

---

## Quality Gate

### RISK_TIER: MEDIUM
Justification: Core error and validator changes; low blast radius but affects repo-wide gates.

### DONE_MEANS (binary; no partials)
- `just validator-error-codes` passes (no stringly errors and no nondeterminism patterns without an explicit waiver marker).
- LLM paths do not use `Result<_, String>` in production (`LlmError` is the error type per spec 4.2.3.1).
- Startup does not convert typed errors into ad-hoc strings (`map_err(|e| format!(...))` removed from production).
- `Instant::now()` uses in production have an explicit waiver marker and are limited to latency/duration measurement (not persisted determinism-sensitive state).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes.
- Task Board updated to track this remediation WP.

### TEST_PLAN
```bash
# Backend
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Governance / workflow
just validator-scan
just validator-error-codes
just post-work WP-1-Validator-Error-Codes-v1
```

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

---

## Authority
- **SPEC_CURRENT**: `docs/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.98.md`
- **SPEC_ANCHOR**:
  - 4.2.3 (HSK-TRAIT-004: LlmClient + LlmError)
- **Codex**: `Handshake Codex v1.4.md`
- **Validator Protocol**: `docs/VALIDATOR_PROTOCOL.md` (typed errors + determinism enforcement)
- **Task Board**: `docs/TASK_BOARD.md`

---

## BOOTSTRAP
- **FILES_TO_OPEN**:
  - `docs/VALIDATOR_PROTOCOL.md`
  - `docs/TASK_PACKET_TEMPLATE.md`
  - `docs/TASK_BOARD.md`
  - `docs/SPEC_CURRENT.md`
  - `Handshake_Master_Spec_v02.98.md`
  - `scripts/validation/validator-error-codes.mjs`
  - `src/backend/handshake_core/src/llm/mod.rs`
  - `src/backend/handshake_core/src/main.rs`
  - `src/backend/handshake_core/src/llm/ollama.rs`
  - `src/backend/handshake_core/src/terminal/mod.rs`
- **SEARCH_TERMS**:
  - "validator-error-codes"
  - "Err(format!"
  - "map_err(|e| format!"
  - "Instant::now("
  - "SystemTime::now("
  - "Result<String, String>"
  - "LLMClient"
  - "OllamaClient"
  - "WAIVER [CX-573E]"
  - "HSK-TRAIT-004"
- **RUN_COMMANDS**: See TEST_PLAN.
- **RISK_MAP**:
  - "LLM legacy layer removal" -> compile/test to confirm no external callers in repo
  - "Validator false negatives" -> waiver marker must be adjacent to each nondeterminism use (no file-wide waivers)
  - "Startup error regression" -> keep typed propagation; no string conversions

---

## SKELETON (Proposed)

### Backend (Rust)
- `src/backend/handshake_core/src/llm/mod.rs`
  - Remove the unused deprecated chat-based `LLMClient`/`OllamaClient` layer (or migrate it to typed `LlmError`); eliminate `Err(format!(...))` in production.
- `src/backend/handshake_core/src/main.rs`
  - Remove `map_err(|e| format!(...))` string conversion on `storage::init_storage()`; propagate typed error (`?`) so startup remains typed.
- `src/backend/handshake_core/src/llm/ollama.rs`
  - Add a local waiver marker adjacent to `Instant::now()` used for `latency_ms` measurement.
- `src/backend/handshake_core/src/terminal/mod.rs`
  - Add a local waiver marker adjacent to `Instant::now()` used for `duration_ms` measurement.

### Validator: `validator-error-codes`
- `scripts/validation/validator-error-codes.mjs`
  - Implement waiver-aware nondeterminism detection: `Instant::now()`/`SystemTime::now()` are only allowed when the same line or immediately preceding line contains `WAIVER [CX-573E]`.
  - (Optional but preferred) Replace the current `rg`/child_process approach with a pure Node file scanner so the validator can run in sandboxed environments without spawning subprocesses.

---

## Skeleton Approval (Blocking)

USER_SIGNATURE: `ilja2912202519`

SKELETON APPROVED ilja2912202519

---

## Validation (to be completed post-implementation; append-only)

- Target File: `scripts/validation/validator-error-codes.mjs`
- Start: 9
- End: 207
- Line Delta: 135
- Pre-SHA1: `b52d97134659f82298e01d9aa5cfd7c556fa508b`
- Post-SHA1: `c9ccf5d8fd040fe275863593a94a83954286f779`
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
  - [x] compilation_clean
  - [x] tests_passed
  - [x] lint_passed
  - [x] task_board_updated
  - [ ] commit_ready
- Lint Results: `just validator-scan` PASS; `just validator-error-codes` PASS
- Artifacts: None
- Timestamp: 2025-12-30T01:44:53.6185597+01:00
- Operator: validator-gpt

## Validation Report (Append Only)

(APPEND-ONLY once validation starts.)

---

## Hygiene Log (Append Only)

(APPEND-ONLY once implementation starts.)

- 2025-12-29: `node scripts/validation/validator-error-codes.mjs` ran (FAIL). Findings: `ace/validators/cache.rs:27,31,36`; `api/bundles.rs:110,117`; `main.rs:173`.
- 2025-12-30: IMPLEMENTATION: Removed `Err("...")` stringly errors in `src/backend/handshake_core/src/ace/validators/cache.rs` and `src/backend/handshake_core/src/api/bundles.rs`; removed `map_err(|_| "...")` in `src/backend/handshake_core/src/main.rs` by returning a typed `std::io::Error` for missing `OLLAMA_URL`.
- 2025-12-30: `just validator-error-codes` ran (PASS).
- 2025-12-30: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` ran (PASS).
- 2025-12-30: `just validator-scan` ran (PASS).
- 2025-12-30: `just cargo-clean` ran (PASS; removed 1148 files, 5.4GiB total from external target dir).

---

VALIDATION REPORT - WP-1-Validator-Error-Codes-v1
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Validator-Error-Codes-v1.md (status: Done)
- Spec: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (anchor: 4.2.3 / HSK-TRAIT-004)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: docs/VALIDATOR_PROTOCOL.md

Files Checked:
- docs/SPEC_CURRENT.md
- docs/TASK_BOARD.md
- docs/VALIDATOR_PROTOCOL.md
- docs/task_packets/WP-1-Validator-Error-Codes-v1.md
- scripts/validation/cor701-spec.json
- scripts/validation/post-work-check.mjs
- scripts/validation/validator-error-codes.mjs
- src/backend/handshake_core/src/ace/validators/cache.rs
- src/backend/handshake_core/src/api/bundles.rs
- src/backend/handshake_core/src/llm/mod.rs
- src/backend/handshake_core/src/llm/ollama.rs
- src/backend/handshake_core/src/main.rs
- src/backend/handshake_core/src/terminal/mod.rs

Findings:
- DONE_MEANS: `just validator-error-codes` passes (evidence: docs/task_packets/WP-1-Validator-Error-Codes-v1.md:178).
- LLM errors are typed (`LlmError`), no `Result<_, String>` production contract (evidence: src/backend/handshake_core/src/llm/mod.rs:154).
- Startup error propagation is typed: storage init uses `?` (evidence: src/backend/handshake_core/src/main.rs:42).
- Missing `OLLAMA_URL` returns a typed error (no `map_err(|_| \"...\")`) (evidence: src/backend/handshake_core/src/main.rs:172).
- Determinism waiver markers are adjacent to `Instant::now()` uses (observability only) (evidence: src/backend/handshake_core/src/llm/ollama.rs:146, src/backend/handshake_core/src/terminal/mod.rs:202).
- Removed stringly errors flagged by validator: cache guard uses `AceError` (evidence: src/backend/handshake_core/src/ace/validators/cache.rs:24); export scope parsing uses `BundleExportError` (evidence: src/backend/handshake_core/src/api/bundles.rs:86).
- Validator is sandbox-compatible (no child_process usage) and waiver-aware (evidence: scripts/validation/validator-error-codes.mjs:13, scripts/validation/validator-error-codes.mjs:130).

Tests:
- `just validator-error-codes`: PASS (docs/task_packets/WP-1-Validator-Error-Codes-v1.md:178)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`: PASS (docs/task_packets/WP-1-Validator-Error-Codes-v1.md:179)

---

REVALIDATION REPORT - WP-1-Validator-Error-Codes-v1
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Validator-Error-Codes-v1.md (Status: Done)
- Spec: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (anchor: 4.2.3 / HSK-TRAIT-004)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: docs/VALIDATOR_PROTOCOL.md

Validation Commands Run:
- just cargo-clean: PASS
- just validator-spec-regression: PASS
- just validator-scan: PASS
- just validator-error-codes: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- just post-work WP-1-Validator-Error-Codes-v1: PASS

Evidence (Spec/DONE_MEANS -> Code):
- Waiver-aware validator: scripts/validation/validator-error-codes.mjs:13 and scripts/validation/validator-error-codes.mjs:130
- Typed LLM errors (HSK-429/402/500): src/backend/handshake_core/src/llm/mod.rs:154
- Startup storage init keeps typed propagation: src/backend/handshake_core/src/main.rs:42
- Missing OLLAMA_URL returns typed error (not Result<_, String>): src/backend/handshake_core/src/main.rs:172
- Instant::now() waiver markers (observability only): src/backend/handshake_core/src/llm/ollama.rs:146 and src/backend/handshake_core/src/terminal/mod.rs:202

REASON FOR PASS:
- Required gates and tests pass, and inspected code satisfies the packet DONE_MEANS for typed errors and deterministic time-source waivers.

Timestamp: 2025-12-30T20:17:39.6328260+01:00
Validator: codex-cli (Validator role)
- `just validator-scan`: PASS (docs/task_packets/WP-1-Validator-Error-Codes-v1.md:180)
- `just cargo-clean`: PASS (docs/task_packets/WP-1-Validator-Error-Codes-v1.md:181)
- `just post-work WP-1-Validator-Error-Codes-v1`: PASS (required; ran with escalated permissions because Node child_process spawnSync is blocked in sandbox with EPERM)

Risks & Suggested Actions:
- Git warns about LF->CRLF normalization for `scripts/validation/validator-error-codes.mjs`; align `.gitattributes` or `core.autocrlf` if this becomes noisy.
- `scripts/validation/post-work-check.mjs` requires subprocess spawning; sandboxed runs need escalation (or a future refactor to avoid child_process).

REASON FOR PASS:
- Deterministic manifest is filled, required gates are checked, `just post-work WP-1-Validator-Error-Codes-v1` passes, and DONE_MEANS checks are satisfied.


