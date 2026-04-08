# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Write-Time-Safeguards-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Write-Time-Safeguards-v1
- BASE_WP_ID: WP-1-FEMS-Write-Time-Safeguards
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_ADD_COVERAGE: SPEC=v02.179; PHASE=7.6.3; LINES=TBD
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §2.6.6.7.6.2.5 Write path — extract, validate, consolidate, commit
  - §2.6.6.7.6.2 Design principles — anti-poisoning, provenance mandatory

## INTENT (DRAFT)
- What: Implement four mechanical write-time safeguards in the FEMS validation step that run without LLM calls: novelty scoring, supersession, contradiction detection, and dedup. Ports battle-tested patterns from repo governance memory write-time guards.
- Why: Without write-time guards, the LongTermMemory store bloats with duplicates, contradictions accumulate silently, and stale procedural items persist alongside their replacements. These checks are cheap (FTS5 + metadata comparison), fast, and prevent problems that are expensive to fix later in consolidation.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Novelty scoring: FTS5 near-duplicate check at MemoryWriteProposal validation time. If a proposed item closely matches an existing item (same scope_ref, high FTS5 similarity), apply a 0.3x importance penalty. Log in MemoryCommitReport warnings.
  - Supersession: when a new procedural item targets the same scope_ref as an existing one, automatically mark the old item as superseded (status change via FR-EVT-MEM-005). The old item remains in store but is excluded from MemoryPack compilation.
  - Contradiction detection: when a new item has the same scope_ref but different content/summary than an existing item, flag both as conflicted. Route to DCC conflict queue rather than silently overwriting.
  - Dedup: exact match on (memory_class + type + scope_refs + summary hash) → skip the write entirely. Log as skipped in MemoryCommitReport.
  - All four guards run in the validate step of the FEMS write path, before any LLM-based consolidation.
  - Auto-validation against current state (VS Code Copilot pattern): at pack-build time, check that scope_refs still resolve to existing entities. If referenced file/entity is deleted or substantially changed, auto-flag the memory item. Cheap (file existence + mtime check), prevents injecting memories about deleted features.
  - JSONL audit trail (BEADS dual-layer pattern): every memory mutation (write, supersede, flag, tombstone) emits a JSONL line alongside the SQLite write. JSONL log is includable in debug bundles and Flight Recorder evidence exports. SQLite is speed, JSONL is audit.
- OUT_OF_SCOPE:
  - LLM-based consolidation (that's the consolidate step, not validate).
  - Anti-poisoning trust segmentation (WP-1-FEMS-Memory-Poisoning-Drift-Guardrails-v1).
  - Hygiene manager scheduling (WP-1-FEMS-Hygiene-Manager-Job-v1).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; hardening the write path | Stub follow-up: THIS_STUB
  - PILLAR: RAG | STATUS: TOUCHED | NOTES: FTS5 index shared with RAG hybrid retrieval infrastructure | Stub follow-up: NONE
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: supersession and contradiction actions emit FR-EVT-MEM-005 | Stub follow-up: NONE
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: contradiction conflicts surface in DCC conflict queue | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- Duplicate MemoryWriteProposal ops are skipped with logged rationale.
- Near-duplicate items receive novelty penalty visible in MemoryCommitReport.
- Superseded items are excluded from subsequent MemoryPack compilations.
- Contradictions are flagged and routed to DCC conflict queue, not silently overwritten.
- All four guards execute without LLM calls (pure mechanical).
- Write path latency increase from guards is <10ms per proposal.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for LongTermMemory store and FTS5 index.

## RISKS / UNKNOWNs (DRAFT)
- Risk: over-aggressive novelty scoring suppresses legitimately similar but distinct items. Threshold needs tuning.
- Risk: FTS5 similarity may not catch semantic duplicates that use different wording. Acceptable for Phase 1; embedding-based dedup is Phase 2.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Write-Time-Safeguards-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Write-Time-Safeguards-v1`.
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
