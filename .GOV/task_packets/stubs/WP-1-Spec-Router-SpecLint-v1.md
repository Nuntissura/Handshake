# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Spec-Router-SpecLint-v1

## STUB_METADATA
- WP_ID: WP-1-Spec-Router-SpecLint-v1
- BASE_WP_ID: WP-1-Spec-Router-SpecLint
- CREATED_AT: 2026-02-26T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.139.md 7.6.3 (Phase 1) -> [ADD v02.139] Prompt->Spec hardening quartet component 3/3: SpecLint preflight gate (G-SPECLINT).
- ROADMAP_ADD_COVERAGE: SPEC=v02.139; PHASE=7.6.3; LINES=46156
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 2.6.8.5.4 SpecLint (Mechanical Preflight) (Normative) [ADD v02.139]
  - Handshake_Master_Spec_v02.139.md 2.6.8.9 Integration Hooks (Normative) [ADD v02.139 provenance persistence]

## INTENT (DRAFT)
- What: Implement the mechanical `G-SPECLINT` preflight that blocks structurally invalid or non-executable specs before rubric/red-team/decomposition stages.
- Why: Phase 1 Prompt->Spec flow requires deterministic early failure paths so bad specs do not leak into execution planning.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define/implement `G-SPECLINT` gate behavior for GOV_STANDARD and GOV_STRICT.
  - Produce SpecLintReport artifacts with stable rule IDs and machine-checkable findings.
  - Block progression on SpecLint failure and persist linkage into Job History + Spec Session Log.
  - Add CI preflight coverage for required spec output surfaces as defined by v02.139.
- OUT_OF_SCOPE:
  - Prompt compilation internals (separate SpecPromptCompiler packet).
  - CapabilitySnapshot generation logic (separate CapabilitySnapshot packet).
  - Rubric scoring and red-team logic beyond SpecLint handoff boundaries.

## ACCEPTANCE_CRITERIA (DRAFT)
- `G-SPECLINT` executes in the required governance modes and blocks progression on failure.
- Each run emits a SpecLintReport artifact linked to the governing spec job/session log records.
- Failure reasons are deterministic, code-based, and reproducible for the same input.
- CI runs SpecLint checks on required spec output classes and fails fast on violations.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: compiled prompt + context provenance fields needed for lint diagnostics.
- Depends on: CapabilitySnapshot references for capability-reference lint rules.

## RISKS / UNKNOWNs (DRAFT)
- Risk: weak/ambiguous lint rules can allow non-executable specs to pass.
- Risk: over-strict rules can block valid specs unless rule taxonomy is explicit and versioned.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Spec-Router-SpecLint-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Spec-Router-SpecLint-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
