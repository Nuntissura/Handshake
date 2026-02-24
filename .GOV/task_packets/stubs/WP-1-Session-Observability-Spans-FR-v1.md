# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Session-Observability-Spans-FR-v1

## STUB_METADATA
- WP_ID: WP-1-Session-Observability-Spans-FR-v1
- BASE_WP_ID: WP-1-Session-Observability-Spans-FR
- CREATED_AT: 2026-02-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> items 36-37 (FR-EVT-SESS* + model_session_id correlation + ModelSessionSpan/ActivitySpan binding)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.18 Session Observability: ActivitySpan and ModelSessionSpan Binding (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.13.5 Flight Recorder Events (FR-EVT-SESS-SCHED-*) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.15.* Spawn FR events (FR-EVT-SESS-SPAWN-*) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 11.5 Flight Recorder Event Shapes & Retention (FR-EVT-SESS-* registry + model_session_id correlation rule)
  - Handshake_Master_Spec_v02.137.md 11.9.1.X Session-Scoped Observability Requirements [ADD v02.137]

## INTENT (DRAFT)
- What: Implement session-scoped observability for multi-session execution: new FR event families, base schema correlation via model_session_id, and span bindings (ModelSessionSpan + ActivitySpan).
- Why: Multi-session orchestration without strong observability becomes un-debuggable and unsafe; the spec requires session-wide queries to work via model_session_id even without spans.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Flight Recorder:
    - Register and emit FR-EVT-SESS-*, FR-EVT-SESS-SCHED-*, FR-EVT-SESS-SPAWN-*.
    - Update FlightRecorderEventBase to support `model_session_id` correlation and enforce rules.
    - Schema validator must include these event families (unknown IDs rejected).
  - Spans:
    - Create/close ModelSessionSpan for each ModelSession lifecycle.
    - Bind model_run + tool calls to ActivitySpan and ModelSessionSpan as required.
  - Queryability:
    - Session-wide queries work via model_session_id even without spans.
- OUT_OF_SCOPE:
  - Advanced deterministic replay/audit beyond Phase 1 baseline.

## ACCEPTANCE_CRITERIA (DRAFT)
- Starting and completing a ModelSession produces FR-EVT-SESS-001..005 events and corresponding spans.
- Session scheduler enqueue/dispatch/cancel events are emitted with correct correlation fields.
- Spawn events are emitted and link requester/child session_ids with summary artifact linkage.
- Schema validator rejects unknown session event IDs and enforces model_session_id correlation rules.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Flight Recorder schema registry/validation infrastructure and the existing observability substrate.
- Depends on: ModelSession + scheduler baseline (WP-1-ModelSession-Core-Scheduler-v1).

## RISKS / UNKNOWNs (DRAFT)
- Risk: inconsistent correlation keys across event families breaks operator queries; model_session_id must be the primary key when applicable.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Session-Observability-Spans-FR-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Session-Observability-Spans-FR-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

