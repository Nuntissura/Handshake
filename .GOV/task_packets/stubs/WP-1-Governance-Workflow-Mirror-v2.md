# Work Packet Stub: WP-1-Governance-Workflow-Mirror-v2

## STUB_METADATA
- WP_ID: WP-1-Governance-Workflow-Mirror-v2
- BASE_WP_ID: WP-1-Governance-Workflow-Mirror
- CREATED_AT: 2026-04-12
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Artifact-Registry, WP-1-Product-Governance-Check-Runner, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Workflow-Projection-Correlation, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Workflow-Transition-Automation-Registry, WP-1-Governance-Pack
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: 7.5.4 Governance Kernel: Mechanical Gated Workflow
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.180.md:7.5.4 (Governance Kernel: Mechanical Gated Workflow)
  - Handshake_Master_Spec_v02.180.md:2.6.8.8 (Spec Session Log)
  - Handshake_Master_Spec_v02.180.md:11.5.4 (FR-EVT-GOV-GATES-001 / FR-EVT-GOV-WP-001)

## INTENT (DRAFT)
- What: Finish the product-side workflow-mirror capability from current `main` by porting the missing per-WP gate-transition mirror slice from the historical `v1` branch without regressing later runtime work already integrated into product governance.
- Why: Current product only partially implements the workflow-mirror behavior described by the original WP. Activation visibility exists, but current `main` still lacks the full per-WP gate-transition event and structured gate/activation projection layer needed for authoritative product governance state.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add product-runtime support for per-WP governance gate transitions keyed by `work_packet_id`.
  - Emit and validate `FR-EVT-GOV-GATES-001` payloads for gate transitions on current `main`.
  - Project structured workflow-mirror gate and activation summaries into current product query surfaces.
  - Append workflow-facing Spec Session Log entries for gate transitions and activation on current `main`.
  - Add or update targeted tests proving the mirror slice on current product code, not on the stale historical branch.
- OUT_OF_SCOPE:
  - Reopening or mutating the historical `WP-1-Governance-Workflow-Mirror-v1` packet as live execution.
  - Whole-file replay of the old `v1` branch onto current `main`.
  - Repo-governance `.GOV` feature expansion beyond the new remediation packet, dossier, and governed records required for activation/closeout.

## ACCEPTANCE_CRITERIA (DRAFT)
- Current product exposes per-WP gate transition state without colliding across parallel WPs.
- Gate transitions and packet activations are visible through Flight Recorder and workflow-facing session-log/query surfaces on current `main`.
- The remediation preserves later product work already present on `main`, including newer session-spawn/checkpoint/workspace-isolation paths.
- New `FR-EVT-GOV-*` payloads and mirror summaries are schema-validated and covered by targeted tests on current product code.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Master Spec current main-body clauses remain authoritative via `.GOV/roles_shared/SPEC_CURRENT.md`.
- Historical `WP-1-Governance-Workflow-Mirror-v1` branch/worktree is source material only; all implementation decisions must be revalidated against current `main`.

## RISKS / UNKNOWNs (DRAFT)
- Porting only the missing workflow-mirror slice from a stale branch without regressing later `flight_recorder` and `workflows` changes already contained in `main`.
- Ensuring new mirror summaries fit current query contracts and session-log surfaces without widening scope into unrelated product-governance follow-ons.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement still exists in the current Master Spec main body and that the gap is product behavior, not packet paperwork.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain a fresh USER_SIGNATURE for this remediation packet.
- [ ] Create `.GOV/refinements/WP-1-Governance-Workflow-Mirror-v2.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Governance-Workflow-Mirror-v2`.
- [ ] Update traceability so `WP-1-Governance-Workflow-Mirror` points to the active remediation packet rather than the historical `v1` closure.
- [ ] Move the task-board entry out of STUB when the packet becomes active.
