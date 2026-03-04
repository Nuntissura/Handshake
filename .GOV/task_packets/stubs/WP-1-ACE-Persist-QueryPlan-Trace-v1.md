# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-ACE-Persist-QueryPlan-Trace-v1

## STUB_METADATA
- WP_ID: WP-1-ACE-Persist-QueryPlan-Trace-v1
- BASE_WP_ID: WP-1-ACE-Persist-QueryPlan-Trace
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-ACE-Runtime, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (ACE retrieval traceability persistence)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md QueryPlan + RetrievalTrace persistence requirements (deep-linkable artifacts)
  - Handshake_Master_Spec_v02.139.md ContextSnapshot "what did it see and why" auditability

## INTENT (DRAFT)
- What: Persist ACE artifacts (QueryPlan, RetrievalTrace, ContextSnapshot) for doc jobs so retrieval-backed calls are auditable and deep-linkable.
- Why: Current doc job paths leave candidate_list/context_snapshot refs null, which breaks traceability requirements in v02.139.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Persist artifacts:
    - ace_query_plan.json
    - ace_retrieval_trace.json
    - context_snapshot.json
  - Populate the previously-null artifact refs in job records.
  - Add targeted tests asserting the artifacts exist and are linked.
- OUT_OF_SCOPE:
  - New UI surfaces (Operator Consoles can display raw artifacts first).

## ACCEPTANCE_CRITERIA (DRAFT)
- Running DocSummarize/DocEdit produces persisted ACE artifacts and links them in job records.
- A targeted test fails if the artifacts are not persisted/linked.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Artifact storage primitives exist.
- Decide canonical serialization formats and stable artifact path conventions.

## RISKS / UNKNOWNs (DRAFT)
- Privacy: ContextSnapshot must obey minimization and redaction rules; must be leak-safe by default.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-ACE-Persist-QueryPlan-Trace-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-ACE-Persist-QueryPlan-Trace-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
