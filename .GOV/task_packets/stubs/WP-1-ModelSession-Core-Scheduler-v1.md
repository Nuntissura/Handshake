# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-ModelSession-Core-Scheduler-v1

## STUB_METADATA
- WP_ID: WP-1-ModelSession-Core-Scheduler-v1
- BASE_WP_ID: WP-1-ModelSession-Core-Scheduler
- CREATED_AT: 2026-02-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> items 28-29 (ModelSession + Scheduler + persistence layer)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.13 Session Scheduler: Model Calls as Queued Work (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 7.2.0.5 (updated to normative in v02.137 note; workflow/job alignment)

## INTENT (DRAFT)
- What: Introduce ModelSession as the persisted unit of multi-turn orchestration and implement the Phase 1 Session Scheduler baseline (`model_run` job_kind) with queueing, cancellation, and concurrency limits.
- Why: Without a first-class session data model + scheduler, parallel and multi-turn execution becomes completion-only, non-auditable, and unsafe to steer/recover.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - ModelSession schema and persistence (workspace-local DB):
    - ModelSession fields, lifecycle state machine, and session/thread storage (artifact-first message content with content hashes).
    - Bindings for wp_id/mt_id/work_profile/execution_mode and memory policy.
  - Session Scheduler:
    - `model_run` job_kind with deterministic queueing, dispatch, cancellation, concurrency groups/lanes, and rate limiting.
    - Enforce the scheduling invariants (no direct completion calls bypassing scheduler in production paths).
  - Minimal surfacing:
    - Session state/queued state visible at least via Job History / Flight Recorder hooks (DCC panel is tracked separately).
- OUT_OF_SCOPE:
  - Spawn lifecycle (tracked in WP-1-Session-Spawn-Contract-v1).
  - Session-scoped capability intersection / consent gate (tracked in WP-1-Session-Scoped-Capabilities-Consent-Gate-v1).
  - Provider feature coverage (tracked in WP-1-Provider-Feature-Coverage-Agentic-Ready-v1).
  - Workspace safety boundaries (tracked in WP-1-Workspace-Safety-Parallel-Sessions-v1).
  - Crash recovery/checkpointing (tracked in WP-1-Session-Crash-Recovery-Checkpointing-v1).
  - Observability spans + FR event families (tracked in WP-1-Session-Observability-Spans-FR-v1).

## ACCEPTANCE_CRITERIA (DRAFT)
- A ModelSession can be created, persisted, and reloaded with a durable message thread where message content is stored as artifacts (hash-only in logs/events).
- `model_run` jobs are scheduled via the Session Scheduler; queueing and cancellation behave deterministically and are visible in system telemetry.
- Concurrency limits are enforced: dispatch does not exceed configured limits; queued jobs are not dropped.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: AI Job Model + Workflow Engine primitives (existing Phase 1 foundations).
- Coordinates with: DCC sessions panel (WP-1-Dev-Command-Center-MVP-v1) for UI surfacing; Flight Recorder updates for session event families.

## RISKS / UNKNOWNs (DRAFT)
- Risk: mixing “session state” and “job state” leads to drift; ensure explicit mapping.
- Risk: storing message content inline breaks redaction/export rules; must be artifact-first.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-ModelSession-Core-Scheduler-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-ModelSession-Core-Scheduler-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
