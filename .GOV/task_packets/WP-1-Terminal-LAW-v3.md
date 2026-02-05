# Task Packet: WP-1-Terminal-LAW-v3

## METADATA
- TASK_ID: WP-1-Terminal-LAW-v3
- WP_ID: WP-1-Terminal-LAW-v3
- DATE: 2026-01-01T22:19:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: claude-opus-4-5-20251101
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja010120262218
- SUPERSEDES: WP-1-Terminal-LAW-v2 (revalidation FAIL / governance drift; v3 is protocol-clean)

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Terminal-LAW-v3.md
- Rule: Implementation is BLOCKED until refinement is approved/signed and `just pre-work WP-1-Terminal-LAW-v3` passes.

## USER_CONTEXT (Non-Technical Explainer)

Terminal execution is one of the highest-risk capabilities: it can modify files, run arbitrary programs, and potentially leak secrets.

This WP ensures:
- AI terminal usage is isolated from human-created terminals by default (no attach/read/type into HUMAN_DEV sessions).
- Every allowed/denied terminal execution attempt is auditable via Flight Recorder events.
- Terminal execution requests have a stable, spec-defined shape (timeouts, output limits, env overrides, idempotency keys).

## WHY THIS PACKET WAS RECREATED (STOP-WORK NOTICE)

The original `.GOV/task_packets/WP-1-Terminal-LAW-v3.md` and its refinement file were missing from the working tree. Without the packet+refinement pair, the workflow gates cannot run, scope boundaries cannot be enforced, and work cannot be validated deterministically.

Do not start or continue coding until:
1) This packet exists on disk, and
2) `just pre-work WP-1-Terminal-LAW-v3` passes, and
3) The SKELETON is written and reviewed (interface-first), and
4) The validator instructs you to proceed.

## SCOPE
- What: Align and harden the `TerminalService::run_command` contract and terminal session isolation rules to SPEC_CURRENT v02.100 (Terminal Law 10.1), including deterministic auditing for both allow and deny outcomes.
- Why: Terminal execution is security-critical (RCE + data exfiltration surface). This WP restores governance-valid artifacts so the work can be performed and validated deterministically.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/session.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/terminal/config.rs
  - src/backend/handshake_core/src/terminal/redaction.rs
  - src/backend/handshake_core/tests/terminal_session_tests.rs
  - src/backend/handshake_core/tests/terminal_guards_tests.rs
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps (see `.GOV/refinements/WP-1-Terminal-LAW-v3.md`).
  - Any work in `app/` (frontend UI) or `src/` outside the IN_SCOPE_PATHS above.
  - Any changes to the MEX WP locked paths:
    - src/backend/handshake_core/src/mex/mod.rs
    - src/backend/handshake_core/src/mex/envelope.rs
    - src/backend/handshake_core/src/mex/gates.rs
    - src/backend/handshake_core/src/mex/registry.rs
    - src/backend/handshake_core/src/mex/runtime.rs
    - src/backend/handshake_core/src/mex/conformance.rs
    - src/backend/handshake_core/src/lib.rs
    - src/backend/handshake_core/mechanical_engines.json
    - src/backend/handshake_core/tests/mex_tests.rs
  - Do not edit `.GOV/roles_shared/TASK_BOARD.md` in a two-coder session (orchestrator updates it to avoid collisions).

## WAIVERS GRANTED
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Gate 0: must pass before touching code
just pre-work WP-1-Terminal-LAW-v3

# Backend format/lint/tests (required):
cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Forbidden patterns (scoped):
rg "unwrap\\(|expect\\(|todo!\\(|unimplemented!\\(|dbg!\\(|println!\\(|eprintln!\\(" src/backend/handshake_core/src/terminal

