# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Spec-Router-SpecPromptCompiler-v1

## STUB_METADATA
- WP_ID: WP-1-Spec-Router-SpecPromptCompiler-v1
- BASE_WP_ID: WP-1-Spec-Router-SpecPromptCompiler
- CREATED_AT: 2026-02-26T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.139.md 7.6.3 (Phase 1) -> [ADD v02.139] Prompt->Spec hardening quartet component 1/3: deterministic SpecPromptPack + SpecPromptCompiler compilation.
- ROADMAP_ADD_COVERAGE: SPEC=v02.139; PHASE=7.6.3; LINES=46156
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 2.6.8.5.2 SpecPromptPack + SpecPromptCompiler (Normative) [ADD v02.139]
  - Handshake_Master_Spec_v02.139.md 2.6.8.9 Integration Hooks (Normative) [ADD v02.139 provenance persistence]

## INTENT (DRAFT)
- What: Implement deterministic prompt compilation for `spec_router` using a versioned SpecPromptPack, producing reproducible PromptEnvelope hashes and ContextSnapshot lineage.
- Why: Prompt->Spec routing must be replayable and auditable; deterministic compilation prevents hidden prompt drift.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add/validate SpecPromptPack asset location + loading contract for `spec_router`.
  - Implement deterministic SpecPromptCompiler rules (stable prefix/suffix assembly, token caps, truncation flags).
  - Emit compilation provenance fields (pack id/sha, context snapshot id, envelope hashes) to required artifacts/events.
  - Ensure PromptEnvelope + ContextSnapshot linkage is preserved for downstream validation.
- OUT_OF_SCOPE:
  - Capability allowlist generation logic (separate CapabilitySnapshot packet).
  - SpecLint rule execution engine (separate SpecLint packet).
  - Non-`spec_router` prompt compiler expansion.

## ACCEPTANCE_CRITERIA (DRAFT)
- `spec_router` uses a SpecPromptPack artifact and records both pack id and pack SHA-256 per run.
- PromptEnvelope compilation is deterministic for identical inputs (same stable/variable hashes + truncation outcomes).
- ContextSnapshot includes every compilation input handle/hash required by the spec.
- Provenance fields are visible in Flight Recorder and linked surfaces per integration hooks.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Coordinates with: WP-1-Spec-Router-CapabilitySnapshot-v1 for allowlist injection into compiled prompts.
- Coordinates with: WP-1-Spec-Router-SpecLint-v1 for post-draft gating before rubric/red-team/MT decomposition.

## RISKS / UNKNOWNs (DRAFT)
- Risk: pack/template drift can silently change model behavior if hash capture is incomplete.
- Risk: non-deterministic token truncation can break replay guarantees if compiler ordering/rules are not fixed.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Spec-Router-SpecPromptCompiler-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Spec-Router-SpecPromptCompiler-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
