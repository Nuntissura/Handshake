# Work Packet Stub: WP-1-Dev-Experience-ADRs

## STUB_METADATA
- WP_ID: WP-1-Dev-Experience-ADRs
- CREATED_AT: 2026-01-07
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.101.md 7.6.3 item 9 (Dev experience and ADRs)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.101.md 2.6.6.6.3 Schema Contracts (AI Jobs, PlannedOperation, Flight Recorder) (ADRs referenced for schema changes)
  - Handshake_Master_Spec_v02.101.md 4.2.2.2 Ollama - The Easy Choice (includes one-command model run example)
- NOTES:
  - The Roadmap item includes "one-command dev startup" and ADR creation. If these are intended to be binding closure requirements, verify they exist in Main Body; otherwise spec enrichment may be required before activation.

## INTENT (DRAFT)
- What: Provide a one-command developer startup path (or equivalent) and create initial ADRs for key architectural choices (runtime, DB layout, capability model shape).
- Why: Phase 1 Roadmap requires developer onboarding and decision traceability to prevent drift.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define the minimal "one-command dev startup" target (likely via `justfile` + docs) including local model runtime (or mock) and sample jobs.
  - Add initial ADRs under `docs/adr/` for the required key choices.
- OUT_OF_SCOPE:
  - Full CI pipeline redesign or multi-platform packaging polish beyond Phase 1 needs.

## ACCEPTANCE_CRITERIA (DRAFT)
- New contributor can start the system in one command with a local model runtime (or mock) and execute at least one sample job.
- ADRs exist for runtime selection, jobs/Flight Recorder DB layout, and capability model shape, and are referenced where schema changes require ADR linkage.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Requires agreement on what "one-command dev startup" means in this repo (scripts vs just targets vs docs-only).
- If Main Body does not currently bind this requirement, activation may require a spec enrichment WP first.

## RISKS / UNKNOWNs (DRAFT)
- Risk of expanding into broad devops/build-system work; likely needs strict scoping.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm authoritative SPEC_ANCHOR (Main Body) or run spec enrichment workflow if needed.
- [ ] Produce in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Dev-Experience-ADRs.md` (approved/signed).
- [ ] Create official task packet via `just create-task-packet WP-1-Dev-Experience-ADRs` (in `docs/task_packets/`).
- [ ] Move Task Board entry out of STUB into Ready for Dev / In Progress as appropriate.

