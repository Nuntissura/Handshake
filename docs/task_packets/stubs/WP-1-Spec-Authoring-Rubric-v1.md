# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `docs/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `docs/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Spec-Authoring-Rubric-v1

## STUB_METADATA
- WP_ID: WP-1-Spec-Authoring-Rubric-v1
- BASE_WP_ID: WP-1-Spec-Authoring-Rubric
- CREATED_AT: 2026-01-12T21:49:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: A§7.6.3 (Phase 1) -> Spec Router + governance session log (MVP)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - A§2.6.8.11 Spec Authoring Rubric (Normative)
  - A§11.1.5.1 Required Roles: Fixed Trinity (Normative)
  - A§2.6.8 Prompt-to-Spec Governance Pipeline (Normative)

## INTENT (DRAFT)
- What: Make Spec Router produce high-quality, audit-ready specs with deterministic required sections, minimal clarifying questions, and trinity enforcement for GOV_STANDARD/STRICT.
- Why: Spec creation is a core omission in the governance kernel; without it, complex work devolves into prompt interpretation drift.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Spec template enforcement (required headings) for GOV_STANDARD and GOV_STRICT.
  - Clarifying-question policy (3-7 max) + assumption recording (`NEEDS_CONFIRMATION`).
  - Trinity enforcement in Spec Router decisions; block when missing roles.
  - Spec artifacts are artifact-first (no reliance on chat transcripts).
- OUT_OF_SCOPE:
  - Implementing unrelated product features.
  - Weakening governance modes to reduce friction (GOV_STANDARD/STRICT remain strict).

## ACCEPTANCE_CRITERIA (DRAFT)
- GOV_STANDARD/STRICT spec outputs include all required headings and are mechanically checkable.
- Spec Router sets `required_roles` to include ORCHESTRATOR+CODER+VALIDATOR and blocks otherwise.
- Assumptions are explicitly captured and gated before execution when scope/safety relevant.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Spec Router pipeline and policy evaluation (11.1.5).
- Template system for spec artifacts.
- Capability system for gating spec generation and repo exports when needed.

## RISKS / UNKNOWNs (DRAFT)
- Over-constraining templates for creative tasks; ensure GOV_LIGHT remains available for small tasks.
- Ambiguous prompts causing incorrect assumptions; must surface `NEEDS_CONFIRMATION`.
- Consistency between spec rubric checks and validator expectations.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Spec-Authoring-Rubric-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Spec-Authoring-Rubric-v1` (in `docs/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `docs/TASK_BOARD.md` entry from STUB to Ready for Dev.

