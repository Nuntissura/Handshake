# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Lens-ViewMode-v1

## STUB_METADATA
- WP_ID: WP-1-Lens-ViewMode-v1
- BASE_WP_ID: WP-1-Lens-ViewMode
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> [ADD v02.123] Implement ViewMode UI + enforcement for Lens outputs (SFW hard-drop projection)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md Addendum: 2.4 ViewMode (SFW/NSFW)
  - Handshake_Master_Spec_v02.123.md 6.3.3.5.7.22 NSFW/SFW policy (raw ingest; filtered view/output only) [ADD v02.123]

## INTENT (DRAFT)
- What: Implement `ViewMode` UI and runtime enforcement for Lens outputs, including strict SFW hard-drop projection.
- Why: Provide operator-controlled projection without mutating stored raw artifacts and without â€œblur-but-revealableâ€ leakage.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - `ViewMode` UI surface (NSFW default; explicit SFW toggle with clear labeling).
  - Enforcement: in `ViewMode="SFW"`, Lens outputs MUST exclude any non-`sfw` results (strict drop).
  - Evidence preservation: internal evidence pointers remain intact; ViewMode switching does not mutate stored Raw/Derived artifacts.
  - Traceability: ViewMode is treated as a metadata filter and recorded in traces.
- OUT_OF_SCOPE:
  - Rewriting/censoring stored descriptors/facts to satisfy SFW posture (explicitly forbidden).
  - â€œCollapsed but revealableâ€ UX for non-sfw items in SFW mode (explicitly forbidden).

## ACCEPTANCE_CRITERIA (DRAFT)
- In SFW ViewMode, non-sfw items never appear in Lens result sets (hard drop).
- Switching ViewMode does not change stored artifacts; only affects projection/output.
- QueryPlan/RetrievalTrace (or equivalent trace) records ViewMode as a filter.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: content tier classification in stored artifacts and Lens retrieval/projection pipeline.
- Requires: UI + retrieval path integration so ViewMode is consistently applied and traced.

## RISKS / UNKNOWNs (DRAFT)
- Risk: inconsistent enforcement across surfaces (panel vs search vs exports); must centralize the projection filter.
- Risk: accidental mutation of stored artifacts on toggle; must be prohibited and validated.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Lens-ViewMode-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Lens-ViewMode-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


