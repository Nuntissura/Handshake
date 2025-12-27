# Task Packet: WP-1-Workflow-Engine-v3

## Metadata
- TASK_ID: WP-1-Workflow-Engine-v3
- WP_ID: WP-1-Workflow-Engine-v3
- DATE: 2025-12-26T23:37:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
**STATUS:** VALIDATED

---

## RE-AUDIT VALIDATION REPORT (Forensic)
Verdict: PASS

### Evidence Verification (Code Reality)
- `main.rs:51-66`: SATISFIED. Boot-time `tokio::spawn` initiates the mandatory recovery loop.
- `workflows.rs:201-260`: SATISFIED. `mark_stalled_workflows` accepts `is_startup_recovery` flag and identifies `System` actor.

### REASON FOR PASS
Startup recovery logic verified via manual inspection. Audit trails correctly identify system-initiated recovery events.

**STATUS:** HARD-VALIDATED (2025-12-27)

## User Context
We are fixing a reliability gap in the "brain" of the app. Currently, if the app crashes, it only notices and fixes stalled tasks when you start a *new* task. We need the app to automatically scan for and fix any interrupted tasks immediately every time it starts up, so nothing is ever left in a broken state.

## Scope
- **What**: Implement mandatory boot-time crash recovery for interrupted workflows per §2.6.1.
- **Why**: Remediate Strategic Audit failure in WP-1-Workflow-Engine-v2. The engine MUST actively scan for and recover 'Running' workflows on startup, not just opportunistically.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/main.rs (Integration of startup hook)
  * src/backend/handshake_core/src/workflows.rs (Recovery logic enhancement)
- **OUT_OF_SCOPE**:
  * Changes to the Database trait (already supports find_stalled_workflows).
  * UI changes for stalled workflows.

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Modifies the core application startup sequence and system-wide recovery state machine.
- **TEST_PLAN**:
  ```bash
  # 1. Compile check
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  
  # 2. Verify recovery logic unit tests
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_mark_stalled_workflows
  
  # 3. Manual Verification:
  # - Start a workflow
  # - Kill the process while it is 'Running'
  # - Restart the app
  # - Verify in logs/DB that it transitioned to 'Stalled' immediately on boot
  
  # 4. Final hygiene
  just post-work WP-1-Workflow-Engine-v3
  ```
- **DONE_MEANS**:
  * ✅ `main.rs` calls `mark_stalled_workflows` (or equivalent) during initialization.
  * ✅ Recovery loop is non-blocking (does not delay API startup).
  * ✅ Transitions are logged to Flight Recorder with event type `FR-EVT-WF-RECOVERY`.
  * ✅ All existing workflow tests pass.

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * src/backend/handshake_core/src/main.rs (Look for init_storage and run loop)
  * src/backend/handshake_core/src/workflows.rs (See mark_stalled_workflows)
  * docs/SPEC_CURRENT.md (v02.93)
- **SEARCH_TERMS**:
  * "mark_stalled_workflows"
  * "init_storage"
  * "FR-EVT-WF-RECOVERY"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Startup Hang" -> Recovery loop blocks the main thread (Fix: use tokio::spawn or run before server start)
  * "State Race" -> Recovery runs after new jobs start (Fix: execute immediately after storage init)

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.93)
- **SPEC_ANCHOR**: §2.6.1 [HSK-WF-002], [HSK-WF-003]
- **Strategic Pause Approval**: [ilja271220250057]

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220252337