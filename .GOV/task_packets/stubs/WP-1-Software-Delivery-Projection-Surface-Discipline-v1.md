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

# Work Packet Stub: WP-1-Software-Delivery-Projection-Surface-Discipline-v1

## STUB_METADATA
- WP_ID: WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- BASE_WP_ID: WP-1-Software-Delivery-Projection-Surface-Discipline
- CREATED_AT: 2026-04-17T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.181.md 7.6.3 (Phase 1) -> [ADD v02.181] Projection-surface discipline
- ROADMAP_ADD_COVERAGE: SPEC=v02.181; PHASE=7.6.3; LINES=47943,48046
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.181.md 2.6.8.8 Spec Session Log (Task Board + Work Packets) (Normative)
  - Handshake_Master_Spec_v02.181.md 2.6.8.10 Role Mailbox (Normative)
  - Handshake_Master_Spec_v02.181.md 7.2 Multi-Agent Orchestration
  - Handshake_Master_Spec_v02.181.md 10.11 Dev Command Center (Sidecar Integration)

## INTENT (DRAFT)
- What: Define and later implement the projection-discipline contract that keeps Dev Command Center, Task Board, and Role Mailbox as projection or control surfaces over one runtime truth for software-delivery work.
- Why: `v02.181` explicitly forbids layouts, inbox chronology, unread state, or Markdown mirror freshness from becoming hidden authority. Projection behavior now needs its own backlog slice instead of being implied by older UI work.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Projection-surface rules for Dev Command Center, Task Board, and Role Mailbox on software-delivery profiles.
  - Canonical field-provenance and action-preview contracts that explain which runtime fields and governed actions each surface is showing.
  - Guardrails that prevent lane placement, thread order, or mirror freshness from silently deciding validation, ownership, or completion posture.
  - Surface-level rules for when a projection may steer state through governed actions versus merely explain or summarize state.
- OUT_OF_SCOPE:
  - Cosmetic UI redesign.
  - General-purpose layout registry work outside the software-delivery projection-discipline problem.
  - Human-readable mirrors becoming canonical state.

## ACCEPTANCE_CRITERIA (DRAFT)
- Dev Command Center, Task Board, and Role Mailbox can all explain the same software-delivery runtime truth without contradicting one another.
- Projection surfaces can show validation, ownership, follow-up, and recovery posture without deriving authority from lane placement, inbox chronology, or mirror freshness.
- Operators can preview the governed target fields or actions behind a projection-surface interaction before mutation.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on DCC control-plane work and shared structured-collaboration contracts so all surfaces read from the same canonical records.
- Depends on Role Mailbox integration because software-delivery projection discipline specifically includes mailbox-linked waiting and follow-up state.

## RISKS / UNKNOWNs (DRAFT)
- Risk: projection-only surfaces continue to accumulate hidden workflow logic that diverges from the governed action model.
- Risk: Task Board and mailbox-derived views drift into second-authority behavior if field provenance and preview rules remain implicit.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Software-Delivery-Projection-Surface-Discipline-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Software-Delivery-Projection-Surface-Discipline-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
