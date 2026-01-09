# Work Packet Stub: WP-1-Global-Silent-Edit-Guard

## STUB_METADATA
- WP_ID: WP-1-Global-Silent-Edit-Guard
- CREATED_AT: 2026-01-08
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.102.md 7.6.3 item 23 ([ADD v02.102] Global Silent Edit Guard (WP-1-Global-Silent-Edit-Guard))
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.102.md 2.9.3 Mutation Traceability (normative)
  - Handshake_Master_Spec_v02.102.md 2.9.3.1 Persistence Schema (Normative)
  - Handshake_Master_Spec_v02.102.md 2.9.3.2 Storage Guard Trait
  - Handshake_Master_Spec_v02.102.md 2.9 Deterministic Edit Process (COR-701)

## INTENT (DRAFT)
- What: Enforce the "No Silent Edits" invariant at the storage boundary so AI-authored writes are rejected unless they carry valid job/workflow context and persisted MutationMetadata.
- Why: Prevent untraceable AI mutations; make audit/debug bundles and provenance reliable and enforceable.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Implement/finish the StorageGuard policy enforcement for all persistence writes (documents/blocks/canvas/etc):
    - If actor is AI, require non-null job_id and workflow_id.
    - Generate and persist a unique edit_event_id per mutation.
    - Emit normalized diagnostics on rejection (`HSK-403-SILENT-EDIT`).
  - Ensure required MutationMetadata columns exist and are populated on every write path (schema + application enforcement).
  - Ensure violations surface in Operator Consoles Problems/Evidence (diagnostics + trace linkage).
- OUT_OF_SCOPE:
  - User-facing editor UX work beyond minimal error surfacing.
  - Multi-user attribution rules (Phase 4).

## ACCEPTANCE_CRITERIA (DRAFT)
- Any attempted AI write without job_id/workflow_id is rejected deterministically with `HSK-403-SILENT-EDIT`.
- All AI-authored content rows contain non-null last_job_id/last_workflow_id and a stable edit_event_id.
- At least one end-to-end rewrite flow shows MutationMetadata persisted and discoverable via diagnostics and Flight Recorder linkage.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Storage abstraction boundary exists and all writes route through it (WP-1-Storage-Abstraction-Layer / successors).
- Migration framework exists to apply required schema changes (WP-1-Migration-Framework / successors).

## RISKS / UNKNOWNs (DRAFT)
- Cross-cutting surface area: multiple write paths may bypass a single guard call if not audited carefully.
- Schema requirements may differ between SQLite and PostgreSQL; must remain portable.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm authoritative SPEC_ANCHOR set (Main Body sections above; not Roadmap).
- [ ] Produce in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Global-Silent-Edit-Guard.md` (approved/signed).
- [ ] Create official task packet via `just create-task-packet WP-1-Global-Silent-Edit-Guard` (in `docs/task_packets/`).
- [ ] Move Task Board entry out of STUB into Ready for Dev (In Progress is handled via Validator status-sync when work begins).
