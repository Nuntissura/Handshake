# Task Packet: WP-1-Terminal-Integration-Baseline

## Metadata
- TASK_ID: WP-1-Terminal-Integration-Baseline
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** In Progress
- USER_SIGNATURE: <pending>

## Scope
- **What**: Implement a secure, capability-gated terminal execution tool in the backend and expose it to the workflow engine.
- **Why**: Fulfill Master Spec §7.6.3 Item 14 (Terminal LAW) to allow mechanical jobs (builds, git operations) under strict governance.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/terminal.rs (New)
  * src/backend/handshake_core/src/api/jobs.rs (Route for terminal jobs)
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/models.rs
  * src/backend/handshake_core/Cargo.toml
- **OUT_OF_SCOPE**:
  * Interactive PTY / streaming terminal UI (Phase 2).
  * Arbitrary shell access without capability tokens.

## Quality Gate
- **RISK_TIER**: HIGH
- **TEST_PLAN**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  node scripts/validation/post-work-check.mjs WP-1-Terminal-Integration-Baseline
  ```
- **DONE_MEANS**:
  * `TerminalService` exists and wraps `std::process::Command`.
  * `run_terminal_job` logic added to `workflows.rs`.
  * Terminal jobs fail if `term.exec` capability is missing in the job profile.
  * Command stdout/stderr is captured and logged to Flight Recorder (event_type: `terminal_exec`).
  * Backend test verifies capability gate failure and success execution.
- **ROLLBACK_HINT**:
  ```bash
  rm src/backend/handshake_core/src/terminal.rs
  git checkout src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/models.rs
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * Handshake_Master_Spec_v02.50.md (§7.6.3 Item 14)
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/models.rs
  * src/backend/handshake_core/src/flight_recorder.rs
- **SEARCH_TERMS**:
  * "term.exec"
  * "Command::new"
  * "log_event"
  * "run_job"
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Command injection" -> Security (use structured args, not shell string)
  * "Long running process blocks workflow" -> Workflow Engine (use timeout)
  * "Capability bypass" -> Capability Gate

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Implementation Detail**: Do not use `sh -c` or `cmd /c` by default. Execute binaries directly to minimize injection surface unless explicitly requested by a "shell" job kind. Use `tokio::process::Command`.

## Validation
- cargo check --manifest-path src/backend/handshake_core/Cargo.toml -> PASS (warnings: unused ApiJobError, TerminalError::Exec)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml -> PASS (warnings: unused ApiJobError, TerminalError::Exec)
- node scripts/validation/post-work-check.mjs WP-1-Terminal-Integration-Baseline -> PASS
- AI review: not run (not requested for this WP)

## Status / Handoff
- WP_STATUS: Completed (implementation and tests done; post-work check passed)
- What changed: Added TerminalService and capability gate (`term.exec`), added terminal job handling in workflows with Flight Recorder logging, capability profile map (default vs terminal), optional capability_profile_id/job_inputs in job API, backend tests for capability fail/success.
- Next step / handoff hint: Integrate UI/trigger if needed. Update capability profiles/policies when adding new job kinds. Consider AI review if desired for HIGH risk.
