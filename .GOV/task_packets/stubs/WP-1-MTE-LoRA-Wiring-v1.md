# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-MTE-LoRA-Wiring-v1

## STUB_METADATA
- WP_ID: WP-1-MTE-LoRA-Wiring-v1
- BASE_WP_ID: WP-1-MTE-LoRA-Wiring
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (LoRA selection must be effective)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Micro-Task Executor: LoRA selection by task_tags
  - Handshake_Master_Spec_v02.139.md Provider request envelope fields (model + adapters)

## INTENT (DRAFT)
- What: Make LoRA selection actually affect the provider request path by adding an optional lora_id field and plumbing it end-to-end.
- Why: Current code tracks lora_id in state but the LLM request has no LoRA field, making selection a no-op and violating spec intent.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Extend the LLM completion request envelope with optional lora_id (or equivalent adapter identifier).
  - Plumb through provider clients and logs (leak-safe).
  - Add targeted tests asserting the selected LoRA is included in requests (or mocked provider receives it).
- OUT_OF_SCOPE:
  - Training/build automation and LoRA lifecycle management (separate track).

## ACCEPTANCE_CRITERIA (DRAFT)
- Selecting a LoRA at runtime results in a request annotated with that LoRA.
- A targeted test fails if lora_id is dropped before reaching the client boundary.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Provider abstraction must support passing adapter metadata (may require minimal provider surface expansion).

## RISKS / UNKNOWNs (DRAFT)
- Provider feature parity: some providers may not support LoRA; must be explicit and test-gated.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-MTE-LoRA-Wiring-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-MTE-LoRA-Wiring-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

