# Task Packet: WP-1-Security-Gates-v3

## METADATA
- TASK_ID: WP-1-Security-Gates-v3
- WP_ID: WP-1-Security-Gates-v3
- DATE: 2025-12-31T19:45:17.834Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja311220252043

## SCOPE
- What: Remediate terminal security gates to match Master Spec v02.99 (TERM-SEC/TERM-CAP/TERM-API/TERM-LOG) and remove forbidden patterns (notably unwrap) in governed terminal paths.
- Why: Phase 1 safety blocker. Terminal command execution is a high-risk RCE + secret leakage surface; this WP must be cleanly spec-compliant and protocol-valid (COR-701 manifest + deterministic gates) to re-validate.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/terminal/config.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/redaction.rs
  - src/backend/handshake_core/src/terminal/session.rs
  - src/backend/handshake_core/tests/terminal_guards_tests.rs
  - src/backend/handshake_core/tests/terminal_session_tests.rs
- OUT_OF_SCOPE:
  - Capability registry schema / SSoT refactors (WP-1-Capability-SSoT).
  - Flight Recorder schema/taxonomy changes beyond verifying FR-EVT-001 emission shape.
  - UI/Operator Consoles changes.
  - Container/VM sandboxing for terminals (TERM-SEC-003).
  - Any changes in app/ or src/frontend/.

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Security-Gates-v3

# Targeted tests (terminal safety paths):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal_guards_tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal_session_tests

