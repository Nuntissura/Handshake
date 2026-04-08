# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Calibration-Dashboard-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Calibration-Dashboard-v1
- BASE_WP_ID: WP-1-FEMS-Calibration-Dashboard
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System, WP-1-FEMS-Hygiene-Manager-Job
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_ADD_COVERAGE: SPEC=v02.179; PHASE=7.6.3; LINES=TBD
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §10.11.5.14 Front End Memory Panel (FEMS) — Memory Browser, MemoryPack Preview, Conflict queue
  - §2.6.6.7.6.2 FEMS Design principles — hard budgets, deterministic degradation

## INTENT (DRAFT)
- What: Add operational health calibration signals to the DCC Front End Memory Panel so the operator can see whether FEMS is healthy or degrading. Ports the calibration signals pattern from repo governance memory hygiene reports.
- Why: The DCC Memory Panel (§10.11.5.14) specifies Memory Browser, Write Review, MemoryPack Preview, and Conflict queue — but no health dashboard. Without calibration signals, memory degradation (bloat, stale dominance, trust drift, embedding gaps) is invisible until it manifests as poor model behavior.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Calibration signal panel in DCC Memory Panel showing:
    - Active memory count vs budget cap (healthy <80% of cap).
    - Type distribution (working/episodic/semantic/procedural breakdown).
    - Trust distribution (user-explicit vs job-extracted vs auto-derived).
    - Novelty penalty rate (% of recent writes that hit dedup/novelty guard).
    - Session diversity (memories per source session in active pool; >8 = dominance).
    - Embedding coverage (% of active items with embeddings; <50% = degraded hybrid search).
    - Contradiction count (unresolved conflicts in queue).
    - Average importance of active items (0.3-0.7 healthy).
    - Last hygiene run timestamp and summary.
    - Retrieval degradation tier distribution (how often FTS5-only vs full hybrid).
  - Color-coded health status per signal (green/amber/red thresholds).
  - Hygiene report artifact viewer (output from FEMS Hygiene Manager Job).
- OUT_OF_SCOPE:
  - Memory Browser/Write Review/MemoryPack Preview (already in §10.11.5.14 base spec).
  - Backend hygiene logic (WP-1-FEMS-Hygiene-Manager-Job-v1).
  - Scoring formula implementation (WP-1-FEMS-Injection-Scoring-Graceful-Degradation-v1).

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; making memory health visible | Stub follow-up: THIS_STUB
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: extends DCC Memory Panel with calibration view | Stub follow-up: THIS_STUB
  - PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: calibration signals derived from FR-EVT-MEM-* event stream analysis | Stub follow-up: NONE
  - PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: calibration data is structured for model consumption (model can self-diagnose memory health) | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- DCC Memory Panel displays all calibration signals with color-coded health thresholds.
- Operator can see at a glance whether FEMS is healthy or needs attention.
- Hygiene report from most recent manager run is viewable from the panel.
- Signals update after each hygiene run and after each MemoryPack build.
- All calibration data is queryable by models (LLM-friendly data principle).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for LongTermMemory store.
- Depends on: WP-1-FEMS-Hygiene-Manager-Job-v1 for hygiene report artifacts.

## RISKS / UNKNOWNs (DRAFT)
- Risk: too many signals overwhelm the operator; need progressive disclosure (summary → detail drill-down).
- Risk: signal thresholds need tuning per workspace size; defaults from governance (500 cap, 30% novelty) are starting points.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Calibration-Dashboard-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Calibration-Dashboard-v1`.
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
