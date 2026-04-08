# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Bitemporal-Indexing-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Bitemporal-Indexing-v1
- BASE_WP_ID: WP-1-FEMS-Bitemporal-Indexing
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §2.6.6.7.6.2 FEMS Design principles — determinism and replay
  - §2.6.6.7.6.2.3 MemoryPack schema — memory_pack_hash, determinism_mode
  - §5.4.8 FEMS-EVAL-001.4 — determinism and replay tests

## INTENT (DRAFT)
- What: Add bi-temporal timestamps to MemoryItem — `valid_from`/`valid_until` (when the fact was true in the world) and `recorded_at`/`invalidated_at` (when we recorded/invalidated it). Ports the Graphiti/Zep temporal knowledge graph pattern.
- Why: FEMS has `created_at` and `last_verified_at` but cannot answer "what did the model know at time T?" Replay mode (FEMS-EVAL-001.4) requires reconstructing the exact MemoryPack from a past session — which means knowing which memories existed AND were considered valid at that moment. Without bi-temporal indexing, replay can't distinguish between "memory existed but was superseded" and "memory didn't exist yet".

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add `valid_from: ISO timestamp` (when the fact became true) and `valid_until: Option<ISO timestamp>` (when superseded/invalidated; null = still valid) to MemoryItem.
  - Add `recorded_at: ISO timestamp` (when we stored it) and `invalidated_at: Option<ISO timestamp>` (when tombstoned; null = active) as transaction-time fields.
  - MemoryPack compilation in replay mode filters by both valid_time AND transaction_time at the target timestamp — reconstructing the exact memory state the model saw.
  - Supersession (from Write-Time-Safeguards) sets `valid_until` on the old item when the new one is committed.
  - Tombstoning sets `invalidated_at` rather than deleting rows — preserving history for audit/replay.
  - Calendar pillar interaction: temporal memory queries can correlate with Calendar time windows ("what did the model remember during this work block?").
- OUT_OF_SCOPE:
  - Temporal knowledge graph (GraphRAG overlay — separate, higher effort).
  - Migration of existing MemoryItems (backfill valid_from from created_at).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; foundational schema change to MemoryItem | Stub follow-up: THIS_STUB
  - PILLAR: Calendar | STATUS: TOUCHED | NOTES: temporal memory queries correlate with Calendar ActivitySpan time windows | Stub follow-up: NONE
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: FR event timeline + memory valid_time enables "what did the model know when this FR event fired?" | Stub follow-up: NONE
  - PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: bi-temporal columns must use portable SQL (ISO text timestamps, no DB-specific temporal types) | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- MemoryItem has valid_from, valid_until, recorded_at, invalidated_at fields.
- Replay mode MemoryPack compilation at timestamp T produces the same pack as the original session at time T.
- Superseded items have valid_until set; tombstoned items have invalidated_at set.
- No rows are physically deleted — all history preserved for audit.
- FEMS-EVAL-001.4 replay tests pass with bi-temporal filtering.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for MemoryItem schema.
- Enhances: WP-1-FEMS-Acceptance-Replay-Eval-v1 for replay correctness.

## RISKS / UNKNOWNs (DRAFT)
- Risk: storage growth from never-delete policy. Mitigated by hygiene manager archiving very old invalidated rows.
- Risk: bi-temporal queries are more complex than single-timestamp. Need clean query helpers.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Bitemporal-Indexing-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Bitemporal-Indexing-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
