# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-MT-Handoff-Memory-Context-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-MT-Handoff-Memory-Context-v1
- BASE_WP_ID: WP-1-FEMS-MT-Handoff-Memory-Context
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System, WP-1-Session-Spawn-Contract
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §2.6.6.8.5 MicroTaskExecutorJob — escalation and handoff
  - §2.6.6.7.6.2.3 MemoryPack schema — scope_refs, session binding
  - §4.3.9.12.7 ModelSession FEMS integration — memory_policy per session

## INTENT (DRAFT)
- What: When the MicroTask executor hands work between sessions (escalation to cloud, role handoff, retry with different model), include a typed memory context object — not just the generic MemoryPack but the specific memories relevant to the handoff: what was tried, what failed, what the previous session learned. Ports the Azure/ADK structured handoff pattern from the research.
- Why: Currently, escalation passes the MT prompt + history but not the memory context that informed the previous attempt. The receiving session builds its own MemoryPack from scratch, losing the predecessor's working knowledge. A typed handoff context carries the relevant episodic and procedural items forward, giving the receiving session a head start.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - `MemoryHandoffContext` typed schema: source_session_id, target_session_id, handoff_reason (escalation/retry/role_switch), carried_items (subset of source session's MemoryPack + any INSIGHT checkpoints from the session), failed_attempts (episodic items from this MT's history), recommended_items (procedural items the source session found useful).
  - MT executor populates MemoryHandoffContext at escalation/handoff time.
  - Receiving session merges handoff context into its MemoryPack compilation — handoff items get a trust/scoring boost as "predecessor-recommended".
  - Flight Recorder: FR-EVT-MEM-004 on the receiving session records that handoff items were included, with source_session_id provenance.
  - Locus integration: handoff context linked to MT iteration record for traceability.
- OUT_OF_SCOPE:
  - Cross-WP memory handoff (this is intra-WP, intra-MT only).
  - Automatic memory merge from handoff into LongTermMemory (handoff items are session-scoped unless explicitly promoted).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; defines handoff context as a MemoryPack variant | Stub follow-up: THIS_STUB
  - PILLAR: MicroTask | STATUS: TOUCHED | NOTES: MT executor creates handoff context at escalation/retry | Stub follow-up: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: session spawn includes handoff context in session initialization | Stub follow-up: NONE
  - PILLAR: Locus | STATUS: TOUCHED | NOTES: handoff linked to MT iteration for traceability | Stub follow-up: NONE
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: handoff items carry provenance from source session | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- Escalated MTs include MemoryHandoffContext in the new session initialization.
- Receiving session's MemoryPack includes handoff items with scoring boost.
- Failed attempt history from handoff is visible to the receiving model.
- FR events trace handoff item provenance back to source session.
- Locus MT iteration record links to handoff context artifact.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for MemoryPack schema.
- Depends on: WP-1-Session-Spawn-Contract-v1 for session initialization contract.

## RISKS / UNKNOWNs (DRAFT)
- Risk: handoff context may carry poisoned/wrong memories from a failed session. Trust scoring from the source session's outcome should downweight items from failed runs.
- Risk: handoff context size could exceed target session's memory budget. Truncation by handoff-specific scoring needed.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-MT-Handoff-Memory-Context-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-MT-Handoff-Memory-Context-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
