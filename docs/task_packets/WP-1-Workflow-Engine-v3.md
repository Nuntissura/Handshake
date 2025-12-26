# Task Packet: WP-1-Workflow-Engine-v3

## Metadata
- TASK_ID: WP-1-Workflow-Engine-v3
- WP_ID: WP-1-Workflow-Engine-v3
- DATE: 2025-12-26T23:37:00Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: Done

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

## Notes
- **Assumptions**: None.
- **Open Questions**: None.
- **Dependencies**: Foundational.

---

# BOOTSTRAP
- Verified [HSK-WF-002] requirements.
- Identified main.rs and workflows.rs as primary targets.

# SKELETON
- mark_stalled_workflows(..., is_startup_recovery: bool)
- FR-EVT-WF-RECOVERY event emission.
- Startup hook in main.rs via tokio::spawn.

SKELETON APPROVED [ilja261220252345]

---

**Last Updated:** 2025-12-26

---

## VALIDATION REPORT (APPENDED per CX-WP-001)

**Validator:** Senior Red Hat Auditor
**Date:** 2025-12-26
**Verdict:** PASS

### Evidence Mapping (Spec → Code)

| Requirement | Evidence |
|-------------|----------|
| [HSK-WF-002] Startup Recovery Loop | `main.rs:51-66` - Boot-time tokio::spawn scan |
| Non-Blocking Execution | `main.rs:51` - Async spawn avoids server delay |
| [FR-EVT-WF-RECOVERY] Emission | `workflows.rs:302-322` - FR-EVT-WF-RECOVERY with System actor |
| Workflow Stabilization | `workflows.rs:275-300` - Transition to Stalled state |

### Tests Executed

| Command | Result |
|---------|--------|
| `cargo test workflows::tests::test_mark_stalled_workflows` | PASS |
| `cargo check` | PASS |

### REASON FOR PASS

Interrupted workflow recovery is correctly implemented as a mandatory boot-time service. Audit trails are maintained via Flight Recorder. Happy path verified by unit test.

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md (Master Spec v02.93)
- **SPEC_ANCHOR**: §2.6.1 [HSK-WF-002]
- **Strategic Pause Approval**: [ilja261220252337]

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220252337

## VALIDATION [CX-623]
========================================
Command: cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_mark_stalled_workflows
Result: ✅ PASS (1 passed, 0 failed)
Output: test workflows::tests::test_mark_stalled_workflows ... ok

Command: cargo test --manifest-path src/backend/handshake_core/Cargo.toml
Result: ✅ PASS (111 passed, 0 failed)

DONE_MEANS VERIFICATION [CX-625A]
============================================
* ✅ main.rs calls mark_stalled_workflows during initialization. (src/backend/handshake_core/src/main.rs:55-73)
* ✅ Recovery loop is non-blocking (tokio::spawn used). (src/backend/handshake_core/src/main.rs:60)
* ✅ Transitions are logged to Flight Recorder with event type FR-EVT-WF-RECOVERY. (src/backend/handshake_core/src/workflows.rs:231-242)
* ✅ All existing workflow tests pass. (111/111 passed)

EVIDENCE_MAPPING [CX-627]
============================================
* [HSK-WF-002] Crash Recovery State Machine: workflows.rs:200-250 (mark_stalled_workflows)
* [HSK-WF-003] Startup Recovery Loop: main.rs:55-73 (Integration in run loop)
========================================

