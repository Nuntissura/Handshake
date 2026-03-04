# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-MTE-Summaries-v1

## STUB_METADATA
- WP_ID: WP-1-MTE-Summaries-v1
- BASE_WP_ID: WP-1-MTE-Summaries
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Micro-Task summary requirements)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Micro-Task Executor: per-MT and aggregate summaries
  - Handshake_Master_Spec_v02.139.md Artifact/provenance + traceability requirements

## INTENT (DRAFT)
- What: Generate and persist per-microtask summaries and an aggregate run summary, and link them via stable artifact refs.
- Why: Without summaries, runs are not reviewable/auditable, and downstream orchestration loses condensed state.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Persist summary artifacts:
    - per-MT summary (e.g., `mt_summaries/{mt_id}.md` or JSON+md)
    - aggregate summary for the full execution run
  - Populate summary references in ProgressArtifact / RunLedger entries.
  - Add targeted tests that fail if summary refs are not written.
- OUT_OF_SCOPE:
  - UI polish for summary rendering (Operator Consoles can be minimal first).

## ACCEPTANCE_CRITERIA (DRAFT)
- A completed Micro-Task Executor run writes summary artifacts and sets summary_ref fields.
- A targeted test verifies summary artifacts exist and are linked.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Artifact system foundations exist.
- Decide canonical artifact formats (md + json).

## RISKS / UNKNOWNs (DRAFT)
- Summary content must be leak-safe and policy-aligned if it includes tool outputs.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-MTE-Summaries-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-MTE-Summaries-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

