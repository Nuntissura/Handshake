# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1

## STUB_METADATA
- WP_ID: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- BASE_WP_ID: WP-1-Postgres-Structured-Collaboration-Artifact-Parity
- CREATED_AT: 2026-04-03T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Trait-Purity, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Migration-Framework, WP-1-Dual-Backend-Tests
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: Post-smoketest product follow-on after WP-1-Storage-Trait-Purity-v1; storage portability + Locus structured collaboration backend parity
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
  - Handshake_Master_Spec_v02.179.md 2.3.15 Locus Work Tracking System (storage architecture, query contract, sync, and multi-user posture)
  - Handshake_Master_Spec_v02.179.md Canonical structured collaboration artifact family [ADD v02.167]
  - Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
  - Handshake_Master_Spec_v02.179.md Project-agnostic workflow-state and transition contract [ADD v02.171, ADD v02.172]

## Why this stub exists
This stub exists because `WP-1-Storage-Trait-Purity-v1` intentionally stopped at honest capability law. It removed hidden backend branching, but it did not implement true PostgreSQL support for structured collaboration artifacts or the Locus-backed canonical work-state paths that consume them.

Current code still returns `StorageError::NotImplemented("structured collaboration artifacts")` from `PostgresDatabase` for `structured_collab_work_packet_row`, `structured_collab_work_packet_rows`, `structured_collab_micro_task_metadata`, `structured_collab_micro_task_status_rows`, and `structured_collab_micro_task_rows`, while SQLite remains the only real implementation surface.

## Prior packet
- Prior WP_ID: `WP-1-Storage-Trait-Purity-v1`
- Prior packet: `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/packet.md`

## Known current gap (Task Board summary)
- / STUB: PostgreSQL still capability-denies canonical Work Packet, Micro-Task, and Task Board artifact paths, so backend portability is honest but not yet parity-complete for structured collaboration state.

## INTENT (DRAFT)
- What: implement real PostgreSQL parity for canonical structured collaboration artifacts and the minimum Locus-backed read/write paths needed to keep Work Packets, Micro-Tasks, and Task Board projections backend-portable.
- Why: as long as Postgres can only deny these paths, structured collaboration remains effectively SQLite-only and the product cannot honestly claim portable canonical work-state persistence beyond the local-first backend.

## CURRENT_CODE_SURFACES (DRAFT)
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/src/locus/types.rs`
- `src/backend/handshake_core/src/locus/task_board.rs`
- `src/backend/handshake_core/src/storage/tests.rs`
- `src/backend/handshake_core/migrations/`

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - add portable schema + migration coverage for the structured collaboration tables or equivalent shared storage layout used by:
    - canonical Work Packet records
    - canonical Micro-Task records
    - Task Board projection rows
    - any required dependency or summary linkage those records rely on
  - implement PostgreSQL-backed storage methods so these no longer return `NotImplemented` for supported flows:
    - `structured_collab_work_packet_row`
    - `structured_collab_work_packet_rows`
    - `structured_collab_micro_task_metadata`
    - `structured_collab_micro_task_status_rows`
    - `structured_collab_micro_task_rows`
    - any required write/update path such as `locus_task_board_update_work_packet` or a bounded subset of `execute_locus_operation`
  - preserve the same base-envelope, compact-summary, `workflow_state_family`, `queue_reason_code`, `allowed_action_ids`, `project_profile_kind`, and `mirror_state` semantics on both backends
  - add dual-backend conformance tests for canonical read/list/update flows and negative-path regression checks
  - make any remaining unsupported Locus operations explicit and narrow instead of hiding them behind a blanket backend denial
- OUT_OF_SCOPE:
  - Dev Command Center UI work
  - mailbox/viewer/layout registry work that only consumes structured artifacts
  - CRDT or real-time multi-user conflict resolution beyond the bounded storage contract required for structured artifact parity

## ACCEPTANCE_CRITERIA (DRAFT)
- PostgreSQL no longer returns `NotImplemented("structured collaboration artifacts")` for the canonical structured artifact flows claimed by the packet.
- The same canonical structured Work Packet, Micro-Task, and Task Board record families round-trip on SQLite and PostgreSQL with matching base-envelope semantics.
- Dual-backend tests prove at least one read-single, read-list, status/metadata, and update/sync path for structured collaboration records on both backends.
- Any remaining unsupported Locus operations are individually declared and tested instead of piggybacking on a broad backend denial.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Storage Trait Purity is already done and gives this packet an honest capability boundary.
- Structured collaboration schema/contract packets are already done and provide the canonical data contract this packet must preserve.
- Migration strategy must stay compatible with the broader no-runtime-DDL direction; refinement must choose whether to reuse compatibility shims temporarily or move the shared tables fully into numbered migrations.

## RISKS / UNKNOWNs (DRAFT)
- Scope can silently explode into "full Locus Postgres backend" unless the refinement keeps the contract bounded to canonical structured artifact parity.
- Schema choices that are too SQLite-shaped may force a later rewrite instead of true portable parity.
- Parity work touches shared workflow and task-board projection code, so regression coverage must be broader than a single storage-file patch.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
