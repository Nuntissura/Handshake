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

## VALIDATION REPORT — 2025-12-27 (Revalidation)
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Workflow-Engine-v3.md (STATUS: VALIDATED)
- Spec: Handshake_Master_Spec_v02.93 (A2.6.1 [HSK-WF-002], [HSK-WF-003]) via docs/SPEC_CURRENT.md
- Codex: Handshake Codex v1.4.md

Files Checked:
- src/backend/handshake_core/src/main.rs:54-80 (boot-time startup recovery spawn before server start)
- src/backend/handshake_core/src/workflows.rs:200-273 (mark_stalled_workflows transitions and FR-EVT-WorkflowRecovery emission)
- src/backend/handshake_core/src/workflows.rs:865-915 (test_mark_stalled_workflows)

Findings:
- Startup recovery runs non-blocking before API start and marks stale workflows/jobs as Stalled with workflow recovery events.
- Forbidden Pattern Audit [CX-573E]: PASS for in-scope files (no unwrap/expect/todo!/panic!/split_whitespace in production paths).
- Zero Placeholder Policy [CX-573D]: PASS; recovery logic is fully implemented (no stubs).
- Spec alignment [CX-598]: Behaviour matches A2.6.1 and emits traceable recovery telemetry.

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_mark_stalled_workflows` (PASS)

REASON FOR PASS: Evidence confirms the startup recovery loop and supporting telemetry satisfy DONE_MEANS, and the targeted test from the TEST_PLAN passes.

---

## VALIDATION REPORT - 2025-12-30 (Revalidation, Batch 5)
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Workflow-Engine-v3.md
- Spec (SPEC_CURRENT): docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md
- Protocol: docs/VALIDATOR_PROTOCOL.md

Commands Run:
- just validator-spec-regression: PASS
- just cargo-clean: PASS (Removed 0 files)
- just gate-check WP-1-Workflow-Engine-v3: FAIL (Implementation detected without SKELETON APPROVED marker)
- node scripts/validation/post-work-check.mjs WP-1-Workflow-Engine-v3: FAIL (non-ASCII + missing COR-701 validation manifest fields/gates)
- just validator-packet-complete WP-1-Workflow-Engine-v3: FAIL (STATUS missing/invalid; requires canonical **Status:** Ready for Dev / In Progress / Done)
- just post-work WP-1-Workflow-Engine-v3: FAIL (blocked at gate-check)

Blocking Findings:
- Phase gate violation [CX-GATE-001]: gate-check fails because implementation is present without a prior "SKELETON APPROVED" marker in this packet.
- Deterministic manifest gate (COR-701): post-work-check reports missing required manifest fields (target_file, start, end, pre_sha1, post_sha1, line_delta) and missing/unchecked gates (C701-G01, C701-G02, C701-G04, C701-G05, C701-G06, C701-G08).
- ASCII-only requirement: post-work-check reports non-ASCII characters in the task packet.
  - NON_ASCII_COUNT=7 (sample: Line 30 Col 88 U+00A7, Line 60 Col 5 U+2705, Line 97 Col 22 U+2014)
- Spec mismatch: this packet asserts Master Spec v02.93, but docs/SPEC_CURRENT.md points to v02.98. Prior PASS claims are not valid against the current spec.

Spec-to-Code Findings (v02.98):
- [HSK-WF-003] Startup Recovery Loop requirement includes: "This recovery MUST occur before the system begins accepting new AI jobs." (Handshake_Master_Spec_v02.98.md:4932).
  - Current code spawns the recovery scan and then starts the server without awaiting recovery completion (src/backend/handshake_core/src/main.rs:60-99). This does not prove the required ordering.

Manual Spot-Checks (evidence only; does not override the failures above):
- Recovery scan is initiated with threshold 30s + startup flag: src/backend/handshake_core/src/main.rs:60-75 and src/backend/handshake_core/src/workflows.rs:208-266.
- Startup recovery emits WorkflowRecovery as System: src/backend/handshake_core/src/workflows.rs:241-262.

REASON FOR FAIL:
- Blocking process gates (phase gate + COR-701 manifest + ASCII-only + STATUS marker) fail; spec alignment to v02.98 is not demonstrated.
- Additionally, [HSK-WF-003] ordering ("before accepting new AI jobs") is not proven by the current startup flow.

Required Fixes:
1) Bring this packet back into protocol: include proper BOOTSTRAP/SKELETON/IMPLEMENTATION/HYGIENE/VALIDATION sections and obtain explicit "SKELETON APPROVED" before implementation evidence.
2) Make the task packet ASCII-only (remove/replace non-ASCII characters; rerun post-work-check until clean).
3) Add a COR-701 validation manifest (target_file/start/end/pre_sha1/post_sha1/line_delta + gates checklist) and ensure `just post-work WP-1-Workflow-Engine-v3` passes.
4) Re-anchor DONE_MEANS + evidence mapping to Handshake_Master_Spec_v02.98.md.
5) Address [HSK-WF-003] ordering by adding an explicit barrier so startup recovery completes (or is otherwise enforced) before new AI jobs are accepted; revalidate with evidence.

**Status:** Ready for Dev

Addendum (2025-12-30):
- The canonical **Status:** line above addresses the earlier status-marker failure, but packet completeness still fails because the required user signature field is missing.


