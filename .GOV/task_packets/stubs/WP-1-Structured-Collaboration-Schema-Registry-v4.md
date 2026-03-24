# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Structured-Collaboration-Schema-Registry-v4

## STUB_METADATA
- WP_ID: WP-1-Structured-Collaboration-Schema-Registry-v4
- BASE_WP_ID: WP-1-Structured-Collaboration-Schema-Registry
- CREATED_AT: 2026-03-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Project-Profile-Extension-Registry, WP-1-Markdown-Mirror-Sync-Drift-Guard, WP-1-Dev-Command-Center-Structured-Artifact-Viewer
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Governance remediation after AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.178.md shared structured-collaboration envelope and validator requirements
  - Handshake_Master_Spec_v02.178.md Task Board projection contract and workflow-state fields
  - Handshake_Master_Spec_v02.178.md Role Mailbox export contract and structured nested payload rules

## INTENT (DRAFT)
- What: Re-open the Schema Registry work as a remediation pass that hardens the shared validator and the negative-path proof surface instead of trusting the v3 happy-path closure.
- Why: The 2026-03-21 audit found that v3 improved emitted artifacts but did not fully enforce required workflow fields, nested payload structure, or constrained string formats under the current Master Spec.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Enforce `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on canonical Work Packet, Micro-Task, and Task Board records.
  - Add nested validation for Task Board `rows[]`, Role Mailbox `threads[]`, and `transcription_links[]`.
  - Add validator checks for constrained string contracts where the spec treats fields as typed values rather than generic strings.
  - Add validator-owned negative-path regression tests for the missing enforcement paths.
- OUT_OF_SCOPE:
  - Broad Dev Command Center UI expansion.
  - Unrelated structured-collaboration product work outside the shared validator/export contract.

## ACCEPTANCE_CRITERIA (DRAFT)
- The shared validator rejects missing required workflow-state fields on Work Packet, Micro-Task, and Task Board records.
- Nested Task Board and Role Mailbox payloads are validated per item, not only by outer array presence.
- Validator-owned negative-path tests fail when the newly required fields or nested contracts are removed or malformed.
- Closure is based on adversarial proof, not only on emitted happy-path artifacts.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Governance workflow-proof run should complete first so this remediation does not double as live workflow experimentation.
- Shared structured-collaboration surfaces remain high-risk and require validator-owned review evidence.

## RISKS / UNKNOWNs (DRAFT)
- Changes touch shared validators and may expose previously hidden drift in downstream artifact readers.
- The exact constrained-string checks should be narrowed during refinement so the remediation stays diff-scoped and auditable.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Structured-Collaboration-Schema-Registry-v4.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Structured-Collaboration-Schema-Registry-v4` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
