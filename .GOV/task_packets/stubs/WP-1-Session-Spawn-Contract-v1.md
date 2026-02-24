# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Session-Spawn-Contract-v1

## STUB_METADATA
- WP_ID: WP-1-Session-Spawn-Contract-v1
- BASE_WP_ID: WP-1-Session-Spawn-Contract
- CREATED_AT: 2026-02-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> item 30 (Session spawn contract / OpenClaw pattern)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.15 Session Spawn Contract and Lifecycle (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.12 ModelSession (parent_session_id/spawn_depth)
  - Handshake_Master_Spec_v02.137.md 4.3.9.17 Workspace Safety Boundaries (tool narrowing for children)
  - Handshake_Master_Spec_v02.137.md 4.3.9.20 Inbound Trust Boundary Rules (TRUST-003 narrowing)
  - Handshake_Master_Spec_v02.137.md 11.5 Flight Recorder events (FR-EVT-SESS-SPAWN-*)

## INTENT (DRAFT)
- What: Implement the session spawn lifecycle (SessionSpawnRequest/Response), enforce hard depth + active-children caps, and provide announce-back via Role Mailbox with summary artifacts.
- Why: Prevent runaway delegation storms and make sub-session work auditable, bounded, and safely mergeable back into primary flows.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Spawn request/response contract and validation (depth limits, per-session spawn caps, role assignment).
  - Spawn lane isolation semantics (primary vs subagent/background) and cascade cancel behavior.
  - Announce-back and status reporting:
    - summary artifacts and mailbox message correlation.
  - Flight Recorder event coverage for spawn events (family FR-EVT-SESS-SPAWN-*).
- OUT_OF_SCOPE:
  - Provider tool calling/streaming capabilities (tracked in WP-1-Provider-Feature-Coverage-Agentic-Ready-v1).
  - Workspace/file isolation mechanics (tracked in WP-1-Workspace-Safety-Parallel-Sessions-v1).

## ACCEPTANCE_CRITERIA (DRAFT)
- Spawn depth and max-children caps are enforced (hard block with explicit reason and Flight Recorder evidence).
- A child session can run, produce a summary artifact, and announce-back to the requester via Role Mailbox with deterministic correlation.
- Cascade cancel cancels children deterministically and records the set of cancelled session_ids.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: ModelSession persistence + scheduler baseline (WP-1-ModelSession-Core-Scheduler-v1).
- Depends on: Role Mailbox primitives (existing role mailbox WPs) for announce-back.

## RISKS / UNKNOWNs (DRAFT)
- Risk: spawn without strict tool narrowing becomes a remote action pipeline; must integrate with session-scoped capability intersection and tool narrowing rules.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Session-Spawn-Contract-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Session-Spawn-Contract-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

