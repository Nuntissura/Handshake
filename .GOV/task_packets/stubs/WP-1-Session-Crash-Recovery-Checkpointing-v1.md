# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Session-Crash-Recovery-Checkpointing-v1

## STUB_METADATA
- WP_ID: WP-1-Session-Crash-Recovery-Checkpointing-v1
- BASE_WP_ID: WP-1-Session-Crash-Recovery-Checkpointing
- CREATED_AT: 2026-02-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> item 34 (crash recovery / resume)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.19 Session Crash Recovery and Checkpointing (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.13 Session Scheduler (cancellation boundaries; idempotency)
  - Handshake_Master_Spec_v02.137.md 4.3.9.12 ModelSession (checkpoint_artifact_id fields)

## INTENT (DRAFT)
- What: Add checkpointing and idempotent recovery/resume for interrupted `model_run` jobs and sessions, with deterministic failure modes and explicit evidence.
- Why: Session state loss on crash is explicitly listed as an incident-predicting anti-pattern; without checkpointing, multi-session orchestration becomes fragile and unsafe.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Session checkpoint artifacts:
    - checkpoint format and storage as artifacts; last-checkpoint fields persisted in ModelSession.
  - Recovery flow:
    - detect interrupted runs, resume deterministically when allowed, or fail with explicit state and recovery guidance.
    - idempotent recovery to prevent partial side effects / duplicate actions.
  - Scheduler integration:
    - cooperative cancellation boundaries and safe resume points (between streaming chunks / tool boundaries).
- OUT_OF_SCOPE:
  - Deterministic replay for high-stakes audit beyond Phase 1 baseline (spec mentions advanced replay later).

## ACCEPTANCE_CRITERIA (DRAFT)
- Crash mid-stream or mid-tool boundary produces a recoverable session with a checkpoint artifact; restart resumes or blocks deterministically with explicit reason.
- Recovery does not duplicate side effects; idempotency is enforced via scheduler/tool gate boundaries.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: ModelSession persistence + scheduler baseline (WP-1-ModelSession-Core-Scheduler-v1).
- Coordinates with: Tool Gate idempotency key enforcement (Unified Tool Surface Contract).

## RISKS / UNKNOWNs (DRAFT)
- Risk: checkpoint content may leak secrets if not redacted/classified; must remain artifact-first and policy-safe.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Session-Crash-Recovery-Checkpointing-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Session-Crash-Recovery-Checkpointing-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

