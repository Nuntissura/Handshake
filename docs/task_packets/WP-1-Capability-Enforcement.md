# Task Packet: WP-1-Capability-Enforcement

## Metadata
- TASK_ID: WP-1-Capability-Enforcement
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** In Progress
- USER_SIGNATURE: <pending>

## Scope
- **What**: Implement mandatory capability checks in the Workflow Engine. Jobs must possess required capabilities (e.g., `doc.summarize`) to execute.
- **Why**: Enforce "Safety through Architecture" by preventing unauthorized AI mutations or data access, fulfilling Phase 1 Item 4 of the Master Spec.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/jobs.rs
  * src/backend/handshake_core/src/models.rs
- **OUT_OF_SCOPE**:
  * A complex UI for managing capability roles (keep as code-defined maps for now).
  * Dynamic user-driven capability delegation.

## Quality Gate
- **RISK_TIER**: MEDIUM
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  node scripts/validation/post-work-check.mjs WP-1-Capability-Enforcement
  ```
- **DONE_MEANS**:
  * The `Workflow Engine` verifies if the job's `capability_profile_id` contains the required permissions for the `job_kind`.
  * If unauthorized, the job status moves to `failed` with an `error_message` containing "Unauthorized: Missing capability X".
  * The `doc_summarize` job kind is gated by the `doc.summarize` capability.
  * Added at least one backend test verifying that a job with an invalid profile fails.
- **ROLLBACK_HINT**:
  ```bash
  git checkout src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/src/jobs.rs
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * Handshake_Master_Spec_v02.50.md (§7.6.3 Item 4)
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/jobs.rs
  * src/backend/handshake_core/src/models.rs
- **SEARCH_TERMS**:
  * "capability_profile_id"
  * "run_job"
  * "doc_summarize"
  * "AiJob"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Valid jobs fail due to strictness" -> Workflow Engine
  * "Capability bypass via job_kind rename" -> Logic Layer
  * "Error message leaks sensitive info" -> Security

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: docs/TASK_BOARD.md
- **Logger**: Optional (not used for this WP)

## Notes
- **Implementation Detail**: Define a simple static constant or internal helper in `workflows.rs` that maps `job_kind` strings to a required capability string. For now, the "default" profile should contain `doc.read` and `doc.summarize`.

## Validation
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml -> PASS
- pnpm -C app test -> PASS

## Status / Handoff
- WP_STATUS: Completed
- What changed: Added capability map/enforcement; unauthorized profiles fail with explicit message; regression test added in workflows.rs.
- Next step / handoff hint: Add persistent capability profile management when new job kinds/profiles are introduced.