just cargo-clean
just post-work WP-1-Terminal-LAW-v3
```

### DONE_MEANS
- Packet governance: `just pre-work WP-1-Terminal-LAW-v3` passes.
- Terminal API contract (Master Spec v02.100 10.1.1.3 TERM-API-001..005):
  - Request supports: command, cwd, mode, timeout_ms, env_overrides, max_output_bytes, capture flags, stdin_chunks, idempotency_key.
  - Deterministic logging includes at least: job_id/model_id/session_id when present plus command/cwd/exit_code/duration_ms plus timed_out/cancelled/truncated_bytes.
- Human vs AI isolation (Master Spec v02.100 10.1.2.3 TERM-UX-001..003):
  - AI context cannot attach/read/type into HUMAN_DEV sessions unless explicitly requested, capability-guarded, and user-confirmed.
- Capability auditability (Master Spec v02.100 11.1 HSK-4001 + audit requirement):
  - Unknown capability IDs rejected with HSK-4001 (no default-allow).
  - Allow/Deny outcomes recorded to Flight Recorder with capability_id, actor_id, job_id when present, decision_outcome.
- Flight Recorder (Master Spec v02.100 11.5 FR-EVT-001):
  - Allowed terminal executions emit FR-EVT-001 (TerminalCommandEvent) with required minimum fields; event validates successfully.
- Tests/quality: All commands in TEST_PLAN pass and `just post-work WP-1-Terminal-LAW-v3` returns PASS with complete COR-701 manifests for every changed non-doc file.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-01T22:19:00Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.100.md 10.1.1.3 run_command API contract (TERM-API-001..005)
  - Handshake_Master_Spec_v02.100.md 10.1.2.3 Human vs AI terminal invariants (TERM-UX-001..003)
  - Handshake_Master_Spec_v02.100.md 11.1 Capabilities & Consent Model (HSK-4001 + audit requirement)
  - Handshake_Master_Spec_v02.100.md 11.5 Flight Recorder Event Shapes - FR-EVT-001 (TerminalCommandEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles/coder/CODER_PROTOCOL.md
  - .GOV/roles/validator/VALIDATOR_PROTOCOL.md
  - .GOV/refinements/WP-1-Terminal-LAW-v3.md
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/terminal/session.rs
  - src/backend/handshake_core/src/terminal/redaction.rs
  - src/backend/handshake_core/tests/terminal_session_tests.rs
- SEARCH_TERMS:
  - "TerminalService::run_command"
  - "TerminalRequest"
  - "TerminalResult"
  - "terminal.attach_human"
  - "HSK-4001"
  - "TerminalCommandEvent"
  - "FR-EVT-001"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Terminal-LAW-v3
  rg "TerminalService::run_command|TerminalRequest|TerminalResult|terminal\\.attach_human|HSK-4001|TerminalCommandEvent" src/backend/handshake_core
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --tests terminal_session_tests
  ```
- RISK_MAP:
  - "AI attaches to HUMAN_DEV terminal" -> "RCE/data exfiltration + audit failure"
  - "Denied attempts not logged" -> "no traceability + compliance failure"
  - "Secrets in logs/output" -> "credential leakage via Flight Recorder"

## SKELETON
- Proposed interfaces/types/contracts:
  ```rust
  /// Emits a CapabilityAction audit event to the Flight Recorder.
  /// Per HSK-4001 audit requirement: every Allow/Deny must be recorded.
  /// Pattern matches workflows.rs:log_capability_check
  async fn emit_capability_audit(
      flight_recorder: &dyn FlightRecorder,
      trace_id: Uuid,
      capability_id: &str,
      profile_id: Option<&str>,
      job_id: Option<&str>,
      outcome: &str,  // "allowed" or "denied"
  ) -> Result<(), TerminalError>;
  ```
- Payload fields for FlightRecorderEventType::CapabilityAction:
  ```json
  {
      "capability_id": "terminal.exec" | "terminal.attach_human",
      "profile_id": "<from req.job_context.capability_profile_id>",
      "job_id": "<from req.job_context.job_id>",
      "outcome": "allowed" | "denied"
  }
  ```
- Open questions: None (resolved during skeleton review)
- Notes: Actor ID uses capability_profile_id per workflows.rs:550 pattern

SKELETON APPROVED

## IMPLEMENTATION
- Added `emit_capability_audit` helper function in `terminal/mod.rs:231-267` that emits CapabilityAction events to Flight Recorder
- Modified `TerminalService::run_command` to:
  - Detect AI-attaching-to-HUMAN_DEV scenarios
  - Emit CapabilityAction audit events for `terminal.attach_human` checks (allowed/denied)
  - Emit CapabilityAction audit events for capability checks (allowed/denied)
- Updated tests in `terminal_guards_tests.rs` to find TerminalCommandEvent by type (not rely on `.last()`)
- Updated tests in `terminal_session_tests.rs` with same fix
- Added 3 new tests: `audit_logs_capability_allowed`, `audit_logs_capability_denied`, `audit_logs_isolation_denied`

