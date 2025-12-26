# Task Packet: WP-1-Terminal-Integration-Baseline

## Metadata
- TASK_ID: WP-1-Terminal-Integration-Baseline
- DATE: 2025-12-25
- REQUESTOR: User
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- STATUS: Ready for Dev
- RISK_TIER: HIGH
  - Justification: Enables arbitrary command execution on the host system; high security risk if capability gate is bypassed.
- USER_SIGNATURE: ilja251220251821

---

## SCOPE

### Executive Summary

Implement a secure, capability-gated terminal execution tool in the backend and expose it to the workflow engine. This involves replacing the current "disabled during security hardening" placeholder with actual execution logic that respects the `term.exec` capability.

**Goal:** Allow mechanical jobs (builds, git operations) to run under strict governance.

### IN_SCOPE_PATHS
- src/backend/handshake_core/src/terminal.rs (Implementation)
- src/backend/handshake_core/src/workflows.rs (Workflow integration)
- src/backend/handshake_core/src/api/jobs.rs (Job API surface)
- src/backend/handshake_core/src/lib.rs (AppState / Service wiring)
- src/backend/handshake_core/src/models.rs (Job kinds/inputs)

### OUT_OF_SCOPE
- Interactive PTY / streaming terminal UI (Phase 2).
- Arbitrary shell access without capability tokens (Forbidden).
- Long-running processes without timeout enforcement (Handled in Phase 1 but limited).
- Multi-user terminal isolation (Phase 2/Security hardening).
- Input sanitization for shell-specific characters (Focus is on direct binary execution).
- Custom environment variable injection (Standard env only for now).
- Terminal output paging/streaming (Full capture only).

---

## QUALITY GATE

- **TEST_PLAN**:
  ```bash
  # Compile check
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  
  # Run storage and workflow tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  
  # Validate workflow compliance
  just gate-check WP-1-Terminal-Integration-Baseline
  
  # Final hygiene and traceability
  just post-work WP-1-Terminal-Integration-Baseline
  ```
- **DONE_MEANS**:
  - ✅ `TerminalService` exists and wraps `tokio::process::Command` (not `std::process`).
  - ✅ Placeholder "terminal jobs are disabled" in `workflows.rs` is REMOVED.
  - ✅ Terminal jobs ONLY run if the job profile contains the `term.exec` capability.
  - ✅ Command stdout/stderr is captured and logged to Flight Recorder.
  - ✅ Success and failure (missing capability) are verified by integration tests.
  - ✅ Evidence mapping block is complete and accurate.

---

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.84.md (§2.3.12, §7.6.3)
  * src/backend/handshake_core/src/terminal.rs
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/flight_recorder.rs
  * src/backend/handshake_core/src/lib.rs
  * src/backend/handshake_core/src/api/jobs.rs
  * src/backend/handshake_core/src/models.rs
  * tests/README.md

- **SEARCH_TERMS**:
  * "term.exec" (Capability search)
  * "terminal jobs are disabled" (Placeholder location)
  * "tokio::process::Command" (Execution pattern)
  * "execute_terminal_job" (Function signature)
  * "WorkflowError" (Error variants)
  * "AiJob" (Data structure)
  * "CapabilityProfile" (Gate logic)
  * "log_event" (Flight recorder integration)
  * "timeout" (Execution safety)
  * "current_user" (Actor attribution)

- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gate-check WP-1-Terminal-Integration-Baseline
  ```

- **RISK_MAP**:
  * "Command injection" -> Security (use structured args, not shell strings)
  * "Process leak" -> Resource Management (ensure child is killed on timeout)
  * "Capability bypass" -> Governance (logic error in gate)

## SKELETON
- API: `/jobs` accepts `term_exec|terminal_exec` and maps capability profile server-side (`terminal`), rejecting unknown job kinds.
- Service: `TerminalService::run(program, args, timeout_ms) -> TerminalOutput { stdout, stderr, status_code }` using `tokio::process::Command`.
- Workflow: `execute_terminal_job` calls `TerminalService::run` and enforces `term.exec` capability before execution.

---

## RISK_TIER
- Level: HIGH
- Rationale: Spec-governed audit; failure blocks Phase 1 closure.

## TEST_PLAN
`ash
just validator-spec-regression
just validator-scan WP-1-Terminal-Integration-Baseline
just validator-hygiene-full
`

## DONE_MEANS
- Spec requirements from referenced anchors are fully implemented or gaps recorded with FAIL.
- Forbidden-pattern audit is clean or explicitly justified.
- TEST_PLAN commands executed and outputs captured in the validation report.
- Evidence mapping lists file:line for every requirement.

## BOOTSTRAP
- FILES_TO_OPEN:
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.84.md
  * docs/TASK_BOARD.md
- SEARCH_TERMS:
  * "WP-1-Terminal-Integration-Baseline"
  * spec anchor keywords
- RUN_COMMANDS:
  `ash
  just validator-spec-regression
  just validator-scan WP-1-Terminal-Integration-Baseline
  `
- RISK_MAP:
  * "Spec mismatch" -> validate SPEC_CURRENT and anchors
  * "Placeholder evidence" -> block until file:line mapping exists
  * "Forbidden patterns" -> run validator-scan and fix findings

## AUTHORITY
- SPEC_CURRENT: Handshake_Master_Spec_v02.84.md
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md\n\n## EVIDENCE_MAPPING
- TerminalService implementation with timeout kill + captured stdout/stderr: src/backend/handshake_core/src/terminal.rs:26-67
- Capability gate mapping for job kinds: src/backend/handshake_core/src/workflows.rs:36-66
- Workflow integration (terminal execution path): src/backend/handshake_core/src/workflows.rs:178-233
- Flight Recorder logging of terminal execution: src/backend/handshake_core/src/workflows.rs:222-228
- API surface allows terminal job creation with server-mapped capability profile: src/backend/handshake_core/src/api/jobs.rs:53-99
- Tests: capability and terminal execution coverage: src/backend/handshake_core/src/workflows.rs:277-356; API gating tests: src/backend/handshake_core/src/api/jobs.rs:142-207

---

## AUTHORITY
- **SPEC_ANCHOR**: §7.6.3 Item 14 (Terminal LAW)
- **Codex**: Handshake Codex v1.4.md
- RE-OPENED: Validation failed. Feature is hard-blocked in code and evidence mapping is missing.

## VALIDATION REPORT — WP-1-Terminal-Integration-Baseline
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Terminal-Integration-Baseline.md
- Spec: Handshake_Master_Spec_v02.84.md

Files Checked:
- src/backend/handshake_core/src/terminal.rs
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/storage/sqlite.rs

Findings:
- **Evidence Mapping**: Verified. Links point to valid implementation.
- **Functionality**:
    - The "disabled" placeholder is removed.
    - `execute_terminal_job` is wired in `start_workflow_for_job`.
    - Capability gate `term.exec` is enforced in `enforce_capabilities`.
    - `TerminalService` uses `tokio::process::Command`.
- **Hygiene**:
    - Fixed a bug in `sqlite.rs` (`update_ai_job_status`) using `COALESCE` to prevent wiping outputs.
    - Tests updated to be platform-agnostic (using `cmd` on Windows, `echo` on Unix).
    - No forbidden patterns in production code.
- **Tests**: `terminal_job_runs_when_authorized` passed. `job_fails_when_missing_required_capability` passed.

Status Update:
- **DONE**. Feature is implemented, tested, and verified.


---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220251950

## VALIDATION REPORT — WP-1-Terminal-Integration-Baseline (Final PASS)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Terminal-Integration-Baseline.md
- Spec: Handshake_Master_Spec_v02.84.md (§7.6.3 Terminal LAW)
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/terminal.rs
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/api/jobs.rs

Findings:
- **Evidence Mapping [CX-627]**: Verified. Every MUST requirement from the spec (terminal execution, capability gating, logging) is mapped to specific code paths and verified by tests.
- **Forbidden Pattern Audit [CX-573E]**: PASS. No `unwrap`, `expect`, `todo!`, or `panic!` found in production paths. `unwrap_or` usage in tests is within protocol bounds.
- **Zero Placeholder Policy [CX-573D]**: PASS. The "disabled" placeholder has been replaced with a functional `TerminalService` using `tokio::process::Command`.
- **Functionality**:
    - `TerminalService` correctly handles timeouts with `kill_on_drop(true)`.
    - `workflows.rs` enforces `term.exec` capability and logs events to Flight Recorder.
    - `api/jobs.rs` allows terminal job creation while server-side mapping the capability profile to prevent client escalation.
- **Storage DAL Audit**: PASS. No database leaks or SQL portability issues introduced.

Tests:
- `cargo test`: PASS (7 tests: 3 workflow, 2 health, 2 API).
- `just gate-check`: PASS (SKELETON APPROVED sequence verified).
- `just post-work`: PASS.

**REASON FOR PASS**: The implementation is robust, satisfies all architectural invariants for terminal execution, and includes comprehensive test coverage for both authorized and unauthorized execution paths.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** ilja251220251950





