# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Governance-Pack-v1

## STUB_METADATA
- WP_ID: WP-1-Governance-Pack-v1
- BASE_WP_ID: WP-1-Governance-Pack
- CREATED_AT: 2026-01-12T21:49:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: AÂ§7.6.3 (Phase 1) -> governance kernel adoption; local-first agentic posture
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - AÂ§7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
  - AÂ§7.5.4 Governance Kernel (HARD)
  - AÂ§2.6.8 Prompt-to-Spec Governance Pipeline (Normative)

## INTENT (DRAFT)
- What: Implement Governance Pack generation/instantiation in Handshake so projects can adopt the same strict workflow without Handshake-specific hardcoding.
- Why: Handshake must "use itself" as a governance engine for other projects; the repo workflow becomes a reusable product capability.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - `ProjectIdentity` schema and persistence (project_code, naming_policy, language/layout profile, external tool paths).
  - Governance Pack manifest and instantiation logic (templates + mechanical gate semantics + command surface contract).
  - No Handshake-hardcoded file naming when instantiating a non-Handshake project.
  - Conformance harness requirement for alternate implementations, including explicit "intent" equivalence notes.
- OUT_OF_SCOPE:
  - Changing the current repo governance implementation files (Codex/protocol/scripts) as a refactor.
  - Shipping additional product subsystems unrelated to governance.

## ACCEPTANCE_CRITERIA (DRAFT)
- A new project can be bootstrapped with a project-specific Codex + Master Spec naming policy (underscore style, deterministic).
- Language/layout guardrails exist per project (no Handshake path coupling).
- External tool paths are prompted/configured per project and persist deterministically.
- Conformance checks can prove gate semantics equivalence across implementations.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Workspace file I/O and settings persistence.
- Template system and deterministic rendering rules.
- Capability/consent model for repo writes and exporting governance artifacts.

## RISKS / UNKNOWNs (DRAFT)
- Drift between "reference repo implementation" and product Governance Pack templates.
- Cross-platform path differences and quoting rules.
- Over-tight coupling between Governance Pack and Git-only workflows.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Governance-Pack-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Governance-Pack-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