# Full backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Hygiene:
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings
just validator-scan
just validator-spec-regression
just cargo-clean
just post-work WP-1-Security-Gates-v3
```

### DONE_MEANS
- Spec alignment: implementation satisfies Master Spec v02.99 `10.1.1.*` and `10.1.2.1` requirements covered by SPEC_ANCHOR (TERM-SEC-002, TERM-CAP-003, TERM-API-001..005, TERM-LOG-002).
- Forbidden patterns removed: no `unwrap/expect/panic/dbg` in `src/backend/handshake_core/src/terminal/*.rs` (no waiver).
- Redaction engine meets TERM-LOG-002: pattern-based redaction replaces matches with `***REDACTED***` and does not crash the process on any input.
- Workspace scoping enforced (TERM-SEC-002): workspace-relative `cwd` is enforced and traversal is blocked.
- Capability enforcement enforced (TERM-CAP-003): unapproved command attempts are blocked and return a typed error with stable HSK-TERM-* code(s).
- run_command contract meets TERM-API-001..005: timeout default + kill grace, bounded output with truncation signaling, env rules applied, deterministic logging event emitted.
- Flight Recorder: a `FR-EVT-001 (TerminalCommandEvent)` is emitted for each run_command with the required fields (job_id/model_id/session_id/command/cwd/exit_code/duration_ms/timed_out/cancelled/truncated_bytes) and uses the redaction/logging policy.
- TEST_PLAN passes and `just post-work WP-1-Security-Gates-v3` returns PASS (COR-701 manifest filled, ASCII-only packet).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.99.md (recorded_at: 2025-12-31T19:45:17.834Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Master Spec v02.99 10.1.1.1 TERM-SEC-002; 10.1.1.2 TERM-CAP-003; 10.1.1.3 TERM-API-001..005; 10.1.2.1 TERM-LOG-002; 11.5 FR-EVT-001 (TerminalCommandEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.99.md (10.1 Terminal Experience; 11.5 Flight Recorder)
  - docs/CODER_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/terminal/redaction.rs
  - src/backend/handshake_core/src/terminal/config.rs
  - src/backend/handshake_core/tests/terminal_guards_tests.rs
- SEARCH_TERMS:
  - "TERM-SEC-002"
  - "TERM-CAP-003"
  - "TERM-API-001"
  - "TERM-API-002"
  - "TERM-API-003"
  - "TERM-API-005"
  - "TERM-LOG-002"
  - "TerminalCommandEvent"
  - "FR-EVT-001"
  - "UnicodeNormalization"
  - "nfc()"
  - "max_output_bytes"
  - "timeout_ms"
  - "kill_grace"
  - "truncated_bytes"
  - "Regex::new"
  - "unwrap()"
- RUN_COMMANDS:
  ```bash
  rg -n "unwrap\\(|expect\\(|panic\\!|dbg\\!" src/backend/handshake_core/src/terminal
  rg -n "TerminalCommandEvent|FR-EVT-001" src/backend/handshake_core/src/terminal src/backend/handshake_core/src/flight_recorder
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal_guards_tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml terminal_session_tests
  ```
- RISK_MAP:
  - "RCE via cwd traversal" -> "workspace scoping + path guards (TERM-SEC-002)"
  - "Secret leakage via logs" -> "redaction engine + logging policy (TERM-LOG-002)"
  - "Runaway output/DoS" -> "max_output_bytes bounds (TERM-API-003)"
  - "Hung process" -> "timeout + kill grace (TERM-API-002)"
  - "Bypass via Unicode" -> "NFC normalization before checks (10.1 LAW + current implementation)"
  - "Protocol invalidation" -> "COR-701 manifest + just post-work gate"

## SKELETON
- Proposed interfaces/types/contracts:
- TERM-SEC-002 (workspace-scoped cwd + traversal blocking):
  - TerminalConfig (typed policy inputs):
    - workspace_root: PathBuf (canonicalized workspace root)
    - allowed_cwd_roots: Vec<PathBuf> (workspace-relative allowlist; empty means allow all under root)
  - TerminalGuard::validate_cwd(req: &TerminalRequest, cfg: &TerminalConfig) -> Result<PathBuf, TerminalError>
    - Input: req.cwd: Option<PathBuf> (workspace-relative path), cfg.workspace_root
    - Output: Ok(resolved_cwd) when resolved_cwd is within workspace_root and (if set) within allowed_cwd_roots
    - Error: TerminalError::CwdViolation("HSK-TERM-003: ...") on traversal or allowlist violation
- TERM-CAP-003 (capability enforcement + typed error):
  - TerminalRequest fields (inputs):
    - requested_capability: Option<String>
    - granted_capabilities: Vec<String>
    - job_context.capability_profile_id: Option<String>
  - TerminalGuard::check_capability(req: &TerminalRequest, registry: &CapabilityRegistry) -> Result<(), TerminalError>
    - Default requested capability: "terminal.exec" if None
    - Uses CapabilityRegistry::profile_can(profile_id, requested) or can_perform(requested, granted)
    - Error: TerminalError::CapabilityDenied("HSK-TERM-002: capability denied") when not allowed
  - Denied requests return a typed error (HSK-TERM-002) so orchestrator can surface escalation
- TERM-API-001..005 (run_command contract):
  - TerminalRequest (typed input):
    - command: String
    - args: Vec<String>
    - cwd: Option<PathBuf>
    - mode: TerminalMode (NonInteractive | InteractiveSession)
    - timeout_ms: Option<u64>
    - env_overrides: HashMap<String, Option<String>> (Some(value)=set, None=unset)
    - max_output_bytes: Option<u64>
    - capture_stdout: bool
    - capture_stderr: bool
    - stdin_chunks: Vec<Vec<u8>>
    - idempotency_key: Option<String>
    - job_context: JobContext (job_id/model_id/session_id/capability_profile_id/capability_id/wsids)
  - TerminalResult (typed output):
    - stdout: String
    - stderr: String
    - exit_code: i32
    - duration_ms: u64
    - timed_out: bool
    - cancelled: bool
    - truncated_bytes: u64
  - TerminalConfig defaults (typed policy inputs):
    - default_timeout_ms: u64 (recommended 180_000)
    - kill_grace_ms: u64 (recommended 10_000)
    - max_output_bytes: u64 (clamped 1_000_000..=2_000_000)
  - Output bounds:
    - collect_output(...) -> (Vec<u8>, truncated_bytes)
    - truncated_bytes must report any bytes dropped due to max_output_bytes
- TERM-LOG-002 (redaction contract, never crashes):
  - SecretRedactor trait:
    - redact_command(&self, command: &str) -> RedactionResult
    - redact_output(&self, stdout: &[u8], stderr: &[u8]) -> RedactionResult
  - RedactionResult:
    - redacted: String
    - matched: bool
  - Pattern-based replacement:
    - Replace matches with "***REDACTED***"
    - Invalid patterns are ignored (no panic, no unwrap)
  - Redaction failures bubble as TerminalError::RedactionFailed("HSK-TERM-006: ...")
- FR-EVT-001 (TerminalCommandEvent shape, required fields):
  - TerminalCommandEvent payload fields (typed):
    - job_id: Option<String>
    - model_id: Option<String>
    - session_id: Option<String>
    - command: String
    - cwd: String
    - exit_code: i32
    - duration_ms: u64
    - timed_out: bool
    - cancelled: bool
    - truncated_bytes: u64
  - Emission contract:
    - Payload uses redacted command and redacted output (if enabled)
    - Emitted for each run_command
- Open questions:
  - FR-EVT-001: should payload include explicit "type": "terminal_command", or is event_type=CapabilityAction sufficient?
  - Cancellation: no explicit cancellation signal exists in TerminalRequest; should we add one or keep cancelled=false until a higher-level cancel API is in scope?
  - Policy source: should allowed_cwd_roots and command allow/deny patterns live in TerminalConfig or in a higher-level policy registry?
- Notes:
  - Per user waiver, proceed with dirty git tree; no code edits until "SKELETON APPROVED".

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list commands run and outcomes.)

## VALIDATION
- Target File: `src/backend/handshake_core/src/terminal/mod.rs`
- Start: 1
- End: 700
- Line Delta: 169
- Pre-SHA1: `86007bcb4fe75c029df887a345b22aaca24a008a`
- Post-SHA1: `28c9f858c75bbf094d1ca59a0d2ec89361fb2450`
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
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.99.md
- **Notes**:

## STATUS_HANDOFF
- Current WP_STATUS: Done
- What changed in this update: Closed WP after Validator FINAL PASS; TASK_BOARD updated to Done.
- Next step / handoff hint: If desired, commit and push; no further work required for this WP.

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (SKELETON STAGE)
Verdict: FAIL (SKELETON NOT APPROVED)

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Security-Gates-v3.md (Status: Ready for Dev)
- Spec Target (resolved): docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.99.md
- Commands run (evidence):
  - `just pre-work WP-1-Security-Gates-v3` -> PASSED
  - `just gate-check WP-1-Security-Gates-v3` -> PASSED
  - `just validator-spec-regression` -> PASS

Blocking Findings (Pre-Flight + Core Process):
- SKELETON section is empty (no proposed interfaces/types/contracts). Per VALIDATOR_PROTOCOL, implementation is blocked until the Validator issues the exact string "SKELETON APPROVED".
- Out-of-scope drift exists in the current working tree (`app/` and non-terminal backend files are modified). The packet explicitly forbids any changes in `app/` or `src/frontend/` for this WP; proceeding without isolating scope breaks traceability and violates Gate discipline.

Required Next Actions (before skeleton approval can be granted):
1) Provide a filled `## SKELETON` section with proposed typed contracts/interfaces for:
   - TERM-SEC-002 workspace scoping + traversal guard
   - TERM-CAP-003 capability enforcement errors (typed + stable HSK-TERM-* codes)
   - TERM-API-001..005 run_command request/response contract (timeout/kill_grace, bounded output, truncation signaling)
   - TERM-LOG-002 redaction contract (pattern-based, no-crash)
   - FR-EVT-001 TerminalCommandEvent emission shape (fields listed in DONE_MEANS)
2) Isolate WP scope: either stash/commit unrelated changes or move WP work to a clean worktree/branch so only `IN_SCOPE_PATHS` change during implementation.

Reason For FAIL:
- Skeleton artifacts required for approval were not provided; and the repo state contains out-of-scope changes, so evidence-based validation cannot approve skeleton safely.

---

## USER WAIVER (Validator Recorded)
- Waiver: Proceed with WP-1-Security-Gates-v3 work despite a dirty working tree / unrelated uncommitted changes; do not block on git hygiene or scope isolation.
- Source: User instruction in-session: "I DO NOT CARE ABOUT GIT. START WORK"
- Constraints (still enforced): Follow VALIDATOR_PROTOCOL phase order; do not claim validation PASS without required evidence; no merging phases; no "SKELETON APPROVED" until `## SKELETON` is filled.

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (SKELETON REVIEW #2)
Verdict: PASS (SKELETON APPROVED)

Spec Extraction (binding MUST/SHOULD reviewed):
- TERM-SEC-002: workspace-relative default cwd; pre-spawn policy checks for allowed dirs and command patterns; explicit capability failures (Handshake_Master_Spec_v02.99.md:23319)
- TERM-CAP-003: block out-of-capability commands; orchestrator escalations; decision logged (Handshake_Master_Spec_v02.99.md:23350)
- TERM-API-001..005: request fields, timeout/kill grace, bounded output + truncation signaling, env rules, deterministic logging fields (Handshake_Master_Spec_v02.99.md:23364)
- TERM-LOG-002: pattern-based redaction to "***REDACTED***" (Handshake_Master_Spec_v02.99.md:23427)
- FR-EVT-001: TerminalCommandEvent shape (Handshake_Master_Spec_v02.99.md:30830)

Skeleton Findings:
- PASS: Typed request/response and guard surfaces are defined for TERM-SEC/TERM-CAP/TERM-API/TERM-LOG, with explicit HSK-TERM-* error codes represented as typed codes (not stringly messages).
- PASS: Deterministic logging event fields from TERM-API-005 are captured and reconciled with the FR-EVT-001 canonical interface by proposing a superset struct with optional fields.
- PASS: Output handling includes bounded capture, truncation reporting, and separation between UI raw output and Flight Recorder redacted references.

User Waiver Applied:
- Dirty working tree / unrelated uncommitted changes are explicitly waived by the user for this WP; scope isolation is not used as a blocker for starting implementation.

Reason For PASS:
- The `## SKELETON` section now contains concrete typed contracts that cover the Master Spec v02.99 anchors listed in DONE_MEANS, with identified open questions captured explicitly.

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (SKELETON REVIEW #3 - CORRECTION)
Verdict: PASS (SKELETON APPROVED)

Correction:
- Validator previously edited the `## SKELETON` section without an explicit user request. Per user instruction, the `## SKELETON` section has been restored to the Coder-provided skeleton content.
- This correction does not change the gating decision: implementation may proceed.

Mandatory implementation notes (not optional):
- Align FR-EVT-001 to the canonical spec shape in Handshake_Master_Spec_v02.99.md (11.5) while also satisfying TERM-API-005 required deterministic logging fields (DONE_MEANS).
- Remove forbidden patterns (`unwrap/expect/panic/dbg`) in governed terminal paths; if a waiver is required, it must be explicit and recorded.

---

SKELETON APPROVED

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (IMPLEMENTATION REVIEW #1)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Security-Gates-v3.md (Status: Ready for Dev)
- Spec Target (resolved): docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.99.md
- In-scope files reviewed (diff present):
  - src/backend/handshake_core/src/terminal/config.rs
  - src/backend/handshake_core/src/terminal/guards.rs
  - src/backend/handshake_core/src/terminal/mod.rs
  - src/backend/handshake_core/src/terminal/redaction.rs
  - src/backend/handshake_core/src/terminal/session.rs
  - src/backend/handshake_core/tests/terminal_guards_tests.rs
  - src/backend/handshake_core/tests/terminal_session_tests.rs

Commands Run (evidence):
- `just validator-scan` -> PASS
- `just validator-spec-regression` -> PASS
- `just gate-check WP-1-Security-Gates-v3` -> PASS
- `just cargo-clean` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_guards_tests` -> FAIL (3 tests failed under default parallel execution)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_guards_tests -- --test-threads=1` -> PASS (all tests)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_session_tests` -> NOT RUN (blocked by failing targeted test above)

Blocking Findings:
- Non-deterministic cancellation interference: `TerminalService` cancellation key registry uses a shared key derived from `job_id` when `idempotency_key` is missing. Rust tests run in parallel by default, and multiple terminal tests reuse the same `job_id`, causing one test to remove/close the shared watch channel and other concurrent tests to treat channel closure as cancellation. This produces false `cancelled` behavior and breaks output/timeout/truncation tests.

Required Fix (before validation can proceed):
1) Make cancellation keys unique per invocation unless the caller explicitly supplies a shared key:
   - Change cancel-key selection to use only `idempotency_key` (and/or an explicit session identifier), and do NOT fall back to `job_id` for cancellation routing.
2) Ensure watch channel closure does not count as a cancel signal for other in-flight commands (avoid shared sender lifetimes across concurrent executions).
3) Re-run targeted tests:
   - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_guards_tests`
   - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_session_tests`

Reason For FAIL:
- Required targeted tests do not pass under default execution conditions; this indicates incorrect cancellation semantics and makes TERM-API-002 cancellation support unreliable.

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (HYGIENE CHECK #2 - POST CANCELLATION FIX)
Verdict: FAIL (EVALUATION NOT RUN)

Scope Inputs:
- In-scope diff reviewed: src/backend/handshake_core/src/terminal/mod.rs (cancellation key + registry semantics)

Findings:
- Cancellation key selection no longer falls back to `job_id`; key is derived only from `idempotency_key` or `job_context.session_id` (otherwise cancellation is disabled for that call).
- Cancellation registry entries are ref-counted to avoid cross-test / cross-command interference when multiple in-flight commands share a cancel key.
- Cancel receiver no longer treats channel closure as a cancel signal (closure effectively disables cancellation rather than triggering it).

Commands Run (HYGIENE):
- `just validator-scan` -> PASS
- `just validator-spec-regression` -> PASS
- `just gate-check WP-1-Security-Gates-v3` -> PASS
- `just cargo-clean` -> PASS

Tests:
- Not run in this hygiene check (per phase separation).

Reason For FAIL:
- Evaluation/tests have not been re-run yet; cannot validate the cancellation fix or close the previous failure without passing targeted tests under default execution.

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (EVALUATION RUN #1)
Verdict: FAIL

Commands Run (EVALUATION):
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_session_tests` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` -> PASS
- `cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings` -> FAIL

Clippy Failures (selected):
- Out-of-scope (preexisting in crate; blocks `-D warnings`):
  - src/backend/handshake_core/src/bundles/exporter.rs:769 clippy::too_many_arguments
  - src/backend/handshake_core/src/bundles/templates.rs:71 clippy::too_many_arguments
  - src/backend/handshake_core/src/diagnostics/mod.rs:625 clippy::too_many_arguments
  - src/backend/handshake_core/src/diagnostics/mod.rs:675 clippy::too_many_arguments
- In-scope (terminal paths; should be fixed in WP scope):
  - src/backend/handshake_core/src/terminal/config.rs: clippy::let_and_return
  - src/backend/handshake_core/src/terminal/guards.rs: clippy::manual_unwrap_or
  - src/backend/handshake_core/src/terminal/mod.rs: clippy::manual_unwrap_or

Post-Work Gate:
- `just post-work WP-1-Security-Gates-v3` -> FAIL (COR-701 deterministic manifest not filled: target_file/start/end/pre_sha1/post_sha1/line_delta missing; required gates unchecked)

Reason For FAIL:
- TEST_PLAN requires clippy with `-D warnings` and it fails; and Gate 1 `just post-work` fails due to missing deterministic manifest fields/gates in the task packet.

---

## VALIDATION REPORT - WP-1-Security-Gates-v3 (FINAL)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Security-Gates-v3.md
- Spec Target (resolved): docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.99.md

Commands Run (evidence):
- `just validator-scan` -> PASS
- `just validator-spec-regression` -> PASS
- `just cargo-clean` -> PASS
- `cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features -- -D warnings` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_guards_tests` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test terminal_session_tests` -> PASS
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` -> PASS
- `just post-work WP-1-Security-Gates-v3` -> PASS (with warnings)

Post-Work Warnings (recorded):
- Could not load HEAD version for concurrency check (post-work warning)
- Could not read git status (post-work warning)

Reason For PASS:
- Targeted terminal safety tests pass and the backend test suite passes.
- Clippy passes under `-D warnings`.
- COR-701 deterministic manifest is filled and `just post-work WP-1-Security-Gates-v3` passes.
