# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Atelier-Collaboration-Panel-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Collaboration-Panel-v1
- BASE_WP_ID: WP-1-Atelier-Collaboration-Panel
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> [ADD v02.123] Implement Atelier Collaboration Panel (selection-scoped) in the main text editor
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md Addendum: 14.2.1 Atelier Collaboration Panel (selection-scoped) (HARD)
  - Handshake_Master_Spec_v02.123.md Addendum: 14.3 Validators (ATELIER-LENS-VAL-SCOPE-001 / selection-bounded patchsets)

## INTENT (DRAFT)
- What: Implement an Atelier Collaboration Panel (selection-scoped) in text editor surfaces that runs role passes against the current selection and applies only range-bounded patches.
- Why: Provide safe, auditable in-editor collaboration without silent edits outside the operatorâ€™s selected span.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - A selection-scoped collaboration UI (panel/sidebar) that:
    - Shows all roles.
    - Allows each role to emit 0..n suggestions (multi-suggestion preferred).
    - Supports selecting one or more suggestions and applying them.
  - Strict range-bounded patch application for Monaco/Docs (byte-identical outside selection).
  - Validator enforcement rejecting any patch that modifies outside the selection (except explicit boundary-normalization if enabled).
  - Provenance logging for applied patches (role_id, contract_id, model_id/tool_id, evidence refs, before/after spans).
- OUT_OF_SCOPE:
  - True multi-user collaboration (CRDT, presence, conflict resolution).
  - Auto-apply without operator review/selection.

## ACCEPTANCE_CRITERIA (DRAFT)
- Applying suggestions never changes text outside the selected span (byte-identical outside the range).
- Any patch that touches outside the selection is rejected by validators.
- Every applied patch is recorded with provenance + evidence refs and is visible in audit surfaces (Flight Recorder / job history).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: role pass execution plumbing (claim/glance/extract/compose) and editor patchset apply pipeline.
- Requires: selection range canonicalization across editors (Monaco vs Docs) and a shared validator hook point.

## RISKS / UNKNOWNs (DRAFT)
- Risk: differing editor range semantics can create â€œoff-by-oneâ€ or normalization surprises; must define canonical span model.
- Risk: boundary-normalization exceptions can become a loophole; must be explicit, minimal, and validated.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Atelier-Collaboration-Panel-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Atelier-Collaboration-Panel-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


