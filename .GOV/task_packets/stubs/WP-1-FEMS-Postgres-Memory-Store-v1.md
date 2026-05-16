# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Postgres-Memory-Store-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Postgres-Memory-Store-v1
- BASE_WP_ID: WP-1-FEMS-Postgres-Memory-Store
- CREATED_AT: 2026-05-05T17:55:00Z
- STUB_STATUS: SUPERSEDED (FOLDED INTO WP-1-Postgres-Control-Plane-Shift-Bundle-v1)
- FOLDED_INTO: WP-1-Postgres-Control-Plane-Shift-Bundle-v1
- KERNEL002_TRANSITIVE_FOLDED_INTO: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- KERNEL002_FOLD_STATUS: FULL_STUB_FOLDED_TRANSITIVE
- KERNEL_RESET_TRANSFERRED_TO: WP-KERNEL-001-Event-Ledger-Session-Broker-v1
- KERNEL_RESET_TRANSFER_SCOPE: Only session lifecycle/checkpoint vocabulary and explicit memory-runtime deferral moved into Kernel001; full FEMS memory-store runtime remains downstream residual scope.
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Postgres-Primary-Control-Plane-Foundation, WP-1-Postgres-Dev-Test-Container-Matrix, WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-ModelSession-Postgres-Queue-Workers, WP-1-Front-End-Memory-System
- BUILD_ORDER_BLOCKS: WP-1-FEMS-Bitemporal-Indexing, WP-1-FEMS-Working-Memory-Checkpoint-Schema, WP-1-FEMS-Injection-Scoring-Graceful-Degradation, WP-1-FEMS-Outcome-Feedback-Loop, WP-1-FEMS-Pinned-Core-Memory
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ROADMAP_POINTER: SPEC_CURRENT FEMS and storage portability anchors
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - FEMS integration anchors around Master Spec line 95.
  - ModelSession memory policy anchor around Master Spec line 11967.

## INTENT (DRAFT)
- What: Make PostgreSQL the authoritative store for FEMS memory records, memory packs, write-time safeguards, memory job claims, and replayable memory selection metadata.
- Why: Parallel models need shared memory systems that survive process boundaries and can be queried, replayed, compacted, and guarded without SQLite entering Handshake as runtime authority, cache, fixture, fallback, compatibility path, or temporary adapter.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - PostgreSQL tables or storage capability APIs for MemoryItem, MemoryPack, memory-job, provenance, tombstone, and replay metadata.
  - Parallel-safe memory write/read operations integrated with ModelSession IDs and memory policies.
  - Hooks for bitemporal indexing, pinned core memory, injection scoring, and outcome feedback follow-up stubs.
  - Tests for concurrent writes, duplicate prevention, tombstone/replay behavior, and failed memory-job recovery.
- OUT_OF_SCOPE:
  - Prompt-engineering changes to memory injection content.
  - UI dashboard work beyond projection-ready status/evidence fields.
  - Vector embedding optimization unless required for schema viability.

## ACCEPTANCE_CRITERIA (DRAFT)
- Memory records written by one model session are visible to other eligible sessions through PostgreSQL authority.
- Concurrent writes cannot silently overwrite or duplicate canonical memory items.
- MemoryPack compilation records enough provenance for replay and later FEMS evaluation.
- SQLite is rejected for cache, offline, index, fixture, fallback, compatibility, harness, example, temporary-adapter, and test roles. PostgreSQL remains the only accepted storage target for runtime memory state.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on PostgreSQL foundation, test matrix, lease/backpressure primitives, ModelSession queue workers, and validated FEMS baseline.
- Blocks the existing FEMS follow-up stubs that assume a durable shared memory substrate.

## RISKS / UNKNOWNs (DRAFT)
- Risk: memory poisoning protections may need to land with the first Postgres memory store slice rather than as a later refinement.
- Unknown: whether pgvector is in scope for Phase 1 or whether embeddings remain a separate index/cache concern.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body or create/approve spec enrichment first.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create the signed refinement.
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Postgres-Memory-Store-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
