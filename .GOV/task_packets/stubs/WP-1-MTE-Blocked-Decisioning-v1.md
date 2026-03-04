# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-MTE-Blocked-Decisioning-v1

## STUB_METADATA
- WP_ID: WP-1-MTE-Blocked-Decisioning-v1
- BASE_WP_ID: WP-1-MTE-Blocked-Decisioning
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (blocked handling decision tree)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Micro-Task Executor: <blocked> handling and recoverable retry logic

## INTENT (DRAFT)
- What: Implement spec-aligned blocked decisioning (retry vs gate vs escalate) for <blocked> signals and add conformance tests.
- Why: Current behavior marks blocked then escalates anyway, which diverges from spec intent for recoverable blocks.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add classification for blocked reasons (recoverable vs non-recoverable).
  - Implement retry logic for recoverable blocks within bounded caps.
  - Add tests for blocked cases (recoverable retries and non-recoverable escalation).
- OUT_OF_SCOPE:
  - New UI surfaces for blocked remediation (Operator Consoles can show event evidence only initially).

## ACCEPTANCE_CRITERIA (DRAFT)
- Recoverable blocks retry deterministically without escalating immediately.
- Non-recoverable blocks produce a clear gate/escalation with explicit evidence and error codes.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Requires a spec-approved list of canonical blocked reasons or an allowlist-driven classifier.

## RISKS / UNKNOWNs (DRAFT)
- If blocked reasons are free-form, classification may become ambiguous; refinement must pin the contract.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-MTE-Blocked-Decisioning-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-MTE-Blocked-Decisioning-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

