# Task Packet: WP-1-Tokenization-Service-v2

## Metadata
- TASK_ID: WP-1-Tokenization-Service-v2
- DATE: 2025-12-28
- REQUESTOR: Orchestrator
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: Done

## Scope
- **What**: Implement the normative `Tokenizer` trait and a `TiktokenAdapter` to decouple the system from specific tokenization libraries per Master Spec v02.95 §4.6.1.
- **Why**: To provide accurate cost estimation and context window management for LLM requests (Phase 1 Closure requirement). The system must handle unknown models gracefully without panicking.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/tokenization/
  * src/backend/handshake_core/src/llm/
- **OUT_OF_SCOPE**:
  * Database changes (Storage track)
  * AppState refactoring (Storage track)
  * Frontend UI

## Quality Gate
- **RISK_TIER**: MEDIUM
  - Justification: New trait implementation; critical for cost tracking but low risk of system-wide regression if isolated.
- **TEST_PLAN**:
  ```bash
  # 1. Compile
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml

  # 2. Run tokenization tests
  # Create a new test file: tests/tokenization_tests.rs or add to existing
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml tokenization

  # 3. Verify Fallback Behavior (Panic Safety)
  # Ensure count_tokens("test", "unknown-model") returns Result, not panic
  
  # 4. External Cargo target hygiene
  just cargo-clean

  # 5. Post-work validation
  just post-work WP-1-Tokenization-Service-20251228
  ```
- **DONE_MEANS**:
  * ✅ `Tokenizer` trait defined exactly as per Spec §4.6.1 (async methods: `count_tokens`, `truncate`).
  * ✅ `TiktokenTokenizer` struct implements this trait using `tiktoken-rs`.
  * ✅ Fallback logic implemented: `count_tokens` warns and uses `cl100k_base` for unknown models (NO PANIC).
  * ✅ `TokenBudget` struct (if not present) or logic uses the trait, not direct library calls.
  * ✅ Unit tests verify count accuracy for "gpt-4o" and "cl100k_base".
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-hash>
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md (v02.95)
  * src/backend/handshake_core/src/tokenization/mod.rs (Trait definition)
  * src/backend/handshake_core/src/tokenization/tiktoken.rs (Implementation)
- **SEARCH_TERMS**:
  * "pub trait Tokenizer"
  * "tiktoken_rs::get_bpe_from_model"
  * "cl100k_base"
  * "async fn count_tokens"
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Panic on unknown model" -> Runtime instability [§4.6.1 constraint]
  * "Async trait overhead" -> Performance (use `async_trait` macro or future-proofing)

## Authority
- **SPEC_ANCHOR**: §4.6.1 [CX-LLM-005]
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.96.md [ilja281220250525]
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Dependencies**: None. Can run parallel to Storage Foundation.
- **Waiver**: If `tiktoken-rs` has specific async limitations, wrap in `spawn_blocking` if needed, but keep trait signature async.

---

## VALIDATION
- **Deterministic Manifest (current workflow)**:
  - **Target File**: `<fill before post-work>`
  - **Start**: 1
  - **End**: 1
  - **Line Delta**: 0
  - **Pre-SHA1**: `0000000000000000000000000000000000000000`
  - **Post-SHA1**: `0000000000000000000000000000000000000000`
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
    - [ ] compilation_clean
    - [ ] tests_passed
    - [ ] outside_window_pristine
    - [ ] lint_passed
    - [ ] ai_review (if required)
    - [ ] task_board_updated
    - [ ] commit_ready
  - **Lint Results**: <suite + pass/fail summary>
  - **Artifacts**: <paths if any>
  - **Timestamp**:
  - **Operator**:
  - **Notes**:
- **Validation Commands / Results**:
  - just post-work WP-1-Tokenization-Service-20251228

---

## REVALIDATION NOTE 2025-12-28
- STATUS: In-Progress (revalidation required after code refactor to align with v02.96 A 4.6.1).
- ACTION: Rerun TEST_PLAN and validator scans; update EVIDENCE_MAPPING once validated.

## VALIDATION REPORT — 2025-12-28 (Revalidation, Spec v02.96)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Tokenization-Service-20251228.md (STATUS: Done)
- Spec: Handshake_Master_Spec_v02.96 §4.6.1 (Tokenizer Trait)

Findings:
- Panic-safe token counting/truncation with cl100k_base fallback for unknown models; heuristic used if fallback unavailable, avoiding panic (tokenization.rs).
- Async trait surface present; tests cover known + unknown model and truncate behavior.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests --quiet` (PASS)

Reason for PASS: Tokenizer behavior matches §4.6.1 requirements with panic-free fallback; targeted tests pass.

## STATUS CANONICAL (2025-12-28)
- Authoritative STATUS: Done (validated against Master Spec v02.96).
- Earlier status lines in this packet are historical and retained for audit only.
**Last Updated:** 2025-12-28
**User Signature Locked:** ilja281220250435
