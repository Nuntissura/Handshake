# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: {{WP_ID}}

## STUB_METADATA
- WP_ID: {{WP_ID}}
- BASE_WP_ID: {{WP_ID}} (stable ID without `-vN`; equals WP_ID for stubs; if WP_ID includes `-vN`, override to the base ID)
- CREATED_AT: {{DATE_ISO}}
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: {{ROADMAP_POINTER}}
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - {{SPEC_ANCHOR_1}}
  - {{SPEC_ANCHOR_2}}

## INTENT (DRAFT)
- What:
- Why:

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - ...
- OUT_OF_SCOPE:
  - ...

## ACCEPTANCE_CRITERIA (DRAFT)
- ...

## DEPENDENCIES / BLOCKERS (DRAFT)
- ...

## RISKS / UNKNOWNs (DRAFT)
- ...

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/{{WP_ID}}.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet {{WP_ID}}` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


