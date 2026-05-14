# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-SQLite-Cache-Offline-Boundaries-v1

## STUB_METADATA
- WP_ID: WP-1-SQLite-Cache-Offline-Boundaries-v1
- BASE_WP_ID: WP-1-SQLite-Cache-Offline-Boundaries
- CREATED_AT: 2026-05-05T17:55:00Z
- STUB_STATUS: SUPERSEDED (FOLDED INTO WP-1-Postgres-Control-Plane-Shift-Bundle-v1)
- FOLDED_INTO: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- KERNEL002_TRANSITIVE_FOLDED_INTO: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- KERNEL002_FOLD_STATUS: FULL_STUB_FOLDED_TRANSITIVE
- KERNEL_RESET_TRANSFERRED_TO: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- KERNEL_RESET_TRANSFER_SCOPE: Kernel V1 no-SQLite-authority guard and leakage tripwire moved into Kernel001; broader cache/offline labeling remains residual product hardening scope.
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Storage-Abstraction-Layer
- BUILD_ORDER_BLOCKS: WP-1-DCC-Postgres-Control-Plane-Projections
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT local-first storage and storage portability anchors
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - SQLite index/cache anchors around Master Spec lines 2247-2264.
  - Storage portability anchors around Master Spec lines 3248-3520.

## INTENT (DRAFT)
- What: Define and implement the boundary between PostgreSQL-primary runtime authority and SQLite as cache, search index, embedded/offline mode, or rebuildable local projection.
- Why: Moving early to PostgreSQL-primary should not remove local-first ergonomics, but SQLite must stop being an accidental second authority for self-hosted control-plane state.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Clear storage-mode policy for Postgres-primary, SQLite-cache, SQLite-offline, and test modes.
  - Guardrails that prevent runtime control-plane writes from silently landing in SQLite when PostgreSQL authority is required.
  - Rebuild/invalidation semantics for SQLite indexes and local projections derived from PostgreSQL or file authorities.
  - Tests for mode detection, missing PostgreSQL behavior, and stale cache invalidation.
- OUT_OF_SCOPE:
  - Replacing PostgreSQL runtime storage.
  - Implementing full offline sync conflict resolution.
  - New UI beyond status/error surfaces needed for DCC projection.

## ACCEPTANCE_CRITERIA (DRAFT)
- The app can report which storage mode is active and which surfaces are authoritative.
- Runtime control-plane writes fail closed when PostgreSQL is required but unavailable.
- SQLite indexes/projections are rebuildable and carry freshness/source metadata.
- Tests prove SQLite fallback does not hide PostgreSQL outages for control-plane operations.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the foundation WP and storage abstraction layer.
- Blocks DCC projection work that needs to label runtime truth and derived/cache surfaces correctly.

## RISKS / UNKNOWNs (DRAFT)
- Risk: "fallback" can become silent split brain; activation should define explicit operator-visible degradation states.
- Unknown: whether embedded single-user demos should allow SQLite-primary mode behind a clear non-control-plane profile.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-SQLite-Cache-Offline-Boundaries-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
