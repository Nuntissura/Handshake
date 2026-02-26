# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Spec-Router-CapabilitySnapshot-v1

## STUB_METADATA
- WP_ID: WP-1-Spec-Router-CapabilitySnapshot-v1
- BASE_WP_ID: WP-1-Spec-Router-CapabilitySnapshot
- CREATED_AT: 2026-02-26T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.139.md 7.6.3 (Phase 1) -> [ADD v02.139] Prompt->Spec hardening quartet component 2/3: CapabilitySnapshot explicit allowlist enforcement.
- ROADMAP_ADD_COVERAGE: SPEC=v02.139; PHASE=7.6.3; LINES=46156
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 2.6.8.5.3 CapabilitySnapshot (Normative) [ADD v02.139]
  - Handshake_Master_Spec_v02.139.md 2.6.8 invariants [ADD v02.139 CapabilitySnapshot requirement for spec_router jobs]
  - Handshake_Master_Spec_v02.139.md 2.6.8.9 Integration Hooks (Normative) [ADD v02.139 provenance persistence]

## INTENT (DRAFT)
- What: Add a machine-checkable CapabilitySnapshot artifact path so `spec_router` can only reference explicitly allowed capabilities/tools.
- Why: Prevent spec hallucinations and non-executable plans caused by inventing unavailable tools, surfaces, or engines.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define/generate CapabilitySnapshot artifact payload from active CapabilityRegistry + SemanticCatalog mapping.
  - Wire `capability_snapshot_ref` into SpecIntent and SpecRouterDecision.
  - Enforce allowlist-only references during spec authoring and downstream checks.
  - Persist CapabilitySnapshot ref/hash in provenance/deep-link surfaces.
- OUT_OF_SCOPE:
  - Tool registry redesign beyond required snapshot extraction.
  - SpecPromptCompiler implementation details (separate packet).
  - SpecLint rule taxonomy implementation beyond snapshot-related failures.

## ACCEPTANCE_CRITERIA (DRAFT)
- Every `spec_router` run emits exactly one CapabilitySnapshot artifact referenced by SpecIntent and SpecRouterDecision.
- Snapshot contains only allowed capabilities/tools for the active route/mode and excludes forbidden entries.
- Any spec reference outside the snapshot is detected as a deterministic failure path.
- CapabilitySnapshot ref/hash appears in persisted provenance and deep links.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: deterministic prompt compilation packet for compiler integration point.
- Coordinates with: SpecLint packet for enforcing capability-reference validation outcomes.

## RISKS / UNKNOWNs (DRAFT)
- Risk: snapshot over-inclusion weakens safety by allowing accidental capability creep.
- Risk: snapshot under-inclusion blocks legitimate routes unless generation rules are transparent and deterministic.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Spec-Router-CapabilitySnapshot-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Spec-Router-CapabilitySnapshot-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
