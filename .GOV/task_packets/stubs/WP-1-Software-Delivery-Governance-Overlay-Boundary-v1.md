# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
- For any stub covering current-spec Phase 1 roadmap additions (`[ADD v<current>]`), `ROADMAP_ADD_COVERAGE` MUST enumerate the exact spec line numbers so governance checks can verify no additions were missed.

---

# Work Packet Stub: WP-1-Software-Delivery-Governance-Overlay-Boundary-v1

## STUB_METADATA
- WP_ID: WP-1-Software-Delivery-Governance-Overlay-Boundary-v1
- BASE_WP_ID: WP-1-Software-Delivery-Governance-Overlay-Boundary
- CREATED_AT: 2026-04-17T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Artifact-Registry, WP-1-Product-Governance-Check-Runner, WP-1-Governance-Pack
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.181.md 7.6.3 (Phase 1) -> [ADD v02.181] Software-delivery governance overlay boundary
- ROADMAP_ADD_COVERAGE: SPEC=v02.181; PHASE=7.6.3; LINES=47940,48043
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.181.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
  - Handshake_Master_Spec_v02.181.md 7.5.4.9 Governance Check Runner: Bounded Execution Contract (HARD) [ADD v02.180]
  - Handshake_Master_Spec_v02.181.md 10.11 Dev Command Center (Sidecar Integration)

## INTENT (DRAFT)
- What: Define and later implement the boundary that keeps repository `/.GOV/**` artifacts as imported software-delivery overlay source material or evidence while product-owned runtime records and workflow-backed governed actions remain the live authority.
- Why: The new software-delivery overlay law in `v02.181` is explicit that repo governance material may be imported, projected, exported, and evidenced, but it may not silently become the operational source of truth inside Handshake runtime.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Imported-overlay boundary rules for repository `/.GOV/**` artifacts used by software-delivery governance flows.
  - Canonical distinction between:
    - imported overlay source material
    - imported overlay evidence
    - product-owned runtime authority
  - Governance Pack import/export behavior that preserves imported overlay artifacts without bypassing workflow-backed runtime law.
  - Projection and audit rules that let Dev Command Center and related surfaces show imported overlay posture without elevating repo paths into live authority.
- OUT_OF_SCOPE:
  - Executing imported governance checks or scripts.
  - Replacing Handshake-native governance with repository files.
  - Generalized non-software governance-pack work outside the software-delivery overlay boundary.

## ACCEPTANCE_CRITERIA (DRAFT)
- At least one workflow-backed software-delivery flow preserves repository `/.GOV/**` artifacts as imported overlay source material or evidence instead of live runtime truth.
- Governance Pack import/export preserves those overlay artifacts without allowing them to bypass product-owned workflow and gate law.
- Runtime and UI projections can explain which overlay artifacts were imported and why, while still treating product-owned records and governed actions as operational authority.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the imported-governance artifact registry so overlay inputs are typed and provenance-linked.
- Depends on the governed check runner and Governance Pack work so imported overlay artifacts can be carried and evidenced without becoming a raw shell-path authority surface.

## RISKS / UNKNOWNs (DRAFT)
- Risk: imported repo artifacts leak back into product runtime as de facto authority because a projection or export path reuses repo state directly.
- Risk: the software-delivery overlay flattens broader Handshake governance layers if the boundary is defined as a one-way import rather than an additive profile-specific overlay.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Software-Delivery-Governance-Overlay-Boundary-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Software-Delivery-Governance-Overlay-Boundary-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
