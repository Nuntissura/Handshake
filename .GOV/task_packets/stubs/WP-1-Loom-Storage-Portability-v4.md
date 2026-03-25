# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Loom-Storage-Portability-v4

## STUB_METADATA
- WP_ID: WP-1-Loom-Storage-Portability-v4
- BASE_WP_ID: WP-1-Loom-Storage-Portability
- CREATED_AT: 2026-03-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Loom-MVP, WP-1-Storage-Abstraction-Layer, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Governance remediation after AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture
  - Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom storage trait, graph traversal, metrics, and source-anchor portability
  - Handshake_Master_Spec_v02.178.md Loom API and portability acceptance clauses cited by the v3 packet

## INTENT (DRAFT)
- What: Re-open the Loom portability work as a narrowly scoped remediation/proof pass that separates real portability evidence from narrative closure.
- Why: The 2026-03-21 audit did not find a fresh Loom defect comparable to the Schema Registry gaps, but it also did not re-prove full portability closure, and the v3 packet remains blocked legacy history under current governance law.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Re-validate the exact Loom portability clauses still required by the current spec and by any future remediation refinement.
  - Narrow the remediation to concrete proof gaps or fresh defects found on current `main`; avoid speculative churn if the audited Loom slice is already materially correct.
  - Preserve dual-backend evidence for graph traversal, directional edges, metrics recomputation, and source-anchor durability where those clauses remain in scope.
- OUT_OF_SCOPE:
  - Broad Loom feature expansion unrelated to storage portability.
  - Media bridge or archive integration work owned by downstream Loom packets.

## ACCEPTANCE_CRITERIA (DRAFT)
- Any remaining Loom portability remediation is tied to explicit current-spec clauses and current-main evidence, not to the old v3 packet narrative alone.
- If a concrete Loom defect exists, the remediation packet proves it with validator-owned checks on both supported backends.
- If no concrete Loom defect remains, the future refinement must say so explicitly and reduce scope rather than inventing new churn.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Governance workflow-proof run should complete first so this remediation does not double as live workflow experimentation.
- Future refinement must decide whether the next action is code remediation, proof-only closure, or explicit outdated-only archival handling.

## RISKS / UNKNOWNs (DRAFT)
- The v3 packet mixed concrete storage work with broader closure claims; refinement must narrow the next packet to what is still actually open.
- Dual-backend proof can become expensive if the next remediation broadens beyond the audited slice.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Loom-Storage-Portability-v4.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Loom-Storage-Portability-v4` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