## HYGIENE
- `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check`: PASS
- `cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml -p handshake_core --lib -- -D warnings`: PASS
- `rg "unwrap\\(|expect\\(|todo!\\(|unimplemented!\\(|dbg!\\(|println!\\(|eprintln!\\(" src/backend/handshake_core/src/terminal`: No matches (PASS)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`: 158 tests passed, 0 failed
- Note: `cargo clippy --all-targets` has pre-existing failures in `oss_register_enforcement_tests.rs` (out of scope)

## VALIDATION

### Manifest Entry 1
- **Target File**: `src/backend/handshake_core/src/terminal/mod.rs`
- **Start**: 6
- **End**: 408
- **Line Delta**: 122
- **Pre-SHA1**: `4e1ff22861ebfd76ef92ebf7721e729ad3999d94`
- **Post-SHA1**: `4aa838ebf806095017969b86265362af63e7366d`
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

### Manifest Entry 2
- **Target File**: `src/backend/handshake_core/tests/terminal_guards_tests.rs`
- **Start**: 343
- **End**: 505
- **Line Delta**: 148
- **Pre-SHA1**: `4ab09bbddb07a54bb1effe3d410adec6d65d92d0`
- **Post-SHA1**: `7e6ffe85d15b68cc4eb7aff903fb5314b2468249`
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

### Manifest Entry 3
- **Target File**: `src/backend/handshake_core/tests/terminal_session_tests.rs`
- **Start**: 192
- **End**: 225
- **Line Delta**: 7
- **Pre-SHA1**: `3b780ef13efe35b84963a0c910805721adca5af6`
- **Post-SHA1**: `f52c6c8d611b2632500468df724c896f76f1895d`
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

- **Lint Results**: cargo fmt --check PASS, cargo clippy (lib) PASS
- **Artifacts**: 3 new tests added
- **Timestamp**: 2026-01-01
- **Operator**: Coder-B (claude-opus-4-5-20251101)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- **Notes**: Added capability audit logging per HSK-4001 audit requirement

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete, pending final validation
- What changed in this update:
  - `src/backend/handshake_core/src/terminal/mod.rs`: Added emit_capability_audit helper, modified run_command
  - `src/backend/handshake_core/tests/terminal_guards_tests.rs`: Fixed test event ordering, added 3 audit tests
  - `src/backend/handshake_core/tests/terminal_session_tests.rs`: Fixed test event ordering
- Next step / handoff hint: Ready for Validator to re-run after clippy fix lands

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

- REVALIDATION REPORT - WP-1-Terminal-LAW-v3 (2026-01-01)
  - Verdict: PASS
  - Why this revalidation exists:
    - Prior attempt could not close due to repo/worktree instability and formatting/manifest drift; this re-run re-established deterministic COR-701 compliance and re-executed the full validator command set.
  - Commands executed (validator):
    - cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Cargo Target/handshake-cargo-target": PASS
    - just validator-spec-regression: PASS
    - cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml: PASS
    - cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check: PASS
    - cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings: PASS
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
    - just validator-scan: PASS
    - just validator-dal-audit: PASS
    - just validator-git-hygiene: PASS
    - just post-work WP-1-Terminal-LAW-v3: PASS (staged-only)
  - Evidence mapping (requirements -> code/tests):
    - Capability allow/deny audit events emitted:
      - src/backend/handshake_core/src/terminal/mod.rs:231 (emit_capability_audit)
      - src/backend/handshake_core/src/terminal/mod.rs:323 (audit on attach_human allow/deny)
      - src/backend/handshake_core/src/terminal/mod.rs:357 (audit on terminal.exec allow/deny)
    - Tests verifying allow/deny/isolation audit events:
      - src/backend/handshake_core/tests/terminal_guards_tests.rs:367 (emits_capability_audit_on_allowed)
      - src/backend/handshake_core/tests/terminal_guards_tests.rs:408 (emits_capability_audit_on_denied)
      - src/backend/handshake_core/tests/terminal_guards_tests.rs:447 (emits_attach_human_audit_on_denied)
    - Unknown capability rejection path exists (HSK-4001):
      - src/backend/handshake_core/src/capabilities.rs:30 (UnknownCapability)
  - REASON FOR PASS:
    - DONE_MEANS satisfied with direct evidence and passing TEST_PLAN + hygiene + deterministic post-work gate for the staged WP diff set.
- VALIDATION REPORT - WP-1-Terminal-LAW-v3 (2026-01-01)
  - Verdict: PASS
  - Scope inputs:
    - Task Packet: .GOV/task_packets/WP-1-Terminal-LAW-v3.md (Status: Blocked at time of validation; implementation present)
    - Refinement: .GOV/refinements/WP-1-Terminal-LAW-v3.md (USER_REVIEW_STATUS: APPROVED)
    - Spec target resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
  - Commands executed (validator):
    - cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Cargo Target/handshake-cargo-target": PASS
    - just validator-spec-regression: PASS
    - cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check: PASS
    - cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings: PASS
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
    - just validator-scan: PASS
    - just validator-dal-audit: PASS
    - just validator-git-hygiene: PASS
    - just post-work WP-1-Terminal-LAW-v3: PASS (staged-only; concurrent non-WP changes were temporarily stashed)
  - Evidence mapping (requirements -> code/tests):
    - Capability allow/deny audit events emitted:
      - src/backend/handshake_core/src/terminal/mod.rs:231 (emit_capability_audit)
      - src/backend/handshake_core/src/terminal/mod.rs:328 (audit on attach_human allow/deny)
      - src/backend/handshake_core/src/terminal/mod.rs:357 (audit on terminal.exec allow/deny)
    - Tests verifying allow/deny/isolation audit events:
      - src/backend/handshake_core/tests/terminal_guards_tests.rs:368 (audit_logs_capability_allowed)
      - src/backend/handshake_core/tests/terminal_guards_tests.rs:414 (audit_logs_capability_denied)
      - src/backend/handshake_core/tests/terminal_guards_tests.rs:459 (audit_logs_isolation_denied)
    - Unknown capability rejection path exists (HSK-4001):
      - src/backend/handshake_core/src/capabilities.rs:30 (UnknownCapability)
  - REASON FOR PASS:
    - DONE_MEANS are satisfied with direct code+test evidence and full TEST_PLAN/hygiene PASS, and deterministic post-work gate PASS for the staged WP change set.


