# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `docs/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `docs/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Role-Turn-Isolation-v1

## STUB_METADATA
- WP_ID: WP-1-Role-Turn-Isolation-v1
- BASE_WP_ID: WP-1-Role-Turn-Isolation
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> [ADD v02.123] Implement role-turn isolation as the default execution mode for role passes
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md Addendum: 4.5 Role-turn isolation (recommended; determinism support)
  - Handshake_Master_Spec_v02.123.md Addendum: 3.2 Deterministic replay (HARD) (pins/tie-break persistence for replayability)

## INTENT (DRAFT)
- What: Make role-turn isolation the default execution mode for role passes (claim/glance/extract), resetting role/context windows between turns and recording per-turn pins for deterministic replay.
- Why: Reduce cross-role contamination and stabilize small local model behavior while keeping runs auditable and replayable.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define and implement a “role turn” execution model: explicit role reset + context window reset boundaries per role pass.
  - Persist per-turn pins/inputs so replays can follow the same effective turn boundaries and selected spans.
  - Ensure traces/provenance record requested vs effective execution mode (isolated vs non-isolated).
- OUT_OF_SCOPE:
  - Advanced scheduling/parallelism features (beyond what Phase 1 already ships).
  - Guarantees of identical model outputs; rely on deterministic replay artifacts (pins/tie-breaks) when instability exists.

## ACCEPTANCE_CRITERIA (DRAFT)
- Default mode executes each role pass as an isolated turn (role + context reset).
- Each turn records pins/inputs sufficient for deterministic replay (or explicit degraded markers).
- Cross-role context bleed is prevented by default in role pass execution paths.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: workflow/job execution runner and trace artifact plumbing (pins/hashes).
- Requires: clear definition of what constitutes a “turn reset” across supported model backends.

## RISKS / UNKNOWNs (DRAFT)
- Risk: added latency/cost due to isolation; must remain configurable but default-safe.
- Risk: unclear backend support for strict resets; must encode fallbacks and record effective mode.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Role-Turn-Isolation-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Role-Turn-Isolation-v1` (in `docs/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `docs/TASK_BOARD.md` entry from STUB to Ready for Dev.

