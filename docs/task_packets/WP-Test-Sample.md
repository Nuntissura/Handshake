# Task Packet: WP-Test-Sample

## Metadata
- TASK_ID: WP-Test-Sample
- DATE: 2025-12-18T21:10:54.506Z
- REQUESTOR: User
- AGENT_ID: Gemini (Orchestrator)
- ROLE: Orchestrator
- **Status:** In Progress
- USER_SIGNATURE: <pending>

## Scope
- **What**: A sample task to validate the new workflow automation scripts.
- **Why**: To confirm that the tooling introduced in Codex v0.8 is functioning correctly before it is used for real development work.
- **IN_SCOPE_PATHS**:
  * `docs/task_packets/WP-Test-Sample.md`
- **OUT_OF_SCOPE**:
  * All application source code.
  * Any changes to the database or backend.

## Quality Gate
- **RISK_TIER**: LOW
- **TEST_PLAN**:
  ```bash
  # Commands coder MUST run before claiming done:
  just pre-work WP-Test-Sample
  ```
- **DONE_MEANS**:
  * The `just pre-work` command for this packet passes.
  * The `just post-work` command for this packet passes.
- **ROLLBACK_HINT**:
  ```bash
  rm docs/task_packets/WP-Test-Sample.md
  ```

## Bootstrap (Coder Work Plan) [BOOTSTRAP]
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * .claude/ORCHESTRATOR_PROTOCOL.md
  * .claude/CODER_PROTOCOL.md
  * scripts/validation/pre-work-check.mjs
- **SEARCH_TERMS**:
  * "validation"
  * "task packet"
- **RUN_COMMANDS**:
  ```bash
  just pre-work WP-Test-Sample
  ```
- **RISK_MAP**:
  * "Validation script fails due to environment issue" -> "Local machine config / dependencies"
  * "Logger entry not found" -> "Orchestrator protocol step missed"

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Latest Logger**: Handshake_logger_20251218_v3.3_20251218T204200.md
- **ADRs**: None

## Notes
- **Assumptions**: This packet is for testing purposes only and will not be delegated to a coder agent for implementation.
- **Open Questions**: None
- **Dependencies**: None
