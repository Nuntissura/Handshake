# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Structured-Collaboration-Contract-Hardening-v1

## STUB_METADATA
- WP_ID: WP-1-Structured-Collaboration-Contract-Hardening-v1
- BASE_WP_ID: WP-1-Structured-Collaboration-Contract-Hardening
- CREATED_AT: 2026-03-25T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Structured-Collaboration-Schema-Registry-v4, WP-1-Structured-Collaboration-Artifact-Family-v1, WP-1-Role-Mailbox-v1
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Master-spec closure remediation after `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- AUDIT_DRIVERS:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.178.md 10.11.5.21 Workflow State Families, Queue Reasons, and Governed Actions [ADD v02.171]
  - Handshake_Master_Spec_v02.178.md lines 60470-60472 Dev Command Center governed-action and explicit workflow-projection clauses [ADD v02.170-v02.171]
  - Handshake_Master_Spec_v02.178.md lines 11047-11120 RoleMailbox bounded redaction fields and RoleMailboxExportGate mechanical gate

## INTENT (DRAFT)
- What: Close the remaining Master Spec contract gaps left after `WP-1-Structured-Collaboration-Schema-Registry-v4` by hardening governed action identifiers, authoritative Task Board projection semantics, and RoleMailbox export leak-safety validation in product code.
- Why: The v4 recovery pass fixed the original shallow-schema defects, but the 2026-03-25 smoketest review still found product-code gaps large enough to keep full Master Spec correctness at `FAIL`.

## AUDIT_FINDINGS_THIS_STUB_COVERS (DRAFT)
- `allowed_action_ids` are still emitted as ad hoc UI verbs instead of registered `GovernedActionDescriptorV1.action_id` values.
- Task Board rows still flatten authoritative workflow semantics into board-status heuristics instead of preserving linked `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids`.
- RoleMailbox export validation still treats `subject_redacted` and `note_redacted` as generic non-empty strings rather than mechanically bounded, leak-safe redacted fields.
- Negative-path test proof is still weaker than the Master Spec requires for these contract areas.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Replace ad hoc `allowed_action_ids` emission in canonical Work Packet, Micro-Task, Task Board, and live SQLite-backed progress paths with registry-backed governed action ids.
  - Harden shared validator logic so `allowed_action_ids` prove registry-backed legality instead of only proving "array of strings."
  - Preserve authoritative workflow-state triplets into Task Board rows instead of recomputing them from lane or status heuristics.
  - Harden RoleMailboxExportGate validation for `subject_redacted`, `note_redacted`, and related bounded redaction rules.
  - Add negative-path tests that fail malformed action ids, malformed workflow projection state, and malformed redacted export fields.
- OUT_OF_SCOPE:
  - Broad Loom portability work.
  - Governance workflow-harness remediation; that belongs on the repo-governance board, not this WP.
  - UI redesign beyond the product surfaces needed to prove contract correctness.

## CODE_REALITY_HINTS (DRAFT)
- Path: `src/backend/handshake_core/src/workflows.rs` | Covers: governed action emission + Task Board projection | Notes: currently emits ad hoc action verbs and derives board posture from board status heuristics.
- Path: `src/backend/handshake_core/src/storage/locus_sqlite.rs` | Covers: live SQLite-backed progress metadata | Notes: current path still emits weaker `allowed_action_ids` values and must align or be retired.
- Path: `src/backend/handshake_core/src/locus/types.rs` | Covers: shared validator enforcement | Notes: current validator checks `allowed_action_ids` too shallowly and does not enforce bounded redacted export field semantics.
- Path: `src/backend/handshake_core/src/role_mailbox.rs` | Covers: export manifest and redaction emitter | Notes: emitter behavior is closer to spec than the validator is, but the hard mechanical gate still needs stronger proof.
- Path: `src/backend/handshake_core/tests/role_mailbox_tests.rs` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs` | Covers: proof | Notes: future packet must add negative-path tests for all remaining contract gaps, not only happy-path emission.

## ACCEPTANCE_CRITERIA (DRAFT)
- Every canonical structured-collaboration record family member in scope emits `allowed_action_ids` that resolve to registered `GovernedActionDescriptorV1.action_id` values only.
- All live emitters, including SQLite-backed progress metadata, either share the same governed-action registry path or are removed from canonical output.
- Task Board rows preserve the authoritative `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` from linked backend records instead of inferring them from lane position or board-status heuristics.
- RoleMailboxExportGate fails malformed, unbounded, or non-redacted `subject_redacted` and `note_redacted` values.
- Product tests include negative-path coverage proving the above failures are mechanically blocked.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Signed activation should cite the 2026-03-25 smoketest review as the required evidence driver.
- The packet refinement should decide whether any existing workflow-state registry backlog stub can be partially retired after this narrower remediation lands.

## RISKS / UNKNOWNs (DRAFT)
- Action-id hardening can expose hidden coupling between UI verbs, Task Board quick actions, and backend mutation routes.
- If Task Board projections keep any fallback heuristic path, local-small-model routing may still consume semantically weaker state than the spec allows.
- Mailbox export leak-safety can look "good enough" in happy-path tests while still failing malformed or oversized redaction inputs.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Re-read `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` before narrowing scope.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Structured-Collaboration-Contract-Hardening-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Structured-Collaboration-Contract-Hardening-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
