# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Injection-Scoring-Graceful-Degradation-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Injection-Scoring-Graceful-Degradation-v1
- BASE_WP_ID: WP-1-FEMS-Injection-Scoring-Graceful-Degradation
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_ADD_COVERAGE: SPEC=v02.179; PHASE=7.6.3; LINES=TBD
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §2.6.6.7.6.2 Design principles — hard budgets, deterministic degradation
  - §2.6.6.7.6.2.3 MemoryPack schema — token/item budgets, truncation warnings
  - §2.6.6.7.6.2.4 Read path — retrieve, rank, pack

## INTENT (DRAFT)
- What: Implement the deterministic scoring formula for MemoryPack item ranking and a progressive retrieval pipeline that degrades gracefully under CPU/GPU load instead of failing hard latency gates. Ports the injection scoring formula from repo governance and applies the progressive retrieval pattern from the memory systems research.
- Why: FEMS spec says "drop lowest priority items" on budget overflow but defines no scoring algorithm. The p95 500ms latency target is unrealistic under heavy load (parallel local models, browser, other apps). Handshake needs tiered degradation — skip expensive retrieval tiers under load, build MemoryPack from whatever tier completes, log the degradation.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Injection scoring formula: `importance * recency_decay * access_boost * scope_match * staleness_factor * trust_multiplier * session_diversity_cap`. Deterministic — same inputs always produce same ranking.
  - Progressive retrieval tiers: Tier 1 exact match (~0ms) → Tier 2 FTS5 keyword (~5ms) → Tier 3 vector similarity (~50ms) → Tier 4 graph traversal (~100ms). Each tier is optional; pack builds from whatever tiers complete within a configurable time budget.
  - Graceful degradation: under load, skip Tier 3-4 and build from FTS5-only. Log degradation tier in MemoryPack warnings. No hard failure — always produce a pack.
  - CRAG-style quality scoring: score retrieval confidence per item before inclusion. Below threshold → discard rather than waste the ≤24 item budget on low-quality matches.
  - Truncation determinism: when items exceed budget, drop by ascending score. Log truncated item IDs in MemoryPack warnings. Same score set always produces same truncation.
  - Spec update: replace hard p95 latency target with degradation tier definitions.
  - Cross-encoder reranking (research §4.1): optional Tier 2.5 — after FTS5 broad retrieval, narrow with BGE-reranker (self-hosted cross-encoder). +50-100ms but significantly better precision. Skipped under load (progressive degradation), used when latency budget allows.
  - Contextual compression at pack-build (research §4.1): after retrieval and scoring, compress each selected item's content to only the parts relevant to the current query/scope. EmbeddingsFilter (cheap, fast) for default, LLM-based extraction as optional high-quality mode. Allows fitting 30+ items in the ≤500 token budget instead of the ≤24 item hard cap.
- OUT_OF_SCOPE:
  - Embedding model selection (separate decision, nomic-embed-text recommended).
  - FEMS core write path (WP-1-Front-End-Memory-System-v1).
  - Hygiene/consolidation (WP-1-FEMS-Hygiene-Manager-Job-v1).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; defines the scoring and retrieval that builds every MemoryPack | Stub follow-up: THIS_STUB
  - PILLAR: RAG | STATUS: TOUCHED | NOTES: shares retrieval infrastructure (FTS5, vector, graph); progressive retrieval pattern applies to both | Stub follow-up: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: scoring formula determines what LLM-friendly memory data reaches the model | Stub follow-up: NONE
  - PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: degradation tiers interact with session scheduler load awareness | Stub follow-up: NONE
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: FR-EVT-MEM-004 records degradation tier used, items considered vs selected | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- Scoring formula produces identical rankings given identical inputs (deterministic).
- MemoryPack builds succeed under simulated heavy CPU load by degrading to FTS5-only tier.
- Degradation tier and skipped tiers are logged in MemoryPack warnings and FR-EVT-MEM-004.
- Low-confidence retrieval results (below configurable threshold) are excluded from pack.
- Truncation is deterministic and logged.
- FEMS-EVAL-001.1 (budget + truncation) passes with the new scoring formula.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for LongTermMemory store and MemoryPack schema.

## RISKS / UNKNOWNs (DRAFT)
- Risk: scoring formula weights need tuning; wrong weights could surface stale items over fresh ones. Default weights from governance (proven) reduce this risk.
- Risk: progressive retrieval may produce noticeably different MemoryPacks in replay mode when tiers differ. Replay mode should pin the tier used in the original run.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Injection-Scoring-Graceful-Degradation-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Injection-Scoring-Graceful-Degradation-v1`.
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
