# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FR-ModelSessionId-v1

## STUB_METADATA
- WP_ID: WP-1-FR-ModelSessionId-v1
- BASE_WP_ID: WP-1-FR-ModelSessionId
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (FR model_session_id correlation)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Flight Recorder event base envelope must include model_session_id
  - Handshake_Master_Spec_v02.139.md Observability correlation rules (trace_id + tool_call_id + session)

## INTENT (DRAFT)
- What: Add model_session_id to Flight Recorder event base envelope and persistence sinks (DuckDB) and populate it for session-scoped operations.
- Why: Without model_session_id, cross-event correlation is incomplete and violates v02.139 observability requirements.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Extend the event struct/envelope to carry model_session_id.
  - Extend DuckDB schema and indexes.
  - Populate model_session_id for session-scoped emitters (tool calls, MT executor, spec router, etc).
  - Add targeted tests/assertions for presence.
- OUT_OF_SCOPE:
  - Rewriting Flight Recorder storage engine (use existing system).

## ACCEPTANCE_CRITERIA (DRAFT)
- Session-scoped events persist with model_session_id and can be queried by session.
- A targeted test fails if model_session_id is missing for a session-scoped event.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Requires a consistent definition of "session-scoped event" in the codebase and/or validator.

## RISKS / UNKNOWNs (DRAFT)
- Adding a required field can break older artifacts; requires migration and backward compatibility strategy.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FR-ModelSessionId-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-FR-ModelSessionId-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

