# Task Packet: WP-1-Workflow-Engine-v2

## Metadata
- TASK_ID: WP-1-Workflow-Engine-v2
- WP_ID: WP-1-Workflow-Engine-v2
- DATE: 2025-12-26T03:12:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator


## SKELETON APPROVED
## User Context
We are making the "brain" of the app reliable. Currently, if the app crashes while an AI is working, we lose track of what it was doing. This task ensures every step the AI takes is written down immediately, so it can pick up right where it left off after a restart.

## Scope
- **What**: Implement normative Workflow Engine persistence and state management per ??2.6.1.
- **Why**: Transition from a "minimal" async wrapper to a durable execution engine that satisfies Phase 1 Strategic Audit criteria.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/workflows.rs (Core engine logic & state machine)
  * src/backend/handshake_core/src/storage/mod.rs (Workflow metadata structs)
  * src/backend/handshake_core/src/storage/sqlite.rs (Persistence implementation)
  * src/backend/handshake_core/migrations/0007_workflow_persistence.sql (New tables for node history)
- **OUT_OF_SCOPE**:
  * Visual node-graph UI (Phase 2).
  * Implementation of concrete AI nodes (handled by specific feature WPs).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Core logic for AI execution; breaking change for system reliability and recovery.
- **TEST_PLAN**:
  ```bash
  # Compile check
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  
  # Unit/Integration tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  
  # Final hygiene and traceability
  just post-work WP-1-Workflow-Engine-v2
  ```
- **DONE_MEANS**:
  * ??? [HSK-WF-001] Every node execution and status transition is persisted to SQLite.
  * ??? [HSK-WF-002] Engine can identify `Running` workflows on startup and mark as `Stalled`.
  * ??? `Database` trait updated with methods for node-level persistence.
  * ??? Conformance tests verify that interrupted workflows are detectable after "restart" (DB reload).

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/storage/mod.rs
  * .GOV/roles_shared/SPEC_CURRENT.md (v02.90)
- **SEARCH_TERMS**:
  * "WorkflowRun"
  * "JobState"
  * "HSK-WF-001"
  * "execute_node"
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Concurrent DB lock" -> SQLite access (use transactions for state updates)
  * "State desync" -> Memory vs DB state (always prefer DB as SSoT)
  * "Trait conflict" -> Coder must coordinate with WP-1-SAL-v2 on Database trait changes.

## Authority
- **SPEC_CURRENT**: .GOV/roles_shared/SPEC_CURRENT.md (Master Spec v02.90)
- **SPEC_ANCHOR**: ??2.6.1 [HSK-WF-001], [HSK-WF-002]
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250312

---

### VALIDATION REPORT ??? WP-1-Workflow-Engine-v2 (2025-12-26)
Verdict: PASS ???

**Scope Inputs:**
- Task Packet: `.GOV/task_packets/WP-1-Workflow-Engine-v2.md`
- Spec: `Handshake_Master_Spec_v02.90 ??2.6.1`
- Coder: [[coder gpt codex]]

**Files Checked:**
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/migrations/0007_workflow_persistence.sql`

**Findings:**
- **[HSK-WF-001] Durable Execution Core:** PASS. The engine now persists every node execution and status transition. `WorkflowNodeExecution` struct and associated trait methods enable granular tracking of inputs, outputs, and errors per node. Evidence: `workflows.rs:119-209`.
- **[HSK-WF-002] Crash Recovery:** PASS. Implemented `last_heartbeat` tracking and `Stalled` state logic. On startup, the engine can identify `Running` workflows that have timed out and mark them as `Stalled`. Evidence: `workflows.rs:51-99`, `sqlite.rs:1435-1595`.
- **Database Trait Purity:** PASS. All new methods (`create_workflow_node_execution`, `update_workflow_node_execution_status`, etc.) use domain types (`Uuid`, `JobState`) and return `StorageResult`, maintaining the backend-agnostic abstraction.
- **Postgres Portability:** PASS. `postgres.rs` includes implementation for node executions and stalled detection, ensuring dual-backend readiness.
- **Forbidden Pattern Audit:** PASS. `just validator-scan` returns PASS.
- **Tests:** PASS. Dedicated recovery tests (`stalled_workflows_are_detected_by_heartbeat`) and persistence tests (`workflow_node_execution_persists_inputs_and_outputs`) pass.

**REASON FOR PASS:**
The implementation fulfills the durability and reliability mandates of ??2.6.1. It successfully transitions the Workflow Engine from a "hollow" async wrapper to a persistent execution environment capable of crash recovery and detailed auditability. This completes a critical strategic audit item for Phase 1.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250312




