# Task Packet: WP-1-Workflow-Engine-v4

## METADATA
- TASK_ID: WP-1-Workflow-Engine-v4
- WP_ID: WP-1-Workflow-Engine-v4
- DATE: 2025-12-31T17:47:07.262Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja311220251846

## User Context (Non-Technical Explainer)
The workflow engine must clean up "in progress" jobs after a crash before any new jobs are allowed to start. This packet makes that cleanup happen first and records an audit event so the operator console can explain what happened.

## SCOPE
- What: Enforce HSK-WF-003 ordering so startup recovery completes before accepting any new AI jobs, and emit FR-EVT-WF-RECOVERY with actor system.
- Why: Revalidation found the current startup flow can accept jobs before recovery finishes, which violates the Master Spec and weakens auditability.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
- OUT_OF_SCOPE:
  - New workflow features or state transitions beyond Running -> Stalled recovery
  - Any changes to JobKind or JobState enums
  - Flight Recorder event taxonomy changes beyond FR-EVT-WF-RECOVERY
  - DB schema or migrations
  - UI changes in app/

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Workflow-Engine-v4

# Targeted tests (add or update as needed):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_mark_stalled_workflows
cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_startup_recovery_blocks_job_acceptance

# Full backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Governance and workflow gates:
just validator-scan
just validator-spec-regression
just cargo-clean
just post-work WP-1-Workflow-Engine-v4
```

### DONE_MEANS
- Startup recovery runs before the system accepts new AI jobs (explicit barrier or await in startup path).
- Recovery scan transitions stale Running workflows to Stalled when last_heartbeat > 30s.
- FR-EVT-WF-RECOVERY is emitted for each transition with actor set to system.
- A targeted test proves job acceptance is blocked until recovery completes.
- All existing workflow tests pass.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## NOTES
### Assumptions
- Flight Recorder event type FR-EVT-WF-RECOVERY exists in code and can be emitted without schema changes.

### Open Questions
- None.

### Dependencies
- None. This work relies on existing workflow storage and Flight Recorder wiring.

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.99.md (recorded_at: 2025-12-31T17:47:07.262Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: A2.6.1 [HSK-WF-003] Startup Recovery Loop (Normative); A11.5 FR-EVT-WF-RECOVERY
- Strategic Pause Approval: [ilja311220251846]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.99.md (A2.6.1 HSK-WF-003, A11.5 FR-EVT-WF-RECOVERY)
  - .GOV/roles_shared/ARCHITECTURE.md
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "mark_stalled_workflows"
  - "last_heartbeat"
  - "Stalled"
  - "FR-EVT-WF-RECOVERY"
  - "WorkflowRecovery"
  - "FlightRecorderEventType"
  - "create_job"
  - "enqueue"
  - "job_queue"
  - "accepting new AI jobs"
  - "actor"
  - "run()"
- RUN_COMMANDS:
  ```bash
  rg -n "mark_stalled_workflows|last_heartbeat|Stalled" src/backend/handshake_core/src
  rg -n "FR-EVT-WF-RECOVERY|WorkflowRecovery" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_mark_stalled_workflows
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml workflows::tests::test_startup_recovery_blocks_job_acceptance
  ```
- RISK_MAP:
  - "Recovery runs after job acceptance" -> workflow engine correctness
  - "Recovery blocks startup forever" -> service availability
  - "Missing FR-EVT-WF-RECOVERY event" -> auditability and diagnostics
  - "Actor not system" -> governance compliance
  - "No targeted test" -> regression risk

## SKELETON
- Proposed interfaces/types/contracts:
  - Startup recovery barrier (Notify, oneshot, or AtomicBool) that must be satisfied before job acceptance.
  - Startup recovery function returns completion signal used by the barrier.
  - FR-EVT-WF-RECOVERY emission uses actor system.
- Open questions:
  - None.
- Notes:
  - Keep recovery scan non-blocking for runtime, but block job acceptance until completion.

SKELETON APPROVED

## IMPLEMENTATION
- Added startup recovery gate (`enable_startup_recovery_gate` + `wait_for_startup_recovery`) and enforced it in `jobs::create_job` so job acceptance blocks until startup recovery completes.
- Enforced HSK-WF-003 exclusivity by making `mark_stalled_workflows(..., is_startup_recovery=false)` a no-op (no Running->Stalled outside startup scan).
- Aligned FR-EVT-WF-RECOVERY payload (`FrEvt006WorkflowRecovery`) to Master Spec v02.99 and emitted the event with actor system for each recovered workflow.
- Added targeted test proving job acceptance is blocked until startup recovery completion.

## HYGIENE
- cargo test (targeted): PASS (`workflows::tests::test_mark_stalled_workflows`, `workflows::tests::test_startup_recovery_blocks_job_acceptance`)
- cargo test (full): PASS (`cargo test --manifest-path src/backend/handshake_core/Cargo.toml`)
- just validator-scan: PASS
- just validator-spec-regression: PASS
- just cargo-clean: PASS (Removed 1040 files, 5.0GiB total)
- just post-work WP-1-Workflow-Engine-v4: PASS (warnings: node could not run git-based concurrency/status checks in this environment)

## VALIDATION
- Target File: `src/backend/handshake_core/src/workflows.rs`
- Start: 2
- End: 1274
- Line Delta: 200
- Pre-SHA1: `2ba2e5aac8839b4f9a4292e290fc8c9993cb0e9d`
- Post-SHA1: `b82fd380d4d2a730cca794271ab128c708d616f7`
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
- Lint Results: cargo test PASS; validator-scan PASS
- Artifacts: None
- Timestamp: 2025-12-31
- Operator: codex-cli (Coder) + codex-cli (Validator)
- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.99.md
- Notes: Window/start/end and SHA1 values computed from `git diff`/`git show` vs HEAD for COR-701 post-work-check.

## STATUS_HANDOFF
- Current WP_STATUS: Done
- What changed in this update: Implemented HSK-WF-003 startup recovery ordering + FR-EVT-WF-RECOVERY alignment; validation gates and tests executed; ready for closure.
- Next step / handoff hint: Validator appended PASS report; TASK_BOARD moved to Done (VALIDATED).

---

**Last Updated:** 2025-12-31
**User Signature Locked:** ilja311220251846

IMPORTANT: This packet is locked. No edits allowed.
If changes needed: Create a new packet (WP-1-Workflow-Engine-v5). Do not edit this one.

---

## VALIDATION REPORT - 2025-12-31 (Validator)
Verdict: PASS

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Workflow-Engine-v4.md` (**Status:** Done)
- Spec (SPEC_CURRENT): `.GOV/roles_shared/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.99.md` (`Handshake_Master_Spec_v02.99.md:4932`, `Handshake_Master_Spec_v02.99.md:30925`)
- Codex: `Handshake Codex v1.4.md`
- Protocol: `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

Files Checked:
- `src/backend/handshake_core/src/main.rs`
- `src/backend/handshake_core/src/jobs.rs`
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/src/flight_recorder/mod.rs`
- `.GOV/task_packets/WP-1-Workflow-Engine-v4.md`
- `.GOV/roles_shared/TASK_BOARD.md`

