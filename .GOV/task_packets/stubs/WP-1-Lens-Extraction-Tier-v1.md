# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Lens-Extraction-Tier-v1

## STUB_METADATA
- WP_ID: WP-1-Lens-Extraction-Tier-v1
- BASE_WP_ID: WP-1-Lens-Extraction-Tier
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> [ADD v02.123] Implement LensExtractionTier plumbing (Tier1 default) and surface it in Lens job traces
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md Addendum: 2.3 Two different â€œtiersâ€ (do not confuse) + LensExtractionTier type
  - Handshake_Master_Spec_v02.123.md Addendum: 4.3 Tier1 default selection (HARD) + Addendum: 14.3 Validators (ATELIER-LENS-VAL-TIER-001)

## INTENT (DRAFT)
- What: Implement `LensExtractionTier` (Tier1 default) as a first-class runtime/planning input and ensure it is visible in Lens job traces.
- Why: Separate extraction depth/compute budget from NSFW/SFW governance (`content_tier`) and ensure deterministic, auditable extraction behavior.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Schema/model updates to represent `LensExtractionTier` (Tier0/Tier1/Tier2) in Lens jobs.
  - Default posture: Tier1 unless explicitly overridden; Tier0/Tier2 require explicit selection/escalation.
  - Trace/provenance: Lens job traces record requested vs effective tier.
  - Validation: block invalid tier usage and enforce Tier1 default where required.
- OUT_OF_SCOPE:
  - Tier2 auto-when-idle scheduling (explicitly scheduled in Phase 2 per Roadmap).
  - Implementing new Tier2 detectors/enrichment content.

## ACCEPTANCE_CRITERIA (DRAFT)
- Tier1 is the default LensExtractionTier when unspecified.
- Any Tier0/Tier2 run records explicit override intent and shows up in Lens job traces.
- Validator enforces default Tier1 and rejects misuse where the spec requires Tier1.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Lens job model + trace model surfaces, and role pass execution wiring.
- Requires: clear separation from `content_tier` governance/projection fields.

## RISKS / UNKNOWNs (DRAFT)
- Risk: confusing `LensExtractionTier` with `content_tier` in UI/UX; must keep them orthogonal and explicit.
- Risk: â€œimplicit Tier2â€ creep via heuristics; must remain explicit in Phase 1.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Lens-Extraction-Tier-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Lens-Extraction-Tier-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


