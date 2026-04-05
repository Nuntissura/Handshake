# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Dev-Command-Center-Control-Plane-Backend-v1

## STUB_METADATA
- WP_ID: WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- BASE_WP_ID: WP-1-Dev-Command-Center-Control-Plane-Backend
- CREATED_AT: 2026-04-05T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Governance-Workflow-Mirror, WP-1-Session-Spawn-Contract, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Session-Crash-Recovery-Checkpointing, WP-1-Session-Observability-Spans-FR, WP-1-Locus-Phase1-QueryContract-Autosync, WP-1-Role-Mailbox, WP-1-Workflow-Projection-Correlation, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-MVP, WP-1-Dev-Command-Center-Layout-Projection-Registry, WP-1-Dev-Command-Center-Structured-Artifact-Viewer, WP-1-Consent-Audit-Projection
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Backend-first self-hosting split for Dev Command Center control-plane completion
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)
  - Handshake_Master_Spec_v02.179.md [ADD v02.159] through [ADD v02.165] Dev Command Center backend projection passes
  - Handshake_Master_Spec_v02.179.md [ADD v02.166] through [ADD v02.172] structured-collaboration, workflow-state, and transition projection rules
  - Handshake_Master_Spec_v02.179.md 10.1 Terminal Experience

## INTENT (DRAFT)
- What: Build the backend projection and API layer that turns existing runtime systems into one Dev Command Center control plane inside Handshake.
- Why: Handshake already has major backend pieces, but they are not yet unified as one product-side control plane for work packets, micro-tasks, sessions, approvals, governance overlay state, role-mailbox coordination, VCS posture, and recorder-linked evidence. Without this layer, DCC UI work would be another thin shell over disconnected APIs.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Backend projections and APIs for:
    - Work Packets
    - Micro-Tasks
    - Task Board freshness and ready-query state
    - session scheduler state
    - approval and capability state
    - role-mailbox coordination state
    - governance overlay execution state
    - VCS/worktree binding state
    - recorder/evidence joins
  - Stable projection identifiers and correlation contracts.
  - Compact-summary-first control-plane payloads suitable for local-small-model routing and operator views.
  - No-bypass rule: DCC backend remains a projection/steering layer over authoritative backend artifacts, not a second authority.
- OUT_OF_SCOPE:
  - Full DCC frontend shell.
  - Monaco editor shell and terminal shell polish.
  - New repo-governance files becoming runtime authority.

## ACCEPTANCE_CRITERIA (DRAFT)
- Product backend exposes one coherent control-plane API/projection surface for DCC over work, session, approval, mailbox, governance, and evidence state.
- DCC UI packets can consume this backend without inventing local authority or repo-path coupling.
- Local-small-model routing can read compact readiness, blockers, queue reasons, and session state from the DCC backend without transcript or Markdown replay.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the governance workflow mirror plus the missing session substrate packets.
- Depends on structured collaboration artifacts, role mailbox, workflow correlation, and Locus query/autosync state already being recorder-visible and joinable.

## RISKS / UNKNOWNs (DRAFT)
- Risk: jumping straight to UI would hide missing backend joins behind ad hoc frontend state and create rework.
- Risk: if DCC backend treats repo-governance overlay as universal authority, it will collapse non-software and broader Handshake governance layers into software-delivery assumptions.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Dev-Command-Center-Control-Plane-Backend-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Dev-Command-Center-Control-Plane-Backend-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