Findings:
- HSK-WF-003 ordering (recovery before job acceptance): job acceptance blocks on startup recovery gate (`src/backend/handshake_core/src/jobs.rs:29`) and startup enables/completes/fails the gate around the recovery scan (`src/backend/handshake_core/src/main.rs:60`, `src/backend/handshake_core/src/main.rs:71`, `src/backend/handshake_core/src/main.rs:75`).
- HSK-WF-003 exclusivity (Running->Stalled only at startup): non-startup invocations are a no-op (`src/backend/handshake_core/src/workflows.rs:316`).
- FR-EVT-WF-RECOVERY payload shape: `FrEvt006WorkflowRecovery` matches the v02.99 fields (`src/backend/handshake_core/src/flight_recorder/mod.rs:274`) and is emitted with actor system during startup recovery transitions (verified by test `workflows::tests::test_mark_stalled_workflows`).

Tests:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`: PASS (all handshake_core tests).

Process Gates:
- `just validator-spec-regression`: PASS
- `just validator-scan`: PASS
- `just cargo-clean`: PASS
- `just post-work WP-1-Workflow-Engine-v4`: PASS (warnings: node could not run git subprocess checks in this environment; manifest SHA1/window/line_delta were computed from `git diff`/`git show` and post-work-check file integrity checks passed).

REASON FOR PASS:
- DONE_MEANS satisfied: recovery ordering is enforced via a startup gate, stale running workflows are transitioned to stalled only during startup recovery, FR-EVT-WF-RECOVERY is emitted with actor system and spec-aligned payload, and tests pass.

