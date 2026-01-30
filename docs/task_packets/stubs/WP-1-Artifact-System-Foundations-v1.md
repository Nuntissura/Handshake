# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `docs/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `docs/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Artifact-System-Foundations-v1

## STUB_METADATA
- WP_ID: WP-1-Artifact-System-Foundations-v1
- BASE_WP_ID: WP-1-Artifact-System-Foundations
- CREATED_AT: 2026-01-30T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.123.md 7.6.3 (Phase 1) -> Artifact store bootstrap + Materialize API + Retention/pinning MVP (artifact system)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.123.md 2.3.10 Artifact System (incl. 2.3.10.9 Materialize semantics) (Normative)
  - Handshake_Master_Spec_v02.123.md 2.3.11 Data Retention and GC (Normative)

## INTENT (DRAFT)
- What: Ensure Phase 1 artifact system foundations are complete and consistently used across exports and jobs: artifact store bootstrap + single atomic Materialize API + retention/pinning/GC with visible reports.
- Why: Prevent untraceable side effects, ensure reproducible evidence bundles, and avoid disk bloat/caches drifting silently.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Artifact store bootstrap (workspace tree + manifests; SHA-256 everywhere).
  - A single “Materialize” API for all save/export-to-path writes (atomic tmp+rename; no UI bypass; capability-gated; Flight Recorder logged).
  - Retention/pinning MVP (pin/unpin + TTL + deterministic GC job/command; never deletes pinned; emits a retention report artifact).
  - Explicit enforcement that tooling/engines write via artifact-first outputs + materialize-only semantics (no random filesystem side effects).
- OUT_OF_SCOPE:
  - Multi-user sync/replication and remote blob storage GC.
  - Phase 2+ archival/export formats beyond Phase 1 bundles.

## ACCEPTANCE_CRITERIA (DRAFT)
- All export paths use a single atomic materialize implementation (no ad-hoc writes).
- Artifacts and bundles have deterministic hashing (SHA-256) with manifests present and stable.
- Retention/GC runs deterministically, never deletes pinned items, and produces a retention report artifact visible to Operator.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Capability SSoT enforcement, Flight Recorder logging, and bundle export foundations.
- Coordinates with: Debug Bundle / Workspace Bundle WPs (artifact/bundle usage and hashing) to avoid redundant implementations.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Multiple competing “export/materialize” implementations cause drift and bypass policies; must centralize.
- Risk: GC/retention bugs can delete important data; require strict invariants, dry-run mode, and audit artifacts.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Artifact-System-Foundations-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Artifact-System-Foundations-v1` (in `docs/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `docs/TASK_BOARD.md` entry from STUB to Ready for Dev.

