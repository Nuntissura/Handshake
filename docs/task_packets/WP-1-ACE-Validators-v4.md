# Task Packet: WP-1-ACE-Validators-v4

## METADATA
- TASK_ID: WP-1-ACE-Validators-v4
- WP_ID: WP-1-ACE-Validators-v4
- DATE: 2026-01-06T23:23:44.061Z
- REQUESTOR: ilja
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- CODER_MODEL: Claude Code
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja070120260018

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-ACE-Validators-v4.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement/align ACE runtime validators and enforcement invariants per SPEC_CURRENT (Handshake_Master_Spec_v02.101.md 2.6.6.7.11), including content-awareness, NFC-normalized scanning, cloud leakage blocking, and atomic poisoning behavior.
- Why: Security-critical runtime enforcement; prevents prompt injection and data leakage bypasses and enforces deterministic, spec-aligned safety behavior.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/artifact.rs
  - src/backend/handshake_core/src/ace/validators/boundary.rs
  - src/backend/handshake_core/src/ace/validators/budget.rs
  - src/backend/handshake_core/src/ace/validators/cache.rs
  - src/backend/handshake_core/src/ace/validators/compaction.rs
  - src/backend/handshake_core/src/ace/validators/determinism.rs
  - src/backend/handshake_core/src/ace/validators/drift.rs
  - src/backend/handshake_core/src/ace/validators/freshness.rs
  - src/backend/handshake_core/src/ace/validators/injection.rs
  - src/backend/handshake_core/src/ace/validators/leakage.rs
  - src/backend/handshake_core/src/ace/validators/payload.rs
  - src/backend/handshake_core/src/ace/validators/promotion.rs
  - src/backend/handshake_core/src/workflows.rs
- OUT_OF_SCOPE:
  - Any storage layer work and dual-backend CI/test plumbing (reserved for WP-1-Dual-Backend-Tests-v2), including:
    - .github/workflows/ci.yml
    - docker-compose.test.yml
    - src/backend/handshake_core/src/storage/
    - src/backend/handshake_core/tests/storage_conformance.rs
  - UI work (frontend Flight Recorder lenses, editors, etc.)
  - Spec edits (spec enrichment requires separate workflow + signature)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before starting:
just pre-work WP-1-ACE-Validators-v4

# Build + test:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Targeted tests (recommended):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace::validators::injection
cargo test --manifest-path src/backend/handshake_core/Cargo.toml ace::validators::leakage

# Hygiene:
just validator-scan
just validator-dal-audit
just validator-git-hygiene

# Run before handoff:
just cargo-clean
just post-work WP-1-ACE-Validators-v4
```

### DONE_MEANS
- [HSK-ACE-VAL-100] Content-awareness: PromptInjectionGuard and CloudLeakageGuard operate on resolved raw UTF-8 content of retrieved snippets (not hashes/handles); missing raw content is treated as a blocking error for validation.
- [HSK-ACE-VAL-102] Normalization: injection substring scanning is performed on NFC-normalized, case-folded text, including the required pattern list and any profile-specific patterns.
- [HSK-ACE-VAL-101] Atomic poisoning: PromptInjectionDetected triggers JobState::Poisoned, terminates active workflow nodes, emits FR-EVT-SEC-VIOLATION, and prevents further workspace mutation for the poisoned job_id.
- CloudLeakageGuard blocks Cloud model calls when exportable=false or sensitivity=high, including recursive checks for bundle/dataset_slice SourceRefs.
- Validator set: all validators listed in Master Spec 2.6.6.7.11.1-2.6.6.7.11.12 exist and are invoked by the runtime on applicable job steps (deterministic ordering).
- Gates: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes and `just post-work WP-1-ACE-Validators-v4` passes.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.101.md (recorded_at: 2026-01-06T23:23:44.061Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.101.md 2.6.6.7.11 Validators (runtime-enforced; required) + 2.6.6.7.12 Logging + Acceptance Tests (minimum)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.101.md (2.6.6.7.11-2.6.6.7.12)
  - docs/task_packets/WP-1-ACE-Validators-v3.md (history; do not edit)
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/ace/validators/mod.rs
  - src/backend/handshake_core/src/ace/validators/injection.rs
  - src/backend/handshake_core/src/ace/validators/leakage.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "HSK-ACE-VAL-100"
  - "HSK-ACE-VAL-101"
  - "HSK-ACE-VAL-102"
  - "PromptInjectionDetected"
  - "FR-EVT-SEC-VIOLATION"
  - "model_tier"
  - "exportable"
  - "sensitivity"
  - "bundle"
  - "dataset_slice"
  - "determinism_mode"
  - "ids_hash"
  - "tool_delta_inline_char_limit"
  - "local_only_payload_ref"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-ACE-Validators-v4
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- RISK_MAP:
  - "Hollow validator (hash/handle-only scan)" -> "Prompt injection/cloud leakage bypass"
  - "Missing NFC normalization/case folding" -> "Unicode/casing bypass of injection patterns"
  - "Incomplete poisoning directive" -> "Workspace mutation continues after injection detection"
  - "Over-broad pattern matching" -> "False positives poison jobs incorrectly"
  - "Non-recursive leakage checks" -> "bundle/dataset_slice leaks classified members to Cloud"
  - "Non-deterministic validator ordering" -> "Replay/strict determinism drift"

## SKELETON
- Proposed interfaces/types/contracts:
- Validator entrypoint resolves raw content for retrieved snippets before calling PromptInjectionGuard/CloudLeakageGuard.
- WorkflowEngine has a global trap for AceError::PromptInjectionDetected enforcing JobState::Poisoned + FR-EVT-SEC-VIOLATION + node termination.
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
